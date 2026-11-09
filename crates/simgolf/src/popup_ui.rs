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
    /// The hidden Cheats box (the port's own, cheats_ui.rs): a list of cheats, open until Done.
    Cheats,
    /// Preferences > Graphics (the port's own, prefs_ui.rs): HD, Enhanced, pixel-perfect, Enhanced graphics settings.
    PrefsGraphics,
    /// Preferences > Game files (the port's own, prefs_ui.rs): your SimGolf files or the free graphics, Content packs,
    /// Language.
    PrefsFiles,
    /// Preferences > More (the port's own, prefs_ui.rs): the bug fixes, the update checks, Search, Report a problem, Support
    /// OpenSGolf.
    PrefsMore,
    /// Report a problem's confirmation (the port's own, bugreport.rs): what will open, a screenshot tick, OK or Cancel.
    Report,
    /// The support reminder (the port's own, support.rs): Support on Buy Me a Coffee, Maybe later, Don't ask again; over
    /// the title menu or the course.
    Support,
    /// "How would you like to play?" (the port's own, welcome.rs): the four presets, once, over the title menu.
    Welcome,
    /// "What's new in OpenSGolf" (the port's own, welcome.rs): the version's highlights with their Try it pills, See
    /// everything that's new and OK, once after an update, over the title menu.
    WhatsNew,
}

#[derive(Clone, Debug)]
pub struct Popup {
    pub kind: PopupKind,
    /// Lines starting with a space are options; the others are centred headings.
    pub lines: Vec<String>,
    /// Option bits that are greyed out.
    pub disabled: u32,
    /// Checkbox bits (Preferences and its boxes, by option).
    pub checks: u32,
    /// The option lit by the pointer or the keys: none at first (0x46d6e0 starts at -1), the one under the pointer, or the
    /// last one the Up and Down keys reached (kept while the pointer is off the options).
    pub sel: Option<usize>,
    pub keyed: bool,
    pub cx: f32,
    pub top: f32,
    /// The screen the popup came from.
    pub back: Screen,
    /// Preferences and its boxes (prefs_ui.rs): the options, in order, and what they depend on.
    pub items: Vec<crate::prefs_ui::Line>,
    pub ctx: crate::prefs_ui::Ctx,
    /// Preferences and its boxes on a touch screen: the line tapped once, whose second tap opens it (prefs_ui::opens).
    pub armed: Option<usize>,
    /// Enhanced graphics and the OpenSGolf extras: the page (tab) shown. Those boxes and the search box keep the settings as
    /// ticked so far in `pref` (put in use by OK).
    pub page: u8,
    pub pref: crate::prefs_search::Prefs,
    /// The tab under the pointer.
    pub tab_hover: Option<u8>,
    /// The Extras box and Preferences' boxes: the option last under the pointer or tapped, whose description shows at the
    /// box's foot on a touch screen (no pointer hovers there).
    pub last_pick: Option<usize>,
    /// The search box: the words typed, "Show only changed", and the options found.
    pub query: String,
    pub changed_only: bool,
    pub hits: Vec<crate::prefs_search::Hit>,
    /// The What's new box (welcome.rs): the highlights that only tell (no Try it).
    pub tell: u32,
}

impl Popup {
    /// Shows another page of the Enhanced graphics or Extras box (what was ticked is kept).
    pub fn set_page(&mut self, page: u8) {
        if page != self.page && (page as usize) < self.page_count() {
            self.page = page;
            self.sel = None;
            self.last_pick = None;
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

    /// Lines between the title and the tabs: the Extras box's preset line and a gap (extras_ui::HEAD_LINES).
    fn head_lines(&self) -> usize {
        if self.kind == PopupKind::Extras {
            crate::extras_ui::HEAD_LINES
        } else {
            0
        }
    }

    /// A page's name (its tab).
    fn page_name(&self, page: u8) -> &'static str {
        let key =
            if self.kind == PopupKind::Extras { crate::extras_ui::PAGES[page as usize] } else { crate::enhanced::PAGES[page as usize] };
        sg_core::lang::tr(key)
    }

    /// The banner under the tabs: what the page's ticks change (the Extras box), or that graphics never change play (the
    /// Enhanced graphics box).
    fn banner(&self) -> Option<&'static str> {
        match self.kind {
            PopupKind::Extras => Some(sg_core::lang::tr(crate::extras_ui::BANNERS[self.page as usize])),
            PopupKind::Enhanced => Some(sg_core::lang::tr("STR_ENHANCED_BANNER")),
            _ => None,
        }
    }
}

/// The rows under the tabs the Enhanced graphics and Extras boxes keep for their banner: its own and a gap before the
/// first option.
pub const BANNER_LINES: usize = 2;

/// The banner's font: Manual SSi Bold 20 (0x519928), as the tabs and options.
pub const BANNER_FONT: crate::ui::Fnt = crate::ui::F_MANUAL20;

/// The description at the Extras box's foot: Manual SSi Bold 16 (0x821f28), 14 (0x821ee8) when it needs more than two
/// lines at 16.
const FOOT_FONTS: [crate::ui::Fnt; 2] = [crate::ui::F_INFO16, crate::ui::F_INFO14];

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
    /// The port's own box (not one of the exe's): its headings take a dark edge all round (`OWN_EDGE`) instead of the
    /// exe's one-pixel dark red shadow, which leaves pale yellow or white over the lavender hard to read.
    pub own: bool,
    /// The port's welcome boxes (welcome.rs): the notes under an option are part of its row for the pointer and taps, so
    /// each choice is a big, touch-sized target; an empty note ends it. In a radio box the `plain` options only tell: a
    /// bullet for the ball, never lit.
    pub note_hits: bool,
}

/// The dark ink of the port's own boxes' parts (the Extras and Enhanced graphics boxes' page shown and banner, the
/// descriptions at the Extras box's foot): a deep navy of the original's 15-bit palette, in the family of the list box's
/// dark blue headings (0x46de70), about 6 to 1 against the lavender 0x4e79 and 8 to 1 against the paler blue 0x5ade.
pub const OWN_INK: u32 = 0x0010;
/// The port's own hints and notes (the Extras box's foot, the note under Preferences' bug fixes): the options' teal 0x0210
/// darkened until it reads on the lavender (about 5 to 1; the exe's teal is under 2 to 1).
pub const OWN_TEAL: u32 = 0x00c6;
/// The edge round the port's own boxes' headings and the shadow under their other light labels (a near-black navy).
pub const OWN_EDGE: u32 = 0x0004;
/// The Extras and Enhanced graphics boxes' pages not shown: a darker shade of their old 0x3dd4, so white labels read
/// on them (about 7 to 1).
const TAB_OFF: u32 = 0x2d50;

/// The popup's font: the default font object 0x519928, Manual SSi Bold 20 (0x46d6e0 selects it first).
pub const CHOICE_FONT: crate::ui::Fnt = crate::ui::F_MANUAL20;

/// The port's note under an option (a line starting with a tab, e.g. the Preferences' note under the bug fixes): no option,
/// drawn small where an option's text starts, in Manual SSi Bold 14 (0x821ee8).
pub const NOTE_FONT: crate::ui::Fnt = crate::ui::F_INFO14;

/// Whether a line is the port's note under an option.
fn is_note(l: &str) -> bool {
    l.starts_with('\t')
}

/// Whether a line is Preferences' divider between the original's ticks and the port's lines (prefs_ui::DIVIDER).
fn is_divider(l: &str) -> bool {
    l.starts_with(crate::prefs_ui::DIVIDER)
}

/// A line's width in the box's font `f` (an option with its leading space; a note in its own font at the same text size,
/// from where an option's text starts; the divider with room for its rules).
fn line_width(l: &str, f: crate::ui::Fnt) -> f32 {
    if is_note(l) {
        f.width(" ") + note_font(f).width(&l[1..])
    } else if is_divider(l) {
        f.width(&l[1..]) + 64.0
    } else {
        f.width(l)
    }
}

/// The note's font for a box drawn in `f` (it grows with the text size, textsize.rs).
fn note_font(f: crate::ui::Fnt) -> crate::ui::Fnt {
    NOTE_FONT.scaled(f.px / CHOICE_FONT.px)
}

impl ChoiceBox {
    pub fn new(lines: Vec<String>, cx: f32, top: f32) -> ChoiceBox {
        ChoiceBox { lines, cx, top, checks: None, disabled: 0, plain: 0, min_w: 0.0, own: false, note_hits: false }
    }

    /// The indexes of the option lines.
    pub fn options(&self) -> Vec<usize> {
        self.lines.iter().enumerate().filter(|(_, l)| l.starts_with(' ')).map(|(i, _)| i).collect()
    }

    /// The text size the box is drawn at (textsize.rs; the port's own): the player's, cut to what keeps the box on the
    /// 800 x 600 screen, in steps of 5%; 1 at normal size, where everything below is the exe's.
    pub fn k(&self) -> f32 {
        let want = crate::textsize::scale();
        if want == 1.0 {
            return 1.0;
        }
        let widest = (self.widest(CHOICE_FONT) as f32).max(self.min_w).max(1.0);
        let h1 = (self.lines.len() as f32 * 3.0 + 3.0) * 8.0;
        let k = want.min(592.0 / h1).min((792.0 - 0x40 as f32) / widest).max(1.0);
        (k * 20.0).floor() / 20.0
    }

    /// The box's font: the popup's, at the box's text size.
    pub fn font(&self) -> crate::ui::Fnt {
        CHOICE_FONT.scaled(self.k())
    }

    /// The room a line of the box has (the port's own, issue #164): the box grows with its widest line up to the screen's
    /// width (800 pixels with its margins); a longer line (a translation) is drawn smaller or cut to this
    /// (ui.rs fit_line), so it never runs off the screen. No line of the original's boxes comes near it.
    pub const MAX_TEXT_W: i32 = 752;

    /// A line as drawn: smaller or cut when it is wider than the box can grow (`MAX_TEXT_W`), with how far down to move
    /// it; as it is otherwise (notes and the divider always).
    fn fitted<'a>(f: crate::ui::Fnt, l: &'a str) -> (crate::ui::Fnt, std::borrow::Cow<'a, str>, f32) {
        if is_note(l) || is_divider(l) {
            return (f, std::borrow::Cow::Borrowed(l), 0.0);
        }
        crate::ui::fit_line(f, l, Self::MAX_TEXT_W as f32)
    }

    fn widest(&self, f: crate::ui::Fnt) -> i32 {
        self.lines.iter().map(|l| line_width(l, f).round() as i32).max().unwrap_or(0)
    }

    /// The box as 0x46d6e0 computes it (before the frame's rounding): x = cx - w / 2, y = the call's y. With bigger text
    /// (the port's own) it grows with its lines and moves in from the screen's edges to stay on it.
    pub fn rect(&self) -> Rect {
        let k = self.k();
        let widest = self.widest(self.font()).max((self.min_w * k) as i32).min(Self::MAX_TEXT_W);
        let w = (((widest - 1) | 15) + 0x31) as f32;
        let h = ((self.lines.len() as f32 * 3.0 + 3.0) * 8.0 * k).round();
        let (x, y) = (self.cx - (w / 2.0).trunc(), self.top);
        if k == 1.0 {
            return Rect::new(x, y, w, h);
        }
        Rect::new(x.min(796.0 - w).max(4.0), y.min(596.0 - h).max(4.0), w, h)
    }

    /// The x the box's headings are centred on (the call's x, or where the box moved to).
    pub fn centre(&self) -> f32 {
        let r = self.rect();
        r.x + (r.w / 2.0).trunc()
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
        let k = self.k();
        let mut y = self.rect().y + 12.0 * k;
        let mut seen_option = false;
        let mut v = Vec::with_capacity(self.lines.len());
        for l in &self.lines {
            v.push(y.round());
            seen_option |= l.starts_with(' ');
            y += if seen_option { 24.0 } else { 18.0 } * k;
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
        let right = self.centre() + (r.w / 2.0).trunc() - 48.0;
        if x <= r.x || x >= right || y < y0 {
            return None;
        }
        let mut line = first + ((y - y0) / (24.0 * self.k())) as usize;
        if self.note_hits {
            // a note's row belongs to the option over it (through the notes between them)
            while line > first && self.lines.get(line).is_some_and(|l| is_note(l) && l.len() > 1) {
                line -= 1;
            }
        }
        opts.iter().position(|&o| o == line)
    }

    /// The OK tick of a checkbox list: the 50 x 50 cut at (cx + w / 2 - 40, top + h - 40); a click within 20 of its
    /// centre (both ways) accepts.
    pub fn ok_tick(&self) -> Rect {
        let (x, y) = self.ok_corner();
        Rect::new(x + 1.0, y + 1.0, 39.0, 39.0)
    }

    /// Where the OK tick's 50 x 50 cut is drawn.
    pub fn ok_corner(&self) -> (f32, f32) {
        let r = self.rect();
        (self.centre() + (r.w / 2.0).trunc() - 40.0, r.y + r.h - 40.0)
    }

    /// How far below a line's top its 24-pixel-high art (the check boxes, the radio balls) goes with bigger text, to sit
    /// in the middle of the taller row.
    pub fn art_dy(&self) -> f32 {
        ((24.0 * self.k() - 24.0) / 2.0).floor()
    }
}

impl App {
    /// Draws a choice box with `hover` the option under the pointer (or picked by the keys).
    pub fn draw_choice_box(&self, g: &mut Gfx, s: &Ui, b: &ChoiceBox, hover: Option<usize>) {
        use crate::info_ui::c15;
        let r = b.rect();
        self.draw_info_frame(g, s, r.x, r.y, r.w, r.h);
        if self.on_free_set() {
            self.draw_choice_box_free(g, s, b, hover);
            return;
        }
        let f = b.font();
        let (tk, cx, dy) = (b.k(), b.centre(), b.art_dy());
        // the text's place in its row, the exe's 8 (7 lit) at normal size
        let ty = (8.0 * tk).round();
        let white = c15(0x7fff);
        let teal = c15(0x0210);
        let tops = b.line_tops();
        let opts = b.options();
        for (i, line) in b.lines.iter().enumerate() {
            let y = tops[i];
            let (lf, l, fy) = ChoiceBox::fitted(f, line);
            let Some(k) = opts.iter().position(|&o| o == i) else {
                if is_note(&l) {
                    // the port's note, in the dark teal of the Extras box's hints
                    s.put(g, note_font(f), r.x + 36.0 + f.width(" "), y + tk.round(), &l[1..], c15(OWN_TEAL));
                    continue;
                }
                if is_divider(&l) {
                    draw_divider(g, s, f, &r, y + ty, &l[1..], c15(OWN_INK));
                    continue;
                }
                let c = if b.checks.is_some() { c15(0x7ff0) } else { white };
                if b.own {
                    put_edged(g, s, lf, cx, y + fy, &l, c);
                } else {
                    s.put_shadowed(g, lf, cx, y + fy, &l, c, true);
                }
                continue;
            };
            let x = r.x + 36.0;
            if b.disabled & (1 << k) != 0 {
                s.put(g, lf, x, y + ty + fy, &l, c15(0x6318));
                continue;
            }
            let plain = b.plain & (1 << k) != 0;
            // the welcome boxes' options that only tell are never lit
            let lit = hover == Some(k) && !(plain && b.checks.is_none());
            let chosen = match b.checks {
                Some(_) if plain => lit,
                Some(m) => m & (1 << k) != 0,
                None => lit,
            };
            let c = if chosen { white } else { teal };
            if lit {
                s.put_shadowed(g, lf, x, y + ty + fy - 1.0, &l, c, false);
            } else {
                s.put(g, lf, x, y + ty + fy, &l, c);
            }
            let y = y + dy;
            match b.checks {
                Some(_) if plain => {}
                // a small dot where the ball goes (the font has no bullet)
                None if plain => s.fill(g, r.x + 24.0, y + 12.0, 5.0, 5.0, teal),
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
            let (x, y) = b.ok_corner();
            s.image_part(g, &self.art.trans, x, y, 350.0, 140.0, 50.0, 50.0);
        }
    }

    /// The free set's choice box (the same lines, places and hit areas): a header band behind the title, ink on the candy
    /// body, the row under the pointer on a light capsule, switches for check boxes and gold dots for radio choices.
    fn draw_choice_box_free(&self, g: &mut Gfx, s: &Ui, b: &ChoiceBox, hover: Option<usize>) {
        use crate::glass_ui::{self as gl, DIM_INK, INK};
        use crate::info_ui::c15;
        let r = b.rect();
        let (fx, fw) = crate::message_ui::round16(r.x, r.w);
        let (fy, _) = crate::message_ui::round16(r.y, r.h);
        let f = b.font();
        let (tk, cx) = (b.k(), b.centre());
        let ty = (8.0 * tk).round();
        // the switches and dots in the middle of the row (15 below its top at normal size)
        let mid = (12.0 * tk).round() + 3.0;
        let row = (24.0 * tk).round();
        let white = c15(0x7fff);
        let tops = b.line_tops();
        let opts = b.options();
        let head_bottom = tops.get(1).copied().unwrap_or(tops[0] + 22.0) + 1.0;
        gl::header(g, s, fx + 3.0, fy + 3.0, fw - 6.0, head_bottom - fy - 3.0, [0.24, 0.33, 0.78, 1.0]);
        for (i, line) in b.lines.iter().enumerate() {
            let y = tops[i];
            let (lf, l, fy) = ChoiceBox::fitted(f, line);
            let Some(k) = opts.iter().position(|&o| o == i) else {
                if is_note(&l) {
                    s.put(g, note_font(f), r.x + 36.0 + f.width(" "), y + tk.round(), &l[1..], INK);
                    continue;
                }
                if is_divider(&l) {
                    draw_divider(g, s, f, &r, y + ty, &l[1..], INK);
                    continue;
                }
                if i == 0 {
                    let c = if b.checks.is_some() { c15(0x7ff0) } else { white };
                    s.put_shadowed(g, lf, cx, y + fy, &l, c, true);
                } else {
                    s.put_centered(g, lf, cx, y + fy, &l, INK);
                }
                continue;
            };
            let x = r.x + 36.0;
            let plain = b.plain & (1 << k) != 0;
            let lit = hover == Some(k) && !(plain && b.checks.is_none());
            let off = b.disabled & (1 << k) != 0;
            if lit && !off {
                // the welcome boxes: the capsule takes in the notes that are part of the option's row
                let n = if b.note_hits { b.lines[i + 1..].iter().take_while(|l| is_note(l) && l.len() > 1).count() } else { 0 };
                gl::capsule(g, s, fx + 6.0, y + 3.0, fw - 12.0, row * (1 + n) as f32, [1.0, 0.93, 0.62, 0.95]);
            }
            s.put(g, lf, x, y + ty + fy, &l, if off { DIM_INK } else { INK });
            match b.checks {
                None if plain => s.fill(g, r.x.max(fx + 7.0) + 11.0, y + mid - 3.0, 6.0, 6.0, INK),
                Some(_) if plain => s.put(g, f, r.x + 12.0, y + ty, "\u{bb}", if off { DIM_INK } else { INK }),
                Some(m) => gl::switch(g, s, r.x.max(fx + 7.0), y + mid, m & (1 << k) != 0, lit),
                None => gl::radio(g, s, r.x.max(fx + 7.0) + 13.0, y + mid, lit),
            }
        }
        if b.checks.is_some() && self.art.trans.tex.is_some() {
            let (x, y) = b.ok_corner();
            s.image_part(g, &self.art.trans, x, y, 350.0, 140.0, 50.0, 50.0);
        }
    }

    /// The opaque InfoButtons frame (0x40cc00): sizes grown to 15 mod 16 about the centre, the inside filled with 0x4e79
    /// from four pixels in, then the 16 x 16 pieces cut at (200 + 17 col, 17 row): a corner at each end, (w - 17) / 16 top
    /// and bottom pieces from x + 16 when w passes 32, as many side pieces down from y + 16 when h does.
    pub fn draw_info_frame(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
        let (x, w) = crate::message_ui::round16(x, w);
        let (y, h) = crate::message_ui::round16(y, h);
        if self.on_free_set() {
            crate::glass_ui::panel(g, s, x, y, w, h);
            return;
        }
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
            checks: (crate::prefs_ui::is_prefs_box(self.kind)
                || matches!(self.kind, PopupKind::Enhanced | PopupKind::Extras | PopupKind::Search | PopupKind::Report))
            .then_some(self.checks),
            disabled: self.disabled,
            plain: match self.kind {
                k if crate::prefs_ui::is_prefs_box(k) => crate::prefs_ui::plain_bits(&self.items),
                PopupKind::Enhanced => crate::enhanced::popup_plain(self.page),
                PopupKind::Extras => crate::extras_ui::popup_plain(self.page),
                PopupKind::Search => crate::prefs_search::popup_plain(&self.hits),
                PopupKind::Report => crate::bugreport::BOX_PLAIN,
                PopupKind::WhatsNew => self.tell,
                _ => 0,
            },
            min_w: match self.kind {
                // Preferences' boxes: wide enough for their descriptions at the foot
                k if crate::prefs_ui::is_prefs_box(k) => PREFS_MIN_W,
                PopupKind::Enhanced => crate::enhanced::popup_widest(&self.pref.enh),
                PopupKind::Extras => crate::extras_ui::popup_widest(&self.pref.ex, &self.pref.ui),
                PopupKind::Search => crate::prefs_search::LINE_W,
                // room for the Try it pills right of the highlights
                PopupKind::WhatsNew => {
                    let f = CHOICE_FONT;
                    let widest = self.lines.iter().filter(|l| l.starts_with(' ')).map(|l| f.width(l)).fold(0.0, f32::max);
                    widest + crate::welcome::pill_w() + 36.0
                }
                _ => 0.0,
            },
            own: !matches!(
                self.kind,
                PopupKind::Info | PopupKind::System | PopupKind::Prefs | PopupKind::LandOffer | PopupKind::CareerOver | PopupKind::Retire
            ),
            note_hits: matches!(self.kind, PopupKind::Welcome | PopupKind::WhatsNew),
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
        let y = tops[1 + self.head_lines()] + 2.0;
        let h = (30.0 * b.k()).round();
        (0..n).map(|i| (i as u8, Rect::new(x0 + w * i as f32 + 2.0, y, w - 4.0, h))).collect()
    }
    fn options(&self) -> Vec<usize> {
        self.choice_box().options()
    }
}

/// What Preferences and its boxes return for their OK (an option that does something returns its index).
pub const BOX_OK: usize = usize::MAX - 1;
/// The narrowest Preferences and its boxes are drawn, so their foot holds a description in two lines.
const PREFS_MIN_W: f32 = 440.0;
/// What the search box returns: OK; a found key (its action after this) to change on Change keys; Reset to original.
pub const SEARCH_OK: usize = 0;
pub const SEARCH_RESET: usize = 1;
pub const SEARCH_KEY: usize = 2;
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
        // Preferences and its boxes (prefs_ui.rs): the files line, the language line (docs/TRANSLATING.md, only when there is
        // more than English to choose from) and the update check depend on the build and what is installed
        let ctx = self.prefs_ctx();
        let prefs_lines =
            if crate::prefs_ui::is_prefs_box(kind) { crate::prefs_ui::lines(kind, &ctx, crate::textsize::pct()) } else { Vec::new() };
        let set_par_lines: [String; 2];
        let cheat_lines: Vec<String>;
        let report_lines: Vec<String>;
        let support_lines: Vec<String>;
        let welcome_lines: Vec<String>;
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
            // the original's four ticks, then the port's lines (prefs_ui.rs); higher than the original's place with the port's
            // lines, so the box keeps clear of the bottom of the screen
            PopupKind::Prefs => (prefs_lines.iter().map(|l| l.as_str()).collect(), 0, 400.0, 128.0),
            PopupKind::PrefsGraphics | PopupKind::PrefsFiles | PopupKind::PrefsMore => {
                (prefs_lines.iter().map(|l| l.as_str()).collect(), 0, 400.0, 176.0)
            }
            // filled in below from extras_ui's pages (Gameplay, Sandbox, Looks, Info, Helpers), with the preset line over the
            // tabs, the banner under them and the description at the foot; the tallest page keeps its OK tick on an
            // 800 x 600 screen
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
            // the port's own Report a problem box (bugreport.rs): in the middle of the screen
            PopupKind::Report => {
                report_lines = crate::bugreport::box_lines();
                let top = crate::bugreport::box_top(report_lines.len());
                (report_lines.iter().map(|l| l.as_str()).collect(), 0, 400.0, top)
            }
            // the port's own support reminder (support.rs): in the middle of the screen
            PopupKind::Support => {
                support_lines = crate::support::box_lines(self.support.record.played);
                let top = crate::bugreport::box_top(support_lines.len());
                (support_lines.iter().map(|l| l.as_str()).collect(), 0, 400.0, top)
            }
            // the port's own welcome boxes (welcome.rs): in the middle of the screen
            PopupKind::Welcome | PopupKind::WhatsNew => {
                welcome_lines = match (kind, self.welcome.news) {
                    (PopupKind::WhatsNew, Some((v, h))) => crate::welcome::whatsnew_lines(v, h, self.on_free_set()),
                    _ => crate::welcome::welcome_lines(self.on_free_set()),
                };
                let top = crate::bugreport::box_top(welcome_lines.len());
                (welcome_lines.iter().map(|l| l.as_str()).collect(), 0, 400.0, top)
            }
            // the port's own Cheats box: below the dock's top, clear of the course's middle
            PopupKind::Cheats => {
                let (l, disabled) = self.cheat_lines();
                cheat_lines = l;
                (cheat_lines.iter().map(|l| l.as_str()).collect(), disabled, 400.0, 120.0)
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
        let enh = crate::enhanced::settings();
        // the Enhanced graphics box opens on its first page; its lines are made by refresh below
        // the port's own: the goals of a scenario or of the packs' scripts (docs/MODDING.md 5), after the original's lines
        if kind == PopupKind::Info && self.has_goals() {
            lines.push(tr("STR_MENU_INFO_GOALS"));
        }
        // the OpenSGolf extras' lines (extras.rs), last: Buy land while the Keep the land offer extra keeps an approval
        if kind == PopupKind::Info {
            for (on, key) in [
                (self.land_waiting(), "STR_MENU_INFO_BUY_LAND"),
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
        let items = crate::prefs_ui::items(kind, &ctx);
        let checks = items.iter().enumerate().fold(0, |m, (k, &l)| m | (self.prefs_tick(l) as u32) << k);
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
            items,
            ctx,
            armed: None,
            page: 0,
            pref: crate::prefs_search::Prefs {
                enh,
                ex: self.extras_shown(),
                ui: crate::extras::settings(),
                keys: crate::controls::settings(),
                text: crate::textsize::pct(),
                hd: self.hd.enabled,
            },
            tab_hover: None,
            last_pick: None,
            query: self.pref_query.clone(),
            changed_only: false,
            hits: Vec::new(),
            tell: if kind == PopupKind::WhatsNew { self.whatsnew_tell() } else { 0 },
        });
        // the cut content's line only with the original game's files (extras_ui.rs)
        crate::extras_ui::show_cut_content(!self.on_free_set());
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

    /// A popup over the title menu (the support reminder, support.rs): the menu is drawn under it and its music goes on.
    pub fn popup_over_title(&self) -> bool {
        self.screen == Screen::Popup && self.popup.as_ref().is_some_and(|p| p.back == Screen::Menu)
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
        if let Some(t) = p.banner() {
            draw_banner(g, &s, &b, t, self.on_free_set(), p.head_lines());
        }
        if p.kind == PopupKind::Extras {
            self.draw_extras_tags(g, &s, &p, &b);
        }
        if p.kind == PopupKind::WhatsNew {
            self.draw_try_pills(g, &s, &p, &b);
        }
        if crate::prefs_ui::is_prefs_box(p.kind) {
            self.draw_prefs_foot(g, &s, &p, &b);
        }
        g.flush();
    }

    /// The Extras box's own parts (the port's own): the tag pill at the right end of each gameplay and sandbox line, and at
    /// the foot the description of the line under the pointer (on a touch screen the line last tapped; with none, the
    /// page's hint; the preset line's is the preset in use).
    fn draw_extras_tags(&self, g: &mut Gfx, s: &Ui, p: &Popup, b: &ChoiceBox) {
        use crate::info_ui::c15;
        let r = b.rect();
        let tops = b.line_tops();
        // the pills grow with the text (textsize.rs)
        let tk = b.k();
        let pw = (crate::extras_ui::pill_w() * tk).round();
        let pf = crate::extras_ui::PILL_FONT.scaled(tk);
        for ((k, l), &line) in crate::extras_ui::box_lines(p.page).into_iter().enumerate().zip(&b.options()) {
            let Some(t) = crate::extras_ui::tag(l) else { continue };
            let (fill, edge) = t.colours();
            let (x, y, h) = (r.x + r.w - 24.0 - pw, tops[line] + (5.0 * tk).round(), (18.0 * tk).round());
            if self.on_free_set() {
                let c = c15(fill);
                crate::glass_ui::capsule(g, s, x, y, pw, h + 2.0, [c[0] * 1.1, c[1] * 1.1, c[2] * 1.1, 1.0]);
                let lit = p.sel == Some(k);
                put_dark_shadow(g, s, pf, x + pw * 0.5, y + 1.0, tr(t.key()), c15(if lit { 0x7fff } else { 0x7bde }), true);
                continue;
            }
            s.fill(g, x, y, pw, h, c15(edge));
            s.fill(g, x + 1.0, y + 1.0, pw - 2.0, h - 2.0, c15(fill));
            // rounded: the edge's corner pixels take the box's colour
            for (cx, cy) in [(x, y), (x + pw - 1.0, y), (x, y + h - 1.0), (x + pw - 1.0, y + h - 1.0)] {
                s.fill(g, cx, cy, 1.0, 1.0, c15(0x4e79));
            }
            let label = tr(t.key());
            let lit = p.sel == Some(k);
            put_dark_shadow(g, s, pf, x + pw * 0.5, y + 1.0, label, c15(if lit { 0x7fff } else { 0x7bde }), true);
        }
        let shown = p.sel.or(if self.xs.touched { p.last_pick } else { None });
        let (text, desc) = crate::extras_ui::foot_text(p.page, shown, &p.pref.ex, &p.pref.ui);
        self.draw_foot(g, s, b, crate::extras_ui::FOOT_LINES, text, desc);
    }

    /// The What's new box's Try it pills (welcome.rs): at the right end of each highlight that is an option, as the Extras
    /// box's tag pills, in its green; On once the option is in use.
    fn draw_try_pills(&self, g: &mut Gfx, s: &Ui, p: &Popup, b: &ChoiceBox) {
        use crate::info_ui::c15;
        let r = b.rect();
        let tops = b.line_tops();
        let tk = b.k();
        let pw = (crate::welcome::pill_w() * tk).round();
        let pf = crate::extras_ui::PILL_FONT.scaled(tk);
        for ((k, x), &line) in self.whatsnew_shown().iter().enumerate().zip(&b.options()) {
            let Some(t) = x.action.filter(|&t| self.can_try(t)) else { continue };
            let on = self.tried(t);
            let label = tr(if on { "STR_WHATSNEW_ON" } else { "STR_WHATSNEW_TRY" });
            // the Extras box's green tag (its Light pill), the tabs' paler blue once on
            let (fill, edge) = if on { (0x3dd4, 0x2d50) } else { (0x11e6, 0x0d03) };
            let (x, y, h) = (r.x + r.w - 24.0 - pw, tops[line] + (5.0 * tk).round(), (18.0 * tk).round());
            let lit = p.sel == Some(k);
            if self.on_free_set() {
                let c = c15(fill);
                crate::glass_ui::capsule(g, s, x, y, pw, h + 2.0, [c[0] * 1.1, c[1] * 1.1, c[2] * 1.1, 1.0]);
            } else {
                s.fill(g, x, y, pw, h, c15(edge));
                s.fill(g, x + 1.0, y + 1.0, pw - 2.0, h - 2.0, c15(fill));
                for (cx, cy) in [(x, y), (x + pw - 1.0, y), (x, y + h - 1.0), (x + pw - 1.0, y + h - 1.0)] {
                    s.fill(g, cx, cy, 1.0, 1.0, c15(0x4e79));
                }
            }
            put_dark_shadow(g, s, pf, x + pw * 0.5, y + 1.0, label, c15(if lit { 0x7fff } else { 0x7bde }), true);
        }
    }

    /// The description at the foot of the Extras box and of Preferences and its boxes (the port's own): a rule over it, then
    /// the text in up to two lines, left of the OK tick, in dark ink (a hint in dark teal) on the lavender.
    fn draw_foot(&self, g: &mut Gfx, s: &Ui, b: &ChoiceBox, foot_lines: usize, text: &str, desc: bool) {
        use crate::info_ui::c15;
        let r = b.rect();
        let tops = b.line_tops();
        let tk = b.k();
        let n = b.lines.len();
        let y0 = tops[n - foot_lines] + 10.0;
        s.fill(g, r.x + 16.0, y0, r.w - 32.0, 1.0, c15(0x3dd4));
        let w = r.w - 20.0 - 60.0;
        // with bigger text (textsize.rs) the foot steps down from its size to the original's until two lines hold it
        let fonts: Vec<crate::ui::Fnt> = FOOT_FONTS.iter().flat_map(|&f| [f.scaled(tk), f.scaled((tk + 1.0) / 2.0), f]).collect();
        let (f, lines) = fonts
            .iter()
            .map(|&f| (f, crate::extras::wrap(f, text, w)))
            .find(|(_, l)| l.len() <= 2)
            .unwrap_or_else(|| (FOOT_FONTS[1], crate::extras::wrap(FOOT_FONTS[1], text, w)));
        for (i, l) in lines.iter().take(2).enumerate() {
            let y = y0 + 5.0 + i as f32 * f.line();
            // dark ink on the lavender (the port's own: pale yellow and teal read poorly there)
            let ink = match (self.on_free_set(), desc) {
                (true, _) => crate::glass_ui::INK,
                (false, true) => c15(OWN_INK),
                (false, false) => c15(OWN_TEAL),
            };
            s.put(g, f, r.x + 20.0, y, l, ink);
        }
    }

    /// Preferences and its boxes' foot (prefs_ui.rs): what the line under the pointer (on a touch screen the line last
    /// tapped) does, or the box's hint.
    fn draw_prefs_foot(&self, g: &mut Gfx, s: &Ui, p: &Popup, b: &ChoiceBox) {
        let shown = p.sel.or(if self.xs.touched { p.last_pick } else { None });
        let armed = shown.is_some() && p.armed == shown;
        let (text, desc) = crate::prefs_ui::foot_text(p.kind, &p.ctx, shown, armed);
        self.draw_foot(g, s, b, crate::prefs_ui::FOOT_LINES, &text, desc);
    }

    /// The Enhanced graphics and Extras boxes' tabs (the port's own): the page shown in dark ink on the box's paler blue,
    /// the others white over a dark shadow on a darker blue (pale yellow under the pointer); a tab under the pointer is
    /// outlined.
    fn draw_page_tabs(&self, g: &mut Gfx, s: &Ui, p: &Popup) {
        use crate::info_ui::c15;
        let k = p.choice_box().k();
        let f = CHOICE_FONT.scaled(k);
        // the label's place in the tab, the exe's 5 (6 when not lit) at normal size
        let (y5, y6) = ((5.0 * k).round(), (6.0 * k).round());
        if self.on_free_set() {
            for (page, r) in p.tabs() {
                let on = page == p.page;
                let hover = p.tab_hover == Some(page);
                let tint = if on { [1.0, 0.86, 0.38, 1.0] } else { [0.28, 0.36, 0.78, 1.0] };
                crate::glass_ui::capsule(g, s, r.x, r.y + 1.0, r.w, r.h - 2.0, tint);
                if hover && !on {
                    crate::glass_ui::ring(g, s, r.x, r.y + 1.0, r.w, r.h - 2.0, [1.0, 1.0, 1.0, 0.9]);
                }
                // a page name too wide for its tab (a translation): smaller or cut (ui.rs fit_line; issue #164)
                let (f, label, fy) = crate::ui::fit_line(f, p.page_name(page), r.w);
                let x = r.x + (r.w - f.width(&label)) * 0.5;
                if on {
                    s.put(g, f, x, r.y + y5 + fy, &label, crate::glass_ui::INK);
                } else {
                    put_dark_shadow(g, s, f, x, r.y + y5 + fy, &label, c15(0x7fff), false);
                }
            }
            return;
        }
        for (page, r) in p.tabs() {
            let on = page == p.page;
            let hover = p.tab_hover == Some(page);
            let edge = if on || hover { c15(0x7fff) } else { c15(0x2d6b) };
            s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, edge);
            s.fill(g, r.x, r.y, r.w, r.h, if on { c15(0x5ade) } else { c15(TAB_OFF) });
            let (f, label, fy) = crate::ui::fit_line(f, p.page_name(page), r.w);
            let x = r.x + (r.w - f.width(&label)) * 0.5;
            if on {
                s.put(g, f, x, r.y + y5 + fy, &label, c15(OWN_INK));
            } else {
                put_dark_shadow(g, s, f, x, r.y + y6 + fy, &label, c15(if hover { 0x7ff0 } else { 0x7fff }), false);
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
                if p.sel.is_none()
                    && !crate::prefs_ui::is_prefs_box(p.kind)
                    && !matches!(p.kind, PopupKind::Enhanced | PopupKind::Extras | PopupKind::Search | PopupKind::Report)
                {
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
            p.last_pick = hit;
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
        let touched = self.xs.touched;
        let p = self.popup.as_mut()?;
        let k = pick.or(p.sel).unwrap_or(0);
        if crate::prefs_ui::is_prefs_box(p.kind) {
            use crate::prefs_ui::Line;
            let line = p.items.get(k).copied().filter(|_| pick.is_some() || p.sel.is_some());
            let Some(line) = line else { return Some(BOX_OK) };
            if crate::prefs_ui::is_tick(line) {
                if pick.is_some() {
                    p.checks ^= 1 << k;
                    return Some(usize::MAX); // toggled, stay open
                }
                // Enter on a tick: OK
                return Some(BOX_OK);
            }
            if matches!(line, Line::TextSize | Line::Language) {
                // the next text size or language, at once: the box is rebuilt at it with the ticks and the lit line kept
                let (kind, checks, sel, keyed, last) = (p.kind, p.checks, p.sel, p.keyed, p.last_pick);
                if line == Line::TextSize {
                    crate::textsize::cycle();
                } else {
                    crate::language::cycle();
                }
                self.open_popup(kind);
                if let Some(p) = self.popup.as_mut() {
                    (p.checks, p.sel, p.keyed, p.last_pick) = (checks, sel, keyed, last);
                }
                return Some(usize::MAX);
            }
            // on a touch screen a line that opens something shows what it does on the first tap and opens on the second
            if pick.is_some() && touched && crate::prefs_ui::opens(line) && p.armed != Some(k) {
                p.armed = Some(k);
                return Some(usize::MAX);
            }
            // the ticks so far are kept, as with OK, then the line's action runs (main.rs)
            return Some(k);
        }
        if p.kind == PopupKind::Extras {
            if let Some(l) = pick.and_then(|k| crate::extras_ui::box_lines(p.page).get(k).copied()) {
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
        // Report a problem (bugreport.rs): the screenshot tick toggles, Cancel cancels, OK (its line, the tick, Enter) goes
        if p.kind == PopupKind::Report {
            return match pick.or(p.sel) {
                Some(crate::bugreport::BOX_SHOT) if pick.is_some() => {
                    p.checks ^= 1 << crate::bugreport::BOX_SHOT;
                    Some(usize::MAX)
                }
                Some(crate::bugreport::BOX_CANCEL) => None,
                _ => Some(crate::bugreport::BOX_GO),
            };
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
        if p.kind == PopupKind::Cheats && k != crate::cheats_ui::DONE {
            // a cheat is used and the box stays open, rebuilt (the switches' states, what is greyed), the lit line kept
            let (sel, keyed) = (p.sel, p.keyed);
            self.use_cheat(k);
            self.open_popup(PopupKind::Cheats);
            if let Some(p) = self.popup.as_mut() {
                (p.sel, p.keyed) = (sel, keyed);
            }
            return Some(usize::MAX);
        }
        Some(k)
    }
}

/// The banner of the Enhanced graphics and Extras boxes (the port's own): a band under the tabs in the shown tab's paler
/// blue and white edge, as if the page hung from it, with the page's meaning centred in the shown tab's dark ink.
fn draw_banner(g: &mut Gfx, s: &Ui, b: &ChoiceBox, text: &str, free: bool, head: usize) {
    use crate::info_ui::c15;
    let r = b.rect();
    let y = b.line_tops()[1 + head + crate::enhanced::TAB_LINES] + 1.0;
    let (x0, x1) = (r.x + 18.0, r.x + r.w - 18.0);
    // with bigger text the band grows with its words (textsize.rs)
    let k = b.k();
    let (f, h, ty) = (crate::textsize::fit(BANNER_FONT, k, text, x1 - x0 - 8.0), (26.0 * k).round(), (3.0 * k).round());
    if free {
        // a ribbon across the box
        crate::glass_ui::capsule(g, s, x0, y - 1.0, x1 - x0, h + 2.0, [0.12, 0.46, 0.3, 1.0]);
        put_dark_shadow(g, s, f, (x0 + x1) * 0.5, y + ty, text, c15(0x7fff), true);
        return;
    }
    s.fill(g, x0 - 1.0, y - 1.0, x1 - x0 + 2.0, h + 2.0, c15(0x7fff));
    s.fill(g, x0, y, x1 - x0, h, c15(0x5ade));
    s.put_centered(g, f, (x0 + x1) * 0.5, y + ty, text, c15(OWN_INK));
}

/// Preferences' divider (prefs_ui::DIVIDER; the port's own): its words centred on the box in dark ink, with a rule either
/// side at the middle of the letters.
fn draw_divider(g: &mut Gfx, s: &Ui, f: crate::ui::Fnt, r: &Rect, y: f32, t: &str, c: [f32; 4]) {
    let cx = r.x + (r.w / 2.0).trunc();
    let half = (f.width(t) / 2.0).floor();
    s.put(g, f, cx - half, y, t, c);
    let ry = y + (f.px * 0.55).round();
    let (x0, x1) = (r.x + 20.0, r.x + r.w - 20.0);
    s.fill(g, x0, ry, (cx - half - 10.0 - x0).max(0.0), 2.0, c);
    s.fill(g, cx + half + 10.0, ry, (x1 - cx - half - 10.0).max(0.0), 2.0, c);
}

/// A light label of the port's own over a dark shadow one pixel below and right (`OWN_EDGE`; the exe's shadowed calls use
/// a dark red one pixel below, too faint to carry pale text on the port's mid-tone tabs and pills).
#[allow(clippy::too_many_arguments)]
fn put_dark_shadow(g: &mut Gfx, s: &Ui, f: crate::ui::Fnt, x: f32, y: f32, t: &str, c: [f32; 4], centred: bool) {
    let x = if centred { x - (f.width(t) / 2.0).floor() } else { x };
    let e = crate::info_ui::c15(OWN_EDGE);
    s.put(g, f, x + 1.0, y + 1.0, t, e);
    s.put(g, f, x, y + 1.0, t, e);
    s.put(g, f, x, y, t, c);
}

/// A heading of the port's own boxes, centred on x: the exe's colour inside a dark edge one pixel all round (`OWN_EDGE`),
/// so pale yellow and white read on the lavender.
fn put_edged(g: &mut Gfx, s: &Ui, f: crate::ui::Fnt, cx: f32, y: f32, t: &str, c: [f32; 4]) {
    let x = cx - (f.width(t) / 2.0).floor();
    let e = crate::info_ui::c15(OWN_EDGE);
    for (dx, dy) in [(-1.0, -1.0), (0.0, -1.0), (1.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0), (0.0, 2.0)] {
        s.put(g, f, x + dx, y + dy, t, e);
    }
    s.put(g, f, x, y, t, c);
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
                            // the person's record is the club's: an action (replay.rs)
                            let rec = r.max(0) as usize;
                            if let Some(mut p) = self.club.roster.get(rec).cloned() {
                                p.bio = n;
                                if let Ok(person) = serde_json::to_value(&p) {
                                    let fixed = (p.fixed & 7) as u8;
                                    self.act(sg_core::action::Action::Person { record: rec, person, slot: None, toggles: fixed });
                                }
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
                    self.act(sg_core::action::Action::Rename(n));
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
        let free = self.on_free_set();
        if free {
            crate::glass_ui::dialog(g, &s, r.x, r.y, r.w, r.h, 36.0);
        } else {
            s.fill(g, r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, rgba(0.55, 0.55, 0.85, 0.95));
            s.fill(g, r.x, r.y, r.w, r.h, rgba(0.92, 0.92, 1.0, 0.97));
        }
        let title = match self.xs.rename_for {
            Some(crate::extras::RenameFor::Employee(_)) => tr("STR_RENAME_EMPLOYEE"),
            Some(crate::extras::RenameFor::Bio(_)) => tr("STR_RENAME_BIO"),
            None => tr("STR_RENAME_COURSE"),
        };
        if free {
            s.text_centered(g, 400.0, 266.0, title, 17.0, crate::glass_ui::HEAD);
            crate::glass_ui::field(g, &s, r.x + 20.0, 284.0, r.w - 40.0, 28.0);
        } else {
            s.text_centered(g, 400.0, 268.0, title, 17.0, rgb(0.15, 0.1, 0.4));
            s.fill(g, r.x + 20.0, 282.0, r.w - 40.0, 28.0, rgb(1.0, 1.0, 1.0));
        }
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

#[cfg(test)]
mod tests {

    #[test]
    fn every_preferences_description_fits_the_foot() {
        use crate::prefs_ui::{desc_key, items, Ctx};
        // each box as drawn at normal text size; the foot's text left of the OK tick, in its smaller font
        let width = |kind: PopupKind, c: &Ctx| {
            let b = ChoiceBox {
                lines: crate::prefs_ui::lines(kind, c, 100),
                checks: Some(0),
                plain: crate::prefs_ui::plain_bits(&items(kind, c)),
                min_w: PREFS_MIN_W,
                own: kind != PopupKind::Prefs,
                ..ChoiceBox::new(Vec::new(), 400.0, 100.0)
            };
            b.rect().w - 20.0 - 60.0
        };
        let all = Ctx {
            files: Some(("Use my SimGolf files...", false)),
            languages: true,
            update_now: true,
            play_count: true,
            played: 0,
            make_hd: Some(crate::hd_make::Offer::Make),
            remove_hd: true,
        };
        let free = Ctx { files: Some(("Use the free graphics", true)), ..all };
        let mut long = Vec::new();
        for c in [all, free] {
            for kind in [PopupKind::Prefs, PopupKind::PrefsGraphics, PopupKind::PrefsFiles, PopupKind::PrefsMore] {
                let w = width(kind, &c);
                for l in items(kind, &c) {
                    if let Some(d) = desc_key(l, &c) {
                        let t = sg_core::lang::trf("STR_PREFS_TAP_AGAIN", &[("DESC", &sg_core::lang::tr(d))]);
                        // "Tap again" only on the lines that open something (foot_text's armed)
                        let plain = sg_core::lang::tr(d).to_string();
                        let texts = if crate::prefs_ui::opens(l) { vec![plain, t] } else { vec![plain] };
                        for text in texts {
                            let n = crate::extras::wrap(FOOT_FONTS[1], &text, w).len();
                            if n > 2 {
                                long.push(format!("{d} takes {n} lines: {text}"));
                            }
                        }
                    }
                }
            }
        }
        long.dedup();
        assert!(long.is_empty(), "{long:#?}");
    }
    use super::*;

    /// WCAG contrast ratio of two 15-bit colours as the port shows them (`info_ui::c15`).
    fn contrast(a: u32, b: u32) -> f32 {
        let lum = |v: u32| {
            let c = crate::info_ui::c15(v);
            let lin = |x: f32| if x <= 0.03928 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) };
            0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
        };
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn the_ports_own_labels_read_on_their_grounds() {
        // dark ink on the lavender fill and on the page shown and banner
        assert!(contrast(OWN_INK, 0x4e79) >= 4.5);
        assert!(contrast(OWN_INK, 0x5ade) >= 4.5);
        assert!(contrast(OWN_TEAL, 0x4e79) >= 4.5);
        // white labels on the other tabs and on the tag pills
        assert!(contrast(0x7fff, TAB_OFF) >= 4.5);
        assert!(contrast(0x7ff0, TAB_OFF) >= 4.5);
        for t in [
            crate::extras_ui::Tag::Light,
            crate::extras_ui::Tag::Noticeable,
            crate::extras_ui::Tag::Big,
            crate::extras_ui::Tag::Experimental,
        ] {
            assert!(contrast(0x7bde, t.colours().0) >= 4.5, "{t:?}");
        }
        // what was there before: the exe's teal on the old tab and the white banner on the paler blue
        assert!(contrast(0x0210, 0x3dd4) < 2.0 && contrast(0x7fff, 0x5ade) < 2.5);
    }

    #[test]
    fn bigger_text_grows_the_box_and_keeps_it_on_the_screen() {
        let lines: Vec<String> = std::iter::once("Title".to_string()).chain((0..14).map(|i| format!(" Option number {i}"))).collect();
        let b = ChoiceBox::new(lines, 400.0, 176.0);
        crate::textsize::set(100);
        assert_eq!(b.k(), 1.0);
        let normal = b.rect();
        // at normal size the exe's geometry: (lines * 3 + 3) * 8 high from the call's y, lines 18 then 24 apart
        assert_eq!((normal.y, normal.h), (176.0, (15.0 * 3.0 + 3.0) * 8.0));
        assert_eq!(b.line_tops()[..3], [188.0, 206.0, 230.0]);
        for p in [125, 150] {
            crate::textsize::set(p);
            let r = b.rect();
            assert!(b.k() > 1.0 && b.k() <= p as f32 / 100.0);
            assert!(r.h > normal.h && r.y >= 4.0 && r.y + r.h <= 596.0 && r.x >= 4.0 && r.x + r.w <= 796.0);
            // the rows follow the size, and a click on a row finds its option
            let tops = b.line_tops();
            assert!(tops[2] - tops[1] > 24.0);
            assert_eq!(b.option_at(r.x + 50.0, tops[5] + 2.0), Some(4));
            assert!(b.ok_tick().y + 39.0 <= r.y + r.h);
        }
        crate::textsize::set(100);
    }
}
