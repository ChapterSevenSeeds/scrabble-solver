# Scrabble Crate

Core Scrabble engine crate used by both host console and WASM entrypoint crates.

## Crate Outline

- `src/game.rs`: game loop, turn handling, move generation, scoring.
- `src/tile_bag.rs`: tile distribution, replenishment, exchanges.
- `src/board.rs` + `src/board_iterator.rs`: board storage and iteration helpers.
- `src/common.rs`: shared score tables, modifiers, move/tile types.
- `src/utils.rs`: helper utilities for bitmasks and tile matching.
- `src/words.txt`: dictionary used for move validation.
- `tests/game_seeded_e2e.rs`: seeded end-to-end integration tests against fixtures.
- `tests/fixtures/*.json`: per-turn deterministic checkpoints.

## Basic Testing

Run all seeded fixture-backed integration checks:

```powershell
cargo test -p scrabble --test game_seeded_e2e
```

Regenerate fixture files (ignored test):

```powershell
cargo test -p scrabble --test game_seeded_e2e regenerate_fixtures -- --ignored --nocapture
```

After regeneration, rerun validation:

```powershell
cargo test -p scrabble --test game_seeded_e2e
```

## Fixture Notes

- Fixtures are intentionally verbose and include per-turn board/rack state.
- Regenerate fixtures when behavior changes make manual patching impractical.
- See `tests/fixtures/README.md` for fixture-specific details.

