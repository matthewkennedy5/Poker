extern crate serde;
extern crate serde_json;
extern crate itertools;
extern crate indicatif;
#[macro_use(c)]
extern crate cute;
extern crate rand;

mod card_abstraction;
mod card_utils;
mod tests;

use card_utils::Card;


fn test_memory() {
    // So the heap size of the turn cards looks ok...
    let mut turn: Vec<Vec<(u8, String)>> = Vec::new();
    // let cards = vec![Card::new("3c"), Card::new("9c"), Card::new("4d"),
    //                  Card::new("9h"), Card::new("Ts"), Card::new("As")]; lots of memory but works
    // let cards = "AsKsQsJsTs";    0 memory
    // let cards = vec!["As", "Ks", "Qs", "Js", "Ts"];  // More significant memory but fine
    // let cards = ["As", "Ks", "Qs", "Js", "Ts", "5s"]; small memory
    // Yeah, the String is taking up a lot of memory
    let cards = vec![(5, String::from("d")), (10, String::from("h")), (11, String::from("c")), (12, String::from("d")), (13, String::from("d")), (14, String::from("s"))];
    println!("{}", std::mem::size_of_val(&cards));
    for i in 0..5718089 {
        turn.push(cards.clone());
    }
    println!("{}", turn.len());
}


fn main() {

    // test_memory();
    let cards = card_utils::deal_canonical(6);
    println!("{}", cards.len());
    println!("{:#?}", cards);


    // let cards = vec![Card::new("3c"), Card::new("9c"), Card::new("4d"),
    //                  Card::new("9h"), Card::new("Ts")];

    // // let cards = vec![Card::new("6d"), Card::new("2d")];
    // println!("{}", card_utils::is_canonical(&cards));
    // let a = card_abstraction::Abstraction::new();
    // let bin = a.abstract_id(&cards);
    // println!("Bin: {}", bin);
}
