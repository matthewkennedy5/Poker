use crate::card_utils::{self, Card};
use crate::config::CONFIG;
use crate::ranges::*;
use crate::trainer::*;
use crate::trainer_utils::*;
use std::collections::HashMap;

pub struct Bot {
    // Right now the blueprint stores the mixed strategy for each infoset. To reduce
    // memory usage, we could pre-sample actions and just store a mapping of infoset -> action.
    blueprint: HashMap<InfoSet, Node>,
}

impl Bot {
    pub fn new() -> Bot {
        let blueprint = load_nodes(&CONFIG.nodes_path);
        Bot { blueprint }
    }

    pub fn get_action(&self, hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
        let strategy = self.get_strategy(hand, board, history);
        let action = sample_action_from_strategy(&strategy);
        action
    }

    // Wrapper for the real time solving for the bot's strategy
    // TODO: Refactor this to maybe just input an infoset, or just a hand. The hole and board inputs add complexity
    // since it's different than the rest of the codebase.
    pub fn get_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> HashMap<Action, f64> {
        if !CONFIG.subgame_solving || history.is_empty() {
            self.get_strategy_action_translation(hole, board, history)
        } else {
            self.unsafe_nested_subgame_solving(hole, board, history)
        }
    }

    fn get_strategy_action_translation(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> HashMap<Action, f64> {
        assert!(hole.len() == 2);
        // Only look at board cards for this street
        let board = &board[..board_length(history.street)];    
        let translated = history.translate(&CONFIG.bet_abstraction);
        let infoset = InfoSet::from_hand(hole, board, &translated);
        let node = lookup_or_new(&self.blueprint, &infoset, &CONFIG.bet_abstraction);
        let node_strategy = node.cumulative_strategy();

        let mut adjusted_strategy: HashMap<Action, f64> = HashMap::new();
        for (action, prob) in node_strategy {
            adjusted_strategy.insert(history.adjust_action(&action), prob);
        }
        adjusted_strategy
    }

    fn unsafe_nested_subgame_solving(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> HashMap<Action, f64> {
        let subgame_root: ActionHistory = history.without_last_action();
        let translated = subgame_root.translate(&CONFIG.bet_abstraction);

        // Get our beliefs of the opponent's range given their actions up to the subgame root.
        // Use action translation to map the actions so far to infosets in the blueprint strategy.
        let get_strategy = |hole: &[Card], board: &[Card], history: &ActionHistory| {
            self.get_strategy_action_translation(hole, board, &history)
        };

        let opp_range = Range::get_opponent_range(hole, board, &translated, get_strategy);

        let nodes = self.solve_subgame(
            &subgame_root,
            &opp_range,
            history.last_action().unwrap(),
            CONFIG.subgame_iters,
        );

        // That gives us our strategy in response to their action.
        let mut this_history = subgame_root.clone();
        this_history.add(&history.last_action().unwrap());
        let infoset = InfoSet::from_hand(hole, board, &this_history);
        let node = lookup_or_new(&nodes, &infoset, &CONFIG.bet_abstraction);
        node.cumulative_strategy()
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
    fn solve_subgame(
        &self,
        history: &ActionHistory,
        opp_range: &Range,
        opp_action: Action,
        iters: u64,
    ) -> HashMap<InfoSet, Node> {
        let mut nodes: HashMap<InfoSet, Node> = HashMap::new();
        let mut bet_abstraction = CONFIG.bet_abstraction.clone();
        let pot_frac = (opp_action.amount as f64) / (history.pot() as f64);
        if opp_action.action == ActionType::Bet && !bet_abstraction[history.street].contains(&pot_frac) {
            // If the opponent made an off-tree bet, add it to the bet abstraction
            bet_abstraction[history.street].push(pot_frac);
        }
        bet_abstraction[history.street].push(pot_frac);
        for _i in 0..iters {
            let opp_hand = opp_range.sample_hand();
            let mut deck = card_utils::deck();
            // Remove opponent's cards (blockers) from the deck
            deck.retain(|card| !opp_hand.contains(card));
            cfr_iteration(&deck, history, &mut nodes, &bet_abstraction);
        }
        nodes
    }
}
