// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils::*;
use crate::config::CONFIG;
use crate::rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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
            Abstraction::preflop_bin(&cards)
        } else {
            self.postflop_bin(&cards)
        }
    }

    // Inverse lookup - returns a hand in the given bin. Inverse of preflop_bin
    pub fn preflop_hand(bin: i32) -> String {
        let suited: bool = bin % 2 == 1;
        let rank_bin = (if suited { bin - 1 } else { bin }) / 2;
        let rank1 = rank_bin / 100;
        let rank2 = rank_bin % 100;
        let hand = format!(
            "{}{}{}",
            rank_str(rank1 as u8),
            rank_str(rank2 as u8),
            if suited { "s" } else { "o" }
        );
        return hand;
    }

    fn preflop_bin(cards: &[Card]) -> i32 {
        let mut cards = cards.to_vec();
        cards.sort_by_key(|c| c.rank);
        let rank1 = cards[0].rank as i32;
        let rank2 = cards[1].rank as i32;
        let mut bin = 2 * (rank1 * 100 + rank2);
        if cards[0].suit == cards[1].suit {
            bin += 1;
        }
        assert!(bin >= 404 && bin <= 2829);
        return bin as i32;
    }

    // Lookup methods: Translate the card to its isomorphic version and return
    fn postflop_bin(&self, cards: &[Card]) -> i32 {
        let isomorphic = isomorphic_hand(cards, true);
        let hand = cards2hand(&isomorphic);
        match cards.len() {
            5 => self.flop.get(&hand).unwrap().clone(),
            6 => self.turn.get(&hand).unwrap().clone(),
            7 => self.river.get(&hand).unwrap().clone(),
            _ => panic!("Bad number of cards"),
        }
    }
}

fn load_abstraction(path: &str, n_cards: usize, n_buckets: i32) -> HashMap<u64, i32> {
    match File::open(path) {
        Err(_error) => make_abstraction(n_cards, n_buckets),
        Ok(file) => read_serialized(file),
    }
}

// Returns all isomorphic hands paired with their E[HS^2] values, in sorted order
// by E[HS^2].
fn get_sorted_hand_ehs2(n_cards: usize) -> Vec<(u64, f64)> {
    let isomorphic_hands = match n_cards {
        5 => load_flop_isomorphic(),
        6 => load_turn_isomorphic(),
        7 => load_river_isomorphic(),
        _ => panic!("Bad number of cards"),
    };

    // Cluster the hands based on E[HS^2] percentile bucketing.
    let bar = pbar(isomorphic_hands.len() as u64);
    // Calculate all E[HS^2] values in parallel
    let mut hand_ehs2: Vec<(u64, f64)> = isomorphic_hands
        .par_iter()
        .map(|h| {
            let ehs2 = expected_hs2(h.clone());
            bar.inc(1);
            (h.clone(), ehs2)
        })
        .collect();

    bar.finish();
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
        let bucket: i32 = ((n_buckets as f64) * (idx as f64) / (hand_ehs2.len() as f64)) as i32;
        clusters.insert(hand.clone(), bucket.clone());
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
