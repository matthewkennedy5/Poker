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




if __name__ == '__main__':
    unittest.main()
