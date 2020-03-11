extern crate indicatif;
extern crate itertools;
extern crate serde;
extern crate serde_json;
#[macro_use(c)]
extern crate cute;
extern crate rand;
#[macro_use]
extern crate lazy_static;
extern crate bio;
extern crate rayon;

mod card_abstraction;
mod card_utils;
mod tests;
mod trainer;

fn main() {
    trainer::train(10000);
}
