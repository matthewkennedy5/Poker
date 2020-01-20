import React, {Component} from 'react';

const BUTTON_STYLE = "btn btn-outline-dark m-1"

class Buttons extends Component {
    state = {};

    nextHand() {
        return 0;
    }

    render() {
        return (
            <div>
              <button onClick={this.nextHand} className={BUTTON_STYLE}>Next Hand</button>
              <button onClick={this.fold} className={BUTTON_STYLE}>Fold</button>
              <button onClick={this.fold} className={BUTTON_STYLE}>Check</button>
              <button onClick={this.fold} className={BUTTON_STYLE}>Call</button>
              <button onClick={this.fold} className={BUTTON_STYLE}>Min Bet</button>
              <button onClick={this.fold} className={BUTTON_STYLE}>Bet Half Pot</button>
              <button onClick={this.fold} className={BUTTON_STYLE}>Bet Pot</button>
              <button onClick={this.fold} className={BUTTON_STYLE}>All In</button>
              <button onClick={this.fold} className={BUTTON_STYLE}>Bet</button>
            </div>
        );
    };
}

export default Buttons;