import React, {Component} from 'react';

class Table extends Component {

    state = {};

    feltStyle = {
        fill: "green",
        stroke: "green",
        opacity: 0.9
    };

    render() {
        return (
            <div class="table">
                <img class="humanCard1" src="/cards/As.svg"></img>
                <img class="humanCard2" src="/cards/Ah.svg"></img>
                <img class="cpuCard1" src="/cards/back.png"></img>
                <img class="cpuCard2" src="/cards/back.png"></img>
                <label class="pot">pot: $1234</label>
                <svg width="2000" height="600">
                    <rect x="50" y="20" rx="100" ry="100" width="1000" height="500" style={this.feltStyle}/>
                </svg>
            </div>
        );
    };
};

export default Table;