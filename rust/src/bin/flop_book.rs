//! Flop book builder.
//!
//!   flop-book lines --log results/x.jsonl [--log results/y.jsonl ...] [--top 12]
//!       Prints the abstract preflop lines that reached the flop, by frequency.
//!   flop-book build --lines lines.txt [--dir products/flopbook] [--budget-s 300]
//!                   [--target 0.003] [--max-iters 20000]
//!       Solves every canonical flop for every line in the file (one line per row,
//!       '#' comments allowed), resuming where it left off. [--shard 0,1/3] builds only
//!       the flops whose index % 3 is 0 or 1 (split a line across boxes; copy the files
//!       into one dir under different names and the loader merges them).
//!   flop-book stats [--dir products/flopbook]
use optimus::*;
use std::collections::HashMap;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let get = |flag: &str, default: &str| -> String {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
            .unwrap_or_else(|| default.to_string())
    };
    let get_all = |flag: &str| -> Vec<String> {
        args.iter()
            .enumerate()
            .filter(|(_, a)| a.as_str() == flag)
            .filter_map(|(i, _)| args.get(i + 1).cloned())
            .collect()
    };
    match cmd {
        "lines" => {
            let top: usize = get("--top", "15").parse().unwrap();
            lines_from_logs(&get_all("--log"), top);
        }
        "build" => {
            init_thread_pool();
            let dir = get("--dir", "products/flopbook");
            let budget = Duration::from_secs_f64(get("--budget-s", "300").parse().unwrap());
            let target: f32 = get("--target", "0.003").parse().unwrap();
            let max_iters: u32 = get("--max-iters", "20000").parse().unwrap();
            let max_flops: usize = get("--max-flops", "100000").parse().unwrap();
            let shard = match args.iter().any(|a| a == "--shard") {
                true => Shard::parse(&get("--shard", "")).unwrap_or_else(|e| panic!("{}", e)),
                false => Shard::all(),
            };
            let lines_file = get("--lines", "");
            let lines: Vec<String> = std::fs::read_to_string(&lines_file)
                .expect("could not read --lines file")
                .lines()
                .map(|l| l.split('#').next().unwrap().trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            let blueprint = load_blueprint_bot();
            let mut cfg = CONFIG.search.clone();
            if args.iter().any(|a| a == "--prune") {
                cfg.range_prune_threshold = get("--prune", "0").parse().unwrap();
            }
            println!("[BOOK] tree: {}", tree_description(&cfg));
            for line in &lines {
                match build_line(blueprint.nodes(), &cfg, &dir, line, budget, target, max_iters, true, max_flops, &shard) {
                    Ok(n) => println!("[BOOK] line '{}' done ({} new flops)", line, n),
                    Err(e) => println!("[BOOK] line '{}' FAILED: {}", line, e),
                }
            }
        }
        "stats" => {
            let dir = get("--dir", "products/flopbook");
            let book = FlopBook::load(&dir);
            for (line, lb) in &book.lines {
                let n = lb.solutions.len();
                let avg_expl: f32 = lb.solutions.values().map(|s| s.exploitability / s.pot as f32).sum::<f32>() / n.max(1) as f32;
                let avg_iters: f32 = lb.solutions.values().map(|s| s.iters as f32).sum::<f32>() / n.max(1) as f32;
                println!(
                    "{:40} pot {:5} flops {:4}/1755 avg iters {:.0} avg expl {:.2}% pot",
                    line, lb.header.pot, n, avg_iters, 100.0 * avg_expl
                );
            }
        }
        _ => {
            eprintln!("usage: flop-book lines|build|stats ... (see source)");
            std::process::exit(1);
        }
    }
}

/// Parses the preflop part of a Slumbot action string ("b200c", "b250b775c") into
/// project actions, mirroring play_against_slumbot.py's TranslateAction.
fn slumbot_preflop_to_actions(action: &str) -> Option<Vec<Action>> {
    let pre = action.split('/').next()?;
    let mut out = Vec::new();
    let mut street_bets = [0i64, 0i64];
    let mut player = 1usize; // Slumbot's preflop first actor index in its own numbering
    let mut i = 0;
    let bytes = pre.as_bytes();
    while i < bytes.len() {
        let c = bytes[i] as char;
        let (action, amount): (ActionType, i64) = match c {
            'b' => {
                let mut j = i + 1;
                while j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
                    j += 1;
                }
                let total: i64 = pre[i + 1..j].parse().ok()?;
                i = j;
                (ActionType::Bet, total - street_bets[player])
            }
            'c' => {
                i += 1;
                let to_call = if out.is_empty() { 100 } else { street_bets[1 - player] - street_bets[player] };
                (ActionType::Call, to_call)
            }
            'k' => {
                i += 1;
                let to_call = if out.len() == 1 { 100 } else { 0 };
                (ActionType::Call, to_call)
            }
            'f' => {
                i += 1;
                (ActionType::Fold, 0)
            }
            _ => return None,
        };
        out.push(Action { action, amount: amount as Amount });
        street_bets[player] += amount;
        player = 1 - player;
    }
    Some(out)
}

fn lines_from_logs(logs: &[String], top: usize) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total = 0usize;
    for log in logs {
        let text = std::fs::read_to_string(log).expect("could not read log");
        for l in text.lines() {
            let v: serde_json::Value = match serde_json::from_str(l) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let action = match v.get("action").and_then(|a| a.as_str()) {
                Some(a) => a,
                None => continue,
            };
            let board_len = v.get("board").and_then(|b| b.as_array()).map(|b| b.len()).unwrap_or(0);
            if board_len < 3 {
                continue;
            }
            let actions = match slumbot_preflop_to_actions(action) {
                Some(a) => a,
                None => continue,
            };
            let mut hist = ActionHistory::new();
            let mut ok = true;
            for a in &actions {
                if !hist.is_legal_next_action(a) {
                    ok = false;
                    break;
                }
                hist.add(a);
            }
            if !ok || hist.street != FLOP {
                continue;
            }
            let key = abstract_preflop_line(&hist);
            *counts.entry(key).or_insert(0) += 1;
            total += 1;
        }
    }
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("# {} hands reached the flop; abstract preflop lines by frequency", total);
    let mut cum = 0.0;
    for (line, n) in v.iter().take(top) {
        cum += *n as f64 / total as f64;
        println!("{:45} # {:6} hands {:5.1}%  cumulative {:5.1}%", line, n, 100.0 * *n as f64 / total as f64, 100.0 * cum);
    }
}
