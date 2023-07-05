// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils::*;
use crate::config::CONFIG;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use itertools::Itertools;
use std::{collections::HashMap, fs::File, path::Path, io::BufReader};

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

// Returns all isomorphic hands in sorted order by E[HS^2]
fn get_sorted_hand_ehs2(n_cards: usize) -> Vec<u64> {
    let path = format!("products/ehs2_{n_cards}.bin");
    if Path::new(&path).exists() {
        let file = File::open(path.as_str()).unwrap();
        let reader = BufReader::new(file);
        let ehs2: Vec<u64> = bincode::deserialize_from(reader).unwrap();
        return ehs2;
    }
    
    let isomorphic_hands = match n_cards {
        5 => load_flop_isomorphic(),
        6 => load_turn_isomorphic(),
        7 => load_river_isomorphic(),
        _ => panic!("Bad number of cards"),
    };

    // Cluster the hands based on E[HS^2] percentile bucketing.
    // Calculate all E[HS^2] values in parallel
    let bar = pbar(isomorphic_hands.len() as u64);
    let mut hand_ehs2: Vec<(u64, f64)> = isomorphic_hands
        .par_iter()
        .map(|h| {
            let ehs2 = expected_hs2(*h);
            bar.inc(1);
            (*h, ehs2)
        })
        .collect();
    bar.finish_with_message("Done");

    hand_ehs2.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let sorted_hands: Vec<u64> = hand_ehs2.iter().map(|(hand, ehs2)| *hand).collect();

    let buffer = File::create(path.as_str()).unwrap();
    bincode::serialize_into(buffer, &sorted_hands).unwrap();
    sorted_hands
}

pub fn get_hand_counts(n_cards: usize) -> HashMap<u64, i32> {

    let path = format!("products/hand_counts_{n_cards}.bin");

    if Path::new(&path).exists() {
        let hand_counts = read_serialized(&path);
        return hand_counts;
    }

    println!("[INFO] Getting {n_cards} hand counts...");
    let deck = deck();
    let mut hand_counts: HashMap<u64, i32> = HashMap::new();
    let bar = pbar(match n_cards {
        5 => 25989600,
        6 => 305377800,
        7 => 2_809_475_760,
        _ => 0,
    });
    for preflop in deck.iter().combinations(2) {
        let mut rest_of_deck = deck.clone();
        rest_of_deck.retain(|c| !preflop.contains(&c));
        for board in rest_of_deck.iter().combinations(n_cards - 2) {
            let cards = [deepcopy(&preflop), deepcopy(&board)].concat();
            let hand = cards2hand(&isomorphic_hand(&cards, true));
            let current_count: i32 = match hand_counts.get(&hand) {
                Some(count) => count.clone(),
                None => 0
            };
            hand_counts.insert(hand, current_count + 1);
            bar.inc(1);
        }
    }
    bar.finish();

    serialize(hand_counts.clone(), path.as_str());
    hand_counts
}

fn make_abstraction(n_cards: usize, n_buckets: i32) -> HashMap<u64, i32> {
    match n_cards {
        5 => println!("[INFO] Preparing the flop abstraction."),
        6 => println!("[INFO] Preparing the turn abstraction."),
        7 => println!("[INFO] Preparing the river abstraction."),
        _ => panic!("Bad number of cards"),
    };
    let hand_ehs2 = get_sorted_hand_ehs2(n_cards);
    let hand_counts = get_hand_counts(n_cards);
    let total_hands: u64 = match n_cards {
        5 => 25989600,
        6 => 305377800,
        7 => 2_809_475_760,
        _ => 0,
    };
    let mut clusters = HashMap::new();
    let mut sum: u64 = 0;
    for hand in hand_ehs2 {
        // Bucket the hand according to the percentile of its E[HS^2]
        let count = hand_counts.get(&hand).unwrap().clone() as u64;
        let bucket: i32 = ((n_buckets as f64) * (sum as f64) / (total_hands as f64)) as i32;
        sum += count;
        clusters.insert(hand, bucket);
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
