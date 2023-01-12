import os
import pickle
import time
import numpy as np
import pdb
from tqdm import tqdm

import poker_utils

PASS, BET = range(2)
NUM_ACTIONS = 2     # For now, the player can just pass or bet. In this future
                    # this will include various bet sizes.
SAVE_PATH = '/home/matthew/Desktop/Poker/poker_nodes.pkl'

# For the information set representation
HISTORY, HOLE, FLOP, TURN, RIVER = range(5)


class Node:

    def __init__(self, info_set):
        self.info_set = info_set
        self.regret_sum = np.zeros(NUM_ACTIONS)
        self.strategy_sum = np.zeros(NUM_ACTIONS)

    def get_strategy(self, realization_weight):
        strategy = self.regret_sum.copy()
        strategy[strategy < 0] = 0
        if np.sum(strategy) == 0:
            strategy = np.ones(NUM_ACTIONS) / NUM_ACTIONS
        else:
            strategy /= np.sum(strategy)
        self.strategy_sum += strategy * realization_weight
        return strategy

    def get_average_strategy(self):
        if np.sum(self.strategy_sum) > 0:
            return self.strategy_sum / np.sum(self.strategy_sum)
        else:
            return np.ones(NUM_ACTIONS) / NUM_ACTIONS

    def add_regret(self, action, regret):
        self.regret_sum[action] += regret

    def __str__(self):
        return '%s: %s' % (self.info_set, str(self.get_average_strategy()))


class Trainer:

    def __init__(self):
        self.node_map = {}

    def train(self, iterations):
        cards = poker_utils.get_deck()
        util = 0
        for i in tqdm(range(iterations)):
            np.random.shuffle(cards)
            util += self._cfr(tuple(cards), '', 1, 1)
        pickle.dump(self.node_map, open(SAVE_PATH, 'wb'))


    def _cfr(self, cards, history, p0, p1):
        player = len(history) % 2
        opponent = 1 - player
        # Return payoff for terminal states
        info_set, opp_hole = get_info_set(cards, history)
        if game_is_over(history):
            return self.terminal_utility(info_set, opp_hole)

        # Get information set node
        if info_set not in self.node_map:
            self.node_map[info_set] = Node(info_set)
        node = self.node_map[info_set]

        # Recurse
        if player == 0:
            player_weight = p0
            opponent_weight = p1
        else:
            player_weight = p1
            opponent_weight = p0
        strategy = node.get_strategy(player_weight)
        util = np.zeros(NUM_ACTIONS)
        node_util = 0
        for action in range(NUM_ACTIONS):
            if action == PASS:
                next_history = history + 'p'
            else:
                next_history = history + 'b'
            if player == 0:
                util[action] = -self._cfr(cards, next_history,
                                          p0*strategy[action], p1)
            else:
                util[action] = -self._cfr(cards, next_history, p0,
                                          p1*strategy[action])
            node_util += strategy[action] * util[action]
        # Accumulate counterfactual regret
        for action in range(NUM_ACTIONS):
            regret = util[action] - node_util
            node.add_regret(action, opponent_weight * regret)

        return node_util

    @staticmethod
    def terminal_utility(info_set, opponent_hole):
        history = info_set[HISTORY]
        pot = 0
        for ch in history:
            if ch == 'b':
                pot += 1

        opponent_fold = history[-1] == 'p'
        if opponent_fold:
            return pot
        else:   # Showdown
            if info_set[TURN] is None:
                raise ValueError('Showdown happening before betting is over')

            board = [info_set[FLOP], info_set[TURN], info_set[RIVER]]
            my_hand = list(info_set[HOLE]) + board
            opponent_hand = list(opponent_hole) + board
            if my_hand > opponent_hand:
                return pot
            else:
                return -pot
            # TODO: Deal with ties


##### Functions #####

def get_info_set(cards, history):
    player = len(history) % 2

    if player == 0:
        hole = cards[0:2]
        opp_hole = cards[2:4]
    else:
        hole = cards[2:4]
        opp_hole = cards[0:2]

    # TODO: This is wrong. There can be up to 3 moves per street.
    flop = None
    turn = None
    river = None
    if len(history) >= 2:
        # TODO: Order of the flop (and hand cards in general) shouldn't matter
        flop = cards[4:7]
    if len(history) >= 4:
        turn = cards[7]
    if len(history) >= 6:
        river = cards[8]

    info_set = (
        history,
        tuple(hole),
        flop,
        turn,
        river
    )
    return info_set, opp_hole


def street_bets(history):
    # Given a history, return the bets for each street as a dict.
    prev_index = 0
    bets = {}
    for street in ('preflop', 'flop', 'turn', 'river'):
        if history[prev_index:prev_index+2] == 'pb':
            bets[street] = history[prev_index:prev_index+3]
            prev_index += 3
        else:
            bets[street] = history[prev_index:prev_index+2]
            prev_index += 2
    return bets


def game_is_over(history):
    bets = street_bets(history)
    for street in bets:
        if 'bp' in bets[street]:    # Somebody folded
            return True
    return (len(bets['river']) >= 2) or (bets['river'] == 'pp') or (bets['river'] == 'bb')


class Game:

    def __init__(self):
        self.cards = poker_utils.get_deck()
        self.history = ''
        self.computer = 1
        self.human = 0
        self.node_map = pickle.load(open(SAVE_PATH, 'rb'))
        self.winnings = 0

    def reset(self):
        self.history = ''
        self.human, self.computer = self.computer, self.human

    def play(self):
        while True:
            np.random.shuffle(self.cards)
            print()
            print('Your hand is: ')  # Human's cards are always the first two
            print(self.cards[0])
            print(self.cards[1])
            print()
            while not game_is_over(self.history):
                # time.sleep(1)
                self.play_turn()
            # time.sleep(1)
            print('CPU cards: %d' % self.cards[2:4])
            self.update_score()
            print('Winnings: $%d' % self.winnings)
            self.reset()

    def update_score(self):
        util = Trainer.terminal_utility(self.cards, self.history)
        if len(self.history) % 2 == self.computer:
            util = -util
        self.winnings += util

    def input_move(self):
        while True:
            move = input('Check/fold or bet? (p/b): ')
            if move == 'b' or move == 'p':
                return move

    def play_turn(self):
        player = len(self.history) % 2
        if player == self.human:
            move = self.input_move()
            self.history += move
        else:
            info_set, _ = get_info_set(self.cards, self.history)
            strategy = self.node_map[info_set].get_average_strategy()
            move = np.random.choice((PASS, BET), p=strategy)
            if move == PASS:
                print('pass')
                self.history += 'p'
            else:
                print('bet')
                self.history += 'b'


def print_strategy():
    node_map = pickle.load(open(SAVE_PATH, 'rb'))
    for node in node_map:
        print(node_map[node])


if __name__ == '__main__':
    from poker_cfr import Node, Trainer
    if not os.path.isfile(SAVE_PATH):
        print('[INFO] Starting training...')
        trainer = Trainer()
        trainer.train(int(1e5))

    game = Game()
    game.play()
