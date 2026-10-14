// Integration tests for the postflop-solver search layer. These do not need any
// product files.
use optimus::*;
use postflop_solver::{Action as SolverAction, CardConfig, PostFlopGame, NOT_DEALT};
use std::time::Duration;

fn river_game_with_line(line: &[SolverAction]) -> PostFlopGame {
    let cfg = SearchConfig::default();
    let tree = build_tree(RIVER, 1000, 5000, &cfg, line).unwrap();
    let board = str2cards("Td9d6hQc7s");
    let ids: Vec<u8> = board.iter().map(to_solver_card).collect();
    let card_config = CardConfig {
        range: [
            "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s"
                .parse()
                .unwrap(),
            "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+"
                .parse()
                .unwrap(),
        ],
        flop: [ids[0], ids[1], ids[2]],
        turn: ids[3],
        river: ids[4],
    };
    PostFlopGame::with_config(card_config, tree).unwrap()
}

#[test]
fn off_tree_line_is_added_and_solvable() {
    // OOP checks, IP bets an off-tree 333, OOP to act.
    let line = vec![SolverAction::Check, SolverAction::Bet(333)];
    let mut game = river_game_with_line(&line);
    let stats = solve_with_budget(&mut game, Duration::from_secs(10), 0.0, 100, true);
    assert_eq!(stats.iters, 100);
    assert!(stats.exploitability.is_finite());

    let indices = line_indices(&mut game, &line).unwrap();
    assert_eq!(indices.len(), 2);
    game.apply_history(&indices);
    assert!(!game.is_terminal_node() && !game.is_chance_node());
    assert_eq!(game.current_player(), 0);
    let actions = game.available_actions();
    assert!(actions.contains(&SolverAction::Fold));
    assert!(actions.contains(&SolverAction::Call));

    let n = game.private_cards(0).len();
    let strat = game.strategy();
    assert_eq!(strat.len(), n * actions.len());
    for h in 0..n {
        let sum: f32 = (0..actions.len()).map(|a| strat[a * n + h]).sum();
        assert!((sum - 1.0).abs() < 1e-3, "hand {} strategy sums to {}", h, sum);
    }
    game.back_to_root();

    // A line that is not in the tree is reported as such.
    assert!(line_indices(&mut game, &[SolverAction::Check, SolverAction::Bet(777)]).is_err());
}

#[test]
fn purify_removes_small_probabilities() {
    let mut strategy = Strategy::new();
    strategy.insert(FOLD, 0.01);
    strategy.insert(Action { action: ActionType::Call, amount: 300 }, 0.69);
    strategy.insert(Action { action: ActionType::Bet, amount: 900 }, 0.30);
    purify(&mut strategy, 0.03);
    assert_eq!(strategy.len(), 2);
    let total: f64 = strategy.values().sum();
    assert!((total - 1.0).abs() < 1e-9);
    assert!(!strategy.contains_key(&FOLD));
}

#[test]
fn project_line_round_trip_on_river() {
    // Real history reaching the river with a bet facing us; check that the solver
    // line and the project legalization agree with the pot/stack bookkeeping.
    let history = ActionHistory::from_strings(vec![
        "Bet 250", "Call 250", // preflop
        "Call 0", "Bet 250", "Call 250", // flop
        "Call 0", "Call 0", // turn
        "Bet 500", // river: BB bets 500
    ]);
    assert_eq!(history.street, RIVER);
    assert_eq!(history.player, DEALER);
    let slices = split_by_street(&history);
    let river = &slices[RIVER];
    assert_eq!(river.pot_start, 1000);
    assert_eq!(river.stacks_start, [19500, 19500]);
    let line = to_solver_line(river, 19500).unwrap();
    assert_eq!(line.actions, vec![SolverAction::Bet(500)]);
    let raise = to_project_action(&SolverAction::Raise(1500), &history, &line);
    assert_eq!(raise, Action { action: ActionType::Bet, amount: 1500 });
    assert!(history.is_legal_next_action(&raise));
    let call = to_project_action(&SolverAction::Call, &history, &line);
    assert!(history.is_legal_next_action(&call));
}
