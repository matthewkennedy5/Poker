import {Random} from "random-js";
import React, {Component} from 'react';

const PREFLOP = 0;
const FLOP = 1;
const TURN = 2;
const RIVER = 3;

// Constants for card placement in deck
const HUMAN_CARD_1 = 0;
const HUMAN_CARD_2 = 1;
const CPU_CARD_1 = 2;
const CPU_CARD_2 = 3;
const FLOP_CARD_1 = 4;
const FLOP_CARD_2 = 5;
const FLOP_CARD_3 = 6;
const TURN_CARD = 7;
const RIVER_CARD = 8;

const DECK = ['2c', '2s', '2h', '2d',
              '3c', '3s', '3h', '3d',
              '4c', '4s', '4h', '4d',
              '5c', '5s', '5h', '5d',
              '6c', '6s', '6h', '6d',
              '7c', '7s', '7h', '7d',
              '8c', '8s', '8h', '8d',
              '9c', '9s', '9h', '9d',
              'Tc', 'Ts', 'Th', 'Td',
              'Jc', 'Js', 'Jh', 'Jd',
              'Qc', 'Qs', 'Qh', 'Qd',
              'Kc', 'Ks', 'Kh', 'Kd',
              'Ac', 'As', 'Ah', 'Ad'];


class Game extends Component {

    constructor(props) {
        super(props);
        this.dealer = "cpu";
        this.street = PREFLOP;
        this.pot = 0;
        this.bets = [0, 0];
        this.deck = DECK;
        this.humanCards = [];
        this.cpuCards = [];
        this.board = [];
    };

    nextHand = () => {
        this.props.clearLog();
        this.props.clearPot();
        const random = new Random();
        random.shuffle(this.deck);
        // The deck card order is as follows:
        // human1 human2 cpu1 cpu2 flop1 flop2 flop3 turn river
        this.humanCards = this.deck.slice(0, 2);
        this.cpuCards = this.deck.slice(2, 4);
        this.board = this.deck.slice(4, 9);
        this.props.dealHumanCards(this.humanCards);
        if (this.dealer === "human") {
            this.props.setEnabledButtons(["fold", "call", "minBet", "betHalfPot", "betPot", "allIn", "betCustom"])
        } else {
            this.cpuAction();
        }
    };

    fold = () => {

    };

    check() {
      // ...
      this.cpuAction();
    };

    call() {};
    minBet() {};
    betHalfPot() {};
    betPot() {};
    allIn() {};
    betCustom(amount) {};

    cpuAction() {
        // send card / history info to server
        // wait for action from server
        // if (this.bettingIsOver()) {
            // handle next street
        // } else {
            // turn on human's buttons
        // }
    };

};

export default Game;
