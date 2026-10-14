//! Conversions between this project's game representation (`Card`, `Action`,
//! `ActionHistory`) and the representation used by the external
//! `postflop_solver` crate, which drives real-time search on the flop, turn
//! and river.
//!
//! Conventions (verified against both codebases):
//!
//! * Project `Card { rank: 2..=14, suit: c=0,d=1,h=2,s=3 }`.
//!   Solver card = `(rank - 2) * 4 + suit` (same suit order), so `2c` = 0 and `As` = 51.
//! * Project `OPPONENT` (big blind) acts first postflop, i.e. is the solver's
//!   out-of-position player 0. Project `DEALER` is the solver's in-position player 1.
//! * Project `Bet.amount` = chips added by that action; `Call.amount` = amount to call.
//!   Solver `Bet(x)` / `Raise(x)` / `AllIn(x)` carry the acting player's *total*
//!   contribution on the current street.

use crate::card_utils::Card;
use crate::config::CONFIG;
use crate::trainer_utils::*;
use postflop_solver::Action as SolverAction;

pub fn to_solver_card(c: &Card) -> u8 {
    (c.rank - 2) * 4 + c.suit
}

pub fn from_solver_card(id: u8) -> Card {
    Card {
        rank: id / 4 + 2,
        suit: id % 4,
    }
}

/// Hole cards as a `(low, high)` solver pair, matching the ordering used by
/// `PostFlopGame::private_cards`.
pub fn solver_hole(hole: &[Card]) -> (u8, u8) {
    let a = to_solver_card(&hole[0]);
    let b = to_solver_card(&hole[1]);
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Project player index -> solver player index (0 = OOP, 1 = IP).
pub fn solver_player(project_player: usize) -> usize {
    1 - project_player
}

/// One street's worth of real actions plus the state at the start of that street.
#[derive(Debug, Clone, PartialEq)]
pub struct StreetSlice {
    pub street: usize,
    pub actions: Vec<Action>,
    pub first_player: usize,
    pub pot_start: Amount,
    pub stacks_start: [Amount; 2],
}

/// Splits a history into per-street slices. A slice is present for every street
/// from preflop up to and including `history.street` (as long as the hand is still
/// being played on a real street), even if no action has been taken on it yet.
pub fn split_by_street(history: &ActionHistory) -> Vec<StreetSlice> {
    let mut replay = ActionHistory::new_with_stacks(history.starting_stacks());
    let mut slices = vec![StreetSlice {
        street: PREFLOP,
        actions: Vec::new(),
        first_player: replay.player,
        pot_start: replay.pot(),
        stacks_start: replay.stack_sizes(),
    }];
    for action in history.get_actions() {
        let street_before = replay.street;
        slices.last_mut().unwrap().actions.push(action.clone());
        replay.add(&action);
        if replay.street != street_before && replay.street <= RIVER && !replay.hand_over() {
            slices.push(StreetSlice {
                street: replay.street,
                actions: Vec::new(),
                first_player: replay.player,
                pot_start: replay.pot(),
                stacks_start: replay.stack_sizes(),
            });
        }
    }
    slices
}

/// The solver-side view of one street's real action line.
#[derive(Debug, Clone, PartialEq)]
pub struct SolverLine {
    pub actions: Vec<SolverAction>,
    /// Total chips each *project* player has put in on this street.
    pub totals: [i32; 2],
    /// Size of the last raise increment on this street (0 if no bet yet).
    pub last_increment: i32,
    /// Project player to act next.
    pub next_player: usize,
}

/// Converts a street's project actions into solver actions. `eff_stack` is the
/// effective stack at the start of the street (chips behind for the shorter stack).
pub fn to_solver_line(slice: &StreetSlice, eff_stack: i32) -> Result<SolverLine, String> {
    let mut totals = [0i32; 2];
    let mut player = slice.first_player;
    let mut any_bet = false;
    let mut last_increment = 0i32;
    let mut actions = Vec::with_capacity(slice.actions.len());
    for a in &slice.actions {
        match a.action {
            ActionType::Fold => actions.push(SolverAction::Fold),
            ActionType::Call => {
                if a.amount == 0 {
                    actions.push(SolverAction::Check);
                } else {
                    actions.push(SolverAction::Call);
                    totals[player] = totals[1 - player];
                }
            }
            ActionType::Bet => {
                let new_total = totals[player] + a.amount as i32;
                if new_total >= eff_stack {
                    actions.push(SolverAction::AllIn(eff_stack));
                    last_increment = eff_stack - totals[1 - player];
                    totals[player] = eff_stack;
                } else {
                    if any_bet {
                        actions.push(SolverAction::Raise(new_total));
                    } else {
                        actions.push(SolverAction::Bet(new_total));
                    }
                    last_increment = new_total - totals[1 - player];
                    totals[player] = new_total;
                }
                any_bet = true;
            }
        }
        player = 1 - player;
    }
    Ok(SolverLine {
        actions,
        totals,
        last_increment,
        next_player: player,
    })
}

/// Converts the solver's chosen action into a legal project action for the real
/// game state. `line` is the current street's line so far (used for min-raise
/// rules and our total contribution on the street).
pub fn to_project_action(
    solver_action: &SolverAction,
    history: &ActionHistory,
    line: &SolverLine,
) -> Action {
    let me = history.player;
    let my_total = line.totals[me];
    let opp_total = line.totals[1 - me];
    let to_call = history.to_call() as i32;
    let max_bet = history.max_bet() as i32;
    let big_blind = CONFIG.big_blind as i32;

    let bet_added = |total: i32| -> Action {
        // Chips to add for the desired street total, then legalize.
        let mut added = total - my_total;
        if added >= max_bet {
            return Action {
                action: ActionType::Bet,
                amount: max_bet as Amount,
            };
        }
        // Minimum legal raise: match the previous raise increment (at least 1 BB).
        let min_total = opp_total + line.last_increment.max(big_blind);
        let min_added = min_total - my_total;
        if added < min_added {
            added = min_added;
        }
        if added >= max_bet {
            added = max_bet;
        }
        if added <= to_call {
            // Should be unreachable, but never emit a "bet" that is actually a call.
            added = (to_call + big_blind).min(max_bet);
        }
        Action {
            action: ActionType::Bet,
            amount: added as Amount,
        }
    };

    match solver_action {
        SolverAction::Fold => FOLD,
        SolverAction::Check => Action {
            action: ActionType::Call,
            amount: 0,
        },
        SolverAction::Call => Action {
            action: ActionType::Call,
            amount: to_call as Amount,
        },
        SolverAction::Bet(t) | SolverAction::Raise(t) => bet_added(*t),
        SolverAction::AllIn(_) => Action {
            action: ActionType::Bet,
            amount: max_bet as Amount,
        },
        SolverAction::Chance(_) | SolverAction::None => {
            panic!("Solver returned a non-player action: {:?}", solver_action)
        }
    }
}

/// Number of board cards for a street.
pub fn board_len_for_street(street: usize) -> usize {
    match street {
        PREFLOP => 0,
        FLOP => 3,
        TURN => 4,
        RIVER => 5,
        _ => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_utils::str2cards;

    #[test]
    fn card_conversion() {
        assert_eq!(to_solver_card(&Card::new("2c")), 0);
        assert_eq!(to_solver_card(&Card::new("3d")), 5);
        assert_eq!(to_solver_card(&Card::new("4h")), 10);
        assert_eq!(to_solver_card(&Card::new("As")), 51);
        for id in 0..52u8 {
            assert_eq!(to_solver_card(&from_solver_card(id)), id);
        }
        assert_eq!(solver_hole(&str2cards("AsKd")), (45, 51));
    }

    #[test]
    fn player_mapping() {
        assert_eq!(solver_player(OPPONENT), 0);
        assert_eq!(solver_player(DEALER), 1);
    }

    #[test]
    fn split_and_convert_streets() {
        // SB raises to 250, BB calls. Flop: BB checks, SB bets 300, BB raises to 900, SB calls.
        let history = ActionHistory::from_strings(vec![
            "Bet 250", "Call 250", "Call 0", "Bet 300", "Bet 900", "Call 600",
        ]);
        let slices = split_by_street(&history);
        assert_eq!(slices.len(), 3); // preflop, flop, turn (empty)
        let flop = &slices[1];
        assert_eq!(flop.street, FLOP);
        assert_eq!(flop.first_player, OPPONENT);
        assert_eq!(flop.pot_start, 500);
        assert_eq!(flop.stacks_start, [19750, 19750]);
        let line = to_solver_line(flop, 19750).unwrap();
        assert_eq!(
            line.actions,
            vec![
                SolverAction::Check,
                SolverAction::Bet(300),
                SolverAction::Raise(900),
                SolverAction::Call
            ]
        );
        assert_eq!(line.totals, [900, 900]);
        assert_eq!(line.last_increment, 600);
        let turn = &slices[2];
        assert_eq!(turn.street, TURN);
        assert!(turn.actions.is_empty());
        assert_eq!(turn.pot_start, 2300);
        assert_eq!(turn.stacks_start, [18850, 18850]);
    }

    #[test]
    fn all_in_maps_to_allin() {
        let history = ActionHistory::from_strings(vec!["Bet 250", "Call 250", "Bet 19750"]);
        let slices = split_by_street(&history);
        let line = to_solver_line(&slices[1], 19750).unwrap();
        assert_eq!(line.actions, vec![SolverAction::AllIn(19750)]);
        assert_eq!(line.next_player, DEALER);
    }

    #[test]
    fn project_action_legalization() {
        // Flop after SB 250 / BB call: BB checks, SB bets 300. BB to act.
        let history =
            ActionHistory::from_strings(vec!["Bet 250", "Call 250", "Call 0", "Bet 300"]);
        let slices = split_by_street(&history);
        let line = to_solver_line(&slices[1], 19750).unwrap();
        assert_eq!(history.player, OPPONENT);
        assert_eq!(
            to_project_action(&SolverAction::Raise(900), &history, &line),
            Action { action: ActionType::Bet, amount: 900 }
        );
        assert_eq!(
            to_project_action(&SolverAction::Call, &history, &line),
            Action { action: ActionType::Call, amount: 300 }
        );
        assert_eq!(to_project_action(&SolverAction::Fold, &history, &line), FOLD);
        // Below the min-raise (300 + 300 = 600 total) gets bumped up.
        assert_eq!(
            to_project_action(&SolverAction::Raise(500), &history, &line),
            Action { action: ActionType::Bet, amount: 600 }
        );
        // All-in caps at the stack.
        assert_eq!(
            to_project_action(&SolverAction::Raise(50000), &history, &line),
            Action { action: ActionType::Bet, amount: 19750 }
        );
        assert_eq!(
            to_project_action(&SolverAction::AllIn(19750), &history, &line),
            Action { action: ActionType::Bet, amount: 19750 }
        );

        // Opening bet in a limped pot (pot 200): a 66-chip bet is bumped to 1 BB.
        let history = ActionHistory::from_strings(vec!["Call 100", "Call 100"]);
        let slices = split_by_street(&history);
        let line = to_solver_line(&slices[1], 19900).unwrap();
        assert_eq!(history.player, OPPONENT);
        assert_eq!(
            to_project_action(&SolverAction::Bet(66), &history, &line),
            Action { action: ActionType::Bet, amount: 100 }
        );
        assert_eq!(
            to_project_action(&SolverAction::Check, &history, &line),
            Action { action: ActionType::Call, amount: 0 }
        );
    }
}
