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
FLOP_SAVE_NAME = 'flop_abstraction.pkl'
TURN_SAVE_NAME = 'turn_abstraction.pkl'
ARCHETYPAL_FLOP_FILENAME = 'flop_hands.pkl'
ARCHETYPAL_TURN_FILENAME = 'turn_hands.pkl'
FLOP_EQUITY_DISTIBUTIONS = 'flop_equity.pkl'
TURN_EQUITY_DISTRIBUTIONS = 'turn_equity.pkl'
HAND_TABLE = HandTable()


def print_equities():
    equities = pickle.load(open(TURN_EQUITY_DISTRIBUTIONS, 'rb'))
    hands = list(equities.keys())
    np.random.shuffle(hands)
    for hand in hands[:10]:
        print(hand, equities[hand])


def print_abstraction():
    params = json.load(open(PARAM_FILE, 'r'))
    # print(PreflopAbstraction())
    # print(FlopAbstraction(**params['flop']))
    # abst = FlopAbstraction(**params['flop'])
    abst = TurnAbstraction(**params['turn'])
    # print_equities()
    inspect_abstraction(abst, params['turn']['buckets'], 'turn')
    # print(RiverAbstraction(**params['river']))


def inspect_abstraction(abstraction, n_buckets, street):
    hands = list(abstraction.abstraction.table.keys())
    if street == 'flop':
        equities = pickle.load(open(FLOP_EQUITY_DISTIBUTIONS, 'rb'))
    elif street == 'turn':
        equities = pickle.load(open(TURN_EQUITY_DISTRIBUTIONS, 'rb'))
    np.random.shuffle(hands)
    for i in range(n_buckets):
        print(i)
        count = 0
        for hand in hands:
            if abstraction[hand] == i:
                print(hand, equities[hand])
                count += 1
                if count > 5:
                    break


def unique_cards(cards):
    """Returns True if there are no repeated cards in the given list.

    Input:
        cards - tuple/list of cards in the standard 'Ad' format
    """
    return len(np.unique(cards)) == len(cards)


def archetypal_flop_hands():
    if os.path.isfile(ARCHETYPAL_FLOP_FILENAME):
        return pickle.load(open(ARCHETYPAL_FLOP_FILENAME, 'rb'))
    print('Preparing archetypal flop hands...')
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


def archetypal_turn_hands():
    if os.path.isfile(ARCHETYPAL_TURN_FILENAME):
        return pickle.load(open(ARCHETYPAL_TURN_FILENAME, 'rb'))
    print('Preparing archetypal turn hands...')
    hands = []
    deck = get_deck()
    used_hands = {}
    with tqdm(total=29304600 * 52, smoothing=0) as t:
        for preflop, flop, turn in product(combinations(deck, 2), combinations(deck, 3), deck):
            hand = preflop + flop + (turn,)
            if unique_cards(hand):
                hand = archetypal_hand(hand)
                if hand not in used_hands:
                    used_hands[hand] = True
            t.update()
    hands = list(used_hands.keys())
    pickle.dump(hands, open(ARCHETYPAL_TURN_FILENAME, 'wb'))
    return hands


def archetypal_river_hands():
    raise NotImplementedError


def archetypal_hands(street):
    """Returns a list of all archetypal hands for the given street.

    An archetypal hand is a hand with unnecessary order and suit information
    removed. For example, it doesn't matter what order the flop cards are in, so
    we can sort the flop cards without losing information. Same with the preflop.
    Also, we can only consider one suit isomorphism out of the many possible
    isomorphisms. For example, a flush of hearts is functionally the same as a
    flush of diamonds. The returned hand will be sorted by the preflop and flop
    and be run through the suit isomorphism algorithm. Using these techniques
    greatly reduces the size of the flop abstraction lookup table.

    Inputs:
        street - 'preflop' or 'flop'
    """
    if street == 'flop':
        return archetypal_flop_hands()
    elif street == 'turn':
        return archetypal_turn_hands()
    else:
        raise ValueError('Unknown street')


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
        hand += (turn,)
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
            if remaining_cards == 2:
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


class StreetAbstraction(CardAbstraction):

    # Base class for the flop and turn abstractions since they're so similar.
    def __init__(self, street, buckets=5000, equity_bins=50, iters=20,
                 opponent_samples=100, rollout_samples=100):
        if street not in ('turn', 'flop'):
            raise ValueError('Unknown street')
        self.street = street
        self.buckets = buckets
        self.equity_bins = equity_bins
        self.iters = iters
        self.opponent_samples = opponent_samples
        self.rollout_samples = rollout_samples
        self.table = self.compute_abstraction()

    def compute_abstraction(self):
        """Clusters all possible flop hands into groups."""
        abstraction_file = ''
        equity_file = ''
        if self.street == 'flop':
            abstraction_file = FLOP_SAVE_NAME
            equity_file = FLOP_EQUITY_DISTIBUTIONS
        elif self.street == 'turn':
            abstraction_file = TURN_SAVE_NAME
            equity_file = TURN_EQUITY_DISTRIBUTIONS

        if os.path.isfile(abstraction_file):
            return pickle.load(open(abstraction_file, 'rb'))

        print('Computing the %s abstraction...' % (self.street,))
        if os.path.isfile(equity_file):
            equity_distributions = pickle.load(open(equity_file, 'rb'))
        else:
            print('Calculating equity distributions...')
            hands = archetypal_hands(self.street)
            distributions = pbar_map(self.hand_equity, hands)
            equity_distributions = dict(zip(hands, distributions))
            pickle.dump(equity_distributions, open(equity_file, 'wb'))

        print('Performing k-means clustering...')
        abstraction = Cluster(equity_distributions, self.buckets, self.iters)()
        pickle.dump(abstraction, open(abstraction_file, 'wb'))
        return abstraction

    def hand_equity(self, hand):
        """Returns the equity distribution for the given flop hand.

        Inputs:
            hand - list of cards with the first two cards being the preflop and
                the next three being the flop. The last card (if given) is the
                turn card

        Returns:
            distribution - Estimate of the equity distribution (histogram) over
                all possible opponent holdings and rollouts.

        """
        preflop = hand[:2]
        flop = hand[2:5]
        turn = None
        if len(hand) > 5:
            turn = hand[5]
        distribution = get_equity_distribution(preflop, flop, turn,
                                               equity_bins=self.equity_bins,
                                               opponent_samples=self.opponent_samples,
                                               rollout_samples=self.rollout_samples)
        return distribution

    def __getitem__(self, cards):
        return self.table[archetypal_hand(cards)]

    def __str__(self):
        return str(self.table)


class FlopAbstraction(CardAbstraction):
    """Finds similar flop hands and groups them together.

    Similarity is based on the Earth Movers Distance of the hands' equity
    distributions, and clustering is performed using k_means clustering.
    """
    def __init__(self, buckets=5000, equity_bins=50, iters=20,
                 opponent_samples=100, rollout_samples=100):
        self.abstraction = StreetAbstraction('flop', buckets, equity_bins, iters,
                                             opponent_samples, rollout_samples)

    def __getitem__(self, cards):
        return self.abstraction[cards]

    def __str__(self):
        return str(self.abstraction)


class TurnAbstraction(CardAbstraction):

    def __init__(self, buckets=5000, equity_bins=50, iters=20,
                 opponent_samples=100, rollout_samples=100):
        self.abstraction = StreetAbstraction('turn', buckets, equity_bins, iters,
                                             opponent_samples, rollout_samples)

    def __getitem__(self, cards):
        return self.abstraction[cards]

    def __str__(self):
        return str(self.abstraction)


# class RiverAbstraction(CardAbstraction):

    # def __init__(self, buckets=5000, )



if __name__ == '__main__':
    #print_equities()
    print_abstraction()
