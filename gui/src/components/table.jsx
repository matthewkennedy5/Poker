import React from 'react';

export default function Table({
  pot, cpuStack, humanStack, humanCards, cpuCards, board,
  humanIsDealer, isHumanTurn, isCPUTurn, humanContribution, cpuContribution,
}) {
  return (
    <div className="poker-table">
      <div className={`player-badge ${isCPUTurn ? 'active-turn' : ''}`}>
        <div className="player-name-row">
          <span className="player-name">CPU</span>
          {!humanIsDealer && <span className="dealer-chip">D</span>}
        </div>
        <span className="player-stack">${cpuStack}</span>
      </div>

      <div className="hand-cards">
        <img className="card" src={"/cards/" + cpuCards[0] + ".svg"} alt="" />
        <img className="card" src={"/cards/" + cpuCards[1] + ".svg"} alt="" />
      </div>

      {cpuContribution > 0 && <div className="bet-badge">${cpuContribution}</div>}

      <div className="board-cards">
        {board.map((c, i) => (
          <img key={i} className="card board-card" src={"/cards/" + c + ".svg"} alt="" />
        ))}
      </div>

      <div className="pot-display">POT: ${pot}</div>

      {humanContribution > 0 && <div className="bet-badge">${humanContribution}</div>}

      <div className="hand-cards human-cards">
        <img className="card" src={"/cards/" + humanCards[0] + ".svg"} alt="" />
        <img className="card" src={"/cards/" + humanCards[1] + ".svg"} alt="" />
      </div>

      <div className={`player-badge ${isHumanTurn ? 'active-turn' : ''}`}>
        <div className="player-name-row">
          <span className="player-name">YOU</span>
          {humanIsDealer && <span className="dealer-chip">D</span>}
        </div>
        <span className="player-stack">${humanStack}</span>
      </div>
    </div>
  );
}
