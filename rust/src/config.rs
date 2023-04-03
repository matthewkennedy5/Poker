// For reading and storing the configuration file info

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;


pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config_string = fs::read_to_string("../params.toml").unwrap();
    
    toml::from_str(&config_string).expect("Could not parse TOML config file")
});

#[derive(Serialize, Deserialize)]
pub struct Config {
    // Chip parameters
    pub stack_size: i32,
    pub big_blind: i32,
    pub small_blind: i32,

    // Abstraction
    pub bet_abstraction: Vec<Vec<f64>>,
    pub flop_buckets: i32,
    pub turn_buckets: i32,
    pub river_buckets: i32,

    // DCFR parameters
    pub alpha: f64,
    pub beta: f64,

    // File paths
    pub nodes_path: String,

    // Training
    pub train_iters: u64,
    pub lbr_iters: u64,
    pub eval_every: u64,

    // Real time solving
    pub subgame_solving: bool,
    pub subgame_iters: u64,
    pub depth_limit: i32,

    // Preflop chart
    pub preflop_strategy_path: String,
}
