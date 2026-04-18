use crate::board::ScrabbleBoard;
use crate::common::{Coords};

pub struct ScrabbleBoardIterator<'a> {
    horizontal: bool,
    forwards: bool,
    board: &'a ScrabbleBoard,

    // So that we actually iterate over everything, we use i32 so that we can see if we go negative when going backwards.
    current_coords: (i32, i32),
}

pub struct ScrabbleBoardIteratorItem {
    pub coords: Coords,
    pub char_at_coords: char,
}

impl ScrabbleBoardIterator<'_> {
    pub fn new(
        board: &'_ ScrabbleBoard,
        start: Coords,
        horizontal: bool,
        forwards: bool,
    ) -> ScrabbleBoardIterator<'_> {
        ScrabbleBoardIterator {
            forwards,
            horizontal,
            current_coords: (start.0 as i32, start.1 as i32),
            board,
        }
    }
}

impl Iterator for ScrabbleBoardIterator<'_> {
    type Item = ScrabbleBoardIteratorItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_coords.0 < 0
            || self.current_coords.0 > 14
            || self.current_coords.1 < 0
            || self.current_coords.1 > 14
        {
            return None;
        }

        let current_coords = (
            self.current_coords.0 as usize,
            self.current_coords.1 as usize,
        );
        let current_char = self.board[current_coords];

        // Don't return None until we have gone past the end in either direction.

        if self.horizontal {
            if self.forwards {
                self.current_coords.1 += 1;
            } else {
                self.current_coords.1 -= 1;
            }
        } else {
            if self.forwards {
                self.current_coords.0 += 1;
            } else {
                self.current_coords.0 -= 1;
            }
        }

        Some(ScrabbleBoardIteratorItem {
            coords: current_coords,
            char_at_coords: current_char,
        })
    }
}
