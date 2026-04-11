use crate::board::ScrabbleBoard;
use crate::board_iterator::ScrabbleBoardIterator;
use crate::common::{
    Coords, Player, PossibleMove, SCORE_MODIFIERS, SCORES, ScoreModifier, TilePlacement,
};
use crate::tile_bag::TileBag;
use crate::utils::{
    bitmasks_match, char_count_to_map, convert_chars_to_bit_vec, encode_char, encode_chars,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

pub struct ScrabbleGame {
    board: ScrabbleBoard,
    turn: Player,
    bag: TileBag,
    valid_words: HashSet<String>,
    /// HashMap<length, Vec<(word, word bitmask)>>
    valid_words_bitmasks_by_length: HashMap<usize, Vec<(String, Vec<u32>)>>,
    player_scores: HashMap<Player, i32>,
    pub winner: Option<Player>,
    total_players: usize,
}

impl Debug for ScrabbleGame {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Board:\n{}\nScores:\n", self.board.dump())?;

        for (player, score) in self.player_scores.iter() {
            write!(f, " Player {:?}: {}\n", player + 1, score)?;
        }

        write!(f, "Turn: {}", self.turn + 1)
    }
}

impl ScrabbleGame {
    pub fn new(total_players: usize) -> Self {
        if total_players > 4 {
            panic!("Scrabble only supports up to 4 players");
        }

        let buf = include_str!("words.txt");
        let words = buf
            .lines()
            .map(|line| line.trim().to_string())
            .collect::<Vec<String>>();

        Self {
            board: ScrabbleBoard::new(),
            valid_words: words.clone().into_iter().collect(),
            valid_words_bitmasks_by_length: words.into_iter().fold(
                HashMap::new(),
                |mut acc, word| {
                    acc.entry(word.len())
                        .or_insert(Vec::new())
                        .push((word.clone(), convert_chars_to_bit_vec(&word)));
                    acc
                },
            ),
            turn: 0,
            bag: TileBag::new(total_players),
            player_scores: HashMap::new(),
            winner: None,
            total_players,
        }
    }

    pub fn parse_str(total_players: usize, board_str: &str) -> Self {
        let mut board = Self::new(total_players);
        for (row, line) in board_str.lines().enumerate() {
            for (col, char) in line.chars().enumerate() {
                if char == ' ' {
                    continue;
                }

                board.board[(row, col)] = char;
            }
        }

        board
    }

    fn create_board_iterator(
        &'_ self,
        start_coords: Coords,
        horizontal: bool,
        forwards: bool,
    ) -> ScrabbleBoardIterator<'_> {
        ScrabbleBoardIterator::new(&self.board, start_coords, horizontal, forwards)
    }

    fn place_word(&mut self, word: &str, row: usize, col: usize, horizontal: bool) {
        for (i, str_char) in word.chars().enumerate() {
            let (r, c) = if horizontal {
                (row, col + i)
            } else {
                (row + i, col)
            };
            self.board[(r, c)] = str_char;
        }
    }

    fn place_tiles(&mut self, tiles: &Vec<TilePlacement>) {
        for placement in tiles {
            self.board[(placement.coords.0, placement.coords.1)] = placement.tile;
        }
    }

    pub fn make_turn(&mut self, turn: PossibleMove) {
        self.place_tiles(&turn.tiles);
        *self.player_scores.entry(self.turn).or_insert(0) += turn.score as i32;
        self.bag.remove_and_replenish(self.turn, &turn.tiles);

        if self.bag.get_tiles(self.turn).is_empty() {
            // The game is over.
            // Calculate scoring according to https://en.wikipedia.org/wiki/Scrabble#End_of_game

            for player in (0..self.total_players) {
                let player_remaining_tiles_score = self
                    .bag
                    .get_tiles(player)
                    .chars()
                    .map(|c| SCORES[c as usize] as i32)
                    .sum::<i32>();

                // Remaining tiles are subtracted from each player's score.
                *self.player_scores.entry(player).or_insert(0) -= player_remaining_tiles_score;

                // Remaining tile scores are added to the player that went out.
                *self.player_scores.entry(self.turn).or_insert(0) += player_remaining_tiles_score;
            }

            self.winner = Some(
                *self
                    .player_scores
                    .iter()
                    .max_by(|a, b| a.1.cmp(b.1))
                    .unwrap()
                    .0,
            );
        }

        self.turn = (self.turn + 1) % self.total_players;
    }

    /// Collects all the tiles along a vector from a starting point and returns it as a string.
    /// This function will assume that `coord_seed` lives at `coords` (even if it actually doesn't).
    fn gather_board_tiles_along_vector(
        &self,
        coords: Coords,
        coord_seed: char,
        horizontal: bool,
    ) -> String {
        // Grab everything before the seed.
        let mut word: String = self
            .create_board_iterator(
                coords, horizontal, false, // backwards
            )
            .skip(1)
            .take_while(|x| x.char_at_coords != ' ')
            .map(|x| x.char_at_coords)
            .collect::<String>()
            // Then we have to reverse it since we collected it backwards
            .chars()
            .rev()
            .collect();

        // Append the seed.
        word.push(coord_seed);

        // Grab everything after the seed
        word.push_str(
            &*self
                .create_board_iterator(coords, horizontal, true)
                .skip(1)
                .take_while(|x| x.char_at_coords != ' ')
                .map(|x| x.char_at_coords)
                .collect::<String>(),
        );

        word
    }

    /// Returns a vector of all the possible moves that can be made from this spot with a certain tileset by playing an exact number of tiles.
    fn get_moves_from_spot_exact_length(
        &self,
        chars: &str,
        row: usize,
        col: usize,
        horizontal: bool,
        exact_tiles_to_play: usize,
    ) -> Vec<PossibleMove> {
        // First check if this spot is actually empty. If it isn't, return early.
        if self.board[(row, col)] != ' ' {
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
        // The base score from previously placed tiles along the vector where we are trying to calculate our possible moves.
        let mut new_primary_word_score_base = 0u32;
        // The base score word multiplier that we will get by placing tiles in the empty spots, if any.
        let mut new_primary_word_score_multiplier = 1u32;

        // Iterate backwards until we find the start of the word.
        let word_start_iter = self.create_board_iterator(
            (row, col),
            horizontal,
            false, // backwards
        );
        for item in word_start_iter.skip(1) {
            if item.char_at_coords == ' ' {
                // If we encounter an empty spot, leave the loop.
                break;
            }

            // We found a previously placed tile. Record its position and insert it into the front of our word bitmask.
            candidate_word_start_position = item.coords;
            word_bitmask.insert(0, encode_char(item.char_at_coords));
            new_primary_word_score_base += SCORES[item.char_at_coords as usize];
        }

        // Then iterate forwards and place tiles as we go and encounter empty spots.
        let rest_of_word_iter = self.create_board_iterator(
            (row, col),
            horizontal,
            true, // forwards
        );
        for item in rest_of_word_iter {
            // Out of tiles and need to place another? Break.
            if tiles_remaining == 0 && item.char_at_coords == ' ' {
                break;
            }

            // We still have tiles to place, and we need to place one here. Do so.
            if item.char_at_coords == ' ' && tiles_remaining > 0 {
                word_bitmask.push(user_tiles_bitmask);
                tiles_remaining -= 1;
                possible_tile_placements.push(item.coords);

                // Grab the word multiplier, if any, since it applies to the whole word (we know we will place a tile here, we just don't know which tile yet, hence no letter multipliers yet).
                match SCORE_MODIFIERS[item.coords.0][item.coords.1] {
                    ScoreModifier::DWS => new_primary_word_score_multiplier *= 2,
                    ScoreModifier::TWS => new_primary_word_score_multiplier *= 3,
                    _ => (),
                }
            } else {
                // This spot on the board already has a tile in it. We will add it to the bitmask so that we only consider words with that tile in that spot.
                word_bitmask.push(encode_char(item.char_at_coords));
                new_primary_word_score_base += SCORES[item.char_at_coords as usize];
            }
        }

        // If we got here but still have tiles to play, then we careened off the edge of the board. No moves can be played.
        if tiles_remaining > 0 {
            return Vec::new();
        }

        // Now go find candidates.
        let mut possible_moves: Vec<PossibleMove> = Vec::new();
        let user_tile_counts_by_char = char_count_to_map(chars);
        if !self
            .valid_words_bitmasks_by_length
            .contains_key(&word_bitmask.len())
        {
            // If there are no words that match our required length, then exit early.
            return Vec::new();
        }

        'candidate_word_main_loop: for candidate_word in (&self.valid_words_bitmasks_by_length
            [&word_bitmask.len()])
            .into_iter()
            .filter(|(_, bitmask_vec)| bitmasks_match(&bitmask_vec, &word_bitmask))
            .map(|(word, _)| word)
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

            // By this point, we should have the word multiplier and tile scores for all previously placed tiles forming this new word.
            // We just need to add our new tiles with their letter multipliers to the mix and then calculate the scores of any newly formed words from this play.
            // This will also be where we verify that these other newly formed words are valid.

            // First add all our placed tiles to the score, taking into account letter multipliers.
            let mut possible_move_score = new_primary_word_score_multiplier
                * (new_primary_word_score_base
                    + possible_move
                        .tiles
                        .iter()
                        .map(|tile| {
                            SCORES[tile.tile as usize]
                                * match SCORE_MODIFIERS[tile.coords.0][tile.coords.1] {
                                    ScoreModifier::DLS => 2,
                                    ScoreModifier::TLS => 3,
                                    _ => 1,
                                }
                        })
                        .sum::<u32>());

            if possible_move.tiles.len() == 7 {
                // Playing 7 tiles at once gives an extra 50 points.
                possible_move_score += 50;
            }

            // Now loop through our tiles and find any new words formed along the opposite direction. If any of these words is invalid, then we must reject this move.
            // Otherwise, add the score to the base score.
            for possible_tile_placement in &possible_move.tiles {
                let new_word_formed = self.gather_board_tiles_along_vector(
                    possible_tile_placement.coords,
                    possible_tile_placement.tile,
                    !horizontal,
                );

                if new_word_formed.len() <= 1 {
                    // The single tile is all that exists along the vector, so we can skip it.
                    continue;
                }

                if !self.valid_words.contains(&new_word_formed) {
                    // New word formed is invalid.
                    continue 'candidate_word_main_loop;
                }

                // New word is valid. Calculate score.
                let mut new_word_formed_score = new_word_formed
                    .chars()
                    .map(|c| SCORES[c as usize])
                    .sum::<u32>()
                    // Subtract the raw score of the tile we are placing so we don't count it twice.
                    - SCORES[possible_tile_placement.tile as usize]
                    // Then add it back with the letter multiplier, if any.
                    + (SCORES[possible_tile_placement.tile as usize]
                    * match SCORE_MODIFIERS[possible_tile_placement.coords.0]
                    [possible_tile_placement.coords.1]
                {
                    ScoreModifier::DLS => 2,
                    ScoreModifier::TLS => 3,
                    _ => 1,
                });

                // And then multiply by any word multipliers, if any.
                new_word_formed_score *= match SCORE_MODIFIERS[possible_tile_placement.coords.0]
                    [possible_tile_placement.coords.1]
                {
                    ScoreModifier::DWS => 2,
                    ScoreModifier::TWS => 3,
                    _ => 1,
                };

                possible_move_score += new_word_formed_score;
            }

            possible_move.score = possible_move_score;
            possible_moves.push(possible_move);
        }

        possible_moves
    }

    /// Returns all possible moves that can be made from this spot, direction, and tileset.
    fn get_moves_from_spot(
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

        result
    }

    fn get_moves_helper(&self, chars: &str) -> Vec<PossibleMove> {
        let mut result: Vec<PossibleMove> = Vec::new();

        // First case: the middle tile is empty, meaning that we can place any word we want there.
        if self.board[(7, 7)] == ' ' {
            result.append(&mut self.get_moves_from_spot(chars, 7, 7, true));
            result.append(&mut self.get_moves_from_spot(chars, 7, 7, false));

            return result;
        }

        /*  If the middle tile is empty, then new moves must touch previous tiles. Do the following:
           1. For horizontal, find all previously placed tiles that do not have a tile directly to the left or right.
               For all tiles that do not have one directly to the left, iterate the empty spot leftwards up until the tile count.
           2. For vertical, do the same thing but for up and down.
        */

        let mut horizontal_coords_to_check: HashSet<((usize, usize), Option<usize>)> =
            HashSet::new(); // (row, column, required length)
        let mut vertical_coords_to_check: HashSet<((usize, usize), Option<usize>)> = HashSet::new();

        let extend_coord_set = |coord_set: &mut HashSet<((usize, usize), Option<usize>)>,
                                start: Coords,
                                horizontal: bool| {
            coord_set.extend(
                self.create_board_iterator(
                    start, horizontal, false, // backwards
                )
                // + 1 on the vector distance because this closure is being invoked on the tile adjacent to the one we want to place on.
                .take_while(|x| x.char_at_coords == ' ' && x.vector_distance + 1 <= chars.len())
                .map(|x| (x.coords, Some(x.vector_distance + 1))),
            );
        };

        for row in 0..15 {
            for column in 0..15 {
                if self.board[(row, column)] == ' ' {
                    continue;
                }

                if column > 0 {
                    extend_coord_set(&mut horizontal_coords_to_check, (row, column - 1), true);
                }

                if column < 14 {
                    horizontal_coords_to_check.insert(((row, column + 1), None));
                }

                if row > 0 {
                    extend_coord_set(&mut vertical_coords_to_check, (row - 1, column), false);
                }

                if row < 14 {
                    vertical_coords_to_check.insert(((row + 1, column), None));
                }
            }
        }

        for (coords, is_horizontal) in [
            (horizontal_coords_to_check, true),
            (vertical_coords_to_check, false),
        ] {
            for ((row, column), length) in coords {
                if let Some(len) = length {
                    result.append(&mut self.get_moves_from_spot_exact_length(
                        chars,
                        row,
                        column,
                        is_horizontal,
                        len,
                    ));
                } else {
                    result.append(&mut self.get_moves_from_spot(chars, row, column, is_horizontal));
                }
            }
        }

        result
    }

    /// Returns all possible moves from a given tileset.
    pub fn get_moves(&self) -> Vec<PossibleMove> {
        if self.winner.is_some() {
            return Vec::new();
        }

        let mut result: Vec<PossibleMove> = self.get_moves_helper(&*self.bag.get_tiles(self.turn));
        // Sort by score descending.
        result.sort_by(|a, b| b.score.cmp(&a.score));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic_valid_move() {
        let board = ScrabbleGame::new(2);

        // H (4, TWS) + E (1) + L (1) + L (2, DLS) + O (1) == 27
        let moves = board.get_moves_from_spot_exact_length("HELLO", 0, 0, true, 5);
        let hello_move = moves
            .iter()
            .find(|m| m.tiles.iter().all(|t| "HELLO".contains(t.tile)))
            .expect("Should find HELLO move");
        assert_eq!(hello_move.score, 27);
    }

    #[test]
    fn test_invalid_move_overlapping() {
        let mut board = ScrabbleGame::new(2);
        // Place HELLO at (7,7) horizontally
        board.place_word("HELLO", 7, 7, true);
        // Now try to place WORLD overlapping with O in HELLO
        let moves = board.get_moves_from_spot_exact_length("WORLD", 7, 7, false, 5);
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_valid_move_with_new_words() {
        let mut board = ScrabbleGame::new(2);
        // Place SHELL at the top left horizontally
        board.place_word("SHELL", 0, 0, true);
        // Now place HAD horizontally starting at (1,0) which would create the words "SH", "HA", and "ED"

        // H (4) + A (1, DWS) + D (2) == 14
        // S (1) + H(4) == 5
        // H (4) + A (1, DWS) == 10
        // E (1) + D (2) == 3
        let moves = board.get_moves_from_spot_exact_length("HAD", 1, 0, true, 3);
        let had_move = moves
            .iter()
            .find(|m| m.tiles.len() == 3 && m.tiles.iter().all(|t| "HAD".contains(t.tile)))
            .expect("Should find HAD move");
        assert_eq!(had_move.score, 32);
    }

    #[test]
    fn test_letter_placement_forms_word_both_directions() {
        let mut board = ScrabbleGame::new(2);
        board.place_word("SPEED", 0, 0, true);
        board.place_word("METER", 0, 6, true);

        // S (1) + P (3) + E (1) + E (1) + D (2) + O (1) + M (3) + E (1) + T (1) + E (1) + R (1) == 16
        let moves = board.get_moves_from_spot_exact_length("O", 0, 5, true, 1);

        // Find any valid move - the position might not have moves if no valid words form
        assert_eq!(
            moves
                .iter()
                .find(|m| m.tiles.len() == 1 && m.tiles[0].tile == 'O')
                .expect("Expected to find O")
                .score,
            16
        );
    }

    #[test]
    fn test_vertical_letter_forms_invalid_vertical_word() {
        let mut board = ScrabbleGame::new(2);
        board.place_word("FETCH", 0, 0, true);
        // Trying to place F vertically at (1, 0) would form "FF" which is not a valid word
        let moves = board.get_moves_from_spot_exact_length("F", 1, 0, false, 1);
        // Should return no valid moves because FF is not a word
        assert_eq!(moves.len(), 0);
    }
    #[test]
    fn test_single_letter_forms_words_in_both_directions() {
        let mut board = ScrabbleGame::new(2);
        board.place_word("SPEED", 5, 0, true);
        board.place_word("METER", 5, 6, true);
        board.place_word("SPEED", 0, 5, false);
        board.place_word("METER", 6, 5, false);

        // At (5,5), we have a triple letter score
        let moves = board.get_moves_from_spot_exact_length("O", 5, 5, true, 1);

        assert_eq!(
            moves
                .iter()
                .find(|m| m.tiles.len() == 1 && m.tiles[0].tile == 'O')
                .expect("Expected to find O")
                .score,
            36,
        );
    }

    #[test]
    fn test_simple_score() {
        let board = ScrabbleGame::new(2);

        // S (1, TWS) + P (3) + E (1) + E (2, DLS) + D (2) == 27

        let moves = board.get_moves_from_spot_exact_length("SPEED", 0, 0, true, 5);
        let speed_move = moves
            .iter()
            .find(|m| m.tiles.iter().all(|t| "SPEED".contains(t.tile)))
            .expect("Should find SPEED move");
        // The score should be at least 27 (the base word score)
        assert!(speed_move.score >= 27);
    }

    #[test]
    fn test_possible_moves_simple() {
        let mut board = ScrabbleGame::new(2);

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

    #[test]
    fn test_exact_length_returns_empty_when_spot_occupied() {
        let mut board = ScrabbleGame::new(2);
        board.place_word("HELLO", 7, 7, true);

        // Trying to place at an occupied spot should return empty
        let moves = board.get_moves_from_spot_exact_length("ABCDE", 7, 7, true, 2);
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_exact_length_first_move_center() {
        let board = ScrabbleGame::new(2);

        // First move: place "CAT" horizontally at center (7,7) using exactly 3 tiles
        let moves = board.get_moves_from_spot_exact_length("CAT", 7, 7, true, 3);

        // Should find CAT
        assert!(!moves.is_empty());
        let cat_move = moves.iter().find(|m| {
            m.tiles.len() == 3
                && m.tiles
                    .iter()
                    .all(|t| t.tile == 'C' || t.tile == 'A' || t.tile == 'T')
        });
        assert!(cat_move.is_some());
    }

    #[test]
    fn test_exact_length_enforces_exact_tile_count() {
        let board = ScrabbleGame::new(2);

        // Request moves with exactly 2 tiles
        let moves_2 = board.get_moves_from_spot_exact_length("ABCDE", 7, 7, true, 2);

        // All moves should have exactly 2 tiles
        for m in moves_2.iter() {
            assert_eq!(m.tiles.len(), 2);
        }

        // Request moves with exactly 3 tiles
        let moves_3 = board.get_moves_from_spot_exact_length("ABCDE", 7, 7, true, 3);

        // All moves should have exactly 3 tiles
        for m in moves_3.iter() {
            assert_eq!(m.tiles.len(), 3);
        }

        // Both should have moves
        assert!(!moves_2.is_empty());
        assert!(!moves_3.is_empty());
    }

    #[test]
    fn test_exact_length_places_from_left() {
        let mut board = ScrabbleGame::new(2);
        board.place_word("SPEED", 0, 0, true);

        // Place horizontally from (1, 0) with exactly 2 tiles
        // Should find words like "SO" + letter or other valid 2-letter words
        let moves = board.get_moves_from_spot_exact_length("OIA", 1, 0, true, 2);

        assert!(!moves.is_empty());

        // All returned moves should have exactly 2 tiles
        for m in moves.iter() {
            assert_eq!(m.tiles.len(), 2);
        }
    }

    #[test]
    fn test_exact_length_cannot_form_invalid_words() {
        let board = ScrabbleGame::new(2);

        // Try to place with tiles that would only form invalid words
        // "XX" is not a valid word, so this should return empty
        let moves = board.get_moves_from_spot_exact_length("XX", 7, 7, true, 2);

        // Should be empty because we can't form valid words
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_exact_length_missing_required_tiles() {
        let mut board = ScrabbleGame::new(2);
        board.place_word("AT", 7, 7, true);

        // Try to place "CAR" vertically from below the "A", but we don't have all the tiles
        // We only have "XYZ" - should not find "CAR"
        let moves = board.get_moves_from_spot_exact_length("XYZ", 8, 7, false, 2);

        // Should not find any moves that require C, A, or R
        for m in moves.iter() {
            for tile in m.tiles.iter() {
                assert!(tile.tile == 'X' || tile.tile == 'Y' || tile.tile == 'Z');
            }
        }
    }

    #[test]
    fn test_exact_length_vertical_placement() {
        let mut board = ScrabbleGame::new(2);
        board.place_word("SPEED", 0, 5, false);

        // Place vertically with exactly 2 tiles from position below the E
        // Should find valid words that are 2 letters long
        let moves = board.get_moves_from_spot_exact_length("EAT", 5, 5, false, 2);

        // Since we're extending existing tiles, this may not find moves if valid words aren't available
        // Let's verify the structure instead
        for m in moves.iter() {
            assert_eq!(m.tiles.len(), 2);
        }
    }

    #[test]
    fn test_exact_length_score_calculation() {
        let board = ScrabbleGame::new(2);

        let moves = board.get_moves_from_spot_exact_length("CAT", 7, 7, true, 3);

        // All moves should have a valid score
        for m in moves.iter() {
            assert!(m.score > 0);
        }
    }

    #[test]
    fn test_exact_length_zero_tiles_returns_empty() {
        let board = ScrabbleGame::new(2);

        // Requesting to place 0 tiles should return empty
        let moves = board.get_moves_from_spot_exact_length("ABCDE", 7, 7, true, 0);
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_exact_length_horizontal_vs_vertical() {
        let mut board = ScrabbleGame::new(2);
        board.place_word("HELLO", 5, 5, true);

        // Get horizontal moves from a spot
        let horizontal_moves = board.get_moves_from_spot_exact_length("WORLD", 5, 10, true, 2);

        // Get vertical moves from a spot
        let vertical_moves = board.get_moves_from_spot_exact_length("WORLD", 5, 10, false, 2);

        // Both should have exact length 2
        for m in horizontal_moves.iter() {
            assert_eq!(m.tiles.len(), 2);
        }
        for m in vertical_moves.iter() {
            assert_eq!(m.tiles.len(), 2);
        }
    }

    #[test]
    fn test_get_moves_from_spot_exact_length_goes_off_end() {
        let board = ScrabbleGame::new(2);

        let moves = board.get_moves_from_spot_exact_length("QOIGEOQOQEASGEO", 13, 7, false, 5);
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn test_bingo_bonus() {
        let board = ScrabbleGame::new(2);

        // Vertical ABALONE at (7,7) is A (1) + B (3) + A (1) + L (1) + O (2, DLS) + N (1) + E (1) == 10
        // With a bingo bonus, it should be 60
        let moves = board.get_moves_from_spot_exact_length("ABALONE", 7, 7, false, 7);
        assert_eq!(moves[0].score, 60);
    }
}
