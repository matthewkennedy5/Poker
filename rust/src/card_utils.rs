use crate::itertools::Itertools;
use moka::sync::Cache;
use once_cell::sync::Lazy;
use rs_poker::core::{Hand, Rank, Rankable};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, ToSmallVec};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
    time::Duration,
};

const FAST_HAND_TABLE_PATH: &str = "products/fast_strengths.bin";
const FLOP_CANONICAL_PATH: &str = "products/flop_isomorphic.txt";
const TURN_CANONICAL_PATH: &str = "products/turn_isomorphic.txt";
const RIVER_CANONICAL_PATH: &str = "products/river_isomorphic.txt";

pub type SmallVecHand = SmallVec<[Card; 7]>;
type IsomorphicHandCache = Cache<(SmallVecHand, bool), SmallVecHand>;

pub static FAST_HAND_TABLE: Lazy<FastHandTable> = Lazy::new(FastHandTable::new);
static ISOMORPHIC_HAND_CACHE: Lazy<IsomorphicHandCache> = Lazy::new(|| Cache::new(100_000));

pub const CLUBS: i32 = 0;
pub const DIAMONDS: i32 = 1;
pub const HEARTS: i32 = 2;
pub const SPADES: i32 = 3;

#[derive(Hash, Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
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
        Card {
            rank,
            suit: suit as u8,
        }
    }
}

impl Ord for Card {
    // orders first based on rank, and if ranks are equal, then on alphebetical
    // order of the suit
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.rank, self.suit).cmp(&(other.rank, other.suit))
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
        write!(f, "{rank}{suit}")
    }
}

pub fn deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    let ranks = std::ops::Range { start: 2, end: 15 };
    for rank in ranks {
        for suit in 0..4 {
            deck.push(Card { rank, suit });
        }
    }
    deck
}

pub fn deepcopy(vec: &[&Card]) -> Vec<Card> {
    let vec = vec.to_owned();
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
fn sort_isomorphic(cards: &[Card], streets: bool) -> SmallVecHand {
    let mut sorted: SmallVecHand = SmallVec::with_capacity(7);
    if streets && cards.len() > 2 {
        let mut preflop: SmallVecHand = cards[..2].to_smallvec();
        let mut board: SmallVecHand = cards[2..].to_smallvec();
        preflop.sort_unstable_by_key(|c: &Card| (c.suit, c.rank));
        board.sort_unstable_by_key(|c: &Card| (c.suit, c.rank));
        sorted.extend(preflop);
        sorted.extend(board);
    } else {
        sorted = cards.to_smallvec();
        sorted.sort_unstable_by_key(|c| (c.suit, c.rank));
    }
    sorted
}

// https://stackoverflow.com/a/3831682
pub fn isomorphic_hand(cards: &[Card], streets: bool) -> SmallVec<[Card; 7]> {
    let inputs = (cards.to_smallvec(), streets);
    if let Some(iso) = ISOMORPHIC_HAND_CACHE.get(&inputs) {
        return iso;
    }

    let cards = sort_isomorphic(cards, streets);

    let mut by_suits: SmallVec<[SmallVec<[u8; 7]>; 4]> = smallvec![smallvec![0; 7]; 4];
    for card in &cards {
        by_suits[card.suit as usize].push(card.rank);
    }

    let mut suit_indices: SmallVec<[usize; 4]> = (0..4).collect();
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

    let mut isomorphic: SmallVec<[Card; 7]> = cards
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
        let strength = *self
            .strengths
            .get(&compact)
            .unwrap_or_else(|| panic!("{} not in FastHandTable", cards2str(hand)));
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
    ((hand & 0xFF << (8 * card_index)) >> (8 * card_index)) as i32
}

pub fn suit(card: i32) -> i32 {
    card / 15_i32
}

pub fn rank(card: i32) -> i32 {
    card % 15_i32
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
        result += card << (4 * i)
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
        result.push(Card { suit, rank });
    }
    result
}

// Converts the old fashioned Vec<Card> representation into the compact u64
// representation.
pub fn cards2hand(cards: &[Card]) -> u64 {
    let mut result = 0;
    for (i, card) in cards.iter().enumerate() {
        let card = (15 * card.suit + card.rank) as u64;
        result += card << (8 * i);
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
                buffer.write(hand2str(*hand).as_bytes()).unwrap();
                buffer.write(b"\n").unwrap();
            }
            println!("[INFO] Wrote isomorphic hands to {path}.");
        }
    };
    isomorphic
}

pub fn isomorphic_preflop_hands() -> HashSet<Vec<Card>> {
    let mut preflop_hands: HashSet<Vec<Card>> = HashSet::new();
    for i in 2..15 {
        for j in i..15 {
            preflop_hands.insert(vec![Card {rank: i as u8, suit: CLUBS as u8}, Card {rank: j as u8, suit: DIAMONDS as u8}]);
            if i != j {
                preflop_hands.insert(vec![Card {rank: i as u8, suit: CLUBS as u8}, Card {rank: j as u8, suit: CLUBS as u8}]);
            }
        }
    }
    assert_eq!(preflop_hands.len(), 169);
    preflop_hands
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
        for preflop in deck.iter().combinations(2) {
            let mut rest_of_deck = deck.clone();
            rest_of_deck.retain(|c| !preflop.contains(&c));
            for board in rest_of_deck.iter().combinations(n_cards - 2) {
                let cards = [deepcopy(&preflop), deepcopy(&board)].concat();
                let hand = cards2hand(&isomorphic_hand(&cards, true));
                isomorphic.insert(hand);
            }
        }
    } else {
        let hands = deck.iter().combinations(n_cards);
        for hand in hands {
            let cards = deepcopy(&hand);
            let hand = cards2hand(&isomorphic_hand(&cards, false));
            isomorphic.insert(hand);
        }
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
    deck.retain(|c| !hand.contains(c));

    if hand.len() == 7 {
        let equity = river_equity(&hand);
        return equity.powi(2);
    }
    for rollout in deck.iter().combinations(7 - hand.len()) {
        let full_hand = [hand.clone(), deepcopy(&rollout)].concat();
        let equity = river_equity(&full_hand);
        sum += equity.powi(2);
        count += 1.0;
    }
    sum / count
}

fn river_equity(hand: &Vec<Card>) -> f64 {
    let mut deck = deck();
    // Remove the already-dealt cards from the deck
    deck.retain(|c| !hand.contains(c));

    let board = hand[2..].to_vec();
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

    n_wins / (n_runs as f64)
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
