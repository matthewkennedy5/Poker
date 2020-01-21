import React, {Component} from 'react';
import logo from './logo.svg';
import Score from './components/score'
import Table from './components/table'
import Buttons from './components/buttons'
import Log from './components/log'
import './App.css';

// TODO: Figure out how to change tab thumbnail and title
class App extends Component {
  state = {};
  render() {
    return (
      <div>
        <Score class="score"/>
        <Table />
        <Buttons />
        <Log />
      </div>
    );
  };
}

export default App;
