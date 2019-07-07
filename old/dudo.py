# CFR solver for 1v1 Dudo.
import numpy as np
import progressbar
import pdb

# TODO: Why is the strategy always the same?
# TODO: What is the proper outcome when the max bet is called?

NUM_SIDES = 6
NUM_ACTIONS = 2 * NUM_SIDES + 1
DUDO = NUM_ACTIONS - 1
CLAIM_NUM = [1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2]
CLAIM_RANK = [2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6, 1]


def init_progress_bar(iterations):
    widgets = [' ', progressbar.Percentage(),
               ' ', progressbar.Bar(),
               ' ', progressbar.ETA()]
    return progressbar.ProgressBar(widgets=widgets, max_value=iterations)


def roll_dice():
    return np.random.randint(1, NUM_SIDES + 1, size=[2])


def history_to_string(history):
    result = ''
    for action in range(NUM_ACTIONS):
        if history[action]:
            result += '%dx%d, ' % (CLAIM_NUM[action], CLAIM_RANK[action])
    return result


def strongest_claim(history):
    strongest_claim = -1
    for action in range(NUM_ACTIONS-1):
        if history[action]:
            strongest_claim = action
    return strongest_claim


def action_is_valid(history, action):
    if action == DUDO:
        return True
    return not history[action] and action > strongest_claim(history)


class Node:

    def __init__(self, info_set):
        self.info_set = info_set
        self.regret_sum = np.zeros(NUM_ACTIONS)
        self.strategy_sum = np.zeros(NUM_ACTIONS)

    def strategy(self, weight):
        if (self.regret_sum <= 0).all():
            strategy = 1.0 / NUM_ACTIONS * np.ones(NUM_ACTIONS)
        else:
            positive_regrets = self.regret_sum.copy()
            positive_regrets[positive_regrets < 0] = 0
            strategy = positive_regrets / np.sum(positive_regrets)
        self.strategy_sum += weight * strategy
        return strategy

    def average_strategy(self):
        if np.sum(self.strategy_sum) > 0:
            return self.strategy_sum / np.sum(self.strategy_sum)
        else:
            return np.ones(NUM_ACTIONS) / NUM_ACTIONS

    def add_regret(self, regret, action):
        self.regret_sum[action] += regret

    def __str__(self):
        return '%s %s' % (self.info_set, self.average_strategy())


class DudoTrainer:

    def __init__(self):
        self.nodes = {}

    def train(self, iterations):
        bar = init_progress_bar(iterations)
        bar.start()
        utility = 0
        for i in range(iterations):
            dice = roll_dice()
            utility += self._cfr(dice, history=[False for i in range(NUM_ACTIONS)], p0=1, p1=1)
            bar.update(i + 1)
        bar.finish()
        print('Average game value: %f' % (utility/iterations,))
        # for info_set in self.nodes:
        #     print(self.nodes[info_set])


    def _dudo(self, player, history, dice):
        # This is called when the opponent called dudo
        opponent = 1 - player
        claim = strongest_claim(history)
        claim_num = CLAIM_NUM[claim]
        rank = CLAIM_RANK[claim]
        rank_count = np.sum(dice[dice == rank])
        if rank != 1:
            rank_count += np.sum(dice[dice == 1])   # Add wildcard values

        if claim_num < rank_count:
            return 1
        else:
            return -1


    def _cfr(self, dice, history, p0, p1):
        # Return terminal states
        player = np.sum(history) % 2
        # The possible terminal states are DUDO and the strongest claim
        if history[NUM_ACTIONS-2]:
            # TODO: What should happen here?
            return 0
        if history[DUDO]:
            # Showdown
            return self._dudo(player, history, dice)
        # Retrieve / init the node
        info_set = str(dice[player]) + ' ' + history_to_string(history)
        if info_set in self.nodes:
            node = self.nodes[info_set]  # Does this copy or pass by reference?
        else:
            node = Node(info_set)
            self.nodes[info_set] = node
        # Recurse
        node_utility = 0
        utils = np.zeros(NUM_ACTIONS)
        if player == 0:
            strategy = node.strategy(p0)
        else:
            strategy = node.strategy(p1)
        for action in range(NUM_ACTIONS):
            if action_is_valid(history, action):
                next_history = history.copy()
                next_history[action] = True
                if player == 0:
                    utils[action] = -self._cfr(dice, next_history, strategy[action] * p0, p1)
                else:
                    utils[action] = -self._cfr(dice, next_history, p0, strategy[action] * p1)
        # Update regrets
        node_utility = np.dot(utils, strategy)
        for action in range(NUM_ACTIONS):
            regret = utils[action] - node_utility
            if player == 0:
                node.add_regret(p1 * regret, action)
            else:
                node.add_regret(p0 * regret, action)
        return node_utility


if __name__ == '__main__':
    trainer = DudoTrainer()
    trainer.train(int(1e2))
