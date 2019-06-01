# For playing Rhode Island Hold'em
# Rules: https://www.cs.cmu.edu/~gilpin/gsi.html

import functools
import numpy as np
import pdb
import pickle
from rhode_utils import *
from rhode_trainer import InfoSet, CFRPNode

PLAYER, COMPUTER = range(2)

nodes = pickle.load(open(SAVE_PATH, 'rb'))

class Game:

    def __init__(self):
        self.pot = 0
        self.player_card = None
        self.cpu_card = None
        self.computer = 1
        self.board = []
        self.street = PREFLOP
        self.hand_is_over = False
        self.player_folded = False
        self.deck = get_deck()
        self.stacks = [INITIAL_STACK_SIZE, INITIAL_STACK_SIZE]
        self.bet_history = []

    def play(self):
        """Initiate a sequence of hands for human vs. human play."""
        print("Welcome to Rhode Island Hold'em!")
        while True:
            while not self.hand_is_over:
                self.advance_hand()
                self.betting()
                print()
                self.street += 1
            if not self.player_folded:
                self.showdown()
            print('Your stack:', self.stacks[PLAYER])
            print("Computer's stack:", self.stacks[COMPUTER])
            self.hand_is_over = False

    def advance_hand(self):
        if self.street == PREFLOP:
            self.preflop()
        elif self.street == FLOP:
            self.flop()
        elif self.street == TURN:
            self.turn()
            self.hand_is_over = True

    def preflop(self):
        np.random.shuffle(self.deck)
        self.pot = 0
        self.player_card = self.deck[0]
        self.cpu_card = self.deck[1]
        print("Your card:", self.player_card)

    def flop(self):
        self.board.append(self.deck[2])
        print('Flop:', self.board[0])

    def turn(self):
        self.board.append(self.deck[3])
        print('Turn:', self.board[1])

    def betting(self):
        """Process player inputs for a round of betting."""
        if self.street == PREFLOP:
            bet_size = BET_SIZES[0]
        else:
            bet_size = BET_SIZES[1]
        # bet, check
        # fold, call, raise | check, bet

        betting_over = False
        action = False     # Whether a player has bet (as opposed to checking)
        n_raises = 0
        n_checks = 0
        player = 0
        while not betting_over:
            if player == self.computer:
                # get strategy based on information set
                infoset = InfoSet(self.deck, self.bet_history, self.computer)
                # pdb.set_trace()
                node = nodes[infoset]
                strategy = node.get_cumulative_strategy()
                player_action = np.random.choice(ACTIONS, p=strategy)
                print('Computer ' + player_action + 's.')
            else:
                player_action = self.input_action('Player', action, n_raises >= MAX_RAISES)

            self.bet_history.append(player_action)
            if player_action == 'bet':
                self.pot += bet_size
                self.stacks[player] -= bet_size
                action = True
                n_raises += 1
            elif player_action == 'check':
                n_checks += 1
            elif player_action == 'call':
                self.pot += bet_size
                self.stacks[player] -= bet_size
                action = False
                betting_over = True
            elif player_action == 'fold':
                self.hand_is_over = True
                self.player_folded = True
                self.stacks[1 - player] += self.pot
                return
            elif player_action == 'raise':
                self.pot += 2 * bet_size
                self.stacks[player] -= 2 * bet_size
                n_raises += 1

            if not action and (n_checks == 2 or n_raises == 3):
                betting_over = True
            player = 1 - player

    def showdown(self):
        """Gives the pot to the player with the best hand."""
        print('Computer has', self.cpu_card)
        player_hand = RhodeHand(self.player_card, self.board[0], self.board[1])
        computer_hand = RhodeHand(self.cpu_card, self.board[0], self.board[1])
        if player_hand > computer_hand:
            self.stacks[PLAYER] += self.pot
        elif computer_hand > player_hand:
            self.stacks[COMPUTER] += self.pot
        elif player_hand == computer_hand:
            self.stacks[PLAYER] += self.pot / 2
            self.stacks[COMPUTER] += self.pot / 2

    @staticmethod
    def input_action(name, previous_bet, bet_limit_reached):
        """Get a bet input from the user.

        Inputs:
            name - The name of the player
            previous_bet - There has been a bet and the player needs to call,
                raise, or fold
            bet_limit_reached - Whether the max number of bets (3) have already
                been bet and the player can only call or fold.
        """
        allowed_actions = []
        if previous_bet:
            allowed_actions += ['call', 'fold']
            if not bet_limit_reached:
                allowed_actions += ['raise']
        else:
            allowed_actions += ['check', 'bet']

        while True:
            print(name + ' action: ')
            action = input('> ').lower()
            if action in allowed_actions:
                return action
            else:
                actions_string = ', '.join(allowed_actions[:-1]) + ' or ' + allowed_actions[-1] + '.'
                print(actions_string)


if __name__ == '__main__':

    game = Game()
    game.play()

