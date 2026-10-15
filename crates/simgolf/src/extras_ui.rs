//! The port's optional gameplay changes (sg_core::extras; docs/PLAYING.md, "OpenSGolf extras"): the Preferences box that
//! switches them, the default for new games kept in settings.json, and the game side of each: Tab turning a placed building
//! or stepping through an item's designs, the extra pages of the landmark strip (cut landmarks, animals), buying animals and
//! the Hole Stats' par buttons.
//! Nothing here runs while the extras are off.

use crate::app::App;
use crate::render::Rect;
use sg_core::extras::Extras;
use sg_core::land;
use sg_core::objects::{ANIMAL_BASE, CUT_BASE, CUT_LANDMARKS};

/// The settings.json key of the extras new games start with.
const SETTINGS_KEY: &str = "extras";

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The extras new games start with (all off when none were saved).
pub fn saved() -> Extras {
    serde_json::from_value(settings_json()[SETTINGS_KEY].clone()).unwrap_or_default()
}

fn save(e: &Extras) {
    let mut v = settings_json();
    if let Ok(x) = serde_json::to_value(e) {
        v[SETTINGS_KEY] = x;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE), t.as_bytes());
    }
}

// ---- the Extras box (Preferences > OpenSGolf extras) ------------------------------------------------------------------------

/// The Extras box's pages (tabs along its top, drawn as the Enhanced graphics box's): the gameplay extras, the calendar and
/// sandbox switches, the interface extras. (One list grew too tall for an 800 x 600 screen once the calendar and sandbox
/// switches came.)
pub const PAGES: [&str; 3] = ["STR_EXTRAS_PAGE_GAMEPLAY", "STR_EXTRAS_PAGE_TIME", "STR_EXTRAS_PAGE_INTERFACE"];

/// What an option line of the Extras box is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtrasLine {
    /// Option i of sg_core::extras::OPTION_KEYS.
    Game(usize),
    /// Interface extra i (crate::extras::LINES' ticks, bit i of their Settings).
    Ui(usize),
}

/// The option lines of a page, in order.
pub fn page_lines(page: u8) -> Vec<ExtrasLine> {
    match page {
        0 => (0..sg_core::extras::SEASON_SPEED).map(ExtrasLine::Game).collect(),
        1 => (sg_core::extras::SEASON_SPEED..sg_core::extras::OPTION_KEYS.len()).map(ExtrasLine::Game).collect(),
        _ => (0..crate::extras::UI_KEYS.len()).map(ExtrasLine::Ui).collect(),
    }
}

/// The most option lines a page has (every page is drawn that tall).
fn most_lines() -> usize {
    (0..PAGES.len() as u8).map(|p| page_lines(p).len()).max().unwrap_or(0)
}

/// A line's text (with its value for the calendar speed) and whether it is on.
pub fn line_text(l: ExtrasLine, e: &Extras, ui: &crate::extras::Settings) -> (String, bool) {
    use sg_core::lang::{tr, trf};
    match l {
        ExtrasLine::Game(i) if i == sg_core::extras::SEASON_SPEED => {
            let v = e.option(i);
            (trf(sg_core::extras::OPTION_KEYS[i], &[("VALUE", &tr(sg_core::extras::SPEED_KEYS[v as usize]))]), v != 0)
        }
        ExtrasLine::Game(i) => (tr(sg_core::extras::OPTION_KEYS[i]).to_string(), e.option(i) != 0),
        ExtrasLine::Ui(i) => (tr(crate::extras::UI_KEYS[i]).to_string(), ui.bits() & 1 << i != 0),
    }
}

/// A click on a line: a tick flips, the calendar speed steps on.
pub fn toggle_line(l: ExtrasLine, e: &mut Extras, ui: &mut crate::extras::Settings) {
    match l {
        ExtrasLine::Game(i) => e.step_option(i),
        ExtrasLine::Ui(i) => *ui = crate::extras::Settings::from_bits(ui.bits() ^ 1 << i),
    }
}

/// The lines of a page of the Extras box: the title, room for the tabs, the page's options, padded with empty lines to the
/// longest page, and an empty line that keeps the OK tick clear of the last option.
pub fn popup_lines(page: u8, e: &Extras, ui: &crate::extras::Settings) -> Vec<String> {
    let mut v = vec![sg_core::lang::tr("STR_EXTRAS_TITLE").to_string()];
    v.extend((0..crate::enhanced::TAB_LINES).map(|_| String::new()));
    v.extend(page_lines(page).into_iter().map(|l| line_text(l, e, ui).0));
    let n = 1 + crate::enhanced::TAB_LINES + most_lines() + 1;
    while v.len() < n {
        v.push(String::new());
    }
    v
}

/// The box's ticks on a page (bit i the page's option i).
pub fn popup_checks(page: u8, e: &Extras, ui: &crate::extras::Settings) -> u32 {
    page_lines(page).into_iter().enumerate().fold(0, |m, (i, l)| m | (line_text(l, e, ui).1 as u32) << i)
}

/// The page's lines drawn without a tick box: the calendar speed, which has three values.
pub fn popup_plain(page: u8) -> u32 {
    page_lines(page)
        .into_iter()
        .enumerate()
        .filter(|&(_, l)| l == ExtrasLine::Game(sg_core::extras::SEASON_SPEED))
        .fold(0, |m, (i, _)| m | 1 << i)
}

/// The widest line of every page, and room for the tabs (the box keeps one width whatever the page).
pub fn popup_widest(e: &Extras, ui: &crate::extras::Settings) -> f32 {
    let f = crate::popup_ui::CHOICE_FONT;
    let lines = (0..PAGES.len() as u8).flat_map(|p| popup_lines(p, e, ui)).map(|l| f.width(&l).round()).fold(0.0, f32::max);
    let tab = PAGES.iter().map(|k| f.width(sg_core::lang::tr(k))).fold(0.0, f32::max) + 24.0;
    lines.max(tab * PAGES.len() as f32)
}

/// Pages of the landmark strip: the club's own landmarks (the original's strip), the cut ones, the animals.
pub const PAGE_OWNED: u8 = 0;
pub const PAGE_CUT: u8 = 1;
pub const PAGE_ANIMALS: u8 = 2;

/// Building kinds Tab may turn once placed: landmarks and the buildings drawn as one object (the driving range is drawn
/// tile by tile and home sites by their lot, so they keep their way).
fn turnable(kind: i32) -> bool {
    kind == land::K_LANDMARK || (6..=15).contains(&kind) && kind != land::K_DRIVING_RANGE
}

/// The game style Select Difficulty shows (docs/PLAYING.md, "Original game or OpenSGolf extras").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStyle {
    /// Every extra off and Classic graphics: the game exactly as it was (fixes for the original's own bugs stay on).
    Original,
    /// The interface extras and the gameplay extras of `Extras::OPENSGOLF`.
    Extras,
    /// Anything else the player ticked in Preferences.
    Mix,
}

impl App {
    /// The date now: the game tick, or with the Slower seasons and Pause the calendar extras the calendar's tick
    /// (sg_core::extras::Calendar).
    pub fn date(&self) -> u32 {
        self.club.cal.date(self.game_tick)
    }

    /// The game style of what the next game starts with.
    pub fn game_style(&self) -> GameStyle {
        let ui = crate::extras::settings();
        if self.extras_next.is_off() && ui == crate::extras::Settings::ALL_OFF && !crate::enhanced::settings().enabled {
            GameStyle::Original
        } else if self.extras_next == Extras::OPENSGOLF && ui == crate::extras::Settings::ALL_ON {
            GameStyle::Extras
        } else {
            GameStyle::Mix
        }
    }

    /// A click on the game style: the original game becomes the OpenSGolf extras, anything else the original game. The
    /// choice is saved like the Preferences ticks it sets; the extras leave the graphics as they are, the original game
    /// turns Enhanced graphics off.
    pub fn next_game_style(&mut self, g: &mut crate::gfx::Gfx) {
        let (e, ui) = if self.game_style() == GameStyle::Original {
            (Extras::OPENSGOLF, crate::extras::Settings::ALL_ON)
        } else {
            (Extras::default(), crate::extras::Settings::ALL_OFF)
        };
        self.extras_next = e;
        save(&e);
        crate::extras::set(ui);
        crate::extras::save(&ui);
        if ui == crate::extras::Settings::ALL_OFF {
            self.xs.path_view = false;
            let mut enh = crate::enhanced::settings();
            if enh.enabled {
                enh.enabled = false;
                if self.apply_enhanced(g, enh) {
                    self.ui_ok &= crate::load_art(self, g);
                }
            }
        }
    }

    /// The Extras box's OK: the game being played takes the ticked extras, and they become the default for new games.
    pub fn set_extras(&mut self, e: Extras) {
        self.club.extras = e;
        self.econ.extras = e;
        self.extras_next = e;
        save(&e);
        if !e.cut_content && !e.animals {
            self.strip_page = PAGE_OWNED;
        }
        if !e.set_par {
            self.club.restore_pars();
        }
    }

    /// The Set par extra's buttons on Hole Stats, right of the par: the par each one offers and its rectangle (none for a par
    /// out of `PAR_RANGE`, nothing at all when the hole's par may not be changed now).
    pub fn par_buttons(&self) -> Vec<(i32, Rect)> {
        let h = self.info.stats_hole.clamp(1, 18);
        if !self.club.can_set_par(h) {
            return Vec::new();
        }
        let par = self.club.holes[h].par;
        [(par - 1, 568.0), (par + 1, 589.0)]
            .into_iter()
            .filter(|(p, _)| sg_core::extras::PAR_RANGE.contains(p))
            .map(|(p, x)| (p, Rect::new(x, 122.0, 18.0, 17.0)))
            .collect()
    }

    /// Draws the par buttons ("-" and "+", lit under the pointer).
    pub fn draw_par_buttons(&self, g: &mut crate::gfx::Gfx, s: &crate::ui::Screen) {
        use crate::info_ui::c15;
        let par = self.club.holes[self.info.stats_hole.clamp(1, 18)].par;
        let (px, py) = self.info.pointer;
        for (p, r) in self.par_buttons() {
            let lit = r.has(px, py);
            s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, c15(0x0000));
            s.fill(g, r.x, r.y, r.w, r.h, c15(if lit { 0x5ade } else { 0x3dd4 }));
            let t = if p < par { "-" } else { "+" };
            s.text_centered(g, r.x + r.w * 0.5, r.y + 14.0, t, 14.0, c15(if lit { 0x7fff } else { 0x0000 }));
        }
    }

    /// A click on Hole Stats: a par button asks to confirm the change (true), anything else is the original's close.
    pub fn hole_stats_click(&mut self, x: f32, y: f32) -> bool {
        let Some((p, _)) = self.par_buttons().into_iter().find(|(_, r)| r.has(x, y)) else { return false };
        self.ask_par(p);
        true
    }

    /// The Set par extra's keys on Hole Stats: minus and plus ask for one par less or more. Returns whether the key was one.
    pub fn hole_stats_key(&mut self, k: miniquad::KeyCode) -> bool {
        use miniquad::KeyCode;
        let par = self.club.holes[self.info.stats_hole.clamp(1, 18)].par;
        let want = match k {
            KeyCode::Minus | KeyCode::KpSubtract => par - 1,
            KeyCode::Equal | KeyCode::KpAdd => par + 1,
            _ => return false,
        };
        if !self.par_buttons().iter().any(|&(p, _)| p == want) {
            return false;
        }
        self.ask_par(want);
        true
    }

    /// The confirmation box for hole Stats' hole getting par `p`.
    pub fn ask_par(&mut self, p: i32) {
        self.par_change = Some((self.info.stats_hole.clamp(1, 18), p));
        self.ui_sound(0x0b);
        self.open_popup(crate::popup_ui::PopupKind::SetPar);
    }

    /// The confirmation box's Yes: the hole gets the par asked for.
    pub fn confirm_par(&mut self) {
        if let Some((h, p)) = self.par_change.take() {
            self.club.set_par(h, p);
        }
    }

    /// The gameplay extras the extras box shows: in a game, its own; on the title screens, what the next game starts with.
    pub fn extras_shown(&self) -> Extras {
        if self.land.is_some() {
            self.club.extras
        } else {
            self.extras_next
        }
    }

    /// Tab with the Rotate extra on: steps through the designs of the armed garden item or landmark strip, or, with no
    /// building tool in hand, turns the building under the pointer a quarter (the clubhouse too). Returns true when it did
    /// one of those (the original's Tab is not taken then).
    pub fn extras_tab(&mut self) -> bool {
        if !self.club.extras.rotate {
            return false;
        }
        if self.edit && self.tool == 4 {
            let kind = self.build_idx as i32;
            let designs = self.strip_designs(kind);
            if designs.is_empty() {
                return false;
            }
            let cur = self.design.filter(|d| d.0 == kind).map(|d| d.1);
            let next = match cur.and_then(|c| designs.iter().position(|&d| d == c)) {
                Some(i) => designs[(i + 1) % designs.len()],
                None => designs[0],
            };
            self.design = Some((kind, next));
            self.design_hover = next;
            self.ui_sound(0x0b);
            return true;
        }
        if !self.has_hit {
            return false;
        }
        let (a, b) = self.terrain.tile_of(self.hit_x, self.hit_z);
        self.ensure_land();
        let Some(l) = self.land.as_mut() else { return false };
        // any object whose footprint holds the tile (the generator's clubhouse carries no footprint flag, so not object_on)
        let on = |o: &land::Object| {
            let s = l.footprint_size(o);
            (o.a..o.a + s).contains(&a) && (o.b..o.b + s).contains(&b)
        };
        let Some(i) = l.objects.iter().position(|o| o.kind >= 0 && turnable(o.kind) && on(o)) else { return false };
        l.objects[i].dir = (l.objects[i].dir + 1) & 3;
        self.after_object_change();
        self.snd("Interface/Building.wav", 0.5, false);
        true
    }

    /// The pages the landmark strip has: the club's own landmarks while it owns one, then the cut landmarks and the animals
    /// as their extras are on.
    pub fn strip_pages(&self) -> Vec<u8> {
        let mut v = Vec::new();
        if self.club.landmarks_shown() & 0x3fff != 0 {
            v.push(PAGE_OWNED);
        }
        if self.club.extras.cut_content {
            v.push(PAGE_CUT);
        }
        if self.club.extras.animals && !sg_core::wildlife::buyable_kinds(self.exe_theme()).is_empty() {
            v.push(PAGE_ANIMALS);
        }
        v
    }

    /// The landmark strip's page shown (the first one there is when the one chosen is not there).
    pub fn strip_page_shown(&self) -> Option<u8> {
        let pages = self.strip_pages();
        if pages.contains(&self.strip_page) {
            Some(self.strip_page)
        } else {
            pages.first().copied()
        }
    }

    /// The landmark button pressed again while its strip is open: the next page.
    pub fn next_strip_page(&mut self) {
        let pages = self.strip_pages();
        if pages.len() > 1 {
            let cur = self.strip_page_shown().unwrap_or(PAGE_OWNED);
            let i = pages.iter().position(|&p| p == cur).unwrap_or(0);
            self.strip_page = pages[(i + 1) % pages.len()];
            self.design = None;
            self.design_hover = -1;
        }
    }

    /// The landmark types of an extra page in strip order (empty for the club's own page, which is the original's).
    pub fn extra_page_types(&self, page: u8) -> Vec<i32> {
        match page {
            PAGE_CUT => (0..CUT_LANDMARKS.len() as i32).map(|i| CUT_BASE + i).collect(),
            PAGE_ANIMALS => sg_core::wildlife::buyable_kinds(self.exe_theme()).into_iter().map(|k| ANIMAL_BASE + k).collect(),
            _ => Vec::new(),
        }
    }

    /// The designs Tab steps through for a kind: the strip's designs shown (for landmarks, the page shown).
    fn strip_designs(&self, kind: i32) -> Vec<i32> {
        match kind {
            land::K_BENCH => (0..5).collect(),
            land::K_BRIDGE => (0..8).collect(),
            land::K_FLOWERS => (0..15).collect(),
            land::K_WILLOW => (0..7).collect(),
            land::K_LANDMARK => match self.strip_page_shown() {
                Some(PAGE_OWNED) => (0..14).filter(|&t| self.club.landmarks_shown() & (1 << t) != 0).collect(),
                Some(p) => self.extra_page_types(p),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// Whether landmark type `t` may be placed: the club owns it (the original) or the Unlock all extra is on, or it is a cut
    /// landmark or an animal and that extra is on.
    pub fn landmark_allowed(&self, t: i32) -> bool {
        if (0..CUT_BASE).contains(&t) {
            return self.club.landmarks_shown() & (1 << t) != 0;
        }
        if (CUT_BASE..CUT_BASE + CUT_LANDMARKS.len() as i32).contains(&t) {
            return self.club.extras.cut_content;
        }
        self.club.extras.animals && sg_core::wildlife::buyable_kinds(self.exe_theme()).contains(&(t - ANIMAL_BASE))
    }

    /// The price in units of a landmark type the original does not have (a cut landmark, an animal pair).
    pub fn extra_landmark_units(t: i32) -> i32 {
        if t >= ANIMAL_BASE {
            sg_core::extras::ANIMAL_UNITS
        } else {
            sg_core::extras::CUT_LANDMARK_UNITS
        }
    }

    /// The landmark tool with an animal chosen: buys a pair of that animal on the tile (the Buyable animals extra).
    pub fn buy_animal(&mut self, a: i32, b: i32, t: i32) {
        let ok_tile = sg_core::course::inside(a, b) && {
            let ty = self.course.ty[sg_core::course::idx(a, b)];
            !self.course.oob(a, b) && ty != land::T_WATER && ty != land::T_BUILDING && ty != land::T_HOME_SITE
        };
        if !ok_tile {
            self.ui_sound(24);
            return self.show_toast(sg_core::lang::tr("STR_TOAST_ANIMAL_NOT_HERE"));
        }
        let cost = sg_core::extras::ANIMAL_UNITS as f64 * sg_core::economy::Economy::UNIT;
        if !self.econ.affordable(cost, self.hole_numbers.len()) {
            return self.show_toast(&sg_core::lang::trf(
                "STR_TOAST_CHANGE_COSTS",
                &[("COST", &crate::ui::money(cost as i64)), ("CASH", &crate::ui::money(self.econ.cash as i64))],
            ));
        }
        if !self.club.wildlife.buy(&mut self.exe_rng, a, b, t - ANIMAL_BASE) {
            self.ui_sound(24);
            return self.show_toast(sg_core::lang::tr("STR_TOAST_ANIMALS_FULL"));
        }
        self.econ.spend_to(sg_core::economy::LEDGER_FACILITIES, cost);
    }
}
