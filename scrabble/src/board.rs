use crate::utils::{char_count_to_map, encode_char, encode_chars, word_matches_bitmask};
use regex::Regex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;

pub struct ScrabbleGame {
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

type Coords = (usize, usize);

#[derive(Clone, Copy)]
pub struct TilePlacement {
    coords: Coords,
    pub tile: char,
}

impl Debug for TilePlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.coords, self.tile)
    }
}

#[derive(Clone, Debug)]
pub struct PossibleMove {
    pub tiles: Vec<TilePlacement>,
    pub score: u32,
}

type ScrabbleBoard = [[char; 15]; 15];

struct ScrabbleBoardIterator<'a> {
    horizontal: bool,
    forwards: bool,
    board: &'a ScrabbleBoard,

    // So that we actually iterate over everything, we use i32 so that we can see if we go negative when going backwards.
    current_coords: (i32, i32),
}
impl Iterator for ScrabbleBoardIterator<'_> {
    type Item = (Coords, char);

    fn next(&mut self) -> Option<Self::Item> {
        let current_coords = (
            self.current_coords.0 as usize,
            self.current_coords.1 as usize,
        );
        let current_char = self.board[current_coords.0][current_coords.1];

        // Don't return None until we have gone past the end in either direction.

        if self.horizontal {
            if self.forwards {
                if self.current_coords.1 > 14 {
                    return None;
                }

                self.current_coords.1 += 1;
            } else {
                if self.current_coords.1 < 0 {
                    return None;
                }

                self.current_coords.1 -= 1;
            }
        } else {
            if self.forwards {
                if self.current_coords.0 > 14 {
                    return None;
                }

                self.current_coords.0 += 1;
            } else {
                if self.current_coords.0 < 0 {
                    return None;
                }

                self.current_coords.0 -= 1;
            }
        }

        Some((current_coords, current_char))
    }
}

impl ScrabbleGame {
    pub fn new() -> Self {
        let buf = include_str!("words.txt");
        let words = buf
            .lines()
            .map(|line| line.trim().to_string())
            .collect::<Vec<String>>();

        Self {
            board: [[' '; 15]; 15],
            valid_words: words.into_iter().collect(),
        }
    }

    fn create_board_iterator(
        &'_ self,
        start_coords: (usize, usize),
        horizontal: bool,
        forwards: bool,
    ) -> ScrabbleBoardIterator<'_> {
        ScrabbleBoardIterator {
            forwards,
            current_coords: (start_coords.0 as i32, start_coords.1 as i32),
            horizontal,
            board: &self.board,
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

    pub fn dump(&self) -> String {
        let mut buf = String::new();
        for row in self.board {
            buf.push_str(&*format!("{:?}\n", row));
        }
        return buf;
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

    pub fn place_tiles(&mut self, tiles: &Vec<TilePlacement>) {
        for placement in tiles {
            self.board[placement.coords.0][placement.coords.1] = placement.tile;
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
                if self.board[row][new_col] == ' ' && tiles_placed >= tile_placements.len() {
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
                if self.board[new_row][col] == ' ' && tiles_placed >= tile_placements.len() {
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

        // Playing 7 tiles all at once is an extra 50 points
        if tile_placements.len() == 7 {
            return Some(score + 50);
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
        let mut candidate_word_start_position: Coords = (row, col);
        // Coordinates where we would have to place tiles for our turn.
        let mut possible_tile_placements: Vec<Coords> = Vec::new();
        // Temporary to keep track of how many empty spots we have seen as we iterate along the board.
        let mut tiles_remaining = exact_tiles_to_play;
        // A vector of encoded char bitmasks to help in filtering the valid word list given some constraints (already placed tiles we encounter, the player's tile set, etc.).
        let mut word_bitmask: Vec<u32> = Vec::new();
        // The user's tiles encoded as a bitwise OR mask.
        let user_tiles_bitmask = encode_chars(chars);

        // Iterate backwards until we find the start of the word.
        let word_start_iter = self.create_board_iterator(
            (row, col),
            horizontal,
            false, // backwards
        );
        for (new_coords, c) in word_start_iter {
            if c == ' ' {
                // If we encounter an empty spot, leave the loop.
                break;
            }

            // We found a previously placed tile. Record its position and insert it into the front of our word bitmask.
            candidate_word_start_position = new_coords;
            word_bitmask.insert(0, encode_char(c));
        }

        // Then iterate forwards and place tiles as we go and encounter empty spots.
        let rest_of_word_iter = self.create_board_iterator(
            (row, col),
            horizontal,
            true, // forwards
        );

        for (new_coords, c) in rest_of_word_iter {
            // Out of tiles and need to place another? Break.
            if tiles_remaining == 0 && c == ' ' {
                break;
            }

            // We still have tiles to place, and we need to place one here. Do so.
            if c == ' ' && tiles_remaining > 0 {
                word_bitmask.push(user_tiles_bitmask);
                tiles_remaining -= 1;
                possible_tile_placements.push(new_coords);
            } else {
                // This spot on the board already has a tile in it. We will add it to the bitmask so that we only consider words with that tile in that spot.
                word_bitmask.push(encode_char(c));
            }
        }

        // Now go find candidates.
        let mut possible_moves: Vec<PossibleMove> = Vec::new();
        let user_tile_counts_by_char = char_count_to_map(chars);
        for candidate_word in self
            .valid_words
            .iter()
            .filter(|word| word_matches_bitmask(word, &word_bitmask))
        {
            let mut possible_move = PossibleMove {
                tiles: vec![],
                score: 0,
            };

            let mut required_tile_counts: HashMap<char, usize> = HashMap::new();

            // Grab all characters from this word that would need to come from our tiles.
            for potential_tile_placement in possible_tile_placements.iter() {
                let tile_to_place = candidate_word.as_bytes()[if horizontal {
                    potential_tile_placement.1 - candidate_word_start_position.1
                } else {
                    potential_tile_placement.0 - candidate_word_start_position.0
                }] as char;
                *required_tile_counts.entry(tile_to_place).or_insert(0) += 1;

                possible_move.tiles.push(TilePlacement {
                    coords: *potential_tile_placement,
                    tile: tile_to_place,
                });
            }

            // Loop through the required tiles in order to form this new word.
            if required_tile_counts.iter().any(
                |(candidate_word_char, candidate_word_char_count)| {
                    // Are there any that the player doesn't have in their set?
                    !user_tile_counts_by_char.contains_key(candidate_word_char)
                        // Or are there any that the user doesn't have enough of?
                        || candidate_word_char_count > &user_tile_counts_by_char[candidate_word_char]
                },
            ) {
                // If so, this word can't be played with the players tiles.
                continue;
            }

            // If we got here, then the user can play the word. Now we will go calculate the score for this play.
            let move_score = self.get_move_score(&possible_move.tiles, horizontal, true);
            if move_score.is_none() {
                // If we didn't get a score, then there is an invalid word formed by one of the player's tile placements.
                // TODO: Refactor scoring into this method.
                continue;
            }

            // We have a score. The play is valid.
            possible_move.score = move_score.unwrap();

            possible_moves.push(possible_move);
        }

        possible_moves
    }

    pub fn get_moves_from_spot(
        &self,
        chars: &str,
        row: usize,
        col: usize,
        horizontal: bool,
    ) -> Vec<PossibleMove> {
        let mut result: Vec<PossibleMove> = Vec::new();
        for tiles in 1..=chars.len() {
            result.append(
                &mut self.get_moves_from_spot_exact_length(chars, row, col, horizontal, tiles),
            );
        }

        return result;
    }

    fn get_moves_helper(&self, chars: &str) -> Vec<PossibleMove> {
        let mut result: Vec<PossibleMove> = Vec::new();

        // First case: the middle tile is empty, meaning that we can place any word we want there.
        if self.board[7][7] == ' ' {
            result.append(&mut self.get_moves_from_spot(chars, 7, 7, true));
            result.append(&mut self.get_moves_from_spot(chars, 7, 7, false));

            return result;
        }

        /*  If the middle tile is empty, then new moves must touch previous tiles. Do the following:
           1. For horizontal, find all previously placed tiles that do not have a tile directly to the left or right.
               For all tiles that do not have one directly to the left, iterate the empty spot leftwards up until the tile count.
           2. For vertical, do the same thing but for up and down.
        */

        // First, horizontal.
        let mut horizontal_coords_to_check: HashSet<(usize, usize, Option<usize>)> = HashSet::new(); // (row, column, required length)
        for row in 0..15 {
            for column in 1..14 {
                if self.board[row][column] == ' ' {
                    continue;
                }

                // Iterate to the left
                for column_to_left in (0..column - 1).rev() {
                    if self.board[row][column_to_left] != ' '
                        || column - column_to_left > chars.len()
                    {
                        break;
                    }

                    horizontal_coords_to_check.insert((
                        row,
                        column_to_left,
                        Some(column - column_to_left),
                    ));
                }

                // Grab one to the right
                if self.board[row][column + 1] == ' ' {
                    horizontal_coords_to_check.insert((row, column + 1, None));
                }
            }
        }

        // Next, vertical.
        let mut vertical_coords_to_check: HashSet<(usize, usize, Option<usize>)> = HashSet::new();
        for row in 1..14 {
            for column in 0..15 {
                if self.board[row][column] == ' ' {
                    continue;
                }

                // Iterate upwards
                for row_above in (0..row - 1).rev() {
                    if self.board[row_above][column] != ' ' || row - row_above > chars.len() {
                        break;
                    }

                    vertical_coords_to_check.insert((row_above, column, Some(row - row_above)));
                }

                // Grab one downward
                if self.board[row + 1][column] == ' ' {
                    vertical_coords_to_check.insert((row + 1, column, None));
                }
            }
        }

        for (row, column, length) in horizontal_coords_to_check {
            if length.is_some() {
                result.append(&mut self.get_moves_from_spot_exact_length(
                    chars,
                    row,
                    column,
                    true,
                    length.unwrap(),
                ));
            } else {
                result.append(&mut self.get_moves_from_spot(chars, row, column, true));
            }
        }

        for (row, column, length) in vertical_coords_to_check {
            if length.is_some() {
                result.append(&mut self.get_moves_from_spot_exact_length(
                    chars,
                    row,
                    column,
                    false,
                    length.unwrap(),
                ));
            } else {
                result.append(&mut self.get_moves_from_spot(chars, row, column, false));
            }
        }

        return result;
    }

    pub fn get_moves(&self, chars: &str) -> Vec<PossibleMove> {
        let mut result: Vec<PossibleMove> = self.get_moves_helper(chars);
        // Sort by score descending.
        result.sort_by(|a, b| b.score.cmp(&a.score));
        result
    }
}

#[cfg(test)]
mod tests {
    fn to_tiles(word: &str, row: usize, col: usize, horizontal: bool) -> Vec<TilePlacement> {
        let mut result: Vec<TilePlacement> = Vec::new();
        for (i, str_char) in word.chars().enumerate() {
            let (new_row, new_col) = if horizontal {
                (row, col + i)
            } else {
                (row + i, col)
            };
            result.push(TilePlacement {
                tile: str_char,
                coords: (new_row, new_col),
            });
        }

        return result;
    }
    use super::*;
    #[test]
    fn test_basic_valid_move() {
        let board = ScrabbleGame::new();

        // H (4, TWS) + E (1) + L (1) + L (2, DLS) + O (1) == 27
        assert_eq!(
            board
                .get_move_score(&to_tiles("HELLO", 0, 0, true), true, true)
                .unwrap(),
            27
        );
    }

    #[test]
    fn test_invalid_move_out_of_bounds() {
        let board = ScrabbleGame::new();
        assert!(
            board
                .get_move_score(&to_tiles("HELLO", 0, 12, true), true, true)
                .is_none()
        );
    }

    #[test]
    fn test_invalid_move_overlapping() {
        let mut board = ScrabbleGame::new();
        // Place HELLO at (7,7) horizontally
        board.place_word("HELLO", 7, 7, true);
        // Now try to place WORLD overlapping with O in HELLO
        assert!(
            !board
                .get_move_score(&to_tiles("WORLD", 5, 5, false), false, true)
                .is_none()
        );
    }

    #[test]
    fn test_valid_move_with_new_words() {
        let mut board = ScrabbleGame::new();
        // Place SCRAP at the top left horizontally
        board.place_word("SHELL", 0, 0, true);
        // Now place HAD vertically starting at (0,2) which would create the words "SH", "HA", and "ED"

        // H (4) + A (1, DWS) + D (2) == 14
        // S (1) + H(4) == 5
        // H (4) + A (1, DWS) == 10
        // E (1) + D (2) == 3
        assert_eq!(
            board
                .get_move_score(&to_tiles("HAD", 1, 0, true), true, true)
                .unwrap(),
            32
        );
    }

    #[test]
    fn test_letter_placement_forms_word_both_directions() {
        let mut board = ScrabbleGame::new();
        board.place_word("SPEED", 0, 0, true);
        board.place_word("METER", 0, 6, true);

        // S (1) + P (3) + E (1) + E (1) + D (2) + O (1) + M (3) + E (1) + T (1) + E (1) + R (1) == 16
        assert_eq!(
            board
                .get_move_score(&to_tiles("O", 0, 5, true), true, true)
                .unwrap(),
            16
        );
    }

    #[test]
    fn test_vertical_letter_forms_invalid_vertical_word() {
        let mut board = ScrabbleGame::new();
        board.place_word("FETCH", 0, 0, true);
        assert!(
            board
                .get_move_score(&to_tiles("F", 1, 0, false), false, true)
                .is_none()
        );
    }
    #[test]
    fn test_single_letter_forms_words_in_both_directions() {
        let mut board = ScrabbleGame::new();
        board.place_word("SPEED", 5, 0, true);
        board.place_word("METER", 5, 6, true);
        board.place_word("SPEED", 0, 5, false);
        board.place_word("METER", 6, 5, false);

        // TLS twice on the O == 18 for both == 36
        assert_eq!(
            board
                .get_move_score(&to_tiles("O", 5, 5, true), true, true)
                .unwrap(),
            36
        );
    }

    #[test]
    fn test_simple_score() {
        let board = ScrabbleGame::new();

        // S (1, TWS) + P (3) + E (1) + E (2, DLS) + D (2) == 27
        assert_eq!(
            board
                .get_move_score(&to_tiles("SPEED", 0, 0, true), true, true)
                .unwrap(),
            27
        );
    }

    #[test]
    fn test_possible_moves_simple() {
        let mut board = ScrabbleGame::new();

        board.place_word("SPEED", 0, 0, true);
        let possible_words = board.get_moves_from_spot("AFOOI", 1, 0, true);

        // Two words from (1, 0) horizontally: OI and OAF.

        let short_word = possible_words.iter().find(|x| x.tiles.len() == 2).unwrap();
        // First, OI which also forms SO and PI.
        // O (1) + I (1, DWS) == 4
        // S (1) + O (1) == 2
        // P (3) + I (1, DWS) == 8
        // Total == 14
        assert_eq!(short_word.tiles.len(), 2);
        assert_eq!(short_word.tiles[0].coords, (1, 0));
        assert_eq!(short_word.tiles[0].tile, 'O');
        assert_eq!(short_word.tiles[1].coords, (1, 1));
        assert_eq!(short_word.tiles[1].tile, 'I');
        assert_eq!(short_word.score, 14);

        let long_word = possible_words.iter().find(|x| x.tiles.len() == 3).unwrap();
        // Then, OAF which also forms SO, PA, and EF.
        // O (1) + A (1, DWS) + F (4) == 12
        // S (1) + O (1) == 2
        // P (3) + A (1, DWS) == 8
        // E (1) + F (4) == 5
        // Total == 27
        assert_eq!(long_word.tiles.len(), 3);
        assert_eq!(long_word.tiles[0].coords, (1, 0));
        assert_eq!(long_word.tiles[0].tile, 'O');
        assert_eq!(long_word.tiles[1].coords, (1, 1));
        assert_eq!(long_word.tiles[1].tile, 'A');
        assert_eq!(long_word.tiles[2].coords, (1, 2));
        assert_eq!(long_word.tiles[2].tile, 'F');
        assert_eq!(long_word.score, 27);
    }
}
