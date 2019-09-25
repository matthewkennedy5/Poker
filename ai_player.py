import pickle
import numpy as np
from pypokerengine.players import BasePokerPlayer
from trainer import InfoSet, ActionHistory, Node
from hand_abstraction import PreflopAbstraction, FlopAbstraction, TurnAbstraction, RiverAbstraction

PREFLOP_ABSTRACTION = PreflopAbstraction()
FLOP_ABSTRACTION = FlopAbstraction()
TURN_ABSTRACTION = TurnAbstraction()
RIVER_ABSTRACTION = RiverAbstraction()
nodes = pickle.load(open('blueprint.pkl', 'rb'))


# TODO: Other streets besides preflop

def translate_card(card):
    return card[1] + card[0].lower()


def translate_actions(round_state):
    big_blind = 2 * round_state['small_blind_amount']
    preflop = []
    for i, action in enumerate(round_state['action_histories']['preflop']):
        if action['action'] == 'call':
            if i == 0:
                preflop.append('limp')
            else:
                preflop.append('call')
        elif action['action'] == 'raise':
            # Translate raise amounts
            bet = action['amount']
            if bet < 5 * big_blind:
                preflop.append('raise')
            if bet < 18 * big_blind:
                preflop.append('3-bet')
            elif bet < 54 * big_blind:
                preflop.append('4-bet')
            else:
                preflop.append('all-in')

    # TODO: Also translate the actions for other streets.
    return ActionHistory(
        preflop=preflop
    )


def translate_infoset(hole_cards, round_state):
    hole_cards = [translate_card(card) for card in hole_cards]
    board = [translate_card(card) for card in round_state['community_card']]
    player = 1 - round_state['next_player']
    if player == 1:
        deck = hole_cards + [None, None] + board
    else:
        deck = [None, None] + hole_cards + board

    history = translate_actions(round_state)
    return InfoSet(deck, history)


def sample_action(strategy, round_state):
    # Sample an action in my format ('3-bet' etc)
    possible_actions = list(strategy.keys())
    probabilities = [strategy[action] for action in possible_actions]
    choice = np.random.choice(possible_actions, p=probabilities)

    # Translate the action to the PyPokerGUI format, including the chip amounts
    # Basically do the inverse of translate_actions()
    big_blind = 2 * round_state['small_blind_amount']
    history = translate_actions(round_state)
    stack_sizes = history.stack_sizes()
    player = history.whose_turn()
    prev_amount = round_state['action_histories']['preflop'][-1]['amount']
    if choice == 'fold':
        action = 'fold'
        amount = 0
    elif choice == 'limp':
        action = 'call'
        amount = big_blind
    elif choice == 'call':
        action = 'call'
        amount = stack_sizes[player] - stack_sizes[1-player]
    elif choice == 'raise':
        action = 'raise'
        amount = 3 * big_blind
    elif choice == '3-bet':
        action = 'raise'
        amount = 3 * prev_amount
    elif choice == '4-bet':
        action = 'raise'
        action = 3 * prev_amount
    elif choice == 'all-in':
        action = 'raise'
        amount = stack_sizes[player]
    else:
        raise ValueError('Unknown action "{}"'.format(choice))

    return action, amount


# Returns the amount of chips the player has to put in the pot to perform the
# given action.
# def get_amount(action, round_state):
#     big_blind = 2 * round_state['small_blind_amount']
#     history = translate_infoset(None, round_state).history
#     if action == 'fold':
#         return 0
#     elif action == 'limp':
#         return big_blind
#     elif action == 'call':
#     elif action == 'raise':



class Spartacus(BasePokerPlayer):

    def declare_action(self, valid_actions, hole_cards, round_state):
        # valid_actions format => [raise_action_info, call_action_info, fold_action_info]
        # Translate the round_state etc to an information set
        infoset = translate_infoset(hole_cards, round_state)
        if infoset.history.street() != 'preflop':
            raise ValueError('Preflop only right now')
        # Lookup the information set in the nodes
        node = nodes[infoset]
        strategy = node.cumulative_strategy()
        # Sample an action from the strategy
        action, amount = sample_action(strategy, round_state)

        # Print info for debugging purposes
        print()
        print(hole_cards)
        print(strategy)
        print(action, amount)
        return action, amount

    def receive_game_start_message(self, game_info):
        pass

    def receive_round_start_message(self, round_count, hole_card, seats):
        pass

    def receive_street_start_message(self, street, round_state):
        pass

    def receive_game_update_message(self, action, round_state):
        pass

    def receive_round_result_message(self, winners, hand_info, round_state):
        pass


def setup_ai():
    return Spartacus()

