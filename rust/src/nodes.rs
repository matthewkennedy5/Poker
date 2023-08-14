use crate::card_utils::*;
use crate::config::CONFIG;
use crate::trainer_utils::*;
use dashmap::DashMap;
use smallvec::SmallVec;
use std::sync::Mutex;

// Upper limit on branching factor of blueprint game tree.
pub const NUM_ACTIONS: usize = 4;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes {
    pub dashmap: DashMap<ActionHistory, Mutex<Vec<Node>>>,
    pub bet_abstraction: Vec<Vec<f64>>,
}

impl Nodes {
    // TODO REFACTOR: Change the bet_abstraction &[Vec<f64>] to be its own BetAbstraction type
    pub fn new(bet_abstraction: &[Vec<f64>]) -> Nodes {
        Nodes {
            dashmap: DashMap::new(),
            bet_abstraction: bet_abstraction.to_vec(),
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

    pub fn add_regret(&self, infoset: &InfoSet, action_index: usize, regret: f64) {
        let history = infoset.history.clone();
        // TODO: There's a data race here on initialization, but it's not that important
        if !self.dashmap.contains_key(&history) {
            self.initialize_node_vec(&history);
        }
        let node_vec_lock = self.dashmap.get_mut(&history).unwrap();
        let mut node_vec = node_vec_lock.lock().unwrap();
        let node = node_vec.get_mut(infoset.card_bucket as usize).unwrap();
        // debug_assert!(action_index < node.num_actions);
        let accumulated_regret = node.regrets[action_index] + regret as f32;
        node.regrets[action_index] = accumulated_regret;
    }

    pub fn update_strategy_sum(&self, infoset: &InfoSet, prob: f32) {
        let history = infoset.history.clone();
        if !self.dashmap.contains_key(&history) {
            self.initialize_node_vec(&history);
        }
        let node_vec_lock = self.dashmap.get_mut(&history).unwrap();
        let mut node_vec = node_vec_lock.lock().unwrap();
        let node = node_vec.get_mut(infoset.card_bucket as usize).unwrap();
        let positive_regrets: SmallVecFloats = node
            .regrets
            .iter()
            .take(node.num_actions)
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let current_strategy: SmallVecFloats = normalize_smallvec(&positive_regrets);
        if prob > 0.0 {
            for i in 0..current_strategy.len() {
                // Add this action's probability to the cumulative strategy sum
                node.strategy_sum[i] += current_strategy[i] * prob;
            }
        }
        node.t += 1;
    }

    pub fn reset_strategy_sum(&self, infoset: &InfoSet) {
        let history = infoset.history.clone();
        if !self.dashmap.contains_key(&history) {
            self.initialize_node_vec(&history);
        }
        let node_vec_lock = self.dashmap.get_mut(&history).unwrap();
        let mut node_vec = node_vec_lock.lock().unwrap();
        let node = node_vec.get_mut(infoset.card_bucket as usize).unwrap();
        node.strategy_sum = [0.0; NUM_ACTIONS];
    }

    pub fn get_current_strategy(&self, infoset: &InfoSet) -> SmallVecFloats {
        let num_actions = infoset.next_actions(&self.bet_abstraction).len();
        if !self.dashmap.contains_key(&infoset.history) {
            self.initialize_node_vec(&infoset.history);
        }
        let node = self.get(infoset).unwrap();
        let positive_regrets: SmallVecFloats = node
            .regrets
            .iter()
            .take(node.num_actions)
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let regret_norm: SmallVecFloats = normalize_smallvec(&positive_regrets);
        debug_assert!(regret_norm.len() == node.num_actions);
        debug_assert!(num_actions == node.num_actions);
        regret_norm
    }

    fn initialize_node_vec(&self, history: &ActionHistory) {
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
        let new_node = Node::new(history.next_actions(&self.bet_abstraction).len());
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

    pub fn get_strategy(&self, hole: &[Card], board: &[Card], history: &ActionHistory) -> Strategy {
        let infoset = InfoSet::from_hand(hole, board, history);
        let num_actions = infoset.next_actions(&self.bet_abstraction).len();
        debug_assert!(
            num_actions > 0,
            "No valid next actions for history {}",
            infoset.history
        );
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
        let actions = infoset.next_actions(&self.bet_abstraction);
        let cumulative_strategy = node.cumulative_strategy();
        let sum: f32 = cumulative_strategy.iter().sum();
        // println!("Infoset: {infoset}");
        // println!("Actions: {:?}", actions);
        // println!("Cumulative strategy: {:?}", cumulative_strategy);
        debug_assert!((sum - 1.0).abs() < 0.01);
        for (action, prob) in actions.iter().zip(node.cumulative_strategy().iter()) {
            strategy.insert(action.clone(), prob.clone() as f64);
        }
        let sum: f64 = strategy.values().sum();
        debug_assert!(
            { (sum - 1.0).abs() < 0.01 },
            "Strategy {:?} sums to {}",
            strategy,
            sum,
        );
        strategy
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub regrets: [f32; NUM_ACTIONS],
    pub strategy_sum: [f32; NUM_ACTIONS],
    pub num_actions: usize,
    pub t: u64,
}

impl Node {
    pub fn new(num_actions: usize) -> Node {
        debug_assert!(num_actions > 0);
        Node {
            regrets: [0.0; NUM_ACTIONS],
            strategy_sum: [0.0; NUM_ACTIONS],
            num_actions: num_actions,
            t: 0,
        }
    }

    pub fn cumulative_strategy(&self) -> SmallVecFloats {
        normalize_smallvec(&self.strategy_sum[..self.num_actions])
    }
}
