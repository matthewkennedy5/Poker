import os
import json
import numpy as np
import seaborn as sns
from matplotlib import pyplot as plt

# Example preflop chart:
# https://www.google.com/search?sxsrf=AJOqlzXtVL4Mg02k83tjr3oofuk1Cn0vnw:1676914735727&q=preflop+chart&tbm=isch&source=univ&fir=_dtnhgAPl74M7M%252CXZwV9Ml7HySfNM%252C_%253BdwseKxdsE8ZsiM%252CQULlOHNTW1SNfM%252C_%253B-cAvj6CDNLNDfM%252C42nb4DkGlriWNM%252C_%253BaPNB1oxQs9wXqM%252CCuKvWsnTGb901M%252C_%253BGHEbGDYhKzIfeM%252C9y5op-86OQlIuM%252C_%253BnyOsjl8vyDeltM%252C9kVi3oskCqEUHM%252C_%253ByzkiclE-fg7HeM%252Cq62KLVXgU3YmFM%252C_%253BiwjXVjJjD2pgVM%252CoxkAcm7670gj2M%252C_%253BSMpTY2t4HSJs8M%252CRnTunq_nD-oJ5M%252C_%253B5WFpDTDApgobpM%252CdUNCGyfEcShgtM%252C_%253BBetl5kDq0azwSM%252C9kVi3oskCqEUHM%252C_%253BDRDAzxyb9Jok3M%252CQg5NDCz1BE8muM%252C_%253BCzGo1NTUWBQuJM%252C-SvGtHwQNPh2dM%252C_%253Bx44dK9IS4MTpcM%252CWOazE2PZihgDNM%252C_%253BORCTsR2tyTt_NM%252C9kVi3oskCqEUHM%252C_&usg=AI4_-kQ3cLo8hD5pIWyvaj3U5oHmcoAeWQ&sa=X&ved=2ahUKEwjkx5CZ0qT9AhUb57sIHf5FCqQQjJkEegQICBAC&biw=896&bih=960&dpr=2#imgrc=_dtnhgAPl74M7M


# Run the Rust code to write the preflop strategy to a JSON file
os.chdir('rust')
os.system('cargo run --release --bin preflop-chart')

with open('products/preflop_strategy.json', 'r') as f:
    strategy = json.load(f)

# Check that the total probability of each node is 1
for hand in strategy:
    total = sum(strategy[hand].values())
    assert np.abs(total - 1) < 0.1, f'Hand {hand} has a total probability of {total}'


cards = 'A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'
shape = (len(cards), len(cards))
fold_chart = np.zeros(shape)
raise_chart = np.zeros(shape)
all_in_chart = np.zeros(shape)
hand_labels = np.empty(shape, dtype=object)
for i, card1 in enumerate(cards):
    for j, card2 in enumerate(cards):
        suited = 's' if i < j else 'o'
        hand = ''
        hand_strat = {}
        try:
            hand = card1 + card2 + suited
            hand_strat = strategy[hand]
        except KeyError:
            hand = card2 + card1 + suited
            hand_strat = strategy[hand]

        # Don't include 'suited' or 'off-suit' in the label for pocket pairs
        hand_labels[i, j] = hand[:-1] if i == j else hand

        fold_chart[i, j] = hand_strat['fold 0']
        all_in_chart[i, j] = hand_strat['bet 20000']
        for key in hand_strat:
            if 'bet' in key or 'call' in key:
                raise_chart[i, j] += hand_strat[key]
        

assert(np.allclose(fold_chart + raise_chart, 1))


# Draw and save the charts
def draw_chart(data, label):
    sns.set(font_scale=1.2)
    fig, ax = plt.subplots(figsize=(10, 7))
    sns.heatmap(data, annot=hand_labels, fmt='', linewidths=.5, ax=ax, xticklabels=[], yticklabels=[], cmap='coolwarm')
    ax.set_title(f'Opening {label} probabilities - Dealer')
    path = f"products/preflop_charts/{label}_chart.png"
    plt.savefig(path)
    os.system(f'open {path} &')


charts = fold_chart, raise_chart, all_in_chart
labels = 'fold', 'raise', 'all_in'
for c, l in zip(charts, labels):
    draw_chart(c, l)
