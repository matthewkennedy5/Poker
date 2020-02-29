use crate::card_utils;
use crate::card_utils::Card;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::Write;

// TODO: Use a parameter file
const BLUEPRINT_STRATEGY_PATH: &str = "blueprint.json";

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
enum ActionType {
    fold,
    call,
    bet,
}

// TODO: Chance sample opponent actions or all possible opponent actions?

// Writes out the approximate Nash equilibrium strategy to a JSON
pub fn train(iters: i32) {
    println!("[INFO]: Beginning training.");
    let mut deck = card_utils::deck();
    let mut rng = &mut rand::thread_rng();
    let mut nodes: HashMap<InfoSet, Node> = HashMap::new();
    let bar = card_utils::pbar(iters as u64);
    for i in 0..iters {
        deck.shuffle(&mut rng);
        iterate(0, &deck, ActionHistory::new(), [0.0, 0.0], &mut nodes);
        deck.shuffle(&mut rng);
        iterate(1, &deck, ActionHistory::new(), [0.0, 0.0], &mut nodes);
        bar.inc(1);
    }
    bar.finish();

    // Convert nodes to have string keys for JSON serialization
    let mut str_nodes: HashMap<String, Node> = HashMap::new();
    for (infoset, node) in nodes {
        str_nodes.insert(infoset.to_string(), node);
    }

    let json = serde_json::to_string_pretty(&str_nodes).unwrap();
    let mut file = File::create(BLUEPRINT_STRATEGY_PATH).unwrap();
    file.write_all(json.as_bytes()).unwrap();
}

// Parser for the string format I'm using to store infoset keys in the JSON.
// Format example: "312|bet 500, call 500; fold"
fn str2infoset(str: String) -> InfoSet {
    // TODO
    InfoSet {
        history: ActionHistory::new(),
        card_bucket: 0,
    }
}

// START HERE: Implement the helper functions to get this thing churning out strategies!
// Incorporate the abstractions.

fn iterate(
    player: usize,
    deck: &[Card],
    history: ActionHistory,
    weights: [f64; 2],
    nodes: &mut HashMap<InfoSet, Node>,
) -> f64 {
    if history.hand_over() {
        return terminal_utility(&deck, history, player);
    }

    // Look up the CFR node for this information set, or make a new one if it
    // doesn't exist
    let mut infoset = InfoSet::from(&deck, &history);
    if !nodes.contains_key(&infoset) {
        nodes.insert(infoset.clone(), Node::new(&infoset));
    }
    let mut node = nodes.get(&infoset).unwrap();

    let opponent = 1 - player;
    if history.whose_turn() == opponent {
        // Process the opponent's turn
        history.add(opponent_action(node));
        if history.hand_over() {
            return terminal_utility(&deck, history, player);
        }
        infoset = InfoSet::from(&deck, &history);
        if !nodes.contains_key(&infoset) {
            nodes.insert(infoset.clone(), Node::new(&infoset));
        }
        node = nodes.get(&infoset).unwrap();
    }

    // Grab the current strategy at this node
    let [p0, p1] = weights;
    let strategy = node.current_strategy(weights[player]);
    let mut utilities: HashMap<Action, f64> = HashMap::new();
    let mut node_utility = 0.0;

    // Recurse to further nodes in the game tree. Find the utilities for each action.
    for (action, prob) in strategy {
        let mut next_history = history.clone();
        next_history.add(action.clone());
        let weights = match player {
            1 => [p0 * prob, p1],
            2 => [p0, p1 * prob],
            _ => panic!("Bad player value"),
        };
        let utility = iterate(player, &deck, next_history, weights, nodes);
        utilities.insert(action, utility);
        node_utility += prob * utility;
    }

    let node = nodes.get(&infoset).unwrap();

    // Update regrets
    for (action, utility) in &utilities {
        let regret = utilities.get(&action).unwrap() - node_utility;
        node.add_regret(&action, weights[opponent] * regret);
    }

    node_utility
}

fn opponent_action(node: &Node) -> Action {
    Action {
        action: ActionType::fold,
        amount: 0,
    }
}

fn terminal_utility(deck: &[Card], history: ActionHistory, player: usize) -> f64 {
    // TODO
    0.0
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
struct Action {
    action: ActionType,
    amount: i32,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let a = match &self.action {
            ActionType::fold => "fold",
            ActionType::call => "call",
            ActionType::bet => "bet",
        };
        write!(f, "{} {}", a, self.amount)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
struct ActionHistory {
    preflop: Vec<Action>,
    flop: Vec<Action>,
    turn: Vec<Action>,
    river: Vec<Action>,
}

impl ActionHistory {
    pub fn new() -> ActionHistory {
        ActionHistory {
            preflop: Vec::new(),
            flop: Vec::new(),
            turn: Vec::new(),
            river: Vec::new(),
        }
    }

    pub fn hand_over(&self) -> bool {
        // TODO
        false
    }

    pub fn whose_turn(&self) -> usize {
        // TODO
        return 0;
    }

    pub fn add(&self, action: Action) {
        // TODO
    }
}

impl fmt::Display for ActionHistory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::new();
        for street in &[&self.preflop, &self.flop, &self.turn, &self.river] {
            for action in street.to_vec() {
                result.push_str(&action.to_string());
                result.push_str(",");
            }
            result.push_str(";");
        }
        write!(f, "{}", result)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
struct InfoSet {
    history: ActionHistory,
    card_bucket: i32,
}

impl InfoSet {
    pub fn from(deck: &[Card], history: &ActionHistory) -> InfoSet {
        // TODO
        InfoSet {
            history: ActionHistory::new(),
            card_bucket: 0,
        }
    }
}

impl fmt::Display for InfoSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}|{}", self.card_bucket, self.history.to_string())
    }
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Node {
    // TODO: Does a Node really have to store its corresponding InfoSet?
    infoset: InfoSet,
    regrets: HashMap<Action, f64>,
    t: i32,
}

impl Node {
    pub fn new(infoset: &InfoSet) -> Node {
        Node {
            infoset: infoset.clone(),
            regrets: HashMap::new(),
            t: 0,
        }
    }

    pub fn current_strategy(&self, prob: f64) -> HashMap<Action, f64> {
        // TODO
        HashMap::new()
    }

    pub fn add_regret(&self, action: &Action, regret: f64) {
        // TODO
    }
}
