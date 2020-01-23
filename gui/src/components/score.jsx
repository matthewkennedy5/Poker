import React, {Component} from 'react';

class Score extends Component {

    render() {
        return (
            <div className="score">
                <b>Session: {this.props.score} <br></br>
                   Hands: {this.props.hands}</b>
            </div>
        );
    };

};

export default Score;

