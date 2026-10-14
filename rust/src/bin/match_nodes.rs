//! Blueprint-vs-blueprint match between two nodes files trained with the same
//! configuration (e.g. two checkpoints of one run). Duplicate deals, pure
//! strategy lookups, no search.
//!
//! Usage (from rust/, with the matching POKER_PARAMS):
//!   match-nodes --a products/checkpoints/nodes_v2_20000.bin --b products/nodes_v2.bin --hands 20000
use optimus::*;
use rand::prelude::*;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str, default: &str| -> String {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
            .unwrap_or_else(|| default.to_string())
    };
    let path_a = get("--a", "");
    let path_b = get("--b", "");
    let num_deals: usize = get("--hands", "10000").parse().expect("bad --hands");
    let purify: f64 = get("--purify", "0.05").parse().expect("bad --purify");
    if path_a.is_empty() || path_b.is_empty() {
        eprintln!("usage: match-nodes --a <nodes.bin> --b <nodes.bin> [--hands N]");
        std::process::exit(1);
    }
    println!("[MATCH] loading A: {}", path_a);
    let a = load_nodes(&path_a);
    println!("[MATCH] loading B: {}", path_b);
    let b = load_nodes(&path_b);
    println!("[MATCH] playing {} duplicate deals", num_deals);

    let deck = deck();
    let mut rng = thread_rng();
    let mut results: Vec<f64> = Vec::with_capacity(num_deals);
    let start = Instant::now();
    for deal in 0..num_deals {
        let mut cards = deck.clone();
        cards.shuffle(&mut rng);
        let hole_x = vec![cards[0], cards[1]];
        let hole_y = vec![cards[2], cards[3]];
        let board: Vec<Card> = cards[4..9].to_vec();
        let mut total = 0.0;
        for a_seat in [DEALER, OPPONENT] {
            let (dealer_hole, opp_hole) = if a_seat == DEALER {
                (&hole_x, &hole_y)
            } else {
                (&hole_y, &hole_x)
            };
            total += play_one(&a, &b, a_seat, dealer_hole, opp_hole, &board, purify);
        }
        results.push(total / 2.0);
        if (deal + 1) % 1000 == 0 || deal + 1 == num_deals {
            let n = results.len() as f64;
            let mean = results.iter().sum::<f64>() / n;
            let var = results.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
            let se = (var / n).sqrt();
            println!(
                "[MATCH] {} deals: A {:+.0} ± {:.0} mbb/hand ({:.0} s)",
                results.len(),
                mean / CONFIG.big_blind as f64 * 1000.0,
                1.96 * se / CONFIG.big_blind as f64 * 1000.0,
                start.elapsed().as_secs_f64()
            );
        }
    }
}

fn pick(nodes: &Nodes, hole: &[Card], board: &[Card], history: &ActionHistory, purify_t: f64) -> Action {
    let mut strategy = nodes.get_strategy(hole, board, history);
    purify(&mut strategy, purify_t);
    sample_action_from_strategy(&strategy)
}

fn play_one(
    a: &Nodes,
    b: &Nodes,
    a_seat: usize,
    dealer_hole: &[Card],
    opp_hole: &[Card],
    board: &[Card],
    purify_t: f64,
) -> f64 {
    let mut history = ActionHistory::new();
    while !history.hand_over() {
        let player = history.player;
        let hole = if player == DEALER { dealer_hole } else { opp_hole };
        let visible = &board[..board_length(history.street).min(board.len())];
        let nodes = if player == a_seat { a } else { b };
        let action = pick(nodes, hole, visible, &history, purify_t);
        history.add(&action);
    }
    server_terminal_utility(
        if a_seat == DEALER { dealer_hole } else { opp_hole },
        if a_seat == DEALER { opp_hole } else { dealer_hole },
        board,
        &history,
        a_seat,
    )
}
