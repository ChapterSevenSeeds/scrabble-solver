import { spawnSync } from "node:child_process";

const args = [
  "build",
  "wasm",
  "--target",
  "web",
  "--out-dir",
  "../web/public/pkg",
  "--release",
];

const result = spawnSync("wasm-pack", args, {
  stdio: "inherit",
  shell: process.platform === "win32",
});

if (result.error && result.error.code === "ENOENT") {
  console.error("\nMissing dependency: wasm-pack");
  console.error("Install it with: cargo install wasm-pack");
  console.error("Docs: https://rustwasm.github.io/wasm-pack/installer/");
  console.error("Then re-run: npm run build:wasm\n");
  process.exit(1);
}

if (result.status !== 0) {
  console.error("\nWASM build failed.");
  console.error("If wasm-pack is missing, install it with: cargo install wasm-pack");
  process.exit(result.status ?? 1);
}

process.exit(0);



