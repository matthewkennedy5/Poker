// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils::*;
use crate::config::CONFIG;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{collections::HashMap, fs::File};

const FLOP_PATH: &str = "products/flop_abstraction.bin";
const TURN_PATH: &str = "products/turn_abstraction.bin";
const RIVER_PATH: &str = "products/river_abstraction.bin";

pub struct Abstraction {
    flop: HashMap<u64, i32>,
    turn: HashMap<u64, i32>,
    river: HashMap<u64, i32>,
}

impl Abstraction {
    pub fn new() -> Abstraction {
        Abstraction {
            flop: load_abstraction(FLOP_PATH, 5, CONFIG.flop_buckets),
            turn: load_abstraction(TURN_PATH, 6, CONFIG.turn_buckets),
            river: load_abstraction(RIVER_PATH, 7, CONFIG.river_buckets),
        }
    }

    pub fn bin(&self, cards: &[Card]) -> i32 {
        if cards.len() == 2 {
            Abstraction::preflop_bin(cards)
        } else {
            self.postflop_bin(cards)
        }
    }

    // Map each possible preflop hand to an integer in (0..169)
    fn preflop_bin(cards: &[Card]) -> i32 {
        let mut cards = cards.to_vec();
        cards.sort_by_key(|c| c.rank);
        // Ranks start at 2, so shift it to start at 0
        let rank1: i32 = cards[0].rank as i32 - 2;
        let rank2: i32 = cards[1].rank as i32 - 2;
        let bin = if cards[0].suit == cards[1].suit {
            rank1 * 13 + rank2
        } else {
            rank2 * 13 + rank1
        };
        bin
    }

    // Lookup methods: Translate the card to its isomorphic version and return
    fn postflop_bin(&self, cards: &[Card]) -> i32 {
        let isomorphic = isomorphic_hand(cards, true);
        let hand = cards2hand(&isomorphic);
        match cards.len() {
            5 => *self.flop.get(&hand).unwrap(),
            6 => *self.turn.get(&hand).unwrap(),
            7 => *self.river.get(&hand).unwrap(),
            _ => panic!("Bad number of cards"),
        }
    }
}

fn load_abstraction(path: &str, n_cards: usize, n_buckets: i32) -> HashMap<u64, i32> {
    match File::open(path) {
        Err(_error) => make_abstraction(n_cards, n_buckets),
        Ok(_) => read_serialized(path),
    }
}

// Returns all isomorphic hands paired with their E[HS^2] values, in sorted order
// by E[HS^2].
fn get_sorted_hand_ehs2(n_cards: usize) -> Vec<(u64, f32)> {
    let isomorphic_hands = match n_cards {
        5 => load_flop_isomorphic(),
        6 => load_turn_isomorphic(),
        7 => load_river_isomorphic(),
        _ => panic!("Bad number of cards"),
    };

    // Cluster the hands based on E[HS^2] percentile bucketing.
    // Calculate all E[HS^2] values in parallel
    let bar = pbar(isomorphic_hands.len() as u64);
    let mut hand_ehs2: Vec<(u64, f32)> = isomorphic_hands
        .par_iter()
        .map(|h| {
            let ehs2 = expected_hs2(*h);
            bar.inc(1);
            (*h, ehs2)
        })
        .collect();
    bar.finish_with_message("Done");

    hand_ehs2.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    hand_ehs2
}

fn make_abstraction(n_cards: usize, n_buckets: i32) -> HashMap<u64, i32> {
    match n_cards {
        5 => println!("[INFO] Preparing the flop abstraction."),
        6 => println!("[INFO] Preparing the turn abstraction."),
        7 => println!("[INFO] Preparing the river abstraction."),
        _ => panic!("Bad number of cards"),
    };
    let hand_ehs2 = get_sorted_hand_ehs2(n_cards);
    let mut clusters = HashMap::new();
    for (idx, (hand, _ehs2)) in hand_ehs2.iter().enumerate() {
        // Bucket the hand according to the percentile of its E[HS^2]
        let bucket: i32 = ((n_buckets as f32) * (idx as f32) / (hand_ehs2.len() as f32)) as i32;
        clusters.insert(*hand, bucket);
    }
    let path = match n_cards {
        5 => FLOP_PATH,
        6 => TURN_PATH,
        7 => RIVER_PATH,
        _ => panic!("Bad hand length"),
    };
    serialize(clusters.clone(), path);
    clusters
}
