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
        // this.score = 0;
        this.numHands = 0;
        this.stacks = {"human": STACK_SIZE, "cpu": STACK_SIZE};
        this.pot = 0;
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
        this.pot = 0;
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
        // if (player === "human") {
        //     this.props.addToScore(-losings);
        // } else {
        //     this.props.addToScore(losings);
        // }
        this.props.incrementHands();
    }

    fold = () => {
        this.player_fold("human");
    };

    registerAction(action) {
        this.pot += action["amount"];
        this.history[this.street].push(action);
        this.stacks["human"] -= action["amount"];
        if (this.bettingIsOver()) {
            this.advanceStreet();
            this.playStreet();
        } else {
            this.cpuAction();
        }
    }

    advanceStreet() {
        if (this.stacks["human"] + this.stacks["cpu"] === 0) {
            // Both players are all-in, so skip to the showdown
            this.street = "showdown";
        } else if (this.street === "preflop") {
            this.street = "flop";
        } else if (this.street === "flop") {
            this.street = "turn";
        } else if (this.street === "turn") {
            this.street = "river";
        } else if (this.street === "river") {
            this.street = "showdown"
        }
    }

    check = () => {
        this.registerAction({action: "check", amount: 0})
    };

    call = () => {
        const amount = this.getCallAmount();
        const action = {action: "call", amount: amount};
        this.registerAction(action);
    };

    roundToSmallBlind(number) {
        return Math.round(number / SMALL_BLIND) * SMALL_BLIND;
    };

    betHalfPot = () => {
        const amount = this.roundToSmallBlind(this.pot / 2);
        this.registerAction({action: "bet", amount: amount})
    };

    bet = (amount) => {
        this.registerAction({action: "bet", amount: amount})
    };

    async cpuAction() {
        const result = await this.props.getCPUAction(this.cpuCards, this.history);
        const action = result.data;
        this.stacks["cpu"] -= action["amount"];
        // this.updateLog("cpu", action);
        if (action["action"] === "fold") {
            this.player_fold("cpu");
            return;
        }
        this.pot += action["amount"];
        this.history[this.street].push(action);
        if (this.bettingIsOver()) {
            this.advanceStreet();
            this.playStreet();
        }
    };

    // Returns the total amount of money bet by each player on the current street
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
        const prevAction = this.getPrevAction();
        let minBetAmount = SMALL_BLIND;
        if (prevAction === undefined) {
            // On the first action of the preflop, the min bet is the big blind.
            minBetAmount = BIG_BLIND;
        } else if (prevAction && prevAction["amount"] > 0) {
            // Otherwise the min bet is twice the previous bet. 
            minBetAmount = 2 * prevAction["amount"];
        }
        return minBetAmount;
    }

    getAllInAmount() {
        return this.stacks["human"];
    }

    getStacks() {
        return this.stacks;
    }

    getPrevAction() {
        try {
            return this.history[this.street].slice(-1)[0];
        } catch (e) {
            return undefined;
        }
    }

    getPrevHumanAction() {
        return JSON.stringify(this.getPrevAction());
    }

    getPrevCPUAction() {
        return JSON.stringify(this.getPrevAction());
    }

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
        const humanHand = this.humanCards.concat(this.board);
        const cpuHand = this.cpuCards.concat(this.board);
        const result = await this.props.evaluateHands(humanHand, cpuHand);
        const winner = result.data

        // if (winner === "human") {
        //     this.props.addToScore(this.pot);
        // } else if (winner === "cpu") {
        //     this.props.addToScore(-this.pot);
        // } else if (winner === "tie") {
        // }
        this.props.incrementHands();
    };

    playStreet() {

        if (this.street === "showdown") {
            this.showdown();
            return;
        }

        if (this.street === "preflop") {
            if (this.dealer === "human") {
            } else {
                this.cpuAction();
            }
        } else {
            if (this.dealer === "cpu") {
            } else {
                this.cpuAction();
            }
        }
    };
};

export default Game;
