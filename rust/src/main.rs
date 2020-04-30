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
extern crate qstring;
extern crate rayon;

mod backend;
mod bot;
mod card_abstraction;
mod card_utils;
mod exploiter;
mod tests;
mod trainer;
mod trainer_utils;

fn main() {
    // let hand = card_utils::strvec2cards(&vec!["Ac", "Ad", "As", "Ah", "Qh"]);
    // let abs = card_abstraction::LightAbstraction::new();
    // println!("bin: {}", abs.bin(&hand));

    // backend::main();
    trainer::train(100_000);
    // let nodes = trainer::load_strategy();
    // exploiter::exploitability(&nodes);
    // exploiter::exploitability(&std::collections::HashMap::new());
}
