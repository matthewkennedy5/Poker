use crate::card_abstraction::Abstraction;
use crate::card_utils;
use crate::card_utils::Card;
use crate::exploiter::exploitability;
use crate::trainer_utils::*;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Write};
use rayon::prelude::*;

// TODO: Use a parameter file
const BLUEPRINT_STRATEGY_PATH: &str = "products/blueprint.bin";

pub fn train(iters: u64) {
    let mut rng = thread_rng();
    let mut deck = card_utils::deck();
    let mut nodes: HashMap<InfoSet, Node> = HashMap::new();
    lazy_static::initialize(&HAND_TABLE);
    lazy_static::initialize(&ABSTRACTION);
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
        if i % 1_000_000 == 0 {
            serialize_strategy(&nodes);
        }
        bar.inc(1);
    }
    bar.finish();
    exploitability(&nodes);

    // view_preflop(&nodes);

    println!("{} nodes reached.", nodes.len());
    println!(
        "Utilities:
            Dealer:   {} BB/h,
            Opponent: {} BB/h",
        p0_util / (iters as f64) / (BIG_BLIND as f64),
        p1_util / (iters as f64) / (BIG_BLIND as f64),
    );

    serialize_strategy(&nodes);
    // println!("Exploitability: {}", exploitability(&nodes));
}

pub fn view_preflop(nodes: &HashMap<InfoSet, Node>) {
    // Print the preflop strategy
    for (infoset, node) in nodes {
        if infoset.history.street == PREFLOP {
            if node.t > 100.0 {
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

pub fn load_strategy() -> HashMap<InfoSet, Node> {
    println!("[INFO] Loading strategy...");
    let file = File::open(BLUEPRINT_STRATEGY_PATH).expect("Blueprint strategy file not found");
    let reader = BufReader::new(file);
    let nodes = bincode::deserialize_from(reader).expect("Failed to deserialize nodes");
    println!("[INFO] Done loading strategy");
    nodes
}

fn serialize_strategy(nodes: &HashMap<InfoSet, Node>) {
    let bincode: Vec<u8> = bincode::serialize(nodes).unwrap();
    let mut file = File::create(BLUEPRINT_STRATEGY_PATH).unwrap();
    file.write_all(&bincode).unwrap();
    println!("[INFO] Saved strategy to disk.");
}


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
    let mut infoset = InfoSet::from_deck(&deck, &history);
    if !nodes.contains_key(&infoset) {
        let new_node = Node::new(&infoset);
        nodes.insert(infoset.clone(), new_node);
    }
    let mut node: Node = nodes.get(&infoset).unwrap().clone();
    let mut history = history.clone();

    let opponent = 1 - player;
    if history.player == opponent {
        // Process the opponent's turn
        history.add(&sample_action_from_node(&node));

        if history.hand_over() {
            return terminal_utility(&deck, history, player);
        }
        infoset = InfoSet::from_deck(&deck, &history);
        if !nodes.contains_key(&infoset) {
            nodes.insert(infoset.clone(), Node::new(&infoset));
        }
        node = nodes.get(&infoset).unwrap().clone();
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

    // Update regrets
    for (action, utility) in &utilities {
        let regret = utilities.get(&action).unwrap() - node_utility;
        node.add_regret(&action, weights[opponent] * regret);
    }

    let updated = node.clone();
    nodes.insert(infoset, updated);
    node_utility
}
