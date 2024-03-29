#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_snake_case)]

extern crate actix_files;
extern crate actix_rt;
extern crate actix_web;
extern crate bincode;
extern crate indicatif;
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate smallvec;

mod backend;
mod bot;
mod card_abstraction;
mod card_utils;
mod config;
mod exploiter;
mod ranges;
mod trainer;
mod nodes;
mod trainer_utils;

pub use backend::*;
pub use bot::*;
pub use card_abstraction::*;
pub use card_utils::*;
pub use config::*;
pub use exploiter::*;
pub use ranges::*;
pub use trainer::*;
pub use trainer_utils::*;
pub use nodes::*;
