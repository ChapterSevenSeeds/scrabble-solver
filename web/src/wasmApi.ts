export type GameState = {
  board_rows: string[];
  current_turn: number;
  total_players: number;
  scores: number[];
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
  autoPlayUntilHumanOrEnd: () => void;
};

type WasmModule = {
  default: () => Promise<void>;
  WasmGame: {
    new (totalPlayers: number, aiPlayerIndexesJson: string): WasmGameHandle;
    fromJson: (json: string) => WasmGameHandle;
  };
};

let wasmModulePromise: Promise<WasmModule> | null = null;
const dynamicImport = new Function(
  "path",
  "return import(path);"
) as (path: string) => Promise<WasmModule>;

async function loadModule(): Promise<WasmModule> {
  if (!wasmModulePromise) {
    wasmModulePromise = dynamicImport("/pkg/wasm.js");
  }
  const module = await wasmModulePromise;
  await module.default();
  return module;
}

export async function createGame(
  totalPlayers: number,
  aiIndexes: number[]
): Promise<WasmGameHandle> {
  const module = await loadModule();
  return new module.WasmGame(totalPlayers, JSON.stringify(aiIndexes));
}

export async function loadGameFromJson(json: string): Promise<WasmGameHandle> {
  const module = await loadModule();
  return module.WasmGame.fromJson(json);
}

export function readState(game: WasmGameHandle): GameState {
  return JSON.parse(game.getStateJson()) as GameState;
}



