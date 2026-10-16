//! Benchmarks the cost of turn-net training data: random turn spots solved to the
//! end of the hand by postflop-solver, one single-threaded solve per worker (the
//! way data generation runs), for a few candidate trees.
//!
//! Usage: datagen-bench <tree: live|small|rich> [spots] [workers] [seed] [book dir]
//! With a book dir, spots are flop ends harvested from the book plus a random
//! turn card instead of random spots.
//! Prints per-spot iterations/time to 1% and 0.5% of the pot, memory, and the
//! implied throughput.
use optimus::spots::{harvest_flop_ends, random_spot, turn_spot};
use optimus::{cap_raises, FlopBook, Hands};
use postflop_solver::{
    compute_exploitability, solve_step, ActionTree, BetSizeOptions, BoardState, CardConfig,
    PostFlopGame, Range, TreeConfig, NOT_DEALT,
};
use rand::prelude::*;
use rand::rngs::StdRng;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

fn sizes(b: &str, r: &str) -> BetSizeOptions {
    BetSizeOptions::try_from((b, r)).unwrap()
}

struct Res {
    t1: Option<f64>,
    i1: Option<u32>,
    t05: Option<f64>,
    i05: Option<u32>,
    mem_mb: f64,
    spr: f64,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let tree_name = args.first().cloned().unwrap_or_else(|| "live".to_string());
    let spots: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(16);
    let workers: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(4);
    let seed: u64 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(7);
    let (turn, river) = match tree_name.as_str() {
        "live" => (sizes("50%, 100%", "2.5x"), sizes("50%, 100%, a", "2.5x")),
        "small" => (sizes("66%", "2.5x"), sizes("66%, a", "2.5x")),
        "rich" => (sizes("33%, 75%, 125%", "2.5x"), sizes("33%, 75%, 125%, a", "2.5x")),
        t => panic!("unknown tree {}", t),
    };

    let book_dir = args.get(4).cloned();
    let mut rng = StdRng::seed_from_u64(seed);
    let all: Vec<_> = match &book_dir {
        None => (0..spots).map(|_| random_spot(4, &mut rng)).collect(),
        Some(dir) => {
            let book = FlopBook::load(dir);
            let mut ends = Vec::new();
            for (line, lb) in &book.lines {
                for sol in lb.solutions.values() {
                    ends.extend(harvest_flop_ends(sol, line));
                }
            }
            let mut sprs: Vec<f64> = ends.iter().map(|e| e.stack as f64 / e.pot as f64).collect();
            sprs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let q = |f: f64| sprs[((sprs.len() - 1) as f64 * f) as usize];
            println!(
                "harvested {} flop ends from {} lines; turn SPR quartiles {:.1} / {:.1} / {:.1}, max {:.1}",
                ends.len(), book.lines.len(), q(0.25), q(0.5), q(0.75), q(1.0)
            );
            let mut out = Vec::new();
            while out.len() < spots {
                let fe = ends.choose(&mut rng).unwrap();
                if let Some(s) = turn_spot(fe, rng.gen_range(0..52)) {
                    out.push(s);
                }
            }
            out
        }
    };
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Res>> = Mutex::new(Vec::new());
    let wall = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
                loop {
                    let k = next.fetch_add(1, Ordering::SeqCst);
                    if k >= all.len() {
                        break;
                    }
                    let spot = &all[k];
                    let hands = Hands::new(&spot.board);
                    let ranges: Vec<Range> = (0..2)
                        .map(|p| Range::from_hands_weights(&hands.cards, &spot.weights[p]).unwrap())
                        .collect();
                    let tc = TreeConfig {
                        initial_state: BoardState::Turn,
                        starting_pot: spot.pot,
                        effective_stack: spot.stack,
                        rake_rate: 0.0,
                        rake_cap: 0.0,
                        flop_bet_sizes: [sizes("", ""), sizes("", "")],
                        turn_bet_sizes: [turn.clone(), turn.clone()],
                        river_bet_sizes: [river.clone(), river.clone()],
                        turn_donk_sizes: None,
                        river_donk_sizes: None,
                        add_allin_threshold: 1.5,
                        force_allin_threshold: 0.15,
                        merging_threshold: 0.1,
                    };
                    let mut tree = ActionTree::new(tc).unwrap();
                    cap_raises(&mut tree, [2, 2], false, 0).unwrap();
                    tree.back_to_root();
                    let cc = CardConfig {
                        range: [ranges[0], ranges[1]],
                        flop: [spot.board[0], spot.board[1], spot.board[2]],
                        turn: spot.board[3],
                        river: NOT_DEALT,
                    };
                    let res = pool.install(|| {
                        let mut game = PostFlopGame::with_config(cc, tree).unwrap();
                        let (mem, _) = game.memory_usage();
                        game.allocate_memory(false);
                        let start = Instant::now();
                        let pot = spot.pot as f32;
                        let mut r = Res {
                            t1: None,
                            i1: None,
                            t05: None,
                            i05: None,
                            mem_mb: mem as f64 / 1e6,
                            spr: spot.stack as f64 / spot.pot as f64,
                        };
                        for t in 0..2000u32 {
                            solve_step(&game, t);
                            if (t + 1) % 10 == 0 {
                                let e = compute_exploitability(&game);
                                let el = start.elapsed().as_secs_f64();
                                if r.t1.is_none() && e <= 0.01 * pot {
                                    r.t1 = Some(el);
                                    r.i1 = Some(t + 1);
                                }
                                if e <= 0.005 * pot {
                                    r.t05 = Some(el);
                                    r.i05 = Some(t + 1);
                                    break;
                                }
                                if el > 300.0 {
                                    break;
                                }
                            }
                        }
                        r
                    });
                    println!(
                        "spot {:3} spr {:6.2} mem {:7.1} MB | 1%: {:>5} it {:>7.1} s | 0.5%: {:>5} it {:>7.1} s",
                        k,
                        res.spr,
                        res.mem_mb,
                        res.i1.map_or("-".into(), |x| x.to_string()),
                        res.t1.unwrap_or(f64::NAN),
                        res.i05.map_or("-".into(), |x| x.to_string()),
                        res.t05.unwrap_or(f64::NAN),
                    );
                    results.lock().unwrap().push(res);
                }
            });
        }
    });
    let wall = wall.elapsed().as_secs_f64();
    let res = results.into_inner().unwrap();
    let stat = |f: &dyn Fn(&Res) -> Option<f64>| {
        let mut v: Vec<f64> = res.iter().filter_map(|r| f(r)).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean = v.iter().sum::<f64>() / v.len().max(1) as f64;
        (v.len(), mean, v.get(v.len() / 2).cloned().unwrap_or(f64::NAN), v.last().cloned().unwrap_or(f64::NAN))
    };
    let (n1, m1, med1, max1) = stat(&|r| r.t1);
    let (n05, m05, med05, max05) = stat(&|r| r.t05);
    let (_, mm, _, mmax) = stat(&|r| Some(r.mem_mb));
    println!("\ntree {} | {} spots, {} workers, wall {:.0} s", tree_name, res.len(), workers, wall);
    println!("  to 1% pot:   {}/{} reached, mean {:.1} s, median {:.1} s, max {:.1} s (single thread)", n1, res.len(), m1, med1, max1);
    println!("  to 0.5% pot: {}/{} reached, mean {:.1} s, median {:.1} s, max {:.1} s (single thread)", n05, res.len(), m05, med05, max05);
    println!("  memory: mean {:.0} MB, max {:.0} MB per solve", mm, mmax);
    println!(
        "  throughput here: {:.0} spots/hour ({:.1} core-seconds per spot at 0.5%)",
        res.len() as f64 / wall * 3600.0,
        wall * workers as f64 / res.len() as f64
    );
}
