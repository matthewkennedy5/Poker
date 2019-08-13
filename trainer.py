# https://arxiv.org/abs/1809.04040

import copy
import pickle
import multiprocessing as mp
import numpy as np
from tqdm import trange
from texas_utils import *
from hand_abstraction import PreflopAbstraction, FlopAbstraction, TurnAbstraction, RiverAbstraction
from hand_table import HandTable

SAVE_PATH = 'blueprint.pkl'
PREFLOP_ACTIONS = 'fold', 'call', 'limp', 'raise', '3-bet', '4-bet', 'all-in'
POSTFLOP_ACTIONS = 'fold', 'check', 'call', 'half_pot', 'pot', 'min_raise', 'all-in'
ACTIONS = PREFLOP_ACTIONS + POSTFLOP_ACTIONS
SMALL_BLIND = 50
BIG_BLIND = 100
STACK_SIZE = 20000

# TODO - Parameters
PREFLOP_ABSTRACTION = PreflopAbstraction()
FLOP_ABSTRACTION = FlopAbstraction()
TURN_ABSTRACTION = TurnAbstraction()
RIVER_ABSTRACTION = RiverAbstraction()
HAND_TABLE = HandTable()

np.random.seed(123)


def conditional_copy(l):
    if l is None:
        return None
    return list(l).copy()


def normalize(dictionary):
    total = sum(dictionary.values())
    for key in dictionary:
        dictionary[key] /= total
    return dictionary


def draw_deck(deck, player, return_hand=False):
    if player == 0:
        hole = deck[:2]
    else:
        hole = deck[2:4]
    flop = deck[4:7]
    turn = deck[7:8]
    river = deck[8:9]
    if return_hand:
        return hole + flop + turn + river
    else:
        return hole, flop, turn, river


# TODO: Action translation
class ActionHistory:

    def __init__(self, preflop=None, flop=None, turn=None, river=None):
        self.preflop = preflop
        self.flop = flop
        self.turn = turn
        self.river = river

    def pot_size(self, return_stack_sizes=False):
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
            elif action == 'fold':
                break

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
                        # TODO: This is wrong for re-raises (but it's consistent)
                        bet = 2 * prev_bet
                    elif action == 'all-in':
                        bet = stack_sizes[player]
                    elif action == 'fold':
                        break

                    bets[player].append(bet)
                    prev_bet = bets[player][-1]
                    stack_sizes[player] -= prev_bet
                    player = 1 - player
                    pot += bet

        if stack_sizes[0] < 0 or stack_sizes[1] < 0:
            raise ValueError('Invalid bet history: bets exceed stack size.')

        if return_stack_sizes:
            return pot, stack_sizes
        else:
            return pot

    def stack_sizes(self):
        return self.pot_size(return_stack_sizes=True)[1]

    def street(self):
        street = ''
        if self.river is not None:
            if self.street_is_over(self.river):
                street = 'over'
            else:
                street = 'river'
        elif self.turn is not None:
            if self.street_is_over(self.turn):
                street = 'river'
            else:
                street = 'turn'
        elif self.flop is not None:
            if self.street_is_over(self.flop):
                street = 'turn'
            else:
                street = 'flop'
        else:
            if self.street_is_over(self.preflop):
                street = 'flop'
            else:
                street = 'preflop'
        return street

    def street_is_over(self, street_history):
        return ((len(street_history) >= 1 and street_history[-1] == 'call')
                or (len(street_history) >= 2 and street_history[-1] == 'check'
                                             and street_history[-2] == 'check'))

    def whose_turn(self):
        history = self.current_street_history()
        if history is None:
            return 0
        else:
            return len(history) % 2

    def current_street_history(self):
        street = self.street()
        if street == 'preflop':
            return self.preflop
        elif street == 'flop':
            return self.flop
        elif street == 'turn':
            return self.turn
        elif street == 'river':
            return self.river

    def legal_actions(self):
        history = self.current_street_history()
        if history is None or len(history) == 0:
            prev_action = None
        else:
            prev_action = history[-1]
        if self.street() == 'preflop':
            if prev_action is None:
                return ('fold', 'limp', 'raise')
            elif prev_action == 'limp':
                return ('fold', 'call', 'raise')
            elif prev_action == 'raise':
                return ('fold', 'call', '3-bet')
            elif prev_action == '3-bet':
                return ('fold', 'call', '4-bet', 'all-in')
            elif prev_action == '4-bet':
                return ('fold', 'call', 'all-in')
            elif prev_action == 'all-in':
                return ('fold', 'call')

        # Postflop
        pot = self.pot_size()
        if prev_action is None:
            actions = ['check', 'half_pot', 'pot', 'all-in']
        elif prev_action == 'check':
            actions = ['check', 'half_pot', 'pot', 'all-in']
        elif prev_action in ('half_pot', 'pot', 'min_raise'):
            actions = ['fold', 'call', 'min_raise', 'all-in']
        elif prev_action == 'all-in':
            actions = ['fold', 'call']
        else:
            raise ValueError('Unknown previous action')

        for action in actions.copy():
            trial = self + action
            try:
                trial.pot_size()
            except ValueError:
                # The action is invalid because the bets are larger than
                # the stack sizes
                actions.remove(action)

        return tuple(actions)

    def hand_over(self):
        if self.street() == 'over':
            return True
        if self.last_action() == 'fold':
            return True
        return False

    def last_action(self):
        history = self.current_street_history()
        if history is None or len(history) == 0:
            return None
        else:
            return history[-1]

    def __str__(self):
        return 'Preflop: {}, Flop: {}, Turn: {}, River: {}'.format(self.preflop, self.flop, self.turn, self.river)

    def __hash__(self):
        return hash(str(self))

    def __add__(self, action):
        preflop = conditional_copy(self.preflop)
        flop = conditional_copy(self.flop)
        turn = conditional_copy(self.turn)
        river = conditional_copy(self.river)

        street = self.street()
        action = (action,)
        if street == 'preflop':
            preflop += action
        elif street == 'flop':
            if flop is None:
                flop = ()
            flop += action
        elif street == 'turn':
            if turn is None:
                turn = ()
            turn += action
        elif street == 'river':
            if river is None:
                river = ()
            river += action
        return ActionHistory(preflop, flop, turn, river)

    def __eq__(self, other):
        return str(self) == str(other)


class InfoSet:

    def __init__(self, deck, history):
        self.history = history
        street = history.street()
        player = history.whose_turn()
        hole, flop, turn, river = draw_deck(deck, player)
        if street == 'preflop':
            hand = hole
            self.card_bucket = PREFLOP_ABSTRACTION[hand]
        elif street == 'flop':
            hand = hole + flop
            self.card_bucket = FLOP_ABSTRACTION[hand]
        elif street == 'turn':
            hand = hole + flop + turn
            self.card_bucket = TURN_ABSTRACTION[hand]
        elif street == 'river':
            hand = hole + flop + turn + river
            self.card_bucket = RIVER_ABSTRACTION[hand]
        else:
            raise ValueError('Unknown street.')
        self.hand = hand

    def __eq__(self, other):
        return self.card_bucket == other.card_bucket and self.history == other.history

    # Make sure equal infosets are hashed equally
    def __hash__(self):
        return self.card_bucket + hash(self.history)

    def __str__(self):
        return ('Information set:\n\tPlayer: {}\n\tCard bucket: {}\n\tHistory: {}\n\tHand: {}'
               '\n\tStreet: {}').format(self.history.whose_turn(), self.card_bucket, self.history, self.hand,
                                       self.history.street())

    def legal_actions(self):
        return self.history.legal_actions()


class Node:

    def __init__(self, infoset, alpha=1.5, beta=0, gamma=2):
        self.infoset = infoset
        self.regrets = {}
        for action in self.infoset.legal_actions():
            self.regrets[action] = 0
        self.weighted_strategy_sum = self.regrets.copy()
        self.t = 0

    def current_strategy(self, prob=0):
        actions = self.infoset.legal_actions()
        strategy = {}
        if sum(self.regrets.values()) == 0:
            for action in actions:
                strategy[action] = 1
        else:
            for action in actions:
                chance = self.regrets[action]
                if chance < 0:
                    chance = 0
                strategy[action] = chance

        strategy = normalize(strategy)
        # TODO: DCFR implementation
        for action in actions:
            self.weighted_strategy_sum[action] += strategy[action] * prob
        if prob > 0:
            self.t += 1
        return strategy

    def cumulative_strategy(self):
        actions = self.infoset.legal_actions()
        strategy = {}
        if sum(self.weighted_strategy_sum.values()) == 0:
            for action in actions:
                strategy[action] = 1
        else:
            strategy = self.weighted_strategy_sum
        strategy = normalize(strategy)
        return strategy

    def add_regret(self, action, regret):
        # TODO: DCFR
        self.regrets[action] += regret

    def __str__(self):
        return '{}\nStrategy: {}\nHits: {}'.format(self.infoset, self.cumulative_strategy(), self.t+1)


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
    t.train(int(1e2))
