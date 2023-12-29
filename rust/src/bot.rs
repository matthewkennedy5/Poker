use crate::card_utils::*;
use crate::config::CONFIG;
use crate::nodes::*;
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
}

impl Bot {
    pub fn new(blueprint: Nodes, subgame_solving: bool) -> Bot {
        Bot {
            blueprint,
            preflop_cache: Cache::new(10_000),
            subgame_solving: subgame_solving,
        }
    }

    pub fn get_action(&self, hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
        let mut strategy = self.get_strategy(hand, board, history);

        // Smoothing - if an action prob is below 3%, make it 0%. sample_action_from_strategy doesn't
        // need a normalized strategy
        for (action, prob) in strategy.clone() {
            if prob < 0.05 {
                strategy.insert(action, 0.0);
            }
        }

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
        if !self.subgame_solving || history.is_empty() || history.street < RIVER {
            self.get_strategy_action_translation(hole, board, history)
        } else {
            // Preflop cache
            let key = (ABSTRACTION.bin(hole), history.clone());
            match self.preflop_cache.get(&key) {
                Some(strategy) => strategy,
                None => {
                    let strategy = self.solve_subgame_unsafe(hole, board, history);
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
        assert!(&translated == history);
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

    fn solve_subgame_unsafe(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        let board: SmallVecHand = board[..board_length(history.street)].to_smallvec();
        let hole: [Card; 2] = [hole[0], hole[1]];

        let nodes = Nodes::new(&CONFIG.bet_abstraction);
        let infoset = InfoSet::from_hand(&hole, &board, &history);

        let depth_limit_bot = Bot::new(load_nodes(&CONFIG.nodes_path), false);

        const NUM_EPOCHS: u64 = 2;
        let epoch = CONFIG.subgame_iters / NUM_EPOCHS;
        for i in 0..NUM_EPOCHS {
            if i == 1 {
                nodes.reset_strategy_sum(&infoset);
            }
            let bar = pbar(epoch);
            (0..epoch).into_par_iter().for_each(|_| {
                for traverser in [DEALER, OPPONENT].iter() {
                    let mut deck = deck();
                    deck.retain(|c| !hole.contains(c));
                    deck.retain(|c| !board.contains(c));
                    deck.shuffle(&mut rand::thread_rng());

                    let mut board = board.clone();
                    board.extend(deck.iter().take(5 - board.len()).cloned());
                    let board = [board[0], board[1], board[2], board[3], board[4]];

                    let preflop_hands = non_blocking_preflop_hands(&board);

                    // Get reach probs for each player based on their actions
                    let mut traverser_reach_probs = vec![1.0; preflop_hands.len()];
                    let mut opp_reach_probs = vec![1.0; preflop_hands.len()];
                    let mut history_iter = ActionHistory::new();
                    for action in history.get_actions() {
                        for (i, preflop_hand) in preflop_hands.iter().enumerate() {
                            let strat = self.get_strategy_action_translation(
                                preflop_hand,
                                &board,
                                &history_iter,
                            );
                            let prob = strat.get(&action).expect("Action not in strategy");
                            if &history_iter.player == traverser {
                                traverser_reach_probs[i] *= prob;
                            } else {
                                opp_reach_probs[i] *= prob;
                            }
                        }
                        history_iter.add(&action);
                    }

                    iterate(
                        traverser.clone(),
                        preflop_hands,
                        board,
                        &history,
                        traverser_reach_probs,
                        opp_reach_probs,
                        &nodes,
                        0,
                        Some(&depth_limit_bot),
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
            println!(
                "Hand: {} Board: {} | History: {}",
                cards2str(&hole),
                cards2str(&board),
                history
            );
            println!("Strategy:");
            for (action, prob) in infoset
                .next_actions(&CONFIG.bet_abstraction)
                .iter()
                .zip(strategy.iter())
            {
                println!("{action}: {prob}");
            }
            bar.finish();
        }

        let strategy = nodes.get_strategy(&hole, &board, history);
        strategy
    }
}
