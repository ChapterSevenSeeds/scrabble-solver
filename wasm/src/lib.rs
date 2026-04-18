use scrabble::game::ScrabbleGame;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use rand::random;

#[derive(Clone, Copy, Serialize, Deserialize)]
enum SeatKind {
    Human,
    Ai,
}

#[derive(Serialize, Deserialize)]
struct SavedSession {
    game_json: String,
    seats: Vec<SeatKind>,
}

#[derive(Serialize)]
struct MoveView {
    score: u32,
    tiles: Vec<TileView>,
}

#[derive(Serialize, Deserialize)]
struct TileView {
    row: usize,
    col: usize,
    tile: char,
}

#[derive(Serialize)]
struct GameStateView {
    board_rows: Vec<String>,
    current_turn: usize,
    total_players: usize,
    scores: Vec<i32>,
    tiles_in_bag: usize,
    winner: Option<usize>,
    racks: Vec<String>,
    current_rack: String,
    is_ai_turn: bool,
    no_moves_hint: bool,
    can_exchange: bool,
}

#[wasm_bindgen]
pub struct WasmGame {
    game: ScrabbleGame,
    seats: Vec<SeatKind>,
}

impl WasmGame {
    fn is_ai_turn(&self) -> bool {
        matches!(self.seats[self.game.current_turn()], SeatKind::Ai)
    }

    fn legal_moves_json_internal(&self) -> Result<String, JsValue> {
        let views = self
            .game
            .get_moves()
            .iter()
            .map(|mv| MoveView {
                score: mv.get_score(),
                tiles: mv
                    .get_tiles()
                    .iter()
                    .map(|tile| TileView {
                        row: tile.coords.0,
                        col: tile.coords.1,
                        tile: tile.tile,
                    })
                    .collect(),
            })
            .collect::<Vec<MoveView>>();

        serde_json::to_string(&views).map_err(|err| JsValue::from_str(&err.to_string()))
    }

    fn rack_can_supply_tiles(&self, tiles: &str) -> bool {
        let mut rack_counts = std::collections::HashMap::<char, usize>::new();
        for c in self.game.current_player_rack().chars() {
            *rack_counts.entry(c).or_insert(0) += 1;
        }

        for c in tiles.chars() {
            let entry = rack_counts.entry(c).or_insert(0);
            if *entry == 0 {
                return false;
            }
            *entry -= 1;
        }

        true
    }

}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(total_players: usize, ai_player_indexes_json: String) -> Result<WasmGame, JsValue> {
        if total_players == 0 || total_players > 4 {
            return Err(JsValue::from_str("total_players must be between 1 and 4"));
        }

        let ai_indexes: Vec<usize> = serde_json::from_str(&ai_player_indexes_json)
            .map_err(|err| JsValue::from_str(&format!("Invalid ai_player_indexes_json: {err}")))?;

        let mut seats = vec![SeatKind::Human; total_players];
        for ai_index in ai_indexes {
            if ai_index >= total_players {
                return Err(JsValue::from_str("AI seat index out of range"));
            }
            seats[ai_index] = SeatKind::Ai;
        }

        Ok(WasmGame {
            game: ScrabbleGame::new(total_players, random()),
            seats,
        })
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: String) -> Result<WasmGame, JsValue> {
        let saved: SavedSession = serde_json::from_str(&json)
            .map_err(|err| JsValue::from_str(&format!("Invalid save JSON: {err}")))?;

        let game = ScrabbleGame::from_json(&saved.game_json)
            .map_err(|err| JsValue::from_str(&format!("Invalid game state: {err}")))?;

        if saved.seats.len() != game.total_players() {
            return Err(JsValue::from_str("Seat count does not match game player count"));
        }

        Ok(WasmGame {
            game,
            seats: saved.seats,
        })
    }

    #[wasm_bindgen(js_name = saveJson)]
    pub fn save_json(&self) -> Result<String, JsValue> {
        let game_json = self
            .game
            .to_json()
            .map_err(|err| JsValue::from_str(&format!("Failed to serialize game: {err}")))?;
        let session = SavedSession {
            game_json,
            seats: self.seats.clone(),
        };

        serde_json::to_string(&session).map_err(|err| JsValue::from_str(&err.to_string()))
    }

    #[wasm_bindgen(js_name = getStateJson)]
    pub fn get_state_json(&self) -> Result<String, JsValue> {
        let board_rows = self.game.board_rows();
        let legal_move_count = self.game.get_moves().len();
        let is_ai_turn = self.is_ai_turn();
        let view = GameStateView {
            board_rows,
            current_turn: self.game.current_turn(),
            total_players: self.game.total_players(),
            scores: self.game.scores(),
            tiles_in_bag: self.game.bag_tile_count(),
            winner: self.game.winner_index(),
            racks: (0..self.game.total_players())
                .map(|player| self.game.rack_for_player(player))
                .collect(),
            current_rack: self.game.current_player_rack(),
            is_ai_turn,
            no_moves_hint: !is_ai_turn && legal_move_count == 0,
            can_exchange: self.game.can_exchange(),
        };

        serde_json::to_string(&view).map_err(|err| JsValue::from_str(&err.to_string()))
    }

    #[wasm_bindgen(js_name = getLegalMovesJson)]
    pub fn get_legal_moves_json(&self) -> Result<String, JsValue> {
        self.legal_moves_json_internal()
    }

    #[wasm_bindgen(js_name = playMoveByIndex)]
    pub fn play_move_by_index(&mut self, index: usize) -> Result<(), JsValue> {
        if self.is_ai_turn() {
            return Err(JsValue::from_str("Current seat is AI; use stepAiTurn"));
        }

        let mv = self
            .game
            .get_moves()
            .get(index)
            .cloned()
            .ok_or_else(|| JsValue::from_str("Move index out of range"))?;

        self.game.make_turn(mv);
        Ok(())
    }

    #[wasm_bindgen(js_name = playMoveByPlacementsJson)]
    pub fn play_move_by_placements_json(&mut self, placements_json: String) -> Result<(), JsValue> {
        if self.is_ai_turn() {
            return Err(JsValue::from_str("Current seat is AI; human moves are not allowed"));
        }

        let placements: Vec<TileView> = serde_json::from_str(&placements_json)
            .map_err(|err| JsValue::from_str(&format!("Invalid placements JSON: {err}")))?;
        if placements.is_empty() {
            return Err(JsValue::from_str("No tile placements provided"));
        }

        let mv = self
            .game
            .get_moves()
            .into_iter()
            .find(|candidate| {
                if candidate.get_tiles().len() != placements.len() {
                    return false;
                }

                let mut move_tiles = candidate
                    .get_tiles()
                    .iter()
                    .map(|t| (t.coords.0, t.coords.1, t.tile))
                    .collect::<Vec<(usize, usize, char)>>();
                let mut input_tiles = placements
                    .iter()
                    .map(|t| (t.row, t.col, t.tile))
                    .collect::<Vec<(usize, usize, char)>>();

                move_tiles.sort_unstable();
                input_tiles.sort_unstable();
                move_tiles == input_tiles
            })
            .ok_or_else(|| JsValue::from_str("Placed tiles do not form a legal move"))?;

        self.game.make_turn(mv);
        Ok(())
    }

    #[wasm_bindgen(js_name = passTurn)]
    pub fn pass_turn(&mut self) {
        self.game.pass();
    }

    #[wasm_bindgen(js_name = exchangeCurrentRack)]
    pub fn exchange_current_rack(&mut self) -> Result<(), JsValue> {
        if !self.game.can_exchange() {
            return Err(JsValue::from_str("Cannot exchange with fewer than 7 tiles in the bag"));
        }
        let rack = self.game.current_player_rack();
        self.game.exchange(rack);
        Ok(())
    }

    #[wasm_bindgen(js_name = exchangeTiles)]
    pub fn exchange_tiles(&mut self, tiles: String) -> Result<(), JsValue> {
        if !self.game.can_exchange() {
            return Err(JsValue::from_str("Cannot exchange with fewer than 7 tiles in the bag"));
        }

        if tiles.is_empty() {
            return Err(JsValue::from_str("Choose at least one tile to exchange"));
        }

        if !self.rack_can_supply_tiles(&tiles) {
            return Err(JsValue::from_str("Selected exchange tiles are not in the current rack"));
        }

        self.game.exchange(tiles);
        Ok(())
    }

    #[wasm_bindgen(js_name = stepAiTurn)]
    pub fn step_ai_turn(&mut self) -> Result<bool, JsValue> {
        if !self.is_ai_turn() {
            return Ok(false);
        }

        let moves = self.game.get_moves();
        if let Some(best_move) = moves.first() {
            self.game.make_turn(best_move.clone());
            return Ok(true);
        }

        if self.game.can_exchange() {
            let rack = self.game.current_player_rack();
            self.game.exchange(rack);
            return Ok(true);
        }

        self.game.pass();
        Ok(true)
    }

    #[wasm_bindgen(js_name = autoPlayUntilHumanOrEnd)]
    pub fn auto_play_until_human_or_end(&mut self) -> Result<(), JsValue> {
        while self.game.winner_index().is_none() && self.is_ai_turn() {
            self.step_ai_turn()?;
        }
        Ok(())
    }
}
