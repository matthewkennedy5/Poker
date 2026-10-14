//! Flop book: precomputed flop-street strategies for the common preflop lines,
//! solved with postflop-solver (exact hands, to the river) for every one of the
//! 1,755 suit-distinct flops.
//!
//! Build: `flop-book build --lines <file>` (see bin/flop_book.rs). One file per
//! preflop line in `products/flopbook/`, holding one `FlopSolution` per canonical
//! flop, appended as it is solved (so a build can be resumed).
//!
//! Use: `FlopBook::load(dir)` at server start; `SearchBot` consults it on the flop
//! and seeds the turn search from the ranges implied by the stored strategies.

use crate::card_utils::*;
use crate::config::{SearchConfig, CONFIG};
use crate::readonly::ReadOnlyNodes;
use crate::search::{blueprint_entry_ranges, build_tree, solve_with_budget, SolveStats};
use crate::search_utils::*;
use crate::trainer_utils::*;
use postflop_solver::{Action as SolverAction, CardConfig, PostFlopGame, NOT_DEALT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Suit isomorphism
// ---------------------------------------------------------------------------

/// A permutation of the four suits: `map[real_suit] = canonical_suit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuitPerm {
    pub map: [u8; 4],
}

impl SuitPerm {
    pub fn inverse(&self) -> SuitPerm {
        let mut inv = [0u8; 4];
        for s in 0..4 {
            inv[self.map[s] as usize] = s as u8;
        }
        SuitPerm { map: inv }
    }

    pub fn apply(&self, c: &Card) -> Card {
        Card {
            rank: c.rank,
            suit: self.map[c.suit as usize],
        }
    }
}

/// Canonical form of a flop under suit permutation: cards sorted by rank
/// descending, suits relabelled in order of first appearance (then any suits not
/// on the board get the remaining labels in order). Returns the canonical flop and
/// the permutation that maps real suits to canonical suits.
pub fn canonical_flop(flop: &[Card]) -> ([Card; 3], SuitPerm) {
    assert_eq!(flop.len(), 3);
    // Consider every ordering of the cards that is non-increasing in rank (equal
    // ranks may be swapped), label suits by first appearance, and keep the
    // lexicographically smallest result so the form is unique up to suit symmetry.
    let orders: [[usize; 3]; 6] = [
        [0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0],
    ];
    let mut best: Option<([Card; 3], SuitPerm, [u8; 6])> = None;
    for order in orders.iter() {
        let cards = [flop[order[0]], flop[order[1]], flop[order[2]]];
        if cards[0].rank < cards[1].rank || cards[1].rank < cards[2].rank {
            continue;
        }
        let mut map = [u8::MAX; 4];
        let mut next = 0u8;
        for c in &cards {
            if map[c.suit as usize] == u8::MAX {
                map[c.suit as usize] = next;
                next += 1;
            }
        }
        for s in 0..4 {
            if map[s] == u8::MAX {
                map[s] = next;
                next += 1;
            }
        }
        let perm = SuitPerm { map };
        let canon = [perm.apply(&cards[0]), perm.apply(&cards[1]), perm.apply(&cards[2])];
        let key = [
            255 - canon[0].rank, canon[0].suit, 255 - canon[1].rank, canon[1].suit,
            255 - canon[2].rank, canon[2].suit,
        ];
        if best.as_ref().map_or(true, |(_, _, k)| key < *k) {
            best = Some((canon, perm, key));
        }
    }
    let (canon, perm, _) = best.unwrap();
    (canon, perm)
}

/// All canonical flops (1,755 of them), in a fixed order.
pub fn all_canonical_flops() -> Vec<[Card; 3]> {
    let deck = deck();
    let mut seen: HashMap<[u8; 6], [Card; 3]> = HashMap::new();
    let mut out = Vec::new();
    for i in 0..deck.len() {
        for j in i + 1..deck.len() {
            for k in j + 1..deck.len() {
                let (canon, _) = canonical_flop(&[deck[i], deck[j], deck[k]]);
                let key = [
                    canon[0].rank, canon[0].suit, canon[1].rank, canon[1].suit, canon[2].rank,
                    canon[2].suit,
                ];
                if !seen.contains_key(&key) {
                    seen.insert(key, canon);
                    out.push(canon);
                }
            }
        }
    }
    out
}

pub fn flop_key(flop: &[Card; 3]) -> String {
    cards2str(flop)
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

/// Serializable encoding of a solver action.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct SAct {
    pub kind: u8,
    pub amount: i32,
}

pub fn enc(a: SolverAction) -> SAct {
    match a {
        SolverAction::None => SAct { kind: 0, amount: 0 },
        SolverAction::Fold => SAct { kind: 1, amount: 0 },
        SolverAction::Check => SAct { kind: 2, amount: 0 },
        SolverAction::Call => SAct { kind: 3, amount: 0 },
        SolverAction::Bet(x) => SAct { kind: 4, amount: x },
        SolverAction::Raise(x) => SAct { kind: 5, amount: x },
        SolverAction::AllIn(x) => SAct { kind: 6, amount: x },
        SolverAction::Chance(c) => SAct { kind: 7, amount: c as i32 },
    }
}

pub fn dec(s: SAct) -> SolverAction {
    match s.kind {
        1 => SolverAction::Fold,
        2 => SolverAction::Check,
        3 => SolverAction::Call,
        4 => SolverAction::Bet(s.amount),
        5 => SolverAction::Raise(s.amount),
        6 => SolverAction::AllIn(s.amount),
        7 => SolverAction::Chance(s.amount as u8),
        _ => SolverAction::None,
    }
}

/// Strategy at one flop-street decision node of the solved tree.
#[derive(Serialize, Deserialize, Clone)]
pub struct NodeStrategy {
    /// Solver actions from the start of the flop to this node.
    pub path: Vec<SAct>,
    /// Solver player to act (0 = OOP/big blind, 1 = IP/button).
    pub player: u8,
    pub actions: Vec<SAct>,
    /// `strategy[action_idx * num_hands + hand_idx]`.
    pub strategy: Vec<f32>,
    /// Expected value (chips, solver convention: share of the pot) of each action
    /// for each hand, same layout as `strategy`. Empty for v1 books.
    pub ev: Vec<f32>,
}

/// Book node format before per-action values were stored (record version 1).
#[derive(Serialize, Deserialize, Clone)]
pub struct NodeStrategyV1 {
    pub path: Vec<SAct>,
    pub player: u8,
    pub actions: Vec<SAct>,
    pub strategy: Vec<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FlopSolutionV1 {
    pub flop: String,
    pub pot: i32,
    pub eff_stack: i32,
    pub hands: [Vec<(u8, u8)>; 2],
    pub entry_weights: [Vec<f32>; 2],
    pub nodes: Vec<NodeStrategyV1>,
    pub iters: u32,
    pub exploitability: f32,
}

impl From<FlopSolutionV1> for FlopSolution {
    fn from(o: FlopSolutionV1) -> FlopSolution {
        FlopSolution {
            flop: o.flop,
            pot: o.pot,
            eff_stack: o.eff_stack,
            hands: o.hands,
            entry_weights: o.entry_weights,
            nodes: o
                .nodes
                .into_iter()
                .map(|n| NodeStrategy {
                    path: n.path,
                    player: n.player,
                    actions: n.actions,
                    strategy: n.strategy,
                    ev: Vec::new(),
                })
                .collect(),
            iters: o.iters,
            exploitability: o.exploitability,
            root_ev: [Vec::new(), Vec::new()],
        }
    }
}

impl NodeStrategy {
    /// Per-action value of hand `h` at this node, if values were stored.
    pub fn action_values(&self, h: usize) -> Option<Vec<f64>> {
        if self.ev.is_empty() {
            return None;
        }
        let n = self.strategy.len() / self.actions.len().max(1);
        Some((0..self.actions.len()).map(|a| self.ev[a * n + h] as f64).collect())
    }

    pub fn actions(&self) -> Vec<SolverAction> {
        self.actions.iter().map(|&a| dec(a)).collect()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FlopSolution {
    /// Canonical flop as a string, e.g. "Td9d6h" relabelled.
    pub flop: String,
    pub pot: i32,
    pub eff_stack: i32,
    /// Private hands of each solver player, solver card ids, (lo, hi), sorted.
    pub hands: [Vec<(u8, u8)>; 2],
    /// Entry range weight of each hand (same order as `hands`).
    pub entry_weights: [Vec<f32>; 2],
    pub nodes: Vec<NodeStrategy>,
    pub iters: u32,
    pub exploitability: f32,
    /// Expected value of each hand of each player at the flop root. Empty for v1.
    pub root_ev: [Vec<f32>; 2],
}

/// Header record written once at the start of each line file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LineHeader {
    /// Abstract preflop line as project action strings, e.g. "Bet 200,Call 100".
    pub line: String,
    pub pot: i32,
    pub eff_stack: i32,
    /// Human-readable description of the tree shape used.
    pub tree: String,
}

#[derive(Serialize, Deserialize)]
enum Record {
    Header(LineHeader),
    /// v1 solutions (no values). Variant order is part of the file format.
    Solution(FlopSolutionV1),
    SolutionV2(FlopSolution),
}

pub struct LineBook {
    pub header: LineHeader,
    pub solutions: HashMap<String, FlopSolution>,
}

pub struct FlopBook {
    /// Keyed by abstract preflop line string.
    pub lines: HashMap<String, LineBook>,
}

/// Project preflop actions -> canonical line string.
pub fn line_string(actions: &[Action]) -> String {
    actions
        .iter()
        .map(|a| {
            let kind = match a.action {
                ActionType::Bet => "Bet",
                ActionType::Call => "Call",
                ActionType::Fold => "Fold",
            };
            format!("{} {}", kind, a.amount)
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub fn parse_line(line: &str) -> Vec<Action> {
    line.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut it = s.split(' ');
            let kind = it.next().unwrap();
            let amount: Amount = it.next().unwrap().parse().unwrap();
            Action {
                action: match kind {
                    "Bet" => ActionType::Bet,
                    "Call" => ActionType::Call,
                    "Fold" => ActionType::Fold,
                    _ => panic!("bad action {}", s),
                },
                amount,
            }
        })
        .collect()
}

pub fn tree_description(cfg: &SearchConfig) -> String {
    let later = match cfg.flop_tree_later_max_raises {
        0 => String::new(),
        n => format!("/{}", n),
    };
    format!(
        "flop[{}|{}] turn[{}|{}] river[{}|{}] cap{}{} allin{}/{} merge{}",
        cfg.flop_bet_sizes,
        cfg.flop_raise_sizes,
        cfg.flop_tree_turn_bet_sizes,
        cfg.flop_tree_turn_raise_sizes,
        cfg.flop_tree_river_bet_sizes,
        cfg.flop_tree_river_raise_sizes,
        cfg.flop_tree_max_raises,
        later,
        cfg.add_allin_threshold,
        cfg.force_allin_threshold,
        cfg.merging_threshold
    )
}

pub fn line_file_name(line: &str) -> String {
    let safe: String = line
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{}.bin", safe)
}

// ---------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------

/// Solves one (line, canonical flop) spot.
pub fn solve_spot(
    nodes: &ReadOnlyNodes,
    cfg: &SearchConfig,
    line_actions: &[Action],
    flop: &[Card; 3],
    budget: Duration,
    target_frac: f32,
    max_iters: u32,
) -> Result<(FlopSolution, SolveStats), String> {
    let mut hist = ActionHistory::new();
    for a in line_actions {
        hist.add(a);
    }
    if hist.street != FLOP {
        return Err(format!("line {} does not end at the flop", line_string(line_actions)));
    }
    let pot = hist.pot() as i32;
    let stacks = hist.stack_sizes();
    let eff = stacks[0].min(stacks[1]) as i32;
    let board: Vec<Card> = flop.to_vec();
    // Entry ranges for both players. No "own hand" floor: the book is for any hand.
    let dummy_hole = (0u8, 1u8);
    let ranges = blueprint_entry_ranges(
        nodes,
        &board,
        line_actions,
        hist.starting_stacks(),
        FLOP,
        dummy_hole,
        0.0,
        cfg.range_prune_threshold as f32,
        usize::MAX, // no player is "ours"
    )?;
    let tree = build_tree(FLOP, pot, eff, cfg, &[])?;
    let ids: Vec<u8> = flop.iter().map(to_solver_card).collect();
    let card_config = CardConfig {
        range: ranges.clone(),
        flop: [ids[0], ids[1], ids[2]],
        turn: NOT_DEALT,
        river: NOT_DEALT,
    };
    let mut game = PostFlopGame::with_config(card_config, tree)?;
    let stats = solve_with_budget(&mut game, budget, target_frac, max_iters, cfg.compression);

    // Collect flop-street strategies by DFS over player nodes.
    let hands = [game.private_cards(0).to_vec(), game.private_cards(1).to_vec()];
    let entry_weights = [
        hands[0].iter().map(|&(a, b)| ranges[0].get_weight_by_cards(a, b)).collect(),
        hands[1].iter().map(|&(a, b)| ranges[1].get_weight_by_cards(a, b)).collect(),
    ];
    let mut out_nodes = Vec::new();
    let mut path: Vec<SolverAction> = Vec::new();
    collect_flop_nodes(&mut game, &mut path, &mut out_nodes);
    game.back_to_root();
    game.cache_normalized_weights();
    let root_ev = [game.expected_values(0), game.expected_values(1)];
    Ok((
        FlopSolution {
            flop: flop_key(flop),
            pot,
            eff_stack: eff,
            hands,
            entry_weights,
            nodes: out_nodes,
            iters: stats.iters,
            exploitability: stats.exploitability,
            root_ev,
        },
        stats,
    ))
}

fn collect_flop_nodes(game: &mut PostFlopGame, path: &mut Vec<SolverAction>, out: &mut Vec<NodeStrategy>) {
    if game.is_terminal_node() || game.is_chance_node() {
        return;
    }
    let actions = game.available_actions();
    let player = game.current_player() as u8;
    let strategy = game.strategy();
    game.cache_normalized_weights();
    let ev = game.expected_values_detail(player as usize);
    out.push(NodeStrategy {
        path: path.iter().map(|&a| enc(a)).collect(),
        player,
        actions: actions.iter().map(|&a| enc(a)).collect(),
        strategy,
        ev,
    });
    for (i, a) in actions.iter().enumerate() {
        game.play(i);
        path.push(*a);
        collect_flop_nodes(game, path, out);
        path.pop();
        // back up one level: replay the path from the root
        let history = game.history().to_vec();
        game.back_to_root();
        game.apply_history(&history[..history.len() - 1]);
    }
}

/// Appends solutions for every canonical flop not already present in the line's
/// file. Returns the number of newly solved flops.
/// Which canonical flops (by index in `all_canonical_flops()`) a build covers:
/// index % n in `residues`. "0,1/3" takes two thirds, so a faster box can take a
/// bigger share; each box writes its own file and the loader merges files of a line.
#[derive(Debug, Clone)]
pub struct Shard {
    pub residues: Vec<usize>,
    pub n: usize,
}

impl Shard {
    pub fn all() -> Shard {
        Shard { residues: vec![0], n: 1 }
    }

    /// Parses "k/n" or "k1,k2/n".
    pub fn parse(s: &str) -> Result<Shard, String> {
        let (r, n) = s.split_once('/').ok_or(format!("bad shard '{}': want k/n", s))?;
        let n: usize = n.trim().parse().map_err(|_| format!("bad shard count in '{}'", s))?;
        let residues = r
            .split(',')
            .map(|k| k.trim().parse::<usize>().map_err(|_| format!("bad shard index in '{}'", s)))
            .collect::<Result<Vec<_>, _>>()?;
        if n == 0 || residues.iter().any(|&k| k >= n) {
            return Err(format!("bad shard '{}'", s));
        }
        Ok(Shard { residues, n })
    }

    pub fn contains(&self, i: usize) -> bool {
        self.residues.contains(&(i % self.n))
    }
}

impl std::fmt::Display for Shard {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let r: Vec<String> = self.residues.iter().map(|k| k.to_string()).collect();
        write!(f, "{}/{}", r.join(","), self.n)
    }
}

pub fn build_line(
    nodes: &ReadOnlyNodes,
    cfg: &SearchConfig,
    dir: &str,
    line: &str,
    budget: Duration,
    target_frac: f32,
    max_iters: u32,
    log: bool,
    max_flops: usize,
    shard: &Shard,
) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = format!("{}/{}", dir, line_file_name(line));
    let actions = parse_line(line);
    let mut hist = ActionHistory::new();
    for a in &actions {
        hist.add(a);
    }
    let pot = hist.pot() as i32;
    let stacks = hist.stack_sizes();
    let eff = stacks[0].min(stacks[1]) as i32;
    let header = LineHeader {
        line: line.to_string(),
        pot,
        eff_stack: eff,
        tree: tree_description(cfg),
    };
    let existing = if Path::new(&path).exists() {
        let lb = load_line_file(&path)?;
        if lb.header.tree != header.tree || lb.header.pot != pot {
            return Err(format!(
                "{} was built with a different tree/pot ({:?}); delete it to rebuild",
                path, lb.header
            ));
        }
        lb.solutions
    } else {
        let mut w = BufWriter::new(File::create(&path).map_err(|e| e.to_string())?);
        bincode::serialize_into(&mut w, &Record::Header(header.clone())).map_err(|e| e.to_string())?;
        w.flush().map_err(|e| e.to_string())?;
        HashMap::new()
    };
    let flops = all_canonical_flops();
    let todo: Vec<[Card; 3]> = flops
        .iter()
        .enumerate()
        .filter(|(i, f)| shard.contains(*i) && !existing.contains_key(&flop_key(f)))
        .map(|(_, f)| f)
        .cloned()
        .take(max_flops)
        .collect();
    println!(
        "[BOOK] line '{}' pot {} eff {}: {} of {} flops already solved, {} to go (shard {})",
        line,
        pot,
        eff,
        existing.len(),
        flops.len(),
        todo.len(),
        shard
    );
    let mut file = OpenOptions::new().append(true).open(&path).map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();
    let mut done = 0usize;
    for flop in &todo {
        let (solution, stats) = solve_spot(nodes, cfg, &actions, flop, budget, target_frac, max_iters)?;
        let bytes = bincode::serialize(&Record::SolutionV2(solution)).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        done += 1;
        if log {
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "[BOOK] {} {}/{}: {} iters, expl {:.2}% pot, {:.0} s ({:.0} MB); {:.1} s/flop avg, ETA {:.1} h",
                flop_key(flop),
                done,
                todo.len(),
                stats.iters,
                100.0 * stats.exploitability / pot as f32,
                stats.elapsed.as_secs_f64(),
                stats.memory_bytes as f64 / 1e6,
                elapsed / done as f64,
                elapsed / done as f64 * (todo.len() - done) as f64 / 3600.0
            );
        }
    }
    Ok(done)
}

fn load_line_file(path: &str) -> Result<LineBook, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let header = match bincode::deserialize_from::<_, Record>(&mut reader) {
        Ok(Record::Header(h)) => h,
        _ => return Err(format!("{}: missing header", path)),
    };
    let mut solutions = HashMap::new();
    loop {
        match bincode::deserialize_from::<_, Record>(&mut reader) {
            Ok(Record::Solution(s)) => {
                let s: FlopSolution = s.into();
                solutions.insert(s.flop.clone(), s);
            }
            Ok(Record::SolutionV2(s)) => {
                solutions.insert(s.flop.clone(), s);
            }
            Ok(Record::Header(_)) => return Err(format!("{}: duplicate header", path)),
            Err(_) => break, // EOF (or a truncated trailing record, which is dropped)
        }
    }
    Ok(LineBook { header, solutions })
}

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

impl FlopBook {
    pub fn load(dir: &str) -> FlopBook {
        let mut lines = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "bin").unwrap_or(false) {
                    match load_line_file(path.to_str().unwrap()) {
                        Ok(lb) => {
                            println!(
                                "[BOOK] loaded '{}' from {}: {} flops (pot {})",
                                lb.header.line,
                                path.display(),
                                lb.solutions.len(),
                                lb.header.pot
                            );
                            // Several files of one line (shards built on different boxes)
                            // are merged when they share a tree and pot.
                            match lines.get_mut(&lb.header.line) {
                                None => {
                                    lines.insert(lb.header.line.clone(), lb);
                                }
                                Some(prev) => {
                                    let prev: &mut LineBook = prev;
                                    if prev.header.tree == lb.header.tree && prev.header.pot == lb.header.pot {
                                        prev.solutions.extend(lb.solutions);
                                    } else {
                                        println!(
                                            "[BOOK] skipping {}: '{}' already loaded with a different tree",
                                            path.display(),
                                            lb.header.line
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => println!("[BOOK] skipping {}: {}", path.display(), e),
                    }
                }
            }
        }
        for lb in lines.values() {
            println!("[BOOK] line '{}': {} flops", lb.header.line, lb.solutions.len());
        }
        FlopBook { lines }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Finds the solution for an abstract preflop line and a real flop. Returns the
    /// solution and the suit permutation (real -> canonical).
    pub fn lookup(&self, line: &str, real_flop: &[Card]) -> Option<(&FlopSolution, SuitPerm)> {
        let lb = self.lines.get(line)?;
        let (canon, perm) = canonical_flop(real_flop);
        let sol = lb.solutions.get(&flop_key(&canon))?;
        Some((sol, perm))
    }
}

impl FlopSolution {
    /// Index of a canonical hole-card pair in `hands[player]`.
    pub fn hand_index(&self, player: usize, canon_hole: (u8, u8)) -> Option<usize> {
        self.hands[player].binary_search(&canon_hole).ok()
    }

    /// Node reached by a solver-action path, if it is in the stored tree.
    pub fn node(&self, path: &[SolverAction]) -> Option<&NodeStrategy> {
        let p: Vec<SAct> = path.iter().map(|&a| enc(a)).collect();
        self.nodes.iter().find(|n| n.path == p)
    }

    /// Maps a real line onto the stored tree: every action must either be present
    /// or be a bet/raise, which is snapped to the nearest stored bet/raise of the
    /// same kind (never to all-in unless it was an all-in). Returns the mapped path.
    pub fn map_line(&self, line: &[SolverAction]) -> Option<Vec<SolverAction>> {
        let mut path: Vec<SolverAction> = Vec::new();
        for a in line {
            let node = self.node(&path)?;
            let node_actions = node.actions();
            let mapped = if node_actions.contains(a) {
                *a
            } else {
                match a {
                    SolverAction::Bet(x) | SolverAction::Raise(x) => {
                        let mut best: Option<(i64, SolverAction)> = None;
                        for c in &node_actions {
                            let amt = match c {
                                SolverAction::Bet(y) | SolverAction::Raise(y) => *y,
                                _ => continue,
                            };
                            let d = ((amt as f64).ln() - (*x as f64).ln()).abs();
                            let d = (d * 1e6) as i64;
                            if best.map_or(true, |(bd, _)| d < bd) {
                                best = Some((d, *c));
                            }
                        }
                        match best {
                            Some((_, c)) => c,
                            None => {
                                // no sized bet stored here: use all-in if present
                                *node_actions.iter().find(|c| matches!(c, SolverAction::AllIn(_)))?
                            }
                        }
                    }
                    SolverAction::AllIn(_) => {
                        *node_actions.iter().find(|c| matches!(c, SolverAction::AllIn(_)))?
                    }
                    _ => return None,
                }
            };
            path.push(mapped);
        }
        Some(path)
    }

    /// Reach-weighted ranges of both players at the end of `path` (entry weights
    /// times the stored strategy probabilities of the actions taken). Returned in
    /// canonical coordinates, same order as `hands`.
    pub fn ranges_after(&self, path: &[SolverAction]) -> Option<[Vec<f32>; 2]> {
        let mut w = [self.entry_weights[0].clone(), self.entry_weights[1].clone()];
        let mut prefix: Vec<SolverAction> = Vec::new();
        for a in path {
            let node = self.node(&prefix)?;
            let ai = node.actions.iter().position(|c| *c == enc(*a))?;
            let p = node.player as usize;
            let n = self.hands[p].len();
            for h in 0..n {
                w[p][h] *= node.strategy[ai * n + h];
            }
            prefix.push(*a);
        }
        Some(w)
    }
}

/// Convenience: the abstract preflop line string for a real history.
pub fn abstract_preflop_line(history: &ActionHistory) -> String {
    let translated = history.translate(&CONFIG.bet_abstraction);
    let mut pre = Vec::new();
    let mut iter = ActionHistory::new();
    for a in translated.get_actions() {
        if iter.street != PREFLOP {
            break;
        }
        pre.push(a.clone());
        iter.add(&a);
    }
    line_string(&pre)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_flops_count() {
        assert_eq!(all_canonical_flops().len(), 1755);
    }

    #[test]
    fn canonical_perm_applies() {
        let (canon, perm) = canonical_flop(&str2cards("6hTd9d"));
        assert_eq!(cards2str(&canon), "Tc9c6d");
        let (a, _) = canonical_flop(&str2cards("AhAd5h"));
        let (b, _) = canonical_flop(&str2cards("AdAh5d"));
        assert_eq!(cards2str(&a), cards2str(&b));
        // diamonds -> clubs, hearts -> diamonds
        assert_eq!(perm.apply(&Card::new("Ad")), Card::new("Ac"));
        assert_eq!(perm.apply(&Card::new("Ah")), Card::new("Ad"));
        // the two unseen suits map to the remaining two labels
        let s = perm.apply(&Card::new("As")).suit;
        let c = perm.apply(&Card::new("Ac")).suit;
        assert!(s != c && s >= 2 && c >= 2);
    }

    #[test]
    fn line_round_trip() {
        let line = "Bet 200,Call 100";
        assert_eq!(line_string(&parse_line(line)), line);
    }
}
