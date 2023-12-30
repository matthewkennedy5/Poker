use crate::card_utils::*;
use crate::config::CONFIG;
use crate::trainer_utils::*;
use dashmap::DashMap;
use std::sync::Mutex;

// Upper limit on branching factor of blueprint game tree.
pub const NUM_ACTIONS: usize = 4;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes {
    pub dashmap: DashMap<ActionHistory, Vec<Mutex<Node>>>,
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
        let node_mutex = match nodes.value().get(infoset.card_bucket as usize) {
            Some(mutex) => mutex,
            None => return None,
        };
        let node_guard = node_mutex.lock().unwrap();
        Some(node_guard.clone())
    }

    pub fn add_regret_vectorized(
        &self,
        infosets: &[InfoSet],
        action_utility: &[f64],
        node_utility: &[f64],
        action_index: usize,
    ) {
        let history = infosets[0].history.clone();
        let node_vec = self.dashmap.get(&history).unwrap();
        for (hand_idx, utility) in action_utility.iter().enumerate() {
            let regret = utility - node_utility[hand_idx];

            let infoset = &infosets[hand_idx];
            let node_mutex = node_vec.get(infoset.card_bucket as usize).unwrap();
            let mut node = node_mutex.lock().unwrap();
            let accumulated_regret = node.regrets[action_index] + regret as f32;
            node.regrets[action_index] = accumulated_regret;
        }
    }

    pub fn update_strategy_sum_vectorized(&self, infosets: &[InfoSet], probs: &[f64]) {
        let history = infosets[0].history.clone();
        let node_vec = self.dashmap.get(&history).unwrap();
        for (infoset, &prob) in infosets.iter().zip(probs.iter()) {
            let node_mutex = node_vec.get(infoset.card_bucket as usize).unwrap();
            let mut node = node_mutex.lock().unwrap();
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
                    node.strategy_sum[i] += current_strategy[i] * prob as f32;
                }
            }
        }
    }

    pub fn update_strategy_sum(&self, infoset: &InfoSet, prob: f32) {
        let history = infoset.history.clone();
        let node_vec = self.dashmap.get(&history).unwrap();
        let node_mutex = node_vec.get(infoset.card_bucket as usize).unwrap();
        let mut node = node_mutex.lock().unwrap();
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
    }

    pub fn reset_strategy_sum(&self, infoset: &InfoSet) {
        let history = infoset.history.clone();
        let node_vec = self.dashmap.get(&history).unwrap();
        let node_mutex = node_vec.get(infoset.card_bucket as usize).unwrap();
        let mut node = node_mutex.lock().unwrap();
        node.strategy_sum = [0.0; NUM_ACTIONS];
    }

    pub fn get_current_strategy(&self, infoset: &InfoSet) -> SmallVecFloats {
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
        regret_norm
    }

    pub fn get_current_strategy_vectorized(&self, infosets: &[InfoSet]) -> Vec<SmallVecFloats> {
        let history: &ActionHistory = &infosets[0].history;
        if !self.dashmap.contains_key(history) {
            self.initialize_node_vec(history);
        }
        let node_vec_ref = self.dashmap.get(history).unwrap();
        let node_vec = node_vec_ref.value();
        let positive_regrets: Vec<[f32; NUM_ACTIONS]> = infosets
            .iter()
            .map(|infoset| {
                let mut r = node_vec
                    .get(infoset.card_bucket as usize)
                    .unwrap()
                    .lock()
                    .unwrap()
                    .regrets;
                for i in 0..r.len() {
                    if r[i] < 0.0 {
                        r[i] = 0.0;
                    }
                }
                r
            })
            .collect();

        let regret_norms: Vec<SmallVecFloats> = positive_regrets
            .iter()
            .map(|r| normalize_smallvec(r))
            .collect();
        regret_norms
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
        let new_node: Node = Node::new(history.next_actions(&self.bet_abstraction).len());
        let new_mutex_nodes: Vec<Mutex<Node>> = (0..n_buckets)
            .map(|i| Mutex::new(new_node.clone()))
            .collect();
        self.dashmap.insert(history.clone(), new_mutex_nodes);
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
        let num_actions = infoset.next_actions(&self.bet_abstraction).len();
        debug_assert!(
            num_actions > 0,
            "No valid next actions for history {}",
            infoset.history
        );
        let node = self.get(&infoset).expect("Node not found").clone(); // All nodes must be in infoset
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
        debug_assert!((sum - 1.0).abs() < 0.01);
        for (action, prob) in actions.iter().zip(node.cumulative_strategy().iter()) {
            strategy.insert(action.clone(), *prob as f64);
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
    pub num_actions: usize, // TODO: Can you delete num_actions? and just go based on nonzero values
}

impl Node {
    pub fn new(num_actions: usize) -> Node {
        debug_assert!(num_actions > 0 && num_actions <= NUM_ACTIONS);
        Node {
            regrets: [0.0; NUM_ACTIONS],
            strategy_sum: [0.0; NUM_ACTIONS],
            num_actions,
        }
    }

    pub fn cumulative_strategy(&self) -> SmallVecFloats {
        normalize_smallvec(&self.strategy_sum[..self.num_actions])
    }
}
