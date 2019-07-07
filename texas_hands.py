"""Module for representing and comparing Texas Hold'em hands and abstractions."""

import numpy
import functools
import pickle
import pdb

HIGH_CARD, PAIR, FLUSH, STRAIGHT, THREE_OF_A_KIND, STRAIGHT_FLUSH = range(6)

RANKS = {'A': 14, 'K': 13, 'Q': 12, 'J': 11, 'T': 10, '9': 9, '8': 8, '7': 7,
         '6': 6, '5': 5, '4': 4, '3': 3, '2': 2}
SUITS = ('c', 'd', 'h', 's')


def get_deck():
    """Returns the standard 52-card deck, represented as a list of strings."""
    return [rank + suit for suit in SUITS for rank in RANKS]



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
        raise NotImplementedError

    def classify(self):
        raise NotImplementedError

    def __lt__(self, other):
        raise NotImplementedError

    def __eq__(self, other):
        raise NotImplementedError

    def __str__(self):
        raise NotImplementedError

