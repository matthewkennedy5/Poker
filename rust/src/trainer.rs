use crate::card_utils::*;
use crate::card_utils;
use crate::config::CONFIG;
use crate::exploiter::*;
use crate::nodes::*;
use crate::trainer_utils::*;
use crate::bot::Bot;
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
        Nodes::new(&CONFIG.bet_abstraction)
    };
    println!("[INFO] Beginning training.");
    let num_epochs = iters / eval_every;
    for epoch in 0..num_epochs {
        println!("[INFO] Training epoch {}/{}", epoch + 1, num_epochs);
        let bar = card_utils::pbar(eval_every);

        (0..eval_every).into_par_iter().for_each(|_| {
            cfr_iteration(
                &deck,
                &ActionHistory::new(),
                &nodes,
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
        for reference in &nodes.dashmap {
            let history_nodes = reference.lock().unwrap();
            for n in history_nodes.iter() {
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
            None,
            None,
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
    depth_limit_bot: Option<&Bot>,
    bot_position: Option<usize>,
    remaining_depth: i32,
) -> f64 {
    if history.hand_over() {
        return terminal_utility(deck, history, player);
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let mut history = history.clone();
    let mut infoset = InfoSet::from_deck(deck, &history);

    // Depth limited solving - just sample actions until the end of the game to estimate the utility
    // of this information set
    if remaining_depth == 0 {
        if let Some(depth_limit_bot) = depth_limit_bot {
            // TODO: Make sure history.player is the bot's opponent before going to the depth limit
            // return depth_limit_utility(
            //     player, 
            //     deck,
            //     &history,
            //     weights, 
            //     nodes,
            //     depth_limit_bot,
            //     // TODO REFACTOR: Make a wrapper function to avoid passing all these optionals all the time
            // );
            // println!("Estimating depth limit utility at infoset {infoset}");
            loop {
                let hand = get_hand(deck, history.player, history.street);
                let hole = &hand[..2];
                let board = &hand[2..];
                let strategy = depth_limit_bot.get_strategy_action_translation(hole, board, &history);
                // println!("Strategy: {:?}", strategy);
                let action = sample_action_from_strategy(&strategy);
                // println!("Hand: {} Board: {}", cards2str(hole), cards2str(board));
                // println!("Sampled action {action} from strategy {:?}", strategy);
                history.add(&action);
                if history.hand_over() {
                    let utility = terminal_utility(deck, &history, player);
                    // println!("Depth limit node utility for traversing player {player}: {utility}");
                    return utility;
                }
            }

        }
    }

    let mut weights = weights;
    let mut strategy = nodes.get_current_strategy(&infoset);
    let opponent = 1 - player;
    if history.player == player {
        nodes.update_strategy_sum(&infoset, weights[player]);
    } 

    let actions = infoset.next_actions(&nodes.bet_abstraction);
    let mut node_utility = 0.0;
    // Recurse to further nodes in the game tree. Find the utilities for each action.
    let utilities: SmallVec<[f64; NUM_ACTIONS]> = (0..actions.len())
        .map(|i| {
            let prob = strategy[i];

            let mut next_history = history.clone();
            next_history.add(&actions[i]);

            let new_weights = match history.player {
                0 => [weights[0] * prob, weights[1]],
                1 => [weights[0], weights[1] * prob],
                _ => panic!("Bad player value"),
            };

            if weights[0] < 1e-10 && weights[1] < 1e-10 {
                return 0.0;
            }

            let utility = iterate(
                player,
                deck,
                &next_history,
                new_weights,
                nodes,
                depth_limit_bot,
                bot_position,
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
            nodes.add_regret(&infoset, index, weights[opponent] * regret);
        }
    }
    node_utility
}

// Implements Depth Limited Solving - at the depth limit, instead of choosing an action to play, the
// opponent chooses a strategy to play until the end of the hand. This gives a good estimate of the
// utility at the depth limit nodes because it allows the opponent to respond to the player's strategy. 
// https://arxiv.org/pdf/1805.08195.pdf
fn depth_limit_utility(
    player: usize,
    deck: &[Card], 
    history: &ActionHistory, 
    weights: [f64; 2], 
    nodes: &Nodes, 
    depth_limit_bot: &Bot
) -> f64 {
    let opponent = 1 - player;
    // The history.player is the bot's opponent
    let bot_opponent = 1 - history.player;
    debug_assert!(false);
    let infoset = InfoSet::from_deck(deck, history);
    let strategy = nodes.get_current_strategy(&infoset);
    let mut strategy_biases: Vec<Option<Action>> = infoset.next_actions(&nodes.bet_abstraction)
        .iter().map(|s| Some(s.clone())).collect();
    strategy_biases.push(None);
    let utilities: Vec<f64> = strategy_biases.iter().map(|bias| {
        // Play until the end of the game with the opponent using their biased strategy, or the 
        // blueprint strategy. The terminal utility will update the regret for the depth limit 
        // opponent node. 
        let mut history_past_depth = history.clone();
        loop {
            let hand = get_hand(deck, history_past_depth.player, history_past_depth.street);
            let hole = &hand[..2];
            let board = &hand[2..];
            let mut node_strategy = depth_limit_bot.get_strategy_action_translation(hole, board, &history_past_depth);
            // Bias the strategy by multiplying one of the actions by 10 and renormalizing. Otherwise
            // just play according to the blueprint strategy at the current infoset, if it's the bot's
            // turn, or if we're in the blueprint meta-strategy for the opponent. 
            if history_past_depth.player == bot_opponent{
                if let Some(b) = bias {
                    for (node_action, prob) in node_strategy.clone() {
                        if node_action.action == b.action {
                            node_strategy.insert(node_action, prob * 10.0);
                        }
                    }
                    node_strategy = normalize(&node_strategy);
                }
            }

            let action = sample_action_from_strategy(&node_strategy);
            history_past_depth.add(&action);
            if history_past_depth.hand_over() {
                return terminal_utility(deck, &history_past_depth, player);
            }
        };
    }).collect();
    
    let mut node_utility = 0.0;
    for i in 0..strategy.len() {
        node_utility += utilities[i] * strategy[i];
    }

    if history.player == player {
        for (index, utility) in utilities.iter().enumerate() {
            let regret = utility - node_utility;
            nodes.add_regret(&infoset, index, weights[opponent] * regret);
        }
    }

    node_utility
}
