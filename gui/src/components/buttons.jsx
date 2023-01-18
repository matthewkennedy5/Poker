import React, {Component} from 'react';

const BUTTON_STYLE = "btn btn-custom m-1"

class Buttons extends Component {

    render() {
        const enabled = this.props.enabledButtons;
        if (enabled["nextHand"]) {
            return (
                <div className="buttons">
                    <button onClick={this.props.nextHand} className={BUTTON_STYLE}>
                        Next Hand
                    </button>
                </div>
            )
        } else {
            return (
                <div className="buttons">

                    {enabled["fold"] ? 
                        (<button onClick={this.props.fold} className={BUTTON_STYLE} disabled={!enabled["fold"]}>
                            Fold
                        </button>)
                        : (<button onClick={this.props.nextHand} className={BUTTON_STYLE} disabled={!enabled["nextHand"]}>
                            Next Hand
                        </button>)}

                    {enabled["check"] ?
                        (<button onClick={this.props.check} className={BUTTON_STYLE} disabled={!enabled["check"]}>
                            Check
                        </button>)
                        : (<button onClick={this.props.call} className={BUTTON_STYLE} disabled={!enabled["call"]}>
                            Call ${this.props.callAmount}
                        </button>)}

                    <button onClick={this.props.betCustom} className={BUTTON_STYLE} disabled={!enabled["betCustom"]}>
                        Raise ${this.props.customBetAmount}
                    </button>

                {/* <input type="text" onChange={this.props.updateCustomBet} className="m-1" size="5"></input> */}

                <input type="range" 
                       className="m-1" 
                    //    min={this.props.minBetAmount} 
                    //    max={this.props.allInAmount} 
                        min="0"
                        max="1"
                       disabled={!enabled["betCustom"]}
                       step="0.01"
                       onChange={this.props.updateCustomBet}
                />

                </div>
            );
        }
    };
}

export default Buttons;
