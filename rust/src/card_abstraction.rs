// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils;
use crate::card_utils::{Card, HandData};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::io::{BufRead, BufReader, Read, Write};

const FLOP_PATH: &str = "products/flop_abstraction.txt";
const TURN_PATH: &str = "products/turn_abstraction.txt";
const RIVER_PATH: &str = "products/river_abstraction.txt";
const RIVER_SORTED_DIR: &str = "products/river_sorted_ehs2";

const N_FLOP_CANONICAL: i32 = 1_342_562;
const N_TURN_CANONICAL: i32 = 14_403_610;
const N_RIVER_CANONICAL: i32 = 125_756_657;

const FLOP_BUCKETS: i32 = 1000;
const TURN_BUCKETS: i32 = 1000;
const RIVER_BUCKETS: i32 = 1000;

pub struct Abstraction {
    flop: HandData,
    turn: HandData,
    river: HandData,
}

impl Abstraction {
    pub fn new() -> Abstraction {
        Abstraction {
            flop: load_abstraction(FLOP_PATH, 5, FLOP_BUCKETS),
            turn: load_abstraction(TURN_PATH, 6, TURN_BUCKETS),
            river: load_abstraction(RIVER_PATH, 7, RIVER_BUCKETS),
        }
    }

    pub fn bin(&self, cards: &[Card]) -> i32 {
        if cards.len() == 2 {
            Abstraction::preflop_bin(&cards)
        } else {
            self.postflop_bin(&cards)
        }
    }

    fn preflop_bin(cards: &[Card]) -> i32 {
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
        let hand = card_utils::cards2hand(&canonical);
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

// Returns all canonical hands paired with their E[HS^2] values, in sorted order
// by E[HS^2].
fn get_sorted_hand_ehs2(n_cards: usize) -> Vec<(u64, f64)> {
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
    hand_ehs2.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    hand_ehs2
}

// TODO: Store the E[HS^2] values themselves instead of the abstract buckets.
// That way I only have to ever calculate them once, and can just re-bucket
// whenever.
fn make_abstraction(n_cards: usize, n_buckets: i32) -> HandData {
    match n_cards {
        5 => println!("[INFO] Preparing the flop abstraction."),
        6 => println!("[INFO] Preparing the turn abstraction."),
        7 => println!("[INFO] Preparing the river abstraction."),
        _ => panic!("Bad number of cards"),
    };
    let hand_ehs2 = get_sorted_hand_ehs2(n_cards);
    let mut clusters = HandData::new();
    for (idx, (hand, _ehs2)) in hand_ehs2.iter().enumerate() {
        // Bucket the hand according to the percentile of its E[HS^2]
        let bucket: i32 = ((n_buckets as f64) * (idx as f64) / (hand_ehs2.len() as f64)) as i32;
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

// Writes text files of canonical hands sorted by E[HS^2] from low to high, split
// into different files depending on the first card in the canonical hand.
pub fn write_sorted_hands() {
    let hands = get_sorted_hand_ehs2(7);
    println!("[INFO] Writing sorted river hands for the LightAbstraction");
    fs::create_dir(RIVER_SORTED_DIR).expect("Couldn't create river directory");
    let bar = card_utils::pbar(hands.len() as u64);
    for card in card_utils::deck() {
        // We find every canonical river hand that starts with card, and add it
        // to this text file in order of E[HS^2].
        let fname = format!("{}/{}.txt", RIVER_SORTED_DIR, card);
        let mut buffer = match OpenOptions::new().append(true).open(&fname) {
            Err(_e) => File::create(fname).expect("Could not create file"),
            Ok(f) => f,
        };
        let mut index = 0;
        for (hand, ehs2) in &hands {
            // let first_card = card_utils::card(hand.clone(), 0);
            // if card_utils::suit(first_card) as u8 == card.suit && card_utils::rank(first_card) as u8 == card.rank {
            let hand_str = card_utils::hand2str(hand.clone());
            let first_card = &hand_str[0..2];
            if first_card == card.to_string() {
                let to_write = format!("{} {}\n", hand_str, index);
                buffer.write(to_write.as_bytes()).unwrap();
                bar.inc(1);
            }
            index += 1;
        }
    }
    bar.finish();
}

// The LightAbstraction is a slower verison of Abstraction that uses way less
// memory because it reads the river abstraction from disk rather than keeping
// it in memory, which will allow us to run the LightAbstraction on normal laptops.
pub struct LightAbstraction {
    flop: HandData,
    turn: HandData,
}

impl LightAbstraction {
    pub fn new() -> LightAbstraction {
        LightAbstraction {
            flop: load_abstraction(FLOP_PATH, 5, FLOP_BUCKETS),
            turn: load_abstraction(TURN_PATH, 6, TURN_BUCKETS),
        }
    }

    pub fn bin(&self, cards: &[Card]) -> i32 {
        if cards.len() == 2 {
            Abstraction::preflop_bin(&cards)
        } else {
            self.postflop_bin(&cards)
        }
    }

    fn postflop_bin(&self, cards: &[Card]) -> i32 {
        let canonical = card_utils::canonical_hand(cards, true);
        let hand = card_utils::cards2hand(&canonical);
        // look up hand number from 1 to 125 million or whatever, bucket it based on that
        match cards.len() {
            5 => self.flop.get(&hand).clone(),
            6 => self.turn.get(&hand).clone(),
            7 => {
                let index = hand_lookup(&canonical).expect("hand not found");
                let bin =
                    ((index as f64) / (N_RIVER_CANONICAL as f64) * RIVER_BUCKETS as f64) as i32;
                bin
            }
            _ => panic!("Bad number of cards"),
        }
    }
}

fn hand_lookup(cards: &[Card]) -> Result<i32, ErrorKind> {
    let target_hand = card_utils::cards2hand(cards);
    let first_card_str = cards[0].to_string();
    let path = format!("{}/{}.txt", RIVER_SORTED_DIR, first_card_str);
    match File::open(path) {
        Err(_e) => {
            write_sorted_hands();
            panic!("Run again");
        }
        Ok(f) => {
            let reader = BufReader::new(f);
            for line in reader.lines() {
                let line_str = line.unwrap();
                let mut data = line_str.split_whitespace();
                let hand = data.next().unwrap();
                let index: i32 = data.next().unwrap().to_string().parse().unwrap();
                let hand = card_utils::str2hand(&hand);
                if hand == target_hand {
                    return Ok(index);
                }
            }
        }
    }
    Err(ErrorKind::NotFound)
}

// TODO: Should I consider multiplicity of canonical hands for percentile bucketing?
// Might not be a big deal if bucket sizes vary.
