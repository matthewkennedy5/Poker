use crate::bot::Bot;
use crate::card_utils;
use crate::card_utils::*;
use crate::config::CONFIG;
use crate::exploiter::*;
use crate::nodes::*;
use crate::trainer_utils::*;
use rand::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

pub fn train(iters: u64, eval_every: u64, warm_start: bool) {
    let deck = card_utils::deck();
    let nodes = if warm_start {
        load_nodes(&CONFIG.nodes_path)
    } else {
        Nodes::new(&CONFIG.bet_abstraction)
    };
    println!("[INFO] Beginning training.");
    let num_epochs = iters / eval_every;
    for epoch in 0..num_epochs {
        println!("[INFO] Training epoch {}/{}", epoch + 1, num_epochs);
        let bar = card_utils::pbar(eval_every);

        (0..eval_every).into_par_iter().for_each(|_| {
            cfr_iteration(&deck, &ActionHistory::new(), &nodes, -1);
            bar.inc(1);
        });

        bar.finish_with_message("Done");
        serialize_nodes(&nodes);
        blueprint_exploitability(&nodes, CONFIG.lbr_iters);

        let infoset = InfoSet::from_hand(
            &str2cards("6h6d"),
            &str2cards("2s3dAc6c2h"),
            &ActionHistory::from_strings(vec![
                "Bet 300", "Call 300", "Call 0", "Call 0", "Call 0", "Call 0", "Call 0",
            ]),
        );

        let infoset = InfoSet::from_hand(&str2cards("2c7h"), &Vec::new(), &ActionHistory::new());
        println!("InfoSet: {infoset}");
        println!(
            "Actions: {:?}",
            infoset.next_actions(&CONFIG.bet_abstraction)
        );
        println!("Node: {:?}", nodes.get(&infoset));
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
        let board: Vec<Card> = deck
            .choose_multiple(&mut rand::thread_rng(), 5)
            .cloned()
            .collect();
        let board: [Card; 5] = board.try_into().unwrap();
        let preflop_hands = non_blocking_preflop_hands(&board);
        let N = preflop_hands.len();

        iterate(
            traverser,
            preflop_hands,
            board,
            &ActionHistory::new(),
            vec![1.0; N],
            vec![1.0; N],
            nodes,
            i32::MAX,
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
    depth: i32,
    depth_limit_bot: Option<&Bot>,
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
        );
    }

    // if depth < 0 && history.current_street_length == 0 {
    //     // depth limited solving for future streets
    //     return depth_limit_utility(
    //         traverser,
    //         preflop_hands,
    //         board,
    //         history,
    //         traverser_reach_probs,
    //         opp_reach_probs,
    //         depth_limit_bot.expect("Depth limit bot not provided for depth limit utility"),
    //     );
    // }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let history = history.clone();
    let infosets: Vec<InfoSet> = preflop_hands
        .iter()
        .map(|h| InfoSet::from_hand(h, &board, &history))
        .collect();

    let strategies: Vec<SmallVecFloats> = nodes.get_current_strategy_vectorized(&infosets);
    let opponent = 1 - traverser;

    let actions = history.next_actions(&nodes.bet_abstraction);
    let mut node_utility: Vec<f64> = vec![0.0; N];
    // Recurse to further nodes in the game tree. Find the utilities for each action.
    let action_utilities: Vec<Vec<f64>> = (0..actions.len())
        .map(|i| -> Vec<f64> {
            // Maps traverser_preflop_hand to prob of taking this action
            let probs: Vec<f32> = strategies.iter().map(|s| s[i]).collect();
            let mut next_history = history.clone();
            next_history.add(&actions[i]);

            let mut traverser_reach_probs = traverser_reach_probs.clone();
            let mut opp_reach_probs = opp_reach_probs.clone();

            if history.player == traverser {
                for i in 0..N {
                    traverser_reach_probs[i] *= probs[i] as f64;
                }
            } else {
                assert!(opp_reach_probs.len() == strategies.len());
                for i in 0..opp_reach_probs.len() {
                    opp_reach_probs[i] *= probs[i] as f64;
                }
            }

            let utility: Vec<f64> = iterate(
                traverser,
                preflop_hands.clone(),
                board,
                &next_history,
                traverser_reach_probs,
                opp_reach_probs,
                nodes,
                depth - 1,
                depth_limit_bot,
            );

            for n in 0..node_utility.len() {
                let prob: f32 = if history.player == traverser {
                    probs[n]
                } else {
                    1.0
                };
                node_utility[n] += prob as f64 * utility[n];
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
        // Theory for what's going wrong with pruning: With pruning, node.t is incremented a lot less,
        // so the strategy takes too long to pass t > 100 etc.
        nodes.update_strategy_sum_vectorized(&infosets, &traverser_reach_probs);
    }
    node_utility
}

// For now, this samples rollouts at the depth limit to estimate the utility.
// https://arxiv.org/pdf/1805.08195.pdf
fn depth_limit_utility(
    traverser: usize,
    preflop_hands: Vec<[Card; 2]>,
    board: [Card; 5],
    history: &ActionHistory,
    traverser_reach_probs: Vec<f64>,
    opp_reach_probs: Vec<f64>,
    depth_limit_bot: &Bot,
) -> Vec<f64> {
    if history.hand_over() {
        return terminal_utility_vectorized(
            preflop_hands,
            opp_reach_probs,
            &board,
            history,
            traverser,
        );
    }
    // Basically the same as iterate(), except:
    // - get the strategies / prob updates from the depth_limit_bot
    // - don't update any nodes
    let strategies: Vec<Strategy> = preflop_hands
        .iter()
        .map(|preflop_hand| {
            depth_limit_bot.get_strategy_action_translation(preflop_hand, &board, history)
        })
        .collect();

    // Sample a random action depending on the total probability for each action
    let N = preflop_hands.len();
    let next_actions = history.next_actions(&CONFIG.bet_abstraction);

    let action_prob_sums: HashMap<Action, f64> = next_actions
        .iter()
        .map(|action| {
            let mut prob_sum: f64 = 0.0;
            for i in 0..N {
                let action_prob = strategies[i].get(action).unwrap().clone();
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
    for action in next_actions {
        let probs: Vec<f64> = strategies
            .iter()
            .map(|strategy| strategy.get(&action).unwrap().clone())
            .collect();

        let mut next_history = history.clone();
        next_history.add(&action);

        let mut traverser_reach_probs = traverser_reach_probs.clone();
        let mut opp_reach_probs = opp_reach_probs.clone();

        if history.player == traverser {
            for i in 0..N {
                traverser_reach_probs[i] *= probs[i] as f64;
            }
        } else {
            assert!(opp_reach_probs.len() == strategies.len());
            for i in 0..opp_reach_probs.len() {
                opp_reach_probs[i] *= probs[i] as f64;
            }
        }

        if traverser_reach_probs.iter().all(|&x| x < 1e-10)
            && opp_reach_probs.iter().all(|&x| x < 1e-10)
        {
            return vec![0.0; N];
        }

        let utility = depth_limit_utility(
            traverser,
            preflop_hands.clone(),
            board,
            &next_history,
            traverser_reach_probs,
            opp_reach_probs,
            depth_limit_bot,
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
