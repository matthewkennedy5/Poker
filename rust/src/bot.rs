// Real-time bot logic. Right now this just does action translation, but this
// is where I will add depth-limited solving.
use crate::card_utils::{self, Card};
use crate::config::CONFIG;
use crate::trainer;
use crate::trainer_utils::*;
use crate::ranges::*;
use rand::prelude::*;
use std::collections::HashMap;

pub struct Bot {
    // Right now the blueprint stores the mixed strategy for each infoset. To reduce
    // memory usage, we could pre-sample actions and just store a mapping of infoset -> action.
    blueprint: HashMap<CompactInfoSet, Node>,
}

impl Bot {
    pub fn new() -> Bot {
        let blueprint = trainer::load_nodes(&CONFIG.nodes_path);
        Bot { blueprint }
    }

    pub fn get_action(&self, hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
        let strategy = self.get_strategy(hand, board, history);
        let action = sample_action_from_strategy(&strategy);
        action
    }

    // Wrapper for the real time solving for the bot's strategy
    // TODO: Refactor this to maybe just input an infoset, or just a hand. The hole and board inputs add complexity
    // sine it's different than the rest of the codebase. 
    pub fn get_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> HashMap<Action, f64> {
        // self.get_strategy_action_translation(hole, board, history)
        if history.is_empty() {
            return self.get_strategy_action_translation(hole, board, history);
        } else {
            return self.unsafe_nested_subgame_solving(hole, board, history);
        }
    }

    fn get_strategy_action_translation(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> HashMap<Action, f64> {
        assert!(hole.len() == 2);
        assert!(board.len() <= 5);
        let translated = history.translate(&CONFIG.bet_abstraction);
        let hand = [hole, board].concat();
        let infoset = InfoSet::from_hand(&hand, &translated).compress();

        let node = self
            .blueprint
            .get(&infoset)
            .expect(format!("Infoset not in blueprint: {:?}", infoset).as_str());
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
        // // 1. Perform action translation for all actions except the last one
        // let subgame_root: ActionHistory = history.without_last_action()
        //                                          .translate(&CONFIG.bet_abstraction);

        // // 2. Get our beliefs of the opponent's range given their actions (not including the last)
        // // action, which might be off-tree
        // let opp_range = Range::get_opponent_range();//&history, board, &self.blueprint);

        // // 2. Solve the opponent's subgame, including their action in the abstraction 
        // let nodes = self.solve_subgame(&subgame_root, 
        //                                                        &opp_range, 
        //                                                        history.last_action().unwrap(),
        //                                                        100);

        // // 3. That gives us our strategy in response to their action. 
        // let mut this_history = subgame_root.clone();
        // this_history.add(&history.last_action().unwrap());
        // let hand = [hole, board].concat();
        // let infoset: CompactInfoSet = InfoSet::from_hand(&hand, &this_history).compress();
        // let node = nodes.get(&infoset).expect("Infoset not in subgame");
        // node.cumulative_strategy()
        panic!("Not implemented yet");
    }

    // Use unsafe subgame solving to return the Nash equilibrium strategy for the current spot, 
    // assuming that the opponent is playing with the given range. 
    // 
    // Inputs:
    //      history: The history of actions leading up to this spot, not including the opponent's action
    //      opp_range: Our Bayesian belief distribution of the cards the opponent has
    //      opp_action: Action the opponent took at this spot, which might not be in the action abstraction
    //      iters: How many iterations of CFR to run
    // 
    // Returns:
    //      nodes: The solved CFR tree nodes for each infoset in the subgame
    fn solve_subgame(
        &self,
        history: &ActionHistory,
        opp_range: &HashMap<Vec<Card>, f64>,
        opp_action: Action,
        iters: u64,
    ) -> HashMap<CompactInfoSet, Node> {

        let mut nodes: HashMap<CompactInfoSet, Node> = HashMap::new();

        for _i in 0..iters {
            panic!("Not implemented yet");
            // sample an opponent hand from the range
            // remove blockers from the deck
            // shuffle the rest of the deck
            // run a CFR iteration using the range for the opponent weight (I think?)
            // shuffle the deck again
            // run a CFR iteration for the hero
        }

        nodes
    }
}
