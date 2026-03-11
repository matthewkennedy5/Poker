import React, { useState } from 'react';
import useGame from './useGame';
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

function getDisplayedBoard(board, street) {
  if (street === 'preflop') return ['back', 'back', 'back', 'back', 'back'];
  if (street === 'flop') return [board[0], board[1], board[2], 'back', 'back'];
  if (street === 'turn') return [board[0], board[1], board[2], board[3], 'back'];
  if (street === 'river' || street === 'showdown') return board;
  throw new Error('Bad street value');
}

function getEnabledButtons(street, history, callAmount, minBetAmount, allInAmount) {
  let prevAction;
  if (history.length > 0) {
    prevAction = history[history.length - 1];
  }
  // eslint-disable-next-line no-mixed-operators
  if (prevAction !== undefined && prevAction.action === 'Fold' || street === 'showdown') {
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

function getCPUActionText(game) {
  const { street, winner, pot, history, whoseTurn, humanPosition } = game;

  if (street === 'showdown') {
    if (winner === 'human') return 'You win $' + pot;
    if (winner === 'cpu') return 'CPU wins $' + pot;
  }

  let prevAction;
  if (whoseTurn === humanPosition && history.length >= 1) {
    prevAction = history[history.length - 1];
  } else if (whoseTurn !== humanPosition && history.length >= 2) {
    prevAction = history[history.length - 2];
  }

  if (prevAction === undefined) {
    return whoseTurn === humanPosition ? 'Your turn' : '';
  }
  if (prevAction.action === 'Fold') return 'CPU folds';
  if (prevAction.action === 'Call') {
    return prevAction.amount === 0 ? 'CPU checks' : 'CPU calls $' + prevAction.amount;
  }
  if (prevAction.action === 'Bet') return 'CPU bets $' + prevAction.amount;
  return 'Your turn';
}

export default function App() {
  const game = useGame();
  const [betAmount, setBetAmount] = useState(0);

  const humanStack = game.stacks[game.humanPosition];
  const cpuStack = game.stacks[game.humanPosition === 'dealer' ? 'opponent' : 'dealer'];
  const displayedBoard = getDisplayedBoard(game.board, game.street);
  const displayedHumanCards = getDisplayedHumanCards(game.humanCards);
  const displayedCPUCards = getDisplayedCPUCards(game.cpuCards, game.street);
  const enabledButtons = getEnabledButtons(game.street, game.history, game.callAmount, game.minBetAmount, game.allInAmount);
  const cpuActionText = getCPUActionText(game);

  const updateBetAmount = (amount) => setBetAmount(amount);
  const updateBetAmountFromEvent = (e) => {
    const amount = parseInt(e.target.value);
    if (amount > 0) setBetAmount(amount);
  };

  return (
    <div className="app">
      <Score score={game.score} hands={game.numHands} />
      <Table
        pot={game.pot}
        cpuStack={cpuStack}
        humanStack={humanStack}
        cpuActionText={cpuActionText}
        humanCards={displayedHumanCards}
        cpuCards={displayedCPUCards}
        board={displayedBoard}
      />
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
