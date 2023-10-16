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

        let infoset = InfoSet::from_hand(&str2cards("AcAd"), &Vec::new(), &ActionHistory::new());
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
    [DEALER, OPPONENT].iter().for_each(|&player| {
        let mut deck = deck.to_vec();
        deck.shuffle(&mut rand::thread_rng());
        let opp_hand = &deck[..2];
        let board = &deck[2..7];

        // TODO: Figure out a better way of getting rid of the impossible blocked preflop hands
        let mut range = Range::new();
        range.remove_blockers(opp_hand);
        range.remove_blockers(board);
        let mut preflop_hands = Vec::with_capacity(range.hands.len());
        for hand_index in 0..range.hands.len() {
            let prob = range.probs[hand_index];
            if prob > 0.0 {
                preflop_hands.push(range.hands[hand_index]);
            }
        }

        let player_hand_probs = vec![1.0; preflop_hands.len()];
        iterate(
            player,
            &preflop_hands,
            player_hand_probs,
            opp_hand,
            1.0,
            board,
            history,
            &nodes,
            None,
            None,
            depth_limit,
        );
    });
}

pub fn iterate(
    traverser: usize,
    preflop_hands: &[[Card; 2]],
    traverser_hand_probs: Vec<f64>,
    opp_hand: &[Card],
    opp_hand_prob: f64,
    board: &[Card],
    history: &ActionHistory,
    nodes: &Nodes,
    depth_limit_bot: Option<&Bot>,
    bot_position: Option<usize>,
    remaining_depth: i32,
) -> Vec<f64> {
    if history.hand_over() {
        // TODO: You could also vectorize terminal_utility itself if that's faster
        // let mut terminal_utils = [0.0; 1326];
        // for i in 0..preflop_hands.len() {
        //     terminal_utils[i] =
        //         terminal_utility(&preflop_hands[i], opp_hand, board, history, traverser);
        // }
        // return terminal_utils;

        let terminal_utils: Vec<f64> = preflop_hands
            .iter()
            .map(|&h| {
                let u = terminal_utility(&h, opp_hand, board, history, traverser);
                // println!(
                //     "Hand: {}, Opp Hand: {}, Board: {}, History: {}, Traverser: {}",
                //     cards2str(&h),
                //     cards2str(opp_hand),
                //     cards2str(board),
                //     history,
                //     traverser
                // );
                // println!("Terminal Utility: {u}");
                u
            })
            .collect();
        return terminal_utils;
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let mut infosets: Vec<InfoSet> = Vec::with_capacity(preflop_hands.len());
    let mut strategies: Vec<SmallVecFloats> = Vec::with_capacity(preflop_hands.len());
    for i in 0..preflop_hands.len() {
        let player_hand = preflop_hands[i];
        let infoset = InfoSet::from_hand(
            &player_hand,
            &board[..board_length(history.street)],
            history,
        );
        let strategy = nodes.get_current_strategy(&infoset);
        infosets.push(infoset.clone());
        strategies.push(strategy);
        if history.player == traverser {
            nodes.update_strategy_sum(&infoset, traverser_hand_probs[i] as f32);
            // TODO: maybe rename player_hand_probs to player_reach_probs or something
        }
    }

    // For opponent chance sampling, we only have infoset and strategy if its the opponents turn.
    // This is ugly but will go away when we vectorize the opponent.
    let opponent = 1 - traverser;
    // let mut infoset: Option<InfoSet> = None;
    let mut strategy: Option<SmallVecFloats> = None;
    if history.player == opponent {
        let infoset_val =
            InfoSet::from_hand(&opp_hand, &board[..board_length(history.street)], history);
        // infoset = Some(infoset_val.clone());
        strategy = Some(nodes.get_current_strategy(&infoset_val));
    }

    let actions = history.next_actions(&nodes.bet_abstraction);
    // let mut node_utility = 0.0;
    let mut node_utilities: Vec<f64> = vec![0.0; preflop_hands.len()];
    // Recurse to further nodes in the game tree. Find the utilities for each action.
    // TODO: Maybe just use arrays everywhere instead of SmallVec. I think you often know the size of things
    let utilities: Vec<Vec<f64>> = (0..actions.len())
        .map(|i| {
            let mut next_history = history.clone();
            next_history.add(&actions[i]);

            // Update the player reach probabilities based on the probability of this action for each
            // possible player hand
            let mut new_player_probs = traverser_hand_probs.clone();
            let mut new_opp_hand_prob = opp_hand_prob.clone();
            if history.player == traverser {
                for hand_index in 0..preflop_hands.len() {
                    new_player_probs[hand_index] *= strategies[hand_index][i] as f64;
                }
            } else {
                new_opp_hand_prob *= strategy.clone().unwrap()[i] as f64;
            }

            // Not sure how to handle pruning for this. Not sure you can in the same way
            // if weights[0] < 1e-10 && weights[1] < 1e-10 {
            //     return 0.0;
            // }

            let u = iterate(
                traverser,
                preflop_hands,
                new_player_probs,
                opp_hand,
                new_opp_hand_prob,
                board,
                &next_history,
                nodes,
                depth_limit_bot,
                bot_position,
                remaining_depth - 1,
            );
            for hand_index in 0..preflop_hands.len() {
                node_utilities[hand_index] +=
                    strategies[hand_index][i] as f64 * u[hand_index] as f64;
            }

            u
        })
        .collect();

    // Update regrets for the traversing player
    if history.player == traverser {
        for (action_index, u) in utilities.iter().enumerate() {
            for hand_index in 0..preflop_hands.len() {
                let regret = u[hand_index] - node_utilities[hand_index];
                nodes.add_regret(&infosets[hand_index], action_index, opp_hand_prob * regret);
            }
        }
    }
    node_utilities
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
