#![allow(dead_code)]
#![allow(unused_variables)]

extern crate itertools;
extern crate indicatif;
extern crate rayon;
#[macro_use(c)]
extern crate cute;

mod bot;
mod card_utils;
mod trainer_utils;
mod ranges;
mod trainer;
mod config;
mod exploiter;
mod card_abstraction;
mod backend;

fn main() {
    backend::main().expect("Could not launch server");
}