use optimus::*;

use std::mem::size_of_val;
use std::fs;

// fn check_t() {
//     let nodes = load_nodes(&CONFIG.nodes_path);
//     let t: Vec<f64> = nodes.iter().map(|entry| {
//         entry.t
//     }).collect();
//     let mean = statistical::mean(&t);
//     let median = statistical::median(&t);
//     let std = statistical::standard_deviation(&t, Some(mean));
//     println!("Mean: {mean}");
//     println!("Median: {median}");
//     println!("Std: {std}");
// }

fn check_infoset_node_size() {
    let opening_history = ActionHistory::new();
    let infoset = InfoSet::from_hand(
        &str2cards("AsAd"),
        &str2cards(""),
        &opening_history
    );
    let node = Node::new(opening_history.next_actions(&CONFIG.bet_abstraction).len());
    println!("InfoSet size: {}", size_of_val(&infoset));
    println!("Node size: {}", size_of_val(&node));
}

fn check_floating_stability() {
    let nodes = load_nodes(&CONFIG.nodes_path);
    let o27 = InfoSet::from_hand(
        &str2cards("AcAh"),
        &str2cards("7c6dTcAd7d"),
        &ActionHistory::from_strings(vec![
            "Bet 200",
            "Call 200",
            "Call 0",
            "Bet 400",
            "Call 400",
            "Call 0",
            "Bet 1200",
            "Call 1200"
        ]),
    );
    println!("InfoSet: {o27}");
    println!("Actions: {:?}", o27.next_actions(&CONFIG.bet_abstraction));
    println!("Node: {:?}", nodes.get(&o27));
}

fn t_json() {
    let nodes = load_nodes(&CONFIG.nodes_path);
    let mut ts: Vec<u64> = Vec::new();
    let mut zero = 0;
    for (history, history_nodes) in nodes.dashmap {
        for n in history_nodes {
            ts.push(n.t);
            if n.t == 0 {
                zero += 1;
            }
        }
    }
    println!("Percent zeros: {}", (zero as f64) / (ts.len() as f64));

    // ts.sort();
    // let json = serde_json::to_string(&ts).unwrap();
    // fs::write("products/t_histogram.json", json).unwrap();
}

fn main() {
    // t_json();
    train(CONFIG.train_iters, CONFIG.warm_start);
    // check_floating_stability();
}



