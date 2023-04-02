use criterion::*;
use optimus::*;
use std::time::Duration;
use dashmap::DashMap;

fn bench_cfr(c: &mut Criterion) {
    let nodes: Nodes = DashMap::new();
    let mut group = c.benchmark_group("cfr");
    group.warm_up_time(Duration::new(90, 0));
    group.bench_function("cfr", |b| {
        b.iter(|| {
            cfr_iteration(
                &deck(),
                &ActionHistory::new(),
                &nodes,
                &CONFIG.bet_abstraction,
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

criterion_group!(
    name=benches;
    config=Criterion::default().configure_from_args();
    targets=bench_cfr, bench_isomorphic_hand
);
criterion_main!(benches);
