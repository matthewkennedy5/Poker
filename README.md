# Poker

## Description

Recent advancements in game AI have produced astounding results, such as Google's AlphaGo and OpenAI's Dota bot. However, for many years, the goal of writing a computer program to beat humans at No Limit Texas Holdem has remained elusive. That is until 2017, when researchers at Carnegie Mellon University were finally able to crack the barrier, when their bot Libratus defeated top human opponents with statistical significance. Libratus ran on a supercomputer, but in 2018 a technique called Depth Limited Solving was discovered which vastly reduced the required computational resources to the point that a superhuman poker program could run on your laptop. The goal of this project is to create the first opensource version of this superhuman Texas Holdem bot. 

## Running the program
TODO

## How this works

Poker is an "imperfect information game", which means it's a lot harder to solve than perfect information games like chess. In chess you can just search the game tree for the best move, but in poker, you don't actually know where you are in the game tree because you can't see your opponent's cards. This changes things, so instead of solving for a "best move" you have to solve for an optimal strategy--the Nash Equilibrium.

A simple example of an imperfect information game is Rock Paper Scissors. The optimal strategy is to choose randomly between rock, paper, and scissors, and if you do this, you can't lose in the long run. This is the only strategy that is unexploitable, because if you're biased towards one move, the opponent can use that information against you. There is no one "best move" because you need to choose randomly otherwise the opponent can adapt to beat you. This is like in poker, where sometimes there is no one optimal move, but you have to switch it up between several actions, because if you did the same thing every time in each spot, your opponent could figure it out and know what cards you have. 

It's guaranteed that for any two player imperfect information game (like Heads Up No Limit Texas Holdem) there exists at least one Nash Equilibrium strategy, and if you play by it, you cannot lose in the long run. In real life though, if you play a Nash Equilibrium strategy in poker, you will destroy your opponents since chances are they aren't playing an optimal strategy. 

So we know that this strategy exists, but how do we solve for it? In 2007, researchers at Google Brain invented a technique known as "Counterfactual Regret Minimization" which can solve for the Nash Equilibrium. This algorithm works by having two bots play against each other, where first they just make random moves, but at each decision point, they write down what they regret not doing (ie what would have made them win more). Then, they do more of what they regret doing and less of what they don't regret. It is proven that if you run this algorithm for long enough, it will give you a Nash Equilibrium strategy. 

While this algorithm works in theory, in reality HUNL Texas Holdem is just too big a game to solve directly. Checkers has 1e20 possible positions, chess has 1e120, but HUNL has 1e180, which is more than the number of atoms in the universe squared. So in order to solve it, you need to find a way to shrink the game down. This is accomplished using a technique known as **abstraction**. We can shrink down poker in two ways:
1. Treat similar hands as identical. AsKs 2c3cAc is close enough to AsKs 3c4cAc. 
2. Treat similar bet sizes as equal. A bet of $1000 is close enough to a bet of $1001. 

When the bot is training, it translates the cards and bets that it sees into their abstracted versions, and calculates its strategy only for those. Then when it comes to playing the game, you have to hope that your abstract strategy will work well for the full game. However, in practice it turns out that this is hard to do, and a worst-case opponent will be able to destroy most abstract strategies, because there will be chinks in the armor that can be exploited. As the abstraction becomes larger, better representing the full game, this becomes less of a problem, and historically poker bots have improved by adopting larger and larger abstractions, requiring more and more memory. 

So it would be nice if there were a way to figure out what to do when you encounter a situation outside the abstraction. This can be accomplished using **Depth Limited Solving** which refines the abstract strategy by playing it against several possible opponent strategies. Using Depth Limited Solving allows the bot to close the chinks in the abstract strategy and play a true approximate Nash equilibrium. 

## Implementation Details

### Abstraction

#### Card abstraction

Finding good ways to group hands together for abstraction has historically been a hot research area, and finding better abstractions was the key driver of progress before nested subgame solving was invented. Initially, people would manually write down rules to classify hands, such as by type, draws, and stuff like that. Over time, numeric techniques were used to greater success, such as calculating a hand's expected equity or expected equity squared. However, if you just look at these variables, you run into an issue because some vastly different hands can have similar equities. For example, a straight draw could have a similar equity to a low pair, even though they are vastly different types of hands. To better classify hands, it pays to look at the full **equity distribution** over all possible rollouts. *add pictures of graphs* Then these equity distributions are clustered using k-means clustering with the **Earth Mover's Distance** so that hands with similar equity distributions will be put together. (cite paper)

The above analysis only applies to the flop and turn. On the river, there are no rollouts and no notion of an equity distributions, so hands are just clustered based on their equity. On the preflop, there are only 169 strategically distinct hands, so we can just include all of them in the abstraction.

It is up to the programmer to decide how many clusters to use--more clusters will produce a better strategy, but will be slower to train. I chose to use ?? buckets on the flop, ?? on the turn, and ?? on the river. 

#### Action abstraction

Simplifying the actions is a much simpler task and can be succesfully done by a human. Since there are many possible Nash equilibria, chances are good that one will exist for the actions you choose to include in your abstraction. For my bot I allowed the following bet sizes: (half pot, full pot, 2x pot, all in). 

### Training

Once the abstraction is all set, it is time to calculate a blueprint Nash equilibrium strategy. It's okay if this strategy isn't great, because the bot will improve on it later using depth-limited solving. Since the discovery of Counterfactual Regret Minimization (CFR), people have figured out ways to tweak it so it converges faster. The variant I chose to use is called **Discounted Regret Minimization** (cite paper) with parameters ???

### Measuring Exploitability

It is useful to have a notion of the quality of a strategy, akin to "loss" in machine learning. For us this quantity is called **exploitability**, which is a measurement of how badly your opponent could beat you if they knew the exact strategy you play by. The exploitability of a Nash equilibrium strategy is 0 because it is not possible to beat a Nash equilibrium strategy in expectation. 

However, to truly calculate the exploitability, you'd have to run CFR to train an opponent against your strategy, and that would take a while. Fortunately, researchers at ??? came up with a fast way to calculate a lower bound of the exploitability which ends up being pretty good in practice. You can run this during training to see how good your blueprint strategy is getting. 

### Real time solving

### Heads-up comparison vs. other top bots

### Compute requirements

Another goal of this project was to write a bot that could be trained and run on laptop, no supercomputer needed. I first started writing this in Python, but then switched to Rust because Python was too slow and inefficient to accomplish this goal. All computation for the final version of this bot was done on my laptop which has an 8-core CPU and 8 GB of RAM. 

## Papers cited

## Further reading


