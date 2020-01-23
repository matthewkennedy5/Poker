import {Random} from "random-js";
import React, {Component} from 'react';

const PREFLOP = 0;
const FLOP = 1;
const TURN = 2;
const RIVER = 3;
const SHOWDOWN = 4;

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
        this.history = [];
    };

    nextHand = () => {
        this.street = PREFLOP;
        this.props.clearLog();
        this.props.clearPot();
        const random = new Random();
        random.shuffle(this.deck);
        // The deck card order is as follows:
        // human1 human2 cpu1 cpu2 flop1 flop2 flop3 turn river
        this.humanCards = this.deck.slice(0, 2);
        this.cpuCards = this.deck.slice(2, 4);
        this.board = this.deck.slice(4, 9);
        this.advanceStreet();
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
        if (this.bettingIsOver()) {
            this.advanceStreet();
        } else {
        // TODO: Write a Flask server to integrate with the bot
        // send card / history info to server
        // wait for action from server
        // right now I'm going to say that the bot always check/calls
            const action = {action: "bet", amount: 20};  // placeholder, but a good format for the action
            this.updateLog("cpu", action);
            this.addToPot(action["amount"]);
            // TODO: keep track and update action history
            this.enableHumanButtons();
        }
    };

    updateLog(player, action) {
        let message = player.toUpperCase() + " ";
        if (action["action"] === "bet") {
            message += "bets $" + action["amount"];
        } else {
            alert("Action not understood");
        }
        this.props.logMessage(message);
    };

    addToPot(amount) {

    };

    enableHumanButtons() {

    };

    bettingIsOver() {
        // TODO
        return false;
    };

    advanceStreet() {
        if (this.street === PREFLOP) {
            this.props.dealHumanCards(this.humanCards);
            if (this.dealer === "human") {
                this.props.setEnabledButtons(["fold", "call", "minBet", "betHalfPot", "betPot", "allIn", "betCustom"])
            } else {
                this.cpuAction();
            }
        } else if (this.street === FLOP) {

        } else if (this.street === TURN) {

        } else if (this.street === RIVER) {

        } else if (this.street === SHOWDOWN) {

        }
        this.street++;
    }

};

export default Game;
