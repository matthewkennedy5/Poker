import {Random} from "random-js";
import {Component} from 'react';

const STACK_SIZE = 20000;
const BIG_BLIND = 200;
const SMALL_BLIND = 100;

const FLOP = 0;
const TURN = 0
const RIVER = 0
const SHOWDOWN = 0

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
        this.dealer = "human";
        this.street = "preflop";
        this.bets = [0, 0];
        this.deck = DECK;
        this.humanCards = [];
        this.cpuCards = [];
        this.board = [];
        this.history = {
            preflop: [],
            flop: [],
            turn: [],
            river: []
        };
        this.score = 0;
        this.numHands = 0;
        this.stacks = {"human": STACK_SIZE, "cpu": STACK_SIZE};
        this.custBetAmount = 0;
    };

    nextHand = () => {
        this.street = "preflop";
        if (this.dealer === "cpu") {
            this.dealer = "human";
        } else {
            this.dealer = "cpu";
        }
        this.history = {
            preflop: [],
            flop: [],
            turn: [],
            river: []
        };
        this.stacks = {"human": STACK_SIZE, "cpu": STACK_SIZE};
        this.bets = [0, 0];
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
        // update score with losings from this hand
        const losings = STACK_SIZE - this.stacks["human"];
        this.props.addToScore(-losings);
        this.props.incrementHands();
        this.props.logMessage("HUMAN folds.");
        this.props.setEnabledButtons(["nextHand"]);
    };

    registerAction(action) {
        this.props.addToPot(action["amount"]);
        this.history[this.street].push(action);
        this.stacks["human"] -= action["amount"];
        this.updateLog("human", action);
        if (this.bettingIsOver()) {
            this.advanceStreet();
        }
    }

    check = () => {};

    call = () => {
        const amount = this.getCallAmount();
        const action = {action: "call", amount: amount};
        this.registerAction(action);
    };
    // TODO: Group public button methods

    minBet = () => {
        const amount = this.getMinBetAmount();
        const action = {action: "bet", amount: amount};
        this.registerAction(action);
    };

    roundToSmallBlind(number) {
        return Math.round(number / SMALL_BLIND) * SMALL_BLIND;
    };

    betHalfPot = () => {
        const amount = this.roundToSmallBlind(this.props.getPot() / 2);
        this.registerAction({action: "bet", amount: amount})
    };

    betPot = () => {
        this.registerAction({action: "bet", amount: this.props.getPot()});
    };

    allIn = () => {
        this.registerAction({action: "bet", amount: this.stacks["human"]})
    };

    updateCustomBet = (event) => {
        this.custBetAmount = event.target.value;
    };

    betCustom = () => {
        const amount = parseInt(this.custBetAmount);
        if (isNaN(amount)) {
            alert("Invalid bet amount")
        } else if (amount > this.stacks["human"]) {
            alert("Bet size is too large");
        } else if (amount < this.getMinBetAmount()) {
            alert("Must bet at least " + this.getMinBetAmount());
        } else {
            this.registerAction({action: "bet", amount: amount})
        }
    };

    cpuAction() {
        if (this.bettingIsOver()) {
            this.advanceStreet();
        } else {
        // TODO: Write a Flask server to integrate with the bot
        // send card / history info to server
        // wait for action from server
        // right now I'm going to say that the bot always check/calls
            const action = {action: "bet", amount: 3*BIG_BLIND};  // placeholder, but a good format for the action
            this.stacks["cpu"] -= action["amount"];
            this.updateLog("cpu", action);
            this.props.addToPot(action["amount"]);
            this.history[this.street].push(action);
            this.enableHumanButtons();
        }
    };

    updateLog(player, action) {
        let message = player.toUpperCase() + " ";
        if (action["action"] === "bet") {
            message += "bets $" + action["amount"];
        } else if (action["action"] === "call") {
            message += "calls";
        } else {
            alert("Action not understood");
        }
        this.props.logMessage(message);
    };

    streetBets(streetActions) {
        // We know the previous bet was from the CPU, so we use that fact to know
        // which player is which in the action history.
        let cpuTotal = 0;
        let humanTotal = 0;

        streetActions.reverse();
        for (let i = 0; i < streetActions.length; i++) {
            if (i % 2 === 0) {
                // This is a CPU action
                cpuTotal += streetActions[i]["amount"];
            } else {
                humanTotal += streetActions[i]["amount"];
            }
        }
        return [humanTotal, cpuTotal];
    }

    getCallAmount() {
        // for the human
        const history = this.history[this.street]
        const bets = this.streetBets(history);
        const totalHumanBet = bets[0];
        const totalCPUBet = bets[1];
        return (totalCPUBet - totalHumanBet);
    }

    getMinBetAmount() {
        let minBetAmount = SMALL_BLIND;
        const prevAction = this.history[this.street].slice(-1)[0];
        if (prevAction && prevAction["amount"] > 0) {
            minBetAmount = 2 * prevAction["amount"];
        }
        return minBetAmount;
    }

    enableHumanButtons() {
        // TODO: Dynamically figure out which human buttons should be allowed
        // depending on the previous bets / pot size.
        const history = this.history[this.street]
        const firstAction = history.length === 0;
        const bets = this.streetBets(history);
        const totalHumanBet = bets[0];
        const totalCPUBet = bets[1];
        let prevAction = "";
        if (!firstAction) {
            prevAction = history[history.length - 1]
        }

        let enabled = ["fold"];
        if (this.street !== "preflop" && (firstAction || prevAction["action"] === "check")) {
            enabled.push("check");
        }
        if (totalHumanBet < totalCPUBet) {
            enabled.push("call");
        }
        const stack = this.stacks["human"];
        if (this.getMinBetAmount() <= stack) {
            enabled.push("minBet");
        }
        const pot = this.props.getPot();
        if (stack >= pot/2) {
            enabled.push("betHalfPot");
        }
        if (stack >= pot) {
            enabled.push("betPot");
        }
        enabled.push("allIn");
        enabled.push("betCustom");
        this.props.setEnabledButtons(enabled);
    };

    bettingIsOver() {
        // TODO
        return false;
    };

    advanceStreet() {
        if (this.street === "preflop") {
            this.props.dealHumanCards(this.humanCards);
            if (this.dealer === "human") {
                this.enableHumanButtons();
            } else {
                this.cpuAction();
            }
            this.street = "flop";
        } else if (this.street === FLOP) {

        } else if (this.street === TURN) {

        } else if (this.street === RIVER) {

        } else if (this.street === SHOWDOWN) {

        }
    }

};

export default Game;


