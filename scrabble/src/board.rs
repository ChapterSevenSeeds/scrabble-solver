use crate::common::Coords;
use std::ops::{Index, IndexMut};

pub struct ScrabbleBoard {
    board: [[char; 15]; 15],
}

impl ScrabbleBoard {
    pub fn new() -> Self {
        ScrabbleBoard {
            board: [[' '; 15]; 15],
        }
    }

    pub fn dump(&self) -> String {
        let mut buf = String::new();
        for row in self.board {
            buf.push_str(&*format!("{:?}\n", row));
        }
        buf
    }
}

impl Index<Coords> for ScrabbleBoard {
    type Output = char;

    fn index(&self, index: Coords) -> &Self::Output {
        &self.board[index.0][index.1]
    }
}

impl IndexMut<Coords> for ScrabbleBoard {
    fn index_mut(&mut self, index: Coords) -> &mut Self::Output {
        &mut self.board[index.0][index.1]
    }
}
