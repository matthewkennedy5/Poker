use optimus::*;

fn main() {
    // create_abstraction_clusters();
    // print_abstraction();
    train(CONFIG.train_iters, CONFIG.eval_every, CONFIG.warm_start);
    // bucket_sizes();
}

