# Poker

## Description

Recent advancements in game AI have produced astounding results, such as Google's AlphaGo and OpenAI's Dota bot. However, for many years, the goal of writing a computer program to beat humans at No Limit Texas Holdem has remained elusive. That is until 2017, when researchers at Carnegie Mellon University were finally able to crack the barrier, when their bot Libratus defeated top human opponents with statistical significance. Libratus ran on a supercomputer, but in 2018 a technique called Depth Limited Solving was discovered which vastly reduced the required computational resources to the point that a superhuman poker program could run on your laptop. The goal of this project is to create the first opensource version of this superhuman Texas Holdem bot. 

## Running the program
TODO

## How this works

Poker is an "imperfect information game", which means it's a lot harder to solve than perfect information games like chess. In chess you can just search the game tree for the best move, but in poker, you don't actually know where you are in the game tree because you can't see your opponent's cards. This changes things, so instead of solving for a "best move" you have to solve for an optimal strategy--the Nash Equilibrium.

A simple example of an imperfect information game is Rock Paper Scissors. The optimal strategy is to choose randomly between rock, paper, and scissors, and if you do this, you can't lose in the long run. This is the only strategy that is unexploitable, because if you're biased towards one move, the opponent can use that information against you. There is no one "best move" because you need to choose randomly otherwise the opponent can adapt to beat you. This is like in poker, where sometimes there is no one optimal move, but you have to switch it up between several actions, because if you did the same thing every time in each spot, your opponent could figure it out and know what cards you have. 

It's guaranteed that for any two player imperfect information game (like Heads Up No Limit Texas Holdem) there exists at least one Nash Equilibrium strategy, and if you play by it, you cannot lose in the long run. In real life though, if you play a Nash Equilibrium strategy in poker, you will destroy your opponents since chances are they aren't playing an optimal strategy. 

So we know that this strategy exists, but how do we solve for it? In 2007, researchers at Google Brain invented a technique known as "Counterfactual Regret Minimization" which can solve for the Nash Equilibrium. This algorithm works by having two bots play against each other, where first they just make random moves, but at each decision, they write down what they regret not doing (ie what would have made them win more). Then, they do more of what they regret doing and less of what they don't regret. 

While this algorithm works in theory, in reality HUNL Texas Holdem is just too big a game to solve directly. Checkers has 1e20 possible positions, chess has 1e120, but HUNL has 1e180, which is more than the number of atoms in the universe squared. So in order to solve it, you need to find a way to shrink the game down. This is accomplished using a technique known as **abstraction**. We can shrink down poker in two ways:
1. Treat similar hands as identical. AsKs 2c3cAc is close enough to AsKs 3c4cAc. 
2. Treat similar bet sizes as equal. A bet of $1000 is close enough to a bet of $1001. 

When the bot is playing, it translates the cards and bets that it sees into their abstracted versions, and calculates its strategy only for those. 
## Implementation Details

### Abstraction

### Training

### Measuring Exploitability

### Heads-up comparison vs. other top bots


