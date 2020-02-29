
// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils;
use crate::card_utils::Card;
use crate::card_utils::deepcopy;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use itertools::Itertools;
use rand::thread_rng;
use rand::prelude::SliceRandom;

const FLOP_PATH: &str = "products/flop_abstraction.json";
const TURN_PATH: &str = "products/turn_abstraction.json";
const RIVER_PATH: &str = "products/river_abstraction.json";

const FLOP_BUCKETS: i32 = 100;
const TURN_BUCKETS: i32 = 100;
const RIVER_BUCKETS: i32 = 100;

// flop and turn map card strings such as "As4d8c9h2d" to their corresponding
// abastract bin. Each string key is an archetypal hand, meaning that
// there is just one entry for every equivalent hand. For example, we don't care
// about the order of the flop cards, so we don't need separate entries for
// every permutation.
pub struct Abstraction {
    flop: HashMap<u64, i32>,
    turn: HashMap<u64, i32>,
    river: HashMap<u64, i32>,
}

impl Abstraction {

    pub fn new() -> Abstraction {
        Abstraction {
            flop: load_abstraction(FLOP_PATH, 5, FLOP_BUCKETS),
            turn: load_abstraction(TURN_PATH, 6, TURN_BUCKETS),
            river: load_abstraction(RIVER_PATH, 7, RIVER_BUCKETS)
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
            5 => self.flop.get(&hand).unwrap().clone(),
            6 => self.turn.get(&hand).unwrap().clone(),
            7 => self.river.get(&hand).unwrap().clone(),
            _ => panic!("Bad number of cards")
        }
    }
}

fn load_abstraction(path: &str, n_cards: usize, n_buckets: i32) -> HashMap<u64, i32> {
    match File::open(path) {
        Err(_error) => make_abstraction(n_cards, n_buckets),
        Ok(mut file) => {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).expect("Error reading file");
            serde_json::from_str(&buffer).unwrap()
        }
    }
    // TODO here: translate str hands to u64s
}

fn make_abstraction(n_cards: usize, n_buckets: i32) -> HashMap<u64, i32> {
    if n_cards != 5 && n_cards != 6 && n_cards != 7 {
        panic!("Must have 5 or 6 cards for flop or turn abstraction");
    }
    // Cluster the hands based on E[HS^2] percentile bucketing.
    let canonical_hands = match n_cards {
        5 => card_utils::load_flop_canonical(),
        6 => card_utils::load_turn_canonical(),
        7 => card_utils::load_river_canonical(),
        _ => panic!("Bad number of cards")
    };
    let mut hand_ehs2: Vec<(u64, f64)> = Vec::new();
    for hand in canonical_hands {
        hand_ehs2.push((hand, card_utils::expected_hs2(hand)));
    }
    hand_ehs2.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut clusters = HashMap::new();
    for (idx, (hand, val)) in hand_ehs2.iter().enumerate() {
        // Bucket the hand according to the percentile of its E[HS^2]
        let bucket: i32 = ((n_buckets as f64) * (idx as f64) / (hand_ehs2.len() as f64)) as i32;
        clusters.insert(hand.clone(), bucket);
    }
    clusters
    // TODO: Serialize the abstraction
}
