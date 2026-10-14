import { useReducer, useRef, useEffect } from 'react';
import { Random } from 'random-js';
import { getDisplayedBoard } from './utils';

const DECK = [
  '2c', '2s', '2h', '2d', '3c', '3s', '3h', '3d',
  '4c', '4s', '4h', '4d', '5c', '5s', '5h', '5d',
  '6c', '6s', '6h', '6d', '7c', '7s', '7h', '7d',
  '8c', '8s', '8h', '8d', '9c', '9s', '9h', '9d',
  'Tc', 'Ts', 'Th', 'Td', 'Jc', 'Js', 'Jh', 'Jd',
  'Qc', 'Qs', 'Qh', 'Qd', 'Kc', 'Ks', 'Kh', 'Kd',
  'Ac', 'As', 'Ah', 'Ad',
];

const CPU_ACTION_PAUSE_MS = 1000;

const initialState = {
  humanCards: [],
  cpuCards: [],
  board: [],
  history: [],
  street: 'preflop',
  pot: 0,
  stacks: { dealer: 20000, opponent: 20000 },
  whoseTurn: 'dealer',
  winner: '',
  score: 0,
  numHands: 0,
  humanPosition: 'opponent',
  callAmount: 0,
  minBetAmount: 0,
  allInAmount: 0,
  loading: false,
  handStartScore: 0,
  gameOver: false,
};

function reducer(state, payload) {
  return { ...state, ...payload };
}

function otherPosition(position) {
  return position === 'dealer' ? 'opponent' : 'dealer';
}

// Stacks reset to 200 BB every hand (like Slumbot): the bot is trained at that depth,
// and the running score is kept separately.
function deriveStacks(score, humanPosition) { // eslint-disable-line no-unused-vars
  return { dealer: 20000, opponent: 20000 };
}

async function fetchHistoryInfo(history, humanPosition, humanCards, cpuCards, board, stacks) {
  const dealerCards = humanPosition === 'dealer' ? humanCards : cpuCards;
  const opponentCards = humanPosition === 'dealer' ? cpuCards : humanCards;
  const res = await fetch('/api/historyInfo', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ history, dealerCards, opponentCards, boardCards: board, stacks }),
  });
  return res.json();
}

async function fetchCPUAction(cpuCards, displayedBoard, history, stacks) {
  const res = await fetch('/api/bot', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ cpuCards, board: displayedBoard, history, stacks }),
  });
  return res.json();
}

const sleep = (ms) => new Promise(r => setTimeout(r, ms));

function resolveHand(gameState, history, humanPosition, currentScore, currentNumHands) {
  let winner = '';
  let score = currentScore;
  let numHands = currentNumHands;
  let street = gameState.street;

  if (gameState.winnings !== 0) {
    const humanProfit = humanPosition === 'dealer' ? gameState.winnings : -gameState.winnings;
    winner = humanProfit > 0 ? 'human' : (humanProfit < 0 ? 'cpu' : '');
    score += humanProfit;
    numHands += 1;
    if (history[history.length - 1].action !== 'Fold') {
      street = 'showdown';
    }
  }

  return { winner, score, numHands, street };
}

function makeGameDispatch(gameState, history, extra) {
  return {
    history,
    street: gameState.street,
    pot: gameState.pot,
    stacks: gameState.stacks,
    whoseTurn: gameState.whoseTurn,
    callAmount: gameState.callAmount,
    minBetAmount: gameState.minBetAmount,
    allInAmount: gameState.allInAmount,
    ...extra,
  };
}

export default function useGame() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const started = useRef(false);

  // Run CPU actions one at a time, dispatching after each so the UI shows it
  async function runCPULoop(history, humanPosition, humanCards, cpuCards, board, gameState, stacks) {
    while (gameState.winnings === 0 && gameState.whoseTurn !== humanPosition) {
      const displayedBoard = getDisplayedBoard(board, gameState.street);
      const cpuAction = await fetchCPUAction(cpuCards, displayedBoard, history, stacks);
      history = [...history, cpuAction];
      gameState = await fetchHistoryInfo(history, humanPosition, humanCards, cpuCards, board, stacks);

      // If there are MORE CPU actions coming, show this one and pause
      if (gameState.winnings === 0 && gameState.whoseTurn !== humanPosition) {
        dispatch(makeGameDispatch(gameState, history, { loading: true }));
        await sleep(CPU_ACTION_PAUSE_MS);
      }
    }
    return { history, gameState };
  }

  const nextHand = async () => {
    dispatch({ loading: true });

    try {
      const newPosition = otherPosition(state.humanPosition);
      const deck = [...DECK];
      new Random().shuffle(deck);
      const humanCards = deck.slice(0, 2);
      const cpuCards = deck.slice(2, 4);
      const board = deck.slice(4, 9);

      const stacks = deriveStacks(state.score, newPosition);

      let gameState = await fetchHistoryInfo([], newPosition, humanCards, cpuCards, board, stacks);

      // Show the new hand before CPU acts
      dispatch(makeGameDispatch(gameState, [], {
        humanCards, cpuCards, board,
        humanPosition: newPosition,
        winner: '',
        loading: true,
        handStartScore: state.score,
      }));

      const { history, gameState: finalState } = await runCPULoop([], newPosition, humanCards, cpuCards, board, gameState, stacks);
      const resolved = resolveHand(finalState, history, newPosition, state.score, state.numHands);

      const newStacks = deriveStacks(resolved.score, newPosition);
      const minStack = Math.min(newStacks.dealer, newStacks.opponent);
      const gameOver = minStack < 100;

      dispatch(makeGameDispatch(finalState, history, {
        humanCards, cpuCards, board,
        humanPosition: newPosition,
        street: resolved.street,
        winner: resolved.winner,
        score: resolved.score,
        numHands: resolved.numHands,
        loading: false,
        gameOver,
      }));
    } catch (e) {
      console.error('nextHand error:', e);
      dispatch({ loading: false });
    }
  };

  const performAction = async (action) => {
    dispatch({ loading: true });

    try {
      const humanPosition = state.humanPosition;
      const humanCards = state.humanCards;
      const cpuCards = state.cpuCards;
      const board = state.board;

      const stacks = deriveStacks(state.score, humanPosition);

      let history = [...state.history, action];
      let gameState = await fetchHistoryInfo(history, humanPosition, humanCards, cpuCards, board, stacks);

      // Show updated state after human action before CPU acts
      if (gameState.winnings === 0 && gameState.whoseTurn !== humanPosition) {
        dispatch(makeGameDispatch(gameState, history, { loading: true }));
      }

      const { history: finalHistory, gameState: finalState } = await runCPULoop(history, humanPosition, humanCards, cpuCards, board, gameState, stacks);
      const resolved = resolveHand(finalState, finalHistory, humanPosition, state.score, state.numHands);

      const newStacks = deriveStacks(resolved.score, humanPosition);
      const minStack = Math.min(newStacks.dealer, newStacks.opponent);
      const gameOver = minStack < 100;

      dispatch(makeGameDispatch(finalState, finalHistory, {
        street: resolved.street,
        winner: resolved.winner,
        score: resolved.score,
        numHands: resolved.numHands,
        loading: false,
        gameOver,
      }));
    } catch (e) {
      console.error('performAction error:', e);
      dispatch({ loading: false });
    }
  };

  const fold = () => performAction({ action: 'Fold', amount: 0 });
  const check = () => performAction({ action: 'Call', amount: 0 });
  const call = () => performAction({ action: 'Call', amount: state.callAmount });
  const bet = (amount) => performAction({ action: 'Bet', amount });

  const resetGame = () => {
    dispatch(initialState);
    started.current = false;
  };

  useEffect(() => {
    if (!started.current) {
      started.current = true;
      nextHand();
    }
  }); // eslint-disable-line react-hooks/exhaustive-deps

  const handScore = state.handStartScore !== undefined ? state.handStartScore : state.score;
  const startingStacks = deriveStacks(handScore, state.humanPosition);
  const humanPos = state.humanPosition;
  const cpuPos = humanPos === 'dealer' ? 'opponent' : 'dealer';
  const humanContribution = Math.max(0, startingStacks[humanPos] - state.stacks[humanPos]);
  const cpuContribution = Math.max(0, startingStacks[cpuPos] - state.stacks[cpuPos]);

  return { ...state, humanContribution, cpuContribution, nextHand, fold, check, call, bet, gameOver: state.gameOver, resetGame };
}
