use optimus::*;

use std::path::Path;

fn main() {
    // If the abstraction file doesn't exist, we want to first create the
    // abstraction before the Lazy cell is called, because the Lazy initializer
    // prevents parallelization
    // if !Path::new(FLOP_ABSTRACTION_PATH).exists() {
    //     make_abstraction(5, CONFIG.flop_buckets);
    // }
    // if !Path::new(TURN_ABSTRACTION_PATH).exists() {
    //     make_abstraction(6, CONFIG.turn_buckets);
    // }
    // if !Path::new(RIVER_ABSTRACTION_PATH).exists() {
    //     make_abstraction(7, CONFIG.river_buckets);
    // }
    // print_abstraction();

    // START HERE: after making sure everything is still working, also start here to make sure your
    // changes are generally working for texas holdem. Like, bucket lookup post flop? isomorphic or pure lookup?

    create_abstraction_clusters();
    expand_abstraction_keys();
    train(CONFIG.train_iters, CONFIG.eval_every, CONFIG.warm_start);
}
