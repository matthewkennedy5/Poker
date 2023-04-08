use crate::trainer_utils::*;
use crate::config::CONFIG;
use smallvec::SmallVec;
use std::slice::Iter;
use std::sync::RwLock;

// Upper limit on branching factor of blueprint game tree. For setting the SmallVec size.
pub const NUM_ACTIONS: usize = 5;  

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes {
    root: RwLock<TreeNode>
}

impl Nodes {
    pub fn new() -> Nodes {
        panic!("Not implemented")
    }

    pub fn get(&self, infoset: &InfoSet) -> Option<Node> {
        panic!("Not implemented")
    }

    pub fn insert(&self, infoset: InfoSet, node: Node) {
        panic!("Not implemented")
    }

    pub fn len(&self) -> usize {
        panic!("Not implemented")
    }

    pub fn iter(&self) -> Iter<Node> {
        panic!("Not implemented")
    }
}

impl Iterator for Nodes {
    type Item = Node;
    fn next(&mut self) -> Option<Self::Item> {
        panic!("Not implemented");
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TreeNode {
    pub nodes: Vec<Node>,
    pub children: Vec<Box<TreeNode>>
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub regrets: [f32; NUM_ACTIONS], 
    strategy_sum: [f32; NUM_ACTIONS],
    // Depending on the action history, there may be fewer than NUM_ACTIONS legal next actions at
    // this spot. In that case, the trailing extra elements of regrets and strategy_sum will just
    // be zeros. actions.len() is the source of truth for the branching factor at this node. 
    pub actions: SmallVec<[Action; NUM_ACTIONS]>,
    // pub children: SmallVec<[usize; NUM_ACTIONS]>,
    pub t: f32,     // 8 bytes
}

impl Node {
    pub fn new(infoset: &InfoSet, bet_abstraction: &[Vec<f32>]) -> Node {
        let actions = infoset.next_actions(bet_abstraction);
        Node {
            regrets: [0.0; NUM_ACTIONS],
            strategy_sum: [0.0; NUM_ACTIONS],
            actions,
            t: 0.0,
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
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let regret_norm: SmallVec<[f32; NUM_ACTIONS]>= normalize_smallvec(&positive_regrets);
        for i in 0..regret_norm.len() {
            // Add this action's probability to the cumulative strategy sum using DCFR+ update rules
            let new_prob = regret_norm[i] * prob;
            let weight = if self.t < 100.0 { 0.0 } else { self.t - 100.0 };
            self.strategy_sum[i] += weight * new_prob;
        }
        if prob > 0.0 {
            self.t += 1.0;
        }
        regret_norm
    }

    pub fn cumulative_strategy(&self) -> SmallVec<[f32; NUM_ACTIONS]> {
        normalize_smallvec(&self.strategy_sum)
    }

    pub fn add_regret(&mut self, action_index: usize, regret: f32) {
        let mut accumulated_regret = self.regrets[action_index] + regret;
        // Update the accumulated regret according to Discounted Counterfactual
        // Regret Minimization rules
        if accumulated_regret >= 0.0 {
            accumulated_regret *= self.t.powf(CONFIG.alpha) / (self.t.powf(CONFIG.alpha) + 1.0);
        } else {
            accumulated_regret *= self.t.powf(CONFIG.beta) / (self.t.powf(CONFIG.beta) + 1.0);
        }
        self.regrets[action_index] = accumulated_regret;
    }
}
