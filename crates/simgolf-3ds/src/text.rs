//! Text for the interface: Liberation Sans Bold (the game crate's own stand-in font) rasterized once with fontdue into a small
//! atlas at two sizes, drawn as one batch of quads. The 3 x 5 pixel font in plan.rs stays for the tool labels; this is for
//! anything a player reads in sentences: the status line, the top screen's bar, golfers' thoughts, the information panels.

use crate::gfx::{Gfx, Mode, TextureId, Uniforms, Vert};
use sg_core::assets::Rgba;

const FONT: &[u8] = include_bytes!("../../simgolf/fonts/LiberationSans-Bold.ttf");

/// The sizes: small for dense panels and labels, normal for the status line and messages, large for the title screen's
/// buttons, title for the game's name on it.
#[derive(Clone, Copy, PartialEq)]
pub enum Size {
    Small,
    Normal,
    Large,
    Title,
}

#[derive(Clone, Copy, Default)]
struct Glyph {
    /// Atlas rectangle in texels.
    u: u16,
    v: u16,
    w: u16,
    h: u16,
    /// Offset of the bitmap's top-left from the pen at the baseline, and the advance.
    ox: f32,
    oy: f32,
    adv: f32,
}

pub struct Text {
    tex: TextureId,
    atlas_w: f32,
    atlas_h: f32,
    /// Printable ASCII (32..127) per size.
    glyphs: [[Glyph; 95]; 4],
    /// Ascent per size: the baseline's distance below the top of a line.
    ascent: [f32; 4],
    line: [f32; 4],
}

const PX: [f32; 4] = [9.0, 11.0, 16.0, 30.0];

impl Text {
    pub fn new(g: &mut Gfx) -> Text {
        let font = fontdue::Font::from_bytes(FONT, fontdue::FontSettings::default()).expect("interface font");
        const W: u32 = 256;
        let mut rows: Vec<u8> = Vec::new(); // alpha, W wide, grown as needed
        let (mut x, mut y, mut row_h) = (0u32, 0u32, 0u32);
        let mut glyphs = [[Glyph::default(); 95]; 4];
        let mut ascent = [0.0; 4];
        let mut line = [0.0; 4];
        for (si, &px) in PX.iter().enumerate() {
            if let Some(m) = font.horizontal_line_metrics(px) {
                ascent[si] = m.ascent.ceil();
                line[si] = (m.ascent - m.descent + m.line_gap).ceil();
            }
            for c in 32u8..127 {
                let (m, bmp) = font.rasterize(c as char, px);
                let (gw, gh) = (m.width as u32, m.height as u32);
                if x + gw + 1 > W {
                    x = 0;
                    y += row_h + 1;
                    row_h = 0;
                }
                let need = ((y + gh + 1) * W) as usize;
                if rows.len() < need {
                    rows.resize(need, 0);
                }
                for j in 0..gh {
                    for i in 0..gw {
                        rows[((y + j) * W + x + i) as usize] = bmp[(j * gw + i) as usize];
                    }
                }
                glyphs[si][(c - 32) as usize] = Glyph {
                    u: x as u16,
                    v: y as u16,
                    w: gw as u16,
                    h: gh as u16,
                    ox: m.xmin as f32,
                    oy: -(m.ymin as f32 + gh as f32),
                    adv: m.advance_width,
                };
                x += gw + 1;
                row_h = row_h.max(gh);
            }
        }
        let h = (y + row_h + 1).next_power_of_two().max(8);
        rows.resize((W * h) as usize, 0);
        let mut px = vec![255u8; (W * h * 4) as usize];
        for (i, a) in rows.iter().enumerate() {
            px[i * 4 + 3] = *a;
        }
        // its own texture (not the sprite atlas): crisp, the size it is drawn at
        let tex = g.texture_levels(&Rgba { w: W, h, px }, 0);
        Text { tex, atlas_w: W as f32, atlas_h: h as f32, glyphs, ascent, line }
    }

    fn si(s: Size) -> usize {
        match s {
            Size::Small => 0,
            Size::Normal => 1,
            Size::Large => 2,
            Size::Title => 3,
        }
    }

    /// Height of a line of text.
    pub fn line_height(&self, s: Size) -> f32 {
        self.line[Self::si(s)]
    }

    /// Width of `text` (characters outside printable ASCII count as '?').
    pub fn width(&self, text: &str, s: Size) -> f32 {
        let gs = &self.glyphs[Self::si(s)];
        text.chars().map(|c| gs[Self::index(c)].adv).sum::<f32>().ceil()
    }

    fn index(c: char) -> usize {
        let c = match c {
            '\u{2019}' | '\u{2018}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            '\u{a7}' => '$',
            c => c,
        };
        let b = c as u32;
        if (32..127).contains(&b) {
            (b - 32) as usize
        } else {
            ('?' as u32 - 32) as usize
        }
    }

    /// Draws `text` with its top-left at (x, y); returns its width.
    pub fn draw(&self, g: &mut Gfx, u: &Uniforms, x: f32, y: f32, text: &str, s: Size, c: [f32; 4]) -> f32 {
        let si = Self::si(s);
        let gs = &self.glyphs[si];
        let base = y.round() + self.ascent[si];
        let mut pen = x.round();
        let mut verts: Vec<Vert> = Vec::with_capacity(text.len() * 6);
        for ch in text.chars() {
            let gl = gs[Self::index(ch)];
            if gl.w > 0 && gl.h > 0 {
                let (x0, y0) = ((pen + gl.ox).round(), (base + gl.oy).round());
                let (x1, y1) = (x0 + gl.w as f32, y0 + gl.h as f32);
                let (u0, v0) = (gl.u as f32 / self.atlas_w, gl.v as f32 / self.atlas_h);
                let (u1, v1) = ((gl.u + gl.w) as f32 / self.atlas_w, (gl.v + gl.h) as f32 / self.atlas_h);
                let p = |x: f32, y: f32, u: f32, v: f32| Vert::new(x, y, 0.0, u, v).col(c);
                verts.extend_from_slice(&[p(x0, y0, u0, v0), p(x1, y0, u1, v0), p(x1, y1, u1, v1), p(x0, y0, u0, v0), p(x1, y1, u1, v1), p(x0, y1, u0, v1)]);
            }
            pen += gl.adv;
        }
        if !verts.is_empty() {
            g.tris(Mode::Flat, Some(self.tex), u, &verts);
        }
        (pen - x).ceil()
    }

    /// `draw` with a one-pixel dark shadow, for text over the 3D view.
    pub fn draw_shadowed(&self, g: &mut Gfx, u: &Uniforms, x: f32, y: f32, text: &str, s: Size, c: [f32; 4]) -> f32 {
        self.draw(g, u, x + 1.0, y + 1.0, text, s, [0.0, 0.0, 0.0, c[3] * 0.8]);
        self.draw(g, u, x, y, text, s, c)
    }

    /// `text` cut to fit in `w` (ending in "..." when cut).
    pub fn fit(&self, text: &str, s: Size, w: f32) -> String {
        if self.width(text, s) <= w {
            return text.to_string();
        }
        let mut t: Vec<char> = text.chars().collect();
        while !t.is_empty() {
            t.pop();
            let cut: String = t.iter().collect::<String>().trim_end().to_string() + "...";
            if self.width(&cut, s) <= w {
                return cut;
            }
        }
        String::new()
    }

    /// Splits `text` into lines no wider than `w`.
    pub fn wrap(&self, text: &str, s: Size, w: f32) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        for word in text.split_whitespace() {
            let trial = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
            if self.width(&trial, s) > w && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
                cur = word.to_string();
            } else {
                cur = trial;
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    }
}

/// "$12,345" (negative with a leading minus).
pub fn money(dollars: i64) -> String {
    let digits = dollars.unsigned_abs().to_string();
    let mut s = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            s.push(',');
        }
        s.push(ch);
    }
    format!("{}${s}", if dollars < 0 { "-" } else { "" })
}
