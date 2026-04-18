# Scrabble Crate

Core Scrabble engine crate used by both host console and WASM entrypoint crates.

## Crate Outline

- `src/game.rs`: game loop, turn handling, move generation, scoring.
- `src/tile_bag.rs`: tile distribution, replenishment, exchanges.
- `src/board.rs` + `src/board_iterator.rs`: board storage and iteration helpers.
- `src/common.rs`: shared score tables, modifiers, move/tile types.
- `src/utils.rs`: helper utilities for bitmasks and tile matching.
- `src/words.txt`: dictionary used for move validation.

## Basic Testing

Run crate tests:

```powershell
cargo test -p scrabble
```

