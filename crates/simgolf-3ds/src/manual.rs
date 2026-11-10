//! The instruction manual, readable in the game: from the title screen (Manual), Start's pause menu (Manual) and INFO's HELP
//! button. The top screen is for reading: the section's text, scrolled with the Circle Pad or the D-pad. The bottom screen is
//! for choosing: a big button per section (the stylus or a fingertip), and Close. L / R step through the sections, B or Start
//! closes. It opens again at the section and place it was left at.
//!
//! The text is built in (manual.txt beside Cargo.toml) so it always matches the build; a manual.txt in sdmc:/3ds/opensgolf/
//! replaces it. Its format is a small subset of Markdown: "# " starts a section, "## " a heading, "- " a list item, "1. " a
//! numbered step, "| a | b" a table row (two columns), a blank line ends a paragraph.

use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use crate::input::Event;
use crate::shim::{KeyCode, MouseButton, TouchPhase};
use crate::text::{Size, Text};

const BUILT_IN: &str = include_str!("../manual.txt");
const ON_CARD: &str = "sdmc:/3ds/opensgolf/manual.txt";

const TW: f32 = 400.0;
const BW: f32 = 320.0;
const H: f32 = 240.0;
/// The top screen's header band and footer band.
const HEAD: f32 = 24.0;
const FOOT: f32 = 16.0;
/// Left and right margins of the text.
const MX: f32 = 14.0;

const GOLD: [f32; 4] = [1.0, 0.86, 0.35, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BODY: [f32; 4] = [0.9, 0.93, 0.88, 1.0];
const DIM: [f32; 4] = [0.65, 0.72, 0.65, 1.0];
const KEY: [f32; 4] = [0.75, 0.92, 1.0, 1.0];
const ROW: [f32; 4] = [0.12, 0.3, 0.16, 1.0];
const ROW_ON: [f32; 4] = [0.95, 0.8, 0.25, 1.0];
const INK: [f32; 4] = [0.08, 0.08, 0.05, 1.0];
pub const BG_TOP: [u8; 3] = [0x10, 0x1a, 0x12];
pub const BG_BOTTOM: [u8; 3] = [0x0c, 0x16, 0x0e];

enum Block {
    Para(String),
    Head(String),
    /// A list item: its mark ("-" or "1.") and text.
    Item(String, String),
    /// A table row: left and right text, and whether it starts its table.
    Row(String, String, bool),
}

struct Section {
    title: String,
    blocks: Vec<Block>,
}

/// One laid-out line of the top screen: x, y from the top of the section, text, size, colour.
struct Line {
    x: f32,
    y: f32,
    s: String,
    size: Size,
    c: [f32; 4],
}

pub struct Manual {
    sections: Vec<Section>,
    pub open: bool,
    cur: usize,
    /// Scroll position of each section, in pixels.
    scroll: Vec<f32>,
    /// The current section laid out (built on first draw after a change), and its height.
    lines: Vec<Line>,
    height: f32,
    laid: Option<usize>,
    /// The press began on this button (it is picked on release only if it ends there).
    pressed: Option<usize>,
}

impl Manual {
    pub fn new() -> Manual {
        let src = std::fs::read_to_string(ON_CARD).ok().filter(|t| t.contains("# ")).unwrap_or_else(|| BUILT_IN.to_string());
        let sections = parse(&src);
        let n = sections.len();
        Manual { sections, open: false, cur: 0, scroll: vec![0.0; n], lines: Vec::new(), height: 0.0, laid: None, pressed: None }
    }

    pub fn show(&mut self) {
        self.open = true;
        self.pressed = None;
    }

    fn go(&mut self, i: usize) {
        if i < self.sections.len() {
            self.cur = i;
        }
    }

    /// The section buttons (and Close, last) on the bottom screen: x, y, w, h. Two columns.
    fn layout(&self) -> Vec<(f32, f32, f32, f32)> {
        let n = self.sections.len();
        let lines = n.div_ceil(2);
        let (top, close_h, gap) = (6.0, 28.0, 4.0);
        // rule 11: 6 px from every edge, 4 px between buttons (the last section row ends 4 px above Close)
        let pitch = ((H - top - close_h - 6.0) / lines.max(1) as f32).min(40.0);
        let (x0, full) = (8.0, BW - 16.0);
        let mut v: Vec<_> = (0..n)
            .map(|i| (x0 + (i % 2) as f32 * (full * 0.5 + gap * 0.5), top + (i / 2) as f32 * pitch, full * 0.5 - gap * 0.5, pitch - gap))
            .collect();
        // last row: the Left hand toggle, then Close
        let tw = full * 0.62;
        v.push((x0, H - close_h - 6.0, tw, close_h));
        v.push((x0 + tw + gap, H - close_h - 6.0, full - tw - gap, close_h));
        v
    }

    fn hit(&self, x: f32, y: f32) -> Option<usize> {
        self.layout().iter().position(|&(rx, ry, rw, rh)| x >= rx && x < rx + rw && y >= ry && y < ry + rh)
    }

    fn max_scroll(&self) -> f32 {
        (self.height - (H - HEAD - FOOT - 8.0)).max(0.0)
    }

    fn scroll_by(&mut self, d: f32) {
        let m = self.max_scroll();
        let s = &mut self.scroll[self.cur];
        *s = (*s + d).clamp(0.0, m);
    }

    /// One input event while the manual is open. Returns whether something changed on screen; sets `open` false to close.
    pub fn event(&mut self, ev: &Event, mixer: &sg_core::mixer::Mixer) -> bool {
        let n = self.sections.len();
        match *ev {
            Event::KeyDown(KeyCode::Up, _) => self.scroll_by(-36.0),
            Event::KeyDown(KeyCode::Down, _) => self.scroll_by(36.0),
            // the Circle Pad and C-stick (top-screen pixels per frame)
            Event::Pan(_, dy) => {
                if dy.abs() < 0.5 {
                    return false;
                }
                self.scroll_by(dy * 0.8);
            }
            Event::KeyDown(KeyCode::PageUp | KeyCode::Left, _) => {
                self.go((self.cur + n - 1) % n);
                mixer.play("Interface/Button2.wav", 0.5, false);
            }
            Event::KeyDown(KeyCode::PageDown | KeyCode::Right, _) => {
                self.go((self.cur + 1) % n);
                mixer.play("Interface/Button2.wav", 0.5, false);
            }
            Event::MouseDown(MouseButton::Right, _, _) | Event::KeyDown(KeyCode::Escape, _) => {
                self.open = false;
                mixer.play("Interface/Button2.wav", 0.6, false);
            }
            Event::Pen(phase, x, y) => {
                let at = self.hit(x, y);
                match phase {
                    TouchPhase::Started => self.pressed = at,
                    TouchPhase::Moved => {}
                    _ => {
                        if let (Some(i), true) = (at, at == self.pressed) {
                            if i == n {
                                crate::input::set_lefty(!crate::input::lefty());
                                mixer.play("Interface/Button1.wav", 0.6, false);
                            } else if i == n + 1 {
                                self.open = false;
                                mixer.play("Interface/Button2.wav", 0.6, false);
                            } else {
                                self.go(i);
                                mixer.play("Interface/Button1.wav", 0.6, false);
                            }
                        }
                        self.pressed = None;
                    }
                }
            }
            _ => return false,
        }
        true
    }

    /// Lays the current section out for the top screen's width.
    fn lay_out(&mut self, text: &Text) {
        if self.laid == Some(self.cur) {
            return;
        }
        self.laid = Some(self.cur);
        self.lines.clear();
        let w = TW - MX * 2.0 - 6.0; // room for the scroll bar
        let lh = text.line_height(Size::Normal);
        let mut y = 0.0;
        let mut out: Vec<Line> = Vec::new();
        let mut push = |x: f32, y: f32, s: String, size: Size, c: [f32; 4]| out.push(Line { x, y, s, size, c });
        // the table's left column: as wide as its widest entry in the section, within limits
        let col = self
            .sections
            .get(self.cur)
            .map(|s| s.blocks.iter().filter_map(|b| if let Block::Row(l, _, _) = b { Some(text.width(l, Size::Normal)) } else { None }).fold(0.0f32, f32::max))
            .unwrap_or(0.0)
            .clamp(60.0, 130.0)
            + 10.0;
        let Some(sec) = self.sections.get(self.cur) else { return };
        let mut prev_row = false;
        for b in &sec.blocks {
            // space under a table before what follows it
            if prev_row && !matches!(b, Block::Row(..)) {
                y += 6.0;
            }
            match b {
                Block::Head(t) => {
                    y += 6.0;
                    for l in text.wrap(t, Size::Normal, w) {
                        push(MX, y, l, Size::Normal, GOLD);
                        y += lh;
                    }
                    y += 2.0;
                }
                Block::Para(t) => {
                    for l in text.wrap(t, Size::Normal, w) {
                        push(MX, y, l, Size::Normal, BODY);
                        y += lh;
                    }
                    y += 6.0;
                }
                Block::Item(mark, t) => {
                    let ind = if mark == "-" { 12.0 } else { 16.0 };
                    let m = if mark == "-" { "-".to_string() } else { mark.clone() };
                    push(MX + 2.0, y, m, Size::Normal, GOLD);
                    for l in text.wrap(t, Size::Normal, w - ind) {
                        push(MX + ind, y, l, Size::Normal, BODY);
                        y += lh;
                    }
                    y += 3.0;
                }
                Block::Row(l, r, first) => {
                    if *first && prev_row {
                        y += 4.0;
                    }
                    let ls = text.wrap(l, Size::Normal, col - 10.0);
                    let rs = text.wrap(r, Size::Normal, w - col);
                    let rows = ls.len().max(1) as f32 * lh;
                    let rrows = rs.len() as f32 * lh;
                    for (i, s) in ls.into_iter().enumerate() {
                        push(MX, y + i as f32 * lh, s, Size::Normal, KEY);
                    }
                    for (i, s) in rs.into_iter().enumerate() {
                        push(MX + col, y + i as f32 * lh, s, Size::Normal, BODY);
                    }
                    y += rows.max(rrows) + 3.0;
                }
            }
            prev_row = matches!(b, Block::Row(..));
        }
        drop(push);
        self.lines = out;
        self.height = y;
        let m = self.max_scroll();
        self.scroll[self.cur] = self.scroll[self.cur].clamp(0.0, m);
    }

    /// The top screen: the section's title, its text, a scroll bar and the controls.
    pub fn render_top(&mut self, g: &mut Gfx, text: &Text) {
        self.lay_out(text);
        let u = ui(TW);
        let n = self.sections.len();
        let title = self.sections.get(self.cur).map(|s| s.title.as_str()).unwrap_or("");
        let body_top = HEAD + 4.0;
        let body_bot = H - FOOT;
        let scroll = self.scroll.get(self.cur).copied().unwrap_or(0.0);
        for l in &self.lines {
            let y = body_top + l.y - scroll;
            let lh = text.line_height(l.size);
            // only whole lines inside the text area (the bands would cover part of a line)
            if y < body_top - 1.0 || y + lh > body_bot + 1.0 {
                continue;
            }
            text.draw(g, &u, l.x, y, &l.s, l.size, l.c);
        }
        // the bands
        rect(g, &u, 0.0, 0.0, TW, HEAD, [0.0, 0.05, 0.02, 1.0]);
        rect(g, &u, 0.0, HEAD, TW, 1.5, [1.0, 0.86, 0.35, 0.8]);
        text.draw(g, &u, MX, 5.0, "MANUAL", Size::Small, DIM);
        let tx = MX + text.width("MANUAL", Size::Small) + 10.0;
        text.draw(g, &u, tx, 4.0, title, Size::Large, WHITE);
        let page = format!("{} / {}", self.cur + 1, n);
        text.draw(g, &u, TW - MX - text.width(&page, Size::Small), 6.0, &page, Size::Small, DIM);
        rect(g, &u, 0.0, body_bot, TW, FOOT, [0.0, 0.05, 0.02, 1.0]);
        let hint = crate::input::lh("Circle Pad: scroll    L / R: section    B: close");
        text.draw(g, &u, (TW - text.width(hint, Size::Small)) * 0.5, body_bot + 3.0, hint, Size::Small, DIM);
        // the scroll bar
        let m = self.max_scroll();
        if m > 0.0 {
            let track = body_bot - body_top - 4.0;
            let view = H - HEAD - FOOT - 8.0;
            let th = (track * view / self.height).max(14.0);
            let ty = body_top + (track - th) * (scroll / m);
            rect(g, &u, TW - 8.0, body_top, 3.0, track, [1.0, 1.0, 1.0, 0.12]);
            rect(g, &u, TW - 8.0, ty, 3.0, th, [1.0, 0.86, 0.35, 0.8]);
        }
    }

    /// The bottom screen: a button per section, and Close.
    pub fn render_bottom(&self, g: &mut Gfx, text: &Text) {
        let u = ui(BW);
        let lay = self.layout();
        let n = self.sections.len();
        for (i, &(x, y, w, h)) in lay.iter().enumerate() {
            let on = i == self.cur;
            let close = i == n + 1;
            let toggle = i == n;
            let on = on || (toggle && crate::input::lefty());
            let label = if close {
                "Close"
            } else if toggle {
                if crate::input::lefty() { "Left hand: on" } else { "Left hand: off" }
            } else {
                self.sections[i].title.as_str()
            };
            rect(g, &u, x, y, w, h, if on { ROW_ON } else if close { [0.3, 0.16, 0.14, 1.0] } else { ROW });
            rect(g, &u, x, y + h - 2.0, w, 2.0, [0.0, 0.0, 0.0, 0.35]);
            let c = if on { INK } else { WHITE };
            let size = if text.width(label, Size::Normal) <= w - 10.0 { Size::Normal } else { Size::Small };
            let lines = text.wrap(label, size, w - 10.0);
            let lh = text.line_height(size);
            let shown = lines.len().min(2);
            let ly = y + (h - 2.0 - lh * shown as f32) * 0.5;
            for (k, l) in lines.iter().take(2).enumerate() {
                let lw = text.width(l, size);
                text.draw(g, &u, x + (w - lw) * 0.5, ly + k as f32 * lh, l, size, c);
            }
        }
    }
}

fn parse(src: &str) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    let mut para = String::new();
    let mut in_table = false;
    fn flush(out: &mut [Section], para: &mut String) {
        if !para.trim().is_empty() {
            if let Some(s) = out.last_mut() {
                s.blocks.push(Block::Para(para.trim().to_string()));
            }
        }
        para.clear();
    }
    for raw in src.lines() {
        let line = raw.trim_end().replace("**", "").replace('`', "");
        let t = line.trim_start();
        if let Some(title) = t.strip_prefix("# ") {
            flush(&mut out, &mut para);
            out.push(Section { title: title.trim().to_string(), blocks: Vec::new() });
            in_table = false;
            continue;
        }
        if out.is_empty() {
            out.push(Section { title: "Manual".into(), blocks: Vec::new() });
        }
        if t.is_empty() {
            flush(&mut out, &mut para);
            in_table = false;
            continue;
        }
        let sec = out.len() - 1;
        if let Some(h) = t.strip_prefix("## ").or_else(|| t.strip_prefix("### ")) {
            flush(&mut out, &mut para);
            out[sec].blocks.push(Block::Head(h.trim().to_string()));
            in_table = false;
        } else if let Some(i) = t.strip_prefix("- ") {
            flush(&mut out, &mut para);
            out[sec].blocks.push(Block::Item("-".into(), i.trim().to_string()));
            in_table = false;
        } else if let Some((num, rest)) = t.split_once(". ").filter(|(a, _)| !a.is_empty() && a.len() <= 2 && a.chars().all(|c| c.is_ascii_digit())) {
            flush(&mut out, &mut para);
            out[sec].blocks.push(Block::Item(format!("{num}."), rest.trim().to_string()));
            in_table = false;
        } else if let Some(row) = t.strip_prefix('|') {
            flush(&mut out, &mut para);
            let cells: Vec<&str> = row.trim_end_matches('|').split('|').map(str::trim).collect();
            // a Markdown table's separator row
            if cells.iter().all(|c| c.chars().all(|ch| ch == '-' || ch == ':')) {
                continue;
            }
            let l = cells.first().copied().unwrap_or("").to_string();
            let r = cells[1.min(cells.len())..].join("  ");
            out[sec].blocks.push(Block::Row(l, r, !in_table));
            in_table = true;
        } else {
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(t);
            in_table = false;
        }
    }
    flush(&mut out, &mut para);
    out
}

fn ui(w: f32) -> Uniforms {
    Uniforms::flat(&Mat4::ortho(0.0, w, H, 0.0, -1.0, 1.0), &Mat4::identity())
}

fn rect(g: &mut Gfx, u: &Uniforms, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
    let v = |x: f32, y: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
    g.quad(Mode::Flat, None, u, [v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)]);
}
