
// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::card_utils;
use crate::card_utils::Card;
use crate::card_utils::deepcopy;
use crate::cluster;
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

const FLOP_BUCKETS: i32 = 100;
const TURN_BUCKETS: i32 = 100;
const RIVER_BUCKETS: i32 = 100;
const EQUITY_BINS: usize = 50;

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
        return bin as i32;
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
    match File::open(FLOP_PATH) {
        Err(_error) => make_abstraction(5, FLOP_BUCKETS),
        Ok(mut file) => {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).expect("Error reading file");
            serde_json::from_str(&buffer).unwrap()
        }
    }
}

fn load_turn_abstraction() -> HashMap<String, i32> {
    match File::open(TURN_PATH) {
        Err(_error) => make_abstraction(6, TURN_BUCKETS),
        Ok(mut file) => {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).expect("Error reading file");
            serde_json::from_str(&buffer).unwrap()
        }
    }
}

fn make_abstraction(n_cards: i32, n_buckets: i32) -> HashMap<String, i32> {
    if n_cards != 5 && n_cards != 6 {
        panic!("Must have 5 or 6 cards for flop or turn abstraction");
    }
    let distributions = make_equity_distributions(n_cards);
    cluster::cluster_ehs2(&distributions, n_buckets)
}

fn make_equity_distributions(n_cards: i32) -> HashMap<String, Vec<f64>> {
    let mut distributions: HashMap<String, Vec<f64>> = HashMap::new();
    let hands = card_utils::deal_canonical(n_cards as u32, true);
    let bar = card_utils::pbar(hands.len() as u64);
    for hand in hands {
        let equity = equity_distribution(&hand);
        let hand_str = card_utils::cards2str(&hand).to_string();
        distributions.insert(hand_str, equity);
        bar.inc(1);
    }
    bar.finish();
    return distributions;
}

fn equity_distribution(cards: &[Card]) -> Vec<f64> {
    let cards = cards.to_vec();
    let mut distribution: Vec<f64> = vec![0.0; EQUITY_BINS];
    let board = (&cards[2..]).to_vec();

    let equity_table = card_utils::EquityTable::new();

    let mut deck = card_utils::deck();
    // Remove the already-dealt cards from the deck
    deck.retain(|c| !cards.contains(&c));

    for rollout in deck.iter().combinations(7 - cards.len()) {
        let rollout = rollout.to_vec();
        let my_hand = [cards.clone(), deepcopy(&rollout)].concat();
        let equity = equity_table.lookup(&my_hand);
        let mut equity_bin = (equity * EQUITY_BINS as f64) as usize;
        if equity_bin == EQUITY_BINS {
            equity_bin -= 1;
        }
        distribution[equity_bin] += 1.0;
    }
    distribution = card_utils::normalize(&distribution);
    return distribution;
}


