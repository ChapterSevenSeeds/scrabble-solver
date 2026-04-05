use crate::board::ScoreModifier::TripleWord;
use std::{fs::File, io::Read, os};

pub struct ScrabbleBoard {
    board: [[char; 15]; 15],
    valid_words: std::collections::HashSet<String>,
}

enum ScoreModifier {
    None,
    TWS,
    DWS,
    TLS,
    DLS,
}

const NON: ScoreModifier = ScoreModifier::None;
const TWS: ScoreModifier = ScoreModifier::TWS;
const DWS: ScoreModifier = ScoreModifier::DWS;
const TLS: ScoreModifier = ScoreModifier::TLS;
const DLS: ScoreModifier = ScoreModifier::DLS;

#[rustfmt::skip]
const SCORE_MODIFIERS: [[ScoreModifier; 15]; 15] = {
    [
        [TWS, NON, NON, DLS, NON, NON, NON, TWS, NON, NON, NON, DLS, NON, NON, TWS],
        [NON, DWS, NON, NON, NON, TLS, NON, NON, NON, TLS, NON, NON, NON, DWS, NON],
        [NON, NON, DWS, NON, NON, NON, DLS, NON, DLS, NON, NON, NON, DWS, NON, NON],
        [DLS, NON, NON, DWS, NON, NON, NON, DLS, NON, NON, NON, DWS, NON, NON, DLS],
        [NON, NON, NON, NON, DWS, NON, NON, NON, NON, NON, DWS, NON, NON, NON, NON],
        [NON, TLS, NON, NON, NON, TLS, NON, NON, NON, TLS, NON, NON, NON, TLS, NON],
        [NON, NON, DLS, NON, NON, NON, DLS, NON, DLS, NON, NON, NON, DLS, NON, NON],
        [TWS, NON, NON, DLS, NON, NON, NON, NON, NON, NON, NON, DLS, NON, NON, TWS],
        [NON, NON, DLS, NON, NON, NON, DLS, NON, DLS, NON, NON, NON, DLS, NON, NON],
        [NON, TLS, NON, NON, NON, TLS, NON, NON, NON, TLS, NON, NON, NON, TLS, NON],
        [NON, NON, NON, NON, DWS, NON, NON, NON, NON, NON, DWS, NON, NON, NON, NON],
        [DLS, NON, NON, DWS, NON, NON, NON, DLS, NON, NON, NON, DWS, NON, NON, DLS],
        [NON, NON, DWS, NON, NON, NON, DLS, NON, DLS, NON, NON, NON, DWS, NON, NON],
        [NON, DWS, NON, NON, NON, TLS, NON, NON, NON, TLS, NON, NON, NON, DWS, NON],
        [TWS, NON, NON, DLS, NON, NON, NON, TWS, NON, NON, NON, DLS, NON, NON, TWS]
    ]
};

const SCORES: [u32; 91] = {
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 1, 3, 3, 2, 1, 4, 2, 4, 1, 8, 5, 1, 3, 1, 1, 3, 10, 1, 1, 1, 1, 4, 4, 8, 4, 10
    ]
};

impl ScrabbleBoard {
    pub fn new() -> Self {
        let mut buf = String::new();
        File::open("words.txt")
            .expect("Failed to open config file")
            .read_to_string(&mut buf)
            .expect("Failed to read config file");
        let words = buf
            .lines()
            .map(|line| line.trim().to_string())
            .collect::<Vec<String>>();

        Self {
            board: [[' '; 15]; 15],
            valid_words: words.into_iter().collect(),
        }
    }

    pub fn place_word(&mut self, word: &str, row: usize, col: usize, horizontal: bool) {
        for (i, str_char) in word.chars().enumerate() {
            let (r, c) = if horizontal {
                (row, col + i)
            } else {
                (row + i, col)
            };
            self.board[r][c] = str_char;
        }
    }

    pub fn is_valid_move(&self, word: &str, row: usize, col: usize, horizontal: bool) -> bool {
        if !self.valid_words.contains(word) {
            return false;
        }

        // Check if the word fits on the board
        if horizontal {
            if col + word.len() > 15 {
                return false;
            }
        } else {
            if row + word.len() > 15 {
                return false;
            }
        }

        // Make sure the spot on the board is empty and accommodates the word
        for (i, _str_char) in word.chars().enumerate() {
            let (r, c) = if horizontal {
                (row, col + i)
            } else {
                (row + i, col)
            };
            if self.board[r][c] != ' ' {
                return false;
            }
        }

        // Now collect all new possible words formed by this move and check if they are valid
        let mut new_words = Vec::new();
        new_words.push(word.to_string()); // The main word being placed
        for (i, str_char) in word.chars().enumerate() {
            let (r, c) = if horizontal {
                (row, col + i)
            } else {
                (row + i, col)
            };
            // Check for vertical words if placing horizontally
            if horizontal {
                let mut vertical_word = String::new();
                // Check upwards
                for r_up in (0..r).rev() {
                    if self.board[r_up][c] == ' ' {
                        break;
                    }
                    vertical_word.insert(0, self.board[r_up][c]);
                }
                // Add the current letter
                vertical_word.push(str_char);
                // Check downwards
                for r_down in r + 1..15 {
                    if self.board[r_down][c] == ' ' {
                        break;
                    }
                    vertical_word.push(self.board[r_down][c]);
                }
                if !vertical_word.is_empty() && !self.valid_words.contains(&vertical_word) {
                    return false;
                }
            } else {
                // Check for horizontal words if placing vertically
                let mut horizontal_word = String::new();
                // Check left
                for c_left in (0..c).rev() {
                    if self.board[r][c_left] == ' ' {
                        break;
                    }
                    horizontal_word.insert(0, self.board[r][c_left]);
                }
                // Add the current letter
                horizontal_word.push(str_char);
                // Check right
                for c_right in c + 1..15 {
                    if self.board[r][c_right] == ' ' {
                        break;
                    }
                    horizontal_word.push(self.board[r][c_right]);
                }
                if !horizontal_word.is_empty() && !self.valid_words.contains(&horizontal_word) {
                    return false;
                }
            }
        }

        // Check if all new words are valid
        for new_word in new_words {
            if !self.valid_words.contains(&new_word) {
                return false;
            }
        }

        true
    }
}

// Some tests
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic_valid_move() {
        let mut board = ScrabbleBoard::new();
        assert!(board.is_valid_move("HELLO", 7, 7, true));
    }

    #[test]
    fn test_invalid_move_out_of_bounds() {
        let mut board = ScrabbleBoard::new();
        assert!(!board.is_valid_move("HELLO", 7, 12, true));
    }

    #[test]
    fn test_invalid_move_overlapping() {
        let mut board = ScrabbleBoard::new();
        // Place HELLO at (7,7) horizontally
        board.place_word("HELLO", 7, 7, true);
        // Now try to place WORLD overlapping with O in HELLO
        assert!(!board.is_valid_move("WORLD", 6, 9, false));
    }

    #[test]
    fn test_valid_move_with_new_words() {
        let mut board = ScrabbleBoard::new();
        // Place SCRAP at the top left horizontally
        board.place_word("SHELL", 0, 0, true);
        // Now place HAD vertically starting at (0,2) which would create the words "SH", "HA", and "ED"
        assert!(board.is_valid_move("HAD", 1, 0, true));
    }
}
