import os
import pdb
from matplotlib import pyplot as plt
import numpy as np
from scipy import stats
from tqdm import tqdm, trange
import pickle
import pprint
from rhode_utils import *

N_PREFLOP_BUCKETS = 13
N_FLOP_BUCKETS = 10
N_TURN_BUCKETS = 50
EQUITY_SAVE_NAME = 'rhode_flop_equities.pkl'
FLOP_SAVE_NAME = 'rhode_flop_abstraction.pkl'
TURN_SAVE_NAME = 'rhode_turn_abstraction.pkl'
N_EQUITY_BINS = 50  # How many bins to divide the equity histogram into
ITERATIONS = 100    # How many iterations of k-means clustering to run

# TODO: Run k-means clustering on the Earth Mover's Distances.

# A straightforward preflop abstraction is to treat all suits the same, leading
# to only 13 distinct preflop hands.


def deal_2_cards():
    """Generator that yields all n_cards permutations from the deck.

    Inputs:
        n_cards - How many cards to deal. 0 <= min_cards <= 52

    Yields:
        cards: Tuple of len n_cards containing the dealt cards for that permutation
            of the deck.
    """
    for card1 in get_deck():
        for card2 in get_deck():
            if card1 == card2:
                continue
            yield card1, card2


def average_equity(hole, flop):
    """Returns the equity of the given hand averaged over all opponent hands."""
    equity_sum = 0
    for opponent_hole in get_deck():
        if opponent_hole not in (hole, flop):
            equity_sum += headsup_equity(hole, opponent_hole, flop)
    return equity_sum / 50    # 52 cards minus hole and flop


def headsup_equity(hole, opponent_hole, flop):
    """Samples every possible turn card and calculates the equity.

    All cards are input in 'Ts' string format.

    Inputs:
        hole - Player's hole card
        opponent_hole - Opponent's hole card
        flop - Flop card

    Returns:
        equity - Chance of winning + 1/2 * chance of tying.
    """
    n_wins = 0
    for turn in get_deck():
        if turn not in (hole, opponent_hole, flop):
            player_hand = RhodeHand(hole, flop, turn)
            opponent_hand = RhodeHand(opponent_hole, flop, turn)
            if player_hand > opponent_hand:
                n_wins += 1
            elif player_hand == opponent_hand:
                n_wins += 0.5
    return n_wins / 49      # 49 = 52 - 3 (52 cards minus hole, opponent_hole, flop)


def equity_distribution(hole, flop, bins=N_EQUITY_BINS):
    """Returns a histogram of the equity across all possible opponent hands.

    Inputs:
        hole - Player's hole card
        flop - Flop card
        bins - Number of discrete buckets for the histogram.

    Returns:
        equity_distribution - A numpy array containing the number of times the
            given equity was hit for all discretized equity values in the range
            [0, 1].
    """
    equity_distribution = np.zeros(bins)
    for opponent_hole in get_deck():
        if opponent_hole not in (hole, flop):
            equity = headsup_equity(hole, opponent_hole, flop)
            bucket = int(equity // (1 / bins))
            equity_distribution[bucket] += 1
    return equity_distribution


def show_equity_distributions():
    """Plots equity distributions for all possible hands."""

    shuffled_deck = get_deck()
    np.random.shuffle(shuffled_deck)
    for hole in shuffled_deck:
        for flop in shuffled_deck:
            if flop == hole:
                continue
            distro = equity_distribution(hole, flop)
            plt.figure()
            x_range = np.linspace(0, 1, num=distro.shape[0], endpoint=False)
            plt.bar(x_range, distro, width=x_range[1])
            plt.title('Hole: %s, Flop: %s' % (hole, flop))
            plt.ylabel('Probability')
            plt.xlabel('Equity')
            plt.show()


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


def get_equity_distributions():
    """Returns the equity distribution for every hole/flop combination."""
    if not os.path.isfile(EQUITY_SAVE_NAME):
        print('[INFO] Calculating equity distributions...')
        equities = {}
        with tqdm(range(52 * 51)) as t:
            for hole, flop in deal_2_cards():
                equities[hole+flop] = equity_distribution(hole, flop)
                t.update()
        pickle.dump(equities, open(EQUITY_SAVE_NAME, 'wb'))

    equities = pickle.load(open(EQUITY_SAVE_NAME, 'rb'))
    return equities


def cluster_with_means(matrix, means):
    """Groups every item in the matrix by assigning it to its nearest mean.

    Inputs:
        matrix - Array containing the equity distributions for each hand.
        means - Previous centroids to use to update the assignments of hands to
            clusters.

    Returns:
        clusters - Groupings of hands based on nearest (EMD) mean.
    """
    clusters = [[] for mean in means]
    for hand in range(matrix.shape[0]):
        nearest_mean = 0
        nearest_distance = np.Inf
        for j, mean in enumerate(means):
            # Compute the Earth Movers Distance (aka Wasserstein Distance)
            # TODO: Get EMD to work by using the k-means++ initialization.
            # distance = stats.wasserstein_distance(matrix[hand, :], mean)
            # Using L2 distance for now because EMD isn't clustering properly.
            distance = np.linalg.norm(matrix[hand, :] - mean)
            if distance < nearest_distance:
                nearest_mean = j
                nearest_distance = distance
        clusters[nearest_mean].append(hand)
    return clusters


# TODO: How to find centroid using EMD?
def update_means(matrix, clusters):
    """Returns the centroids of the given clusters.

    This finds the centroid in Euclidian space.

    Inputs:
        matrix - Numpy array containing the equity distributions for each hand.
        clusters - Groupings of hands based on proximity to the former means.

    Returns:
        means - The updated centroids of each cluster in Euclidian space.
    """
    means = np.zeros((len(clusters), N_EQUITY_BINS))
    for i, cluster in enumerate(clusters):
        means[i, :] = np.mean(matrix[cluster], axis=0)
    return means


def flop_abstraction():
    """Finds similar flop hands and groups them together.

    Similarity is based on the Earth Movers Distance of the hands' equity
    distributions, and clustering is performed using k_means clustering.

    Returns:
        abstraction -
    """
    if os.path.isfile(FLOP_SAVE_NAME):
        return pickle.load(open(FLOP_SAVE_NAME, 'rb'))

    print('[INFO] Computing the flop abstraction...')
    hands = get_equity_distributions()

    # Create a matrix of points of shape (n_hands, n_bins)
    matrix = np.zeros((len(hands), N_EQUITY_BINS))
    hands_list = []     # For storing the order of the hands in the matrix
    for i, hand in enumerate(hands):
        matrix[i, :] = hands[hand]
        hands_list.append(hand)

    # Sample K initial means initialized to be existing points.
    means = np.zeros((N_FLOP_BUCKETS, N_EQUITY_BINS))
    for i in range(means.shape[0]):
        random_hand = np.random.randint(matrix.shape[0])
        means[i, :] = matrix[random_hand, :]

    for i in trange(ITERATIONS):
        clusters = cluster_with_means(matrix, means)
        means = update_means(matrix, clusters)

    abstraction = {}
    for idx, cluster in enumerate(clusters):
        for hand in cluster:
            hand_string = hands_list[hand]
            abstraction[hand_string] = idx + N_PREFLOP_BUCKETS

    pickle.dump(abstraction, open(FLOP_SAVE_NAME, 'wb'))
    return abstraction


def preflop_abstraction():
    """Returns the abstraction used for the preflop.

    The only abstraction used here is just grouping all ranks together; that is,
    it drops the suit information.

    Returns:
        abstraction - A dictionary mapping preflop hands to their abstact bucket
            index.
    """
    abstraction = {}
    for card in get_deck():
        abstraction[card] = Card(card).rank - 2     # Puts 2 in the 0th bucket
    return abstraction


def turn_equity(hole, flop, turn):
    """Returns the player's chance of winning over all possible opponent hands.

    This assumes an even distribution of opponent hands, and calculates the
    expected equity.

    Inputs:
        hole - Player's hole card
        flop - Flop card
        turn - Turn card

    Returns:
        equity - Expected equity of this hand.
    """
    equity = 0
    for opponent_card in get_deck():
        if opponent_card not in (hole, flop, turn):
            player_hand = RhodeHand(hole, flop, turn)
            opponent_hand = RhodeHand(opponent_card, flop, turn)
            if player_hand > opponent_hand:
                equity += 1
            elif player_hand == opponent_hand:
                equity += 0.5
    return equity / 49      # There are 49 (52 - 3) possible opponent cards


# TODO: Are these abstractions good? Evaluate abstraction quality.
def turn_abstraction():
    """Returns the abstraction dictionary used for the turn.

    Here, we only need to cluster the equity (expected hand strength), since
    there are no equity distributions.
    """
    if os.path.isfile(TURN_SAVE_NAME):
        return pickle.load(open(TURN_SAVE_NAME, 'rb'))

    print('[INFO] Computing turn abstractions...')
    # This finds the largest gaps between equity values and segments based on
    # that. TODO: If this is not powerful enough, try Kernel Density Estimation (KDE).
    equities = []
    hand_list = []   # To keep track of which hand corresponds to which equity
    with tqdm(range(52 * 51 * 50)) as t:
        for hole in get_deck():         # Iterates over all possible hole/flop/turn combos
            for flop in get_deck():
                if hole == flop:
                    continue
                for turn in get_deck():
                    if turn in (hole, flop):
                        continue

                    equity = turn_equity(hole, flop, turn)
                    equities.append(equity)
                    hand_list.append(hole + flop + turn)
                    t.update()

    equities = np.array(equities)
    hand_list = np.array(hand_list)
    sort_indices = np.argsort(equities)
    equities = equities[sort_indices]
    hand_list = hand_list[sort_indices]
    differences = np.diff(equities)
    largest_diff_locations = np.argsort(differences)[-N_TURN_BUCKETS+1:]

    abstraction = {}
    prev_idx = 0
    for label, d in enumerate(sorted(largest_diff_locations)):
        for hand in hand_list[prev_idx:d]:
            abstraction[hand] = label
        prev_idx = d
    label += 1
    for hand in hand_list[prev_idx:]:
        abstraction[hand] = label

    # Offset the label so it doesn't clash with the preflop or flop
    for a in abstraction:
        abstraction[a] += N_FLOP_BUCKETS + N_PREFLOP_BUCKETS

    pickle.dump(abstraction, open(TURN_SAVE_NAME, 'wb'))
    return abstraction


def prepare_abstraction():
    """Returns a dict containinig all hand abstractions for Rhode Island Holdem."""
    preflop = preflop_abstraction()
    flop = flop_abstraction()
    turn = turn_abstraction()
    return {**preflop, **flop, **turn}


def print_abstraction():
    abst = prepare_abstraction()
    bins = [[] for i in range(N_PREFLOP_BUCKETS + N_FLOP_BUCKETS + N_TURN_BUCKETS)]
    for a in abst:
        bins[abst[a]].append(a)
    print('preflop:')
    for i, b in enumerate(bins):
        if i == N_PREFLOP_BUCKETS:
            print('flop')
        elif i == N_FLOP_BUCKETS + N_PREFLOP_BUCKETS:
            print('turn')
        print('Bin', i, b)



if __name__ == '__main__':

    print_abstraction()

    # abstraction = compute_abstractions()
    # bins = [[] for i in range(N_BUCKETS)]
    # for hand in abstraction:
    #     bins[abstraction[hand]].append(hand)
    # pprint.pprint(bins)
    # # show_equity_distributions()
