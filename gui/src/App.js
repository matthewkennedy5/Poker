import React, {Component} from 'react';
import Game from './game';
import Score from './components/score';
import Table from './components/table';
import Buttons from './components/buttons';
import Log from './components/log';
import './App.css';

const axios = require('axios');

const URL = 'https://www.pokertrainer.info'

class App extends Component {

  clearLog = () => {
    this.state.log = "";
    this.setState({log: ""});
  };

  logMessage = (message) => {
    let log = "";
    if (this.state.log === "") {
        log = message;
    } else {
        log = this.state.log + "\n" + message;
    }
    this.setState({log: log});
    this.state.log = log;
  };

  clearPot = () => {
      this.state.pot = 0;
      this.setState({pot: 0});
  };

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

  clearCards = () => {
      const board = ["back", "back", "back", "back", "back"];
      const hand = ["back", "back"];
      this.setState({board: board, humanCards: hand, cpuCards: hand});
      this.state.board = board;
      this.state.humanCards = hand;
      this.state.cpuCards = hand;
  }

  setEnabledButtons = (buttons) => {
      let enabled = {}
      for (let button of buttons) {
          enabled[button] = true;
          this.state.enabledButtons[button] = true;
      }
      this.setState({enabledButtons: enabled})
  }

  addToPot = (amount) => {
    let newPot = this.state.pot + amount;
    this.state.pot = newPot;
    this.setState({pot: newPot});
  }

  getPot = () => {
      if (this.state.pot === 0) {
        return this.state.game.BIG_BLIND + this.state.game.SMALL_BLIND;
      }
      return this.state.pot;
  }

  addToScore = (winnings) => {
      const score = this.state.score + winnings;
      this.state.score = score;
      this.setState({score: score});
  }

  incrementHands = () => {
      const nHands = this.state.hands + 1;
      this.state.hands = nHands;
      this.setState({hands: nHands});
  }

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
              board: this.state.board.join(),
              history: history
          }
      });
      return response;
  }

  state = {
    game: new Game({logMessage: this.logMessage,
                    clearLog: this.clearLog,
                    clearPot: this.clearPot,
                    getPot: this.getPot,
                    clearCards: this.clearCards,
                    dealHumanCards: this.dealHumanCards,
                    showCPUCards: this.showCPUCards,
                    dealFlop: this.dealFlop,
                    dealTurn: this.dealTurn,
                    dealRiver: this.dealRiver,
                    evaluateHands: this.evaluateHands,
                    setEnabledButtons: this.setEnabledButtons,
                    addToPot: this.addToPot,
                    addToScore: this.addToScore,
                    incrementHands: this.incrementHands,
                    getCPUAction: this.getCPUAction}),
    log: "Welcome to Poker!",
    pot: 0,
    humanCards: ["back", "back"],
    cpuCards: ["back", "back"],
    board: ["back", "back", "back", "back", "back"], 
    score: 0,
    hands: 0,
    enabledButtons: {
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
    }
  };

  render() {
    return (
      <div className="app">
        <Score className="score"
               score={this.state.score}
               hands={this.state.hands}/> 
        <Table pot={this.state.pot}
               humanCards={this.state.humanCards}
               cpuCards={this.state.cpuCards}
               board={this.state.board}/>
        <Buttons onNextHand={this.state.game.nextHand}
                 fold={this.state.game.fold}
                 check={this.state.game.check}
                 call={this.state.game.call}
                 minBet={this.state.game.minBet}
                 betHalfPot={this.state.game.betHalfPot}
                 betPot={this.state.game.betPot}
                 allIn={this.state.game.allIn}
                 betCustom={this.state.game.betCustom}
                 updateCustomBet={this.state.game.updateCustomBet}
                 enabledButtons={this.state.enabledButtons}
          />
        <Log text={this.state.log}/>
      </div>
    );
  };

};


export default App;

// Improvement ideas
// TODO: Figure out how to change tab thumbnail and title
// TODO: Make a display label for Slumbot's bets (and the user's bet)
// TODO: The player's cards move in a weird way when the window gets resized.

