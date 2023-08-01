use crate::bot::Bot;
use crate::trainer_utils::*;
use crate::trainer::load_nodes;
use crate::config::CONFIG;
use crate::{card_utils::*, OPPONENT};
use actix_cors::Cors;
use actix_files as fs;
use actix_web::{web, App, HttpServer, Responder};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

static BOT: Lazy<Bot> = Lazy::new(|| Bot::new(
    load_nodes(&CONFIG.nodes_path),
    CONFIG.subgame_solving
));

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
}

async fn get_cpu_action(infoset: web::Json<InfoSetJSON>) -> impl Responder {
    // println!("[INFO] Received CPU action request: {:#?}", infoset);

    let cpu_cards = parse_cards(&infoset.cpuCards);
    let board = parse_cards(&infoset.board);
    let history = parse_history(&infoset.history);

    let action = BOT.get_action(&cpu_cards, &board, &history);
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
}

async fn get_history_info(json: web::Json<HistoryAndCardsJSON>) -> impl Responder {
    let history = parse_history(&json.history);
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
        let mut cards: Vec<Card> = Vec::new();
        cards.extend(parse_cards(&json.dealerCards));
        cards.extend(parse_cards(&json.opponentCards));
        cards.extend(parse_cards(&json.boardCards));
        winnings = terminal_utility(&cards, &history, DEALER);
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

fn parse_history(h: &[ActionJSON]) -> ActionHistory {
    let mut history = ActionHistory::new();
    for action_json in h.clone() {
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

// Converts from the list ["5d", "7c", "Jh", "back", "back"] to the Vec<Card> representation
fn parse_cards(cards: &[String]) -> Vec<Card> {
    let mut cards: Vec<String> = cards.to_vec();
    cards.retain(|c| c != "back");
    let cardvec = cards.iter().map(|card| Card::new(card)).collect();
    cardvec
}

#[actix_rt::main]
pub async fn start_server() -> std::io::Result<()> {
    println!("[INFO] Launched server");
    HttpServer::new(|| {
        App::new()
            .route("/api/bot", web::post().to(get_cpu_action))
            .route("/api/historyInfo", web::post().to(get_history_info))
            .service(fs::Files::new("/", "../gui/build").index_file("index.html"))
            .wrap(Cors::permissive()
        )
    })
    .bind("0.0.0.0:80")?
    .run()
    .await
}
