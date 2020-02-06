import os
import itertools
from tqdm import tqdm
import pickle
from texas_utils import *
from texas_hands import TexasHand
import pdb

TABLE_NAME = 'hand_table.pkl'

class HandTable:

    def __init__(self):
        if os.path.isfile(TABLE_NAME):
            self.table = pickle.load(open(TABLE_NAME, 'rb'))
        else:
            self.table = self.make_table()
            pickle.dump(self.table, open(TABLE_NAME, 'wb'), protocol=pickle.HIGHEST_PROTOCOL)

    def __getitem__(self, cards):
        if not 5 <= len(cards) <= 7:
            raise ValueError('Wrong number of cards.')

        # Since the table holds 5 card hands, find the best possible 5 card hand
        # out of the list of 5-7 cards.
        best_strength = max([self.table[isomorphic_hand(hand)] for hand in itertools.combinations(cards, 5)])
        return best_strength

    # TODO: This unintentionally makes there be too many integers since hands is
    # a list. len(self.table) = 160537 but some hands are in the millions. Not
    # sure if this is an issue or not.
    def make_table(self):
        hands = []
        print('Constructing the lookup table for hand evaluation...')
        for cards in itertools.combinations(get_deck(), 5):
            hand = TexasHand(isomorphic_hand(cards))
            hands.append(hand)

        table = {}
        hands = sorted(hands)
        for i, hand in enumerate(hands):
            table[tuple(hand.cards)] = i
        # Hands that are equal need to be assigned the same integer.
        for i, hand in enumerate(hands[1:]):
            prev_hand = hands[i-1]
            if hand == prev_hand:
                table[tuple(hand.cards)] = table[tuple(prev_hand.cards)]
        print('Done.')
        return table

