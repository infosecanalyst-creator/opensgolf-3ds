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
//! * `more_visitors`: More visitors: touring pros and invented celebrities come to play now and then (sg_core::visitors,
//!   data/visitors.toml); pros pay more and rate the course, celebrities draw a few extra golfers.
//! * `no_weeds`: Sandbox: no new dandelions. Weeds no longer spread over the course (staff::spread_weeds is not run) and
//!   unhappy golfers no longer start them where they stand (Club's mood change), so
//!   a course needs no groundskeepers or landmarks to keep it clear (issue #105); the ones already there stay until
//!   pulled.
//! * `keep_land`: Keep the land offer: when the County commissioner approves an expansion, the approval waits until land
//!   is bought (`Club::land_kept`). Saying No to his question, closing it, or leaving the land screen without buying keeps
//!   it, and the Information menu's Buy land line opens the land screen whenever the player likes. In the original the
//!   approval is gone once the question is answered, though he comes again on a later visitor turn while the club's
//!   rating stays over his mark (vips.rs `vip_check`). At most one approval waits; buying a tract uses it.
//! * `second_course`: Second course: once Course A's 18 holes are open, the hole tool can build a Course B of up to 18
//!   more holes on the same property (sg_core::second); golfers play one course or the other. The SGA, tournaments,
//!   ratings and the original's reports keep to Course A.
//! * `tees`: Multiple tees: besides its own tee (the back tee), a hole may have a forward and a middle tee, placed with the
//!   Tees tool (`Club::place_extra_tee`), and ordinary golfers tee off from the one that suits their skill
//!   (`Tee::for_golfer`). The hole's length, par, statistics and ratings stay the back tee's, the hole as the original
//!   defines it, and tournaments play from the back tee. See `Tee`.
//!
//! Freeze happiness and Unlock all are named "Sandbox: ..." in the Extras box as they are what a sandbox player wants
//! most, but each works in a normal game too. The Extras box shows Slower seasons and Pause the calendar as one line,
//! *Calendar: normal / half speed / quarter speed / stopped* (`Extras::calendar`), over the same two stored fields, so
//! saves and settings keep their format and older versions read a newer save's calendar as it was meant.
//!
//! Of these, `rotate` (only the picture turns) and `path_style` (only the look of paths, and lamps for decoration) change
//! how the game looks, not how it plays: the Extras box lists them on its Looks page and the presets treat them as looks.

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
    pub more_visitors: bool,
    pub no_weeds: bool,
    pub keep_land: bool,
    pub second_course: bool,
    /// Multiple tees: a hole may have a forward and a middle tee besides its own (`Tee`). Saved only when on, so the
    /// extras of a game that never ticks it are written as before.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub tees: bool,
}

/// The options in the order the Extras box lists them (text keys of their lines). Option 6 has three values (a line with
/// {VALUE}, `SPEED_KEYS`), the others are ticks; `Extras::option` and `Extras::set_option` read and set them by index.
/// New options are appended so bits and saves keep working; the page each is shown on (Gameplay, Sandbox or Looks) is
/// extras_ui.rs `PAGE_LINES`'s.
pub const OPTION_KEYS: [&str; 18] = [
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
    "STR_EXTRAS_MORE_VISITORS",
    "STR_EXTRAS_NO_WEEDS",
    "STR_EXTRAS_KEEP_LAND",
    "STR_EXTRAS_SECOND_COURSE",
    "STR_EXTRAS_TEES",
];

/// The option of `OPTION_KEYS` that is Weather that matters.
pub const WEATHER: usize = 11;

/// The option of `OPTION_KEYS` that is Left-handed golfers.
pub const LEFTIES: usize = 10;

/// The option of `OPTION_KEYS` that is Path styles and lamps (several values: `PATH_STYLE_KEYS`).
pub const PATHS: usize = 12;
/// The option of `OPTION_KEYS` that is More visitors.
pub const MORE_VISITORS: usize = 13;
/// The option of `OPTION_KEYS` that is Sandbox: no new dandelions.
pub const NO_WEEDS: usize = 14;
/// The option of `OPTION_KEYS` that is Keep the land offer.
pub const KEEP_LAND: usize = 15;
/// The option of `OPTION_KEYS` that is Second course.
pub const SECOND_COURSE: usize = 16;
/// The option of `OPTION_KEYS` that is Multiple tees.
pub const TEES: usize = 17;

/// The option of `OPTION_KEYS` that is the calendar speed.
pub const SEASON_SPEED: usize = 6;

/// The calendar speeds' names ({VALUE} of the Slower seasons line), by `season_speed`.
pub const SPEED_KEYS: [&str; 3] = ["STR_EXTRAS_SPEED_1", "STR_EXTRAS_SPEED_HALF", "STR_EXTRAS_SPEED_QUARTER"];

/// The Calendar line's values ({VALUE} of STR_EXTRAS_CALENDAR), by `Extras::calendar`: normal (the original), half speed,
/// quarter speed, stopped.
pub const CALENDAR_KEYS: [&str; 4] =
    ["STR_EXTRAS_CALENDAR_NORMAL", "STR_EXTRAS_CALENDAR_HALF", "STR_EXTRAS_CALENDAR_QUARTER", "STR_EXTRAS_CALENDAR_STOPPED"];
/// The Calendar line's value for a stopped calendar (Pause the calendar).
pub const CALENDAR_STOPPED: u8 = 3;

impl Extras {
    /// The extras of the "Original + helpers" preset (the simgolf crate's extras_ui.rs `Preset`): only the two that change
    /// how the game looks, not how it plays (Tab turns a building's picture; path styles and lamps, with the original's
    /// paths until the player picks another style).
    pub const LOOKS: Extras = Extras {
        rotate: true,
        cut_content: false,
        hardcore: false,
        more_staff: false,
        animals: false,
        set_par: false,
        season_speed: 0,
        pause_calendar: false,
        freeze_moods: false,
        unlock_all: false,
        lefties: false,
        weather: false,
        path_style: 1,
        more_visitors: false,
        no_weeds: false,
        keep_land: false,
        second_course: false,
        tees: false,
    };

    /// The extras of the "OpenSGolf" preset, the game with OpenSGolf's own additions: the looks, higher staff limits,
    /// buyable animals, weather that matters, forward and middle tees and the second course. Not Hardcore, not the
    /// experimental cut content, not More visitors, Set par, Left-handed golfers, Keep the land offer or the sandbox
    /// switches.
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
        weather: true,
        // purely cosmetic and nothing changes until the player picks a style or places a lamp
        path_style: 1,
        more_visitors: false,
        no_weeds: false,
        // it changes when land can be bought: not part of the preset
        keep_land: false,
        // nothing changes until the first 18 holes are open and the player builds on
        second_course: true,
        // nothing changes until the player places a forward or middle tee
        tees: true,
    };

    /// Everything off (the original game).
    pub fn is_off(&self) -> bool {
        *self == Extras::default()
    }

    /// The options as bits (the `X:` test hook and `--extras`): one bit per tick in `OPTION_KEYS` order, except that the
    /// calendar speed takes two (bits 6 and 7), so the ticks after it sit one bit further on (8 to 12), Path styles and lamps
    /// takes three (13 to 15), More visitors is bit 16, Sandbox: no new dandelions bit 17, Keep the land offer bit 18,
    /// Second course bit 19 and Multiple tees bit 20. The first six keep the bits they always had.
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
            | (self.more_visitors as u32) << 16
            | (self.no_weeds as u32) << 17
            | (self.keep_land as u32) << 18
            | (self.second_course as u32) << 19
            | (self.tees as u32) << 20
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
            more_visitors: b & 1 << 16 != 0,
            no_weeds: b & 1 << 17 != 0,
            keep_land: b & 1 << 18 != 0,
            second_course: b & 1 << 19 != 0,
            tees: b & 1 << 20 != 0,
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
            MORE_VISITORS => self.more_visitors as u8,
            NO_WEEDS => self.no_weeds as u8,
            KEEP_LAND => self.keep_land as u8,
            SECOND_COURSE => self.second_course as u8,
            TEES => self.tees as u8,
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
            MORE_VISITORS => self.more_visitors = on,
            NO_WEEDS => self.no_weeds = on,
            KEEP_LAND => self.keep_land = on,
            SECOND_COURSE => self.second_course = on,
            TEES => self.tees = on,
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

    /// The Calendar line of the Extras box: 0 normal (the original), 1 half speed, 2 quarter speed, 3 stopped
    /// (`CALENDAR_STOPPED`). A stopped calendar wins over the speed, as `Calendar::step` has it, so a game saved with both
    /// Slower seasons and Pause the calendar on reads as stopped.
    pub fn calendar(&self) -> u8 {
        if self.pause_calendar {
            CALENDAR_STOPPED
        } else {
            self.season_speed.min(2)
        }
    }

    /// Sets the Calendar line's value: stopped is Pause the calendar with the speed back at normal; the speeds untick the
    /// pause. Only the two stored fields change, so the save's format is the one older versions read.
    pub fn set_calendar(&mut self, v: u8) {
        if v >= CALENDAR_STOPPED {
            self.pause_calendar = true;
            self.season_speed = 0;
        } else {
            self.pause_calendar = false;
            self.season_speed = v;
        }
    }

    /// A click on the Calendar line: normal, half, quarter, stopped, normal.
    pub fn step_calendar(&mut self) {
        self.set_calendar((self.calendar() + 1) % CALENDAR_KEYS.len() as u8);
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
    /// The last game tick stepped did not move the date (so a month start the date stands on is not taken again). Saved
    /// while set, as the next game tick's golfers read it before the calendar steps again (older saves: not held).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
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
        // the hold is saved with the club while set, so a loaded game does not begin the month the date stands on again
        let mut club = crate::golfer::Club::default();
        club.cal = cal;
        let json = serde_json::to_string(&club).unwrap();
        let back: crate::golfer::Club = serde_json::from_str(&json).unwrap();
        assert!(back.cal.held && !back.cal.new_month(0x10000));
        // not held: nothing more is written (a Classic save has no calendar at all)
        club.cal.held = false;
        assert!(!serde_json::to_string(&club.cal).unwrap().contains("held"));
        assert!(!serde_json::to_string(&crate::golfer::Club::default()).unwrap().contains("\"cal\""));
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
    fn the_calendar_line_reads_and_writes_the_two_stored_fields() {
        // old saves with either bit read as the matching value, and with both as stopped (the pause wins, as it plays)
        for (bits, want) in [(0u32, 0u8), (1 << 6, 1), (2 << 6, 2), (1 << 8, 3), (1 << 8 | 2 << 6, 3), (1 << 8 | 1 << 6, 3)] {
            assert_eq!(Extras::from_bits(bits).calendar(), want, "bits {bits:#x}");
        }
        let old: Extras = serde_json::from_str(r#"{"season_speed":2,"pause_calendar":true}"#).unwrap();
        assert_eq!(old.calendar(), CALENDAR_STOPPED);
        assert!(!Calendar::default().step(4, &old), "plays stopped");
        // each value is stored as the fields older versions know
        let mut e = Extras::default();
        let mut seen = Vec::new();
        for _ in 0..4 {
            e.step_calendar();
            seen.push((e.calendar(), e.season_speed, e.pause_calendar));
        }
        assert_eq!(seen, vec![(1, 1, false), (2, 2, false), (3, 0, true), (0, 0, false)]);
        assert!(e.is_off());
        for v in 0..4 {
            let mut e = Extras::default();
            e.set_calendar(v);
            assert_eq!(Extras::from_bits(e.bits()).calendar(), v);
            // the date moves as the two old ticks made it move
            assert_eq!(Calendar::rate(&e), [1.0, 0.5, 0.25, 0.0][v as usize]);
        }
        // the presets leave the calendar alone (the original's)
        assert_eq!((Extras::LOOKS.calendar(), Extras::OPENSGOLF.calendar()), (0, 0));
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

    #[test]
    fn second_course_has_its_own_bit_and_is_only_in_the_opensgolf_preset() {
        let e = Extras { second_course: true, ..Default::default() };
        assert_eq!(e.bits(), 1 << 19);
        assert_eq!(Extras::from_bits(e.bits()), e);
        assert_eq!(e.option(SECOND_COURSE), 1);
        const { assert!(Extras::OPENSGOLF.second_course && !Extras::LOOKS.second_course, "only in OpenSGolf") };
    }

    #[test]
    fn keep_the_land_offer_has_its_own_bit_and_saves_its_approval_only_while_it_waits() {
        let e = Extras { keep_land: true, ..Default::default() };
        assert_eq!(e.bits(), 1 << 18);
        assert_eq!(Extras::from_bits(e.bits()), e);
        assert_eq!(e.option(KEEP_LAND), 1);
        const { assert!(!Extras::OPENSGOLF.keep_land && !Extras::LOOKS.keep_land, "in no preset") };
        let mut club = crate::golfer::Club::default();
        let plain = serde_json::to_string(&club).unwrap();
        assert!(!plain.contains("land_kept"));
        club.land_kept = true;
        let back: crate::golfer::Club = serde_json::from_str(&serde_json::to_string(&club).unwrap()).unwrap();
        assert!(back.land_kept);
        // a save from before the extra has none waiting
        let old: crate::golfer::Club = serde_json::from_str(&plain).unwrap();
        assert!(!old.land_kept);
    }
}

// ---- Multiple tees (the port's own) ---------------------------------------------------------------------------------------

/// One of a hole's tees under the Multiple tees extra. `Back` is the hole's own tee, the one the original has: the hole's
/// length, par, statistics, ratings and the SGA's report all stay the back tee's, and tournaments always play from it. A
/// hole may also have a `Forward` and a `Middle` tee (`Hole::front_tee`, `Hole::middle_tee`), each one tee tile placed with
/// the Tees tool (Tab steps the tool from the hole's tee to the forward tee and the middle tee) nearer the cup than the
/// back tee, the forward tee nearer than the middle one. Ordinary golfers play from the tee of their skill
/// (`Tee::for_golfer`); a missing forward tee sends its golfers to the middle tee, and a missing middle tee to the back
/// tee. With the extra off, or on a hole without either, every golfer tees off as in the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tee {
    Back,
    Middle,
    Forward,
}

/// How far (tiles) a forward or middle tee may be from the line from its hole's back tee to the cup.
pub const TEE_REACH: i32 = 6;

/// Why the Tees tool did not place a forward or middle tee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeeRefusal {
    /// The extra is off, or the tool is placing the hole's own tee.
    Off,
    /// Tees cannot change during a tournament.
    Tournament,
    /// No hole with a tee and a cup passes near enough, with its back tee farther from the cup than this tile.
    NoHole,
    /// The tile is a tee or a green already.
    Taken,
    /// The hole has this tee already (paint over it to move it).
    Has(usize),
    /// A forward tee must be nearer the cup than the hole's middle tee, a middle tee farther than its forward tee.
    Order(usize),
}

impl Tee {
    /// The tee an ordinary golfer plays from, by his skill: his level (the game's own 0 to 2 for each golfer, the exe's
    /// "tee set" byte, which on Normal and harder adds to his driving distance) plus one for each of the Length, Accuracy
    /// and Imagination classes he has beyond the first. Skill 0 or 1 plays the forward tee, 2 the middle tee, 3 and up the
    /// back tee. Pros, VIPs and the player's pro (any golfer of a kind other than ordinary) play the back tee.
    pub fn for_golfer(g: &crate::golfer::Golfer) -> Tee {
        if g.kind != 0 {
            return Tee::Back;
        }
        let skill = g.level.clamp(0, 2) + ((g.class & 7).count_ones() as i32 - 1).max(0);
        match skill {
            ..=1 => Tee::Forward,
            2 => Tee::Middle,
            _ => Tee::Back,
        }
    }

    /// The tee's name ({TEE} of the Tees tool's words).
    pub fn key(self) -> &'static str {
        match self {
            Tee::Back => "STR_TEE_BACK",
            Tee::Middle => "STR_TEE_MIDDLE",
            Tee::Forward => "STR_TEE_FORWARD",
        }
    }

    /// The Tees tool's step (Tab with the extra on): 0 the hole's own tee (the original's tool), 1 forward, 2 middle.
    pub fn of_step(step: u8) -> Tee {
        match step % 3 {
            1 => Tee::Forward,
            2 => Tee::Middle,
            _ => Tee::Back,
        }
    }
}

fn dist2(p: (i32, i32), q: (i32, i32)) -> i64 {
    let (dx, dy) = ((p.0 - q.0) as i64, (p.1 - q.1) as i64);
    dx * dx + dy * dy
}

/// Squared distance (tiles) from p to the segment from a to b, scaled by |ab|^2 to stay in integers.
fn seg_dist2_scaled(p: (i32, i32), a: (i32, i32), b: (i32, i32)) -> (i64, i64) {
    let l = dist2(a, b).max(1);
    let (abx, aby) = ((b.0 - a.0) as i64, (b.1 - a.1) as i64);
    let (apx, apy) = ((p.0 - a.0) as i64, (p.1 - a.1) as i64);
    let t = (apx * abx + apy * aby).clamp(0, l);
    // p - (a + ab * t / l), times l
    let (qx, qy) = (apx * l - abx * t, apy * l - aby * t);
    (qx * qx + qy * qy, l * l)
}

impl crate::golfer::Hole {
    /// The hole's forward or middle tee (none for the back tee, which is `back`).
    pub fn extra_tee(&self, t: Tee) -> Option<(i32, i32)> {
        match t {
            Tee::Back => None,
            Tee::Middle => self.middle_tee,
            Tee::Forward => self.front_tee,
        }
    }

    fn extra_tee_mut(&mut self, t: Tee) -> Option<&mut Option<(i32, i32)>> {
        match t {
            Tee::Back => None,
            Tee::Middle => Some(&mut self.middle_tee),
            Tee::Forward => Some(&mut self.front_tee),
        }
    }

    /// The facing of the hole's forward or middle tee on tile `at`: toward the hole's cup (none for any other tile).
    pub fn extra_tee_facing(&self, at: (i32, i32)) -> Option<i32> {
        self.extra_tees().find(|&(_, p)| p == at)?;
        Some(crate::geom::dir8(crate::geom::angle(self.pin.0 - at.0, self.pin.1 - at.1)))
    }

    /// The hole's forward and middle tees that are set, with which each is.
    pub fn extra_tees(&self) -> impl Iterator<Item = (Tee, (i32, i32))> + '_ {
        [Tee::Forward, Tee::Middle].into_iter().filter_map(|t| self.extra_tee(t).map(|p| (t, p)))
    }
}

impl crate::golfer::Club {
    /// Where golfer `me` tees off on hole `h` under the Multiple tees extra: the forward or middle tee of his skill
    /// (`Tee::for_golfer`; a missing forward tee falls back to the middle one). None, the original's choice, with the extra
    /// off, during a tournament, for the back tee's golfers and on a hole without the tee wanted.
    pub fn extra_tee_for(&self, me: &crate::golfer::Golfer, h: i32) -> Option<(i32, i32)> {
        if !self.extras.tees || self.game & crate::golfer::game::TOURNAMENT != 0 || !(1..19).contains(&h) {
            return None;
        }
        let hr = &self.holes[h as usize];
        match Tee::for_golfer(me) {
            Tee::Back => None,
            Tee::Middle => hr.middle_tee,
            Tee::Forward => hr.front_tee.or(hr.middle_tee),
        }
    }

    /// The hole and tee whose forward or middle tee is on tile (a, b), if any, on the course shown (`holes`).
    pub fn extra_tee_at(&self, a: i32, b: i32) -> Option<(usize, Tee)> {
        extra_tee_in(&self.holes, a, b)
    }

    /// The hole a forward or middle tee on tile (a, b) would belong to, on the course shown: of the holes with a tee and a
    /// cup (open, or the hole being built), the one whose line from the back tee to the cup passes nearest the tile, no
    /// farther than `TEE_REACH` tiles, with its back tee farther from the cup than the tile.
    pub fn extra_tee_hole(&self, a: i32, b: i32) -> Option<usize> {
        tee_hole_in(&self.holes, self.next_hole, a, b)
    }

    /// Whether the Tees tool may put tee `t` on tile (a, b) of type `old`, and for which hole of the course shown
    /// (`place_extra_tee`'s checks, without placing it).
    pub fn extra_tee_check(&self, c: &crate::course::Course, t: Tee, a: i32, b: i32, old: u8) -> Result<usize, TeeRefusal> {
        self.extra_tee_check_on(self.b_shown(), c, t, a, b, old)
    }

    /// `extra_tee_check` on Course B (`course_b`, the Second course extra: sg_core::second) or Course A, whichever is
    /// shown (the pointer's label asks without swapping the records).
    pub fn extra_tee_check_on(
        &self,
        course_b: bool,
        c: &crate::course::Course,
        t: Tee,
        a: i32,
        b: i32,
        old: u8,
    ) -> Result<usize, TeeRefusal> {
        use crate::course::{idx, inside, t as ty};
        if !self.extras.tees || t == Tee::Back {
            return Err(TeeRefusal::Off);
        }
        if self.game & crate::golfer::game::TOURNAMENT != 0 {
            return Err(TeeRefusal::Tournament);
        }
        if !inside(a, b) || old == ty::TEE || old == ty::GREEN || c.flags[idx(a, b)] & crate::course::f::CUP != 0 {
            return Err(TeeRefusal::Taken);
        }
        let holes = self.course_holes(course_b);
        let h = tee_hole_in(holes, self.course_next_hole(course_b), a, b).ok_or(TeeRefusal::NoHole)?;
        let hr = &holes[h];
        if hr.extra_tee(t).is_some() {
            return Err(TeeRefusal::Has(h));
        }
        let d = dist2((a, b), hr.pin);
        let ordered = match t {
            Tee::Forward => hr.middle_tee.is_none_or(|m| d < dist2(m, hr.pin)),
            _ => hr.front_tee.is_none_or(|f| d > dist2(f, hr.pin)),
        };
        if !ordered {
            return Err(TeeRefusal::Order(h));
        }
        Ok(h)
    }

    /// The Tees tool with the Multiple tees extra stepped to the forward or middle tee: tile (a, b), of type `old`, becomes
    /// that tee of the hole `extra_tee_hole` finds on the course shown. The tile carries the hole's number as its other
    /// tees do. Returns the hole; the caller paints the tile a tee and charges for it.
    pub fn place_extra_tee(&mut self, c: &mut crate::course::Course, t: Tee, a: i32, b: i32, old: u8) -> Result<usize, TeeRefusal> {
        let h = self.extra_tee_check(c, t, a, b, old)?;
        if let Some(slot) = self.holes[h].extra_tee_mut(t) {
            *slot = Some((a, b));
        }
        let i = crate::course::idx(a, b);
        c.flags[i] = (c.flags[i] & !crate::course::f::LOW) | h as u16;
        Ok(h)
    }

    /// Takes the forward or middle tee on tile (a, b) from its hole, of either course (the tile is being painted over).
    /// Returns whether there was one. The hole stays open: only its own tee and cup green take it out of play.
    pub fn remove_extra_tee(&mut self, a: i32, b: i32) -> bool {
        let shown = self.b_shown();
        for course_b in [shown, !shown] {
            let Some((h, t)) = extra_tee_in(self.course_holes(course_b), a, b) else { continue };
            if let Some(slot) = self.course_hole_mut(course_b, h).and_then(|r| r.extra_tee_mut(t)) {
                *slot = None;
            }
            return true;
        }
        false
    }

    /// After the course changed (`sync_course`): a forward or middle tee whose tile is no longer a tee (an undo, a course
    /// edit) is dropped, and with the extra on the tiles of the others carry their hole's number. Both courses (the Second
    /// course extra's Course B too); nothing is done on a course that has none.
    pub fn sync_extra_tees(&mut self, c: &mut crate::course::Course) {
        let on = self.extras.tees;
        sync_tees_in(&mut self.holes, c, on);
        if let Some(cb) = self.course_b.as_mut() {
            sync_tees_in(&mut cb.holes, c, on);
        }
    }
}

fn extra_tee_in(holes: &[crate::golfer::Hole], a: i32, b: i32) -> Option<(usize, Tee)> {
    (1..19.min(holes.len())).find_map(|h| holes[h].extra_tees().find(|&(_, p)| p == (a, b)).map(|(t, _)| (h, t)))
}

fn tee_hole_in(holes: &[crate::golfer::Hole], next_hole: i32, a: i32, b: i32) -> Option<usize> {
    let p = (a, b);
    let mut best: Option<(usize, (i64, i64))> = None;
    for (h, hr) in holes.iter().enumerate().take(19).skip(1) {
        if hr.back.0 == 0 || hr.pin.0 == 0 || (hr.par == 0 && h as i32 != next_hole) || p == hr.back {
            continue;
        }
        if dist2(p, hr.pin) >= dist2(hr.back, hr.pin) {
            continue;
        }
        let (d, l) = seg_dist2_scaled(p, hr.back, hr.pin);
        if d > (TEE_REACH as i64 * TEE_REACH as i64) * l {
            continue;
        }
        // nearest first (d / l against the best's, cross-multiplied), the lower hole number on a tie
        if best.is_none_or(|(_, (bd, bl))| (d as i128) * (bl as i128) < (bd as i128) * (l as i128)) {
            best = Some((h, (d, l)));
        }
    }
    best.map(|(h, _)| h)
}

fn sync_tees_in(holes: &mut [crate::golfer::Hole], c: &mut crate::course::Course, on: bool) {
    use crate::course::{idx, inside, t as ty};
    for (h, hr) in holes.iter_mut().enumerate().take(19).skip(1) {
        for t in [Tee::Forward, Tee::Middle] {
            let Some((a, b)) = hr.extra_tee(t) else { continue };
            if !inside(a, b) || c.ty[idx(a, b)] != ty::TEE || (a, b) == hr.back || (a, b) == hr.fwd {
                if let Some(slot) = hr.extra_tee_mut(t) {
                    *slot = None;
                }
                continue;
            }
            if on {
                let i = idx(a, b);
                c.flags[i] = (c.flags[i] & !crate::course::f::LOW) | h as u16;
            }
        }
    }
}

#[cfg(test)]
mod tee_tests {
    use super::*;
    use crate::course::{idx, t as ty, Course};
    use crate::golfer::{Club, Golfer, Hole};

    /// A club with hole 1 open from the back tee at (10, 10) to the cup at (10, 30), on a course of grass with those tiles
    /// a tee and a green.
    fn club_and_course() -> (Club, Course) {
        let mut club = Club::default();
        club.holes[1] = Hole { par: 4, length: 500, back: (10, 10), fwd: (10, 10), pin: (10, 30), ..club.holes[1].clone() };
        club.next_hole = 2;
        let mut c = Course::default();
        c.ty[idx(10, 10)] = ty::TEE;
        c.ty[idx(10, 30)] = ty::GREEN;
        c.flags[idx(10, 30)] |= crate::course::f::CUP | 1;
        (club, c)
    }

    fn golfer(class: u8, level: i32, kind: u8) -> Golfer {
        Golfer { class, level, kind, ..Default::default() }
    }

    #[test]
    fn golfers_choose_a_tee_by_skill() {
        // two skill classes: level 0 forward, 1 middle, 2 back
        assert_eq!(Tee::for_golfer(&golfer(3, 0, 0)), Tee::Forward);
        assert_eq!(Tee::for_golfer(&golfer(6, 1, 0)), Tee::Middle);
        assert_eq!(Tee::for_golfer(&golfer(5, 2, 0)), Tee::Back);
        // all three: one tee further back
        assert_eq!(Tee::for_golfer(&golfer(7, 0, 0)), Tee::Middle);
        assert_eq!(Tee::for_golfer(&golfer(7, 1, 0)), Tee::Back);
        // a single class (a person of a fixed class) plays as two at the same level, and the wager nibble is no skill
        assert_eq!(Tee::for_golfer(&golfer(1, 1, 0)), Tee::Forward);
        assert_eq!(Tee::for_golfer(&golfer(0x23, 1, 0)), Tee::Middle);
        // pros, VIPs and the player's pro: the back tee whatever their numbers
        for kind in [0x20, 0x24, 0x40, 0x60, 0x80] {
            assert_eq!(Tee::for_golfer(&golfer(3, 0, kind)), Tee::Back);
        }
    }

    #[test]
    fn the_tee_played_follows_the_skill_the_extra_and_the_tournament() {
        let (mut club, mut c) = club_and_course();
        let weak = golfer(3, 0, 0);
        let average = golfer(3, 1, 0);
        let strong = golfer(7, 2, 0);
        // nothing placed, or the extra off: the original's choice
        club.extras.tees = true;
        assert_eq!(club.extra_tee_for(&weak, 1), None);
        assert_eq!(club.place_extra_tee(&mut c, Tee::Middle, 10, 20, ty::ROUGH), Ok(1));
        assert_eq!(c.flags[idx(10, 20)] & crate::course::f::LOW, 1, "the tile carries the hole's number");
        // the forward tee's golfers go to the middle tee while there is no forward tee
        assert_eq!(club.extra_tee_for(&weak, 1), Some((10, 20)));
        assert_eq!(club.extra_tee_for(&average, 1), Some((10, 20)));
        assert_eq!(club.extra_tee_for(&strong, 1), None);
        assert_eq!(club.place_extra_tee(&mut c, Tee::Forward, 10, 24, ty::ROUGH), Ok(1));
        assert_eq!(club.extra_tee_for(&weak, 1), Some((10, 24)));
        assert_eq!(club.extra_tee_for(&average, 1), Some((10, 20)));
        // tournaments play from the back tee, and so does everybody with the extra off
        club.game |= crate::golfer::game::TOURNAMENT;
        assert_eq!(club.extra_tee_for(&weak, 1), None);
        club.game &= !crate::golfer::game::TOURNAMENT;
        club.extras.tees = false;
        assert_eq!(club.extra_tee_for(&weak, 1), None);
        // the hole as the original defines it is untouched
        let h = &club.holes[1];
        assert_eq!((h.par, h.length, h.back, h.fwd, h.pin), (4, 500, (10, 10), (10, 10), (10, 30)));
    }

    #[test]
    fn extra_tees_go_in_front_of_the_back_tee_of_a_near_hole_in_order() {
        let (mut club, mut c) = club_and_course();
        assert_eq!(club.extra_tee_check(&c, Tee::Forward, 10, 20, ty::ROUGH), Err(TeeRefusal::Off));
        club.extras.tees = true;
        assert_eq!(club.extra_tee_check(&c, Tee::Back, 10, 20, ty::ROUGH), Err(TeeRefusal::Off));
        // behind the back tee, too far from the line, on the green or a tee: no
        assert_eq!(club.extra_tee_check(&c, Tee::Forward, 10, 8, ty::ROUGH), Err(TeeRefusal::NoHole));
        assert_eq!(club.extra_tee_check(&c, Tee::Forward, 10 + TEE_REACH + 1, 20, ty::ROUGH), Err(TeeRefusal::NoHole));
        assert_eq!(club.extra_tee_check(&c, Tee::Forward, 10 + TEE_REACH, 20, ty::ROUGH), Ok(1));
        assert_eq!(club.extra_tee_check(&c, Tee::Forward, 10, 30, ty::GREEN), Err(TeeRefusal::Taken));
        assert_eq!(club.extra_tee_check(&c, Tee::Forward, 10, 10, ty::TEE), Err(TeeRefusal::Taken));
        // the forward tee nearer the cup than the middle one
        assert_eq!(club.place_extra_tee(&mut c, Tee::Forward, 10, 22, ty::ROUGH), Ok(1));
        assert_eq!(club.extra_tee_check(&c, Tee::Forward, 10, 25, ty::ROUGH), Err(TeeRefusal::Has(1)));
        assert_eq!(club.extra_tee_check(&c, Tee::Middle, 10, 25, ty::ROUGH), Err(TeeRefusal::Order(1)));
        assert_eq!(club.extra_tee_check(&c, Tee::Middle, 10, 16, ty::ROUGH), Ok(1));
        // a tournament keeps the tees as they are
        club.game |= crate::golfer::game::TOURNAMENT;
        assert_eq!(club.extra_tee_check(&c, Tee::Middle, 10, 16, ty::ROUGH), Err(TeeRefusal::Tournament));
        club.game &= !crate::golfer::game::TOURNAMENT;
        // of two holes, the one whose line passes nearer
        club.holes[2] = Hole { par: 3, back: (14, 12), fwd: (14, 12), pin: (14, 22), ..club.holes[2].clone() };
        club.next_hole = 3;
        assert_eq!(club.extra_tee_hole(13, 16), Some(2));
        assert_eq!(club.extra_tee_hole(11, 16), Some(1));
        // a closed hole other than the one being built takes none
        club.holes[2].par = 0;
        assert_eq!(club.extra_tee_hole(13, 16), Some(1));
    }

    #[test]
    fn painting_over_an_extra_tee_takes_only_that_tee_and_the_hole_stays_open() {
        let (mut club, mut c) = club_and_course();
        club.extras.tees = true;
        club.place_extra_tee(&mut c, Tee::Forward, 10, 22, ty::ROUGH).unwrap();
        c.ty[idx(10, 22)] = ty::TEE;
        assert_eq!(club.extra_tee_at(10, 22), Some((1, Tee::Forward)));
        assert!(club.paint_hole_tile(&mut c, 10, 22, ty::TEE, ty::FAIRWAY, false));
        assert_eq!(club.holes[1].front_tee, None);
        assert_eq!(club.holes[1].par, 4, "the hole stays open");
        assert_eq!(club.next_hole, 2);
        assert_eq!(c.flags[idx(10, 22)] & crate::course::f::LOW, 0);
        // a green painted over it is no cup of a new hole either
        club.place_extra_tee(&mut c, Tee::Middle, 10, 18, ty::ROUGH).unwrap();
        c.ty[idx(10, 18)] = ty::TEE;
        assert!(club.paint_hole_tile(&mut c, 10, 18, ty::TEE, ty::GREEN, false));
        assert_eq!((club.holes[1].middle_tee, club.holes[1].par, club.holes[1].back), (None, 4, (10, 10)));
        // a tee tile that stopped being a tee (an undo) is dropped on the next sync
        club.place_extra_tee(&mut c, Tee::Middle, 10, 16, ty::ROUGH).unwrap();
        club.sync_extra_tees(&mut c);
        assert_eq!(club.holes[1].middle_tee, None);
    }

    #[test]
    fn course_b_holes_take_extra_tees_and_its_golfers_choose_them_alike() {
        let (mut club, mut c) = club_and_course();
        club.extras.tees = true;
        club.extras.second_course = true;
        let mut cb = crate::second::SecondCourse::new();
        cb.holes[1] = Hole { par: 4, length: 400, back: (30, 10), fwd: (30, 10), pin: (30, 26), ..cb.holes[1].clone() };
        cb.next_hole = 2;
        club.course_b = Some(cb);
        c.ty[idx(30, 10)] = ty::TEE;
        // with Course A shown the tile is no hole's: Course A's hole 1 is far
        assert_eq!(club.extra_tee_check(&c, Tee::Forward, 30, 20, ty::ROUGH), Err(TeeRefusal::NoHole));
        assert_eq!(club.extra_tee_check_on(true, &c, Tee::Forward, 30, 20, ty::ROUGH), Ok(1));
        assert_eq!(club.on_course(true, |cl| cl.place_extra_tee(&mut c, Tee::Forward, 30, 20, ty::ROUGH)), Ok(1));
        assert_eq!(club.course_holes(true)[1].front_tee, Some((30, 20)));
        assert_eq!(club.holes[1].front_tee, None, "Course A's hole 1 is untouched");
        c.ty[idx(30, 20)] = ty::TEE;
        assert!(club.tile_course(1, 30, 20), "the tile is Course B's");
        // a Course B golfer (Course B shown around his update) plays it by the same rule
        let weak = golfer(3, 0, 0);
        assert_eq!(club.on_course(true, |cl| cl.extra_tee_for(&weak, 1)), Some((30, 20)));
        assert_eq!(club.extra_tee_for(&weak, 1), None);
        // the sync keeps it and numbers its tile; painting over it with Course A shown still takes it from Course B
        club.sync_extra_tees(&mut c);
        assert_eq!(c.flags[idx(30, 20)] & crate::course::f::LOW, 1);
        assert!(club.paint_hole_tile(&mut c, 30, 20, ty::TEE, ty::FAIRWAY, false));
        assert_eq!(club.course_holes(true)[1].front_tee, None);
        assert_eq!((club.course_holes(true)[1].par, club.holes[1].par), (4, 4));
    }

    #[test]
    fn saves_without_extra_tees_are_as_before_and_extra_tees_round_trip() {
        let (mut club, mut c) = club_and_course();
        let plain = serde_json::to_string(&club).unwrap();
        assert!(!plain.contains("front_tee") && !plain.contains("middle_tee") && !plain.contains("tees"));
        // the extra on with nothing placed changes nothing of the holes
        let hole = serde_json::to_string(&club.holes[1]).unwrap();
        club.extras.tees = true;
        assert_eq!(serde_json::to_string(&club.holes[1]).unwrap(), hole);
        // an older save (no such fields) reads as no extra tees and the extra off
        let old: Club = serde_json::from_str(&plain).unwrap();
        assert!(old.holes.iter().all(|h| h.front_tee.is_none() && h.middle_tee.is_none()));
        assert!(!old.extras.tees && old.extras.is_off());
        assert_eq!(serde_json::to_string(&old).unwrap(), plain, "an old save writes back unchanged");
        // the tees placed are kept with the game
        club.place_extra_tee(&mut c, Tee::Forward, 10, 24, ty::ROUGH).unwrap();
        club.place_extra_tee(&mut c, Tee::Middle, 10, 18, ty::ROUGH).unwrap();
        let back: Club = serde_json::from_str(&serde_json::to_string(&club).unwrap()).unwrap();
        assert_eq!((back.holes[1].front_tee, back.holes[1].middle_tee), (Some((10, 24)), Some((10, 18))));
        assert!(back.extras.tees);
        // the bit and the option
        let e = Extras { tees: true, ..Default::default() };
        assert_eq!(e.bits(), 1 << 20);
        assert_eq!(Extras::from_bits(e.bits()), e);
        assert_eq!(e.option(TEES), 1);
        const { assert!(Extras::OPENSGOLF.tees && !Extras::LOOKS.tees, "only in OpenSGolf") };
        // other extras' saves do not gain the tick while it is off
        let other = serde_json::to_string(&Extras { weather: true, ..Default::default() }).unwrap();
        assert!(!other.contains("tees"));
    }
}
