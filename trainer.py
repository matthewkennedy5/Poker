# https://arxiv.org/abs/1809.04040

import numpy as np
from texas_utils import *
from hand_abstraction import PreflopAbstraction, FlopAbstraction, RiverAbstraction

PREFLOP_ACTIONS = 'fold', 'call', 'limp', 'raise', '3-bet', '4-bet', 'all_in'
ACTIONS = 'fold', 'check', 'call', 'half_pot', 'pot', 'min_raise', 'all_in'
SMALL_BLIND = 50
BIG_BLIND = 100
STACK_SIZE = 20000

# TODO: Action translation
class ActionHistory:

    def __init__(self, preflop=None, flop=None, turn=None, river=None):
        raise NotImplementedError

    def add_action(self, action):
        raise NotImplementedError

    def pot_size(self):
        raise NotImplementedError

    def street(self):
        raise NotImplementedError

    def whose_turn(self):
        raise NotImplementedError

    def legal_actions(self):
        raise NotImplementedError

    def hand_over(self):
        raise NotImplementedError

    def __str__(self):
        raise NotImplementedError


class InfoSet:

    def __init__(self, cards, action_history):
        self.cards = cards
        self.action_history = action_history

    def __eq__(self, other):
        raise NotImplementedError

    # Make sure equal infosets are hashed equally
    def __hash__(self, other):
        raise NotImplementedError

    def __str__(self):
        raise NotImplementedError


class Node:

    def __init__(self, infoset, alpha, beta, gamma):
        raise NotImplementedError

    def current_strategy(self, prob):
        raise NotImplementedError

    def cumulative_strategy(self):
        raise NotImplementedError

    def add_regret(self, action, regret):
        raise NotImplementedError


class Trainer:

    def __init__(self):
        self.nodes = {}

    def train(self, iterations):
        raise NotImplementedError

    def iterate(self, deck, bet_history=[], p0=1, p1=1):
        raise NotImplementedError

    def terminal_utility(self, deck, bet_history):
        raise NotImplementedError
