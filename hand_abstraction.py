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


FLOP_SAVE_NAME = 'texas_flop_abstraction.pkl'
N_EQUITY_BINS = 10
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


def flop_gen():
    """Yields archetypal flop hands."""
    deck = get_deck()
    used_hands = {}
    for preflop, flop in product(combinations(deck, 2), combinations(deck, 3)):
        hand = preflop + flop
        if len(np.unique(hand)) == len(hand):
            hand = archetypal_hand(hand)
            if hand not in used_hands:
                used_hands[hand] = True
                yield hand


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
    for preflop_index in np.random.choice(range(len(preflops)), opponent_samples, replace=False):
        # Calculate the equity of this hand against the opponent_hand
        n_wins = 0
        n_games = 0
        opponent_preflop = preflops[preflop_index]
        all_remaining = list(permutations(deck, remaining_cards))
        for remaining_index in np.random.choice(range(len(all_remaining)), rollout_samples):
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
        result = ''
        for bucket in sorted(self.table.values()):
            result += str(bucket) + ': '
            for hand in self.table:
                if self.table[hand] == bucket:
                    result += str(tuple(hand)) + ' '
            result += '\n'
        return result


class FlopAbstraction(CardAbstraction):
    """Finds similar flop hands and groups them together.

    Similarity is based on the Earth Movers Distance of the hands' equity
    distributions, and clustering is performed using k_means clustering.
    """
    def __init__(self, n_buckets=100):
        self.table = {}
        self.n_buckets = n_buckets
        self.compute_abstraction()

    def compute_abstraction(self):
        """Clusters all possible flop hands into groups."""
        if os.path.isfile(FLOP_SAVE_NAME):
            return pickle.load(open(FLOP_SAVE_NAME, 'rb'))

        # TODO: pickle the list of archetypal flop hands
        equity_distribution = {}
        with mp.Pool(8) as p:
            result = p.map(self.hand_equity, flop_gen())
            # result = list(tqdm(p.imap(self.hand_equity, flop_gen()), total=500000))

        pickle.dump(result, open('flopresult.pkl', 'wb'))
        print(result)
        pdb.set_trace()

        self.cluster(equity_distributions, n_buckets=n_buckets)

    def hand_equity(self, hand):
        preflop = hand[:2]
        flop = hand[2:]
        distribution = get_equity_distribution(preflop, flop,
                                               opponent_samples=10, rollout_samples=10)
        return distribution

    def __getitem__(self, cards):
        pass

    def __str__(self):
        return 'nope'


if __name__ == '__main__':
    print_abstraction()
