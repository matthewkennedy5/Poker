use once_cell::sync::Lazy;
#[cfg(test)]
use optimus::*;
use rand::prelude::*;
use rayon::iter::*;

static BOT: Lazy<Bot> = Lazy::new(|| Bot::new());

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
    let royal_flush = fast_hand_strength(royal_flush, &table);
    let royal_flush2 = fast_hand_strength(royal_flush2, &table);
    let straight_flush = fast_hand_strength(straight_flush, &table);
    let four = fast_hand_strength(four, &table);
    let full_house = fast_hand_strength(full_house, &table);
    let same_full_house = fast_hand_strength(same_full_house, &table);
    let better_full_house = fast_hand_strength(better_full_house, &table);

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

fn light_hand_strength(hand: Vec<&str>, table: &LightHandTable) -> i32 {
    return table.hand_strength(&strvec2cards(&hand));
}

fn fast_hand_strength(hand: Vec<&str>, table: &FastHandTable) -> i32 {
    return table.hand_strength(&strvec2cards(&hand));
}

#[test]
fn hand_comparisons() {
    let table = LightHandTable::new();

    // define the hands we'll be using
    let royal_flush = vec!["Jd", "As", "Js", "Ks", "Qs", "Ts", "2c"];
    let royal_flush2 = vec!["Jd", "Ac", "Jc", "Kc", "Qc", "Tc", "2c"];
    let straight_flush = vec!["7d", "2c", "8d", "Jd", "9d", "3d", "Td"];
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
    let royal_flush = light_hand_strength(royal_flush, &table);
    let royal_flush2 = light_hand_strength(royal_flush2, &table);
    let straight_flush = light_hand_strength(straight_flush, &table);
    let full_house = light_hand_strength(full_house, &table);
    let full_house2 = light_hand_strength(full_house2, &table);
    let full_house3 = light_hand_strength(full_house3, &table);
    let same_full_house = light_hand_strength(same_full_house, &table);
    let better_full_house = light_hand_strength(better_full_house, &table);
    let flush = light_hand_strength(flush, &table);
    let same_flush = light_hand_strength(same_flush, &table);
    let better_flush = light_hand_strength(better_flush, &table);
    let straight = light_hand_strength(straight, &table);
    let better_straight = light_hand_strength(better_straight, &table);
    let trips = light_hand_strength(trips, &table);
    let two_pair = light_hand_strength(two_pair, &table);
    let better_two_pair = light_hand_strength(better_two_pair, &table);
    let pair = light_hand_strength(pair, &table);
    let ace_pair = light_hand_strength(ace_pair, &table);
    let better_kicker = light_hand_strength(better_kicker, &table);
    let high_card = light_hand_strength(high_card, &table);
    let other_high_card = light_hand_strength(other_high_card, &table);

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

#[test]
fn high_pair_beats_low_pair() {
    let table = LightHandTable::new();

    let human = vec!["Td", "Tc", "9c", "5s", "4d", "6h", "Jd"];
    let cpu = vec!["As", "4s", "9c", "5s", "4d", "6h", "Jd"];
    assert!(light_hand_strength(human, &table) > light_hand_strength(cpu, &table));

    let human = vec!["Td", "Tc", "9c", "6h", "Jd"];
    let cpu = vec!["As", "4s", "9c", "4d", "Jd"];
    assert!(light_hand_strength(human, &table) > light_hand_strength(cpu, &table));

    let human = vec!["9c", "Tc", "Td", "Jd", "6h"];
    let cpu = vec!["4c", "Jc", "4d", "Ad", "9h"];
    assert!(light_hand_strength(human, &table) > light_hand_strength(cpu, &table));
}

// Helper function for tests that get the bot's response at a certain spot
fn bot_strategy_contains_amount(
    amount: i32,
    hole: &str,
    board: &str,
    actions: Vec<Action>,
) -> bool {
    let mut history = ActionHistory::new();
    for a in actions {
        history.add(&a);
    }
    let hole = str2cards(hole);
    let board = str2cards(board);
    let strategy = BOT.get_strategy(&hole, &board, &history);
    println!("{:?}", strategy);
    let amounts: Vec<i32> = strategy.keys().map(|action| action.amount).collect();
    return amounts.contains(&amount);
}

#[test]
fn negative_bet_size() {
    // TODO: this is giving a stack overflow error for some reason
    let actions = vec![
        Action {
            action: ActionType::Call,
            amount: 100,
        },
        Action {
            action: ActionType::Call,
            amount: 100,
        },
        Action {
            action: ActionType::Bet,
            amount: 50,
        },
        Action {
            action: ActionType::Bet,
            amount: 100,
        },
        Action {
            action: ActionType::Bet,
            amount: 200,
        },
    ];
    assert!(!bot_strategy_contains_amount(
        19834, "Ts8s", "Js5sQc", actions
    ));
}

// Due to a bug in action translation, the out of position player's "all in" size (18750) bigger
// can be bigger than its remaining stack (18450)
#[test]
fn cpu_bets_more_than_stack() {
    let actions = vec![
        Action {
            action: ActionType::Call,
            amount: 100,
        },
        Action {
            action: ActionType::Bet,
            amount: 250,
        },
        Action {
            action: ActionType::Bet,
            amount: 500,
        },
        Action {
            action: ActionType::Call,
            amount: 350,
        },
        Action {
            action: ActionType::Bet,
            amount: 1000,
        },
        Action {
            action: ActionType::Bet,
            amount: 2000,
        },
    ];
    assert!(!bot_strategy_contains_amount(
        18750, "QdQs", "6dTcJd", actions
    ));
}

#[test]
fn action_translation_sizes() {
    let actions = vec![
        Action {
            action: ActionType::Bet,
            amount: 250,
        },
        Action {
            action: ActionType::Bet,
            amount: 750,
        },
        Action {
            action: ActionType::Bet,
            amount: 2500,
        },
        Action {
            action: ActionType::Call,
            amount: 2000,
        },
        Action {
            action: ActionType::Call,
            amount: 0,
        },
        Action {
            action: ActionType::Bet,
            amount: 17250,
        },
    ];
    assert!(!bot_strategy_contains_amount(
        17500, "Qs4h", "8c6h4d", actions
    ));
    let actions = vec![
        Action {
            action: ActionType::Call,
            amount: 100,
        },
        Action {
            action: ActionType::Bet,
            amount: 300,
        },
        Action {
            action: ActionType::Bet,
            amount: 2000,
        },
        Action {
            action: ActionType::Bet,
            amount: 4500,
        },
        Action {
            action: ActionType::Call,
            amount: 2700,
        },
        Action {
            action: ActionType::Bet,
            amount: 4500,
        },
        Action {
            action: ActionType::Bet,
            amount: 15200,
        },
    ];
    assert!(!bot_strategy_contains_amount(
        15500, "Kh9s", "Ah7h2d", actions
    ));
}

#[test]
fn min_bet_at_least_double() {
    let actions: Vec<Action> = vec![Action {
        action: ActionType::Bet,
        amount: 200,
    }];
    assert!(!bot_strategy_contains_amount(250, "Ah8h", "", actions));
}

#[test]
fn bet_size_too_small() {
    let actions: Vec<Action> = vec![
        Action {
            action: ActionType::Bet,
            amount: 500,
        },
        Action {
            action: ActionType::Bet,
            amount: 1500,
        },
        Action {
            action: ActionType::Call,
            amount: 1000,
        },
        Action {
            action: ActionType::Bet,
            amount: 1500,
        },
        Action {
            action: ActionType::Bet,
            amount: 18450,
        },
    ];
    assert!(!bot_strategy_contains_amount(
        18500, "6h5s", "Js8s7h", actions
    ));
}

#[test]
fn all_in_size_allowed() {
    let actions: Vec<Action> = vec![
        Action {
            action: ActionType::Bet,
            amount: 200,
        },
        Action {
            action: ActionType::Bet,
            amount: 1250,
        },
        Action {
            action: ActionType::Call,
            amount: 1050,
        },
        Action {
            action: ActionType::Call,
            amount: 0,
        },
        Action {
            action: ActionType::Bet,
            amount: 750,
        },
        Action {
            action: ActionType::Bet,
            amount: 18650,
        },
    ];
    assert!(bot_strategy_contains_amount(
        18000, "AdJc", "8h4d2s", actions
    ));
}

#[test]
fn no_bet_zero() {
    let actions: Vec<Action> = vec![
        Action {
            action: ActionType::Bet,
            amount: 250,
        },
        Action {
            action: ActionType::Call,
            amount: 250,
        },
        Action {
            action: ActionType::Bet,
            amount: 19750,
        },
    ];
    assert!(bot_strategy_contains_amount(0, "AdJc", "8h4d2s", actions));
}

#[test]
fn history_all_in_size_allowed() {
    let actions: Vec<Action> = vec![
        Action {
            action: ActionType::Bet,
            amount: 200,
        },
        Action {
            action: ActionType::Bet,
            amount: 1250,
        },
        Action {
            action: ActionType::Call,
            amount: 1050,
        },
        Action {
            action: ActionType::Call,
            amount: 0,
        },
        Action {
            action: ActionType::Bet,
            amount: 750,
        },
        Action {
            action: ActionType::Bet,
            amount: 18650,
        },
    ];
    let mut history = ActionHistory::new();
    for a in actions {
        history.add(&a);
    }
    assert!(history.is_legal_next_action(&Action {
        action: ActionType::Bet,
        amount: 18000
    }));
}

#[test]
fn action_translation_all_in() {
    let actions = vec![
        Action {
            action: ActionType::Bet,
            amount: 200,
        },
        Action {
            action: ActionType::Call,
            amount: 200,
        },
        Action {
            action: ActionType::Bet,
            amount: 19750,
        },
        Action {
            action: ActionType::Bet,
            amount: 19800,
        },
    ];
    let mut history = ActionHistory::new();
    for a in actions {
        history.add(&a);
    }
    history.translate(&CONFIG.bet_abstraction);
}

#[test]
fn limp_is_call_not_bet() {
    let history = ActionHistory::new();
    let bet_abstraction: Vec<Vec<f64>> = vec![vec![1.0]];
    // assumes that bet_abstraction contains POT on preflop
    let call = Action {
        action: ActionType::Call,
        amount: CONFIG.big_blind,
    };
    let bet = Action {
        action: ActionType::Bet,
        amount: CONFIG.big_blind,
    };
    let next_actions = history.next_actions(&bet_abstraction);
    assert!(next_actions.contains(&call));
    assert!(!next_actions.contains(&bet));
}

#[test]
fn all_in_call() {
    let mut history = ActionHistory::new();
    history.add(&Action {
        action: ActionType::Bet,
        amount: 20000,
    });
    assert!(!history.is_legal_next_action(&Action {
        action: ActionType::Bet,
        amount: 20000
    }));
}

fn play_hand_always_call() -> f64 {
    let mut deck: Vec<Card> = deck();
    let mut rng = &mut rand::thread_rng();
    deck.shuffle(&mut rng);
    let bot = [DEALER, OPPONENT].choose(&mut rng).unwrap().clone();
    let mut history = ActionHistory::new();
    while !history.hand_over() {
        let action = if history.player == bot {
            let hand = get_hand(&deck, bot, history.street);
            let hole = &hand[..2];
            let board = &hand[2..];
            BOT.get_action(hole, board, &history)
        } else {
            // Opponent only uses check/call actions
            Action {
                action: ActionType::Call,
                amount: history.to_call(),
            }
        };
        history.add(&action);
    }
    terminal_utility(&deck, &history, bot)
}

#[test]
fn bot_beats_always_call() {
    println!("[INFO] Starting game against always call bot...");
    let iters = 10_000;
    let bar = pbar(iters);
    let winnings: Vec<f64> = (0..iters)
        .into_par_iter()
        .map(|_i| {
            let score = play_hand_always_call() / (CONFIG.big_blind as f64);
            bar.inc(1);
            score
        })
        .collect();
    bar.finish();
    let mean = statistical::mean(&winnings);
    let std = statistical::standard_deviation(&winnings, Some(mean));
    let confidence = 1.96 * std / (iters as f64).sqrt();
    println!(
        "Score against check/call bot: {} +/- {} BB/h\n",
        mean, confidence
    );
}

// TODO: Write a test to make sure that the nodes contain all the infosets (no gaps)
// Only matters for the final training process

#[test]
fn cpu_action_backend() {
    let history = ActionHistory::from_strings(vec![
        "Bet 200",
        "Bet 1000",
        "Call 800",
        "Call 0",
        "Bet 1000",
        "Bet 3000",
        "Call 2000",
        "Bet 4000",
        "Call 4000",
        "Bet 8000",
        "Bet 12000",
    ]);
    BOT.get_action(&str2cards("As3s"), &str2cards("Qd9c3h9h8h"), &history);
}

#[test]
fn more_action_translation() {
    let history = ActionHistory::from_strings(vec![
        // Preflop
        "Bet 200",      // 250
        "Bet 1000",     // 1250
        "Call 800",     // 1000
        // Flop         
        "Call 0",       // 0
        "Bet 1000",     // 1250
        "Bet 3000",     // 3750
        "Call 2000",    // 2500
        // Turn         
        "Bet 4000",     // 5000
        "Call 4000",    // 5000
        // River
        "Bet 8000",
        "Bet 12000",
    ]);
    history.translate(&CONFIG.bet_abstraction);
}
