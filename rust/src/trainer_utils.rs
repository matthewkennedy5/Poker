use crate::card_abstraction;
use crate::card_utils;
use crate::card_utils::Card;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::cmp::Eq;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::fs::File;
use std::io::Write;

pub const SMALL_BLIND: i32 = 50;
pub const BIG_BLIND: i32 = 100;
pub const STACK_SIZE: i32 = 200 * BIG_BLIND;

pub const PREFLOP: usize = 0;
pub const FLOP: usize = 1;
pub const TURN: usize = 2;
pub const RIVER: usize = 3;

pub const DEALER: usize = 0;
pub const OPPONENT: usize = 1;
pub const FOLD: Action = Action {
    action: ActionType::Fold,
    amount: 0,
};

// Allowed bets in terms of pot fractions. We mark the all-in action as -1.
pub const ALL_IN: f64 = -1.0;
pub const BET_ABSTRACTION: [f64; 5] = [0.33, 0.67, 1.0, 2.5, ALL_IN];
// pub const BET_ABSTRACTION: [f64; 4] = [0.5, 1.0, 2.0, ALL_IN];
// pub const BET_ABSTRACTION: [f64; 2] = [1.0, ALL_IN];

// Discounted Regret Minimization parameters
const ALPHA: f64 = 1.5;
const BETA: f64 = 0.0;
const GAMMA: f64 = 2.0;

pub const BLUEPRINT_STRATEGY_PATH: &str = "products/compact_blueprint.bin";

lazy_static! {
    pub static ref ABSTRACTION: card_abstraction::Abstraction = card_abstraction::Abstraction::new();
    pub static ref HAND_TABLE: card_utils::HandTable = card_utils::HandTable::new();

    // pub static ref ABSTRACTION: card_abstraction::LightAbstraction = card_abstraction::LightAbstraction::new();
    // pub static ref HAND_TABLE: card_utils::LightHandTable = card_utils::LightHandTable::new();

}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub enum ActionType {
    Fold,
    Call,
    Bet,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub struct Action {
    pub action: ActionType,
    pub amount: i32,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let a = match &self.action {
            ActionType::Fold => "fold",
            ActionType::Call => "call",
            ActionType::Bet => "bet",
        };
        write!(f, "{} {}", a, self.amount)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionHistory {
    history: Vec<Vec<Action>>, // Each index is a street
    last_action: Option<Action>,
    pub street: usize,
    pub player: usize,
    stacks: [i32; 2],
}

impl ActionHistory {
    pub fn new() -> ActionHistory {
        ActionHistory {
            history: vec![Vec::new(); 4],
            street: PREFLOP,
            last_action: None,
            player: DEALER,
            stacks: [STACK_SIZE, STACK_SIZE],
        }
    }

    // Returns true if the hand is over (either someone has folded or it's time for
    // a showdown).
    pub fn hand_over(&self) -> bool {
        match &self.last_action {
            None => {}
            Some(action) => {
                if action.action == ActionType::Fold {
                    // The last player folded
                    return true;
                }
            }
        }
        let stacks = self.stack_sizes();
        if stacks == [0, 0] {
            // All-in action has happened
            return true;
        }
        if self.street > RIVER {
            // Showdown
            return true;
        }
        return false;
    }

    // Add an new action to this history, and update the state
    pub fn add(&mut self, action: &Action) {
        let action = action.clone();
        self.stacks[self.player] -= action.amount;
        self.player = 1 - self.player;
        self.last_action = Some(action.clone());
        self.history[self.street].push(action);
        if self.stacks[0] == self.stacks[1] && self.history[self.street].len() >= 2 {
            self.street += 1;
            self.player = OPPONENT;
        }
    }

    pub fn last_action(&self) -> Option<Action> {
        self.last_action.clone()
    }

    pub fn stack_sizes(&self) -> [i32; 2] {
        self.stacks
    }

    pub fn pot(&self) -> i32 {
        let pot = 2 * STACK_SIZE - self.stacks[0] - self.stacks[1];
        if pot == 0 {
            BIG_BLIND
        } else {
            pot
        }
    }

    // Returns the amount needed to call, so 0 for checking
    pub fn to_call(&self) -> i32 {
        if self.street == PREFLOP && self.history[PREFLOP].len() == 0 {
            BIG_BLIND
        } else {
            self.stacks[self.player] - self.stacks[1 - self.player]
        }
    }

    pub fn min_bet(&self) -> i32 {
        match &self.last_action {
            Some(action) => 2 * action.amount,
            None => BIG_BLIND,
        }
    }

    pub fn is_bet_legal(&self, bet: i32) -> bool {
        let max_bet = self.stacks[self.player];
        return self.min_bet() <= bet && bet <= max_bet;
    }

    // Returns a vector of the possible next actions after this state, that are
    // allowed in our action abstraction.
    pub fn next_actions(&self, bet_abstraction: &Vec<f64>) -> Vec<Action> {
        let mut actions = Vec::new();
        let min_bet = match &self.last_action {
            Some(action) => 2 * action.amount,
            None => BIG_BLIND,
        };
        let max_bet = self.stacks[self.player];
        let pot = self.pot();
        for fraction in bet_abstraction.iter() {
            let bet = if fraction == &ALL_IN {
                self.stacks[self.player]
            } else {
                (fraction.clone() * (pot as f64)) as i32
            };
            // Add the bet if the amount is legal and it's distinct from the
            // call amount.
            if min_bet <= bet && bet <= max_bet && bet != self.to_call() {
                actions.push(Action {
                    action: ActionType::Bet,
                    amount: bet,
                });
            }
        }

        // Add call/check action.
        let to_call = self.to_call();
        actions.push(Action {
            action: ActionType::Call,
            amount: to_call,
        });
        // Add the fold action, unless we can just check
        if to_call > 0 {
            actions.push(FOLD)
        }

        actions
    }

    // Performs action translation and returns a translated version of the
    // current history, with actions mapped to those of the given bet abstraction.
    // This assumes that folding and calling are always going to be implicitly
    // allowed in the abstraction.
    pub fn translate(&self, bet_abstraction: &Vec<f64>) -> ActionHistory {
        let mut translated = ActionHistory::new();
        for street in self.history.clone() {
            for action in street {
                let next = translated.next_actions(bet_abstraction);
                if next.contains(&action) {
                    translated.add(&action);
                } else {
                    // The action is not in the abstraction--time to perform
                    // action translation by finding the closest action.
                    let mut closest_action = next[0].clone();
                    for candidate_action in next {
                        if (candidate_action.amount - action.amount).abs()
                            < (closest_action.amount - action.amount).abs()
                        {
                            closest_action = candidate_action;
                        }
                    }
                    translated.add(&closest_action);
                }
            }
        }
        translated
    }

    pub fn compress(&self, bet_abstraction: &Vec<f64>) -> Vec<u8> {
        let mut compressed = Vec::new();
        let mut builder = ActionHistory::new();
        for street in self.history.clone() {
            for action in street {
                for (i, candidate) in builder.next_actions(bet_abstraction).iter().enumerate() {
                    if action == candidate.clone() {
                        compressed.push(i as u8);
                        break
                    }
                }
                builder.add(&action);
            }
        }
        compressed
    }
}

impl fmt::Display for ActionHistory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::new();
        for street in &self.history {
            for action in street.to_vec() {
                result.push_str(&action.to_string());
                result.push_str(",");
            }
            result.push_str(";");
        }
        write!(f, "{}", result)
    }
}

pub fn get_hand(deck: &[Card], player: usize, street: usize) -> Vec<Card> {
    // In this implementation, the deck cards are defined as follows:
    // dealer1 dealer2 opponent1 opponent2 flop1 flop2 flop3 turn river
    let hole = match player {
        DEALER => &deck[0..2],
        OPPONENT => &deck[2..4],
        _ => panic!("Bad player ID"),
    };
    let board = match street {
        PREFLOP => &[],
        FLOP => &deck[4..7],
        TURN => &deck[4..8],
        RIVER => &deck[4..9],
        _ => panic!("Invalid street"),
    };
    let cards = [hole, board].concat();
    cards
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub struct InfoSet {
    pub history: ActionHistory,
    card_bucket: i32,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompactInfoSet {
    history: Vec<u8>,
    card_bucket: i32,
}

impl InfoSet {
    // The dealer's cards are the first two cards in the deck, and the opponent's
    // are the second two cards. They are followed by the 5 board cards.
    pub fn from_deck(deck: &[Card], history: &ActionHistory) -> InfoSet {
        let cards = get_hand(&deck, history.player, history.street);
        let card_bucket = ABSTRACTION.bin(&cards);
        InfoSet {
            history: history.clone(),
            card_bucket: card_bucket,
        }
    }

    pub fn from_hand(hand: &[Card], history: &ActionHistory) -> InfoSet {
        // hand should contain the exact right number of cards for the current
        // street of the history. Maybe do error checking for this in the future.
        InfoSet {
            history: history.clone(),
            card_bucket: ABSTRACTION.bin(hand),
        }
    }

    pub fn next_actions(&self) -> Vec<Action> {
        self.history.next_actions(&BET_ABSTRACTION.to_vec())
    }

    pub fn compress(&self) -> CompactInfoSet {
        CompactInfoSet {
            history: self.history.compress(&BET_ABSTRACTION.to_vec()),
            card_bucket: self.card_bucket,
        }
    }
}

impl fmt::Display for InfoSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let card_display = hand_with_bucket(self.card_bucket, self.history.street);
        write!(f, "{}|{}", card_display, self.history.to_string())
    }
}

// Returns a representative hand which is in the given abstraction bucket.
fn hand_with_bucket(bucket: i32, street: usize) -> String {
    let mut deck = card_utils::deck();
    let mut rng = thread_rng();
    loop {
        deck.shuffle(&mut rng);
        let hand = get_hand(&deck, 0, street);
        if ABSTRACTION.bin(&hand) == bucket {
            return card_utils::cards2str(&hand);
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    regrets: HashMap<Action, f64>,
    strategy_sum: HashMap<Action, f64>,
    pub t: f64,
}

impl Node {
    pub fn new(infoset: &InfoSet) -> Node {
        // Create a HashMap of action -> 0.0 to initialize the regrets and
        // cumulative strategy sum
        let mut zeros = HashMap::new();
        for action in infoset.next_actions() {
            zeros.insert(action, 0.0);
        }
        Node {
            regrets: zeros.clone(),
            strategy_sum: zeros,
            t: 0.0,
        }
    }

    pub fn current_strategy(&mut self, prob: f64) -> HashMap<Action, f64> {
        let mut strat: HashMap<Action, f64> = HashMap::new();
        for (action, regret) in self.regrets.clone() {
            if regret > 0.0 {
                strat.insert(action, regret);
            } else {
                strat.insert(action, 0.0);
            }
        }
        strat = normalize(&strat);
        for action in strat.keys() {
            // Add this action's probability to the cumulative strategy sum
            let sum_prob = self.strategy_sum.get(action).unwrap().clone();
            let new_prob = strat.get(action).unwrap() * prob;
            let mut cumulative_strategy = sum_prob + new_prob;
            // Multiply the cumulative strategy sum according to Discounted
            // Counterfactual Regret Minimization
            cumulative_strategy *= (self.t / (self.t + 1.0)).powf(GAMMA);
            self.strategy_sum
                .insert(action.clone(), cumulative_strategy);
        }
        if prob > 0.0 {
            self.t += 1.0;
        }
        strat
    }

    pub fn cumulative_strategy(&self) -> HashMap<Action, f64> {
        // TODO: DCFR
        normalize(&self.strategy_sum)
    }

    pub fn add_regret(&mut self, action: &Action, regret: f64) {
        let mut accumulated_regret = self.regrets[action] + regret;
        // Update the accumulated regret according to Discounted Counterfactual
        // Regret Minimization rules
        if accumulated_regret >= 0.0 {
            accumulated_regret *= self.t.powf(ALPHA) / (self.t.powf(ALPHA) + 1.0);
        } else {
            accumulated_regret *= self.t.powf(BETA) / (self.t.powf(BETA) + 1.0);
        }
        self.regrets.insert(action.clone(), accumulated_regret);
    }
}

// Normalizes the values of a HashMap so that its elements sum to 1.
pub fn normalize<T: Eq + Hash + Clone>(map: &HashMap<T, f64>) -> HashMap<T, f64> {
    let mut map = map.clone();
    let mut sum = 0.0;
    for elem in map.values() {
        sum += elem;
    }
    for (action, val) in map.clone() {
        let newval = if sum == 0.0 {
            // If all values are 0, then just return a uniform distribution
            1.0 / map.len() as f64
        } else {
            // Otherwise normalize based on the sum.
            val / sum
        };
        map.insert(action.clone(), newval);
    }
    map
}

// Randomly sample an action given the strategy at this node.
pub fn sample_action_from_node(node: &Node) -> Action {
    let node = &mut node.clone();
    let strategy = node.current_strategy(0.0);
    sample_action_from_strategy(&strategy)
}

pub fn sample_action_from_strategy(strategy: &HashMap<Action, f64>) -> Action {
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
pub fn terminal_utility(deck: &[Card], history: ActionHistory, player: usize) -> f64 {
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
    let player_strength = HAND_TABLE.hand_strength(&player_hand);
    let opponent_strength = HAND_TABLE.hand_strength(&opponent_hand);

    if player_strength > opponent_strength {
        return (pot / 2) as f64;
    } else if player_strength < opponent_strength {
        return (-pot / 2) as f64;
    } else {
        // It's a tie: player_strength == opponent_strength
        return 0.0;
    }
}

// Presamples actions and represents the blueprint strategy in a much more
// compact format.
pub fn write_compact_blueprint(nodes: &HashMap<InfoSet, Node>) {
    let mut compressed = HashMap::new();
    println!("[INFO] Compressing the blueprint strategy");
    let bar = card_utils::pbar(nodes.len() as u64);
    for (infoset, node) in nodes {
        let action = sample_action_from_node(&node);
        compressed.insert(infoset.compress(), action);
        bar.inc(1);
    }
    bar.finish();
    let bincode: Vec<u8> = bincode::serialize(&compressed).unwrap();
    let mut file = File::create(BLUEPRINT_STRATEGY_PATH).unwrap();
    file.write_all(&bincode).unwrap();
    println!("[INFO] Wrote compressed blueprint strategy to disk");
}
