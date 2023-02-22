use std::collections::{HashMap, HashSet};
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
            let mut hand = deepcopy(&hand);
            range.insert(hand, 1.0);
        }
        range = normalize(&range);
        Range {range: range}
    }

    pub fn remove_blockers(&mut self, blockers: &[Card]) { panic!("Not implemented yet"); }

    // Performs a Bayesian update of our beliefs about the opponent's range
    pub fn update<F>(&mut self, action: Action, probs: F)
    where F: Fn(&Vec<Card>) -> HashMap<Action, f64> {
        panic!("Not implemented yet");    
    }

    pub fn hand_prob(&self, hand: &[Card]) -> f64 { panic!("Not implemented yet"); }

    pub fn get_opponent_range() -> Range { panic!("Not implemented yet"); }

}

// pub fn get_opponent_range(
//     history: &ActionHistory, 
//     board: &[Card], 
//     blueprint: &HashMap<CompactInfoSet, Node>
// ) -> HashMap<Vec<Card>, f64> {
//     panic!("Not implemented yet");
// }

// // Updates the opponent's range given the fact that they made a certain action
// // at a certain infoset. action is not included in history.
// fn update_range(
//     range: &mut HashMap<Vec<Card>, f64>,
//     action: &Action,
//     bot: &Bot,
//     history: &ActionHistory,
//     board: &[Card],
// ) {
//     let mut new_range = range.clone();
//     for (hole, prob) in range.clone() {
//         let strategy = bot.get_strategy(&hole, board, &history);
//         // If the action is not found in the strategy, it has a probability of 0.
//         let new_prob = match strategy.get(action) {
//             Some(p) => prob * p,
//             None => 0.0,
//         };
//         new_range.insert(hole.clone(), new_prob);
//     }
//     new_range = normalize(&new_range);
//     *range = new_range;
// }

// fn construct_opponent_range(deck: &[Card], exploiter: usize) -> HashMap<Vec<Card>, f64> {
//     let mut range = HashMap::new();
//     let exploiter_hole = get_hand(deck, exploiter, PREFLOP);
//     // Remove the exploiter's preflop hand from the range since the opponent
//     // can't have the exploiter's cards
//     let mut deck = deck.to_vec();
//     deck.retain(|c| !exploiter_hole.contains(&c));
//     for hand in deck.iter().combinations(2) {
//         let hand = card_utils::deepcopy(&hand);
//         range.insert(hand, 1.0);
//     }
//     range = normalize(&range);
//     range
// }

// // When new cards come on the table, the opponent's range cannot contain those
// // cards, so we delete them from the range and renormalize.
// fn remove_blockers(
//     opp_range: &mut HashMap<Vec<Card>, f64>,
//     deck: &[Card],
//     exploiter: usize,
//     street: usize,
// ) {
//     let exploiter_hand = get_hand(deck, exploiter, street);
//     let mut new_range = opp_range.clone();
//     for hand in opp_range.keys() {
//         if exploiter_hand.contains(&hand[0]) || exploiter_hand.contains(&hand[1]) {
//             new_range.remove(hand);
//         }
//     }
//     new_range = normalize(&new_range);
//     *opp_range = new_range;
// }
