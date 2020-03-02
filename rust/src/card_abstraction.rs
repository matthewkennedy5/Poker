// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils;
use crate::card_utils::{Card, HandData};
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::io::{BufRead, BufReader};

const FLOP_PATH: &str = "products/flop_abstraction.txt";
const TURN_PATH: &str = "products/turn_abstraction.txt";
const RIVER_PATH: &str = "products/river_abstraction.txt";

const FLOP_BUCKETS: i32 = 10;
const TURN_BUCKETS: i32 = 10;
const RIVER_BUCKETS: i32 = 10;

pub struct Abstraction {
    flop: HandData,
    turn: HandData,
    river: HandData,
}

impl Abstraction {
    pub fn new() -> Abstraction {
        Abstraction {
            flop: load_abstraction(FLOP_PATH, 7, FLOP_BUCKETS),
            turn: load_abstraction(TURN_PATH, 6, TURN_BUCKETS),
            river: load_abstraction(RIVER_PATH, 7, RIVER_BUCKETS),
        }
    }

    pub fn abstract_id(&self, cards: &[Card]) -> i32 {
        if cards.len() == 2 {
            self.preflop_bin(&cards)
        } else {
            self.postflop_bin(&cards)
        }
    }

    fn preflop_bin(&self, cards: &[Card]) -> i32 {
        let mut cards = cards.to_vec();
        cards.sort_by_key(|c| c.rank);
        let rank1 = cards[0].rank;
        let rank2 = cards[1].rank;
        let mut bin = 2 * (rank1 * 100 + rank2);
        if cards[0].suit == cards[1].suit {
            bin += 1;
        }
        return bin as i32;
    }

    // Lookup methods: Translate the card to its canonical version and return
    // the ID stored in the corresponding abstraction lookup table

    fn postflop_bin(&self, cards: &[Card]) -> i32 {
        let canonical = card_utils::canonical_hand(cards, true);
        let hand_str = card_utils::cards2str(&canonical);
        let hand = card_utils::str2hand(&hand_str);
        match cards.len() {
            5 => self.flop.get(&hand).clone(),
            6 => self.turn.get(&hand).clone(),
            7 => self.river.get(&hand).clone(),
            _ => panic!("Bad number of cards"),
        }
    }
}

fn load_abstraction(path: &str, n_cards: usize, n_buckets: i32) -> HandData {
    match File::open(path) {
        Err(_error) => make_abstraction(n_cards, n_buckets),
        Ok(file) => HandData::read_serialized(file),
    }
}

fn make_abstraction(n_cards: usize, n_buckets: i32) -> HandData {
    match n_cards {
        5 => println!("[INFO] Preparing the flop abstraction."),
        6 => println!("[INFO] Preparing the turn abstraction."),
        7 => println!("[INFO] Preparing the river abstraction."),
        _ => panic!("Bad number of cards"),
    };
    let canonical_hands = match n_cards {
        5 => card_utils::load_flop_canonical(),
        6 => card_utils::load_turn_canonical(),
        7 => card_utils::load_river_canonical(),
        _ => panic!("Bad number of cards"),
    };

    // Cluster the hands based on E[HS^2] percentile bucketing.
    let bar = card_utils::pbar(canonical_hands.len() as u64);
    // Calculate all E[HS^2] values in parallel
    let mut hand_ehs2: Vec<(u64, f64)> = canonical_hands
        .par_iter()
        .map(|h| {
            let ehs2 = card_utils::expected_hs2(h.clone());
            bar.inc(1);
            (h.clone(), ehs2)
        })
        .collect();

    bar.finish();
    hand_ehs2.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut clusters = HandData::new();
    for (idx, (hand, _ehs2)) in hand_ehs2.iter().enumerate() {
        // Bucket the hand according to the percentile of its E[HS^2]
        let bucket: i32 = ((n_buckets as f64) * (idx as f64) / (hand_ehs2.len() as f64)) as i32;
        // TODO: Write to file here instead of all at once at the end.
        clusters.insert(hand, bucket);
    }
    let path = match n_cards {
        5 => FLOP_PATH,
        6 => TURN_PATH,
        7 => RIVER_PATH,
        _ => panic!("Bad hand length"),
    };
    clusters.serialize(path);
    clusters
}
