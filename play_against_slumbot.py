# The API utilizes HTTP POST requests.  Requests and responses have a JSON body.
# There are two endpoints:
#   /api/new_hand
#   /api/act
# To initiate a new hand, send a request to /api/new_hand.  To take an action, send a
# request to /api/act.
#
# The body of a sample request to /api/new_hand:
#   {"token": "a2f42f44-7ff6-40dd-906b-4c2f03fcee57"}
# The body of a sample request to /api/act:
#   {"token": "a2f42f44-7ff6-40dd-906b-4c2f03fcee57", "incr": "c"}
#
# A sample response from /api/new_hand or /api/act:
#   {'old_action': '', 'action': 'b200', 'client_pos': 0, 'hole_cards': ['Ac', '9d'], 'board': [], 'token': 'a2f42f44-7ff6-40dd-906b-4c2f03fcee57'}
#
# Note that if the bot is first to act, then the response to /api/new_hand will contain the
# bot's initial action.
#
# A token should be passed into every request.  With the exception that on the initial request to
# /api/new_hand, the token may be missing.  But all subsequent requests should contain a token.
# The token can in theory change over the course of a session (usually only if there is a long
# pause) so always check if there is a new token in a response and use it going forward.
#
# Sample action that you might get in a response looks like this:
#   b200c/kk/kk/kb200
# An all-in can contain streets with no action.  For example:
#   b20000c///
#
# Slumbot plays with blinds of 50 and 100 and a stack size of 200 BB (20,000 chips).  The stacks
# reset after each hand.

import requests
import re
import json
import sys
import os
import argparse
import time
from tqdm import trange, tqdm
import multiprocessing as mp
import numpy as np

host = 'slumbot.com'

NUM_STREETS = 4
SMALL_BLIND = 50
BIG_BLIND = 100
STACK_SIZE = 20000

PRINT = True

def ParseAction(action):
    """
    Returns a dict with information about the action passed in.
    Returns a key "error" if there was a problem parsing the action.
    pos is returned as -1 if the hand is over; otherwise the position of the player next to act.
    street_last_bet_to only counts chips bet on this street, total_last_bet_to counts all
      chips put into the pot.
    Handles action with or without a final '/'; e.g., "ck" or "ck/".
    """
    st = 0
    street_last_bet_to = BIG_BLIND
    total_last_bet_to = BIG_BLIND
    last_bet_size = BIG_BLIND - SMALL_BLIND
    last_bettor = 0
    sz = len(action)
    pos = 1
    if sz == 0:
        return {
            'st': st,
            'pos': pos,
            'street_last_bet_to': street_last_bet_to,
            'total_last_bet_to': total_last_bet_to,
            'last_bet_size': last_bet_size,
            'last_bettor': last_bettor,
        }

    check_or_call_ends_street = False
    i = 0
    while i < sz:
        if st >= NUM_STREETS:
            return {'error': 'Unexpected error'}
        c = action[i]
        i += 1
        if c == 'k':
            if last_bet_size > 0:
                return {'error': 'Illegal check'}
            if check_or_call_ends_street:
	        # After a check that ends a pre-river street, expect either a '/' or end of string.
                if st < NUM_STREETS - 1 and i < sz:
                    if action[i] != '/':
                        return {'error': 'Missing slash'}
                    i += 1
                if st == NUM_STREETS - 1:
	            # Reached showdown
                    pos = -1
                else:
                    pos = 0
                    st += 1
                street_last_bet_to = 0
                check_or_call_ends_street = False
            else:
                pos = (pos + 1) % 2
                check_or_call_ends_street = True
        elif c == 'c':
            if last_bet_size == 0:
                return {'error': 'Illegal call'}
            if total_last_bet_to == STACK_SIZE:
	        # Call of an all-in bet
	        # Either allow no slashes, or slashes terminating all streets prior to the river.
                if i != sz:
                    for st1 in range(st, NUM_STREETS - 1):
                        if i == sz:
                            return {'error': 'Missing slash (end of string)'}
                        else:
                            c = action[i]
                            i += 1
                            if c != '/':
                                return {'error': 'Missing slash'}
                if i != sz:
                    return {'error': 'Extra characters at end of action'}
                st = NUM_STREETS - 1
                pos = -1
                last_bet_size = 0
                return {
                    'st': st,
                    'pos': pos,
                    'street_last_bet_to': street_last_bet_to,
                    'total_last_bet_to': total_last_bet_to,
                    'last_bet_size': last_bet_size,
                    'last_bettor': last_bettor,
                }
            if check_or_call_ends_street:
	        # After a call that ends a pre-river street, expect either a '/' or end of string.
                if st < NUM_STREETS - 1 and i < sz:
                    if action[i] != '/':
                        return {'error': 'Missing slash'}
                    i += 1
                if st == NUM_STREETS - 1:
	            # Reached showdown
                    pos = -1
                else:
                    pos = 0
                    st += 1
                street_last_bet_to = 0
                check_or_call_ends_street = False
            else:
                pos = (pos + 1) % 2
                check_or_call_ends_street = True
            last_bet_size = 0
            last_bettor = -1
        elif c == 'f':
            if last_bet_size == 0:
                return {'error', 'Illegal fold'}
            if i != sz:
                return {'error': 'Extra characters at end of action'}
            pos = -1
            return {
                'st': st,
                'pos': pos,
                'street_last_bet_to': street_last_bet_to,
                'total_last_bet_to': total_last_bet_to,
                'last_bet_size': last_bet_size,
                'last_bettor': last_bettor,
            }
        elif c == 'b':
            j = i
            while i < sz and action[i] >= '0' and action[i] <= '9':
                i += 1
            if i == j:
                return {'error': 'Missing bet size'}
            try:
                new_street_last_bet_to = int(action[j:i])
            except (TypeError, ValueError):
                return {'error': 'Bet size not an integer'}
            new_last_bet_size = new_street_last_bet_to - street_last_bet_to
            # Validate that the bet is legal
            remaining = STACK_SIZE - total_last_bet_to
            if last_bet_size > 0:
                min_bet_size = last_bet_size
	        # Make sure minimum opening bet is the size of the big blind.
                if min_bet_size < BIG_BLIND:
                    min_bet_size = BIG_BLIND
            else:
                min_bet_size = BIG_BLIND
            # Can always go all-in
            if min_bet_size > remaining:
                min_bet_size = remaining
            if new_last_bet_size < min_bet_size:
                return {'error': 'Bet too small'}
            max_bet_size = remaining
            if new_last_bet_size > max_bet_size:
                return {'error': 'Bet too big'}
            last_bet_size = new_last_bet_size
            street_last_bet_to = new_street_last_bet_to
            total_last_bet_to += last_bet_size
            last_bettor = pos
            pos = (pos + 1) % 2
            check_or_call_ends_street = True
        else:
            return {'error': 'Unexpected character in action'}

    return {
        'st': st,
        'pos': pos,
        'street_last_bet_to': street_last_bet_to,
        'total_last_bet_to': total_last_bet_to,
        'last_bet_size': last_bet_size,
        'last_bettor': last_bettor,
    }


def NewHand(token):
    data = {}
    if token:
        data['token'] = token
    # Use verify=false to avoid SSL Error
    # If porting this code to another language, make sure that the Content-Type header is
    # set to application/json.
    response = requests.post(f'https://{host}/api/new_hand', headers={}, json=data)
    success = getattr(response, 'status_code') == 200
    if not success:
        print('Status code: %s' % repr(response.status_code))
        try:
            print('Error response: %s' % repr(response.json()))
        except ValueError:
            pass
        sys.exit(-1)

    try:
        r = response.json()
    except ValueError:
        print('Could not get JSON from response')
        sys.exit(-1)

    if 'error_msg' in r:
        print('Error: %s' % r['error_msg'])
        sys.exit(-1)
        
    return r


def Act(token, action):
    data = {'token': token, 'incr': action}
    # Use verify=false to avoid SSL Error
    # If porting this code to another language, make sure that the Content-Type header is
    # set to application/json.
    response = requests.post(f'https://{host}/api/act', headers={}, json=data)
    success = getattr(response, 'status_code') == 200
    if not success:
        print('Status code: %s' % repr(response.status_code))
        try:
            print('Error response: %s' % repr(response.json()))
        except ValueError:
            pass
        sys.exit(-1)

    try:
        r = response.json()
    except ValueError:
        print('Could not get JSON from response')
        sys.exit(-1)

    if 'error_msg' in r:
        print('Error: %s' % r['error_msg'])
        raise ValueError
        
    return r


def TestTranslateAction():
    result = [
        {'action': 'Bet', 'amount': 250},
        {'action': 'Call', 'amount': 250},
        {'action': 'Bet', 'amount': 19750}
    ]
    assert(TranslateAction("b250c/b19750") == (result, 0))


def TranslateAction(action):
    """Translates the Slumbot action format to Optimus"""
    slumbot_history = action.split('/')
    optimus_history = []
    streets = 'preflop', 'flop', 'turn', 'river'
    for street, history in zip(streets, slumbot_history):
        actions = re.findall(r"([ck]|b\d+)", history)   # Thanks GPT 4
        street_bets = [0, 0]
        player = 1 if street == 'preflop' else 0
        for action in actions:
            if action[0] == 'b':
                amount = int(action[1:]) - street_bets[player] 
                # The Optimus history includes the blinds in the preflop bet sizes, but slumbot
                # treats them separately, so we have to adjust for that here
                action = {'action': 'Bet', 'amount': amount}
            elif action == 'c':
                if len(optimus_history) == 0:
                    to_call = BIG_BLIND
                else:
                    to_call = street_bets[1 - player] - street_bets[player]
                action = {'action': 'Call', 'amount': to_call}
            elif action == 'k':
                to_call = 0
                if len(optimus_history) == 1:
                    to_call = BIG_BLIND
                action = {'action': 'Call', 'amount': to_call}
            else:
                raise ValueError()
            optimus_history.append(action)
            street_bets[player] += action['amount']
            player = 1 - player

    return optimus_history, street_bets[player]


def BotAction(response):
    """Gets Optimus's action in the given situation."""
    board = response.get('board')
    if len(board) < 5:
        board += ['back'] * (5 - len(board))
    
    optimus_history, bet_so_far = TranslateAction(response.get('action'))

    data = {
        'cpuCards': response.get('hole_cards'),
        'board': board,
        'history': optimus_history 
    }

    while True:
        try:
            response = requests.post('http://localhost/api/bot?', json=data)
            json = response.json()
            break
        except:
            time.sleep(10)

    action = json['action']
    amount = json['amount']
    if action == 'Call' and amount == 0:
        return 'k'
    elif action == 'Call':
        return 'c'
    elif action == 'Bet':
        return f'b{amount + bet_so_far}'
    elif action == 'Fold':
        return 'f'
    else:
        raise ValueError


def PlayHand(token):
    r = NewHand(token)
    # We may get a new token back from /api/new_hand
    new_token = r.get('token')
    if new_token:
        token = new_token
    while True:
        if PRINT:
            print('-----------------')
            print(repr(r))
        action = r.get('action')
        client_pos = r.get('client_pos')
        hole_cards = r.get('hole_cards')
        board = r.get('board')
        winnings = r.get('winnings')
        # print('Action: %s' % action)
        # if client_pos:
            # print('Client pos: %i' % client_pos)
        # print('Client hole cards: %s' % repr(hole_cards))
        # print('Board: %s' % repr(board))
        if winnings is not None:
            if PRINT:
                print('Hand winnings: %i' % winnings)
            return (token, winnings)
        # Need to check or call
        a = ParseAction(action)
        if 'error' in a:
            print('Error parsing action %s: %s' % (action, a['error']))
            sys.exit(-1)
        incr = BotAction(r)

        if PRINT:
            print('Sending incremental action: %s' % incr)
        r = Act(token, incr)
        
    # Should never get here

        
def Login(username, password):
    data = {"username": username, "password": password}
    # If porting this code to another language, make sure that the Content-Type header is
    # set to application/json.
    response = requests.post(f'https://{host}/api/login', json=data)
    success = getattr(response, 'status_code') == 200
    if not success:
        print('Login failed: status code: %s' % repr(response.status_code))
        try:
            print('Error response: %s' % repr(response.json()))
        except ValueError:
            pass
        sys.exit(-1)

    try:
        r = response.json()
    except ValueError:
        print('Could not get JSON from response')
        sys.exit(-1)

    if 'error_msg' in r:
        print('Error: %s' % r['error_msg'])
        sys.exit(-1)
        
    token = r.get('token')
    if not token:
        print('Did not get token in response to /api/login')
        sys.exit(-1)
    return token


def play_hands(num_hands):
    token = None
    scores = []
    for i in trange(num_hands, smoothing=0):
        token, score = PlayHand(token)
        scores.append(score)
    return scores


def main():
    parser = argparse.ArgumentParser(description='Slumbot API example')
    parser.add_argument('--username', type=str)
    parser.add_argument('--password', type=str)
    parser.add_argument('--num_hands', type=int)
    args = parser.parse_args()
    username = args.username
    password = args.password
    if username and password:
        token = Login(username, password)
    else:
        token = None
    
    num_hands = args.num_hands
    num_procs = 100
    hands_per_cpu = int(num_hands / num_procs)
    input =[hands_per_cpu for p in range(num_procs)]

    scores = []
    with mp.Pool(num_procs) as pool:
        for score_list in tqdm(pool.imap(play_hands, input), total=num_hands, smoothing=0):
            scores += score_list

    mean = np.mean(scores) / BIG_BLIND
    std = np.std(scores) / BIG_BLIND
    conf = 1.96 * std / np.sqrt(num_hands)
    print(f'Winnings: {mean} +/- {conf} BB/h')
    
if __name__ == '__main__':
    main()
    # TestTranslateAction()
