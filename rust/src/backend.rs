use crate::config::CONFIG;
use crate::readonly::{ReadOnlyAbstraction, ReadOnlyBot, ReadOnlyNodes};
use crate::search::{Decision, SearchBot};
use crate::trainer_utils::*;
use crate::{card_utils::*, OPPONENT};
use actix_files as fs;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Loads the blueprint bot from the readonly product files.
pub fn load_blueprint_bot() -> ReadOnlyBot {
    let abstraction = ReadOnlyAbstraction::open(
        "products/readonly_flop.bin",
        "products/readonly_turn.bin",
        "products/readonly_river.bin",
    );
    let nodes = ReadOnlyNodes::open(
        "products/readonly_nodes.idx",
        "products/readonly_nodes.dat",
        abstraction,
    );
    ReadOnlyBot::new(nodes)
}

/// Sizes rayon's global pool from the search config. Call once at startup.
pub fn init_thread_pool() {
    let threads = CONFIG.search.threads;
    if threads > 0 {
        if let Err(e) = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
        {
            println!("[WARN] Could not configure rayon pool: {}", e);
        }
    }
}

static BOT: Lazy<SearchBot> = Lazy::new(|| {
    SearchBot::new(Arc::new(load_blueprint_bot()), CONFIG.search.clone())
});

fn env_usize(name: &str) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(0)
}

/// Limits for a public deployment, from env vars (0 = off, the default, so local and
/// Slumbot-eval servers are unaffected):
///   POKER_MAX_SEARCHES   concurrent turn/river searches; others wait up to
///                        POKER_SEARCH_WAIT_MS (default 5000), then use book/blueprint
///   POKER_RATE_PER_MIN   API requests per client IP per minute
struct Limits {
    max_searches: usize,
    search_wait: Duration,
    rate_per_min: usize,
}

static LIMITS: Lazy<Limits> = Lazy::new(|| Limits {
    max_searches: env_usize("POKER_MAX_SEARCHES"),
    search_wait: Duration::from_millis(match env_usize("POKER_SEARCH_WAIT_MS") {
        0 => 5000,
        n => n as u64,
    }),
    rate_per_min: env_usize("POKER_RATE_PER_MIN"),
});

/// Counting semaphore for search slots.
static SEARCH_SLOTS: Lazy<(Mutex<usize>, Condvar)> = Lazy::new(|| (Mutex::new(0), Condvar::new()));

/// Takes a search slot, waiting up to `wait`. Returns false if none freed up.
fn acquire_search_slot(max: usize, wait: Duration) -> bool {
    let (lock, cv) = &*SEARCH_SLOTS;
    let deadline = Instant::now() + wait;
    let mut in_use = lock.lock().unwrap_or_else(|e| e.into_inner());
    while *in_use >= max {
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        in_use = cv.wait_timeout(in_use, deadline - now).unwrap_or_else(|e| e.into_inner()).0;
    }
    *in_use += 1;
    true
}

fn release_search_slot() {
    let (lock, cv) = &*SEARCH_SLOTS;
    let mut in_use = lock.lock().unwrap_or_else(|e| e.into_inner());
    *in_use = in_use.saturating_sub(1);
    cv.notify_one();
}

/// Requests per client IP in the current one-minute window.
static RATE: Lazy<Mutex<HashMap<String, (Instant, usize)>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// True if this client is over `POKER_RATE_PER_MIN`. The server only listens on
/// 127.0.0.1 behind Caddy, which sets X-Forwarded-For, so the real-IP header is trusted.
fn rate_limited(req: &HttpRequest) -> bool {
    let limit = LIMITS.rate_per_min;
    if limit == 0 {
        return false;
    }
    let ip = req.connection_info().realip_remote_addr().unwrap_or("unknown").to_string();
    let mut map = RATE.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    if map.len() > 10_000 {
        map.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
    }
    let entry = map.entry(ip).or_insert((now, 0));
    if now.duration_since(entry.0) >= Duration::from_secs(60) {
        *entry = (now, 0);
    }
    entry.1 += 1;
    entry.1 > limit
}

/// Parses and sanity-checks a bot request; Err(reason) for malformed or impossible
/// input (bad cards, duplicates, illegal action sequence, wrong board for the street).
fn parse_request(infoset: &InfoSetJSON) -> Result<(Vec<Card>, Vec<Card>, ActionHistory), String> {
    let parsed = catch_unwind(AssertUnwindSafe(|| {
        let cards = parse_cards(&infoset.cpuCards);
        let board = parse_cards(&infoset.board);
        (cards, board)
    }))
    .map_err(|_| "bad card string".to_string())?;
    let (cards, board) = parsed;
    if cards.len() != 2 {
        return Err("need exactly 2 hole cards".to_string());
    }
    if ![0, 3, 4, 5].contains(&board.len()) {
        return Err("board must have 0, 3, 4 or 5 cards".to_string());
    }
    let mut all: Vec<Card> = cards.iter().chain(board.iter()).cloned().collect();
    all.sort_by_key(|c| (c.rank, c.suit));
    all.dedup();
    if all.len() != cards.len() + board.len() {
        return Err("duplicate cards".to_string());
    }
    if infoset.history.len() > 64 {
        return Err("history too long".to_string());
    }
    let mut history = match &infoset.stacks {
        Some(s) => {
            let max = CONFIG.stack_size as Amount;
            if s.dealer <= 0 || s.opponent <= 0 || s.dealer > max || s.opponent > max {
                return Err("bad stack sizes".to_string());
            }
            ActionHistory::new_with_stacks([s.dealer, s.opponent])
        }
        None => ActionHistory::new(),
    };
    for a in &infoset.history {
        let action = Action {
            action: match a.action.as_str() {
                "Bet" => ActionType::Bet,
                "Call" | "Check" => ActionType::Call,
                "Fold" => ActionType::Fold,
                _ => return Err(format!("unknown action '{}'", a.action)),
            },
            amount: a.amount,
        };
        let legal = catch_unwind(AssertUnwindSafe(|| history.is_legal_next_action(&action))).unwrap_or(false);
        if !legal {
            return Err(format!("illegal action {:?}", action));
        }
        history.add(&action);
    }
    if history.hand_over() {
        return Err("hand is already over".to_string());
    }
    let need = match history.street {
        PREFLOP => 0,
        FLOP => 3,
        TURN => 4,
        _ => 5,
    };
    if board.len() != need {
        return Err(format!("street {} needs {} board cards, got {}", history.street, need, board.len()));
    }
    Ok((cards, board, history))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HandCompJSON {
    humanHand: Vec<String>,
    cpuHand: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ActionJSON {
    action: String,
    amount: Amount,
}

#[derive(Debug, Serialize, Deserialize)]
struct InfoSetJSON {
    cpuCards: Vec<String>,
    board: Vec<String>,
    history: Vec<ActionJSON>,
    stacks: Option<StacksJSON>,
}

async fn get_cpu_action(req: HttpRequest, infoset: web::Json<InfoSetJSON>) -> HttpResponse {
    if rate_limited(&req) {
        return HttpResponse::TooManyRequests().body("rate limit exceeded; slow down");
    }
    let (cpu_cards, board, history) = match parse_request(&infoset) {
        Ok(x) => x,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    println!("[BOT] cards={:?} board={:?} history={}", infoset.cpuCards, infoset.board, format_history(&infoset.history));

    // Search can take seconds; keep it off the async executor threads.
    let (cards2, board2, history2) = (cpu_cards.clone(), board.clone(), history.clone());
    let decision = match web::block(move || {
        let max = LIMITS.max_searches;
        if max == 0 {
            return BOT.decide(&cards2, &board2, &history2);
        }
        if acquire_search_slot(max, LIMITS.search_wait) {
            let d = catch_unwind(AssertUnwindSafe(|| BOT.decide(&cards2, &board2, &history2)));
            release_search_slot();
            match d {
                Ok(d) => d,
                Err(e) => std::panic::resume_unwind(e),
            }
        } else {
            println!("[BOT] all {} search slots busy; book/blueprint only", max);
            BOT.decide_with(&cards2, &board2, &history2, false)
        }
    })
    .await
    {
        Ok(d) => d,
        Err(e) => {
            println!("[BOT] blocking task failed ({}), using blueprint", e);
            let action = BOT.safe_blueprint_action(&cpu_cards, &board, &history);
            Decision {
                sampled: action.clone(),
                action,
                source: "blueprint",
                probs: Vec::new(),
                values: None,
                legalized: false,
            }
        }
    };
    println!("[BOT] -> {:?} ({})", decision.action, decision.source);
    // `action` and `amount` at the top level keep the response compatible with the
    // GUI; the rest is decision detail for evaluation logging.
    let resp = serde_json::json!({
        "action": decision.action.action,
        "amount": decision.action.amount,
        "source": decision.source,
        "sampled": decision.sampled,
        "legalized": decision.legalized,
        "probs": decision.probs,
        "values": decision.values,
    });
    HttpResponse::Ok().content_type("application/json").body(resp.to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HistoryInfo {
    pot: Amount,
    street: String,
    callAmount: Amount,
    minBetAmount: Amount,
    allInAmount: Amount,
    whoseTurn: String,
    stacks: StacksJSON,
    winnings: f64,
    handOver: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StacksJSON {
    dealer: Amount,
    opponent: Amount,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HistoryAndCardsJSON {
    history: Vec<ActionJSON>,
    dealerCards: Vec<String>,
    opponentCards: Vec<String>,
    boardCards: Vec<String>,
    stacks: Option<StacksJSON>,
}

async fn get_history_info(req: HttpRequest, json: web::Json<HistoryAndCardsJSON>) -> HttpResponse {
    if rate_limited(&req) {
        return HttpResponse::TooManyRequests().body("rate limit exceeded; slow down");
    }
    if json.history.len() > 64 {
        return HttpResponse::BadRequest().body("history too long");
    }
    match catch_unwind(AssertUnwindSafe(|| history_info(&json))) {
        Ok(body) => HttpResponse::Ok().content_type("application/json").body(body),
        Err(_) => HttpResponse::BadRequest().body("invalid hand state"),
    }
}

fn history_info(json: &HistoryAndCardsJSON) -> String {
    println!("[GAME] history={} dealer={:?} opponent={:?} board={:?}",
        format_history(&json.history), json.dealerCards, json.opponentCards, json.boardCards);
    let history = parse_history(&json.history, &json.stacks);
    let street = match history.street {
        0 => "preflop",
        1 => "flop",
        2 => "turn",
        3 => "river",
        4 => "showdown",
        _ => panic!("Bad street"),
    };
    let whose_turn = match history.player {
        0 => "dealer",
        1 => "opponent",
        _ => panic!("Bad player ID"),
    };
    let stack_sizes = history.stack_sizes();
    let stacks = StacksJSON {
        dealer: stack_sizes[DEALER],
        opponent: stack_sizes[OPPONENT],
    };

    let mut winnings = 0.0;
    if history.hand_over() {
        winnings = server_terminal_utility(
            &parse_cards(&json.dealerCards),
            &parse_cards(&json.opponentCards),
            &parse_cards(&json.boardCards),
            &history,
            DEALER,
        );
    }

    let history_info = HistoryInfo {
        pot: history.pot(),
        street: street.to_string(),
        callAmount: history.to_call(),
        minBetAmount: history.min_bet(),
        allInAmount: history.max_bet(),
        whoseTurn: whose_turn.to_string(),
        stacks,
        winnings,
        handOver: history.hand_over(),
    };

    serde_json::to_string(&history_info).unwrap()
}

fn format_history(h: &[ActionJSON]) -> String {
    h.iter().map(|a| {
        if a.amount > 0 { format!("{} ${}", a.action, a.amount) } else { a.action.clone() }
    }).collect::<Vec<_>>().join(" → ")
}

fn parse_history(h: &[ActionJSON], stacks: &Option<StacksJSON>) -> ActionHistory {
    let mut history = match stacks {
        Some(s) => ActionHistory::new_with_stacks([s.dealer, s.opponent]),
        None => ActionHistory::new(),
    };
    for action_json in h {
        let action = Action {
            action: match action_json.action.as_str() {
                "Bet" => ActionType::Bet,
                "Call" => ActionType::Call,
                "Check" => ActionType::Call,
                "Fold" => ActionType::Fold,
                _ => panic!("unexpected action string"),
            },
            amount: action_json.amount,
        };
        history.add(&action);
    }
    history
}

/// Like terminal_utility() but uses rs_poker directly instead of the 1.5GB
/// precomputed FAST_HAND_TABLE. Fine for the server which only evaluates
/// one showdown at a time.
pub fn server_terminal_utility(
    player_hand: &[Card],
    opp_hand: &[Card],
    board: &[Card],
    history: &ActionHistory,
    player: usize,
) -> f64 {
    if history.last_action().unwrap().action == ActionType::Fold {
        let winner = history.player;
        let folder = 1 - winner;
        let winnings = (history.starting_stacks()[folder] - history.stack_sizes()[folder]) as f64;
        return if winner == player { winnings } else { -winnings };
    }

    let pot = history.pot();
    let p_strength = hand_strength(&[player_hand, board].concat());
    let o_strength = hand_strength(&[opp_hand, board].concat());
    if p_strength > o_strength {
        pot as f64 / 2.0
    } else if p_strength < o_strength {
        -(pot as f64) / 2.0
    } else {
        0.0
    }
}

// Converts from the list ["5d", "7c", "Jh", "back", "back"] to the Vec<Card> representation
fn parse_cards(cards: &[String]) -> Vec<Card> {
    let mut cards: Vec<String> = cards.to_vec();
    cards.retain(|c| c != "back");
    let cardvec = cards.iter().map(|card| Card::new(card)).collect();
    cardvec
}

#[actix_rt::main]
pub async fn start_server() -> std::io::Result<()> {
    init_thread_pool();
    // Force eager initialization so the first request isn't slow
    Lazy::force(&BOT);
    println!(
        "[INFO] Bot loaded, starting server on port {}",
        std::env::var("POKER_PORT").unwrap_or_else(|_| "8080".to_string())
    );
    HttpServer::new(|| {
        App::new()
            .route("/api/bot", web::post().to(get_cpu_action))
            .route("/api/historyInfo", web::post().to(get_history_info))
            .service(fs::Files::new("/", "../gui/build").index_file("index.html"))
    })
    .bind(format!(
        "127.0.0.1:{}",
        std::env::var("POKER_PORT").unwrap_or_else(|_| "8080".to_string())
    ))?
    .run()
    .await
}
