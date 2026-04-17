# Scrabble Solver Monorepo

This repository contains:

- `scrabble/`: core Rust game engine crate.
- `wasm/`: `wasm-bindgen` wrapper exposing game APIs for web clients.
- `web/`: React + Vite single-page app using the WASM backend.
- `console/`: host-side scratch runner.

## Build And Run (Together)

Prerequisites:

- Rust toolchain
- Node.js + npm
- `wasm-pack` (recommended in this repo: `cargo install wasm-pack`)

Install `wasm-pack` with Cargo:

```powershell
cargo install wasm-pack
```

Install JS dependencies once:

```powershell
npm install
```

Run web dev (builds WASM first):

```powershell
npm run dev
```

Build production assets (WASM + web):

```powershell
npm run build
```

Build only WASM package output for the SPA (`web/public/pkg`):

```powershell
npm run build:wasm
```

## Rust Tests

```powershell
cargo test -p scrabble
cargo test -p scrabble --test game_seeded_e2e
```

## Save/Resume Behavior

The SPA keeps one save slot in browser local storage and updates it after every turn.



