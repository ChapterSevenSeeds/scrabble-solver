use crate::utils::char_count_to_map;
use std::collections::HashSet;
use std::{fs::File, io::Read};

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

// 0 Points: Blank
// 1 Point: A, E, I, L, N, O, R, S, T, U
// 2 Points: D, G
// 3 Points: B, C, M, P
// 4 Points: F, H, V, W, Y
// 5 Points: K
// 8 Points: J, X
// 10 Points: Q, Z
const SCORES: [u32; 91] = {
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 1, 3, 3, 2, 1, 4, 2, 4, 1, 8, 5, 1, 3, 1, 1, 3, 10, 1, 1, 1, 1, 4, 4, 8, 4,
        10,
    ]
};

struct MoveConstraint {
    prefix: Option<String>,
    suffix: Option<String>,
    gap: u32, // The space between the prefix and suffix, or the left/top of the board to the prefix, or the right/bottom of the board to the suffix
}

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

    pub fn dump(&self) {
        for row in self.board {
            println!("{:?}", row)
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

    pub fn get_move_score(&self, word: &str, row: usize, col: usize, horizontal: bool) -> u32 {
        let mut score = 0;
        let mut word_multiplier = 1;

        if word.len() > 1 {
            for (i, str_char) in word.chars().enumerate() {
                let (r, c) = if horizontal {
                    (row, col + i)
                } else {
                    (row + i, col)
                };
                let letter_score = SCORES[(str_char as u8) as usize];
                match SCORE_MODIFIERS[r][c] {
                    ScoreModifier::None => score += letter_score,
                    ScoreModifier::DLS => score += letter_score * 2,
                    ScoreModifier::TLS => score += letter_score * 3,
                    ScoreModifier::DWS => {
                        score += letter_score;
                        word_multiplier *= 2;
                    }
                    ScoreModifier::TWS => {
                        score += letter_score;
                        word_multiplier *= 3;
                    }
                }
            }
        }

        score *= word_multiplier;

        for (i, str_char) in word.chars().enumerate() {
            let (r, c) = if horizontal {
                (row, col + i)
            } else {
                (row + i, col)
            };

            // Check for vertical words if placing horizontally
            if horizontal || word.len() == 1 {
                let mut extra_word_score: u32 = 0;
                let mut extra_word_multiplier: u32 = 1;

                let mut vertical_word = String::new();
                // Check upwards
                for r_up in (0..r).rev() {
                    if self.board[r_up][c] == ' ' {
                        break;
                    }
                    vertical_word.insert(0, self.board[r_up][c]);
                    extra_word_score += SCORES[self.board[r_up][c] as usize];
                }
                // Add the current letter
                vertical_word.push(str_char);
                let letter_score = SCORES[str_char as usize];
                match SCORE_MODIFIERS[r][c] {
                    ScoreModifier::None => extra_word_score += letter_score,
                    ScoreModifier::DLS => extra_word_score += letter_score * 2,
                    ScoreModifier::TLS => extra_word_score += letter_score * 3,
                    ScoreModifier::DWS => {
                        extra_word_score += letter_score;
                        extra_word_multiplier *= 2;
                    }
                    ScoreModifier::TWS => {
                        extra_word_score += letter_score;
                        extra_word_multiplier *= 3;
                    }
                }

                // Check downwards
                for r_down in r + 1..15 {
                    if self.board[r_down][c] == ' ' {
                        break;
                    }
                    vertical_word.push(self.board[r_down][c]);
                    extra_word_score += SCORES[self.board[r_down][c] as usize];
                }

                if vertical_word.len() > 1 {
                    score += extra_word_score * extra_word_multiplier;
                }
            }

            if !horizontal || word.len() == 1 {
                let mut extra_word_score: u32 = 0;
                let mut extra_word_multiplier: u32 = 1;

                // Check for horizontal words if placing vertically
                let mut horizontal_word = String::new();
                // Check left
                for c_left in (0..c).rev() {
                    if self.board[r][c_left] == ' ' {
                        break;
                    }
                    horizontal_word.insert(0, self.board[r][c_left]);
                    extra_word_score += SCORES[self.board[r][c_left] as usize];
                }
                // Add the current letter
                horizontal_word.push(str_char);
                let letter_score = SCORES[str_char as usize];
                match SCORE_MODIFIERS[r][c] {
                    ScoreModifier::None => extra_word_score += letter_score,
                    ScoreModifier::DLS => extra_word_score += letter_score * 2,
                    ScoreModifier::TLS => extra_word_score += letter_score * 3,
                    ScoreModifier::DWS => {
                        extra_word_score += letter_score;
                        extra_word_multiplier *= 2;
                    }
                    ScoreModifier::TWS => {
                        extra_word_score += letter_score;
                        extra_word_multiplier *= 3;
                    }
                }
                // Check right
                for c_right in c + 1..15 {
                    if self.board[r][c_right] == ' ' {
                        break;
                    }
                    horizontal_word.push(self.board[r][c_right]);
                    extra_word_score += SCORES[self.board[r][c_right] as usize];
                }

                if horizontal_word.len() > 1 {
                    score += extra_word_score * extra_word_multiplier;
                }
            }
        }

        score
    }

    pub fn is_valid_move(&self, word: &str, row: usize, col: usize, horizontal: bool) -> bool {
        // If the move contains more than one letter, then it must be a valid word as-is.
        if word.len() > 1 && !self.valid_words.contains(word) {
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
        for (i, str_char) in word.chars().enumerate() {
            let (r, c) = if horizontal {
                (row, col + i)
            } else {
                (row + i, col)
            };
            // Check for vertical words if placing horizontally
            if horizontal || word.len() == 1 {
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
                if vertical_word.len() > 1 && !self.valid_words.contains(&vertical_word) {
                    return false;
                }
            }

            if !horizontal || word.len() == 1 {
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
                if horizontal_word.len() > 1 && !self.valid_words.contains(&horizontal_word) {
                    return false;
                }
            }
        }

        true
    }

    pub fn get_possible_words_from_chars(
        &self,
        chars: &str,
        constraint: Option<MoveConstraint>,
    ) -> Vec<String> {
        let mut result = Vec::new();

        let chars_set = char_count_to_map(chars);
        for valid_word in self.valid_words.iter() {
            if constraint.as_ref().is_some()
                && (constraint.as_ref().unwrap().prefix.is_some()
                    && !valid_word
                        .starts_with(constraint.as_ref().unwrap().prefix.as_ref().unwrap()))
                || (constraint.as_ref().unwrap().suffix.is_some()
                    && !valid_word.ends_with(constraint.as_ref().unwrap().suffix.as_ref().unwrap()))
            {
                continue;
            }

            // If we got here, then the prefix and suffix are satisfied. If either are specified, then the user could potentially put down tiles that don't form a word by themselves, but extend another word.

            // Valid words that have characters not in the player's char set are disqualified.
            if valid_word.chars().any(|x| !chars_set.contains_key(&x)) {
                continue;
            }

            // Is the letters of this word a subset of the chars argument?
            let valid_word_count_map = char_count_to_map(valid_word);
            if valid_word_count_map
                .iter()
                .any(|(valid_word_char, valid_word_char_count)| {
                    valid_word_char_count > &chars_set[valid_word_char]
                })
            {
                continue;
            }

            result.push(valid_word.clone());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic_valid_move() {
        let board = ScrabbleBoard::new();
        assert!(board.is_valid_move("HELLO", 7, 7, true));
    }

    #[test]
    fn test_invalid_move_out_of_bounds() {
        let board = ScrabbleBoard::new();
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

    #[test]
    fn test_letter_placement_forms_word_both_directions() {
        let mut board = ScrabbleBoard::new();
        board.place_word("SPEED", 0, 0, true);
        board.place_word("METER", 0, 6, true);
        assert!(board.is_valid_move("O", 0, 5, true));
    }

    #[test]
    fn test_vertical_letter_forms_invalid_vertical_word() {
        let mut board = ScrabbleBoard::new();
        board.place_word("FETCH", 0, 0, true);
        assert!(!board.is_valid_move("F", 1, 0, false));
    }

    #[test]
    fn test_single_letter_forms_words_in_both_directions() {
        let mut board = ScrabbleBoard::new();
        board.place_word("SPEED", 5, 0, true);
        board.place_word("METER", 5, 6, true);
        board.place_word("SPEED", 0, 5, false);
        board.place_word("METER", 6, 5, false);
        assert!(board.is_valid_move("O", 5, 5, true));
    }

    #[test]
    fn test_simple_score() {
        let board = ScrabbleBoard::new();
        assert_eq!(board.get_move_score("SPEED", 0, 0, true), 27)
        // S (1, TWS) + P (3) + E (1) + E (2, DLS) + D (2) == 27
    }

    #[test]
    fn test_multiple_word_score() {
        let mut board = ScrabbleBoard::new();
        board.place_word("SHELL", 0, 0, true);
        // Now place HAD vertically starting at (0,2) which would create the words "SH", "HA", and "ED"
        // H (4) + A (1, DWS) + D (2) == 14
        // S (1) + H (4) == 5
        // H (4) + A (1, DWS) == 10
        // E (1) + D (2) == 3
        // Total == 32
        assert_eq!(board.get_move_score("HAD", 1, 0, true), 32);
    }

    #[test]
    fn test_single_letter_forms_long_word() {
        let mut board = ScrabbleBoard::new();
        board.place_word("SPEED", 0, 0, true);
        board.place_word("METER", 0, 6, true);

        // S (1) + P (3) + E (1) + E (1) + D (2) + O (1) + M (3) + E (1) + T (1) + E (1) + R (1) == 16

        assert_eq!(board.get_move_score("O", 0, 5, true), 16);
    }
}
