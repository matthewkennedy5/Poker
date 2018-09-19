from poker_utils import *
import numpy as np
import pdb

deck = get_deck()
i = 0
while True:
    cards = np.random.choice(deck, size=5, replace=False)
    hand = Hand(cards)
    if hand.get_type() == HandType.FOUR_OF_A_KIND:
        print(hand)
        print(hand.get_rank())
        input()
    if i % 1e4 == 0:
        print(i)
    i += 1
