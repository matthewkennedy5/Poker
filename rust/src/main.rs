extern crate serde;
extern crate serde_json;
extern crate itertools;
extern crate indicatif;
#[macro_use(c)]
extern crate cute;
extern crate rand;

mod card_abstraction;
mod card_utils;
mod tests;

use card_utils::Card;

fn main() {
    let cards = vec![Card::new("3c"), Card::new("9c"), Card::new("4d"),
                     Card::new("9h"), Card::new("Ts")];

    let a = card_abstraction::Abstraction::new();
    let bin = a.abstract_id(&cards);
    println!("Bin: {}", bin);
}
