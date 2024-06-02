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
    // create_abstraction_clusters();
    //
    // START HERE: Prune the initial preflop actions to only have Bet 200, not Call 100 or Bet 300. That will hugely reduce the size of the action tree, speeding everything up.

    if CONFIG.last_street != "river" {
        println!("Warning: last_street is {}", CONFIG.last_street);
    }
    train(CONFIG.train_iters, CONFIG.eval_every, CONFIG.warm_start);
    // subgame_solving_beats_blueprint();
}
