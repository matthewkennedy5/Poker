from flask import Flask
from flask_cors import CORS
from flask import request
import json

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


@app.route('/bot')
def get_cpu_action():
    cpu_cards = request.args.get('cpuCards').split(',')
    board = request.args.get('board').split(',')
    history = request.args.get('history')
    history = json.loads(history)

    # TODO: Get this to link up to the AI

    return {"action": "call", "amount": 600}

