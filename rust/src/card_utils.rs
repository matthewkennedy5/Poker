
pub fn archetype(cards: Vec<Card>) -> Vec<Card> {
    cards
}

#[derive(Debug, Clone)]
pub struct Card {
    pub rank: i32,
    pub suit: String
}

pub fn card(card: &str) -> Card {
    let rank = match &card[0..1] {
        "2" => 2,
        "3" => 3,
        "4" => 4,
        "5" => 5,
        "6" => 6,
        "7" => 7,
        "8" => 8,
        "9" => 9,
        "T" => 10,
        "J" => 11,
        "Q" => 12,
        "K" => 13,
        "A" => 14,
        _ => panic!("Bad card string")
    };
    let suit = String::from(&card[1..2]);
    return Card { rank: rank, suit: suit };
}