import React from 'react';

export default function Table({ pot, cpuStack, humanStack, cpuActionText, humanCards, cpuCards, board }) {
  return (
    <div className="poker-table">
      <div className="cpu-stack">
        <label>stack: ${cpuStack}</label>
      </div>
      <div className="cpu-cards">
        <img className="single-card" src={"/cards/" + cpuCards[0] + ".svg"} alt="" />
        <img className="single-card" src={"/cards/" + cpuCards[1] + ".svg"} alt="" />
      </div>
      <div className="cpu-action">
        <label>{cpuActionText}</label>
      </div>
      <div className="board-cards">
        <img className="single-card" src={"/cards/" + board[0] + ".svg"} alt="" />
        <img className="single-card" src={"/cards/" + board[1] + ".svg"} alt="" />
        <img className="single-card" src={"/cards/" + board[2] + ".svg"} alt="" />
        <img className="single-card" src={"/cards/" + board[3] + ".svg"} alt="" />
        <img className="single-card" src={"/cards/" + board[4] + ".svg"} alt="" />
      </div>
      <div className="pot">
        <label>pot: ${pot}</label>
      </div>
      <div className="human-cards">
        <img className="single-card" src={"/cards/" + humanCards[0] + ".svg"} alt="" />
        <img className="single-card" src={"/cards/" + humanCards[1] + ".svg"} alt="" />
      </div>
      <div className="human-stack">
        <label>stack: ${humanStack}</label>
      </div>
    </div>
  );
}
