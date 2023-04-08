use optimus::*;
use rand::Rng;

use std::mem::size_of_val;

fn check_t() {
    let nodes = load_nodes(&CONFIG.nodes_path);
    let mut rng = rand::thread_rng();
    let mut t: Vec<f32> = Vec::with_capacity(nodes.len() / 1000);
    for (_, node) in nodes {
        if rng.gen_bool(0.001) {
            t.push(node.t);
        }
    }
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
    check_t();
    // train(CONFIG.train_iters);
    // check_infoset_node_size();
}
