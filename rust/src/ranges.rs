use crate::itertools::Itertools;
use rand::{prelude::SliceRandom, thread_rng, distributions::WeightedIndex, distributions::Distribution};
use std::collections::HashMap;

use crate::card_utils::*;
use crate::trainer_utils::*;

// If a hand's probability is below PROB_CUTOFF in the range, just skip it since it has a negligible
// contribution to the range.
pub const PROB_CUTOFF: f64 = 0.0001;

#[derive(Debug, Clone)]
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
        Range { range }
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
    pub fn update<F>(&mut self, action: &Action, get_strategy: F)
    where
        F: Fn(&Vec<Card>) -> Strategy,
    {
        let mut new_range = self.range.clone();
        for (hole, prob) in self.range.clone() {
            if prob < PROB_CUTOFF {
                continue;
            }
            let p = *get_strategy(&hole)
                .get(action)
                .unwrap_or_else(|| panic!("Action {} is not in strategy: {:?}",
                    action,
                    get_strategy(&hole)));
            let new_prob = prob * p;
            new_range.insert(hole.clone(), new_prob);
        }
        self.range = normalize(&new_range);
    }

    pub fn hand_prob(&self, hand: &[Card]) -> f64 {
        *self.range.get(hand).unwrap()
    }

    pub fn sample_hand(&self) -> Vec<Card> {
        let weights: Vec<f64> = self.range.values().cloned().collect();
        let hands: Vec<&Vec<Card>> = self.range.keys().collect();
        let dist = WeightedIndex::new(&weights).unwrap();
        let hand = hands[dist.sample(&mut rand::thread_rng())];
        hand.clone()
    }

    // In this function, it's our turn and we're trying to figure out the range of the opponent
    // (who just made the last action).
    pub fn get_opponent_range<F>(
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
        get_strategy: F,
    ) -> Range
    where
        F: Fn(&[Card], &[Card], &ActionHistory) -> Strategy,
    {
        let mut opp_range = Range::new();
        let mut history_iter = ActionHistory::new();
        // By Bayes's rule, since the multiplication commutes, we can zero out the probabilties for
        // the blocker cards up front.
        opp_range.remove_blockers(board);
        opp_range.remove_blockers(hole);
        for action in history.get_actions() {
            if history_iter.player == history.player {
                // If the opponent just made an action, then update their range based on their action
                let strategy = |hole: &Vec<Card>| get_strategy(hole, board, &history_iter);
                opp_range.update(&action, strategy);
            }
            history_iter.add(&action);
        }
        opp_range
    }
}
