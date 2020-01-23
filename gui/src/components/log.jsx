import React, {Component} from 'react';

class Log extends Component {

    render() {
        return <label className="log">{this.props.text}</label>;
    };
};

export default Log;