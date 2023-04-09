use crate::config::CONFIG;
use crate::trainer_utils::*;
use smallvec::SmallVec;
use std::slice::Iter;
use std::sync::RwLock;

// Upper limit on branching factor of blueprint game tree. For setting the SmallVec size.
pub const NUM_ACTIONS: usize = 5;

// To efficiently store the CFR nodes during training, I'm storing them in the game tree, where each
// GameTreeNode corresponds to an ActionHistory. Then each GameTreeNode contains a Vec with all the
// Nodes (1 Node for each card abstraction bucket). This makes get() and insert() a little slower,
// but saves memory by not needing to store the InfoSet key for each Node. For concurrency, there 
// is a RwLock around the game tree. If the lock becomes a bottleneck, we can increase the
// granularity by adding a RwLock for each TreeNode or something. 
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes {
    root: RwLock<GameTreeNode>,
    bet_abstraction: Vec<Vec<f32>>,
}

impl Nodes {
    pub fn new() -> Nodes {
        Nodes {
            root: RwLock::new(GameTreeNode::new(&ActionHistory::new(), &CONFIG.bet_abstraction)),
            bet_abstraction: CONFIG.bet_abstraction.clone(),
        }
    }

    pub fn get(&self, infoset: &InfoSet) -> Option<Node> {
        // The action history must be within the bet abstraction
        assert!(infoset.history == infoset.history.translate(&self.bet_abstraction));
        // Traverse the action history tree to find the set of nodes corresponding to the 
        // infoset's history
        let root = self.root.read().unwrap();
        let mut current_node: &GameTreeNode = &*root;
        let mut current_history = ActionHistory::new();
        for action in infoset.history.get_actions() {
            let next_actions = current_history.next_actions(&self.bet_abstraction);
            let child_index = next_actions.iter().position(|a| a.clone() == action).unwrap();
            current_node = match current_node.children.get(child_index) {
                Some(n) => n.as_ref(),
                None => return None
            };
            current_history.add(&action);
        }
        // Then, lookup the node with the right card bucket
        current_node.nodes.get(infoset.card_bucket as usize).cloned()
    }

    pub fn insert(&self, infoset: InfoSet, node: Node) {
        // Find the GameTreeNode for this ActionHistory, creating it if it doesn't exist
        // The action history must be within the bet abstraction
        assert!(infoset.history == infoset.history.translate(&self.bet_abstraction));
        // Traverse the action history tree to find the set of nodes corresponding to the 
        // infoset's history
        let mut root = self.root.write().unwrap();
        let mut current_node: &mut GameTreeNode = &mut *root;
        let mut current_history = ActionHistory::new();
        for action in infoset.history.get_actions() {
            let next_actions = current_history.next_actions(&self.bet_abstraction);
            let child_index = next_actions.iter().position(|a| a.clone() == action).unwrap();
            current_history.add(&action);
            if current_node.children.is_empty() {
                // If the children aren't initialized yet, intiialize the GameTreeNode children.
                // This will create a bunch of new Nodes because it creates a Node for each card
                // bucket in each GameTreeNode.
                let children: Vec<Box<GameTreeNode>> = next_actions.iter().map(|a| {
                    Box::new(GameTreeNode::new(&current_history, &self.bet_abstraction))
                }).collect();  
                current_node.children = children;
            }
            current_node = current_node.children.get_mut(child_index).unwrap();
        }
        // Insert the node at the card_bucket index in the GameTreeNode's vec
        if infoset.card_bucket >= current_node.nodes.len() as i32 {
            println!("{:?}, {:?}", infoset, infoset.history);
        }
        current_node.nodes[infoset.card_bucket as usize] = node;
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
struct GameTreeNode {
    pub nodes: Vec<Node>,
    pub children: Vec<Box<GameTreeNode>>
}

impl GameTreeNode {
    fn new(history: &ActionHistory, bet_abstraction: &[Vec<f32>]) -> GameTreeNode {
        let new_node = Node::new(history, bet_abstraction);
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
        GameTreeNode {
            nodes: vec![new_node; n_buckets],
            children: Vec::new()
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub regrets: [f32; NUM_ACTIONS],
    strategy_sum: [f32; NUM_ACTIONS],
    // Depending on the action history, there may be fewer than NUM_ACTIONS legal next actions at
    // this spot. In that case, the trailing extra elements of regrets and strategy_sum will just
    // be zeros. actions.len() is the source of truth for the branching factor at this node.
    pub actions: SmallVec<[Action; NUM_ACTIONS]>,   // TODO: See if you can get rid of actions as well
    pub t: f32 
}

impl Node {
    pub fn new(history: &ActionHistory, bet_abstraction: &[Vec<f32>]) -> Node {
        let actions = history.next_actions(bet_abstraction);
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
        let regret_norm: SmallVec<[f32; NUM_ACTIONS]> = normalize_smallvec(&positive_regrets);
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
