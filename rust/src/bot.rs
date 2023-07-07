use crate::card_utils::*;
use crate::config::CONFIG;
use crate::ranges::*;
use crate::trainer::*;
use crate::trainer_utils::*;
use crate::nodes::*;
use rayon::prelude::*;
use moka::sync::Cache;

type PreflopCache = Cache<(i32, ActionHistory), Strategy>;

pub struct Bot {
    // Right now the blueprint stores the mixed strategy for each infoset. To reduce
    // memory usage, we could pre-sample actions and just store a mapping of infoset -> action.
    pub blueprint: Nodes,
    pub preflop_cache: PreflopCache,
    pub subgame_solving: bool
}

impl Bot {
    pub fn new() -> Bot {
        let blueprint = load_nodes(&CONFIG.nodes_path);
        Bot {
            blueprint,
            preflop_cache: Cache::new(100_000),
            subgame_solving: CONFIG.subgame_solving
        }
    }

    pub fn get_action(&self, hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
        let strategy = self.get_strategy(hand, board, history);
        sample_action_from_strategy(&strategy)
    }

    // Wrapper for the real time solving for the bot's strategy
    pub fn get_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
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
        let node_strategy = self.blueprint.get_strategy(hole, board, &translated, &CONFIG.bet_abstraction);
        let adjusted_strategy = node_strategy.iter().map(|(action, prob)| {
            (history.adjust_action(&action), prob.clone())
        }).collect();
        adjusted_strategy
    }
    
    // TODO: I think you should also your hand (hole and board) to the card abstraction here. That way you 
    // perfectly understand the cards on the table when computing your strategy.  
    fn unsafe_nested_subgame_solving(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        let subgame_root: ActionHistory = history.without_last_action();
        let translated = subgame_root.translate(&CONFIG.bet_abstraction);

        // Get our beliefs of the opponent's range given their actions up to the subgame root.
        // Use action translation to map the actions so far to infosets in the blueprint strategy.
        let get_strategy = |hole: &[Card], board: &[Card], history: &ActionHistory| {
            self.get_strategy_action_translation(hole, board, history)
        };

        let opp_range = Range::get_opponent_range(hole, board, &translated, get_strategy);
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

        let nodes = Bot::solve_subgame(
            &subgame_root,
            &opp_range,
            &new_abstraction,
            CONFIG.subgame_iters,
            CONFIG.depth_limit
        );

        // Debug info
        let infoset = InfoSet::from_hand(hole, board, history);
        let node = nodes.get(&infoset).unwrap();
        println!("InfoSet: {infoset}");
        println!("Actions: {:?}", infoset.next_actions(&new_abstraction));
        println!("Node: {:?}", node);

        let strategy = nodes.get_strategy(hole, board, history, &new_abstraction);
        strategy
    }

    // Uses unsafe subgame solving to return the Nash equilibrium strategy for the current spot,
    // assuming that the opponent is playing with the given range.
    //
    // Inputs:
    //      history: The history of actions leading up to this spot, not including the opponent's
    //          most recent action
    //      opp_range: Our Bayesian belief distribution of the cards the opponent has
    //      opp_action: Action the opponent took at this spot, which might not be in the action
    //          abstraction
    //      iters: How many iterations of CFR to run
    //
    // Returns:
    //      nodes: The solved CFR tree nodes for each infoset in the subgame
    pub fn solve_subgame(
        history: &ActionHistory,
        opp_range: &Range,
        bet_abstraction: &[Vec<f64>],
        iters: u64,
        depth_limit: i32
    ) -> Nodes {
        let nodes: Nodes = Nodes::new();
        let bar = pbar(iters);
        (0..iters).into_par_iter().for_each(|_i| {
            let opp_hand = opp_range.sample_hand();
            let mut deck = deck();
            // Remove opponent's cards (blockers) from the deck
            deck.retain(|card| !opp_hand.contains(card));
            cfr_iteration(&deck, history, &nodes, bet_abstraction, depth_limit);
            bar.inc(1);
        });
        bar.finish();
        nodes
    }
}
