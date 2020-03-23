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
extern crate rayon;

mod card_abstraction;
mod card_utils;
mod exploiter;
mod tests;
mod trainer;
mod trainer_utils;

fn main() {
    exploiter::exploitability(&std::collections::HashMap::new());
}
