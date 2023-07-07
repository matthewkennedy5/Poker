use optimus::*;

fn main() {
    // train(CONFIG.train_iters, CONFIG.eval_every, CONFIG.warm_start);

    let history = ActionHistory::new();
    println!("{}", history.pot());
}



