use crate::itertools::Itertools;
use crate::rand::prelude::IteratorRandom;
use bio::stats::combinatorics::combinations;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::io::{BufRead, BufReader};

const HAND_TABLE_PATH: &str = "products/strengths7.txt";
const EQUITY_TABLE_PATH: &str = "products/equity_table.txt";
const FLOP_CANONICAL_PATH: &str = "products/flop_canonical.txt";
const TURN_CANONICAL_PATH: &str = "products/turn_canonical.txt";
const RIVER_CANONICAL_PATH: &str = "products/river_canonical.txt";

// TODO: To reduce memory usage if needed, incorporate the equity information
// and hand strength information in one big lookup table, like HashMap<u64, (f64, i32)>
lazy_static! {
    pub static ref HAND_TABLE: HandTable = HandTable::new();
    static ref EQUITY_TABLE: EquityTable = EquityTable::new();
}

pub const CLUBS: i32 = 0;
pub const DIAMONDS: i32 = 1;
pub const HEARTS: i32 = 2;
pub const SPADES: i32 = 3;

#[derive(Hash, Debug, Clone, PartialOrd, Eq, Serialize, Deserialize)]
pub struct Card {
    pub rank: u8,
    pub suit: u8,
}

impl Card {
    pub fn new(card: &str) -> Card {
        let rank = match &card[0..1] {
            "2" => 2,
            "3" => 3,
            "4" => 4,
            "5" => 5,
            "6" => 6,
            "7" => 7,
            "8" => 8,
            "9" => 9,
            "T" => 10,
            "J" => 11,
            "Q" => 12,
            "K" => 13,
            "A" => 14,
            _ => panic!("bad card string"),
        };
        let suit = match &card[1..2] {
            "c" => CLUBS,
            "d" => DIAMONDS,
            "h" => HEARTS,
            "s" => SPADES,
            _ => panic!("bad card string"),
        };
        return Card {
            rank: rank,
            suit: suit as u8,
        };
    }
}

impl PartialEq<Card> for Card {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank && self.suit == other.suit
    }
}

impl Ord for Card {
    // orders first based on rank, and if ranks are equal, then on alphebetical
    // order of the suit
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.rank, self.suit.clone()).cmp(&(other.rank, other.suit.clone()))
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rank = match self.rank {
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            10 => "T",
            11 => "J",
            12 => "Q",
            13 => "K",
            14 => "A",
            _ => panic!("Bad rank value"),
        };
        let suit = match self.suit as i32 {
            CLUBS => "c",
            DIAMONDS => "d",
            HEARTS => "h",
            SPADES => "s",
            _ => panic!("Bad suit value"),
        };
        write!(f, "{}{}", rank, suit)
    }
}

pub fn deck() -> Vec<Card> {
    let mut deck = Vec::new();
    let ranks = std::ops::Range { start: 2, end: 15 };
    for rank in ranks {
        for suit in 0..4 {
            deck.push(Card {
                rank: rank,
                suit: suit,
            });
        }
    }
    return deck;
}

pub fn deepcopy(vec: &Vec<&Card>) -> Vec<Card> {
    let vec = vec.clone();
    let mut result: Vec<Card> = Vec::new();
    for v in vec {
        result.push(v.clone());
    }
    result
}

pub fn cards2str(cards: &[Card]) -> String {
    let mut result = String::from("");
    for card in cards {
        result.push_str(&card.to_string());
    }
    result
}

pub fn strvec2cards(strvec: &[&str]) -> Vec<Card> {
    let mut cardvec = Vec::new();
    for card in strvec {
        cardvec.push(Card::new(card));
    }
    cardvec
}

pub fn pbar(n: u64) -> indicatif::ProgressBar {
    let bar = indicatif::ProgressBar::new(n);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}/{eta_precise}] {wide_bar} {pos:>7}/{len:7} {msg}"),
    );
    // make sure the drawing doesn't dominate computation for large n
    bar.set_draw_delta(n / 100_000);
    bar
}

// canonical / archetypal hand methods
// thanks to stackoverflow user Daniel Slutzbach: https://stackoverflow.com/a/3831682

// returns true if the given list of ints contains duplicate elements.
fn contains_duplicates(list: &[u8]) -> bool {
    for i in 0..list.len() {
        for j in i + 1..list.len() {
            if &list[i] == &list[j] {
                return true;
            }
        }
    }
    false
}

fn suit_first_greater_than(card1: &Card, card2: &Card) -> bool {
    if card1.suit < card2.suit {
        return false;
    }
    if card1.suit == card2.suit && card1.rank <= card2.rank {
        return false;
    }
    return true;
}

fn sorted_correctly(cards: &[Card], streets: bool) -> bool {
    if streets {
        if cards.len() >= 2 && suit_first_greater_than(&cards[0], &cards[1]) {
            // preflop is out of order
            return false;
        }
        if cards.len() > 2 {
            // Check that the board cards are sorted relative to each other
            let board = &cards[2..].to_vec();
            let mut sorted_board = board.clone();
            sorted_board.sort_by_key(|c| (c.suit.clone(), c.rank));
            return board == &sorted_board;
        }
        // 1 or 0 cards, always sorted correctly
        return true;
    } else {
        // make sure that all the cards are sorted since we aren't looking at streets
        let mut sorted_cards = cards.to_vec();
        sorted_cards.sort_by_key(|c| (c.suit.clone(), c.rank));
        return sorted_cards == cards;
    }
}

// Given a list of cards, returns true if they are canonical. the rules for being
// canonical are as follows:
// 1. cards must be in sorted order (suit-first, alphabetic order of suits)
// 2. each suit must have at least as many cards as later suits
// 3. when two suits have the same number of cards, the first suit must have
//    lower or equal ranks lexicographically, eg ([1, 5] < [2, 4])
// 4. no duplicate cards
// Thanks again to StackOverflow user Daniel Stutzbach for thinking of these rules.
//
// Inputs:
// cards - array of card instances representing the hand
// streets - whether to distinguish cards from different streets. meaning,
//    if the first two cards represent the preflop and the last three are the,
//    flop, that asad|jh9c2s is different than as2s|jh9cad. both hands have a
//    pair of aces, but in the first hand, the player has pocket aces on the
//    preflop and is in a much stronger position than the second hand.
pub fn is_canonical(cards: &[Card], streets: bool) -> bool {
    if !sorted_correctly(cards, streets) {
        // rule 1
        return false;
    }
    // by_suits is a different way of representing the hand -- it maps suits to
    // the ranks present for that suit
    let mut by_suits: Vec<Vec<u8>> = Vec::new();
    for suit in 0..4 {
        let ranks = c![card.rank, for card in cards, if card.suit == suit];
        by_suits.push(ranks.to_vec());
        if contains_duplicates(&ranks) {
            // duplicate cards have been provided, so this cannot be a real hand
            // rule 4
            return false;
        }
    }
    for i in 1..4 {
        let suit1 = &by_suits[i - 1];
        let suit2 = &by_suits[i];
        if suit1.len() < suit2.len() {
            // rule 2
            return false;
        }
        if suit1.len() == suit2.len() && suit1 > suit2 {
            // rule 3. the ranks of the cards are compared for lexicographic ordering.
            return false;
        }
    }
    true
}

fn sort_canonical(cards: &[Card], streets: bool) -> Vec<Card> {
    let mut sorted;
    if streets && cards.len() > 2 {
        let mut preflop = (&cards[..2]).to_vec();
        let mut board = (&cards[2..]).to_vec();
        preflop.sort_by_key(|c| (c.suit.clone(), c.rank));
        board.sort_by_key(|c| (c.suit.clone(), c.rank));
        sorted = [preflop, board].concat();
    } else {
        sorted = cards.to_vec();
        sorted.sort_by_key(|c| (c.suit.clone(), c.rank));
    }
    sorted
}

// Translates the given cards into their equivalent canonical representation.
// When dealing with poker hands that come up in the game, there is some
// information that doesn't matter. For example, we don't care about the order
// of the flop cards or the hole cards. There is also suit isomorphism, where
// for example a 5-card flush of hearts is essentially the same as a 5-card
// flush of diamonds. This function maps the set of all hands to the much
// smaller set of distinct isomorphic hands.
pub fn canonical_hand(cards: &[Card], streets: bool) -> Vec<Card> {
    let cards_copy = cards.clone();
    let cards = &sort_canonical(&cards, streets);
    // Separate the cards by suit
    let mut by_suits: Vec<Vec<u8>> = Vec::new();
    for suit in 0..4 {
        let ranks = c![card.rank, for card in cards, if card.suit == suit];
        by_suits.push(ranks.to_vec());
    }

    // Define a mapping from old suits to new suits. suit_mapping[old_suit] = new_suit.
    let mut suit_mapping = [0, 0, 0, 0];

    // Retrieve the suits in size order with lexicographic tie breaking

    let mut unused_suits = vec![0, 1, 2, 3];
    for new_suit in 0..4 {
        let mut max = unused_suits[0];
        for old_suit in &unused_suits {
            let old_suit = *old_suit as usize;
            // The next suit must have the largest length, using lower lexicographic ordering
            // to break ties.
            if by_suits[old_suit].len() > by_suits[max].len() {
                max = old_suit;
            } else if by_suits[old_suit].len() == by_suits[max].len()
                && by_suits[old_suit] < by_suits[max]
            {
                max = old_suit;
            }
        }
        suit_mapping[max] = new_suit;
        // Wipe the current suit in by_suits so it doesn't get used twice
        unused_suits.retain(|s| s != &max);
    }
    let mut canonical = Vec::new();
    for card in cards {
        canonical.push(Card {
            rank: card.rank,
            suit: suit_mapping[card.suit as usize],
        });
    }
    canonical = sort_canonical(&canonical, streets);

    // TODO: Remove once I'm convinced it's working
    // if !is_canonical(&canonical, streets) {
    //     panic!("Not canonical: {}\nOriginal: {}", cards2str(&canonical), cards2str(&cards_copy));
    // }

    canonical
}

// For fast poker hand comparison, look up relative strength values in a table
pub struct HandTable {
    strengths: HandData,
}

impl HandTable {
    pub fn new() -> HandTable {
        HandTable {
            strengths: HandTable::load_hand_strengths(),
        }
    }

    pub fn hand_strength(&self, hand: &[Card]) -> i32 {
        let canonical = canonical_hand(&hand, false);
        let compact = cards2hand(&canonical);
        let strength = self.strengths.get(&compact).clone();
        strength
    }

    fn load_hand_strengths() -> HandData {
        match File::open(HAND_TABLE_PATH) {
            Err(_e) => panic!("Hand table not found"),
            Ok(file) => HandData::read_serialized(file),
        }
    }
}

// Writes a file containing all canonical river hand strengths. This can be used
// if you want to convert 5-card lookup table to a 7-card lookup table for a
// lookup speed boost. I wish I had more RAM.
fn bootstrap_river_strengths() {
    let canonical = load_river_canonical();
    let mut buffer = File::create("products/strengths7.txt").unwrap();
    let bar = pbar(canonical.len() as u64);
    for hand in canonical {
        let strength = HAND_TABLE.hand_strength(&hand2cards(hand));
        let to_write = format!("{} {}\n", hand2str(hand.clone()), strength);
        buffer.write(to_write.as_bytes());
        bar.inc(1);
    }
    bar.finish();
}

// Normalize a vector so that its elements sum to 1.
pub fn normalize(vector: &Vec<f64>) -> Vec<f64> {
    let mut sum = 0.0;
    for elem in vector {
        sum += elem;
    }
    let mut noramlized = Vec::new();
    for elem in vector {
        noramlized.push(elem / sum);
    }
    noramlized
}

// u64 hand representation
// Each card is a single u8 byte, where
//
//      suit = card / 15   (integer division)
//      rank = card % 15
//
// I chose 15 so that ranks can have their true value, ie a 7c has rank 7.
// The cards are stored as bytes from right to left in the u64, which is
// right-padded by zeros. If no card is present, the value will be zero, which
// is how the length is determined. This byte representation allows for a
// small memory footprint without needing to use Rust's lifetime parameters.

pub fn card(hand: u64, card_index: i32) -> i32 {
    ((hand & 0xFF << 8 * card_index) >> 8 * card_index) as i32
}

pub fn suit(card: i32) -> i32 {
    card / 15 as i32
}

pub fn rank(card: i32) -> i32 {
    card % 15 as i32
}

pub fn len(hand: u64) -> i32 {
    for n in 0..8 {
        if 256_u64.pow(n) > hand {
            return n as i32;
        }
    }
    -1
}

pub fn str2hand(hand_str: &str) -> u64 {
    let mut result: u64 = 0;
    let hand_str = hand_str.to_string();
    for i in (0..hand_str.len()).step_by(2) {
        let rank = match &hand_str[i..i + 1] {
            "2" => 2,
            "3" => 3,
            "4" => 4,
            "5" => 5,
            "6" => 6,
            "7" => 7,
            "8" => 8,
            "9" => 9,
            "T" => 10,
            "J" => 11,
            "Q" => 12,
            "K" => 13,
            "A" => 14,
            _ => panic!("bad card string"),
        };
        let suit = match &hand_str[i + 1..i + 2] {
            "c" => CLUBS,
            "d" => DIAMONDS,
            "h" => HEARTS,
            "s" => SPADES,
            _ => panic!("bad card string"),
        };
        let card = (15 * suit + rank) as u64;
        result += card << 4 * i
    }
    result
}

pub fn hand2str(hand: u64) -> String {
    let mut hand_str = String::new();
    for card_index in 0..len(hand) {
        let rank = match rank(card(hand, card_index)) {
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            10 => "T",
            11 => "J",
            12 => "Q",
            13 => "K",
            14 => "A",
            _ => panic!("Bad rank value"),
        };
        let suit = match suit(card(hand, card_index)) {
            CLUBS => "c",
            DIAMONDS => "d",
            HEARTS => "h",
            SPADES => "s",
            _ => panic!("Bad suit value"),
        };
        hand_str.push_str(rank);
        hand_str.push_str(suit);
    }
    hand_str
}

// Converts the compact u64 hand representation to the old-fashioned vector of
// Card instances.
pub fn hand2cards(hand: u64) -> Vec<Card> {
    let mut result = Vec::new();
    for i in 0..len(hand) {
        let suit = suit(card(hand, i)) as u8;
        let rank = rank(card(hand, i)) as u8;
        result.push(Card {
            suit: suit,
            rank: rank,
        });
    }
    result
}

// Converts the old fashioned Vec<Card> representation into the compact u64
// representation.
pub fn cards2hand(cards: &[Card]) -> u64 {
    let mut result = 0;
    for (i, card) in cards.iter().enumerate() {
        let card = (15 * card.suit + card.rank) as u64;
        result += card << 8 * i;
    }
    result
}

pub fn load_flop_canonical() -> HashSet<u64> {
    load_canonical(5, FLOP_CANONICAL_PATH)
}

pub fn load_turn_canonical() -> HashSet<u64> {
    load_canonical(6, TURN_CANONICAL_PATH)
}

pub fn load_river_canonical() -> HashSet<u64> {
    println!("[INFO] Loading canonical river hands.");
    let canonical = load_canonical(7, RIVER_CANONICAL_PATH);
    println!("[INFO] Done.");
    canonical
}

fn load_canonical(n_cards: usize, path: &str) -> HashSet<u64> {
    let mut canonical = HashSet::new();
    match File::open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                canonical.insert(str2hand(&line.unwrap()));
            }
        }
        Err(_e) => {
            // Find the canonical hands and write them to disk.
            canonical = deal_canonical(n_cards);
            let mut buffer = File::create(path).unwrap();
            for hand in &canonical {
                buffer.write(hand2str(hand.clone()).as_bytes()).unwrap();
                buffer.write(b"\n").unwrap();
            }
            println!("[INFO] Wrote canonical hands to {}.", path);
        }
    };
    canonical
}

fn deal_canonical(n_cards: usize) -> HashSet<u64> {
    match n_cards {
        5 => println!("[INFO] Finding all canonical flop hands."),
        6 => println!("[INFO] Finding all canonical turn hands."),
        7 => println!("[INFO] Finding all canonical river hands."),
        _ => panic!("Bad number of cards"),
    };

    let mut canonical: HashSet<u64> = HashSet::new();
    let deck = deck();
    let bar = pbar((combinations(52, 2) * combinations(50, (n_cards - 2) as u64)) as u64);
    for preflop in deck.iter().combinations(2) {
        let mut subdeck = deck.clone();
        subdeck.retain(|c| !preflop.contains(&c));
        for board in subdeck.iter().combinations(n_cards - 2) {
            let hand = [deepcopy(&preflop), deepcopy(&board)].concat();
            let hand_str = cards2str(&canonical_hand(&hand, true));
            let hand = str2hand(&hand_str);
            canonical.insert(hand);
            bar.inc(1);
        }
    }
    bar.finish();
    canonical
}

// Returns the second moment of the hand's equity distribution.
pub fn expected_hs2(hand: u64) -> f64 {
    // For river hands, this just returns HS^2 since there is no distribution
    // Flop and turn, deals rollouts for the E[HS^2] value.
    let hand = hand2cards(hand);
    let mut sum = 0.0;
    let mut count = 0.0;
    let mut deck = deck();
    deck.retain(|c| !hand.contains(&c));
    let mut rng = &mut rand::thread_rng();

    if hand.len() == 7 {
        let equity = EQUITY_TABLE.lookup(&hand);
        return equity.powi(2);
    }

    for rollout in deck
        .iter()
        .combinations(7 - hand.len())
    {
        let full_hand = [hand.clone(), deepcopy(&rollout)].concat();
        let equity = EQUITY_TABLE.lookup(&full_hand);
        sum += equity.powi(2);
        count += 1.0;
    }
    let average = sum / count;
    average
}

fn river_equity(hand: &[Card]) -> f64 {
    let mut deck = deck();
    // Remove the already-dealt cards from the deck
    deck.retain(|c| !hand.contains(&c));

    let board = (&hand[2..]).to_vec();
    let mut n_wins = 0.0;
    let mut n_runs = 0;

    let mut rng = &mut rand::thread_rng();

    for opp_preflop in deck.iter().combinations(2)
    {
        n_runs += 1;

        // Create the poker hands by concatenating cards
        let my_hand = hand.to_vec();
        let opp_hand = [deepcopy(&opp_preflop), board.clone()].concat();

        let my_strength = HAND_TABLE.hand_strength(&my_hand);
        let opp_strength = HAND_TABLE.hand_strength(&opp_hand);

        if my_strength > opp_strength {
            n_wins += 1.0;
        } else if my_strength == opp_strength {
            n_wins += 0.5;
        }
    }
    let equity = n_wins / (n_runs as f64);
    equity
}

// START HERE: Do a new branch for use with larger-RAM machines (Intel, GCloud) and

// For many applications (abstraction, hand strength, equity lookup) I need to
// be able to store and lookup an integer corresponding to each hand
pub struct HandData {
    data: HashMap<u64, i32>,
}

impl HandData {
    pub fn new() -> HandData {
        HandData {
            data: HashMap::new(),
        }
    }

    pub fn get(&self, hand: &u64) -> i32 {
        self.data.get(hand).unwrap().clone()
    }

    pub fn insert(&mut self, hand: &u64, data: i32) {
        self.data.insert(hand.clone(), data);
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    // There are multiple places where I have to serialize a HashMap of cards->i32
    // with some sort of data such as hand strength or abstraction ID. This loads
    // that data from a file desciptor and returns the HashMap lookup table.
    pub fn read_serialized(file: File) -> HandData {
        let mut table = HandData::new();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line_str = line.unwrap();
            let mut data = line_str.split_whitespace();
            let hand = data.next().unwrap();
            let bucket = data.next().unwrap();
            let hand = str2hand(hand);
            let bucket = bucket.to_string().parse().unwrap();
            table.insert(&hand, bucket);
        }
        table
    }

    pub fn serialize(&self, path: &str) {
        let mut buffer = File::create(path).unwrap();
        for (hand, data) in &self.data {
            let to_write = format!("{} {}\n", hand2str(hand.clone()), data);
            buffer.write(to_write.as_bytes()).unwrap();
        }
    }
}


struct EquityTable {
    table: HashMap<u64, f64>,
}

impl EquityTable {

    fn new() -> EquityTable {
        match File::open(EQUITY_TABLE_PATH) {
            Err(_e) => {
                let table = EquityTable::create();
                EquityTable{ table: table}
            },
            Ok(file) => {
                // Read from the file
                println!("[INFO] Loading the equity lookup table.");
                let mut table = HashMap::new();
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line_str = line.unwrap();
                    let mut data = line_str.split_whitespace();
                    let hand = data.next().unwrap();
                    let equity = data.next().unwrap();
                    let hand = str2hand(hand);
                    let equity: f64 = equity.to_string().parse().unwrap();
                    table.insert(hand, equity);
                }
                println!("[INFO] Done loading the equity lookup table.");
                EquityTable{ table: table}
            }
        }
    }

    fn create() -> HashMap<u64, f64> {
        println!("[INFO] Creating the river equity lookup table...");
        let canonical = load_river_canonical();
        let bar = pbar(canonical.len() as u64);
        let equities: Vec<(u64, f64)> = canonical
            .par_iter()
            .map(|h| {
                let equity = river_equity(&hand2cards(h.clone()));
                bar.inc(1);
                (h.clone(), equity)
            })
            .collect();

        bar.finish();
        let mut table = HashMap::new();
        // Serialize the equity table and construct the HashMap to return
        let mut buffer = File::create(EQUITY_TABLE_PATH).unwrap();
        for (hand, equity) in &equities {
            let to_write = format!("{} {}\n", hand2str(hand.clone()), equity);
            buffer.write(to_write.as_bytes()).unwrap();
            table.insert(hand.clone(), equity.clone());
        }
        println!("[INFO] Done creating the river equity lookup table.");
        table
    }

    pub fn lookup(&self, hand: &[Card]) -> f64 {
        let hand = cards2hand(&canonical_hand(hand, true));
        self.table.get(&hand).unwrap().clone()
    }

}
