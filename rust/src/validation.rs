// Sanity checks for the blueprint strategy.

use crate::bot;
use crate::card_utils::{cards2str, pbar, Card};
use crate::trainer_utils::*;

// Displays the preflop strategy matrix for opening / raising.
pub fn preflop_matrix() {
    let dealer_history = ActionHistory::new();
    let mut p2_history = ActionHistory::new();
    p2_history.add(&Action {
        action: ActionType::Bet,
        amount: 250,
    });
    let mut hands = Vec::new();
    for rank in 2..15 {
        for rank2 in rank..15 {
            let off_suit = vec![
                Card {
                    suit: 0,
                    rank: rank,
                },
                Card {
                    suit: 1,
                    rank: rank2,
                },
            ];
            hands.push(off_suit);
            if rank != rank2 {
                let suited = vec![
                    Card {
                        suit: 0,
                        rank: rank,
                    },
                    Card {
                        suit: 0,
                        rank: rank2,
                    },
                ];
                hands.push(suited);
            }
        }
    }

    // let nodes = crate::trainer::load_nodes();
    let fold = Action {
        action: ActionType::Fold,
        amount: 0,
    };
    let call = Action {
        action: ActionType::Call,
        amount: 100,
    };
    let raise = Action {
        action: ActionType::Bet,
        amount: 250,
    };
    let all_in = Action {
        action: ActionType::Bet,
        amount: 20000,
    };

    println!("Dealer's opening strategy: ");
    for hand in &hands {
        // let infoset = InfoSet::from_hand(&hand, &dealer_history);
        // let node = nodes.get(&infoset).unwrap();
        // let strat = node.cumulative_strategy();
        // println!("{}: {:#?}, t={}", cards2str(&hand), strat, node.t);
        // println!(
        //     "{}
        //             Fold:      {}
        //             Call 100:  {}
        //             Raise 250: {}
        //             All-in:    {}",
        //     cards2str(&hand),
        //     strat[&fold],
        //     strat[&call],
        //     strat[&raise],
        //     strat[&all_in]
        // );
        let action = bot::bot_action(&hand, &vec![], &dealer_history);
        println!("{}: {}", cards2str(&hand), action);
    }
    // println!("\nBig Blind's response to raise of 250:");
    // let fold = Action {
    //     action: ActionType::Fold,
    //     amount: 0,
    // };
    // let call = Action {
    //     action: ActionType::Call,
    //     amount: 250,
    // };
    // let raise = Action {
    //     action: ActionType::Bet,
    //     amount: 650,
    // };
    // let all_in = Action {
    //     action: ActionType::Bet,
    //     amount: 20000,
    // };

    // for hand in &hands {
    //     // let action = bot::bot_action(&hand, &vec![], &p2_history);
    //     // println!("{}: {}", cards2str(&hand), action);
    //     let infoset = InfoSet::from_hand(&hand, &p2_history);
    //     let node = nodes.get(&infoset).unwrap();
    //     let strat = node.cumulative_strategy();
    //     // println!("{}: {:#?}, t={}", cards2str(&hand), strat, node.t);
    //     println!(
    //         "{}
    //                 Fold:      {}
    //                 Call 250:  {}
    //                 Raise 650: {}
    //                 All-in:    {}",
    //         cards2str(&hand),
    //         strat[&fold],
    //         strat[&call],
    //         strat[&raise],
    //         strat[&all_in]
    //     );
    // }
    // TODO: Output a graphical preflop chart
}

// Calculates the average donk bet percentage across all hands on the flop.
// pub fn donk_percentage() {
//     let mut avg_prob = 0.0;
//     let mut n_infosets = 0;

//     let mut donk_history = ActionHistory::new();
//     donk_history.add(&Action {
//         action: ActionType::Bet,
//         amount: 250,
//     });
//     donk_history.add(&Action {
//         action: ActionType::Call,
//         amount: 250,
//     });
//     let donk_history = donk_history.compress(&BET_ABSTRACTION);
//     let nodes = crate::trainer::load_nodes();

//     for (infoset, node) in &nodes {
//         if infoset.history == donk_history {
//             let strat = node.cumulative_strategy();
//             for (action, prob) in &strat {
//                 if action.action == ActionType::Bet {
//                     avg_prob += prob;
//                 }
//             }
//             n_infosets += 1;
//         }
//     }
//     println!(
//         "Donk frequency: {:.1}%",
//         avg_prob / (n_infosets as f64) * 100.0
//     );
// }
