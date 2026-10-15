//! The port's optional gameplay changes (sg_core::extras; docs/PLAYING.md, "OpenSGolf extras"): the Preferences box that
//! switches them, the default for new games kept in settings.json, and the game side of each: Tab turning a placed building
//! or stepping through an item's designs, the extra pages of the landmark strip (cut landmarks, animals) and buying animals.
//! Nothing here runs while the extras are off.

use crate::app::App;
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
    }

    /// The extras box's ticks, one bit per option.
    pub fn extras_checks(&self) -> u32 {
        // in a game, its own extras; on the title screens, what the next game starts with
        if self.land.is_some() {
            self.club.extras.bits()
        } else {
            self.extras_next.bits()
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
        if self.club.landmarks_owned & 0x3fff != 0 {
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
                Some(PAGE_OWNED) => (0..14).filter(|&t| self.club.landmarks_owned & (1 << t) != 0).collect(),
                Some(p) => self.extra_page_types(p),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// Whether landmark type `t` may be placed: the club owns it (the original), or it is a cut landmark or an animal and
    /// that extra is on.
    pub fn landmark_allowed(&self, t: i32) -> bool {
        if (0..CUT_BASE).contains(&t) {
            return self.club.landmarks_owned & (1 << t) != 0;
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
