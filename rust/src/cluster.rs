use std::collections::HashMap;
use rand::thread_rng;
use rand::prelude::SliceRandom;
use ndarray::Array;
use crate::card_utils;

const ITERATIONS: i32 = 10;

// Cluster the data using k-means clustering with the Earth Mover's Distance.
// Returns a HashMap mapping hand strings to their abstraction bucket ID number.
pub fn cluster(distributions: &HashMap<String, Vec<f64>>, bins: i32) -> HashMap<String, i32> {
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
