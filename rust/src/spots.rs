//! Training spots for the value networks: a public state at the start of a street
//! (board, pot, remaining effective stack) plus both players' ranges.
//!
//! Two sources:
//! - random: strength-correlated ranges from DeepStack's recursive range generator,
//!   mixed with unstructured random ranges, over a wide spread of pot and stack
//!   sizes (stack depth is a net input, which is what gives variable stacks);
//! - book: realistic ranges at the end of the flop (start of the turn) from the
//!   flop book's solved strategies, for every line that ends the flop without a
//!   fold or an all-in.
//!
//! Weights are per hand in `Hands::new(board)` order, scaled so the max is 1.
use crate::card_utils::*;
use crate::flopbook::*;
use crate::search_utils::*;
use crate::street_solver::{strength_keys, Hands};
use postflop_solver::Action as SolverAction;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Spot {
    /// Solver card ids (4 * rank + suit), 3..=5 cards.
    pub board: Vec<u8>,
    pub pot: i32,
    /// Remaining effective stack.
    pub stack: i32,
    /// [OOP, IP] weights in `Hands::new(board)` order.
    pub weights: [Vec<f32>; 2],
    /// 0 = random, 1 = book.
    pub source: u8,
}

/// Flop-end ranges from the book: becomes a turn spot once a turn card is added.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FlopEnd {
    pub flop: [u8; 3],
    pub pot: i32,
    pub stack: i32,
    pub hands: [Vec<(u8, u8)>; 2],
    pub weights: [Vec<f32>; 2],
    pub line: String,
}

fn normalize_max(w: &mut [f32]) -> bool {
    let max = w.iter().cloned().fold(0.0f32, f32::max);
    if !(max > 0.0) {
        return false;
    }
    w.iter_mut().for_each(|x| *x /= max);
    true
}

/// Hands ordered from weakest to strongest by average showdown strength over the
/// remaining board cards (sampled for flops).
fn strength_order<R: Rng>(board: &[u8], hands: &Hands, rng: &mut R) -> Vec<usize> {
    let rest: Vec<u8> = (0..52u8).filter(|c| !board.contains(c)).collect();
    let mut score = vec![0.0f64; hands.len()];
    let mut count = vec![0u32; hands.len()];
    let samples = match board.len() {
        5 => 1,
        4 => rest.len(),
        _ => 60,
    };
    for k in 0..samples {
        let mut full = board.to_vec();
        match board.len() {
            5 => {}
            4 => full.push(rest[k]),
            _ => {
                let pick: Vec<u8> = rest.choose_multiple(rng, 5 - board.len()).cloned().collect();
                full.extend(pick);
            }
        }
        let keys = strength_keys(&full, hands);
        let max = keys.iter().flatten().cloned().max().unwrap_or(1).max(1) as f64;
        for (i, k) in keys.iter().enumerate() {
            if let Some(k) = k {
                score[i] += *k as f64 / max;
                count[i] += 1;
            }
        }
    }
    let mut order: Vec<usize> = (0..hands.len()).collect();
    let avg = |i: usize| if count[i] > 0 { score[i] / count[i] as f64 } else { 0.0 };
    order.sort_by(|&a, &b| avg(a).partial_cmp(&avg(b)).unwrap());
    order
}

/// DeepStack's recursive range generator: split the strength-sorted hands in half
/// and divide the probability mass between the halves uniformly at random.
fn recursive_range<R: Rng>(order: &[usize], mass: f64, w: &mut [f32], rng: &mut R) {
    if order.len() == 1 {
        w[order[0]] = mass as f32;
        return;
    }
    let mid = order.len() / 2;
    let p: f64 = rng.gen();
    recursive_range(&order[..mid], mass * p, w, rng);
    recursive_range(&order[mid..], mass * (1.0 - p), w, rng);
}

fn random_weights<R: Rng>(order: &[usize], n: usize, rng: &mut R) -> Vec<f32> {
    let mut w = vec![0.0f32; n];
    match rng.gen_range(0..10) {
        // strength-correlated
        0..=5 => recursive_range(order, 1.0, &mut w, rng),
        // unstructured: squared uniform weights with a random share of absent hands
        6..=7 => {
            let absent: f32 = rng.gen_range(0.0..0.8);
            for x in w.iter_mut() {
                *x = if rng.gen::<f32>() < absent { 0.0 } else { rng.gen::<f32>().powi(2) };
            }
        }
        // a mix of both
        _ => {
            recursive_range(order, 1.0, &mut w, rng);
            normalize_max(&mut w);
            let mix: f32 = rng.gen_range(0.05..0.5);
            for x in w.iter_mut() {
                *x = (1.0 - mix) * *x + mix * rng.gen::<f32>().powi(2);
            }
        }
    }
    normalize_max(&mut w);
    w
}

fn log_uniform<R: Rng>(lo: f64, hi: f64, rng: &mut R) -> f64 {
    (lo.ln() + rng.gen::<f64>() * (hi.ln() - lo.ln())).exp()
}

/// A random spot at the start of a street with `board_len` board cards: pot
/// log-uniform in 2-200 big blinds (100-chip blinds) and stack-to-pot ratio
/// log-uniform in 0.05-60, limited to starting stacks of at most 300 big blinds.
pub fn random_spot<R: Rng>(board_len: usize, rng: &mut R) -> Spot {
    let mut deck: Vec<u8> = (0..52).collect();
    deck.shuffle(rng);
    let board: Vec<u8> = deck[..board_len].to_vec();
    let hands = Hands::new(&board);
    let order = strength_order(&board, &hands, rng);
    let weights = [random_weights(&order, hands.len(), rng), random_weights(&order, hands.len(), rng)];
    loop {
        let pot = log_uniform(200.0, 20000.0, rng).round() as i32;
        let stack = (log_uniform(0.05, 60.0, rng) * pot as f64).round() as i32;
        if stack >= 50 && pot / 2 + stack <= 30000 {
            return Spot { board, pot, stack, weights, source: 0 };
        }
    }
}

/// Street-ending lines of a book flop solution (no fold, no all-in) with the
/// ranges at the end of each, in solver card ids of the canonical flop.
pub fn harvest_flop_ends(sol: &FlopSolution, line: &str) -> Vec<FlopEnd> {
    let flop_cards = str2cards(&sol.flop);
    let flop = [
        to_solver_card(&flop_cards[0]),
        to_solver_card(&flop_cards[1]),
        to_solver_card(&flop_cards[2]),
    ];
    let mut out = Vec::new();
    for node in &sol.nodes {
        let path: Vec<SolverAction> = node.path.iter().map(|&a| dec(a)).collect();
        for &a in &node.actions {
            let a = dec(a);
            if a == SolverAction::Fold {
                continue;
            }
            let mut child = path.clone();
            child.push(a);
            if sol.node(&child).is_some() {
                continue; // the street goes on
            }
            let mut bet = 0;
            for x in &child {
                if let SolverAction::Bet(v) | SolverAction::Raise(v) | SolverAction::AllIn(v) = x {
                    bet = *v;
                }
            }
            let stack = sol.eff_stack - bet;
            if stack <= 0 {
                continue;
            }
            let Some(mut weights) = sol.ranges_after(&child) else { continue };
            if !normalize_max(&mut weights[0]) || !normalize_max(&mut weights[1]) {
                continue;
            }
            out.push(FlopEnd {
                flop,
                pot: sol.pot + 2 * bet,
                stack,
                hands: sol.hands.clone(),
                weights,
                line: format!("{} | {:?}", line, child),
            });
        }
    }
    out
}

/// The turn spot after `turn` is dealt on a flop end. None if a range empties.
pub fn turn_spot(fe: &FlopEnd, turn: u8) -> Option<Spot> {
    if fe.flop.contains(&turn) {
        return None;
    }
    let board = vec![fe.flop[0], fe.flop[1], fe.flop[2], turn];
    let hands = Hands::new(&board);
    let mut weights = [vec![0.0f32; hands.len()], vec![0.0f32; hands.len()]];
    for p in 0..2 {
        for (k, &(a, b)) in fe.hands[p].iter().enumerate() {
            if let Some(i) = hands.index(a, b) {
                weights[p][i] = fe.weights[p][k];
            }
        }
        if !normalize_max(&mut weights[p]) {
            return None;
        }
    }
    Some(Spot { board, pot: fe.pot, stack: fe.stack, weights, source: 1 })
}
