import React, {Component} from 'react';

const BUTTON_STYLE = "btn btn-outline-dark m-1"

class Buttons extends Component {

    render() {
        return (
            <div className="buttons">
              <button onClick={this.props.onNextHand} className={BUTTON_STYLE}>Next Hand</button>
              <button onClick={this.props.fold} className={BUTTON_STYLE}>Fold</button>
              <button onClick={this.props.check} className={BUTTON_STYLE}>Check</button>
              <button onClick={this.props.call} className={BUTTON_STYLE}>Call</button>
              <button onClick={this.props.minBet} className={BUTTON_STYLE}>Min Bet</button>
              <button onClick={this.props.betHalfPot} className={BUTTON_STYLE}>Bet Half Pot</button>
              <button onClick={this.props.betPot} className={BUTTON_STYLE}>Bet Pot</button>
              <button onClick={this.props.allIn} className={BUTTON_STYLE}>All In</button>
              <button onClick={this.props.betCustom} className={BUTTON_STYLE}>Bet</button>
              <label></label>
              <input type="text" className="m-1" size="5"></input>

            </div>
        );
    };
}

export default Buttons;