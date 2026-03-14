use crate::readonly::{ReadOnlyAbstraction, ReadOnlyBot, ReadOnlyNodes};
use crate::trainer_utils::*;
use crate::{card_utils::*, OPPONENT};
use actix_files as fs;
use actix_web::{web, App, HttpServer, Responder};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

static BOT: Lazy<ReadOnlyBot> = Lazy::new(|| {
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
});

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

async fn get_cpu_action(infoset: web::Json<InfoSetJSON>) -> impl Responder {
    let cpu_cards = parse_cards(&infoset.cpuCards);
    let board = parse_cards(&infoset.board);
    let history = parse_history(&infoset.history, &infoset.stacks);

    println!("[BOT] cards={:?} board={:?} history={}", infoset.cpuCards, infoset.board, format_history(&infoset.history));

    let action = BOT.get_action(&cpu_cards, &board, &history);
    println!("[BOT] -> {:?}", action);
    let action_json = serde_json::to_string(&action).unwrap();
    action_json
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

async fn get_history_info(json: web::Json<HistoryAndCardsJSON>) -> impl Responder {
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
fn server_terminal_utility(
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
    // Force eager initialization so the first request isn't slow
    Lazy::force(&BOT);
    println!("[INFO] Bot loaded, starting server");
    HttpServer::new(|| {
        App::new()
            .route("/api/bot", web::post().to(get_cpu_action))
            .route("/api/historyInfo", web::post().to(get_history_info))
            .service(fs::Files::new("/", "../gui/build").index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
