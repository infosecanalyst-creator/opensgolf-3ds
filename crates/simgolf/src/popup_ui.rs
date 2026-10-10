//! The exe's generic popup menu (0x46d6e0, docs/DECODE_MENUS.md 1): a box of centred heading lines and selectable option
//! lines, sized from its longest line, with the Up and Down keys, Enter or Space to accept, Esc to cancel, a click on an
//! option, and greyed options that refuse with the error sound. The Information and System Functions menus of the dock and
//! the Preferences checkboxes are built on it. The box geometry follows the decode; its look follows footage of the
//! original (the retirement question, the SGA's offer, the skill points waiver, the tournament recommendations and the
//! Information menu): see `ChoiceBox`.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{rgb, rgba, Screen as Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupKind {
    /// The dock's Information button.
    Info,
    /// The dock's System Functions button (and Esc during play).
    System,
    /// Preferences: a checkbox list.
    Prefs,
    /// The County Commissioner's offer to buy land.
    LandOffer,
    /// The end of a career: the board terminated the contract (main loop 0x420ae2).
    CareerOver,
    /// System Functions, Quit: "After a short career <pro> plans his retirement." (docs/DECODE_WORLD2.md 6.5, footage of
    /// the original at its (400, 100)).
    Retire,
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
    /// The option lit by the pointer or the keys: none at first (0x46d6e0 starts at -1), the one under the pointer, or the
    /// last one the Up and Down keys reached (kept while the pointer is off the options).
    pub sel: Option<usize>,
    pub keyed: bool,
    pub cx: f32,
    pub top: f32,
    /// The screen the popup came from.
    pub back: Screen,
}

/// The generic popup (0x46d6e0, EXACT from the code): one text blob split into lines, those starting with a space being
/// options. Its box is as wide as the widest line (options measured with their leading space) rounded up to 15 mod 16,
/// plus 0x31, centred on the call's x; it is (lines * 3 + 3) * 8 high from the call's y. An even y (every call in the
/// exe) draws the InfoButtons frame (0x40cc00: the box grown to 15 mod 16 about its centre, the inside filled with
/// 0x4e79 four pixels in, the 3 x 3 border of 16 x 16 pieces); an odd y would draw a flat box instead (not used).
///
/// The lines start 12 below the call's y, 18 apart until the first option and 24 apart from it on. Headings are centred
/// on the call's x in the shadowed call (0x404bc0): white in a radio box (mode 1), 0x7ff0 pale yellow in a checkbox list
/// (mode 0). Options are drawn 36 into the box (the leading space included): the chosen one (a radio box's option under
/// the pointer or picked by the keys, a checkbox list's ticked ones) white, the others teal 0x0210, the one under the
/// pointer in the shadowed call a pixel higher (y + 7, the others y + 8), greyed ones 0x6318. A radio box puts the
/// TransPopups ball at (box x + 12, line y + 4): the lit ball for the chosen option, the dim ball otherwise; a checkbox
/// list puts the InfoButtons box at (box x + 8, line y), green-ticked when ticked and red-crossed when not, and its OK
/// tick (TransPopups (350, 140)) 40 in from the box's right and bottom.
#[derive(Clone, Debug)]
pub struct ChoiceBox {
    pub lines: Vec<String>,
    pub cx: f32,
    pub top: f32,
    /// Checkbox mode: the ticked bits.
    pub checks: Option<u32>,
    /// Greyed option bits.
    pub disabled: u32,
}

/// The popup's font: the default font object 0x519928, Manual SSi Bold 20 (0x46d6e0 selects it first).
pub const CHOICE_FONT: crate::ui::Fnt = crate::ui::F_MANUAL20;

impl ChoiceBox {
    pub fn new(lines: Vec<String>, cx: f32, top: f32) -> ChoiceBox {
        ChoiceBox { lines, cx, top, checks: None, disabled: 0 }
    }

    /// The indexes of the option lines.
    pub fn options(&self) -> Vec<usize> {
        self.lines.iter().enumerate().filter(|(_, l)| l.starts_with(' ')).map(|(i, _)| i).collect()
    }

    /// The box as 0x46d6e0 computes it (before the frame's rounding): x = cx - w / 2, y = the call's y.
    pub fn rect(&self) -> Rect {
        let widest = self.lines.iter().map(|l| CHOICE_FONT.width(l).round() as i32).max().unwrap_or(0);
        let w = (((widest - 1) | 15) + 0x31) as f32;
        let h = (self.lines.len() as f32 * 3.0 + 3.0) * 8.0;
        Rect::new(self.cx - (w / 2.0).trunc(), self.top, w, h)
    }

    /// The frame as drawn (0x40cc00 grows a size that is not a multiple of 16 to 15 mod 16 and moves back by half of what
    /// it added).
    pub fn frame(&self) -> Rect {
        let r = self.rect();
        let (x, w) = crate::message_ui::round16(r.x, r.w);
        let (y, h) = crate::message_ui::round16(r.y, r.h);
        Rect::new(x, y, w, h)
    }

    /// The y of each line (the top the text calls are given; options draw their text 8 below it, 7 when under the
    /// pointer).
    pub fn line_tops(&self) -> Vec<f32> {
        let mut y = self.top + 12.0;
        let mut seen_option = false;
        let mut v = Vec::with_capacity(self.lines.len());
        for l in &self.lines {
            v.push(y);
            seen_option |= l.starts_with(' ');
            y += if seen_option { 24.0 } else { 18.0 };
        }
        v
    }

    /// The option (counted from the first option line) under (x, y): x strictly between the box's left and 48 short of its
    /// right, y in the 24 pixel rows from the first option's line.
    pub fn option_at(&self, x: f32, y: f32) -> Option<usize> {
        let r = self.rect();
        let opts = self.options();
        let first = *opts.first()?;
        let y0 = self.line_tops()[first];
        let right = self.cx + (r.w / 2.0).trunc() - 48.0;
        if x <= r.x || x >= right || y < y0 {
            return None;
        }
        let k = ((y - y0) / 24.0) as usize;
        (k < opts.len()).then_some(k)
    }

    /// The OK tick of a checkbox list: the 50 x 50 cut at (cx + w / 2 - 40, top + h - 40); a click within 20 of its
    /// centre (both ways) accepts.
    pub fn ok_tick(&self) -> Rect {
        let r = self.rect();
        let (x, y) = (self.cx + (r.w / 2.0).trunc() - 40.0, self.top + r.h - 40.0);
        Rect::new(x + 1.0, y + 1.0, 39.0, 39.0)
    }
}

impl App {
    /// Draws a choice box with `hover` the option under the pointer (or picked by the keys).
    pub fn draw_choice_box(&self, g: &mut Gfx, s: &Ui, b: &ChoiceBox, hover: Option<usize>) {
        use crate::info_ui::c15;
        let r = b.rect();
        self.draw_info_frame(g, s, r.x, r.y, r.w, r.h);
        let f = CHOICE_FONT;
        let white = c15(0x7fff);
        let teal = c15(0x0210);
        let tops = b.line_tops();
        let opts = b.options();
        for (i, l) in b.lines.iter().enumerate() {
            let y = tops[i];
            let Some(k) = opts.iter().position(|&o| o == i) else {
                let c = if b.checks.is_some() { c15(0x7ff0) } else { white };
                s.put_shadowed(g, f, b.cx, y, l, c, true);
                continue;
            };
            let x = r.x + 36.0;
            if b.disabled & (1 << k) != 0 {
                s.put(g, f, x, y + 8.0, l, c15(0x6318));
                continue;
            }
            let lit = hover == Some(k);
            let chosen = match b.checks {
                Some(m) => m & (1 << k) != 0,
                None => lit,
            };
            let c = if chosen { white } else { teal };
            if lit {
                s.put_shadowed(g, f, x, y + 7.0, l, c, false);
            } else {
                s.put(g, f, x, y + 8.0, l, c);
            }
            match b.checks {
                Some(_) => {
                    // the InfoButtons boxes 0x561260[1] (ticked) and [2] (crossed), 26 x 26 at (300 + 50 k, 0)
                    let cut = if chosen { 350.0 } else { 400.0 };
                    if self.art.info_buttons.tex.is_some() {
                        s.image_part(g, &self.art.info_buttons, r.x + 8.0, y, cut, 0.0, 26.0, 26.0);
                    }
                }
                None if chosen => {
                    if self.art.trans.tex.is_some() {
                        s.image_part(g, &self.art.trans, r.x + 12.0, y + 4.0, 300.0, 300.0, 30.0, 30.0);
                    }
                }
                None => {
                    if self.art.radio_dim.tex.is_some() {
                        s.image(g, &self.art.radio_dim, r.x + 12.0, y + 4.0);
                    }
                }
            }
        }
        if b.checks.is_some() && self.art.trans.tex.is_some() {
            let (x, y) = (b.cx + (r.w / 2.0).trunc() - 40.0, b.top + r.h - 40.0);
            s.image_part(g, &self.art.trans, x, y, 350.0, 140.0, 50.0, 50.0);
        }
    }

    /// The opaque InfoButtons frame (0x40cc00): sizes grown to 15 mod 16 about the centre, the inside filled with 0x4e79
    /// from four pixels in, then the 16 x 16 pieces cut at (200 + 17 col, 17 row): a corner at each end, (w - 17) / 16 top
    /// and bottom pieces from x + 16 when w passes 32, as many side pieces down from y + 16 when h does.
    pub fn draw_info_frame(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
        let (x, w) = crate::message_ui::round16(x, w);
        let (y, h) = crate::message_ui::round16(y, h);
        // 0x4e79 as the 16-bit screen shows it next to the frame art (whose inside is the same (152, 152, 200)): each
        // channel shifted up by 3
        let fill = |v: u32| [((v >> 10) & 31) as f32 * 8.0 / 255.0, ((v >> 5) & 31) as f32 * 8.0 / 255.0, (v & 31) as f32 * 8.0 / 255.0, 1.0];
        s.fill(g, x + 4.0, y + 4.0, w - 8.0, h - 8.0, fill(0x4e79));
        let t = &self.art.info_buttons;
        if t.tex.is_none() {
            return;
        }
        let piece = |g: &mut Gfx, col: usize, row: usize, dx: f32, dy: f32| {
            s.image_part(g, t, dx, dy, 200.0 + 17.0 * col as f32, 17.0 * row as f32, 16.0, 16.0);
        };
        let (r, bt) = (x + w - 16.0, y + h - 16.0);
        piece(g, 0, 0, x, y);
        if w - 16.0 > 16.0 {
            for k in 1..=((w as i32 - 17) >> 4) {
                piece(g, 1, 0, x + 16.0 * k as f32, y);
                piece(g, 1, 2, x + 16.0 * k as f32, bt);
            }
        }
        if h - 16.0 > 16.0 {
            for k in 1..=((h as i32 - 17) >> 4) {
                piece(g, 0, 1, x, y + 16.0 * k as f32);
                piece(g, 2, 1, r, y + 16.0 * k as f32);
            }
        }
        piece(g, 2, 0, r, y);
        piece(g, 2, 2, r, bt);
        piece(g, 0, 2, x, bt);
    }
}

impl Popup {
    fn choice_box(&self) -> ChoiceBox {
        ChoiceBox {
            lines: self.lines.clone(),
            cx: self.cx,
            top: self.top,
            checks: (self.kind == PopupKind::Prefs).then_some(self.checks),
            disabled: self.disabled,
        }
    }
    fn options(&self) -> Vec<usize> {
        self.choice_box().options()
    }
}

/// What a popup returned: an option index (counted from the first option line), or None when cancelled.
pub type PopupResult = Option<usize>;

impl App {
    pub fn open_popup(&mut self, kind: PopupKind) {
        let tour = self.club.game & sg_core::golfer::game::TOURNAMENT != 0;
        let pro_out = self.club.gary != -1;
        let retire = {
            let me = self.club.roster.first();
            let his = if me.map(|p| p.male_bit()).unwrap_or(1) != 0 { "his" } else { "her" };
            // EXACT (main loop 0x420b4e): "lengthy" once the year index (ticks >> 13, 0 for 2001) is above 9, so from
            // 2011 on; in championship play (flag 0x4000000) the heading is "After an exciting championship" /
            // "<pro> returns home." instead. The name is the first roster golfer's and "her" follows its female bit.
            if self.club.game & sg_core::championship::CHAMPIONSHIP != 0 {
                ["After an exciting championship".to_string(), format!("{} returns home.", self.pro_name())]
            } else {
                let span = if self.econ.year_index() > 9 { "lengthy" } else { "short" };
                [format!("After a {span} career"), format!("{} plans {his} retirement.", self.pro_name())]
            }
        };
        let (lines, disabled, cx, top): (Vec<&str>, u32, f32, f32) = match kind {
            PopupKind::Retire => (
                vec![
                    retire[0].as_str(),
                    retire[1].as_str(),
                    " Wait, I don't want to quit yet!",
                    " I'd like to save this game first.",
                    " So long for now.",
                ],
                0,
                400.0,
                100.0,
            ),
            PopupKind::Info => (
                vec![
                    // footage of the original (G4 review): "Information..."
                    "Information...",
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
                250.0,
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
                250.0,
                340.0,
            ),
            PopupKind::Prefs => (
                vec![
                    "Preferences...",
                    " Show golfers' thoughts",
                    " Advisor and first-time messages",
                    " Ambient animals",
                    " Sound",
                    " HD graphics (off: Classic)",
                ],
                0,
                400.0,
                200.0,
            ),
            // the commissioner's approval (main routine 0x41e2a0): the two-choice box centred on x 0x1c2 from y 0xa0 (EXACT,
            // the call's pushes at 0x41e2ee and 0x41e303), texts 0x4c5d74 and 0x4c5d48
            PopupKind::LandOffer => (
                vec![
                    "Do you wish to purchase additional",
                    "land to expand your course?",
                    " Yup, I've got big plans.",
                    " No, I'm fine.",
                ],
                0,
                450.0,
                160.0,
            ),
            // the debt counter past 2 (three year ends in the red): the box at (250, 150), text 0x4c5ad0
            PopupKind::CareerOver => (
                vec![
                    "Your career as a golf course designer",
                    "has ended.  Will you...",
                    " Continue this game in Sandbox Mode.",
                    " Return to Main Menu.",
                ],
                0,
                250.0,
                150.0,
            ),
        };
        let checks = if kind == PopupKind::Prefs {
            self.show_thoughts as u32
                | (self.show_advisor as u32) << 1
                | (self.club.wildlife.enabled as u32) << 2
                | (!self.mute as u32) << 3
                | (self.hd.enabled as u32) << 4
        } else {
            0
        };
        let back = if self.screen == Screen::Popup { self.popup.as_ref().map(|p| p.back).unwrap_or(Screen::Play) } else { self.screen };
        self.popup = Some(Popup { kind, lines: lines.iter().map(|s| s.to_string()).collect(), disabled, checks, sel: None, keyed: false, cx, top, back });
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
        // footage of the original: the screen under the box is not dimmed
        self.draw_choice_box(g, &s, &p.choice_box(), p.sel);
        g.flush();
    }

    /// Keyboard (0x46d6e0, EXACT): Up and Down move without wrapping (Down from none lights the first option), Enter or
    /// Space accept the lit option (none lit: cancelled), Esc cancels. Returns Some when the popup is done.
    pub fn popup_key(&mut self, k: miniquad::KeyCode) -> Option<PopupResult> {
        use miniquad::KeyCode;
        let p = self.popup.as_mut()?;
        let n = p.options().len();
        match k {
            KeyCode::Up => {
                p.keyed = true;
                if let Some(s) = p.sel.filter(|&s| s > 0) {
                    p.sel = Some(s - 1);
                }
            }
            KeyCode::Down => {
                p.keyed = true;
                match p.sel {
                    None if n > 0 => p.sel = Some(0),
                    Some(s) if s + 1 < n => p.sel = Some(s + 1),
                    _ => {}
                }
            }
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => {
                if p.sel.is_none() && p.kind != PopupKind::Prefs {
                    return Some(None);
                }
                return Some(self.popup_accept(None));
            }
            KeyCode::Escape => return Some(None),
            _ => {}
        }
        None
    }

    /// Pointer: hovering moves the selection, a click on an option chooses it, a click on the OK ball accepts, a click
    /// outside the box cancels.
    pub fn popup_pointer(&mut self, vx: f32, vy: f32, click: bool) -> Option<PopupResult> {
        let p = self.popup.as_mut()?;
        let b = p.choice_box();
        let r = b.frame();
        let hit = b.option_at(vx, vy);
        if hit.is_some() {
            p.sel = hit;
            p.keyed = false;
        } else if !p.keyed {
            p.sel = None;
        }
        if !click {
            return None;
        }
        if b.checks.is_some() && b.ok_tick().has(vx, vy) {
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
        let k = pick.or(p.sel).unwrap_or(0);
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
