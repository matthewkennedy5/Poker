use rand::{distributions::Distribution, distributions::WeightedIndex};
use std::collections::HashMap;

use crate::card_utils::*;
use crate::trainer_utils::*;

// If a hand's probability is below PROB_CUTOFF in the range, just skip it since it has a negligible
// contribution to the range.
pub const PROB_CUTOFF: f64 = 1e-12;

#[derive(Debug, Clone)]
pub struct Range {
    // This is the full 1326 2 card preflop combinations, not isomorphic
    pub range: HashMap<Vec<Card>, f64>,
}

impl Range {
    pub fn new() -> Range {
        let mut range = HashMap::new();
        let deck = deck();
        for i in 0..deck.len() {
            for j in i+1..deck.len() {
                let hand = vec![deck[i], deck[j]];
                range.insert(hand, 1.0);
            }
        }
        debug_assert!(range.len() == 1326);
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
        let keys: Vec<_> = self.range.keys().cloned().collect();

        for hole in keys {
            let prob = self.range[&hole];
            if prob < PROB_CUTOFF {
                continue;
            }

            let strategy = get_strategy(&hole);
            let p = *strategy
                .get(action)
                .unwrap_or_else(|| panic!("Action {} is not in strategy: {:?}", action, strategy));

            let new_prob = prob * p;
            self.range.insert(hole, new_prob); // Modify the range in place
        }

        self.range = normalize(&self.range);
    }

    pub fn hand_prob(&self, hand: &[Card]) -> f64 {
        let mut sorted: Vec<Card> = hand.to_vec();
        sorted.sort();
        *self
            .range
            .get(&sorted)
            .expect(format!("Hand {} not found in range", cards2str(hand)).as_str())
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
        debug_assert!(board.len() == board_length(history.street));
        let mut opp_range = Range::new();
        let mut history_iter = ActionHistory::new();
        let opponent = 1 - history.player;
        opp_range.remove_blockers(board);
        opp_range.remove_blockers(hole);
        for action in history.get_actions() {
            if history_iter.player == opponent {
                // Update the opponent's range based on their action
                let strategy = |hole: &Vec<Card>| get_strategy(hole, board, &history_iter);
                opp_range.update(&action, strategy);
            }
            history_iter.add(&action);
        }
        opp_range
    }
}
