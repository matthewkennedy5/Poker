# For playing Rhode Island Hold'em
# Rules: https://www.cs.cmu.edu/~gilpin/gsi.html

import functools
import numpy as np
import pdb

PREFLOP, FLOP, TURN = range(3)
STRAIGHT_FLUSH, THREE_OF_KIND, STRAIGHT, FLUSH, PAIR, HIGH_CARD = range(6)

RANKS = {'A': 14, 'K': 13, 'Q': 12, 'J': 11, 'T': 10, '9': 9, '8': 8, '7': 7,
         '6': 6, '5': 5, '4': 4, '3': 3, '2': 2}

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
        if card_str[0] not in RANKS or card_str[1] not in ['h', 's', 'c', 'd']:
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
class RhodeHand:
    """Represents a 3-card hand for Rhode Island Poker.

    Rhode Island Poker hands have different rankings than standard 5-card poker,
    as follows:

        Straight Flush
        Three of a Kind
        Straight
        Flush
        Pair
        High Card

    Inputs:
        card0, card1, card2 -  Three cards represented like '8c', 'Qs', etc.
    """

    def __init__(self, card0, card1, card2):
        self.cards = [Card(card) for card in (card0, card1, card2)]
        self.type = None
        self.rank = None
        self.classify()

    def classify(self):
        self.rank = self.max_rank()
        if self.is_straight_flush():
            self.type = STRAIGHT_FLUSH
        elif self.is_three_of_kind():
            self.type = THREE_OF_KIND
        elif self.is_straight():
            self.type = STRAIGHT
        elif self.is_flush():
            self.type = FLUSH
        elif self.is_pair():
            self.type = PAIR
        else:
            self.type = HIGH_CARD

    def max_rank(self):
        highest_rank = 2
        for card in self.cards:
            if card.rank > highest_rank:
                highest_rank = card.rank
        return highest_rank

    def is_straight_flush(self):
        return self.is_straight() and self.is_flush()

    def is_three_of_kind(self):
        return self.cards[0].rank == self.cards[1].rank == self.cards[2].rank

    def is_straight(self):
        sorted_ranks = sorted([card.rank for card in self.cards])
        if RANKS['A'] in sorted_ranks:
            # Account for ace low straights, where sorted_ranks = [2, 3, 14]
            return sorted_ranks == [12, 13, 14] or sorted_ranks == [2, 3, 14]
        else:
            return (sorted_ranks[0] + 1 == sorted_ranks[1] and sorted_ranks[1] + 1 == sorted_ranks[2])

    def is_flush(self):
        return self.cards[0].suit == self.cards[1].suit == self.cards[2].suit

    def is_pair(self):
        return (self.cards[0].rank == self.cards[1].rank
                or self.cards[1].rank == self.cards[2].rank
                or self.cards[0].rank == self.cards[2].rank)

    def __lt__(self, other):
        return False

    def __eq__(self, other):
        return True


class Game:

    def __init__(self):
        self.pot = 0
        self.player1card = None
        self.player2card = None
        self.board = []
        self.street = PREFLOP
        self.hand_is_over = False

    def play(self):
        """Initiate a sequence of hands for human vs. human play."""
        print('Welcome to Rhode Island Holdem!')
        while not self.hand_is_over:
            self.advance_hand()

    def advance_hand(self):
        if self.street == PREFLOP:
            pass
        elif self.street == FLOP:
            pass
        elif self.street == TURN:
            pass
        else:
            self.hand_is_over = True

    def preflop(self):
        # Deal cards
        # Issue: both players will see both cards. Welp.
        # Betting round
        pass

    def flop(self):
        # Deal flop
        # Betting round
        pass

    def turn(self):
        # Deal turn
        # Betting round
        pass


if __name__ == '__main__':

    three = RhodeHand('Th', 'Td', 'Tc')
    assert(three.type == THREE_OF_KIND)
    not_three = RhodeHand('Th', 'Jh', 'Tc')
    assert(not_three.type != THREE_OF_KIND)
    straight_flush = RhodeHand('Ah', '3h', '2h')
    not_straight_flush = RhodeHand('Kd', '2d', 'Ad')
    assert(straight_flush.type == STRAIGHT_FLUSH)
    assert(not_straight_flush.type != STRAIGHT_FLUSH)
    straight = RhodeHand('Tc', '8s', '9c')
    assert(straight.type == STRAIGHT)
    flush = RhodeHand('2h', '4h', 'Ah')
    assert(flush.type == FLUSH)
    pair = RhodeHand('Ks', 'Kd', 'Js')
    assert(pair.type == PAIR)
    nothing = RhodeHand('Jd', 'Tc', 'As')
    assert(nothing.type == HIGH_CARD)
    # game = game()
    # game.play()
