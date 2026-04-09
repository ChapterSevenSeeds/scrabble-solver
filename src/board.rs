use crate::utils::char_count_to_map;
use regex::Regex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::BufRead;
use std::{fs::File, io::Read};

pub struct ScrabbleBoard {
    board: [[char; 15]; 15],
    pub valid_words: std::collections::HashSet<String>,
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

#[derive(Clone, Copy, Debug)]
struct TilePlacement {
    coords: (usize, usize),
    tile: char,
}

#[derive(Clone, Debug)]
pub struct PossibleMove {
    tiles: Vec<TilePlacement>,
    score: u32,
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

    pub fn parse_str(board_str: &str) -> Self {
        let mut board = Self::new();
        for (row, line) in board_str.lines().enumerate() {
            for (col, char) in line.chars().enumerate() {
                if char == ' ' {
                    continue;
                }

                board.board[row][col] = char;
            }
        }

        board
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

    pub fn get_move_score(
        &self,
        tile_placements: &Vec<TilePlacement>,
        horizontal: bool,
        recurse: bool,
    ) -> Option<u32> {
        let all_rows: BTreeSet<usize> = tile_placements.iter().map(|x| x.coords.0).collect();
        let all_columns: BTreeSet<usize> = tile_placements.iter().map(|x| x.coords.1).collect();

        if all_rows.len() > 1 && all_columns.len() > 1 {
            return None;
        }

        let mut score = 0;
        let mut word_multiplier = 1;
        let mut temp_word_scratch = String::new();
        let tile_placements_by_coords: HashMap<(usize, usize), &TilePlacement> =
            tile_placements.iter().map(|x| (x.coords, x)).collect();
        let mut tiles_placed = 0;

        // First, grab the score of the word formed along the tile placement vector
        if horizontal {
            let row = *all_rows.first().unwrap();
            // Go backwards to grab the letters from before it.
            for new_col in (0usize..*all_columns.first().unwrap()).rev() {
                if self.board[row][new_col] == ' ' {
                    break;
                }

                temp_word_scratch.insert(0, self.board[row][new_col]);
                score += SCORES[self.board[row][new_col] as usize];
            }

            // Then go forwards
            for new_col in *all_columns.first().unwrap()..15 {
                if tiles_placed >= tile_placements.len() {
                    break;
                }

                if tile_placements_by_coords.contains_key(&(row, new_col))
                    && self.board[row][new_col] != ' '
                {
                    // Trying to place a tile where there is already a tile. Return bad.
                    return None;
                }

                if self.board[row][new_col] != ' ' {
                    temp_word_scratch.push(self.board[row][new_col]);
                    score += SCORES[self.board[row][new_col] as usize];
                } else {
                    // If we got here, then we should be placing a tile.
                    tiles_placed += 1;
                    let tile_to_place = tile_placements_by_coords[&(row, new_col)].tile;
                    temp_word_scratch.push(tile_to_place);
                    let letter_score = SCORES[tile_to_place as usize];
                    match SCORE_MODIFIERS[row][new_col] {
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
        } else {
            let col = *all_columns.first().unwrap();
            // Go backwards to grab the letters from before it.
            for new_row in (0usize..*all_rows.first().unwrap()).rev() {
                if self.board[new_row][col] == ' ' {
                    break;
                }

                temp_word_scratch.insert(0, self.board[new_row][col]);
                score += SCORES[self.board[new_row][col] as usize];
            }

            // Then go forwards
            for new_row in *all_rows.first().unwrap()..15 {
                if tiles_placed >= tile_placements.len() {
                    break;
                }

                if tile_placements_by_coords.contains_key(&(new_row, col))
                    && self.board[new_row][col] != ' '
                {
                    // Trying to place a tile where there is already a tile. Return bad.
                    return None;
                }

                if self.board[new_row][col] != ' ' {
                    temp_word_scratch.push(self.board[new_row][col]);
                    score += SCORES[self.board[new_row][col] as usize];
                } else {
                    // If we got here, then we should be placing a tile.
                    tiles_placed += 1;
                    let tile_to_place = tile_placements_by_coords[&(new_row, col)].tile;
                    temp_word_scratch.push(tile_to_place);
                    let letter_score = SCORES[tile_to_place as usize];
                    match SCORE_MODIFIERS[new_row][col] {
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
        }

        if tile_placements.len() == 1 && temp_word_scratch.len() <= 1 {
            // If we're just trying to place one tile, but we didn't form any new words from the placement, then the placement is valid, just scoreless.
            return Some(0);
        }

        if !self.valid_words.contains(&temp_word_scratch) {
            return None;
        }

        score *= word_multiplier;

        if recurse {
            // Call this function once again for each tile with the opposite direction.
            for placement in tile_placements {
                let placement_vec = vec![*placement];
                let recursed_score = self.get_move_score(&placement_vec, !horizontal, false);
                if recursed_score.is_none() {
                    return None;
                }

                score += recursed_score.unwrap();
            }
        }

        Some(score)
    }

    pub fn get_moves_from_spot_exact_length(
        &self,
        chars: &str,
        row: usize,
        col: usize,
        horizontal: bool,
        exact_tiles_to_play: usize,
    ) -> Vec<PossibleMove> {
        // First check if this spot is actually empty. If it isn't, return early.
        if self.board[row][col] != ' ' {
            return Vec::new();
        }

        // The spot is empty. Iterate backwards from the coords to find the very beginning of what would potentially be our new word.
        // Then, iterate forwards up until we would have potentially placed all our tiles.

        // This represents either the row or the column where our word starts (if we are placing a new word or if we are extending another word).
        let mut candidate_word_start_position: usize = 0;
        let mut possible_tile_placements: Vec<TilePlacement> = Vec::new();
        let mut tiles_remaining = exact_tiles_to_play;
        let mut regex_str = String::from("^");
        if horizontal {
            // Iterate backwards until we find the start of the word.
            for new_col in (0..col).rev() {
                if self.board[row][new_col] == ' ' {
                    break;
                }

                candidate_word_start_position = new_col;
                regex_str.insert(1, self.board[row][new_col]);
            }

            // Then place as many tiles as we are allowed to
            for new_col in col..15 {
                if tiles_remaining == 0 && self.board[row][new_col] == ' ' {
                    break;
                }

                if self.board[row][new_col] == ' ' && tiles_remaining > 0 {
                    regex_str.push_str(r"\w");
                    tiles_remaining -= 1;
                    possible_tile_placements.push(TilePlacement {
                        coords: (row, new_col),
                        tile: ' ',
                    });
                } else {
                    regex_str.push(self.board[row][new_col]);
                }
            }
        } else {
            for new_row in (0..row).rev() {
                if self.board[row][col] == ' ' {
                    break;
                }

                candidate_word_start_position = new_row;
                regex_str.insert(1, self.board[new_row][col]);
            }

            regex_str.push_str(r"\w");
            tiles_remaining -= 1;
            possible_tile_placements.push(TilePlacement {
                coords: (row, col),
                tile: ' ',
            });

            for new_row in row + 1..15 {
                if tiles_remaining == 0 && self.board[new_row][col] == ' ' {
                    break;
                }

                if self.board[new_row][col] == ' ' && tiles_remaining > 0 {
                    regex_str.push_str(r"\w");
                    tiles_remaining -= 1;
                    possible_tile_placements.push(TilePlacement {
                        coords: (new_row, col),
                        tile: ' ',
                    });
                } else {
                    regex_str.push(self.board[new_row][col]);
                }
            }
        }

        // Then, mark the end of the string.
        regex_str.push('$');
        let candidate_regex = Regex::new(&*regex_str).unwrap();

        // Now go find candidates.
        let mut possible_moves: Vec<PossibleMove> = Vec::new();
        let chars_set = char_count_to_map(chars);
        for candidate_word in self
            .valid_words
            .iter()
            .filter(|word| candidate_regex.is_match(word))
        {
            let mut possible_move = PossibleMove {
                tiles: vec![],
                score: 0,
            };

            let mut required_tile_counts: HashMap<char, usize> = HashMap::new();

            // Grab all characters from this word that would need to come from our tiles.
            for potential_tile_placement in possible_tile_placements.iter() {
                let tile_to_place = candidate_word.as_bytes()[if horizontal {
                    potential_tile_placement.coords.1
                } else {
                    potential_tile_placement.coords.0
                } - candidate_word_start_position] as char;
                *required_tile_counts.entry(tile_to_place).or_insert(0) += 1;

                let mut tile_placement_copy = potential_tile_placement.clone();
                tile_placement_copy.tile = tile_to_place;
                possible_move.tiles.push(tile_placement_copy);
            }

            // Valid words that have characters not in the player's char set are disqualified.
            // And, are the letters of this word a subset of the chars argument?
            if required_tile_counts.iter().any(
                |(candidate_word_char, candidate_word_char_count)| {
                    !chars_set.contains_key(candidate_word_char)
                        || candidate_word_char_count > &chars_set[candidate_word_char]
                },
            ) {
                continue;
            }

            let move_score = self.get_move_score(&possible_move.tiles, horizontal, true);
            if move_score.is_none() {
                continue;
            }

            possible_moves.push(possible_move);
        }

        return possible_moves;
    }

    pub fn get_moves_from_spot(
        &self,
        chars: &str,
        row: usize,
        col: usize,
        horizontal: bool,
        max_tiles_to_play: usize,
    ) -> Vec<PossibleMove> {
        let mut result: Vec<PossibleMove> = Vec::new();
        for tiles in 1..=max_tiles_to_play {
            result.append(
                &mut self.get_moves_from_spot_exact_length(chars, row, col, horizontal, tiles),
            );
        }

        return result;
    }

    // pub fn get_moves(&self, chars: &str) -> Vec<PossibleMove> {
    //     let mut result: Vec<PossibleMove> = Vec::new();
    //
    //     // First case: the middle tile is empty, meaning that we can place any word we want there.
    //     if self.board[7][7] == ' ' {
    //         // Because we will only have up to 7 chars, just find all possible horizontal words and duplicate those into vertical words.
    //         for horizontal_word in self.get_moves_from_spot(chars, 7, 7, true, chars.len()) {
    //             result.push(PossibleMove{
    //                 coords: (7, 7),
    //                 horizontal: true,
    //                 chars: horizontal_word.clone(),
    //                 score: self.get_move_score(&*horizontal_word, 7, 7, true)
    //             });
    //
    //             result.push(PossibleMove{
    //                 coords: (7, 7),
    //                 horizontal: false,
    //                 chars: horizontal_word.clone(),
    //                 score: self.get_move_score(&*horizontal_word, 7, 7, false)
    //             });
    //         }
    //
    //         return result;
    //     }
    //
    //     /*  If the middle tile is empty, then new moves must touch previous tiles. Do the following:
    //         1. For horizontal, find all previously placed tiles that do not have a tile directly to the left or right.
    //             For all tiles that do not have one directly to the left, iterate the empty spot leftwards up until 7
    //      */
    // }
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
