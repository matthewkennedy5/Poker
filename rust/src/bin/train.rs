use optimus::*;
use rand::Rng;

use std::mem::size_of_val;

// fn check_t() {
//     let nodes = load_nodes(&CONFIG.nodes_path);
//     let t: Vec<f32> = nodes.iter().map(|entry| {
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

fn main() {
    train(CONFIG.train_iters, CONFIG.warm_start);
}
