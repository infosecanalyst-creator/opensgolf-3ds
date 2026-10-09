//! The exe's generic popup menu (0x46d6e0, docs/DECODE_MENUS.md 1): a box of centred heading lines and selectable option
//! lines, sized from its longest line, with the Up and Down keys, Enter or Space to accept, Esc to cancel, a click on an
//! option, and greyed options that refuse with the error sound. The Information and System Functions menus of the dock and
//! the Preferences checkboxes are built on it. Drawing is our own; the box geometry follows the decode.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{rgb, rgba, text_width, Screen as Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupKind {
    /// The dock's Information button.
    Info,
    /// The dock's System Functions button (and Esc during play).
    System,
    /// Preferences: a checkbox list.
    Prefs,
}

#[derive(Clone, Debug)]
pub struct Popup {
    pub kind: PopupKind,
    /// Lines starting with a space are options; the others are centred headings.
    pub lines: Vec<String>,
    /// Option bits that are greyed out.
    pub disabled: u32,
    /// Checkbox bits (Prefs).
    pub checks: u32,
    pub sel: usize,
    pub cx: f32,
    pub top: f32,
    /// The screen the popup came from.
    pub back: Screen,
}

const PITCH: f32 = 24.0;
const SIZE: f32 = 15.0;

impl Popup {
    fn options(&self) -> Vec<usize> {
        self.lines.iter().enumerate().filter(|(_, l)| l.starts_with(' ')).map(|(i, _)| i).collect()
    }
    fn rect(&self) -> Rect {
        let w = self.lines.iter().map(|l| text_width(l.trim(), SIZE)).fold(0.0, f32::max) + 49.0 + 30.0;
        let h = (self.lines.len() as f32 * 3.0 + 3.0) * 8.0 + 20.0;
        Rect::new(self.cx - w / 2.0, self.top, w, h)
    }
    fn ok_ball(&self) -> (f32, f32) {
        let r = self.rect();
        (r.x + r.w - 20.0, r.y + r.h - 18.0)
    }
}

/// What a popup returned: an option index (counted from the first option line), or None when cancelled.
pub type PopupResult = Option<usize>;

impl App {
    pub fn open_popup(&mut self, kind: PopupKind) {
        let tour = self.club.game & sg_core::golfer::game::TOURNAMENT != 0;
        let pro_out = self.club.gary != -1;
        let (lines, disabled, cx, top): (Vec<&str>, u32, f32, f32) = match kind {
            PopupKind::Info => (
                vec![
                    "Information",
                    " Repeat Last Message",
                    " Course Report",
                    " Player Comments",
                    " Routing Map",
                    " Histogram",
                    " SGA Evaluation",
                    " Financial Report",
                    " Membership Roster",
                    " Professional Accomplishments",
                    " World Map",
                    " Best Scores",
                    " Top 10 Designers",
                ],
                if self.last_message.is_empty() { 1 } else { 0 },
                200.0,
                40.0,
            ),
            PopupKind::System => (
                vec![
                    "System Functions",
                    " Save the current game",
                    " Load a previous game",
                    " Cancel match or tournament",
                    " Save your pro for Championship",
                    " Rename your course",
                    " Preferences",
                    " Save course for Championship",
                    " Quit to the title screen",
                    " Back to the game",
                ],
                if tour || pro_out { 0 } else { 1 << 2 } | if tour { 1 << 6 } else { 0 },
                400.0,
                110.0,
            ),
            PopupKind::Prefs => (
                vec!["Preferences...", " Show golfers' thoughts", " Advisor and first-time messages", " Ambient animals", " Sound"],
                0,
                400.0,
                150.0,
            ),
        };
        let checks = if kind == PopupKind::Prefs {
            self.show_thoughts as u32
                | (self.show_advisor as u32) << 1
                | (self.club.wildlife.enabled as u32) << 2
                | (!self.mute as u32) << 3
        } else {
            0
        };
        let back = if self.screen == Screen::Popup { self.popup.as_ref().map(|p| p.back).unwrap_or(Screen::Play) } else { self.screen };
        self.popup = Some(Popup { kind, lines: lines.iter().map(|s| s.to_string()).collect(), disabled, checks, sel: 0, cx, top, back });
        self.screen = Screen::Popup;
    }

    pub fn close_popup(&mut self) {
        if let Some(p) = self.popup.take() {
            self.screen = p.back;
        }
    }

    pub fn draw_popup(&mut self, g: &mut Gfx) {
        let Some(p) = self.popup.clone() else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        let r = p.rect();
        s.fill(g, 0.0, 0.0, 800.0, 600.0, rgba(0.0, 0.0, 0.0, 0.25));
        s.fill(g, r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, rgba(0.55, 0.55, 0.85, 0.95));
        s.fill(g, r.x, r.y, r.w, r.h, rgba(0.92, 0.92, 1.0, 0.97));
        let opts = p.options();
        for (i, l) in p.lines.iter().enumerate() {
            let y = r.y + 26.0 + PITCH * i as f32;
            if let Some(k) = opts.iter().position(|&o| o == i) {
                let off = p.disabled & (1 << k) != 0;
                if k == p.sel {
                    s.fill(g, r.x + 8.0, y - SIZE - 2.0, r.w - 16.0, PITCH - 2.0, rgba(0.98, 0.85, 0.2, 0.9));
                }
                let mut x = r.x + 36.0;
                if p.kind == PopupKind::Prefs {
                    s.fill(g, r.x + 14.0, y - 13.0, 14.0, 14.0, rgb(1.0, 1.0, 1.0));
                    if p.checks & (1 << k) != 0 {
                        s.text(g, r.x + 15.0, y, "x", 15.0, rgb(0.1, 0.1, 0.3));
                    }
                    x += 4.0;
                }
                let c = if off { rgb(0.6, 0.6, 0.65) } else { rgb(0.08, 0.08, 0.25) };
                s.text(g, x, y, l.trim(), SIZE, c);
            } else {
                s.text_centered(g, r.x + r.w / 2.0, y, l, SIZE + 2.0, rgb(0.15, 0.1, 0.4));
            }
        }
        let (bx, by) = p.ok_ball();
        s.fill(g, bx - 14.0, by - 12.0, 28.0, 24.0, rgba(0.95, 0.8, 0.15, 1.0));
        s.text_centered(g, bx, by + 5.0, "OK", 12.0, rgb(0.1, 0.1, 0.3));
        g.flush();
    }

    /// Keyboard: Up/Down move, Enter or Space accept, Esc cancels. Returns Some when the popup is done.
    pub fn popup_key(&mut self, k: miniquad::KeyCode) -> Option<PopupResult> {
        use miniquad::KeyCode;
        let p = self.popup.as_mut()?;
        let n = p.options().len();
        match k {
            KeyCode::Up => p.sel = (p.sel + n - 1) % n.max(1),
            KeyCode::Down => p.sel = (p.sel + 1) % n.max(1),
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => return Some(self.popup_accept(None)),
            KeyCode::Escape => return Some(None),
            _ => {}
        }
        None
    }

    /// Pointer: hovering moves the selection, a click on an option chooses it, a click on the OK ball accepts, a click
    /// outside the box cancels.
    pub fn popup_pointer(&mut self, vx: f32, vy: f32, click: bool) -> Option<PopupResult> {
        let p = self.popup.as_mut()?;
        let r = p.rect();
        let opts = p.options();
        let hit = opts.iter().position(|&i| {
            let y = r.y + 26.0 + PITCH * i as f32;
            vx >= r.x && vx < r.x + r.w && vy >= y - SIZE - 2.0 && vy < y - SIZE - 2.0 + PITCH
        });
        if let Some(k) = hit {
            p.sel = k;
        }
        if !click {
            return None;
        }
        let (bx, by) = p.ok_ball();
        if (vx - bx).abs() < 15.0 && (vy - by).abs() < 13.0 {
            return Some(self.popup_accept(None));
        }
        if let Some(k) = hit {
            return Some(self.popup_accept(Some(k)));
        }
        if !r.has(vx, vy) {
            return Some(None);
        }
        None
    }

    /// Accepting an option: a greyed one refuses with the error sound and keeps the popup open (None result is not
    /// returned then; the caller sees the same popup); a checkbox list toggles until OK.
    fn popup_accept(&mut self, pick: Option<usize>) -> PopupResult {
        let p = self.popup.as_mut()?;
        let k = pick.unwrap_or(p.sel);
        if p.kind == PopupKind::Prefs {
            if pick.is_some() {
                p.checks ^= 1 << k;
                return Some(usize::MAX); // toggled, stay open
            }
            return Some(p.checks as usize);
        }
        if p.disabled & (1 << k) != 0 {
            self.ui_sound(0x18);
            return Some(usize::MAX);
        }
        Some(k)
    }
}

impl App {
    /// System Functions, Rename Your Course: a one-line prompt (at most 32 characters) over the stopped game.
    pub fn open_rename(&mut self) {
        self.rename = Some(self.course_name.clone());
    }

    /// A key while the rename prompt is open: Enter keeps a valid name (refused with the error sound otherwise), Esc
    /// cancels, Backspace deletes. Returns false when the prompt is not open.
    pub fn rename_key(&mut self, k: miniquad::KeyCode) -> bool {
        use miniquad::KeyCode;
        let Some(name) = self.rename.as_mut() else { return false };
        match k {
            KeyCode::Escape => self.rename = None,
            KeyCode::Backspace => {
                name.pop();
            }
            KeyCode::Enter | KeyCode::KpEnter => {
                let n = name.trim().to_string();
                if n.is_empty() || n.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
                    self.ui_sound(0x18);
                } else {
                    self.course_name = n.clone();
                    self.club.course_name = n;
                    self.rename = None;
                }
            }
            _ => {}
        }
        true
    }

    pub fn rename_char(&mut self, c: char) {
        if let Some(name) = self.rename.as_mut() {
            if !c.is_control() && name.chars().count() < 32 {
                name.push(c);
            }
        }
    }

    pub fn draw_rename(&self, g: &mut Gfx) {
        let Some(name) = &self.rename else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        let r = Rect::new(220.0, 240.0, 360.0, 90.0);
        s.fill(g, r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, rgba(0.55, 0.55, 0.85, 0.95));
        s.fill(g, r.x, r.y, r.w, r.h, rgba(0.92, 0.92, 1.0, 0.97));
        s.text_centered(g, 400.0, 268.0, "Rename Course...", 17.0, rgb(0.15, 0.1, 0.4));
        s.fill(g, r.x + 20.0, 282.0, r.w - 40.0, 28.0, rgb(1.0, 1.0, 1.0));
        let caret = if (self.clock * 2.0) as i64 % 2 == 0 { "|" } else { "" };
        s.text(g, r.x + 26.0, 302.0, &format!("{name}{caret}"), 16.0, rgb(0.05, 0.05, 0.2));
        g.flush();
    }
}
