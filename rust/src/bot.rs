use crate::card_utils::*;
use crate::config::CONFIG;
use crate::nodes::*;
use crate::ranges::*;
use crate::trainer::*;
use crate::trainer_utils::*;
use moka::sync::Cache;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use smallvec::*;

type PreflopCache = Cache<(i32, ActionHistory), Strategy>;

pub struct Bot {
    // Right now the blueprint stores the mixed strategy for each infoset. To reduce
    // memory usage, we could pre-sample actions and just store a mapping of infoset -> action.
    blueprint: Nodes,
    preflop_cache: PreflopCache,
    subgame_solving: bool,
    early_stopping: bool,
    depth_limit: i32,
}

impl Bot {
    pub fn new(
        blueprint: Nodes,
        subgame_solving: bool,
        early_stopping: bool,
        depth_limit: i32,
    ) -> Bot {
        Bot {
            blueprint,
            preflop_cache: Cache::new(10_000),
            subgame_solving: subgame_solving,
            early_stopping: early_stopping,
            depth_limit: depth_limit,
        }
    }

    pub fn get_action(&self, hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
        let strategy = self.get_strategy(hand, board, history);
        let action = sample_action_from_strategy(&strategy);
        debug_assert!({
            let prob = strategy.get(&action).unwrap().clone();
            println!("Picked action {action} with probability {prob}");
            prob > 0.0
        });
        action
    }

    // Wrapper for the real time solving for the bot's strategy
    pub fn get_strategy(&self, hole: &[Card], board: &[Card], history: &ActionHistory) -> Strategy {
        if !self.subgame_solving || history.is_empty() {
            self.get_strategy_action_translation(hole, board, history)
        } else {
            // Preflop cache
            let key = (ABSTRACTION.bin(hole), history.clone());
            match self.preflop_cache.get(&key) {
                Some(strategy) => strategy,
                None => {
                    let strategy = self.unsafe_nested_subgame_solving(hole, board, history);
                    if history.street == PREFLOP {
                        self.preflop_cache.insert(key, strategy.clone());
                    }
                    strategy
                }
            }
        }
    }

    pub fn get_strategy_action_translation(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        debug_assert!(hole.len() == 2);
        // Only look at board cards for this street
        let board = &board[..board_length(history.street)];
        let translated = history.translate(&CONFIG.bet_abstraction);
        let node_strategy = self.blueprint.get_strategy(hole, board, &translated);
        let adjusted_strategy: Strategy = node_strategy
            .iter()
            .map(|(action, prob)| (history.adjust_action(&action), prob.clone()))
            .collect();
        let sum: f64 = adjusted_strategy.values().sum();
        debug_assert!(
            { (sum - 1.0).abs() < 0.01 },
            "Adjusted strategy {:?} sums to {} for original strategy {:?}",
            adjusted_strategy,
            sum,
            node_strategy
        );
        adjusted_strategy
    }

    fn unsafe_nested_subgame_solving(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        let board: SmallVecHand = board[..board_length(history.street)].to_smallvec();
        let mut hole: [Card; 2] = [hole[0], hole[1]];
        hole.sort();

        let subgame_root = history;
        let translated_history = subgame_root.translate(&CONFIG.bet_abstraction);

        // Get our beliefs of the opponent's range given their actions up to the subgame root.
        // Use action translation to map the actions so far to infosets in the blueprint strategy.
        let get_strategy = |hole: &[Card], board: &[Card], history: &ActionHistory| {
            self.get_strategy_action_translation(hole, board, history)
            // TODO: The other move here is to get the opponent range via past unsafe subgame solving,
            // and keep track of the range as you go.
        };

        let opp_range = Range::get_opponent_range(&hole, &board, &translated_history, get_strategy);
        // println!("Player hand: {}", cards2str(&hole));
        // println!("History: {}", translated);
        // println!("Board: {}", cards2str(&board));
        // println!("Beliefs about opponent hands:");

        // The blueprint currently has weird ideas about the opponent's range, but keep in mind
        // it is necessarily correct when playing against the blueprint.
        // let mut beliefs: Vec<([Card; 2], f64)> = opp_range
        //     .get_map()
        //     .iter()
        //     .map(|(hand, prob)| ([hand[0], hand[1]], prob.clone()))
        //     .collect();
        // beliefs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // for (hand, prob) in beliefs {
        //     println!("{}: {}", cards2str(&hand), prob)
        // }

        let opp_action = history.last_action().unwrap();

        // TODO: Add the opponent's action to the bet abstraction

        let nodes = Nodes::new(&CONFIG.bet_abstraction);
        let infoset = InfoSet::from_hand(&hole, &board, &translated_history);
        // TODO Refactor: rename this SmallVecFloats thing to like F32SmallVec or something
        let mut prev_strategy: SmallVecFloats =
            smallvec![-1.0; infoset.next_actions(&CONFIG.bet_abstraction).len()];

        let num_epochs = 10;
        let epoch = CONFIG.subgame_iters / num_epochs;
        for i in 0..num_epochs {
            if i > 0 {
                nodes.reset_strategy_sum(&infoset);
            }
            let bar = pbar(epoch);
            (0..epoch).into_par_iter().for_each(|_| {
                for player in [DEALER, OPPONENT].iter() {
                    let mut deck = deck();
                    deck.retain(|c| !hole.contains(c));
                    deck.retain(|c| !board.contains(c));
                    deck.shuffle(&mut rand::thread_rng());

                    let mut board = board.clone();
                    board.extend(deck.iter().take(5 - board.len()).cloned());
                    let board = [board[0], board[1], board[2], board[3], board[4]];

                    let mut range = Range::new();
                    range.remove_blockers(&board);
                    let mut preflop_hands = Vec::with_capacity(range.hands.len());
                    // TODO Refactor: have a clean way to return a list of the non blocking hands. this
                    // is duplicated in cfr_iteration as well.
                    let mut traverser_reach_probs = Vec::with_capacity(range.hands.len());
                    let mut opp_reach_probs = Vec::with_capacity(range.hands.len());
                    for hand_index in 0..range.hands.len() {
                        let prob = range.probs[hand_index];
                        if prob > 0.0 {
                            preflop_hands.push(range.hands[hand_index]);
                            if range.hands[hand_index] == hole {
                                traverser_reach_probs.push(1.0);
                            } else {
                                traverser_reach_probs.push(0.0);
                            }
                            opp_reach_probs.push(opp_range.probs[hand_index]);
                        }
                    }
                    iterate(
                        player.clone(),
                        preflop_hands,
                        board,
                        &translated_history,
                        traverser_reach_probs,
                        opp_reach_probs,
                        &nodes,
                        self.depth_limit,
                        Some(&self),
                    );
                }
                bar.inc(1);
            });

            // Stop early if the cumulative strategy has not changed in 1000 iters
            let node = nodes.get(&infoset).expect(
                format!(
                    "Infoset {:?} not found in subgame nodes: {:?}",
                    infoset, nodes
                )
                .as_str(),
            );
            let strategy = node.cumulative_strategy();
            let diff: f32 = strategy
                .iter()
                .zip(prev_strategy.iter())
                .map(|(&a, &b)| (a - b).abs())
                .sum();
            println!(
                "Hand: {} Board: {} | History: {}",
                cards2str(&hole),
                cards2str(&board),
                history
            );
            println!(
                "Actions: {:?}",
                infoset.next_actions(&CONFIG.bet_abstraction)
            );
            println!("Node: {:?}", node);
            println!("Strategy: {:?}", strategy);
            if self.early_stopping && diff < 0.01 {
                println!("Stopping early because CFR strategy has converged.");
                break;
            }
            prev_strategy = strategy;
            bar.finish();
        }

        let strategy = nodes.get_strategy(&hole, &board, history);
        strategy
    }
}
