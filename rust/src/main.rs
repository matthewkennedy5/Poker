#![allow(dead_code)]
#![allow(unused_variables)]

extern crate indicatif;
extern crate itertools;
extern crate serde;
extern crate serde_json;
#[macro_use(c)]
extern crate cute;
extern crate rand;
#[macro_use]
extern crate lazy_static;
extern crate bincode;
extern crate bio;
extern crate qstring;
extern crate rayon;

mod tests;
mod trainer;
mod trainer_utils;
mod card_abstraction;
mod card_utils;
mod exploiter;

fn main() {

    card_utils::benchmark_hand_lookup_evaluator();
}
