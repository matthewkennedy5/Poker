import React, { useState, useRef, useEffect } from 'react';
import useGame from './useGame';
import { getDisplayedBoard, annotateHistory } from './utils';
import Score from './components/score';
import Table from './components/table';
import Buttons from './components/buttons';
import './App.css';

function getDisplayedHumanCards(humanCards) {
  if (!humanCards || humanCards.length === 0) return ['back', 'back'];
  return humanCards;
}

function getDisplayedCPUCards(cpuCards, street) {
  if (street === 'showdown') return cpuCards;
  return ['back', 'back'];
}

function getEnabledButtons(street, history, callAmount, minBetAmount, allInAmount) {
  let prevAction;
  if (history.length > 0) {
    prevAction = history[history.length - 1];
  }
  if ((prevAction !== undefined && prevAction.action === 'Fold') || street === 'showdown') {
    return ['nextHand'];
  }
  const enabled = ['Fold'];
  if (street !== 'preflop' && (prevAction === undefined || prevAction.action === 'Call')) {
    enabled.push('Check');
  }
  if (callAmount > 0) {
    enabled.push('Call');
  }
  if (minBetAmount < allInAmount) {
    enabled.push('Raise');
  }
  return enabled;
}

export default function App() {
  const game = useGame();
  const [betAmount, setBetAmount] = useState(0);
  const logRef = useRef(null);

  const humanStack = game.stacks[game.humanPosition];
  const cpuStack = game.stacks[game.humanPosition === 'dealer' ? 'opponent' : 'dealer'];
  const displayedBoard = getDisplayedBoard(game.board, game.street);
  const displayedHumanCards = getDisplayedHumanCards(game.humanCards);
  const displayedCPUCards = getDisplayedCPUCards(game.cpuCards, game.street);
  const enabledButtons = getEnabledButtons(game.street, game.history, game.callAmount, game.minBetAmount, game.allInAmount);

  const humanIsDealer = game.humanPosition === 'dealer';
  const handInProgress = game.street !== 'showdown' && game.winner === '';
  const isHumanTurn = handInProgress && game.whoseTurn === game.humanPosition;
  const isCPUTurn = handInProgress && game.whoseTurn !== game.humanPosition;
  const logEntries = annotateHistory(game.history, game.humanPosition, game.winner, game.pot);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [logEntries.length]);

  const updateBetAmount = (amount) => setBetAmount(amount);
  const updateBetAmountFromEvent = (e) => {
    const amount = parseInt(e.target.value);
    if (amount > 0) setBetAmount(amount);
  };

  return (
    <div className="app">
      <Score score={game.score} hands={game.numHands} />
      <div className="game-area">
        <Table
          pot={game.pot}
          cpuStack={cpuStack}
          humanStack={humanStack}
          humanCards={displayedHumanCards}
          cpuCards={displayedCPUCards}
          board={displayedBoard}
          humanIsDealer={humanIsDealer}
          isHumanTurn={isHumanTurn}
          isCPUTurn={isCPUTurn}
          humanContribution={game.humanContribution}
          cpuContribution={game.cpuContribution}
        />
        <div className="action-log" ref={logRef}>
          {logEntries.map((entry, i) => (
            <div key={i} className={`log-entry log-${entry.type}`}>
              {entry.player && (
                <span className={`log-player ${entry.player === 'You' ? 'log-you' : 'log-cpu'}`}>
                  {entry.player}
                </span>
              )}
              <span className="log-text">{entry.text}</span>
            </div>
          ))}
        </div>
      </div>
      <Buttons
        nextHand={game.nextHand}
        fold={game.fold}
        check={game.check}
        call={game.call}
        callAmount={game.callAmount}
        bet={() => game.bet(betAmount)}
        updateBetAmount={updateBetAmount}
        updateBetAmountFromEvent={updateBetAmountFromEvent}
        betAmount={betAmount}
        minBetAmount={game.minBetAmount}
        allInAmount={game.allInAmount}
        pot={game.pot}
        enabledButtons={enabledButtons}
        disabled={game.loading}
      />
    </div>
  );
}
