use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use rand::thread_rng;
use rand::prelude::SliceRandom;
use crate::card_utils;
use crate::card_utils::Card;

const BLUEPRINT_STRATEGY_PATH: &str = "blueprint.json";

// Writes out the approximate Nash equilibrium strategy to a JSON
pub fn train(iters: i32) {
    println!("[INFO]: Beginning training.");
    let mut deck = card_utils::deck();
    let mut rng = &mut rand::thread_rng();
    let mut nodes: HashMap<InfoSet, Node> = HashMap::new();
    let bar = card_utils::pbar(iters as u64);
    for i in 0..iters {
        deck.shuffle(&mut rng);
        iterate(0, &deck, ActionHistory::new(), (0.0, 0.0));
        deck.shuffle(&mut rng);
        iterate(1, &deck, ActionHistory::new(), (0.0, 0.0));
        bar.inc(1);
    }
    bar.finish();

    let json = serde_json::to_string_pretty(&nodes).unwrap();
    let mut file = File::create(BLUEPRINT_STRATEGY_PATH).unwrap();
    file.write_all(json.as_bytes());
}

fn iterate(player: i32, deck: &[Card], history: ActionHistory, weights: (f64, f64)) -> i32 {
    if history.hand_over() {
        return terminal_utility(&deck, history, player);
    }
    return 0;
    // TODO
}

fn terminal_utility(deck: &[Card], history: ActionHistory, player: i32) -> i32 {
    // TODO
    0
}

#[derive(Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct Action {
    action: String,
    amount: i32
}

#[derive(Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct ActionHistory {
    preflop: Vec<Action>,
    flop: Vec<Action>,
    turn: Vec<Action>,
    river: Vec<Action>
}

impl ActionHistory {

    pub fn new() -> ActionHistory {
        ActionHistory {
            preflop: Vec::new(),
            flop: Vec::new(),
            turn: Vec::new(),
            river: Vec::new()
        }
    }

    pub fn hand_over(&self) -> bool {
        // TODO
        false
    }
}

#[derive(Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct InfoSet {
    history: ActionHistory,
    card_bucket: i32
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Node {
    infoset: InfoSet,
    regrets: HashMap<Action, f64>,
    t: i32
}