# For playing Rhode Island Hold'em
# Rules: https://www.cs.cmu.edu/~gilpin/gsi.html

import functools
import numpy as np
import pdb
from tqdm import trange
import pickle

BET_SIZES = 10, 20, 20
INITIAL_STACK_SIZE = 1000
STRATEGY_DELAY = 250    # How many iterations to wait before starting to keep
                        # track of the cumulative strategy
PREFLOP, FLOP, TURN = range(3)
HIGH_CARD, PAIR, FLUSH, STRAIGHT, THREE_OF_KIND, STRAIGHT_FLUSH = range(6)
N_ACTIONS = 5
ACTIONS = 'fold', 'check', 'call', 'bet', 'raise'
FOLD, CHECK, CALL, BET, RAISE = range(5)
SAVE_PATH = 'rhode_island_nodes.pkl'
FLOP_CARD = 2
TURN_CARD = 3

RANKS = {'A': 14, 'K': 13, 'Q': 12, 'J': 11, 'T': 10, '9': 9, '8': 8, '7': 7,
         '6': 6, '5': 5, '4': 4, '3': 3, '2': 2}
SUITS = ('c', 'd', 'h', 's')


### Functions ###

def get_deck():
    """Returns the standard 52-card deck, represented as a list of strings."""
    return [rank + suit for suit in SUITS for rank in RANKS]

def game_is_over(bet_history):
    """Returns True if the hand has reached a terminal state (fold or showdown)."""
    return get_street(bet_history)[1]

def get_street(bet_history):
    """Returns the current street given by the bet history.

    If bets are completed for a street, then this method will return the next
    street.

    Inputs:
        bet_history - A list of previous bets in the hand.

    Returns:
        street - one of PREFLOP, FLOP, or TURN
        game_is_over - Whether the hand has reached a terminal state (fold or showdown)
    """
    street = PREFLOP
    previous_bet = None
    for bet in bet_history:
        if bet == 'fold':
            return street, True
        if bet == 'call' or (bet == 'check' and previous_bet == 'check'):
            street += 1
            previous_bet = None
        else:
            previous_bet = bet
        if street > TURN:
            return street, True
    return street, False  # Reached the end of the bet history without the game being over

# TODO: There's an ante that both players have to post. Add that, otherwise a Nash
# equilibrium is just checking and folding.
def pot_size(bet_history):
    """Returns the number of chips in the pot given this bet history."""
    pot = 0
    street = PREFLOP
    previous_action = None
    for action in bet_history:
        if street > TURN:
            raise ValueError('Bet history is too long: %s' % (bet_history,))
        if action == 'bet' or action == 'call':
            pot += BET_SIZES[street]
        elif action == 'raise':
            pot += 2 * BET_SIZES[street]

        if action == 'call' or (action == 'check' and previous_action == 'check'):
            street += 1
            previous_action = None
        else:
            previous_action = action
    return pot

### Classes ###

@functools.total_ordering
class Card:
    """Class for representing a card using the format '8c', 'Th', etc.

    Example:

    card = Card('9d')
    card2 = Card('Th')
    card2 > card1 == True

    Attributes:
        self.suit - The suit of the card, represented by 'h', 'c', etc.
        self.rank - The rank of the card, given as an integer, so 'A' -> 14

    Input:
        card_str - Input string in the standard card format '2d', 'Jh', etc.

    Throws:
        ArgumentError if the input string is not in the correct format.
    """

    def __init__(self, card_str):
        if card_str[0] not in RANKS or card_str[1] not in SUITS:
            raise ValueError('card_str must be in the format like "Kc", "4h"')
        self.card_str = card_str
        self.rank = RANKS[self.card_str[0]]
        self.suit = self.card_str[1]

    def __eq__(self, other):
        return self.card_str[0] == other.card_str[0]

    def __lt__(self, other):
        return self.rank < other.rank

    def __hash__(self):
        # Simple hash function--return the memory address of the object.
        return id(self)

    def __str__(self):
        return self.card_str


@functools.total_ordering
class RhodeHand:
    """Represents a 3-card hand for Rhode Island Poker.

    Rhode Island Poker hands have different rankings than standard 5-card poker,
    as follows:

        Straight Flush
        Three of a Kind
        Straight
        Flush
        Pair
        High Card

    Inputs:
        card0, card1, card2 -  Three cards represented like '8c', 'Qs', etc.
    """

    def __init__(self, card0, card1, card2):
        self.cards = [Card(card) for card in (card0, card1, card2)]
        self.type = None
        self.rank = None
        self.classify()

    def classify(self):
        self.rank = self.max_rank()
        if self.is_straight_flush():
            self.type = STRAIGHT_FLUSH
        elif self.is_three_of_kind():
            self.type = THREE_OF_KIND
        elif self.is_straight():
            self.type = STRAIGHT
        elif self.is_flush():
            self.type = FLUSH
        elif self.is_pair():
            self.type = PAIR
        else:
            self.type = HIGH_CARD

    def max_rank(self):
        highest_rank = 2
        for card in self.cards:
            if card.rank > highest_rank:
                highest_rank = card.rank
        return highest_rank

    def is_straight_flush(self):
        return self.is_straight() and self.is_flush()

    def is_three_of_kind(self):
        return self.cards[0].rank == self.cards[1].rank == self.cards[2].rank

    def is_straight(self):
        sorted_ranks = sorted([card.rank for card in self.cards])
        if RANKS['A'] in sorted_ranks:
            # Account for ace low straights, where sorted_ranks = [2, 3, 14]
            return sorted_ranks == [12, 13, 14] or sorted_ranks == [2, 3, 14]
        else:
            return (sorted_ranks[0] + 1 == sorted_ranks[1] and sorted_ranks[1] + 1 == sorted_ranks[2])

    def is_flush(self):
        return self.cards[0].suit == self.cards[1].suit == self.cards[2].suit

    def is_pair(self):
        return (self.cards[0].rank == self.cards[1].rank
                or self.cards[1].rank == self.cards[2].rank
                or self.cards[0].rank == self.cards[2].rank)

    def __lt__(self, other):
        if self.type == other.type:
            if self.rank == other.rank:
                # If the kicker is what determines the hand
                our_ranks = sorted(card.rank for card in self.cards)
                other_ranks = sorted(card.rank for card in other.cards)
                for i in range(len(our_ranks)):
                    if our_ranks[i] != other_ranks[i]:
                        return our_ranks[i] < other_ranks[i]
                return False    # The hand ranks are totally equivalent
            return self.rank < other.rank
        else:
            return self.type < other.type

    def __eq__(self, other):
        if self.type != other.type:
            return False
        our_ranks = sorted(card.rank for card in self.cards)
        other_ranks = sorted(card.rank for card in other.cards)
        return our_ranks == other_ranks

    def __str__(self):
        return ' '.join([str(card) for card in self.cards])



class Game:

    def __init__(self):
        self.pot = 0
        self.player1_card = None
        self.player2_card = None
        self.board = []
        self.street = PREFLOP
        self.hand_is_over = False
        self.player_folded = False
        self.deck = get_deck()
        self.stacks = [INITIAL_STACK_SIZE, INITIAL_STACK_SIZE]

    def play(self):
        """Initiate a sequence of hands for human vs. human play."""
        print("Welcome to Rhode Island Hold'em!")
        while not self.hand_is_over:
            self.advance_hand()
            self.betting()
            print()
            self.street += 1
        if not self.player_folded:
            self.showdown()
        print(self.stacks)

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
        self.player1_card = self.deck[0]
        self.player2_card = self.deck[1]
        print("Player 1's card:", self.player1_card)
        print("Player 2's card:", self.player2_card)

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
            player_action = self.input_action('Player ' + str(player+1), action, n_raises == 3)
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
        player1_hand = RhodeHand(self.player1_card, self.board[0], self.board[1])
        player2_hand = RhodeHand(self.player2_card, self.board[0], self.board[1])
        if player1_hand > player2_hand:
            self.stacks[0] += self.pot
        elif player2_hand > player1_hand:
            self.stacks[1] += self.pot
        elif player1_hand == player2_hand:
            self.stacks[0] += self.pot / 2
            self.stacks[1] += self.pot / 2

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


class InfoSet:
    """Represents all of everything a player can know about the game.

    All attributes that don't exist yet in the game (like the turn bets if we're
    on the flop) are None.

    Attributes:
        self.hole - The player's hole card
        self.flop - The flop card (None if not dealt yet)
        self.turn - The turn card (None if not dealt yet)
        self.bet_history - Contains the betting history up to this point.

    Inputs:
        deck - The shuffled deck of cards, the first few of which
            characterize this information set (not including the opponent's card)
        bet_history - All of the bets up to this point in the hand.
    """

    def __init__(self, deck, bet_history):
        self.bet_history = bet_history
        player = len(bet_history) % 2
        hole = deck[player]
        flop = deck[2]
        turn = deck[3]
        street = get_street(bet_history)[0]
        self.cards = [hole]
        if street >= FLOP:
            self.cards.append(flop)
        if street >= TURN:
            self.cards.append(turn)

    # TODO: I'm sometimes using the BET form to store actions, and sometimes
    # using 'bet'. The first form is more compact and works well with numpy, but
    # is less human-readable.
    def legal_actions(self):
        """Returns the legal next actions at this infoset.

        Three possibilities here:
        1. First to act, or previous player checked: check or bet
        2. Previous player bet (<3 bets): fold, call, or raise
        3. Previous player bet (>3 bets): fold or call.
        """
        if len(self.bet_history) == 0 or self.bet_history[-1] == 'check' or self.bet_history[-1] == 'call':
            return [CHECK, BET]
        if self.bet_history[-1] == 'bet' or self.bet_history[-1] == 'raise':
            if self.bet_history.count('raise') >= 2:    # Max 3 bets (2 raises) allowed
                return [FOLD, CALL]
            else:
                return [FOLD, CALL, RAISE]

    def __eq__(self, other):
        return self.cards == other.cards and self.bet_history == other.bet_history

    def __hash__(self):
        return hash(str(self))

    def __str__(self):
        return '%s, %s' % (self.cards, self.bet_history)


class CFRPNode:
    """Stores the counterfactual regret and calculates strategies for an infoset."""

    def __init__(self, infoset):
        self.infoset = infoset
        # Not all actions are legal at every node in the game tree. We will just
        # store zeros for illegal action regrets and make sure their probabilities
        # are zero.
        self.regrets = np.zeros(N_ACTIONS)
        self.weighted_strategy_sum = np.zeros(N_ACTIONS)
        self.t = 0      # Number of times this node has been reached by CFR+

    def get_current_strategy(self, probability):
        """Returns the current CFR+ strategy at this node.

        Inputs:
            probability - The probability of reaching this node given the previous history.
                This is the product of the players' strategies of each node up
                until this one in the hand history.

        Returns:
            strategy - A probability distribution over all actions. Illegal actions
                will have zero probability. With enough training, this strategy
                should be an approximate Nash equilibrium.
        """
        legal_actions = self.infoset.legal_actions()
        strategy = np.zeros(N_ACTIONS)
        if np.sum(self.regrets) == 0:
            strategy[legal_actions] = np.ones(N_ACTIONS)[legal_actions]   # Unnormalized uniform distribution
        else:
            strategy[legal_actions] = self.regrets[legal_actions]

        strategy /= np.sum(strategy)
        self.weighted_strategy_sum += np.maximum(self.t - STRATEGY_DELAY, 0) * strategy * probability
        self.t += 1
        return strategy

    def get_cumulative_strategy(self):
        """Returns the weighted average CFR+ strategy at this node.

        In CFR, the average of all strategies over time converges to the Nash
        equilibrium, not the current strategy. This method calculates that
        average but uses a special weighted average described on page 3 of
        http://johanson.ca/publications/poker/2015-ijcai-cfrplus/2015-ijcai-cfrplus.pdf
        and in the CFR+ paper. Given enough training iterations, this strategy
        should approximate a Nash equilibrium.
        """
        legal_actions = self.infoset.legal_actions()
        strategy = np.zeros(N_ACTIONS)
        if np.sum(self.weighted_strategy_sum) == 0:
            strategy[legal_actions] = np.ones(N_ACTIONS)[legal_actions]
        else:
            strategy = self.weighted_strategy_sum
        strategy /= np.sum(strategy)
        return strategy

    def add_regret(self, action, regret):
        """Updates the regret tables at this node by adding the new regret value.

        CFR+ does not allow negative cumulative regrets and sets them to 0.

        Inputs:
            action - The action to add regret to, as an integer (FOLD, BET, etc.)
            regret - The amount of regret to add
        """
        self.regrets[action] = np.maximum(self.regrets[action] + regret, 0)

    def __str__(self):
        return '%s:\n%s' % (self.infoset, str(self.get_cumulative_strategy()))


class CFRPTrainer:
    """Runs CFR+ over the game tree to find an approximate Nash equilibrium."""

    def __init__(self):
        self.nodes = {}     # Each node corresponds to an information set.

    def train(self, iterations):
        """Runs the CFR+ algorithm for the given number of iterations."""
        # TODO: If shuffling 52 cards takes too long, just sample the first 4
        # cards because that's all we need.
        deck = get_deck()
        for i in trange(iterations):
            np.random.shuffle(deck)
            self.cfrplus(deck)
        pickle.dump(self.nodes, open(SAVE_PATH, 'wb'))

    def cfrplus(self, deck, bet_history=[], p0=1, p1=1):
        """Runs an iteration of the CFR+ algorithm on Rhode Island Hold'em.

        Inputs:
            deck - A shuffled 52 card deck. In this implementation, the first
                four deck cards are: [player1 hole, player2 hole, flop, turn]
            bet_history - All the actions up to this point.
            p0 - Prior probability that player 1 reaches the root node.
            p1 - Prior probability that player 2 reaches the root node

        Returns:
            node_utility - The utility of reaching this node in the game tree.
        """
        player = len(bet_history) % 2
        opponent = 1 - player
        # Return terminal utilities if we are at a leaf node of the game tree
        if game_is_over(bet_history):
            return self.terminal_utility(deck, bet_history)

        infoset = InfoSet(deck, bet_history)
        if infoset not in self.nodes:
            self.nodes[infoset] = CFRPNode(infoset)
        node = self.nodes[infoset]

        if player == 0:
            player_weight = p0
            opponent_weight = p1
        else:
            player_weight = p1
            opponent_weight = p0
        strategy = node.get_current_strategy(player_weight)
        utility = np.zeros(N_ACTIONS)
        node_utility = 0
        for action in infoset.legal_actions():
            next_history = bet_history + [ACTIONS[action]]
            if player == 0:
                utility[action] = -self.cfrplus(deck, next_history, p0*strategy[action], p1)
            elif player == 1:
                utility[action] = -self.cfrplus(deck, next_history, p0, p1*strategy[action])
            node_utility += strategy[action] * utility[action]

        # Accumulate counterfactual regret
        for action in infoset.legal_actions():
            regret = utility[action] - node_utility
            node.add_regret(action, opponent_weight * regret)
        return node_utility

    def terminal_utility(self, deck, bet_history):
        """Returns the utility of the current leaf node.

        Inputs:
            deck - 52 card shuffled deck
            bet_history - List of all bets up to this point. Must either have
                a terminal fold or be complete all the way through the turn.

        Returns:
            utility - The utility (chips) won (or lost) for the player.
        """
        player = len(bet_history) % 2
        opponent = 1 - player
        pot = pot_size(bet_history)
        if bet_history[-1] == 'fold':
            return pot / 2
        else:
            # Showdown
            player_hand = RhodeHand(deck[player], deck[2], deck[3])
            opponent_hand = RhodeHand(deck[opponent], deck[2], deck[3])
            if player_hand > opponent_hand:
                return pot / 2
            elif player_hand < opponent_hand:
                return -pot / 2
            elif player_hand == opponent_hand:
                return 0


if __name__ == '__main__':

    infoset = InfoSet(get_deck(), ['check', 'check', 'check', 'bet', 'call'])
    assert(infoset.legal_actions() is not None)
    assert(game_is_over(['check'] * 6) == True)
    trainer = CFRPTrainer()
    trainer.train(1000)
    print(len(trainer.nodes))
    # infoset = InfoSet(hole='8s', bet_history=[['check']])
    # node = CFRPNode(infoset)
    # for i in range(1000):
    #     node.get_current_strategy(probability=1)
    # print(node.get_cumulative_strategy())
    # print(node)
    # game = Game()
    # game.play()

