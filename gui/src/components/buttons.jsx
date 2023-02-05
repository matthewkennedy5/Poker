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
                            <button onClick={this.props.bet} id="raise-button" className={BUTTON_STYLE}>RAISE</button>
                            <button onClick={this.hideRaiseUI} id="back-button" className={BUTTON_STYLE + " back"}>BACK</button>
                        </div>
                    </div>
                    <div className="raise-controller">
                        <div className="default-buttons">
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.minBetAmount)}>
                                MIN RAISE
                            </button>
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.pot / 2)}>
                                1/2 POT
                            </button>
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.pot * 3/4)}>
                                3/4 POT
                            </button>
                            <button className="btn btn-custom default-bet m-1" onClick={() => this.props.updateBetAmount(this.props.pot)}>
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
                    {enabled.includes("check") ?
                        (<button onClick={this.props.check} className={BUTTON_STYLE} disabled={!enabled.includes("check")}>
                            CHECK
                        </button>)
                        : (<button onClick={this.props.call} className={BUTTON_STYLE} disabled={!enabled.includes("call")}>
                            {call_text}
                        </button>)}

                    <button onClick={this.showRaiseUI} className={BUTTON_STYLE} disabled={!enabled.includes("raise")}>
                        RAISE
                    </button>

                    <button onClick={this.props.fold} className={BUTTON_STYLE + " fold"} disabled={!enabled.includes("fold")}>
                        FOLD
                    </button>
                </div>
            );
        }
    };
}

export default Buttons;
