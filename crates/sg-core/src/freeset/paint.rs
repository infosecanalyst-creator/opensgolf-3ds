//! A small vector painter for the free set: shapes given by signed distance functions (negative inside), filled with
//! antialiased edges and a colour that may vary over the shape (gradients, shading), and text in the free set's display font.
use crate::assets::Rgba;

/// A colour: r, g, b in 0..255 and alpha in 0..1.
pub type Col = [f32; 4];

pub const fn rgb(r: u8, g: u8, b: u8) -> Col {
    [r as f32, g as f32, b as f32, 1.0]
}

/// `c` lightened (k > 0) or darkened (k < 0), k in -1..1.
pub fn shade(c: Col, k: f32) -> Col {
    let f = |v: f32| if k >= 0.0 { v + (255.0 - v) * k } else { v * (1.0 + k) };
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

pub fn mix(a: Col, b: Col, t: f32) -> Col {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t]
}

pub fn alpha(c: Col, a: f32) -> Col {
    [c[0], c[1], c[2], c[3] * a]
}

/// A small deterministic hash noise in 0..1.
pub fn noise(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xFFFF) as f32 / 65535.0
}

/// Smooth value noise in 0..1 with cells of `scale` pixels, wrapping every `wrap` cells (0: no wrap).
pub fn smooth_noise(x: f32, y: f32, scale: f32, seed: u32, wrap: i32) -> f32 {
    let (fx, fy) = (x / scale, y / scale);
    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let w = |v: i32| if wrap > 0 { v.rem_euclid(wrap) } else { v };
    let n = |a: i32, b: i32| noise(w(a), w(b), seed);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(tx), s(ty));
    let top = n(ix, iy) + (n(ix + 1, iy) - n(ix, iy)) * sx;
    let bot = n(ix, iy + 1) + (n(ix + 1, iy + 1) - n(ix, iy + 1)) * sx;
    top + (bot - top) * sy
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    /// Straight (not premultiplied) RGBA, 0..255 and alpha 0..1.
    pub px: Vec<Col>,
}

impl Canvas {
    pub fn new(w: usize, h: usize, bg: Col) -> Canvas {
        Canvas { w, h, px: vec![bg; w * h] }
    }

    /// Draws colour `c` over pixel (x, y) with coverage `cov`.
    pub fn blend(&mut self, x: i32, y: i32, c: Col, cov: f32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let a = (c[3] * cov).clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let d = &mut self.px[y as usize * self.w + x as usize];
        let out_a = a + d[3] * (1.0 - a);
        if out_a <= 0.0 {
            return;
        }
        for k in 0..3 {
            d[k] = (c[k] * a + d[k] * d[3] * (1.0 - a)) / out_a;
        }
        d[3] = out_a;
    }

    /// Sets a pixel, replacing what was there.
    pub fn set(&mut self, x: i32, y: i32, c: Col) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = c;
        }
    }

    #[cfg(test)]
    pub fn get(&self, x: i32, y: i32) -> Col {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize]
        } else {
            [0.0; 4]
        }
    }

    /// Fills the shape `sd` (signed distance in pixels, negative inside) within the box (x0, y0)..(x1, y1) with the colour
    /// `col` gives at each pixel centre.
    pub fn fill(&mut self, b: (f32, f32, f32, f32), sd: impl Fn(f32, f32) -> f32, col: impl Fn(f32, f32) -> Col) {
        let (x0, y0, x1, y1) = b;
        for y in (y0.floor() as i32 - 1)..=(y1.ceil() as i32 + 1) {
            if y < 0 || y as usize >= self.h {
                continue;
            }
            for x in (x0.floor() as i32 - 1)..=(x1.ceil() as i32 + 1) {
                if x < 0 || x as usize >= self.w {
                    continue;
                }
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let d = sd(px, py);
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    let c = col(px, py);
                    self.blend(x, y, c, cov);
                }
            }
        }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Col) {
        self.fill((x, y, x + w, y + h), |px, py| sd_rect(px, py, x, y, w, h, 0.0), |_, _| c);
    }

    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Col) {
        self.fill((x, y, x + w, y + h), |px, py| sd_rect(px, py, x, y, w, h, r), |_, _| c);
    }

    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Col) {
        self.fill((cx - rx, cy - ry, cx + rx, cy + ry), |px, py| sd_ellipse(px, py, cx, cy, rx, ry), |_, _| c);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Col) {
        self.ellipse(cx, cy, r, r, c);
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, c: Col) {
        let r = width / 2.0;
        self.fill(
            (x0.min(x1) - r, y0.min(y1) - r, x0.max(x1) + r, y0.max(y1) + r),
            |px, py| sd_segment(px, py, x0, y0, x1, y1) - r,
            |_, _| c,
        );
    }

    pub fn poly(&mut self, pts: &[(f32, f32)], c: Col) {
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        self.fill((x0, y0, x1, y1), |px, py| sd_poly(px, py, pts), |_, _| c);
    }

    /// Text in the free font at `size` pixels (the em), its top at `y`; `align` 0 left of x, 1 centred on x, 2 right of x.
    pub fn text(&mut self, s: &str, x: f32, y: f32, size: f32, c: Col, align: u8) {
        let Some(f) = font() else { return };
        let width: f32 = s.chars().map(|ch| f.metrics(ch, size).advance_width).sum();
        let lm = f.horizontal_line_metrics(size).map(|m| m.ascent).unwrap_or(size * 0.9);
        let mut pen = match align {
            1 => x - width / 2.0,
            2 => x - width,
            _ => x,
        };
        let base = y + lm;
        for ch in s.chars() {
            let (m, bm) = f.rasterize(ch, size);
            let gx = pen.round() as i32 + m.xmin;
            let gy = base.round() as i32 - m.height as i32 - m.ymin;
            for j in 0..m.height {
                for i in 0..m.width {
                    let a = bm[j * m.width + i] as f32 / 255.0;
                    if a > 0.0 {
                        self.blend(gx + i as i32, gy + j as i32, c, a);
                    }
                }
            }
            pen += m.advance_width;
        }
    }

    /// The width of `s` in the free font at `size`.
    pub fn text_width(s: &str, size: f32) -> f32 {
        font().map(|f| s.chars().map(|ch| f.metrics(ch, size).advance_width).sum()).unwrap_or(0.0)
    }

    /// The picture: pixels with alpha below `cut` become `key` (a colour key) and fully transparent, the rest opaque.
    pub fn to_rgba_keyed(&self, key: [u8; 3], cut: f32) -> Rgba {
        let mut out = Rgba::new(self.w as u32, self.h as u32);
        for (i, c) in self.px.iter().enumerate() {
            let p = &mut out.px[i * 4..i * 4 + 4];
            if c[3] < cut {
                p.copy_from_slice(&[key[0], key[1], key[2], 0]);
            } else {
                p.copy_from_slice(&[c[0].round() as u8, c[1].round() as u8, c[2].round() as u8, 255]);
            }
        }
        out
    }

    /// The picture with its alpha.
    pub fn to_rgba(&self) -> Rgba {
        let mut out = Rgba::new(self.w as u32, self.h as u32);
        for (i, c) in self.px.iter().enumerate() {
            out.px[i * 4..i * 4 + 4].copy_from_slice(&[
                c[0].round().clamp(0.0, 255.0) as u8,
                c[1].round().clamp(0.0, 255.0) as u8,
                c[2].round().clamp(0.0, 255.0) as u8,
                (c[3] * 255.0).round().clamp(0.0, 255.0) as u8,
            ]);
        }
        out
    }

    /// The alpha as a black and white picture (the interface's _A sheets: white opaque, black clear).
    pub fn alpha_sheet(&self) -> Rgba {
        let mut out = Rgba::new(self.w as u32, self.h as u32);
        for (i, c) in self.px.iter().enumerate() {
            let v = (c[3] * 255.0).round().clamp(0.0, 255.0) as u8;
            out.px[i * 4..i * 4 + 4].copy_from_slice(&[v, v, v, 255]);
        }
        out
    }
}

/// The free set's display face (Fredoka SemiBold, SIL Open Font License 1.1): the headings' font (KLEPTO__.TTF) and the
/// lettering of the drawn pictures.
pub static DISPLAY_TTF: &[u8] = include_bytes!("../../../simgolf/fonts/Fredoka-SemiBold.ttf");
/// The free set's text face (Nunito ExtraBold, SIL Open Font License 1.1): the body text's font (manu3_.TTF).
pub static TEXT_TTF: &[u8] = include_bytes!("../../../simgolf/fonts/Nunito-ExtraBold.ttf");

fn font() -> Option<&'static fontdue::Font> {
    static F: std::sync::OnceLock<Option<fontdue::Font>> = std::sync::OnceLock::new();
    F.get_or_init(|| fontdue::Font::from_bytes(DISPLAY_TTF, fontdue::FontSettings::default()).ok()).as_ref()
}

// ---- signed distances ------------------------------------------------------------------------------------------------------

/// A rectangle with corner radius r.
pub fn sd_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let qx = (px - cx).abs() - (w / 2.0 - r);
    let qy = (py - cy).abs() - (h / 2.0 - r);
    let out = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    out + qx.max(qy).min(0.0) - r
}

/// An ellipse (approximate distance, exact for circles).
pub fn sd_ellipse(px: f32, py: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> f32 {
    let (dx, dy) = ((px - cx) / rx.max(0.01), (py - cy) / ry.max(0.01));
    let k = (dx * dx + dy * dy).sqrt();
    // scale the normalised distance back by the gradient length
    let g = ((dx / rx.max(0.01)).powi(2) + (dy / ry.max(0.01)).powi(2)).sqrt().max(1e-4);
    (k - 1.0) * k / g
}

pub fn sd_segment(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let (bx, by) = (x1 - x0, y1 - y0);
    let (ax, ay) = (px - x0, py - y0);
    let t = ((ax * bx + ay * by) / (bx * bx + by * by).max(1e-6)).clamp(0.0, 1.0);
    ((ax - bx * t).powi(2) + (ay - by * t).powi(2)).sqrt()
}

/// A polygon (any simple polygon), by its corners.
pub fn sd_poly(px: f32, py: f32, pts: &[(f32, f32)]) -> f32 {
    let n = pts.len();
    let mut d = f32::MAX;
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        d = d.min(sd_segment(px, py, a.0, a.1, b.0, b.1));
        if (a.1 > py) != (b.1 > py) && px < (b.0 - a.0) * (py - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
    }
    if inside {
        -d
    } else {
        d
    }
}
