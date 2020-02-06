// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use std::collections::HashMap;
use crate::card_utils;
use crate::card_utils::Card;

struct Abstraction {
    flop: HashMap<Vec<String>, i32>,
    turn: HashMap<Vec<String>, i32>
}

pub fn abstract_id(cards: Vec<Card>) -> i32 {
    let cards = card_utils::archetype(cards);
    match cards.len() {
        2 => preflop_bin(cards),
        5 => flop_bin(cards),
        6 => turn_bin(cards),
        7 => river_bin(cards),
        _ => panic!("Bad number of cards")
    }
}

fn preflop_bin(cards: Vec<Card>) -> i32 {
    let mut cards = cards.to_vec();
    cards.sort_by_key(|c| c.rank);
    let rank1 = cards[0].rank;
    let rank2 = cards[1].rank;
    let mut bin = 2 * (rank1 * 100 + rank2);
    if cards[0].suit == cards[1].suit {
        bin += 1;
    }
    return bin;
}

fn flop_bin(cards: Vec<Card>) -> i32 {
    return 0;
}

fn turn_bin(cards: Vec<Card>) -> i32 {
    return 0;
}

fn river_bin(cards: Vec<Card>) -> i32 {
    return 0;
}
