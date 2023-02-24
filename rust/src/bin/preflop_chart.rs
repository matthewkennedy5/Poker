use optimus::*;

fn main() {
    // TODO: Use bot instead of nodes here
    let nodes = load_nodes(&CONFIG.nodes_path);
    write_preflop_strategy(&nodes, &CONFIG.preflop_strategy_path);
}
