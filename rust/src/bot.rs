// Real-time bot logic. Right now this just does action translation, but this
// is where I will add depth-limited solving.
use crate::card_utils::Card;
use crate::trainer_utils::{Action, ActionHistory};

pub fn bot_action(hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
    unimplemented!();
}
