// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils;
use crate::card_utils::Card;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use itertools::Itertools;
use rand::thread_rng;
use rand::prelude::SliceRandom;

const FLOP_PATH: &str = "products/flop_abstraction.json";
const TURN_PATH: &str = "products/get_turn_abstraction.json";
const FLOP_EQUITY_PATH: &str = "products/flop_equity_distributions.json";
const TURN_EQUITY_PATH: &str = "products/turn_equity_distributions.json";

// flop and turn map card strings such as "As4d8c9h2d" to their corresponding
// abastract bin. Each string key is an archetypal hand, meaning that
// there is just one entry for every equivalent hand. For example, we don't care
// about the order of the flop cards, so we don't need separate entries for
// every permutation.
pub struct Abstraction {
    flop: HashMap<String, i32>,
    turn: HashMap<String, i32>,
}

impl Abstraction {

    pub fn new() -> Abstraction {
        Abstraction {
            flop: load_flop_abstraction(),
            turn: load_turn_abstraction()
        }
    }

    pub fn abstract_id(&self, cards: &[Card]) -> i32 {
        // let cards = card_utils::archetype(cards);
        match cards.len() {
            2 => self.preflop_bin(&cards),
            5 => self.flop_bin(&cards),
            6 => self.turn_bin(&cards),
            7 => self.river_bin(&cards),
            _ => panic!("Bad number of cards"),
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
        return bin;
    }

    fn flop_bin(&self, cards: &[Card]) -> i32 {
        return 0;
    }

    fn turn_bin(&self, cards: &[Card]) -> i32 {
        return 0;
    }

    fn river_bin(&self, cards: &[Card]) -> i32 {
        return 0;
    }
}

fn load_flop_abstraction() -> HashMap<String, i32> {
    let abst = match File::open(FLOP_PATH) {
        Err(_error) => make_flop_abstraction(),
        Ok(mut file) => {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).expect("Error reading file");
            serde_json::from_str(&buffer).unwrap()
        }
    };
    return HashMap::new();
}

fn load_turn_abstraction() -> HashMap<String, i32> {
    return HashMap::new();
}

fn make_flop_abstraction() -> HashMap<String, i32> {
    let distributions = make_flop_equity();
    cluster(distributions)
}

fn make_flop_equity() -> HashMap<String, Vec<f64>> {
    let mut distributions = HashMap::new();
    let bar = card_utils::pbar(311875200);
    let mut n_total = 0;
    let mut n_canon = 0;
    let mut canonical = Vec::new();
    let mut deck = card_utils::deck();
    let mut rng = thread_rng();
    deck.shuffle(&mut rng);

    for hand in deck.iter().permutations(5) {
        // println!("{:?}", hand);
        let hand = card_utils::deepcopy(hand);
        // if hand == card_utils::archetype(hand.as_slice()) {
        n_total += 1;
        if card_utils::is_canonical(&hand) {
            n_canon += 1;
            // println!("{} = {}", card_utils::cards2str(&hand), card_utils::cards2str(&card_utils::archetype(hand.as_slice())));
            let equity = equity_distribution(hand.as_slice());
            // We store hands as strings in the HashMap for their equity distributions
            let hand_str = card_utils::cards2str(hand.as_slice());
            // &distributions.insert(hand_str, equity);
            canonical.push(hand_str.clone());
        }
        bar.inc(1);
    }
    bar.finish();
    println!("{}, {}", n_canon, n_total);
    let mut file = File::create("rust_canonical.txt").unwrap();
    let canonical_str = format!("{:#?}", canonical);
    file.write_all(canonical_str.as_bytes());
    return distributions;
}

fn equity_distribution(cards: &[Card]) -> Vec<f64> {
    return Vec::new();
}

fn cluster(data: HashMap<String, Vec<f64>>) -> HashMap<String, i32> {
    return HashMap::new();
}

