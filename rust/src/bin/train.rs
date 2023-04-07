use optimus::*;

use std::mem::size_of_val;

fn main() {
    train(CONFIG.train_iters);

    let infoset = InfoSet::from_hand(
        &str2cards("AsAd"),
        &str2cards(""),
        &ActionHistory::new()
    );
    let node = Node::new(&infoset, &CONFIG.bet_abstraction);
    println!("InfoSet size: {}", size_of_val(&infoset));
    println!("Node size: {}", size_of_val(&node));
}
