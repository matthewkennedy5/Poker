use std::fmt;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::collections::HashMap;
use rand::thread_rng;
use rand::prelude::SliceRandom;
use serde::Serialize;
use serde::Deserialize;

const HAND_TABLE_PATH: &str = "products/strengths.json";

// When dealing with poker hands that come up in the game, there is some
// information that doesn't matter. For example, we don't care about the order
// of the flop cards or the hole cards. There is also suit isomorphism, where
// for example a 5-card flush of hearts is essentially the same as a 5-card
// flush of diamonds. This function maps the set of all hands to the much
// smaller set of distinct isomorphic hands.
pub fn archetype(cards: &[Card]) -> Vec<Card> {
//     let mut sorted = Vec::new();
//     // Sort the preflop and flop since order doesn't matter within those streets
//     // let mut cards = cards.to_vec();
//     let mut cards = cards.to_vec();
//     // let (&preflop, &flop) = cards.split_at(2);
//     let mut preflop = (&cards[..2]).to_vec();
//     let mut flop = (&cards[2..5]).to_vec();
//     preflop.sort();
//     flop.sort();
//     sorted = [preflop, flop].concat();
//     let mut suits = CANONICAL_SUIT_ORDER.to_vec();
//     suits.reverse();
//     let mut suit_mapping: HashMap<String, String> = HashMap::new();
//     let mut result = Vec::new();
//     for card in sorted {
//         if suit_mapping.contains_key(&card.suit) {
//             // We've seen this suit before -- use the correct corresponding suit
//             let new_suit = suit_mapping.get(&card.suit).unwrap().to_string();
//             result.push(Card { rank: card.rank, suit: new_suit});
//         } else {
//             // New suit -- choose an isomorphic suit from the standard order
//             let new_suit = suits.pop().unwrap().to_string();
//             suit_mapping.insert(card.suit, new_suit.clone());
//             result.push(Card { rank: card.rank, suit: new_suit});
//         }
//     }
//     // Sort by suit because the suits have changed and there can be redundancies
//     let mut preflop = (&result[..2]).to_vec();
//     let mut flop = (&result[2..5]).to_vec();
//     preflop.sort();
//     flop.sort();
//     result = [preflop, flop].concat();
//     result
    Vec::new()
}

const CLUBS: u8 = 0;
const DIAMONDS: u8 = 1;
const HEARTS: u8 = 2;
const SPADES: u8 = 3;

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
            _ => panic!("bad card string")
        };
        let suit = match &card[1..2] {
            "c" => CLUBS,
            "d" => DIAMONDS,
            "h" => HEARTS,
            "s" => SPADES,
            _ => panic!("bad card string")
        };
        return Card { rank: rank, suit: suit };
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
            _ => panic!("Bad rank value")
        };
        let suit = match self.suit {
            CLUBS => "c",
            DIAMONDS => "d",
            HEARTS => "h",
            SPADES => "s",
            _ => panic!("Bad suit value")
        };
        write!(f, "{}{}", rank, suit)
    }
}

pub fn deck() -> Vec<Card> {
    let mut deck = Vec::new();
    let ranks = std::ops::Range { start: 2, end: 15};
    for rank in ranks {
        for suit in 0..4 {
            deck.push(Card { rank: rank, suit: suit});
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
    bar.set_style(indicatif::ProgressStyle::default_bar()
        .template("[{elapsed_precise}/{eta_precise}] {wide_bar} {pos:>7}/{len:7} {msg}"));
    // make sure the drawing doesn't dominate computation for large n
    bar.set_draw_delta(n / 1000);
    bar
}


// canonical / archetypal hand methods
// thanks to stackoverflow user Daniel Slutzbach: https://stackoverflow.com/a/3831682

// returns true if the given list of ints contains duplicate elements.
fn contains_duplicates(list: &[u8]) -> bool {
    for i in 0..list.len() {
        for j in i+1..list.len() {
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
        let suit1 = &by_suits[i-1];
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

// recursively deals canonical hands of a given length.
fn deal_cards(mut permutations: &mut Vec<Vec<Card>>, n: u32, cards: &[Card]) {
    let mut cards = cards.to_vec();
    if cards.len() as u32 == n {
        permutations.push(cards);
        return
    }
    for card in deck() {
        cards.push(card);
        if is_canonical(&cards, false) {
            deal_cards(&mut permutations, n, &cards);
        }
        cards.pop();
    }
}

// returns all possible canonical hands of length n.
pub fn deal_canonical(n: u32) -> Vec<Vec<Card>> {
    let mut permutations = Vec::new();
    deal_cards(&mut permutations, n, &[]);
    permutations
}

// Writes canonical hands to a file
pub fn write_canonical(n: u32, fname: &str) {
    let hands = deal_canonical(n);
    let mut hands_str = String::new();
    for hand in hands {
        hands_str += &cards2str(&hand);
        hands_str += "\n";
    }

    let mut file = File::create(fname).unwrap();
    file.write_all(hands_str.as_bytes());
}

// Translates the given cards into their equivalent canonical representation.
fn canonical_hand(cards: &[Card], streets: bool) -> Vec<Card> {
    // Separate the cards by suit
    let mut by_suits: Vec<Vec<u8>> = Vec::new();
    for suit in 0..4 {
        let ranks = c![card.rank, for card in cards, if card.suit == suit];
        by_suits.push(ranks.to_vec());
    }

    let mut canonical = Vec::new();
    // Retrieve the suits in size order with lexicographic tie breaking
    for new_suit in 0..4 {
        let mut min = 0;
        for old_suit in 1..4 {
            // The next suit must have the largest length, using lower lexicographic ordering
            // to break ties.
            if (by_suits[old_suit].len() > by_suits[min].len()) {
                min = old_suit;
            } else if (by_suits[old_suit].len() == by_suits[min].len() && by_suits[old_suit] < by_suits[min]) {
                min = old_suit;
            }
        }
        for rank in by_suits[min].clone() {
            canonical.push(Card {rank: rank, suit: new_suit})
        }
        by_suits[min] = vec![];
    }

    // Sort the cards correctly
    if streets && cards.len() > 2 {
        let mut preflop = (&canonical[..2]).to_vec();
        let mut board = (&canonical[2..]).to_vec();
        preflop.sort_by_key(|c| (c.suit.clone(), c.rank));
        board.sort_by_key(|c| (c.suit.clone(), c.rank));
        canonical = [preflop, board].concat();
    } else {
        canonical.sort_by_key(|c| (c.suit.clone(), c.rank));
    }

    if !is_canonical(&canonical, streets) {
        panic!("Not canonical: {:?}", canonical);
    }
    canonical
}

// For fast poker hand comparison, look up relative strength values in a table
pub struct HandTable {
    strengths: HashMap<String, i32>
}

impl HandTable {

    pub fn new() -> HandTable {
        HandTable{ strengths: HandTable::load_hand_strengths() }
    }

    pub fn hand_strength(&self, hand: &[Card]) -> i32 {
        let hand = canonical_hand(&hand, false);
        self.strengths.get(&cards2str(&hand)).unwrap().clone()
    }

    fn load_hand_strengths() -> HashMap<String, i32> {
        match File::open(HAND_TABLE_PATH) {
            Err(e) => panic!("Hand table not found"),
            Ok(mut file) => {
                // Load up the hand table from the JSON
                let mut buffer = String::new();
                file.read_to_string(&mut buffer).expect("Error");
                serde_json::from_str(&buffer).unwrap()
            }
        }
    }
}


