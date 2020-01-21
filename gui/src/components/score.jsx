import React, {Component} from 'react';

class Score extends Component {

    state = {
        winnings: 0,
        num_hands: 0
    };

    render() {
        return (
            <div class="score">
                <b>Session: {this.state.winnings} <br></br>
                   Hands: {this.state.num_hands}</b>
            </div>
        );
    };

};

export default Score;

