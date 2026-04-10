use scrabble::board::ScrabbleGame;
use scrabble::tile_bag::TileBag;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() {
    let mut board = ScrabbleGame::new();
    let mut tile_bag = TileBag::new();
    let mut player_tiles = tile_bag.take(7);
    let mut score = 0;

    loop {
        // let mut timer = stopwatch::Stopwatch::start_new();
        let possible_words = board.get_moves(&*player_tiles);
        // timer.stop();
        board.place_tiles(&possible_words[0].tiles);
        score += possible_words[0].score;
        for tile in &possible_words[0].tiles {
            player_tiles.remove(player_tiles.find(tile.tile).unwrap());
        }
        player_tiles.push_str(&*tile_bag.take(7 - player_tiles.len()));

        // println!("{}, {}", timer, score);
        let board_str = board.dump();
        web_sys::console::log_1(&board_str.into());
    }
}
