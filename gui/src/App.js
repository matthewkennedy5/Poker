import React, {Component} from 'react';
import logo from './logo.svg';
import Score from './components/score'
import Table from './components/table'
import Buttons from './components/buttons'
import Log from './components/log'
import './App.css';

class App extends Component {
  state = {};
  render() {
    return (
      <div>
        <Score />
        <Table />
        <Buttons />
        <Log />
      </div>
    );
  };
}

export default App;
