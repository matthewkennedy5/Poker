# Poker (brrrbot)

AI for heads-up no-limit Texas Hold'em. Play it at [brrrbot.com](https://brrrbot.com).

- **Preflop:** a CFR blueprint trained on a card/action abstraction (`rust/src/trainer.rs`).
- **Flop:** a precomputed flop book: every canonical flop solved with exact hands for the common
  preflop lines (`rust/src/flopbook.rs`, `rust/src/bin/flop_book.rs`).
- **Turn and river:** real-time re-solving with exact hands using
  [postflop-solver](https://github.com/b-inary/postflop-solver) (`rust/src/search.rs`).
- **Evaluation:** matches against [Slumbot](https://www.slumbot.com) (`play_against_slumbot.py`)
  with AIVAT-style variance reduction (`rust/src/bin/aivat.rs`).

## License

AGPL-3.0 (see `LICENSE`), because the bot links postflop-solver, which is AGPL-3.0. If you
run a modified version as a network service, you must offer its source to its users.
