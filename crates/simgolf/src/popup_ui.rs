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
use sg_core::lang::{tr, trf};

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
    /// Preferences > Enhanced graphics (the port's own, enhanced.rs): a checkbox list of the Enhanced options.
    Enhanced,
    /// System Functions, Quit: "After a short career <pro> plans his retirement." (docs/DECODE_WORLD2.md 6.5, footage of
    /// the original at its (400, 100)).
    Retire,
    /// Preferences > OpenSGolf extras (the port's own): a checkbox list of the optional gameplay changes (sg_core::extras)
    /// and the interface extras (extras.rs).
    Extras,
    /// The Set par extra's confirmation (the port's own, extras_ui.rs): change a hole's par, yes or no.
    SetPar,
    /// Quick load's confirmation (the port's own, comfort.rs): load the quick save, yes or no.
    QuickLoad,
    /// Preferences > Search (the port's own, prefs_search.rs): find an option on any page, show only the changed ones,
    /// reset to the original.
    Search,
    /// The search box's "Reset to original..." confirmation (the port's own): yes or no.
    ResetOriginal,
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
    /// Preferences: the option of the port's "Use my SimGolf files..." line (gamefiles.rs), when it is shown.
    pub files_line: Option<usize>,
    /// Enhanced graphics and the OpenSGolf extras: the page (tab) shown. Those boxes and the search box keep the settings as
    /// ticked so far in `pref` (put in use by OK).
    pub page: u8,
    pub pref: crate::prefs_search::Prefs,
    /// The tab under the pointer.
    pub tab_hover: Option<u8>,
    /// The search box: the words typed, "Show only changed", and the options found.
    pub query: String,
    pub changed_only: bool,
    pub hits: Vec<crate::prefs_search::Hit>,
}

impl Popup {
    /// Shows another page of the Enhanced graphics or Extras box (what was ticked is kept).
    pub fn set_page(&mut self, page: u8) {
        if page != self.page && (page as usize) < self.page_count() {
            self.page = page;
            self.sel = None;
            self.refresh();
        }
    }

    /// The pages (tabs) of the box: the Enhanced graphics and Extras boxes have them, the others one page.
    pub fn page_count(&self) -> usize {
        match self.kind {
            PopupKind::Enhanced => crate::enhanced::PAGES.len(),
            PopupKind::Extras => crate::extras_ui::PAGES.len(),
            _ => 1,
        }
    }

    /// A page's name (its tab).
    fn page_name(&self, page: u8) -> &'static str {
        let key =
            if self.kind == PopupKind::Extras { crate::extras_ui::PAGES[page as usize] } else { crate::enhanced::PAGES[page as usize] };
        sg_core::lang::tr(key)
    }
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
    /// Checkbox mode: option bits that are commands rather than checkboxes (drawn without a box; the port's Content packs
    /// line of Preferences).
    pub plain: u32,
    /// The widest line to size the box for at least (the Enhanced graphics box keeps one size on all its pages; 0 for the
    /// exe's boxes, sized by their own lines).
    pub min_w: f32,
}

/// The popup's font: the default font object 0x519928, Manual SSi Bold 20 (0x46d6e0 selects it first).
pub const CHOICE_FONT: crate::ui::Fnt = crate::ui::F_MANUAL20;

impl ChoiceBox {
    pub fn new(lines: Vec<String>, cx: f32, top: f32) -> ChoiceBox {
        ChoiceBox { lines, cx, top, checks: None, disabled: 0, plain: 0, min_w: 0.0 }
    }

    /// The indexes of the option lines.
    pub fn options(&self) -> Vec<usize> {
        self.lines.iter().enumerate().filter(|(_, l)| l.starts_with(' ')).map(|(i, _)| i).collect()
    }

    /// The box as 0x46d6e0 computes it (before the frame's rounding): x = cx - w / 2, y = the call's y.
    pub fn rect(&self) -> Rect {
        let widest = self.lines.iter().map(|l| CHOICE_FONT.width(l).round() as i32).max().unwrap_or(0).max(self.min_w as i32);
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
    /// right, y in the 24 pixel rows from the first option's line. A line that is not an option among them (the port's Extras
    /// box has a heading between its two lists) takes its row and is no option.
    pub fn option_at(&self, x: f32, y: f32) -> Option<usize> {
        let r = self.rect();
        let opts = self.options();
        let first = *opts.first()?;
        let y0 = self.line_tops()[first];
        let right = self.cx + (r.w / 2.0).trunc() - 48.0;
        if x <= r.x || x >= right || y < y0 {
            return None;
        }
        let line = first + ((y - y0) / 24.0) as usize;
        opts.iter().position(|&o| o == line)
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
            let plain = b.plain & (1 << k) != 0;
            let chosen = match b.checks {
                Some(_) if plain => lit,
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
                Some(_) if plain => {}
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
        let fill =
            |v: u32| [((v >> 10) & 31) as f32 * 8.0 / 255.0, ((v >> 5) & 31) as f32 * 8.0 / 255.0, (v & 31) as f32 * 8.0 / 255.0, 1.0];
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

/// The exe's list box (0x46de70 / 0x46dea0, drawn by 0x46e260; EXACT from the code). It works in a 320 x 240 space that
/// 0x404970 scales to the screen ((800 v + 160) / 320 at 800 x 600): the box's corner (x0, y0) is given in that space, each
/// line is 7 below the last, the text starting at (x0 + 5, y0 + 5 + 7 i), and the box reaches x0 + 8 + the widest line's
/// pixel width * 2 / 5 and y0 + 6 + 7 lines. An odd y0 draws the box as a one pixel white outline (0x46e710, 4 higher
/// than the lines reach) with the headings in cyan 0x3ff, the options in grey 0x6318 and the option under the pointer
/// white; an even y0 draws the theme's pop-up frame (0x40d0b0, 8 and 6 out, 16 and 12 bigger) with the headings in 0x18
/// navy, the options in grey 0x4210 and the one under the pointer black. Text is Manual SSi Bold 20 (0x519928) without a
/// shadow. Lines starting with a space are the options, counted from 0.
#[derive(Clone, Debug)]
pub struct ListBox {
    pub lines: Vec<String>,
    pub x0: i32,
    pub y0: i32,
}

/// 0x404970: a 320 x 240 coordinate to the 800 x 600 screen.
pub fn scale320(v: i32) -> i32 {
    (800 * v + 160).div_euclid(320)
}

impl ListBox {
    pub fn new(lines: Vec<String>, x0: i32, y0: i32) -> ListBox {
        ListBox { lines, x0, y0 }
    }

    /// The lines shown: as many as fit above y 0xe8 of the 320 x 240 space.
    fn shown(&self) -> usize {
        self.lines.len().min(((0xe8 - self.y0) / 7).max(0) as usize)
    }

    /// The far corner (x1, y1) in the 320 x 240 space.
    fn corner(&self) -> (i32, i32) {
        let widest = self.lines.iter().map(|l| CHOICE_FONT.width(l).round() as i32).max().unwrap_or(0);
        (widest * 2 / scale320(2) + 8 + self.x0, 7 * self.shown() as i32 + 6 + self.y0)
    }

    /// The index of the first option line (0x83927c).
    fn first_option(&self) -> Option<usize> {
        self.lines.iter().position(|l| l.starts_with(' ') || l.starts_with('_'))
    }

    /// The option under the screen point (x, y), as 0x46dea0 tests it: the point taken back to the 320 x 240 space
    /// (times 4 over 0x404970(4)), inside the box, row (y - y0 - 4) / 7 counted from the first option line.
    pub fn option_at(&self, x: f32, y: f32) -> Option<usize> {
        let first = self.first_option()? as i32;
        let (lx, ly) = ((x as i32) * 4 / scale320(4), (y as i32) * 4 / scale320(4));
        let (x1, y1) = self.corner();
        if lx < self.x0 || lx > x1 || ly < self.y0 || ly > y1 {
            return None;
        }
        let row = (ly - self.y0 - 4) / 7 - first;
        let n = self.lines.iter().filter(|l| l.starts_with(' ') || l.starts_with('_')).count() as i32;
        (row >= 0 && row < n).then_some(row as usize)
    }

    /// The screen rectangle of the drawn box.
    pub fn rect(&self) -> Rect {
        let (x1, y1) = self.corner();
        if self.y0 & 1 != 0 {
            let (x, y) = (scale320(self.x0), scale320(self.y0));
            Rect::new(x as f32, y as f32, scale320(x1 - self.x0) as f32, scale320(y1 - self.y0 + 4) as f32)
        } else {
            Rect::new(
                scale320(self.x0 - 8) as f32,
                scale320(self.y0 - 6) as f32,
                scale320(x1 - self.x0 + 16) as f32,
                scale320(y1 - self.y0 + 12) as f32,
            )
        }
    }
}

impl App {
    /// Draws a list box with `hover` the option under the pointer.
    pub fn draw_list_box(&self, g: &mut Gfx, s: &Ui, b: &ListBox, hover: Option<usize>) {
        use crate::info_ui::c15;
        let r = b.rect();
        let odd = b.y0 & 1 != 0;
        if odd {
            let white = c15(0x7fff);
            s.fill(g, r.x, r.y, r.w + 1.0, 1.0, white);
            s.fill(g, r.x, r.y + r.h, r.w + 1.0, 1.0, white);
            s.fill(g, r.x, r.y, 1.0, r.h + 1.0, white);
            s.fill(g, r.x + r.w, r.y, 1.0, r.h + 1.0, white);
        } else {
            self.popup_frame(g, s, r.x, r.y, r.w, r.h);
        }
        let (heading, option, lit) = if odd { (0x03ff, 0x6318, 0x7fff) } else { (0x0018, 0x4210, 0x0000) };
        let mut k: Option<usize> = None;
        for (i, l) in b.lines.iter().take(b.shown()).enumerate() {
            let is_option = l.starts_with(' ') || l.starts_with('_');
            if is_option {
                k = Some(k.map_or(0, |k| k + 1));
            }
            let c = match (is_option, k == hover && hover.is_some()) {
                (false, _) => heading,
                (true, true) => lit,
                (true, false) => option,
            };
            let (x, y) = (scale320(b.x0 + 5), scale320(7 * i as i32 + 5 + b.y0));
            s.put(g, CHOICE_FONT, x as f32, y as f32, l, c15(c));
        }
    }
}

impl Popup {
    fn choice_box(&self) -> ChoiceBox {
        ChoiceBox {
            lines: self.lines.clone(),
            cx: self.cx,
            top: self.top,
            checks: matches!(self.kind, PopupKind::Prefs | PopupKind::Enhanced | PopupKind::Extras | PopupKind::Search)
                .then_some(self.checks),
            disabled: self.disabled,
            plain: match self.kind {
                PopupKind::Prefs => {
                    1 << PREFS_CONTENT
                        | 1 << PREFS_ENHANCED
                        | 1 << PREFS_KEYS
                        | 1 << PREFS_EXTRAS
                        | 1 << PREFS_SEARCH
                        | 1 << PREFS_LANGUAGE
                        | self.files_line.map_or(0, |k| 1 << k)
                }
                PopupKind::Enhanced => crate::enhanced::popup_plain(self.page),
                PopupKind::Extras => crate::extras_ui::popup_plain(self.page),
                PopupKind::Search => crate::prefs_search::popup_plain(&self.hits),
                _ => 0,
            },
            min_w: match self.kind {
                PopupKind::Enhanced => crate::enhanced::popup_widest(&self.pref.enh),
                PopupKind::Extras => crate::extras_ui::popup_widest(&self.pref.ex, &self.pref.ui),
                PopupKind::Search => crate::prefs_search::LINE_W,
                _ => 0.0,
            },
        }
    }

    /// The Enhanced graphics, Extras and search boxes: their lines and ticks for the page shown and the settings ticked so
    /// far (the search box: for the words typed).
    pub fn refresh(&mut self) {
        match self.kind {
            PopupKind::Enhanced => {
                self.lines = crate::enhanced::popup_lines(self.page, &self.pref.enh);
                self.checks = crate::enhanced::popup_checks(self.page, &self.pref.enh);
            }
            PopupKind::Extras => {
                self.lines = crate::extras_ui::popup_lines(self.page, &self.pref.ex, &self.pref.ui);
                self.checks = crate::extras_ui::popup_checks(self.page, &self.pref.ex, &self.pref.ui);
            }
            PopupKind::Search => {
                self.hits = crate::prefs_search::search(&self.query, self.changed_only, &self.pref);
                self.lines = crate::prefs_search::popup_lines(&self.query, true, &self.hits, &self.pref);
                self.checks = crate::prefs_search::popup_checks(self.changed_only, &self.hits, &self.pref);
                // the lit line stays on the box's options
                let n = self.options().len();
                self.sel = self.sel.filter(|&k| k < n);
            }
            _ => {}
        }
    }

    /// The page tabs of the Enhanced graphics and Extras boxes: (page, rectangle), side by side on the two lines under the
    /// title (none for the other boxes).
    pub fn tabs(&self) -> Vec<(u8, Rect)> {
        let n = self.page_count();
        if n < 2 {
            return Vec::new();
        }
        let b = self.choice_box();
        let r = b.rect();
        let tops = b.line_tops();
        let (x0, x1) = (r.x + 16.0, r.x + r.w - 16.0);
        let w = (x1 - x0) / n as f32;
        let y = tops[1] + 2.0;
        (0..n).map(|i| (i as u8, Rect::new(x0 + w * i as f32 + 2.0, y, w - 4.0, 30.0))).collect()
    }
    fn options(&self) -> Vec<usize> {
        self.choice_box().options()
    }
}

/// The option of the Preferences list that opens Preferences > Content.
pub const PREFS_CONTENT: usize = 5;
/// The option of the Preferences list that switches Enhanced graphics on and off (its master switch, enhanced.rs).
pub const PREFS_ENHANCED_ON: usize = 6;
/// The option of the Preferences list that opens Preferences > Enhanced graphics (enhanced.rs).
pub const PREFS_ENHANCED: usize = 7;
/// The option of the Preferences list that opens Preferences > Keyboard controls (keys_ui.rs).
pub const PREFS_KEYS: usize = 8;
/// The option of the Preferences list that opens Preferences > OpenSGolf extras (extras_ui.rs, extras.rs).
pub const PREFS_EXTRAS: usize = 9;
/// The option of the Preferences list that opens Preferences > Search (prefs_search.rs).
pub const PREFS_SEARCH: usize = 10;
/// The Preferences' option that switches the language (shown only when there is a translation to switch to).
const PREFS_LANGUAGE: usize = 11;
/// What the search box returns: OK; a found key (its action after this) to change on Change keys; Reset to original.
pub const SEARCH_OK: usize = 0;
pub const SEARCH_RESET: usize = 1;
pub const SEARCH_KEY: usize = 2;
/// The bit of a Preferences result that asks for the "Use my SimGolf files..." line's action (gamefiles.rs), after the
/// ticks are applied.
pub const PREFS_FILES_FLAG: usize = 20;

/// What a popup returned: an option index (counted from the first option line), or None when cancelled.
pub type PopupResult = Option<usize>;

impl App {
    pub fn open_popup(&mut self, kind: PopupKind) {
        let tour = self.club.game & sg_core::golfer::game::TOURNAMENT != 0;
        let pro_out = self.club.gary != -1;
        let retire = {
            let me = self.club.roster.first();
            let his = if me.map(|p| p.male_bit()).unwrap_or(1) != 0 { tr("STR_RETIRE_HIS") } else { tr("STR_RETIRE_HER") };
            // EXACT (main loop 0x420b4e): "lengthy" once the year index (ticks >> 13, 0 for 2001) is above 9, so from
            // 2011 on; in championship play (flag 0x4000000) the heading is "After an exciting championship" /
            // "<pro> returns home." instead. The name is the first roster golfer's and "her" follows its female bit.
            if self.club.game & sg_core::championship::CHAMPIONSHIP != 0 {
                [tr("STR_RETIRE_CHAMPIONSHIP").to_string(), trf("STR_RETIRE_RETURNS_HOME", &[("PRO", &self.pro_name())])]
            } else {
                let career = if self.econ.year_index() > 9 { "STR_RETIRE_LENGTHY_CAREER" } else { "STR_RETIRE_SHORT_CAREER" };
                [tr(career).to_string(), trf("STR_RETIRE_PLANS", &[("PRO", &self.pro_name()), ("HIS", &his)])]
            }
        };
        // the language line (docs/TRANSLATING.md), only when there is more than English to choose from
        let languages = if kind == PopupKind::Prefs { crate::language::available().len() } else { 0 };
        let language =
            if languages > 1 { trf("STR_MENU_PREFS_LANGUAGE", &[("LANGUAGE", &crate::language::current_name())]) } else { String::new() };
        let set_par_lines: [String; 2];
        let (mut lines, disabled, cx, mut top): (Vec<&str>, u32, f32, f32) = match kind {
            PopupKind::Retire => (
                vec![retire[0].as_str(), retire[1].as_str(), tr("STR_RETIRE_WAIT"), tr("STR_RETIRE_SAVE_FIRST"), tr("STR_RETIRE_SO_LONG")],
                0,
                400.0,
                100.0,
            ),
            PopupKind::Info => (
                vec![
                    // footage of the original (G4 review): "Information..."
                    tr("STR_MENU_INFO_TITLE"),
                    tr("STR_MENU_INFO_REPEAT"),
                    tr("STR_MENU_INFO_REPORT"),
                    tr("STR_MENU_INFO_COMMENTS"),
                    tr("STR_MENU_INFO_ROUTING"),
                    tr("STR_MENU_INFO_HISTOGRAM"),
                    tr("STR_MENU_INFO_SGA"),
                    tr("STR_MENU_INFO_FINANCE"),
                    tr("STR_MENU_INFO_ROSTER"),
                    tr("STR_MENU_INFO_ACCOMPLISHMENTS"),
                    tr("STR_MENU_INFO_WORLD_MAP"),
                    tr("STR_MENU_INFO_BEST_SCORES"),
                    tr("STR_MENU_INFO_TOP10"),
                ],
                if self.last_message.is_empty() { 1 } else { 0 },
                200.0,
                250.0,
            ),
            PopupKind::System => (
                vec![
                    tr("STR_MENU_SYSTEM_TITLE"),
                    tr("STR_MENU_SYSTEM_SAVE"),
                    tr("STR_MENU_SYSTEM_LOAD"),
                    tr("STR_MENU_SYSTEM_CANCEL_MATCH"),
                    tr("STR_MENU_SYSTEM_SAVE_PRO"),
                    tr("STR_MENU_SYSTEM_RENAME"),
                    tr("STR_MENU_SYSTEM_PREFERENCES"),
                    tr("STR_MENU_SYSTEM_SAVE_COURSE"),
                    tr("STR_MENU_SYSTEM_QUIT"),
                ]
                .into_iter()
                // the port's own Course sharing extra (share.rs), before Back
                .chain(crate::extras::sharing().then(|| tr("STR_MENU_SYSTEM_SHARE")))
                .chain([tr("STR_MENU_SYSTEM_BACK")])
                .collect(),
                if tour || pro_out { 0 } else { 1 << 2 } | if tour { 1 << 6 | 1 << 8 } else { 0 },
                250.0,
                // one line higher with the sharing line, so the box stays on the screen
                if crate::extras::sharing() { 308.0 } else { 340.0 },
            ),
            PopupKind::Prefs => (
                vec![
                    tr("STR_MENU_PREFS_TITLE"),
                    tr("STR_MENU_PREFS_THOUGHTS"),
                    tr("STR_MENU_PREFS_ADVISOR"),
                    tr("STR_MENU_PREFS_ANIMALS"),
                    tr("STR_MENU_PREFS_SOUND"),
                    tr("STR_MENU_PREFS_HD"),
                    // the port's own: Preferences > Content (docs/MODDING.md)
                    tr("STR_MENU_PREFS_CONTENT"),
                    // the port's own: Enhanced graphics on or off, and its settings (docs/ENHANCED.md)
                    tr("STR_MENU_PREFS_ENHANCED_ON"),
                    tr("STR_MENU_PREFS_ENHANCED"),
                    // the port's own: Preferences > Keyboard controls (keys_ui.rs)
                    tr("STR_MENU_PREFS_KEYS"),
                    // the port's own: Preferences > OpenSGolf extras (extras_ui.rs, extras.rs)
                    tr("STR_MENU_PREFS_EXTRAS"),
                    // the port's own: Preferences > Search (prefs_search.rs)
                    tr("STR_MENU_PREFS_SEARCH"),
                ],
                0,
                400.0,
                // a row higher with the Search line, so the box keeps clear of the bottom of the screen
                176.0,
            ),
            // filled in below from extras_ui's pages: the gameplay extras, the calendar and sandbox switches, the interface
            // (two rows higher since the interface page's history graphs and zoom towards the pointer, to keep its OK tick on
            // an 800 x 600 screen)
            PopupKind::Extras => (Vec::new(), 0, 400.0, 92.0),
            // filled in below from prefs_search
            PopupKind::Search => (Vec::new(), 0, 400.0, 92.0),
            PopupKind::ResetOriginal => (
                vec![tr("STR_RESET_ORIGINAL_1"), tr("STR_RESET_ORIGINAL_2"), tr("STR_RESET_ORIGINAL_YES"), tr("STR_RESET_ORIGINAL_NO")],
                0,
                400.0,
                200.0,
            ),
            // the commissioner's approval (main routine 0x41e2a0): the two-choice box centred on x 0x1c2 from y 0xa0 (EXACT,
            // the call's pushes at 0x41e2ee and 0x41e303), texts 0x4c5d74 and 0x4c5d48
            // filled in below from enhanced::OPTIONS
            PopupKind::Enhanced => (Vec::new(), 0, 400.0, 140.0),
            PopupKind::SetPar => {
                let (h, new) = self.par_change.unwrap_or((1, 4));
                let old = self.club.holes[h.clamp(1, 18)].par;
                set_par_lines = [trf("STR_SET_PAR_1", &[("HOLE", &h), ("OLD", &old), ("NEW", &new)]), tr("STR_SET_PAR_2").to_string()];
                (vec![set_par_lines[0].as_str(), set_par_lines[1].as_str(), tr("STR_SET_PAR_YES"), tr("STR_SET_PAR_NO")], 0, 400.0, 200.0)
            }
            PopupKind::QuickLoad => {
                (vec![tr("STR_QUICK_LOAD_1"), tr("STR_QUICK_LOAD_2"), tr("STR_QUICK_LOAD_YES"), tr("STR_QUICK_LOAD_NO")], 0, 400.0, 200.0)
            }
            PopupKind::LandOffer => {
                (vec![tr("STR_LAND_OFFER_1"), tr("STR_LAND_OFFER_2"), tr("STR_LAND_OFFER_YES"), tr("STR_LAND_OFFER_NO")], 0, 450.0, 160.0)
            }
            // the debt counter past 2 (three year ends in the red): the box at (250, 150), text 0x4c5ad0
            PopupKind::CareerOver => (
                vec![tr("STR_CAREER_OVER_1"), tr("STR_CAREER_OVER_2"), tr("STR_CAREER_OVER_SANDBOX"), tr("STR_CAREER_OVER_MENU")],
                0,
                250.0,
                150.0,
            ),
        };
        if languages > 1 {
            lines.push(&language);
        }
        // the port's own: "Use my SimGolf files..." or "Use the free graphics" (gamefiles.rs), last
        let files = if kind == PopupKind::Prefs { self.files_option().map(|(_, l)| format!(" {l}")) } else { None };
        let files_line = files.as_ref().map(|_| lines.len() - 1);
        if let Some(f) = &files {
            lines.push(f);
        }
        let enh = crate::enhanced::settings();
        // the Enhanced graphics box opens on its first page; its lines are made by refresh below
        // the port's own: the goals of a scenario or of the packs' scripts (docs/MODDING.md 5), after the original's lines
        if kind == PopupKind::Info && self.has_goals() {
            lines.push(tr("STR_MENU_INFO_GOALS"));
        }
        // the OpenSGolf extras' lines (extras.rs), last
        if kind == PopupKind::Info {
            for (on, key) in [
                (crate::extras::people(), "STR_MENU_INFO_PEOPLE"),
                (crate::extras::history(), "STR_MENU_INFO_HISTORY"),
                (crate::extras::path_help(), "STR_MENU_INFO_PATH_VIEW"),
                (crate::extras::landing_map(), "STR_MENU_INFO_LANDING_MAP"),
                (crate::extras::hole_comments(), "STR_MENU_INFO_HOLE_COMMENTS"),
                (crate::extras::graphs(), "STR_MENU_INFO_GRAPHS"),
            ] {
                if on {
                    lines.push(tr(key));
                    // the box moves up a row for each, to stay on the screen
                    top -= 24.0;
                }
            }
        }
        let checks = if kind == PopupKind::Prefs {
            self.show_thoughts as u32
                | (self.show_advisor as u32) << 1
                | (self.club.wildlife.enabled as u32) << 2
                | (!self.mute as u32) << 3
                | (self.hd.enabled as u32) << 4
                | (crate::enhanced::settings().enabled as u32) << PREFS_ENHANCED_ON
        } else {
            0
        };
        let back = if self.screen == Screen::Popup { self.popup.as_ref().map(|p| p.back).unwrap_or(Screen::Play) } else { self.screen };
        self.popup = Some(Popup {
            kind,
            lines: lines.iter().map(|s| s.to_string()).collect(),
            disabled,
            checks,
            sel: None,
            keyed: false,
            cx,
            top,
            back,
            files_line,
            page: 0,
            pref: crate::prefs_search::Prefs {
                enh,
                ex: self.extras_shown(),
                ui: crate::extras::settings(),
                keys: crate::controls::settings(),
            },
            tab_hover: None,
            query: self.pref_query.clone(),
            changed_only: false,
            hits: Vec::new(),
        });
        if let Some(p) = self.popup.as_mut().filter(|p| matches!(p.kind, PopupKind::Extras | PopupKind::Search)) {
            p.refresh();
        }
        if let Some(p) = self.popup.as_mut().filter(|p| p.kind == PopupKind::Enhanced) {
            p.refresh();
            // the port's own box: moved up when a page would run off the bottom of the screen (the original's popups keep
            // their places)
            let r = p.choice_box().frame();
            let over = r.y + r.h - 596.0;
            if over > 0.0 {
                p.top = (p.top - over).max(4.0);
            }
        }
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
        let mut b = p.choice_box();
        if p.kind == PopupKind::Search {
            // the typed words' caret blinks (as the rename prompt's)
            let caret = (self.clock * 2.0) as i64 % 2 == 0;
            b.lines = crate::prefs_search::popup_lines(&p.query, caret, &p.hits, &p.pref);
        }
        self.draw_choice_box(g, &s, &b, p.sel);
        self.draw_page_tabs(g, &s, &p);
        g.flush();
    }

    /// The Enhanced graphics and Extras boxes' tabs (the port's own): the page shown lit white on the box's paler blue, the
    /// others teal on a darker blue; a tab under the pointer is outlined.
    fn draw_page_tabs(&self, g: &mut Gfx, s: &Ui, p: &Popup) {
        use crate::info_ui::c15;
        let f = CHOICE_FONT;
        for (page, r) in p.tabs() {
            let on = page == p.page;
            let hover = p.tab_hover == Some(page);
            let edge = if on || hover { c15(0x7fff) } else { c15(0x2d6b) };
            s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, edge);
            s.fill(g, r.x, r.y, r.w, r.h, if on { c15(0x5ade) } else { c15(0x3dd4) });
            let label = p.page_name(page);
            let x = r.x + (r.w - f.width(label)) * 0.5;
            if on {
                s.put_shadowed(g, f, x, r.y + 5.0, label, c15(0x7fff), false);
            } else {
                s.put(g, f, x, r.y + 6.0, label, c15(if hover { 0x7fff } else { 0x0210 }));
            }
        }
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
            // the search box: Space is typed (char_event), Backspace takes back the last character
            KeyCode::Space if p.kind == PopupKind::Search => {}
            KeyCode::Backspace if p.kind == PopupKind::Search => {
                p.query.pop();
                p.refresh();
            }
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => {
                if p.sel.is_none() && !matches!(p.kind, PopupKind::Prefs | PopupKind::Enhanced | PopupKind::Extras | PopupKind::Search) {
                    return Some(None);
                }
                return Some(self.popup_accept(None));
            }
            KeyCode::Escape => return Some(None),
            // the Enhanced graphics and Extras boxes' pages (the port's own): Left and Right, or Tab, change the page
            KeyCode::Left | KeyCode::Right | KeyCode::Tab if p.page_count() > 1 => {
                let n = p.page_count() as u8;
                let page = if k == KeyCode::Left { (p.page + n - 1) % n } else { (p.page + 1) % n };
                p.set_page(page);
            }
            _ => {}
        }
        None
    }

    /// Pointer: hovering moves the selection, a click on an option chooses it, a click on the OK ball accepts, a click
    /// outside the box cancels.
    pub fn popup_pointer(&mut self, vx: f32, vy: f32, click: bool) -> Option<PopupResult> {
        let p = self.popup.as_mut()?;
        if p.page_count() > 1 {
            // the page tabs (a tap is a click here too)
            p.tab_hover = p.tabs().into_iter().find(|t| t.1.has(vx, vy)).map(|t| t.0);
            if let Some(page) = p.tab_hover {
                if click {
                    p.set_page(page);
                    return Some(Some(usize::MAX));
                }
                return None;
            }
        }
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
            // "Use my SimGolf files...": the ticks so far are kept, as with OK, then the line's action runs
            if p.files_line.is_some_and(|f| f == k && (pick.is_some() || p.sel == Some(k))) {
                return Some((p.checks | 1 << PREFS_FILES_FLAG) as usize);
            }
            if pick == Some(PREFS_LANGUAGE) {
                // the next language, at once: the box is rebuilt in it with the ticks kept
                let checks = p.checks;
                crate::language::cycle();
                self.open_popup(PopupKind::Prefs);
                if let Some(p) = self.popup.as_mut() {
                    p.checks = checks;
                }
                return Some(usize::MAX);
            }
            // the Content packs line opens its screen (the ticks so far are kept, as with OK)
            if k == PREFS_CONTENT && (pick.is_some() || p.sel == Some(k)) {
                return Some((p.checks | 1 << PREFS_CONTENT) as usize);
            }
            // so does the Enhanced graphics line
            if k == PREFS_ENHANCED && (pick.is_some() || p.sel == Some(k)) {
                return Some((p.checks | 1 << PREFS_ENHANCED) as usize);
            }
            // and the Keyboard controls line
            if k == PREFS_KEYS && (pick.is_some() || p.sel == Some(k)) {
                return Some((p.checks | 1 << PREFS_KEYS) as usize);
            }
            // and the OpenSGolf extras line
            if k == PREFS_EXTRAS && (pick.is_some() || p.sel == Some(k)) {
                return Some((p.checks | 1 << PREFS_EXTRAS) as usize);
            }
            // and the Search line
            if k == PREFS_SEARCH && (pick.is_some() || p.sel == Some(k)) {
                return Some((p.checks | 1 << PREFS_SEARCH) as usize);
            }
            if pick.is_some() {
                p.checks ^= 1 << k;
                return Some(usize::MAX); // toggled, stay open
            }
            return Some(p.checks as usize);
        }
        if p.kind == PopupKind::Extras {
            if let Some(l) = pick.and_then(|k| crate::extras_ui::page_lines(p.page).get(k).copied()) {
                crate::extras_ui::toggle_line(l, &mut p.pref.ex, &mut p.pref.ui);
                p.refresh();
                return Some(usize::MAX); // toggled, stay open
            }
            // OK: the caller takes the ticked extras from `pref`
            return Some(0);
        }
        if p.kind == PopupKind::Search {
            let Some(k) = pick else { return Some(SEARCH_OK) };
            let n = p.hits.len().min(crate::prefs_search::MAX_HITS);
            if k == 0 {
                p.changed_only = !p.changed_only;
            } else if k <= n {
                let h = p.hits[k - 1];
                if let crate::prefs_search::Hit::Key(i) = h {
                    return Some(SEARCH_KEY + i);
                }
                crate::prefs_search::toggle(h, &mut p.pref);
            } else {
                return Some(SEARCH_RESET);
            }
            // the list stays as it was found while options are ticked (only the words and "Show only changed" search
            // again), so a line does not leave the box under the pointer
            if k == 0 {
                p.refresh();
            } else {
                p.lines = crate::prefs_search::popup_lines(&p.query, true, &p.hits, &p.pref);
                p.checks = crate::prefs_search::popup_checks(p.changed_only, &p.hits, &p.pref);
            }
            return Some(usize::MAX);
        }
        if p.kind == PopupKind::Enhanced {
            if pick.is_some() {
                crate::enhanced::toggle(p.page, k, &mut p.pref.enh);
                p.refresh();
                return Some(usize::MAX); // toggled, stay open
            }
            // OK: the caller takes the ticked settings from `enh`
            return Some(0);
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
        // the OpenSGolf extras' prompts: an employee's name, a golfer's bio (extras.rs)
        if let Some(what) = self.xs.rename_for {
            match k {
                KeyCode::Escape => {
                    self.rename = None;
                    self.xs.rename_for = None;
                }
                KeyCode::Backspace => {
                    name.pop();
                }
                KeyCode::Enter | KeyCode::KpEnter => {
                    let n = name.trim().to_string();
                    match what {
                        crate::extras::RenameFor::Employee(i) => {
                            if n.is_empty() {
                                self.xs.kept.staff_names.remove(&i);
                            } else {
                                self.xs.kept.staff_names.insert(i, n);
                            }
                        }
                        crate::extras::RenameFor::Bio(r) => {
                            if let Some(p) = self.club.roster.get_mut(r.max(0) as usize) {
                                p.bio = n;
                            }
                        }
                    }
                    self.rename = None;
                    self.xs.rename_for = None;
                }
                _ => {}
            }
            return true;
        }
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
        // an employee's name as long as the course's, a bio as long as the person record keeps (500)
        let most = if matches!(self.xs.rename_for, Some(crate::extras::RenameFor::Bio(_))) { 500 } else { 32 };
        if let Some(name) = self.rename.as_mut() {
            if !c.is_control() && name.chars().count() < most {
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
        let title = match self.xs.rename_for {
            Some(crate::extras::RenameFor::Employee(_)) => tr("STR_RENAME_EMPLOYEE"),
            Some(crate::extras::RenameFor::Bio(_)) => tr("STR_RENAME_BIO"),
            None => tr("STR_RENAME_COURSE"),
        };
        s.text_centered(g, 400.0, 268.0, title, 17.0, rgb(0.15, 0.1, 0.4));
        s.fill(g, r.x + 20.0, 282.0, r.w - 40.0, 28.0, rgb(1.0, 1.0, 1.0));
        let caret = if (self.clock * 2.0) as i64 % 2 == 0 { "|" } else { "" };
        // a long text (a bio) shows its end, where the typing is
        let mut shown = format!("{name}{caret}");
        while crate::ui::text_width(&shown, 16.0) > r.w - 52.0 && shown.chars().count() > 1 {
            shown.remove(0);
        }
        s.text(g, r.x + 26.0, 302.0, &shown, 16.0, rgb(0.05, 0.05, 0.2));
        g.flush();
    }
}
