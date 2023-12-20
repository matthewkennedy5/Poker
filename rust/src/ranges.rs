use crate::card_utils::*;
use rand::distributions::WeightedIndex;
use rand::prelude::*;

// If a hand's probability is below PROB_CUTOFF in the range, just skip it since it has a negligible
// contribution to the range.
pub const PROB_CUTOFF: f64 = 1e-12;
const N_PREFLOP_HANDS: usize = 1326;

#[derive(Debug, Clone)]
pub struct Range {
    // This is the full 1326 2 card preflop combinations, not isomorphic
    pub hands: Vec<[Card; 2]>,
    pub probs: Vec<f64>,
}

impl Range {
    pub fn new() -> Range {
        let mut hands: Vec<[Card; 2]> = Vec::with_capacity(1326);
        let deck = deck();
        for i in 0..deck.len() {
            for j in i + 1..deck.len() {
                let hand = [deck[i], deck[j]];
                hands.push(hand);
            }
        }
        debug_assert!(hands.len() == N_PREFLOP_HANDS);
        let probs = vec![1.0 / N_PREFLOP_HANDS as f64; N_PREFLOP_HANDS];
        Range {
            hands: hands,
            probs: probs,
        }
    }

    pub fn remove_blockers(&mut self, blockers: &[Card]) {
        // this could be O(n) instead of O(n^2)
        for i in 0..self.hands.len() {
            let hand = self.hands[i];
            if blockers.contains(&hand[0]) || blockers.contains(&hand[1]) {
                self.probs[i] = 0.0;
            }
        }
        self.normalize_range();
    }

    // Performs a Bayesian update of our beliefs about the opponent's range
    pub fn update<F>(&mut self, hand_likelihood: F)
    where
        F: Fn(&[Card]) -> f64,
    {
        for i in 0..self.hands.len() {
            let hand = self.hands[i];
            let prob = self.probs[i];
            if prob < PROB_CUTOFF {
                continue;
            }

            let p = hand_likelihood(&hand);
            let new_prob = prob * p;
            self.probs[i] = new_prob;
        }

        self.normalize_range();
    }

    pub fn sample_hand(&self) -> Vec<Card> {
        let dist = WeightedIndex::new(&self.probs).unwrap();
        let index = dist.sample(&mut rand::thread_rng());
        let hand = self.hands[index];
        hand.to_vec()
    }

    pub fn normalize_range(&mut self) {
        let sum: f64 = self.probs.iter().sum();
        if sum == 0.0 {
            self.probs = vec![1.0 / N_PREFLOP_HANDS as f64; N_PREFLOP_HANDS];
        } else {
            self.probs = self.probs.iter().map(|prob| prob / sum).collect();
        }
    }
}
