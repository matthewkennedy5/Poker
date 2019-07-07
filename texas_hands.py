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


# TODO: Do we even really need this card class?
@functools.total_ordering
class Card:
    """Class for representing a card using the format '8c', 'Th', etc.
    Example:
    card = Card('9d')
    card2 = Card('Th')
    card2 > card1 == True
    Attributes:
        self.suit - The suit of the card, represented by 'h', 'c', etc.
        self.rank - The rank of the card, given as an integer, so 'A' -> 14
    Input:
        card_str - Input string in the standard card format '2d', 'Jh', etc.
    Throws:
        ArgumentError if the input string is not in the correct format.
    """

    def __init__(self, card_str):
        if card_str[0] not in RANKS or card_str[1] not in SUITS:
            raise ValueError('card_str must be in the format like "Kc", "4h"')
        self.card_str = card_str
        self.rank = RANKS[self.card_str[0]]
        self.suit = self.card_str[1]

    def __eq__(self, other):
        return self.card_str[0] == other.card_str[0]

    def __lt__(self, other):
        return self.rank < other.rank

    def __hash__(self):
        # Simple hash function--return the memory address of the object.
        return id(self)

    def __str__(self):
        return self.card_str


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
        """Sets the self.type and self.rank variables to the hand type and rank."""
        if len(self.cards) > 5:
            best_subhand = None
            for five_cards in itertools.combinations(self.cards, 5):
                hand = TexasHand(five_cards)
                if hand > best_subhand:
                    best_subhand = hand
            self.type = best_subhand.type
            self.rank = best_subhand.rank
        else:
            self.rank = self.max_rank()
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

    def __lt__(self, other):
        if self.type < other.type:
            return True
        elif other.type < self.type:
            return False
        elif self.type == other.type:
            # TODO: Comparing ranks won't work for two pair when the second pairs
            # differ but the top pair is the same. Use every rank in turn to compare.
            return self.rank < other.rank

    def __eq__(self, other):
        raise NotImplementedError

    def __str__(self):
        raise NotImplementedError

