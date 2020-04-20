// Real-time bot logic. Right now this just does action translation, but this
// is where I will add depth-limited solving.
use crate::card_utils::Card;
use crate::trainer_utils::*;
use std::collections::HashMap;

lazy_static! {
    static ref NODES: HashMap<InfoSet, Node> = crate::trainer::load_strategy();
}

pub fn bot_action(hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {
    let translated = history.translate(&BET_ABSTRACTION.to_vec());
    let hand = [hand, board].concat();
    let infoset = InfoSet::from_hand(&hand, &translated);
    let node = NODES.get(&infoset).unwrap();
    let action = sample_action_from_node(&node);
    action
}
