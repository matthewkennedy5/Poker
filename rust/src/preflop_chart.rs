#![allow(dead_code)]
#![allow(unused_variables)]

extern crate itertools;
extern crate rayon;
extern crate indicatif;
#[macro_use(c)]
extern crate cute;

mod bot;
mod config;
mod trainer;
mod trainer_utils;
mod card_abstraction;
mod card_utils;
mod exploiter;
mod ranges;

fn main() {
    // TODO: Use bot instead of nodes here
    let nodes = trainer::load_nodes(&config::CONFIG.nodes_path);
    trainer_utils::write_preflop_strategy(&nodes, &config::CONFIG.preflop_strategy_path);
}
