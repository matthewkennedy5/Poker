// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils;
use crate::card_utils::{Card, HandData};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::collections::HashMap;

const FLOP_PATH: &str = "products/flop_abstraction.txt";
const TURN_PATH: &str = "products/turn_abstraction.txt";
const RIVER_PATH: &str = "products/river_abstraction.txt";

const FLOP_SORTED_PATH: &str = "products/flop_sorted_ehs2.txt";
const TURN_SORTED_PATH: &str = "products/turn_sorted_ehs2.txt";
const RIVER_SORTED_PATH: &str = "products/river_sorted_ehs2.txt";

const EHS2_CUTOFF_PATH: &str = "products/ehs2_cutoffs.json";
const N_FLOP_CANONICAL: i32 =    1_342_562;
const N_TURN_CANONICAL: i32 =   14_403_610;
const N_RIVER_CANONICAL: i32 = 125_756_657;

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

// Writes text files of canonical hands sorted by E[HS^2] from low to high.
pub fn write_sorted_hands() {
    for n_cards in 5..8 {
        let fname = match n_cards {
            5 => FLOP_SORTED_PATH,
            6 => TURN_SORTED_PATH,
            7 => RIVER_SORTED_PATH,
            _ => panic!("Bad number of cards"),
        };
        let hands = get_sorted_hand_ehs2(n_cards);
        let mut buffer = File::create(fname).unwrap();
        for (hand, ehs2) in hands {
            let to_write = format!("{}\n", card_utils::hand2str(hand.clone()));
            buffer.write(to_write.as_bytes()).unwrap();
        }
    }
}

pub struct LightAbstraction {
    ehs2_cutoffs: HashMap<usize, Vec<f64>>,
}

impl LightAbstraction {
    pub fn new() -> LightAbstraction {
        LightAbstraction {
            ehs2_cutoffs: match File::open(EHS2_CUTOFF_PATH) {
                Err(_e) => ehs2_cutoff_values(),
                Ok(mut file) => {
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer).expect("Error");
                    serde_json::from_str(&buffer).unwrap()
                },
            }
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
        let hand_str = card_utils::cards2str(&canonical);
        let hand = card_utils::str2hand(&hand_str);
        let ehs2 = card_utils::expected_hs2(hand);
        let cutoffs = self.ehs2_cutoffs.get(&cards.len()).unwrap();
        let bucket = match cutoffs.binary_search_by(|x| x.partial_cmp(&ehs2).unwrap()) {
            Ok(index) => index - 1,
            Err(index) => index - 1,
        };
        bucket as i32
    }
}

// TODO: Should I consider multiplicity of canonical hands for percentile bucketing?
// Might not be a big deal if bucket sizes vary.

// Writes and returns the E[HS^2] bucket percentile cutoff values by reading
// through the sorted canonical hands file
fn ehs2_cutoff_values() -> HashMap<usize, Vec<f64>> {
    println!("[INFO] Finding bucket cutoff E[HS^2] values");
    let mut values = HashMap::new();
    for n_cards in 5..8 {
        let fname = match n_cards {
            5 => FLOP_SORTED_PATH,
            6 => TURN_SORTED_PATH,
            7 => RIVER_SORTED_PATH,
            _ => panic!("Bad number of cards"),
        };
        let n_buckets = match n_cards {
            5 => FLOP_BUCKETS,
            6 => TURN_BUCKETS,
            7 => RIVER_BUCKETS,
            _ => panic!("Bad number of cards"),
        };
        let n_canonical = match n_cards {
            5 => N_FLOP_CANONICAL,
            6 => N_TURN_CANONICAL,
            7 => N_RIVER_CANONICAL,
            _ => panic!("Bad number of cards"),
        };

        let reader = match File::open(fname) {
            Err(e) => {
                write_sorted_hands();
                panic!("Just wrote sorted hands... re-run program.")
            }
            Ok(file) => BufReader::new(file),
        };
        // Figure out which hands we need to find the E[HS^2] values of
        let cutoff_lines = c![n * (n_canonical / n_buckets), for n in 0..n_buckets];
        let mut cutoffs = Vec::new();
        // Iterate through the file, calculating cutoff E[HS^2] values at regular intervals
        for (i, line) in reader.lines().enumerate() {
            if cutoff_lines.contains(&(i as i32)) {
                let hand = card_utils::str2hand(&line.unwrap());
                let ehs2 = card_utils::expected_hs2(hand);
                cutoffs.push(ehs2);
            }
        }
        values.insert(n_cards, cutoffs);
    }
    let json = serde_json::to_string(&values).unwrap();
    let mut file = File::create(EHS2_CUTOFF_PATH).unwrap();
    file.write_all(json.as_bytes()).unwrap();
    values
}

