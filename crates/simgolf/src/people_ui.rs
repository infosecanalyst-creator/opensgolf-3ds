//! OpenSGolf extras: the People screen (extras.rs). Three tabs in the look of the port's other own screens (keys_ui: the
//! System Functions box's frame and fill, its fonts and colours, the OK tick; tabs as the Enhanced graphics box's):
//!
//! * Golfers: everyone on the course, sortable by name, type, hole, mood, handicap and money spent this round, with Locate,
//!   Follow, the golfer card and the golfer's profile page (profile_ui.rs);
//! * Staff: every employee, sortable by name, job, date hired, wage, golfers served and pay so far, with Hire (the
//!   original's hire dialog, its rules and wages), Fire (the Employee panel's, 25 units under Salaries), Move (the panel's
//!   pick-up-and-place), Rename, Locate and Follow;
//! * Your pro: the original's pro screens one click away (Customise, skills, practice round, match, tournament), each under
//!   the same conditions as on the Player panel.
//!
//! Opened by Shift+G or the Information menu's "People" line. Keys: Left and Right (or Tab) change the tab, Up and Down the
//! row, PageUp and PageDown scroll, Enter locates, F follows, C opens the card, P the profile; on the staff tab H hires,
//! Delete fires, M moves and R renames; Esc, a right click or the OK tick close the screen.
use crate::app::*;
use crate::extras::{draw_arrow, draw_button, draw_tab, Follow, RenameFor};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{rgba, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use sg_core::lang::{tr, trf};

const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
/// The column headings' line, the first row and the row pitch.
const HEAD_Y: f32 = BOX.y + 70.0;
const ROW0: f32 = BOX.y + 90.0;
const PITCH: f32 = 20.0;
const ROWS: usize = 17;
/// The scroll arrows, right of the list.
const UP: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: ROW0, w: 20.0, h: 20.0 };
const DOWN: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: ROW0 + PITCH * ROWS as f32 - 20.0, w: 20.0, h: 20.0 };
/// The action buttons' line.
const BUTTON_Y: f32 = BOX.y + BOX.h - 56.0;
const BUTTON_W: f32 = 84.0;

fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

fn tab_rect(i: usize) -> Rect {
    Rect::new(BOX.x + 24.0 + 132.0 * i as f32, BOX.y + 40.0, 126.0, 18.0)
}

fn button_rect(i: usize) -> Rect {
    Rect::new(BOX.x + 24.0 + (BUTTON_W + 8.0) * i as f32, BUTTON_Y, BUTTON_W, 20.0)
}

/// The tabs.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Tab {
    #[default]
    Golfers,
    Staff,
    Pro,
}
const TABS: [Tab; 3] = [Tab::Golfers, Tab::Staff, Tab::Pro];

/// The screen's state: the tab, each list's sort column (and whether it runs downward), the selected row (a person record
/// for a golfer, an index into the employees for staff), the first row shown, and the screen to go back to.
#[derive(Default)]
pub struct PeopleUi {
    pub tab: Tab,
    pub sort: [(usize, bool); 2],
    pub golfer: Option<i32>,
    pub staff: Option<usize>,
    pub off: usize,
    pub back: Option<Screen>,
}

/// A column: its heading (a text key), its left edge from the box's and its width.
struct Col(&'static str, f32, f32);
const GOLFER_COLS: [Col; 6] = [
    Col("STR_PEOPLE_COL_NAME", 24.0, 196.0),
    Col("STR_PEOPLE_COL_TYPE", 222.0, 100.0),
    Col("STR_PEOPLE_COL_HOLE", 324.0, 56.0),
    Col("STR_PEOPLE_COL_MOOD", 382.0, 70.0),
    Col("STR_PEOPLE_COL_HCP", 454.0, 70.0),
    Col("STR_PEOPLE_COL_SPENT", 526.0, 100.0),
];
// wide enough for the longest job ("Groundskeeper") and month ("September 2001") in the game's own font
const STAFF_COLS: [Col; 6] = [
    Col("STR_PEOPLE_COL_NAME", 24.0, 160.0),
    Col("STR_PEOPLE_COL_JOB", 186.0, 136.0),
    Col("STR_PEOPLE_COL_HIRED", 324.0, 128.0),
    Col("STR_PEOPLE_COL_WAGE", 454.0, 70.0),
    Col("STR_PEOPLE_COL_SERVED", 526.0, 62.0),
    Col("STR_PEOPLE_COL_PAID", 590.0, 82.0),
];

/// A row of either list: the cells' text, the sort keys (a number or a text), the face to draw in the mood column, and
/// the golfer's person record or the employee's index.
struct Line {
    cells: [String; 6],
    keys: [(i64, String); 6],
    face: Option<(i32, usize)>,
    id: i64,
}

/// The pro screens' buttons on the Your pro tab: label key and help key.
const PRO_BUTTONS: [(&str, &str); 5] = [
    ("STR_PEOPLE_PRO_PROFILE", "STR_PEOPLE_PRO_PROFILE_HELP"),
    ("STR_PEOPLE_PRO_SKILLS", "STR_PEOPLE_PRO_SKILLS_HELP"),
    ("STR_PEOPLE_PRO_PRACTICE", "STR_PEOPLE_PRO_PRACTICE_HELP"),
    ("STR_PEOPLE_PRO_MATCH", "STR_PEOPLE_PRO_MATCH_HELP"),
    ("STR_PEOPLE_PRO_TOURNAMENT", "STR_PEOPLE_PRO_TOURNAMENT_HELP"),
];

fn pro_button_rect(i: usize) -> Rect {
    Rect::new(BOX.x + 40.0, BOX.y + 150.0 + 44.0 * i as f32, 150.0, 24.0)
}

const LEVEL_KEYS: [&str; 6] =
    ["STR_PEOPLE_GUEST", "STR_MEMBER_VISITOR", "STR_MEMBER_MEMBER", "STR_MEMBER_SILVER", "STR_MEMBER_GOLD", "STR_PEOPLE_PLATINUM"];

impl App {
    pub fn open_people(&mut self) {
        if !matches!(self.screen, Screen::People | Screen::Profile | Screen::History) {
            self.xs.people.back = Some(if self.screen == Screen::Popup { Screen::Play } else { self.screen });
        }
        self.screen = Screen::People;
        self.hover = -1;
    }

    fn close_people(&mut self) {
        self.screen = Screen::Play;
        self.xs.people.back = None;
        self.hover = -1;
    }

    /// The golfers listed: everyone on the course (holes 1 to 18) or walking off it.
    fn people_golfers(&self) -> Vec<usize> {
        (0..sg_core::golfer::SLOTS).filter(|&g| (1..=19).contains(&self.club.g[g].hole) && self.club.g[g].roster >= 0).collect()
    }

    /// The employees listed (the Employee panel's: hired and still working).
    pub fn people_staff(&self) -> Vec<usize> {
        (0..self.employees.len()).filter(|&i| self.employees[i].active && (-5..=-2).contains(&self.employees[i].job)).collect()
    }

    /// An employee's name: the one given with Rename, else the original's first name and title.
    pub fn staff_name(&self, i: usize) -> String {
        if let Some(n) = self.xs.kept.staff_names.get(&i) {
            return n.clone();
        }
        self.employees.get(i).map(|e| self.employee_name(e)).unwrap_or_default()
    }

    /// A golfer's type: Pro or VIP for the special golfers, else the membership level.
    pub fn golfer_type(&self, g: usize) -> String {
        let gg = &self.club.g[g];
        match gg.kind & 0xe0 {
            0x20 => tr("STR_PEOPLE_PRO").into(),
            // the More visitors extra's visitors (sg_core::visitors)
            sg_core::visitors::KIND_TOURING_PRO => tr("STR_PEOPLE_TOURING_PRO").into(),
            sg_core::visitors::KIND_CELEBRITY => tr("STR_PEOPLE_CELEBRITY").into(),
            0 => {
                let lvl = self.club.members.get(gg.roster.max(0) as usize).map(|m| (m.level & 7) as usize).unwrap_or(0);
                tr(LEVEL_KEYS[lvl.min(5)]).into()
            }
            _ => tr("STR_PEOPLE_VIP").into(),
        }
    }

    /// A golfer's handicap as the roster keeps it (the member's average over par), None before a round is finished.
    pub fn golfer_hcp(&self, roster: i32) -> Option<i32> {
        self.club.members.get(roster.max(0) as usize).filter(|m| m.rounds > 0).map(|m| m.avg as i32)
    }

    fn people_lines(&self) -> Vec<Line> {
        let money = |d: i64| crate::ui::money(d);
        let mut v: Vec<Line> = match self.xs.people.tab {
            Tab::Golfers => self
                .people_golfers()
                .into_iter()
                .map(|g| {
                    let gg = &self.club.g[g];
                    let name = self.club.vip_name(g);
                    let ty = self.golfer_type(g);
                    let hole = if (1..19).contains(&gg.hole) { gg.hole } else { 19 };
                    let hole_t = if hole < 19 { format!("{hole}") } else { tr("STR_PEOPLE_LEAVING").into() };
                    let hcp = self.golfer_hcp(gg.roster);
                    let spent = self.spent_by(g);
                    Line {
                        cells: [
                            name.clone(),
                            ty.clone(),
                            hole_t,
                            format!("{:+}", gg.mood),
                            hcp.map(|h| format!("{h:+}")).unwrap_or_else(|| "-".into()),
                            money(spent),
                        ],
                        keys: [
                            (0, name.to_lowercase()),
                            (0, ty),
                            (hole as i64, String::new()),
                            (gg.mood as i64, String::new()),
                            (hcp.unwrap_or(99) as i64, String::new()),
                            (spent, String::new()),
                        ],
                        face: Some((gg.roster, self.club.attitude(g) as usize)),
                        id: gg.roster as i64,
                    }
                })
                .collect(),
            Tab::Staff => self
                .people_staff()
                .into_iter()
                .map(|i| {
                    let e = self.employees[i];
                    let kind = (-2 - e.job as i32).clamp(0, 3) as usize;
                    let name = self.staff_name(i);
                    let job = staff_title(kind, e.upgraded).to_string();
                    let month = tr(crate::hud_ui::MONTH_KEYS[(e.hired & 7) as usize]);
                    let hired = trf("STR_PEOPLE_MONTH_YEAR", &[("MONTH", &month), ("YEAR", &(2001 + (e.hired >> 3)))]);
                    let wage = sg_core::data::wages()[kind][e.upgraded as usize] as i64 * 100;
                    Line {
                        cells: [name.clone(), job.clone(), hired, money(wage), format!("{}", e.served), money(e.paid as i64 * 100)],
                        keys: [
                            (0, name.to_lowercase()),
                            (0, job),
                            (e.hired as i64, String::new()),
                            (wage, String::new()),
                            (e.served as i64, String::new()),
                            (e.paid as i64, String::new()),
                        ],
                        face: None,
                        id: i as i64,
                    }
                })
                .collect(),
            Tab::Pro => Vec::new(),
        };
        let t = (self.xs.people.tab == Tab::Staff) as usize;
        let (col, down) = self.xs.people.sort[t];
        v.sort_by(|a, b| {
            let o = a.keys[col].cmp(&b.keys[col]);
            // downward on the column, names still from A on a tie
            if down { o.reverse() } else { o }.then_with(|| a.keys[0].cmp(&b.keys[0]))
        });
        v
    }

    fn people_selected(&self, lines: &[Line]) -> Option<usize> {
        let id = match self.xs.people.tab {
            Tab::Golfers => self.xs.people.golfer? as i64,
            Tab::Staff => self.xs.people.staff? as i64,
            Tab::Pro => return None,
        };
        lines.iter().position(|l| l.id == id)
    }

    fn people_select(&mut self, lines: &[Line], k: usize) {
        let Some(l) = lines.get(k) else { return };
        match self.xs.people.tab {
            Tab::Golfers => self.xs.people.golfer = Some(l.id as i32),
            Tab::Staff => self.xs.people.staff = Some(l.id as usize),
            Tab::Pro => {}
        }
        // keep the selection on the page
        if k < self.xs.people.off {
            self.xs.people.off = k;
        } else if k >= self.xs.people.off + ROWS {
            self.xs.people.off = k + 1 - ROWS;
        }
    }

    fn people_set_tab(&mut self, t: Tab) {
        self.xs.people.tab = t;
        self.xs.people.off = 0;
    }

    /// The action buttons of the tab: (label key, enabled).
    fn people_buttons(&self, has_sel: bool) -> Vec<(&'static str, bool)> {
        match self.xs.people.tab {
            Tab::Golfers => vec![
                ("STR_PEOPLE_LOCATE", has_sel),
                ("STR_PEOPLE_FOLLOW", has_sel),
                ("STR_PEOPLE_CARD", has_sel),
                ("STR_PEOPLE_PROFILE", has_sel),
            ],
            Tab::Staff => vec![
                ("STR_PEOPLE_HIRE", self.people_staff().len() < crate::panels_ui::max_employees(self.club.extras.staff_limit())),
                ("STR_PEOPLE_FIRE", has_sel),
                ("STR_PEOPLE_MOVE", has_sel),
                ("STR_PEOPLE_RENAME", has_sel),
                ("STR_PEOPLE_LOCATE", has_sel),
                ("STR_PEOPLE_FOLLOW", has_sel),
            ],
            Tab::Pro => Vec::new(),
        }
    }

    /// Whether each of the pro's buttons can be used now (the Player panel's own conditions, player_panel.rs).
    fn pro_buttons_ok(&self) -> [bool; 5] {
        use sg_core::golfer::game;
        let c = &self.club;
        let free = c.gary == -1 && c.game & game::REPEAT == 0;
        let practice = free && c.next_hole >= 2;
        let play = practice && c.game & sg_core::pro::CHALLENGE != 0 && c.challenge_pro != -1;
        let tournament = free && c.game & sg_core::tournament::OFFERED != 0;
        [true, true, practice, play, tournament]
    }

    /// Runs action `a` of the tab on the selected row.
    fn people_action(&mut self, a: &str) {
        let lines = self.people_lines();
        let sel = self.people_selected(&lines);
        let golfer = self.xs.people.golfer.and_then(|r| self.golfer_of(r));
        let staff = sel.and(self.xs.people.staff).filter(|&i| self.employees.get(i).is_some_and(|e| e.active));
        match (self.xs.people.tab, a) {
            (Tab::Golfers, "STR_PEOPLE_LOCATE") | (Tab::Golfers, "STR_PEOPLE_FOLLOW") | (Tab::Golfers, "STR_PEOPLE_CARD") => {
                let Some(g) = golfer.filter(|_| sel.is_some()) else { return self.ui_sound(0x18) };
                self.close_people();
                self.look_at((self.club.g[g].x, self.club.g[g].y));
                if a == "STR_PEOPLE_FOLLOW" {
                    self.xs.follow = Some(Follow::Golfer(self.club.g[g].roster));
                }
                if a == "STR_PEOPLE_CARD" {
                    if self.card != Some(g) {
                        self.card_ui = crate::screens_ui::CardUi { mouse: self.card_ui.mouse, ..Default::default() };
                    }
                    self.card = Some(g);
                }
            }
            (Tab::Golfers, "STR_PEOPLE_PROFILE") => match self.xs.people.golfer.filter(|_| sel.is_some()) {
                Some(r) => self.open_profile(r),
                None => self.ui_sound(0x18),
            },
            (Tab::Staff, "STR_PEOPLE_HIRE") => {
                if self.people_staff().len() >= crate::panels_ui::max_employees(self.club.extras.staff_limit()) {
                    return self.ui_sound(0x18);
                }
                // the original's hire dialog over the Employee panel, its rules and wages (panels_ui)
                self.close_people();
                self.pstate.emp_flag = false;
                self.pstate.emp_base = 4;
                self.panel = 3;
                self.pstate.hire_open = true;
            }
            (Tab::Staff, _) => {
                let Some(i) = staff else { return self.ui_sound(0x18) };
                match a {
                    "STR_PEOPLE_FIRE" => {
                        if self.fire_employee(i) {
                            self.snd("Interface/Button2.wav", 1.0, false);
                            self.xs.people.staff = None;
                            if self.xs.follow == Some(Follow::Staff(i)) {
                                self.xs.follow = None;
                            }
                        } else {
                            self.ui_sound(0x18);
                        }
                    }
                    "STR_PEOPLE_MOVE" => {
                        // the Employee panel's Move: the next click on the course is the employee's new post
                        let e = self.employees[i];
                        self.close_people();
                        self.moving_employee = Some(i);
                        self.look_at((e.x, e.y));
                        let name = staff_title((-2 - e.job as i32).clamp(0, 3) as usize, e.upgraded);
                        self.show_toast(&trf("STR_TOAST_EMPLOYEE_CLICK", &[("EMPLOYEE", &name)]));
                    }
                    "STR_PEOPLE_RENAME" => self.open_staff_rename(i),
                    _ => {
                        let e = self.employees[i];
                        self.close_people();
                        self.look_at((e.x, e.y));
                        if a == "STR_PEOPLE_FOLLOW" {
                            self.xs.follow = Some(Follow::Staff(i));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Rename (the People screen's and the Employee panel's Rename button): the course rename prompt, for an employee.
    pub fn open_staff_rename(&mut self, i: usize) {
        self.xs.rename_for = Some(RenameFor::Employee(i));
        self.rename = Some(self.staff_name(i));
    }

    fn pro_action(&mut self, b: usize) {
        if !self.pro_buttons_ok()[b] {
            return self.ui_sound(0x18);
        }
        self.close_people();
        match b {
            0 => self.open_customise_player(),
            1 => self.pro_button(2),
            2 => self.pro_button(0),
            3 => self.pro_button(1),
            _ => self.begin_tournament(),
        }
    }

    pub fn people_wheel(&mut self, y: f32) {
        self.people_scroll(if y > 0.0 { -2 } else { 2 });
    }

    fn people_scroll(&mut self, d: i32) {
        let n = self.people_lines().len() as i32;
        self.xs.people.off = (self.xs.people.off as i32 + d).clamp(0, (n - ROWS as i32).max(0)) as usize;
    }

    pub fn people_click(&mut self, x: f32, y: f32, right: bool) {
        if right || ok_tick().has(x, y) || !BOX.has(x, y) {
            return self.close_people();
        }
        if let Some(i) = (0..3).find(|&i| tab_rect(i).has(x, y)) {
            return self.people_set_tab(TABS[i]);
        }
        if self.xs.people.tab == Tab::Pro {
            if let Some(b) = (0..5).find(|&b| pro_button_rect(b).has(x, y)) {
                self.pro_action(b);
            }
            return;
        }
        if UP.has(x, y) {
            return self.people_scroll(-3);
        }
        if DOWN.has(x, y) {
            return self.people_scroll(3);
        }
        let lines = self.people_lines();
        let has_sel = self.people_selected(&lines).is_some();
        if let Some((a, _)) = self.people_buttons(has_sel).into_iter().enumerate().find(|(k, _)| button_rect(*k).has(x, y)).map(|(_, b)| b)
        {
            return self.people_action(a);
        }
        let cols = if self.xs.people.tab == Tab::Golfers { &GOLFER_COLS } else { &STAFF_COLS };
        if (HEAD_Y - 4.0..ROW0 - 2.0).contains(&y) {
            if let Some(c) = cols.iter().position(|c| (BOX.x + c.1..BOX.x + c.1 + c.2).contains(&x)) {
                let t = (self.xs.people.tab == Tab::Staff) as usize;
                let s = &mut self.xs.people.sort[t];
                *s = if s.0 == c { (c, !s.1) } else { (c, c >= 2) };
            }
            return;
        }
        if (ROW0..ROW0 + PITCH * ROWS as f32).contains(&y) && x < UP.x {
            let k = self.xs.people.off + ((y - ROW0) / PITCH) as usize;
            if k < lines.len() {
                // a second click on the selected golfer opens the profile, on an employee locates them
                let again = self.people_selected(&lines) == Some(k);
                self.people_select(&lines, k);
                if again {
                    let a = if self.xs.people.tab == Tab::Golfers { "STR_PEOPLE_PROFILE" } else { "STR_PEOPLE_LOCATE" };
                    self.people_action(a);
                }
            }
        }
    }

    pub fn people_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let tab = TABS.iter().position(|&t| t == self.xs.people.tab).unwrap_or(0);
        let lines = self.people_lines();
        let sel = self.people_selected(&lines);
        let staff = self.xs.people.tab == Tab::Staff;
        match k {
            KeyCode::Escape => self.close_people(),
            KeyCode::G if self.shift_held => self.close_people(),
            KeyCode::Right => self.people_set_tab(TABS[(tab + 1) % 3]),
            KeyCode::Tab if !self.shift_held => self.people_set_tab(TABS[(tab + 1) % 3]),
            KeyCode::Left | KeyCode::Tab => self.people_set_tab(TABS[(tab + 2) % 3]),
            KeyCode::Down => self.people_select(&lines, sel.map(|s| (s + 1).min(lines.len().saturating_sub(1))).unwrap_or(0)),
            KeyCode::Up => self.people_select(&lines, sel.map(|s| s.saturating_sub(1)).unwrap_or(0)),
            KeyCode::PageDown => self.people_scroll(ROWS as i32),
            KeyCode::PageUp => self.people_scroll(-(ROWS as i32)),
            KeyCode::Enter | KeyCode::KpEnter if self.xs.people.tab != Tab::Pro => self.people_action("STR_PEOPLE_LOCATE"),
            KeyCode::F => self.people_action("STR_PEOPLE_FOLLOW"),
            KeyCode::C if !staff => self.people_action("STR_PEOPLE_CARD"),
            KeyCode::P if !staff => self.people_action("STR_PEOPLE_PROFILE"),
            KeyCode::H if staff => self.people_action("STR_PEOPLE_HIRE"),
            KeyCode::Delete if staff => self.people_action("STR_PEOPLE_FIRE"),
            KeyCode::M if staff => self.people_action("STR_PEOPLE_MOVE"),
            KeyCode::R if staff => self.people_action("STR_PEOPLE_RENAME"),
            KeyCode::Key1 if self.xs.people.tab == Tab::Pro => self.pro_action(0),
            KeyCode::Key2 if self.xs.people.tab == Tab::Pro => self.pro_action(1),
            KeyCode::Key3 if self.xs.people.tab == Tab::Pro => self.pro_action(2),
            KeyCode::Key4 if self.xs.people.tab == Tab::Pro => self.pro_action(3),
            KeyCode::Key5 if self.xs.people.tab == Tab::Pro => self.pro_action(4),
            _ => {}
        }
    }

    pub fn draw_people(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, yellow, navy, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x6318));
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_PEOPLE_TITLE"), yellow, true);
        let (px, py) = self.info.pointer;
        let counts = [self.people_golfers().len(), self.people_staff().len()];
        for (i, t) in TABS.iter().enumerate() {
            let label = match t {
                Tab::Golfers => trf("STR_PEOPLE_TAB_GOLFERS", &[("COUNT", &counts[0])]),
                Tab::Staff => {
                    let limit = crate::panels_ui::max_employees(self.club.extras.staff_limit());
                    trf("STR_PEOPLE_TAB_STAFF", &[("COUNT", &counts[1]), ("LIMIT", &limit)])
                }
                Tab::Pro => tr("STR_PEOPLE_TAB_PRO").to_string(),
            };
            let r = tab_rect(i);
            draw_tab(g, &s, r, crate::textsize::fit(F_ARIAL10, 1.25, &label, r.w - 6.0), &label, *t == self.xs.people.tab, r.has(px, py));
        }
        if self.xs.people.tab == Tab::Pro {
            self.draw_pro_tab(g, &s);
        } else {
            let lines = self.people_lines();
            let sel = self.people_selected(&lines);
            let t = (self.xs.people.tab == Tab::Staff) as usize;
            let cols = if t == 0 { &GOLFER_COLS } else { &STAFF_COLS };
            let (sc, down) = self.xs.people.sort[t];
            // with bigger text (textsize.rs) the list keeps its 20 pixel rows: its words grow to 120% at most, each cut to
            // its column as before
            let cell = F_MANUAL15.scaled(crate::textsize::k(1.2));
            let foot = |t: &str| crate::textsize::fit(F_ARIAL10, 1.5, t, BOX.w - 110.0);
            for (c, col) in cols.iter().enumerate() {
                let x = BOX.x + col.1;
                let lit = (HEAD_Y - 4.0..ROW0 - 2.0).contains(&py) && (x..x + col.2).contains(&px);
                let head = crate::textsize::fit(F_MANUAL15, 1.2, tr(col.0), col.2 - 16.0);
                s.put_shadowed(g, head, x + 4.0, HEAD_Y, tr(col.0), if lit { white } else { yellow }, false);
                if c == sc {
                    draw_arrow(g, &s, x + 10.0 + head.width(tr(col.0)), HEAD_Y + 8.0, !down, white);
                }
            }
            s.fill(g, BOX.x + 24.0, ROW0 - 3.0, UP.x - BOX.x - 32.0, 1.0, rgba(0.0, 0.0, 0.3, 0.4));
            if lines.is_empty() {
                let key = if t == 0 { "STR_PEOPLE_NO_GOLFERS" } else { "STR_PEOPLE_NO_STAFF" };
                s.put_centered(g, cell, 400.0, ROW0 + 60.0, tr(key), white);
            }
            let off = self.xs.people.off.min(lines.len().saturating_sub(ROWS));
            self.xs.people.off = off;
            for (k, l) in lines.iter().enumerate().skip(off).take(ROWS) {
                let y = ROW0 + PITCH * (k - off) as f32;
                let hover = (y..y + PITCH).contains(&py) && (BOX.x + 20.0..UP.x).contains(&px);
                if sel == Some(k) {
                    s.fill(g, BOX.x + 20.0, y, UP.x - BOX.x - 28.0, PITCH - 1.0, c15(0x7f40));
                } else if hover {
                    s.fill(g, BOX.x + 20.0, y, UP.x - BOX.x - 28.0, PITCH - 1.0, rgba(1.0, 1.0, 1.0, 0.2));
                }
                let ink = if sel == Some(k) { navy } else { white };
                for (c, col) in cols.iter().enumerate() {
                    let mut x = BOX.x + col.1 + 4.0;
                    if c == 3 {
                        if let Some((r, face)) = l.face {
                            self.small_head(g, &s, r, face, x, y + 1.0, 18.0);
                            x += 22.0;
                        }
                    }
                    let mut text = l.cells[c].clone();
                    while cell.width(&text) > col.2 - 6.0 && text.chars().count() > 1 {
                        text.pop();
                    }
                    s.put(g, cell, x, y + 2.0 - (cell.px - F_MANUAL15.px).min(2.0), &text, ink);
                }
            }
            for (b, up, ok) in [(UP, true, off > 0), (DOWN, false, off + ROWS < lines.len())] {
                draw_button(g, &s, b, "", ok, b.has(px, py));
                draw_arrow(g, &s, b.x + b.w / 2.0, b.y + b.h / 2.0, up, if ok { navy } else { grey });
            }
            for (k, (label, ok)) in self.people_buttons(sel.is_some()).into_iter().enumerate() {
                let r = button_rect(k);
                draw_button(g, &s, r, tr(label), ok, r.has(px, py));
            }
            let help = tr(if t == 0 { "STR_PEOPLE_HELP_GOLFERS" } else { "STR_PEOPLE_HELP_STAFF" });
            s.put(g, foot(help), BOX.x + 24.0, BOX.y + BOX.h - 24.0, help, navy);
        }
        if self.art.trans.tex.is_some() {
            let r = ok_tick();
            s.image_part(g, &self.art.trans, r.x, r.y, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }

    /// The Your pro tab: the pro's name and record, and a button for each of the original's pro screens.
    fn draw_pro_tab(&self, g: &mut Gfx, s: &Ui) {
        let (white, yellow, navy, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x6318));
        let c = &self.club;
        s.put_shadowed(g, F_MANUAL20, BOX.x + 40.0, BOX.y + 76.0, &self.pro_name(), white, false);
        let level: i32 = c.pro_skill[..10].iter().map(|&v| v as i32).sum();
        let info = trf("STR_PEOPLE_PRO_RECORD", &[("POINTS", &c.skill_points), ("SKILLS", &level), ("TROPHIES", &c.trophies)]);
        // with bigger text (textsize.rs): the lines 18 apart grow to 115%, the others as far as their room holds
        let line = |t: &str, w: f32| crate::textsize::fit(F_MANUAL15, 1.15, t, w);
        s.put(g, line(&info, BOX.w - 80.0), BOX.x + 40.0, BOX.y + 104.0, &info, navy);
        let status = if c.gary != -1 { tr("STR_PEOPLE_PRO_PLAYING") } else { tr("STR_PEOPLE_PRO_FREE") };
        s.put(g, line(status, BOX.w - 80.0), BOX.x + 40.0, BOX.y + 122.0, status, navy);
        let (px, py) = self.info.pointer;
        let ok = self.pro_buttons_ok();
        for (b, (label, help)) in PRO_BUTTONS.iter().enumerate() {
            let r = pro_button_rect(b);
            draw_button(g, s, r, tr(label), ok[b], r.has(px, py));
            let room = BOX.x + BOX.w - 24.0 - (r.x + r.w + 16.0);
            let f = crate::textsize::fit(F_ARIAL10, 1.5, tr(help), room);
            s.put(g, f, r.x + r.w + 16.0, r.y + 7.0 - ((f.px - 10.0) / 2.0).floor(), tr(help), if ok[b] { navy } else { grey });
        }
        let note = tr("STR_PEOPLE_PRO_NOTE");
        s.put_shadowed(g, crate::textsize::fit(F_MANUAL15, 1.5, note, BOX.w - 80.0), BOX.x + 40.0, BOX.y + 384.0, note, yellow, false);
        let help = tr("STR_PEOPLE_HELP_PRO");
        s.put(g, crate::textsize::fit(F_ARIAL10, 1.5, help, BOX.w - 110.0), BOX.x + 24.0, BOX.y + BOX.h - 24.0, help, navy);
    }
}
