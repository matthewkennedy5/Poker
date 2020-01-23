import React, {Component} from 'react';

class Table extends Component {

    feltStyle = {
        fill: "green",
        stroke: "green",
        opacity: 0.9
    };

    card2imagePath = {

    }

    render() {
        return (
            <div className="table">
                <img className="humanCard1" src={"/cards/" + this.props.humanCards[0] + ".svg"} alt=""></img>
                <img className="humanCard2" src={"/cards/" + this.props.humanCards[1] + ".svg"} alt=""></img>
                <img className="cpuCard1" src={"/cards/" + this.props.cpuCards[0] + ".svg"} alt=""></img>
                <img className="cpuCard2" src={"/cards/" + this.props.cpuCards[1] + ".svg"} alt=""></img>
                <img className="board1" src={"/cards/" + this.props.board[0] + ".svg"} alt=""></img>
                <img className="board2" src={"/cards/" + this.props.board[1] + ".svg"} alt=""></img>
                <img className="board3" src={"/cards/" + this.props.board[2] + ".svg"} alt=""></img>
                <img className="board4" src={"/cards/" + this.props.board[3] + ".svg"} alt=""></img>
                <img className="board5" src={"/cards/" + this.props.board[4] + ".svg"} alt=""></img>
                <label className="pot">pot: ${this.props.pot}</label>
                <svg width="2000" height="600">
                    <rect x="50" y="20" rx="100" ry="100" width="1000" height="500" style={this.feltStyle}/>
                </svg>
            </div>
        );
    };
};

export default Table;