use crate::card_abstraction::Abstraction;
use crate::card_utils::*;
use crate::config::CONFIG;
use once_cell::sync::Lazy;
use dashmap::DashMap;
use rand::{prelude::SliceRandom, thread_rng};
use std::{cmp::Eq, collections::HashMap, fmt, hash::Hash};

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
pub const ALL_IN: f64 = -1.0;

pub static ABSTRACTION: Lazy<Abstraction> = Lazy::new(Abstraction::new);

pub type Nodes = DashMap<InfoSet, Node>;

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
    pub stacks: [i32; 2],
}

impl ActionHistory {
    pub fn new() -> ActionHistory {
        ActionHistory {
            history: vec![Vec::new(); 4],
            street: PREFLOP,
            last_action: None,
            player: DEALER,
            stacks: [CONFIG.stack_size, CONFIG.stack_size],
        }
    }

    // Example:
    // let history = ActionHistory::from_strings(vec!["Bet 200", "Call 200"]);
    pub fn from_strings(actions: Vec<&str>) -> ActionHistory {
        let mut history = ActionHistory::new();
        for str in actions {
            let tokens: Vec<&str> = str.split(' ').collect();
            let action = match tokens[0] {
                "Bet" => ActionType::Bet,
                "Call" => ActionType::Call,
                "Fold" => ActionType::Fold,
                _ => panic!("Bad action string"),
            };
            let amount: i32 = tokens[1].parse().expect("Bad action amount");
            history.add(&Action {
                action,
                amount,
            });
        }
        history
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
        false
    }

    // Add an new action to this history, and update the state
    pub fn add(&mut self, action: &Action) {
        assert!(
            self.is_legal_next_action(action),
            "Action {:?} is illegal for history {:#?}",
            action,
            self
        );
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

    pub fn is_legal_next_action(&self, action: &Action) -> bool {
        match action.action {
            ActionType::Bet => {
                // The bet size must be different than the call size, and within the correct range
                let not_call = action.amount != self.to_call();
                let size_ok =
                    (action.amount >= self.min_bet()) && (action.amount <= self.max_bet());
                size_ok && not_call
            }
            ActionType::Call => action.amount == self.to_call(),
            ActionType::Fold => self.to_call() != 0,
        }
    }

    pub fn last_action(&self) -> Option<Action> {
        self.last_action.clone()
    }

    // TODO: The stack sizes should reflect that the blinds are posted in the beginning. So they start
    // as like 19000 or whatever.
    pub fn stack_sizes(&self) -> [i32; 2] {
        self.stacks
    }

    pub fn pot(&self) -> i32 {
        let pot = 2 * CONFIG.stack_size - self.stacks[0] - self.stacks[1];
        if pot == 0 {
            CONFIG.big_blind
        } else {
            pot
        }
    }

    // Returns the amount needed to call, so 0 for checking
    pub fn to_call(&self) -> i32 {
        if self.street == PREFLOP && self.history[PREFLOP].is_empty() {
            CONFIG.big_blind
        } else {
            self.stacks[self.player] - self.stacks[1 - self.player]
        }
    }

    pub fn min_bet(&self) -> i32 {
        if self.history[PREFLOP].is_empty() {
            CONFIG.big_blind
        } else if self.history[self.street].is_empty() {
            0
        } else {
            let last_action: Action = self.last_action.clone().unwrap();
            if 2 * last_action.amount > self.max_bet() {
                // Can always go all-in
                self.max_bet()
            } else {
                2 * last_action.amount
            }
        }
    }

    pub fn max_bet(&self) -> i32 {
        self.stacks[self.player]
    }

    // Returns a vector of the possible next actions after this state, that are
    // allowed in our action abstraction.
    pub fn next_actions(&self, bet_abstraction: &[Vec<f64>]) -> Vec<Action> {
        // Add all the potential bet sizes in the abstraction, and call and fold actions.
        // Then later we filter out the illegal actions.
        let mut candidate_actions = Vec::new();
        let pot = self.pot();
        for fraction in bet_abstraction[self.street].iter() {
            let bet = if fraction == &ALL_IN {
                self.stacks[self.player]
            } else {
                (*fraction * (pot as f64)) as i32
            };
            candidate_actions.push(Action {
                action: ActionType::Bet,
                amount: bet,
            });
        }
        candidate_actions.push(Action {
            action: ActionType::Call,
            amount: self.to_call(),
        });
        candidate_actions.push(FOLD);
        candidate_actions.retain(|a| self.is_legal_next_action(a));

        candidate_actions
    }

    pub fn is_empty(&self) -> bool {
        self.history[PREFLOP].len() == 0
    }

    pub fn without_last_action(&self) -> ActionHistory {
        if self.is_empty() {
            panic!("Can't remove last action from empty history");
        }
        // Remove the last action from the history
        let mut prev_history = self.clone();
        if prev_history.history[self.street].is_empty() {
            prev_history.street -= 1;
        }
        prev_history.history[prev_history.street].pop();

        // There's a lot of complex rules, so just create a new history and add all the actions
        // in prev_history.
        let mut history = ActionHistory::new();
        for action in prev_history.get_actions() {
            history.add(&action);
        }

        // Check that we recover the original history when we add back the last action
        let mut added = history.clone();
        added.add(&self.last_action().unwrap());
        assert!(added == self.clone());

        history
    }

    // Performs action translation and returns a translated version of the
    // current history, with actions mapped to those of the given bet abstraction.
    // This assumes that folding and calling are always going to be implicitly
    // allowed in the abstraction.
    pub fn translate(&self, bet_abstraction: &Vec<Vec<f64>>) -> ActionHistory {
        let mut translated = ActionHistory::new();
        let mut untranslated = ActionHistory::new();
        for action in self.get_actions() {
            let translated_next_actions = translated.next_actions(bet_abstraction);
            let translated_action;
            if action.action == ActionType::Fold {
                translated_action = FOLD;
            } else if action.action == ActionType::Call {
                translated_action = Action {
                    action: ActionType::Call,
                    amount: translated.to_call(),
                };
            } else if action.action == ActionType::Bet {
                if action.amount == untranslated.max_bet() {
                    // All in
                    translated_action = Action {
                        action: ActionType::Bet,
                        amount: translated.max_bet(),
                    };
                } else {
                    // To translate the bet, find the bet size in the abstraction which is closest in
                    // log space to the real bet size. This does not include the all-in action, because
                    // that would end the hand.
                    let mut candidate_bets = Vec::new();
                    for a in translated_next_actions {
                        if a.action == ActionType::Bet && a.amount != translated.max_bet() {
                            candidate_bets.push(a.amount);
                        }
                    }
                    if candidate_bets.is_empty() {
                        // The only legal bet size in the abstraction is the all-in amount, but
                        // we don't want to end the hand, so we reduce the bet size slightly.
                        candidate_bets.push(translated.max_bet() - CONFIG.big_blind);
                    }
                    let closest_bet_size = find_closest_log(candidate_bets, action.amount);
                    translated_action = Action {
                        action: ActionType::Bet,
                        amount: closest_bet_size,
                    };
                }
            } else {
                panic!("Action not translating");
            }
            translated.add(&translated_action);
            untranslated.add(&action);
        }
        assert!(
            translated.street == self.street,
            "Different streets: History: {}\nTranslated: {}",
            self,
            translated
        );
        translated
    }

    pub fn adjust_action(&self, action: &Action) -> Action {
        // The translated action is based off a misunderstanding off the true bet
        // sizes, so we may have to adjust our call amount to line up with what's
        // actually in the pot as opposed to our approximation.
        let mut adjusted = action.clone();
        if action.action == ActionType::Call {
            adjusted.amount = self.to_call();
        } else if action.action == ActionType::Bet {
            if action.amount > self.max_bet() {
                adjusted.amount = self.max_bet();
            } else if action.amount < self.min_bet() {
                adjusted.amount = self.min_bet();
            }
        }
        adjusted
    }

    pub fn get_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        for street in self.history.clone() {
            for action in street {
                actions.push(action);
            }
        }
        actions
    }
}

impl fmt::Display for ActionHistory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::new();
        for street in &self.history {
            for action in street {
                result.push_str(&action.to_string());
                result.push(',');
            }
            result.push(';');
        }
        write!(f, "{result}")
    }
}

// Returns the element which is closest in log space to the input
fn find_closest_log(v: Vec<i32>, n: i32) -> i32 {
    assert!(!v.is_empty());
    let log_n = (n as f64).ln();
    let mut closest_v = 0;
    let mut log_closest_diff = f64::MAX;
    for candidate in v {
        let log_candidate_diff = ((candidate as f64).ln() - log_n).abs();
        if log_candidate_diff < log_closest_diff {
            closest_v = candidate;
            log_closest_diff = log_candidate_diff;
        }
    }
    closest_v
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
    
    [hole, board].concat()
}

pub fn board_length(street: usize) -> usize {
    match street {
        PREFLOP => 0,
        FLOP => 3,
        TURN => 4,
        RIVER => 5,
        _ => panic!("Bad street"),
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub struct InfoSet {
    pub history: ActionHistory,
    pub card_bucket: i32,
}

impl InfoSet {
    // The dealer's cards are the first two cards in the deck, and the opponent's
    // are the second two cards. They are followed by the 5 board cards.
    pub fn from_deck(deck: &[Card], history: &ActionHistory) -> InfoSet {
        let cards = get_hand(deck, history.player, history.street);
        let card_bucket = ABSTRACTION.bin(&cards);
        InfoSet {
            history: history.clone(),
            card_bucket,
        }
    }

    pub fn from_hand(hole: &[Card], board: &[Card], history: &ActionHistory) -> InfoSet {
        assert!(!board.contains(&hole[0]) && !board.contains(&hole[1]));
        let board = &board[..board_length(history.street)];
        assert!(board.len() == board_length(history.street));
        let hand = [hole, board].concat();
        InfoSet {
            history: history.clone(),
            card_bucket: ABSTRACTION.bin(&hand),
        }
    }

    pub fn next_actions(&self, bet_abstraction: &Vec<Vec<f64>>) -> Vec<Action> {
        self.history.next_actions(bet_abstraction)
    }
}

impl fmt::Display for InfoSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let card_display = hand_with_bucket(self.card_bucket, self.history.street);
        write!(f, "{}|{}", card_display, self.history)
    }
}

// Returns a representative hand which is in the given abstraction bucket.
fn hand_with_bucket(bucket: i32, street: usize) -> String {
    let mut deck = deck();
    let mut rng = thread_rng();
    loop {
        deck.shuffle(&mut rng);
        let hand = get_hand(&deck, 0, street);
        if ABSTRACTION.bin(&hand) == bucket {
            return cards2str(&hand);
        }
    }
}

pub fn lookup_or_new(
    nodes: &Nodes,
    infoset: &InfoSet,
    bet_abstraction: &Vec<Vec<f64>>,
) -> Node {
    match nodes.get(infoset) {
        Some(n) => n.clone(),
        None => Node::new(infoset, bet_abstraction),
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub regrets: Vec<f64>,
    strategy_sum: Vec<f64>,
    pub actions: Vec<Action>,
    pub t: f64,
}

impl Node {
    pub fn new(infoset: &InfoSet, bet_abstraction: &Vec<Vec<f64>>) -> Node {
        // Create a HashMap of action -> 0.0 to initialize the regrets and
        // cumulative strategy sum
        let actions = infoset.next_actions(bet_abstraction);
        Node {
            regrets: vec![0.0; actions.len()],
            strategy_sum: vec![0.0; actions.len()],
            actions,
            t: 0.0,
        }
    }

    // Returns the current strategy for this node, and updates the cumulative strategy
    // as a side effect.
    // Input: prob is the probability of reaching this node
    pub fn current_strategy(&mut self, prob: f64) -> Vec<f64> {
        // Normalize the regrets for this iteration of CFR
        let positive_regrets: Vec<f64> = self
            .regrets
            .iter()
            .map(|r| if *r >= 0.0 { *r } else { 0.0 })
            .collect();
        let regret_norm: Vec<f64> = normalize_vec(&positive_regrets);
        for i in 0..regret_norm.len() {
            // Add this action's probability to the cumulative strategy sum using DCFR+ update rules
            let new_prob = regret_norm[i] * prob;
            let weight = if self.t < 100.0 { 0.0 } else { self.t - 100.0 };
            self.strategy_sum[i] += weight * new_prob;
        }
        if prob > 0.0 {
            self.t += 1.0;
        }
        regret_norm
    }

    pub fn cumulative_strategy(&self) -> Vec<f64> {
        normalize_vec(&self.strategy_sum)
    }

    pub fn add_regret(&mut self, action_index: usize, regret: f64) {
        let mut accumulated_regret = self.regrets[action_index] + regret;
        // Update the accumulated regret according to Discounted Counterfactual
        // Regret Minimization rules
        if accumulated_regret >= 0.0 {
            accumulated_regret *= self.t.powf(CONFIG.alpha) / (self.t.powf(CONFIG.alpha) + 1.0);
        } else {
            accumulated_regret *= self.t.powf(CONFIG.beta) / (self.t.powf(CONFIG.beta) + 1.0);
        }
        self.regrets[action_index] = accumulated_regret;
    }
}

// Normalizes the values of a HashMap so that its elements sum to 1.
// TODO: Remove this in favor of normalize_vec
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

fn normalize_vec(v: &[f64]) -> Vec<f64> {
    for elem in v {
        assert!(*elem >= 0.0);
    }
    let sum: f64 = v.iter().sum();
    let norm: Vec<f64> = v
        .iter()
        .map(|e| {
            if sum == 0.0 {
                // If all values are 0, then just return a uniform distribution
                1.0 / v.len() as f64
            } else {
                e / sum
            }
        })
        .collect();
    let norm_sum: f64 = norm.iter().sum();
    assert!(
        (norm_sum - 1.0).abs() < 1e-6,
        "Sum of normalized vector: {}. Input vector: {:?}",
        norm_sum,
        v
    );
    norm
}

// Randomly sample an action given the strategy at this node.
pub fn sample_action_from_node(node: &mut Node) -> Action {
    let strategy = node.current_strategy(0.0);
    let action_indexes: Vec<usize> = (0..node.actions.len()).collect();
    let index: usize = *action_indexes
        .choose_weighted(&mut thread_rng(), |i| strategy[(*i)])
        .unwrap_or_else(|_| panic!("Invalid strategy distribution: {:?}", &strategy));
    node.actions.get(index).unwrap().clone()
}

pub fn sample_action_from_strategy(strategy: &HashMap<Action, f64>) -> Action {
    let actions: Vec<&Action> = strategy.keys().collect();
    let mut rng = thread_rng();
    let action = (*actions
        .choose_weighted(&mut rng, |a| strategy.get(a).unwrap())
        .unwrap())
        .clone();
    action
}

// Assuming history represents a terminal state (someone folded, or it's a showdown),
// return the utility, in chips, that the given player gets.
pub fn terminal_utility(deck: &[Card], history: &ActionHistory, player: usize) -> f64 {
    let opponent = 1 - player;
    if history.last_action().unwrap().action == ActionType::Fold {
        // Someone folded -- assign the chips to the winner.
        let winner = history.player;
        let folder = 1 - winner;
        let mut winnings: f64 = (CONFIG.stack_size - history.stack_sizes()[folder]) as f64;

        // If someone folded on the first preflop round, they lose their blind
        if winnings == 0.0 {
            winnings += match folder {
                DEALER => CONFIG.small_blind as f64,
                OPPONENT => CONFIG.big_blind as f64,
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
    let player_hand = get_hand(deck, player, RIVER);
    let opponent_hand = get_hand(deck, opponent, RIVER);
    let player_strength = FAST_HAND_TABLE.hand_strength(&player_hand);
    let opponent_strength = FAST_HAND_TABLE.hand_strength(&opponent_hand);

    if player_strength > opponent_strength {
        (pot / 2) as f64
    } else if player_strength < opponent_strength {
        return (-pot / 2) as f64;
    } else {
        // It's a tie: player_strength == opponent_strength
        return 0.0;
    }
}

// For making preflop charts
// TODO: Use a bot instance -- this shouldnt depend on the node implementation. Orthogonality!
// pub fn write_preflop_strategy(nodes: &Nodes, path: &str) {
//     let mut preflop_strategy: HashMap<String, HashMap<String, f64>> = HashMap::new();
//     for (infoset, node) in nodes {
//         if infoset.history.is_empty() {
//             let hand = Abstraction::preflop_hand(infoset.card_bucket);
//             let strategy: HashMap<String, f64> = node
//                 .cumulative_strategy()
//                 .iter()
//                 .map(|(action, prob)| (action.to_string(), *prob))
//                 .collect();

//             preflop_strategy.insert(hand, strategy);
//         }
//     }
//     // Write the preflop strategy to a JSON
//     let json = serde_json::to_string_pretty(&preflop_strategy).unwrap();
//     let mut file = File::create(&path).unwrap();
//     file.write(json.as_bytes()).unwrap();
// }
