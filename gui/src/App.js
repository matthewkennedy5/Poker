import React, {Component} from 'react';
import Game from './game';
import Score from './components/score';
import Table from './components/table';
import Buttons from './components/buttons';
import Log from './components/log';
import './App.css';

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
  };

  clearPot = () => {
    this.state.pot = 0;
    this.setState({pot: 0})
  };

  dealHumanCards = (humanCards) => {
      this.setState({humanCards: humanCards});
  };

  dealFlop = (card1, card2, card3) => {

  };

  dealTurn = (card) => {

  };

  dealRiver = (card) => {

  };

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

  state = {
    game: new Game({logMessage: this.logMessage,
                    clearLog: this.clearLog,
                    clearPot: this.clearPot,
                    dealHumanCards: this.dealHumanCards,
                    setEnabledButtons: this.setEnabledButtons,
                    addToPot: this.addToPot}),
    log: "Welcome to Poker!",
    pot: 0,
    humanCards: ["back", "back"],
    cpuCards: ["back", "back"],
    board: ["back", "back", "back", "back", "back"],  // TODO: Make board not appear before its time

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
      <div>
        <Score className="score"/>
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
