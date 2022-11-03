// Real-time bot logic. Right now this just does action translation, but this
// is where I will add depth-limited solving.
use std::collections::HashMap;
use crate::card_utils::Card;
use crate::trainer_utils::*;

lazy_static! {
    static ref BLUEPRINT: HashMap<CompactInfoSet, Action> = crate::trainer::load_blueprint();
}

pub fn bot_action(hand: &[Card], board: &[Card], history: &ActionHistory) -> Action {

    let translated = history.translate(&BET_ABSTRACTION.to_vec());
    let hand = [hand, board].concat();
    let infoset = InfoSet::from_hand(&hand, &translated).compress();

    let mut action: Action = {
        // Action translation
        match BLUEPRINT.get(&infoset) {
            Some(action) => action.clone(),
            None => {
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
    }
    action
}

// After an opponent makes an off-tree action, we need to solve a new subgame
// starting from that action.
// Solves a subgame starting with their move, but including their action in the
// abstraction now. Then we can use the probability assigned to that action for
// each possible hands to update our belief distribution over their hands.
fn solve_subgame(hand: &[Card], history: &ActionHistory, opp_range: &HashMap<Vec<Card>, f64>) {
    // let mut nodes: HashMap<InfoSet, Node> = HashMap::new();
    // for i in 0..1000 {
    //     sample opponent hand from range
    //     iterate(DEALER, deck, history, [1.0, 1.0], &mut nodes);
    //     iterate(OPPONENT, deck, history, [1.0, 1.0], &mut nodes);
    // }
    // let strategy = nodes.get(this infoset);
    // strategy
}
