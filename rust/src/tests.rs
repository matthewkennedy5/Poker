use crate::card_utils::strvec2cards;
use crate::card_utils::cards2str;
use crate::card_utils;

#[test]
fn hand_comparisons() {
        let table = card_utils::HandTable::new();

        // Define the hands we'll be using
        let royal_flush = vec!["Jd", "As", "Js", "Ks", "Qs", "Ts", "2c"];
        let straight_flush = vec!["7d", "2c", "8d", "Jd", "9d", "3d", "Td"];
        let four = vec!["2h", "2c", "3d", "5c", "7d", "2d", "2s"];
        let full_house = vec!["As", "Jd", "Qs", "Jc", "2c", "Ac", "Ah"];
        let same_full_house = vec!["As", "Js", "2s", "Jc", "2c", "Ac", "Ah"];
        let flush = vec!["Jh", "2c", "2h", "3h", "7h", "As", "9h"];
        let same_flush = vec!["Jh", "2c", "2h", "3h", "7h", "2s", "9h"];
        let better_flush = vec!["Jh", "2c", "Ah", "3h", "7h", "Ts", "9h"];
        let straight = vec!["Ah", "2s", "3d", "5c", "4c"];
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
        let straight_flush = table.hand_strength(&strvec2cards(&straight_flush));
        let four = table.hand_strength(&strvec2cards(&four));
        let full_house = table.hand_strength(&strvec2cards(&full_house));
        let same_full_house = table.hand_strength(&strvec2cards(&same_full_house));
        let flush = table.hand_strength(&strvec2cards(&flush));
        let same_flush = table.hand_strength(&strvec2cards(&same_flush));
        let better_flush = table.hand_strength(&strvec2cards(&better_flush));
        let straight = table.hand_strength(&strvec2cards(&straight));
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

        // Test for ties
        assert_eq!(same_full_house, full_house);
        assert_eq!(other_high_card, high_card);
        assert_eq!(same_flush, flush);
}