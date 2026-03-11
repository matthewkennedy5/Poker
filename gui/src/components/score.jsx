import React from 'react';

export default function Score({ score, hands }) {
  const scoreColor = score > 0 ? '#4ade80' : score < 0 ? '#f87171' : '#aaa';
  const sign = score > 0 ? '+' : '';
  return (
    <div className="score-bar">
      <span className="score-item">
        Session: <span style={{ color: scoreColor }}>{sign}${score}</span>
      </span>
      <span className="score-divider">|</span>
      <span className="score-item">Hands: {hands}</span>
    </div>
  );
}
