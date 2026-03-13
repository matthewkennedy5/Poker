import React from 'react';

export default function Table({ pot, cpuStack, humanStack, cpuActionText, humanCards, cpuCards, board }) {
  return (
    <div className="poker-table">
      <div className="player-badge">
        <span className="player-name">CPU</span>
        <span className="player-stack">${cpuStack}</span>
      </div>

      <div className="hand-cards">
        <img className="card" src={"/cards/" + cpuCards[0] + ".svg"} alt="" />
        <img className="card" src={"/cards/" + cpuCards[1] + ".svg"} alt="" />
      </div>

      <div className="board-cards">
        {board.map((c, i) => (
          <img key={i} className="card board-card" src={"/cards/" + c + ".svg"} alt="" />
        ))}
      </div>

      <div className="pot-display">POT: ${pot}</div>

      <div className="action-text">{cpuActionText}</div>

      <div className="hand-cards human-cards">
        <img className="card" src={"/cards/" + humanCards[0] + ".svg"} alt="" />
        <img className="card" src={"/cards/" + humanCards[1] + ".svg"} alt="" />
      </div>

      <div className="player-badge">
        <span className="player-name">YOU</span>
        <span className="player-stack">${humanStack}</span>
      </div>
    </div>
  );
}
