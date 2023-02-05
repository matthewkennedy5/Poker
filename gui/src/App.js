import React, {Component} from 'react';
import Game from './game';
import Score from './components/score';
import Table from './components/table';
import Buttons from './components/buttons';
import Log from './components/log';
import './App.css';

const axios = require('axios');

const URL = 'http://localhost'

class App extends Component {

  dealHumanCards = (humanCards) => {
      this.setState({humanCards: humanCards});
  };

  showCPUCards = (cpuCards) => {
      this.setState({cpuCards: cpuCards});
  };

  dealFlop = (flopCards) => {
      flopCards.push("back");
      flopCards.push("back");
      this.setState({board: flopCards})
      this.state.board = flopCards;
  };

  dealTurn = (card) => {
      let board = this.state.board;
      board[3] = card;
      this.setState({board: board});
  };

  dealRiver = (card) => {
      let board = this.state.board;
      board[4] = card;
      this.setState({board: board});
  };

  setEnabledButtons = (buttons) => {
      let enabled = {}
      for (let button of buttons) {
          enabled[button] = true;
          this.state.enabledButtons[button] = true;
      }
      this.setState({enabledButtons: enabled})
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

  // addToScore = (winnings) => {
  //     const score = this.state.score + winnings;
  //     this.state.score = score;
  //     this.setState({score: score});
  // }

  incrementHands = () => {
      const nHands = this.state.hands + 1;
      this.state.hands = nHands;
      this.setState({hands: nHands});
  }

  // The backend API has 2 endpoints:
  // 1. /api/compare - Evaluates which player's hand is better
  // 2. /api/bot - gets the bot's action at a given spot

  evaluateHands = async(humanHand, cpuHand) => {
      humanHand = humanHand.join();
      cpuHand = cpuHand.join();
      const response = await axios.get(URL + '/api/compare?humanHand=' + humanHand + '&cpuHand=' + cpuHand);
      return response;
  }

  getCPUAction = async(cpuCards, history) => {
      const response = await axios.get(URL + '/api/bot', {
          params: {
              cpuCards: cpuCards.join(),
              board: this.getDisplayedBoardCards().join(),
              history: history
          }
      });
      return response;
  }

  getDisplayedHumanCards = () => {
      // The human's cards are always visible
      const humanCards = this.state.game.humanCards;
      if (humanCards.empty) {
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
      } else {
        return boardCards;
      }
  }

  state = {
    game: new Game({
      dealHumanCards: this.dealHumanCards,
      showCPUCards: this.showCPUCards,
      dealFlop: this.dealFlop,
      dealTurn: this.dealTurn,
      dealRiver: this.dealRiver,
      evaluateHands: this.evaluateHands,
      setEnabledButtons: this.setEnabledButtons,  // game.js shouldn't know about the buttons or UI
      // addToScore: this.addToScore,
      incrementHands: this.incrementHands,
      getCPUAction: this.getCPUAction
    }),
    // score: 0,
    hands: 0,
    enabledButtons: {  // TODO: I think enabledButtons is unused now
        nextHand: true,
        fold: false,
        check: false,
        call: false,
        minBet: false,
        betHalfPot: false,
        betPot: false,
        allIn: false,
        peek: false,
        betCustom:false
    },
    betAmount: 0
  };

  // Automatically start the first hand when the app loads
  componentDidMount() {
    this.state.game.nextHand();
  }

  render() {
    return (
      <div className="app">
        {/* <Score className="score"
               score={this.state.score}
               hands={this.state.hands}/>  */}
        <Table pot={this.state.game.pot}
               stacks={this.state.game.getStacks()}
               humanActionText={this.state.game.getPrevHumanAction()}
               cpuActionText={this.state.game.getPrevCPUAction()}
               humanCards={this.getDisplayedHumanCards()}
               cpuCards={this.getDisplayedCPUCards()}
               board={this.getDisplayedBoardCards()}/>
        <Buttons nextHand={this.state.game.nextHand}
                 fold={this.state.game.fold}
                 check={this.state.game.check}
                 call={this.state.game.call}
                 callAmount={this.state.game.getCallAmount()}
                 bet={() => this.state.game.bet(this.state.betAmount)}
                 updateBetAmount={this.updateBetAmount}
                 updateBetAmountFromEvent={this.updateBetAmountFromEvent}
                 betAmount={this.state.betAmount}
                 minBetAmount={this.state.game.getMinBetAmount()}
                 allInAmount={this.state.game.getAllInAmount()}
                 pot={this.state.game.pot}
                 enabledButtons={this.state.enabledButtons}/>
      </div>
    );
  };

};

export default App;
