import numpy as np
import pdb
import progressbar

S = 5   # Number of soldiers on each team
N = 3   # Number of battlefields
ACTIONS = ('500', '410', '401', '311', '302', '320', '203', '230', '212', '221',
           '104', '140', '131', '113', '122', '050', '005', '041', '014', '023',
           '032')

def init_progress_bar(iterations):
    widgets = [' ', progressbar.Percentage(),
               ' ', progressbar.Bar(),
               ' ', progressbar.ETA()]
    return progressbar.ProgressBar(widgets=widgets, max_value=iterations)


class Trainer:

    def __init__(self):
        self.regret_sum = np.zeros(len(ACTIONS))
        self.strategy_sum = np.zeros(len(ACTIONS))
        self.opp_regret_sum = self.regret_sum.copy()
        self.opp_strategy_sum = self.strategy_sum.copy()

    def get_strategy(self):
        strategy = self.regret_sum.copy()
        strategy[strategy < 0] = 0
        if np.sum(strategy) == 0:
            strategy = np.ones(len(ACTIONS)) / len(ACTIONS)
        else:
            strategy /= np.sum(strategy)
        self.strategy_sum += strategy
        return strategy

    def get_opp_strategy(self):
        strategy = self.opp_regret_sum.copy()
        strategy[strategy < 0] = 0
        if np.sum(strategy) == 0:
            strategy = np.ones(len(ACTIONS)) / len(ACTIONS)
        else:
            strategy /= np.sum(strategy)
        self.opp_strategy_sum += strategy
        return strategy

    def get_opp_action(self):
        strategy = self.get_opp_strategy()
        return np.random.choice(ACTIONS, p=strategy)

    def get_action(self):
        strategy = self.get_strategy()
        return np.random.choice(ACTIONS, p=strategy)

    def train(self, iterations):
        action_utils = np.zeros(len(ACTIONS))
        bar = init_progress_bar(iterations)
        bar.start()
        for i in range(iterations):
            my_action = self.get_action()
            opp_action = self.get_opp_action()
            util = utility(my_action, opp_action)
            opp_util = -util
            for j, action in enumerate(ACTIONS):
                self.regret_sum[j] += utility(action, opp_action) - util
                self.opp_regret_sum[j] += utility(action, my_action) - opp_util
            bar.update(i+1)
        bar.finish()

    def get_average_strategy(self):
        if np.sum(self.strategy_sum) > 0:
            return self.strategy_sum / np.sum(self.strategy_sum)
        else:
            return np.ones(len(ACTIONS)) / len(ACTIONS)

    def get_opp_average_strategy(self):
        if np.sum(self.opp_strategy_sum) > 0:
            return self.opp_strategy_sum / np.sum(self.opp_strategy_sum)
        else:
            return np.ones(len(ACTIONS)) / len(ACTIONS)


def utility(player_action, opponent_action):
    num_won = 0
    num_lost = 0
    for battlefield in range(N):
        play = int(player_action[battlefield])
        opp_play = int(opponent_action[battlefield])
        if play > opp_play:
            num_won += 1
        elif play < opp_play:
            num_lost += 1
    if num_won > num_lost:
        return 1
    elif num_won < num_lost:
        return -1
    else:
        return 0












