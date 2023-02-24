
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

mod bot;
mod backend;
mod card_abstraction;
mod card_utils;
mod config;
mod exploiter;
mod ranges;
mod trainer_utils;
mod trainer;

// TODO: Only expose public functions (dont use wildcard)
pub use bot::*;
pub use backend::*;
pub use card_abstraction::*;
pub use card_utils::*;
pub use config::*;
pub use exploiter::*;
pub use ranges::*;
pub use trainer_utils::*;
pub use trainer::*;
