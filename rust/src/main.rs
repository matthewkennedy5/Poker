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
extern crate bio;

mod card_abstraction;
mod card_utils;
// mod trainer;
mod tests;

use card_utils::Card;
use std::fs::File;
use std::io::Write;
use std::mem::size_of_val;
use std::time::Instant;
use std::collections::HashMap;

fn main() {

    let cards = vec![Card::new("2c"), Card::new("2d"), Card::new("2h"),
                     Card::new("3c"), Card::new("3d")];

    let flop = card_utils::load_flop_canonical();
    let turn = card_utils::load_turn_canonical();
    let river = card_utils::load_river_canonical();
    println!("flop: {}\nturn: {}\nriver: {}", flop.len(), turn.len(), river.len());
    let flop = card_utils::load_flop_canonical();
    let turn = card_utils::load_turn_canonical();
    let river = card_utils::load_river_canonical();
    println!("flop: {}\nturn: {}\nriver: {}", flop.len(), turn.len(), river.len());

    let a = card_abstraction::Abstraction::new();
    let bin = a.abstract_id(&cards);
    println!("Bin: {}", bin);
}
