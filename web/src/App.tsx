import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  Alert,
  Badge,
  Box,
  Button,
  Card,
  Group,
  Modal,
  NumberInput,
  Select,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { notifications } from "@mantine/notifications";
import {
  createGame,
  loadGameFromJson,
  readState,
  type GameState,
  type WasmGameHandle,
} from "./wasmApi";

const SAVE_SLOT_KEY = "scrabble_save_slot_v1";
const AI_TURN_DELAY_MS = 450;

type DraftPlacement = {
  rackIndex: number;
  rackTile: string;
  placedTile: string;
  row: number;
  col: number;
};

type LegalMoveView = {
  tiles: Array<{ row: number; col: number; tile: string }>;
};

type PendingWildcardPlacement = {
  rackIndex: number;
  row: number;
  col: number;
};

type DragGhost = {
  kind: "rack" | "pending";
  rackIndex: number;
};

type CellCoords = {
  row: number;
  col: number;
};

const cellId = (row: number, col: number) => `cell:${row}:${col}`;
const rackId = (rackIndex: number) => `rack:${rackIndex}`;
const pendingId = (rackIndex: number) => `pending:${rackIndex}`;
const rackDropId = "rack-drop";
const placementKey = (placements: Array<{ row: number; col: number; tile: string }>) =>
  placements
    .map((p) => `${p.row}:${p.col}:${p.tile}`)
    .sort()
    .join("|");

const parseCellId = (id: string): CellCoords | null => {
  const [prefix, row, col] = id.split(":");
  if (prefix !== "cell") {
    return null;
  }
  const parsedRow = Number(row);
  const parsedCol = Number(col);
  if (Number.isNaN(parsedRow) || Number.isNaN(parsedCol)) {
    return null;
  }
  return { row: parsedRow, col: parsedCol };
};

const parseDragId = (id: string): DragGhost | null => {
  const [prefix, index] = id.split(":");
  const rackIndex = Number(index);
  if (Number.isNaN(rackIndex)) {
    return null;
  }
  if (prefix === "rack") {
    return { kind: "rack", rackIndex };
  }
  if (prefix === "pending") {
    return { kind: "pending", rackIndex };
  }
  return null;
};

function DraggableTile(props: {
  id: string;
  label: string;
  className: string;
  disabled?: boolean;
  onClick?: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: props.id,
    disabled: props.disabled,
  });

  const style = {
    transform: CSS.Translate.toString(transform),
    opacity: isDragging ? 0.55 : 1,
  };

  return (
    <button
      ref={setNodeRef}
      type="button"
      className={props.className}
      style={style}
      disabled={props.disabled}
      onClick={props.onClick}
      {...listeners}
      {...attributes}
    >
      {props.label}
    </button>
  );
}

function DroppableCell(props: {
  id: string;
  className: string;
  occupied: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  const { isOver, setNodeRef } = useDroppable({ id: props.id });
  return (
    <button
      ref={setNodeRef}
      type="button"
      className={`${props.className} ${isOver ? "cell-drop-target" : ""}`.trim()}
      data-occupied={props.occupied}
      onClick={props.onClick}
    >
      {props.children}
    </button>
  );
}

function DroppableRack(props: { id: string; children: ReactNode }) {
  const { isOver, setNodeRef } = useDroppable({ id: props.id });
  return (
    <div ref={setNodeRef} className={`rack-drop-zone ${isOver ? "rack-drop-target" : ""}`.trim()}>
      {props.children}
    </div>
  );
}

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
  const stateRef = useRef<GameState | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const [totalPlayers, setTotalPlayers] = useState(2);
  const [aiSeats, setAiSeats] = useState<boolean[]>([false, true]);
  const [allAiMode, setAllAiMode] = useState(false);

  const [draftPlacements, setDraftPlacements] = useState<DraftPlacement[]>([]);
  const [activeDrag, setActiveDrag] = useState<DragGhost | null>(null);
  const [lastMoveCells, setLastMoveCells] = useState<Set<string>>(new Set());

  const [exchangeInput, setExchangeInput] = useState("");
  const [pendingWildcardPlacement, setPendingWildcardPlacement] =
    useState<PendingWildcardPlacement | null>(null);
  const [wildcardLetter, setWildcardLetter] = useState("");

  const setupSeats = useMemo(
    () => Array.from({ length: totalPlayers }, (_, i) => aiSeats[i] ?? i > 0),
    [aiSeats, totalPlayers]
  );

  // Serialize all wasm handle method calls to avoid recursive aliasing at runtime.
  const gameOpChainRef = useRef<Promise<void>>(Promise.resolve());

  const queueGameOp = <T,>(op: () => Promise<T> | T): Promise<T> => {
    const next = gameOpChainRef.current.then(() => op());
    gameOpChainRef.current = next.then(
      () => undefined,
      () => undefined
    );
    return next;
  };

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  const wait = async (ms: number) =>
    new Promise<void>((resolve) => {
      window.setTimeout(resolve, ms);
    });

  const persistAndSetState = (activeGame: WasmGameHandle, nextState: GameState) => {
    const prevState = stateRef.current;
    if (prevState) {
      const changed = new Set<string>();
      for (let row = 0; row < 15; row += 1) {
        const prevRow = prevState.board_rows[row];
        const nextRow = nextState.board_rows[row];
        for (let col = 0; col < 15; col += 1) {
          if (prevRow[col] !== nextRow[col] && nextRow[col] !== " ") {
            changed.add(`${row}-${col}`);
          }
        }
      }
      setLastMoveCells(changed);
    }

    stateRef.current = nextState;
    setState(nextState);
    localStorage.setItem(SAVE_SLOT_KEY, activeGame.saveJson());
  };

  const syncState = async (activeGame: WasmGameHandle, delayAiTurns = allAiMode) => {
    let nextState = readState(activeGame);
    persistAndSetState(activeGame, nextState);

    while (nextState.winner === null && nextState.is_ai_turn) {
      if (delayAiTurns) {
        await wait(AI_TURN_DELAY_MS);
      }
      activeGame.stepAiTurn();
      // Give wasm-bindgen a tick between mutable and immutable calls on the same handle.
      await wait(0);
      nextState = readState(activeGame);
      persistAndSetState(activeGame, nextState);
    }
  };

  useEffect(() => {
    setAiSeats((curr) =>
      Array.from({ length: totalPlayers }, (_, i) => {
        if (curr[i] !== undefined) {
          return curr[i];
        }
        return i > 0;
      })
    );
  }, [totalPlayers]);

  useEffect(() => {
    const boot = async () => {
      const saved = localStorage.getItem(SAVE_SLOT_KEY);
      if (!saved) {
        return;
      }

      try {
        setLoading(true);
        let savedAllAiMode = false;
        try {
          const parsed = JSON.parse(saved) as { seats?: unknown };
          if (Array.isArray(parsed.seats)) {
            savedAllAiMode = parsed.seats.every((x) => x === "Ai");
            setAllAiMode(savedAllAiMode);
          }
        } catch {
          // Ignore save metadata parse failures and still attempt restoration.
        }

        const restored = await loadGameFromJson(saved);
        setGame(restored);
        await queueGameOp(() => syncState(restored, savedAllAiMode));
      } catch (e) {
        console.error(e);
        setError("Could not restore saved game. You can start a new one.");
      } finally {
        setLoading(false);
      }
    };

    void boot();
  }, []);

  const resetInteractionState = () => {
    setDraftPlacements([]);
    setActiveDrag(null);
    setLastMoveCells(new Set());
    setExchangeInput("");
    setPendingWildcardPlacement(null);
    setWildcardLetter("");
  };

  const startGame = async () => {
    try {
      setLoading(true);
      setError("");
      const aiIndexes = setupSeats
        .map((isAi, idx) => ({ isAi, idx }))
        .filter((x) => x.isAi)
        .map((x) => x.idx);

      const isAllAi = setupSeats.every((isAi) => isAi);
      setAllAiMode(isAllAi);

      const created = await createGame(totalPlayers, aiIndexes);
      setGame(created);
      resetInteractionState();
      await queueGameOp(() => syncState(created, isAllAi));
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
      notifications.show({
        color: "yellow",
        title: "No tiles placed",
        message: "Place at least one tile before submitting a move",
      });
      return;
    }

    try {
      await queueGameOp(async () => {
        const legalMoves = JSON.parse(game.getLegalMovesJson()) as LegalMoveView[];
        const draftKey = placementKey(
          draftPlacements.map((p) => ({ row: p.row, col: p.col, tile: p.placedTile }))
        );
        const moveIndex = legalMoves.findIndex((move) => placementKey(move.tiles) === draftKey);

        if (moveIndex < 0) {
          throw new Error("Placed tiles are not a legal move");
        }

        game.playMoveByIndex(moveIndex);
        await wait(0);
        resetInteractionState();
        setError("");
        await syncState(game);
      });
    } catch (e) {
      console.error(e);
      notifications.show({
        color: "red",
        title: "Invalid move",
        message: "Placed tiles are not a legal move",
      });
    }
  };

  const passTurn = async () => {
    if (!game) return;
    await queueGameOp(async () => {
      game.passTurn();
      await wait(0);
      await syncState(game);
    });
  };

  const exchange = async () => {
    if (!game) return;
    try {
      await queueGameOp(async () => {
        game.exchangeTiles(exchangeInput.trim().toUpperCase());
        await wait(0);
        resetInteractionState();
        setError("");
        await syncState(game);
      });
    } catch (e) {
      console.error(e);
      notifications.show({
        color: "red",
        title: "Exchange failed",
        message: "Enter only tiles from your rack (e.g. AEI)",
      });
    }
  };

  const newGameReset = () => {
    localStorage.removeItem(SAVE_SLOT_KEY);
    setGame(null);
    setState(null);
    stateRef.current = null;
    gameOpChainRef.current = Promise.resolve();
    resetInteractionState();
    setError("");
    setAllAiMode(false);
  };

  const updateSeat = (index: number, isAi: boolean) => {
    const next = [...setupSeats];
    next[index] = isAi;
    setAiSeats(next);
  };

  const placeSelectedTile = (source: DragGhost, row: number, col: number) => {
    if (!state) return;
    if (state.board_rows[row][col] !== " ") {
      if (source.kind === "pending") {
        removeDraftByRackIndex(source.rackIndex);
      }
      return;
    }

    const existing = draftPlacements.find((p) => p.row === row && p.col === col);
    const current = draftPlacements.find((p) => p.rackIndex === source.rackIndex);

    if (source.kind === "rack" && current) {
      return;
    }

    const rackTile = state.current_rack[source.rackIndex] ?? current?.rackTile;
    if (!rackTile) {
      return;
    }

    const hasTargetPending = !!existing && existing.rackIndex !== source.rackIndex;

    if (rackTile === "*") {
      let nextDraft = draftPlacements.filter((p) => p.rackIndex !== source.rackIndex);
      if (hasTargetPending && existing) {
        if (source.kind === "pending" && current) {
          nextDraft = nextDraft.map((p) =>
            p.rackIndex === existing.rackIndex ? { ...p, row: current.row, col: current.col } : p
          );
        } else {
          nextDraft = nextDraft.filter((p) => p.rackIndex !== existing.rackIndex);
        }
      }

      setDraftPlacements(nextDraft);
      setPendingWildcardPlacement({ rackIndex: source.rackIndex, row, col });
      setWildcardLetter("");
      setError("");
      return;
    }

    let nextDraft = draftPlacements.filter((p) => p.rackIndex !== source.rackIndex);

    if (hasTargetPending && existing) {
      if (source.kind === "pending" && current) {
        nextDraft = nextDraft.map((p) =>
          p.rackIndex === existing.rackIndex ? { ...p, row: current.row, col: current.col } : p
        );
      } else {
        nextDraft = nextDraft.filter((p) => p.rackIndex !== existing.rackIndex);
      }
    }

    nextDraft = [
      ...nextDraft,
      {
        rackIndex: source.rackIndex,
        rackTile,
        placedTile: current?.rackTile === "*" ? current.placedTile : rackTile,
        row,
        col,
      },
    ];

    setDraftPlacements(nextDraft);
    setError("");
  };

  const handleDragStart = (event: DragStartEvent) => {
    const source = parseDragId(String(event.active.id));
    setActiveDrag(source);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const source = parseDragId(String(event.active.id));
    const overId = event.over ? String(event.over.id) : "";
    const target = overId ? parseCellId(overId) : null;
    setActiveDrag(null);

    if (!source) {
      return;
    }

    if (source.kind === "pending" && overId === rackDropId) {
      removeDraftByRackIndex(source.rackIndex);
      return;
    }

    if (!target) {
      return;
    }

    placeSelectedTile(source, target.row, target.col);
  };

  const removeDraftByRackIndex = (rackIndex: number) => {
    setDraftPlacements((curr) => curr.filter((p) => p.rackIndex !== rackIndex));
  };

  const confirmWildcardPlacement = () => {
    if (!state || !pendingWildcardPlacement) {
      return;
    }

    const letter = wildcardLetter.trim().toUpperCase();
    if (!/^[A-Z]$/.test(letter)) {
      notifications.show({
        color: "red",
        title: "Invalid wildcard",
        message: "Choose a single letter from A-Z",
      });
      return;
    }

    const rackTile = state.current_rack[pendingWildcardPlacement.rackIndex];
    if (rackTile !== "*") {
      notifications.show({
        color: "red",
        title: "Wildcard unavailable",
        message: "Wildcard is no longer available in your rack",
      });
      setPendingWildcardPlacement(null);
      setWildcardLetter("");
      return;
    }

    const nextDraft = [
      ...draftPlacements.filter((p) => p.rackIndex !== pendingWildcardPlacement.rackIndex),
      {
        rackIndex: pendingWildcardPlacement.rackIndex,
        rackTile,
        placedTile: letter,
        row: pendingWildcardPlacement.row,
        col: pendingWildcardPlacement.col,
      },
    ];

    setDraftPlacements(nextDraft);
    setPendingWildcardPlacement(null);
    setWildcardLetter("");
    setError("");
  };

  const getBonusLabel = (row: number, col: number) => BONUS_LABELS[row][col];
  const draftByCell = new Map(
    draftPlacements.map((placement) => [`${placement.row}-${placement.col}`, placement])
  );

  const overlayLabel = (() => {
    if (!state || !activeDrag) {
      return "";
    }

    if (activeDrag.kind === "rack") {
      return state.current_rack[activeDrag.rackIndex] ?? "";
    }

    const pending = draftPlacements.find((p) => p.rackIndex === activeDrag.rackIndex);
    return pending?.rackTile === "*" ? `${pending.placedTile}*` : pending?.placedTile ?? "";
  })();


  if (!game || !state) {
    return (
      <Box className="container">
        <Stack gap="md">
          <Title order={1}>Scrabble WASM</Title>
          <Text c="dimmed">Set up players (Human/AI). AI turns play automatically.</Text>
          <Card withBorder radius="md" padding="md">
            <Stack gap="sm">
              <NumberInput
                label="Total players"
                min={1}
                max={4}
                value={totalPlayers}
                onChange={(value) => {
                  const next = Number(value);
                  if (Number.isNaN(next)) {
                    return;
                  }
                  setTotalPlayers(Math.max(1, Math.min(4, next)));
                }}
              />

              <div className="seat-list">
                {Array.from({ length: totalPlayers }, (_, i) => (
                  <div key={i} className="seat-row">
                    <Text>Player {i + 1}</Text>
                    <Select
                      w={130}
                      data={[
                        { value: "human", label: "Human" },
                        { value: "ai", label: "AI" },
                      ]}
                      value={setupSeats[i] ? "ai" : "human"}
                      onChange={(value) => updateSeat(i, value === "ai")}
                    />
                  </div>
                ))}
              </div>

              <Button onClick={startGame} loading={loading}>
                Start game
              </Button>
            </Stack>
          </Card>

          {!!error && <Alert color="red">{error}</Alert>}
        </Stack>
      </Box>
    );
  }

  return (
    <Box className="container">
      <Stack gap="sm">
        <Group justify="space-between" align="center">
          <Title order={1}>Scrabble WASM</Title>
          <Button variant="light" onClick={newGameReset}>
            New game
          </Button>
        </Group>

        <Group gap="sm">
          <Badge size="lg" variant="light" color="indigo">
            Turn: Player {state.current_turn + 1}
          </Badge>
          <Badge size="lg" variant="outline" color="grape">
            Winner: {state.winner === null ? "-" : `Player ${state.winner + 1}`}
          </Badge>
          <Badge size="lg" variant="light" color="teal">
            Bag: {state.tiles_in_bag}
          </Badge>
        </Group>

        <div className="score-grid">
          {state.scores.map((score, idx) => (
            <Card
              key={idx}
              className={`score-card ${idx === state.current_turn ? "score-card-active" : ""}`}
              withBorder
              padding="sm"
              radius="md"
            >
              <Text size="sm" className="score-card-label">
                Player {idx + 1}
              </Text>
              <Text className="score-card-value">{score}</Text>
            </Card>
          ))}
        </div>

        {state.no_moves_hint && (
          <Alert color="yellow" className="hint">
            No legal moves for this rack. You must pass or exchange.
          </Alert>
        )}

        <DndContext sensors={sensors} onDragStart={handleDragStart} onDragEnd={handleDragEnd}>
          <div className="board">
            {state.board_rows.map((row, rIdx) =>
              row.split("").map((cell, cIdx) => {
                const pending = draftByCell.get(`${rIdx}-${cIdx}`);
                const pendingLabel = pending
                  ? pending.rackTile === "*"
                    ? `${pending.placedTile}*`
                    : pending.placedTile
                  : "";
                const bonus = getBonusLabel(rIdx, cIdx).toLowerCase() || "none";

                return (
                  <DroppableCell
                    key={`${rIdx}-${cIdx}`}
                    id={cellId(rIdx, cIdx)}
                    className={`cell bonus-${bonus} ${pending ? "cell-pending" : ""} ${
                      lastMoveCells.has(`${rIdx}-${cIdx}`) ? "cell-last-move" : ""
                    }`}
                    occupied={cell !== " "}
                    onClick={() => {
                      // No click-to-return behavior.
                    }}
                  >
                    {cell !== " " ? (
                      cell
                    ) : pending ? (
                      <DraggableTile
                        id={pendingId(pending.rackIndex)}
                        label={pendingLabel}
                        className="pending-tile"
                        onClick={() => {
                          if (pending.rackTile === "*") {
                            setPendingWildcardPlacement({
                              rackIndex: pending.rackIndex,
                              row: pending.row,
                              col: pending.col,
                            });
                            setWildcardLetter(pending.placedTile);
                          }
                        }}
                      />
                    ) : getBonusLabel(rIdx, cIdx) ? (
                      <span className="bonus-label">{getBonusLabel(rIdx, cIdx)}</span>
                    ) : (
                      ""
                    )}
                  </DroppableCell>
                );
              })
            )}
          </div>

          <div className="control-panels">
            <Card withBorder radius="md" padding="sm">
              <Stack gap={6}>
                <Title order={4}>Rack</Title>
                <DroppableRack id={rackDropId}>
                  <div className="rack">
                    {state.current_rack.split("").map((tile, idx) => {
                      const isUsed = draftPlacements.some((p) => p.rackIndex === idx);
                      if (isUsed) {
                        return null;
                      }
                      return (
                        <DraggableTile
                          key={`${tile}-${idx}`}
                          id={rackId(idx)}
                          label={tile}
                          className="tile"
                        />
                      );
                    })}
                  </div>
                </DroppableRack>
                <Text size="xs" c="dimmed">
                  Drag to board. Drag pending tiles back here to return.
                </Text>
              </Stack>
            </Card>

            {!state.is_ai_turn && state.winner === null && (
              <Card withBorder radius="md" padding="sm">
                <Stack gap={6}>
                  <Title order={4}>Turn</Title>
                  <Group className="actions">
                    <Button size="xs" onClick={submitDraftMove} disabled={draftPlacements.length === 0}>
                      Play
                    </Button>
                    <Button
                      size="xs"
                      variant="default"
                      onClick={() => setDraftPlacements([])}
                      disabled={draftPlacements.length === 0}
                    >
                      Clear
                    </Button>
                    <Button size="xs" variant="default" onClick={passTurn}>
                      Pass
                    </Button>
                  </Group>

                  <Group className="actions actions-compact">
                    <TextInput
                      size="xs"
                      value={exchangeInput}
                      maxLength={7}
                      placeholder="Exchange (AEI)"
                      onChange={(e) =>
                        setExchangeInput(
                          e.currentTarget.value.replace(/[^A-Za-z*]/g, "").toUpperCase()
                        )
                      }
                    />
                    <Button
                      size="xs"
                      onClick={exchange}
                      disabled={!state.can_exchange || exchangeInput.trim().length === 0}
                    >
                      Exchange
                    </Button>
                  </Group>
                </Stack>
              </Card>
            )}
          </div>

          <DragOverlay>
            {activeDrag && <div className="tile tile-drag-overlay">{overlayLabel}</div>}
          </DragOverlay>
        </DndContext>

        {!!error && <Alert color="red">{error}</Alert>}
      </Stack>

      <Modal
        opened={pendingWildcardPlacement !== null}
        onClose={() => {
          setPendingWildcardPlacement(null);
          setWildcardLetter("");
        }}
        title="Choose wildcard letter"
      >
        <Stack gap="sm">
          <Text size="sm">
            This blank tile can represent any letter. Choose one letter (A-Z) for this placement.
          </Text>
          <TextInput
            value={wildcardLetter}
            maxLength={1}
            placeholder="A"
            onChange={(e) =>
              setWildcardLetter(e.currentTarget.value.replace(/[^A-Za-z]/g, "").toUpperCase())
            }
          />
          <Group justify="flex-end">
            <Button
              variant="default"
              onClick={() => {
                setPendingWildcardPlacement(null);
                setWildcardLetter("");
              }}
            >
              Cancel
            </Button>
            <Button onClick={confirmWildcardPlacement}>Confirm</Button>
          </Group>
        </Stack>
      </Modal>
    </Box>
  );
}

