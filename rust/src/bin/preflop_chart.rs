use optimus::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

fn main() {
    let bot = Bot::new(
        load_nodes(&CONFIG.nodes_path),
        CONFIG.subgame_solving
    );
    write_preflop_strategy(&bot, &CONFIG.preflop_strategy_path);
}

// For making preflop charts
pub fn write_preflop_strategy(bot: &Bot, path: &str) {
    let mut preflop_strategy: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let starting_history = ActionHistory::new();
    for hand in isomorphic_preflop_hands() {
        let strategy = bot.get_strategy(&hand, &Vec::new(), &starting_history);
        let str_strategy: HashMap<String, f64> = strategy
            .iter()
            .map(|(action, prob)| (action.to_string(), prob.clone()))
            .collect();
        let hand_str = format!(
            "{}{}{}",
            rank_str(hand[0].rank),
            rank_str(hand[1].rank),
            if hand[0].suit == hand[1].suit { "s" } else { "o" }
        );
        preflop_strategy.insert(hand_str, str_strategy);
    }

    // Write the preflop strategy to a JSON
    let json = serde_json::to_string_pretty(&preflop_strategy).unwrap();
    let mut file = File::create(&path).unwrap();
    file.write(json.as_bytes()).unwrap();
}
