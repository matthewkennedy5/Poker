#[cfg(test)]

use crate::card_utils::*;

#[test]
fn card_bitmap() {
    let hand = vec!["2d", "9s", "Qd", "Qs", "Ac", "Ah", "As"];
    
/*     filler       clubs           diamonds      hearts        spades
 *
 *    |            |23456789TJQKA|23456789TJQKA|23456789TJQKA|23456789TJQKA|
 *    |000000000000|0000000000001|1000000000100|0000000000001|0000000100101|
 */
    let expected = 0b0000000000000000000000001100000000010000000000000010000000100101;
    assert_eq!(cards2bitmap(&strvec2cards(&hand)), expected);

}

#[test]
// Tests hand evaluation using the direct lookup table from u64 bitmap to the
// hand's strength
fn fast_hand_evaluation() {
    let table = FastHandTable::new();

    // define the hands we'll be using
    let royal_flush = vec!["Jd", "As", "Js", "Ks", "Qs", "Ts", "2c"];
    let royal_flush2 = vec!["Jd", "Ac", "Jc", "Kc", "Qc", "Tc", "2c"];
    let straight_flush = vec!["7d", "2c", "8d", "Jd", "9d", "3d", "Td"];
    let four = vec!["2h", "2c", "3d", "5c", "7d", "2d", "2s"];
    let full_house = vec!["As", "Jd", "Qs", "Jc", "2c", "Ac", "Ah"];
    let same_full_house = vec!["As", "Js", "2s", "Jc", "2c", "Ac", "Ah"];
    let better_full_house = vec!["2d", "9s", "Qd", "Qs", "Ac", "Ah", "As"];

    // Get strengths
    let royal_flush = table.hand_strength(&strvec2cards(&royal_flush));
    let royal_flush2 = table.hand_strength(&strvec2cards(&royal_flush2));
    let straight_flush = table.hand_strength(&strvec2cards(&straight_flush));
    let four = table.hand_strength(&strvec2cards(&four));
    let full_house = table.hand_strength(&strvec2cards(&full_house));
    let same_full_house = table.hand_strength(&strvec2cards(&same_full_house));
    let better_full_house = table.hand_strength(&strvec2cards(&better_full_house));

    // Test comparisons
    assert!(royal_flush > straight_flush);
    assert!(royal_flush > four);
    assert!(straight_flush > better_full_house);
    assert!(better_full_house > full_house);

    // Test for ties
    assert_eq!(royal_flush, royal_flush2);
    assert_eq!(same_full_house, full_house);
}

#[test]
fn uint_hands() {
    let hand: u64 = str2hand("Ac2d7h9cTd2s8c");
    assert_eq!(suit(card(hand, 0)), CLUBS);
    assert_eq!(rank(card(hand, 0)), 14);
    assert_eq!(suit(card(hand, 1)), DIAMONDS);
    assert_eq!(rank(card(hand, 1)), 2);
    assert_eq!(suit(card(hand, 4)), DIAMONDS);
    assert_eq!(rank(card(hand, 4)), 10);
    assert_eq!(suit(card(hand, 6)), CLUBS);
    assert_eq!(rank(card(hand, 6)), 8);
    assert_eq!(len(hand), 7);
    assert_eq!(hand2str(str2hand("9d8c7c6s5hQh")), "9d8c7c6s5hQh");

    let cards = vec![
        Card::new("8d"),
        Card::new("7c"),
        Card::new("2d"),
        Card::new("9c"),
        Card::new("Qd"),
        Card::new("Ah"),
    ];
    assert_eq!(hand2cards(cards2hand(&cards)), cards);
}

#[test]
fn hand_comparisons() {
    let table = LightHandTable::new();

    // define the hands we'll be using
    let royal_flush = vec!["Jd", "As", "Js", "Ks", "Qs", "Ts", "2c"];
    let royal_flush2 = vec!["Jd", "Ac", "Jc", "Kc", "Qc", "Tc", "2c"];
    let straight_flush = vec!["7d", "2c", "8d", "Jd", "9d", "3d", "Td"];
    let four = vec!["2h", "2c", "3d", "5c", "7d", "2d", "2s"];
    let full_house = vec!["As", "Jd", "Qs", "Jc", "2c", "Ac", "Ah"];
    let same_full_house = vec!["As", "Js", "2s", "Jc", "2c", "Ac", "Ah"];
    let better_full_house = vec!["2d", "9s", "Qd", "Qs", "Ac", "Ah", "As"];
    let full_house3 = vec!["3d", "3h", "3c", "2c", "2d"];
    let full_house2 = vec!["3d", "3h", "2c", "2h", "2d"];
    let flush = vec!["Jh", "2c", "2h", "3h", "7h", "As", "9h"];
    let same_flush = vec!["Jh", "2c", "2h", "3h", "7h", "2s", "9h"];
    let better_flush = vec!["Jh", "2c", "Ah", "3h", "7h", "Ts", "9h"];
    let straight = vec!["Ah", "2s", "3d", "5c", "4c"];
    let better_straight = vec!["6h", "2s", "3d", "5c", "4c"];
    let trips = vec!["5d", "4c", "6d", "6h", "6c"];
    let two_pair = vec!["6d", "5c", "5h", "Ah", "Ac"];
    let better_two_pair = vec!["Td", "Th", "Ad", "Ac", "6h"];
    let pair = vec!["Ah", "2d", "2s", "3c", "5c"];
    let ace_pair = vec!["Ac", "As", "2s", "3d", "6c"];
    let better_kicker = vec!["Ac", "As", "Ts", "3d", "6c"];
    let high_card = vec!["Kh", "Ah", "Qh", "2h", "3s"];
    let other_high_card = vec!["Ks", "As", "Qs", "2h", "3s"];

    // Get strengths
    let royal_flush = table.hand_strength(&strvec2cards(&royal_flush));
    let royal_flush2 = table.hand_strength(&strvec2cards(&royal_flush2));
    let straight_flush = table.hand_strength(&strvec2cards(&straight_flush));
    let four = table.hand_strength(&strvec2cards(&four));
    let full_house = table.hand_strength(&strvec2cards(&full_house));
    let full_house2 = table.hand_strength(&strvec2cards(&full_house2));
    let full_house3 = table.hand_strength(&strvec2cards(&full_house3));
    let same_full_house = table.hand_strength(&strvec2cards(&same_full_house));
    let better_full_house = table.hand_strength(&strvec2cards(&better_full_house));
    let flush = table.hand_strength(&strvec2cards(&flush));
    let same_flush = table.hand_strength(&strvec2cards(&same_flush));
    let better_flush = table.hand_strength(&strvec2cards(&better_flush));
    let straight = table.hand_strength(&strvec2cards(&straight));
    let better_straight = table.hand_strength(&strvec2cards(&better_straight));
    let trips = table.hand_strength(&strvec2cards(&trips));
    let two_pair = table.hand_strength(&strvec2cards(&two_pair));
    let better_two_pair = table.hand_strength(&strvec2cards(&better_two_pair));
    let pair = table.hand_strength(&strvec2cards(&pair));
    let ace_pair = table.hand_strength(&strvec2cards(&ace_pair));
    let better_kicker = table.hand_strength(&strvec2cards(&better_kicker));
    let high_card = table.hand_strength(&strvec2cards(&high_card));
    let other_high_card = table.hand_strength(&strvec2cards(&other_high_card));

    // Test different hand type comparisons
    assert!(royal_flush > straight_flush);
    assert!(royal_flush > trips);
    assert!(straight_flush > full_house);
    assert!(trips > two_pair);
    assert!(high_card < pair);
    assert!(straight < flush);

    // Test rank levels within hands
    assert!(better_two_pair > two_pair);
    assert!(better_flush > flush);
    assert!(better_kicker > ace_pair);
    assert!(better_straight > straight);
    assert!(better_full_house > full_house);
    assert!(full_house3 > full_house2);
    assert!(full_house > full_house3);

    // Test for ties
    assert_eq!(royal_flush, royal_flush2);
    assert_eq!(same_full_house, full_house);
    assert_eq!(other_high_card, high_card);
    assert_eq!(same_flush, flush);
}
