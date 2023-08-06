use crate::bot::Bot;
use crate::card_utils;
use crate::card_utils::Card;
use crate::config::CONFIG;
use crate::exploiter::*;
use crate::nodes::*;
use crate::ranges::*;
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
        Nodes::new(&CONFIG.bet_abstraction)
    };
    let dummy_depth_limit_hack = Bot::new(Nodes::new(&CONFIG.bet_abstraction), false, false, -1); // TODO REFACTOR
    println!("[INFO] Beginning training.");
    let num_epochs = iters / eval_every;
    for epoch in 0..num_epochs {
        println!("[INFO] Training epoch {}/{}", epoch + 1, num_epochs);
        let bar = card_utils::pbar(eval_every);

        (0..eval_every).into_iter().for_each(|_| {
            cfr_iteration(
                &deck,
                &ActionHistory::new(),
                &nodes,
                &dummy_depth_limit_hack,
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
    depth_limit_bot: &Bot,
    depth_limit: i32,
) {
    [DEALER, OPPONENT].iter().for_each(|&player| {
        let mut deck = deck.to_vec();
        deck.shuffle(&mut rand::thread_rng());
        let mut range = Range::new();
        range.remove_blockers(&deck[..9]);
        let opponent_hands: Vec<Vec<Card>> = (0..1000).map(|_| range.sample_hand()).collect();
        debug_assert!(
            {
                let player_hand = get_hand(&deck, player, RIVER);
                let mut duplicates = false;
                for opp_hand in opponent_hands.clone() {
                    if player_hand.contains(&opp_hand[0]) || player_hand.contains(&opp_hand[1]) {
                        duplicates = true;
                    }
                }
                !duplicates
            },
            "Duplicates between opponent hand and other cards"
        );

        iterate(
            player,
            &deck,
            &opponent_hands,
            history,
            [1.0, 1.0],
            &nodes,
            depth_limit_bot,
            depth_limit,
        );
    });
}

pub fn iterate(
    player: usize,
    deck: &[Card],
    opponent_hands: &[Vec<Card>],
    history: &ActionHistory,
    weights: [f64; 2],
    nodes: &Nodes,
    depth_limit_bot: &Bot,
    remaining_depth: i32,
) -> f64 {
    let opponent_hands: Vec<Vec<Card>> = opponent_hands.to_vec();

    if history.hand_over() {
        // println!("History: {history}");
        // println!(
        //     "Player hand: {}",
        //     card_utils::cards2str(&get_hand(deck, player, RIVER))
        // );
        let opp_hand_utilities: Vec<f64> = opponent_hands
            .iter()
            .map(|opp_hand| {
                let mut opp_deck = Vec::with_capacity(52);
                if player == DEALER {
                    opp_deck.extend(&deck[0..2]);
                    opp_deck.extend(opp_hand);
                    opp_deck.extend(&deck[4..]);
                } else {
                    opp_deck.extend(opp_hand);
                    opp_deck.extend(&deck[2..]);
                }
                let u = terminal_utility(&opp_deck, history, player);
                if history.last_action().unwrap() != FOLD {
                    // println!("opp_deck: {}", card_utils::cards2str(&opp_deck));
                    // println!(
                    //     "Player hand {} beats opponent hand {} by {}",
                    //     card_utils::cards2str(&get_hand(&opp_deck, player, RIVER)),
                    //     card_utils::cards2str(&get_hand(&opp_deck, 1 - player, RIVER)),
                    //     u
                    // );
                }
                u
            })
            .collect();
        let utility_sum: f64 = opp_hand_utilities.iter().sum();
        let avg_utility = utility_sum / opp_hand_utilities.len() as f64;
        // println!("Average winnings across all opponent hands: {avg_utility}\n");
        return avg_utility;
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let history = history.clone();
    // let infoset = InfoSet::from_deck(deck, &history);

    // Depth limited solving - just sample actions until the end of the game to estimate the utility
    // of this information set
    // TODO: Let the opponent choose between several strategies
    // if remaining_depth == 0 {
    //     loop {
    //         let hand = get_hand(deck, player, history.street);
    //         let hole = &hand[..2];
    //         let board = &hand[2..];
    //         let strategy = depth_limit_bot.get_strategy_action_translation(hole, board, &history);
    //         let action = sample_action_from_strategy(&strategy);
    //         history.add(&action);
    //         if history.hand_over() {
    //             return terminal_utility(deck, &history, player);
    //         }
    //     }
    // }

    let infoset = InfoSet::from_deck(deck, &history);
    let strategy = if history.player == player {
        nodes.get_current_strategy(&infoset)
    } else {
        let mut avg_strategy = smallvec![0.0; NUM_ACTIONS];
        for opp_hand in opponent_hands.clone() {
            let opp_hand_infoset = InfoSet::from_hand(&opp_hand, &deck[4..], &history);
            let opp_hand_strategy = nodes.get_current_strategy(&opp_hand_infoset);
            // println!("Opponent hand {} has strategy {:?}", card_utils::cards2str(&opp_hand), opp_hand_strategy);
            for i in 0..opp_hand_strategy.len() {
                avg_strategy[i] += opp_hand_strategy[i] / opponent_hands.len() as f64;
            }
        }
        // println!("Avg opponent strategy: {:?}", avg_strategy);
        avg_strategy
    };

    if history.player == player {
        nodes.update_strategy_sum(&infoset, weights[player]);
    }

    let opponent = 1 - player;
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
                &opponent_hands,
                &next_history,
                new_weights,
                nodes,
                depth_limit_bot,
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
