// Real-time bot logic. Right now this just does action translation, but this
// is where I will add depth-limited solving.
use crate::card_utils::Card;
use crate::trainer_utils::{Action, ActionHistory, sample_action_from_strategy};

pub fn bot_action(hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
    // TODO: Implement. This is a stub that always calls.
    let strategy = crate::exploiter::always_call(history);
    let action = sample_action_from_strategy(&strategy);
    action
}
