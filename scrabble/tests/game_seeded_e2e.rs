use scrabble::game::ScrabbleGame;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
struct ScenarioConfig {
    fixture_name: &'static str,
    total_players: usize,
    seed: u64,
    max_turns: usize,
    forced_pass_turns: &'static [usize],
    forced_exchange_turns: &'static [usize],
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ScenarioResult {
    scenario: String,
    total_players: usize,
    seed: u64,
    max_turns: usize,
    turn_count: usize,
    reached_turn_cap: bool,
    checkpoints: Vec<TurnCheckpoint>,
    final_scores: Vec<i32>,
    winner: Option<usize>,
    final_board_dump: String,
    final_board_fingerprint: u64,
    saw_play: bool,
    saw_pass: bool,
    saw_exchange: bool,
    saw_can_exchange_false: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct TurnCheckpoint {
    turn_index: usize,
    acting_player: usize,
    action: String,
    played_tiles: usize,
    played_score: u32,
    bag_tile_count: usize,
    can_exchange: bool,
    scores: Vec<i32>,
    player_racks_before: Vec<String>,
    player_racks_after: Vec<String>,
    rack_sizes: Vec<usize>,
    winner: Option<usize>,
    next_turn: usize,
    board_dump: String,
    board_fingerprint: u64,
}

fn scenarios() -> Vec<ScenarioConfig> {
    vec![
        ScenarioConfig {
            fixture_name: "baseline_2p_seed_20260416.json",
            total_players: 2,
            seed: 20_260_416,
            max_turns: 600,
            forced_pass_turns: &[],
            forced_exchange_turns: &[],
        },
        ScenarioConfig {
            fixture_name: "baseline_3p_seed_20260417.json",
            total_players: 3,
            seed: 20_260_417,
            max_turns: 700,
            forced_pass_turns: &[],
            forced_exchange_turns: &[],
        },
        ScenarioConfig {
            fixture_name: "baseline_4p_seed_20260418.json",
            total_players: 4,
            seed: 20_260_418,
            max_turns: 800,
            forced_pass_turns: &[],
            forced_exchange_turns: &[],
        },
        ScenarioConfig {
            fixture_name: "branch_forced_pass_2p_seed_777001.json",
            total_players: 2,
            seed: 777_001,
            max_turns: 220,
            forced_pass_turns: &[0, 1, 2, 3],
            forced_exchange_turns: &[],
        },
        ScenarioConfig {
            fixture_name: "branch_forced_exchange_2p_seed_888002.json",
            total_players: 2,
            seed: 888_002,
            max_turns: 240,
            forced_pass_turns: &[],
            forced_exchange_turns: &[0, 1, 2, 3],
        },
        ScenarioConfig {
            fixture_name: "stress_4p_seed_999003.json",
            total_players: 4,
            seed: 999_003,
            max_turns: 1200,
            forced_pass_turns: &[],
            forced_exchange_turns: &[],
        },
    ]
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn board_fingerprint(board: &str) -> u64 {
    // FNV-1a 64-bit for deterministic compact board checkpoints.
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in board.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn run_scenario(config: &ScenarioConfig) -> ScenarioResult {
    let mut game = ScrabbleGame::new_with_seed(config.total_players, config.seed);

    let mut checkpoints = Vec::new();
    let mut saw_play = false;
    let mut saw_pass = false;
    let mut saw_exchange = false;
    let mut saw_can_exchange_false = false;

    for turn_idx in 0..config.max_turns {
        if game.winner_index().is_some() {
            break;
        }

        let acting_player = game.current_turn();
        let player_racks_before = (0..game.total_players())
            .map(|player| game.rack_for_player(player))
            .collect::<Vec<String>>();
        let can_exchange_before = game.can_exchange();
        if !can_exchange_before {
            saw_can_exchange_false = true;
        }

        let (action, played_tiles, played_score) = if config.forced_pass_turns.contains(&turn_idx) {
            game.pass();
            saw_pass = true;
            ("pass".to_string(), 0, 0)
        } else if config.forced_exchange_turns.contains(&turn_idx) && can_exchange_before {
            let tiles = game.current_player_rack();
            let exchanged = tiles.len();
            game.exchange(tiles);
            saw_exchange = true;
            ("exchange".to_string(), exchanged, 0)
        } else {
            let possible_moves = game.get_moves();
            if let Some(best_move) = possible_moves.first() {
                let move_to_play = best_move.clone();
                let tiles = move_to_play.get_tiles().len();
                let score = move_to_play.get_score();
                game.make_turn(move_to_play);
                saw_play = true;
                ("play".to_string(), tiles, score)
            } else if can_exchange_before {
                let tiles = game.current_player_rack();
                let exchanged = tiles.len();
                game.exchange(tiles);
                saw_exchange = true;
                ("exchange".to_string(), exchanged, 0)
            } else {
                game.pass();
                saw_pass = true;
                ("pass".to_string(), 0, 0)
            }
        };

        let board_dump = game.board_dump();
        let player_racks_after = (0..game.total_players())
            .map(|player| game.rack_for_player(player))
            .collect::<Vec<String>>();
        let rack_sizes = (0..game.total_players())
            .map(|player| game.rack_for_player(player).len())
            .collect::<Vec<usize>>();

        checkpoints.push(TurnCheckpoint {
            turn_index: turn_idx,
            acting_player,
            action,
            played_tiles,
            played_score,
            bag_tile_count: game.bag_tile_count(),
            can_exchange: game.can_exchange(),
            scores: game.scores(),
            player_racks_before,
            player_racks_after,
            rack_sizes,
            winner: game.winner_index(),
            next_turn: game.current_turn(),
            board_dump: board_dump.clone(),
            board_fingerprint: board_fingerprint(&board_dump),
        });
    }

    let final_board = game.board_dump();
    ScenarioResult {
        scenario: config.fixture_name.to_string(),
        total_players: config.total_players,
        seed: config.seed,
        max_turns: config.max_turns,
        turn_count: checkpoints.len(),
        reached_turn_cap: checkpoints.len() == config.max_turns && game.winner_index().is_none(),
        checkpoints,
        final_scores: game.scores(),
        winner: game.winner_index(),
        final_board_dump: final_board.clone(),
        final_board_fingerprint: board_fingerprint(&final_board),
        saw_play,
        saw_pass,
        saw_exchange,
        saw_can_exchange_false,
    }
}

fn load_fixture(name: &str) -> ScenarioResult {
    let path = fixture_path(name);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing fixture file: {}", path.display()));
    serde_json::from_str::<ScenarioResult>(&content)
        .unwrap_or_else(|_| panic!("invalid fixture JSON: {}", path.display()))
}

fn write_fixture(name: &str, result: &ScenarioResult) {
    let path = fixture_path(name);
    let content = serde_json::to_string_pretty(result).expect("failed to serialize fixture");
    fs::write(&path, content).unwrap_or_else(|_| panic!("failed writing fixture: {}", path.display()));
}

fn assert_scenario_against_fixture(config: &ScenarioConfig) {
    let actual = run_scenario(config);
    let expected = load_fixture(config.fixture_name);
    assert_eq!(actual, expected, "scenario mismatch for {}", config.fixture_name);
}

#[test]
fn seeded_scenarios_match_fixtures() {
    for config in scenarios() {
        assert_scenario_against_fixture(&config);
    }
}

#[test]
fn branch_coverage_signals_present() {
    let all_runs = scenarios()
        .into_iter()
        .map(|config| run_scenario(&config))
        .collect::<Vec<ScenarioResult>>();

    assert!(all_runs.iter().any(|run| run.saw_play));
    assert!(all_runs.iter().any(|run| run.saw_pass));
    assert!(all_runs.iter().any(|run| run.saw_exchange));
    assert!(all_runs.iter().any(|run| run.saw_can_exchange_false));
}

#[test]
fn winner_stops_move_generation() {
    let mut game = ScrabbleGame::new_with_seed(2, 42);
    let first_move = game
        .get_moves()
        .first()
        .cloned()
        .expect("expected opening move");

    game.winner = Some(0);
    let turn_before = game.current_turn();
    let score_before = game.scores();
    let board_before = game.board_dump();

    game.make_turn(first_move);

    assert_eq!(game.current_turn(), turn_before);
    assert_eq!(game.scores(), score_before);
    assert_eq!(game.board_dump(), board_before);
    assert!(game.get_moves().is_empty());
}

#[test]
#[ignore]
fn regenerate_fixtures() {
    for config in scenarios() {
        let result = run_scenario(&config);
        write_fixture(config.fixture_name, &result);
    }
}

