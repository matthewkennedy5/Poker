// Sanity checks for the blueprint strategy.

use crate::bot;
use crate::trainer_utils::*;
use crate::card_utils::{Card, cards2str};

// Displays the preflop strategy matrix for opening / raising.
pub fn preflop_matrix() {
    let dealer_history = ActionHistory::new();
    let mut p2_history = ActionHistory::new();
    p2_history.add(&Action {action: ActionType::Bet, amount: 250});
    let mut hands = Vec::new();
    for rank in 2..15 {
        for rank2 in rank..15 {
            let off_suit = vec![Card{ suit: 0, rank: rank}, Card{ suit: 1, rank: rank2}];
            hands.push(off_suit);
            if rank != rank2 {
                let suited = vec![Card{ suit: 0, rank: rank}, Card{ suit: 0, rank: rank2}];
                hands.push(suited);
            }
        }
    }
    println!("Dealer's opening strategy: ");
    for hand in &hands {
        let action = bot::bot_action(&hand, &vec![], &dealer_history);
        println!("{}: {}", cards2str(&hand), action);
    }
    println!("\nBig Blind's response to raise of 250:");
    for hand in &hands {
        let action = bot::bot_action(&hand, &vec![], &p2_history);
        println!("{}: {}", cards2str(&hand), action);
    }
    // TODO: Output a graphical preflop chart
}

// Calculates the average donk bet percentage across all hands on the flop.
pub fn donk_percentage() {

}

