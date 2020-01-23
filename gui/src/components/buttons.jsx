import React, {Component} from 'react';

const BUTTON_STYLE = "btn btn-outline-dark m-1"

class Buttons extends Component {

    render() {
        const enabled = this.props.enabledButtons;
        return (
            <div className="buttons">

              <button onClick={this.props.onNextHand}
                      className={BUTTON_STYLE}
                      disabled={!enabled["nextHand"]}>Next Hand</button>

              <button onClick={this.props.fold}
                      className={BUTTON_STYLE}
                      disabled={!enabled["fold"]}>Fold</button>

              <button onClick={this.props.check}
                      className={BUTTON_STYLE}
                      disabled={!enabled["check"]}>Check</button>

              <button onClick={this.props.call}
                      className={BUTTON_STYLE}
                      disabled={!enabled["call"]}>Call</button>

              <button onClick={this.props.minBet}
                      className={BUTTON_STYLE}
                      disabled={!enabled["minBet"]}>Min Bet</button>

              <button onClick={this.props.betHalfPot}
                      className={BUTTON_STYLE}
                      disabled={!enabled["betHalfPot"]}>Bet Half Pot</button>

              <button onClick={this.props.betPot}
                      className={BUTTON_STYLE}
                      disabled={!enabled["betPot"]}>Bet Pot</button>

              <button onClick={this.props.allIn}
                      className={BUTTON_STYLE}
                      disabled={!enabled["allIn"]}>All In</button>

              <button onClick={this.props.betCustom}
                      className={BUTTON_STYLE}
                      disabled={!enabled["betCustom"]}>Bet</button>

              <input type="text" onChange={this.props.updateCustomBet} className="m-1" size="5"></input>

            </div>
        );
    };
}

export default Buttons;