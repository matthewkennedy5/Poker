use std::collections::HashMap;
use rand::thread_rng;
use rand::prelude::SliceRandom;
use ndarray::Array;
use crate::card_utils;

// Cluster hands based on the second moment of the equity distribution, aka E[HS^2].
// This approach is much faster but becomes inferior for larger abstractions.
// However, it may be sufficient when using depth-limited solving. This uses
// percentile bucketing.
pub fn cluster_ehs2(distributions: &HashMap<u64, Vec<f64>>, bins: i32) -> HashMap<u64, i32> {
    let mut ehs2: Vec<(String, f64)> = second_moments(distributions);
    // a.1 and b.1 are the ehs2 of the tuple, each tuple contains (hand, ehs2).
    ehs2.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let mut clusters = HashMap::new();
    for (idx, (hand, val)) in ehs2.iter().enumerate() {
        let bucket: i32 = ((bins as f64) * (idx as f64) / (ehs2.len() as f64)) as i32;
        clusters.insert(hand.clone(), bucket);
    }
    clusters
}

// Assumes all distributions are normalized to 1
fn second_moments(distributions: &HashMap<u64, Vec<f64>>) -> Vec<(u64, f64)> {
    let mut ehs2 = Vec::new();
    for (hand, dist) in distributions {
        let squared: Vec<f64> = dist.iter().map(|x| x.powi(2)).collect();
        let sum: f64 = squared.iter().sum();
        let mean = sum / squared.len() as f64;
        ehs2.push((hand.clone(), mean));
    }
    ehs2
}
