use rand::prelude::SliceRandom;

pub struct TileBag {
    tiles: Vec<char>,
}

impl TileBag {
    pub fn new() -> TileBag {
        let mut bag = TileBag { tiles: Vec::new() };

        let mut insert = |letter: char, count: usize| {
            for _ in 0..count {
                bag.tiles.push(letter);
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

        bag
    }

    pub fn take(&mut self, count: usize) -> String {
        let mut rng = rand::rng();
        self.tiles.shuffle(&mut rng);
        self.tiles[..count].iter().collect()
    }
}
