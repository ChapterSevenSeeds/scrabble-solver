declare module "/pkg/wasm.js" {
  export default function init(input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module): Promise<void>;

  export class WasmGame {
    constructor(totalPlayers: number, aiPlayerIndexesJson: string);
    static fromJson(json: string): WasmGame;
    saveJson(): string;
    getStateJson(): string;
    getLegalMovesJson(): string;
    playMoveByIndex(index: number): void;
    playMoveByPlacementsJson(placementsJson: string): void;
    passTurn(): void;
    exchangeCurrentRack(): void;
    exchangeTiles(tiles: string): void;
    stepAiTurn(): boolean;
    autoPlayUntilHumanOrEnd(): void;
  }
}
