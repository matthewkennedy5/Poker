import {Random} from "random-js";
import {Component} from 'react';

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
        this.humanPosition = "opponent";  
        this.deck = DECK;
        this.humanCards = [];
        this.cpuCards = [];
        this.board = [];
        this.clearHistory();

        // Game state info. These are updated together by the API call
        this.street = "preflop";
        this.pot = 0;
        this.stacks = {
            'dealer': 20000,
            'opponent': 20000,
        };
        this.whoseTurn = "dealer";
        this.winner = "";
        this.minBetAmount = 0;
        this.allInAmount = 0;
        
        this.score = 0;     // Human's cumulative score across all hands in the session
        this.numHands = 0;  // Total hands played in the session
    };

    nextHand = async() => {
        this.humanPosition = this.otherPosition(this.humanPosition);
        this.clearHistory();
        const random = new Random();
        random.shuffle(this.deck);
        // The deck card order is as follows:
        // human1 human2 cpu1 cpu2 flop1 flop2 flop3 turn river
        this.humanCards = this.deck.slice(0, 2);
        this.cpuCards = this.deck.slice(2, 4);
        this.board = this.deck.slice(4, 9);
        this.nextMove();
    };

    fold = () => {
        this.playerAction("human", {action: "Fold", amount: 0});
    };

    check = () => {
        this.playerAction("human", {action: "Call", amount: 0})
    };

    call = () => {
        const amount = this.getCallAmount();
        this.playerAction("human", {action: "Call", amount: amount});
    };

    bet = (amount) => {
        this.playerAction("human", {action: "Bet", amount: amount})
    };

    clearHistory() {
        this.winner = "";
        this.history = [];
    }

    async nextMove() {
        await this.updateGameState();
        if (this.winnings !== 0) {
            this.handOver()
        } else {
            if (this.whoseTurn === this.humanPosition) {
                this.props.listenForHumanAction();
            } else {
                this.cpuAction();
            }
        }
    }

    playerAction(player, action) {
        this.history.push(action);
        this.nextMove();
    }

    handOver() {
        if (this.humanPosition == "dealer") {
            if (this.winnings > 0) {
                this.winner = "human";
            } else {
                this.winner = "cpu";
            }
        } else {
            if (this.winnings > 0) {
                this.winner = "cpu";
            } else {
                this.winner = "human";
            }
        }
        if (this.winner === "human") {
            this.score += this.winnings;
        } else {
            this.score -= this.winnings;
        }
        if (this.history[this.history.length-1].action !== "Fold") {
            this.street = "showdown";
        }
        this.numHands += 1;
        this.props.listenForHumanAction();
    }

    getCallAmount() {
        return this.callAmount;
    }

    getMinBetAmount() {
        return this.minBetAmount;
    }

    getAllInAmount() {
        return this.allInAmount;
    }

    otherPosition(position) {
        if (position === "dealer") {
            return "opponent";
        } else if (position === "opponent") {
            return "dealer";
        } else {
            throw new Error("Bad position string");
        }
    }

    getHumanStack() {
        return this.stacks[this.humanPosition]
    }

    getCPUStack() {
        return this.stacks[this.otherPosition(this.humanPosition)]
    }

    getPrevCPUAction() {
        if (this.whoseTurn === this.humanPosition && this.history.length >= 1) {
            return this.history[this.history.length - 1];
        } else if (this.whoseTurn !== this.humanPosition && this.history.length >= 2) {
            return this.history[this.history.length - 2];
        }
        return undefined;
    }

    async cpuAction() {
        const action = await this.props.getCPUAction(this.cpuCards, this.history);
        this.playerAction("cpu", action);
    };

    async updateGameState() {
        let dealerCards = [];
        let opponentCards = [];
        if (this.humanPosition == "dealer") {
            dealerCards = this.humanCards;
            opponentCards = this.cpuCards;
        } else {
            dealerCards = this.cpuCards;
            opponentCards = this.humanCards;
        }
        const result = await this.props.getHistoryInfo(this.history, dealerCards, opponentCards, this.board);
        this.pot = result.pot;
        this.street = result.street;
        this.callAmount = result.callAmount;
        this.minBetAmount = result.minBetAmount;
        this.allInAmount = result.allInAmount;
        this.whoseTurn = result.whoseTurn;
        this.stacks = result.stacks;
        this.winnings = result.winnings;

    }
};

export default Game;
