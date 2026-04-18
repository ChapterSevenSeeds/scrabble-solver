use rand::random;
use scrabble::game::ScrabbleGame;

pub fn main() {
    let mut board = ScrabbleGame::new(2, random());

    loop {
        if board.winner.is_some() {
            println!("Winner: {:?}", board.winner.unwrap() + 1);
            break;
        }

        let possible_moves = board.get_moves();
        if let Some(best_move) = possible_moves.first() {
            board.make_turn(best_move.clone());
        } else if board.can_exchange() {
            board.exchange(board.current_player_rack());
        } else {
            board.pass();
        }
        println!("{:?}", board);
    }
}
