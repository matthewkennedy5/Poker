from flask import Flask
from flask_cors import CORS
from flask import request

from texas_hands import TexasHand

app = Flask(__name__)
CORS(app)

@app.route('/compare')
def compare_hands():
    human_hand = request.args.get('humanHand')
    cpu_hand = request.args.get('cpuHand')
    human_cards = human_hand.split(',')
    cpu_cards = cpu_hand.split(',')
    human_hand = TexasHand(human_cards)
    cpu_hand = TexasHand(cpu_cards)
    if human_hand > cpu_hand:
        return "human"
    else:
        return "cpu"
