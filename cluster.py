# TODO: k-means++ initialization

from tqdm import trange
import numpy as np
from scipy import stats


def earth_movers_distance(hole1, flop1, hole2, flop2):
    """Finds the Earth Mover's Distance (EMD) between the hands.

    We are comparing distinct card situations at the flop only.

    Inputs:
        hole1 - Hole card for hand 1
        flop1 - Flop card for hand 1
        hole2 -
        flop2

    Returns:
        distance - The Earth Mover's Distance between the hand states.
    """
    equity1 = equity_distribution(hole1, flop1)
    equity2 = equity_distribution(hole2, flop2)
    # Take the EMD of the equity distributions
    distance = stats.wasserstein_distance(equity1, equity2)
    return distance


def cluster_with_means(data, means):
    """Groups every item in the data by assigning it to its nearest mean.

    Inputs:
        data - Array containing the equity distributions for each hand.
        means - Previous centroids to use to update the assignments of hands to
            clusters.

    Returns:
        clusters - Groupings of hands based on nearest (EMD) mean.
    """
    clusters = [[] for mean in means]
    for hand in range(data.shape[0]):
        nearest_mean = 0
        nearest_distance = np.Inf
        for j, mean in enumerate(means):
            # Compute the Earth Movers Distance (aka Wasserstein Distance)
            distance = stats.wasserstein_distance(data[hand, :], mean)
            # Using L2 distance for now because EMD isn't clustering properly.
            # distance = np.linalg.norm(data[hand, :] - mean)
            if distance < nearest_distance:
                nearest_mean = j
                nearest_distance = distance
        clusters[nearest_mean].append(hand)
    return clusters


# TODO: How to find centroid using EMD?
def update_means(data, clusters):
    """Returns the centroids of the given clusters.

    This finds the centroid in Euclidian space.

    Inputs:
        data - Numpy array containing the equity distributions for each hand.
        clusters - Groupings of hands based on proximity to the former means.

    Returns:
        means - The updated centroids of each cluster in Euclidian space.
    """
    means = np.zeros((len(clusters), N_EQUITY_BINS))
    for i, cluster in enumerate(clusters):
        means[i, :] = np.mean(data[cluster], axis=0)
    return means


def cluster(equity_distributions, iterations, n_buckets):
    hand_list = list(equity_distributions.keys())
    distributions = np.array([equity_distributions[h] for h in hand_list])

    # TODO: Initiailize with k-means++

    # Sample K initial means initialized to be existing points.
    means = np.zeros((n_buckets, distributions.shape[1]))
    for i in range(means.shape[0]):
        random_hand = np.random.randint(distributions.shape[0])
        means[i, :] = distributions[random_hand, :]

    for i in trange(iterations):
        clusters = cluster_with_means(distributions, means)
        means = update_means(distributions, clusters)

    abstraction = {}
    for idx, cluster in enumerate(clusters):
        for hand in cluster:
            hand_string = hands_list[hand]
            abstraction[hand_string] = idx
