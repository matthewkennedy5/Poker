import {Random} from "random-js";
import {Component} from 'react';

// TODO: Read these from a config file, however this seems hard to do on the 
// frontend. 
const STACK_SIZE = 20000;
const BIG_BLIND = 100;
const SMALL_BLIND = 50;

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
        super(props);   // TODO: change this deprecated function
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
        console.log('Next hand!');
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
        this.props.clearCards();
        const random = new Random();
        random.shuffle(this.deck);
        // The deck card order is as follows:
        // human1 human2 cpu1 cpu2 flop1 flop2 flop3 turn river
        this.humanCards = this.deck.slice(0, 2);
        this.cpuCards = this.deck.slice(2, 4);
        this.board = this.deck.slice(4, 9);
        this.playStreet();
    };

    player_fold(player) {
        // Update score with losings from this hand
        let losings = STACK_SIZE - this.stacks[player];
        if (losings === 0) {
            if (this.dealer === player) {
                losings = SMALL_BLIND;
            } else {
                losings = BIG_BLIND;
            }
        }
        if (player === "human") {
            this.props.logMessage("HUMAN folds.");
            this.props.addToScore(-losings);
        } else {
            this.props.addToScore(losings);
        }
        this.props.incrementHands();
        this.props.setEnabledButtons(["nextHand"]);
    }

    // TODO: for AIVAT, I may need to incorporate terminal_utility() into this.
    fold = () => {
        this.player_fold("human");
    };

    registerAction(action) {
        this.props.addToPot(action["amount"]);
        this.history[this.street].push(action);
        this.stacks["human"] -= action["amount"];
        this.updateLog("human", action);
        if (this.bettingIsOver()) {
            this.advanceStreet();
            this.playStreet();
        } else {
            this.cpuAction();
        }
    }

    advanceStreet() {
        if (this.street === "preflop") {
            this.street = "flop";
        } else if (this.street === "flop") {
            this.street = "turn";
        } else if (this.street === "turn") {
            this.street = "river";
        } else if (this.street === "river") {
            this.street = "showdown"
        }
        this.props.logMessage("--------------------------------");
    }

    check = () => {
        this.registerAction({action: "check", amount: 0})
    };

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

    async cpuAction() {
        const result = await this.props.getCPUAction(this.cpuCards, this.history);
        const action = result.data;
        this.stacks["cpu"] -= action["amount"];
        this.updateLog("cpu", action);
        if (action["action"] === "fold") {
            this.player_fold("cpu");
            return;
        }
        this.props.addToPot(action["amount"]);
        this.history[this.street].push(action);
        this.enableHumanButtons();
        if (this.bettingIsOver()) {
            this.advanceStreet();
            this.playStreet();
        }
    };

    updateLog(player, action) {
        let message = player.toUpperCase() + " ";
        if (action["action"] === "bet") {
            message += "bets $" + action["amount"];
        } else if (action["action"] === "call") {
            message += "calls $" + action["amount"];
        } else if (action["action"] === "check") {
            message += "checks"
        } else if (action["action"] === "fold") {
            message += "folds"
        } else {
            alert("Action not understood: " + action["action"]);
        }
        this.props.logMessage(message);
    };

    streetBets(streetActions) {
        // We know the previous bet was from the CPU, so we use that fact to know
        // which player is which in the action history.
        let cpuTotal = 0;
        let humanTotal = 0;

        streetActions = streetActions.slice().reverse();
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
        return (this.stacks["human"] - this.stacks["cpu"])
    }

    getMinBetAmount() {
        let minBetAmount = SMALL_BLIND;
        const prevAction = this.getPrevAction();
        if (prevAction && prevAction["amount"] > 0) {
            minBetAmount = 2 * prevAction["amount"];
        }
        return minBetAmount;
    }

    getPrevAction() {
        return this.history[this.street].slice(-1)[0];
    }

    enableHumanButtons() {
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
        if (stack >= pot/2 && pot/2 > this.getCallAmount()) {
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
        const prevAction = this.getPrevAction()["action"];
        if (prevAction === "check") {
            // If the second player to go checks, we're done with betting.
            return (this.history[this.street].length === 2);
        } else {
            return (this.stacks["human"] === this.stacks["cpu"]);
        }
    };

    async showdown() {
        // show all cards
        this.props.showCPUCards(this.cpuCards);
        this.props.dealFlop(this.board.slice(0, 3));
        this.props.dealTurn(this.board[3]);
        this.props.dealRiver(this.board[4]);

        const humanHand = this.humanCards.concat(this.board);
        const cpuHand = this.cpuCards.concat(this.board);
        const result = await this.props.evaluateHands(humanHand, cpuHand);
        const winner = result.data

        const pot = this.props.getPot();
        if (winner === "human") {
            this.props.addToScore(pot);
            this.props.logMessage("HUMAN wins a pot of $" + pot);
        } else if (winner === "cpu") {
            this.props.addToScore(-pot);
            this.props.logMessage("CPU wins a pot of $" + pot);
        } else if (winner === "tie") {
            this.props.logMessage("Split pot");
        }
        this.props.incrementHands();
        this.props.setEnabledButtons(["nextHand"]);
    };

    playStreet() {
        if (this.street === "showdown" || this.stacks["human"] + this.stacks["cpu"] === 0) {
            this.showdown();
            return;
        }

        if (this.street === "preflop") {
            this.props.dealHumanCards(this.humanCards);
            if (this.dealer === "human") {
                this.enableHumanButtons();
            } else {
                this.cpuAction();
            }
        } else {
            if (this.street === "flop") {
                this.props.dealFlop(this.board.slice(0, 3));
            } else if (this.street === "turn") {
                this.props.dealTurn(this.board[3]);
            } else if (this.street === "river") {
                this.props.dealRiver(this.board[4]);
            }
            if (this.dealer === "cpu") {
                this.enableHumanButtons();
            } else {
                this.cpuAction();
            }
        }
    };
};

export default Game;
