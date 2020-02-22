use std::collections::HashMap;
use rand::thread_rng;
use rand::prelude::SliceRandom;
use ndarray::Array;
use crate::card_utils;

const ITERATIONS: i32 = 10;

// Cluster hands based on the second moment of the equity distribution, aka E[HS^2].
// This approach is much faster but becomes inferior for larger abstractions.
// However, it may be sufficient when using depth-limited solving. This uses
// percentile bucketing.
pub fn cluster_ehs2(distributions: &HashMap<String, Vec<f64>>, bins: i32) -> HashMap<String, i32> {
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
fn second_moments(distributions: &HashMap<String, Vec<f64>>) -> Vec<(String, f64)> {
    let mut ehs2 = Vec::new();
    for (hand, dist) in distributions {
        let squared: Vec<f64> = dist.iter().map(|x| x.powi(2)).collect();
        let sum: f64 = squared.iter().sum();
        let mean = sum / squared.len() as f64;
        ehs2.push((hand.clone(), mean));
    }
    ehs2
}

// Cluster the data using k-means clustering with the Earth Mover's Distance.
// Returns a HashMap mapping hand strings to their abstraction bucket ID number.
pub fn cluster_emd(distributions: &HashMap<String, Vec<f64>>, bins: i32) -> HashMap<String, i32> {
    // clusters maps hands to their abstraction bucket ID number
    let mut clusters: HashMap<String, i32> = HashMap::new();
    let mut means = init_means(&distributions.values().collect(), bins);
    for i in 0..ITERATIONS {
        clusters = assign_clusters(&distributions, &means);
        means = update_means(&clusters);
    }
    clusters
}

fn init_means(data: &Vec<&Vec<f64>>, bins: i32) -> Vec<Vec<f64>> {
    let mut means: Vec<Vec<f64>> = Vec::new();
    let mut rng = &mut thread_rng();
    let first_mean = data.choose(&mut rng).unwrap();
    means.push(first_mean.to_vec());
    while (means.len() as i32) < bins{
        let prev_mean = &means[means.len()-1];
        // Compute the squared Earth Mover's Distances for the probability distribution
        let squared_distances = c![d.powi(2), for d in distances(prev_mean, &data)];
        let probs = card_utils::normalize(&squared_distances);
        let indices: Vec<usize> = (0..probs.len()).collect();
        let new_mean = *indices.choose_weighted(&mut rng, |x| probs[*x]).unwrap();
        means.push(data[new_mean].to_vec());
    }
    means
}

// TODO if too slow, try changing Vec<Vec<.. to ndarray for better locality
fn assign_clusters(distributions: &HashMap<String, Vec<f64>>, means: &Vec<Vec<f64>>) -> HashMap<String, i32> {
    HashMap::new()
}

fn update_means(clusters: &HashMap<String, i32>) -> Vec<Vec<f64>> {
    Vec::new()
}

// Compute Earth Mover's Distances between the start point and all other points
fn distances(start: &Vec<f64>, points: &Vec<&Vec<f64>>) -> Vec<f64> {
    let mut result = Vec::new();
    let start = Array::from(start.clone());
    for p in points {
        let point = Array::from(p.clone().to_vec());
        let distance = emd::distance(&start.clone().view(), &point.view());
        result.push(distance);
    }
    result
}
