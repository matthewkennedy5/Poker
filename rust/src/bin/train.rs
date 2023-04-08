use optimus::*;

use std::mem::size_of_val;

fn check_t() {
    let nodes = load_nodes(&CONFIG.nodes_path);
    let t: Vec<f32> = nodes.iter().map(|entry| {
        entry.t
    }).collect();
    let mean = statistical::mean(&t);
    let median = statistical::median(&t);
    let std = statistical::standard_deviation(&t, Some(mean));
    println!("Mean: {mean}");
    println!("Median: {median}");
    println!("Std: {std}");
}

fn check_infoset_node_size() {
    let infoset = InfoSet::from_hand(
        &str2cards("AsAd"),
        &str2cards(""),
        &ActionHistory::new()
    );
    let node = Node::new(&infoset, &CONFIG.bet_abstraction);
    println!("InfoSet size: {}", size_of_val(&infoset));
    println!("Node size: {}", size_of_val(&node));
}

fn main() {
    // check_t();
    train(CONFIG.train_iters, CONFIG.warm_start);
    // check_infoset_node_size();
}
