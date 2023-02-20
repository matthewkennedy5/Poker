#![allow(dead_code)]
#![allow(unused_variables)]

extern crate indicatif;
extern crate itertools;
extern crate serde;
extern crate serde_json;
#[macro_use(c)]
extern crate cute;
extern crate rand;
extern crate bincode;
extern crate bio;
extern crate qstring;
extern crate rayon;
extern crate actix_web;
extern crate actix_rt;
extern crate actix_files;

mod config;
mod trainer;
mod trainer_utils;
mod card_abstraction;
mod card_utils;
mod exploiter;
#[cfg(test)]
mod tests;
mod backend;
mod bot;

fn main() {
    // trainer::train(10_000);
    let nodes = trainer::load_nodes(&config::CONFIG.nodes_path);
    trainer_utils::preflop_chart(&nodes);
}

fn launch_server() {
    backend::main().expect("Could not launch server");
}
