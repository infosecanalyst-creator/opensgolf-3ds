//! A minimal club economy.
//!
//! WHAT THE ORIGINAL MANUAL SAYS (no amounts are given anywhere in it): the player starts with a fixed allotment of cash that depends
//! on the property chosen and on the difficulty (Easy, Moderate, Difficult, Impossible); Sandbox mode has unlimited funds; a golfer
//! pays a green fee when completing a hole, and it is the primary income; Top 100 / Top 18 holes earn higher fees and an Airstrip
//! raises fees; building lots earn income; prize money from challenges and tournaments; employees are paid; moving the Clubhouse
//! costs a fee; skilled staff need a daily fee course of six or more holes.
//! FROM THE PUBLISHER'S golf.exe (docs/PUBLISHER_EXE_NOTES.md): money is stored in units of 100, a new game starts with 1000 units,
//! and the terrain costs below. Upkeep, wages, the day length and the debt ladder timing are PLACEHOLDERS.
use crate::terrain::*;

pub const STAFF_KINDS: usize = 4;
pub const CLUB_PRO: usize = 0;
pub const RANGER: usize = 1;
pub const GROUNDSKEEPER: usize = 2;
pub const SODA_VENDOR: usize = 3;

/// Columns of the exe's yearly money ledger, in the Financial Report's order (docs/PUBLISHER_EXE_NOTES.md).
pub const LEDGER_GREEN_FEES: usize = 0;
pub const LEDGER_HOME_SITES: usize = 1;
pub const LEDGER_FOOD_DRINK: usize = 2;
pub const LEDGER_BUILD_COURSE: usize = 3;
pub const LEDGER_FACILITIES: usize = 4;
pub const LEDGER_SALARIES: usize = 5;
pub const LEDGER_MAINTENANCE: usize = 6;
pub const LEDGER_OTHER: usize = 7;
pub const LEDGER_COLUMNS: usize = 8;
/// The report's row labels (the exe's own wording; the money sign is the section sign).
pub const LEDGER_LABELS: [&str; LEDGER_COLUMNS] =
    ["Greens Fees", "Home Sites", "Food/Drink", "Build course", "Facilities", "Salaries", "Maint./Interest", "Other"];
/// Months per year in the exe's calendar (March to October).
pub const MONTHS_PER_YEAR: i32 = 8;

#[derive(Clone, Debug)]
pub struct Economy {
    /// Starting amount: 1000 stored units of $100 in the publisher's golf.exe.
    pub start_cash: f64,
    pub cash: f64,
    /// Sandbox mode: unlimited funds, nothing is charged and the game cannot end.
    pub sandbox: bool,
    /// Real seconds per game day, which the game counts as one month: 1024 ticks of 87 ms (about 89 s).
    pub day_length: f64,
    pub day: i32,
    pub holes_played: i32,
    pub days_in_red: i32,
    /// PLACEHOLDER: days in the red before the board ends the game (the game text says "two years to return to positive cash").
    pub grace_days: i32,
    /// 0 none, 1 warned, 2 board concerned, 3 board very worried. The days at which each fires are PLACEHOLDERS.
    pub debt_stage: i32,
    /// Latest message for the player, taken (cleared) by the game once shown.
    pub notice: String,
    pub game_over: bool,
    pub income: f64,
    pub upkeep_paid: f64,
    /// PLACEHOLDER, recomputed from the course.
    pub daily_upkeep: f64,
    /// Bumps whenever cash changes (for the HUD).
    pub version: u32,
    /// Basic employees (manual p. 20): Club Pro, Ranger, Groundskeeper, Soda Vendor. Wages and effects are PLACEHOLDERS.
    pub staff: [i32; STAFF_KINDS],
    pub wages_paid: f64,
    /// Golfer fun, 0..100, the average of the golfers' comments. Shown as the attitude colour: red, yellow, green.
    pub fun: f64,
    /// How the golfer who just finished felt, 0..100.
    pub last_mood: f64,
    /// One record per year since the start (index 0 is 2001 in the exe's report), in dollars per ledger column.
    pub ledger: Vec<[f64; LEDGER_COLUMNS]>,
    clock: f64,
}

impl Default for Economy {
    fn default() -> Self {
        Economy {
            start_cash: 100000.0,
            cash: 100000.0,
            sandbox: false,
            day_length: crate::flight::TICKS_PER_MONTH as f64 * crate::flight::TICK_MS as f64 / 1000.0,
            day: 1,
            holes_played: 0,
            days_in_red: 0,
            grace_days: 24,
            debt_stage: 0,
            notice: String::new(),
            game_over: false,
            income: 0.0,
            upkeep_paid: 0.0,
            daily_upkeep: 0.0,
            version: 0,
            staff: [0; STAFF_KINDS],
            wages_paid: 0.0,
            fun: 50.0,
            last_mood: 50.0,
            ledger: Vec::new(),
            clock: 0.0,
        }
    }
}

/// PLACEHOLDER upkeep per tile per game day, by tile type.
fn tile_upkeep(ty: u8) -> f64 {
    match ty {
        TT_PUTTING_GREEN | TT_TRICKY_GREEN => 3.0,
        TT_TEE => 2.0,
        TT_FAIRWAY | TT_FIRM_FAIRWAY => 1.5,
        TT_ROUGH | TT_GRASSY_SAND => 0.4,
        TT_POT_SAND_BUNKER | TT_SAND | 27..=30 | TT_ZEN_SAND | TT_GRASS_BUNKER => 1.0,
        TT_FLOWER_BED => 1.2,
        TT_BUILDING => 5.0,
        TT_WATER_SHALLOW | TT_WATER_MIDDLE | TT_WATER_DEEP | TT_MARSH => 0.3,
        _ => 0.1,
    }
}

impl Economy {
    /// Money in the original is stored in units of 100.
    pub const UNIT: f64 = 100.0;
    /// Placeholder upkeep and wages are scaled by this to stay in proportion.
    pub const MONEY_SCALE: f64 = 25.0;
    /// Laying a path tile costs 100 dollars a square (a strategy guide). Removing a tile is free here.
    pub const PATH_TILE_COST: f64 = 100.0;

    pub fn staff_name(k: usize) -> &'static str {
        ["Club Pro", "Ranger", "Groundskeeper", "Soda Vendor"].get(k).copied().unwrap_or("?")
    }
    pub fn staff_count(&self) -> i32 {
        self.staff.iter().sum()
    }
    pub fn attitude(&self) -> &'static str {
        if self.fun < 35.0 {
            "red"
        } else if self.fun < 65.0 {
            "yellow"
        } else {
            "green"
        }
    }
    pub fn fun_event(&mut self, delta: f64) {
        self.fun = (self.fun + delta).clamp(0.0, 100.0);
        self.version = self.version.wrapping_add(1);
    }

    /// Cost per tile of laying a terrain type, in units of 100, from the exe's terrain table (the figure its build menu shows is this
    /// times 100). Indexed by the original tile id 0..19; ids without an entry (building lots, editor-only ids) cost nothing.
    pub fn terrain_cost_units(ty: i32) -> i32 {
        const K: [i32; 20] = [5, 10, 3, 3, 1, 2, 4, 6, 4, 8, 10, 4, 4, 10, 10, 10, 25, 50, 2, 6];
        match ty {
            0..=19 => K[ty as usize],
            23..=25 => 50, // water depths
            26 => 10,      // tricky green
            27 => 6,       // sand bunker
            _ => 0,
        }
    }

    pub fn upkeep_for(t: &Terrain) -> f64 {
        let mut sum = 0.0;
        for &ty in &t.ty {
            sum += tile_upkeep(ty) * 0.2;
        }
        for &p in &t.path_kind {
            sum += match p {
                2 => 0.12,
                1 => 0.06,
                _ => 0.0,
            };
        }
        for &w in &t.wall_mask {
            for b in 0..4 {
                if w & (1 << b) != 0 {
                    sum += 0.05;
                }
            }
        }
        sum
    }

    pub fn init(&mut self, t: &Terrain) {
        self.staff = [0; STAFF_KINDS];
        self.wages_paid = 0.0;
        self.fun = 50.0;
        self.cash = self.start_cash;
        self.day = 1;
        self.holes_played = 0;
        self.days_in_red = 0;
        self.debt_stage = 0;
        self.notice.clear();
        self.game_over = false;
        self.income = 0.0;
        self.upkeep_paid = 0.0;
        self.clock = 0.0;
        self.ledger.clear();
        self.update_upkeep(t);
        self.version = self.version.wrapping_add(1);
    }

    /// Call after editing the course.
    pub fn update_upkeep(&mut self, t: &Terrain) {
        self.daily_upkeep = Self::upkeep_for(t) * Self::MONEY_SCALE;
    }

    /// Collects a green fee (in units of 100).
    pub fn hole_completed(&mut self, fee_units: f64) {
        if self.game_over {
            return;
        }
        let fee = fee_units * Self::UNIT;
        self.cash += fee;
        self.income += fee;
        self.book(LEDGER_GREEN_FEES, fee);
        self.holes_played += 1;
        self.version = self.version.wrapping_add(1);
    }

    /// PLACEHOLDER daily wages.
    pub fn daily_wages(&self) -> f64 {
        const W: [f64; STAFF_KINDS] = [30.0, 25.0, 20.0, 15.0];
        (0..STAFF_KINDS).map(|k| W[k] * self.staff[k] as f64).sum::<f64>() * Self::MONEY_SCALE
    }

    /// False when there is no money (sandbox always works).
    pub fn hire(&mut self, kind: usize) -> bool {
        if kind >= STAFF_KINDS || (!self.sandbox && self.cash < 100.0) {
            return false; // PLACEHOLDER: hiring needs some cash in hand
        }
        self.staff[kind] += 1;
        self.version = self.version.wrapping_add(1);
        true
    }

    pub fn fire(&mut self, kind: usize) -> bool {
        if kind >= STAFF_KINDS || self.staff[kind] == 0 {
            return false;
        }
        self.staff[kind] -= 1;
        self.version = self.version.wrapping_add(1);
        true
    }

    /// Collects the fee (in units of 100) and applies the effect on fun.
    pub fn hole_finished(&mut self, strokes: i32, par: i32, fee_units: f64) {
        self.hole_completed(fee_units);
        // PLACEHOLDER mood model: par or better pleases golfers, a bad hole annoys them; the Club Pro and Soda Vendor help.
        let mut mood: f64 = if strokes <= par {
            85.0
        } else if strokes <= par + 1 {
            55.0
        } else {
            20.0
        };
        mood += 8.0 * (self.staff[CLUB_PRO] > 0) as i32 as f64 + 6.0 * (self.staff[SODA_VENDOR] > 0) as i32 as f64;
        mood = mood.min(100.0);
        self.last_mood = mood;
        self.fun_event(0.12 * (mood - self.fun));
    }

    /// Year index of the current game day (one game day is one month here).
    pub fn year_index(&self) -> usize {
        ((self.day - 1).max(0) / MONTHS_PER_YEAR) as usize
    }

    /// Adds a signed amount (dollars) to this year's ledger column.
    pub fn book(&mut self, column: usize, amount: f64) {
        let y = self.year_index();
        if self.ledger.len() <= y {
            self.ledger.resize(y + 1, [0.0; LEDGER_COLUMNS]);
        }
        self.ledger[y][column] += amount;
    }

    /// Income booked to a ledger column.
    pub fn earn_to(&mut self, column: usize, amount: f64) {
        self.earn(amount);
        self.book(column, amount);
    }

    /// Expense booked to a ledger column (nothing in sandbox mode).
    pub fn spend_to(&mut self, column: usize, amount: f64) {
        if self.sandbox {
            return;
        }
        self.spend(amount);
        self.book(column, -amount);
    }

    pub fn earn(&mut self, amount: f64) {
        self.cash += amount;
        self.version = self.version.wrapping_add(1);
    }
    pub fn spend(&mut self, amount: f64) {
        if self.sandbox {
            return;
        }
        self.cash -= amount;
        self.version = self.version.wrapping_add(1);
    }

    pub fn step(&mut self, dt: f64) {
        if self.game_over {
            return;
        }
        self.clock += dt;
        while self.clock >= self.day_length {
            self.clock -= self.day_length;
            self.day += 1;
            self.version = self.version.wrapping_add(1);
            if self.sandbox {
                continue; // unlimited funds: nothing is charged and the game cannot end
            }
            let wages = self.daily_wages();
            self.cash -= self.daily_upkeep + wages;
            self.upkeep_paid += self.daily_upkeep;
            self.wages_paid += wages;
            self.days_in_red = if self.cash < 0.0 { self.days_in_red + 1 } else { 0 };
            if self.days_in_red == 0 {
                self.debt_stage = 0;
            } else if self.days_in_red >= self.grace_days {
                self.game_over = true;
                self.notice = "The board has ended your contract. Game over.".into();
            } else if self.days_in_red >= self.grace_days * 3 / 4 && self.debt_stage < 3 {
                self.debt_stage = 3;
                self.notice = "The board is very worried about the lingering debt.".into();
            } else if self.days_in_red >= self.grace_days / 2 && self.debt_stage < 2 {
                self.debt_stage = 2;
                self.notice = "The board is concerned about the club's negative cash.".into();
            } else if self.debt_stage < 1 {
                self.debt_stage = 1;
                self.notice = "Warning: you have two years to get the club back into the black.".into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debt_ladder_ends_the_game() {
        let t = Terrain::demo_course(10, 10, 1);
        let mut e = Economy { start_cash: -1.0, ..Default::default() };
        e.init(&t);
        e.step(e.day_length);
        assert_eq!(e.debt_stage, 1);
        for _ in 0..30 {
            e.step(e.day_length);
        }
        assert!(e.game_over);
    }

    #[test]
    fn sandbox_never_pays() {
        let t = Terrain::demo_course(10, 10, 1);
        let mut e = Economy { sandbox: true, ..Default::default() };
        e.init(&t);
        e.hire(CLUB_PRO);
        e.step(e.day_length * 10.0 + 0.001);
        assert_eq!(e.cash, e.start_cash);
        assert_eq!(e.day, 11);
    }

    #[test]
    fn ledger_books_by_year() {
        let t = Terrain::demo_course(10, 10, 1);
        let mut e = Economy::default();
        e.init(&t);
        e.hole_completed(4.0);
        e.spend_to(LEDGER_BUILD_COURSE, 500.0);
        e.day = 1 + MONTHS_PER_YEAR;
        e.earn_to(LEDGER_FOOD_DRINK, 200.0);
        assert_eq!(e.ledger.len(), 2);
        assert_eq!(e.ledger[0][LEDGER_GREEN_FEES], 400.0);
        assert_eq!(e.ledger[0][LEDGER_BUILD_COURSE], -500.0);
        assert_eq!(e.ledger[1][LEDGER_FOOD_DRINK], 200.0);
    }

    #[test]
    fn terrain_costs_from_exe_table() {
        assert_eq!(Economy::terrain_cost_units(TT_WATER_SHALLOW as i32), 50);
        assert_eq!(Economy::terrain_cost_units(TT_PUTTING_GREEN as i32), 10);
        assert_eq!(Economy::terrain_cost_units(TT_BUILDING as i32), 0);
    }
}
