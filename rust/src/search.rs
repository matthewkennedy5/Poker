//! Real-time search for the flop, turn and river using the external
//! `postflop_solver` crate ("nested unsafe re-solving with exact hands").
//!
//! Flow for a decision on a postflop street:
//! 1. Both players' ranges at the start of the street. For the flop they come from
//!    replaying the (translated) preflop line against the blueprint. For the turn and
//!    river they are the reach-weighted ranges at the end of the previous street's
//!    solved game.
//! 2. A game tree for the street is built from the real pot and effective stack, with
//!    a small set of bet sizes, and the real action line so far is added to the tree,
//!    so no action translation is ever needed postflop.
//! 3. The tree is solved under a wall-clock budget with DCFR.
//! 4. The solved game is navigated along the real line and we sample from the
//!    strategy for our exact hole cards.
//!
//! Solves are cached per hand so later decisions on the same street reuse them. Any
//! failure falls back to the blueprint bot.

use crate::card_utils::*;
use crate::config::{SearchConfig, CONFIG};
use crate::flopbook::*;
use crate::readonly::{ReadOnlyBot, ReadOnlyNodes};
use crate::search_utils::*;
use crate::trainer_utils::*;
use postflop_solver::{
    compute_exploitability, finalize, solve_step, Action as SolverAction, ActionTree,
    BetSizeOptions, BoardState, CardConfig, PostFlopGame, Range as SolverRange, TreeConfig,
    NOT_DEALT,
};
use std::collections::{HashMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct SolveStats {
    pub iters: u32,
    pub exploitability: f32,
    pub elapsed: Duration,
    pub memory_bytes: u64,
}

/// A solved street, kept in the per-hand cache.
pub struct StreetSolve {
    pub street: usize,
    pub board: Vec<u8>,
    pub pot_start: i32,
    pub eff_stack: i32,
    /// [OOP, IP] ranges at the start of the street.
    pub entry_ranges: [SolverRange; 2],
    /// The real line that was guaranteed to be in the tree when solving.
    pub solved_line: Vec<SolverAction>,
    /// Finalized game. Invariant: left at the root between uses.
    pub game: PostFlopGame,
    pub stats: SolveStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HandKey {
    hole: (u8, u8),
    preflop: Vec<Action>,
    starting_stacks: [Amount; 2],
}

/// Cached solves for one hand. The key is immutable; the solves are locked per
/// hand so different hands can be searched concurrently.
struct HandEntry {
    key: HandKey,
    streets: Mutex<[Option<StreetSolve>; 3]>,
}

/// A decision with the distribution it was actually sampled from and, when the
/// solver provides them, per-action values for our exact hand (used by the
/// evaluation's AIVAT action terms).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Decision {
    /// Action actually played (after legalization).
    pub action: Action,
    /// Action drawn from `probs` (before legalization).
    pub sampled: Action,
    /// "book", "search" or "blueprint"
    pub source: &'static str,
    /// Sampling distribution after purification: (action, probability).
    pub probs: Vec<(Action, f64)>,
    /// Per-action values (chips, common offset per decision), same order as probs.
    pub values: Option<Vec<f64>>,
    /// True if legalization changed the sampled action.
    pub legalized: bool,
}

pub type ActionValues = HashMap<Action, f64>;

pub struct SearchBot {
    blueprint: Arc<ReadOnlyBot>,
    cfg: SearchConfig,
    /// Most recently used first. Only held briefly, to find or insert an entry.
    cache: Mutex<VecDeque<Arc<HandEntry>>>,
    book: Option<FlopBook>,
}

impl SearchBot {
    pub fn new(blueprint: Arc<ReadOnlyBot>, cfg: SearchConfig) -> Self {
        let book = if cfg.enabled && !cfg.flop_book_dir.is_empty() {
            let b = FlopBook::load(&cfg.flop_book_dir);
            if b.is_empty() {
                println!("[BOOK] no flop book found in {}", cfg.flop_book_dir);
                None
            } else {
                Some(b)
            }
        } else {
            None
        };
        SearchBot {
            blueprint,
            cfg,
            cache: Mutex::new(VecDeque::new()),
            book,
        }
    }

    pub fn has_book(&self) -> bool {
        self.book.is_some()
    }

    /// Flop decision from the precomputed book, if this spot is covered.
    pub fn book_flop_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Option<(Strategy, Option<ActionValues>)> {
        let book = self.book.as_ref()?;
        if history.street != FLOP || board.len() < 3 {
            return None;
        }
        let line_key = abstract_preflop_line(history);
        let miss = |why: &str| {
            if self.cfg.log {
                println!("[BOOK] miss on {} line '{}': {}", cards2str(&board[..3]), line_key, why);
            }
        };
        let Some((sol, perm)) = book.lookup(&line_key, &board[..3]) else {
            miss("line or flop not in book");
            return None;
        };
        let canon_hole = solver_hole(&[perm.apply(&hole[0]), perm.apply(&hole[1])]);
        let me = solver_player(history.player);
        let Some(idx) = sol.hand_index(me, canon_hole) else {
            miss("our hand is not in the book range");
            return None;
        };
        let slices = split_by_street(history);
        let slice = slices.get(FLOP)?;
        let eff = slice.stacks_start[0].min(slice.stacks_start[1]) as i32;
        let real_pot = slice.pot_start as i32;
        let line = to_solver_line(slice, eff).ok()?;
        let scale = sol.pot as f64 / real_pot.max(1) as f64;
        let scaled: Vec<SolverAction> = line.actions.iter().map(|a| scale_action(*a, scale)).collect();
        let Some(path) = sol.map_line(&scaled) else {
            miss(&format!("line {:?} has no book node", scaled));
            return None;
        };
        let node = sol.node(&path)?;
        if node.player as usize != me {
            miss("book node belongs to the other player");
            return None;
        }
        let n = sol.hands[me].len();
        let node_values = node.action_values(idx);
        let mut strategy = Strategy::new();
        let mut acc: HashMap<Action, (f64, f64, f64, usize)> = HashMap::new(); // (p, p*v, v, count)
        for (ai, sa) in node.actions().iter().enumerate() {
            let p = node.strategy[ai * n + idx] as f64;
            let real_action = scale_action(*sa, 1.0 / scale);
            let pa = to_project_action(&real_action, history, &line);
            *strategy.entry(pa.clone()).or_insert(0.0) += p;
            if let Some(vals) = &node_values {
                // Book values are in the book spot's chip scale; rescale to the real pot.
                let v = vals[ai] / scale;
                let e = acc.entry(pa).or_insert((0.0, 0.0, 0.0, 0));
                e.0 += p;
                e.1 += p * v;
                e.2 += v;
                e.3 += 1;
            }
        }
        let values = node_values.map(|_| merge_values(&acc));
        if self.cfg.log {
            let mut items: Vec<(String, f64)> = strategy.iter().map(|(a, p)| (format!("{}", a), *p)).collect();
            items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            println!(
                "[BOOK] {} on {} line '{}' path {:?}: {:?}",
                cards2str(hole), cards2str(&board[..3]), line_key, path, items
            );
        }
        Some((strategy, values))
    }

    /// Turn entry ranges [OOP, IP] implied by the book's flop strategies, if covered.
    fn book_turn_entry_ranges(
        &self,
        board: &[Card],
        slices: &[StreetSlice],
        history: &ActionHistory,
        our_hole: (u8, u8),
        floor: f32,
    ) -> Option<[SolverRange; 2]> {
        let book = self.book.as_ref()?;
        let line_key = abstract_preflop_line(history);
        let (sol, perm) = book.lookup(&line_key, &board[..3])?;
        let slice = slices.get(FLOP)?;
        let eff = slice.stacks_start[0].min(slice.stacks_start[1]) as i32;
        let real_pot = slice.pot_start as i32;
        let line = to_solver_line(slice, eff).ok()?;
        let scale = sol.pot as f64 / real_pot.max(1) as f64;
        let scaled: Vec<SolverAction> = line.actions.iter().map(|a| scale_action(*a, scale)).collect();
        let path = sol.map_line(&scaled)?;
        let weights = sol.ranges_after(&path)?;
        let inv = perm.inverse();
        let me = solver_player(history.player);
        let mut out: Vec<SolverRange> = Vec::with_capacity(2);
        for p in 0..2 {
            let mut pairs: Vec<(u8, u8)> = sol.hands[p]
                .iter()
                .map(|&(a, b)| {
                    let ca = inv.apply(&from_solver_card(a));
                    let cb = inv.apply(&from_solver_card(b));
                    solver_hole(&[ca, cb])
                })
                .collect();
            let mut w: Vec<f64> = weights[p].iter().map(|x| *x as f64).collect();
            if p == me && !pairs.contains(&our_hole) {
                // Our hand was pruned from the stored range: add it at the floor
                // weight (relative to the max) so the solve still covers it.
                let max = w.iter().cloned().fold(0.0f64, f64::max);
                pairs.push(our_hole);
                w.push(max * floor as f64);
            }
            let r = make_range(&pairs, &w, if p == me { Some(our_hole) } else { None }, floor, 0.0).ok()?;
            out.push(r);
        }
        let ip = out.pop().unwrap();
        let oop = out.pop().unwrap();
        if self.cfg.log {
            println!("[BOOK] turn ranges from book for line '{}' path {:?}", line_key, path);
        }
        Some([oop, ip])
    }

    pub fn blueprint(&self) -> &ReadOnlyBot {
        &self.blueprint
    }

    /// First street handled by search for the given project seat (clamped to
    /// flop..=river).
    pub fn first_street(&self, seat: usize) -> usize {
        let base = self.cfg.first_search_street.clamp(FLOP, RIVER);
        let flop_on = match self.cfg.flop_search_seats.as_str() {
            "both" => true,
            "bb" => seat == OPPONENT,
            "sb" => seat == DEALER,
            _ => false,
        };
        if flop_on {
            FLOP
        } else {
            base
        }
    }

    pub fn config(&self) -> &SearchConfig {
        &self.cfg
    }

    /// Action to take, with blueprint fallback on any failure. Never panics and
    /// never returns an action that is illegal in the real game.
    pub fn get_action(&self, hole: &[Card], board: &[Card], history: &ActionHistory) -> Action {
        self.decide(hole, board, history).action
    }

    /// Chooses an action and reports the distribution it was sampled from.
    pub fn decide(&self, hole: &[Card], board: &[Card], history: &ActionHistory) -> Decision {
        self.decide_with(hole, board, history, true)
    }

    /// Like `decide`, but with `allow_search = false` only the book and the blueprint
    /// are used (cheap; for when the server is overloaded).
    pub fn decide_with(&self, hole: &[Card], board: &[Card], history: &ActionHistory, allow_search: bool) -> Decision {
        let mut d = self.decide_inner(hole, board, history, allow_search);
        let legal = legalize(d.action.clone(), history);
        if legal != d.action {
            d.legalized = true;
            d.action = legal;
        }
        d
    }

    fn from_strategy(
        &self,
        mut strategy: Strategy,
        values: Option<ActionValues>,
        source: &'static str,
    ) -> Option<Decision> {
        purify(&mut strategy, self.cfg.purify_threshold);
        if strategy.is_empty() {
            return None;
        }
        let action = sample_action_from_strategy(&strategy);
        let mut probs: Vec<(Action, f64)> = strategy.into_iter().collect();
        probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let values = values.and_then(|v| {
            probs.iter().map(|(a, _)| v.get(a).copied()).collect::<Option<Vec<f64>>>()
        });
        Some(Decision { sampled: action.clone(), action, source, probs, values, legalized: false })
    }

    fn blueprint_decision(&self, hole: &[Card], board: &[Card], history: &ActionHistory) -> Decision {
        let action = self.safe_blueprint_action(hole, board, history);
        Decision {
            sampled: action.clone(),
            action,
            source: "blueprint",
            probs: Vec::new(),
            values: None,
            legalized: false,
        }
    }

    fn decide_inner(&self, hole: &[Card], board: &[Card], history: &ActionHistory, allow_search: bool) -> Decision {
        if self.cfg.enabled && history.street == FLOP && self.book.is_some() && !history.hand_over() {
            let r = catch_unwind(AssertUnwindSafe(|| self.book_flop_strategy(hole, board, history)));
            if let Ok(Some((strategy, values))) = r {
                if let Some(d) = self.from_strategy(strategy, values, "book") {
                    return d;
                }
            }
        }
        if !self.cfg.enabled
            || !allow_search
            || history.street < self.first_street(history.player)
            || history.street > RIVER
            || history.hand_over()
        {
            return self.blueprint_decision(hole, board, history);
        }
        let result = catch_unwind(AssertUnwindSafe(|| self.search_strategy(hole, board, history)));
        match result {
            Ok(Ok((strategy, values))) => match self.from_strategy(strategy, Some(values), "search") {
                Some(d) => d,
                None => {
                    println!("[SEARCH] fallback to blueprint: empty strategy after purification");
                    self.blueprint_decision(hole, board, history)
                }
            },
            Ok(Err(msg)) => {
                println!("[SEARCH] fallback to blueprint: {}", msg);
                self.blueprint_decision(hole, board, history)
            }
            Err(_) => {
                println!("[SEARCH] fallback to blueprint: panic during search");
                self.blueprint_decision(hole, board, history)
            }
        }
    }

    /// Blueprint action that can never panic: if the blueprint itself fails, check
    /// when possible, otherwise call.
    pub fn safe_blueprint_action(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Action {
        match catch_unwind(AssertUnwindSafe(|| self.blueprint.get_action(hole, board, history))) {
            Ok(action) => action,
            Err(_) => {
                println!("[SEARCH] blueprint panicked too; checking/calling");
                Action {
                    action: ActionType::Call,
                    amount: history.to_call(),
                }
            }
        }
    }

    /// Strategy (probabilities over project actions) from search, or an error.
    pub fn search_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Result<(Strategy, ActionValues), String> {
        let start = Instant::now();
        let street = history.street;
        let first_street = self.first_street(history.player);
        if !(first_street..=RIVER).contains(&street) {
            return Err(format!("street {} is not searched", street));
        }
        if hole.len() != 2 {
            return Err("need exactly two hole cards".to_string());
        }
        let n_board = board_len_for_street(street);
        if board.len() < n_board {
            return Err(format!("board has {} cards, need {}", board.len(), n_board));
        }
        let board_ids: Vec<u8> = board[..n_board].iter().map(to_solver_card).collect();
        let our_hole = solver_hole(hole);
        let slices = split_by_street(history);
        if slices.len() <= street {
            return Err("history has no slice for the current street".to_string());
        }
        let key = HandKey {
            hole: our_hole,
            preflop: slices[PREFLOP].actions.clone(),
            starting_stacks: history.starting_stacks(),
        };

        // Find or create this hand's entry under the short global lock, then work
        // under the per-hand lock only.
        let entry_arc: Arc<HandEntry> = {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            match cache.iter().position(|h| h.key == key) {
                Some(i) => {
                    let e = cache.remove(i).unwrap();
                    cache.push_front(e.clone());
                    e
                }
                None => {
                    let e = Arc::new(HandEntry {
                        key: key.clone(),
                        streets: Mutex::new([None, None, None]),
                    });
                    cache.push_front(e.clone());
                    while cache.len() > self.cfg.cache_hands.max(1) {
                        cache.pop_back();
                    }
                    e
                }
            }
        };
        let mut streets_guard = entry_arc.streets.lock().unwrap_or_else(|e| e.into_inner());
        let streets: &mut [Option<StreetSolve>; 3] = &mut streets_guard;

        // Make sure every searched street up to the current one has a valid solve.
        for s in first_street..=street {
            let slice = &slices[s];
            let eff = slice.stacks_start[0].min(slice.stacks_start[1]) as i32;
            let pot = slice.pot_start as i32;
            if eff <= 0 {
                return Err("effective stack is zero (all-in), nothing to solve".to_string());
            }
            let line = to_solver_line(slice, eff)?;
            let idx = s - FLOP;
            let street_board = &board_ids[..board_len_for_street(s)];

            // An earlier street's solve is only needed to seed the next street's
            // ranges. Once that next street is solved on this board, skip it (the
            // flop solve may already have been freed).
            if s < street {
                let next_board = &board_ids[..board_len_for_street(s + 1)];
                if matches!(&streets[idx + 1], Some(n) if n.board == next_board) {
                    continue;
                }
            }

            let reusable = match &mut streets[idx] {
                Some(ss) => {
                    ss.board == street_board
                        && ss.pot_start == pot
                        && ss.eff_stack == eff
                        && line_indices(&mut ss.game, &line.actions).is_ok()
                }
                None => false,
            };
            if reusable {
                continue;
            }

            // Ranges at the start of this street.
            let entry_ranges = match &mut streets[idx] {
                Some(ss) if ss.board == street_board && ss.pot_start == pot && ss.eff_stack == eff => {
                    ss.entry_ranges.clone()
                }
                _ => {
                    let prev_available = idx > 0
                        && matches!(&streets[idx - 1], Some(ps)
                            if ps.board == &board_ids[..board_len_for_street(s - 1)]);
                    if s == TURN && (s == first_street || !prev_available)
                        && self.book.is_some()
                        && self.book_turn_entry_ranges(board, &slices, history, our_hole, self.cfg.own_hand_floor as f32).is_some()
                    {
                        self.book_turn_entry_ranges(board, &slices, history, our_hole, self.cfg.own_hand_floor as f32).unwrap()
                    } else if s == first_street || !prev_available {
                        // Replay the blueprint (translated) through all earlier streets.
                        let earlier: Vec<Action> = slices[..s]
                            .iter()
                            .flat_map(|sl| sl.actions.iter().cloned())
                            .collect();
                        blueprint_entry_ranges(
                            self.blueprint.nodes(),
                            &board[..n_board],
                            &earlier,
                            history.starting_stacks(),
                            s,
                            our_hole,
                            self.cfg.own_hand_floor as f32,
                            self.cfg.range_prune_threshold as f32,
                            history.player,
                        )?
                    } else {
                        let prev = streets[idx - 1]
                            .as_mut()
                            .ok_or("previous street not solved")?;
                        let prev_slice = &slices[s - 1];
                        let prev_line = to_solver_line(prev_slice, prev.eff_stack)?;
                        match ranges_after_street(
                            prev,
                            &prev_line.actions,
                            solver_player(history.player),
                            our_hole,
                            self.cfg.own_hand_floor as f32,
                        ) {
                            Ok(r) => r,
                            Err(e) => {
                                // E.g. our own fallback action went off-tree on the
                                // previous street. Use the blueprint instead.
                                if self.cfg.log {
                                    println!("[SEARCH] previous street ranges unavailable ({}), using blueprint", e);
                                }
                                let earlier: Vec<Action> = slices[..s]
                                    .iter()
                                    .flat_map(|sl| sl.actions.iter().cloned())
                                    .collect();
                                blueprint_entry_ranges(
                                    self.blueprint.nodes(),
                                    &board[..n_board],
                                    &earlier,
                                    history.starting_stacks(),
                                    s,
                                    our_hole,
                                    self.cfg.own_hand_floor as f32,
                                    self.cfg.range_prune_threshold as f32,
                                    history.player,
                                )?
                            }
                        }
                    }
                }
            };

            let tree = build_tree(s, pot, eff, &self.cfg, &line.actions)?;
            let card_config = CardConfig {
                range: entry_ranges.clone(),
                flop: [board_ids[0], board_ids[1], board_ids[2]],
                turn: if s >= TURN { board_ids[3] } else { NOT_DEALT },
                river: if s >= RIVER { board_ids[4] } else { NOT_DEALT },
            };
            let mut game = PostFlopGame::with_config(card_config, tree)?;
            let (mem_f32, mem_c) = game.memory_usage();
            let mem = if self.cfg.compression { mem_c } else { mem_f32 };
            if mem > self.cfg.max_memory_mb * 1024 * 1024 {
                return Err(format!(
                    "solve would need {:.1} MB (limit {} MB)",
                    mem as f64 / 1e6,
                    self.cfg.max_memory_mb
                ));
            }
            let budget = Duration::from_millis(match s {
                FLOP => self.cfg.flop_time_ms,
                TURN => self.cfg.turn_time_ms,
                _ => self.cfg.river_time_ms,
            });
            let stats = solve_with_budget(
                &mut game,
                budget,
                self.cfg.target_exploitability as f32,
                self.cfg.max_iters,
                self.cfg.compression,
            );
            if self.cfg.log {
                println!(
                    "[SEARCH] street {} pot {} eff {} line {:?}: {} iters, expl {:.2}% pot, {:.0} ms, {:.0} MB",
                    s,
                    pot,
                    eff,
                    line.actions,
                    stats.iters,
                    100.0 * stats.exploitability / pot as f32,
                    stats.elapsed.as_secs_f64() * 1000.0,
                    stats.memory_bytes as f64 / 1e6
                );
            }
            if stats.iters < self.cfg.min_iters {
                return Err(format!(
                    "only {} iterations within budget (min {})",
                    stats.iters, self.cfg.min_iters
                ));
            }
            streets[idx] = Some(StreetSolve {
                street: s,
                board: street_board.to_vec(),
                pot_start: pot,
                eff_stack: eff,
                entry_ranges,
                solved_line: line.actions.clone(),
                game,
                stats,
            });
            // Flop solves are large (GBs); free them once the turn is solved. Turn
            // re-solves reuse the entry ranges stored on the turn solve itself.
            if s == TURN {
                streets[0] = None;
            }
        }

        // Navigate the current street's game along the real line and read our strategy.
        let slice = &slices[street];
        let eff = slice.stacks_start[0].min(slice.stacks_start[1]) as i32;
        let line = to_solver_line(slice, eff)?;
        let ss = streets[street - FLOP]
            .as_mut()
            .ok_or("current street not solved")?;
        let result = (|| -> Result<(Strategy, ActionValues), String> {
            let indices = line_indices(&mut ss.game, &line.actions)?;
            ss.game.apply_history(&indices);
            if ss.game.is_terminal_node() || ss.game.is_chance_node() {
                return Err("real line leads to a terminal or chance node".to_string());
            }
            let me = solver_player(history.player);
            if ss.game.current_player() != me {
                return Err(format!(
                    "solver player {} to act but we are {}",
                    ss.game.current_player(),
                    me
                ));
            }
            let hands = ss.game.private_cards(me);
            let hand_idx = hands
                .binary_search(&our_hole)
                .map_err(|_| "our hand is not in our range".to_string())?;
            let num_hands = hands.len();
            let strat = ss.game.strategy();
            let actions = ss.game.available_actions();
            ss.game.cache_normalized_weights();
            let ev = ss.game.expected_values_detail(me);
            let mut strategy = Strategy::new();
            let mut acc: HashMap<Action, (f64, f64, f64, usize)> = HashMap::new();
            for (ai, sa) in actions.iter().enumerate() {
                let p = strat[ai * num_hands + hand_idx] as f64;
                let v = ev[ai * num_hands + hand_idx] as f64;
                let pa = to_project_action(sa, history, &line);
                *strategy.entry(pa.clone()).or_insert(0.0) += p;
                let e = acc.entry(pa).or_insert((0.0, 0.0, 0.0, 0));
                e.0 += p;
                e.1 += p * v;
                e.2 += v;
                e.3 += 1;
            }
            Ok((strategy, merge_values(&acc)))
        })();
        ss.game.back_to_root();
        let (strategy, values) = result?;
        if self.cfg.log {
            let mut items: Vec<(String, f64)> = strategy
                .iter()
                .map(|(a, p)| (format!("{}", a), *p))
                .collect();
            items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            println!(
                "[SEARCH] {} on {} after {:?}: {:?} ({:.0} ms total)",
                cards2str(hole),
                cards2str(&board[..n_board]),
                line.actions,
                items,
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
        Ok((strategy, values))
    }
}

/// Value of each project action when several solver actions map to it: the
/// probability-weighted mean, or the plain mean if they all have probability 0.
fn merge_values(acc: &HashMap<Action, (f64, f64, f64, usize)>) -> ActionValues {
    acc.iter()
        .map(|(a, &(p, pv, v, c))| {
            let val = if p > 0.0 { pv / p } else { v / c.max(1) as f64 };
            (a.clone(), val)
        })
        .collect()
}

/// Scales bet/raise/all-in amounts (used to map between a book spot's pot and the
/// real pot when they differ slightly).
pub fn scale_action(a: SolverAction, scale: f64) -> SolverAction {
    if (scale - 1.0).abs() < 1e-9 {
        return a;
    }
    let f = |x: i32| ((x as f64) * scale).round() as i32;
    match a {
        SolverAction::Bet(x) => SolverAction::Bet(f(x)),
        SolverAction::Raise(x) => SolverAction::Raise(f(x)),
        SolverAction::AllIn(x) => SolverAction::AllIn(f(x)),
        other => other,
    }
}

/// Last line of defense: maps an action onto a legal real-game action.
pub fn legalize(action: Action, history: &ActionHistory) -> Action {
    let to_call = history.to_call();
    let max_bet = history.max_bet();
    match action.action {
        ActionType::Fold if to_call == 0 => {
            println!("[SEARCH] fold with nothing to call -> check");
            Action { action: ActionType::Call, amount: 0 }
        }
        ActionType::Fold => action,
        ActionType::Call => Action { action: ActionType::Call, amount: to_call },
        ActionType::Bet => {
            let legal = history.adjust_action(&action);
            if legal.amount == 0 || legal.amount == to_call {
                Action { action: ActionType::Call, amount: to_call }
            } else {
                Action { action: ActionType::Bet, amount: legal.amount.min(max_bet) }
            }
        }
    }
}

/// Zeroes out actions below `threshold` and renormalizes. Keeps the best action if
/// everything would be removed.
pub fn purify(strategy: &mut Strategy, threshold: f64) {
    let best = strategy
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(a, _)| a.clone());
    for (_, p) in strategy.iter_mut() {
        if *p < threshold {
            *p = 0.0;
        }
    }
    let total: f64 = strategy.values().sum();
    if total <= 0.0 {
        if let Some(best) = best {
            strategy.clear();
            strategy.insert(best, 1.0);
        }
        return;
    }
    strategy.retain(|_, p| *p > 0.0);
    for (_, p) in strategy.iter_mut() {
        *p /= total;
    }
}

/// Walks `line` from the root of `game`, returning the action indices, or an error
/// if some action is not in the tree. Leaves the game at the root.
pub fn line_indices(game: &mut PostFlopGame, line: &[SolverAction]) -> Result<Vec<usize>, String> {
    game.back_to_root();
    let mut indices = Vec::with_capacity(line.len());
    let mut result = Ok(());
    for action in line {
        if game.is_terminal_node() || game.is_chance_node() {
            result = Err(format!("line continues past a terminal/chance node at {:?}", action));
            break;
        }
        let actions = game.available_actions();
        match actions.iter().position(|a| a == action) {
            Some(i) => {
                indices.push(i);
                game.play(i);
            }
            None => {
                result = Err(format!("action {:?} not in tree ({:?})", action, actions));
                break;
            }
        }
    }
    game.back_to_root();
    result.map(|_| indices)
}

fn bet_sizes(bets: &str, raises: &str) -> Result<BetSizeOptions, String> {
    BetSizeOptions::try_from((bets, raises))
}

/// Builds the action tree for a street with the real line guaranteed to be present.
pub fn build_tree(
    street: usize,
    pot: i32,
    eff: i32,
    cfg: &SearchConfig,
    line: &[SolverAction],
) -> Result<ActionTree, String> {
    let flop = bet_sizes(&cfg.flop_bet_sizes, &cfg.flop_raise_sizes)?;
    let (turn, river, caps) = if street == FLOP {
        let later = match cfg.flop_tree_later_max_raises {
            0 => cfg.flop_tree_max_raises,
            n => n,
        };
        (
            bet_sizes(&cfg.flop_tree_turn_bet_sizes, &cfg.flop_tree_turn_raise_sizes)?,
            bet_sizes(&cfg.flop_tree_river_bet_sizes, &cfg.flop_tree_river_raise_sizes)?,
            [cfg.flop_tree_max_raises, later],
        )
    } else {
        (
            bet_sizes(&cfg.turn_bet_sizes, &cfg.turn_raise_sizes)?,
            bet_sizes(&cfg.river_bet_sizes, &cfg.river_raise_sizes)?,
            [cfg.max_raises_per_street; 2],
        )
    };
    let tree_config = TreeConfig {
        initial_state: match street {
            FLOP => BoardState::Flop,
            TURN => BoardState::Turn,
            _ => BoardState::River,
        },
        starting_pot: pot,
        effective_stack: eff,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: [flop.clone(), flop],
        turn_bet_sizes: [turn.clone(), turn],
        river_bet_sizes: [river.clone(), river],
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: cfg.add_allin_threshold,
        force_allin_threshold: cfg.force_allin_threshold,
        merging_threshold: cfg.merging_threshold,
    };
    let mut tree = ActionTree::new(tree_config)?;
    cap_raises(&mut tree, caps, false, 0)?;
    tree.back_to_root();
    for action in line {
        if tree.is_terminal_node() || tree.is_chance_node() {
            return Err(format!("real line continues past the end of the street at {:?}", action));
        }
        if !tree.available_actions().contains(action) {
            tree.add_action(*action)
                .map_err(|e| format!("could not add {:?} to tree: {}", action, e))?;
        }
        tree.play(*action)
            .map_err(|e| format!("could not play {:?} in tree: {}", action, e))?;
    }
    tree.back_to_root();
    Ok(tree)
}

/// Removes bet/raise actions (other than all-in) at nodes where the current street
/// already has `cap` bets (the first bet counts), so the tree does not explode at deep
/// stacks. `caps` = [root street, later streets].
pub fn cap_raises(tree: &mut ActionTree, caps: [usize; 2], later: bool, raises_this_street: usize) -> Result<(), String> {
    if tree.is_terminal_node() {
        return Ok(());
    }
    let later = later || tree.is_chance_node();
    let raises = if tree.is_chance_node() { 0 } else { raises_this_street };
    let cap = caps[later as usize];
    let actions: Vec<SolverAction> = tree.available_actions().to_vec();
    for action in actions {
        let is_raise = matches!(action, SolverAction::Bet(_) | SolverAction::Raise(_));
        let is_allin = matches!(action, SolverAction::AllIn(_));
        if is_raise && raises >= cap {
            tree.remove_action(action)?;
            continue;
        }
        tree.play(action)?;
        let next = raises + (is_raise || is_allin) as usize;
        let result = cap_raises(tree, caps, later, next);
        tree.undo()?;
        result?;
    }
    Ok(())
}

/// Runs DCFR until the target exploitability, the iteration cap or the time budget
/// is reached, then finalizes the game.
pub fn solve_with_budget(
    game: &mut PostFlopGame,
    budget: Duration,
    target_frac: f32,
    max_iters: u32,
    compression: bool,
) -> SolveStats {
    let start = Instant::now();
    let (mem_f32, mem_c) = game.memory_usage();
    game.allocate_memory(compression);
    let pot = game.tree_config().starting_pot as f32;
    let target = pot * target_frac;
    let check_every = if game.tree_config().initial_state == BoardState::Flop { 25 } else { 10 };
    let mut exploitability = f32::INFINITY;
    let mut iters = 0u32;
    for t in 0..max_iters {
        solve_step(&*game, t);
        iters = t + 1;
        if iters % check_every == 0 {
            exploitability = compute_exploitability(&*game);
            if exploitability <= target {
                break;
            }
        }
        if start.elapsed() >= budget {
            break;
        }
    }
    if iters % check_every != 0 || exploitability.is_infinite() {
        exploitability = compute_exploitability(&*game);
    }
    finalize(game);
    SolveStats {
        iters,
        exploitability,
        elapsed: start.elapsed(),
        memory_bytes: if compression { mem_c } else { mem_f32 },
    }
}

/// Ranges [OOP, IP] at the start of `street`, from replaying the translated real
/// line through all earlier streets against the blueprint, for every non-blocking combo.
/// `board` must contain at least the cards visible at `street`.
pub fn blueprint_entry_ranges(
    nodes: &ReadOnlyNodes,
    board: &[Card],
    earlier_actions: &[Action],
    starting_stacks: [Amount; 2],
    street: usize,
    our_hole: (u8, u8),
    own_hand_floor: f32,
    prune_threshold: f32,
    our_project_player: usize,
) -> Result<[SolverRange; 2], String> {
    let board = &board[..board_len_for_street(street)];
    let mut real = ActionHistory::new_with_stacks(starting_stacks);
    for a in earlier_actions {
        real.add(a);
    }
    if real.street != street {
        return Err(format!(
            "earlier actions end on street {} but ranges are needed for street {}",
            real.street, street
        ));
    }
    let translated = real.translate(&CONFIG.bet_abstraction);
    let hands = non_blocking_preflop_hands(board);
    let mut reach = [vec![1.0f64; hands.len()], vec![1.0f64; hands.len()]];
    let mut iter = ActionHistory::new();
    for action in translated.get_actions() {
        if iter.street >= street {
            break;
        }
        let player = iter.player;
        // Probe once: if the blueprint has no node for this translated line (e.g.
        // the opponent raised past the abstraction's raise cap), stop here and keep
        // the ranges as they were before the untrained actions.
        let trained = nodes
            .try_get_strategy(&hands[0], board, &iter)
            .map(|st| st.contains_key(&action))
            .unwrap_or(false);
        if !trained {
            println!(
                "[SEARCH] blueprint has no node/action for {} after {} ; ranges stop before it",
                action, iter
            );
            break;
        }
        for (i, hand) in hands.iter().enumerate() {
            let strat = nodes.get_strategy(hand, board, &iter);
            let p = strat.get(&action).copied().unwrap_or(0.0);
            reach[player][i] *= p;
        }
        iter.add(&action);
    }
    let mut ranges: Vec<SolverRange> = Vec::with_capacity(2);
    // Solver order: [OOP, IP] = [OPPONENT, DEALER].
    for project_player in [OPPONENT, DEALER] {
        let weights = &reach[project_player];
        let pairs: Vec<(u8, u8)> = hands.iter().map(|h| solver_hole(h)).collect();
        let range = make_range(
            &pairs,
            weights,
            if project_player == our_project_player { Some(our_hole) } else { None },
            own_hand_floor,
            prune_threshold,
        )?;
        ranges.push(range);
    }
    let ip = ranges.pop().unwrap();
    let oop = ranges.pop().unwrap();
    Ok([oop, ip])
}

/// Ranges [OOP, IP] at the end of a solved street (i.e. the start of the next one).
pub fn ranges_after_street(
    prev: &mut StreetSolve,
    completed_line: &[SolverAction],
    our_solver_player: usize,
    our_hole: (u8, u8),
    own_hand_floor: f32,
) -> Result<[SolverRange; 2], String> {
    let indices = line_indices(&mut prev.game, completed_line)?;
    prev.game.apply_history(&indices);
    let result = (|| -> Result<[SolverRange; 2], String> {
        if !prev.game.is_chance_node() {
            return Err("previous street's line does not end at a chance node".to_string());
        }
        let mut out: Vec<SolverRange> = Vec::with_capacity(2);
        for p in 0..2 {
            let pairs: Vec<(u8, u8)> = prev.game.private_cards(p).to_vec();
            let weights: Vec<f64> = prev.game.weights(p).iter().map(|w| *w as f64).collect();
            out.push(make_range(
                &pairs,
                &weights,
                if p == our_solver_player { Some(our_hole) } else { None },
                own_hand_floor,
                0.0,
            )?);
        }
        let ip = out.pop().unwrap();
        let oop = out.pop().unwrap();
        Ok([oop, ip])
    })();
    prev.game.back_to_root();
    result
}

/// Normalizes weights to [0, 1] (dividing by the max), floors our own hand, and
/// builds a solver range.
fn make_range(
    pairs: &[(u8, u8)],
    weights: &[f64],
    our_hole: Option<(u8, u8)>,
    own_hand_floor: f32,
    prune_threshold: f32,
) -> Result<SolverRange, String> {
    let max = weights.iter().cloned().fold(0.0f64, f64::max);
    if !(max > 0.0) {
        return Err("range has no hands with positive weight".to_string());
    }
    let mut w: Vec<f32> = weights
        .iter()
        .map(|x| {
            let v = ((x / max) as f32).clamp(0.0, 1.0);
            if v < prune_threshold {
                0.0
            } else {
                v
            }
        })
        .collect();
    if let Some(hole) = our_hole {
        match pairs.iter().position(|p| *p == hole) {
            Some(i) => {
                if w[i] < own_hand_floor {
                    w[i] = own_hand_floor;
                }
            }
            None => return Err("our hand is blocked by the board".to_string()),
        }
    }
    SolverRange::from_hands_weights(pairs, &w)
}
