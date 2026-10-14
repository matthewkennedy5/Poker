// For reading and storing the configuration file info

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;

fn default_preflop_raise_limit() -> usize {
    3
}
fn default_exploit_method() -> String {
    "br".to_string()
}
fn default_br_boards() -> usize {
    500
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // POKER_PARAMS overrides the config path (used for test configurations).
    let path = std::env::var("POKER_PARAMS").unwrap_or_else(|_| "../params.toml".to_string());
    let config_string = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Could not read config {}: {}", path, e));
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

    // Tree size
    pub raise_limit: usize, // Max number of bets/raises per street (0 = unlimited)
    #[serde(default = "default_preflop_raise_limit")]
    pub preflop_raise_limit: usize, // Max number of bets/raises preflop

    // Trainer algorithm. Linear CFR weights iteration t's regret and strategy
    // contributions by t (global iteration counter). `regret_floor` clamps negative
    // regrets to zero (regret matching+).
    #[serde(default)]
    pub regret_floor: bool,
    /// "br": exact best response over `br_boards` sampled boards; "lbr": greedy local
    /// best response over `lbr_iters` hands (old, unreliable).
    #[serde(default = "default_exploit_method")]
    pub exploit_method: String,
    #[serde(default = "default_br_boards")]
    pub br_boards: usize,
    /// Test aid: if set (e.g. "Td9d6h"), every sampled board starts with these
    /// cards, so the game has a single flop (used with a lossless abstraction to
    /// check that CFR converges).
    #[serde(default)]
    pub fixed_board: String,

    // Real time solving
    pub subgame_solving: bool,
    pub subgame_iters: usize,
    pub turn_subgame_iters: usize,
    pub depth_limit: i32,

    // Preflop chart
    pub preflop_strategy_path: String,

    // Real-time search with postflop-solver (see search.rs)
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SearchConfig {
    /// Use postflop-solver search on flop/turn/river. If false, SearchBot delegates
    /// everything to the blueprint bot.
    pub enabled: bool,
    /// First street solved in real time (1 = flop, 2 = turn, 3 = river). Earlier
    /// streets use the blueprint. Flop solves to the river are very expensive at
    /// 200bb, so the default starts at the turn.
    pub first_search_street: usize,
    /// Seats that get flop search regardless of `first_search_street`:
    /// "none", "bb" (out of position), "sb" (button), or "both".
    pub flop_search_seats: String,
    /// Tree shape for flop-rooted solves (they contain every turn and river, so they
    /// must stay small): raise cap per street and the turn/river sizes used inside.
    pub flop_tree_max_raises: usize,
    /// Cap on the turn and river inside flop-rooted solves (0 = same as the flop).
    /// The first bet counts, so 1 means bet + all-in only and 2 allows one raise.
    pub flop_tree_later_max_raises: usize,
    pub flop_tree_turn_bet_sizes: String,
    pub flop_tree_turn_raise_sizes: String,
    pub flop_tree_river_bet_sizes: String,
    pub flop_tree_river_raise_sizes: String,
    /// Rayon threads for the solver (0 = rayon default).
    pub threads: usize,
    /// Store node values as 16-bit integers (halves memory).
    pub compression: bool,
    /// Wall-clock budget per solve, in milliseconds.
    pub flop_time_ms: u64,
    pub turn_time_ms: u64,
    pub river_time_ms: u64,
    /// Stop early once exploitability is below this fraction of the starting pot.
    pub target_exploitability: f64,
    pub max_iters: u32,
    /// If the budget expires before this many iterations, fall back to the blueprint.
    pub min_iters: u32,
    /// Bet / raise size strings in postflop-solver syntax, used for both players.
    pub flop_bet_sizes: String,
    pub flop_raise_sizes: String,
    pub turn_bet_sizes: String,
    pub turn_raise_sizes: String,
    pub river_bet_sizes: String,
    pub river_raise_sizes: String,
    pub add_allin_threshold: f64,
    pub force_allin_threshold: f64,
    pub merging_threshold: f64,
    /// Max bets/raises per street in the tree (after that only fold/call/all-in).
    /// Real opponent lines beyond the cap are still added exactly.
    pub max_raises_per_street: usize,
    /// Zero out actions below this probability before sampling.
    pub purify_threshold: f64,
    /// Number of hands whose solves are kept in memory.
    pub cache_hands: usize,
    /// Refuse solves that would need more memory than this.
    pub max_memory_mb: u64,
    /// Floor on our own hand's weight in our own range.
    pub own_hand_floor: f64,
    /// Drop combos whose entry-range weight is below this fraction of the max
    /// weight (never our own hand). Blueprint ranges are mixed across almost every
    /// combo with tiny weights; pruning them makes solves much cheaper.
    pub range_prune_threshold: f64,
    pub log: bool,
    /// Directory of precomputed flop solutions (see flopbook.rs). Empty or missing
    /// means no book.
    pub flop_book_dir: String,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            enabled: false,
            first_search_street: 2,
            flop_search_seats: "none".to_string(),
            flop_tree_max_raises: 1,
            flop_tree_later_max_raises: 0,
            flop_tree_turn_bet_sizes: "50%".to_string(),
            flop_tree_turn_raise_sizes: "a".to_string(),
            flop_tree_river_bet_sizes: "50%, a".to_string(),
            flop_tree_river_raise_sizes: "a".to_string(),
            threads: 0,
            compression: true,
            flop_time_ms: 30000,
            turn_time_ms: 4000,
            river_time_ms: 2000,
            target_exploitability: 0.005,
            max_iters: 5000,
            min_iters: 30,
            flop_bet_sizes: "33%, 75%".to_string(),
            flop_raise_sizes: "60%".to_string(),
            turn_bet_sizes: "50%, 100%".to_string(),
            turn_raise_sizes: "2.5x".to_string(),
            river_bet_sizes: "50%, 100%, a".to_string(),
            river_raise_sizes: "2.5x".to_string(),
            add_allin_threshold: 1.5,
            force_allin_threshold: 0.15,
            merging_threshold: 0.1,
            max_raises_per_street: 2,
            purify_threshold: 0.03,
            cache_hands: 16,
            max_memory_mb: 8000,
            own_hand_floor: 1e-4,
            range_prune_threshold: 0.0,
            log: true,
            flop_book_dir: "products/flopbook".to_string(),
        }
    }
}
