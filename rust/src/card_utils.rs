use crate::itertools::Itertools;
use bio::stats::combinatorics::combinations;
use moka::sync::Cache;
use once_cell::sync::Lazy;
use rs_poker::core::{Hand, Rank, Rankable};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
    thread,
    time::Duration,
};

const FAST_HAND_TABLE_PATH: &str = "products/fast_strengths.bin";
const EQUITY_TABLE_PATH: &str = "products/equity_table.txt";
const FLOP_CANONICAL_PATH: &str = "products/flop_isomorphic.txt";
const TURN_CANONICAL_PATH: &str = "products/turn_isomorphic.txt";
const RIVER_CANONICAL_PATH: &str = "products/river_isomorphic.txt";

pub static FAST_HAND_TABLE: Lazy<FastHandTable> = Lazy::new(|| FastHandTable::new());
static EQUITY_TABLE: Lazy<EquityTable> = Lazy::new(|| EquityTable::new());
static ISOMORPHIC_HAND_CACHE: Lazy<Cache<(Vec<Card>, bool), Vec<Card>>> =
    Lazy::new(|| Cache::new(10_000));

pub const CLUBS: i32 = 0;
pub const DIAMONDS: i32 = 1;
pub const HEARTS: i32 = 2;
pub const SPADES: i32 = 3;

const NUM_THREADS: usize = 100; // Number of threads to use in parallel loops

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
            _ => panic!("bad card string '{}'", card),
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

pub fn rank_str(rank: u8) -> String {
    match rank {
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
        _ => panic!("Bad rank value: {}", rank),
    }
    .to_string()
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rank = rank_str(self.rank);
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
            .template(
                "[{elapsed_precise}/{eta_precise}] {wide_bar} {pos:>7}/{len:7} {per_sec} {msg}",
            )
            .unwrap(),
    );
    bar.enable_steady_tick(Duration::new(0, 1_000_000_000));
    bar
}

// Translates the given cards into their equivalent isomorphic representation.
// When dealing with poker hands that come up in the game, there is some
// information that doesn't matter. For example, we don't care about the order
// of the flop cards or the hole cards. There is also suit isomorphism, where
// for example a 5-card flush of hearts is essentially the same as a 5-card
// flush of diamonds. This function maps the set of all hands to the much
// smaller set of distinct isomorphic hands.
fn sort_isomorphic(cards: &[Card], streets: bool) -> Vec<Card> {
    let mut sorted;
    if streets && cards.len() > 2 {
        let mut preflop = (&cards[..2]).to_vec();
        let mut board = (&cards[2..]).to_vec();
        preflop.sort_unstable_by_key(|c| (c.suit, c.rank));
        board.sort_unstable_by_key(|c| (c.suit, c.rank));
        sorted = [preflop, board].concat();
    } else {
        sorted = cards.to_vec();
        sorted.sort_unstable_by_key(|c| (c.suit, c.rank));
    }
    sorted
}

// https://stackoverflow.com/a/3831682
pub fn isomorphic_hand(cards: &[Card], streets: bool) -> Vec<Card> {
    let inputs = (cards.to_vec(), streets);
    if let Some(iso) = ISOMORPHIC_HAND_CACHE.get(&inputs) {
        return iso;
    }

    let cards = sort_isomorphic(cards, streets);

    let mut by_suits: Vec<Vec<u8>> = vec![Vec::new(); 4];
    for card in &cards {
        by_suits[card.suit as usize].push(card.rank);
    }

    let mut suit_indices: Vec<usize> = (0..4).collect();
    suit_indices.sort_unstable_by(|a, b| {
        let a_len = by_suits[*a].len();
        let b_len = by_suits[*b].len();
        if a_len == b_len {
            by_suits[*a].cmp(&by_suits[*b])
        } else {
            b_len.cmp(&a_len)
        }
    });

    let mut suit_mapping = [0, 0, 0, 0];
    for (new_suit, &old_suit) in suit_indices.iter().enumerate() {
        suit_mapping[old_suit] = new_suit;
    }

    let mut isomorphic: Vec<Card> = cards
        .into_iter()
        .map(|card| Card {
            rank: card.rank,
            suit: suit_mapping[card.suit as usize] as u8,
        })
        .collect();

    isomorphic = sort_isomorphic(&isomorphic, streets);
    ISOMORPHIC_HAND_CACHE.insert(inputs, isomorphic.clone());
    isomorphic
}

pub struct FastHandTable {
    strengths: HashMap<u64, i32>,
}

impl FastHandTable {
    pub fn new() -> FastHandTable {
        FastHandTable {
            strengths: FastHandTable::load_hand_strengths(),
        }
    }

    pub fn hand_strength(&self, hand: &[Card]) -> i32 {
        let compact = cards2bitmap(hand);
        let strength = self
            .strengths
            .get(&compact)
            .expect(&format!("{} not in FastHandTable", compact))
            .clone();
        strength
    }

    fn load_hand_strengths() -> HashMap<u64, i32> {
        if !Path::new(FAST_HAND_TABLE_PATH).exists() {
            println!("[INFO] Creating fast hand table.");
            let mut table: HashMap<u64, i32> = HashMap::new();
            let deck = deck();
            let bar = pbar(133784560);
            for hand in deck.iter().combinations(7) {
                let cards = deepcopy(&hand);
                let strength: i32 = hand_strength(&cards);
                let bitmap = cards2bitmap(&cards);
                table.insert(bitmap, strength);
                bar.inc(1);
            }
            bar.finish();
            serialize(table, FAST_HAND_TABLE_PATH);
        }
        let table: HashMap<u64, i32> = read_serialized(FAST_HAND_TABLE_PATH);
        table
    }
}

// Uses the rs_poker library to evaluate the strength of a hand
pub fn hand_strength(cards: &[Card]) -> i32 {
    let step: i32 = 100_000_000;
    let hand = cards2str(cards);
    let rank = Hand::new_from_str(&hand).unwrap().rank();
    let (rank_val, rank_strength) = match rank {
        Rank::HighCard(strength) => (0, strength),
        Rank::OnePair(strength) => (1, strength),
        Rank::TwoPair(strength) => (2, strength),
        Rank::ThreeOfAKind(strength) => (3, strength),
        Rank::Straight(strength) => (4, strength),
        Rank::Flush(strength) => (5, strength),
        Rank::FullHouse(strength) => (6, strength),
        Rank::FourOfAKind(strength) => (7, strength),
        Rank::StraightFlush(strength) => (8, strength),
    };
    rank_val * step + rank_strength as i32
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

pub fn str2cards(hand_str: &str) -> Vec<Card> {
    let mut result: Vec<Card> = Vec::new();
    let hand_str = hand_str.to_string();
    for i in (0..hand_str.len()).step_by(2) {
        let card = Card::new(&hand_str[i..i + 2]);
        result.push(card);
    }
    result
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
        let rank_number = rank(card(hand, card_index));
        let rank = match rank_number {
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
            _ => panic!("Bad rank value: {}", rank_number),
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

/*
 * Represents hands using 52 bits, one bit for each card. If the card is present
 * in the hand, then the corresponding bit will be 1.
 *
 * | filler     | Clubs       | Diamonds    | Hearts      | Spades      |
 * |            |23456789TJQKA|23456789TJQKA|23456789TJQKA|23456789TJQKA|
 * |000000000000|0000000000000|0000000000000|0000000000000|0000000000000|
 *
 * for a total of 64 bits.
 */
pub fn cards2bitmap(cards: &[Card]) -> u64 {
    let mut bitmap: u64 = 0;
    // Starting with rightmost bit: 1
    // Shift left 52 times to move the bit to the rightmost filler bit
    // Shift right 13 * suits times
    // Shift right rank-1 times because I'm starting with 2 for ranks
    for card in cards {
        let shift = 52 - 13 * card.suit - (card.rank - 1);
        let card_bit: u64 = 1 << shift;
        bitmap += card_bit;
    }
    bitmap
}

pub fn load_flop_isomorphic() -> HashSet<u64> {
    load_isomorphic(5, FLOP_CANONICAL_PATH)
}

pub fn load_turn_isomorphic() -> HashSet<u64> {
    load_isomorphic(6, TURN_CANONICAL_PATH)
}

pub fn load_river_isomorphic() -> HashSet<u64> {
    load_isomorphic(7, RIVER_CANONICAL_PATH)
}

fn load_isomorphic(n_cards: usize, path: &str) -> HashSet<u64> {
    let mut isomorphic = HashSet::new();
    match File::open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                isomorphic.insert(str2hand(&line.unwrap()));
            }
        }
        Err(_e) => {
            // Find the isomorphic hands and write them to disk.
            isomorphic = deal_isomorphic(n_cards, true);
            let mut buffer = File::create(path).unwrap();
            for hand in &isomorphic {
                buffer.write(hand2str(hand.clone()).as_bytes()).unwrap();
                buffer.write(b"\n").unwrap();
            }
            println!("[INFO] Wrote isomorphic hands to {}.", path);
        }
    };
    isomorphic
}

pub fn deal_isomorphic(n_cards: usize, preserve_streets: bool) -> HashSet<u64> {
    match n_cards {
        5 => println!("[INFO] Finding all isomorphic flop hands."),
        6 => println!("[INFO] Finding all isomorphic turn hands."),
        7 => println!("[INFO] Finding all isomorphic river hands."),
        _ => panic!("Bad number of cards"),
    };

    let mut isomorphic: HashSet<u64> = HashSet::new();
    let deck = deck();

    if preserve_streets {
        let bar = pbar((combinations(52, 2) * combinations(50, (n_cards - 2) as u64)) as u64);
        for preflop in deck.iter().combinations(2) {
            let mut rest_of_deck = deck.clone();
            rest_of_deck.retain(|c| !preflop.contains(&c));
            for board in rest_of_deck.iter().combinations(n_cards - 2) {
                let cards = [deepcopy(&preflop), deepcopy(&board)].concat();
                let hand = cards2hand(&isomorphic_hand(&cards, true));
                isomorphic.insert(hand);
                bar.inc(1);
            }
        }
        bar.finish();
    } else {
        let bar = pbar(combinations(52, n_cards as u64) as u64);
        let hands = deck.iter().combinations(n_cards);
        for hand in hands {
            let cards = deepcopy(&hand);
            let hand = cards2hand(&isomorphic_hand(&cards, false));
            isomorphic.insert(hand);
            bar.inc(1);
        }
        bar.finish();
    }
    isomorphic
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

    if hand.len() == 7 {
        let equity = EQUITY_TABLE.lookup(&hand);
        // let equity = river_equity(hand);
        return equity.powi(2);
    }

    for rollout in deck.iter().combinations(7 - hand.len()) {
        let full_hand = [hand.clone(), deepcopy(&rollout)].concat();
        let equity = EQUITY_TABLE.lookup(&full_hand);
        // let equity = river_equity(full_hand);
        sum += equity.powi(2);
        count += 1.0;
    }
    let average = sum / count;
    average
}

fn river_equity(hand: Vec<Card>) -> f64 {
    // fn river_equity(hand: &[Card]) -> f64 {
    let mut deck = deck();
    // Remove the already-dealt cards from the deck
    deck.retain(|c| !hand.contains(&c));

    let board = (&hand[2..]).to_vec();
    let mut n_wins = 0.0;
    let mut n_runs = 0;

    for opp_preflop in deck.iter().combinations(2) {
        n_runs += 1;

        // Create the poker hands by concatenating cards
        let my_hand = hand.to_vec();
        let opp_hand = [deepcopy(&opp_preflop), board.clone()].concat();

        let my_strength = FAST_HAND_TABLE.hand_strength(&my_hand);
        let opp_strength = FAST_HAND_TABLE.hand_strength(&opp_hand);

        if my_strength > opp_strength {
            n_wins += 1.0;
        } else if my_strength == opp_strength {
            n_wins += 0.5;
        }
    }
    let equity = n_wins / (n_runs as f64);
    equity
}

// There are multiple places where I have to serialize a HashMap of cards->i32
// with some sort of data such as hand strength or abstraction ID. This loads
// that data from a file desciptor and returns the lookup table.
pub fn read_serialized(path: &str) -> HashMap<u64, i32> {
    let reader = BufReader::new(File::open(path).unwrap());
    bincode::deserialize_from(reader).unwrap()
}

pub fn serialize(hand_data: HashMap<u64, i32>, path: &str) {
    let buffer = File::create(path).unwrap();
    bincode::serialize_into(buffer, &hand_data).unwrap();
}

struct EquityTable {
    table: HashMap<u64, f64>,
}

impl EquityTable {
    fn new() -> EquityTable {
        match File::open(EQUITY_TABLE_PATH) {
            Err(_e) => {
                let table = EquityTable::create();
                EquityTable { table: table }
            }
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
                EquityTable { table: table }
            }
        }
    }

    fn create() -> HashMap<u64, f64> {
        let isomorphic: Vec<u64> = load_river_isomorphic().iter().map(|x| x.clone()).collect();

        println!("[INFO] Creating the river equity lookup table...");
        let chunk_size = isomorphic.len() / NUM_THREADS;

        let chunks: Vec<Vec<u64>> = isomorphic
            .chunks(chunk_size)
            .map(|s| s.to_owned())
            .collect();
        let mut handles = Vec::new();

        let (tx, rx) = mpsc::channel();

        let (pbar_tx, pbar_rx) = mpsc::channel();

        for chunk in chunks {
            let thread_tx = tx.clone();
            let thread_pbar_tx = pbar_tx.clone();
            handles.push(thread::spawn(move || {
                let equities: Vec<(u64, f64)> = chunk
                    .iter()
                    .map(|h| {
                        thread_pbar_tx
                            .send(1)
                            .expect("could not send pbar increment");
                        (h.clone(), river_equity(hand2cards(h.clone())))
                    })
                    .collect();
                thread_tx.send(equities).expect("Could not send the equity");
            }));
        }

        let bar = pbar(isomorphic.len() as u64);
        for _i in 0..isomorphic.len() {
            let increment = pbar_rx.recv().unwrap();
            bar.inc(increment);
        }
        bar.finish();

        let mut equities: Vec<(u64, f64)> = Vec::new();

        for _i in 0..handles.len() {
            let mut result = rx.recv().expect("Could not receive result");
            equities.append(&mut result);
        }

        for handle in handles {
            handle.join().expect("Could not join the threads");
        }

        let mut table = HashMap::new();

        println!("[INFO] Writing to disk");
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
        let hand = cards2hand(&isomorphic_hand(hand, true));
        self.table
            .get(&hand)
            .expect(&format!("{} not in equity table", hand2str(hand)))
            .clone()
    }
}
