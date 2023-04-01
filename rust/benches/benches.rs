use criterion::*;
use optimus::*;
use std::{collections::HashMap, time::Duration};

fn bench_cfr(c: &mut Criterion) {
    let mut nodes: HashMap<InfoSet, Node> = HashMap::new();
    let mut group = c.benchmark_group("cfr");
    group.warm_up_time(Duration::new(30, 0));
    group.bench_function("cfr", |b| {
        b.iter(|| {
            cfr_iteration(
                &deck(),
                &ActionHistory::new(),
                &mut nodes,
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
