import React, {Component} from 'react';

class Log extends Component {

    render() {
        return <p class="log">{this.props.text}</p>;
    };
};

export default Log;