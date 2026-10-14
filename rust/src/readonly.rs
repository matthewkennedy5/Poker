use crate::card_abstraction::Abstraction;
use crate::card_utils::*;
use crate::config::CONFIG;
use crate::nodes::{Nodes, NUM_ACTIONS};
use crate::trainer::iterate;
use crate::trainer_utils::*;
use ahash::AHashMap;
use ahash::AHashMap as HashMap;
use memmap2::Mmap;
use rand::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};

// ────────────────────────────────────────────────────────────────────────────
// ReadOnlyAbstraction — mmap-backed sorted (u64, i32) arrays with binary search
// ────────────────────────────────────────────────────────────────────────────

struct MmapSortedArray {
    mmap: Mmap,
    entry_count: u64,
}

impl MmapSortedArray {
    fn open(path: &str) -> Self {
        let file = File::open(path).unwrap_or_else(|e| panic!("Failed to open {}: {}", path, e));
        let mmap = unsafe { Mmap::map(&file) }.expect("Failed to mmap abstraction file");
        let entry_count = u64::from_le_bytes(mmap[0..8].try_into().unwrap());
        MmapSortedArray { mmap, entry_count }
    }

    fn lookup(&self, key: u64) -> Option<i32> {
        let data = &self.mmap[8..];
        let n = self.entry_count as usize;
        let mut lo = 0usize;
        let mut hi = n;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let offset = mid * 16;
            let entry_key = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            if entry_key < key {
                lo = mid + 1;
            } else if entry_key > key {
                hi = mid;
            } else {
                let value = i32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());
                return Some(value);
            }
        }
        None
    }
}

pub struct ReadOnlyAbstraction {
    flop: MmapSortedArray,
    turn: MmapSortedArray,
    river: MmapSortedArray,
}

impl ReadOnlyAbstraction {
    pub fn open(flop_path: &str, turn_path: &str, river_path: &str) -> Self {
        println!("[INFO] Opening readonly abstractions...");
        ReadOnlyAbstraction {
            flop: MmapSortedArray::open(flop_path),
            turn: MmapSortedArray::open(turn_path),
            river: MmapSortedArray::open(river_path),
        }
    }

    pub fn bin(&self, cards: &[Card]) -> i32 {
        if cards.len() == 2 {
            Abstraction::preflop_bin(cards)
        } else {
            self.postflop_bin(cards)
        }
    }

    fn postflop_bin(&self, cards: &[Card]) -> i32 {
        let isomorphic = isomorphic_hand(cards);
        let hand = cards2hand(&isomorphic);
        let result = match cards.len() {
            5 => self.flop.lookup(hand),
            6 => self.turn.lookup(hand),
            7 => self.river.lookup(hand),
            _ => panic!("Bad number of cards"),
        };
        result.expect("Hand not found in readonly abstraction")
    }
}

impl CardAbstraction for ReadOnlyAbstraction {
    fn bin(&self, cards: &[Card]) -> i32 {
        ReadOnlyAbstraction::bin(self, cards)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ReadOnlyAbstraction file writer (used by convert binary)
// ────────────────────────────────────────────────────────────────────────────

/// Write a sorted (key, value) flat file from an iterator of (u64, i32) pairs.
pub fn write_readonly_abstraction(
    pairs: &mut Vec<(u64, i32)>,
    out_path: &str,
) {
    pairs.sort_unstable_by_key(|&(k, _)| k);
    let entry_count = pairs.len() as u64;
    let file = File::create(out_path).unwrap_or_else(|e| panic!("Failed to create {}: {}", out_path, e));
    let mut w = BufWriter::new(file);
    w.write_all(&entry_count.to_le_bytes()).unwrap();
    for &(key, value) in pairs.iter() {
        w.write_all(&key.to_le_bytes()).unwrap();
        w.write_all(&value.to_le_bytes()).unwrap();
        w.write_all(&0u32.to_le_bytes()).unwrap(); // padding to 16 bytes
    }
    w.flush().unwrap();
    println!("[CONVERT] Wrote {} entries to {}", entry_count, out_path);
}

// ────────────────────────────────────────────────────────────────────────────
// ReadOnlyNodes — in-RAM index + mmap'd pre-computed strategy data
// ────────────────────────────────────────────────────────────────────────────

struct IndexEntry {
    key_hash: u64,
    data_offset: u64,
    num_buckets: u32,
    num_actions: u32,
}

pub struct ReadOnlyNodes {
    index: Vec<IndexEntry>,
    data_mmap: Mmap,
    abstraction: ReadOnlyAbstraction,
    bet_abstraction: Vec<Vec<f64>>,
}

pub fn hash_history(history: &ActionHistory) -> u64 {
    let bytes = bincode::serialize(history).unwrap();
    xxhash_rust::xxh3::xxh3_64(&bytes)
}

impl ReadOnlyNodes {
    pub fn open(
        index_path: &str,
        data_path: &str,
        abstraction: ReadOnlyAbstraction,
    ) -> Self {
        println!("[INFO] Loading readonly node index from {}...", index_path);
        let index_file = File::open(index_path)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", index_path, e));
        let index_mmap = unsafe { Mmap::map(&index_file) }.expect("Failed to mmap index file");
        let entry_count = u64::from_le_bytes(index_mmap[0..8].try_into().unwrap()) as usize;

        let mut index = Vec::with_capacity(entry_count);
        for i in 0..entry_count {
            let base = 8 + i * 24;
            let key_hash = u64::from_le_bytes(index_mmap[base..base + 8].try_into().unwrap());
            let data_offset =
                u64::from_le_bytes(index_mmap[base + 8..base + 16].try_into().unwrap());
            let num_buckets =
                u32::from_le_bytes(index_mmap[base + 16..base + 20].try_into().unwrap());
            let num_actions =
                u32::from_le_bytes(index_mmap[base + 20..base + 24].try_into().unwrap());
            index.push(IndexEntry {
                key_hash,
                data_offset,
                num_buckets,
                num_actions,
            });
        }
        println!("[INFO] Loaded {} index entries", entry_count);

        println!("[INFO] Memory-mapping node data from {}...", data_path);
        let data_file = File::open(data_path)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", data_path, e));
        let data_mmap = unsafe { Mmap::map(&data_file) }.expect("Failed to mmap data file");

        ReadOnlyNodes {
            index,
            data_mmap,
            abstraction,
            bet_abstraction: CONFIG.bet_abstraction.clone(),
        }
    }

    pub fn get_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        self.try_get_strategy(hole, board, history)
            .unwrap_or_else(|| panic!("History not found in readonly nodes: {}", history))
    }

    /// Like `get_strategy`, but returns `None` if the history is not in the blueprint
    /// (e.g. a translated line that goes past the raise cap).
    pub fn try_get_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Option<Strategy> {
        if history.street > RIVER {
            // A translated line that ended in an untrained all-in.
            return None;
        }
        let expected_board_len = board_length(history.street);
        let board = &board[..expected_board_len.min(board.len())];
        let hand: SmallVecHand = hole.iter().chain(board.iter()).cloned().collect();
        let card_bucket = self.abstraction.bin(&hand) as usize;

        let hash = hash_history(history);

        // Binary search in the sorted index
        let idx = self.index.binary_search_by_key(&hash, |e| e.key_hash).ok()?;
        if !self.verify_key(idx, history) && self.find_colliding(idx, history).is_none() {
            return None;
        }

        // Handle potential hash collisions: scan to find the matching entry
        let entry = &self.index[idx];
        let (offset, num_actions) = self.resolve_entry(idx, history);

        // Read the serialized ActionHistory length and skip past it
        let key_len =
            u32::from_le_bytes(self.data_mmap[offset..offset + 4].try_into().unwrap()) as usize;
        let strategy_start = offset + 4 + key_len;

        // Read strategy for this card bucket
        let bucket_offset = strategy_start + card_bucket * num_actions * 4;
        let mut strategy = Strategy::new();
        let actions = history.next_actions(&self.bet_abstraction);
        debug_assert_eq!(
            actions.len(),
            num_actions,
            "Action count mismatch for history {}: file has {}, computed {}",
            history,
            num_actions,
            actions.len()
        );

        for (i, action) in actions.iter().enumerate() {
            let f_offset = bucket_offset + i * 4;
            let prob = f32::from_le_bytes(
                self.data_mmap[f_offset..f_offset + 4].try_into().unwrap(),
            ) as f64;
            strategy.insert(action.clone(), prob);
        }

        Some(strategy)
    }

    /// Scans neighbors with the same hash for the matching key. Returns the index.
    fn find_colliding(&self, found_idx: usize, history: &ActionHistory) -> Option<usize> {
        let hash = self.index[found_idx].key_hash;
        let mut lo = found_idx;
        while lo > 0 && self.index[lo - 1].key_hash == hash {
            lo -= 1;
            if self.verify_key(lo, history) {
                return Some(lo);
            }
        }
        let mut hi = found_idx + 1;
        while hi < self.index.len() && self.index[hi].key_hash == hash {
            if self.verify_key(hi, history) {
                return Some(hi);
            }
            hi += 1;
        }
        None
    }

    /// Given an index position from binary_search, verify the key matches.
    /// On hash collision, scan neighbors. Returns (data_offset, num_actions).
    fn resolve_entry(&self, found_idx: usize, history: &ActionHistory) -> (usize, usize) {
        // Try the found index first (the common case — no collision)
        if self.verify_key(found_idx, history) {
            let e = &self.index[found_idx];
            return (e.data_offset as usize, e.num_actions as usize);
        }

        // Hash collision — scan left and right for the matching key
        let hash = self.index[found_idx].key_hash;
        let mut lo = found_idx;
        while lo > 0 && self.index[lo - 1].key_hash == hash {
            lo -= 1;
            if self.verify_key(lo, history) {
                let e = &self.index[lo];
                return (e.data_offset as usize, e.num_actions as usize);
            }
        }
        let mut hi = found_idx + 1;
        while hi < self.index.len() && self.index[hi].key_hash == hash {
            if self.verify_key(hi, history) {
                let e = &self.index[hi];
                return (e.data_offset as usize, e.num_actions as usize);
            }
            hi += 1;
        }

        panic!("Hash collision resolution failed for history: {}", history);
    }

    fn verify_key(&self, idx: usize, history: &ActionHistory) -> bool {
        let e = &self.index[idx];
        let offset = e.data_offset as usize;
        let key_len =
            u32::from_le_bytes(self.data_mmap[offset..offset + 4].try_into().unwrap()) as usize;
        let stored: ActionHistory =
            bincode::deserialize(&self.data_mmap[offset + 4..offset + 4 + key_len]).unwrap();
        &stored == history
    }

    /// Batch lookup: one binary search, returns strategy probabilities for ALL buckets.
    /// Returns (strategies_by_bucket, actions) where strategies_by_bucket[bucket][action_idx] = prob.
    pub fn get_strategy_all_buckets(
        &self,
        history: &ActionHistory,
    ) -> (Vec<[f64; NUM_ACTIONS]>, Vec<Action>) {
        let hash = hash_history(history);
        let idx = match self.index.binary_search_by_key(&hash, |e| e.key_hash) {
            Ok(i) => i,
            Err(_) => panic!("History not found in readonly nodes: {}", history),
        };
        let (offset, num_actions) = self.resolve_entry(idx, history);
        let entry = &self.index[idx];
        let num_buckets = entry.num_buckets as usize;

        let key_len =
            u32::from_le_bytes(self.data_mmap[offset..offset + 4].try_into().unwrap()) as usize;
        let strategy_start = offset + 4 + key_len;

        let actions: Vec<Action> = history.next_actions(&self.bet_abstraction).to_vec();
        debug_assert_eq!(actions.len(), num_actions);

        let mut strategies = vec![[0.0f64; NUM_ACTIONS]; num_buckets];
        for bucket in 0..num_buckets {
            let bucket_offset = strategy_start + bucket * num_actions * 4;
            for a in 0..num_actions {
                let f_offset = bucket_offset + a * 4;
                strategies[bucket][a] = f32::from_le_bytes(
                    self.data_mmap[f_offset..f_offset + 4].try_into().unwrap(),
                ) as f64;
            }
        }

        (strategies, actions)
    }

    pub fn bet_abstraction(&self) -> &[Vec<f64>] {
        &self.bet_abstraction
    }

    pub fn abstraction(&self) -> &ReadOnlyAbstraction {
        &self.abstraction
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ReadOnlyNodes file writer (used by convert binary)
// ────────────────────────────────────────────────────────────────────────────

/// Entries collected before sorting and writing.
pub struct ConvertEntry {
    pub key_hash: u64,
    pub history_bytes: Vec<u8>,
    pub num_buckets: u32,
    pub num_actions: u32,
    /// Flat array of f32, length = num_buckets * num_actions.
    /// Layout: bucket 0 action 0, bucket 0 action 1, ..., bucket N action M
    pub strategy_data: Vec<f32>,
}

pub fn write_readonly_nodes(
    mut entries: Vec<ConvertEntry>,
    index_path: &str,
    data_path: &str,
) {
    // Sort by hash for binary search
    entries.sort_unstable_by_key(|e| e.key_hash);

    // First pass: compute data offsets and write data file
    let data_file =
        File::create(data_path).unwrap_or_else(|e| panic!("Failed to create {}: {}", data_path, e));
    let mut data_w = BufWriter::new(data_file);
    let mut offsets: Vec<u64> = Vec::with_capacity(entries.len());

    let mut current_offset: u64 = 0;
    for entry in &entries {
        offsets.push(current_offset);
        let key_len = entry.history_bytes.len() as u32;
        data_w.write_all(&key_len.to_le_bytes()).unwrap();
        data_w.write_all(&entry.history_bytes).unwrap();
        for &val in &entry.strategy_data {
            data_w.write_all(&val.to_le_bytes()).unwrap();
        }
        current_offset += 4 + key_len as u64 + (entry.strategy_data.len() as u64) * 4;
    }
    data_w.flush().unwrap();
    println!(
        "[CONVERT] Wrote {} bytes of node data to {}",
        current_offset, data_path
    );

    // Write index file
    let index_file = File::create(index_path)
        .unwrap_or_else(|e| panic!("Failed to create {}: {}", index_path, e));
    let mut idx_w = BufWriter::new(index_file);
    let entry_count = entries.len() as u64;
    idx_w.write_all(&entry_count.to_le_bytes()).unwrap();
    for (i, entry) in entries.iter().enumerate() {
        idx_w.write_all(&entry.key_hash.to_le_bytes()).unwrap();
        idx_w.write_all(&offsets[i].to_le_bytes()).unwrap();
        idx_w.write_all(&entry.num_buckets.to_le_bytes()).unwrap();
        idx_w.write_all(&entry.num_actions.to_le_bytes()).unwrap();
    }
    idx_w.flush().unwrap();
    println!(
        "[CONVERT] Wrote {} index entries to {}",
        entry_count, index_path
    );
}

// ────────────────────────────────────────────────────────────────────────────
// PrecomputedAbstraction — fast lookup table for river subgame solving
// ────────────────────────────────────────────────────────────────────────────

/// Card abstraction backed by a precomputed lookup table keyed on hole cards.
/// Avoids repeated isomorphic-hand normalization + mmap binary searches during
/// the CFR loop, replacing them with a single HashMap lookup.
struct PrecomputedAbstraction {
    buckets: AHashMap<[Card; 2], i32>,
}

impl PrecomputedAbstraction {
    fn precompute(board: &[Card], abstraction: &dyn CardAbstraction) -> Self {
        let hands = non_blocking_preflop_hands(board);
        let mut buckets = AHashMap::with_capacity(hands.len());
        for hand in hands {
            let full_hand: SmallVecHand = hand.iter().chain(board.iter()).cloned().collect();
            let bucket = abstraction.bin(&full_hand);
            buckets.insert(hand, bucket);
        }
        PrecomputedAbstraction { buckets }
    }
}

impl CardAbstraction for PrecomputedAbstraction {
    fn bin(&self, cards: &[Card]) -> i32 {
        let mut key = [cards[0], cards[1]];
        key.sort();
        *self.buckets.get(&key).unwrap()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ReadOnlyBot — lightweight bot using ReadOnlyNodes for the server
// ────────────────────────────────────────────────────────────────────────────

pub struct ReadOnlyBot {
    nodes: ReadOnlyNodes,
}

impl ReadOnlyBot {
    pub fn new(nodes: ReadOnlyNodes) -> Self {
        ReadOnlyBot { nodes }
    }

    pub fn nodes(&self) -> &ReadOnlyNodes {
        &self.nodes
    }

    pub fn get_action(&self, hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
        let mut strategy = self.get_strategy(hand, board, history);

        // Smoothing — zero out actions with < 5% probability
        for (action, prob) in strategy.clone() {
            if prob < 0.05 {
                strategy.insert(action, 0.0);
            }
        }

        sample_action_from_strategy(&strategy)
    }

    fn get_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        if CONFIG.subgame_solving && !history.is_empty() {
            if history.street >= RIVER {
                self.river_subgame_solve(hole, board, history)
            } else if history.street == TURN {
                self.turn_subgame_solve(hole, board, history)
            } else {
                self.get_strategy_action_translation(hole, board, history)
            }
        } else {
            self.get_strategy_action_translation(hole, board, history)
        }
    }

    fn river_subgame_solve(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        let start = std::time::Instant::now();
        let board = &board[..5];
        println!(
            "[SUBGAME] Starting river solve for hand {} board {} history {}",
            cards2str(hole), cards2str(board), history
        );

        // Step 1: Precompute hand strengths and abstraction buckets (avoids
        // repeated mmap binary searches during the CFR loop)
        let strengths = precompute_river_strengths(board);
        let precomputed_abs = PrecomputedAbstraction::precompute(board, self.nodes.abstraction());
        println!("[SUBGAME] Precomputed {} hand strengths + buckets in {:.1?}", strengths.len(), start.elapsed());

        // Step 2: Compute reach probs by replaying history against blueprint
        let reach_start = std::time::Instant::now();
        let preflop_hands = non_blocking_preflop_hands(board);
        let mut dealer_reach_probs = vec![1.0; preflop_hands.len()];
        let mut oop_reach_probs = vec![1.0; preflop_hands.len()];
        let bet_abs = self.nodes.bet_abstraction().to_vec();
        let translated_history = history.translate(&bet_abs);
        let mut history_iter = ActionHistory::new();
        for action in translated_history.get_actions() {
            for (i, preflop_hand) in preflop_hands.iter().enumerate() {
                let strat = self.nodes.get_strategy(preflop_hand, board, &history_iter);
                let prob = strat.get(&action).copied().unwrap_or(0.0);
                if history_iter.player == DEALER {
                    dealer_reach_probs[i] *= prob;
                } else {
                    oop_reach_probs[i] *= prob;
                }
            }
            history_iter.add(&action);
        }

        // Pre-filter nonzero hands once (reach probs don't change between iterations)
        let n = preflop_hands.len();
        let mut nonzero_hands: Vec<[Card; 2]> = Vec::with_capacity(n);
        let mut nonzero_dealer: Vec<f64> = Vec::with_capacity(n);
        let mut nonzero_oop: Vec<f64> = Vec::with_capacity(n);
        for i in 0..n {
            if dealer_reach_probs[i] > 1e-10 || oop_reach_probs[i] > 1e-10 {
                nonzero_hands.push(preflop_hands[i]);
                nonzero_dealer.push(dealer_reach_probs[i]);
                nonzero_oop.push(oop_reach_probs[i]);
            }
        }
        println!(
            "[SUBGAME] Reach probs computed in {:.1?} ({} hands, {} nonzero)",
            reach_start.elapsed(), n, nonzero_hands.len()
        );

        // Step 3: Create fresh Nodes for the river subgame
        let subgame_nodes = Nodes::new(&bet_abs);
        let board_arr = [board[0], board[1], board[2], board[3], board[4]];

        // Step 4: Run CFR iterations (sequential — board is fixed on river)
        let cfr_start = std::time::Instant::now();
        for _ in 0..CONFIG.subgame_iters {
            for &traverser in [DEALER, OPPONENT].iter() {
                let (traverser_probs, opp_probs) = if traverser == DEALER {
                    (&nonzero_dealer, &nonzero_oop)
                } else {
                    (&nonzero_oop, &nonzero_dealer)
                };

                iterate(
                    traverser,
                    &nonzero_hands,
                    board_arr,
                    history,
                    traverser_probs.clone(),
                    opp_probs.clone(),
                    &subgame_nodes,
                    -1,
                    None,
                    &precomputed_abs,
                    Some(&strengths),
                    None,
                    None,
                    None,
                );
            }
        }

        println!("[SUBGAME] CFR done: {} iters in {:.1?}", CONFIG.subgame_iters, cfr_start.elapsed());

        // Step 5: Extract average strategy for the queried hand
        let strategy = subgame_nodes.get_strategy_with_abstraction(hole, board, history, &precomputed_abs);
        println!(
            "[SUBGAME] Total solve time: {:.1?} | strategy: {:?}",
            start.elapsed(), strategy
        );
        strategy
    }

    fn turn_subgame_solve(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        let start = std::time::Instant::now();
        let turn_board = &board[..4];
        println!(
            "[SUBGAME] Starting turn solve for hand {} board {} history {}",
            cards2str(hole), cards2str(turn_board), history
        );

        // Step 1: Precompute turn abstraction buckets
        let precomputed_abs = PrecomputedAbstraction::precompute(turn_board, self.nodes.abstraction());
        println!("[SUBGAME] Precomputed turn buckets in {:.1?}", start.elapsed());

        // Step 2: Compute reach probs by replaying history against blueprint
        let reach_start = std::time::Instant::now();
        let preflop_hands = non_blocking_preflop_hands(turn_board);
        let mut dealer_reach_probs = vec![1.0; preflop_hands.len()];
        let mut oop_reach_probs = vec![1.0; preflop_hands.len()];
        let bet_abs = self.nodes.bet_abstraction().to_vec();
        let translated_history = history.translate(&bet_abs);
        let mut history_iter = ActionHistory::new();
        for action in translated_history.get_actions() {
            for (i, preflop_hand) in preflop_hands.iter().enumerate() {
                let strat = self.nodes.get_strategy(preflop_hand, board, &history_iter);
                let prob = strat.get(&action).copied().unwrap_or(0.0);
                if history_iter.player == DEALER {
                    dealer_reach_probs[i] *= prob;
                } else {
                    oop_reach_probs[i] *= prob;
                }
            }
            history_iter.add(&action);
        }

        // Pre-filter nonzero hands (tighter filter for turn — reduces hand count significantly)
        let n = preflop_hands.len();
        let mut nonzero_hands: Vec<[Card; 2]> = Vec::with_capacity(n);
        let mut nonzero_dealer: Vec<f64> = Vec::with_capacity(n);
        let mut nonzero_oop: Vec<f64> = Vec::with_capacity(n);
        for i in 0..n {
            if dealer_reach_probs[i] > 1e-4 || oop_reach_probs[i] > 1e-4 {
                nonzero_hands.push(preflop_hands[i]);
                nonzero_dealer.push(dealer_reach_probs[i]);
                nonzero_oop.push(oop_reach_probs[i]);
            }
        }
        println!(
            "[SUBGAME] Reach probs computed in {:.1?} ({} hands, {} nonzero)",
            reach_start.elapsed(), n, nonzero_hands.len()
        );

        // Step 3: Create fresh Nodes for the turn subgame
        let subgame_nodes = Nodes::new(&bet_abs);

        // Collect possible river cards (any card not on the turn board)
        let all_cards = deck();
        let river_cards: Vec<Card> = all_cards
            .into_iter()
            .filter(|c| !turn_board.contains(c))
            .collect();

        // Step 4: Run CFR iterations — sample a river card each iteration
        let cfr_start = std::time::Instant::now();
        let mut rng = rand::thread_rng();
        for iter in 0..CONFIG.turn_subgame_iters {
            if iter % 20 == 0 {
                println!("[SUBGAME] Turn iter {}/{} ({:.1?} elapsed)", iter, CONFIG.turn_subgame_iters, cfr_start.elapsed());
            }
            // Sample a river card for this iteration
            let river_card = river_cards[rng.gen_range(0..river_cards.len())];
            let board_arr = [turn_board[0], turn_board[1], turn_board[2], turn_board[3], river_card];

            // Precompute river hand strengths and abstraction buckets for this board
            let strengths = precompute_river_strengths(&board_arr);
            let river_abs = PrecomputedAbstraction::precompute(&board_arr, self.nodes.abstraction());

            // Leaf evaluator: at river boundary, evaluate using blueprint
            let leaf_fn = |traverser: usize,
                           hands: Vec<[Card; 2]>,
                           board: [Card; 5],
                           hist: &ActionHistory,
                           t_reach: Vec<f64>,
                           o_reach: Vec<f64>| {
                // Precompute bucket for each hand (O(1) HashMap lookups)
                let buckets: Vec<usize> = hands.iter().map(|h| river_abs.bin(h) as usize).collect();
                // Translate history to blueprint's bet abstraction
                let translated = hist.translate(&bet_abs);
                self.blueprint_evaluate_river(
                    traverser, &hands, &buckets, board, &translated, &t_reach, &o_reach, &strengths, &bet_abs,
                )
            };

            for &traverser in [DEALER, OPPONENT].iter() {
                let (traverser_probs, opp_probs) = if traverser == DEALER {
                    (&nonzero_dealer, &nonzero_oop)
                } else {
                    (&nonzero_oop, &nonzero_dealer)
                };

                // Filter out hands that block the sampled river card
                let mut iter_hands = Vec::with_capacity(nonzero_hands.len());
                let mut iter_t_probs = Vec::with_capacity(nonzero_hands.len());
                let mut iter_o_probs = Vec::with_capacity(nonzero_hands.len());
                for i in 0..nonzero_hands.len() {
                    let h = nonzero_hands[i];
                    if h[0] != river_card && h[1] != river_card {
                        iter_hands.push(h);
                        iter_t_probs.push(traverser_probs[i]);
                        iter_o_probs.push(opp_probs[i]);
                    }
                }

                iterate(
                    traverser,
                    &iter_hands,
                    board_arr,
                    history,
                    iter_t_probs,
                    iter_o_probs,
                    &subgame_nodes,
                    -1,
                    None,
                    &precomputed_abs,
                    Some(&strengths),
                    Some(&leaf_fn),
                    None,
                    None,
                );
            }
        }

        println!(
            "[SUBGAME] Turn CFR done: {} iters in {:.1?}",
            CONFIG.turn_subgame_iters, cfr_start.elapsed()
        );

        // Step 5: Extract average strategy for the queried hand
        let strategy = subgame_nodes.get_strategy_with_abstraction(hole, board, history, &precomputed_abs);
        println!(
            "[SUBGAME] Total turn solve time: {:.1?} | strategy: {:?}",
            start.elapsed(), strategy
        );
        strategy
    }

    /// Traverse the blueprint's river subtree with fixed strategies (no regret updates).
    /// Returns expected utility per hand. Uses batch strategy lookup for speed.
    fn blueprint_evaluate_river(
        &self,
        traverser: usize,
        preflop_hands: &[[Card; 2]],
        hand_buckets: &[usize],
        board: [Card; 5],
        history: &ActionHistory, // already translated to blueprint bet abstraction
        traverser_reach_probs: &[f64],
        opp_reach_probs: &[f64],
        precomputed_strengths: &HashMap<[Card; 2], i32>,
        bet_abs: &[Vec<f64>],
    ) -> Vec<f64> {
        let n = preflop_hands.len();
        if n == 0 {
            return Vec::new();
        }

        // Terminal node
        if history.hand_over() {
            return terminal_utility_vectorized(
                preflop_hands.to_vec(),
                opp_reach_probs.to_vec(),
                &board,
                history,
                traverser,
                Some(precomputed_strengths),
            );
        }

        // Batch lookup: one binary search for all buckets
        let (bucket_strategies, actions) = self.nodes.get_strategy_all_buckets(history);
        if actions.is_empty() {
            return vec![0.0; n];
        }
        let num_actions = actions.len();

        // Map each hand to its bucket's strategy
        let hand_probs: Vec<&[f64; NUM_ACTIONS]> = hand_buckets
            .iter()
            .map(|&b| &bucket_strategies[b])
            .collect();

        let mut node_utility = vec![0.0; n];
        for action_idx in 0..num_actions {
            let probs: Vec<f64> = hand_probs.iter().map(|s| s[action_idx]).collect();

            let mut next_t_reach = traverser_reach_probs.to_vec();
            let mut next_o_reach = opp_reach_probs.to_vec();

            if history.player == traverser {
                for i in 0..n {
                    next_t_reach[i] *= probs[i];
                }
            } else {
                for i in 0..n {
                    next_o_reach[i] *= probs[i];
                }
            }

            let mut next_history = history.clone();
            next_history.add(&actions[action_idx]);

            let utility = self.blueprint_evaluate_river(
                traverser,
                preflop_hands,
                hand_buckets,
                board,
                &next_history,
                &next_t_reach,
                &next_o_reach,
                precomputed_strengths,
                bet_abs,
            );

            for i in 0..n {
                let weight = if history.player == traverser {
                    probs[i]
                } else {
                    1.0
                };
                node_utility[i] += weight * utility[i];
            }
        }

        node_utility
    }

    fn get_strategy_action_translation(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
    ) -> Strategy {
        debug_assert!(hole.len() == 2);
        let board = &board[..board_length(history.street).min(board.len())];
        let translated = history.translate(&CONFIG.bet_abstraction);
        let node_strategy = match self.nodes.try_get_strategy(hole, board, &translated) {
            Some(s) => s,
            None => self.over_cap_strategy(hole, board, history, &translated),
        };
        // Map abstract actions onto legal real actions, merging any that collide.
        let mut adjusted_strategy = Strategy::new();
        for (action, prob) in node_strategy.iter() {
            *adjusted_strategy
                .entry(history.adjust_action(action))
                .or_insert(0.0) += *prob;
        }
        adjusted_strategy
    }

    /// Strategy for a translated history that the blueprint does not contain. This
    /// happens when the opponent raises past the abstraction's raise cap, so the
    /// translated line ends in an all-in that was never trained. We reuse the
    /// call/fold mix from our previous decision on this street (facing the
    /// opponent's earlier raise), which is the closest trained node.
    fn over_cap_strategy(
        &self,
        hole: &[Card],
        board: &[Card],
        history: &ActionHistory,
        translated: &ActionHistory,
    ) -> Strategy {
        let mut candidate = translated.clone();
        let mut found: Option<Strategy> = None;
        // Walk back two actions at a time so it is still our decision point.
        while candidate.get_actions().len() >= 2 && candidate.street == translated.street {
            candidate = candidate.without_last_action().without_last_action();
            if candidate.street != translated.street {
                break;
            }
            if let Some(s) = self.nodes.try_get_strategy(hole, board, &candidate) {
                found = Some(s);
                break;
            }
        }
        let mut strategy = Strategy::new();
        let to_call = history.to_call();
        let call = Action { action: ActionType::Call, amount: to_call };
        if to_call == 0 {
            // Nothing to call (e.g. first to act on a later street after an
            // untrained preflop line): folding would be illegal, so check.
            strategy.insert(call, 1.0);
            println!("[BLUEPRINT] untrained line {} -> check", translated);
            return strategy;
        }
        match found {
            Some(prev) => {
                let mut fold_p = 0.0;
                let mut call_p = 0.0;
                for (a, p) in prev.iter() {
                    match a.action {
                        ActionType::Fold => fold_p += p,
                        _ => call_p += p,
                    }
                }
                // Facing a bigger raise than the trained node saw: shift weight to folding.
                let total = fold_p + call_p;
                if total > 0.0 {
                    let fold_frac = (fold_p / total).max(0.5);
                    strategy.insert(FOLD, fold_frac);
                    strategy.insert(call, 1.0 - fold_frac);
                } else {
                    strategy.insert(FOLD, 0.5);
                    strategy.insert(call, 0.5);
                }
            }
            None => {
                strategy.insert(FOLD, 0.5);
                strategy.insert(call, 0.5);
            }
        }
        println!(
            "[BLUEPRINT] untrained line {} -> call/fold heuristic {:?}",
            translated, strategy
        );
        strategy
    }
}
