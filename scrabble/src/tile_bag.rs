use crate::common::{MAX_PLAYER_TILES, Player, TilePlacement};
use rand::prelude::SliceRandom;
use std::cmp::min;
use std::collections::HashMap;

pub struct TileBag {
    tiles: Vec<char>,
    players_tiles: HashMap<Player, HashMap<char, usize>>,
}

impl TileBag {
    pub fn new(total_players: usize) -> TileBag {
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

        insert('B', 2);
        insert('C', 2);
        insert('M', 2);
        insert('P', 2);
        insert('F', 2);
        insert('H', 2);
        insert('V', 2);
        insert('W', 2);
        insert('Y', 2);

        insert('Q', 1);
        insert('Z', 1);
        insert('J', 1);
        insert('X', 1);
        insert('K', 1);

        let mut rng = rand::rng();
        all_tiles.shuffle(&mut rng);

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
            tiles: all_tiles,
            players_tiles,
        }
    }

    pub fn remove_and_replenish(&mut self, turn: Player, placements: &Vec<TilePlacement>) {
        let relevant_set = self.players_tiles.get_mut(&turn).unwrap();

        for placement in placements {
            *relevant_set.get_mut(&placement.tile).unwrap() -= 1;
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

        relevant_set
            .iter()
            .filter_map(|(tile, count)| {
                if *count > 0 {
                    Some(tile.to_string().repeat(*count))
                } else {
                    None
                }
            })
            .collect()
    }
}
