use scrabble::game::ScrabbleGame;

pub fn main() {
    let mut board = ScrabbleGame::new(2);

    loop {
        if board.winner.is_some() {
            println!("Winner: {:?}", board.winner.unwrap() + 1);
            break;
        }

        let possible_moves = board.get_moves();
        board.make_turn(possible_moves[0].clone());
        println!("{:?}", board);
    }
}
