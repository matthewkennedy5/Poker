use dashmap::DashMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
#[cfg(test)]
use optimus::*;
use rand::prelude::*;
use rayon::prelude::*;
use smallvec::*;
use std::collections::{HashMap, HashSet};

static BOT: Lazy<Bot> = Lazy::new(|| {
    Bot::new(
        load_nodes(&CONFIG.nodes_path),
        CONFIG.subgame_solving,
        false,
        CONFIG.depth_limit,
    )
});

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

fn fast_hand_strength(hand: Vec<&str>, table: &FastHandTable) -> i32 {
    table.hand_strength(&strvec2cards(&hand))
}

#[test]
fn hand_comparisons() {
    // define the hands we'll be using
    let royal_flush = str2cards("JdAsJsKsQsTs2c");
    let royal_flush2 = str2cards("JdAcJcKcQcTc2c");
    let straight_flush = str2cards("7d2c8dJd9d3dTd");
    let full_house = str2cards("AsJdQsJc2cAcAh");
    let same_full_house = str2cards("AsJs2sJc2cAcAh");
    let better_full_house = str2cards("2d9sQdQsAcAhAs");
    let full_house3 = str2cards("3d3h3c2c2d");
    let full_house2 = str2cards("3d3h2c2h2d");
    let flush = str2cards("Jh2c2h3h7hAs9h");
    let same_flush = str2cards("Jh2c2h3h7h2s9h");
    let better_flush = str2cards("Jh2cAh3h7hTs9h");
    let straight = str2cards("Ah2s3d5c4c");
    let better_straight = str2cards("6h2s3d5c4c");
    let trips = str2cards("5d4c6d6h6c");
    let two_pair = str2cards("6d5c5hAhAc");
    let better_two_pair = str2cards("TdThAdAc6h");
    let pair = str2cards("Ah2d2s3c5c");
    let ace_pair = str2cards("AcAs2s3d6c");
    let better_kicker = str2cards("AcAsTs3d6c");
    let high_card = str2cards("KhAhQh2h3s");
    let other_high_card = str2cards("KsAsQs2h3s");

    // Get strengths
    let royal_flush = hand_strength(&royal_flush);
    let royal_flush2 = hand_strength(&royal_flush2);
    let straight_flush = hand_strength(&straight_flush);
    let full_house = hand_strength(&full_house);
    let full_house2 = hand_strength(&full_house2);
    let full_house3 = hand_strength(&full_house3);
    let same_full_house = hand_strength(&same_full_house);
    let better_full_house = hand_strength(&better_full_house);
    let flush = hand_strength(&flush);
    let same_flush = hand_strength(&same_flush);
    let better_flush = hand_strength(&better_flush);
    let straight = hand_strength(&straight);
    let better_straight = hand_strength(&better_straight);
    let trips = hand_strength(&trips);
    let two_pair = hand_strength(&two_pair);
    let better_two_pair = hand_strength(&better_two_pair);
    let pair = hand_strength(&pair);
    let ace_pair = hand_strength(&ace_pair);
    let better_kicker = hand_strength(&better_kicker);
    let high_card = hand_strength(&high_card);
    let other_high_card = hand_strength(&other_high_card);

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
    let human = str2cards("TdTc9c5s4d6hJd");
    let cpu = str2cards("As4s9c5s4d6hJd");
    assert!(hand_strength(&human) > hand_strength(&cpu));

    let human = str2cards("TdTc9c6hJd");
    let cpu = str2cards("As4s9c4dJd");
    assert!(hand_strength(&human) > hand_strength(&cpu));

    let human = str2cards("9cTcTdJd6h");
    let cpu = str2cards("4cJc4dAd9h");
    assert!(hand_strength(&human) > hand_strength(&cpu));
}

// Helper function for tests that get the bot's response at a certain spot
fn bot_strategy_contains_amount(
    amount: Amount,
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
    println!("{strategy:?}");
    let amounts: Vec<Amount> = strategy.keys().map(|action| action.amount).collect();
    amounts.contains(&amount)
}

#[test]
fn negative_bet_size() {
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
fn slumbot_bet_size() {
    let actions: Vec<Action> = vec![
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
            amount: 4375,
        },
    ];
    assert!(!bot_strategy_contains_amount(7750, "Qh5s", "", actions));
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
fn next_actions_are_sorted() {
    // Tests that the next actions are returned in the correct order
    let history = ActionHistory::from_strings(vec!["Call 100", "Call 100"]);
    let next_actions = history.next_actions(&CONFIG.bet_abstraction);
    // Order should be:
    // 1. Bet sizes in increasing order
    // 2. Check/Call
    // 3. Fold
    assert_eq!(
        next_actions[next_actions.len() - 1],
        Action {
            action: ActionType::Call,
            amount: 0
        }
    );
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

#[test]
fn terminal_utility_blinds() {
    let history = ActionHistory::from_strings(vec!["Call 100", "Fold 0"]);
    let util = terminal_utility(&deck(), &history, DEALER);
    assert_eq!(util, 100.0);
    let util = terminal_utility(&deck(), &history, OPPONENT);
    assert_eq!(util, -100.0);

    let history = ActionHistory::from_strings(vec!["Fold 0"]);
    let util = terminal_utility(&deck(), &history, DEALER);
    assert_eq!(util, -50.0);
    let util = terminal_utility(&deck(), &history, OPPONENT);
    assert_eq!(util, 50.0);
}

fn play_hand_always_call() -> f64 {
    let mut deck: Vec<Card> = deck();
    let mut rng = &mut rand::thread_rng();
    deck.shuffle(&mut rng);
    let bot = *[DEALER, OPPONENT].choose(&mut rng).unwrap();
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

// #[test]
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
    println!("Score against check/call bot: {mean} +/- {confidence} BB/h\n");
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
        "Bet 200",  // 250
        "Bet 1000", // 1250
        "Call 800", // 1000
        // Flop
        "Call 0",    // 0
        "Bet 1000",  // 1250
        "Bet 3000",  // 3750
        "Call 2000", // 2500
        // Turn
        "Bet 4000",  // 5000
        "Call 4000", // 5000
        // River
        "Bet 8000",
        "Bet 12000",
    ]);
    history.translate(&CONFIG.bet_abstraction);
}

#[test]
fn blinds_stack_sizes() {
    let history = ActionHistory::new();
    assert_eq!(
        history.stack_sizes(),
        [
            CONFIG.stack_size - CONFIG.small_blind,
            CONFIG.stack_size - CONFIG.big_blind
        ]
    );
}

#[test]
fn isomorphic_hand_len() {
    let flop = load_flop_isomorphic();
    assert_eq!(flop.len(), 1_342_562);
    println!("Flop len: {}", flop.len());
    let turn = load_turn_isomorphic();
    assert_eq!(turn.len(), 14_403_610);
    println!("Turn len: {}", turn.len());
    let river = load_river_isomorphic();
    assert_eq!(river.len(), 125_756_657);
    println!("River len: {}", river.len());
}

#[test]
fn isomorphic_hand_example() {
    // Suit order: 2 clubs, 1 heart, 2 spades, 2 diamonds = 7
    //              4cTc      2h       8s4s       KdTd
    //
    // so should go:  spades -> clubs, clubs -> diamonds, diamonds -> hearts, hearts -> spades
    //
    // . my code goes:  diamonds -> clubs, clubs -> diamonds, spades -> hearts

    let hand: Vec<Card> = str2cards("2h8sKd4c4sTcTd");
    let expected_result = str2cards("8c2s4c4dTdThKh");
    //  my result: 8d2s4cTc4dThKh

    // by suits:
    //     clubs: 4cTc
    //     diamonds: KdTd
    //     hearts: 2h
    //     spades: 8s4s
    //
    //  order should be: spades < clubs < diamonds < hearts
    //               so: spades->clubs, clubs->diamonds, diamonds->hearts, hearts->spades

    // if ge:            diamonds < clubs < spades < hearts
    //                   diamonds->clubs, clubs->diamonds, spades->hearts, hearts->spades

    // expected result goes: spades->clubs, clubs->diamonds, diamonds->hearts, hearts->spades (yep!)

    // my code goes: clubs->clubs, spades->diamonds, diamonds->hearts, hearts->spades
    //                  clubs < spades < diamonds < hearts

    let result: Vec<Card> = isomorphic_hand(&hand, true).to_vec();
    assert_eq!(result, expected_result, "{}", cards2str(&result));
    // Card { rank: 8, suit: 0 }, Card { rank: 2, suit: 3 }, Card { rank: 4, suit: 0 }, Card { rank: 4, suit: 1 }, Card { rank: 10, suit: 1 }, Card { rank: 10, suit: 2 }, Card { rank: 13, suit: 2 }]
}

#[test]
fn isomorphic_hand_ehs() {
    // Make sure that every hand that maps to the same isomorphic_hand has the same E[HS] and E[HS^2].

    let deck = deck();
    let n_cards = 7;

    let iso_ehs: DashMap<u64, f64> = DashMap::new();

    let bar = pbar(match n_cards {
        5 => 25989600,
        6 => 305377800,
        7 => 2809475760,
        _ => panic!(),
    });
    for preflop in deck.iter().combinations(2) {
        let preflop: Vec<Card> = preflop.iter().map(|&&c| c).collect();
        let mut rest_of_deck = deck.clone();
        rest_of_deck.retain(|c| !preflop.contains(&c));

        let boards: Vec<Vec<&Card>> = rest_of_deck.iter().combinations(n_cards - 2).collect();
        boards.par_iter().for_each(|board| {
            let board: Vec<Card> = board.iter().map(|&&c| c).collect();
            let mut cards: Vec<Card> = Vec::with_capacity(preflop.len() + board.len());
            cards.extend(&preflop);
            cards.extend(&board);

            let hand: u64 = cards2hand(&cards);
            let ehs2 = equity_distribution_moment(hand, 2);
            let iso = cards2hand(&isomorphic_hand(&cards, true));

            match iso_ehs.get(&iso) {
                Some(existing_ehs2) => {
                    assert!((ehs2 - *existing_ehs2).abs() < 1e-6);
                    // println!(
                    //     "Hand {} has same ehs {} as existing hand.",
                    //     hand2str(hand),
                    //     ehs
                    // );
                }
                None => {
                    iso_ehs.insert(iso, ehs2);
                }
            };
            bar.inc(1);
        });
    }
    bar.finish_with_message("Done");
}

#[test]
fn normalize_sum() {
    let mut hashmap: HashMap<i32, f64> = HashMap::new();
    hashmap.insert(0, 1.0);
    hashmap.insert(1, 2.0);
    hashmap.insert(2, 3.0);
    hashmap.insert(3, 4.0);
    hashmap.insert(4, 5.0);
    let norm = normalize(&hashmap);
    let sum: f64 = norm.values().sum();
    assert!((sum - 1.0).abs() < 1e-12);
}

// Returns all the descendant action histories of history (not including terminal actions)
fn all_histories(history: &ActionHistory) -> Vec<ActionHistory> {
    if history.hand_over() {
        return Vec::new();
    }
    let mut all: Vec<ActionHistory> = vec![history.clone()];
    for next in history.next_actions(&CONFIG.bet_abstraction) {
        let mut child_history = history.clone();
        child_history.add(&next);
        all.extend(all_histories(&child_history));
    }
    all
}

// Test that all canonical preflop hands are put in a different bin.
#[test]
fn test_preflop_buckets() {
    let preflop_hands = isomorphic_preflop_hands();
    let buckets: Vec<i32> = preflop_hands.iter().map(|h| ABSTRACTION.bin(h)).collect();
    let buckets_set: HashSet<i32> = buckets.iter().map(|b| b.clone()).collect();
    assert_eq!(buckets.len(), buckets_set.len());
}

#[test]
fn all_in_showdown_street() {
    let history = ActionHistory::from_strings(vec!["Call 100", "Bet 20000", "Call 19900"]);
    assert_eq!(history.street, SHOWDOWN);
}

#[test]
fn train_performance() {
    train(100_000, 100_000, false);
    let nodes = load_nodes(&CONFIG.nodes_path);
    // Make sure the exploitability is below 0.5 BB/h
    let exploitability = blueprint_exploitability(&nodes, 100_000);
    assert!(exploitability < 0.5);
}

#[test]
fn abstraction_distributes_hands_evenly() {
    // Randomly deal a lot of hands and bucket it to the abstraction. Verify that the largest ratio
    // of counts between abstraction buckets is less than 2.
    let abstraction = Abstraction::new();

    let mut counts: Vec<i32> = vec![0; CONFIG.flop_buckets as usize];
    let mut deck = deck();

    for _ in 0..100_000 {
        deck.shuffle(&mut thread_rng());
        let hand: &[Card] = &deck[0..5];
        let bucket = abstraction.bin(hand) as usize;
        counts[bucket] += 1;
    }

    //  verify that no count is 2x of another
    let max: f64 = counts.iter().max().unwrap().clone() as f64;
    let min: f64 = counts.iter().min().unwrap().clone() as f64;
    let ratio: f64 = max / min;
    assert!(ratio < 2.0);
}

#[test]
fn abstraction_buckets_in_range() {
    let abstraction = Abstraction::new();

    for street in [FLOP, TURN, RIVER].iter() {
        let street = street.clone();
        let mut hands: Vec<u64> = Vec::new();
        let n_buckets;
        if street == FLOP {
            hands = load_flop_isomorphic();
            n_buckets = CONFIG.flop_buckets;
        } else if street == TURN {
            load_turn_isomorphic();
            n_buckets = CONFIG.turn_buckets;
        } else if street == RIVER {
            load_river_isomorphic();
            n_buckets = CONFIG.river_buckets;
        } else {
            panic!("bad street");
        };
        for hand in hands {
            let cards = hand2cards(hand);
            let bucket = abstraction.bin(&cards);
            assert!(
                0 <= bucket && bucket < n_buckets,
                "Hand {} has bucket {} which is outside the range of 0 to {}",
                hand2str(hand),
                bucket,
                n_buckets
            );
        }
    }
}

#[test]
fn test_subgame_solving() {
    BOT.get_strategy(
        &str2cards("AdAs"),
        &Vec::new(),
        &ActionHistory::from_strings(vec!["Bet 250"]),
    );
}

#[test]
fn subgame_solving_beats_blueprint() {
    let blueprint_bot = Bot::new(load_nodes(&CONFIG.nodes_path), false, false, 100);
    let subgame_bot = Bot::new(load_nodes(&CONFIG.nodes_path), true, true, -1);

    let iters = 1_000_000;
    let mut winnings: Vec<f64> = Vec::with_capacity(iters as usize);
    let bar = pbar(iters as u64);
    let mut mean = 0.0;
    for i in 0..iters {
        let amount = play_hand_bots(&blueprint_bot, &subgame_bot);
        winnings.push(amount / CONFIG.big_blind as f64);
        // TODO: DRY with exploiter.rs
        if winnings.len() >= 2 {
            mean = statistical::mean(&winnings);
            let std = statistical::standard_deviation(&winnings, Some(mean));
            let confidence = 1.96 * std / (i as f64).sqrt();
            println!("Subgame solver winnings vs blueprint: {mean} +/- {confidence} BB/h\n");
        }
        bar.inc(1);
    }
    bar.finish();
    assert!(mean > 0.0);
}

fn play_hand_bots(blueprint_bot: &Bot, subgame_bot: &Bot) -> f64 {
    let mut deck: Vec<Card> = deck();
    let mut rng = &mut rand::thread_rng();
    deck.shuffle(&mut rng);
    let subgame_bot_position = *[DEALER, OPPONENT].choose(&mut rng).unwrap();
    let mut history = ActionHistory::new();
    while !history.hand_over() {
        let hand = get_hand(&deck, history.player, history.street);
        let hole = &hand[..2];
        let board = &hand[2..];

        let bot = if history.player == subgame_bot_position {
            subgame_bot
        } else {
            blueprint_bot
        };

        let action = bot.get_action(hole, board, &history);
        history.add(&action);
    }
    terminal_utility(&deck, &history, subgame_bot_position)
}

#[test]
// Tests that the river equity cache fits in memory
fn river_equity_cache_mem_usage() {
    let river_iso = load_river_isomorphic();
    let bar = pbar(river_iso.len() as u64);
    river_iso.into_par_iter().for_each(|hand| {
        let smallvec_hand: SmallVecHand = hand2cards(hand).to_smallvec();
        RIVER_EQUITY_CACHE.insert(smallvec_hand, 0.0);
        bar.inc(1);
    });
    bar.finish();
    assert_eq!(RIVER_EQUITY_CACHE.len(), 125_756_657);
}

// #[test]
fn test_depth_limit_probability() {
    // Compare the subgame solving strategy with and without depth limited solving.
    let full_subgame_bot = Bot::new(load_nodes(&CONFIG.nodes_path), false, true, -1);
    let depth_limit_bot = Bot::new(load_nodes(&CONFIG.nodes_path), false, true, 5);

    let hands = 1_000;
    let bar = pbar(hands as u64);
    for i in 0..hands {
        let mut deck: Vec<Card> = deck();
        let mut rng = &mut rand::thread_rng();
        deck.shuffle(&mut rng);
        let mut history = ActionHistory::new();
        while !history.hand_over() {
            let hand = get_hand(&deck, history.player, history.street);
            let hole = &hand[..2];
            let board = &hand[2..];

            let full_depth_strategy = full_subgame_bot.get_strategy(hole, board, &history);
            let depth_limit_strategy = depth_limit_bot.get_strategy(hole, board, &history);

            let mut total_squared_error = 0.0;
            for action in full_depth_strategy.keys() {
                let full_depth_prob = full_depth_strategy.get(action).unwrap().clone();
                let depth_limited_prob = depth_limit_strategy.get(action).unwrap().clone();
                let squared_error = (full_depth_prob - depth_limited_prob).powf(2.0);
                total_squared_error += squared_error;
            }
            let mse = total_squared_error / full_depth_strategy.len() as f64;

            println!("Hole: {}, Board: {}", cards2str(hole), cards2str(board));
            println!("Full depth strategy: {:?}", full_depth_strategy);
            println!("Depth limit strategy: {:?}", depth_limit_strategy);
            println!("Mean squared error: {}", mse);
            assert!(mse < 0.01);

            let action = sample_action_from_strategy(&full_depth_strategy);
            history.add(&action);
        }

        bar.inc(1);
    }
    bar.finish();
}

// #[test]
fn subgame_strategy_stability() {
    // for depth in [10, 8, 6, 4, 2].iter() {
    let depth = 5;
    let bot = Bot::new(load_nodes(&CONFIG.nodes_path), true, true, depth.clone());
    let strategy = bot.get_strategy(
        &str2cards("8hAd"),
        &str2cards("8dAc7s"),
        &ActionHistory::from_strings(vec!["Call 100", "Call 100"]),
    );
    println!("Depth: {depth}, Strategy: {:?}", strategy);
    // }
    // assert!(
    //     strategy
    //         .get(&Action {
    //             action: ActionType::Bet,
    //             amount: 100
    //         })
    //         .unwrap()
    //         .clone()
    //         > 0.95
    // );
}

#[test]
fn equity_distribution_expectations() {
    // Test that the expectation of each hand's equity distribution is equal to the hand's equity.
    for street in ["turn"].iter() {
        let dists = get_equity_distributions(street);
        let flop_hands = if street.clone() == "flop" {
            load_flop_isomorphic()
        } else {
            load_turn_isomorphic()
        };
        let bar = pbar(dists.len() as u64);
        (0..dists.len()).into_par_iter().for_each(|i| {
            let hand = flop_hands[i];
            let dist = dists[i].clone();
            let hand_ehs: f64 = equity_distribution_moment(hand, 1);
            let hand_ehs2: f64 = equity_distribution_moment(hand, 2);
            // Calculate the expectation and second moment of the equity distribution, by taking the
            // center of the discretized probability buckets
            let mut dist_ehs: f64 = 0.0;
            let mut dist_ehs2: f64 = 0.0;
            let bucket_width = 1.0 / dist.len() as f64;
            let offset = bucket_width / 2.0;
            for (i, &p) in dist.iter().enumerate() {
                let prob = (i as f64) * bucket_width + offset;
                dist_ehs += p as f64 * prob;
                dist_ehs2 += p as f64 * prob * prob;
            }
            assert!(
                (hand_ehs - dist_ehs).abs() < 0.1,
                "{hand_ehs} != {dist_ehs}"
            );
            assert!(
                (hand_ehs2 - dist_ehs2).abs() < 0.1,
                "{hand_ehs2} != {dist_ehs2}"
            );
            bar.inc(1);
        });
        bar.finish();
    }
}
