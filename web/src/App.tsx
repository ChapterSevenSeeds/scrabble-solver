import { useEffect, useMemo, useState } from "react";
import {
  createGame,
  loadGameFromJson,
  readState,
  type GameState,
  type WasmGameHandle,
} from "./wasmApi";

const SAVE_SLOT_KEY = "scrabble_save_slot_v1";

type DragPlacement = {
  rackIndex: number;
  tile: string;
  row: number;
  col: number;
};

const BONUS_LABELS: string[][] = [
  ["TWS", "", "", "DLS", "", "", "", "TWS", "", "", "", "DLS", "", "", "TWS"],
  ["", "DWS", "", "", "", "TLS", "", "", "", "TLS", "", "", "", "DWS", ""],
  ["", "", "DWS", "", "", "", "DLS", "", "DLS", "", "", "", "DWS", "", ""],
  ["DLS", "", "", "DWS", "", "", "", "DLS", "", "", "", "DWS", "", "", "DLS"],
  ["", "", "", "", "DWS", "", "", "", "", "", "DWS", "", "", "", ""],
  ["", "TLS", "", "", "", "TLS", "", "", "", "TLS", "", "", "", "TLS", ""],
  ["", "", "DLS", "", "", "", "DLS", "", "DLS", "", "", "", "DLS", "", ""],
  ["TWS", "", "", "DLS", "", "", "", "", "", "", "", "DLS", "", "", "TWS"],
  ["", "", "DLS", "", "", "", "DLS", "", "DLS", "", "", "", "DLS", "", ""],
  ["", "TLS", "", "", "", "TLS", "", "", "", "TLS", "", "", "", "TLS", ""],
  ["", "", "", "", "DWS", "", "", "", "", "", "DWS", "", "", "", ""],
  ["DLS", "", "", "DWS", "", "", "", "DLS", "", "", "", "DWS", "", "", "DLS"],
  ["", "", "DWS", "", "", "", "DLS", "", "DLS", "", "", "", "DWS", "", ""],
  ["", "DWS", "", "", "", "TLS", "", "", "", "TLS", "", "", "", "DWS", ""],
  ["TWS", "", "", "DLS", "", "", "", "TWS", "", "", "", "DLS", "", "", "TWS"],
];

export function App() {
  const [game, setGame] = useState<WasmGameHandle | null>(null);
  const [state, setState] = useState<GameState | null>(null);
  const [error, setError] = useState<string>("");
  const [loading, setLoading] = useState<boolean>(false);
  const [totalPlayers, setTotalPlayers] = useState<number>(3);
  const [aiSeats, setAiSeats] = useState<boolean[]>([false, false, true]);
  const [draftPlacements, setDraftPlacements] = useState<DragPlacement[]>([]);
  const [selectedRackIndex, setSelectedRackIndex] = useState<number | null>(null);
  const [draggingRackIndex, setDraggingRackIndex] = useState<number | null>(null);
  const [exchangeInput, setExchangeInput] = useState<string>("");

  const setupSeats = useMemo(
    () => Array.from({ length: totalPlayers }, (_, i) => aiSeats[i] ?? i > 0),
    [aiSeats, totalPlayers]
  );

  const syncState = async (activeGame: WasmGameHandle) => {
    let nextState = readState(activeGame);

    if (nextState.winner === null && nextState.is_ai_turn) {
      activeGame.autoPlayUntilHumanOrEnd();
      nextState = readState(activeGame);
    }

    setState(nextState);
    localStorage.setItem(SAVE_SLOT_KEY, activeGame.saveJson());
  };

  useEffect(() => {
    const boot = async () => {
      const saved = localStorage.getItem(SAVE_SLOT_KEY);
      if (!saved) {
        return;
      }
      try {
        setLoading(true);
        const restored = await loadGameFromJson(saved);
        setGame(restored);
        await syncState(restored);
      } catch (e) {
        console.error(e);
        setError("Could not restore saved game. You can start a new one.");
      } finally {
        setLoading(false);
      }
    };

    void boot();
  }, []);

  const startGame = async () => {
    try {
      setLoading(true);
      setError("");
      const aiIndexes = setupSeats
        .map((isAi, idx) => ({ isAi, idx }))
        .filter((x) => x.isAi)
        .map((x) => x.idx);
      const created = await createGame(totalPlayers, aiIndexes);
      setGame(created);
      setDraftPlacements([]);
      setSelectedRackIndex(null);
      setDraggingRackIndex(null);
      setExchangeInput("");
      await syncState(created);
    } catch (e) {
      console.error(e);
      setError("Failed to start game");
    } finally {
      setLoading(false);
    }
  };

  const submitDraftMove = async () => {
    if (!game) return;
    if (draftPlacements.length === 0) {
      setError("Place at least one tile before submitting a move");
      return;
    }

    try {
      game.playMoveByPlacementsJson(
        JSON.stringify(
          draftPlacements.map((p) => ({ row: p.row, col: p.col, tile: p.tile }))
        )
      );
      setDraftPlacements([]);
      setSelectedRackIndex(null);
      setDraggingRackIndex(null);
      setExchangeInput("");
      setError("");
      await syncState(game);
    } catch (e) {
      console.error(e);
      setError("Placed tiles are not a legal move");
    }
  };

  const passTurn = async () => {
    if (!game) return;
    game.passTurn();
    await syncState(game);
  };

  const exchange = async () => {
    if (!game) return;
    try {
      const tiles = exchangeInput.trim().toUpperCase();
      game.exchangeTiles(tiles);
      setExchangeInput("");
      setDraftPlacements([]);
      setSelectedRackIndex(null);
      setDraggingRackIndex(null);
      setError("");
      await syncState(game);
    } catch (e) {
      console.error(e);
      setError("Exchange failed. Enter only tiles from your rack (e.g. AEI)");
    }
  };

  const newGameReset = () => {
    localStorage.removeItem(SAVE_SLOT_KEY);
    setGame(null);
    setState(null);
    setDraftPlacements([]);
    setSelectedRackIndex(null);
    setDraggingRackIndex(null);
    setExchangeInput("");
    setError("");
  };

  const updateSeat = (index: number, value: boolean) => {
    const next = [...setupSeats];
    next[index] = value;
    setAiSeats(next);
  };

  const onRackPointerDown = (rackIndex: number) => {
    setDraggingRackIndex(rackIndex);
    setSelectedRackIndex(rackIndex);
  };

  const placeSelectedTile = (row: number, col: number) => {
    if (!state) return;
    if (state.board_rows[row][col] !== " ") {
      return;
    }

    // If the user taps an already drafted square, remove that draft placement.
    const existingIndex = draftPlacements.findIndex((p) => p.row === row && p.col === col);
    if (existingIndex >= 0) {
      setDraftPlacements((curr) => curr.filter((_, idx) => idx !== existingIndex));
      return;
    }

    const rackIndex = draggingRackIndex ?? selectedRackIndex;
    if (rackIndex === null) {
      return;
    }

    if (draftPlacements.some((p) => p.rackIndex === rackIndex)) {
      // This rack tile has already been used in the draft.
      return;
    }

    const tile = state.current_rack[rackIndex];
    if (!tile) {
      return;
    }

    setDraftPlacements((curr) => [...curr, { rackIndex, tile, row, col }]);
    setDraggingRackIndex(null);
    setSelectedRackIndex(null);
    setError("");
  };

  const removeDraftByRackIndex = (rackIndex: number) => {
    setDraftPlacements((curr) => curr.filter((p) => p.rackIndex !== rackIndex));
  };

  const onBoardPointerUp = (row: number, col: number) => {
    placeSelectedTile(row, col);
  };

  const getBonusLabel = (row: number, col: number) => BONUS_LABELS[row][col];

  const draftByCell = new Map(
    draftPlacements.map((placement) => [`${placement.row}-${placement.col}`, placement])
  );

  if (!game || !state) {
    return (
      <div className="container">
        <h1>Scrabble WASM</h1>
        <p>Set up players (Human/AI). AI turns run immediately.</p>
        <label>
          Total players
          <input
            type="number"
            min={1}
            max={4}
            value={totalPlayers}
            onChange={(e) => setTotalPlayers(Number(e.target.value))}
          />
        </label>

        <div className="seat-list">
          {Array.from({ length: totalPlayers }, (_, i) => (
            <label key={i} className="seat-row">
              Player {i + 1}
              <select
                value={setupSeats[i] ? "ai" : "human"}
                onChange={(e) => updateSeat(i, e.target.value === "ai")}
              >
                <option value="human">Human</option>
                <option value="ai">AI</option>
              </select>
            </label>
          ))}
        </div>

        <button onClick={startGame} disabled={loading}>
          {loading ? "Starting..." : "Start game"}
        </button>
        {!!error && <p className="error">{error}</p>}
      </div>
    );
  }

  return (
    <div className="container">
      <header className="toolbar">
        <h1>Scrabble WASM</h1>
        <div className="toolbar-actions">
          <button onClick={newGameReset}>New game</button>
        </div>
      </header>

      <div className="status-grid">
        <div>Turn: Player {state.current_turn + 1}</div>
        <div>Winner: {state.winner === null ? "-" : `Player ${state.winner + 1}`}</div>
      </div>

      <div className="scores">
        {state.scores.map((score, idx) => (
          <div key={idx} className="score-pill">
            P{idx + 1}: {score}
          </div>
        ))}
      </div>

      {state.no_moves_hint && (
        <p className="hint">No legal moves for this rack. You must pass or exchange.</p>
      )}

      <div className="board">
        {state.board_rows.map((row, rIdx) =>
          row.split("").map((cell, cIdx) => (
            <button
              key={`${rIdx}-${cIdx}`}
              type="button"
              className={`cell bonus-${getBonusLabel(rIdx, cIdx).toLowerCase() || "none"}`}
              onPointerUp={() => onBoardPointerUp(rIdx, cIdx)}
              data-occupied={cell !== " "}
            >
              {cell !== " "
                ? cell
                : draftByCell.get(`${rIdx}-${cIdx}`)?.tile ??
                  (getBonusLabel(rIdx, cIdx) ? (
                    <span className="bonus-label">{getBonusLabel(rIdx, cIdx)}</span>
                  ) : (
                    ""
                  ))}
            </button>
          ))
        )}
      </div>

      <section>
        <h2>Current Rack</h2>
        <div className="rack">
          {state.current_rack.split("").map((tile, idx) => (
            <button
              key={`${tile}-${idx}`}
              className={`tile ${selectedRackIndex === idx ? "tile-selected" : ""}`}
              onPointerDown={() => onRackPointerDown(idx)}
              onClick={() => setSelectedRackIndex(idx)}
            >
              {tile}
            </button>
          ))}
        </div>
        <p className="subtle">Click/tap a rack tile then a square, or drag tile to square.</p>
      </section>

      <section>
        <h2>Draft placements</h2>
        <div className="drafts">
          {draftPlacements.map((p, idx) => (
            <button key={idx} className="draft-chip" onClick={() => removeDraftByRackIndex(p.rackIndex)}>
              {p.tile}@({p.row},{p.col}) x
            </button>
          ))}
        </div>
      </section>

      {!state.is_ai_turn && state.winner === null && (
        <section>
          <h2>Turn Actions</h2>

          <div className="actions">
            <button onClick={submitDraftMove} disabled={draftPlacements.length === 0}>
              Play Placed Tiles
            </button>
            <button onClick={passTurn}>Pass</button>
          </div>

          <div className="actions">
            <input
              type="text"
              value={exchangeInput}
              maxLength={7}
              placeholder="Tiles to exchange (e.g. AEI)"
              onChange={(e) => setExchangeInput(e.target.value.replace(/[^A-Za-z*]/g, "").toUpperCase())}
            />
            <button onClick={exchange} disabled={!state.can_exchange || exchangeInput.trim().length === 0}>
              Exchange Selected Tiles
            </button>
          </div>
        </section>
      )}

      {!!error && <p className="error">{error}</p>}
    </div>
  );
}

