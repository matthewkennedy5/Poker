export function getDisplayedBoard(board, street) {
  if (street === 'preflop') return ['back', 'back', 'back', 'back', 'back'];
  if (street === 'flop') return [board[0], board[1], board[2], 'back', 'back'];
  if (street === 'turn') return [board[0], board[1], board[2], board[3], 'back'];
  if (street === 'river' || street === 'showdown') return board;
  throw new Error('Bad street value');
}

export function annotateHistory(history, humanPosition, winner, pot) {
  const isHumanDealer = humanPosition === 'dealer';
  const dealerLabel = isHumanDealer ? 'You' : 'CPU';
  const opponentLabel = isHumanDealer ? 'CPU' : 'You';

  const entries = [];
  entries.push({ type: 'blind', player: dealerLabel, text: 'SB $50' });
  entries.push({ type: 'blind', player: opponentLabel, text: 'BB $100' });

  const streetNames = ['Flop', 'Turn', 'River'];
  let streetIdx = 0;
  let invested = [0, 0]; // [dealer, opponent] — action amounts are total for the street
  let streetLength = 0;
  let currentActorIsDealer = true; // preflop: dealer (SB) acts first
  let streetHasBet = true; // preflop: BB counts as existing bet

  for (const { action, amount } of history) {
    const actorLabel = currentActorIsDealer ? dealerLabel : opponentLabel;

    let text;
    if (action === 'Fold') {
      text = 'folds';
    } else if (action === 'Call') {
      text = amount === 0 ? 'checks' : `calls $${amount}`;
    } else if (action === 'Bet') {
      if (streetHasBet) {
        text = `raises $${amount}`;
      } else {
        text = `bets $${amount}`;
        streetHasBet = true;
      }
    }

    entries.push({ type: 'action', player: actorLabel, text });

    const actorIdx = currentActorIsDealer ? 0 : 1;
    invested[actorIdx] += amount;
    streetLength++;

    if (action !== 'Fold' && invested[0] === invested[1] && streetLength >= 2) {
      if (streetIdx < streetNames.length) {
        entries.push({ type: 'street', text: streetNames[streetIdx] });
        streetIdx++;
      }
      invested = [0, 0];
      streetLength = 0;
      currentActorIsDealer = false; // opponent acts first postflop
      streetHasBet = false;
    } else {
      currentActorIsDealer = !currentActorIsDealer;
    }
  }

  if (winner === 'human') {
    entries.push({ type: 'result', text: `You win $${pot}` });
  } else if (winner === 'cpu') {
    entries.push({ type: 'result', text: `CPU wins $${pot}` });
  }

  return entries;
}
