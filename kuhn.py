import os
import pickle
import time
import numpy as np
import pdb
from tqdm import trange

PASS = 0
BET = 1
NUM_ACTIONS = 2
SAVE_PATH = '/home/matthew/Desktop/Poker/cfr_bad_nodes.pkl'
ITERATIONS = int(5)


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
        cards = [1, 2, 3]
        util = 0
        for i in trange(iterations):
            np.random.shuffle(cards)
            util += self._cfr(cards, '', 1, 1)
        pickle.dump(self.node_map, open(SAVE_PATH, 'wb'))

    def _cfr(self, cards, history, p0, p1):
        player = len(history) % 2
        opponent = 1 - player
        # Return payoff for terminal states
        if game_is_over(history):
            return self.terminal_utility(cards, history)

        info_set = str(cards[player]) + history
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
    def terminal_utility(cards, history):
        if len(history) < 2:
            return 0
        player = len(history) % 2
        opponent = 1 - player
        terminal_pass = history[-1] == 'p'
        double_bet = history[-2:] == 'bb'
        is_player_card_higher = cards[player] > cards[opponent]
        if terminal_pass:
            if history == 'pp':
                if cards[player] > cards[opponent]:
                    return 1
                elif cards[player] == cards[opponent]:
                    return 0
                else:
                    return -1
            else:
                return 1
        elif double_bet:
            if cards[player] > cards[opponent]:
                return 2
            elif cards[player] == cards[opponent]:
                return 0
            else:
                return -2
        else:
            return 0    # no winner or loser yet


def game_is_over(history):
    return len(history) >= 2 and (history[-2:] == 'bb'or history[-1] == 'p')


class Game:

    def __init__(self):
        self.cards = [1, 2, 3]
        self.history = ''
        self.computer = 1
        self.human = 0
        self.node_map = pickle.load(open(SAVE_PATH, 'rb'))
        self.score = 0

    def reset(self):
        self.history = ''
        self.human, self.computer = self.computer, self.human

    def play(self):
        while True:
            np.random.shuffle(self.cards)
            print()
            print('Your card is: %d' % self.cards[self.human])
            while not game_is_over(self.history):
                # time.sleep(1)
                self.play_turn()
            # time.sleep(1)
            print('CPU card: %d' % self.cards[self.computer])
            self.update_score()
            print('Score: %d' % self.score)
            self.reset()

    def update_score(self):
        util = Trainer.terminal_utility(self.cards, self.history)
        if len(self.history) % 2 == self.computer:
            util = -util
        self.score += util

    def input_move(self):
        while True:
            move = input('Pass or bet? (p/b): ')
            if move == 'b' or move == 'p':
                return move

    def play_turn(self):
        player = len(self.history) % 2
        if player == self.human:
            move = self.input_move()
            self.history += move
        else:
            info_set = str(self.cards[self.computer]) + self.history
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
    for node in sorted(node_map):
        print(node_map[node])

if __name__ == '__main__':
    from kuhn import Node, Trainer
    # if not os.path.isfile(SAVE_PATH):
    print('[INFO] Starting training...')
    trainer = Trainer()
    trainer.train(ITERATIONS)
    print_strategy()

    # game = Game()
    # game.play()
