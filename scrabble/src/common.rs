use std::fmt::Debug;

pub const MAX_PLAYER_TILES: usize = 7;

pub type Coords = (usize, usize);

#[derive(Clone, Copy)]
pub struct TilePlacement {
    pub coords: Coords,
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

/// Zero-indexed player number (0, 1, 2, or 3)
pub(crate) type Player = usize;

pub(crate) enum ScoreModifier {
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
pub const SCORE_MODIFIERS: [[ScoreModifier; 15]; 15] = {
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
pub const SCORES: [u32; 91] = {
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 1, 3, 3, 2, 1, 4, 2, 4, 1, 8, 5, 1, 3, 1, 1, 3, 10, 1, 1, 1, 1, 4, 4, 8, 4,
        10,
    ]
};
