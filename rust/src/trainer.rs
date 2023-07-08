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

pub fn train(iters: u64, eval_every: u64, warm_start: bool) {
    let deck = card_utils::deck();
    let nodes = if warm_start {
        load_nodes(&CONFIG.nodes_path)
    } else {
        Nodes::new()
    };
    println!("[INFO] Beginning training.");
    let num_epochs = iters / eval_every;
    for epoch in 0..num_epochs {
        println!("[INFO] Training epoch {}/{}", epoch + 1, num_epochs);
        let bar = card_utils::pbar(eval_every);
        (0..eval_every).into_par_iter().for_each(|i| {
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

        // Check what percent of nodes have t = 0
        let mut zero = 0;
        let mut total = 0;
        for (history, history_nodes) in nodes.dashmap.clone() {
            for n in history_nodes {
                total += 1;
                if n.t == 0 {
                    zero += 1;
                }
            }
        }
        println!("Percent zeros: {}", zero as f64 / total as f64);

        // Check how the 28o preflop node looks
        let o28 = InfoSet::from_hand(
            &card_utils::str2cards("2c8h"),
            &Vec::new(),
            &ActionHistory::new(),
        );
        println!("InfoSet: {o28}");
        println!("Actions: {:?}", o28.next_actions(&CONFIG.bet_abstraction));
        println!("Node: {:?}", nodes.get(&o28));
    }
    println!("{} nodes reached.", nodes.len());
}

pub fn load_nodes(path: &str) -> Nodes {
    let file = File::open(path).expect("Nodes file not found");
    let reader = BufReader::new(file);
    let nodes: Nodes = bincode::deserialize_from(reader).expect("Failed to deserialize nodes");
    let len = nodes.len();
    nodes
}

pub fn serialize_nodes(nodes: &Nodes) {
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
    bet_abstraction: &[Vec<f64>],
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
    weights: [f64; 2],
    nodes: &Nodes,
    bet_abstraction: &[Vec<f64>],
    remaining_depth: i32,
) -> f64 {
    if history.hand_over() {
        return terminal_utility(deck, history, player);
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let history = history.clone();
    let infoset = InfoSet::from_deck(deck, &history);
    let mut node = lookup_or_new(nodes, &infoset, bet_abstraction);

    let opponent = 1 - player;
    let actions = infoset.next_actions(bet_abstraction);
    let strategy = node.current_strategy(if history.player == player {
        weights[player]
    } else {
        0.0
    });
    let mut node_utility = 0.0;

    // Recurse to further nodes in the game tree. Find the utilities for each action.
    let utilities: SmallVec<[f64; NUM_ACTIONS]> = (0..actions.len())
        .map(|i| {
            let prob = strategy[i];

            // Pruning
            let player_prune = CONFIG.player_prune
                && node.regrets[i] < -100.0 * CONFIG.stack_size as f64
                && rand::thread_rng().gen_bool(0.95);
            let opp_prune = CONFIG.opp_prune && history.player == opponent && prob < 1e-6;
            if player_prune || opp_prune {
                return 0.0;
            }

            let mut next_history = history.clone();
            next_history.add(&actions[i]);

            let new_weights = match history.player {
                0 => [weights[0] * prob, weights[1]],
                1 => [weights[0], weights[1] * prob],
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
            node_utility += prob * utility;
            utility
        })
        .collect();

    // Update regrets for the traversing player
    if history.player == player {
        for (index, utility) in utilities.iter().enumerate() {
            let regret = utility - node_utility;
            node.add_regret(index, weights[opponent] * regret);
        }
        nodes.insert(infoset, node, bet_abstraction);
    }
    node_utility
}
