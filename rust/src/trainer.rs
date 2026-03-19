use crate::card_utils;
use crate::card_utils::*;
use crate::config::CONFIG;
use crate::exploiter::*;
use crate::nodes::*;
use crate::ranges::Range;
use crate::trainer_utils::*;
use ahash::AHashMap as HashMap;
use rand::prelude::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

pub fn train(iters: usize, eval_every: usize, warm_start: bool) {
    let deck = card_utils::deck();
    let nodes = if warm_start {
        load_nodes(&CONFIG.nodes_path)
    } else {
        Nodes::new(&CONFIG.bet_abstraction)
    };
    println!("[INFO] Beginning training.");
    let mut csv = BufWriter::new(File::create("products/exploitability.csv").expect("Failed to create CSV"));
    writeln!(csv, "iteration,exploitability").unwrap();
    let num_epochs = iters / eval_every;
    for epoch in 0..num_epochs {
        println!("[INFO] Training epoch {}/{}", epoch + 1, num_epochs);
        let bar = card_utils::pbar(eval_every as usize);

        (0..eval_every).into_par_iter().for_each(|_| {
            cfr_iteration(&deck, &ActionHistory::new(), &nodes, -1);
            bar.inc(1);
        });
        bar.finish_with_message("Done");
        serialize_nodes(&nodes);
        let exploit = blueprint_exploitability(&nodes, CONFIG.lbr_iters);
        writeln!(csv, "{},{}", (epoch + 1) * eval_every, exploit).unwrap();
        csv.flush().unwrap();

        let infoset = InfoSet::from_hand(&str2cards("2c7h"), &Vec::new(), &ActionHistory::new());
        println!("InfoSet: {infoset}");
        println!(
            "Actions: {:?}",
            infoset.next_actions(&CONFIG.bet_abstraction)
        );
        println!("Node: {:?}", nodes.get(&infoset));

        // River convergence check: nut hand facing bet should never fold
        let river_infoset = InfoSet::from_hand(
            &str2cards("AhKh"),
            &str2cards("QhJhTh5c3d"),
            &ActionHistory::from_strings(vec![
                "Bet 250", "Call 250", "Call 0", "Call 0", "Call 0", "Call 0", "Bet 250",
            ]),
        );
        println!("River InfoSet: {river_infoset}");
        println!(
            "River Actions: {:?}",
            river_infoset.next_actions(&CONFIG.bet_abstraction)
        );
        println!("River Node: {:?}", nodes.get(&river_infoset));

        // Check what percent of nodes have t = 0
        let mut zero = 0;
        let mut total: u64 = 0;
        let mut total_t: u64 = 0;
        for elem in &nodes.dashmap {
            let node_vec = elem.value();
            let nodes_lock = node_vec.nodes.lock().unwrap();
            for node in nodes_lock.iter() {
                total += 1;
                total_t += node.t as u64;
                if node.t == 0 {
                    zero += 1;
                }
            }
        }
        println!("Fraction zeros: {}", zero as f64 / total as f64);
        println!(
            "Average t across all infosets: {}",
            total_t as f64 / total as f64
        );
    }
    println!("{} nodes reached.", nodes.len());
}

pub fn load_nodes(path: &str) -> Nodes {
    let file = File::open(path).expect("Nodes file not found");
    let reader = BufReader::new(file);
    let nodes: Nodes = bincode::deserialize_from(reader).expect("Failed to deserialize nodes");
    nodes
}

pub fn serialize_nodes(nodes: &Nodes) {
    let file = File::create(&CONFIG.nodes_path).unwrap();
    let mut buf_writer = BufWriter::new(file);
    bincode::serialize_into(&mut buf_writer, &nodes).expect("Failed to serialize nodes");
    buf_writer.flush().unwrap();
    println!("[INFO] Saved strategy.");
}

pub fn cfr_iteration(deck: &[Card], history: &ActionHistory, nodes: &Nodes, depth_limit: i32) {
    [DEALER, OPPONENT].iter().for_each(|&traverser| {
        let mut deck = deck.to_vec();
        deck.shuffle(&mut rand::thread_rng());
        let board = [deck[0], deck[1], deck[2], deck[3], deck[4]];
        let mut range = Range::new();
        range.remove_blockers(&board);
        let mut preflop_hands = Vec::with_capacity(range.hands.len());
        for hand_index in 0..range.hands.len() {
            let prob = range.probs[hand_index];
            if prob > 0.0 {
                preflop_hands.push(range.hands[hand_index]);
            }
        }

        let traverser_reach_probs = vec![1.0; preflop_hands.len()];
        let opp_reach_probs = vec![1.0; preflop_hands.len()];

        iterate(
            traverser,
            preflop_hands,
            board,
            &ActionHistory::new(),
            traverser_reach_probs,
            opp_reach_probs,
            nodes,
            -1,
            None,
            &*ABSTRACTION,
            None,
        );
    });
}

pub fn iterate(
    traverser: usize,
    preflop_hands: Vec<[Card; 2]>,
    board: [Card; 5],
    history: &ActionHistory,
    traverser_reach_probs: Vec<f64>,
    opp_reach_probs: Vec<f64>,
    nodes: &Nodes,
    depth_limit: i32,
    depth_limit_nodes: Option<&Nodes>,
    abstraction: &dyn CardAbstraction,
    precomputed_strengths: Option<&HashMap<[Card; 2], i32>>,
) -> Vec<f64> {
    let N = preflop_hands.len();
    if N == 0 {
        return Vec::new();
    }

    if history.hand_over() {
        return terminal_utility_vectorized(
            preflop_hands,
            opp_reach_probs,
            &board,
            history,
            traverser,
            precomputed_strengths,
        );
    }

    // TODO: Don't call this if you're training a blueprint.
    // if depth_limit < CONFIG.depth_limit && history.current_street_length == 0 {
    //     // depth limited solving for future streets

    //     return depth_limit_utility(
    //         traverser,
    //         preflop_hands,
    //         board,
    //         history,
    //         traverser_reach_probs,
    //         opp_reach_probs,
    //         depth_limit_nodes.expect("Depth limit nodes not provided"),
    //     );
    // }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let history = history.clone();
    let infosets: Vec<InfoSet> = preflop_hands
        .iter()
        .map(|h| InfoSet::from_hand_with_abstraction(h, &board, &history, abstraction))
        .collect();

    let strategies: Vec<SmallVecFloats> = nodes.get_current_strategy_vectorized(&infosets);
    let opponent = 1 - traverser;
    if history.player == traverser {
        nodes.update_strategy_sum_vectorized(&infosets, &traverser_reach_probs);
    }

    let actions = history.next_actions(&nodes.bet_abstraction);
    let mut node_utility: Vec<f64> = vec![0.0; N];
    // Recurse to further nodes in the game tree. Find the utilities for each action.
    let action_utilities: Vec<Vec<f64>> = (0..actions.len())
        .map(|i| -> Vec<f64> {
            // Maps traverser_preflop_hand to prob of taking this action
            let probs: Vec<f64> = strategies.iter().map(|s| s[i]).collect();
            let mut next_history = history.clone();
            next_history.add(&actions[i]);

            let mut traverser_reach_probs = traverser_reach_probs.clone();
            let mut opp_reach_probs = opp_reach_probs.clone();

            if history.player == traverser {
                for i in 0..N {
                    traverser_reach_probs[i] *= probs[i];
                }
            } else {
                debug_assert!(opp_reach_probs.len() == strategies.len());
                for i in 0..opp_reach_probs.len() {
                    opp_reach_probs[i] *= probs[i];
                }
            }

            let utility: Vec<f64> = iterate(
                traverser,
                preflop_hands.clone(),
                board,
                &next_history,
                traverser_reach_probs.clone(),
                opp_reach_probs.clone(),
                nodes,
                depth_limit - 1,
                depth_limit_nodes,
                abstraction,
                precomputed_strengths,
            );

            for n in 0..node_utility.len() {
                let prob: f64 = if history.player == traverser {
                    probs[n]
                } else {
                    1.0
                };
                node_utility[n] += prob * utility[n];
            }
            utility
        })
        .collect();

    // Update regrets for the traversing player
    if history.player == traverser {
        // Action utilities is shape [actions, traverser_hands]
        for (action_idx, action_utility) in action_utilities.iter().enumerate() {
            nodes.add_regret_vectorized(&infosets, action_utility, &node_utility, action_idx);
        }
    }
    node_utility
}

// For now, this samples rollouts at the depth limit to estimate the utility.
// https://arxiv.org/pdf/1805.08195.pdf
// Basically the same as iterate(), except:
// - get the strategies / prob updates from the depth_limit_bot
// - don't update any nodes
fn depth_limit_utility(
    traverser: usize,
    preflop_hands: Vec<[Card; 2]>,
    board: [Card; 5],
    history: &ActionHistory,
    traverser_reach_probs: Vec<f64>,
    opp_reach_probs: Vec<f64>,
    depth_limit_nodes: &Nodes,
    abstraction: &dyn CardAbstraction,
    precomputed_strengths: Option<&HashMap<[Card; 2], i32>>,
) -> Vec<f64> {
    let translated_history = history.translate(&depth_limit_nodes.bet_abstraction);
    if translated_history.hand_over() {
        return terminal_utility_vectorized(
            preflop_hands,
            opp_reach_probs,
            &board,
            history,
            traverser,
            precomputed_strengths,
        );
    }
    let translated_infosets: Vec<InfoSet> = preflop_hands
        .iter()
        .map(|h| InfoSet::from_hand_with_abstraction(h, &board, &translated_history, abstraction))
        .collect();

    let strategies: Vec<SmallVecFloats> =
        depth_limit_nodes.get_strategy_vectorized(&translated_infosets);

    // Sample a random action depending on the total probability for each action
    let N = preflop_hands.len();
    let next_actions = translated_history.next_actions(&depth_limit_nodes.bet_abstraction);

    let action_prob_sums: HashMap<Action, f64> = next_actions
        .iter()
        .enumerate()
        .map(|(action_index, action)| {
            let mut prob_sum: f64 = 0.0;
            for i in 0..N {
                let action_prob = *strategies[i].get(action_index).unwrap();
                let reach_prob = if history.player == traverser {
                    traverser_reach_probs[i]
                } else {
                    opp_reach_probs[i]
                };
                prob_sum += action_prob * reach_prob
            }
            (action.clone(), prob_sum)
        })
        .collect();

    let sum: f64 = action_prob_sums.values().sum();
    if sum <= 0.0 {
        return vec![0.0; N];
    }

    let next_actions = vec![next_actions
        .choose_weighted(&mut rand::thread_rng(), |a| {
            action_prob_sums.get(a).unwrap()
        })
        .unwrap()];

    let mut node_utility: Vec<f64> = vec![0.0; N];
    for (action_index, action) in next_actions.iter().enumerate() {
        let probs: Vec<f64> = strategies
            .iter()
            .map(|strategy| *strategy.get(action_index).unwrap())
            .collect();

        let mut next_history = translated_history.clone();
        next_history.add(&action);

        let mut traverser_reach_probs = traverser_reach_probs.clone();
        let mut opp_reach_probs = opp_reach_probs.clone();

        if history.player == traverser {
            for i in 0..N {
                traverser_reach_probs[i] *= probs[i];
            }
        } else {
            assert!(opp_reach_probs.len() == strategies.len());
            for i in 0..opp_reach_probs.len() {
                opp_reach_probs[i] *= probs[i];
            }
        }

        let utility = depth_limit_utility(
            traverser,
            preflop_hands.clone(),
            board,
            &next_history,
            traverser_reach_probs.clone(),
            opp_reach_probs.clone(),
            depth_limit_nodes,
            abstraction,
            precomputed_strengths,
        );

        for n in 0..node_utility.len() {
            let prob: f64 = if history.player == traverser {
                probs[n]
            } else {
                1.0
            };
            node_utility[n] += prob * utility[n];
        }
    }
    node_utility
}
