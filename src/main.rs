mod board;
pub mod utils;
use regex::Regex;

fn main() {
    let mut board = board::ScrabbleBoard::new();
    let mut timer = stopwatch::Stopwatch::start_new();
    board.place_word("SPEED", 0, 0, true);
    board.place_word("METER", 0, 6, true);
    let possible_words = board.get_moves_from_spot("ASDFOIADDSPEROG", 0, 5, true, 10);
    timer.stop();
    println!("{:?}, {}", possible_words, timer);
}
