//! Generates turn-net training data: turn spots solved to the end of the hand by
//! postflop-solver, one single-threaded solve per worker.
//!
//! Each record holds the spot (board, pot, stack, both ranges) and the value of
//! every hand for both players at the turn root: expected chips relative to an
//! even split of the pot, divided by the pot, given the opponent's range (NaN for
//! hands outside the player's range). Records are appended to
//! `<out>/turn_<shard>.bin` (bincode), so runs are resumable; spot k of a shard is
//! generated from a fixed seed, so a rerun continues where it stopped.
//!
//! Usage: datagen --out <dir> [--count N] [--shard k/n] [--workers W]
//!                [--tree live|rich] [--target 0.005] [--book <dir> --book-share 0.5]
//!        datagen --verify <file>   (checks zero-sum and prints statistics)
use optimus::spots::{harvest_flop_ends, random_spot, turn_spot, FlopEnd, Spot};
use optimus::{cap_raises, FlopBook, Hands};
use postflop_solver::{
    compute_exploitability, finalize, solve_step, ActionTree, BetSizeOptions, BoardState,
    CardConfig, PostFlopGame, Range, TreeConfig, NOT_DEALT,
};
use rand::prelude::*;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Serialize, Deserialize)]
pub struct Sample {
    pub index: u64,
    pub spot: Spot,
    /// [OOP, IP] per-hand values in pot units, `Hands::new(board)` order.
    pub values: [Vec<f32>; 2],
    pub iters: u32,
    /// Exploitability in pot units.
    pub exploitability: f32,
    pub seconds: f32,
    pub tree: String,
}

fn sizes(b: &str, r: &str) -> BetSizeOptions {
    BetSizeOptions::try_from((b, r)).unwrap()
}

fn tree_sizes(name: &str) -> (BetSizeOptions, BetSizeOptions) {
    match name {
        "live" => (sizes("50%, 100%", "2.5x"), sizes("50%, 100%, a", "2.5x")),
        "rich" => (sizes("33%, 75%, 125%", "2.5x"), sizes("33%, 75%, 125%, a", "2.5x")),
        t => panic!("unknown tree {}", t),
    }
}

fn solve_spot(spot: &Spot, tree_name: &str, target: f32) -> Result<([Vec<f32>; 2], u32, f32), String> {
    let hands = Hands::new(&spot.board);
    let ranges: Vec<Range> = (0..2)
        .map(|p| Range::from_hands_weights(&hands.cards, &spot.weights[p]))
        .collect::<Result<_, _>>()?;
    let (turn, river) = tree_sizes(tree_name);
    let tc = TreeConfig {
        initial_state: BoardState::Turn,
        starting_pot: spot.pot,
        effective_stack: spot.stack,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: [sizes("", ""), sizes("", "")],
        turn_bet_sizes: [turn.clone(), turn],
        river_bet_sizes: [river.clone(), river],
        turn_donk_sizes: None,
        river_donk_sizes: None,
        add_allin_threshold: 1.5,
        force_allin_threshold: 0.15,
        merging_threshold: 0.1,
    };
    let mut tree = ActionTree::new(tc)?;
    cap_raises(&mut tree, [2, 2], false, 0)?;
    tree.back_to_root();
    let cc = CardConfig {
        range: [ranges[0], ranges[1]],
        flop: [spot.board[0], spot.board[1], spot.board[2]],
        turn: spot.board[3],
        river: NOT_DEALT,
    };
    let mut game = PostFlopGame::with_config(cc, tree)?;
    game.allocate_memory(false);
    let pot = spot.pot as f32;
    let mut iters = 0;
    let mut expl = f32::INFINITY;
    for t in 0..3000u32 {
        solve_step(&game, t);
        iters = t + 1;
        if iters % 10 == 0 {
            expl = compute_exploitability(&game);
            if expl <= target * pot {
                break;
            }
        }
    }
    if iters % 10 != 0 {
        expl = compute_exploitability(&game);
    }
    finalize(&mut game);
    game.back_to_root();
    game.cache_normalized_weights();
    let mut values = [vec![f32::NAN; hands.len()], vec![f32::NAN; hands.len()]];
    for p in 0..2 {
        let ev = game.expected_values(p);
        for (k, &(a, b)) in game.private_cards(p).iter().enumerate() {
            let i = hands.index(a, b).ok_or("hand not in list")?;
            values[p][i] = (ev[k] - 0.5 * pot) / pot;
        }
    }
    Ok((values, iters, expl / pot))
}

fn count_records(path: &str) -> u64 {
    let Ok(f) = std::fs::File::open(path) else { return 0 };
    let mut r = BufReader::new(f);
    let mut n = 0;
    while bincode::deserialize_from::<_, Sample>(&mut r).is_ok() {
        n += 1;
    }
    n
}

/// Range-weighted value of each player over compatible hand pairs; the two must
/// cancel (zero-sum) up to solver precision.
fn weighted_values(s: &Sample) -> [f64; 2] {
    let hands = Hands::new(&s.spot.board);
    let mut out = [0.0f64; 2];
    let mut norm = 0.0f64;
    for p in 0..2 {
        let opp = &s.spot.weights[p ^ 1];
        let mut total = 0.0f64;
        let mut per_card = [0.0f64; 52];
        for (j, &(a, b)) in hands.cards.iter().enumerate() {
            total += opp[j] as f64;
            per_card[a as usize] += opp[j] as f64;
            per_card[b as usize] += opp[j] as f64;
        }
        for (i, &(a, b)) in hands.cards.iter().enumerate() {
            let w = s.spot.weights[p][i] as f64;
            if w == 0.0 {
                continue;
            }
            let m = total - per_card[a as usize] - per_card[b as usize] + opp[i] as f64;
            out[p] += w * m * s.values[p][i] as f64;
            if p == 0 {
                norm += w * m;
            }
        }
    }
    [out[0] / norm, out[1] / norm]
}

fn verify(path: &str) {
    let mut r = BufReader::new(std::fs::File::open(path).unwrap());
    let (mut n, mut worst, mut nan_in_range) = (0usize, 0.0f64, 0usize);
    let (mut secs, mut iters, mut expl, mut book) = (0.0f64, 0u64, 0.0f64, 0usize);
    while let Ok(s) = bincode::deserialize_from::<_, Sample>(&mut r) {
        let v = weighted_values(&s);
        worst = worst.max((v[0] + v[1]).abs());
        for p in 0..2 {
            for (i, &w) in s.spot.weights[p].iter().enumerate() {
                if w > 0.0 && !s.values[p][i].is_finite() {
                    nan_in_range += 1;
                }
            }
        }
        n += 1;
        secs += s.seconds as f64;
        iters += s.iters as u64;
        expl += s.exploitability as f64;
        book += (s.spot.source == 1) as usize;
    }
    println!(
        "{} records ({} from the book): worst zero-sum residual {:.2e} pot, {} missing in-range values, mean {:.1} s / {:.0} iters / expl {:.3}% pot",
        n, book, worst, nan_in_range, secs / n as f64, iters as f64 / n as f64, 100.0 * expl / n as f64
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |k: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone());
    if let Some(path) = get("--verify") {
        verify(&path);
        return;
    }
    let out = get("--out").expect("--out <dir>");
    let count: u64 = get("--count").map_or(1000, |s| s.parse().unwrap());
    let (shard, nshards): (u64, u64) = get("--shard").map_or((0, 1), |s| {
        let (a, b) = s.split_once('/').expect("--shard k/n");
        (a.parse().unwrap(), b.parse().unwrap())
    });
    let workers: usize = get("--workers").map_or(
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
        |s| s.parse().unwrap(),
    );
    let tree = get("--tree").unwrap_or_else(|| "live".to_string());
    let target: f32 = get("--target").map_or(0.005, |s| s.parse().unwrap());
    let book_share: f64 = get("--book-share").map_or(0.5, |s| s.parse().unwrap());
    let ends: Vec<FlopEnd> = match get("--book") {
        None => Vec::new(),
        Some(dir) => {
            let book = FlopBook::load(&dir);
            let mut ends = Vec::new();
            let mut names: Vec<&String> = book.lines.keys().collect();
            names.sort();
            for line in names {
                let lb = &book.lines[line];
                let mut flops: Vec<&String> = lb.solutions.keys().collect();
                flops.sort();
                for f in flops {
                    ends.extend(harvest_flop_ends(&lb.solutions[f], line));
                }
            }
            println!("[DATAGEN] {} flop ends harvested from the book", ends.len());
            ends
        }
    };

    std::fs::create_dir_all(&out).unwrap();
    let path = format!("{}/turn_{}.bin", out, shard);
    let done = count_records(&path);
    println!(
        "[DATAGEN] {}: {} records present, generating up to {} (shard {}/{}, {} workers, tree {}, target {}% pot, book share {})",
        path, done, count, shard, nshards, workers, tree, target * 100.0, if ends.is_empty() { 0.0 } else { book_share }
    );
    let writer = Mutex::new(BufWriter::new(OpenOptions::new().create(true).append(true).open(&path).unwrap()));
    let next = AtomicUsize::new(done as usize);
    let written = AtomicUsize::new(done as usize);
    let start = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
                loop {
                    let k = next.fetch_add(1, Ordering::SeqCst) as u64;
                    if k >= count {
                        break;
                    }
                    // Spot k of this shard is a pure function of (shard, k).
                    let index = k * nshards + shard;
                    let mut rng = StdRng::seed_from_u64(0x5eed_0000_0000 ^ index);
                    let spot = loop {
                        if !ends.is_empty() && rng.gen_bool(book_share) {
                            let fe = &ends[rng.gen_range(0..ends.len())];
                            if let Some(s) = turn_spot(fe, rng.gen_range(0..52)) {
                                break s;
                            }
                        } else {
                            break random_spot(4, &mut rng);
                        }
                    };
                    let t0 = Instant::now();
                    let res = pool.install(|| solve_spot(&spot, &tree, target));
                    let (values, iters, expl) = match res {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("[DATAGEN] spot {} failed: {}", index, e);
                            continue;
                        }
                    };
                    let sample = Sample {
                        index,
                        spot,
                        values,
                        iters,
                        exploitability: expl,
                        seconds: t0.elapsed().as_secs_f32(),
                        tree: tree.clone(),
                    };
                    {
                        let mut w = writer.lock().unwrap();
                        bincode::serialize_into(&mut *w, &sample).unwrap();
                        w.flush().unwrap();
                    }
                    let n = written.fetch_add(1, Ordering::SeqCst) + 1;
                    if n % 100 == 0 {
                        let el = start.elapsed().as_secs_f64();
                        let rate = (n as u64 - done) as f64 / el * 3600.0;
                        println!(
                            "[DATAGEN] {} / {} records, {:.0}/hour, ETA {:.1} h",
                            n, count, rate, (count - n as u64) as f64 / rate
                        );
                    }
                }
            });
        }
    });
    println!("[DATAGEN] done: {} records in {}", written.load(Ordering::SeqCst), path);
}
