import abc
import os
import numpy
from itertools import combinations, product, permutations
from texas_utils import *
from tqdm import tqdm
import numpy as np
from hand_table import HandTable
import multiprocessing as mp
import pickle
from cluster import Cluster


# TODO: Make a parameter file
FLOP_SAVE_NAME = 'texas_flop_abstraction.pkl'
ARCHETYPAL_FLOP_FILENAME = 'flop_hands.pkl'
FLOP_EQUITY_DISTIBUTIONS = 'flop_equity.pkl'
N_EQUITY_BINS = 20
K_MEANS_ITERS = 2
N_FLOP_BUCKETS = 2
N_TURN_BUCKETS = 30
N_RIVER_BUCKETS = 40
HAND_TABLE = HandTable()


def print_abstraction():
    print(PreflopAbstraction())
    print(FlopAbstraction())
    print(TurnAbstraction())
    print(RiverAbstraction())


# TODO: Is this semantically wrong?
def duplicate_cards(cards):
    """Returns True if cards are repeated.

    Input:
        cards - tuple/list of cards in the standard 'Ad' format
    """
    return len(np.unique(cards)) == len(cards)


def archetypal_flop_hands():
    if os.path.isfile(ARCHETYPAL_FLOP_FILENAME):
        return pickle.load(open(ARCHETYPAL_FLOP_FILENAME, 'rb'))

    print('Finding the representative flop hands...')
    hands = []
    deck = get_deck()
    used_hands = {}
    with tqdm(total=29304600, smoothing=0) as t:
        for preflop, flop in product(combinations(deck, 2), combinations(deck, 3)):
            hand = preflop + flop
            if len(np.unique(hand)) == len(hand):
                hand = archetypal_hand(hand)
                if hand not in used_hands:
                    used_hands[hand] = True
            t.update()
    hands = list(used_hands.keys())
    pickle.dump(hands, open(ARCHETYPAL_FLOP_FILENAME, 'wb'))
    return hands


def turn_gen():
    pass


def river_gen():
    pass


def get_equity_distribution(preflop, flop=None, turn=None, opponent_samples=50,
                                                           rollout_samples=50):
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

    equity_distribution = np.zeros(N_EQUITY_BINS)
    preflops = list(combinations(deck, 2))
    # TODO: Is it possible to make a generator for isomorphic rollouts, greatly
    # reducing the number of calls to isomorphic_hand()?
    for preflop_index in np.random.choice(range(len(preflops)), opponent_samples, replace=False):
        # Calculate the equity of this hand against the opponent_hand
        n_wins = 0
        n_games = 0
        opponent_preflop = preflops[preflop_index]
        all_remaining = list(permutations(deck, remaining_cards))
        # TODO: When random sampling, make sure that the sampled hands are always unique.
        # Otherwise there's a chance this crashes with a ZeroDivisionError.
        # for remaining_index in np.random.choice(range(len(all_remaining)), rollout_samples):
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
        bucket = int(equity // (1 / N_EQUITY_BINS))
        equity_distribution[bucket] += 1
    return equity_distribution


def abstraction2str(table):
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
    def __init__(self):
        self.table = {}

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
    def __init__(self, n_buckets=100):
        self.n_buckets = n_buckets
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
        abstraction = Cluster()(equity_distributions, K_MEANS_ITERS, N_FLOP_BUCKETS)
        pickle.dump(abstraction, open(FLOP_SAVE_NAME, 'wb'))
        return abstraction

    def hand_equity(self, hand):
        preflop = hand[:2]
        flop = hand[2:]
        # TODO: Add a paramater in the paramater file for number of samples.
        distribution = get_equity_distribution(preflop, flop,
                                               opponent_samples=20, rollout_samples=20)
        return distribution

    def __getitem__(self, cards):
        raise NotImplementedError

    def __str__(self):
        return abstraction2str(self.table)





if __name__ == '__main__':
    print_abstraction()
