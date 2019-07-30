# TODO: k-means++ initialization

from tqdm import trange
import numpy as np


def cluster_with_means(data, means):
    pass

def update_means(data, clusters):
    pass

def cluster(equity_distributions, iterations):
    hand_list = list(equity_distributions.keys())
    distributions = np.array([equity_distributions[h] for h in hand_list])
    breakpoint()

    # Initiailize with k-means++
    for i in trange(iterations):
        clusters = cluster_with_means(data, means)
        means = update_means(data, clusters)
    return