//! Validates the single-street solver against postflop-solver on identical spots.
//!
//! river: full river tree, showdown terminals (exact comparison).
//! turn / flop: betting on the given street only; later streets are checked down,
//! so the exact street-end leaf value is runout equity (`RunoutEquity`).
//!
//! For each spot it reports postflop-solver's exploitability and EV, the same
//! numbers for postflop-solver's strategy imported into our tree (checks our
//! payoffs and best response), and our own solve.
//!
//! Usage: street-validate <river|turn|flop> [spots] [iterations] [seed]
use optimus::street_solver::*;
use postflop_solver::{
    compute_current_ev, compute_exploitability, finalize, solve_step, ActionTree, BetSizeOptions,
    BoardState, CardConfig, PostFlopGame, Range, TreeConfig, NOT_DEALT,
};
use rand::prelude::*;
use rand::rngs::StdRng;
use std::time::Instant;

fn sizes(b: &str, r: &str) -> BetSizeOptions {
    BetSizeOptions::try_from((b, r)).unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let street = args.first().map(|s| s.as_str()).unwrap_or("river").to_string();
    let spots: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(3);
    let iterations: u32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1000);
    let seed: u64 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(1);
    let mut rng = StdRng::seed_from_u64(seed);
    let board_len = match street.as_str() {
        "river" => 5,
        "turn" => 4,
        "flop" => 3,
        _ => panic!("street must be river, turn or flop"),
    };

    for spot in 0..spots {
        let mut deck: Vec<u8> = (0..52).collect();
        deck.shuffle(&mut rng);
        let board: Vec<u8> = deck[..board_len].to_vec();
        let hands = Hands::new(&board);
        // Random ranges: squared uniform weights, a third of the hands absent.
        let weights: [Vec<f32>; 2] = std::array::from_fn(|_| {
            (0..hands.len())
                .map(|_| if rng.gen::<f32>() < 0.33 { 0.0 } else { rng.gen::<f32>().powi(2) })
                .collect()
        });
        let pot = 1000;
        let eff = [1500, 4000, 9000][spot % 3];

        let bets = sizes("33%, 75%, a", "2.5x, a");
        let none = sizes("", "");
        let (flop_s, turn_s, river_s) = match board_len {
            5 => (none.clone(), none.clone(), bets.clone()),
            4 => (none.clone(), bets.clone(), none.clone()),
            _ => (bets.clone(), none.clone(), none.clone()),
        };
        let tree_config = TreeConfig {
            initial_state: match board_len {
                5 => BoardState::River,
                4 => BoardState::Turn,
                _ => BoardState::Flop,
            },
            starting_pot: pot,
            effective_stack: eff,
            rake_rate: 0.0,
            rake_cap: 0.0,
            flop_bet_sizes: [flop_s.clone(), flop_s],
            turn_bet_sizes: [turn_s.clone(), turn_s],
            river_bet_sizes: [river_s.clone(), river_s],
            turn_donk_sizes: None,
            river_donk_sizes: None,
            add_allin_threshold: 0.0,
            force_allin_threshold: 0.0,
            merging_threshold: 0.0,
        };

        // postflop-solver
        let ranges: Vec<Range> = (0..2)
            .map(|p| Range::from_hands_weights(&hands.cards, &weights[p]).unwrap())
            .collect();
        let card_config = CardConfig {
            range: [ranges[0], ranges[1]],
            flop: [board[0], board[1], board[2]],
            turn: if board_len >= 4 { board[3] } else { NOT_DEALT },
            river: if board_len >= 5 { board[4] } else { NOT_DEALT },
        };
        let t0 = Instant::now();
        let mut game = PostFlopGame::with_config(card_config, ActionTree::new(tree_config.clone()).unwrap()).unwrap();
        game.allocate_memory(false);
        for t in 0..iterations {
            solve_step(&game, t);
        }
        let ps_expl = compute_exploitability(&game) as f64;
        let ps_ev = compute_current_ev(&game);
        finalize(&mut game);
        let ps_time = t0.elapsed().as_secs_f64();

        // our tree and evaluator
        let mut tree = ActionTree::new(tree_config).unwrap();
        let mut ours = StreetGame::new(&board, weights.clone(), &mut tree).unwrap();
        let t1 = Instant::now();
        let eval = RunoutEquity::new(&board, &hands);
        let eval_time = t1.elapsed().as_secs_f64();

        // postflop-solver's strategy imported into our tree
        let mut imported: Vec<Vec<f32>> = (0..ours.num_nodes()).map(|_| Vec::new()).collect();
        for (node, line, player, actions) in ours.player_nodes() {
            game.back_to_root();
            for a in &line {
                let idx = game.available_actions().iter().position(|x| x == a).expect("line not in game");
                game.play(idx);
            }
            assert_eq!(game.available_actions(), actions, "action lists differ at {:?}", line);
            assert_eq!(game.current_player(), player);
            let strat = game.strategy();
            let cards = game.private_cards(player);
            let nk = cards.len();
            let n = hands.len();
            let na = actions.len();
            let mut s = vec![1.0 / na as f32; na * n];
            for (k, &(c1, c2)) in cards.iter().enumerate() {
                let i = hands.index(c1, c2).unwrap();
                for a in 0..na {
                    s[a * n + i] = strat[a * nk + k];
                }
            }
            imported[node] = s;
        }
        game.back_to_root();
        let imp_expl = ours.exploitability_of(imported, &eval);

        let t2 = Instant::now();
        ours.solve(iterations, &eval);
        let our_time = t2.elapsed().as_secs_f64();
        let our_expl = ours.exploitability(&eval);
        let our_ev = ours.expected_values(&eval);

        let pct = |x: f64| 100.0 * x / pot as f64;
        println!(
            "{} spot {} board {:?} eff {} | nodes {} leaves {} hands {}",
            street, spot, board, eff, ours.num_nodes(), ours.num_leaves, hands.len()
        );
        println!(
            "  postflop-solver: expl {:.4}% pot, EV oop {:+.2}  ({:.2} s)",
            pct(ps_expl), ps_ev[0], ps_time
        );
        println!("  its strategy in our tree: expl {:.4}% pot (should match)", pct(imp_expl));
        println!(
            "  ours: expl {:.4}% pot, EV oop {:+.2} ip {:+.2}  ({:.2} s solve, {:.2} s leaf setup)",
            pct(our_expl), our_ev[0], our_ev[1], our_time, eval_time
        );
    }
}
