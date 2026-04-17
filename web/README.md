# Web SPA

React + Vite frontend for the Scrabble WASM backend.

Prerequisite: install `wasm-pack` so the WASM bundle can be generated.

```powershell
cargo install wasm-pack
```

## Development

From repository root:

```powershell
npm run dev
```

## Build

From repository root:

```powershell
npm run build
```

`npm run build` first runs `wasm-pack` and outputs `wasm` artifacts to `web/public/pkg`.



