# https://arxiv.org/abs/1809.04040

import copy
import numpy as np
from tqdm import trange
from texas_utils import *
from hand_abstraction import PreflopAbstraction, FlopAbstraction, TurnAbstraction, RiverAbstraction

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

# TODO: Action translation
class ActionHistory:

    def __init__(self, preflop=None, flop=None, turn=None, river=None):
        self.preflop = preflop
        self.flop = flop
        self.turn = turn
        self.river = river

    def add_action(self, action):
        street = self.street()
        action = (action,)
        if street == 'preflop':
            self.preflop += action
        elif street == 'flop':
            if self.flop is None:
                self.flop = ()
            self.flop += action
        elif street == 'turn':
            if self.turn is None:
                self.turn = ()
            self.turn += action
        elif street == 'river':
            if self.river is None:
                self.river = ()
            self.river += action

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

        if stack_sizes[0] < 0 or stack_sizes[1] < 0:
            raise ValueError('Invalid bet history: bets exceed stack size.')
        return pot

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
        if history is None:
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

        for action in actions:
            trial = copy.deepcopy(self)
            trial.add_action(action)
            try:
                trial.pot_size()
            except ValueError:
                # This action is invalid because the bets are larger than
                # the stack sizes
                actions.remove(action)

        return tuple(actions)

    def hand_over(self):
        return self.street() == 'over'

    def __str__(self):
        return 'Preflop: {}\nFlop: {}\nTurn: {}\nRiver: {}'.format(self.preflop, self.flop, self.turn, self.river)

    def __hash__(self):
        return hash(str(self))


class InfoSet:

    def __init__(self, deck, history):
        self.history = history
        street = history.street()
        if street == 'preflop':
            self.card_bucket = PREFLOP_ABSTRACTION[deck[:2]]
        elif street == 'flop':
            self.card_bucket = FLOP_ABSTRACTION[deck[:5]]
        elif street == 'turn':
            self.card_bucket = TURN_ABSTRACTION[deck[:6]]
        elif street == 'river':
            self.card_bucket = RIVER_ABSTRACTION[deck[:7]]
        else:
            raise ValueError('Unknown street.')

    def __eq__(self, other):
        return self.card_bucket == other.card_bucket and self.history == other.history

    # Make sure equal infosets are hashed equally
    def __hash__(self):
        return self.card_bucket + hash(self.history)

    def __str__(self):
        return 'Information set:\n\tCard bucket: {}\n\tHistory: {}'.format(self.card_bucket, self.history)


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
        print('Beginning training...')
        deck = get_deck()
        for i in trange(iterations):
            np.random.shuffle(deck)
            self.iterate(deck)
        with open(SAVE_PATH, 'wb') as f:
            pickle.dump(self.nodes, f)

    def iterate(self, deck, history=ActionHistory([]), weights=[1, 1]):
        player = history.whose_turn()
        opponent = 1 - player

        if history.hand_over():
            return self.terminal_utility(deck, history)

        infoset = InfoSet(deck, history)
        if infoset not in self.nodes:
            self.nodes[infoset] = Node(infoset)
        node = self.nodes[infoset]

        player_weight = weights[player]
        opponent_weight = weights[opponent]

        strategy = node.current_strategy(player_weight)
        utility = {}
        node_utility = 0
        for action in infoset.legal_actions():
            next_history = history + action
            if player == 0:
                utility[action] = -self.iterate(deck, next_history, p0*strategy[action], p1)
            elif player == 1:
                utility[action] = - self.cfrplus(deck, next_history, p0, p1*strategy[action])
            node_utility += strategy[action] * utility[action]

        for action in infoset.legal_actions():
            regret = utility[action] - node_utility
            node.add_regret(action, opponent_weight * regret)
        return node_utility


    def terminal_utility(self, deck, bet_history):
        raise NotImplementedError


if __name__ == '__main__':
    t = Trainer()
    t.train(int(1e2))
