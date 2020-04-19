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
extern crate qstring;

mod card_abstraction;
mod card_utils;
mod exploiter;
mod tests;
mod trainer;
mod trainer_utils;
mod backend;

fn main() {
    backend::main();
    // trainer::train(10_000_000);
    // let nodes = trainer::load_strategy();
    // exploiter::exploitability(&nodes);
    // exploiter::exploitability(&std::collections::HashMap::new());
}
