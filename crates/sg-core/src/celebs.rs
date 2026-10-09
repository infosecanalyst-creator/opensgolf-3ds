//! Celebrity residents: once a celebrity buys a vacation home, the lot is drawn as a fenced patio with a celebrity house, and
//! one resident walks about on it (actor table 0x56d1b8, routine 0x4017d0). Purely for show: no money or mood comes of it
//! beyond the golfers' pleasure at the view, which the planner already counts.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Celebrity residents"), restated in our own words.

use crate::geom::{DX, DY};
use crate::golfer::Club;
use crate::land::ExeRng;

/// Sprite bases: standing (8 views), the signature action, walking; add the celebrity's sprite type.
pub const STAND: i32 = 0x140;
pub const ACTION: i32 = 0x14d;
pub const WALK: i32 = 0x15a;

/// Sprite type names (Flics/Celebs/<name>_SQ, _Char, _Walk) by celebrity type.
pub const TYPES: [&str; 12] = [
    "ActionStar",
    "Female_PopSinger",
    "Politician",
    "Comedian",
    "Supermodel",
    "FitnessFem",
    "FemComic",
    "GenMale",
    "MoviePrincess",
    "RockStar",
    "Basketball",
    "AgingStar",
];

/// The two celebrity houses the exe draws, by the parity of the home site's object slot (the other four it loads are never
/// reached): house sprite and its ground ("dirt") layer, in Flics/Homes.
pub const HOUSES: [(&str, &str); 2] = [("Homes/Sum_home", "Homes/Sum_home_dirt"), ("Homes/political_home", "Homes/political_home_dirt")];

/// One resident.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Actor {
    /// Celebrity sprite type (0..11) and the celebrity record + 1 that owns the home.
    pub kind: i32,
    pub owner: i32,
    /// Walk layout (0..5), home tile, hold timer, state (a sprite base), position inside the lot in map units.
    pub layout: i32,
    pub home: (i32, i32),
    pub hold: i32,
    pub state: i32,
    pub x: i32,
    pub y: i32,
    pub facing: i32,
    pub steps: i32,
    pub frame: i32,
}

/// The cells of the 4 x 4 half-tile grid a layout's house piece takes. APPROXIMATION: the piece shapes (0x4c11e0) are in a
/// data table that is not available; the piece is taken to cover the 2 x 2 cells at its corner.
pub fn blocked(_layout: i32, u: i32, v: i32) -> bool {
    u < 2 && v < 2
}

fn cell_pos(u: i32, v: i32) -> (i32, i32) {
    ((u * 1024 + 512) / 2, (v * 1024 + 512) / 2)
}

impl Actor {
    pub fn cell(&self) -> (i32, i32) {
        (self.x / 512, self.y / 512)
    }

    /// Position in map units: the lot's 2 x 2 tiles hold 4 x 4 half-tile cells, and sixteen steps of 1024 / 32 cross one.
    pub fn world(&self) -> (i32, i32) {
        (self.home.0 * 1024 + self.x, self.home.1 * 1024 + self.y)
    }

    /// One step of the resident (0x4017d0). `frames` is the frame count of the sprite now shown; `visible` whether it is on
    /// screen. Returns a voice sound slot when the resident does its signature action.
    pub fn step(&mut self, rng: &mut ExeRng, frames: i32, visible: bool, paused: bool, tournament: bool) -> Option<i32> {
        if !visible {
            if !paused && rng.below(500) == 0 {
                self.hold = 500;
            }
        } else if self.hold < 128 && !paused {
            self.frame = (self.frame % frames.max(1) + 1) % frames.max(1);
        }
        let mut voice = None;
        if self.hold != 0 {
            self.state = STAND;
            if !paused {
                self.hold -= 1;
            }
            return None;
        }
        if self.frame == 0 && self.state != WALK && self.state != STAND {
            self.state = STAND;
            self.hold = rng.below(64) + 64;
            return None;
        }
        if self.steps == 0 {
            if self.state == STAND && rng.below(8) == 0 {
                self.facing = (self.facing + rng.below(3) - 1) & 7;
                let (u, v) = self.cell();
                let (tu, tv) = (u + DX[self.facing as usize], v + DY[self.facing as usize]);
                if !(0..4).contains(&tu) || !(0..4).contains(&tv) {
                    if rng.below(if tournament { 64 } else { 24 }) == 0 {
                        voice = Some(0x82 + self.kind);
                        self.state = ACTION;
                        self.frame = 0;
                    }
                } else if !blocked(self.layout, tu, tv) {
                    self.steps = 16;
                }
            }
            if self.steps == 0 {
                return voice;
            }
        }
        self.state = WALK;
        self.x += DX[self.facing as usize] * 1024 / 32;
        self.y += DY[self.facing as usize] * 1024 / 32;
        self.steps -= 1;
        if self.steps < 1 {
            self.state = STAND;
        }
        voice
    }
}

impl Club {
    /// A resident moves in (0x4011e0): a walk layout, then a free starting cell of the lot.
    pub(crate) fn spawn_resident(&mut self, rng: &mut ExeRng, celeb: i32, a: i32, b: i32) {
        let layout = rng.below(6);
        let (u, v) = loop {
            let u = rng.below(4);
            let v = rng.below(4);
            if !blocked(layout, u, v) {
                break (u, v);
            }
        };
        let (x, y) = cell_pos(u, v);
        let kind = self.celebrities.get(celeb as usize).map(|c| c.kind as i32).unwrap_or(7).clamp(0, 11);
        self.residents.push(Actor { kind, owner: celeb + 1, layout, home: (a, b), state: STAND, x, y, ..Default::default() });
    }

    /// Demolishing a home site sends its residents away (0x4011b0).
    pub fn evict(&mut self, a: i32, b: i32) {
        self.residents.retain(|r| r.home != (a, b));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_one_cell_in_sixteen_steps() {
        let mut a = Actor { state: STAND, x: 1792, y: 1792, facing: 2, steps: 16, layout: 0, ..Default::default() };
        let mut rng = ExeRng::from_clock(1);
        for _ in 0..16 {
            a.step(&mut rng, 8, true, false, false);
        }
        assert_eq!((a.x, a.y), (1792 + 512, 1792));
        assert_eq!(a.state, STAND);
    }
}
