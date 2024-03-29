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
    // Game parameters
    pub stack_size: u16,
    pub big_blind: u16,
    pub small_blind: u16,
    pub last_street: String,

    // Abstraction
    pub bet_abstraction: Vec<Vec<f64>>,
    pub flop_buckets: i32,
    pub turn_buckets: i32,
    pub river_buckets: i32,

    // File paths
    pub nodes_path: String,

    // Training
    pub train_iters: usize,
    pub lbr_iters: usize,
    pub eval_every: usize,
    pub warm_start: bool,
    pub k_means_iters: u64,

    // Real time solving
    pub subgame_solving: bool,
    pub subgame_iters: usize,
    pub depth_limit: i32,

    // Preflop chart
    pub preflop_strategy_path: String,
}
