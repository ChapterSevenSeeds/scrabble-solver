mod board;
pub mod utils;

fn main() {
    let mut board = board::ScrabbleBoard::new();
    let mut timer = stopwatch::Stopwatch::start_new();
    let possible_words = board.get_possible_words_from_chars("AOS", None, None);
    timer.stop();
    println!("{:?}, {}", possible_words, timer);
}
