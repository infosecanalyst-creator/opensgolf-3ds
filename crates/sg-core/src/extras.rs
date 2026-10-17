//! The port's own optional gameplay changes ("OpenSGolf extras", docs/PLAYING.md): none of them is in the original, all are
//! off by default, and a game with all of them off plays exactly as before. The choice is kept with each saved game (in the
//! club record, only when something is on, so saves made with everything off are unchanged).
//!
//! * `rotate`: Tab turns a placed building under the pointer a quarter, the clubhouse and putting greens included.
//! * `cut_content`: content the game files hold but the shipped game never offers (docs/CUT_CONTENT.md) can be placed.
//! * `hardcore`: tighter money and more demanding golfers, by the `hardcore_*` numbers of data/economy.toml.
//! * `more_staff`: the employee limit is data/economy.toml's `staff_limit_high` instead of `staff_limit`.
//! * `animals`: the player can buy animals (the wildlife of the theme) that stay near where they were set down.
//! * `set_par`: the player can change an open hole's par on its Hole Stats (`Club::set_par`); everything that goes by par
//!   (scores, golfers' moods, the hole's ratings and name, reports) then goes by the new one.
//! * `season_speed`: Slower seasons, the calendar at 1x (the original), 1/2x or 1/4x: see `Calendar`.
//! * `pause_calendar`: Sandbox: pause the calendar, the time of year never moves on (`Calendar`).
//! * `freeze_moods`: Sandbox: freeze golfer happiness, no golfer's mood ever drops (`Extras::mood_change`, `Club::tick`).
//! * `unlock_all`: Sandbox: unlock all buildings and landmarks, as a Sandbox Mode game has them (`Club::landmarks_shown`;
//!   the building counter in the game, app.rs `unlocked`).
//!
//! * `lefties`: Left-handed golfers: about one new golfer in ten is left-handed (`Golfer::left`, drawn from the game's
//!   generator when the golfer is made), and the player's pro may be (`Club::pro_left`, a tick on Customise Golfer). A
//!   left-hander is drawn mirrored while addressing the ball and swinging, and his shots bend the other way: his draw goes
//!   left to right and his fade, hook and slice are mirrored too (`Club::left_handed`, planner.rs).
//!
//! * `weather`: Weather that matters: the Enhanced graphics' rainy, snowy, stormy and hot days (sg_core::weather) are
//!   played too: wet and snowy ground holds a rolling ball and baked desert ground lets it run, fewer golfers come in
//!   foul weather and some start a little less happy, a heat wave makes golfers thirsty, a thunderstorm holds play for a
//!   while; the morning's ticker says what the day brings (`Club::weather_tick`). By data/economy.toml's `[economy.weather]`.
//! * `path_style`: Path styles and lamps (purely cosmetic): 0 off; 1 on with the original's paths; 2..5 every path of the
//!   course drawn in another material (brick, gravel, dark stone, wooden boardwalk), Tab with the Pathway tool stepping
//!   through them. The landmark strip gets a page with a lamp post the player can place (`Club::lamps`), lit at night by
//!   Enhanced graphics' time of day.
//!
//! Pause the calendar, Freeze happiness and Unlock all are named "Sandbox: ..." in the Extras box as they are what a sandbox
//! player wants most, but each works in a normal game too.

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
    pub set_par: bool,
    /// Slower seasons: 0 the original's calendar (1x), 1 half as fast (1/2x), 2 a quarter as fast (1/4x).
    pub season_speed: u8,
    pub pause_calendar: bool,
    pub freeze_moods: bool,
    pub unlock_all: bool,
    pub lefties: bool,
    pub weather: bool,
    /// Path styles and lamps: 0 off, else 1 + the course's path style (`PATH_STYLE_KEYS`).
    pub path_style: u8,
}

/// The options in the order the Extras box lists them (text keys of their lines). Option 6 has three values (a line with
/// {VALUE}, `SPEED_KEYS`), the others are ticks; `Extras::option` and `Extras::set_option` read and set them by index.
/// Left-handed golfers (10) came after the sandbox switches and is shown on the Gameplay page (extras_ui.rs `page_lines`).
pub const OPTION_KEYS: [&str; 13] = [
    "STR_EXTRAS_ROTATE",
    "STR_EXTRAS_CUT_CONTENT",
    "STR_EXTRAS_HARDCORE",
    "STR_EXTRAS_MORE_STAFF",
    "STR_EXTRAS_ANIMALS",
    "STR_EXTRAS_SET_PAR",
    "STR_EXTRAS_SEASON_SPEED",
    "STR_EXTRAS_PAUSE_CALENDAR",
    "STR_EXTRAS_FREEZE_MOODS",
    "STR_EXTRAS_UNLOCK_ALL",
    "STR_EXTRAS_LEFTIES",
    "STR_EXTRAS_WEATHER",
    "STR_EXTRAS_PATHS",
];

/// The option of `OPTION_KEYS` that is Weather that matters.
pub const WEATHER: usize = 11;

/// The option of `OPTION_KEYS` that is Left-handed golfers.
pub const LEFTIES: usize = 10;

/// The option of `OPTION_KEYS` that is Path styles and lamps (several values: `PATH_STYLE_KEYS`).
pub const PATHS: usize = 12;

/// The option of `OPTION_KEYS` that is the calendar speed.
pub const SEASON_SPEED: usize = 6;

/// The calendar speeds' names ({VALUE} of the Slower seasons line), by `season_speed`.
pub const SPEED_KEYS: [&str; 3] = ["STR_EXTRAS_SPEED_1", "STR_EXTRAS_SPEED_HALF", "STR_EXTRAS_SPEED_QUARTER"];

impl Extras {
    /// The gameplay extras of the "OpenSGolf extras" choice on Select Difficulty: the ones that add to the game without
    /// changing its balance (not Hardcore, not the unfinished cut content).
    pub const OPENSGOLF: Extras = Extras {
        rotate: true,
        cut_content: false,
        hardcore: false,
        more_staff: true,
        animals: true,
        set_par: false,
        season_speed: 0,
        pause_calendar: false,
        freeze_moods: false,
        unlock_all: false,
        lefties: false,
        // it changes the balance: not part of the preset
        weather: false,
        // purely cosmetic and nothing changes until the player picks a style or places a lamp
        path_style: 1,
    };

    /// Everything off (the original game).
    pub fn is_off(&self) -> bool {
        *self == Extras::default()
    }

    /// The options as bits (the `X:` test hook and `--extras`): one bit per tick in `OPTION_KEYS` order, except that the
    /// calendar speed takes two (bits 6 and 7), so the ticks after it sit one bit further on (8, 9, 10, 11), and Path styles
    /// and lamps takes three (12 to 14). The first six keep the bits they always had.
    pub fn bits(&self) -> u32 {
        self.rotate as u32
            | (self.cut_content as u32) << 1
            | (self.hardcore as u32) << 2
            | (self.more_staff as u32) << 3
            | (self.animals as u32) << 4
            | (self.set_par as u32) << 5
            | (self.season_speed.min(2) as u32) << 6
            | (self.pause_calendar as u32) << 8
            | (self.freeze_moods as u32) << 9
            | (self.unlock_all as u32) << 10
            | (self.lefties as u32) << 11
            | (self.weather as u32) << 12
            | (self.path_style.min(PATH_STYLES) as u32) << 13
    }

    pub fn from_bits(b: u32) -> Extras {
        Extras {
            rotate: b & 1 != 0,
            cut_content: b & 2 != 0,
            hardcore: b & 4 != 0,
            more_staff: b & 8 != 0,
            animals: b & 16 != 0,
            set_par: b & 32 != 0,
            season_speed: ((b >> 6) & 3).min(2) as u8,
            pause_calendar: b & 1 << 8 != 0,
            freeze_moods: b & 1 << 9 != 0,
            unlock_all: b & 1 << 10 != 0,
            lefties: b & 1 << 11 != 0,
            weather: b & 1 << 12 != 0,
            path_style: ((b >> 13) & 7).min(PATH_STYLES as u32) as u8,
        }
    }

    /// Option `i` of `OPTION_KEYS`: 0 or 1 for a tick, the speed (0..3) for the calendar speed.
    pub fn option(&self, i: usize) -> u8 {
        match i {
            0 => self.rotate as u8,
            1 => self.cut_content as u8,
            2 => self.hardcore as u8,
            3 => self.more_staff as u8,
            4 => self.animals as u8,
            5 => self.set_par as u8,
            SEASON_SPEED => self.season_speed.min(2),
            7 => self.pause_calendar as u8,
            8 => self.freeze_moods as u8,
            9 => self.unlock_all as u8,
            LEFTIES => self.lefties as u8,
            WEATHER => self.weather as u8,
            PATHS => self.path_style.min(PATH_STYLES),
            _ => 0,
        }
    }

    /// Sets option `i` of `OPTION_KEYS` (a tick is on for any value but 0).
    pub fn set_option(&mut self, i: usize, v: u8) {
        let on = v != 0;
        match i {
            0 => self.rotate = on,
            1 => self.cut_content = on,
            2 => self.hardcore = on,
            3 => self.more_staff = on,
            4 => self.animals = on,
            5 => self.set_par = on,
            SEASON_SPEED => self.season_speed = v.min(2),
            7 => self.pause_calendar = on,
            8 => self.freeze_moods = on,
            9 => self.unlock_all = on,
            LEFTIES => self.lefties = on,
            WEATHER => self.weather = on,
            PATHS => self.path_style = v.min(PATH_STYLES),
            _ => {}
        }
    }

    /// How many values option `i` has (2 for a tick).
    pub fn choices(i: usize) -> u8 {
        if i == SEASON_SPEED {
            SPEED_KEYS.len() as u8
        } else if i == PATHS {
            PATH_STYLES + 1
        } else {
            2
        }
    }

    /// A click on option `i`: a tick flips, the calendar speed steps to the next (1x, 1/2x, 1/4x, 1x).
    pub fn step_option(&mut self, i: usize) {
        let v = self.option(i);
        self.set_option(i, (v + 1) % Self::choices(i));
    }

    /// Game ticks to one tick of the calendar: 1 (the original), 2 or 4.
    pub fn calendar_divisor(&self) -> u32 {
        1 << self.season_speed.min(2)
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

    /// A golfer's mood change: under hardcore a bad moment costs `hardcore_mood_penalty` more; with golfer happiness
    /// frozen a bad moment costs nothing (a good one still counts).
    pub fn mood_change(&self, d: i32) -> i32 {
        if self.freeze_moods && d < 0 {
            0
        } else if self.hardcore && d < 0 {
            d - crate::data::economy().hardcore_mood_penalty
        } else {
            d
        }
    }
}

/// The calendar under the Slower seasons and Pause the calendar extras (the port's own). The original has one clock: the
/// game tick (87 ms) is the date too, a month every 1024 ticks and a year (March to October) every 8192. With the extras the
/// game tick runs on as before (golfers, staff, weeds, wages and upkeep, which the exe charges every 1024/(difficulty+2)
/// game ticks, the light's day of Enhanced graphics) while the date takes one step every 2 or 4 game ticks, or none at all.
/// Whatever goes by the date waits for it: month starts (debt interest, the monthly reminders, the month's history entry,
/// the holes' statistics' monthly decay, the scripts' month hook and the goal checks), year ends (the board, the year-end
/// report, the Top 10, the ledger's year), the SGA's mid-year tournament offer, the dates shown (the dock, SimFotos,
/// messages, accomplishments) and the seasons of Enhanced graphics (blossom and autumn, winter snow). Things that measure
/// how long the player has played rather than the time of year stay on the game tick: the opening story and design tip, the
/// advisor's first year on Easy, the tutorial story pair and the famous golfers' challenges from the second year of play.
///
/// The date is the game tick less `lag`, the game ticks the calendar stood still for; with the extras off it stays 0 and
/// the date is the game tick, as in the exe. It is saved with the club (only when not 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Calendar {
    pub lag: u32,
    /// The last game tick stepped did not move the date (so a month start the date stands on is not taken again).
    #[serde(skip)]
    pub held: bool,
}

impl Calendar {
    pub fn is_zero(&self) -> bool {
        self.lag == 0
    }

    /// Game tick `tick` has come: whether the date moves with it (every tick at 1x, every second at 1/2x, every fourth at
    /// 1/4x, never while paused).
    pub fn step(&mut self, tick: u32, e: &Extras) -> bool {
        self.held = e.pause_calendar || !tick.is_multiple_of(e.calendar_divisor());
        if self.held {
            self.lag = self.lag.wrapping_add(1);
        }
        !self.held
    }

    /// The date (the exe's calendar tick) at game tick `tick`.
    pub fn date(&self, tick: u32) -> u32 {
        tick.wrapping_sub(self.lag)
    }

    /// Whether game tick `tick` (the last one stepped) began a month: the date moved onto a multiple of 1024.
    pub fn new_month(&self, tick: u32) -> bool {
        !self.held && self.date(tick) & 0x3ff == 0
    }

    /// Whether game tick `tick` (the last one stepped) ended a year: the date moved onto a multiple of 8192 past the start.
    pub fn new_year(&self, tick: u32) -> bool {
        let d = self.date(tick);
        !self.held && d > 0 && d & 0x1fff == 0
    }

    /// The share of a game tick the date moves (for what is drawn between ticks): 1, 1/2, 1/4 or 0 while paused.
    pub fn rate(e: &Extras) -> f32 {
        if e.pause_calendar {
            0.0
        } else {
            1.0 / e.calendar_divisor() as f32
        }
    }
}

/// The most employees any setting allows (the original keeps its walkers, the player's pro first, in 64 records).
pub const MAX_STAFF: usize = 63;

/// The pars the Set par extra offers (the game itself gives 2 to very short holes and 6 to very long ones).
pub const PAR_RANGE: std::ops::RangeInclusive<i32> = 2..=6;

impl crate::golfer::Club {
    /// The date (the calendar's tick, `Calendar`) at the club's game tick.
    pub fn date(&self) -> u32 {
        self.cal.date(self.tick)
    }

    /// The landmark types the player may place (bit per type): the ones the club owns, or all fourteen with the Unlock all
    /// extra (as a Sandbox Mode game has from its start).
    pub fn landmarks_shown(&self) -> u32 {
        if self.extras.unlock_all {
            self.landmarks_owned | 0x3fff
        } else {
            self.landmarks_owned
        }
    }

    /// The Left-handed golfers extra: whether golfer slot `g` is left-handed (never while the extra is off, so switching it
    /// off puts every golfer back to the original). The player's pro goes by `pro_left`, whoever holds his slot.
    pub fn left_handed(&self, g: usize) -> bool {
        self.extras.lefties && if g as i32 == self.gary { self.pro_left } else { self.g.get(g).is_some_and(|x| x.left) }
    }

    /// A shot shape's bend as golfer `g` swings it: a left-hander's draw (1) bends as a right-hander's fade (-1) and the
    /// other way round; straight and the other shots (0, 3, 4) are not bent.
    pub fn hand_bend(&self, g: usize, shape: i32) -> i32 {
        if self.left_handed(g) && shape.abs() == 1 {
            -shape
        } else {
            shape
        }
    }

    /// Whether the player may change hole `h`'s par now: the Set par extra is on, the hole is open, and no tournament is
    /// being played or prepared (the tournament sets its own pars).
    pub fn can_set_par(&self, h: usize) -> bool {
        self.extras.set_par && (1..19).contains(&h) && self.holes[h].par != 0 && self.game & crate::golfer::game::TOURNAMENT == 0
    }

    /// The Set par extra: hole `h` gets par `par` (in `PAR_RANGE`), the par the game gave it kept in `game_par` until the
    /// player sets that one again, the hole is rebuilt or the extra is switched off. Returns whether the par changed.
    pub fn set_par(&mut self, h: usize, par: i32) -> bool {
        if !self.can_set_par(h) || !PAR_RANGE.contains(&par) || self.holes[h].par == par {
            return false;
        }
        let hole = &mut self.holes[h];
        if hole.game_par == 0 {
            hole.game_par = hole.par;
        }
        hole.par = par;
        if hole.game_par == par {
            hole.game_par = 0;
        }
        true
    }

    /// The Set par extra switched off: every hole goes back to the par the game gave it.
    pub fn restore_pars(&mut self) {
        for hole in self.holes.iter_mut() {
            if hole.game_par != 0 {
                if hole.par != 0 {
                    hole.par = hole.game_par;
                }
                hole.game_par = 0;
            }
        }
    }
}

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
        for b in 0..64 {
            assert_eq!(Extras::from_bits(b).bits(), b);
        }
    }

    #[test]
    fn new_options_keep_the_old_bits_and_round_trip() {
        // the six first options keep their bits (saves and the X: test hook)
        assert_eq!(Extras { set_par: true, ..Default::default() }.bits(), 32);
        let all = Extras { season_speed: 2, pause_calendar: true, freeze_moods: true, unlock_all: true, ..Extras::from_bits(63) };
        assert_eq!(all.bits(), 63 | 2 << 6 | 7 << 8);
        assert_eq!(Extras::from_bits(all.bits()), all);
        for speed in 0..3 {
            let e = Extras { season_speed: speed, ..Default::default() };
            assert_eq!(Extras::from_bits(e.bits()), e);
        }
        // an option per line of the box, read and set by index
        let mut e = Extras::default();
        for i in 0..OPTION_KEYS.len() {
            e.step_option(i);
            assert_eq!(e.option(i), 1, "option {i}");
        }
        assert_eq!(e.season_speed, 1);
        e.step_option(SEASON_SPEED);
        assert_eq!(e.season_speed, 2);
        e.step_option(SEASON_SPEED);
        assert_eq!(e.season_speed, 0);
        // a save made with the new options off reads them off, and one from before them reads too
        let old: Extras = serde_json::from_str(r#"{"rotate":true}"#).unwrap();
        assert_eq!(old, Extras { rotate: true, ..Default::default() });
    }

    /// Steps a calendar through game ticks `ticks`, returning the game ticks that began a month and ended a year.
    fn run_calendar(cal: &mut Calendar, e: &Extras, ticks: std::ops::RangeInclusive<u32>) -> (Vec<u32>, Vec<u32>) {
        let (mut months, mut years) = (Vec::new(), Vec::new());
        for t in ticks {
            cal.step(t, e);
            if cal.new_month(t) {
                months.push(t);
            }
            if cal.new_year(t) {
                years.push(t);
            }
        }
        (months, years)
    }

    #[test]
    fn the_original_calendar_is_the_game_tick() {
        let mut cal = Calendar::default();
        let (months, years) = run_calendar(&mut cal, &Extras::default(), 1..=0x4000);
        assert_eq!(cal.lag, 0);
        assert_eq!(cal.date(0x4000), 0x4000);
        assert_eq!(months, (1..=16).map(|m| m * 1024).collect::<Vec<_>>());
        assert_eq!(years, vec![0x2000, 0x4000]);
        // what is saved: nothing while the calendar keeps up
        let club = crate::golfer::Club::default();
        assert!(!serde_json::to_string(&club).unwrap().contains("\"cal\""));
    }

    #[test]
    fn slower_seasons_scale_months_and_years() {
        for (speed, k) in [(1u8, 2u32), (2, 4)] {
            let e = Extras { season_speed: speed, ..Default::default() };
            let mut cal = Calendar::default();
            let (months, years) = run_calendar(&mut cal, &e, 1..=0x4000 * k);
            assert_eq!(cal.date(0x4000 * k), 0x4000, "speed {speed}");
            assert_eq!(months, (1..=16).map(|m| m * 1024 * k).collect::<Vec<_>>(), "speed {speed}");
            assert_eq!(years, vec![0x2000 * k, 0x4000 * k], "speed {speed}");
            assert_eq!(Calendar::rate(&e), 1.0 / k as f32);
        }
        // back to 1x halfway: the date goes on from where it was, at the original's pace
        let mut cal = Calendar::default();
        run_calendar(&mut cal, &Extras { season_speed: 1, ..Default::default() }, 1..=2048);
        assert_eq!(cal.date(2048), 1024);
        let (months, _) = run_calendar(&mut cal, &Extras::default(), 2049..=4096);
        assert_eq!(cal.date(4096), 3072);
        assert_eq!(months, vec![3072, 4096]);
        // the lag is saved with the club, and read back
        let mut club = crate::golfer::Club::default();
        club.cal = cal;
        let back: crate::golfer::Club = serde_json::from_str(&serde_json::to_string(&club).unwrap()).unwrap();
        assert_eq!(back.cal.lag, 1024);
    }

    #[test]
    fn a_paused_calendar_never_moves_nor_repeats_its_month() {
        let e = Extras { pause_calendar: true, ..Default::default() };
        let mut cal = Calendar::default();
        // paused right on a month start: that month is not begun again on every tick
        run_calendar(&mut cal, &Extras::default(), 1..=1024);
        let (months, years) = run_calendar(&mut cal, &e, 1025..=0x10000);
        assert!(months.is_empty() && years.is_empty());
        assert_eq!(cal.date(0x10000), 1024);
        assert_eq!(Calendar::rate(&e), 0.0);
        // pause wins over the slower speeds
        let e = Extras { pause_calendar: true, season_speed: 2, ..Default::default() };
        assert!(!cal.step(0x10004, &e));
    }

    #[test]
    fn the_economy_keeps_its_daily_pace_and_the_calendar_its_months() {
        use crate::economy::{Economy, Payroll, CLUB_PRO};
        let e = Extras { season_speed: 1, ..Default::default() };
        let staff = [Payroll { kind: CLUB_PRO, experienced: false }];
        let mut econ = Economy { start_cash: -10000.0, ..Default::default() };
        econ.init();
        let mut cal = Calendar::default();
        let mut rng = crate::land::ExeRng::from_clock(3);
        // Impossible (difficulty 3): a wage every 204 game ticks, whatever the calendar
        let mut wages = 0;
        for t in 1..=4096u32 {
            cal.step(t, &e);
            wages += econ.on_tick_dated(t, &cal, 3, 0, &staff, &mut rng)[0];
        }
        assert_eq!(wages, 3 * (4096 / 204));
        // two calendar months in 4096 game ticks: interest twice (2% of the debt each time), the ledger in year 0
        assert_eq!(econ.tick, 2048);
        assert_eq!(econ.year_index(), 0);
        // (at game tick 2048: 10 wages paid, -130 units, 2% is -2; at 4096: 20 wages, -162 units, -3)
        assert_eq!(econ.ledger[0][crate::economy::LEDGER_MAINTENANCE], -500.0);
        // the board waits for the calendar's year end: 16384 game ticks at 1/2x
        let mut econ = Economy { start_cash: -100.0, ..Default::default() };
        econ.init();
        let mut cal = Calendar::default();
        for t in 1..=0x2000u32 {
            cal.step(t, &e);
            econ.on_tick_dated(t, &cal, 1, 0, &[], &mut rng);
        }
        assert_eq!(econ.debt_stage, 0);
        for t in 0x2001..=0x4000u32 {
            cal.step(t, &e);
            econ.on_tick_dated(t, &cal, 1, 0, &[], &mut rng);
        }
        assert_eq!(econ.debt_stage, 1);
    }

    #[test]
    fn sandbox_switches() {
        let f = Extras { freeze_moods: true, ..Default::default() };
        assert_eq!(f.mood_change(-2), 0);
        assert_eq!(f.mood_change(2), 2);
        let fh = Extras { freeze_moods: true, hardcore: true, ..Default::default() };
        assert_eq!(fh.mood_change(-2), 0);
        // unlock all: every landmark type, the club's own kept
        let mut club = crate::golfer::Club::default();
        club.landmarks_owned = 1 << 3;
        assert_eq!(club.landmarks_shown(), 1 << 3);
        club.extras.unlock_all = true;
        assert_eq!(club.landmarks_shown(), 0x3fff);
        club.extras.unlock_all = false;
        assert_eq!(club.landmarks_shown(), 1 << 3, "turning it off takes nothing the club owns");
    }

    fn club_with_hole() -> crate::golfer::Club {
        let mut club = crate::golfer::Club::default();
        club.holes[1].par = 4;
        club.holes[1].length = 380;
        club
    }

    #[test]
    fn set_par_needs_the_extra_an_open_hole_and_no_tournament() {
        let mut club = club_with_hole();
        assert!(!club.set_par(1, 5));
        assert_eq!(club.holes[1].par, 4);
        club.extras.set_par = true;
        assert!(!club.set_par(2, 4), "hole 2 is not open");
        assert!(!club.set_par(1, 7), "outside the range");
        assert!(!club.set_par(1, 4), "no change");
        club.game |= crate::golfer::game::TOURNAMENT;
        assert!(!club.set_par(1, 5));
        club.game &= !crate::golfer::game::TOURNAMENT;
        assert!(club.set_par(1, 5));
        assert_eq!((club.holes[1].par, club.holes[1].game_par), (5, 4));
        // what everything else reads: the course's par and the score against it
        assert_eq!(club.par(1), 5);
        assert_eq!(crate::ratings::hole_stats(&club.holes[1]).cols[0].0, 3);
        assert!(club.set_par(1, 3));
        assert_eq!((club.holes[1].par, club.holes[1].game_par), (3, 4));
        // back to the game's own par: no override left
        assert!(club.set_par(1, 4));
        assert_eq!((club.holes[1].par, club.holes[1].game_par), (4, 0));
    }

    #[test]
    fn set_par_saved_only_when_set_and_undone_by_restore() {
        let mut club = club_with_hole();
        let plain = serde_json::to_string(&club.holes[1]).unwrap();
        assert!(!plain.contains("game_par"));
        club.extras.set_par = true;
        club.set_par(1, 6);
        let json = serde_json::to_string(&club.holes[1]).unwrap();
        let back: crate::golfer::Hole = serde_json::from_str(&json).unwrap();
        assert_eq!((back.par, back.game_par), (6, 4));
        let old: crate::golfer::Hole = serde_json::from_str(&plain).unwrap();
        assert_eq!((old.par, old.game_par), (4, 0));
        club.restore_pars();
        assert_eq!((club.holes[1].par, club.holes[1].game_par), (4, 0));
        assert_eq!(serde_json::to_string(&club.holes[1]).unwrap(), plain);
    }
}

/// What a cut landmark costs (units of $100; the port's choice, the original has none: about a mid-priced landmark).
pub const CUT_LANDMARK_UNITS: i32 = 60;

/// What a pair of animals costs (units of $100; the port's choice).
pub const ANIMAL_UNITS: i32 = 20;

// ---- Path styles and lamps (the port's own, purely cosmetic) -------------------------------------------------------------

/// The path styles' names ({VALUE} of the Path styles and lamps line), by `Extras::path_style` (0: the extra off).
pub const PATH_STYLE_KEYS: [&str; 6] = [
    "STR_EXTRAS_PATHS_OFF",
    "STR_EXTRAS_PATHS_ORIGINAL",
    "STR_EXTRAS_PATHS_BRICK",
    "STR_EXTRAS_PATHS_GRAVEL",
    "STR_EXTRAS_PATHS_STONE",
    "STR_EXTRAS_PATHS_BOARDWALK",
];

/// The highest `Extras::path_style`.
pub const PATH_STYLES: u8 = PATH_STYLE_KEYS.len() as u8 - 1;

/// The landmark "type" of the lamp post on the Path styles and lamps page of the landmark strip. It never becomes a
/// landmark object: placing one adds to `Club::lamps`.
pub const LAMP_TYPE: i32 = 200;

/// What a lamp post costs (units of $100; the port's choice, about a bench).
pub const LAMP_UNITS: i32 = 3;

/// The most lamp posts a course may have.
pub const MAX_LAMPS: usize = 256;

impl Extras {
    /// Path styles and lamps is on (lamps are drawn and may be placed).
    pub fn lamps(&self) -> bool {
        self.path_style != 0
    }

    /// The course's path material: 0 the original's (also with the extra off), 1 brick, 2 gravel, 3 dark stone, 4 boardwalk.
    pub fn path_material(&self) -> u8 {
        self.path_style.saturating_sub(1).min(PATH_STYLES - 1)
    }

    /// Tab with the Pathway tool: the next path style (the original's after the last). False with the extra off.
    pub fn next_path_style(&mut self) -> bool {
        if !self.lamps() {
            return false;
        }
        self.path_style = self.path_style % PATH_STYLES + 1;
        true
    }
}

impl crate::golfer::Club {
    /// The lamp post standing on tile (a, b), if any (an index into `lamps`).
    pub fn lamp_on(&self, a: i32, b: i32) -> Option<usize> {
        self.lamps.iter().position(|&(x, y)| (x >> 10, y >> 10) == (a, b))
    }

    /// Puts a lamp post at course point (x, y) (units of 1/1024 tile): one per tile, at most `MAX_LAMPS`. Returns whether
    /// it was put.
    pub fn add_lamp(&mut self, x: i32, y: i32) -> bool {
        if !self.extras.lamps() || self.lamps.len() >= MAX_LAMPS || self.lamp_on(x >> 10, y >> 10).is_some() {
            return false;
        }
        self.lamps.push((x, y));
        true
    }

    /// Takes the lamp post on tile (a, b) away. Returns whether there was one.
    pub fn remove_lamp(&mut self, a: i32, b: i32) -> bool {
        match self.lamp_on(a, b) {
            Some(i) => {
                self.lamps.remove(i);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn path_styles_step_and_keep_their_bits() {
        let mut e = Extras::default();
        assert!(!e.lamps() && !e.next_path_style());
        assert_eq!(e.path_material(), 0);
        e.step_option(PATHS);
        assert_eq!((e.path_style, e.path_material()), (1, 0));
        let seen: Vec<u8> = (0..5).map(|_| (e.next_path_style(), e.path_material()).1).collect();
        assert_eq!(seen, vec![1, 2, 3, 4, 0]);
        for v in 0..=PATH_STYLES {
            let e = Extras { path_style: v, lefties: true, ..Default::default() };
            assert_eq!(Extras::from_bits(e.bits()), e);
            assert_eq!(e.option(PATHS), v);
        }
        // the older options keep their bits
        assert_eq!(Extras { path_style: 3, ..Default::default() }.bits(), 3 << 13);
        assert_eq!(Extras::choices(PATHS), 6);
        let mut e = Extras { path_style: PATH_STYLES, ..Default::default() };
        e.step_option(PATHS);
        assert!(e.is_off());
    }

    #[test]
    fn lamps_save_only_when_placed_and_round_trip() {
        let mut club = crate::golfer::Club::default();
        let plain = serde_json::to_string(&club).unwrap();
        assert!(!plain.contains("lamps") && !plain.contains("path_style"));
        assert!(!club.add_lamp(1500, 2600), "the extra is off");
        club.extras.path_style = 3;
        assert!(club.add_lamp(1500, 2600));
        assert!(!club.add_lamp(1100, 2100), "one per tile");
        assert!(club.add_lamp(5000, 2600));
        assert_eq!(club.lamp_on(1, 2), Some(0));
        let json = serde_json::to_string(&club).unwrap();
        let back: crate::golfer::Club = serde_json::from_str(&json).unwrap();
        assert_eq!(back.lamps, vec![(1500, 2600), (5000, 2600)]);
        assert_eq!(back.extras.path_style, 3);
        // a save from before the extra reads as no lamps and the original's paths
        let old: crate::golfer::Club = serde_json::from_str(&plain).unwrap();
        assert!(old.lamps.is_empty() && old.extras.is_off());
        // the last lamp taken away and the extra off: the save is as it was
        assert!(club.remove_lamp(1, 2) && club.remove_lamp(4, 2) && !club.remove_lamp(4, 2));
        club.extras.path_style = 0;
        assert_eq!(serde_json::to_string(&club).unwrap(), plain);
    }
}
