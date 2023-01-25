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

        if (enabled["nextHand"]) {
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
                <div className="buttons">

                    <div className="raise-ui">

                        <input type="range"
                            className="range-slider m-1"
                            min={this.props.minBetAmount}
                            max={this.props.allInAmount}
                            onChange={this.props.updateCustomBet}/>

                    </div>


                    <button onClick={this.hideRaiseUI} className={BUTTON_STYLE + " back"}>BACK</button>

                </div>
            );
        } else {
            // Normal row of buttons: Call Raise Fold
            return (
                <div className="buttons">

                    {enabled["check"] ?
                        (<button onClick={this.props.check} className={BUTTON_STYLE} disabled={!enabled["check"]}>
                            CHECK
                        </button>)
                        : (<button onClick={this.props.call} className={BUTTON_STYLE} disabled={!enabled["call"]}>
                            {call_text}
                        </button>)}

                    <button onClick={this.showRaiseUI} className={BUTTON_STYLE} disabled={!enabled["betCustom"]}>
                        RAISE
                    </button>

                    <button onClick={this.props.fold} className={BUTTON_STYLE + " fold"} disabled={!enabled["fold"]}>
                        FOLD
                    </button>

                {/* <input type="text" onChange={this.props.updateCustomBet} className="m-1" size="5"></input> */}

                </div>
            );
        }
    };
}

export default Buttons;
