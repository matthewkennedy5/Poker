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

mod backend;
mod bot;
mod card_abstraction;
mod card_utils;
mod exploiter;
mod tests;
mod trainer;
mod trainer_utils;
mod validation;

use std::fs::File;
use std::io::Write;
use std::mem::size_of_val;
use std::collections::HashMap;

fn main() {
    // backend::main().expect("Could not launch server");

    trainer::train(1_000_000);
    // validation::preflop_matrix();
    // validation::donk_percentage();

    // trainer::train(100_000_000);
    // let nodes = trainer::load_nodes();
    // trainer::view_preflop(&nodes);
    // crate::trainer_utils::write_compact_blueprint(&nodes);
}
