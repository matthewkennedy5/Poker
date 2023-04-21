import React, {Component} from 'react';
import Game from './game';
import Score from './components/score';
import Table from './components/table';
import Buttons from './components/buttons';
import './App.css';

const axios = require('axios');

const URL = 'http://localhost'

class App extends Component {

  getEnabledButtons = () => {
      const street = this.state.game.street;
      const history = this.state.game.history;
      let prevAction = undefined;
      if (history.length > 0) {
          prevAction = history[history.length - 1];
      } 
      if (prevAction !== undefined && prevAction["action"] === "Fold" || street === "showdown") {
          return ["nextHand"];
      }
      let enabled = ["Fold"];
      if (street !== "preflop" && (prevAction === undefined || prevAction["action"] === "Call")) {
          enabled.push("Check");
      }
      if (this.state.game.getCallAmount() > 0) {
          enabled.push("Call");
      }
      if (this.state.game.getMinBetAmount() < this.state.game.getAllInAmount()) {
        enabled.push("Raise");
      }
      return enabled;
  }

  updateBetAmountFromEvent = (event) => {
    const amount = parseInt(event.target.value);
    if (amount > 0) {
        this.updateBetAmount(amount);
    }
  }

  updateBetAmount = (amount) => {
    this.setState({betAmount: amount});
  }

  incrementHands = () => {
      const nHands = this.state.hands + 1;
      this.state.hands = nHands;
      this.setState({hands: nHands});
  }

  // The backend API has 2 endpoints:
  // 1. /api/compare - Evaluates which player's hand is better
  // 2. /api/bot - gets the bot's action at a given spot

  evaluateHands = async(humanHand, cpuHand) => {
      const response = await axios.post(URL + '/api/compare', {
          humanHand: humanHand,
          cpuHand: cpuHand
      });
      return response;
  }

  listenForHumanAction = () => {
      const enabled = this.getEnabledButtons();
      this.setState({enabledButtons: enabled});
  }

  getCPUAction = async(cpuCards, history) => {
      const response = await axios.post(URL + '/api/bot', {
          cpuCards: cpuCards,
          board: this.getDisplayedBoardCards(),
          history: history
      });
      return response.data;
  }

  getHistoryInfo = async(history, dealerCards, opponentCards, boardCards) => {
      const response = await axios.post(URL + '/api/historyInfo', {
          history: history,
          dealerCards: dealerCards,
          opponentCards: opponentCards,
          boardCards: boardCards
      });
      return response.data;
  }

  getDisplayedHumanCards = () => {
      const humanCards = this.state.game.humanCards;
      if (humanCards === undefined || humanCards.length === 0) {
          return ["back", "back"];
      } else {
          return humanCards;
      }
  }

    getDisplayedCPUCards = () => {
        // The CPU's cards are only visible on showdown
        if (this.state.game.street === "showdown") {
            return this.state.game.cpuCards;
        } else {
            return ["back", "back"];
        }
    }

    getDisplayedBoardCards = () => {
        const boardCards = this.state.game.board;
        const street = this.state.game.street;
        if (street === "preflop") {
            return ["back", "back", "back", "back", "back"];
        } else if (street === "flop") {
            return [boardCards[0], boardCards[1], boardCards[2], "back", "back"];
        } else if (street === "turn") {
            return [boardCards[0], boardCards[1], boardCards[2], boardCards[3], "back"];
        } else if (street === "river" || street === "showdown") {
            return boardCards;
        } else {
            throw new Error("Bad street value");
        }
    }

  getCPUActionText = () => {
      if (this.state.game.winner === "human") {
          return "Human wins $" + this.state.game.pot;
      } else if (this.state.game.winner === "cpu") {
          return "CPU wins $" + this.state.game.pot;
      }
      let action = this.state.game.getPrevCPUAction();
      let text = "CPU ";
      if (action === undefined) {
          return "Your turn";
      } else if (action["action"] === "Fold") {
          text += "folds.";
      } else if (action["action"] === "Call") {
          text += "calls $" + action["amount"];
      } else if (action["action"] === "Bet") {  
          text += "bets $" + action["amount"];
      }
      return text;
  }

  state = {
    game: new Game({
      evaluateHands: this.evaluateHands,
      getCPUAction: this.getCPUAction,
      getHistoryInfo: this.getHistoryInfo,
      listenForHumanAction: this.listenForHumanAction
    }),
    enabledButtons: [],
    betAmount: 0
  };

  // Automatically start the first hand when the app loads
  componentDidMount() {
    this.state.game.nextHand();
  }

  render() {
    return (
      <div className="app">
        <Table pot={this.state.game.pot}
               cpuStack={this.state.game.getCPUStack()}
               humanStack={this.state.game.getHumanStack()}
               cpuActionText={this.getCPUActionText()}
               humanCards={this.getDisplayedHumanCards()}
               cpuCards={this.getDisplayedCPUCards()}
               board={this.getDisplayedBoardCards()}/>
        <Buttons nextHand={this.state.game.nextHand}
                 fold={this.state.game.fold}
                 check={this.state.game.check}
                 call={this.state.game.call}
                 callAmount={this.state.game.callAmount}
                 bet={() => this.state.game.bet(this.state.betAmount)}
                 updateBetAmount={this.updateBetAmount}
                 updateBetAmountFromEvent={this.updateBetAmountFromEvent}
                 betAmount={this.state.betAmount}
                 minBetAmount={this.state.game.minBetAmount}
                 allInAmount={this.state.game.allInAmount}
                 pot={this.state.game.pot}
                 enabledButtons={this.state.enabledButtons}/>
        <Score className="score"
               score={this.state.game.score}
               hands={this.state.game.numHands}/> 
      </div>
    );
  };

};

export default App;
