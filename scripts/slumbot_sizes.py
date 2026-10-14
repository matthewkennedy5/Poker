#!/usr/bin/env python3
"""Histograms Slumbot's bet sizes (as a fraction of the pot) per street from the
JSONL logs written by play_against_slumbot.py. Use it to choose the tree sizes in
params.toml [search] so Slumbot's real sizes are in-tree.

Usage: python3 scripts/slumbot_sizes.py results/*.jsonl
"""
import json
import re
import sys
from collections import defaultdict

SB, BB, STACK = 50, 100, 20000


def iter_street_actions(action_str):
    """Yields (street, player, kind, amount_total, pot_before, street_bet_before)."""
    streets = action_str.split('/')
    total = [SB, BB]  # chips committed by player 0 (SB/dealer) and player 1 (BB)
    for street_idx, street in enumerate(streets):
        street_bets = [0, 0]
        player = 0 if street_idx == 0 else 1
        for tok in re.findall(r"([ckf]|b\d+)", street):
            pot_before = total[0] + total[1]
            if tok[0] == 'b':
                to_total = int(tok[1:])
                added = to_total - street_bets[player]
                yield street_idx, player, 'bet', to_total, pot_before, street_bets
                street_bets[player] = to_total
                total[player] += added
            elif tok == 'c':
                added = street_bets[1 - player] - street_bets[player]
                street_bets[player] = street_bets[1 - player]
                total[player] += added
            player = 1 - player


def main(paths):
    sizes = defaultdict(list)
    for path in paths:
        with open(path) as f:
            for line in f:
                try:
                    rec = json.loads(line)
                except ValueError:
                    continue
                if 'action' not in rec or rec.get('client_pos') is None:
                    continue
                # Slumbot API: client_pos 1 = we are the small blind/button, 0 = big blind.
                # Our numbering here: player 0 = small blind, player 1 = big blind.
                slumbot_player = 1 if rec['client_pos'] == 1 else 0
                for street, player, kind, to_total, pot_before, street_bets in iter_street_actions(rec['action']):
                    if player != slumbot_player or kind != 'bet':
                        continue
                    to_call = street_bets[1 - player] - street_bets[player]
                    raise_amount = to_total - street_bets[player] - to_call
                    pot_after_call = pot_before + to_call
                    if to_total >= STACK:
                        sizes[(street, 'allin')].append(1.0)
                    elif pot_after_call > 0:
                        sizes[(street, 'raise' if to_call > 0 else 'bet')].append(raise_amount / pot_after_call)
    names = ['preflop', 'flop', 'turn', 'river']
    for key in sorted(sizes):
        street, kind = key
        vals = sorted(sizes[key])
        n = len(vals)
        if kind == 'allin':
            print(f'{names[street]:8s} all-in: {n}')
            continue
        buckets = defaultdict(int)
        for v in vals:
            buckets[round(v * 4) / 4] += 1  # quarter-pot buckets
        common = sorted(buckets.items(), key=lambda kv: -kv[1])[:6]
        q = lambda p: vals[min(n - 1, int(p * n))]
        print(f'{names[street]:8s} {kind:5s}: n={n:5d} p10={q(0.1):.2f} p50={q(0.5):.2f} p90={q(0.9):.2f} '
              f'most common (pot fraction: count) {", ".join(f"{k:.2f}: {c}" for k, c in common)}')


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    main(sys.argv[1:])
