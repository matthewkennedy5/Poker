
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

mod card_abstraction;
mod card_utils;
mod tests;

use card_utils::Card;
use std::fs::File;
use std::io::Write;
use std::mem::size_of_val;

fn main() {

    let cards = vec![Card::new("5c"), Card::new("9c"), Card::new("Qc"),
                     Card::new("2d"), Card::new("Ah"), Card::new("2h"), Card::new("As")];

    // for i in 0..100 {
    //     card_utils::EquityTable::river_equity(&cards);
    // }
    // println!("{}", card_utils::EquityTable::river_equity(&cards));

    // let table = card_utils::HandTable::new();
    // println!("{}", table.hand_strength(&cards));
    // let a = card_abstraction::Abstraction::new();
    // let bin = a.abstract_id(&cards);
    // println!("Bin: {}", bin);

    card_utils::EquityTable::make_equity_table();
}


