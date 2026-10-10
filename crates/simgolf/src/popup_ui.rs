//! The exe's generic popup menu (0x46d6e0, docs/DECODE_MENUS.md 1): a box of centred heading lines and selectable option
//! lines, sized from its longest line, with the Up and Down keys, Enter or Space to accept, Esc to cancel, a click on an
//! option, and greyed options that refuse with the error sound. The Information and System Functions menus of the dock and
//! the Preferences checkboxes are built on it. The box geometry follows the decode; its look follows footage of the
//! original (the retirement question, the SGA's offer, the skill points waiver, the tournament recommendations and the
//! Information menu): see `ChoiceBox`.

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
    pub sel: usize,
    pub cx: f32,
    pub top: f32,
    /// The screen the popup came from.
    pub back: Screen,
}

/// The generic popup (0x46d6e0) as footage of the original shows it: an opaque box in the lavender of its option balls'
/// cut (148, 150, 198), lit along the top and left and shaded along the bottom and right; heading lines centred, 18 apart,
/// the first capital 14 below the call's top; the options 24 apart from 23 below the last heading, their text 42 into the
/// box with the option ball 11 into it. The box is (lines * 3 + 3) * 8 high from 4 above the call's top (EXACT height, the 4
/// from footage) and as wide as its longest heading plus 0x31, or its longest option plus 75 (footage of the retirement
/// question and the SGA offer: 324 and 336 wide).
///
/// Radio boxes (mode 1) draw their headings white over a dark red shadow, the options in teal by a dark ball and the
/// option under the pointer in white with the shadow by a lit ball; checkbox lists (mode 0, the tournament
/// recommendations) draw their headings pale yellow and the options white, each over a dark shadow, with a box ticked in
/// green beside each and the yellow OK tick at the lower right.
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

/// The popup's text size (Manual SSi; footage: "2004 San Diego Open tournament" is 287 pixels wide).
pub const CHOICE_SIZE: f32 = 20.0;
const LAVENDER: [f32; 4] = [148.0 / 255.0, 150.0 / 255.0, 198.0 / 255.0, 1.0];

impl ChoiceBox {
    pub fn new(lines: Vec<String>, cx: f32, top: f32) -> ChoiceBox {
        ChoiceBox { lines, cx, top, checks: None, disabled: 0 }
    }

    /// The indexes of the option lines.
    pub fn options(&self) -> Vec<usize> {
        self.lines.iter().enumerate().filter(|(_, l)| l.starts_with(' ')).map(|(i, _)| i).collect()
    }

    pub fn rect(&self) -> Rect {
        let w = self
            .lines
            .iter()
            .map(|l| match l.strip_prefix(' ') {
                Some(o) => text_width(o.trim(), CHOICE_SIZE) + 75.0,
                None => text_width(l, CHOICE_SIZE) + 49.0,
            })
            .fold(0.0, f32::max);
        let h = (self.lines.len() as f32 * 3.0 + 3.0) * 8.0;
        // kept on the 800 x 600 screen (the exe's boxes near the edges, like the land offer at x 160, would leave it)
        let x = (self.cx - w / 2.0).clamp(4.0, (800.0 - w - 4.0).max(4.0));
        let y = (self.top - 4.0).clamp(4.0, (600.0 - h - 4.0).max(4.0));
        Rect::new(x, y, w, h)
    }

    /// The top of each line's capitals.
    pub fn line_tops(&self) -> Vec<f32> {
        let r = self.rect();
        let mut y = r.y + 18.0;
        let mut after_heading = false;
        let mut v = Vec::with_capacity(self.lines.len());
        for l in &self.lines {
            if l.starts_with(' ') {
                if after_heading {
                    y += 5.0;
                }
                v.push(y);
                y += 24.0;
                after_heading = false;
            } else {
                v.push(y);
                y += 18.0;
                after_heading = true;
            }
        }
        v
    }

    /// The option (counted from the first option line) under (x, y).
    pub fn option_at(&self, x: f32, y: f32) -> Option<usize> {
        let r = self.rect();
        if x < r.x || x >= r.x + r.w {
            return None;
        }
        let tops = self.line_tops();
        self.options().iter().position(|&i| y >= tops[i] - 5.0 && y < tops[i] + 19.0)
    }

    /// The yellow OK tick of a checkbox list (footage of the tournament recommendations: 47 in from the right, 46 up from
    /// the bottom).
    pub fn ok_tick(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x + r.w - 47.0, r.y + r.h - 46.0, 50.0, 50.0)
    }
}

impl App {
    /// Draws a choice box with `hover` the option under the pointer (or picked by the keys).
    pub fn draw_choice_box(&self, g: &mut Gfx, s: &Ui, b: &ChoiceBox, hover: Option<usize>) {
        let r = b.rect();
        // APPROXIMATION: the bevel's shades and widths are measured by eye on footage of the original
        s.fill(g, r.x, r.y, r.w, r.h, LAVENDER);
        s.fill(g, r.x, r.y, r.w, 2.0, rgb(0.78, 0.78, 0.9));
        s.fill(g, r.x, r.y, 2.0, r.h, rgb(0.72, 0.72, 0.86));
        s.fill(g, r.x + 2.0, r.y + 2.0, r.w - 4.0, 1.0, rgb(0.66, 0.66, 0.82));
        s.fill(g, r.x + r.w - 5.0, r.y + 2.0, 5.0, r.h - 2.0, rgb(0.5, 0.5, 0.68));
        s.fill(g, r.x + r.w - 3.0, r.y + 3.0, 3.0, r.h - 3.0, rgb(0.3, 0.3, 0.45));
        s.fill(g, r.x + 2.0, r.y + r.h - 5.0, r.w - 2.0, 5.0, rgb(0.5, 0.5, 0.68));
        s.fill(g, r.x + 3.0, r.y + r.h - 3.0, r.w - 3.0, 3.0, rgb(0.3, 0.3, 0.45));
        let white = rgb(1.0, 1.0, 1.0);
        let red_shadow = rgb(0.42, 0.04, 0.12);
        let dark_shadow = rgb(0.16, 0.16, 0.24);
        // APPROXIMATION: the teal and pale yellow as footage shows them, corrected for the video's colour shift
        let teal = rgb(0.09, 0.58, 0.51);
        let yellow = rgb(0.98, 0.93, 0.45);
        let grey = crate::info_ui::c15(0x4210);
        let shadowed = |g: &mut Gfx, x: f32, y: f32, t: &str, c: [f32; 4], sh: [f32; 4], centred: bool| {
            let x = if centred { x - (text_width(t, CHOICE_SIZE) / 2.0).floor() } else { x };
            s.text(g, x + 1.0, y + 1.0, t, CHOICE_SIZE, sh);
            s.text(g, x, y, t, CHOICE_SIZE, c);
        };
        let tops = b.line_tops();
        let opts = b.options();
        let balls = &self.info.art.select;
        for (i, l) in b.lines.iter().enumerate() {
            // the capitals of Manual SSi stand 12 above the baseline at this size
            let base = tops[i] + 12.0;
            let Some(k) = opts.iter().position(|&o| o == i) else {
                let c = if b.checks.is_some() { yellow } else { white };
                let sh = if b.checks.is_some() { dark_shadow } else { red_shadow };
                shadowed(g, r.x + r.w / 2.0, base, l, c, sh, true);
                continue;
            };
            let text = l.trim();
            let lit = hover == Some(k);
            let off = b.disabled & (1 << k) != 0;
            let (bx, by) = (r.x + 11.0, tops[i] - 5.0);
            match b.checks {
                Some(m) => {
                    // PLACEHOLDER: the checkbox art is not located; a cream box with a green tick as footage shows it
                    s.fill(g, bx + 1.0, by + 1.0, 20.0, 20.0, rgb(0.15, 0.15, 0.25));
                    s.fill(g, bx + 3.0, by + 3.0, 16.0, 16.0, rgb(0.93, 0.9, 0.72));
                    if m & (1 << k) != 0 {
                        let green = rgb(0.1, 0.75, 0.2);
                        for t in 0..5 {
                            s.fill(g, bx + 5.0 + t as f32, by + 9.0 + t as f32, 3.0, 3.0, green);
                        }
                        for t in 0..9 {
                            s.fill(g, bx + 10.0 + t as f32, by + 12.0 - 1.4 * t as f32, 3.0, 3.0, green);
                        }
                    }
                }
                None if balls.tex.is_some() => {
                    s.image_part(g, balls, bx, by, 243.0, if lit { 324.0 } else { 303.0 }, 22.0, 20.0);
                }
                None => {}
            }
            let x = r.x + 42.0;
            if off {
                s.text(g, x, base, text, CHOICE_SIZE, grey);
            } else if lit {
                shadowed(g, x, base, text, white, red_shadow, false);
            } else if b.checks.is_some() {
                shadowed(g, x, base, text, white, dark_shadow, false);
            } else {
                s.text(g, x, base, text, CHOICE_SIZE, teal);
            }
        }
        if b.checks.is_some() {
            let t = b.ok_tick();
            if self.art.trans.tex.is_some() {
                s.image_part(g, &self.art.trans, t.x, t.y, 350.0, 140.0, 50.0, 50.0);
            }
        }
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
            // PLACEHOLDER: the exe's test between "short" and "lengthy" is not decoded; footage shows "short" in the fourth
            // year
            let span = if self.econ.year_index() < 10 { "short" } else { "lengthy" };
            [format!("After a {span} career"), format!("{} plans {his} retirement.", self.pro_name())]
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
            // the commissioner's approval (main routine 0x41e2a0): the two-choice box at (0xa0, 0x1c2)
            PopupKind::LandOffer => (
                vec!["Do you wish to purchase additional land to expand your course?", " Yup, I've got big plans.", " No, I'm fine."],
                0,
                160.0,
                450.0,
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
        // footage of the original: the screen under the box is not dimmed
        self.draw_choice_box(g, &s, &p.choice_box(), Some(p.sel));
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
        let b = p.choice_box();
        let r = b.rect();
        let hit = b.option_at(vx, vy);
        if let Some(k) = hit {
            p.sel = k;
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
