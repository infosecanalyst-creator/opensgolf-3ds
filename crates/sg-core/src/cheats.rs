//! The port's Cheats box (docs/PLAYING.md, "Cheats"; the box itself is simgolf's cheats_ui.rs): not in the original, and
//! nothing here runs unless the player opens the box with its hidden key and picks a cheat. Each cheat goes through the
//! game's own records: money is added to or taken from the cash as income or spending that is not booked in the ledger, a
//! forced visit takes the next pair at the first tee through the visitor check of the pairing (vips.rs `vip_check`), and
//! the land is bought tract by tract as the land screen buys it (tracts.rs `buy`), for nothing.
//!
//! Using any cheat marks the game for good (`Club::cheated`, saved with it): the Load list notes "cheats used" and a course
//! shared from it carries no challenge.

use crate::economy::Economy;
use crate::golfer::{game, Club, SLOTS};
use crate::land::{mn, Land, T_OUT};
use crate::vips::{ROSTER_CHALLENGER, ROSTER_COMMISSIONER, ROSTER_HEIRESS};

/// The amounts the box adds or takes away, in dollars.
pub const MONEY_SMALL: f64 = 10_000.0;
pub const MONEY_LARGE: f64 = 100_000.0;

/// Adds `amount` dollars to the cash (a negative amount takes it away), outside the ledger: the Financial Report's income
/// and costs stay what the club earned and spent.
pub fn add_money(club: &mut Club, econ: &mut Economy, amount: f64) {
    if amount >= 0.0 {
        econ.earn(amount);
    } else {
        econ.spend(-amount);
    }
    club.cheated = true;
}

/// Which visitor a forced visit brings for a pair whose number points at visitor turn `v` (0..5, as `vip_check` reads it):
/// the CEO, the commissioner or the heiress, the first of them that can come, starting from the one of that turn and
/// going round. A visitor of the kind already on the course does not come again, the commissioner never to an island and
/// the CEO only while a hole for his next visit is open; the ratings the game asks for otherwise do not count, and a
/// sandbox game gets visitors too. Returns (ceo, commissioner, heiress), at most one true.
pub(crate) fn forced_vip(club: &Club, v: usize) -> (bool, bool, bool) {
    let mut present = 0u32;
    for o in club.g.iter().take(SLOTS) {
        if o.hole > 0 && o.roster >= ROSTER_CHALLENGER {
            present |= 1 << (o.roster - ROSTER_CHALLENGER);
        }
    }
    let k = club.ceo_count;
    // the exe's own presence bits: the CEO and the heiress both look at the heiress's
    let heiress_out = present & 1 << (ROSTER_HEIRESS - ROSTER_CHALLENGER) != 0;
    let can = [
        !heiress_out && (0..8).contains(&k) && club.par(2 + 2 * k) != 0,
        present & 1 << (ROSTER_COMMISSIONER - ROSTER_CHALLENGER) == 0 && !club.island,
        !heiress_out && club.landmarks_owned < 0xffff,
    ];
    let first = (v / 2) % 3;
    match (0..3).map(|i| (first + i) % 3).find(|&i| can[i]) {
        Some(0) => (true, false, false),
        Some(1) => (false, true, false),
        Some(2) => (false, false, true),
        _ => (false, false, false),
    }
}

/// Whether a forced visit can be asked for now: the first hole is open (golfers tee off), no tournament is on, none is
/// waiting already, and some visitor can come.
pub fn vip_possible(club: &Club) -> bool {
    club.par(1) != 0 && club.game & game::TOURNAMENT == 0 && !club.vip_forced && {
        let (a, b, c) = forced_vip(club, 0);
        a || b || c
    }
}

/// Force a VIP visit now: the next pair to tee off brings a visitor. Returns false (nothing changed) when none can come.
pub fn force_vip(club: &mut Club) -> bool {
    if !vip_possible(club) {
        return false;
    }
    club.vip_forced = true;
    club.cheated = true;
    true
}

/// The tracts (0..8) still holding land outside the course's bounds.
pub fn tracts_for_sale(land: &Land) -> Vec<usize> {
    let side = crate::tracts::side();
    (0..9)
        .filter(|&i| {
            let (a0, b0) = crate::tracts::origin(i);
            (0..side).any(|da| (0..side).any(|db| land.ty.get(((a0 + da) * mn() + b0 + db) as usize) == Some(&T_OUT)))
        })
        .collect()
}

/// Buy all the land: every tract still for sale is bought (as the land screen buys one) for nothing. Returns the tracts
/// bought, for the caller to bring the drawn terrain up to date.
pub fn buy_all_land(club: &mut Club, land: &mut Land) -> Vec<usize> {
    let bought = tracts_for_sale(land);
    for &i in &bought {
        crate::tracts::buy(land, i);
    }
    if !bought.is_empty() {
        club.cheated = true;
    }
    bought
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_goes_in_and_out_without_the_ledger() {
        let mut club = Club::default();
        let mut econ = Economy { start_cash: 5000.0, ..Default::default() };
        econ.init();
        let before = econ.cash;
        let ledger = econ.ledger.clone();
        assert!(!club.cheated);
        add_money(&mut club, &mut econ, MONEY_LARGE);
        assert_eq!(econ.cash, before + 100_000.0);
        add_money(&mut club, &mut econ, -MONEY_SMALL);
        assert_eq!(econ.cash, before + 90_000.0);
        assert!(club.cheated);
        assert_eq!(econ.ledger, ledger);
    }

    #[test]
    fn the_cheated_mark_is_saved_only_when_set() {
        let mut club = Club::default();
        let plain = serde_json::to_string(&club).unwrap();
        assert!(!plain.contains("cheated") && !plain.contains("vip_forced") && !plain.contains("pro_left"));
        // a save from before the mark reads unmarked
        let back: Club = serde_json::from_str(&plain).unwrap();
        assert!(!back.cheated);
        club.cheated = true;
        let json = serde_json::to_string(&club).unwrap();
        assert!(json.contains("\"cheated\":true"));
        let back: Club = serde_json::from_str(&json).unwrap();
        assert!(back.cheated);
    }

    fn open_club() -> Club {
        let mut club = Club::default();
        for h in 1..=4 {
            club.holes[h].par = 4;
        }
        club.next_hole = 5;
        club
    }

    #[test]
    fn a_forced_visit_needs_an_open_first_hole_and_no_tournament() {
        let mut club = Club::default();
        assert!(!force_vip(&mut club), "no hole open");
        assert!(!club.cheated);
        let mut club = open_club();
        club.game |= game::TOURNAMENT;
        assert!(!force_vip(&mut club));
        club.game &= !game::TOURNAMENT;
        assert!(force_vip(&mut club));
        assert!(club.vip_forced && club.cheated);
        assert!(!vip_possible(&club), "one waits already");
    }

    #[test]
    fn forced_visitors_go_round_from_the_pair_turn() {
        let mut club = open_club();
        assert_eq!(forced_vip(&club, 1), (true, false, false));
        assert_eq!(forced_vip(&club, 3), (false, true, false));
        assert_eq!(forced_vip(&club, 5), (false, false, true));
        // no commissioner for an island, the CEO's next hole not open: the heiress
        club.island = true;
        club.ceo_count = 2;
        assert_eq!(forced_vip(&club, 1), (false, false, true));
        assert_eq!(forced_vip(&club, 3), (false, false, true));
        // the heiress on the course: no CEO and no heiress either (the exe's presence bit)
        club.g[0].hole = 3;
        club.g[0].roster = ROSTER_HEIRESS;
        assert_eq!(forced_vip(&club, 5), (false, false, false));
        assert!(!vip_possible(&club));
    }

    #[test]
    fn all_the_land_is_bought_for_nothing() {
        use crate::land::{self, ExeRng, Noise, Slot};
        let mut rng = ExeRng::from_clock(4242);
        let noise = Noise::new(&mut rng);
        let acres = land::slot_acres(4, 9, false);
        let mut l = land::generate(4, Slot { property: 9, acres }, 1, false, &mut rng, &noise);
        let mut club = Club::default();
        let for_sale = tracts_for_sale(&l);
        assert!(!for_sale.is_empty(), "a small slot leaves land for sale");
        assert_eq!(buy_all_land(&mut club, &mut l), for_sale);
        assert!(club.cheated);
        // every tract is the club's (the map's one-tile rim is no tract's and stays out of bounds)
        assert!(tracts_for_sale(&l).is_empty());
        assert_eq!(club.purchases, 0, "land bought by cheating does not raise the price of the next tract");
        // nothing left to buy: the game is not marked
        let mut club = Club::default();
        assert!(buy_all_land(&mut club, &mut l).is_empty());
        assert!(!club.cheated);
    }

    #[test]
    fn the_forced_visit_takes_the_next_pair() {
        let mut club = open_club();
        // ratings far below what any visitor asks for, and a pair whose number brings none
        club.ratings.fun = 0;
        club.ratings.skill = 0;
        let mut rng = crate::land::ExeRng::from_clock(7);
        club.vip_check(&mut rng, 1, false);
        assert_eq!(club.g[1].kind, 0, "no visitor of the game's own");
        assert!(force_vip(&mut club));
        // pair 0 (turn 0) starts the round at the CEO
        club.vip_check(&mut rng, 1, false);
        assert_eq!(club.g[1].vip(), 0x60);
        assert!(!club.vip_forced, "taken by that pair");
        // a sandbox game's forced visit comes too
        let mut club = open_club();
        club.sandbox = true;
        assert!(force_vip(&mut club));
        club.vip_check(&mut rng, 7, true);
        assert_eq!(club.g[7].vip(), 0x40, "pair 3 (turn 3): the commissioner");
    }
}
