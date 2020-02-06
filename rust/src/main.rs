mod card_abstraction;
mod card_utils;

fn main() {
    // let cards = vec!["5d", "5c", "2h", "3s", "Ac"];
    let cards = vec![card_utils::card("6d"), card_utils::card("2d")];
    let bin = card_abstraction::abstract_id(cards);
    println!("Bin: {}", bin);
}
