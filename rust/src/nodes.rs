use crate::card_utils::*;
use crate::config::CONFIG;
use crate::trainer_utils::*;
use dashmap::DashMap;
use smallvec::SmallVec;
use std::sync::Mutex;

// Upper limit on branching factor of blueprint game tree.
pub const NUM_ACTIONS: usize = 5;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes {
    // TODO: Maybe change to Mutex since you only read via cloning anyway
    pub dashmap: DashMap<ActionHistory, Mutex<Vec<Node>>>,
}

impl Nodes {
    pub fn new() -> Nodes {
        Nodes {
            dashmap: DashMap::new(),
        }
    }

    pub fn get(&self, infoset: &InfoSet) -> Option<Node> {
        let nodes = match self.dashmap.get(&infoset.history) {
            Some(n) => n,
            None => return None,
        };
        let node = nodes
            .lock()
            .unwrap()
            .get(infoset.card_bucket as usize)
            .cloned();
        node
    }

    // TODO: let's refactor a better solution to avoid needing to pass bet_abstraction all over the place

    pub fn add_regret(
        &self,
        infoset: &InfoSet,
        bet_abstraction: &[Vec<f64>],
        action_index: usize,
        regret: f64,
    ) {
        let history = infoset.history.clone();
        // TODO: There's a data race here on initialization, but it's not that important
        if !self.dashmap.contains_key(&history) {
            self.initialize_node_vec(&history, bet_abstraction);
        }
        let node_vec_lock = self.dashmap.get_mut(&history).unwrap();
        let mut node_vec = node_vec_lock.lock().unwrap();
        let mut node = node_vec.get_mut(infoset.card_bucket as usize).unwrap();
        debug_assert!(action_index < node.num_actions);
        let accumulated_regret = node.regrets[action_index] + regret;
        node.regrets[action_index] = accumulated_regret;
    }

    pub fn update_strategy_sum(&self, infoset: &InfoSet, bet_abstraction: &[Vec<f64>], prob: f64) {
        let history = infoset.history.clone();
        if !self.dashmap.contains_key(&history) {
            self.initialize_node_vec(&history, bet_abstraction);
        }
        let node_vec_lock = self.dashmap.get_mut(&history).unwrap();
        let mut node_vec = node_vec_lock.lock().unwrap();
        let mut node = node_vec.get_mut(infoset.card_bucket as usize).unwrap();
        let positive_regrets: SmallVec<[f64; NUM_ACTIONS]> = node
            .regrets
            .iter()
            .take(node.num_actions)
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let current_strategy: SmallVec<[f64; NUM_ACTIONS]> = normalize_smallvec(&positive_regrets);
        if prob > 0.0 {
            for i in 0..current_strategy.len() {
                // Add this action's probability to the cumulative strategy sum
                node.strategy_sum[i] += current_strategy[i] * prob;
            }
        }
        node.t += 1;
    }

    pub fn reset_strategy_sum(&self, infoset: &InfoSet, bet_abstraction: &[Vec<f64>]) {
        let history = infoset.history.clone();
        if !self.dashmap.contains_key(&history) {
            self.initialize_node_vec(&history, bet_abstraction);
        }
        let node_vec_lock = self.dashmap.get_mut(&history).unwrap();
        let mut node_vec = node_vec_lock.lock().unwrap();
        let mut node = node_vec.get_mut(infoset.card_bucket as usize).unwrap();
        node.strategy_sum = [0.0; NUM_ACTIONS];
    }

    pub fn get_current_strategy(
        &self,
        infoset: &InfoSet,
        bet_abstraction: &[Vec<f64>],
    ) -> SmallVec<[f64; NUM_ACTIONS]> {
        let num_actions = infoset.next_actions(bet_abstraction).len();
        let node = match self.get(infoset) {
            Some(n) => n,
            None => Node::new(num_actions),
        };
        let positive_regrets: SmallVec<[f64; NUM_ACTIONS]> = node
            .regrets
            .iter()
            .take(node.num_actions)
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let regret_norm: SmallVec<[f64; NUM_ACTIONS]> = normalize_smallvec(&positive_regrets);
        debug_assert!(regret_norm.len() == node.num_actions);
        regret_norm
    }

    fn initialize_node_vec(&self, history: &ActionHistory, bet_abstraction: &[Vec<f64>]) {
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
        let new_node = Node::new(history.next_actions(bet_abstraction).len());
        self.dashmap
            .insert(history.clone(), Mutex::new(vec![new_node; n_buckets]));
    }

    pub fn len(&self) -> usize {
        let mut length = 0;
        self.dashmap.iter().for_each(|elem| {
            let nodes = elem.value();
            length += nodes.lock().unwrap().len();
        });
        length
    }

    pub fn get_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
        bet_abstraction: &[Vec<f64>],
    ) -> Strategy {
        let infoset = InfoSet::from_hand(hole, board, history);
        let num_actions = infoset.next_actions(bet_abstraction).len();
        let node = match self.get(&infoset) {
            Some(n) => n.clone(),
            None => Node::new(num_actions),
        };
        debug_assert!(
            node.num_actions == num_actions,
            "{} {}",
            node.num_actions,
            num_actions
        );
        let mut strategy = Strategy::new();
        let actions = infoset.next_actions(bet_abstraction);
        for (action, prob) in actions.iter().zip(node.cumulative_strategy()) {
            strategy.insert(action.clone(), prob);
        }
        strategy
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub regrets: [f64; NUM_ACTIONS],
    pub strategy_sum: [f64; NUM_ACTIONS],
    pub num_actions: usize,
    pub t: u64,
}

impl Node {
    pub fn new(num_actions: usize) -> Node {
        Node {
            regrets: [0.0; NUM_ACTIONS],
            strategy_sum: [0.0; NUM_ACTIONS],
            num_actions: num_actions,
            t: 0,
        }
    }

    pub fn cumulative_strategy(&self) -> SmallVec<[f64; NUM_ACTIONS]> {
        normalize_smallvec(&self.strategy_sum[..self.num_actions])
    }
}
