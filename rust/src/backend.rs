use crate::bot::Bot;
use crate::card_utils::*;
use crate::trainer_utils::{Action, ActionHistory, ActionType};
use actix_cors::Cors;
use actix_files as fs;
use actix_web::{web, App, HttpServer, Responder};
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

static HAND_STRENGTHS: Lazy<LightHandTable> = Lazy::new(|| LightHandTable::new());
static BOT: Lazy<Bot> = Lazy::new(|| Bot::new());

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HandCompJSON {
    humanHand: Vec<String>,
    cpuHand: Vec<String>
}

async fn compare_hands(json: web::Json<HandCompJSON>) -> impl Responder {
    let human_hand = parse_cards(&json.humanHand);
    let cpu_hand = parse_cards(&json.cpuHand);
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

// TODO: Can I unify this with trainer_utils::Action?
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ActionJSON {
    action: String,
    amount: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HistoryJSON {
    preflop: Vec<ActionJSON>,
    flop: Vec<ActionJSON>,
    turn: Vec<ActionJSON>,
    river: Vec<ActionJSON>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InfoSetJSON {
    cpuCards: Vec<String>,
    board: Vec<String>,
    history: HistoryJSON,
}

async fn get_cpu_action(infoset: web::Json<InfoSetJSON>) -> impl Responder {
    println!("[INFO] Received CPU action request: {:#?}", infoset);

    let cpu_cards = parse_cards(&infoset.cpuCards);
    let board = parse_cards(&infoset.board);
    let history = parse_history(&infoset.history);

    let action = BOT.get_action(&cpu_cards, &board, &history);
    let is_check = action
        == Action {
            action: ActionType::Call,
            amount: 0,
        };
    let mut action_json = serde_json::to_string(&action).unwrap();
    if is_check {
        action_json = action_json.replace("Call", "Check");
    }
    action_json
}

fn parse_history(h: &HistoryJSON) -> ActionHistory {
    let mut all_actions: Vec<ActionJSON> = h.preflop.clone();
    all_actions.extend(h.flop.clone());
    all_actions.extend(h.turn.clone());
    all_actions.extend(h.river.clone());

    let mut history = ActionHistory::new();
    for action_json in all_actions {
        let action = Action {
            action: match action_json.action.as_str() {
                "Bet" => ActionType::Bet,
                "Call" => ActionType::Call,
                "Check" => ActionType::Call,
                "Fold" => ActionType::Fold,
                _ => panic!("unexpected action string")
            },
            amount: action_json.amount
        };
        history.add(&action);
    }
    history
}

// Converts from the list ["5d", "7c", "Jh", "back", "back"] to the Vec<Card> representation
fn parse_cards(cards: &[String]) -> Vec<Card> {
    let mut cards: Vec<String> = cards.to_vec();
    cards.retain(|c| c != "back");
    let cardvec = cards.iter().map(|card| Card::new(&card)).collect();
    cardvec
}

#[actix_rt::main]
pub async fn start_server() -> std::io::Result<()> {
    println!("[INFO] Launched server");
    HttpServer::new(|| {
        App::new()
            .route("/api/compare", web::post().to(compare_hands))
            .route("/api/bot", web::post().to(get_cpu_action))
            .service(fs::Files::new("/", "../gui/build").index_file("index.html"))
            .wrap(Cors::permissive())
    })
    .bind("0.0.0.0:80")?
    .run()
    .await
}
