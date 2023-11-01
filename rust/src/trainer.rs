use crate::bot::Bot;
use crate::card_utils;
use crate::card_utils::*;
use crate::config::CONFIG;
use crate::exploiter::*;
use crate::nodes::*;
use crate::ranges::Range;
use crate::trainer_utils::*;
use rand::prelude::*;
use rayon::prelude::*;
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

        // Im trying without parallelization in order to see if thats causing the issue. but idk.
        (0..eval_every).into_par_iter().for_each(|_| {
            cfr_iteration(&deck, &ActionHistory::new(), &nodes, -1);
            bar.inc(1);
        });
        bar.finish_with_message("Done");
        serialize_nodes(&nodes);
        blueprint_exploitability(&nodes, CONFIG.lbr_iters);

        let infoset = InfoSet::from_hand(&str2cards("7c2d"), &Vec::new(), &ActionHistory::new());
        println!("InfoSet: {infoset}");
        println!(
            "Actions: {:?}",
            infoset.next_actions(&CONFIG.bet_abstraction)
        );
        println!("Node: {:?}", nodes.get(&infoset));

        // Check what percent of nodes have t = 0
        let mut zero = 0;
        let mut total = 0;
        for reference in &nodes.dashmap {
            let history_nodes = reference.lock().unwrap();
            for n in history_nodes.iter() {
                total += 1;
                if n.t == 0 {
                    zero += 1;
                }
            }
        }
        println!("Fraction zeros: {}", zero as f64 / total as f64);

        // let infoset = InfoSet::from_hand(&str2cards("AcAd"), &Vec::new(), &ActionHistory::new());
        // println!("InfoSet: {infoset}");
        // println!(
        //     "Actions: {:?}",
        //     infoset.next_actions(&CONFIG.bet_abstraction)
        // );
        // println!("Node: {:?}", nodes.get(&infoset));
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
        let mut opp_hands: Vec<[Card; 2]> = Vec::with_capacity(30);
        for i in (5..31).step_by(2) {
            opp_hands.push([deck[i], deck[i + 1]]);
        }

        // let player_hands: Vec<[Card; 2]> = vec![[deck[7], deck[8]], [deck[9], deck[10]]];
        // let mut player_hands: Vec<[Card; 2]> = Vec::with_capacity(30);
        // for i in (7..51).step_by(2) {
        //     player_hands.push([deck[i], deck[i + 1]]);
        // }

        let mut range = Range::new();
        range.remove_blockers(&opp_hands[0]);
        range.remove_blockers(&opp_hands[1]);
        range.remove_blockers(&board);
        let mut player_hands = Vec::with_capacity(range.hands.len());
        for hand_index in 0..range.hands.len() {
            let prob = range.probs[hand_index];
            if prob > 0.0 {
                player_hands.push(range.hands[hand_index]);
            }
        }

        let traverser_reach_probs = vec![1.0; player_hands.len()];
        let opp_reach_probs = vec![1.0; opp_hands.len()];

        iterate(
            traverser,
            player_hands,
            opp_hands,
            board,
            &ActionHistory::new(),
            traverser_reach_probs,
            opp_reach_probs,
            nodes,
            None,
            None,
            -1,
        );
    });
}

pub fn iterate(
    traverser: usize,
    traverser_preflop_hands: Vec<[Card; 2]>,
    opp_preflop_hands: Vec<[Card; 2]>,
    board: [Card; 5],
    history: &ActionHistory,
    traverser_reach_probs: Vec<f64>,
    opp_reach_probs: Vec<f64>,
    nodes: &Nodes,
    depth_limit_bot: Option<&Bot>,
    bot_position: Option<usize>,
    remaining_depth: i32,
) -> Vec<f64> {
    debug_assert!(board.len() == 5);
    debug_assert!(opp_preflop_hands.len() == opp_reach_probs.len());
    let N = traverser_preflop_hands.len();
    if history.hand_over() {
        let utils: Vec<f64> = traverser_preflop_hands
            .iter()
            .map(|h| {
                let mut total_util = 0.0;
                // for opp_hand in opp_preflop_hands.clone() {
                for i in 0..opp_preflop_hands.len() {
                    let opp_hand = opp_preflop_hands[i];
                    let opp_prob = opp_reach_probs[i];
                    total_util +=
                        opp_prob * terminal_utility(h, &opp_hand, &board, history, traverser);
                }
                total_util / opp_preflop_hands.len() as f64
            })
            .collect();
        return utils;
    }
    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let history = history.clone();
    let infosets: Vec<InfoSet> = if history.player == traverser {
        traverser_preflop_hands
            .iter()
            .map(|h| InfoSet::from_hand(h, &board, &history))
            .collect()
    } else {
        opp_preflop_hands
            .iter()
            .map(|h| InfoSet::from_hand(h, &board, &history))
            .collect()
    };

    let strategies: Vec<SmallVecFloats> = infosets
        .iter()
        .map(|i| nodes.get_current_strategy(&i))
        .collect();
    let opponent = 1 - traverser;
    if history.player == traverser {
        for i in 0..N {
            nodes.update_strategy_sum(&infosets[i], traverser_reach_probs[i] as f32);
        }
    }

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

            if traverser_reach_probs.iter().all(|&x| x < 1e-10)
                && opp_reach_probs.iter().all(|&x| x < 1e-10)
            {
                return vec![0.0; N];
            }

            let utility: Vec<f64> = iterate(
                traverser,
                traverser_preflop_hands.clone(),
                opp_preflop_hands.clone(),
                board,
                &next_history,
                traverser_reach_probs,
                opp_reach_probs,
                nodes,
                depth_limit_bot,
                bot_position,
                remaining_depth - 1,
            );

            for n in 0..node_utility.len() {
                let prob: f32 = if history.player == traverser {
                    probs[n]
                } else {
                    // The opponent's probably of taking this action is the average of probabilities
                    // across the possible hands they could have. (weighting by reach prob?) hmm. no?
                    // let sum: f32 = probs.iter().sum();
                    // sum / probs.len() as f32
                    1.0
                };
                // prob[n] should sum to 1 over all player actions
                node_utility[n] += prob as f64 * utility[n];
            }
            utility
        })
        .collect();

    let opp_reach_probs_sum: f64 = opp_reach_probs.iter().sum();
    // Might be some weird detail here with blockers
    let avg_opp_reach_prob: f64 = opp_reach_probs_sum / N as f64;

    // Update regrets for the traversing player
    if history.player == traverser {
        // Action utilities is shape [actions, traverser_hands]
        for (action_idx, action_utility) in action_utilities.iter().enumerate() {
            for (hand_idx, utility) in action_utility.iter().enumerate() {
                let regret = utility - node_utility[hand_idx];
                nodes.add_regret(&infosets[hand_idx], action_idx, regret);
            }
        }
    }
    node_utility
}

// Implements Depth Limited Solving - at the depth limit, instead of choosing an action to play, the
// opponent chooses a strategy to play until the end of the hand. This gives a good estimate of the
// utility at the depth limit nodes because it allows the opponent to respond to the player's strategy.
// https://arxiv.org/pdf/1805.08195.pdf
// fn depth_limit_utility(
//     player: usize,
//     deck: &[Card],
//     history: &ActionHistory,
//     weights: [f64; 2],
//     nodes: &Nodes,
//     depth_limit_bot: &Bot,
// ) -> f64 {
//     let opponent = 1 - player;
//     // The history.player is the bot's opponent
//     let bot_opponent = 1 - history.player;
//     debug_assert!(false);
//     let infoset = InfoSet::from_deck(deck, history);
//     let strategy = nodes.get_current_strategy(&infoset);
//     let mut strategy_biases: Vec<Option<Action>> = infoset
//         .next_actions(&nodes.bet_abstraction)
//         .iter()
//         .map(|s| Some(s.clone()))
//         .collect();
//     strategy_biases.push(None);
//     let utilities: Vec<f64> = strategy_biases
//         .iter()
//         .map(|bias| {
//             // Play until the end of the game with the opponent using their biased strategy, or the
//             // blueprint strategy. The terminal utility will update the regret for the depth limit
//             // opponent node.
//             let mut history_past_depth = history.clone();
//             loop {
//                 let hand = get_hand(deck, history_past_depth.player, history_past_depth.street);
//                 let hole = &hand[..2];
//                 let board = &hand[2..];
//                 let mut node_strategy = depth_limit_bot.get_strategy_action_translation(
//                     hole,
//                     board,
//                     &history_past_depth,
//                 );
//                 // Bias the strategy by multiplying one of the actions by 10 and renormalizing. Otherwise
//                 // just play according to the blueprint strategy at the current infoset, if it's the bot's
//                 // turn, or if we're in the blueprint meta-strategy for the opponent.
//                 if history_past_depth.player == bot_opponent {
//                     if let Some(b) = bias {
//                         for (node_action, prob) in node_strategy.clone() {
//                             if node_action.action == b.action {
//                                 node_strategy.insert(node_action, prob * 10.0);
//                             }
//                         }
//                         node_strategy = normalize(&node_strategy);
//                     }
//                 }

//                 let action = sample_action_from_strategy(&node_strategy);
//                 history_past_depth.add(&action);
//                 if history_past_depth.hand_over() {
//                     return terminal_utility(deck, &history_past_depth, player);
//                 }
//             }
//         })
//         .collect();

//     let mut node_utility = 0.0;
//     for i in 0..strategy.len() {
//         node_utility += utilities[i] * strategy[i] as f64;
//     }

//     if history.player == player {
//         for (index, utility) in utilities.iter().enumerate() {
//             let regret = utility - node_utility;
//             nodes.add_regret(&infoset, index, weights[opponent] * regret);
//         }
//     }

//     node_utility
// }
