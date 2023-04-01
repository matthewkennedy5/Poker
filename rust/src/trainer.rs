use crate::bot::Bot;
use crate::card_utils;
use crate::card_utils::Card;
use crate::config::CONFIG;
use crate::exploiter::*;
use crate::trainer_utils::*;
use dashmap::DashMap;
use rand::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};

// fn check_convergence(prev_node: &Node, curr_node: &Node) -> bool {
//     if curr_node.t < 1000.0 {
//         return false;
//     }
//     let curr_strat = curr_node.cumulative_strategy();
//     let prev_strat = prev_node.cumulative_strategy();
//     for (action, prob) in &curr_strat {
//         let prev_prob = prev_strat.get(&action).unwrap();
//         if (prob - prev_prob).abs() > 0.01 {
//             return false;
//         }
//         if prob.clone() == 1.0 / (curr_strat.len() as f64) {
//             // If the strategy is a uniform probability distribution, then it's stable but hasn't
//             // been trained yet
//             return false;
//         }
//     }
//     return true;
// }

// pub fn train_until_convergence() -> u64 {
//     let deck = card_utils::deck();
//     let mut nodes: Nodes = HashMap::new();
//     let starting_infoset = InfoSet::from_hand(
//         &card_utils::str2cards("AsAh"),
//         &Vec::new(),
//         &ActionHistory::new(),
//     );
//     let mut prev_node = Node::new(&starting_infoset, &CONFIG.bet_abstraction);
//     let mut counter: i32 = 1;
//     println!("[INFO] Beginning training.");
//     let bar = card_utils::pbar(1_000_000);
//     loop {
//         cfr_iteration(
//             &deck,
//             &ActionHistory::new(),
//             &mut nodes,
//             &CONFIG.bet_abstraction,
//         );
//         let node = lookup_or_new(&nodes, &starting_infoset, &CONFIG.bet_abstraction);
//         println!("{}, {}: {:?}", counter, node.t, node.regrets);
//         if check_convergence(&prev_node, &node) {
//             break;
//         }
//         prev_node = node.clone();
//         counter += 1;
//         bar.inc(1);
//     }
//     bar.finish();
//     counter as u64
// }

pub fn train(iters: u64) {
    let deck = card_utils::deck();
    let mut nodes: Nodes = DashMap::new();
    println!("[INFO] Beginning training.");
    let bar = card_utils::pbar(iters);
    for i in 1..iters + 1 {
        cfr_iteration(
            &deck,
            &ActionHistory::new(),
            &mut nodes,
            &CONFIG.bet_abstraction,
        );
        if i % CONFIG.eval_every == 0 {
            serialize_nodes(&nodes);
            let bot = Bot::new();
            exploitability(&bot, CONFIG.lbr_iters);
        }
        bar.inc(1);
    }
    bar.finish();

    println!("{} nodes reached.", nodes.len());
    serialize_nodes(&nodes);
}

pub fn load_nodes(path: &str) -> Nodes {
    println!("[INFO] Loading strategy at {path} ...");
    let file = File::open(path).expect("Nodes file not found");
    let reader = BufReader::new(file);
    let hashmap: HashMap<InfoSet, Node> =
        bincode::deserialize_from(reader).expect("Failed to deserialize nodes");
    let nodes: Nodes = hashmap
        .into_iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    println!("[INFO] Done loading strategy");
    nodes
}

fn serialize_nodes(nodes: &Nodes) {
    let nodes: HashMap<InfoSet, Node> = nodes
        .into_iter()
        .map(|entry| (entry.key().clone(), entry.value().clone()))
        .collect();
    let bincode: Vec<u8> = bincode::serialize(&nodes).unwrap();
    let mut file = File::create(&CONFIG.nodes_path).unwrap();
    file.write_all(&bincode).unwrap();
    println!("[INFO] Saved strategy.");
}

pub fn cfr_iteration(
    deck: &[Card],
    history: &ActionHistory,
    nodes: &Nodes,
    bet_abstraction: &Vec<Vec<f64>>,
) {
    [DEALER, OPPONENT].par_iter().for_each(|&player| {
        let mut deck = deck.to_vec();
        deck.shuffle(&mut rand::thread_rng());
        iterate(player, &deck, history, [1.0, 1.0], nodes, bet_abstraction);
    });
}

pub fn iterate(
    player: usize,
    deck: &[Card],
    history: &ActionHistory,
    weights: [f64; 2],
    nodes: &Nodes,
    bet_abstraction: &Vec<Vec<f64>>,
) -> f64 {
    if history.hand_over() {
        return terminal_utility(deck, history, player);
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let mut history = history.clone();
    let mut infoset = InfoSet::from_deck(deck, &history);
    let mut node = lookup_or_new(nodes, &infoset, bet_abstraction);

    // If it's not our turn, we sample the other player's action from their
    // current policy, and load our node.
    let opponent = 1 - player;
    if history.player == opponent {
        history.add(&sample_action_from_node(&mut node));
        if history.hand_over() {
            return terminal_utility(deck, &history, player);
        }
        infoset = InfoSet::from_deck(deck, &history);
        node = lookup_or_new(nodes, &infoset, bet_abstraction);
    }

    // Grab the current strategy at this node
    let [p0, p1] = weights;
    if weights[opponent] < 1e-6 {
        return 0.0;
    }
    let actions = infoset.next_actions(bet_abstraction);
    let strategy: Vec<f64> = node.current_strategy(weights[player]);
    let mut utilities: Vec<f64> = Vec::new();
    let mut node_utility = 0.0;

    // Recurse to further nodes in the game tree. Find the utilities for each action.
    for (action, prob) in actions.iter().zip(strategy.iter()) {
        let mut next_history = history.clone();
        next_history.add(action);
        let new_weights = match player {
            0 => [p0 * prob, p1],
            1 => [p0, p1 * prob],
            _ => panic!("Bad player value"),
        };
        let utility = iterate(
            player,
            deck,
            &next_history,
            new_weights,
            nodes,
            bet_abstraction,
        );
        utilities.push(utility);
        node_utility += prob * utility;
    }

    // Update regrets
    for (index, utility) in utilities.iter().enumerate() {
        let regret = utility - node_utility;
        node.add_regret(index, weights[opponent] * regret);
    }

    let updated = node.clone();
    nodes.insert(infoset, updated);
    node_utility
}
