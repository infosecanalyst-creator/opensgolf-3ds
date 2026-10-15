//! The port's own optional gameplay changes ("OpenSGolf extras", docs/PLAYING.md): none of them is in the original, all are
//! off by default, and a game with all of them off plays exactly as before. The choice is kept with each saved game (in the
//! club record, only when something is on, so saves made with everything off are unchanged).
//!
//! * `rotate`: Tab turns a placed building under the pointer a quarter, the clubhouse and putting greens included.
//! * `cut_content`: content the game files hold but the shipped game never offers (docs/CUT_CONTENT.md) can be placed.
//! * `hardcore`: tighter money and more demanding golfers, by the `hardcore_*` numbers of data/economy.toml.
//! * `more_staff`: the employee limit is data/economy.toml's `staff_limit_high` instead of `staff_limit`.
//! * `animals`: the player can buy animals (the wildlife of the theme) that stay near where they were set down.

use serde::{Deserialize, Serialize};

/// The extras a game was started with (or changed to).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Extras {
    pub rotate: bool,
    pub cut_content: bool,
    pub hardcore: bool,
    pub more_staff: bool,
    pub animals: bool,
}

/// The options in the order the Extras box lists them (text keys of their lines).
pub const OPTION_KEYS: [&str; 5] =
    ["STR_EXTRAS_ROTATE", "STR_EXTRAS_CUT_CONTENT", "STR_EXTRAS_HARDCORE", "STR_EXTRAS_MORE_STAFF", "STR_EXTRAS_ANIMALS"];

impl Extras {
    /// The gameplay extras of the "OpenSGolf extras" choice on Select Difficulty: the ones that add to the game without
    /// changing its balance (not Hardcore, not the unfinished cut content).
    pub const OPENSGOLF: Extras = Extras { rotate: true, cut_content: false, hardcore: false, more_staff: true, animals: true };

    /// Everything off (the original game).
    pub fn is_off(&self) -> bool {
        *self == Extras::default()
    }

    /// One bit per option, in `OPTION_KEYS` order.
    pub fn bits(&self) -> u32 {
        self.rotate as u32
            | (self.cut_content as u32) << 1
            | (self.hardcore as u32) << 2
            | (self.more_staff as u32) << 3
            | (self.animals as u32) << 4
    }

    pub fn from_bits(b: u32) -> Extras {
        Extras { rotate: b & 1 != 0, cut_content: b & 2 != 0, hardcore: b & 4 != 0, more_staff: b & 8 != 0, animals: b & 16 != 0 }
    }

    /// How many employees the club may have (the original: 20, the Employee panel's limit). Hardcore lowers it, the higher
    /// staff limits raise it; never above 63, the original's walker table of 64 records less the player's own pro.
    pub fn staff_limit(&self) -> usize {
        let e = crate::data::economy();
        let n = if self.more_staff {
            e.staff_limit_high
        } else if self.hardcore {
            e.hardcore_staff_limit
        } else {
            e.staff_limit
        };
        n.clamp(1, MAX_STAFF)
    }

    /// A cost scaled by a hardcore percentage (unchanged when hardcore is off).
    fn scaled(&self, v: f64, percent: i32) -> f64 {
        if self.hardcore {
            v * percent as f64 / 100.0
        } else {
            v
        }
    }

    /// A wage, under hardcore `hardcore_wage_percent` of it.
    pub fn wage(&self, v: f64) -> f64 {
        self.scaled(v, crate::data::economy().hardcore_wage_percent)
    }

    /// A hole's maintenance, under hardcore `hardcore_maintenance_percent` of it.
    pub fn maintenance(&self, v: f64) -> f64 {
        self.scaled(v, crate::data::economy().hardcore_maintenance_percent)
    }

    /// The price of land (a property on the world map, a tract), under hardcore `hardcore_land_percent` of it.
    pub fn land(&self, v: f64) -> f64 {
        self.scaled(v, crate::data::economy().hardcore_land_percent)
    }

    /// Interest on debt in percent a month.
    pub fn interest_percent(&self) -> i32 {
        if self.hardcore {
            crate::data::economy().hardcore_interest_percent
        } else {
            crate::data::economy().interest_percent
        }
    }

    /// A golfer's green fee for a hole in units: at most `hardcore_fee_cap` under hardcore.
    pub fn fee(&self, units: i32) -> i32 {
        if self.hardcore {
            units.min(crate::data::economy().hardcore_fee_cap)
        } else {
            units
        }
    }

    /// A golfer's mood change: under hardcore a bad moment costs `hardcore_mood_penalty` more.
    pub fn mood_change(&self, d: i32) -> i32 {
        if self.hardcore && d < 0 {
            d - crate::data::economy().hardcore_mood_penalty
        } else {
            d
        }
    }
}

/// The most employees any setting allows (the original keeps its walkers, the player's pro first, in 64 records).
pub const MAX_STAFF: usize = 63;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_changes_nothing() {
        let e = Extras::default();
        assert!(e.is_off());
        assert_eq!(e.staff_limit(), 20);
        assert_eq!(e.wage(300.0), 300.0);
        assert_eq!(e.maintenance(100.0), 100.0);
        assert_eq!(e.land(5000.0), 5000.0);
        assert_eq!(e.interest_percent(), 2);
        assert_eq!(e.fee(14), 14);
        assert_eq!(e.mood_change(-2), -2);
        assert_eq!(Extras::from_bits(e.bits()), e);
    }

    #[test]
    fn hardcore_and_staff_limits() {
        let h = Extras { hardcore: true, ..Default::default() };
        assert_eq!(h.staff_limit(), 12);
        assert_eq!(h.wage(300.0), 450.0);
        assert_eq!(h.maintenance(100.0), 200.0);
        assert_eq!(h.land(5000.0), 7500.0);
        assert_eq!(h.interest_percent(), 5);
        assert_eq!(h.fee(14), 6);
        assert_eq!(h.mood_change(-2), -3);
        assert_eq!(h.mood_change(1), 1);
        let m = Extras { more_staff: true, hardcore: true, ..Default::default() };
        assert_eq!(m.staff_limit(), 40);
        for b in 0..32 {
            assert_eq!(Extras::from_bits(b).bits(), b);
        }
    }
}

/// What a cut landmark costs (units of $100; the port's choice, the original has none: about a mid-priced landmark).
pub const CUT_LANDMARK_UNITS: i32 = 60;

/// What a pair of animals costs (units of $100; the port's choice).
pub const ANIMAL_UNITS: i32 = 20;
