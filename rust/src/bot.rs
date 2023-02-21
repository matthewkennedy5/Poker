// Real-time bot logic. Right now this just does action translation, but this
// is where I will add depth-limited solving.
use crate::card_utils::Card;
use crate::config::CONFIG;
use crate::trainer;
use crate::trainer_utils::*;
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

    pub fn get_strategy(
        &self,
        hand: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> HashMap<Action, f64> {
        assert!(hand.len() == 2);
        assert!(board.len() <= 5);
        let translated = history.translate(&CONFIG.bet_abstraction);
        let hand = [hand, board].concat();
        let infoset = InfoSet::from_hand(&hand, &translated).compress();

        let node = {
            match self.blueprint.get(&infoset) {
                Some(node) => node.clone(),
                None => {
                    Node::new(&infoset.uncompress()).clone()
                }
            }
        };
        let node_strategy = node.cumulative_strategy();
        let mut adjusted_strategy: HashMap<Action, f64> = HashMap::new();
        for (action, prob) in node_strategy {
            adjusted_strategy.insert(history.adjust_action(&action), prob);
        }
        adjusted_strategy
    }

    pub fn get_action(&self, hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
        let strategy = self.get_strategy(hand, board, history);
        let action = sample_action_from_strategy(&strategy);
        action
    }
}
