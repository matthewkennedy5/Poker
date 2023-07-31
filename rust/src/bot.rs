use crate::card_utils::*;
use crate::config::CONFIG;
use crate::nodes::*;
use crate::ranges::*;
use crate::trainer::*;
use crate::trainer_utils::*;
use moka::sync::Cache;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use smallvec::ToSmallVec;
use smallvec::*;

type PreflopCache = Cache<(i32, ActionHistory), Strategy>;

pub struct Bot {
    // Right now the blueprint stores the mixed strategy for each infoset. To reduce
    // memory usage, we could pre-sample actions and just store a mapping of infoset -> action.
    pub blueprint: Nodes,
    pub preflop_cache: PreflopCache,
    pub subgame_solving: bool,
}

impl Bot {
    pub fn new() -> Bot {
        let blueprint = load_nodes(&CONFIG.nodes_path);
        Bot {
            blueprint,
            preflop_cache: Cache::new(10_000),
            subgame_solving: CONFIG.subgame_solving,
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

    fn get_strategy_action_translation(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        debug_assert!(hole.len() == 2);
        // Only look at board cards for this street
        let board = &board[..board_length(history.street)];
        let translated = history.translate(&CONFIG.bet_abstraction);
        let node_strategy =
            self.blueprint
                .get_strategy(hole, board, &translated, &CONFIG.bet_abstraction);
        let adjusted_strategy = node_strategy
            .iter()
            .map(|(action, prob)| (history.adjust_action(&action), prob.clone()))
            .collect();
        adjusted_strategy
    }

    fn unsafe_nested_subgame_solving(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        let board: SmallVecHand = board[..board_length(history.street)].to_smallvec();

        let subgame_root = history;
        let translated = subgame_root.translate(&CONFIG.bet_abstraction);

        // Get our beliefs of the opponent's range given their actions up to the subgame root.
        // Use action translation to map the actions so far to infosets in the blueprint strategy.
        let get_strategy = |hole: &[Card], board: &[Card], history: &ActionHistory| {
            self.get_strategy_action_translation(hole, board, history)
            // TODO: The other move here is to get the opponent range via past unsafe subgame solving,
            // and keep track of the range as you go.
        };

        let opp_range = Range::get_opponent_range(hole, &board, &translated, get_strategy);
        let opp_action = history.last_action().unwrap();

        // Add the opponent's action to the bet abstraction
        let mut new_abstraction = CONFIG.bet_abstraction.clone();
        let pot_frac = (opp_action.amount as f64) / (subgame_root.pot() as f64);
        if opp_action.action == ActionType::Bet
            && !new_abstraction[subgame_root.street].contains(&pot_frac)
        {
            // If the opponent made an off-tree bet, add it to the bet abstraction
            new_abstraction[subgame_root.street].push(pot_frac);
        }

        let nodes = Nodes::new();
        let infoset = InfoSet::from_hand(hole, &board, history);
        // let prev_strategy: Mutex<SmallVec<[f64; NUM_ACTIONS]>> = Mutex::new(smallvec![-1.0; NUM_ACTIONS]);
        // let i: Mutex<i32> = Mutex::new(0);
        let mut prev_strategy: SmallVec<[f64; NUM_ACTIONS]> = smallvec![-1.0; NUM_ACTIONS];
        let bar = pbar(CONFIG.subgame_iters);

        let epoch_size = 10_000;
        let num_epochs = CONFIG.subgame_iters / epoch_size;

        for _ in 0..num_epochs {
            // Clear the cumulative strategy at the begging of each epoch
            nodes.reset_strategy_sum(&infoset, &CONFIG.bet_abstraction);

            (0..epoch_size).into_par_iter().for_each(|_| {
                // Construct a plausible deck using:
                // - Our hand (player's hand)
                // - Opponent hand sampled from our belief of their range
                // - Current board cards
                // - Then shuffle the rest of the deck for the remaining board cards
                let opp_hand = opp_range.sample_hand();
                let mut current_deck: Vec<Card> = Vec::with_capacity(52);
                if history.player == DEALER {
                    current_deck.extend(hole);
                    current_deck.extend(opp_hand);
                } else {
                    current_deck.extend(opp_hand);
                    current_deck.extend(hole);
                }
                current_deck.extend(board.iter());
                let mut rest_of_deck = deck();
                rest_of_deck.retain(|c| !current_deck.contains(&c));
                rest_of_deck.shuffle(&mut rand::thread_rng());
                current_deck.extend(rest_of_deck);

                for player in [DEALER, OPPONENT].iter() {
                    iterate(
                        player.clone(),
                        &current_deck,
                        history,
                        [1.0, 1.0],
                        &nodes,
                        &CONFIG.bet_abstraction,
                        CONFIG.depth_limit,
                    );
                }
                bar.inc(1);
            });

            // Stop early if the cumulative strategy has not changed in 1000 iters
            // let mut i = i.lock().unwrap();
            let node = nodes
                .get(&infoset)
                .expect("Infoset not found in subgame nodes");
            let strategy = node.cumulative_strategy();
            let diff: f64 = strategy
                .iter()
                .zip(prev_strategy.iter())
                .map(|(&a, &b)| (a - b).abs())
                .sum();
            // println!("InfoSet: {infoset}");
            println!(
                "Hand: {} Board: {} | History: {}",
                cards2str(hole),
                cards2str(&board),
                history
            );
            println!("Actions: {:?}", infoset.next_actions(&new_abstraction));
            println!("Node: {:?}", node);
            println!(
                "Strategy: {:?} Prev strategy: {:?}",
                strategy, prev_strategy
            );
            if diff < 1e-3 {
                println!("Stopping early because CFR strategy has converged.");
                break;
            }
            prev_strategy = strategy.clone();
        }
        bar.finish();

        let strategy = nodes.get_strategy(hole, &board, history, &new_abstraction); // TODO: Should this just be CONFIG.bet_abstraction??
        strategy
    }
}
