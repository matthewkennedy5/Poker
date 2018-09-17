import game_state

print(game_state.GameStages.FLOP)

game = game_state.GameState(2)
game.game_is_over()
game.play_turn()
game.play_river()
game.play_flop()
game.preflop()
game.advance()
game.init_player_bets()
game.bet()
game.fold()