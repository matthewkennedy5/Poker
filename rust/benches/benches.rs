use criterion::*;
use optimus::*;
use std::time::Duration;

// TODO: add a bench for the realtime solving time

fn bench_cfr(c: &mut Criterion) {
    let nodes: Nodes = Nodes::new(&CONFIG.bet_abstraction);
    let dummy_depth_limit_hack = Bot::new(Nodes::new(&CONFIG.bet_abstraction), false, false, -1); // TODO REFACTOR
    let mut group = c.benchmark_group("cfr");
    group.warm_up_time(Duration::new(150, 0));
    group.bench_function("cfr", |b| {
        b.iter(|| {
            cfr_iteration(
                &deck(),
                &ActionHistory::new(),
                &nodes,
                &dummy_depth_limit_hack,
                -1
            )
        })
    });
    group.finish();
}

fn bench_isomorphic_hand(c: &mut Criterion) {
    let cards = str2cards("As7h4c8d8c9dTh");
    c.bench_function("isomorphic", |b| {
        b.iter(|| {
            isomorphic_hand(&cards, true)
        })
    });
}

fn bench_win_probability_rollout(c: &mut Criterion) {
    let mut opp_range = Range::new();
    let exploiter_hole = str2cards("8dTd");
    let board = str2cards("Ad7c8c");
    opp_range.remove_blockers(&exploiter_hole);
    opp_range.remove_blockers(&board);
    let mut group = c.benchmark_group("win_probability_rollout");
    group.warm_up_time(Duration::new(90, 0));
    group.bench_function("win_probability_rollout", |b| {
        b.iter(|| {
            win_probability_rollout(&opp_range, &exploiter_hole, &board)
        })
    });
    group.finish();
}

fn bench_play_hand(c: &mut Criterion) {
    let blueprint = load_nodes(&CONFIG.nodes_path);
    let get_strategy = |hole: &[Card], board: &[Card], history: &ActionHistory| {
        blueprint.get_strategy(hole, board, history)
    };
    let mut group = c.benchmark_group("play_hand");
    group.warm_up_time(Duration::new(90, 0));
    group.bench_function("play_hand", |b| {
        b.iter(|| {
            play_hand(&get_strategy)
        })
    });
    group.finish();
}

criterion_group!(
    name=benches;
    config=Criterion::default().configure_from_args();
    targets=bench_cfr, bench_isomorphic_hand, bench_win_probability_rollout, bench_play_hand
);
criterion_main!(benches);
