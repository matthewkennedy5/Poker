use crate::card_abstraction::Abstraction;
use crate::card_utils::*;
use crate::config::CONFIG;
use crate::nodes::*;
use once_cell::sync::Lazy;
use rand::{prelude::SliceRandom, thread_rng};
use smallvec::SmallVec;
use std::{
    cmp::Eq,
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
};

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
pub type SmallVecFloats = SmallVec<[f32; NUM_ACTIONS]>;

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
    pub current_street_length: u8,
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

        let last_street = match CONFIG.last_street.as_str() {
            "flop" => FLOP,
            "turn" => TURN,
            "river" => RIVER,
            _ => panic!(),
        };
        if self.street > last_street {
            // Change to FLOP for flop holdem, RIVER for texas holdem
            // Showdown
            return true;
        }
        false
    }

    // Add an new action to this history, and update the state
    pub fn add(&mut self, action: &Action) {
        debug_assert!(
            // this one is slow
            self.is_legal_next_action(action),
            "Action {:?} is illegal for history {:#?}",
            action,
            self
        );
        let action = action.clone();
        self.stacks[self.player] -= action.amount;
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

        let mut candidate_actions: SmallVec<[Action; NUM_ACTIONS]> =
            SmallVec::with_capacity(NUM_ACTIONS);
        for pot_fraction in bet_abstraction[self.street].iter() {
            let bet_size = if pot_fraction == &ALL_IN {
                self.stacks[self.player]
            } else {
                (pot_fraction * (pot as f64)) as Amount
            };
            let action = Action {
                action: ActionType::Bet,
                amount: bet_size,
            };
            if !candidate_actions.contains(&action) {
                candidate_actions.push(action);
            }
        }

        candidate_actions.push(Action {
            action: ActionType::Call,
            amount: self.to_call(),
        });
        candidate_actions.push(FOLD);
        candidate_actions.retain(|a| self.is_legal_next_action(a));
        debug_assert!(
            {
                // TODO: Might be better if these debug_asserts were standalone tests
                let action_set: HashSet<Action> = candidate_actions.iter().cloned().collect();
                action_set.len() == candidate_actions.len()
            },
            "Duplicate next actions: {:?}",
            candidate_actions
        );
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
            history.add(action);
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
        let board = &board[..board_length(history.street)];
        let hand: SmallVecHand = hole.iter().chain(board.iter()).cloned().collect();
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
    let iso = if street == FLOP {
        load_flop_isomorphic()
    } else if street == TURN {
        load_turn_isomorphic()
    } else if street == RIVER {
        load_river_isomorphic()
    } else if street == PREFLOP {
        isomorphic_preflop_hands()
            .iter()
            .map(|cards: &Vec<Card>| cards2hand(&cards))
            .collect()
    } else {
        panic!()
    };
    for hand in iso {
        if ABSTRACTION.bin(&hand2cards(hand)) == bucket {
            return hand2str(hand);
        }
    }
    panic!("No hands with the given bucket: {bucket} on street {street}");
}

// Normalizes the values of a HashMap so that its elements sum to 1.
pub fn normalize<T: Eq + Hash + Clone>(map: &HashMap<T, f64>) -> HashMap<T, f64> {
    let sum: f64 = map.values().sum();
    let result: HashMap<T, f64> = map
        .iter()
        .map(|(key, value)| (key.clone(), value / sum))
        .collect();
    result
}

pub fn normalize_smallvec(v: &[f32]) -> SmallVecFloats {
    for elem in v {
        debug_assert!(*elem >= 0.0);
    }
    let sum: f32 = v.iter().sum();
    debug_assert!(!v.is_empty());
    let norm: SmallVecFloats = v
        .iter()
        .map(|e| {
            if sum == 0.0 {
                // If all values are 0, then just return a uniform distribution
                1.0 / v.len() as f32
            } else {
                e / sum
            }
        })
        .collect();
    let norm_sum: f32 = norm.iter().sum();
    debug_assert!(
        (norm_sum - 1.0).abs() < 1e-6,
        "Sum of normalized vector: {}. Input vector: {:?}",
        norm_sum,
        v
    );
    norm
}

pub fn sample_action_from_strategy(strategy: &Strategy) -> Action {
    let actions: Vec<Action> = strategy.keys().cloned().collect();
    let action: Action = actions
        .choose_weighted(&mut thread_rng(), |a| strategy.get(a).unwrap())
        .unwrap()
        .clone();
    action
}

#[derive(Debug, Clone)]
struct HandData {
    hand: [Card; 2],
    strength: i32,
    prob: f64,
    index: usize,
}

#[inline]
fn card_index(card: &Card) -> usize {
    13 * card.suit as usize + (card.rank as usize - 2)
}

pub fn terminal_utility_vectorized_fast(
    preflop_hands: Vec<[Card; 2]>,
    opp_reach_probs: Vec<f64>,
    board: &[Card],
    history: &ActionHistory,
    player: usize,
) -> Vec<f64> {
    // START HERE: make sure the current code works (maybe fix the super slow load of IntMap read_serialized)
    // and then keep killing off all the HashMaps. no hash!
    // https://www.cs.cmu.edu/~waugh/publications/johanson11.pdf "Example 3" at end of page 4
    if history.last_action().unwrap().action == ActionType::Fold {
        // Someone folded -- assign the chips to the winner.
        let winner = history.player;
        let folder = 1 - winner;
        let mut winnings: f64 = (CONFIG.stack_size - history.stack_sizes()[folder]) as f64;
        if player == folder {
            winnings = -winnings;
        }
        let opp_prob_sum: f64 = opp_reach_probs.iter().sum();

        // Precompute blocking probs for each card
        let mut blocked_prob_sums: Vec<f64> = vec![0.0; 52];

        // Calculate the sum of probabilities for each card in preflop hands
        for (hand, &prob) in preflop_hands.iter().zip(opp_reach_probs.iter()) {
            blocked_prob_sums[card_index(&hand[0])] += prob;
            blocked_prob_sums[card_index(&hand[1])] += prob;
        }

        let utils: Vec<f64> = preflop_hands
            .iter()
            .enumerate()
            .map(|(i, hand)| {
                let total_prob = opp_prob_sum
                    - blocked_prob_sums[card_index(&hand[0])]
                    - blocked_prob_sums[card_index(&hand[1])]
                    + opp_reach_probs[i];
                total_prob * winnings / preflop_hands.len() as f64
            })
            .collect();
        return utils;
    }

    let mut hand_data: Vec<HandData> = (0..preflop_hands.len())
        .map(|i| {
            let h = preflop_hands[i];
            let river_hand = [h[0], h[1], board[0], board[1], board[2], board[3], board[4]];
            let strength = FAST_HAND_TABLE.hand_strength(&river_hand);
            HandData {
                hand: h,
                strength,
                prob: opp_reach_probs[i],
                index: i,
            }
        })
        .collect();

    hand_data.sort_unstable_by(|a, b| a.strength.cmp(&b.strength));

    let deck = deck();
    // blockers stores the Vec of hands that are blocked by each card. The hands are indexes in hand
    // data, which is already sorted by strength from low to high.
    let mut blockers: Vec<Vec<usize>> = vec![Vec::with_capacity(52); 52];
    for (i, hand_info) in hand_data.iter().enumerate() {
        for card in &hand_info.hand {
            let index = card_index(card);
            let blocking_indexes = blockers.get_mut(index).unwrap();
            blocking_indexes.push(i);
        }
    }

    // Ok so the basic idea here is:
    //  - sort the hands by strength
    //  - (complication) for each of 52 cards, keep track of the total probability it blocks
    //  - for each hand: keep track of total prob of hands better, worse, and equal.
    //  - then in the loop, add or subtract from those total probs.
    // Go from the worst hand to the best hand, changing the probs as you go.
    let total_prob: f64 = opp_reach_probs.iter().sum();
    let mut prob_equal = 0.0; // Total prob of opponent hands equal to current hand
    let mut prob_worse = 0.0; // Total prob of opponent hands worse than current hand
    let mut prob_better: f64 = opp_reach_probs.iter().sum();
    let mut idx_equal = 0;
    let mut idx_better = 0;
    let mut utils: Vec<f64> = vec![0.0; preflop_hands.len()];
    for d in &hand_data {
        // Just moved to a better player hand - need to move some opponent hands from "equal" to
        // "worse", and from "better" to "equal".

        // First, move the idx_equal up until its on a strength greater or equal to the current strength.
        // Add probs to prob_worse and subtract probs from prob_equal as you go. Because when you move
        // to a better hand, hands will move from being equal to being worse.

        let d_strength = d.strength;
        let d_prob = d.prob;
        let d_hand = d.hand;

        loop {
            if hand_data[idx_equal].strength >= d_strength {
                break;
            }
            prob_equal -= hand_data[idx_equal].prob;
            prob_worse += hand_data[idx_equal].prob;
            idx_equal += 1;
        }

        // Same thing but moving up idx_better, adding to prob_equal and subtracting from prob_better.
        loop {
            if idx_better >= hand_data.len() || hand_data[idx_better].strength > d_strength {
                break;
            }
            prob_better -= hand_data[idx_better].prob;
            prob_equal += hand_data[idx_better].prob;
            idx_better += 1;
        }

        // Adjust for blockers
        let mut prob_worse_adjusted = prob_worse;
        let mut prob_better_adjusted = prob_better;

        const MAX_SIZE: usize = 105;
        let mut all_blockers: [usize; MAX_SIZE] = [0; MAX_SIZE];

        // Retrieve the sorted lists from blockers
        let blockers1 = blockers.get(card_index(&d_hand[0])).unwrap();
        let blockers2 = blockers.get(card_index(&d_hand[1])).unwrap();

        // Perform a one-pass merge
        let mut i = 0;
        let mut j = 0;
        let mut k = 0;
        while i < blockers1.len() && j < blockers2.len() {
            if blockers1[i] <= blockers2[j] {
                all_blockers[k] = blockers1[i];
                i += 1;
            } else {
                all_blockers[k] = blockers2[j];
                j += 1;
            }
            k += 1;
        }
        // If there are remaining elements in blockers1
        while i < blockers1.len() {
            all_blockers[k] = blockers1[i];
            i += 1;
            k += 1;
        }

        // If there are remaining elements in blockers2
        while j < blockers2.len() {
            all_blockers[k] = blockers2[j];
            j += 1;
            k += 1;
        }

        // Now all_blockers contains the merged sorted elements
        // k is the actual number of elements in all_blockers

        // You can use &all_blockers[0..k] for further processing

        // Use standard binary search to find any matching index
        let mut equal_or_greater_idx = all_blockers[0..k]
            .binary_search_by(|&index| hand_data[index].strength.cmp(&d_strength))
            .unwrap_or_else(|x| x);

        // Move back to find the first occurrence
        while equal_or_greater_idx > 0
            && hand_data[all_blockers[equal_or_greater_idx - 1]].strength == d_strength
        {
            equal_or_greater_idx -= 1;
        }

        let mut greater_idx = all_blockers[0..k]
            .binary_search_by(|&index| hand_data[index].strength.cmp(&(d_strength + 1)))
            .unwrap_or_else(|x| x);

        // Similar adjustment for greater_idx, if needed
        while greater_idx > 0 && hand_data[all_blockers[greater_idx - 1]].strength == d_strength + 1
        {
            greater_idx -= 1;
        }

        // Summing up probabilities
        let prob_less: f64 = all_blockers[..equal_or_greater_idx]
            .iter()
            .map(|&index| hand_data[index].prob)
            .sum();
        let prob_greater: f64 = all_blockers[greater_idx..k]
            .iter()
            .map(|&index| hand_data[index].prob)
            .sum();

        // Adjust probabilities
        prob_worse_adjusted -= prob_less;
        prob_better_adjusted -= prob_greater;

        let util = history.pot() as f64 / 2.0 * (prob_worse_adjusted - prob_better_adjusted)
            / preflop_hands.len() as f64;

        utils[d.index] = util;
    }
    utils
}

fn terminal_utility_vectorized_slow(
    preflop_hands: Vec<[Card; 2]>,
    opp_reach_probs: Vec<f64>,
    board: &[Card],
    history: &ActionHistory,
    player: usize,
) -> Vec<f64> {
    let opponent = 1 - player;
    // If Showdown, do the full loop. This can be sped up later.
    let utils: Vec<f64> = preflop_hands
        .iter()
        .map(|h| {
            let mut total_util = 0.0;

            for i in 0..preflop_hands.len() {
                let opp_hand = preflop_hands[i];
                if h.contains(&opp_hand[0]) || h.contains(&opp_hand[1]) {
                    continue;
                }
                let opp_prob = opp_reach_probs[i];
                total_util += opp_prob * terminal_utility(h, &opp_hand, board, history, player);
            }
            total_util / preflop_hands.len() as f64
        })
        .collect();
    utils
}

// TODO REFACTOR: make this just terminal_utility_vectorized_fast
pub fn terminal_utility_vectorized(
    preflop_hands: Vec<[Card; 2]>,
    opp_reach_probs: Vec<f64>,
    board: &[Card],
    history: &ActionHistory,
    player: usize,
) -> Vec<f64> {
    let fast = terminal_utility_vectorized_fast(
        preflop_hands.clone(),
        opp_reach_probs.clone(),
        board,
        history,
        player,
    );
    // TODO Refactor: make this an automated test instead of assert
    assert!({
        let slow = terminal_utility_vectorized_slow(
            preflop_hands.clone(),
            opp_reach_probs.clone(),
            board,
            history,
            player,
        );
        assert!(
            fast.iter()
                .zip(slow.iter())
                .all(|(&a, &b)| (a - b).abs() < 1e-6),
            "{} != {}",
            fast[0],
            slow[0]
        );
        true
    });
    fast
}

// Assuming history represents a terminal state (someone folded, or it's a showdown),
// return the utility, in chips, that the given player gets.
pub fn terminal_utility(
    player_preflop_hand: &[Card],
    opp_preflop_hand: &[Card],
    board: &[Card],
    history: &ActionHistory,
    player: usize,
) -> f64 {
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
    let player_hand = [player_preflop_hand, board].concat();
    let opponent_hand = [opp_preflop_hand, board].concat();
    let player_strength = FAST_HAND_TABLE.hand_strength(&player_hand);
    let opponent_strength = FAST_HAND_TABLE.hand_strength(&opponent_hand);

    if player_strength > opponent_strength {
        pot as f64 / 2.0
    } else if player_strength < opponent_strength {
        return -(pot as f64) / 2.0;
    } else {
        // It's a tie: player_strength == opponent_strength
        return 0.0;
    }
}
