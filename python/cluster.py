from tqdm import trange, tqdm
from matplotlib import pyplot as plt
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
        self.loss_history = []

    def __call__(self):
        # TODO: Initiailize with k-means++
        # TODO: Triangle inequality speedup

        # Sample K initial means initialized to be existing points.
        means = np.zeros((self.n_buckets, self.data.shape[1]))
        for i in range(means.shape[0]):
            random_hand = np.random.randint(self.data.shape[0])
            means[i, :] = self.data[random_hand, :]

        # prev_clusters = None
        # i = 0
        # while True:
        #     print(i)
        #     i += 1
        for i in range(self.iterations):
            print('Iteration {}/{}'.format(i+1, self.iterations))
            clusters = self.cluster_with_means(means)
            # if clusters == prev_clusters:
                # break
            # prev_clusters = clusters
            means = self.update_means(clusters)

        abstraction = {}
        for idx, cluster in enumerate(clusters):
            for hand in cluster:
                hand_string = self.hand_list[hand]
                abstraction[hand_string] = idx

        return abstraction

    def init_means(self):
        # Uses the k-means++ algorithm to initialize the means
        print('Initializing means using k-means++...')
        initial_means = []
        n_hands = self.data.shape[0]
        initial_means.append(np.random.randint(n_hands))
        for i in range(1, self.n_buckets):
            squared_distances = np.zeros(n_hands)
            for j in trange(n_hands):
                min_distance = min([stats.wasserstein_distance(self.data[mean], self.data[j]) for mean in initial_means])
                squared_distances[j] = min_distance ** 2
            pdf = squared_distances / np.sum(squared_distances)
            new_mean = np.random.choice(range(n_hands), p=pdf)
            initial_means.append(new_mean)
        means = np.zeros((self.n_buckets, self.data.shape[1]))
        breakpoint()
        raise NotImplementedError

    def earth_movers_distance(self, mean):
        result = []
        for hand in self.data:
            # result.append(stats.wasserstein_distance(hand, mean))
            # TODO: Do k-means++ get EMD to work
            result.append(np.linalg.norm(hand - mean))
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
        distances = np.array(distances).T

        # Update the loss history with the sum of the squared distances of each
        # data point to its nearest mean
        distances[np.isnan(distances)] = np.Inf
        loss = np.sum(np.min(distances, axis=1) ** 2)
        self.loss_history.append(loss)

        nearest_means = np.argmin(distances, axis=1)
        for hand_idx, nearest_mean in enumerate(nearest_means):
            clusters[nearest_mean].append(hand_idx)
        return clusters

    # TODO: Handle degenerate clusters with 0 points
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

    def plot_loss(self):
        plt.figure()
        plt.plot(range(len(self.loss_history)), self.loss_history)
        plt.show()
