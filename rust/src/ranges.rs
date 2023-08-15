use rand::{distributions::Distribution, distributions::WeightedIndex};

use crate::card_utils::*;
use crate::trainer_utils::*;
use std::collections::HashMap;

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
        let mut hands: Vec<[Card; 2]> = Vec::new();
        let deck = deck();
        for i in 0..deck.len() {
            for j in i + 1..deck.len() {
                let hand = [deck[i], deck[j]];
                hands.push(hand);
            }
        }
        debug_assert!(hands.len() == N_PREFLOP_HANDS);
        let probs = vec![1.0 / N_PREFLOP_HANDS as f64; N_PREFLOP_HANDS];
        Range { hands: hands, probs: probs }
    }

    pub fn remove_blockers(&mut self, blockers: &[Card]) {
        for i in 0..self.hands.len() {
            let hand = self.hands[i];
            if blockers.contains(&hand[0]) || blockers.contains(&hand[1]) {
                self.probs[i] = 0.0;
            }
        }
        self.normalize_range(); // TODO: Another optimization is that you only need to normalize in getters
    }

    // Performs a Bayesian update of our beliefs about the opponent's range
    pub fn update<F>(&mut self, action: &Action, get_strategy: F)
    where
        F: Fn(&Vec<Card>) -> Strategy,
    {
        for i in 0..self.hands.len() {
            let hand = self.hands[i];
            let prob = self.probs[i];
            if prob < PROB_CUTOFF {
                continue;
            }

            let strategy = get_strategy(&hand.to_vec()); // TODO: Change to SmallVec
            let p = strategy.get(action).expect(&format!(
                "Action {} is not in strategy: {:?}",
                action, strategy
            ));

            let new_prob = prob * p;
            self.probs[i] = new_prob;
        }

        self.normalize_range();
    }

    // pub fn hand_prob(&self, hand: &[Card]) -> f64 {
    //     let mut sorted: Vec<Card> = hand.to_vec();  // TODO: Array
    //     sorted.sort();
    //     *self
    //         .range
    //         .get(&sorted)
    //         .expect(format!("Hand {} not found in range", cards2str(hand)).as_str())
    // }

    pub fn get_map(&self) -> HashMap<Vec<Card>, f64> {
        self.hands
            .iter()
            .zip(self.probs.iter())
            .map(|(hand, prob)| (hand.to_vec(), prob.clone()))
            .collect()
    }

    pub fn sample_hand(&self) -> Vec<Card> {
        let dist = WeightedIndex::new(&self.probs).unwrap();
        let index = dist.sample(&mut rand::thread_rng());
        let hand = self.hands[index];
        hand.to_vec()
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

    pub fn normalize_range(&mut self) {
        let sum: f64 = self.probs.iter().sum();
        if sum == 0.0 {
            self.probs = vec![1.0 / N_PREFLOP_HANDS as f64; N_PREFLOP_HANDS];
        } else {
            self.probs = self.probs.iter().map(|prob| prob / sum).collect();
        }
    }
}
