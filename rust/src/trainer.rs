use crate::config::CONFIG;
use crate::card_utils;
use crate::card_utils::Card;
use crate::trainer_utils::*;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Write, BufReader};

pub fn train(iters: u64) {
    let mut rng = thread_rng();
    let mut deck = card_utils::deck();
    let mut nodes: HashMap<InfoSet, Node> = HashMap::new();
    println!("[INFO] Beginning training.");
    let mut p0_util = 0.0;
    let mut p1_util = 0.0;
    let bar = card_utils::pbar(iters);
    for i in 0..iters {
        deck.shuffle(&mut rng);
        p0_util += iterate(DEALER, &deck, &ActionHistory::new(), [1.0, 1.0], &mut nodes);
        deck.shuffle(&mut rng);
        p1_util += iterate(
            OPPONENT,
            &deck,
            &ActionHistory::new(),
            [1.0, 1.0],
            &mut nodes,
        );
        if i % CONFIG.eval_every == 0 {
            serialize_nodes(&nodes);
            // exploitability(&nodes);
        }
        bar.inc(1);
    }
    bar.finish();

    println!("{} nodes reached.", nodes.len());
    println!(
        "Utilities:
            Dealer:   {} BB/h,
            Opponent: {} BB/h",
        p0_util / (iters as f64) / (CONFIG.big_blind as f64),
        p1_util / (iters as f64) / (CONFIG.big_blind as f64),
    );

    serialize_nodes(&nodes);
}

pub fn load_nodes(path: &str) -> HashMap<InfoSet, Node> {
    println!("[INFO] Loading strategy at {} ...", path);
    let file = File::open(path).expect("Nodes file not found");
    let reader = BufReader::new(file);
    let nodes = bincode::deserialize_from(reader).expect("Failed to deserialize nodes");
    println!("[INFO] Done loading strategy");
    nodes
}

fn serialize_nodes(nodes: &HashMap<InfoSet, Node>) {
    let bincode: Vec<u8> = bincode::serialize(nodes).unwrap();
    let mut file = File::create(&CONFIG.nodes_path).unwrap();
    file.write_all(&bincode).unwrap();
    println!("[INFO] Saved strategy.");
}

pub fn iterate(
    player: usize,
    deck: &[Card],
    history: &ActionHistory,
    weights: [f64; 2],
    nodes: &mut HashMap<InfoSet, Node>,
) -> f64 {
    if history.hand_over() {
        return terminal_utility(&deck, history, player);
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let mut history = history.clone();
    let mut infoset = InfoSet::from_deck(&deck, &history);
    let mut node: Node = match nodes.get(&infoset) {
        Some(n) => n.clone(),
        None => Node::new(&infoset),
    };

    // If it's not our turn, we sample the other player's action from their
    // current policy, and load our node.
    let opponent = 1 - player;
    if history.player == opponent {
        history.add(&sample_action_from_node(&node));
        if history.hand_over() {
            return terminal_utility(&deck, &history, player);
        }
        infoset = InfoSet::from_deck(&deck, &history);
        node = match nodes.get(&infoset) {
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
        // TODO: Add pruning for low probability actions. MCCFR?
        let mut next_history = history.clone();
        next_history.add(&action);
        let new_weights = match player {
            0 => [p0 * prob, p1],
            1 => [p0, p1 * prob],
            _ => panic!("Bad player value"),
        };
        let utility = iterate(player, &deck, &next_history, new_weights, nodes);
        utilities.insert(action, utility);
        node_utility += prob * utility;
    }

    // Update regrets
    for (action, utility) in &utilities {
        let regret = utility - node_utility;
        node.add_regret(&action, weights[opponent] * regret);
    }

    let updated = node.clone();
    nodes.insert(infoset, updated);
    node_utility
}
