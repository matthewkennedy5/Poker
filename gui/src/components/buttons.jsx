import React, { useState } from 'react';

const BUTTON_STYLE = "btn btn-custom m-1";

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

  let callText = "CALL";
  if (callAmount > 0) {
    callText += " $" + callAmount;
  }

  if (enabled.includes("nextHand")) {
    return (
      <div className="buttons">
        <button onClick={nextHand} className={BUTTON_STYLE} disabled={disabled}>
          NEXT HAND
        </button>
      </div>
    );
  }

  if (raiseUIVisible) {
    return (
      <div>
        <div id="label-and-buttons">
          <div id="your-bet">
            <p id="label">Your bet</p>
            <input type="text" id="betInput" value={betAmount} onChange={updateBetAmountFromEvent} className="m-1" />
          </div>
          <div className="back-raise-buttons">
            <button onClick={raiseButtonPressed} id="raise-button" className={BUTTON_STYLE} disabled={disabled || betAmount < minBetAmount || betAmount > allInAmount}>RAISE</button>
            <button onClick={() => setRaiseUIVisible(false)} id="back-button" className={BUTTON_STYLE + " back"} disabled={disabled}>BACK</button>
          </div>
        </div>
        <div className="raise-controller">
          <div className="default-buttons">
            <button className="btn btn-custom default-bet m-1" onClick={() => updateBetAmount(minBetAmount)} disabled={disabled}>
              MIN RAISE
            </button>
            <button className="btn btn-custom default-bet m-1" onClick={() => updateBetAmount(pot / 2)} disabled={disabled || pot / 2 < minBetAmount}>
              1/2 POT
            </button>
            <button className="btn btn-custom default-bet m-1" onClick={() => updateBetAmount(pot * 3 / 4)} disabled={disabled || pot * 3 / 4 < minBetAmount}>
              3/4 POT
            </button>
            <button className="btn btn-custom default-bet m-1" onClick={() => updateBetAmount(pot)} disabled={disabled || pot < minBetAmount}>
              POT
            </button>
            <button className="btn btn-custom default-bet m-1" onClick={() => updateBetAmount(allInAmount)} disabled={disabled}>
              ALL IN
            </button>
          </div>
          <div className="slider">
            <input type="range"
              className="range-slider m-1"
              id="slider"
              min={minBetAmount}
              max={allInAmount}
              value={betAmount}
              onChange={updateBetAmountFromEvent} />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="buttons">
      {enabled.includes("Check") ? (
        <button onClick={check} className={BUTTON_STYLE} disabled={disabled || !enabled.includes("Check")}>
          CHECK
        </button>
      ) : (
        <button onClick={call} className={BUTTON_STYLE} disabled={disabled || !enabled.includes("Call")}>
          {callText}
        </button>
      )}

      <button onClick={() => setRaiseUIVisible(true)} className={BUTTON_STYLE} disabled={disabled || !enabled.includes("Raise")}>
        RAISE
      </button>

      <button onClick={fold} className={BUTTON_STYLE + " fold"} disabled={disabled || !enabled.includes("Fold")}>
        FOLD
      </button>
    </div>
  );
}
