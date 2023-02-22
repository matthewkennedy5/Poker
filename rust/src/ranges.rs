use std::collections::HashMap;
use crate::itertools::Itertools;

use crate::card_utils::*;
use crate::trainer_utils::*;

pub struct Range {
    pub range: HashMap<Vec<Card>, f64>,
}

impl Range {

    pub fn new() -> Range { 
        let mut range = HashMap::new();
        let deck = deck();
        for hand in deck.iter().combinations(2) {
            // I'm not sure if combinations() always returns the cards in a sorted order, but we'll 
            // cross that bridge if we come to it.
            let hand = deepcopy(&hand);
            range.insert(hand, 1.0);
        }
        range = normalize(&range);
        Range {range: range}
    }

    // When new cards come on the table, the opponent's range cannot contain those
    pub fn remove_blockers(&mut self, blockers: &[Card]) { 
        let mut new_range = self.range.clone();
        for hand in self.range.keys() {
            if blockers.contains(&hand[0]) || blockers.contains(&hand[1]) {
                new_range.remove(hand);
            }
        }
        self.range = normalize(&new_range);
    }

    // Performs a Bayesian update of our beliefs about the opponent's range
    pub fn update<F>(&mut self, action: Action, get_strategy: F)
    where F: Fn(&Vec<Card>) -> HashMap<Action, f64> {
        let mut new_range = self.range.clone();
        for (hole, prob) in self.range.clone() {
            // // If the action is not found in the strategy, it has a probability of 0.
            // let new_prob = match strategy.get(&action) {
            //     Some(p) => prob * p,
            //     None => 0.0,
            // };
            let p = get_strategy(&hole).get(&action).expect("Action is not in strategy").clone();
            let new_prob = prob * p;
            new_range.insert(hole.clone(), new_prob);
        }
        self.range = normalize(&new_range);
    }

    pub fn hand_prob(&self, hand: &[Card]) -> f64 { 
        self.range.get(hand).unwrap().clone()
    }

    pub fn get_opponent_range<F>(history: &ActionHistory, board: &[Card], get_strategy: F) -> Range
    where F: Fn(&Vec<Card>) -> HashMap<Action, f64> {
        panic!("Not implemented yet");
    } 
}