import React from 'react';

export default function Score({ score, hands }) {
  return (
    <div className="score">
      <b>Session: ${score} <br />
         Hands: {hands}</b>
    </div>
  );
}
