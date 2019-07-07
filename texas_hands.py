"""Module for representing and comparing Texas Hold'em hands and abstractions."""

import numpy
import itertools
import functools
import pickle
import pdb

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
        self.cards = list(cards)
        self.type = None
        self.rank = None
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
            else:
                self.type = HIGH_CARD

    def is_royal_flush(self):
        if self.is_straight_flush() and self.has_rank('A')


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

