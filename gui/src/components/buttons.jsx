import React, {Component} from 'react';

const BUTTON_STYLE = "btn btn-custom m-1"

class Buttons extends Component {

    constructor(props) {
        super(props);
        this.state = {
            raiseUIVisible: false,
        }
        this.showRaiseUI = this.showRaiseUI.bind(this);
        this.hideRaiseUI = this.hideRaiseUI.bind(this);
    }

    showRaiseUI() {
        this.setState({raiseUIVisible: true});
    }

    hideRaiseUI() {
        this.setState({raiseUIVisible: false});
    }

    raiseButtonPressed = () => {
        this.hideRaiseUI();
        this.props.bet();
    }

    render() {
        const enabled = this.props.enabledButtons;

        let call_text = "CALL";
        if (this.props.callAmount > 0) {
            call_text += " $" + this.props.callAmount;
        }

        if (enabled.includes("nextHand")) {
            return (
                <div className="buttons">
                    <button onClick={this.props.nextHand} className={BUTTON_STYLE}>
                        NEXT HAND
                    </button>
                </div>
            );
        } else if (this.state.raiseUIVisible) {
            // Display slider and options for betting
            return (
                <div>
                    <div id="label-and-buttons">
                        <div id="your-bet">
                            <p id="label">Your bet</p>
                            <input type="text" id="betInput" value={this.props.betAmount} onChange={this.props.updateBetAmountFromEvent} className="m-1"/>
                        </div>
                        <div className="back-raise-buttons">
                            <button onClick={this.raiseButtonPressed} id="raise-button" className={BUTTON_STYLE} disabled={this.props.betAmount < this.props.minBetAmount || this.props.betAmount > this.props.allInAmount}>RAISE</button>
                            <button onClick={this.hideRaiseUI} id="back-button" className={BUTTON_STYLE + " back"}>BACK</button>
                        </div>
                    </div>
                    <div className="raise-controller">
                        <div className="default-buttons">
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.minBetAmount)}>
                                MIN RAISE
                            </button>
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.pot / 2)} disabled={this.props.pot / 2 < this.props.minBetAmount}>
                                1/2 POT
                            </button>
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.pot * 3/4)} disabled={this.props.pot * 3/4 < this.props.minBetAmount}>
                                3/4 POT
                            </button>
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.pot)} disabled={this.props.pot < this.props.minBetAmount}>
                                POT
                            </button>
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.allInAmount)}>
                                ALL IN
                            </button>
                        </div>
                        <div className="slider">
                            <input type="range"
                                className="range-slider m-1"
                                id="slider"
                                min={this.props.minBetAmount}
                                max={this.props.allInAmount}
                                value={this.props.betAmount}
                                onChange={this.props.updateBetAmountFromEvent}/>
                        </div>
                    </div>
                </div>
            );
        } else {
            // Normal row of buttons: Call Raise Fold
            return (
                <div className="buttons">
                    {enabled.includes("Check") ?
                        (<button onClick={this.props.check} className={BUTTON_STYLE} disabled={!enabled.includes("Check")}>
                            CHECK
                        </button>)
                        : (<button onClick={this.props.call} className={BUTTON_STYLE} disabled={!enabled.includes("Call")}>
                            {call_text}
                        </button>)}

                    <button onClick={this.showRaiseUI} className={BUTTON_STYLE} disabled={!enabled.includes("Raise")}>
                        RAISE
                    </button>

                    <button onClick={this.props.fold} className={BUTTON_STYLE + " fold"} disabled={!enabled.includes("Fold")}>
                        FOLD
                    </button>
                </div>
            );
        }
    };
}

export default Buttons;
