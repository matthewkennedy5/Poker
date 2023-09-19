use optimus::*;

fn main() {
    
    create_abstraction_clusters();

    train(CONFIG.train_iters, CONFIG.eval_every, CONFIG.warm_start);
}

