import os
import abc
import json
import pickle
from itertools import combinations, product, permutations
from tqdm import tqdm
import numpy as np

from hand_table import HandTable
from cluster import Cluster
from texas_utils import *


# TODO: Store the computed abstractions with the parameters so that it will be
# recomputed if the parameters are different.
# TODO: Take the paramater file as a command line argument to the trainer class
PARAM_FILE = 'params.json'
FLOP_SAVE_NAME = 'texas_flop_abstraction.pkl'
ARCHETYPAL_FLOP_FILENAME = 'flop_hands.pkl'
FLOP_EQUITY_DISTIBUTIONS = 'flop_equity.pkl'
HAND_TABLE = HandTable()


def print_abstraction():
    params = json.load(open(PARAM_FILE, 'r'))
    print(PreflopAbstraction())
    print(FlopAbstraction(**params['flop']))
    print(TurnAbstraction())
    print(RiverAbstraction())


def unique_cards(cards):
    """Returns True if there are no repeated cards in the given list.

    Input:
        cards - tuple/list of cards in the standard 'Ad' format
    """
    return len(np.unique(cards)) == len(cards)


def archetypal_flop_hands():
    """Returns a list of all archetypal flop hands.

    An archetypal hand is a hand with unnecessary order and suit information
    removed. For example, it doesn't matter what order the flop cards are in, so
    we can sort the flop cards without losing information. Same with the preflop.
    Also, we can only consider one suit isomorphism out of the many possible
    isomorphisms. For example, a flush of hearts is functionally the same as a
    flush of diamonds. The returned hand will be sorted by the preflop and flop
    and be run through the suit isomorphism algorithm. Using these techniques
    greatly reduces the size of the flop abstraction lookup table.
    """
    if os.path.isfile(ARCHETYPAL_FLOP_FILENAME):
        return pickle.load(open(ARCHETYPAL_FLOP_FILENAME, 'rb'))

    print('Finding the representative flop hands...')
    hands = []
    deck = get_deck()
    used_hands = {}
    with tqdm(total=29304600, smoothing=0) as t:
        for preflop, flop in product(combinations(deck, 2), combinations(deck, 3)):
            hand = preflop + flop
            if unique_cards(hand):
                hand = archetypal_hand(hand)
                if hand not in used_hands:
                    used_hands[hand] = True
            t.update()
    hands = list(used_hands.keys())
    pickle.dump(hands, open(ARCHETYPAL_FLOP_FILENAME, 'wb'))
    return hands


def get_equity_distribution(preflop, flop=None, turn=None, equity_bins=50, opponent_samples=50,
                            rollout_samples=50):
    """Returns an estimate of the equity distribution for the given hand.

    An equity distribution is a histogram that looks like this:

        |
      # |
      h |
      a |
      n |
      d |
      s |
        |______________________
         0      % equity     1

    Effectively, this algorithm finds out how many opponent hands give it a given
    amount of equity for various levels of equity from 0 to 1. This distributon
    can then be used to compare hand similarity, which is a more effective
    measurement than expectation because hands like 6c6d have similar expected
    equity to QsJs, but have very different equity distributions.

    Inputs:
        preflop - The player's preflop holdings
        flop - Flop cards on the board
        turn - Turn card
        equity_bins - Number of bins the histogram should use. Effectively this
            represents the number of discrete equity levels that values get
            rounded to.
        opponent_samples - How many random opponent holdings to deal.
        rollout_samples - How many times to run the rollouts (deal the rest of
            the hand) for each opponent sample

    Returns:
        equity_distribution - Numpy array containing how many opponent hands
            give the player's hand each amount of equity.
    """
    hand = preflop
    remaining_cards = 5
    if flop is not None:
        hand += flop
        remaining_cards = 2
    if turn is not None:
        hand += turn
        remaining_cards = 1
    deck = get_deck()
    for card in hand:
        deck.remove(card)

    equity_distribution = np.zeros(equity_bins)
    preflops = list(combinations(deck, 2))
    # TODO: Is it possible to make a generator for isomorphic rollouts, greatly
    # reducing the number of calls to isomorphic_hand()?
    for preflop_index in np.random.choice(range(len(preflops)), opponent_samples, replace=False):
        # Calculate the equity of this hand against the opponent_hand
        n_wins = 0
        n_games = 0
        opponent_preflop = preflops[preflop_index]
        all_remaining = list(permutations(deck, remaining_cards))
        while n_games < rollout_samples:
            remaining_index = np.random.randint(len(all_remaining))
            remaining = all_remaining[remaining_index]
            if opponent_preflop[0] in remaining or opponent_preflop[1] in remaining:
                continue

            river = remaining[-1]
            turn = remaining[-2]
            player_hand = HAND_TABLE[hand + remaining]
            opponent_hand = HAND_TABLE[opponent_preflop + flop + (turn, river)]
            if player_hand > opponent_hand:
                n_wins += 1
            elif player_hand == opponent_hand:
                n_wins += 0.5
            n_games += 1

        equity = n_wins / n_games
        bucket = int(equity // (1 / equity_bins))
        equity_distribution[bucket] += 1

    equity_distribution /= np.sum(equity_distribution)
    return equity_distribution


def abstraction2str(table):
    """Returns a string represntation of the given abstraction dictionary.

    The string lists each bucket in order with the hands each bucket contains.

    Inputs:
        table - Dictionary mapping hands to bucket indices.

    Returns
        result - String representation of the abstraction table.
    """
    result = ''
    for bucket in tqdm(sorted(table.values())):
        result += str(bucket) + ': '
        for hand in table:
            if table[hand] == bucket:
                result += str(tuple(hand)) + ' '
        result += '\n'
    return result


class CardAbstraction(abc.ABC):
    """Abstract base class for preflop, flop, turn, and river card abstractions.

    These classes handle the mapping of hands to integer bucket ID numbers. This
    can get too dicey for a dict alone because the order of cards matters sometimes.
    For example, if you have pocket aces, that's very different from there being
    two aces on the board. However, the order of the flop doesn't matter.
    """
    @abc.abstractmethod
    def __getitem__(self, cards):
        pass

    @abc.abstractmethod
    def __str__(self):
        pass


class PreflopAbstraction(CardAbstraction):
    """For the preflop, just use the 169 unique two-card combos. This is essentially
    a hash function for logically unique preflop hands.
    """
    def __init__(self):
        self.table = {}
        self.compute_abstraction()

    def compute_abstraction(self):
        """Make a unique index for each logically different preflop hand."""
        for hand in combinations(get_deck(), 2):
            hand = sorted(hand)
            # -2 maps from 2-14 to 0-12. This is kind of like a hash function that
            # gives a unique integer for every logically unique preflop hand.
            first_card = RANKS[hand[0][0]] - 2
            second_card = RANKS[hand[1][0]] - 2
            suited = hand[0][1] == hand[1][1]
            index = 2 * (first_card * len(RANKS) + second_card)
            if suited:
                index += 1
            self.table[frozenset(hand)] = index

    def __getitem__(self, cards):
        return self.table[frozenset(cards)]

    def __str__(self):
        """Prints the groupings of hands together."""
        return abstraction2str(self.table)


class FlopAbstraction(CardAbstraction):
    """Finds similar flop hands and groups them together.

    Similarity is based on the Earth Movers Distance of the hands' equity
    distributions, and clustering is performed using k_means clustering.
    """
    def __init__(self, buckets=5000, equity_bins=50, iters=20,
                 opponent_samples=100, rollout_samples=100):
        self.buckets = buckets
        self.equity_bins = equity_bins
        self.iters = iters
        self.opponent_samples = opponent_samples
        self.rollout_samples = rollout_samples
        self.table = self.compute_abstraction()

    def compute_abstraction(self):
        """Clusters all possible flop hands into groups."""
        if os.path.isfile(FLOP_SAVE_NAME):
            return pickle.load(open(FLOP_SAVE_NAME, 'rb'))

        print('Computing the flop abstraction...')
        if os.path.isfile(FLOP_EQUITY_DISTIBUTIONS):
            equity_distributions = pickle.load(open(FLOP_EQUITY_DISTIBUTIONS, 'rb'))
        else:
            print('Calculating equity distributions...')
            hands = archetypal_flop_hands()
            distributions = pbar_map(self.hand_equity, hands)
            equity_distributions = dict(zip(hands, distributions))
            pickle.dump(equity_distributions, open(FLOP_EQUITY_DISTIBUTIONS, 'wb'))

        print('Performing k-means clustering...')
        # TODO: Make the inputs be to the actual class?
        abstraction = Cluster(equity_distributions, self.iters, self.buckets)()
        pickle.dump(abstraction, open(FLOP_SAVE_NAME, 'wb'))
        return abstraction

    def hand_equity(self, hand):
        """Returns the equity distribution for the given flop hand.

        Inputs:
            hand - list of five cards with the first two cards being the preflop and
                the last three being the flop.

        Returns:
            distribution - Estimate of the equity distribution (histogram) over
                all possible opponent holdings and rollouts.

        """
        preflop = hand[:2]
        flop = hand[2:]
        # TODO: Add a paramater in the paramater file for number of samples.
        distribution = get_equity_distribution(preflop, flop,
                                               equity_bins=self.equity_bins,
                                               opponent_samples=self.opponent_samples,
                                               rollout_samples=self.rollout_samples)
        return distribution

    def __getitem__(self, cards):
        return self.table[isomorphic_hand(cards)]

    def __str__(self):
        return str(self.table)


if __name__ == '__main__':
    print_abstraction()
