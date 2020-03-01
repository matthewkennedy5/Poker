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
extern crate emd;
extern crate rayon;
extern crate thincollections;

mod card_abstraction;
mod card_utils;
// mod trainer;
mod tests;

use card_utils::*;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::collections::HashMap;

fn main() {
    let cards = vec![
        Card::new("2c"),
        Card::new("2d"),
        Card::new("2h"),
        Card::new("3c"),
        Card::new("3d"),
    ];

    // let mut vector: Vec<(u64, i32)> = Vec::new();
    // for i in 0..130_000_000 {
    //     vector.push((i, (i as i32) / 2));
    // }
    // let vector2 = vector.clone();
    // let vector3 = vector.clone();
    // println!("{}", vector.len());


    let a = card_abstraction::Abstraction::new();
    let bin = a.abstract_id(&cards);
    println!("Bin: {}", bin);
}
