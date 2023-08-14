use optimus::*;

use std::path::Path;

fn main() {
    
    if !Path::new(FLOP_ABSTRACTION_PATH).exists() {
        make_abstraction(5, CONFIG.flop_buckets);
    }
    if !Path::new(TURN_ABSTRACTION_PATH).exists() {
        make_abstraction(6, CONFIG.turn_buckets);
    }
    if !Path::new(RIVER_ABSTRACTION_PATH).exists() {
        make_abstraction(7, CONFIG.river_buckets);
    }

    train(CONFIG.train_iters, CONFIG.eval_every, CONFIG.warm_start);
}



