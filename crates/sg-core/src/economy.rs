//! The club's money, as the publisher's golf.exe runs it (docs/PUBLISHER_EXE_NOTES.md, "Money over time").
//!
//! Money is kept in units of 100. Everything happens on the game tick (87 ms): every 1024/(difficulty+2) ticks each active hole
//! costs 1 unit of maintenance and each employee's wage is charged with a chance that grows with the course rank; at each
//! month start (1024 ticks) debt pays 2% interest; at each year end (8 months) the board looks at the cash, and three year ends
//! in a row in the red end the game. There is no upkeep for terrain, paths or buildings, no taxes and no loans. Sandbox games
//! pay the same costs; they only skip the affordability checks and the board.
use crate::land::ExeRng;

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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Economy {
    /// Starting amount: 1000 units in a normal game, 10000 in sandbox, less the property's price.
    pub start_cash: f64,
    pub cash: f64,
    /// Sandbox mode: actions are never refused for lack of money and the board never ends the game. Costs are still paid.
    pub sandbox: bool,
    /// The game tick the economy last ran on.
    pub tick: u32,
    pub holes_played: i32,
    /// Negative year ends in a row (the exe's debt stage); the game ends when it passes 2.
    pub debt_stage: i32,
    /// Latest message for the player, taken (cleared) by the game once shown.
    pub notice: String,
    pub game_over: bool,
    pub income: f64,
    pub upkeep_paid: f64,
    /// Bumps whenever cash changes (for the HUD).
    pub version: u32,
    /// Hired employees by kind (Club Pro, Ranger, Groundskeeper, Soda Vendor).
    pub staff: [i32; STAFF_KINDS],
    pub wages_paid: f64,
    /// One record per year since the start (index 0 is 2001 in the exe's report), in dollars per ledger column.
    pub ledger: Vec<[f64; LEDGER_COLUMNS]>,
}

impl Default for Economy {
    fn default() -> Self {
        Economy {
            start_cash: 100000.0,
            cash: 100000.0,
            sandbox: false,
            tick: 0,
            holes_played: 0,
            debt_stage: 0,
            notice: String::new(),
            game_over: false,
            income: 0.0,
            upkeep_paid: 0.0,
            version: 0,
            staff: [0; STAFF_KINDS],
            wages_paid: 0.0,
            ledger: Vec::new(),
        }
    }
}

/// Wage per charge in money units (the exe's table 0x4c2e2c; the hire screen shows it times 100 "per week"): by kind,
/// basic and experienced.
pub const WAGE_UNITS: [[i32; 2]; STAFF_KINDS] = [[3, 7], [2, 3], [2, 4], [2, 5]];

/// Course rank by number of holes: 0 Municipal (under 6), 1 Daily Fee (6..9), 2 Country Club (10..17), 3 Championship (18).
pub fn rank(holes: usize) -> i32 {
    match holes {
        0..=5 => 0,
        6..=9 => 1,
        10..=17 => 2,
        _ => 3,
    }
}

pub const RANK_NAMES: [&str; 4] = ["Municipal", "Daily Fee", "Country Club", "Championship"];

/// An amount of money as the exe's 0x42dc00 appends it to a message (after a "§" the message holds): a minus sign when
/// negative, then the digits in groups of three with commas.
pub fn money_digits(v: i64) -> String {
    let d = v.unsigned_abs().to_string();
    let mut out = String::from(if v < 0 { "-" } else { "" });
    for (k, c) in d.chars().enumerate() {
        if k > 0 && (d.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// One employee as the wage charge sees it: kind 0..3 and whether experienced.
#[derive(Clone, Copy, Debug)]
pub struct Payroll {
    pub kind: usize,
    pub experienced: bool,
}

impl Economy {
    /// Money in the original is stored in units of 100.
    pub const UNIT: f64 = 100.0;
    /// A pathway tile costs 1 unit (the exe's building table).
    pub const PATH_TILE_COST: f64 = 100.0;
    /// Firing an employee costs 25 units (booked under Salaries).
    pub const FIRING_UNITS: f64 = 25.0;

    pub fn staff_name(k: usize) -> &'static str {
        ["Club Pro", "Ranger", "Groundskeeper", "Soda Vendor"].get(k).copied().unwrap_or("?")
    }
    pub fn staff_count(&self) -> i32 {
        self.staff.iter().sum()
    }

    /// Cost per tile of laying a terrain type, in units of 100, from the exe's terrain table (the figure its build menu shows is this
    /// times 100). Indexed by the original tile id 0..19; ids without an entry (building lots, editor-only ids) cost nothing.
    /// What it costs to clear a terrain type before painting over it (terrain table +0x24): rocks and trees 5, elm and
    /// water 10, wetlands 50, marsh 100, everything else nothing.
    pub fn terrain_clear_units(ty: i32) -> i32 {
        match ty {
            12..=15 => 5,
            16 | 17 | 23..=25 => 10,
            18 => 50,
            19 => 100,
            _ => 0,
        }
    }

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

    pub fn init(&mut self) {
        self.staff = [0; STAFF_KINDS];
        self.wages_paid = 0.0;
        self.cash = self.start_cash;
        self.tick = 0;
        self.holes_played = 0;
        self.debt_stage = 0;
        self.notice.clear();
        self.game_over = false;
        self.income = 0.0;
        self.upkeep_paid = 0.0;
        self.ledger.clear();
        // The exe starts year 0's "Other" column at the cash left after buying the property.
        self.book(LEDGER_OTHER, self.start_cash);
        self.version = self.version.wrapping_add(1);
    }

    /// Whether an action costing `amount` dollars may go ahead: it is cheap enough, or affordable, or a sandbox game, or the
    /// course has fewer than 3 holes (the first two holes can be built on credit).
    pub fn affordable(&self, amount: f64, holes: usize) -> bool {
        amount < Self::UNIT || amount <= self.cash || self.sandbox || holes < 3
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

    /// Hiring is free in the exe; experienced staff need a Daily Fee course (6 holes or more).
    pub fn hire(&mut self, kind: usize) -> bool {
        if kind >= STAFF_KINDS {
            return false;
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
        self.spend_to(LEDGER_SALARIES, Self::FIRING_UNITS * Self::UNIT);
        true
    }

    /// Year index of a tick (8 months of 1024 ticks a year).
    pub fn year_index(&self) -> usize {
        (self.tick >> 13) as usize
    }
    /// Months since the start (0 is March 2001).
    pub fn month_index(&self) -> i32 {
        (self.tick >> 10) as i32
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

    /// Expense booked to a ledger column (sandbox games pay too, as in the exe).
    pub fn spend_to(&mut self, column: usize, amount: f64) {
        self.spend(amount);
        self.book(column, -amount);
    }

    pub fn earn(&mut self, amount: f64) {
        self.cash += amount;
        self.version = self.version.wrapping_add(1);
    }
    pub fn spend(&mut self, amount: f64) {
        self.cash -= amount;
        self.version = self.version.wrapping_add(1);
    }

    /// One game tick: maintenance and wages every 1024/(difficulty+2) ticks, debt interest at month start, the board at year end.
    /// Returns the wage units paid to each entry of `staff` this tick (the exe adds them to the employee's total paid, +0x14).
    pub fn on_tick(&mut self, tick: u32, difficulty: i32, active_holes: usize, staff: &[Payroll], rng: &mut ExeRng) -> Vec<i32> {
        self.tick = tick;
        let mut paid = vec![0; staff.len()];
        if self.game_over {
            return paid;
        }
        let interval = 1024 / (difficulty.clamp(0, 3) as u32 + 2);
        if tick.is_multiple_of(interval) {
            for _ in 0..active_holes {
                self.spend_to(LEDGER_MAINTENANCE, Self::UNIT);
                self.upkeep_paid += Self::UNIT;
            }
            let r = rank(active_holes);
            for (p, paid) in staff.iter().zip(paid.iter_mut()) {
                if rng.below(4 - difficulty.clamp(0, 3)) <= r {
                    let units = WAGE_UNITS[p.kind.min(3)][p.experienced as usize];
                    let w = units as f64 * Self::UNIT;
                    self.spend_to(LEDGER_SALARIES, w);
                    self.wages_paid += w;
                    *paid = units;
                }
            }
        }
        if tick & 0x3ff == 0 && self.cash < 0.0 {
            // 2% a month on debt, in whole units, truncated toward zero.
            let units = (self.cash / Self::UNIT).trunc() as i64;
            let interest = (units / 50) as f64 * Self::UNIT;
            self.earn(interest);
            self.book(LEDGER_MAINTENANCE, interest);
        }
        if tick & 0x1fff == 0 && tick > 0 {
            self.year_end();
        }
        paid
    }

    /// The board at year end, as the year-end report runs it (0x44cff0): with cash below zero outside a sandbox the debt
    /// counter steps 0 -> 1 -> 2 -> 3, and the report adds two lines in red for the step (the board is concerned, two years
    /// to recover; very worried, one more year; no profit, the contract is terminated); cash of zero or more resets the
    /// counter. Once the counter is past 2 the game is over (`game_over`): the main loop asks the player to go on in
    /// Sandbox Mode or return to the main menu (`continue_in_sandbox`).
    fn year_end(&mut self) {
        if self.cash < 0.0 && !self.sandbox {
            let stage = self.debt_stage;
            self.debt_stage += 1;
            self.notice = match stage {
                0 => "The board is concerned about our negative cash situation.\nYou have two years to return to positive cash.",
                1 => "The board is very worried about our lingering debt.\nYou have one more year to get out of debt.",
                2 => "You have been unable to make a profit on this course.\nRegrettably, the board has terminated your contract.",
                _ => "",
            }
            .to_string();
            if self.debt_stage > 2 {
                self.game_over = true;
            }
        } else {
            self.debt_stage = 0;
        }
        self.version = self.version.wrapping_add(1);
    }

    /// "Continue this game in Sandbox Mode." after the contract is terminated (main loop 0x420b27): the debt counter goes
    /// back to 0 and the game carries on as a sandbox game (flag 0x1000000).
    pub fn continue_in_sandbox(&mut self) {
        self.debt_stage = 0;
        self.game_over = false;
        self.sandbox = true;
        self.version = self.version.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_digits_group_by_thousands() {
        assert_eq!(money_digits(0), "0");
        assert_eq!(money_digits(800), "800");
        assert_eq!(money_digits(2000), "2,000");
        assert_eq!(money_digits(-1234567), "-1,234,567");
    }

    #[test]
    fn three_negative_year_ends_end_the_game() {
        let mut e = Economy { start_cash: -100.0, ..Default::default() };
        e.init();
        let mut rng = ExeRng::from_clock(1);
        for t in 1..=3 * 0x2000u32 {
            e.on_tick(t, 1, 0, &[], &mut rng);
            if t == 0x2000 {
                assert_eq!(e.debt_stage, 1);
                assert!(!e.game_over);
            }
        }
        assert!(e.game_over);
        assert_eq!(e.notice.lines().last(), Some("Regrettably, the board has terminated your contract."));
        e.continue_in_sandbox();
        assert!(!e.game_over && e.sandbox && e.debt_stage == 0);
    }

    #[test]
    fn a_positive_year_end_clears_the_debt_counter() {
        let mut e = Economy { start_cash: -100.0, ..Default::default() };
        e.init();
        let mut rng = ExeRng::from_clock(1);
        for t in 1..=0x2000u32 {
            e.on_tick(t, 1, 0, &[], &mut rng);
        }
        assert_eq!(e.debt_stage, 1);
        assert_eq!(e.notice, "The board is concerned about our negative cash situation.\nYou have two years to return to positive cash.");
        e.cash = 0.0;
        for t in 0x2001..=0x4000u32 {
            e.on_tick(t, 1, 0, &[], &mut rng);
        }
        assert_eq!(e.debt_stage, 0);
    }

    #[test]
    fn holes_cost_one_unit_per_interval() {
        let mut e = Economy { start_cash: 100000.0, ..Default::default() };
        e.init();
        let mut rng = ExeRng::from_clock(1);
        for t in 1..=1024u32 {
            e.on_tick(t, 0, 3, &[], &mut rng);
        }
        // Easy: interval 512, two charges a month, 3 holes.
        assert_eq!(e.cash, 100000.0 - 6.0 * 100.0);
    }

    #[test]
    fn wages_on_impossible_always_charged() {
        let mut e = Economy::default();
        e.init();
        let mut rng = ExeRng::from_clock(9);
        let staff = [Payroll { kind: CLUB_PRO, experienced: false }];
        let paid = e.on_tick(204, 3, 0, &staff, &mut rng);
        assert_eq!(e.cash, e.start_cash - 300.0);
        // the units go on the employee's total paid
        assert_eq!(paid, vec![3]);
    }

    #[test]
    fn debt_interest_two_percent() {
        let mut e = Economy { start_cash: -10000.0, ..Default::default() };
        e.init();
        let mut rng = ExeRng::from_clock(1);
        e.on_tick(1024, 0, 0, &[], &mut rng);
        assert_eq!(e.cash, -10200.0);
    }

    #[test]
    fn terrain_costs_from_exe_table() {
        assert_eq!(Economy::terrain_cost_units(17), 50);
        assert_eq!(Economy::terrain_cost_units(1), 10);
        assert_eq!(Economy::terrain_cost_units(22), 0);
    }

    #[test]
    fn rank_by_holes() {
        assert_eq!(rank(5), 0);
        assert_eq!(rank(6), 1);
        assert_eq!(rank(10), 2);
        assert_eq!(rank(18), 3);
    }
}
