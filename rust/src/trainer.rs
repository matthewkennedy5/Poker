use crate::card_utils;
use crate::card_utils::Card;
use crate::config::CONFIG;
use crate::exploiter::*;
use crate::nodes::*;
use crate::trainer_utils::*;
use rand::prelude::*;
use rayon::prelude::*;
use smallvec::SmallVec;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

pub fn train(iters: u64, warm_start: bool) {
    let deck = card_utils::deck();
    let nodes = if warm_start {
        load_nodes(&CONFIG.nodes_path)
    } else {
        Nodes::new()
    };
    println!("[INFO] Beginning training.");
    let num_epochs = iters / CONFIG.eval_every;
    for epoch in 0..num_epochs {
        println!("[INFO] Training epoch {}/{}", epoch + 1, num_epochs);
        let bar = card_utils::pbar(CONFIG.eval_every);
        (0..CONFIG.eval_every).into_par_iter().for_each(|i| {
            cfr_iteration(
                &deck,
                &ActionHistory::new(),
                &nodes,
                &CONFIG.bet_abstraction,
                -1,
            );
            bar.inc(1);
        });
        bar.finish_with_message("Done");
        serialize_nodes(&nodes);
        blueprint_exploitability(&nodes, CONFIG.lbr_iters);
    }

    // println!("{} nodes reached.", nodes.len());
    serialize_nodes(&nodes);
}

pub fn load_nodes(path: &str) -> Nodes {
    println!("[INFO] Loading strategy at {path} ...");
    let file = File::open(path).expect("Nodes file not found");
    let reader = BufReader::new(file);
    let nodes: Nodes = bincode::deserialize_from(reader).expect("Failed to deserialize nodes");
    let len = nodes.len();
    println!("[INFO] Done loading strategy: {len} nodes.");
    nodes
}

pub fn serialize_nodes(nodes: &Nodes) {
    // let nodes_vec: Vec<(InfoSet, Node)> = nodes
    //     .into_iter()
    //     .map(|entry| (entry.key().clone(), entry.value().clone()))
    //     .collect();
    let file = File::create(&CONFIG.nodes_path).unwrap();
    let mut buf_writer = BufWriter::new(file);
    bincode::serialize_into(&mut buf_writer, &nodes).expect("Failed to serialize nodes");
    buf_writer.flush().unwrap();

    println!("[INFO] Saved strategy.");
}

pub fn cfr_iteration(
    deck: &[Card],
    history: &ActionHistory,
    nodes: &Nodes,
    bet_abstraction: &Vec<Vec<f32>>,
    depth_limit: i32,
) {
    [DEALER, OPPONENT].iter().for_each(|&player| {
        let mut deck = deck.to_vec();
        deck.shuffle(&mut rand::thread_rng());
        iterate(
            player,
            &deck,
            history,
            [1.0, 1.0],
            &nodes,
            bet_abstraction,
            depth_limit,
        );
    });
}

pub fn iterate(
    player: usize,
    deck: &[Card],
    history: &ActionHistory,
    weights: [f32; 2],
    nodes: &Nodes,
    bet_abstraction: &[Vec<f32>],
    remaining_depth: i32,
) -> f32 {
    if history.hand_over() {
        return terminal_utility(deck, history, player);
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let mut history = history.clone();
    let mut infoset = InfoSet::from_deck(deck, &history);
    let mut node = lookup_or_new(nodes, &infoset, bet_abstraction);

    // Depth limited solving - just sample actions until the end of the game to estimate the utility
    // TODO: Let the opponent choose between several strategies
    if remaining_depth == 0 {
        loop {
            infoset = InfoSet::from_deck(deck, &history);
            node = lookup_or_new(nodes, &infoset, bet_abstraction);
            // TODO: Right now node will just be a uniform distribution strategy because it's not
            // trained. I think instead you want to sample the action from the blueprint.
            let action =
                sample_action_from_node(&mut node, &infoset.next_actions(bet_abstraction), true);
            history.add(&action);
            if history.hand_over() {
                return terminal_utility(deck, &history, player);
            }
        }
    }

    // If it's not our turn, we sample the other player's action from their
    // current policy, and load our node.
    let opponent = 1 - player;
    if history.player == opponent {
        history.add(&sample_action_from_node(&mut node, &infoset.next_actions(bet_abstraction), false));
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
    let strategy = node.current_strategy(weights[player]);
    let mut utilities: SmallVec<[f32; NUM_ACTIONS]> = SmallVec::with_capacity(NUM_ACTIONS);
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
            remaining_depth - 1,
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
