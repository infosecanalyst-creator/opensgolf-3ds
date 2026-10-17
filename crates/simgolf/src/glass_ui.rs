//! The free set's look for the interface the engine draws itself (popup menus, the port's own boxes, the title screen's
//! extra buttons): candy-glossy rounded panels, capsules, switches and tabs, built once from the free set's vector painter
//! (sg_core::freeset::paint) and drawn as stretchable slices. Used only while the free base set is loaded
//! (`App::on_free_set`); with the original's files the classic pieces are drawn exactly as before.
use crate::gfx::Gfx;
use crate::ui::{Image, Screen as Ui};
use sg_core::freeset::paint::{mix, rgb, sd_rect, shade, Canvas, Col};
use std::cell::RefCell;

/// Texels per interface pixel in the slices (crisp up to a 3x window).
const K: f32 = 3.0;
/// The panel slice: the shadow margin, the corner radius and the size of a corner piece, in interface pixels.
const PANEL_M: f32 = 12.0;
const PANEL_R: f32 = 16.0;
const PANEL_C: f32 = PANEL_M + PANEL_R + 4.0;
const PANEL_TEX: f32 = 2.0 * PANEL_C + 8.0;
/// The capsule: its height in the texture (ends are half of it).
const CAP_H: f32 = 32.0;
const CAP_W: f32 = 96.0;

/// The panel's body colour (a candy periwinkle that keeps the engine's white, yellow and navy text readable).
pub const BODY: Col = rgb(122, 140, 214);
pub const INK: [f32; 4] = [0.07, 0.1, 0.27, 1.0];
pub const DIM_INK: [f32; 4] = [0.33, 0.37, 0.6, 1.0];

#[derive(Default, Clone, Copy)]
struct Art {
    panel: Image,
    capsule: Image,
    ring: Image,
    knob: Image,
    header: Image,
}

thread_local! {
    static ART: RefCell<Option<Art>> = const { RefCell::new(None) };
    static ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Notes, once a frame, whether the free set's look is in use (the free base set is loaded).
pub fn set_active(on: bool) {
    ACTIVE.with(|a| a.set(on));
}

/// Whether the free set's look is in use (for drawing helpers that have no App at hand).
pub fn active() -> bool {
    ACTIVE.with(|a| a.get())
}

fn image(g: &mut Gfx, c: &Canvas) -> Image {
    let tex = g.texture(&c.to_rgba(), false);
    Image { tex: Some(tex), w: c.w as f32, h: c.h as f32 }
}

/// A soft shadow of a rounded rectangle: alpha falling off over `blur` pixels outside it.
fn soft_shadow(c: &mut Canvas, (x, y, w, h): (f32, f32, f32, f32), r: f32, blur: f32, a: f32) {
    c.fill(
        (x - blur, y - blur, x + w + blur, y + h + blur),
        |px, py| sd_rect(px, py, x, y, w, h, r) - blur,
        |px, py| {
            let d = sd_rect(px, py, x, y, w, h, r).max(0.0) / blur;
            let t = (1.0 - d).clamp(0.0, 1.0);
            [10.0, 14.0, 40.0, a * t * t]
        },
    );
}

fn build(g: &mut Gfx) -> Art {
    // the panel: a shadow, a deep navy rim, a candy body with a gloss band along its top and a bright inner line
    let n = (PANEL_TEX * K) as usize;
    let mut c = Canvas::new(n, n, [0.0; 4]);
    let (m, r) = (PANEL_M * K, PANEL_R * K);
    let (x, y, w, h) = (m, m, n as f32 - 2.0 * m, n as f32 - 2.0 * m);
    soft_shadow(&mut c, (x + 2.0 * K, y + 5.0 * K, w, h), r, 9.0 * K, 0.45);
    // the candy extrude under the panel
    c.fill((x, y, x + w, y + h + 5.0 * K), |px, py| sd_rect(px, py, x, y + 5.0 * K, w, h, r), |_, _| rgb(20, 26, 70));
    c.fill(
        (x, y, x + w, y + h),
        |px, py| sd_rect(px, py, x, y, w, h, r),
        |px, py| {
            let d = -sd_rect(px, py, x, y, w, h, r);
            if d < 2.2 * K {
                rgb(24, 30, 84)
            } else if d < 3.6 * K {
                let t = ((py - y) / (PANEL_C * K)).clamp(0.0, 1.0);
                mix(rgb(236, 242, 255), rgb(168, 182, 238), t)
            } else if py - y < 15.0 * K {
                // the gloss along the top
                mix(shade(BODY, 0.32), BODY, ((py - y) / (15.0 * K)).powf(0.7))
            } else {
                BODY
            }
        },
    );
    let panel = image(g, &c);
    // the capsule: white, glossy, with a faint edge (tinted to any colour when drawn)
    let (cw, ch) = ((CAP_W * K) as usize, (CAP_H * K) as usize);
    let mut c = Canvas::new(cw, ch + (3.0 * K) as usize, [0.0; 4]);
    let (w, h, r) = (cw as f32, ch as f32, ch as f32 / 2.0);
    c.fill((0.0, 2.0 * K, w, h + 2.0 * K), |px, py| sd_rect(px, py, 0.0, 2.0 * K, w, h, r), |_, _| [0.0, 0.0, 30.0, 0.35]);
    c.fill(
        (0.0, 0.0, w, h),
        |px, py| sd_rect(px, py, 0.0, 0.0, w, h, r),
        |px, py| {
            let d = -sd_rect(px, py, 0.0, 0.0, w, h, r);
            let t = py / h;
            if d < 1.4 * K {
                rgb(150, 156, 180)
            } else if t < 0.45 && d > 3.0 * K && px > r * 0.6 && px < w - r * 0.6 {
                mix(rgb(255, 255, 255), rgb(246, 248, 255), t / 0.45)
            } else {
                mix(rgb(238, 240, 250), rgb(200, 206, 226), t)
            }
        },
    );
    let capsule = image(g, &c);
    // the ring: a capsule outline (a tab under the pointer, a focus mark)
    let mut c = Canvas::new(cw, ch, [0.0; 4]);
    c.fill((0.0, 0.0, w, h), |px, py| sd_rect(px, py, 0.0, 0.0, w, h, r).abs() - 1.2 * K, |_, _| rgb(255, 255, 255));
    let ring = image(g, &c);
    // the switch's knob: a white ball with a shadow
    let kn = (24.0 * K) as usize;
    let mut c = Canvas::new(kn, kn, [0.0; 4]);
    let kr = 10.0 * K;
    let kc = kn as f32 / 2.0;
    c.fill((0.0, 0.0, kn as f32, kn as f32), |px, py| ((px - kc).hypot(py - kc - K)) - kr - K, |_, _| [0.0, 0.0, 30.0, 0.3]);
    c.fill(
        (0.0, 0.0, kn as f32, kn as f32),
        |px, py| (px - kc).hypot(py - kc) - kr,
        |px, py| {
            let d = (px - kc + kr * 0.35).hypot(py - kc + kr * 0.45) / (kr * 1.6);
            mix(rgb(255, 255, 255), rgb(206, 212, 232), d.clamp(0.0, 1.0))
        },
    );
    let knob = image(g, &c);
    // the header band: rounded at the top, a white to pale grey sheen (tinted when drawn)
    let (hw, hh) = ((CAP_W * K) as usize, (40.0 * K) as usize);
    let mut c = Canvas::new(hw, hh, [0.0; 4]);
    let (w, h, r) = (hw as f32, hh as f32, (PANEL_R - 3.0) * K);
    c.fill(
        (0.0, 0.0, w, h),
        |px, py| sd_rect(px, py, 0.0, 0.0, w, h + r, r),
        |_, py| {
            let t = py / h;
            if t < 0.5 {
                mix(rgb(255, 255, 255), rgb(236, 238, 248), t * 2.0)
            } else {
                mix(rgb(222, 226, 242), rgb(190, 196, 222), (t - 0.5) * 2.0)
            }
        },
    );
    let header = image(g, &c);
    Art { panel, capsule, ring, knob, header }
}

fn art(g: &mut Gfx) -> Art {
    ART.with(|a| *a.borrow_mut().get_or_insert_with(|| build(g)))
}

/// A panel over the box (x, y, w, h): its shadow reaches outside the box, its rim lies on the box's edge.
pub fn panel(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
    let a = art(g);
    nine(g, s, &a.panel, (x - PANEL_M, y - PANEL_M, w + 2.0 * PANEL_M, h + 2.0 * PANEL_M), PANEL_C, PANEL_TEX, [1.0; 4]);
}

/// A nine-slice draw of `im` (a `tex` x `tex` picture in interface pixels with corners `c`) over `dst`.
fn nine(g: &mut Gfx, s: &Ui, im: &Image, dst: (f32, f32, f32, f32), c: f32, tex: f32, tint: [f32; 4]) {
    let (x, y, w, h) = dst;
    let c = c.min(w / 2.0).min(h / 2.0);
    let xs = [(x, c, 0.0, c), (x + c, w - 2.0 * c, c, tex - 2.0 * c), (x + w - c, c, tex - c, c)];
    let ys = [(y, c, 0.0, c), (y + c, h - 2.0 * c, c, tex - 2.0 * c), (y + h - c, c, tex - c, c)];
    for &(dx, dw, sx, sw) in &xs {
        for &(dy, dh, sy, sh) in &ys {
            if dw > 0.0 && dh > 0.0 {
                s.image_scaled_tint(g, im, (dx, dy, dw, dh), (sx * K, sy * K, sw * K, sh * K), tint);
            }
        }
    }
}

/// A three-slice draw of a capsule picture over (x, y, w, h): its ends kept round, its middle stretched.
fn three(g: &mut Gfx, s: &Ui, im: &Image, (x, y, w, h): (f32, f32, f32, f32), tint: [f32; 4]) {
    let src_h = im.h;
    let end = (h / 2.0).min(w / 2.0);
    let src_end = CAP_H * K / 2.0;
    let k = h / (CAP_H * K);
    let dst_h = src_h * k;
    s.image_scaled_tint(g, im, (x, y, end, dst_h), (0.0, 0.0, src_end, src_h), tint);
    if w > 2.0 * end {
        s.image_scaled_tint(g, im, (x + end, y, w - 2.0 * end, dst_h), (src_end, 0.0, im.w - 2.0 * src_end, src_h), tint);
    }
    s.image_scaled_tint(g, im, (x + w - end, y, end, dst_h), (im.w - src_end, 0.0, src_end, src_h), tint);
}

/// A glossy capsule tinted `tint` (a button, a tag, a highlighted row).
pub fn capsule(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, tint: [f32; 4]) {
    let a = art(g);
    three(g, s, &a.capsule, (x, y, w, h), tint);
}

/// A capsule's outline.
pub fn ring(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, tint: [f32; 4]) {
    let a = art(g);
    three(g, s, &a.ring, (x, y, w, h), tint);
}

/// The header band across the top of a panel (x, y, w) down to y + h, tinted.
pub fn header(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, tint: [f32; 4]) {
    let a = art(g);
    let im = a.header;
    let r = PANEL_R - 3.0;
    s.image_scaled_tint(g, &im, (x, y, r, h), (0.0, 0.0, r * K, im.h), tint);
    s.image_scaled_tint(g, &im, (x + r, y, w - 2.0 * r, h), (r * K, 0.0, im.w - 2.0 * r * K, im.h), tint);
    s.image_scaled_tint(g, &im, (x + w - r, y, r, h), (im.w - r * K, 0.0, r * K, im.h), tint);
}

/// An on/off switch with its left at x, centred on y: a green track with the knob at the right when on.
pub fn switch(g: &mut Gfx, s: &Ui, x: f32, cy: f32, on: bool, lit: bool) {
    let (w, h) = (30.0, 16.0);
    let track = if on { [0.3, 0.78, 0.36, 1.0] } else { [0.55, 0.6, 0.78, 1.0] };
    let track = if lit { [track[0] * 1.12, track[1] * 1.08, track[2] * 1.12, 1.0] } else { track };
    capsule(g, s, x, cy - h / 2.0, w, h, track);
    let a = art(g);
    let kx = if on { x + w - 18.0 } else { x - 1.0 };
    s.image_scaled(g, &a.knob, (kx, cy - 9.5, 19.0, 19.0), (0.0, 0.0, a.knob.w, a.knob.h));
}

/// A radio mark: a round dot well, filled gold when chosen.
pub fn radio(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, on: bool) {
    capsule(g, s, cx - 10.0, cy - 10.0, 20.0, 20.0, [0.86, 0.9, 1.0, 1.0]);
    if on {
        capsule(g, s, cx - 6.0, cy - 6.0, 12.0, 12.0, [1.0, 0.72, 0.15, 1.0]);
    }
}

/// A command button: a navy capsule, gold under the pointer, grey when it cannot be used.
pub fn button(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, enabled: bool, lit: bool) {
    let tint = if !enabled {
        [0.72, 0.74, 0.82, 1.0]
    } else if lit {
        [1.0, 0.82, 0.32, 1.0]
    } else {
        [0.86, 0.92, 1.0, 1.0]
    };
    capsule(g, s, x, y, w, h, tint);
}

/// A plain button's fill: a light candy capsule (gold under the pointer) on the free set, the classic flat box otherwise.
pub fn button_fill(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, lit: bool) {
    if active() {
        button(g, s, x, y, w, h + 1.0, true, lit);
    } else {
        s.fill(g, x, y, w, h, if lit { crate::ui::rgb(0.85, 0.85, 1.0) } else { crate::ui::rgb(0.47, 0.47, 0.70) });
    }
}

/// A tab: gold when it is the open one, deep blue otherwise, ringed under the pointer.
pub fn tab(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, on: bool, hover: bool) {
    capsule(g, s, x, y, w, h, if on { [1.0, 0.84, 0.36, 1.0] } else { [0.28, 0.36, 0.78, 1.0] });
    if hover && !on {
        ring(g, s, x, y, w, h, [1.0, 1.0, 1.0, 0.9]);
    }
}

/// A small labelled button of the title screen (the port's own: the game files, updates, the course editor).
pub fn title_button(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, label: &str, lit: bool) {
    capsule(g, s, x - 2.0, y - 1.0, w + 4.0, h + 3.0, if lit { [1.0, 0.82, 0.32, 1.0] } else { [0.22, 0.36, 0.8, 1.0] });
    let f = crate::ui::F_MANUAL15;
    if lit {
        s.put_centered(g, f, x + w / 2.0, y + 4.0, label, INK);
    } else {
        s.put_shadowed(g, f, x + w / 2.0, y + 4.0, label, [1.0, 1.0, 1.0, 1.0], true);
    }
}

/// A dialog in the free set's look over (x, y, w, h): the panel with a header band `head` pixels tall.
pub fn dialog(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, head: f32) {
    panel(g, s, x, y, w, h);
    if head > 0.0 {
        header(g, s, x + 3.0, y + 3.0, w - 6.0, head, [0.24, 0.33, 0.78, 1.0]);
    }
}

/// A text field: a white capsule.
pub fn field(g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
    capsule(g, s, x - 4.0, y, w + 8.0, h, [1.0, 1.0, 1.0, 1.0]);
}

/// The heading colour on a dialog's header band, and the text colour on a button.
pub const HEAD: [f32; 4] = [1.0, 0.97, 0.75, 1.0];
