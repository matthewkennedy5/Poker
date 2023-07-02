use optimus::*;

fn main() {

    // get_hand_counts(5);
    // get_hand_counts(6);
    // get_hand_counts(7);

    train(CONFIG.train_iters, CONFIG.eval_every, CONFIG.warm_start);

    // let nodes = load_nodes(&CONFIG.nodes_path);
    // // Check what percent of nodes have t = 0
    // let mut zero = 0;
    // let mut total = 0;
    // let bar = pbar(nodes.dashmap.len() as u64);
    // for (history, history_nodes) in nodes.dashmap {
    //     let mut bucket = 0;
    //     for n in history_nodes {
    //         total += 1;
    //         let infoset = InfoSet { history: history.clone(), card_bucket: bucket };
    //         if n.t == 0 {
    //             println!("Zero: {infoset}");
    //             zero += 1;
    //         } else {
    //             println!("Nonzero: {}, t={}", infoset, n.t);
    //         }
    //         bucket += 1;
    //     }
    //     bar.inc(1);
    // }
    // bar.finish();
    // println!("Percent zeros: {}", zero as f64 / total as f64);
}



