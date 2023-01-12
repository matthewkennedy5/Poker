import React, {Component} from 'react';

class Table extends Component {

    render() {
        return (
            <div className="poker-table">
                <div className="cards">
                    <div className="cpu-cards">
                        <img className="single-card" src={"/cards/" + this.props.cpuCards[0] + ".svg"} alt=""></img>
                        <img className="single-card" src={"/cards/" + this.props.cpuCards[1] + ".svg"} alt=""></img>
                    </div>
                    <div className="board-cards">
                        <img className="single-card" src={"/cards/" + this.props.board[0] + ".svg"} alt=""></img>
                        <img className="single-card" src={"/cards/" + this.props.board[1] + ".svg"} alt=""></img>
                        <img className="single-card" src={"/cards/" + this.props.board[2] + ".svg"} alt=""></img>
                        <img className="single-card" src={"/cards/" + this.props.board[3] + ".svg"} alt=""></img>
                        <img className="single-card" src={"/cards/" + this.props.board[4] + ".svg"} alt=""></img>
                    </div>
                    <div className="human-cards">
                        <img className="single-card" src={"/cards/" + this.props.humanCards[0] + ".svg"} alt=""></img>
                        <img className="single-card" src={"/cards/" + this.props.humanCards[1] + ".svg"} alt=""></img>
                    </div>
                </div>
                <label className="pot">pot: ${this.props.pot}</label>
            </div>
        );
    };
};

export default Table;