# https://arxiv.org/abs/1809.04040

import numpy as np
from texas_utils import *
from hand_abstraction import PreflopAbstraction, FlopAbstraction, RiverAbstraction

PREFLOP_ACTIONS = 'fold', 'call', 'limp', 'raise', '3-bet', '4-bet', 'all-in'
POSTFLOP_ACTIONS = 'fold', 'check', 'call', 'half_pot', 'pot', 'min_raise', 'all-in'
ACTIONS = PREFLOP_ACTIONS + POSTFLOP_ACTIONS
SMALL_BLIND = 50
BIG_BLIND = 100
STACK_SIZE = 20000

# TODO: Action translation
class ActionHistory:

    def __init__(self, preflop=None, flop=None, turn=None, river=None):
        self.preflop = preflop
        self.flop = flop
        self.turn = turn
        self.river = river

    def add_action(self, action):
        raise NotImplementedError

    def pot_size(self):
        stack_sizes = [STACK_SIZE, STACK_SIZE]
        player = 0
        prev_bet = 0
        bets = [[0], [0]]

        # Preflop bet sizes
        for action in self.preflop:
            if action == 'limp':
                bets[player].append(BIG_BLIND)
            elif action == 'call':
                bet = sum(bets[1-player]) - sum(bets[player])
                bets[player].append(bet)
            elif action == 'raise':
                bets[player].append(3 * BIG_BLIND)
            elif action == '3-bet':
                bets[player].append(3 * prev_bet)
            elif action == '4-bet':
                bets[player].append(3 * prev_bet)
            elif action == 'all-in':
                bets[player].append(stack_sizes[player])

            prev_bet = bets[player][-1]
            stack_sizes[player] -= prev_bet
            player = 1 - player

        pot = sum(bets[0]) + sum(bets[1])
        # Postfop bet sizes
        for street in self.flop, self.turn, self.river:
            if street is not None:
                player = 0
                prev_bet = 0
                for action in street:
                    if action == 'check':
                        bet = 0
                    elif action == 'call':
                        bet = sum(bets[1-player]) - sum(bets[player])
                    elif action == 'half_pot':
                        bet = pot / 2
                    elif action == 'pot':
                        bet = pot
                    elif action == 'min_raise':
                        bet = 2 * prev_bet
                    elif action == 'all-in':
                        bet = stack_sizes[player]

                    bets[player].append(bet)
                    prev_bet = bets[player][-1]
                    stack_sizes[player] -= prev_bet
                    player = 1 - player
                    pot += bet

        return pot

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
