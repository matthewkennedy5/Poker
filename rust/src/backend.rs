use crate::bot::bot_action;
use crate::card_utils::{strvec2cards, Card, LightHandTable};
use crate::trainer_utils::{Action, ActionType, ActionHistory};
use actix_web::{http, web, App, HttpRequest, HttpServer, Responder};
use actix_cors::Cors;
use std::collections::HashMap;

const SERVER: &str = "127.0.0.1:8000";

lazy_static! {
    static ref HAND_STRENGTHS: LightHandTable = LightHandTable::new();
}

async fn compare_hands(req: HttpRequest) -> impl Responder {
    let query = req.query_string();
    let query = qstring::QString::from(query);
    let human_hand = query.get("humanHand").unwrap();
    let cpu_hand = query.get("cpuHand").unwrap();
    let human_hand = parse_cards(human_hand);
    let cpu_hand = parse_cards(cpu_hand);
    let human_strength = HAND_STRENGTHS.hand_strength(&human_hand);
    let cpu_strength = HAND_STRENGTHS.hand_strength(&cpu_hand);
    if human_strength > cpu_strength {
        return String::from("human");
    } else if cpu_strength > human_strength {
        return String::from("cpu");
    } else {
        return String::from("tie");
    }
}

async fn get_cpu_action(req: HttpRequest) -> impl Responder {
    let query = req.query_string();
    let query = qstring::QString::from(query);
    let cpu_cards = query.get("cpuCards").unwrap();
    let board = query.get("board").unwrap();
    let history_json = query.get("history").unwrap();

    let cpu_cards = parse_cards(cpu_cards);
    let board = parse_cards(board);
    let history = parse_history(history_json);
    let action = bot_action(&cpu_cards, &board, &history);
    let is_check = action == Action {action: ActionType::Call, amount: 0};
    let action_json = serde_json::to_string(&action).unwrap();

    // Need to translate from what serde outputs (Rust action representation)
    // to the action strings used by the Javascript code. Differences are in
    // capitalization, and "check" vs. just "call" with amount 0. The Rust
    // representation is more streamlined, and ideally both would be the same,
    // but for now it's easier to just convert between them.
    let mut action_json = action_json.replace("Bet", "bet")
                                 .replace("Call", "call")
                                 .replace("Fold", "fold");
    if is_check {
        action_json = action_json.replace("call", "check");
    }
    action_json
}

fn parse_history(history_json: &str) -> ActionHistory {
    let history_json = String::from(history_json);
    // The Javascript code uses "bet", "call", "check" for action types,
    // but we need "Bet", "Call", "Fold" (capitalized). So we have to replace those words in
    // the JSON, and replace "check" with "Call".
    let history_json = history_json.replace("bet", "Bet")
                                   .replace("call", "Call")
                                   .replace("check", "Call")
                                   .replace("fold", "Fold");
    let streets: HashMap<String, Vec<Action>> = serde_json::from_str(&history_json).unwrap();
    let mut history = ActionHistory::new();
    for street in &["preflop", "flop", "turn", "river"] {
        for action in streets.get(street.clone()).unwrap() {
            history.add(action);
        }
    }
    history
}

fn parse_cards(cards: &str) -> Vec<Card> {
    let mut cards: Vec<&str> = cards.split(",").collect();
    cards.retain(|&c| c != "back");
    let cards = strvec2cards(&cards);
    cards
}

#[actix_rt::main]
pub async fn main() -> std::io::Result<()> {
    println!("[INFO] Launching server at {}", SERVER);
    HttpServer::new(|| {
        App::new()
            .wrap(
                Cors::new()
                  .allowed_origin("http://localhost:3000")
                  .finish())
            .route("/compare", web::get().to(compare_hands))
            .route("/bot", web::get().to(get_cpu_action))
    })
    .bind(SERVER)?
    .run()
    .await
}
