use std::fmt;
use std::collections::HashMap;
use rand::thread_rng;
use rand::prelude::SliceRandom;

// const CANONICAL_SUIT_ORDER: [&str; 4] = ["s", "h", "d", "c"];

const SUITS: [&str; 4] = ["c", "d", "h", "s"];

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

enum Suit {
    CLUBS,
    DIAMONDS,
    HEATS,
    SPADES
}

#[derive(Debug, Clone, PartialOrd, Eq)]
pub struct Card {
    pub rank: u8,
    pub suit: String
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
        let suit = String::from(&card[1..2]);
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
        write!(f, "{}{}", rank, self.suit)
    }
}

pub fn deck() -> Vec<Card> {
    let mut deck = Vec::new();
    let ranks = std::ops::Range { start: 2, end: 15};
    for rank in ranks {
        for suit in &["s", "d", "c", "h"] {
            deck.push(Card { rank: rank, suit: suit.to_string()});
        }
    }
    return deck;
}

pub fn deepcopy(vec: Vec<&Card>) -> Vec<Card> {
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
    if (card1.suit < card2.suit) {
        return false;
    }
    if (card1.suit == card2.suit && card1.rank <= card2.rank) {
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
    let mut by_suits: HashMap<&str, Vec<u8>> = HashMap::new();
    for suit in &SUITS {
        let ranks = c![card.rank, for card in cards, if card.suit == suit.to_string()];
        by_suits.insert(suit, ranks.to_vec());
        if contains_duplicates(&ranks) {
            // duplicate cards have been provided, so this cannot be a real hand
            // rule 4
            return false;
        }

    }
    for i in 1..4 {
        let suit1 = by_suits.get(&SUITS[i-1]).unwrap();
        let suit2 = by_suits.get(&SUITS[i]).unwrap();
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
        if is_canonical(&cards, true) {
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



