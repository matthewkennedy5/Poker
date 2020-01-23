import React, {Component} from 'react';
import Game from './game';
import logo from './logo.svg';
import Score from './components/score';
import Table from './components/table';
import Buttons from './components/buttons';
import Log from './components/log';
import './App.css';

// TODO: Figure out how to change tab thumbnail and title
class App extends Component {

  clearLog = () => {
    this.state.log = "";
    this.setState({log: ""});
  };

  logMessage = (message) => {
    const log = this.state.log + "\n" + message;
    this.setState({log: log});
  };

  clearPot = () => {
    this.state.pot = 0;
    this.setState({pot: 0})
  };

  dealHumanCards = (humanCards) => {
      this.state.humanCards = humanCards
      this.setState({humanCards: humanCards});
  };

  dealFlop = (card1, card2, card3) => {

  };

  dealTurn = (card) => {

  };

  dealRiver = (card) => {

  };


  state = {
    game: new Game({logMessage: this.logMessage,
                    clearLog: this.clearLog,
                    clearPot: this.clearPot,
                    dealHumanCards: this.dealHumanCards}),
    log: "Welcome to Poker!",
    pot: 0,
    humanCards: ["back", "back"],
    cpuCards: ["back", "back"],
    board: ["back", "back", "back", "back", "back"],  // TODO: Make board not appear before its time
  };

  render() {
    return (
      <div>
        <Score className="score"/>
        <Table pot={this.state.pot}
               humanCards={this.state.humanCards}
               cpuCards={this.state.cpuCards}
               board={this.state.board}/>
        <Buttons
          onNextHand={this.state.game.nextHand}
          />
        <Log text={this.state.log}/>
      </div>
    );
  };

};


export default App;
