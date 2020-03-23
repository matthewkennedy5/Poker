use crate::card_abstraction::Abstraction;
use crate::card_utils;
use crate::card_utils::Card;
use crate::exploiter::exploitability;
use crate::trainer_utils::*;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::Write;

// TODO: Use a parameter file
const BLUEPRINT_STRATEGY_PATH: &str = "blueprint.bin";

lazy_static! {
    static ref HAND_TABLE: card_utils::LightHandTable = card_utils::LightHandTable::new();
}

pub fn train(iters: u64) {
    let mut deck = card_utils::deck();
    let mut rng = &mut rand::thread_rng();

    let mut nodes: HashMap<InfoSet, Node> = HashMap::new();
    lazy_static::initialize(&HAND_TABLE);

    println!("[INFO]: Beginning training.");
    let mut p0_util = 0.0;
    let mut p1_util = 0.0;
    let bar = card_utils::pbar(iters);
    for i in 0..iters {
        deck.shuffle(&mut rng);
        p0_util += iterate(DEALER, &deck, ActionHistory::new(), [1.0, 1.0], &mut nodes);
        deck.shuffle(&mut rng);
        p1_util += iterate(
            OPPONENT,
            &deck,
            ActionHistory::new(),
            [1.0, 1.0],
            &mut nodes,
        );
        bar.inc(1);
    }
    bar.finish();

    for (infoset, node) in &nodes {
        if infoset.history.street == PREFLOP {
            // if node.t > 1 {
            println!(
                "{}: {:#?}t: {}\n",
                infoset,
                node.cumulative_strategy(),
                node.t
            );
        }
    }
    println!("{} nodes reached.", nodes.len());
    println!(
        "Utilities: {}, {}",
        p0_util / (iters as f64),
        p1_util / (iters as f64)
    );

    println!("Exploitability: {}", exploitability(&nodes));
    serialize_strategy(&nodes);
}

fn serialize_strategy(nodes: &HashMap<InfoSet, Node>) {
    let bincode: Vec<u8> = bincode::serialize(nodes).unwrap();
    let mut file = File::create(BLUEPRINT_STRATEGY_PATH).unwrap();
    file.write_all(&bincode).unwrap();
}

fn iterate(
    player: usize,
    deck: &[Card],
    history: ActionHistory,
    weights: [f64; 2],
    nodes: &mut HashMap<InfoSet, Node>,
) -> f64 {
    if history.hand_over() {
        return terminal_utility(&deck, history, player);
    }

    // Look up the CFR node for this information set, or make a new one if it
    // doesn't exist
    let mut infoset = InfoSet::from_deck(&deck, &history);
    if !nodes.contains_key(&infoset) {
        let new_node = Node::new(&infoset);
        nodes.insert(infoset.clone(), new_node);
    }
    let mut node: Node = nodes.get(&infoset).unwrap().clone();
    let mut history = history.clone();

    let opponent = 1 - player;
    if history.player == opponent {
        // Process the opponent's turn
        history.add(sample_action(&node));

        if history.hand_over() {
            return terminal_utility(&deck, history, player);
        }
        infoset = InfoSet::from_deck(&deck, &history);
        if !nodes.contains_key(&infoset) {
            nodes.insert(infoset.clone(), Node::new(&infoset));
        }
        node = nodes.get(&infoset).unwrap().clone();
    }

    // Grab the current strategy at this node
    let [p0, p1] = weights;
    let strategy = node.current_strategy(weights[player]);
    let mut utilities: HashMap<Action, f64> = HashMap::new();
    let mut node_utility = 0.0;

    // Recurse to further nodes in the game tree. Find the utilities for each action.
    for (action, prob) in strategy {
        let mut next_history = history.clone();
        next_history.add(action.clone());
        let new_weights = match player {
            0 => [p0 * prob, p1],
            1 => [p0, p1 * prob],
            _ => panic!("Bad player value"),
        };
        let utility = iterate(player, &deck, next_history, new_weights, nodes);
        utilities.insert(action, utility);
        node_utility += prob * utility;
    }

    // Update regrets
    for (action, utility) in &utilities {
        let regret = utilities.get(&action).unwrap() - node_utility;
        node.add_regret(&action, weights[opponent] * regret);
    }

    let updated = node.clone();
    nodes.insert(infoset, updated);
    node_utility
}

// Randomly sample an action given the strategy at this node.
fn sample_action(node: &Node) -> Action {
    let node = &mut node.clone();
    let strategy = node.current_strategy(0.0);
    let actions: Vec<&Action> = strategy.keys().collect();
    let mut rng = thread_rng();
    let action = actions
        .choose_weighted(&mut rng, |a| strategy.get(&a).unwrap())
        .unwrap()
        .clone()
        .clone();
    action
}

// Assuming history represents a terminal state (someone folded, or it's a showdown),
// return the utility, in chips, that the given player gets.
fn terminal_utility(deck: &[Card], history: ActionHistory, player: usize) -> f64 {
    let opponent = 1 - player;
    if history.last_action().unwrap().action == ActionType::Fold {
        // Someone folded -- assign the chips to the winner.
        let winner = history.player;
        let folder = 1 - winner;
        let mut winnings: f64 = (STACK_SIZE - history.stack_sizes()[folder]) as f64;

        // If someone folded on the first preflop round, they lose their blind
        if winnings == 0.0 {
            winnings += match folder {
                DEALER => SMALL_BLIND as f64,
                OPPONENT => BIG_BLIND as f64,
                _ => panic!("Bad player number"),
            };
        }

        let util = if winner == player {
            winnings
        } else {
            -winnings
        };

        return util;
    }

    // Showdown time -- both players have contributed equally to the pot
    let pot = history.pot();
    let player_hand = get_hand(&deck, player, RIVER);
    let opponent_hand = get_hand(&deck, opponent, RIVER);

    // So player 0 always wins the showdown
    // let player_strength = 1 - player;
    // let opponent_strength = player;

    let player_strength = HAND_TABLE.hand_strength(&player_hand);
    let opponent_strength = HAND_TABLE.hand_strength(&opponent_hand);
    // let player_strength = 0;
    // let opponent_strength = 0;

    if player_strength > opponent_strength {
        return (pot / 2) as f64;
    } else if player_strength < opponent_strength {
        return (-pot / 2) as f64;
    } else {
        // It's a tie: player_strength == opponent_strength
        return 0.0;
    }
}
