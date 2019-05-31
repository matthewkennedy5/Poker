import functools
import pdb
import pickle
from tqdm import trange
from rhode_island_holdem import *
from hand_abstraction import prepare_abstraction

abstraction = prepare_abstraction()

### FUNCTIONS ###

# START HERE: player = len(bet_history) % 2 doesn't work. Consider: check, bet, call.
# TODO: Use parameter files!


def print_strategy():
    nodes = pickle.load(open(SAVE_PATH, 'rb'))
    for infoset in nodes:
        print(nodes[infoset])


def get_player(bet_history):
    """Returns the next player to act (0 or 1) based on the bet history."""
    street = PREFLOP
    previous_bet = None
    street_counter = 0
    for bet in bet_history:
        if bet == 'call' or (bet == 'check' and previous_bet == 'check'):
            street += 1
            previous_bet = None
            street_counter = 0
        else:
            street_counter += 1
            previous_bet = bet

    return street_counter % 2

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
        player = get_player(bet_history)
        hole = deck[player]
        flop = deck[2]
        turn = deck[3]
        street = get_street(bet_history)[0]
        self.cards = [hole]
        if street >= FLOP:
            self.cards.append(flop)
        if street >= TURN:
            self.cards.append(turn)

        self.card_bucket = abstraction[''.join(self.cards)]

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
        n_raises = 0
        if len(self.bet_history) == 0 or self.bet_history[-1] == 'check' or self.bet_history[-1] == 'call':
            return [CHECK, BET]
        if self.bet_history[-1] == 'bet' or self.bet_history[-1] == 'raise':
            # START HERE: Only limit raises for the current street, not the whole hand.
            if self.bet_history.count('raise') >= MAX_RAISES:
                return [FOLD, CALL]
            else:
                return [FOLD, CALL, RAISE]

    def __eq__(self, other):
        return (self.card_bucket == other.card_bucket
                and self.bet_history == other.bet_history)

    def __hash__(self):
        # Equivalent informations sets should give the same hash value.
        return self.card_bucket + hash(str(self.bet_history))

    def __str__(self):
        return '%s, %s' % (''.join(self.cards), ', '.join(self.bet_history))


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
        return '%s: %s, %d' % (self.infoset, str(self.get_cumulative_strategy()), self.t)


class CFRPTrainer:
    """Runs CFR+ over the game tree to find an approximate Nash equilibrium."""

    def __init__(self):
        self.nodes = {}     # Each node corresponds to an information set.

    def train(self, iterations):
        """Runs the CFR+ algorithm for the given number of iterations."""
        # TODO: If shuffling 52 cards takes too long, just sample the first 4
        # cards because that's all we need.
        print('[INFO] Beginning training...')
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
        player = get_player(bet_history)
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
        player = get_player(bet_history)
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

    trainer = CFRPTrainer()
    trainer.train(10000)
    print_strategy()
