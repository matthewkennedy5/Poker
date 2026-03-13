import React from 'react';

export default function Score({ score, hands }) {
  const sign = score > 0 ? '+' : '';
  const scoreClass = score > 0 ? 'score-positive' : score < 0 ? 'score-negative' : 'score-neutral';
  return (
    <div className="score-bar">
      <span className={`score-badge ${scoreClass}`}>
        <span className="score-badge-label">Session</span>
        <span className="score-badge-value">{sign}${score}</span>
      </span>
      <span className={`score-badge`}>
        <span className="score-badge-label">Hands</span>
        <span className="score-badge-value">{hands}</span>
      </span>
    </div>
  );
}
