//! Local match: search bot (postflop-solver) vs the blueprint bot.
//!
//! Each deal is played twice with seats swapped (duplicate hands) to cut variance.
//!
//! Usage (from rust/):
//!   cargo run --release --bin play-match -- [--hands 200] [--fast] [--quiet]
use optimus::*;
use rand::prelude::*;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str, default: &str| -> String {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
            .unwrap_or_else(|| default.to_string())
    };
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let num_deals: usize = get("--hands", "200").parse().expect("bad --hands");
    let quiet = has("--quiet");

    let mut cfg = CONFIG.search.clone();
    cfg.enabled = true;
    cfg.log = !quiet;
    if has("--fast") {
        cfg.flop_time_ms = 500;
        cfg.turn_time_ms = 300;
        cfg.river_time_ms = 200;
    }
    init_thread_pool();

    let blueprint = Arc::new(load_blueprint_bot());
    let search = SearchBot::new(blueprint.clone(), cfg);
    let deck = deck();
    let mut rng = thread_rng();

    let mut results: Vec<f64> = Vec::with_capacity(num_deals); // search bot's bb per deal
    let start = Instant::now();
    for deal in 0..num_deals {
        let mut cards = deck.clone();
        cards.shuffle(&mut rng);
        let hole_a = vec![cards[0], cards[1]];
        let hole_b = vec![cards[2], cards[3]];
        let board: Vec<Card> = cards[4..9].to_vec();

        // Seat 0: search bot is DEALER; seat 1: search bot is OPPONENT.
        let mut deal_total = 0.0;
        for search_seat in [DEALER, OPPONENT] {
            let (dealer_hole, opp_hole) = if search_seat == DEALER {
                (&hole_a, &hole_b)
            } else {
                (&hole_b, &hole_a)
            };
            let winnings = play_one(
                &search,
                &blueprint,
                search_seat,
                dealer_hole,
                opp_hole,
                &board,
                quiet,
            );
            deal_total += winnings;
        }
        let per_deal = deal_total / 2.0 / CONFIG.big_blind as f64;
        results.push(per_deal);

        if (deal + 1) % 10 == 0 || deal + 1 == num_deals {
            let n = results.len() as f64;
            let mean = results.iter().sum::<f64>() / n;
            let var = results.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
            let se = (var / n).sqrt();
            println!(
                "[MATCH] {} deals: search bot {:+.1} ± {:.1} bb/100 ({:.1} s elapsed)",
                results.len(),
                100.0 * mean,
                100.0 * 1.96 * se,
                start.elapsed().as_secs_f64()
            );
        }
    }
}

/// Plays one hand; returns the search bot's winnings in chips.
fn play_one(
    search: &SearchBot,
    blueprint: &ReadOnlyBot,
    search_seat: usize,
    dealer_hole: &[Card],
    opp_hole: &[Card],
    board: &[Card],
    quiet: bool,
) -> f64 {
    let mut history = ActionHistory::new();
    while !history.hand_over() {
        let player = history.player;
        let hole = if player == DEALER { dealer_hole } else { opp_hole };
        let visible = &board[..board_length(history.street).min(board.len())];
        let action = if player == search_seat {
            search.get_action(hole, visible, &history)
        } else {
            blueprint.get_action(hole, visible, &history)
        };
        if !history.is_legal_next_action(&action) {
            println!(
                "[MATCH] illegal action {} from {} at {}; folding instead",
                action,
                if player == search_seat { "search" } else { "blueprint" },
                history
            );
            let fallback = if history.to_call() == 0 {
                Action { action: ActionType::Call, amount: 0 }
            } else {
                FOLD
            };
            history.add(&fallback);
        } else {
            history.add(&action);
        }
    }
    let winnings = server_terminal_utility(
        if search_seat == DEALER { dealer_hole } else { opp_hole },
        if search_seat == DEALER { opp_hole } else { dealer_hole },
        board,
        &history,
        search_seat,
    );
    if !quiet {
        println!(
            "[MATCH] search as {} with {} vs {} on {}: {} -> {:+.0}",
            if search_seat == DEALER { "SB" } else { "BB" },
            cards2str(if search_seat == DEALER { dealer_hole } else { opp_hole }),
            cards2str(if search_seat == DEALER { opp_hole } else { dealer_hole }),
            cards2str(board),
            history,
            winnings
        );
    }
    winnings
}
