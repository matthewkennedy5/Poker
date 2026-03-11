import React, { useState } from 'react';

export default function Buttons({
  nextHand, fold, check, call, callAmount, bet,
  updateBetAmount, updateBetAmountFromEvent, betAmount,
  minBetAmount, allInAmount, pot, enabledButtons, disabled,
}) {
  const [raiseUIVisible, setRaiseUIVisible] = useState(false);

  const raiseButtonPressed = () => {
    setRaiseUIVisible(false);
    bet();
  };

  const enabled = enabledButtons;

  if (enabled.includes("nextHand")) {
    return (
      <div className="actions">
        <button onClick={nextHand} className="action-btn btn-call" disabled={disabled}>
          NEXT HAND
        </button>
      </div>
    );
  }

  if (raiseUIVisible) {
    return (
      <div className="actions raise-mode">
        <div className="raise-panel">
          <div className="raise-header">
            <div className="raise-input-group">
              <span className="raise-label">Raise to</span>
              <input
                type="text"
                className="raise-input"
                value={betAmount}
                onChange={updateBetAmountFromEvent}
              />
            </div>
            <div className="raise-confirm-buttons">
              <button
                onClick={raiseButtonPressed}
                className="action-btn btn-raise"
                disabled={disabled || betAmount < minBetAmount || betAmount > allInAmount}
              >
                RAISE
              </button>
              <button
                onClick={() => setRaiseUIVisible(false)}
                className="action-btn btn-back"
                disabled={disabled}
              >
                BACK
              </button>
            </div>
          </div>

          <div className="slider-row">
            <span className="slider-min">${minBetAmount}</span>
            <input
              type="range"
              className="raise-slider"
              min={minBetAmount}
              max={allInAmount}
              value={betAmount}
              onChange={updateBetAmountFromEvent}
            />
            <span className="slider-max">${allInAmount}</span>
          </div>

          <div className="preset-buttons">
            <button className="preset-btn" onClick={() => updateBetAmount(minBetAmount)} disabled={disabled}>
              MIN
            </button>
            <button className="preset-btn" onClick={() => updateBetAmount(Math.round(pot / 2))} disabled={disabled || pot / 2 < minBetAmount}>
              1/2
            </button>
            <button className="preset-btn" onClick={() => updateBetAmount(Math.round(pot * 3 / 4))} disabled={disabled || pot * 3 / 4 < minBetAmount}>
              3/4
            </button>
            <button className="preset-btn" onClick={() => updateBetAmount(pot)} disabled={disabled || pot < minBetAmount}>
              POT
            </button>
            <button className="preset-btn" onClick={() => updateBetAmount(allInAmount)} disabled={disabled}>
              ALL IN
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="actions">
      <button onClick={fold} className="action-btn btn-fold" disabled={disabled || !enabled.includes("Fold")}>
        FOLD
      </button>

      {enabled.includes("Check") ? (
        <button onClick={check} className="action-btn btn-call" disabled={disabled}>
          CHECK
        </button>
      ) : (
        <button onClick={call} className="action-btn btn-call" disabled={disabled || !enabled.includes("Call")}>
          CALL {callAmount > 0 ? '$' + callAmount : ''}
        </button>
      )}

      <button
        onClick={() => { updateBetAmount(minBetAmount); setRaiseUIVisible(true); }}
        className="action-btn btn-raise"
        disabled={disabled || !enabled.includes("Raise")}
      >
        RAISE
      </button>
    </div>
  );
}
