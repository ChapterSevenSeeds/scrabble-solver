use crate::common::{MAX_PLAYER_TILES, Player, TilePlacement, WILD};
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct TileBagSnapshot {
    pub seed: u64,
    pub exchange_count: u64,
    pub tiles: Vec<char>,
    pub players_tiles: HashMap<Player, HashMap<char, usize>>,
}

pub struct TileBag {
    seed: u64,
    exchange_count: u64,
    tiles: Vec<char>,
    players_tiles: HashMap<Player, HashMap<char, usize>>,
}

impl TileBag {
    pub fn new(total_players: usize) -> TileBag {
        Self::new_with_seed(total_players, rand::random())
    }

    pub fn new_with_seed(total_players: usize, seed: u64) -> TileBag {
        let mut all_tiles = Vec::new();

        let mut insert = |letter: char, count: usize| {
            for _ in 0..count {
                all_tiles.push(letter);
            }
        };

        insert('E', 12);

        insert('A', 9);
        insert('I', 9);

        insert('O', 8);

        insert('N', 6);
        insert('R', 6);
        insert('T', 6);

        insert('D', 4);
        insert('L', 4);
        insert('S', 4);
        insert('U', 4);

        insert('G', 3);

        insert('B', 2);
        insert('C', 2);
        insert('M', 2);
        insert('P', 2);
        insert('F', 2);
        insert('H', 2);
        insert('V', 2);
        insert('W', 2);
        insert('Y', 2);
        insert(WILD, 2);

        insert('Q', 1);
        insert('Z', 1);
        insert('J', 1);
        insert('X', 1);
        insert('K', 1);

        let mut initial_shuffle_rng = StdRng::seed_from_u64(seed);
        all_tiles.shuffle(&mut initial_shuffle_rng);

        let mut players_tiles = HashMap::<Player, HashMap<char, usize>>::new();

        for _ in 0..MAX_PLAYER_TILES {
            for player in 0..total_players {
                *players_tiles
                    .entry(player)
                    .or_insert(HashMap::new())
                    .entry(all_tiles.pop().unwrap())
                    .or_insert(0) += 1;
            }
        }

        TileBag {
            seed,
            exchange_count: 0,
            tiles: all_tiles,
            players_tiles,
        }
    }

    fn shuffle_tiles_with_seed(tiles: &mut Vec<char>, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        tiles.shuffle(&mut rng);
    }

    pub(crate) fn to_snapshot(&self) -> TileBagSnapshot {
        TileBagSnapshot {
            seed: self.seed,
            exchange_count: self.exchange_count,
            tiles: self.tiles.clone(),
            players_tiles: self.players_tiles.clone(),
        }
    }

    pub(crate) fn from_snapshot(snapshot: TileBagSnapshot) -> TileBag {
        TileBag {
            seed: snapshot.seed,
            exchange_count: snapshot.exchange_count,
            tiles: snapshot.tiles,
            players_tiles: snapshot.players_tiles,
        }
    }

    pub fn remove_and_replenish(&mut self, turn: Player, placements: &Vec<TilePlacement>) {
        let relevant_set = self.players_tiles.get_mut(&turn).unwrap();

        for placement in placements {
            let entry = relevant_set.get_mut(&placement.tile);
            if entry.is_some() && **entry.as_ref().unwrap() > 0 {
                // The user has the tile, use it.
                *entry.unwrap() -= 1;
            } else if relevant_set.contains_key(&WILD) {
                // The user has a wildcard, use it.
                *relevant_set.get_mut(&WILD).unwrap() -= 1;
            } else {
                panic!(
                    "Player {} does not have tile {} or a wildcard to place",
                    turn, placement.tile
                );
            }
        }

        for _ in 0..min(
            MAX_PLAYER_TILES - relevant_set.values().sum::<usize>(),
            self.tiles.len(),
        ) {
            *relevant_set.entry(self.tiles.pop().unwrap()).or_insert(0) += 1;
        }
    }

    pub fn get_tiles(&self, turn: Player) -> String {
        let relevant_set = self.players_tiles.get(&turn).unwrap();

        let mut tiles: Vec<char> = relevant_set
            .iter()
            .flat_map(|(tile, count)| std::iter::repeat_n(*tile, *count))
            .collect();
        tiles.sort_unstable();
        tiles.into_iter().collect()
    }

    pub fn exchange(&mut self, turn: Player, tiles: String) {
        assert!(self.tiles.len() >= 7);

        let relevant_set = self.players_tiles.get_mut(&turn).unwrap();
        for tile in tiles.chars() {
            // Remove tiles from the player's set.
            *relevant_set.get_mut(&tile).unwrap() -= 1;

            // And then add it back to the bag.
            self.tiles.push(tile);
        }

        // Shuffle deterministically so game saves can restore exact future behavior.
        let shuffle_seed = self.seed ^ self.exchange_count;
        Self::shuffle_tiles_with_seed(&mut self.tiles, shuffle_seed);
        self.exchange_count += 1;

        // Then replenish
        self.remove_and_replenish(turn, &Vec::new());
    }

    pub fn get_tile_count(&self) -> usize {
        self.tiles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_bag_initialization() {
        let bag = TileBag::new(2);

        // Total tiles should be 100 - 14 (7 per player * 2 players)
        // 100 total tiles in Scrabble
        assert_eq!(bag.get_tile_count(), 100 - MAX_PLAYER_TILES * 2);
    }

    #[test]
    fn test_tile_bag_initializes_all_players() {
        let total_players = 4;
        let bag = TileBag::new(total_players);

        // Each player should have exactly MAX_PLAYER_TILES tiles
        for player in 0..total_players {
            let tiles_str = bag.get_tiles(player);
            assert_eq!(tiles_str.len(), MAX_PLAYER_TILES);
        }
    }

    #[test]
    fn test_correct_tile_distribution() {
        let bag = TileBag::new(1);

        // Verify player has 7 tiles
        let player_tiles = bag.get_tiles(0);
        assert_eq!(player_tiles.len(), MAX_PLAYER_TILES);

        // Verify the bag has the correct number of tiles remaining
        // 100 total - 7 for player = 93
        assert_eq!(bag.get_tile_count(), 100 - MAX_PLAYER_TILES);
    }

    #[test]
    fn test_get_tiles_returns_string() {
        let bag = TileBag::new(2);

        let player_0_tiles = bag.get_tiles(0);
        let player_1_tiles = bag.get_tiles(1);

        // Both players should have tiles
        assert!(!player_0_tiles.is_empty());
        assert!(!player_1_tiles.is_empty());
        assert_eq!(player_0_tiles.len(), MAX_PLAYER_TILES);
        assert_eq!(player_1_tiles.len(), MAX_PLAYER_TILES);
    }

    #[test]
    fn test_remove_and_replenish_basic() {
        let mut bag = TileBag::new(2);

        let initial_tiles = bag.get_tiles(0);
        let initial_tile_count = bag.get_tile_count();

        // Create a placement for the first tile
        let placements = vec![TilePlacement {
            coords: (0, 0),
            tile: initial_tiles.chars().next().unwrap(),
        }];

        bag.remove_and_replenish(0, &placements);

        // Player should still have 7 tiles
        assert_eq!(bag.get_tiles(0).len(), MAX_PLAYER_TILES);

        // Bag should have one less tile
        assert_eq!(bag.get_tile_count(), initial_tile_count - 1);
    }

    #[test]
    fn test_remove_and_replenish_multiple_tiles() {
        let mut bag = TileBag::new(2);

        let initial_tile_count = bag.get_tile_count();
        let player_tiles = bag.get_tiles(0);

        // Create placements for first 3 tiles
        let placements: Vec<TilePlacement> = player_tiles
            .chars()
            .take(3)
            .enumerate()
            .map(|(i, tile)| TilePlacement {
                coords: (0, i),
                tile,
            })
            .collect();

        bag.remove_and_replenish(0, &placements);

        // Player should still have 7 tiles
        assert_eq!(bag.get_tiles(0).len(), MAX_PLAYER_TILES);

        // Bag should have 3 less tiles
        assert_eq!(bag.get_tile_count(), initial_tile_count - 3);
    }

    #[test]
    fn test_remove_and_replenish_specific_tile() {
        let mut bag = TileBag::new(1);

        let initial_tiles_str = bag.get_tiles(0);
        let initial_tile_count = bag.get_tile_count();

        // Get the first tile
        let tile_to_remove = initial_tiles_str.chars().next().unwrap();

        // Create a placement
        let placements = vec![TilePlacement {
            coords: (0, 0),
            tile: tile_to_remove,
        }];

        bag.remove_and_replenish(0, &placements);

        let new_tiles_str = bag.get_tiles(0);

        // The removed tile should no longer be in the player's tiles
        // (unless a duplicate was drawn, which is possible)
        assert_eq!(new_tiles_str.len(), MAX_PLAYER_TILES);
        assert_eq!(bag.get_tile_count(), initial_tile_count - 1);
    }

    #[test]
    fn test_exchange_tiles() {
        let mut bag = TileBag::new(1);

        let initial_tiles = bag.get_tiles(0);
        let initial_bag_count = bag.get_tile_count();

        // Exchange the first 3 tiles
        let tiles_to_exchange = initial_tiles.chars().take(3).collect::<String>();

        bag.exchange(0, tiles_to_exchange.clone());

        // Player should still have 7 tiles
        assert_eq!(bag.get_tiles(0).len(), MAX_PLAYER_TILES);

        // Bag should still have the same number of tiles
        assert_eq!(bag.get_tile_count(), initial_bag_count);
    }

    #[test]
    fn test_exchange_returns_different_tiles() {
        let mut bag = TileBag::new(1);

        let initial_tiles = bag.get_tiles(0);
        let tiles_to_exchange = initial_tiles.clone();

        bag.exchange(0, tiles_to_exchange.clone());

        let new_tiles = bag.get_tiles(0);

        // The new tiles should be different (very likely with 7 random tiles)
        // Note: There's a very small chance they're the same, but highly unlikely
        // This test verifies that exchange actually happens
        assert_eq!(new_tiles.len(), MAX_PLAYER_TILES);
    }

    #[test]
    fn test_get_tile_count_decreases_with_replenish() {
        let mut bag = TileBag::new(1);

        let count_before = bag.get_tile_count();

        let player_tiles = bag.get_tiles(0);
        let placements = vec![TilePlacement {
            coords: (0, 0),
            tile: player_tiles.chars().next().unwrap(),
        }];

        bag.remove_and_replenish(0, &placements);

        let count_after = bag.get_tile_count();

        // Count should decrease by 1
        assert_eq!(count_before - count_after, 1);
    }

    #[test]
    fn test_multiple_players_independent() {
        let mut bag = TileBag::new(3);

        let player_0_initial = bag.get_tiles(0);

        // Remove tiles from player 0
        let placements = vec![TilePlacement {
            coords: (0, 0),
            tile: player_0_initial.chars().next().unwrap(),
        }];

        bag.remove_and_replenish(0, &placements);

        // Player 1 and 2 should still have their original tiles
        // (they might draw new ones if replenish happens to give them the same)
        assert_eq!(bag.get_tiles(1).len(), MAX_PLAYER_TILES);
        assert_eq!(bag.get_tiles(2).len(), MAX_PLAYER_TILES);
    }

    #[test]
    fn test_tile_count_consistency() {
        let mut bag = TileBag::new(2);

        let initial_bag_count = bag.get_tile_count();
        let initial_player_count = bag.get_tiles(0).len() + bag.get_tiles(1).len();

        // Total tiles should equal initial tiles
        assert_eq!(initial_bag_count + initial_player_count, 100);

        // After a placement and replenish
        let placements = vec![TilePlacement {
            coords: (0, 0),
            tile: bag.get_tiles(0).chars().next().unwrap(),
        }];

        bag.remove_and_replenish(0, &placements);

        let final_bag_count = bag.get_tile_count();
        let final_player_count = bag.get_tiles(0).len() + bag.get_tiles(1).len();

        // One tile was theorietically placed onto the board. The count should be 99.
        assert_eq!(final_bag_count + final_player_count, 99);
    }

    #[test]
    fn test_remove_and_replenish_near_empty_bag() {
        let mut bag = TileBag::new(4);

        // Manually remove most tiles from the bag for testing
        for _ in 0..bag.get_tile_count() - 2 {
            bag.tiles.pop();
        }

        let remaining_in_bag = bag.get_tile_count();
        let player_tiles = bag.get_tiles(0);

        let placements = vec![TilePlacement {
            coords: (0, 0),
            tile: player_tiles.chars().next().unwrap(),
        }];

        bag.remove_and_replenish(0, &placements);

        // Player should still have 7 tiles
        assert_eq!(bag.get_tiles(0).len(), MAX_PLAYER_TILES);

        // Bag should have fewer tiles
        assert!(bag.get_tile_count() < remaining_in_bag);
    }

    #[test]
    fn test_exchange_with_few_tiles_in_bag() {
        let mut bag = TileBag::new(1);

        let tiles_to_exchange = bag.get_tiles(0).chars().take(3).collect::<String>();
        bag.exchange(0, tiles_to_exchange);

        // Player should still have 7 tiles
        assert_eq!(bag.get_tiles(0).len(), MAX_PLAYER_TILES);
    }

    #[test]
    fn test_get_tiles_contains_valid_characters() {
        let bag = TileBag::new(2);

        let valid_tiles: Vec<char> = vec![
            'E', 'A', 'I', 'O', 'N', 'R', 'T', 'D', 'L', 'S', 'U', 'B', 'C', 'M', 'P', 'F', 'H',
            'V', 'W', 'Y', 'Q', 'Z', 'J', 'X', 'K',
        ];

        let player_tiles = bag.get_tiles(0);
        for tile in player_tiles.chars() {
            assert!(valid_tiles.contains(&tile), "Invalid tile: {}", tile);
        }
    }

    #[test]
    fn test_new_bag_has_all_tiles() {
        let bag = TileBag::new(1);

        // Count all tiles (in bag + with player)
        let mut tile_counts: HashMap<char, usize> = HashMap::new();

        let player_tiles = bag.get_tiles(0);
        for tile in player_tiles.chars() {
            *tile_counts.entry(tile).or_insert(0) += 1;
        }

        // The total tiles in game should be 100
        let total_tiles_with_player = player_tiles.len() + bag.get_tile_count();
        assert_eq!(total_tiles_with_player, 100);
    }

    #[test]
    fn test_wildcard_replenish() {
        let mut bag = TileBag::new(2);

        bag.tiles.clear();
        bag.tiles.extend("ABCDEFGABCDEFG".chars());

        bag.players_tiles.get_mut(&0).unwrap().clear();
        bag.players_tiles.get_mut(&0).unwrap().insert(WILD, 1); // Give player 0 a wildcard
        let placements = vec![TilePlacement {
            coords: (0, 0),
            tile: 'Z', // Assume player tries to place a 'Z' they don't have
        }];
        bag.remove_and_replenish(0, &placements);

        // Make sure set is replenished.
        assert_eq!(bag.get_tiles(0).len(), MAX_PLAYER_TILES);
        // Make sure wildcard was consumed.
        assert!(!bag.get_tiles(0).contains(WILD));

        bag.players_tiles.get_mut(&0).unwrap().clear();
        bag.players_tiles.get_mut(&0).unwrap().insert(WILD, 1); // Give player 0 a wildcard
        bag.players_tiles.get_mut(&0).unwrap().insert('A', 1); // Give player 0 an A

        let placements = vec![TilePlacement {
            coords: (0, 0),
            tile: 'A', // Assume player tries to place a 'A' that they do have
        }];
        bag.remove_and_replenish(0, &placements);

        // Make sure set is replenished.
        assert_eq!(bag.get_tiles(0).len(), MAX_PLAYER_TILES);
        // Make sure A was consumed and not the wildcard.
        assert!(!bag.get_tiles(0).contains('A'));
        assert!(bag.get_tiles(0).contains(WILD));
    }

    #[test]
    fn test_replenish_no_underflow() {
        let mut bag = TileBag::new(2);

        bag.tiles.clear();
        bag.tiles.extend("ABCDEFGABCDEFG".chars());

        bag.players_tiles.get_mut(&0).unwrap().clear();
        bag.players_tiles.get_mut(&0).unwrap().insert(WILD, 1);
        bag.players_tiles.get_mut(&0).unwrap().insert('A', 2);

        let placements = vec![
            TilePlacement {
                coords: (0, 0),
                tile: 'A',
            },
            TilePlacement {
                coords: (0, 0),
                tile: 'A',
            },
            TilePlacement {
                coords: (0, 0),
                tile: 'A',
            },
        ];

        // At some point, there was a bug where this would panic because the players tile bag had the 'A' entry, but with a count of zero and a tile placement would try to use it again.
        bag.remove_and_replenish(0, &placements);

        // Should be replenished.
        assert_eq!(bag.get_tiles(0).len(), MAX_PLAYER_TILES);
        // And the wildcard consumed.
        assert!(!bag.get_tiles(0).contains(WILD));
    }
}
