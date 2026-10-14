use crate::card_utils;
use crate::card_utils::*;
use crate::config::CONFIG;
use crate::exploiter::*;
use crate::nodes::*;
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
    writeln!(csv, "iteration,exploitability,method,seconds").unwrap();
    let num_epochs = iters / eval_every;
    let start = std::time::Instant::now();
    for epoch in 0..num_epochs {
        println!("[INFO] Training epoch {}/{}", epoch + 1, num_epochs);
        let bar = card_utils::pbar(eval_every as usize);
        let epoch_start = std::time::Instant::now();

        let base = epoch * eval_every;
        (0..eval_every).into_par_iter().for_each(|i| {
            // Global iteration number, starting at 1 (linear CFR weight).
            let t = (base + i + 1) as f64;
            cfr_iteration(&deck, &nodes, Some(t));
            bar.inc(1);
        });
        bar.finish_with_message("Done");
        println!(
            "[INFO] Epoch took {:.1}s ({:.1} ms/iteration)",
            epoch_start.elapsed().as_secs_f64(),
            epoch_start.elapsed().as_secs_f64() * 1000.0 / eval_every as f64
        );
        serialize_nodes(&nodes);
        let exploit = if CONFIG.exploit_method == "lbr" {
            blueprint_exploitability(&nodes, CONFIG.lbr_iters)
        } else {
            blueprint_best_response(&nodes, CONFIG.br_boards)
        };
        writeln!(
            csv,
            "{},{},{},{:.0}",
            (epoch + 1) * eval_every,
            exploit,
            CONFIG.exploit_method,
            start.elapsed().as_secs_f64()
        )
        .unwrap();
        csv.flush().unwrap();

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

/// One chance-sampled CFR iteration for each traverser. `weight` is the linear
/// CFR weight (the global iteration number); `None` uses per-node visit counts.
/// Samples a board, honoring `CONFIG.fixed_board` for its first cards.
pub fn sample_board(deck: &[Card]) -> [Card; 5] {
    let fixed = str2cards(&CONFIG.fixed_board);
    let mut deck: Vec<Card> = deck.iter().filter(|c| !fixed.contains(c)).cloned().collect();
    deck.shuffle(&mut rand::thread_rng());
    let mut board: Vec<Card> = fixed;
    board.extend_from_slice(&deck[..5 - board.len()]);
    [board[0], board[1], board[2], board[3], board[4]]
}

pub fn cfr_iteration(deck: &[Card], nodes: &Nodes, weight: Option<f64>) {
    [DEALER, OPPONENT].iter().for_each(|&traverser| {
        let board = sample_board(deck);
        let preflop_hands = non_blocking_preflop_hands(&board[..showdown_board_len()]);
        let strengths = precompute_showdown_strengths(&board);
        let street_buckets = precompute_street_buckets(&preflop_hands, &board, &*ABSTRACTION);

        let traverser_reach_probs = vec![1.0; preflop_hands.len()];
        let opp_reach_probs = vec![1.0; preflop_hands.len()];

        iterate(
            traverser,
            &preflop_hands,
            board,
            &ActionHistory::new(),
            traverser_reach_probs,
            opp_reach_probs,
            nodes,
            -1,
            None,
            &*ABSTRACTION,
            Some(&strengths),
            None,
            weight,
            Some(&street_buckets),
        );
    });
}

/// Exact best response against the blueprint's average strategy, restricted to
/// the abstract action set, with the best-response player seeing exact cards.
/// Returns the mean best-response value in big blinds per hand, averaged over
/// both seats and `n_boards` sampled boards. This is a true exploitability
/// measure of the average strategy within the action abstraction (sampling noise
/// aside), unlike the greedy LBR.
pub fn blueprint_best_response(nodes: &Nodes, n_boards: usize) -> f64 {
    println!("[INFO] Calculating exact best response over {} boards...", n_boards);
    let bar = card_utils::pbar(n_boards);
    let deck = card_utils::deck();
    let values: Vec<f64> = (0..n_boards)
        .into_par_iter()
        .map(|_| {
            let board = sample_board(&deck);
            let hands = non_blocking_preflop_hands(&board[..showdown_board_len()]);
            let strengths = precompute_showdown_strengths(&board);
            let street_buckets = precompute_street_buckets(&hands, &board, &*ABSTRACTION);
            let n = hands.len();
            let valid_pairs = {
                let remaining = 52 - showdown_board_len() - 2;
                (remaining * (remaining - 1) / 2) as f64
            };
            let mut total = 0.0;
            for br_player in [DEALER, OPPONENT] {
                let values = best_response_iterate(
                    br_player,
                    &hands,
                    board,
                    &ActionHistory::new(),
                    vec![1.0; n],
                    nodes,
                    &street_buckets,
                    &strengths,
                );
                // terminal utilities are divided by n; sum over hands then
                // normalize by the number of valid (hand, opponent hand) pairs.
                total += values.iter().sum::<f64>() * n as f64 / valid_pairs / n as f64;
            }
            bar.inc(1);
            total / 2.0 / CONFIG.big_blind as f64
        })
        .collect();
    bar.finish();
    let mean = statistical::mean(&values);
    let std = statistical::standard_deviation(&values, Some(mean));
    let confidence = 1.96 * std / (n_boards as f64).sqrt();
    println!("Best response: {mean:.4} +/- {confidence:.4} BB/hand\n");
    mean
}

fn best_response_iterate(
    br_player: usize,
    hands: &[[Card; 2]],
    board: [Card; 5],
    history: &ActionHistory,
    opp_reach_probs: Vec<f64>,
    nodes: &Nodes,
    street_buckets: &StreetBuckets,
    strengths: &HashMap<[Card; 2], i32>,
) -> Vec<f64> {
    let n = hands.len();
    if history.hand_over() {
        return street_buckets.eval.utility(&opp_reach_probs, history, br_player);
    }
    let actions = history.next_actions(&nodes.bet_abstraction);
    if history.player == br_player {
        let mut best = vec![f64::NEG_INFINITY; n];
        for action in actions.iter() {
            let mut next = history.clone();
            next.add(action);
            let u = best_response_iterate(
                br_player, hands, board, &next, opp_reach_probs.clone(), nodes, street_buckets, strengths,
            );
            for h in 0..n {
                if u[h] > best[h] {
                    best[h] = u[h];
                }
            }
        }
        best
    } else {
        let strategies =
            nodes.try_get_strategy_buckets(history, &street_buckets.buckets[history.street], actions.len());
        let mut value = vec![0.0; n];
        for (ai, action) in actions.iter().enumerate() {
            let next_reach: Vec<f64> = (0..n)
                .map(|h| opp_reach_probs[h] * strategies[h].get(ai).copied().unwrap_or(0.0))
                .collect();
            if next_reach.iter().all(|&p| p <= 0.0) {
                continue;
            }
            let mut next = history.clone();
            next.add(action);
            let u = best_response_iterate(
                br_player, hands, board, &next, next_reach, nodes, street_buckets, strengths,
            );
            for h in 0..n {
                value[h] += u[h];
            }
        }
        value
    }
}

/// Everything about one sampled board that every node of a traversal shares:
/// the card bucket of each hand on each street (`buckets[street][hand]`) and the
/// showdown evaluator.
pub struct BoardCache {
    pub buckets: [Vec<i32>; 4],
    pub eval: BoardEval,
}

pub type StreetBuckets = BoardCache;

pub fn precompute_street_buckets(
    hands: &[[Card; 2]],
    board: &[Card; 5],
    abstraction: &dyn CardAbstraction,
) -> BoardCache {
    let last = showdown_board_len();
    let mut buckets: [Vec<i32>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for street in PREFLOP..=RIVER {
        let n = board_length(street);
        if n > last {
            break;
        }
        buckets[street] = hands
            .iter()
            .map(|h| {
                let hand: SmallVecHand = h.iter().chain(board[..n].iter()).cloned().collect();
                abstraction.bin(&hand)
            })
            .collect();
    }
    BoardCache {
        buckets,
        eval: BoardEval::new(hands, board),
    }
}

pub type DepthLimitFn<'a> = dyn Fn(usize, Vec<[Card; 2]>, [Card; 5], &ActionHistory, Vec<f64>, Vec<f64>) -> Vec<f64> + 'a;

pub fn iterate<'a>(
    traverser: usize,
    preflop_hands: &[[Card; 2]],
    board: [Card; 5],
    history: &ActionHistory,
    traverser_reach_probs: Vec<f64>,
    opp_reach_probs: Vec<f64>,
    nodes: &Nodes,
    depth_limit: i32,
    depth_limit_nodes: Option<&Nodes>,
    abstraction: &dyn CardAbstraction,
    precomputed_strengths: Option<&HashMap<[Card; 2], i32>>,
    depth_limit_fn: Option<&'a DepthLimitFn<'a>>,
    weight: Option<f64>,
    street_buckets: Option<&StreetBuckets>,
) -> Vec<f64> {
    let N = preflop_hands.len();
    if N == 0 {
        return Vec::new();
    }

    if history.hand_over() {
        if let Some(bc) = street_buckets {
            return bc.eval.utility(&opp_reach_probs, history, traverser);
        }
        return terminal_utility_vectorized(
            preflop_hands.to_vec(),
            opp_reach_probs,
            &board,
            history,
            traverser,
            precomputed_strengths,
        );
    }

    // Depth-limited solving: at start of river (after turn subgame), delegate to leaf evaluator
    if let Some(leaf_fn) = depth_limit_fn {
        if history.street >= RIVER && history.current_street_length == 0 {
            return leaf_fn(traverser, preflop_hands.to_vec(), board, history, traverser_reach_probs, opp_reach_probs);
        }
    }

    // Look up the DCFR node for this information set, or make a new one if it
    // doesn't exist
    let history = history.clone();
    let buckets: Vec<i32> = match street_buckets {
        Some(sb) => sb.buckets[history.street].clone(),
        None => preflop_hands
            .iter()
            .map(|h| InfoSet::from_hand_with_abstraction(h, &board, &history, abstraction).card_bucket)
            .collect(),
    };

    let strategies: Vec<SmallVecFloats> = nodes.get_current_strategy_buckets(&history, &buckets);
    let opponent = 1 - traverser;
    if history.player == traverser {
        nodes.update_strategy_sum_buckets(&history, &buckets, &traverser_reach_probs, weight);
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
                preflop_hands,
                board,
                &next_history,
                traverser_reach_probs,
                opp_reach_probs,
                nodes,
                depth_limit - 1,
                depth_limit_nodes,
                abstraction,
                precomputed_strengths,
                depth_limit_fn,
                weight,
                street_buckets,
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
            nodes.add_regret_buckets(&history, &buckets, action_utility, &node_utility, action_idx, weight);
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
