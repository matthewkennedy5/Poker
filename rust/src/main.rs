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
extern crate bio;

mod card_abstraction;
mod card_utils;
// mod trainer;
mod tests;

use card_utils::Card;

fn main() {

    let cards = vec![Card::new("2c"), Card::new("2d"), Card::new("2h"),
                     Card::new("3c"), Card::new("3d")];

    let a = card_abstraction::Abstraction::new();
    let bin = a.abstract_id(&cards);
    println!("Bin: {}", bin);
}
