use crate::config::CONFIG;
use crate::trainer_utils::*;
use smallvec::SmallVec;
use dashmap::DashMap;

// Upper limit on branching factor of blueprint game tree. For setting the SmallVec size.
pub const NUM_ACTIONS: usize = 5;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes {
    dashmap: DashMap<ActionHistory, Vec<Node>>,
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
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub regrets: [f32; NUM_ACTIONS],
    strategy_sum: [f32; NUM_ACTIONS],
    pub t: i32,
    num_actions: usize
}

impl Node {
    pub fn new(num_actions: usize) -> Node {
        Node {
            regrets: [0.0; NUM_ACTIONS],
            strategy_sum: [0.0; NUM_ACTIONS],
            t: 0,
            num_actions: num_actions
        }
    }

    // Returns the current strategy for this node, and updates the cumulative strategy
    // as a side effect.
    // Input: prob is the probability of reaching this node
    pub fn current_strategy(&mut self, prob: f32) -> SmallVec<[f32; NUM_ACTIONS]> {
        // Normalize the regrets for this iteration of CFR
        let positive_regrets: SmallVec<[f32; NUM_ACTIONS]> = self
            .regrets
            .iter()
            .take(self.num_actions)
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let regret_norm: SmallVec<[f32; NUM_ACTIONS]> = normalize_smallvec(&positive_regrets);
        for i in 0..regret_norm.len() {
            // Add this action's probability to the cumulative strategy sum using DCFR update rules
            let new_prob = regret_norm[i] * prob;
            self.strategy_sum[i] += new_prob;
            const GAMMA: f32 = 2.0;
            self.strategy_sum[i] *= (self.t as f32 / (self.t as f32 + 1.0)).powf(GAMMA);
        }
        if prob > 0.0 {
            self.t += 1;
        }
        regret_norm
    }

    pub fn cumulative_strategy(&self) -> SmallVec<[f32; NUM_ACTIONS]> {
        normalize_smallvec(&self.strategy_sum[..self.num_actions])
    }

    pub fn add_regret(&mut self, action_index: usize, regret: f32) {
        assert!(action_index < self.num_actions);
        let mut accumulated_regret = self.regrets[action_index] + regret;
        // Update the accumulated regret according to Discounted Counterfactual
        // Regret Minimization rules
        if accumulated_regret >= 0.0 {
            let t_alpha = (self.t as f32).powf(CONFIG.alpha);
            accumulated_regret *= t_alpha / (t_alpha + 1.0);
        } else {
            let t_beta = (self.t as f32).powf(CONFIG.beta);
            accumulated_regret *= t_beta / (t_beta + 1.0);
        }
        self.regrets[action_index] = accumulated_regret;
    }
}
