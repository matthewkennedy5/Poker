# TODO: k-means++ initialization

from tqdm import trange, tqdm
import numpy as np
from scipy import stats
import multiprocessing as mp
from texas_utils import pbar_map


class Cluster:

    def __init__(self, equity_distributions, n_buckets, iterations):
        self.hand_list = list(equity_distributions.keys())
        distributions = np.array([equity_distributions[h] for h in self.hand_list])
        self.data = distributions
        self.n_buckets = n_buckets
        self.iterations = iterations

    def __call__(self):
        # TODO: Initiailize with k-means++

        # Sample K initial means initialized to be existing points.
        means = np.zeros((self.n_buckets, self.data.shape[1]))
        for i in range(means.shape[0]):
            random_hand = np.random.randint(self.data.shape[0])
            means[i, :] = self.data[random_hand, :]

        prev_clusters = None
        i = 0
        while True:
            print(i)
            i += 1
            clusters = self.cluster_with_means(means)
            if clusters == prev_clusters:
                break
            prev_clusters = clusters
            means = self.update_means(clusters)

        abstraction = {}
        for idx, cluster in enumerate(clusters):
            for hand in cluster:
                hand_string = self.hand_list[hand]
                abstraction[hand_string] = idx

        return abstraction

    def earth_movers_distance(self, mean):
        result = []
        for hand in self.data:
            result.append(stats.wasserstein_distance(hand, mean))
        return result

    def cluster_with_means(self, means):
        """Groups every item in the data by assigning it to its nearest mean.

        Inputs:
            means - Previous centroids to use to update the assignments of hands to
                clusters.

        Returns:
            clusters - Groupings of hands based on nearest (EMD) mean.
        """
        clusters = [[] for mean in means]
        # Precompute all Earth Mover's Distances since that's the bottleneck
        with mp.Pool(mp.cpu_count()) as p:
            distances = pbar_map(self.earth_movers_distance, means)
        # TODO: Test this with the old code to make sure it's the same
        distances = np.array(distances).T
        nearest_means = np.argmin(distances, axis=1)
        for hand_idx, nearest_mean in enumerate(nearest_means):
            clusters[nearest_mean].append(hand_idx)
        return clusters


    def update_means(self, clusters):
        """Returns the centroids of the given clusters.

        This finds the centroid in Euclidian space.

        Inputs:
            clusters - Groupings of hands based on proximity to the former means.

        Returns:
            means - The updated centroids of each cluster in Euclidian space.
        """
        means = np.zeros((len(clusters), self.data.shape[1]))
        for i, cluster in enumerate(clusters):
            means[i, :] = np.mean(self.data[cluster, :], axis=0)
        return means
