//! Preferences > Keyboard controls: every key the game takes, grouped (the port's own screen; the original's F8 Keyboard
//! Shortcuts lists only some of its keys, and none of the port's own). Drawn the way the port's other own screens are (the
//! Content packs screen): the System Functions box's frame and fill, its fonts and colours, the TransPopups OK tick. The
//! groups flow down two columns and on to further pages when a language's text needs more room; the page buttons, the
//! keys (Left and Right, PageUp and PageDown, Space) or a click in the box turn the pages, and Esc, Enter, the OK tick, a
//! right click or a click outside the box close it.
//!
//! The list follows the key handling in main.rs (play_key, key_down, key_down_event) and the other screens' keys; keep it
//! in step with them and with docs/PLAYING.md.
use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{rgb, Fnt, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use sg_core::lang::{tr, trf};

/// The box: the Content screen's (fits 800 x 600).
const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
/// The two columns' left edges and width.
const COL_X: [f32; 2] = [BOX.x + 22.0, 412.0];
const COL_W: f32 = 330.0;
/// The keys' column: the descriptions start this far in (further when a key's label is wider).
const KEY_W: f32 = 118.0;
/// Where the lists start and must end.
const TOP: f32 = BOX.y + 60.0;
const BOTTOM: f32 = BOX.y + BOX.h - 64.0;
/// A description line, and a group heading with the space above it.
const LINE: f32 = 16.0;
const HEAD: f32 = 20.0;
const HEAD_GAP: f32 = 6.0;
const FONT: Fnt = F_MANUAL15;

/// A group heading or a key and what it does (both keys into lang/en.txt).
#[derive(Clone, Copy)]
enum Row {
    Head(&'static str),
    Key(&'static str, &'static str),
}
use Row::{Head, Key};

/// Every key, by group (main.rs play_key and key_down; popup_ui, files_ui, content and touch for the boxes' keys).
const ROWS: &[Row] = &[
    Head("STR_KEYS_HEAD_VIEW"),
    Key("STR_KEYS_K_ARROWS", "STR_KEYS_ARROWS"),
    Key("STR_KEYS_K_ZOOM", "STR_KEYS_ZOOM"),
    Key("STR_KEYS_K_TURN", "STR_KEYS_TURN"),
    Key("STR_KEYS_K_TURN_MAC", "STR_KEYS_TURN_MAC"),
    Key("STR_KEYS_K_TURN_BACK", "STR_KEYS_TURN_BACK"),
    Key("STR_KEYS_K_HOLES", "STR_KEYS_HOLES"),
    Key("STR_KEYS_K_HOLES_BACK", "STR_KEYS_HOLES_BACK"),
    Key("STR_KEYS_K_SHIFT_HELD", "STR_KEYS_SHIFT_HELD"),
    Key("STR_KEYS_K_PHOTO_MODE", "STR_KEYS_PHOTO_MODE"),
    Key("STR_KEYS_K_HIDE_INTERFACE", "STR_KEYS_HIDE_INTERFACE"),
    Key("STR_KEYS_K_MIDDLE_DRAG", "STR_KEYS_MIDDLE_DRAG"),
    Key("STR_KEYS_K_WASD", "STR_KEYS_WASD"),
    Head("STR_KEYS_HEAD_TIME"),
    Key("STR_KEYS_K_PAUSE", "STR_KEYS_PAUSE"),
    Key("STR_KEYS_K_SPEED", "STR_KEYS_SPEED"),
    Head("STR_KEYS_HEAD_BUILD"),
    Key("STR_KEYS_K_GREEN", "STR_KEYS_GREEN"),
    Key("STR_KEYS_K_FAIRWAY", "STR_KEYS_FAIRWAY"),
    Key("STR_KEYS_K_ROUGH", "STR_KEYS_ROUGH"),
    Key("STR_KEYS_K_SAND", "STR_KEYS_SAND"),
    Key("STR_KEYS_K_TREES", "STR_KEYS_TREES"),
    Key("STR_KEYS_K_WATER", "STR_KEYS_WATER"),
    Key("STR_KEYS_K_RAVINE", "STR_KEYS_RAVINE"),
    Key("STR_KEYS_K_PATH", "STR_KEYS_PATH"),
    Key("STR_KEYS_K_BENCH", "STR_KEYS_BENCH"),
    Key("STR_KEYS_K_HOME_SITE", "STR_KEYS_HOME_SITE"),
    Key("STR_KEYS_K_ELEVATION", "STR_KEYS_ELEVATION"),
    Key("STR_KEYS_K_RAISE", "STR_KEYS_RAISE"),
    Key("STR_KEYS_K_TAB", "STR_KEYS_TAB"),
    Key("STR_KEYS_K_SPACE", "STR_KEYS_SPACE"),
    Key("STR_KEYS_K_UNDO", "STR_KEYS_UNDO"),
    Head("STR_KEYS_HEAD_HOLES"),
    Key("STR_KEYS_K_OPEN_HOLE", "STR_KEYS_OPEN_HOLE"),
    Key("STR_KEYS_K_ANALYSIS", "STR_KEYS_ANALYSIS"),
    Key("STR_KEYS_K_ANALYSIS_AGAIN", "STR_KEYS_ANALYSIS_AGAIN"),
    Key("STR_KEYS_K_TREES_OFF", "STR_KEYS_TREES_OFF"),
    Key("STR_KEYS_K_NAMES", "STR_KEYS_NAMES"),
    Key("STR_KEYS_K_MESSAGE", "STR_KEYS_MESSAGE"),
    Key("STR_KEYS_K_MUSIC", "STR_KEYS_MUSIC"),
    Head("STR_KEYS_HEAD_REPORTS"),
    Key("STR_KEYS_K_F1", "STR_KEYS_F1"),
    Key("STR_KEYS_K_F2", "STR_KEYS_F2"),
    Key("STR_KEYS_K_F3", "STR_KEYS_F3"),
    Key("STR_KEYS_K_F4", "STR_KEYS_F4"),
    Key("STR_KEYS_K_F5", "STR_KEYS_F5"),
    Key("STR_KEYS_K_F6", "STR_KEYS_F6"),
    Key("STR_KEYS_K_F7", "STR_KEYS_F7"),
    Key("STR_KEYS_K_F8", "STR_KEYS_F8"),
    Key("STR_KEYS_K_F9", "STR_KEYS_F9"),
    Key("STR_KEYS_K_F10", "STR_KEYS_F10"),
    Key("STR_KEYS_K_ROUTING", "STR_KEYS_ROUTING"),
    Key("STR_KEYS_K_WORLD", "STR_KEYS_WORLD"),
    Key("STR_KEYS_K_ALT_DIGITS", "STR_KEYS_ALT_DIGITS"),
    Key("STR_KEYS_K_CTRL_DIGITS", "STR_KEYS_CTRL_DIGITS"),
    Head("STR_KEYS_HEAD_GOLFERS"),
    Key("STR_KEYS_K_SHOTS", "STR_KEYS_SHOTS"),
    Key("STR_KEYS_K_CANCEL_ROUND", "STR_KEYS_CANCEL_ROUND"),
    Key("STR_KEYS_K_TOURNAMENT", "STR_KEYS_TOURNAMENT"),
    Key("STR_KEYS_K_LET_GO", "STR_KEYS_LET_GO"),
    Head("STR_KEYS_HEAD_FILES"),
    Key("STR_KEYS_K_SAVE", "STR_KEYS_SAVE"),
    Key("STR_KEYS_K_LOAD", "STR_KEYS_LOAD"),
    Key("STR_KEYS_K_CHAMP_COURSE", "STR_KEYS_CHAMP_COURSE"),
    Key("STR_KEYS_K_SAVE_BOX", "STR_KEYS_SAVE_BOX"),
    Key("STR_KEYS_K_EXPORT", "STR_KEYS_EXPORT"),
    Head("STR_KEYS_HEAD_PICTURES"),
    Key("STR_KEYS_K_SCREENSHOT", "STR_KEYS_SCREENSHOT"),
    Key("STR_KEYS_K_PHOTO", "STR_KEYS_PHOTO"),
    // the OpenSGolf extras' keys (extras.rs)
    Head("STR_KEYS_HEAD_EXTRAS"),
    Key("STR_KEYS_K_PEOPLE", "STR_KEYS_PEOPLE"),
    Key("STR_KEYS_K_HISTORY", "STR_KEYS_HISTORY"),
    Key("STR_KEYS_K_TAGS", "STR_KEYS_TAGS"),
    Key("STR_KEYS_K_PATHVIEW", "STR_KEYS_PATHVIEW"),
    Key("STR_KEYS_K_REPORT_TABS", "STR_KEYS_REPORT_TABS"),
    // the port's building tools (build_tools.rs)
    Key("STR_KEYS_K_TAPE", "STR_KEYS_TAPE"),
    Key("STR_KEYS_K_AREA_RECT", "STR_KEYS_AREA_RECT"),
    Key("STR_KEYS_K_AREA_FILL", "STR_KEYS_AREA_FILL"),
    Key("STR_KEYS_K_QUICK_SAVE", "STR_KEYS_QUICK_SAVE"),
    Key("STR_KEYS_K_QUICK_LOAD", "STR_KEYS_QUICK_LOAD"),
    Key("STR_KEYS_K_STAFF_VIEW", "STR_KEYS_STAFF_VIEW"),
    Head("STR_KEYS_HEAD_OTHER"),
    Key("STR_KEYS_K_ESC", "STR_KEYS_ESC"),
    Key("STR_KEYS_K_MENU_MOVE", "STR_KEYS_MENU_MOVE"),
    Key("STR_KEYS_K_MENU_CHOOSE", "STR_KEYS_MENU_CHOOSE"),
    Key("STR_KEYS_K_PAGES", "STR_KEYS_PAGES"),
    Key("STR_KEYS_K_CONTENT_ORDER", "STR_KEYS_CONTENT_ORDER"),
    Key("STR_KEYS_K_BACK_BUTTON", "STR_KEYS_BACK_BUTTON"),
    Key("STR_KEYS_K_BROWSER", "STR_KEYS_BROWSER"),
];

/// One row placed on a page: its column, its top, the key's label (none for a heading) and the text's lines.
struct Placed {
    col: usize,
    y: f32,
    key: Option<&'static str>,
    lines: Vec<String>,
}

/// Splits `t` into lines no wider than `w` in the screen's font (a word longer than the line stays whole).
fn wrap(t: &str, w: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in t.split(' ') {
        let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if !line.is_empty() && FONT.width(&next) > w {
            out.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = next;
        }
    }
    out.push(line);
    out
}

/// The rows laid out on pages: down the left column, then the right, then the next page. A group's heading is never left
/// at the foot of a column without its first key.
fn pages() -> Vec<Vec<Placed>> {
    let mut pages: Vec<Vec<Placed>> = vec![Vec::new()];
    let (mut col, mut y) = (0usize, TOP);
    let next_column = |col: &mut usize, y: &mut f32, pages: &mut Vec<Vec<Placed>>| {
        *y = TOP;
        if *col == 0 {
            *col = 1;
        } else {
            *col = 0;
            pages.push(Vec::new());
        }
    };
    for (i, row) in ROWS.iter().enumerate() {
        match *row {
            Head(h) => {
                let gap = if y > TOP { HEAD_GAP } else { 0.0 };
                // room for the heading and the first key of its group
                let first = match ROWS.get(i + 1) {
                    Some(&Key(k, d)) => key_lines(k, d).len() as f32 * LINE,
                    _ => 0.0,
                };
                if y + gap + HEAD + first > BOTTOM {
                    next_column(&mut col, &mut y, &mut pages);
                }
                let gap = if y > TOP { HEAD_GAP } else { 0.0 };
                pages.last_mut().unwrap().push(Placed { col, y: y + gap, key: None, lines: vec![tr(h).to_string()] });
                y += gap + HEAD;
            }
            Key(k, d) => {
                let lines = key_lines(k, d);
                if y + lines.len() as f32 * LINE > BOTTOM {
                    next_column(&mut col, &mut y, &mut pages);
                }
                let h = lines.len() as f32 * LINE;
                pages.last_mut().unwrap().push(Placed { col, y, key: Some(k), lines });
                y += h;
            }
        }
    }
    pages
}

/// A row's key as printed: the keys the player chose when they moved it (controls.rs), else the row's own text.
fn key_text(k: &'static str) -> String {
    crate::controls::row_label(k).unwrap_or_else(|| tr(k).to_string())
}

/// Where a key's description starts, from the column's left edge.
fn desc_x(k: &'static str) -> f32 {
    KEY_W.max(FONT.width(&key_text(k)) + 16.0)
}

/// A key's description in lines that fit beside its label.
fn key_lines(k: &'static str, d: &'static str) -> Vec<String> {
    wrap(tr(d), COL_W - desc_x(k))
}

/// The page buttons and the OK tick.
#[derive(Clone, Copy, PartialEq)]
enum Hit {
    Prev,
    Next,
    Done,
    Change,
}

fn prev_button() -> Rect {
    Rect::new(BOX.x + 24.0, BOX.y + BOX.h - 52.0, 80.0, 20.0)
}
fn next_button() -> Rect {
    Rect::new(BOX.x + 110.0, BOX.y + BOX.h - 52.0, 80.0, 20.0)
}
fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

impl App {
    /// Preferences > Keyboard controls: opens on its first page over the screen Preferences came from.
    pub fn open_keys(&mut self, back: Screen) {
        self.keys_page = 0;
        self.keys_back = if back == Screen::Keys { Screen::Play } else { back };
        self.screen = Screen::Keys;
        self.hover = -1;
    }

    fn close_keys(&mut self) {
        self.screen = self.keys_back;
        self.hover = -1;
    }

    fn keys_turn(&mut self, forward: bool) {
        let n = pages().len();
        self.keys_page = if forward { (self.keys_page + 1).min(n - 1) } else { self.keys_page.saturating_sub(1) };
    }

    fn keys_hit(&self, x: f32, y: f32) -> Option<Hit> {
        if ok_tick().has(x, y) {
            return Some(Hit::Done);
        }
        if crate::controls_ui::change_keys_button().has(x, y) {
            return Some(Hit::Change);
        }
        if pages().len() > 1 {
            if prev_button().has(x, y) {
                return Some(Hit::Prev);
            }
            if next_button().has(x, y) {
                return Some(Hit::Next);
            }
        }
        None
    }

    /// A click (or a tap): the page buttons turn the pages, the OK tick closes; elsewhere in the box a click shows the next
    /// page and on the last page closes, as does a click outside the box or a right click.
    pub fn keys_click(&mut self, x: f32, y: f32, right: bool) {
        if self.ctl.remap.is_some() {
            return self.remap_click(x, y, right);
        }
        let n = pages().len();
        match self.keys_hit(x, y) {
            _ if right => self.close_keys(),
            Some(Hit::Change) => self.open_remap(),
            Some(Hit::Done) => self.close_keys(),
            Some(Hit::Prev) => self.keys_turn(false),
            Some(Hit::Next) => self.keys_turn(true),
            None if BOX.has(x, y) && self.keys_page + 1 < n => self.keys_turn(true),
            None => self.close_keys(),
        }
    }

    /// Keys: Esc and Enter close; Right, Down, PageDown, Space and Tab show the next page, Left, Up and PageUp the one
    /// before.
    pub fn keys_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        if self.ctl.remap.is_some() {
            return self.remap_key(k);
        }
        match k {
            KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => self.close_keys(),
            KeyCode::Right | KeyCode::Down | KeyCode::PageDown | KeyCode::Space | KeyCode::Tab => self.keys_turn(true),
            KeyCode::Left | KeyCode::Up | KeyCode::PageUp => self.keys_turn(false),
            _ => {}
        }
    }

    pub fn draw_keys(&mut self, g: &mut Gfx) {
        if self.ctl.remap.is_some() {
            return self.draw_remap(g);
        }
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, grey, yellow, navy) = (c15(0x7fff), c15(0x6318), c15(0x7ff0), c15(0x0018));
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_KEYS_TITLE"), yellow, true);
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, tr("STR_KEYS_HELP"), navy);
        let pages = pages();
        let page = self.keys_page.min(pages.len() - 1);
        for p in &pages[page] {
            let x = COL_X[p.col];
            match p.key {
                None => s.put_shadowed(g, FONT, x, p.y + 2.0, &p.lines[0], yellow, false),
                Some(k) => {
                    s.put(g, FONT, x + 6.0, p.y, &key_text(k), white);
                    let dx = desc_x(k);
                    for (i, l) in p.lines.iter().enumerate() {
                        s.put(g, FONT, x + dx, p.y + i as f32 * LINE, l, navy);
                    }
                }
            }
        }
        let (px, py) = self.info.pointer;
        if pages.len() > 1 {
            let more =
                [(prev_button(), tr("STR_CONTENT_PREVIOUS"), page > 0), (next_button(), tr("STR_CONTENT_NEXT"), page + 1 < pages.len())];
            for (b, label, ok) in more {
                let lit = ok && b.has(px, py);
                s.fill(g, b.x, b.y, b.w, b.h, if lit { rgb(0.85, 0.85, 1.0) } else { rgb(0.47, 0.47, 0.70) });
                s.put_centered(g, F_ARIAL10, b.x + b.w / 2.0, b.y + 4.0, label, if ok { navy } else { grey });
            }
            let n = trf("STR_KEYS_PAGE", &[("PAGE", &(page + 1)), ("PAGES", &pages.len())]);
            s.put(g, F_ARIAL10, BOX.x + 200.0, BOX.y + BOX.h - 48.0, &n, navy);
        }
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 24.0, tr("STR_KEYS_CLOSE"), navy);
        // the port's Change keys screen (controls_ui.rs)
        let b = crate::controls_ui::change_keys_button();
        s.fill(g, b.x, b.y, b.w, b.h, if b.has(px, py) { rgb(0.85, 0.85, 1.0) } else { rgb(0.47, 0.47, 0.70) });
        s.put_centered(g, F_ARIAL10, b.x + b.w / 2.0, b.y + 4.0, tr("STR_KEYS_CHANGE"), navy);
        if self.art.trans.tex.is_some() {
            let r = ok_tick();
            s.image_part(g, &self.art.trans, r.x, r.y, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_is_placed_once_and_inside_the_box() {
        let pages = pages();
        assert_eq!(pages.iter().map(|p| p.len()).sum::<usize>(), ROWS.len());
        for p in pages.iter().flatten() {
            let h = p.lines.len() as f32 * LINE;
            assert!(p.y >= TOP && p.y + h <= BOTTOM + HEAD, "a row runs off the page");
        }
    }
}
