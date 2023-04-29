use std::collections::HashSet;
use once_cell::sync::Lazy;
#[cfg(test)]
use optimus::*;
use rayon::prelude::*;
use rand::prelude::*;

static BOT: Lazy<Bot> = Lazy::new(Bot::new);

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
    let history = ActionHistory::from_strings(vec![
        "Call 100",
        "Fold 0"
    ]);
    let util = terminal_utility(&deck(), &history, DEALER);
    assert_eq!(util, 100.0);
    let util = terminal_utility(&deck(), &history, OPPONENT);
    assert_eq!(util, -100.0);

    let history = ActionHistory::from_strings(vec![
        "Fold 0"
    ]);
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

// #[test]
fn depth_limited_solving() {
    // Depth-limited solving should output a similar strategy to solving to the end of the game.
    let action = Action {
        action: ActionType::Bet,
        amount: 300,
    };
    let history = ActionHistory::new();
    let solve_spot = |iters, depth| {
        let nodes = Bot::solve_subgame(&history, &Range::new(), &action, iters, depth);
        // This tests how we respond when we have As6c on the preflop and the opponent
        // opens with Bet 200
        let infoset = InfoSet::from_hand(
            &str2cards("As6c"),
            &Vec::new(),
            &ActionHistory::from_strings(vec!["Bet 200"]),
        );
        let node: Node = nodes.get(&infoset).unwrap().clone();
        node.cumulative_strategy()
    };
    let actions = history.next_actions(&CONFIG.bet_abstraction).to_vec();
    let full_strat = solve_spot(10_000_000, -1);

    for depth in 1..7 {
        let depth_strat = solve_spot(10_000, depth);
        println!("MSE of strategies at depth {}: {}", depth, {
            let mut sum = 0.0;
            for (a, b) in full_strat.iter().zip(depth_strat.iter()) {
                sum += (a - b).powf(2.0);
            }
            sum / full_strat.len() as f64
        });
    }

    for iters in [1_000, 10_000, 100_000, 1_000_000] {
        let iters_strat = solve_spot(iters, 5);
        println!("MSE of strategies at iters {}: {}", iters, {
            let mut sum = 0.0;
            for (a, b) in full_strat.iter().zip(iters_strat.iter()) {
                sum += (a - b).powf(2.0);
            }
            sum / full_strat.len() as f64
        });
    }

    let depth_strat = solve_spot(10_000_000, 5);
    assert!(
        full_strat
            .iter()
            .zip(depth_strat.iter())
            .all(|(a, b)| { (a - b).abs() < 0.1 }),
        "{:?}",
        actions
    );
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
    let turn = load_turn_isomorphic();
    let river = load_river_isomorphic();
    assert_eq!(flop.len(), 1342562);
    assert_eq!(turn.len(), 14403610);
    assert_eq!(river.len(), 125756657);
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

// Fully populate the nodes to check if it will fit in memory. If not, the test will crash
// because the computer ran out of memory.
// #[test]
fn node_memory_stress_test() {
    let nodes: Nodes = Nodes::new();
    let histories = all_histories(&ActionHistory::new());
    println!("All histories: {}", histories.len());
    // Assuming there's 169 preflop buckets, and the same number of buckets for flop, turn, and river.
    // this could change in the future. 
    let bar = pbar((histories.len() as i32 * CONFIG.flop_buckets) as u64);
    histories.into_par_iter().for_each(|history| {
        if history.street == PREFLOP {
            for bucket in 0..169 {
                let infoset = InfoSet { history: history.clone(), card_bucket: bucket };
                let node = Node::new(infoset.next_actions(&CONFIG.bet_abstraction).len());
                nodes.insert(infoset, node);
                bar.inc(1);
            }
        } else {
            for bucket in 0..CONFIG.flop_buckets {
                let infoset = InfoSet { history: history.clone(), card_bucket: bucket };
                let node = Node::new(infoset.next_actions(&CONFIG.bet_abstraction).len());
                nodes.insert(infoset, node);
                bar.inc(1);
            }
        }
    });
    bar.finish_with_message("Success!");
}

#[test]
fn all_in_showdown_street() {
    let history = ActionHistory::from_strings(vec![
        "Call 100",
        "Bet 20000",
        "Call 19900"
    ]);
    assert_eq!(history.street, SHOWDOWN);
}