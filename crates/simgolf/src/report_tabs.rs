//! OpenSGolf extras: tabbed reports (extras.rs). Over the F1 to F10 screens a strip of ten tabs, one per report, flips
//! from one to the next without going back to the course; each report keeps its exact look under it (the strip takes the
//! top 15 pixels of the page, or the room above it on a taller window). A click or a tap on a tab, Tab and Shift+Tab, or
//! the report's own F key (or Alt or Ctrl with its digit) change the report; Esc and the reports' own ways close it. With
//! comments by hole on (insight.rs) an eleventh tab after F10 opens its page, and with the history graphs on (graphs.rs)
//! one more after that opens theirs.
use crate::app::*;
use crate::extras::draw_tab;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{Screen as Ui, F_ARIAL10};
use sg_core::lang::tr;

/// The tabs' labels (text keys), F1 to F10.
pub const TABS: [&str; 10] = [
    "STR_TABS_F1",
    "STR_TABS_F2",
    "STR_TABS_F3",
    "STR_TABS_F4",
    "STR_TABS_F5",
    "STR_TABS_F6",
    "STR_TABS_F7",
    "STR_TABS_F8",
    "STR_TABS_F9",
    "STR_TABS_F10",
];
const TAB_H: f32 = 14.0;
/// The tabs after F10, each while its extra is on: comments by hole (insight.rs), the history graphs (graphs.rs).
#[derive(Clone, Copy, PartialEq, Eq)]
enum After {
    Holes,
    Graphs,
}

fn after_tabs() -> Vec<After> {
    let mut v = Vec::new();
    if crate::extras::hole_comments() {
        v.push(After::Holes);
    }
    if crate::extras::graphs() {
        v.push(After::Graphs);
    }
    v
}

/// How many tabs the strip has: ten, and the extras' after them.
fn tab_count() -> usize {
    10 + after_tabs().len()
}

/// A tab's label (text key).
fn tab_label(i: usize) -> &'static str {
    match TABS.get(i) {
        Some(k) => k,
        None => match after_tabs().get(i - 10) {
            Some(After::Graphs) => "STR_TABS_GRAPHS",
            _ => "STR_TABS_HOLES",
        },
    }
}

/// A tab's width: ten share 740 pixels, eleven 792, twelve 792.
fn tab_w() -> f32 {
    match tab_count() {
        10 => 74.0,
        11 => 72.0,
        _ => 66.0,
    }
}

impl App {
    /// The report on the screen as a tab (0 for F1 to 9 for F10), None when no report is up or the tabs are off.
    pub fn report_tab(&self) -> Option<usize> {
        if !crate::extras::report_tabs() || !self.ui_ok {
            return None;
        }
        Some(match self.screen {
            Screen::Report => 0,
            Screen::Comments => 1,
            Screen::Histograph => 2,
            Screen::Finance => 3,
            Screen::Routing => 4,
            Screen::Property if self.world_move && self.info.world_go.is_none() => 5,
            Screen::Sga if self.sga.as_ref().is_some_and(|s| !s.offer) => 6,
            Screen::Shortcuts => 7,
            Screen::Roster => 8,
            Screen::Board => 9,
            Screen::HoleComments if crate::extras::hole_comments() => 10,
            Screen::Graphs if crate::extras::graphs() => 10 + after_tabs().iter().position(|&a| a == After::Graphs).unwrap_or(0),
            _ => return None,
        })
    }

    /// Changes to report `i` (0 for F1): the one up is closed as the reports close, the other opened as its key opens it.
    pub fn open_report_tab(&mut self, i: usize) {
        if self.report_tab() == Some(i) {
            return;
        }
        self.world_move = false;
        self.sga = None;
        self.screen = Screen::Play;
        self.hover = -1;
        match i {
            0 => {
                self.report_course(true);
                if self.report_art.tex.is_some() {
                    self.screen = Screen::Report;
                }
            }
            1 => self.open_report_screen(Screen::Comments),
            2 => self.open_report_screen(Screen::Histograph),
            3 => self.open_report_screen(Screen::Finance),
            4 => self.open_routing(),
            5 => self.open_world_map(),
            6 => self.sga_report(),
            7 => self.open_report_screen(Screen::Shortcuts),
            i if i >= 10 && after_tabs().get(i - 10) == Some(&After::Graphs) => self.open_graphs(),
            10 => self.open_hole_comments(),
            8 => {
                self.roster_offset = 0;
                self.screen = Screen::Roster;
            }
            _ => {
                self.screen = Screen::Board;
                self.screen_jingle(0x7e, Screen::Board);
            }
        }
        self.snd("Interface/Button1.wav", 1.0, false);
    }

    /// The strip's top: above the page when the window has room there, else over the page's top edge.
    fn tabs_top(&self) -> f32 {
        let s = Ui::new(self.draw_w, self.draw_h);
        if s.extent().1 <= -(TAB_H + 3.0) {
            -(TAB_H + 2.0)
        } else {
            0.0
        }
    }

    fn tab_rect(&self, i: usize) -> Rect {
        let (n, w) = (tab_count() as f32, tab_w());
        Rect::new(400.0 - w * n / 2.0 + w * i as f32 + 1.0, self.tabs_top() + 1.0, w - 2.0, TAB_H - 1.0)
    }

    /// A click on the strip: the tab's report. Returns true when the click was on the strip.
    pub fn report_tabs_click(&mut self, vx: f32, vy: f32) -> bool {
        if self.report_tab().is_none() {
            return false;
        }
        match (0..tab_count()).find(|&i| self.tab_rect(i).has(vx, vy)) {
            Some(i) => {
                self.open_report_tab(i);
                true
            }
            None => false,
        }
    }

    /// A key while a report is up: Tab and Shift+Tab step through the reports, a report's F key changes to it. Returns
    /// true when the key was taken.
    pub fn report_tabs_key(&mut self, k: miniquad::KeyCode, shift: bool) -> bool {
        use miniquad::KeyCode::*;
        let Some(cur) = self.report_tab() else { return false };
        let n = tab_count();
        let to = match k {
            Tab if shift => (cur + n - 1) % n,
            Tab => (cur + 1) % n,
            F1 => 0,
            F2 => 1,
            F3 => 2,
            F4 => 3,
            F5 => 4,
            F6 => 5,
            F7 => 6,
            F8 => 7,
            F9 => 8,
            F10 => 9,
            _ => return false,
        };
        if to == cur {
            // the report's own key closes it, as before
            return false;
        }
        self.open_report_tab(to);
        true
    }

    pub fn draw_report_tabs(&mut self, g: &mut Gfx) {
        let Some(cur) = self.report_tab() else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        let (px, py) = self.info.pointer;
        crate::ui::set_face(None);
        let top = self.tabs_top();
        let (n, w) = (tab_count(), tab_w());
        s.fill(g, 400.0 - w * n as f32 / 2.0 - 1.0, top - 1.0, w * n as f32 + 2.0, TAB_H + 2.0, crate::info_ui::c15(0x0008));
        for i in 0..n {
            let r = self.tab_rect(i);
            // twelve tabs are narrow: a label too wide drops its key ("Histograph" for "F3 Histograph")
            let label = tr(tab_label(i));
            let label = if F_ARIAL10.width(label) > r.w - 2.0 { label.split_once(' ').map_or(label, |l| l.1) } else { label };
            draw_tab(g, &s, r, F_ARIAL10, label, i == cur, r.has(px, py));
        }
        g.flush();
    }
}
