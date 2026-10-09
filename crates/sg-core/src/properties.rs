//! The sixteen properties of the property chooser, in the exe's order (docs/PUBLISHER_EXE_NOTES.md, "Properties and new-game
//! land"). The card a property is drawn on is fixed: properties 0..9 down the left arc, 10..15 down the right. The acreage and
//! price on a card are not fixed: they come from the offer slot the property was dealt into for this game (`land::deal_offer`,
//! `land::SLOT_PRICE_UNITS`). The money symbol of the game is the section sign.

use crate::land::{port_theme, RECORDS};

#[derive(Clone, Copy, Debug)]
pub struct Property {
    pub name: &'static str,
    pub course: &'static str,
    pub bonus: &'static str,
    /// The port's theme order: 0 Parkland, 1 Links, 2 Desert, 3 Tropical.
    pub theme: usize,
    /// 0 left, 1 right in the chooser.
    pub column: i32,
}

/// Starting funds: the exe starts a game with 1000 money units (one unit is 100).
pub const START_FUNDS: i32 = 100_000;

const fn p(i: usize) -> Property {
    let r = &RECORDS[i];
    Property { name: r.name, course: r.course, bonus: r.bonus, theme: port_theme(r.theme), column: if i < 10 { 0 } else { 1 } }
}

pub const PROPERTIES: [Property; 16] =
    [p(0), p(1), p(2), p(3), p(4), p(5), p(6), p(7), p(8), p(9), p(10), p(11), p(12), p(13), p(14), p(15)];
