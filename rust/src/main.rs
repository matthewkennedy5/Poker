#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(proc_macro_hygiene, decl_macro)]

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
#[macro_use]
extern crate lazy_static;
extern crate actix_web;
extern crate actix_rt;
extern crate actix_files;

mod trainer;
mod trainer_utils;
mod card_abstraction;
mod card_utils;
mod exploiter;
mod tests;
mod backend;
mod bot;

fn main() {
    backend::main().expect("Could not launch server");
}

