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
mod backend;
mod bot;

use crate::config::CONFIG;

// TODO: Separate executable targets for server, trainer, and exploiter
fn main() {
    // let bot = bot::Bot::new();
    // exploiter::exploitability(&bot, CONFIG.lbr_iters);
    // trainer::train(CONFIG.train_iters);
    launch_server();
}

fn launch_server() {
    backend::main().expect("Could not launch server");
}
