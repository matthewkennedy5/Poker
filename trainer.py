# https://arxiv.org/abs/1809.04040

import copy
import pickle
import multiprocessing as mp
import numpy as np
from tqdm import trange
from texas_utils import *
from hand_table import HandTable
from hand_abstraction import PreflopAbstraction, FlopAbstraction, TurnAbstraction, RiverAbstraction
from trainer_utils import *

SAVE_PATH = 'blueprint.pkl'

# TODO - Parameters

np.random.seed(123)


class Trainer:

    def __init__(self):
        self.nodes = {}

    def train(self, iterations):
        print('Beginning training...')
        deck = get_deck()
        for i in trange(iterations):
            # np.random.shuffle(deck)
            self.iterate(0, deck)
            # np.random.shuffle(deck)
            self.iterate(1, deck)

        with open(SAVE_PATH, 'wb') as f:
            pickle.dump(self.nodes, f, protocol=pickle.HIGHEST_PROTOCOL)

    def lookup_node(self, deck, history):
        infoset = InfoSet(deck, history)
        if infoset not in self.nodes:
            self.nodes[infoset] = Node(infoset)
        return self.nodes[infoset], infoset

    def opponent_action(self, node, infoset):
        actions = infoset.legal_actions()
        strategy = node.current_strategy()
        # Make sure the strategy values are in the correct order
        strat = []
        for action in actions:
            strat.append(strategy[action])

        action = np.random.choice(actions, p=strat)
        return action

    def iterate(self, player, deck, history=ActionHistory([]), weights=[1, 1]):
        if history.hand_over():
            return self.terminal_utility(deck, history, player)

        node, infoset = self.lookup_node(deck, history)

        opponent = 1 - player
        if history.whose_turn() == opponent:
            history += self.opponent_action(node, infoset)
            if history.hand_over():
                return self.terminal_utility(deck, history, player)
            node, infoset = self.lookup_node(deck, history)

        player_weight = weights[player]
        opponent_weight = weights[opponent]
        p0, p1 = weights

        strategy = node.current_strategy(player_weight)
        utility = {}
        node_utility = 0
        for action in infoset.legal_actions():
            next_history = history + action
            if player == 0:
                weights = [p0*strategy[action], p1]
                utility[action] = self.iterate(player, deck, next_history, weights)
            elif player == 1:
                weights = p0, p1*strategy[action]
                utility[action] = self.iterate(player, deck, next_history, weights)
            node_utility += strategy[action] * utility[action]

        for action in infoset.legal_actions():
            regret = utility[action] - node_utility
            node.add_regret(action, opponent_weight * regret)
        return node_utility

    def terminal_utility(self, deck, history, player):
        last_player = 1 - history.whose_turn()
        if history.last_action() == 'fold':
            stack_sizes = history.stack_sizes()
            return stack_sizes[last_player] - STACK_SIZE

        # Showdown - we can assume both players have contributed equally to the pot
        pot = history.pot_size()
        opponent = 1 - player
        player_hand = draw_deck(deck, player, return_hand=True)
        opponent_hand = draw_deck(deck, opponent, return_hand=True)
        player_strength = HAND_TABLE[player_hand]
        opponent_strength = HAND_TABLE[opponent_hand]
        if player_strength > opponent_strength:
            return pot / 2
        elif player_strength < opponent_strength:
            return -pot / 2
        elif player_strength == opponent_strength:
            return 0


if __name__ == '__main__':
    t = Trainer()
    t.train(int(1e4))
