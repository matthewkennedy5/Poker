//! Single-street vectorized DCFR solver with pluggable leaf values, for
//! depth-limited search.
//!
//! The tree is one betting round, built from postflop-solver's `ActionTree` so bet
//! sizing matches it exactly. Folds and river showdowns are terminals; nodes where
//! the street ends without a showdown (a chance node in postflop-solver's tree, i.e.
//! the next card is due, or an all-in was called) are leaves whose counterfactual
//! values come from a `LeafEvaluator`: exact runout equity for tests and all-ins,
//! a value network later.
//!
//! Conventions follow postflop-solver: player 0 = OOP, 1 = IP; card id =
//! 4 * rank + suit (rank 0 = deuce); payoffs are chips relative to an equal split
//! of the starting pot (a fold wins or loses half the pot implied by the smaller
//! bet), divided by the number of compatible hand pairs, so the sum of
//! `w_p[i] * cfv_p[i]` is player p's expected value. DCFR uses postflop-solver's
//! discounting (alpha 1.5, beta 0.5, gamma 3 with resets at powers of 4).
use postflop_solver::{Action as SAction, ActionTree};
use rs_poker::core::{Card as RsCard, Rank, Rankable, Suit, Value};

/// Every hole-card pair that does not overlap the board, sorted like
/// postflop-solver's `private_cards`.
pub struct Hands {
    pub cards: Vec<(u8, u8)>,
    index: Vec<i32>, // 52 * 52 -> hand index or -1
}

impl Hands {
    pub fn new(board: &[u8]) -> Hands {
        let mut cards = Vec::new();
        let mut index = vec![-1; 52 * 52];
        for a in 0..52u8 {
            for b in a + 1..52u8 {
                if board.contains(&a) || board.contains(&b) {
                    continue;
                }
                index[a as usize * 52 + b as usize] = cards.len() as i32;
                index[b as usize * 52 + a as usize] = cards.len() as i32;
                cards.push((a, b));
            }
        }
        Hands { cards, index }
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn index(&self, a: u8, b: u8) -> Option<usize> {
        match self.index[a as usize * 52 + b as usize] {
            -1 => None,
            i => Some(i as usize),
        }
    }
}

struct Seven([RsCard; 7]);
impl Rankable for Seven {
    fn cards(&self) -> &[RsCard] {
        &self.0
    }
}

fn rs_card(id: u8) -> RsCard {
    RsCard {
        value: Value::from_u8(id / 4),
        suit: Suit::from_u8(id % 4),
    }
}

/// Hand-strength keys on a complete board (higher is stronger, equal means a tie);
/// `None` for hands that overlap the board.
pub fn strength_keys(board: &[u8], hands: &Hands) -> Vec<Option<u32>> {
    assert_eq!(board.len(), 5);
    let b: Vec<RsCard> = board.iter().map(|&c| rs_card(c)).collect();
    let ranks: Vec<Option<Rank>> = hands
        .cards
        .iter()
        .map(|&(x, y)| {
            if board.contains(&x) || board.contains(&y) {
                None
            } else {
                Some(Seven([rs_card(x), rs_card(y), b[0], b[1], b[2], b[3], b[4]]).rank())
            }
        })
        .collect();
    // Dense ordinal keys, so comparisons are integer comparisons.
    let mut distinct: Vec<Rank> = ranks.iter().flatten().cloned().collect();
    distinct.sort_by(|a, b| a.partial_cmp(b).unwrap());
    distinct.dedup();
    ranks
        .into_iter()
        .map(|r| r.map(|r| distinct.binary_search_by(|d| d.partial_cmp(&r).unwrap()).unwrap() as u32))
        .collect()
}

/// Counterfactual values of a fold or showdown for one player against the opponent's
/// reach, with card removal. `amount` is the payoff per unit of opponent reach.
fn fold_cfv(hands: &Hands, opp_reach: &[f32], amount: f64, out: &mut [f32]) {
    let mut total = 0.0f64;
    let mut per_card = [0.0f64; 52];
    for (j, &(a, b)) in hands.cards.iter().enumerate() {
        let r = opp_reach[j] as f64;
        total += r;
        per_card[a as usize] += r;
        per_card[b as usize] += r;
    }
    for (i, &(a, b)) in hands.cards.iter().enumerate() {
        // the identical hand was subtracted twice
        let compatible = total - per_card[a as usize] - per_card[b as usize] + opp_reach[i] as f64;
        out[i] = (amount * compatible) as f32;
    }
}

/// River showdown: hands sorted by strength; win against weaker compatible hands,
/// lose against stronger ones.
pub struct Showdown {
    order: Vec<usize>, // valid hands, ascending strength
    keys: Vec<u32>,    // strength key per hand (unused for invalid hands)
}

impl Showdown {
    pub fn new(board: &[u8], hands: &Hands) -> Showdown {
        let keys = strength_keys(board, hands);
        let mut order: Vec<usize> = (0..hands.len()).filter(|&i| keys[i].is_some()).collect();
        order.sort_by_key(|&i| keys[i].unwrap());
        Showdown { order, keys: keys.into_iter().map(|k| k.unwrap_or(0)).collect() }
    }

    fn cfv(&self, hands: &Hands, opp_reach: &[f32], amount: f64, out: &mut [f32]) {
        out.iter_mut().for_each(|v| *v = 0.0);
        let mut acc = vec![0.0f64; out.len()];
        // ascending: + mass of weaker compatible hands
        let (mut w, mut wc) = (0.0f64, [0.0f64; 52]);
        let n = self.order.len();
        let mut g = 0;
        while g < n {
            let key = self.keys[self.order[g]];
            let mut e = g;
            while e < n && self.keys[self.order[e]] == key {
                e += 1;
            }
            for &i in &self.order[g..e] {
                let (a, b) = hands.cards[i];
                acc[i] += w - wc[a as usize] - wc[b as usize];
            }
            for &j in &self.order[g..e] {
                let (a, b) = hands.cards[j];
                let r = opp_reach[j] as f64;
                w += r;
                wc[a as usize] += r;
                wc[b as usize] += r;
            }
            g = e;
        }
        // descending: - mass of stronger compatible hands
        let (mut s, mut sc) = (0.0f64, [0.0f64; 52]);
        let mut e = n;
        while e > 0 {
            let key = self.keys[self.order[e - 1]];
            let mut g = e;
            while g > 0 && self.keys[self.order[g - 1]] == key {
                g -= 1;
            }
            for &i in &self.order[g..e] {
                let (a, b) = hands.cards[i];
                acc[i] -= s - sc[a as usize] - sc[b as usize];
            }
            for &j in &self.order[g..e] {
                let (a, b) = hands.cards[j];
                let r = opp_reach[j] as f64;
                s += r;
                sc[a as usize] += r;
                sc[b as usize] += r;
            }
            e = g;
        }
        for &i in &self.order {
            out[i] = (amount * acc[i]) as f32;
        }
    }
}

/// A leaf: the street ended (next card due) or an all-in was called before the river.
#[derive(Clone, Debug)]
pub struct LeafInfo {
    pub id: usize,
    /// Pot at the leaf (starting pot plus both bets).
    pub pot: i32,
    /// Remaining effective stack.
    pub stack: i32,
    pub allin: bool,
    /// The street's actions that lead here.
    pub line: Vec<SAction>,
}

pub trait LeafEvaluator: Sync {
    /// Writes `out[i] = sum_j reach[opp][j] * u(i, j)` for `player`'s hand i, where u is
    /// the chip payoff relative to an equal split of the pot at the leaf (so +-pot/2 at
    /// most), over compatible pairs. `reach[player]` is supplied for evaluators whose
    /// values depend on both ranges (value nets).
    fn evaluate(&self, leaf: &LeafInfo, player: usize, reach: [&[f32]; 2], out: &mut [f32]);
}

/// Exact equity over all runouts to the river (no further betting). Exact for
/// all-in leaves; for street-end leaves it means "check it down".
pub struct RunoutEquity {
    n: usize,
    matrix: Vec<f32>, // n * n, average of +1 / 0 / -1 for row hand vs column hand
}

impl RunoutEquity {
    pub fn new(board: &[u8], hands: &Hands) -> RunoutEquity {
        let n = hands.len();
        let rest: Vec<u8> = (0..52u8).filter(|c| !board.contains(c)).collect();
        let need = 5 - board.len();
        let mut runouts: Vec<Vec<u8>> = Vec::new();
        match need {
            0 => runouts.push(vec![]),
            1 => rest.iter().for_each(|&c| runouts.push(vec![c])),
            2 => {
                for (k, &c) in rest.iter().enumerate() {
                    for &d in &rest[k + 1..] {
                        runouts.push(vec![c, d]);
                    }
                }
            }
            _ => panic!("RunoutEquity needs a flop, turn or river board"),
        }
        let mut sum = vec![0i32; n * n];
        for r in &runouts {
            let full: Vec<u8> = board.iter().chain(r.iter()).cloned().collect();
            let keys = strength_keys(&full, hands);
            for i in 0..n {
                let Some(ki) = keys[i] else { continue };
                let (a, b) = hands.cards[i];
                for j in i + 1..n {
                    let Some(kj) = keys[j] else { continue };
                    let (c, d) = hands.cards[j];
                    if a == c || a == d || b == c || b == d {
                        continue;
                    }
                    let s = (ki > kj) as i32 - (ki < kj) as i32;
                    sum[i * n + j] += s;
                    sum[j * n + i] -= s;
                }
            }
        }
        // every compatible pair sees the same number of runouts
        let count = match need {
            0 => 1.0,
            1 => 44.0,
            _ => 990.0,
        };
        let matrix = sum.into_iter().map(|s| s as f32 / count).collect();
        RunoutEquity { n, matrix }
    }
}

impl LeafEvaluator for RunoutEquity {
    fn evaluate(&self, leaf: &LeafInfo, player: usize, reach: [&[f32]; 2], out: &mut [f32]) {
        let opp = reach[player ^ 1];
        let half_pot = 0.5 * leaf.pot as f32;
        for i in 0..self.n {
            let row = &self.matrix[i * self.n..(i + 1) * self.n];
            let v: f32 = row.iter().zip(opp).map(|(m, r)| m * r).sum();
            out[i] = half_pot * v;
        }
    }
}

enum Kind {
    Player { player: usize, children: Vec<usize> },
    Fold { folder: usize, half_pot: f64 },
    Showdown { half_pot: f64 },
    Leaf(LeafInfo),
}

struct Node {
    kind: Kind,
    actions: Vec<SAction>,
    regrets: Vec<f32>,   // actions * hands
    cum_strategy: Vec<f32>,
}

pub struct StreetGame {
    pub board: Vec<u8>,
    pub hands: Hands,
    pub weights: [Vec<f32>; 2],
    pub starting_pot: i32,
    pub num_leaves: usize,
    nodes: Vec<Node>,
    showdown: Option<Showdown>,
    inv_num_combinations: f64,
}

struct Discount {
    alpha: f32,
    beta: f32,
    gamma: f32,
}

impl Discount {
    // Same schedule as postflop-solver's DiscountParams.
    fn new(t: u32) -> Discount {
        let lower_pow4 = match t {
            0 => 0,
            x => 1 << ((x.leading_zeros() ^ 31) & !1),
        };
        let ta = (t as i32 - 1).max(0) as f64;
        let tg = (t - lower_pow4) as f64;
        let pa = ta * ta.sqrt();
        Discount {
            alpha: (pa / (pa + 1.0)) as f32,
            beta: 0.5,
            gamma: (tg / (tg + 1.0)).powi(3) as f32,
        }
    }
}

fn regret_matching(regrets: &[f32], num_actions: usize, n: usize) -> Vec<f32> {
    let mut s: Vec<f32> = regrets.iter().map(|&r| r.max(0.0)).collect();
    for i in 0..n {
        let total: f32 = (0..num_actions).map(|a| s[a * n + i]).sum();
        for a in 0..num_actions {
            s[a * n + i] = if total > 0.0 { s[a * n + i] / total } else { 1.0 / num_actions as f32 };
        }
    }
    s
}

impl StreetGame {
    /// Builds the street's tree from `tree` (positioned anywhere; it is rewound to the
    /// root). `weights` are per hand in `Hands::new(board)` order. Errors if no pair of
    /// hands is compatible.
    pub fn new(board: &[u8], weights: [Vec<f32>; 2], tree: &mut ActionTree) -> Result<StreetGame, String> {
        let hands = Hands::new(board);
        if weights[0].len() != hands.len() || weights[1].len() != hands.len() {
            return Err("weights must have one entry per non-board hand".to_string());
        }
        let config = tree.config().clone();
        let mut num_comb = 0.0f64;
        {
            let mut per_card = [0.0f64; 52];
            let mut total = 0.0f64;
            for (j, &(a, b)) in hands.cards.iter().enumerate() {
                let w = weights[1][j] as f64;
                total += w;
                per_card[a as usize] += w;
                per_card[b as usize] += w;
            }
            for (i, &(a, b)) in hands.cards.iter().enumerate() {
                let compatible = total - per_card[a as usize] - per_card[b as usize] + weights[1][i] as f64;
                num_comb += weights[0][i] as f64 * compatible;
            }
        }
        if num_comb <= 0.0 {
            return Err("no compatible hand pairs".to_string());
        }
        let mut game = StreetGame {
            board: board.to_vec(),
            showdown: if board.len() == 5 { Some(Showdown::new(board, &hands)) } else { None },
            hands,
            weights,
            starting_pot: config.starting_pot,
            num_leaves: 0,
            nodes: Vec::new(),
            inv_num_combinations: 1.0 / num_comb,
        };
        tree.back_to_root();
        let mut line = Vec::new();
        game.build(tree, &mut line, config.starting_pot, config.effective_stack)?;
        tree.back_to_root();
        Ok(game)
    }

    fn build(&mut self, tree: &mut ActionTree, line: &mut Vec<SAction>, starting_pot: i32, eff: i32) -> Result<usize, String> {
        let n = self.hands.len();
        let id = self.nodes.len();
        let bets = tree.total_bet_amount();
        let called = bets[0].min(bets[1]);
        let half_pot = 0.5 * starting_pot as f64 + called as f64;
        let is_fold = line.last() == Some(&SAction::Fold);
        // A called all-in before the river is a terminal in postflop-solver's tree
        // (the runout is dealt by the game); here it is a leaf like the street end.
        let kind = if tree.is_terminal_node() && (is_fold || self.board.len() == 5) {
            if is_fold {
                Kind::Fold { folder: (line.len() - 1) % 2, half_pot }
            } else {
                Kind::Showdown { half_pot }
            }
        } else if tree.is_terminal_node() || tree.is_chance_node() {
            let leaf = LeafInfo {
                id: self.num_leaves,
                pot: starting_pot + 2 * called,
                stack: eff - called,
                allin: called == eff,
                line: line.clone(),
            };
            self.num_leaves += 1;
            Kind::Leaf(leaf)
        } else {
            Kind::Player { player: line.len() % 2, children: Vec::new() }
        };
        let actions: Vec<SAction> = match kind {
            Kind::Player { .. } => tree.available_actions().to_vec(),
            _ => Vec::new(),
        };
        let size = actions.len() * n;
        self.nodes.push(Node { kind, actions: actions.clone(), regrets: vec![0.0; size], cum_strategy: vec![0.0; size] });
        let mut children = Vec::with_capacity(actions.len());
        for a in actions {
            tree.play(a)?;
            line.push(a);
            children.push(self.build(tree, line, starting_pot, eff)?);
            line.pop();
            tree.undo()?;
        }
        if let Kind::Player { children: c, .. } = &mut self.nodes[id].kind {
            *c = children;
        }
        Ok(id)
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Leaves in id order.
    pub fn leaves(&self) -> Vec<LeafInfo> {
        let mut out: Vec<LeafInfo> = self
            .nodes
            .iter()
            .filter_map(|n| match &n.kind {
                Kind::Leaf(l) => Some(l.clone()),
                _ => None,
            })
            .collect();
        out.sort_by_key(|l| l.id);
        out
    }

    /// One DCFR iteration (both players, alternating).
    pub fn solve_step(&mut self, t: u32, eval: &dyn LeafEvaluator) {
        let d = Discount::new(t);
        for player in 0..2 {
            let w = [self.weights[0].clone(), self.weights[1].clone()];
            self.cfr(0, player, [&w[0], &w[1]], &d, eval);
        }
    }

    pub fn solve(&mut self, iterations: u32, eval: &dyn LeafEvaluator) {
        for t in 0..iterations {
            self.solve_step(t, eval);
        }
    }

    fn terminal_cfv(&self, node: usize, player: usize, reach: [&[f32]; 2], eval: &dyn LeafEvaluator) -> Vec<f32> {
        let n = self.hands.len();
        let mut out = vec![0.0f32; n];
        let opp = reach[player ^ 1];
        match &self.nodes[node].kind {
            Kind::Fold { folder, half_pot } => {
                let sign = if *folder == player { -1.0 } else { 1.0 };
                fold_cfv(&self.hands, opp, sign * half_pot * self.inv_num_combinations, &mut out);
            }
            Kind::Showdown { half_pot } => {
                self.showdown.as_ref().expect("showdown before the river").cfv(
                    &self.hands,
                    opp,
                    half_pot * self.inv_num_combinations,
                    &mut out,
                );
            }
            Kind::Leaf(leaf) => {
                eval.evaluate(leaf, player, reach, &mut out);
                let s = self.inv_num_combinations as f32;
                out.iter_mut().for_each(|v| *v *= s);
            }
            Kind::Player { .. } => unreachable!(),
        }
        out
    }

    fn cfr(&mut self, node: usize, player: usize, reach: [&[f32]; 2], d: &Discount, eval: &dyn LeafEvaluator) -> Vec<f32> {
        let n = self.hands.len();
        let (acting, children) = match &self.nodes[node].kind {
            Kind::Player { player: p, children } => (*p, children.clone()),
            _ => return self.terminal_cfv(node, player, reach, eval),
        };
        let na = children.len();
        let strategy = regret_matching(&self.nodes[node].regrets, na, n);
        let mut result = vec![0.0f32; n];
        if acting == player {
            let mut cfv_actions = vec![0.0f32; na * n];
            for (a, &child) in children.iter().enumerate() {
                let s = &strategy[a * n..(a + 1) * n];
                let own: Vec<f32> = reach[player].iter().zip(s).map(|(r, p)| r * p).collect();
                let mut r2 = reach;
                r2[player] = &own;
                let v = self.cfr(child, player, r2, d, eval);
                for i in 0..n {
                    result[i] += s[i] * v[i];
                }
                cfv_actions[a * n..(a + 1) * n].copy_from_slice(&v);
            }
            let nd = &mut self.nodes[node];
            for (x, y) in nd.cum_strategy.iter_mut().zip(&strategy) {
                *x = *x * d.gamma + y;
            }
            for a in 0..na {
                for i in 0..n {
                    let x = &mut nd.regrets[a * n + i];
                    let coef = if x.is_sign_positive() { d.alpha } else { d.beta };
                    *x = *x * coef + cfv_actions[a * n + i] - result[i];
                }
            }
        } else {
            for (a, &child) in children.iter().enumerate() {
                let s = &strategy[a * n..(a + 1) * n];
                let opp: Vec<f32> = reach[acting].iter().zip(s).map(|(r, p)| r * p).collect();
                let mut r2 = reach;
                r2[acting] = &opp;
                let v = self.cfr(child, player, r2, d, eval);
                for i in 0..n {
                    result[i] += v[i];
                }
            }
        }
        result
    }

    /// Average strategy at a player node: actions * hands, uniform where unvisited.
    fn average_strategy(&self, node: usize) -> Vec<f32> {
        let nd = &self.nodes[node];
        let na = nd.actions.len();
        let n = self.hands.len();
        let mut s = nd.cum_strategy.clone();
        for i in 0..n {
            let total: f32 = (0..na).map(|a| s[a * n + i]).sum();
            for a in 0..na {
                s[a * n + i] = if total > 0.0 { s[a * n + i] / total } else { 1.0 / na as f32 };
            }
        }
        s
    }

    /// Per-hand values for `player` when both play `strategies` (None = average
    /// strategy), or when `player` best-responds (`best`).
    fn value(&self, node: usize, player: usize, reach: [&[f32]; 2], best: bool, strategies: &Option<Vec<Vec<f32>>>, eval: &dyn LeafEvaluator) -> Vec<f32> {
        let n = self.hands.len();
        let (acting, children) = match &self.nodes[node].kind {
            Kind::Player { player: p, children } => (*p, children),
            _ => return self.terminal_cfv(node, player, reach, eval),
        };
        let strategy = match strategies {
            Some(s) => s[node].clone(),
            None => self.average_strategy(node),
        };
        let mut result = vec![if acting == player && best { f32::NEG_INFINITY } else { 0.0 }; n];
        for (a, &child) in children.iter().enumerate() {
            let s = &strategy[a * n..(a + 1) * n];
            let moved: Vec<f32> = reach[acting].iter().zip(s).map(|(r, p)| r * p).collect();
            let mut r2 = reach;
            r2[acting] = &moved;
            let v = self.value(child, player, r2, best, strategies, eval);
            for i in 0..n {
                if acting == player && best {
                    result[i] = result[i].max(v[i]);
                } else if acting == player {
                    result[i] += s[i] * v[i];
                } else {
                    result[i] += v[i];
                }
            }
        }
        result
    }

    fn root_value(&self, player: usize, best: bool, strategies: &Option<Vec<Vec<f32>>>, eval: &dyn LeafEvaluator) -> f64 {
        let v = self.value(0, player, [&self.weights[0], &self.weights[1]], best, strategies, eval);
        v.iter().zip(&self.weights[player]).map(|(&v, &w)| v as f64 * w as f64).sum()
    }

    /// Expected values [OOP, IP] of the average strategy (zero-sum).
    pub fn expected_values(&self, eval: &dyn LeafEvaluator) -> [f64; 2] {
        [self.root_value(0, false, &None, eval), self.root_value(1, false, &None, eval)]
    }

    /// Exploitability of the average strategy in chips: mean of the two players'
    /// best-response gains, as postflop-solver's `compute_exploitability`.
    pub fn exploitability(&self, eval: &dyn LeafEvaluator) -> f64 {
        0.5 * (self.root_value(0, true, &None, eval) + self.root_value(1, true, &None, eval))
    }

    /// Exploitability of an external strategy profile: one actions*hands vector per
    /// node (ignored at non-player nodes).
    pub fn exploitability_of(&self, strategies: Vec<Vec<f32>>, eval: &dyn LeafEvaluator) -> f64 {
        let s = Some(strategies);
        0.5 * (self.root_value(0, true, &s, eval) + self.root_value(1, true, &s, eval))
    }

    /// Player nodes with their street line, acting player and actions, so callers
    /// can map strategies to and from other solvers.
    pub fn player_nodes(&self) -> Vec<(usize, Vec<SAction>, usize, Vec<SAction>)> {
        let mut out = Vec::new();
        let mut line = Vec::new();
        self.collect(0, &mut line, &mut out);
        out
    }

    fn collect(&self, node: usize, line: &mut Vec<SAction>, out: &mut Vec<(usize, Vec<SAction>, usize, Vec<SAction>)>) {
        if let Kind::Player { player, children } = &self.nodes[node].kind {
            out.push((node, line.clone(), *player, self.nodes[node].actions.clone()));
            for (a, &c) in children.iter().enumerate() {
                line.push(self.nodes[node].actions[a]);
                self.collect(c, line, out);
                line.pop();
            }
        }
    }

    /// Average strategy of every node (empty vectors at non-player nodes).
    pub fn strategies(&self) -> Vec<Vec<f32>> {
        (0..self.nodes.len())
            .map(|i| match self.nodes[i].kind {
                Kind::Player { .. } => self.average_strategy(i),
                _ => Vec::new(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postflop_solver::{BetSizeOptions, BoardState, TreeConfig};
    use rand::prelude::*;
    use rand::rngs::StdRng;

    fn random_board(rng: &mut StdRng, len: usize) -> Vec<u8> {
        let mut deck: Vec<u8> = (0..52).collect();
        deck.shuffle(rng);
        deck[..len].to_vec()
    }

    fn compatible(a: (u8, u8), b: (u8, u8)) -> bool {
        a.0 != b.0 && a.0 != b.1 && a.1 != b.0 && a.1 != b.1
    }

    #[test]
    fn hand_counts() {
        let mut rng = StdRng::seed_from_u64(1);
        for (len, n) in [(3, 1176), (4, 1128), (5, 1081)] {
            let board = random_board(&mut rng, len);
            let hands = Hands::new(&board);
            assert_eq!(hands.len(), n);
            for (i, &(a, b)) in hands.cards.iter().enumerate() {
                assert_eq!(hands.index(a, b), Some(i));
                assert_eq!(hands.index(b, a), Some(i));
            }
        }
    }

    #[test]
    fn sweeps_match_brute_force() {
        let mut rng = StdRng::seed_from_u64(2);
        let board = random_board(&mut rng, 5);
        let hands = Hands::new(&board);
        let n = hands.len();
        let reach: Vec<f32> = (0..n).map(|_| rng.gen()).collect();
        let keys = strength_keys(&board, &hands);
        let mut fast = vec![0.0f32; n];
        Showdown::new(&board, &hands).cfv(&hands, &reach, 1.0, &mut fast);
        let mut folds = vec![0.0f32; n];
        fold_cfv(&hands, &reach, 1.0, &mut folds);
        for i in 0..n {
            let (mut sd, mut fd) = (0.0f64, 0.0f64);
            for j in 0..n {
                if !compatible(hands.cards[i], hands.cards[j]) {
                    continue;
                }
                let (ki, kj) = (keys[i].unwrap(), keys[j].unwrap());
                sd += reach[j] as f64 * ((ki > kj) as i32 - (ki < kj) as i32) as f64;
                fd += reach[j] as f64;
            }
            assert!((fast[i] as f64 - sd).abs() < 1e-2, "showdown {} vs {}", fast[i], sd);
            assert!((folds[i] as f64 - fd).abs() < 1e-2, "fold {} vs {}", folds[i], fd);
        }
        // the one-runout equity matrix agrees with the sweep
        let leaf = LeafInfo { id: 0, pot: 2, stack: 0, allin: true, line: vec![] };
        let mut eq = vec![0.0f32; n];
        RunoutEquity::new(&board, &hands).evaluate(&leaf, 0, [&reach, &reach], &mut eq);
        for i in 0..n {
            assert!((eq[i] - fast[i]).abs() < 1e-2);
        }
    }

    #[test]
    fn river_converges() {
        let mut rng = StdRng::seed_from_u64(3);
        let board = random_board(&mut rng, 5);
        let hands = Hands::new(&board);
        let weights: [Vec<f32>; 2] = std::array::from_fn(|_| (0..hands.len()).map(|_| rng.gen()).collect());
        let bets = BetSizeOptions::try_from(("50%, a", "2.5x")).unwrap();
        let none = BetSizeOptions::try_from(("", "")).unwrap();
        let config = TreeConfig {
            initial_state: BoardState::River,
            starting_pot: 1000,
            effective_stack: 3000,
            rake_rate: 0.0,
            rake_cap: 0.0,
            flop_bet_sizes: [none.clone(), none.clone()],
            turn_bet_sizes: [none.clone(), none],
            river_bet_sizes: [bets.clone(), bets],
            turn_donk_sizes: None,
            river_donk_sizes: None,
            add_allin_threshold: 0.0,
            force_allin_threshold: 0.0,
            merging_threshold: 0.0,
        };
        let mut tree = ActionTree::new(config).unwrap();
        let mut game = StreetGame::new(&board, weights, &mut tree).unwrap();
        let eval = RunoutEquity::new(&board, &hands);
        game.solve(300, &eval);
        let expl = game.exploitability(&eval);
        assert!(expl < 0.005 * 1000.0, "exploitability {} chips", expl);
        let ev = game.expected_values(&eval);
        assert!((ev[0] + ev[1]).abs() < 1e-3);
    }
}
