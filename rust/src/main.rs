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
mod ranges;
mod trainer;
mod trainer_utils;
mod card_abstraction;
mod card_utils;
mod exploiter;
#[cfg(test)]
mod tests;
mod bot;

use card_utils::strvec2cards;

use crate::config::CONFIG;

// TODO: Separate executable targets for server, trainer, and exploiter
fn main() {
    // let bot = bot::Bot::new();
    // exploiter::exploitability(&bot, CONFIG.lbr_iters);
    // trainer::train(CONFIG.train_iters);

    // Load the 27o preflop node to see what it looks like. Why is it raising so often?
    // It should be folding 100% of the time. (Maybe it just needs to train for longer)
    // let nodes = trainer::load_nodes(&CONFIG.nodes_path);
    // let infoset = trainer_utils::InfoSet::from_hand(
    //     &strvec2cards(&["2d", "8d"]),
    //     &Vec::new(),
    //     &trainer_utils::ActionHistory::new()
    // );
    // let node = nodes.get(&infoset.compress()).unwrap();
    // println!("{:#?}", node);
    // println!("\n{:#?}", node.cumulative_strategy());
}
