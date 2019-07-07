"""Module for representing and comparing Texas Hold'em hands and abstractions."""

import numpy
import itertools
import functools
import pickle
import pdb
import numpy as np

(HIGH_CARD, PAIR, TWO_PAIR, THREE_OF_A_KIND, STRAIGHT, FLUSH, FULL_HOUSE,
 FOUR_OF_A_KIND, STRAIGHT_FLUSH, ROYAL_FLUSH) = range(10)

RANKS = {'A': 14, 'K': 13, 'Q': 12, 'J': 11, 'T': 10, '9': 9, '8': 8, '7': 7,
         '6': 6, '5': 5, '4': 4, '3': 3, '2': 2}
SUITS = ('c', 'd', 'h', 's')


def get_deck():
    """Returns the standard 52-card deck, represented as a list of strings."""
    return [rank + suit for suit in SUITS for rank in RANKS]


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
        # TODO: Throw the necessary errors.
        self.cards = list(cards)
        self.ranks = sorted([RANKS[card[0]] for card in self.cards])
        self.suits = [card[1] for card in self.cards]
        # How many times each rank appears in the hand (useful for hand classification)
        _, self.counts = np.sort(np.unique(self.ranks, return_counts=True))
        self.type = None
        self.classify()

    def classify(self):
        """Identifies which type of poker hand this is."""
        if len(self.cards) > 5:
            best_subhand = None
            for five_cards in itertools.combinations(self.cards, 5):
                hand = TexasHand(five_cards)
                if hand > best_subhand:
                    best_subhand = hand
            self.type = best_subhand.type
        else:
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
        if self.type < other.type:
            return True
        elif other.type < self.type:
            return False
        elif self.type == other.type:
            # TODO: Comparing ranks won't work for two pair when the second pairs
            # differ but the top pair is the same. Use every rank in turn to compare.
            return self.compare_ranks(other)

    def __eq__(self, other):
        raise NotImplementedError

    def __str__(self):
        raise NotImplementedError

