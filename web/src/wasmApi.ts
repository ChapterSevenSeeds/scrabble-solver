export type GameState = {
  board_rows: string[];
  current_turn: number;
  total_players: number;
  scores: number[];
  tiles_in_bag: number;
  winner: number | null;
  racks: string[];
  current_rack: string;
  is_ai_turn: boolean;
  no_moves_hint: boolean;
  can_exchange: boolean;
};

export type WasmGameHandle = {
  getStateJson: () => string;
  getLegalMovesJson: () => string;
  playMoveByIndex: (index: number) => void;
  playMoveByPlacementsJson: (placementsJson: string) => void;
  passTurn: () => void;
  exchangeCurrentRack: () => void;
  exchangeTiles: (tiles: string) => void;
  saveJson: () => string;
  stepAiTurn: () => boolean;
  autoPlayUntilHumanOrEnd: () => void;
};

type WasmModule = {
  default: (options?: unknown) => Promise<unknown>;
  WasmGame: {
    new (totalPlayers: number, aiPlayerIndexesJson: string): WasmGameHandle;
    fromJson: (json: string) => WasmGameHandle;
  };
};

let wasmModulePromise: Promise<WasmModule> | null = null;
let wasmCacheToken = Date.now();
const dynamicImport = new Function(
  "path",
  "return import(path);"
) as (path: string) => Promise<WasmModule>;

async function loadModule(forceReload = false): Promise<WasmModule> {
  if (forceReload) {
    wasmModulePromise = null;
    wasmCacheToken = Date.now();
  }

  if (!wasmModulePromise) {
    wasmModulePromise = dynamicImport(`/pkg/wasm.js?v=${wasmCacheToken}`);
  }

  const module = await wasmModulePromise;
  await module.default({ module_or_path: `/pkg/wasm_bg.wasm?v=${wasmCacheToken}` });
  return module;
}

function shouldRetryWasmLoad(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error ?? "");
  return (
    message.includes("memory access out of bounds") ||
    message.includes("recursive use of an object detected")
  );
}

export async function createGame(
  totalPlayers: number,
  aiIndexes: number[]
): Promise<WasmGameHandle> {
  try {
    const module = await loadModule();
    return new module.WasmGame(totalPlayers, JSON.stringify(aiIndexes));
  } catch (error) {
    if (!shouldRetryWasmLoad(error)) {
      throw error;
    }

    const fresh = await loadModule(true);
    return new fresh.WasmGame(totalPlayers, JSON.stringify(aiIndexes));
  }
}

export async function loadGameFromJson(json: string): Promise<WasmGameHandle> {
  try {
    const module = await loadModule();
    return module.WasmGame.fromJson(json);
  } catch (error) {
    if (!shouldRetryWasmLoad(error)) {
      throw error;
    }

    const fresh = await loadModule(true);
    return fresh.WasmGame.fromJson(json);
  }
}

export function readState(game: WasmGameHandle): GameState {
  return JSON.parse(game.getStateJson()) as GameState;
}



