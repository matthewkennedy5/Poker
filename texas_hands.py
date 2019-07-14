"""Module for representing and comparing Texas Hold'em hands and abstractions."""

import os
import abc
import itertools
from itertools import product, permutations, combinations
import math
import functools
import pickle
import pdb
import numpy as np
from tqdm import tqdm

(HIGH_CARD, PAIR, TWO_PAIR, THREE_OF_A_KIND, STRAIGHT, FLUSH, FULL_HOUSE,
 FOUR_OF_A_KIND, STRAIGHT_FLUSH, ROYAL_FLUSH) = range(10)

RANKS = {'A': 14, 'K': 13, 'Q': 12, 'J': 11, 'T': 10, '9': 9, '8': 8, '7': 7,
         '6': 6, '5': 5, '4': 4, '3': 3, '2': 2}
SUITS = ('c', 'd', 'h', 's')

FLOP_SAVE_NAME = 'texas_flop_abstraction.pkl'


def get_deck():
    """Returns the standard 52-card deck, represented as a list of strings."""
    return [rank + suit for suit in SUITS for rank in RANKS]


def shuffled_deck():
    """Generator for a shuffled 52-card deck."""
    deck = get_deck()
    np.random.shuffle(deck)
    for card in deck:
        yield card



@functools.total_ordering
class TexasHand:
    """Represents a standard 5-card Texas Hold'em hand."""

    def __init__(self, cards):
        """Create a new Texas Holdem hand.

        Inputs:
            cards - list/tuple of cards in the standard format 'Th' '3s' etc.
                Must be at least five and no more than seven cards.

        Throws:
            ValueError if cards contains invalid card strings, the wrong number
            of cards, or duplicate cards.
        """
        self.check_input(cards)
        self.cards = list(cards)
        self.ranks = sorted([RANKS[card[0]] for card in self.cards])
        self.suits = [card[1] for card in self.cards]
        # How many times each rank appears in the hand (useful for hand classification)
        _, self.counts = np.sort(np.unique(self.ranks, return_counts=True))
        self.type = None
        # The 5 cards that actually matter if more than 5 are supplied
        self.playable_cards = None
        self.classify()

    def check_input(self, cards):
        """Makes sure the cards pass the input specifications."""
        if not 5 <= len(cards) <= 7:    # Python rocks
            raise ValueError('Must be between 5 and 7 cards provided')
        for card in cards:
            if not isinstance(card, str):
                raise TypeError('Cards must be strings in the format "5d"')
            if len(card) != 2 or card[0] not in RANKS or card[1] not in SUITS:
                raise ValueError('Invalid card string: ' + card)
        # Test for duplicate cards
        _, counts = np.unique(cards, return_counts=True)
        if np.max(counts) != 1:
            raise ValueError('Hand contains duplicate cards')

    def classify(self):
        """Identifies which type of poker hand this is."""
        if len(self.cards) > 5:
            best_subhand = None
            for five_cards in itertools.combinations(self.cards, 5):
                hand = TexasHand(five_cards)
                if hand > best_subhand:
                    best_subhand = hand
                    self.playable_cards = five_cards
            self.type = best_subhand.type
        else:
            self.playable_cards = self.cards
            if self.is_royal_flush():
                self.type = ROYAL_FLUSH
            elif self.is_straight_flush():
                self.type = STRAIGHT_FLUSH
            elif self.is_four_of_a_kind():
                self.type = FOUR_OF_A_KIND
            elif self.is_full_house():
                self.type = FULL_HOUSE
            elif self.is_flush():
                self.type = FLUSH
            elif self.is_straight():
                self.type = STRAIGHT
            elif self.is_three_of_a_kind():
                self.type = THREE_OF_A_KIND
            elif self.is_two_pair():
                self.type = TWO_PAIR
            elif self.is_pair():
                self.type = PAIR
            else:
                self.type = HIGH_CARD

    def is_royal_flush(self):
        # Ace-high straight flush = royal flush
        return self.is_straight_flush() and RANKS['A'] in self.ranks

    def is_straight_flush(self):
        return self.is_straight() and self.is_flush()

    def is_n_of_a_kind(self, n):
        """Returns True if the hand contains n cards of the same rank.

        Will return True for full houses.
        """
        return np.max(self.counts) == n

    def is_four_of_a_kind(self):
        return self.is_n_of_a_kind(4)

    def is_full_house(self):
        return np.array_equal(self.counts, np.array([2, 3]))

    def is_flush(self):
        suit = self.cards[0][1]
        for card in self.cards:
            if card[1] != suit:
                return False
        return True

    def is_straight(self):
        # True if the ranks are the same as the range (a straight)
        return (self.ranks == list(range(self.ranks[0], self.ranks[-1] + 1))
                or self.ranks == [2, 3, 4, 5, 14])    # Ace low straight

    def is_three_of_a_kind(self):
        return self.is_n_of_a_kind(3)

    def is_two_pair(self):
        # A two pair hand has one unique card and two duplicate-rank cards
        return np.array_equal(self.counts, np.array([1, 2, 2]))

    def is_pair(self):
        return self.is_n_of_a_kind(2)

    def __lt__(self, other):
        if other is None:
            return False
        if self.type < other.type:
            return True
        elif other.type < self.type:
            return False
        elif self.type == other.type:
            return self.compare_ranks(other) == -1

    def __eq__(self, other):
        # TODO: Implement this for real by considering the ranks of the top 5 cards
        if other is None:
            return False
        return self.type == other.type and self.compare_ranks(other) == 0

    def get_two_pair_info(self):
        """For a two pair hand, returns the ranks of the two pairs and kicker."""
        if self.type != TWO_PAIR:
            raise ValueError('Hand must be two pair')

        ranks = [RANKS[card[0]] for card in self.playable_cards]
        unique, counts = np.unique(ranks, return_counts=True)
        pairs = []
        kicker = 0
        for i in reversed(range(len(counts))):
            if counts[i] == 2:
                pairs.append(unique[i])
            if counts[i] == 1:
                kicker = unique[i]
        return pairs, kicker

    def compare_ranks(self, other):
        """Compares the five playable cards of each hand to see which rank higher.

        This method is useful if both hands are the same type (say, flush) and
        you want to know which flush is better.

        Inputs:
            other - TexasHand instance

        Returns:
            cmp: -1 if self < other, 0 if self == other, and 1 if self > other.
        """
        if self.type != other.type:
            raise ValueError('Hand types must match for the compare_ranks method.')
        my_ranks = np.sort([RANKS[card[0]] for card in self.playable_cards])[::-1]
        other_ranks = np.sort([RANKS[card[0]] for card in other.playable_cards])[::-1]
        # Two pair is a special case that is especially difficult to rank.
        if self.type == TWO_PAIR:
            my_pairs, my_kicker = self.get_two_pair_info()
            other_pairs, other_kicker = other.get_two_pair_info()
            for my_pair, other_pair in zip(my_pairs, other_pairs):
                if my_pair < other_pair:
                    return -1
                if my_pair > other_pair:
                    return 1

            if my_kicker < other_kicker:
                return -1
            if my_kicker > other_kicker:
                return 1
            return 0    # Both pairs and the kicker are all equal.

        for i in range(len(my_ranks)):
            if my_ranks[i] < other_ranks[i]:
                return -1
            elif my_ranks[i] > other_ranks[i]:
                return 1
        return 0


    def __str__(self):
        raise NotImplementedError


# TODO: Use lookup tables to speed up hand classification and comparison. Each
# hand could correspond to a strength integer (like 1453)

########### Hand abstraction methods #######################

########### Code modified from hand_abstraction.py ###########


def print_abstraction():
    print(PreflopAbstraction())
    print(FlopAbstraction())
    print(TurnAbstraction())
    print(RiverAbstraction())


def duplicate_cards(cards):
    """Returns True if cards are repeated.

    Input:
        cards - tuple/list of cards in the standard 'Ad' format
    """
    return len(np.unique(cards)) == len(cards)


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
        for hand in itertools.combinations(get_deck(), 2):
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

        import sys
        equity_distribution = {}
        # TODO: Don't iterate over unused permutations if it's too slow.
        deck = get_deck()
        with tqdm(range(29304600)) as t:   # 52C2 * 52C3
            for preflop, flop in product(combinations(deck, 2), combinations(deck, 3)):
                # Hands are registered as tuples of frozensets to preserve order
                # only when it matters.
                hand = (frozenset(preflop), frozenset(flop))
                if hand not in equity_distribution:
                    equity_distribution[hand] = self.get_equity_distribution(hand)
                t.update()

        self.cluster(equity_distributions, n_buckets=n_buckets)

    def get_equity_distribution(self, hand):
        """Returns the equity histogram distribution for the given hand.

        Equity is the chance of winning plus 1/2 the chance of tying.

        Inputs:
            hand - tuple of (frozenset, frozenset) where the first frozenset is
                the preflop cards and the second is the flop cards.
        """
        preflop, flop = hand
        for opponent_preflop in zip(shuffled_deck(), shuffled_deck()):
            pass



    def __getitem__(self, cards):
        pass

    def __str__(self):
        return 'nope'


if __name__ == '__main__':
    print_abstraction()

