use crate::card_utils::*;
use crate::config::CONFIG;
use crate::trainer_utils::*;
use dashmap::DashMap;
use std::sync::Mutex;

// Upper limit on branching factor of blueprint game tree.
pub const NUM_ACTIONS: usize = 5;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NodeVec {
    pub num_actions: u8,
    pub nodes: Mutex<Vec<Node>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes {
    pub dashmap: DashMap<ActionHistory, NodeVec>,
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

    pub fn get(&self, infoset: &InfoSet) -> Option<(Node, u8)> {
        let node_vec = match self.dashmap.get(&infoset.history) {
            Some(n) => n,
            None => return None,
        };
        let num_actions = node_vec.num_actions;
        let nodes = node_vec.nodes.lock().unwrap();
        nodes
            .get(infoset.card_bucket as usize)
            .map(|n| (n.clone(), num_actions))
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
        let mut nodes = node_vec.nodes.lock().unwrap();
        for (hand_idx, utility) in action_utility.iter().enumerate() {
            let regret = utility - node_utility[hand_idx];

            let infoset = &infosets[hand_idx];
            let node = &mut nodes[infoset.card_bucket as usize];
            let mut accumulated_regret = node.regrets[action_index] + regret as f32;

            // DCFR
            let t: f32 = node.t as f32;
            if accumulated_regret > 0.0 {
                accumulated_regret *= t.powf(1.5) / (t.powf(1.5) + 1.0);
            } else {
                accumulated_regret *= t.powf(0.5) / (t.powf(0.5) + 1.0);
            }

            node.regrets[action_index] = accumulated_regret;
        }
    }

    pub fn add_regret(&self, infoset: &InfoSet, action_index: usize, regret: f64) {
        let history = infoset.history.clone();
        let node_vec = self.dashmap.get(&history).unwrap();
        let mut nodes = node_vec.nodes.lock().unwrap();
        let node = &mut nodes[infoset.card_bucket as usize];
        let mut accumulated_regret = node.regrets[action_index] + regret as f32;
        // DCFR discounting (consistent with add_regret_vectorized)
        let t: f32 = node.t as f32;
        if accumulated_regret > 0.0 {
            accumulated_regret *= t.powf(1.5) / (t.powf(1.5) + 1.0);
        } else {
            accumulated_regret *= t.powf(0.5) / (t.powf(0.5) + 1.0);
        }
        node.regrets[action_index] = accumulated_regret;
    }

    pub fn update_strategy_sum_vectorized(&self, infosets: &[InfoSet], probs: &[f64]) {
        let history = infosets[0].history.clone();
        let node_vec = self.dashmap.get(&history).unwrap();
        let num_actions = node_vec.num_actions as usize;
        let mut nodes = node_vec.nodes.lock().unwrap();
        for (infoset, &prob) in infosets.iter().zip(probs.iter()) {
            let node = &mut nodes[infoset.card_bucket as usize];
            let positive_regrets: SmallVecFloats = node
                .regrets
                .iter()
                .take(num_actions)
                .map(|r| if *r >= 0.0 { *r } else { 0.0 })
                .collect();
            let current_strategy: SmallVecFloats = normalize_smallvec(&positive_regrets);
            node.t += 1;
            if prob > 0.0 {
                // DCFR strategy sum weighting: discount old strategy sum so that
                // iteration t's contribution is weighted by t^gamma (gamma=2).
                // Equivalent to multiplying old sum by (t/(t+1))^2 before adding.
                let t = node.t as f32;
                let discount = (t / (t + 1.0)).powi(2);
                for i in 0..current_strategy.len() {
                    node.strategy_sum[i] *= discount;
                    node.strategy_sum[i] += current_strategy[i] * prob as f32;
                }
            }
        }
    }

    pub fn update_strategy_sum(&self, infoset: &InfoSet, prob: f32) {
        let history = infoset.history.clone();
        let node_vec = self.dashmap.get(&history).unwrap();
        let num_actions = node_vec.num_actions as usize;
        let mut nodes = node_vec.nodes.lock().unwrap();
        let node = &mut nodes[infoset.card_bucket as usize];
        let positive_regrets: SmallVecFloats = node
            .regrets
            .iter()
            .take(num_actions)
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let current_strategy: SmallVecFloats = normalize_smallvec(&positive_regrets);
        node.t += 1;
        if prob > 0.0 {
            // DCFR strategy sum weighting (gamma=2)
            let t = node.t as f32;
            let discount = (t / (t + 1.0)).powi(2);
            for i in 0..current_strategy.len() {
                node.strategy_sum[i] *= discount;
                node.strategy_sum[i] += current_strategy[i] * prob;
            }
        }
    }

    pub fn reset_strategy_sum(&self, infoset: &InfoSet) {
        let history = infoset.history.clone();
        let node_vec = self.dashmap.get(&history).unwrap();
        let mut nodes = node_vec.nodes.lock().unwrap();
        let node = &mut nodes[infoset.card_bucket as usize];
        node.strategy_sum = [0.0; NUM_ACTIONS];
    }

    pub fn get_current_strategy(&self, infoset: &InfoSet) -> SmallVecFloats {
        if !self.dashmap.contains_key(&infoset.history) {
            self.initialize_node_vec(&infoset.history);
        }
        let node_vec = self.dashmap.get(&infoset.history).unwrap();
        let num_actions = node_vec.num_actions as usize;
        let nodes = node_vec.nodes.lock().unwrap();
        let node = &nodes[infoset.card_bucket as usize];
        let positive_regrets: SmallVecFloats = node
            .regrets
            .iter()
            .take(num_actions)
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        normalize_smallvec(&positive_regrets)
    }

    pub fn get_current_strategy_vectorized(&self, infosets: &[InfoSet]) -> Vec<SmallVecFloats> {
        let history: &ActionHistory = &infosets[0].history;
        if !self.dashmap.contains_key(history) {
            self.initialize_node_vec(history);
        }
        let node_vec_ref = self.dashmap.get(history).unwrap();
        let num_actions = node_vec_ref.num_actions as usize;
        let nodes = node_vec_ref.nodes.lock().unwrap();
        let regret_norms: Vec<SmallVecFloats> = infosets
            .iter()
            .map(|infoset| {
                let node = &nodes[infoset.card_bucket as usize];
                let positive_regrets: SmallVecFloats = node
                    .regrets
                    .iter()
                    .take(num_actions)
                    .map(|&r| if r >= 0.0 { r } else { 0.0 })
                    .collect();
                normalize_smallvec(&positive_regrets)
            })
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
        let num_actions = history.next_actions(&self.bet_abstraction).len();
        let new_nodes: Vec<Node> = (0..n_buckets).map(|_| Node::new()).collect();
        self.dashmap.insert(
            history.clone(),
            NodeVec {
                num_actions: num_actions as u8,
                nodes: Mutex::new(new_nodes),
            },
        );
    }

    pub fn len(&self) -> usize {
        let mut length = 0;
        self.dashmap.iter().for_each(|elem| {
            let node_vec = elem.value();
            let nodes = node_vec.nodes.lock().unwrap();
            length += nodes.len();
        });
        length
    }

    pub fn get_strategy(&self, hole: &[Card], board: &[Card], history: &ActionHistory) -> Strategy {
        let infoset = InfoSet::from_hand(hole, board, history);
        let node_vec = self
            .dashmap
            .get(&infoset.history)
            .expect(&format!("Node not found for infoset {}", &infoset));
        let num_actions = node_vec.num_actions as usize;
        let nodes = node_vec.nodes.lock().unwrap();
        let node = nodes
            .get(infoset.card_bucket as usize)
            .expect(&format!("Bucket not found for infoset {}", &infoset));
        let mut strategy = Strategy::new();
        let actions = infoset.next_actions(&self.bet_abstraction);
        let cumulative_strategy = node.cumulative_strategy(num_actions);
        for (action, prob) in actions.iter().zip(cumulative_strategy.iter()) {
            strategy.insert(action.clone(), *prob as f64);
        }
        strategy
    }

    /// For a given history and action, returns the probability of that action
    /// for each card bucket. Single DashMap lookup instead of per-hand lookups.
    pub fn get_action_prob_by_bucket(
        &self,
        history: &ActionHistory,
        action: &Action,
    ) -> Option<Vec<f64>> {
        let actions = history.next_actions(&self.bet_abstraction);
        let action_idx = actions.iter().position(|a| a == action);
        let node_vec = self.dashmap.get(history)?;
        let num_actions = node_vec.num_actions as usize;
        let nodes = node_vec.nodes.lock().unwrap();
        let probs: Vec<f64> = nodes
            .iter()
            .map(|node| match action_idx {
                Some(idx) => {
                    let strat = node.cumulative_strategy(num_actions);
                    if idx < strat.len() {
                        strat[idx] as f64
                    } else {
                        0.0
                    }
                }
                None => 0.0,
            })
            .collect();
        Some(probs)
    }

    pub fn get_strategy_vectorized(&self, infosets: &[InfoSet]) -> Vec<SmallVecFloats> {
        let history: &ActionHistory = &infosets[0].history;
        let node_vec_ref = self
            .dashmap
            .get(history)
            .expect(format!("History {} not found in blueprint", history).as_str());
        let num_actions = node_vec_ref.num_actions as usize;
        let nodes = node_vec_ref.nodes.lock().unwrap();

        let strategies: Vec<SmallVecFloats> = infosets
            .iter()
            .map(|infoset| {
                let node = &nodes[infoset.card_bucket as usize];
                node.cumulative_strategy(num_actions)
            })
            .collect();
        strategies
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub regrets: [f32; NUM_ACTIONS],
    pub strategy_sum: [f32; NUM_ACTIONS],
    pub t: i32,
}

impl Node {
    pub fn new() -> Node {
        Node {
            regrets: [0.0; NUM_ACTIONS],
            strategy_sum: [0.0; NUM_ACTIONS],
            t: 0,
        }
    }

    pub fn cumulative_strategy(&self, num_actions: usize) -> SmallVecFloats {
        // TODO: Should this round off low probabilities here? < 0.05
        normalize_smallvec(&self.strategy_sum[..num_actions])
    }
}
