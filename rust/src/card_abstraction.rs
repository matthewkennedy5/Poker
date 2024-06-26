// This is the main interface for card abstractions. You can think of this as a
// black box that maps a poker hand to an ID number corresponding to the
// hand's abstraction bin. The idea is that similar hands will have the same
// abstraction id number, so we can treat similar hands as the same to reduce
// the number of possibilities in the game.

use crate::config::CONFIG;
use crate::{card_utils::*, ABSTRACTION};
use ahash::AHashMap as HashMap;
use dashmap::DashMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use rand::prelude::*;
use rayon::prelude::*;
use smallvec::ToSmallVec;
use std::sync::Mutex;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

pub static RIVER_EQUITY_CACHE: Lazy<DashMap<SmallVecHand, f64>> = Lazy::new(DashMap::new);

pub const FLOP_ABSTRACTION_PATH: &str = "products/flop_abstraction.bin";
pub const TURN_ABSTRACTION_PATH: &str = "products/turn_abstraction.bin";
pub const RIVER_ABSTRACTION_PATH: &str = "products/river_abstraction.bin";

pub struct Abstraction {
    flop: HashMap<u64, i32>,
    turn: HashMap<u64, i32>,
    river: HashMap<u64, i32>,
}

impl Abstraction {
    pub fn new() -> Abstraction {
        Abstraction {
            flop: load_abstraction(FLOP_ABSTRACTION_PATH, 5, CONFIG.flop_buckets),
            turn: load_abstraction(TURN_ABSTRACTION_PATH, 6, CONFIG.turn_buckets),
            river: load_abstraction(RIVER_ABSTRACTION_PATH, 7, CONFIG.river_buckets),
        }
    }

    pub fn bin(&self, cards: &[Card]) -> i32 {
        if cards.len() == 2 {
            Abstraction::preflop_bin(cards)
        } else {
            self.postflop_bin(cards)
        }
    }

    // Map each possible preflop hand to an integer in (0..169)
    fn preflop_bin(cards: &[Card]) -> i32 {
        let mut cards = cards.to_vec();
        cards.sort_by_key(|c| c.rank);
        // Ranks start at 2, so shift it to start at 0
        let rank1: i32 = cards[0].rank as i32 - 2;
        let rank2: i32 = cards[1].rank as i32 - 2;
        let bin = if cards[0].suit == cards[1].suit {
            rank1 * 13 + rank2
        } else {
            rank2 * 13 + rank1
        };
        bin
    }

    fn postflop_bin(&self, cards: &[Card]) -> i32 {
        let isomorphic = isomorphic_hand(cards);
        let hand = cards2hand(&isomorphic);
        let bin_result = match cards.len() {
            5 => self.flop.get(&hand),
            6 => self.turn.get(&hand),
            // TODO: Something's wrong here, AHashMap shouldn't be so slow.
            // Try a different hashmap library?
            7 => self.river.get(&hand),
            _ => panic!("Bad number of cards"),
        };
        *bin_result.unwrap()
    }
}

fn load_abstraction(path: &str, n_cards: usize, n_buckets: i32) -> HashMap<u64, i32> {
    match File::open(path) {
        Err(_error) => make_abstraction(n_cards, n_buckets),
        Ok(_) => {
            let abstraction = read_serialized(path);
            assert!(
                {
                    let max_bucket = abstraction.values().max().unwrap().clone();
                    let min_bucket = abstraction.values().min().unwrap().clone();
                    max_bucket >= n_buckets - 10 && min_bucket == 0
                },
                "Number of {n_cards} abstraction buckets in params.toml does not match the abstraction file."
            );
            abstraction
        }
    }
}

// Returns all isomorphic hands in sorted order by E[HS^2]
pub fn get_sorted_hand_ehs2(n_cards: usize) -> Vec<u64> {
    let path = format!("products/ehs2_{n_cards}.bin");
    if Path::new(&path).exists() {
        let file = File::open(path.as_str()).unwrap();
        let reader = BufReader::new(file);
        let ehs2: Vec<u64> = bincode::deserialize_from(reader).unwrap();
        return ehs2;
    }

    let isomorphic_hands = match n_cards {
        5 => load_flop_isomorphic(),
        6 => load_turn_isomorphic(),
        7 => load_river_isomorphic(),
        _ => panic!("Bad number of cards"),
    };

    // Cluster the hands based on E[HS^2] percentile bucketing.
    // Calculate all E[HS^2] values in parallel
    let bar = pbar(isomorphic_hands.len());
    let mut hand_ehs2: Vec<(u64, f64)> = isomorphic_hands
        .par_iter()
        .map(|h| {
            let ehs2 = equity_distribution_moment(*h, 2);
            bar.inc(1);
            (*h, ehs2)
        })
        .collect();
    bar.finish_with_message("Done");

    hand_ehs2.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let sorted_hands: Vec<u64> = hand_ehs2.iter().map(|(hand, _ehs2)| *hand).collect();

    let buffer = File::create(path.as_str()).unwrap();
    bincode::serialize_into(buffer, &sorted_hands).unwrap();
    sorted_hands
}

pub fn get_hand_counts(n_cards: usize) -> HashMap<u64, i32> {
    let path = format!("products/hand_counts_{n_cards}.bin");

    if Path::new(&path).exists() {
        let hand_counts = read_serialized(&path);
        return hand_counts;
    }

    println!("[INFO] Getting {n_cards} hand counts...");
    let deck = deck();
    let mut hand_counts: HashMap<u64, i32> = HashMap::new();
    let bar = pbar(match n_cards {
        5 => 25_989_600,
        6 => 305_377_800,
        7 => 2_809_475_760,
        _ => 0,
    });
    for preflop in deck.iter().combinations(2) {
        let mut rest_of_deck = deck.clone();
        rest_of_deck.retain(|c| !preflop.contains(&c));
        for board in rest_of_deck.iter().combinations(n_cards - 2) {
            let cards = [deepcopy(&preflop), deepcopy(&board)].concat();
            let hand = cards2hand(&isomorphic_hand(&cards));
            let current_count: i32 = match hand_counts.get(&hand) {
                Some(count) => count.clone(),
                None => 0,
            };
            hand_counts.insert(hand, current_count + 1);
            bar.inc(1);
        }
    }
    bar.finish();

    serialize(hand_counts.clone(), path.as_str());
    hand_counts
}

pub fn make_abstraction(n_cards: usize, n_buckets: i32) -> HashMap<u64, i32> {
    match n_cards {
        5 => println!("[INFO] Preparing the flop abstraction."),
        6 => println!("[INFO] Preparing the turn abstraction."),
        7 => println!("[INFO] Preparing the river abstraction."),
        _ => panic!("Bad number of cards"),
    };
    let hand_ehs2 = get_sorted_hand_ehs2(n_cards);
    let hand_counts = get_hand_counts(n_cards);
    let total_hands: u64 = hand_counts.values().map(|n| n.clone() as u64).sum();
    debug_assert!(
        total_hands
            == match n_cards {
                5 => 25_989_600,
                6 => 305_377_800,
                7 => 2_809_475_760,
                _ => 0,
            }
    );
    let mut clusters = HashMap::new();
    let mut sum: u64 = 0;
    let bar = pbar(hand_ehs2.len());
    for hand in hand_ehs2 {
        // Bucket the hand according to the percentile of its E[HS^2]
        let count = hand_counts.get(&hand).unwrap().clone() as u64;
        let bucket: i32 = ((n_buckets as f64) * (sum as f64) / (total_hands as f64)) as i32;
        sum += count;
        clusters.insert(hand, bucket);
        debug_assert!(
            bucket < n_buckets,
            "Hand {} has bucket {} which is outside the range of 0 to {}",
            hand2str(hand),
            bucket,
            n_buckets
        );
        bar.inc(1);
    }
    bar.finish();
    let path = match n_cards {
        5 => FLOP_ABSTRACTION_PATH,
        6 => TURN_ABSTRACTION_PATH,
        7 => RIVER_ABSTRACTION_PATH,
        _ => panic!("Bad hand length"),
    };
    serialize(clusters.clone(), path);
    clusters
}

// Returns the second moment of the hand's equity distribution.
pub fn equity_distribution_moment(hand: u64, moment: i32) -> f64 {
    // For river hands, this just returns HS^2 since there is no distribution
    // Flop and turn, deals rollouts for the E[HS^2] value.
    let hand = hand2cards(hand);
    let mut sum = 0.0;
    let mut count = 0.0;
    let mut deck = deck();
    deck.retain(|c| !hand.contains(c));

    if hand.len() == 7 {
        let equity = river_equity(&hand);
        return equity.powi(2);
    }
    for rollout in deck.iter().combinations(7 - hand.len()) {
        let full_hand = [hand.clone(), deepcopy(&rollout)].concat();
        let equity = river_equity(&full_hand);
        sum += equity.powi(moment);
        count += 1.0;
    }
    sum / count
}

// The equity here is different from the RIVER_EQUITY_CACHE. This is the equity across all possible
// board rollout cards for a given player hole and opponent hole. The RIVER_EQUITY_CACHE is the
// equity across all possible opponent hole cards for a given player hole and board.
pub fn equity_distribution(hand: u64) -> Vec<f32> {
    let hand = hand2cards(hand);
    let board = &hand[2..];
    const BUCKETS: usize = 50;
    let mut equity_hist = vec![0.0; BUCKETS];
    let mut deck = deck();
    deck.retain(|c| !hand.contains(c));
    for remaining_board in deck.iter().combinations(5 - board.len()) {
        let mut rest_of_deck = deck.clone();
        rest_of_deck.retain(|c| !remaining_board.contains(&c));
        rest_of_deck.shuffle(&mut rand::thread_rng());
        let mut n_wins: f64 = 0.0;
        let mut n_runs: f64 = 0.0;
        for opp_preflop in rest_of_deck.iter().combinations(2) {
            let mut opp_hand = Vec::with_capacity(7);
            opp_hand.extend(opp_preflop.clone());
            opp_hand.extend(board);
            opp_hand.extend(remaining_board.clone());

            let mut my_hand = Vec::with_capacity(7);
            my_hand.extend(hand.clone());
            my_hand.extend(remaining_board.clone());

            let my_strength = FAST_HAND_TABLE.hand_strength(&my_hand);
            let opp_strength = FAST_HAND_TABLE.hand_strength(&opp_hand);

            if my_strength > opp_strength {
                n_wins += 1.0;
            } else if my_strength == opp_strength {
                n_wins += 0.5;
            }
            n_runs += 1.0
        }
        let equity = n_wins / n_runs;
        let mut equity_bucket = (equity * BUCKETS as f64).floor() as usize;
        if equity_bucket == BUCKETS {
            equity_bucket = BUCKETS - 1;
        }
        equity_hist[equity_bucket] += 1.0;
    }
    let sum: f64 = equity_hist.iter().sum();
    let normalized: Vec<f32> = equity_hist.iter().map(|e| (e / sum) as f32).collect();
    normalized
}

pub fn river_equity(hand: &[Card]) -> f64 {
    let iso = isomorphic_hand(hand);
    if let Some(equity) = RIVER_EQUITY_CACHE.get(&iso) {
        return equity.clone();
    }

    let mut deck = deck();
    // Remove the already-dealt cards from the deck
    deck.retain(|c| !hand.contains(c));

    let board = hand[2..].to_vec();
    let mut n_wins = 0.0;
    let mut n_runs = 0;

    for opp_preflop in deck.iter().combinations(2) {
        n_runs += 1;

        let my_hand = hand.to_vec();
        let opp_hand = [deepcopy(&opp_preflop), board.clone()].concat();

        let my_strength = FAST_HAND_TABLE.hand_strength(&my_hand);
        let opp_strength = FAST_HAND_TABLE.hand_strength(&opp_hand);

        if my_strength > opp_strength {
            n_wins += 1.0;
        } else if my_strength == opp_strength {
            n_wins += 0.5;
        }
    }
    let equity = n_wins / (n_runs as f64);
    RIVER_EQUITY_CACHE.insert(iso, equity);
    equity
}

pub fn bucket_sizes() {
    let abs = Abstraction::new();
    let mut lens: Vec<i32> = vec![0; CONFIG.flop_buckets as usize];
    for (hand, bucket) in abs.turn {
        lens[bucket as usize] += 1;
    }
    println!("Hands per bucket: {:?}", lens);
    let min = lens.iter().filter(|x| x > &&0).min().unwrap();
    println!("Smallest bucket: {min}");
}

pub fn print_abstraction() {
    let abs = Abstraction::new();
    for bucket in 0..CONFIG.turn_buckets {
        println!("\nBucket {bucket}");
        for sample in 0..10 {
            let mut hands: Vec<&u64> = abs.turn.keys().collect();
            hands.shuffle(&mut rand::thread_rng());
            for hand in hands {
                let b = abs.turn.get(hand).unwrap().clone();
                if b == bucket {
                    println!("{}", hand2str(hand.clone()));
                    break;
                }
            }
        }
    }
}

pub fn create_abstraction_clusters() {
    let dists = get_equity_distributions("flop");
    let buckets = k_means_cluster(dists, CONFIG.flop_buckets, true);
    let hands = load_flop_isomorphic();
    let abstraction: HashMap<u64, i32> = hands
        .iter()
        .zip(buckets.iter())
        .map(|(&hand, &bucket)| (hand, bucket))
        .collect();
    serialize(abstraction, "products/flop_abstraction.bin");

    let dists = get_equity_distributions("turn");
    let buckets = k_means_cluster(dists, CONFIG.turn_buckets, true);
    let hands = load_turn_isomorphic();
    let abstraction: HashMap<u64, i32> = hands
        .iter()
        .zip(buckets.iter())
        .map(|(&hand, &bucket)| (hand, bucket))
        .collect();
    serialize(abstraction, "products/turn_abstraction.bin");

    // let dists = get_ochs_distributions();

    let reader = BufReader::new(File::open("products/ochs_distributions.bin").unwrap());
    let dists = bincode::deserialize_from(reader).unwrap();

    let buckets = k_means_cluster(dists, CONFIG.river_buckets, false);
    let hands = load_river_isomorphic();
    let abstraction: HashMap<u64, i32> = hands
        .iter()
        .zip(buckets.iter())
        .map(|(&hand, &bucket)| (hand, bucket))
        .collect();
    serialize(abstraction, "products/river_abstraction.bin");
}

pub fn expand_abstraction_keys(n_cards: usize) {
    let deck = deck();
    let mut table: HashMap<u64, i32> = HashMap::new();
    let bar = pbar(match n_cards {
        5 => 25989600,
        6 => 305377800,
        _ => panic!(),
    });
    for preflop in deck.iter().combinations(2) {
        let mut sorted_preflop: SmallVecHand = preflop.iter().cloned().cloned().collect();
        sorted_preflop.sort_unstable();
        let mut rest_of_deck = deck.clone();
        rest_of_deck.retain(|c| !preflop.contains(&c));
        for board in rest_of_deck.iter().combinations(n_cards - 2) {
            let mut sorted_board: SmallVecHand = board.iter().cloned().cloned().collect();
            sorted_board.sort_unstable();

            let mut cards = sorted_preflop.clone();
            cards.extend(sorted_board);

            let bin = ABSTRACTION.bin(&cards);
            table.insert(cards2hand(&cards), bin);
            bar.inc(1);
        }
    }
    bar.finish_with_message("Done.");
    let path = match n_cards {
        5 => "products/flop_abstraction_large.bin",
        6 => "products/turn_abstraction_large.bin",
        _ => panic!(),
    };
    serialize(table, path);
}

pub fn get_ochs_distributions() -> Vec<Vec<f32>> {
    let iso = load_river_isomorphic();
    // from https://webdocs.cs.ualberta.ca/~bowling/papers/13aamas-abstraction.pdf Table 1
    let abstract_preflop_clusters: Vec<Vec<&str>> = vec![
        vec![
            "23s", "24s", "25s", "26s", "27s", "32o", "34s", "35s", "36s", "37s", "42o", "43o",
            "45s", "46s", "52o", "53o", "54o", "62o", "63o", "64o", "65o", "72o", "73o", "74o",
            "82o", "83o",
        ],
        vec![
            "82s", "92s", "T2s", "83s", "93s", "74s", "84s", "94s", "75o", "84o", "85o", "92o",
            "93o", "94o", "95o", "T2o", "T3o", "T4o", "T5o", "J2o", "J3o",
        ],
        vec![
            "56s", "57s", "58s", "59s", "5Ts", "3Ts", "34s", "67s", "68s", "69s", "6Ts", "78s",
            "79s", "89s", "76o", "86o", "87o", "96o", "97o", "98o", "T6o", "T7o", "T8o",
        ],
        vec![
            "J2s", "Q2s", "K2s", "J3s", "Q3s", "J4s", "Q4s", "J5s", "Q5s", "J6s", "J4o", "J5o",
            "J6o", "J7o", "Q2o", "Q3o", "Q4o", "Q5o", "Q6o", "Q7o", "K2o", "K3o", "K4o",
        ],
        vec![
            "Q6s", "T7s", "J7s", "Q7s", "T8s", "J8s", "Q8s", "T9s", "J9s", "Q9s", "JTs", "9To",
            "J8o", "J9o", "JTo", "Q8o", "QTo", "Q9o", "QJo", "22o",
        ],
        vec![
            "A2s", "K3s", "A3s", "K4s", "A4s", "K5s", "A5s", "K6s", "K7s", "K8s", "A6s", "33o",
            "44o", "55o", "A2o", "A3o", "A4o", "A5o", "A6o", "A7o", "A8o", "K5o", "K6o", "K7o",
            "K8o", "K9o",
        ],
        vec![
            "66o", "77o", "A7s", "K9s", "A9s", "A8s", "ATs", "AJs", "AQs", "AKs", "KTs", "KJs",
            "KQs", "AKo", "QTs", "QJs", "QKo", "AQo", "KJo", "AJo", "KTo", "ATo", "A9o",
        ],
        vec!["88o", "99o", "TTo", "JJo", "QQo", "KKo", "AAo"],
    ];
    assert_eq!(
        abstract_preflop_clusters
            .iter()
            .map(|cluster| cluster.len())
            .sum::<usize>(),
        169
    );
    // Expand the preflop clusters to incorporate all suits, instead of categories
    let mut expanded_preflop_clusters: Vec<Vec<[Card; 2]>> = Vec::new();
    for cluster in abstract_preflop_clusters.iter() {
        let mut expanded_cluster: Vec<String> = Vec::new();
        for hand in cluster {
            let cards = hand.chars().collect::<Vec<char>>();
            let rank1 = cards[0];
            let rank2 = cards[1];
            let suited = cards.get(2);

            match suited {
                Some(&'s') => {
                    // Correctly iterate for suited hands - 4 combinations
                    for suit in ['c', 'd', 'h', 's'].iter() {
                        expanded_cluster.push(format!("{}{}{}{}", rank1, suit, rank2, suit));
                    }
                }
                _ => {
                    // Correctly iterate for offsuit hands - 12 combinations
                    let suits = ['c', 'd', 'h', 's'];
                    for i in 0..suits.len() {
                        for j in 0..suits.len() {
                            if (i == j) || (rank1 == rank2 && j < i) {
                                continue;
                            }
                            expanded_cluster
                                .push(format!("{}{}{}{}", rank1, suits[i], rank2, suits[j]));
                        }
                    }
                }
            }
        }
        let expanded_cluster_cards: Vec<[Card; 2]> = expanded_cluster
            .iter()
            .map(|hand_str| {
                let cards = str2cards(hand_str);
                [cards[0], cards[1]]
            })
            .collect();
        expanded_preflop_clusters.push(expanded_cluster_cards);
    }
    assert_eq!(
        expanded_preflop_clusters
            .iter()
            .map(|cluster| cluster.len())
            .sum::<usize>(),
        1326
    );

    let bar = pbar(iso.len());
    let ochs_features: Vec<Vec<f32>> = iso
        .par_iter()
        .map(|hand| {
            let hand = hand2cards(*hand);
            let board: SmallVecHand = hand[2..].to_smallvec();
            let equities: Vec<f32> = expanded_preflop_clusters
                .iter()
                .map(|cluster| {
                    // Get the equity of hand against the cluster
                    let mut n_wins = 0.0;
                    for opp_preflop in cluster {
                        if hand.contains(&opp_preflop[0]) || hand.contains(&opp_preflop[1]) {
                            continue;
                        }
                        let opp_hand = [
                            opp_preflop[0],
                            opp_preflop[1],
                            board[0],
                            board[1],
                            board[2],
                            board[3],
                            board[4],
                        ];
                        let opp_strength = FAST_HAND_TABLE.hand_strength(&opp_hand);
                        let my_strength = FAST_HAND_TABLE.hand_strength(&hand);
                        if my_strength > opp_strength {
                            n_wins += 1.0;
                        } else if my_strength == opp_strength {
                            n_wins += 0.5;
                        }
                    }
                    let equity = n_wins / cluster.len() as f32;
                    equity
                })
                .collect();
            bar.inc(1);
            equities
        })
        .collect();
    bar.finish_with_message("Done");

    let file = File::create(format!("products/ochs_distributions.bin")).unwrap();
    let buffer = BufWriter::new(file);
    bincode::serialize_into(buffer, &ochs_features).unwrap();
    ochs_features
}

pub fn get_equity_distributions(street: &str) -> Vec<Vec<f32>> {
    let path = format!("products/{street}_equity_distributions.bin");
    match File::open(path) {
        Err(_error) => {
            println!("[INFO] Computing {street} equity distributions...");
            let iso: Vec<u64> = if street == "flop" {
                load_flop_isomorphic()
            } else {
                load_turn_isomorphic()
            };
            let bar = pbar(iso.len());
            let dists: Vec<Vec<f32>> = iso
                .par_iter()
                .map(|hand| {
                    let dist = equity_distribution(*hand);
                    bar.inc(1);
                    dist
                })
                .collect();
            bar.finish_with_message("Done.");

            let file = File::create(format!("products/{street}_equity_distributions.bin")).unwrap();
            let buffer = BufWriter::new(file);
            bincode::serialize_into(buffer, &dists).unwrap();
            dists
        }
        Ok(file) => {
            let reader = BufReader::new(file);
            bincode::deserialize_from(reader).unwrap()
        }
    }
}

pub fn k_means_cluster(distributions: Vec<Vec<f32>>, k: i32, use_emd: bool) -> Vec<i32> {
    assert!(k > 0 && k < 1_000_000_000);

    // k-means++ initialization
    let mut centers: Vec<Vec<f32>> = Vec::with_capacity(k as usize);
    let first_center = distributions
        .iter()
        .choose(&mut thread_rng())
        .expect("Distributions cannot be empty")
        .to_vec();
    centers.push(first_center);

    println!("Initializing centroids with k-means++");
    let bar = pbar(k as usize);
    for _ in 1..k {
        let subset = distributions
            .choose_multiple(&mut thread_rng(), 10000)
            .cloned()
            .collect::<Vec<Vec<f32>>>();
        let distances: Vec<f32> = subset
            .par_iter()
            .map(|dist| {
                centers
                    .iter()
                    .map(|center| {
                        dist.iter()
                            .zip(center.iter())
                            .map(|(d, c)| (d - c).powi(2))
                            .sum::<f32>()
                            .sqrt()
                    })
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(f32::INFINITY)
            })
            .collect();

        // Use the squared distances to calculate the sum for the probability distribution
        let sum: f32 = distances.iter().map(|d| d.powi(2)).sum();
        let choice = thread_rng().gen_range(0.0..sum);
        let mut cumulative = 0.0;
        for (i, &dist) in distances.iter().enumerate() {
            cumulative += dist;
            if cumulative >= choice {
                centers.push(subset[i].clone());
                break;
            }
        }
        bar.inc(1);
    }
    bar.finish();

    let mut clusters: Vec<i32> = vec![0; distributions.len()];

    let iters = CONFIG.k_means_iters;
    let bar = pbar(iters as usize);
    let mut prev_distance_sum = 0.0;
    for iter in 0..iters {
        let distance_sum: Mutex<f64> = Mutex::new(0.0);
        let iter_bar = pbar(distributions.len());
        clusters = distributions
            .par_iter()
            .map(|x| {
                // Find the closest center to each hand
                let mut closest_center: i32 = 0;
                let mut closest_distance = f32::INFINITY;
                for (i, center) in centers.iter().enumerate() {
                    let distance: f32 = if use_emd {
                        earth_movers_distance(x, center)
                    } else {
                        x.iter()
                            .zip(center.iter())
                            .map(|(i, j)| (i - j).powf(2.0))
                            .sum()
                    };
                    if distance < closest_distance {
                        closest_center = i as i32;
                        closest_distance = distance;
                    }
                }
                let mut d = distance_sum.lock().unwrap();
                *d += closest_distance as f64;
                iter_bar.inc(1);
                closest_center
            })
            .collect();

        iter_bar.finish();
        let distance_sum_val: f64 = *distance_sum.lock().unwrap();
        println!("Iteration {iter}: {}", distance_sum_val);
        if (distance_sum_val - prev_distance_sum).abs() < 1e-6 {
            println!("Converged.");
            break;
        }
        prev_distance_sum = distance_sum_val;

        centers = (0..k)
            .into_par_iter()
            .map(|cluster| {
                // Find the Euclidian mean of each cluster - the new centroid
                let mut sum: Vec<f32> = vec![0.0; distributions[0].len()];
                let mut num_points = 0;
                for i in 0..distributions.len() {
                    if clusters[i] == cluster {
                        num_points += 1;
                        for j in 0..sum.len() {
                            sum[j] += distributions[i][j];
                        }
                    }
                }
                let mean: Vec<f32> = sum.iter().map(|x| x / num_points as f32).collect();
                mean
            })
            .collect();
        bar.inc(1);
    }
    bar.finish();

    clusters
}

fn earth_movers_distance(v1: &Vec<f32>, v2: &Vec<f32>) -> f32 {
    debug_assert!(v1.len() == v2.len());
    let mut cdf1 = 0.0;
    let mut cdf2 = 0.0;
    let mut emd = 0.0;
    for i in 0..v1.len() {
        cdf1 += v1[i];
        cdf2 += v2[i];
        emd += (cdf1 - cdf2).abs();
    }
    emd
}
