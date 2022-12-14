// Real-time bot logic. Right now this just does action translation, but this
// is where I will add depth-limited solving.
use std::collections::HashMap;
use crate::card_utils::Card;
use crate::trainer_utils::*;
use crate::config::CONFIG;

lazy_static! {
    static ref BLUEPRINT: HashMap<CompactInfoSet, Action> = crate::trainer::load_blueprint();
}

pub fn bot_action(hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {

    let translated = history.translate(&CONFIG.bet_abstraction);
    let hand = [hand, board].concat();
    let infoset = InfoSet::from_hand(&hand, &translated).compress();

    let mut action: Action = {
        match BLUEPRINT.get(&infoset) {
            Some(action) => action.clone(),
            None => {
                println!("Infoset not in strategy");
                let node = Node::new(&infoset.uncompress());
                let action = sample_action_from_node(&node);
                action
            }
        }
    };

    // The translated action is based off a misunderstanding off the true bet
    // sizes, so we may have to adjust our call amount to line up with what's
    // actually in the pot as opposed to our approximation.
    if action.action == ActionType::Call {
        // TODO: Are there other spots where the altered history brings illegal moves?
        // Hopefully not with a large enough bet abstraction, but still.
        action.amount = history.to_call();
    } else if action.action == ActionType::Bet && action.amount < history.min_bet() {
        action.amount = history.min_bet();
    } else if action.action == ActionType::Bet && action.amount > history.max_bet() {
        action.amount = history.max_bet();
    }
    action
}
