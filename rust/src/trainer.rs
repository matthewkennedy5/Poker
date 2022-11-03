use crate::card_utils;
use crate::card_utils::Card;
use crate::trainer_utils::*;
use crate::exploiter::*;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Write, BufReader};

// TODO: Use a parameter file
const NODES_PATH: &str = "products/nodes.bin";

pub fn train(iters: u64) {
    let mut rng = thread_rng();
    let mut deck = card_utils::deck();
    let mut nodes: HashMap<CompactInfoSet, Node> = HashMap::new();
    println!("[INFO] Beginning training.");
    let mut p0_util = 0.0;
    let mut p1_util = 0.0;
    let bar = card_utils::pbar(iters);
    for i in 0..iters {
        deck.shuffle(&mut rng);
        p0_util += iterate(DEALER, &deck, ActionHistory::new(), [1.0, 1.0], &mut nodes);
        deck.shuffle(&mut rng);
        p1_util += iterate(
            OPPONENT,
            &deck,
            ActionHistory::new(),
            [1.0, 1.0],
            &mut nodes,
        );
        if i % 100_000 == 0 {
            serialize_nodes(&nodes);
            exploitability(&nodes);
        }
        bar.inc(1);
    }
    bar.finish();
    // exploitability(&nodes);

    // view_preflop(&nodes);

    println!("{} nodes reached.", nodes.len());
    println!(
        "Utilities:
            Dealer:   {} BB/h,
            Opponent: {} BB/h",
        p0_util / (iters as f64) / (BIG_BLIND as f64),
        p1_util / (iters as f64) / (BIG_BLIND as f64),
    );

    serialize_nodes(&nodes);
    write_compact_blueprint(&nodes);
    // println!("Exploitability: {}", exploitability(&nodes));
}

pub fn view_preflop(nodes: &HashMap<InfoSet, Node>) {
    // Print the preflop strategy
    for (infoset, node) in nodes {
        if infoset.history.street == PREFLOP {
            if node.t > 500.0 {
                println!(
                    "{}: {:#?}t: {}\n",
                    infoset,
                    node.cumulative_strategy(),
                    node.t
                );
            }
        }
    }
}

pub fn load_nodes(path: &str) -> HashMap<CompactInfoSet, Node> {
    println!("[INFO] Loading strategy at {} ...", path);
    let file = File::open(path).expect("Nodes file not found");
    let reader = BufReader::new(file);
    let nodes = bincode::deserialize_from(reader).expect("Failed to deserialize nodes");
    println!("[INFO] Done loading strategy");
    nodes
}

fn serialize_nodes(nodes: &HashMap<CompactInfoSet, Node>) {
    // let bincode: Vec<u8> = bincode::serialize(nodes).unwrap();
    // let mut string_keys: HashMap<String, Node> = HashMap::new();
    // for (infoset, node) in nodes {
    //     let key: String = format!("{:?}", infoset);
    //     string_keys.insert(key, node.clone());
    // }
    // println!("{:#?}", string_keys);
    // let json: String = serde_json::to_string_pretty(&string_keys).unwrap();
    // std::fs::write(NODES_PATH, json).unwrap();
    // file.write_all(&json).unwrap();
    // println!("[INFO] Saved strategy to disk.");

    let bincode: Vec<u8> = bincode::serialize(nodes).unwrap();
    let mut file = File::create(NODES_PATH).unwrap();
    file.write_all(&bincode).unwrap();
    println!("[INFO] Saved strategy to disk.");

}

// The blueprint strategy has pre-sampled actions (rather than probability distributions)
// to save space (at the cost of increased worst-case exploitability). 
pub fn load_blueprint() -> HashMap<CompactInfoSet, Action> {
    println!("[INFO] Loading blueprint strategy...");
    let file = match File::open(BLUEPRINT_STRATEGY_PATH) {
        Err(_e) => {
            write_compact_blueprint(&load_nodes(NODES_PATH));
            File::open(BLUEPRINT_STRATEGY_PATH).unwrap()
        }
        Ok(f) => f,
    };
    let reader = BufReader::new(file);
    let blueprint = bincode::deserialize_from(reader).expect("Failed to deserialize blueprint");
    println!("[INFO] Done loading blueprint strategy");
    blueprint
}

fn iterate(
    player: usize,
    deck: &[Card],
    history: ActionHistory,
    weights: [f64; 2],
    nodes: &mut HashMap<CompactInfoSet, Node>,
) -> f64 {
    if history.hand_over() {
        return terminal_utility(&deck, history, player);
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let mut history = history.clone();
    let mut infoset = InfoSet::from_deck(&deck, &history);
    let mut node: Node = match nodes.get(&infoset.compress()) {
        Some(n) => n.clone(),
        None => Node::new(&infoset),
    };

    // If it's not our turn, we sample the other player's action from their
    // current policy, and load our node.
    let opponent = 1 - player;
    if history.player == opponent {
        history.add(&sample_action_from_node(&node));
        if history.hand_over() {
            return terminal_utility(&deck, history, player);
        }
        infoset = InfoSet::from_deck(&deck, &history);
        node = match nodes.get(&infoset.compress()) {
            Some(n) => n.clone(),
            None => Node::new(&infoset),
        };
    }

    // Grab the current strategy at this node
    let [p0, p1] = weights;
    let strategy = node.current_strategy(weights[player]);
    let mut utilities: HashMap<Action, f64> = HashMap::new();
    let mut node_utility = 0.0;

    // Recurse to further nodes in the game tree. Find the utilities for each action.
    for (action, prob) in strategy {
        let mut next_history = history.clone();
        next_history.add(&action);
        let new_weights = match player {
            0 => [p0 * prob, p1],
            1 => [p0, p1 * prob],
            _ => panic!("Bad player value"),
        };
        let utility = iterate(player, &deck, next_history, new_weights, nodes);
        utilities.insert(action, utility);
        node_utility += prob * utility;
    }
    // TODO: multithread here -- maybe just on the flop. Return a Vec<Node> of updated nodes

    // Update regrets
    for (action, utility) in &utilities {
        let regret = utilities.get(&action).unwrap() - node_utility;
        node.add_regret(&action, weights[opponent] * regret);
    }

    let updated = node.clone();
    nodes.insert(infoset.compress(), updated);
    node_utility
}
