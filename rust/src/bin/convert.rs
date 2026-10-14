/// Converts training artifacts (nodes.bin, *_abstraction.bin) into the
/// compact read-only format used by the server (mmap-friendly flat files).
///
/// Usage:
///   cargo run --release --bin convert
///
/// Reads:
///   products/nodes.bin
///   products/flop_abstraction.bin, turn_abstraction.bin, river_abstraction.bin
///
/// Writes:
///   products/readonly_flop.bin, readonly_turn.bin, readonly_river.bin
///   products/readonly_nodes.idx
///   products/readonly_nodes.dat

use optimus::*;

fn main() {
    convert_abstractions();
    convert_nodes();
    println!("[CONVERT] Done!");
}

fn convert_abstractions() {
    println!("[CONVERT] Converting abstraction files...");
    for (in_path, out_path) in [
        (FLOP_ABSTRACTION_PATH, "products/readonly_flop.bin"),
        (TURN_ABSTRACTION_PATH, "products/readonly_turn.bin"),
        (RIVER_ABSTRACTION_PATH, "products/readonly_river.bin"),
    ] {
        println!("[CONVERT] Reading {}...", in_path);
        let map: ahash::AHashMap<u64, i32> = {
            let file = std::fs::File::open(in_path)
                .unwrap_or_else(|e| panic!("Failed to open {}: {}", in_path, e));
            let reader = std::io::BufReader::new(file);
            bincode::deserialize_from(reader)
                .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", in_path, e))
        };
        let mut pairs: Vec<(u64, i32)> = map.into_iter().collect();
        write_readonly_abstraction(&mut pairs, out_path);
    }
}

fn convert_nodes() {
    println!("[CONVERT] Loading nodes...");
    let nodes = load_nodes(&CONFIG.nodes_path);
    let num_histories = nodes.dashmap.len();
    println!("[CONVERT] Loaded {} histories, converting...", num_histories);

    let bar = pbar(num_histories);
    let mut entries: Vec<ConvertEntry> = Vec::with_capacity(num_histories);

    for entry in nodes.dashmap.iter() {
        let history: &ActionHistory = entry.key();
        let node_vec = entry.value();

        let key_hash = hash_history(history);
        let history_bytes = bincode::serialize(history).unwrap();
        let num_actions = node_vec.num_actions as u32;
        let nodes_lock = node_vec.nodes.lock().unwrap();
        let num_buckets = nodes_lock.len() as u32;

        // Extract pre-computed cumulative strategy for each bucket
        let mut strategy_data: Vec<f32> = Vec::with_capacity((num_buckets * num_actions) as usize);
        for node in nodes_lock.iter() {
            let cum_strategy = node.cumulative_strategy(num_actions as usize);
            for &prob in cum_strategy.iter() {
                strategy_data.push(prob as f32);
            }
        }

        entries.push(ConvertEntry {
            key_hash,
            history_bytes,
            num_buckets,
            num_actions,
            strategy_data,
        });

        bar.inc(1);
    }
    bar.finish();

    write_readonly_nodes(
        entries,
        "products/readonly_nodes.idx",
        "products/readonly_nodes.dat",
    );

    // Print some stats
    let idx_size = std::fs::metadata("products/readonly_nodes.idx")
        .map(|m| m.len())
        .unwrap_or(0);
    let dat_size = std::fs::metadata("products/readonly_nodes.dat")
        .map(|m| m.len())
        .unwrap_or(0);
    println!(
        "[CONVERT] Index: {:.1} MB, Data: {:.1} MB",
        idx_size as f64 / 1_000_000.0,
        dat_size as f64 / 1_000_000.0,
    );
}
