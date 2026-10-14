//! Benchmarks a single street solve with the search configuration in params.toml.
//!
//! Usage (from the rust/ directory):
//!   cargo run --release --bin bench-search -- [--board Td9d6h] [--preflop "Bet 250,Call 250"]
//!       [--line "Call 0,Bet 300"] [--budget-ms 3000] [--uniform] [--no-compression]
//!
//! `--board` with 3/4/5 cards selects the flop/turn/river. `--preflop` is the real
//! preflop line (project actions); `--line` is the real line on the benchmarked
//! street, which is added to the tree if off-tree. `--uniform` skips the blueprint
//! and uses full ranges (no product files needed).
use optimus::*;
use postflop_solver::{CardConfig, PostFlopGame, Range as SolverRange, NOT_DEALT};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str, default: &str| -> String {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
            .unwrap_or_else(|| default.to_string())
    };
    let has = |flag: &str| args.iter().any(|a| a == flag);

    let board_str = get("--board", "Td9d6h");
    let preflop_str = get("--preflop", "Bet 250,Call 250");
    let line_str = get("--line", "");
    let budget_ms: u64 = get("--budget-ms", "0").parse().expect("bad --budget-ms");
    let uniform = has("--uniform");
    let mut cfg = CONFIG.search.clone();
    if has("--no-compression") {
        cfg.compression = false;
    }
    if has("--max-raises") {
        cfg.max_raises_per_street = get("--max-raises", "2").parse().expect("bad --max-raises");
    }
    for (flag, field) in [
        ("--flop-bets", 0), ("--flop-raises", 1), ("--turn-bets", 2),
        ("--turn-raises", 3), ("--river-bets", 4), ("--river-raises", 5),
    ] {
        if has(flag) {
            let v = get(flag, "");
            match field {
                0 => cfg.flop_bet_sizes = v,
                1 => cfg.flop_raise_sizes = v,
                2 => cfg.turn_bet_sizes = v,
                3 => cfg.turn_raise_sizes = v,
                4 => cfg.river_bet_sizes = v,
                _ => cfg.river_raise_sizes = v,
            }
        }
    }
    if has("--prune") {
        cfg.range_prune_threshold = get("--prune", "0").parse().expect("bad --prune");
    }
    if has("--max-memory-mb") {
        cfg.max_memory_mb = get("--max-memory-mb", "8000").parse().expect("bad --max-memory-mb");
    }
    if has("--threads") {
        cfg.threads = get("--threads", "0").parse().expect("bad --threads");
    }
    init_thread_pool_with(cfg.threads);

    let board = str2cards(&board_str);
    let street = match board.len() {
        3 => FLOP,
        4 => TURN,
        5 => RIVER,
        n => panic!("board must have 3, 4 or 5 cards, got {}", n),
    };
    let mut actions: Vec<&str> = preflop_str.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    // Pad with checks to reach the benchmarked street.
    let mut history = ActionHistory::from_strings(actions.clone());
    while history.street < street {
        history.add(&Action { action: ActionType::Call, amount: 0 });
        actions.push("Call 0");
    }
    if !line_str.is_empty() {
        for a in line_str.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let tokens: Vec<&str> = a.split(' ').collect();
            let action = Action {
                action: match tokens[0] {
                    "Bet" => ActionType::Bet,
                    "Call" => ActionType::Call,
                    "Fold" => ActionType::Fold,
                    _ => panic!("bad action"),
                },
                amount: tokens[1].parse().unwrap(),
            };
            history.add(&action);
        }
    }
    assert_eq!(history.street, street, "line must stay on the benchmarked street");

    let slices = split_by_street(&history);
    let slice = &slices[street];
    let eff = slice.stacks_start[0].min(slice.stacks_start[1]) as i32;
    let pot = slice.pot_start as i32;
    let line = to_solver_line(slice, eff).unwrap();
    println!(
        "Street {} board {} pot {} eff {} line {:?}",
        street, board_str, pot, eff, line.actions
    );
    println!(
        "Sizes: flop [{} | {}], turn [{} | {}], river [{} | {}], compression {}",
        cfg.flop_bet_sizes, cfg.flop_raise_sizes, cfg.turn_bet_sizes, cfg.turn_raise_sizes,
        cfg.river_bet_sizes, cfg.river_raise_sizes, cfg.compression
    );

    let our_hole = solver_hole(&str2cards("AsKs"));
    let ranges: [SolverRange; 2] = if uniform {
        [SolverRange::ones(), SolverRange::ones()]
    } else {
        let blueprint = load_blueprint_bot();
        let earlier: Vec<Action> = slices[..street]
            .iter()
            .flat_map(|sl| sl.actions.iter().cloned())
            .collect();
        blueprint_entry_ranges(
            blueprint.nodes(),
            &board,
            &earlier,
            history.starting_stacks(),
            street,
            our_hole,
            cfg.own_hand_floor as f32,
            cfg.range_prune_threshold as f32,
            history.player,
        )
        .unwrap()
    };

    let board_ids: Vec<u8> = board.iter().map(to_solver_card).collect();
    let tree = build_tree(street, pot, eff, &cfg, &line.actions).unwrap();
    let card_config = CardConfig {
        range: ranges,
        flop: [board_ids[0], board_ids[1], board_ids[2]],
        turn: if street >= TURN { board_ids[3] } else { NOT_DEALT },
        river: if street >= RIVER { board_ids[4] } else { NOT_DEALT },
    };
    let mut game = PostFlopGame::with_config(card_config, tree).unwrap();
    let (mem_f32, mem_c) = game.memory_usage();
    println!(
        "Memory: {:.1} MB (f32) / {:.1} MB (compressed); OOP hands {}, IP hands {}",
        mem_f32 as f64 / 1e6,
        mem_c as f64 / 1e6,
        game.private_cards(0).len(),
        game.private_cards(1).len()
    );
    let needed = if cfg.compression { mem_c } else { mem_f32 };
    if needed > cfg.max_memory_mb * 1024 * 1024 {
        println!("Refusing to solve: exceeds max_memory_mb = {}", cfg.max_memory_mb);
        return;
    }

    let budget = Duration::from_millis(if budget_ms > 0 {
        budget_ms
    } else {
        match street {
            FLOP => cfg.flop_time_ms,
            TURN => cfg.turn_time_ms,
            _ => cfg.river_time_ms,
        }
    });
    let stats = solve_with_budget(
        &mut game,
        budget,
        cfg.target_exploitability as f32,
        cfg.max_iters,
        cfg.compression,
    );
    println!(
        "Solved: {} iterations in {:.2} s ({:.1} ms/iter), exploitability {:.3}% of pot (target {:.3}%)",
        stats.iters,
        stats.elapsed.as_secs_f64(),
        stats.elapsed.as_secs_f64() * 1000.0 / stats.iters.max(1) as f64,
        100.0 * stats.exploitability / pot as f32,
        100.0 * cfg.target_exploitability
    );

    // Show the root strategy summary for the first player to act.
    let indices = line_indices(&mut game, &line.actions).unwrap();
    game.apply_history(&indices);
    if !game.is_terminal_node() && !game.is_chance_node() {
        game.cache_normalized_weights();
        let p = game.current_player();
        let actions = game.available_actions();
        let strat = game.strategy();
        let n = game.private_cards(p).len();
        let weights = game.normalized_weights(p);
        let total: f32 = weights.iter().sum();
        println!("Player {} to act; aggregate strategy:", p);
        for (ai, a) in actions.iter().enumerate() {
            let freq: f32 = (0..n).map(|h| strat[ai * n + h] * weights[h]).sum::<f32>() / total;
            println!("  {:?}: {:.1}%", a, 100.0 * freq);
        }
    }
}

fn init_thread_pool_with(threads: usize) {
    if threads > 0 {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global();
    }
}
