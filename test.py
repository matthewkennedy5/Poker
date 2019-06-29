from poker_utils import *
import numpy as np
import pdb
import unittest
import rhode_utils
import rhode_trainer

FOLD, CHECK, CALL, BET, RAISE = range(5)

if __name__ == '__main__':
    TestRaiseLimit.main()

class TestRaiseLimit(unittest.TestCase):
    def test(self, bet_history):
        deck = rhode_utils.get_deck()
        trainer = rhode_trainer.CFRPTrainer()
        info = rhode_trainer.InfoSet(deck, bet_history)
        return info.legal_actions()

    def no_raises(self):
        bet_history = ['check', 'bet', 'call', 'check', 'check']
        actions = self.test(bet_history)
        assert actions == [CHECK, BET]

    def empty(self):
        bet_history = []
        actions = self.test(bet_history)
        assert actions == [CHECK, BET]

    def many_raises(self):
        bet_history = ['bet', 'raise', 'raise', 'raise', 'call',
            'bet', 'raise', 'raise', 'call', 'bet', 'raise']
        actions = self.test(bet_history)
        assert actions == [FOLD, CALL, RAISE]

    def cant_raise(self):
        bet_history = ['bet', 'raise', 'raise', 'raise', 'call',
            'bet', 'raise', 'raise', 'raise']
        actions = self.test(bet_history)
        assert actions == [FOLD, CALL]

    def last_raise(self):
        bet_history = ['check', 'bet', 'raise', 'raise']
        actions = self.test(bet_history)
        assert actions == [FOLD, CALL, RAISE]

    def main()
        tester = TestRaiseLimit()
        tester.no_raises()
        tester.empty()
        tester.many_raises()
        tester.cant_raise()
        tester.last_raise()
        print('Raise Limit Works Properly')
