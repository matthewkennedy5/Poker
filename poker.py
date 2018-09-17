import game_state

num_opponents = 4   # int(input('Number of opponents: '))
g = game_state.GameState(num_opponents)
while not g.game_is_over():
    g.advance()