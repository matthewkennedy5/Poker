

extern crate serde;
extern crate serde_json;
extern crate itertools;
extern crate indicatif;
#[macro_use(c)]
extern crate cute;
extern crate rand;
#[macro_use]
extern crate lazy_static;
extern crate rayon;
extern crate emd;
#[macro_use(array)]
extern crate ndarray;

mod card_abstraction;
mod card_utils;
mod cluster;
mod trainer;
mod tests;

use card_utils::Card;
use std::fs::File;
use std::io::Write;
use std::mem::size_of_val;
use std::time::Instant;
use std::collections::HashMap;

fn main() {

    let cards = vec![Card::new("2c"), Card::new("2d"), Card::new("2h"),
                     Card::new("3c"), Card::new("3d"), Card::new("3h"), Card::new("4c")];

    let hand = [Card::new("2c"), Card::new("2c"), Card::new("2c"),
                Card::new("2c"), Card::new("2c"), Card::new("2c"), Card::new("2c")];

    // let mut abs: HashMap<&[Card], i32> = HashMap::new();
    // abs.insert(&hand, 0);
    // println!("{}", size_of_val(&abs));
    // println!("{}", size_of_val(&cards) + 7 * size_of_val(&Card::new("2c")));

    // card_utils::canonical_hand(&cards, true);
    let canonical = card_utils::deal_river_canonical();
    // println!("{}", canonical.len());
    // let now = Instant::now();
    // let canonical = card_utils::deal_canonical(5, true);
    // println!("{}", canonical.len());
    // println!("Seconds: {}", now.elapsed().as_secs());

    // for i in 0..100 {
    //     card_utils::EquityTable::river_equity(&cards);
    // }
    // println!("{}", card_utils::EquityTable::river_equity(&cards));

    // let table = card_utils::HandTable::new();
    // println!("{}", table.hand_strength(&cards));

    // let hand = vec![Card::new("2c"), Card::new("3c"), Card::new("2d"), Card::new("4d"),
    //                 Card::new("2h"), Card::new("3h"), Card::new("2s")];
    // let hand = vec![Card::new("2c"), Card::new("2d"), Card::new("3c"), Card::new("2h"),
    //                 Card::new("2s"), Card::new("3h"), Card::new("3s")];
    // println!("Original: {}", card_utils::cards2str(&hand));
    // println!("Canonical: {}", card_utils::cards2str(&card_utils::canonical_hand(&hand, true)));
    // println!("is_canonical: {}", card_utils::is_canonical(&card_utils::canonical_hand(&hand, true), true));
    // let canonical = card_utils::deal_canonical(7, true);
    // println!("in canonical: {}", canonical.contains(&card_utils::canonical_hand(&hand, true)));
    // println!("{}", card_utils::is_canonical(&hand, true));
    // println!("{:?}", card_utils::cards2str(&card_utils::canonical_hand(&hand, true)));
    // let a = card_abstraction::Abstraction::new();
    // let bin = a.abstract_id(&cards);
    // println!("Bin: {}", bin);

    // card_utils::EquityTable::make_equity_table();
    // trainer::train(100);
}


