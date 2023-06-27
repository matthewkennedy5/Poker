use crate::config::CONFIG;
use crate::trainer_utils::*;
use crate::card_utils::*;
use smallvec::SmallVec;
use dashmap::DashMap;

// Upper limit on branching factor of blueprint game tree. For setting the SmallVec size.
pub const NUM_ACTIONS: usize = 5;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes {
    pub dashmap: DashMap<ActionHistory, Vec<Node>>,
}

impl Nodes {
    pub fn new() -> Nodes {
        Nodes {
            dashmap: DashMap::new()
        }
    }
    
    pub fn get(&self, infoset: &InfoSet) -> Option<Node> {
        let nodes = match self.dashmap.get(&infoset.history) {
            Some(n) => n,
            None => return None
        };
        nodes.get(infoset.card_bucket as usize).cloned()
    }

    pub fn insert(&self, infoset: InfoSet, node: Node) {
        let history = infoset.history;
        if !self.dashmap.contains_key(&history) {
            // Create the Vec<Node> at this history if it doesn't exist yet
            let n_buckets = if history.street == PREFLOP {
                169
            } else if history.street == FLOP {
                CONFIG.flop_buckets
            } else if history.street == TURN {
                CONFIG.turn_buckets
            } else if history.street == RIVER {
                CONFIG.river_buckets
            } else {
                panic!("Bad street")
            } as usize;
            let new_node = Node::new(history.next_actions(&CONFIG.bet_abstraction).len());
            self.dashmap.insert(history.clone(), vec![new_node; n_buckets]);
        }
        let mut nodes = self.dashmap.get_mut(&history).unwrap();
        let bucket_index = infoset.card_bucket as usize;
        nodes[bucket_index] = node;
    }

    pub fn len(&self) -> usize {
        let mut length = 0;
        self.dashmap.iter().for_each(|elem| {
            let nodes = elem.value();
            length += nodes.len();
        });
        length
    }

    pub fn get_strategy(&self, hole: &[Card], board: &[Card], history: &ActionHistory) -> Strategy {
        let infoset = InfoSet::from_hand(hole, board, history);
        let node = lookup_or_new(self, &infoset, &CONFIG.bet_abstraction);
        let mut strategy = Strategy::new();
        let actions = infoset.next_actions(&CONFIG.bet_abstraction);
        for (action, prob) in actions.iter().zip(node.cumulative_strategy()) {
            strategy.insert(action.clone(), prob);
        }
        strategy
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub regrets: [f64; NUM_ACTIONS],
    strategy_sum: [f64; NUM_ACTIONS],
    num_actions: usize,
    pub t: u64,
}

impl Node {
    pub fn new(num_actions: usize) -> Node {
        Node {
            regrets: [0.0; NUM_ACTIONS],
            strategy_sum: [0.0; NUM_ACTIONS],
            num_actions: num_actions,
            t: 0
        }
    }

    // Returns the current strategy for this node, and updates the cumulative strategy
    // as a side effect.
    // Input: prob is the probability of reaching this node
    pub fn current_strategy(&mut self, prob: f64) -> SmallVec<[f64; NUM_ACTIONS]> {
        // Normalize the regrets for this iteration of CFR
        let positive_regrets: SmallVec<[f64; NUM_ACTIONS]> = self
            .regrets
            .iter()
            .take(self.num_actions)
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let regret_norm: SmallVec<[f64; NUM_ACTIONS]> = normalize_smallvec(&positive_regrets);
        if prob > 0.0 {
            for i in 0..regret_norm.len() {
                // Add this action's probability to the cumulative strategy sum 
                let new_prob = regret_norm[i] * prob;
                self.strategy_sum[i] += new_prob;
                self.strategy_sum[i] *= CONFIG.decay;
            }
            self.t += 1;
        }
        debug_assert!(regret_norm.len() == self.num_actions);
        regret_norm
    }

    pub fn cumulative_strategy(&self) -> SmallVec<[f64; NUM_ACTIONS]> {
        normalize_smallvec(&self.strategy_sum[..self.num_actions])
    }

    pub fn add_regret(&mut self, action_index: usize, regret: f64) {
        debug_assert!(action_index < self.num_actions);
        let accumulated_regret = self.regrets[action_index] + regret;
        self.regrets[action_index] = accumulated_regret;
    }
}
