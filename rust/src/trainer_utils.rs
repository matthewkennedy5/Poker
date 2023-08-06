use crate::card_abstraction::Abstraction;
use crate::card_utils::*;
use crate::config::CONFIG;
use crate::nodes::*;
use once_cell::sync::Lazy;
use rand::{prelude::SliceRandom, thread_rng};
use smallvec::SmallVec;
use std::{cmp::Eq, collections::HashMap, fmt, hash::Hash};

pub const PREFLOP: usize = 0;
pub const FLOP: usize = 1;
pub const TURN: usize = 2;
pub const RIVER: usize = 3;
pub const SHOWDOWN: usize = 4;

pub const DEALER: usize = 0;
pub const OPPONENT: usize = 1;

pub const FOLD: Action = Action {
    action: ActionType::Fold,
    amount: 0,
};
pub const ALL_IN: f64 = -1.0;

pub static ABSTRACTION: Lazy<Abstraction> = Lazy::new(Abstraction::new);

pub type Strategy = HashMap<Action, f64>;
pub type Amount = u16;

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub enum ActionType {
    Fold,
    Call,
    Bet,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, serde::Serialize, serde::Deserialize)]
pub struct Action {
    pub action: ActionType,
    pub amount: Amount,
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
    history: SmallVec<[Action; 10]>,
    last_action: Option<Action>,
    current_street_length: u8,
    stacks: [Amount; 2], // stacks doesn't take into account blinds, but stack_sizes() does
    pub street: usize,
    pub player: usize,
}

impl ActionHistory {
    pub fn new() -> ActionHistory {
        ActionHistory {
            history: SmallVec::with_capacity(10),
            last_action: None,
            current_street_length: 0,
            stacks: [CONFIG.stack_size, CONFIG.stack_size],
            street: PREFLOP,
            player: DEALER,
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
            let amount = tokens[1].parse().expect("Bad action amount");
            history.add(&Action { action, amount });
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
        debug_assert!(   // this one is slow
            self.is_legal_next_action(action),
            "Action {:?} is illegal for history {:#?}",
            action,
            self
        );
        let action = action.clone();
        self.stacks[self.player as usize] -= action.amount;
        self.player = 1 - self.player;
        self.last_action = Some(action.clone());
        self.history.push(action);
        self.current_street_length += 1;
        // The street is over if both players have acted and the bet sizes are equal
        if self.stacks[0] == self.stacks[1] && self.current_street_length >= 2 {
            self.street += 1;
            self.current_street_length = 0;
            self.player = OPPONENT;
        }
        if self.stacks[0] == 0 && self.stacks[1] == 0 {
            self.street = SHOWDOWN;
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

    pub fn stack_sizes(&self) -> [Amount; 2] {
        let mut stacks = self.stacks;
        if stacks[DEALER] == CONFIG.stack_size {
            stacks[DEALER] -= CONFIG.small_blind;
        }
        if stacks[OPPONENT] == CONFIG.stack_size {
            stacks[OPPONENT] -= CONFIG.big_blind;
        }
        stacks
    }

    pub fn pot(&self) -> Amount {
        let pot = 2 * CONFIG.stack_size - self.stacks[0] - self.stacks[1];
        if pot == 0 {
            CONFIG.big_blind
        } else {
            pot
        }
    }

    // Returns the amount needed to call, so 0 for checking
    pub fn to_call(&self) -> Amount {
        if self.street == PREFLOP && self.history.is_empty() {
            CONFIG.big_blind
        } else {
            self.stacks[self.player] - self.stacks[1 - self.player]
        }
    }

    pub fn min_bet(&self) -> Amount {
        if self.history.is_empty() {
            CONFIG.big_blind
        } else if self.current_street_length == 0 {
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

    pub fn max_bet(&self) -> Amount {
        self.stacks[self.player]
    }

    // Returns a vector of the possible next actions after this state, that are
    // allowed in our action abstraction.
    pub fn next_actions(&self, bet_abstraction: &[Vec<f64>]) -> SmallVec<[Action; NUM_ACTIONS]> {
        // Add all the potential bet sizes in the abstraction, and call and fold actions.
        // Then later we filter out the illegal actions.
        debug_assert!(self.street <= RIVER + 1);
        if self.hand_over() {
            return smallvec![];
        }
        let pot = self.pot();
        let mut candidate_actions: SmallVec<[Action; NUM_ACTIONS]> = bet_abstraction[self.street]
            .iter()
            .map(|fraction| {
                let bet_size = if fraction == &ALL_IN {
                    self.stacks[self.player]
                } else {
                    (*fraction * (pot as f64)) as Amount
                };
                Action {
                    action: ActionType::Bet,
                    amount: bet_size,
                }
            })
            .collect();

        candidate_actions.push(Action {
            action: ActionType::Call,
            amount: self.to_call(),
        });
        candidate_actions.push(FOLD);
        candidate_actions.retain(|a| self.is_legal_next_action(a));
        candidate_actions
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    pub fn without_last_action(&self) -> ActionHistory {
        if self.is_empty() {
            panic!("Can't remove last action from empty history");
        }

        // There's a lot of complex rules, so just create a new history and add all the actions
        // in prev_history.
        let prev_history = &self.history[..self.history.len() - 1];
        let mut history = ActionHistory::new();
        for action in prev_history {
            history.add(&action);
        }

        // Check that we recover the original history when we add back the last action
        debug_assert!({
            let mut added = history.clone();
            added.add(&self.last_action().unwrap());
            added == self.clone()
        });

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
        debug_assert!(
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
        self.history.to_vec()
    }
}

impl fmt::Display for ActionHistory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::new();
        for action in &self.history {
            result.push_str(&action.to_string());
            result.push(',');
        }
        write!(f, "{result}")
    }
}

// Returns the element which is closest in log space to the input amount
fn find_closest_log(v: Vec<Amount>, n: Amount) -> Amount {
    debug_assert!(!v.is_empty());
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
        debug_assert!(!board.contains(&hole[0]) && !board.contains(&hole[1]));
        debug_assert!(board.len() == board_length(history.street));
        let board = &board[..board_length(history.street)];
        let hand = [hole, board].concat();
        InfoSet {
            history: history.clone(),
            card_bucket: ABSTRACTION.bin(&hand),
        }
    }

    pub fn next_actions(&self, bet_abstraction: &[Vec<f64>]) -> SmallVec<[Action; NUM_ACTIONS]> {
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

// Normalizes the values of a HashMap so that its elements sum to 1.
pub fn normalize<T: Eq + Hash + Clone>(map: &HashMap<T, f64>) -> HashMap<T, f64> {
    let keys: Vec<T> = map.keys().cloned().collect();
    let probs: Vec<f64> = keys.iter().map(|k| map.get(k).unwrap()).cloned().collect();
    let norm_probs = normalize_smallvec(&probs);
    let result: HashMap<T, f64> = keys
        .iter()
        .zip(norm_probs.iter())
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    result
}

pub fn normalize_smallvec(v: &[f64]) -> SmallVec<[f64; NUM_ACTIONS]> {
    for elem in v {
        debug_assert!(*elem >= 0.0);
    }
    let sum: f64 = v.iter().sum();
    let norm: SmallVec<[f64; NUM_ACTIONS]> = v
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
    debug_assert!(
        (norm_sum - 1.0).abs() < 1e-6,
        "Sum of normalized vector: {}. Input vector: {:?}",
        norm_sum,
        v
    );
    norm
}

pub fn sample_action_from_strategy(strategy: &Strategy) -> Action {
    let actions: Vec<Action> = strategy.keys().map(|a| a.clone()).collect();
    let action: Action = actions.choose_weighted(&mut thread_rng(), |a| strategy.get(a).unwrap()).unwrap().clone();
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
        let winnings: f64 = (CONFIG.stack_size - history.stack_sizes()[folder]) as f64;
        if winner == player {
            return winnings;
        } else {
            return -winnings;
        }
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
        return -(pot as f64) / 2.0;
    } else {
        // It's a tie: player_strength == opponent_strength
        return 0.0;
    }
}
