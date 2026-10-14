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
import multiprocessing as mp
import numpy as np

host = 'slumbot.com'

NUM_STREETS = 4
SMALL_BLIND = 50
BIG_BLIND = 100
STACK_SIZE = 20000

PRINT = False
BOT_URL = 'http://127.0.0.1:8080/api/bot'
# The local bot server must not go through any HTTP proxy from the environment.
LOCAL = requests.Session()
LOCAL.trust_env = False

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
    response = requests.post(f'https://{host}/api/new_hand', headers={}, json=data, timeout=30)
    success = getattr(response, 'status_code') == 200
    if not success:
        print('Status code: %s' % repr(response.status_code))
        try:
            print('Error response: %s' % repr(response.json()))
        except ValueError:
            pass
        raise RuntimeError('new_hand failed: HTTP %s' % response.status_code)

    try:
        r = response.json()
    except ValueError:
        print('Could not get JSON from response')
        raise RuntimeError('new_hand: could not parse JSON')

    if 'error_msg' in r:
        print('Error: %s' % r['error_msg'])
        raise RuntimeError('new_hand error: %s' % r.get('error_msg'))
        
    return r


def Act(token, action):
    data = {'token': token, 'incr': action}
    # Use verify=false to avoid SSL Error
    # If porting this code to another language, make sure that the Content-Type header is
    # set to application/json.
    response = requests.post(f'https://{host}/api/act', headers={}, json=data, timeout=30)
    success = getattr(response, 'status_code') == 200
    if not success:
        print('Status code: %s' % repr(response.status_code))
        try:
            print('Error response: %s' % repr(response.json()))
        except ValueError:
            pass
        raise RuntimeError('act failed: HTTP %s' % response.status_code)

    try:
        r = response.json()
    except ValueError:
        print('Could not get JSON from response')
        raise RuntimeError('act: could not parse JSON')

    if 'error_msg' in r:
        raise ValueError('Slumbot rejected action %r: %s' % (action, r['error_msg']))
        
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


def BotAction(response, record):
    """Gets Optimus's action in the given situation. Appends timing info to record."""
    board = list(response.get('board'))
    if len(board) < 5:
        board += ['back'] * (5 - len(board))

    optimus_history, bet_so_far = TranslateAction(response.get('action'))

    data = {
        'cpuCards': response.get('hole_cards'),
        'board': board,
        'history': optimus_history
    }

    t0 = time.time()
    attempts = 0
    while True:
        try:
            attempts += 1
            resp = LOCAL.post(BOT_URL, json=data, timeout=600)
            json_resp = resp.json()
            break
        except KeyboardInterrupt:
            raise
        except Exception as e:
            if attempts >= 6:
                raise RuntimeError(f'bot server unreachable: {e}')
            time.sleep(10)
    latency = time.time() - t0
    street = len([c for c in board if c != 'back'])
    decision = {'street_cards': street, 'latency': round(latency, 3)}
    # Decision detail for variance reduction (sampling distribution and per-action
    # values); absent for older servers.
    for key in ('source', 'sampled', 'legalized', 'probs', 'values'):
        if key in json_resp:
            decision[key] = json_resp[key]
    record['decisions'].append(decision)

    action = json_resp['action']
    amount = json_resp['amount']
    if action == 'Call' and amount == 0:
        return 'k'
    elif action == 'Call':
        return 'c'
    elif action == 'Bet':
        return f'b{amount + bet_so_far}'
    elif action == 'Fold':
        return 'f'
    else:
        raise ValueError(f'unknown bot action {action}')


def PlayHand(token):
    """Plays one hand. Returns (token, record) where record has the hand details."""
    record = {'decisions': [], 'error': None}
    r = NewHand(token)
    new_token = r.get('token')
    if new_token:
        token = new_token
    while True:
        if PRINT:
            print('-----------------')
            print(repr(r))
        action = r.get('action')
        record['client_pos'] = r.get('client_pos')
        record['hole_cards'] = r.get('hole_cards')
        record['board'] = r.get('board')
        record['action'] = action
        winnings = r.get('winnings')
        if winnings is not None:
            record['winnings'] = winnings
            record['bot_hole_cards'] = r.get('bot_hole_cards')
            return (token, record)
        a = ParseAction(action)
        if 'error' in a:
            raise RuntimeError('Error parsing action %s: %s' % (action, a['error']))
        incr = BotAction(r, record)
        if PRINT:
            print('Sending incremental action: %s' % incr)
        try:
            r = Act(token, incr)
        except ValueError as e:
            raise RuntimeError('%s | hand so far: %s hole=%s board=%s' % (
                e, action, record.get('hole_cards'), record.get('board')))


def Login(username, password):
    data = {"username": username, "password": password}
    response = requests.post(f'https://{host}/api/login', json=data, timeout=30)
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


def summarize(winnings_list, num_failed):
    n = len(winnings_list)
    if n == 0:
        return 'no completed hands'
    arr = np.array(winnings_list, dtype=float) / BIG_BLIND * 1000.0  # mbb per hand
    mean = arr.mean()
    se = arr.std(ddof=1) / np.sqrt(n) if n > 1 else float('nan')
    return (f'{n} hands: {mean:+.1f} +/- {1.96 * se:.1f} mbb/hand '
            f'(95% CI), {num_failed} failed hands')


def load_existing(log_path):
    """Returns the winnings of hands already in the log (for --resume)."""
    winnings = []
    if not log_path or not os.path.exists(log_path):
        return winnings
    with open(log_path) as f:
        for line in f:
            try:
                rec = json.loads(line)
            except ValueError:
                continue
            if rec.get('winnings') is not None:
                winnings.append(rec['winnings'])
    return winnings


def worker(worker_id, num_hands, log_path, bot_url, username, password, stop_event):
    """Plays up to num_hands hands in its own Slumbot session, appending to log_path."""
    global BOT_URL, LOCAL
    BOT_URL = bot_url
    LOCAL = requests.Session()
    LOCAL.trust_env = False
    token = Login(username, password) if username and password else None
    consecutive_failures = 0
    played = 0
    with open(log_path, 'a') as log:
        while played < num_hands and not stop_event.is_set():
            try:
                token, record = PlayHand(token)
            except KeyboardInterrupt:
                return
            except Exception as e:
                consecutive_failures += 1
                log.write(json.dumps({'error': str(e), 'worker': worker_id, 'time': time.time()}) + '\n')
                log.flush()
                if consecutive_failures >= 5:
                    print(f'[worker {worker_id}] too many consecutive failures, stopping: {e}', flush=True)
                    return
                token = None
                time.sleep(5)
                continue
            consecutive_failures = 0
            record['worker'] = worker_id
            record['time'] = time.time()
            log.write(json.dumps(record) + '\n')  # one line per hand, < 4 KB, so appends don't interleave
            log.flush()
            played += 1


def main():
    global BOT_URL, PRINT
    parser = argparse.ArgumentParser(description='Play Optimus against Slumbot')
    parser.add_argument('--username', type=str)
    parser.add_argument('--password', type=str)
    parser.add_argument('--num_hands', type=int, default=1000, help='Total hands across all sessions')
    parser.add_argument('--sessions', type=int, default=1,
                        help='Parallel Slumbot sessions (each is its own process and token)')
    parser.add_argument('--bot_url', type=str, default=BOT_URL)
    parser.add_argument('--log', type=str, default=None,
                        help='JSONL file to append hand records to (default results/slumbot_<timestamp>.jsonl)')
    parser.add_argument('--resume', action='store_true',
                        help='Count hands already in --log towards --num_hands and the summary')
    parser.add_argument('--report_every', type=int, default=25, help='Hands between progress reports')
    parser.add_argument('--verbose', action='store_true')
    args = parser.parse_args()

    BOT_URL = args.bot_url
    PRINT = args.verbose

    log_path = args.log
    if log_path is None:
        os.makedirs('results', exist_ok=True)
        log_path = time.strftime('results/slumbot_%Y%m%d_%H%M%S.jsonl')
    else:
        os.makedirs(os.path.dirname(log_path) or '.', exist_ok=True)
    print(f'Logging hands to {log_path}', flush=True)

    existing = load_existing(log_path) if args.resume else []
    if existing:
        print(f'Resuming: {summarize(existing, 0)}', flush=True)
    remaining = max(0, args.num_hands - len(existing))
    sessions = max(1, args.sessions)
    per_worker = [remaining // sessions + (1 if i < remaining % sessions else 0) for i in range(sessions)]

    stop_event = mp.Event()
    procs = []
    for i in range(sessions):
        if per_worker[i] == 0:
            continue
        p = mp.Process(target=worker, args=(i, per_worker[i], log_path, BOT_URL,
                                            args.username, args.password, stop_event))
        p.start()
        procs.append(p)
        time.sleep(0.5)  # stagger session starts

    start = time.time()
    last_reported = len(existing)
    try:
        while any(p.is_alive() for p in procs):
            time.sleep(5)
            done = count_hands(log_path)
            if done - last_reported >= args.report_every:
                last_reported = done
                winnings, failed = read_log(log_path)
                elapsed = time.time() - start
                rate = (len(winnings) - len(existing)) / (elapsed / 3600.0) if elapsed > 0 else 0.0
                print(f'[{elapsed/60:.1f} min, {rate:.0f} hands/h] {summarize(winnings, failed)}', flush=True)
    except KeyboardInterrupt:
        print('Stopping sessions...', flush=True)
        stop_event.set()
    for p in procs:
        p.join(timeout=60)
    winnings, failed = read_log(log_path)
    print('Final: ' + summarize(winnings, failed), flush=True)
    print(f'Log: {log_path}', flush=True)


def read_log(log_path):
    winnings, failed = [], 0
    if not os.path.exists(log_path):
        return winnings, failed
    with open(log_path) as f:
        for line in f:
            try:
                rec = json.loads(line)
            except ValueError:
                continue
            if rec.get('winnings') is not None:
                winnings.append(rec['winnings'])
            elif rec.get('error'):
                failed += 1
    return winnings, failed


def count_hands(log_path):
    try:
        with open(log_path) as f:
            return sum(1 for _ in f)
    except OSError:
        return 0


if __name__ == '__main__':
    main()
