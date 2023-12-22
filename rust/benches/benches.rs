use criterion::*;
use optimus::*;
use rand::prelude::*;
use std::time::Duration;

fn bench_cfr(c: &mut Criterion) {
    let nodes: Nodes = Nodes::new(&CONFIG.bet_abstraction);
    let mut group = c.benchmark_group("cfr");
    group.warm_up_time(Duration::new(60, 0));
    // group.sample_size(10);
    group.bench_function("cfr", |b| {
        b.iter(|| cfr_iteration(&deck(), &ActionHistory::new(), &nodes, -1))
    });
    group.finish();
}

fn bench_isomorphic_hand(c: &mut Criterion) {
    let mut deck = deck();
    deck.shuffle(&mut rand::thread_rng());
    let cards = &deck[..7];
    c.bench_function("isomorphic", |b| {
        b.iter(|| isomorphic_hand(&cards));
    });
}

fn bench_play_hand(c: &mut Criterion) {
    let blueprint = load_nodes(&CONFIG.nodes_path);
    let get_strategy = |hole: &[Card], board: &[Card], history: &ActionHistory| {
        blueprint.get_strategy(hole, board, history)
    };
    let mut group = c.benchmark_group("play_hand");
    group.warm_up_time(Duration::new(90, 0));
    group.bench_function("play_hand", |b| b.iter(|| play_hand(&get_strategy)));
    group.finish();
}

fn bench_terminal_utility_vectorized(c: &mut Criterion) {
    let mut deck = deck();
    deck.shuffle(&mut rand::thread_rng());
    let board = [deck[0], deck[1], deck[2], deck[3], deck[4]];
    let preflop_hands = non_blocking_preflop_hands(&board);
    let opp_reach_probs = vec![1.0; preflop_hands.len()];
    let history_fold =
        ActionHistory::from_strings(vec!["Bet 300", "Call 300", "Bet 300", "Fold 0"]);
    let history_showdown =
        ActionHistory::from_strings(vec!["Bet 300", "Call 300", "Bet 300", "Call 300"]);

    let mut group = c.benchmark_group("terminal_utility");
    group.bench_function("terminal_utility_fold", |b| {
        b.iter(|| {
            terminal_utility_vectorized(
                preflop_hands.clone(),
                opp_reach_probs.clone(),
                &board,
                &history_fold,
                0,
            )
        })
    });
    group.bench_function("terminal_utility_showdown", |b| {
        b.iter(|| {
            terminal_utility_vectorized(
                preflop_hands.clone(),
                opp_reach_probs.clone(),
                &board,
                &history_showdown,
                0,
            )
        })
    });
    group.finish();
}

criterion_group!(
    name=benches;
    config=Criterion::default().configure_from_args();
    targets=bench_cfr, bench_isomorphic_hand, bench_play_hand, bench_terminal_utility_vectorized
);
criterion_main!(benches);
