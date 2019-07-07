import numpy as np
import pdb
import unittest
import rhode_utils
import rhode_trainer
from texas_hands import *


FOLD, CHECK, CALL, BET, RAISE = range(5)


class TestRaiseLimit(unittest.TestCase):

    def run_test(self, bet_history):
        deck = rhode_utils.get_deck()
        trainer = rhode_trainer.CFRPTrainer()
        info = rhode_trainer.InfoSet(deck, bet_history)
        return info.legal_actions()

    def test_no_raises(self):
        bet_history = ['check', 'bet', 'call', 'check', 'check']
        actions = self.run_test(bet_history)
        self.assertEqual(actions, [CHECK, BET])

    def test_empty(self):
        bet_history = []
        actions = self.run_test(bet_history)
        self.assertTrue(actions == [CHECK, BET])

    def test_many_raises(self):
        bet_history = ['bet', 'raise', 'raise', 'raise', 'call',
            'bet', 'raise', 'raise', 'call', 'bet', 'raise']
        actions = self.run_test(bet_history)
        self.assertTrue(actions == [FOLD, CALL, RAISE])

    def test_cant_raise(self):
        bet_history = ['bet', 'raise', 'raise', 'raise', 'call',
            'bet', 'raise', 'raise', 'raise']
        actions = self.run_test(bet_history)
        self.assertTrue(actions == [FOLD, CALL])

    def test_last_raise(self):
        bet_history = ['check', 'bet', 'raise', 'raise']
        actions = self.run_test(bet_history)
        self.assertTrue(actions == [FOLD, CALL, RAISE])


class TestTexasHands(unittest.TestCase):

    def test_classification(self):
        royal_flush = TexasHand(('As', 'Js', 'Ks', 'Qs', '10s'))
        straight_flush = TexasHand(('7d', '8d', 'Jd', '9d', '10d'))
        four = TexasHand(('2h', '2c', '7d', '2d', '2s'))
        full_house = TexasHand(('As', 'Jd', 'Jc', 'Ac', 'Ah'))
        flush = TexasHand(('Jh', '2h', '3h', '7h', '9h'))
        straight = TexasHand(('Ah', '2s', '3d', '5c', '4c'))
        trips = TexasHand(('5d', '4c', '6d', '6h', '6c'))
        two_pair = TexasHand(('6d', '5c', '5h', 'Ah', 'Ac'))
        pair = TexasHand(('Ah', '2d', '2s', '3c', '5c'))
        high_card = TexasHand('Kh', 'Ah', 'Qh', '2h', '3s')
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)

    def test_six_cards(self):
        pass

    def test_seven_cards(self):
        pass

    # Should raise a value error when incorrect cards are provided.
    def test_errors(self):
        pass

    def test_comparisons(self):
        pass


if __name__ == '__main__':
    unittest.main()
