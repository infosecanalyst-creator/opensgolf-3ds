//! The People panels in the free set's look: the Golfers panel (Interface/MemberPanel), the Player panel (JoeCoolPanel) and
//! the Employee panel (EmployeePanel). The engine draws each panel's body and, over it, only the looks of a control that
//! differ from the idle one (pointed at, selected, disabled), so the body carries every control in its idle look at the
//! place the engine draws the other looks (crates/simgolf/src/member_panel.rs, player_panel.rs and panels_ui.rs
//! draw_employee_panel hold the cuts and places used here). Each look is a glossy disc, oval or pad with its own symbol:
//! normal in blue or cream, brighter with a gold glow when pointed at, gold when selected, grey when unavailable.
use super::paint::{alpha as with_alpha, mix, rgb, sd_ellipse, sd_poly, sd_rect, shade, Canvas, Col};
use super::ui::{glass_tray, glyph, Glyph, BASE, GOLD, GREEN, GREY, PAPER, RED, SLATE};

const BLUE: Col = rgb(58, 119, 182);
const CREAM: Col = rgb(232, 220, 184);
const INK: Col = rgb(22, 34, 46);
const LIGHT: Col = rgb(253, 251, 242);
const GOLD_HI: Col = rgb(255, 224, 138);
const SKY: Col = rgb(159, 208, 242);
/// The light fields the engine writes dark text on.
const FIELD: Col = rgb(214, 228, 245);

/// The look of a control.
#[derive(Clone, Copy, PartialEq)]
enum St {
    Normal,
    Lit,
    Sel,
    Off,
}

fn state_col(base: Col, st: St) -> Col {
    match st {
        St::Normal => base,
        St::Lit => shade(base, 0.15),
        St::Sel => GOLD,
        St::Off => mix(base, GREY, 0.7),
    }
}

/// A symbol's colour on a body of colour `c`: light on a dark body, ink on a light one.
fn ink_on(c: Col) -> Col {
    if c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11 < 150.0 {
        LIGHT
    } else {
        INK
    }
}

/// The symbols of the People panels.
#[derive(Clone, Copy, PartialEq)]
enum Pic {
    /// A group of people: the Golfers panel.
    Group,
    /// A golfer at the top of his swing: the Player panel.
    Golfer,
    /// A groundskeeper with a rake: the Employee panel.
    Staff,
    /// A face: Customise Golfer.
    Face,
    /// A person with a plus: Hire Employees.
    Hire,
    /// A flag on a green: Practice Round.
    Flag,
    /// Two crossed clubs: Play (a match against a pro).
    Clubs,
    /// A cup: Begin Tournament.
    Trophy,
    /// Arrows four ways: Move.
    Move,
    /// A door with an arrow out: Fire.
    Fire,
    /// A pencil: Rename.
    Pencil,
}

fn dist(px: f32, py: f32, x: f32, y: f32) -> f32 {
    ((px - x).powi(2) + (py - y).powi(2)).sqrt()
}

/// A symbol in a circle of radius r about (x, y), in `col`; `hole` is the body's colour for the cut-outs (eyes, outlines),
/// None for the symbol's shadow.
fn pic(c: &mut Canvas, p: Pic, x: f32, y: f32, r: f32, col: Col, hole: Option<Col>) {
    let w = (r * 0.16).max(1.3);
    match p {
        Pic::Group => {
            for s in [-1.0f32, 1.0] {
                c.circle(x + s * r * 0.52, y - r * 0.5, r * 0.22, col);
                c.round_rect(x + s * r * 0.52 - r * 0.3, y - r * 0.24, r * 0.6, r * 0.8, r * 0.26, col);
            }
            if let Some(h) = hole {
                c.circle(x, y - r * 0.28, r * 0.33, h);
                c.round_rect(x - r * 0.4, y - r * 0.02, r * 0.8, r * 1.0, r * 0.32, h);
            }
            c.circle(x, y - r * 0.28, r * 0.25, col);
            c.round_rect(x - r * 0.32, y + 0.04 * r, r * 0.64, r * 0.9, r * 0.28, col);
        }
        Pic::Golfer => {
            c.circle(x - r * 0.05, y - r * 0.58, r * 0.24, col);
            c.line(x - r * 0.05, y - r * 0.32, x + r * 0.05, y + r * 0.22, r * 0.34, col);
            c.line(x + r * 0.05, y + r * 0.2, x - r * 0.34, y + r * 0.88, r * 0.22, col);
            c.line(x + r * 0.05, y + r * 0.2, x + r * 0.42, y + r * 0.88, r * 0.22, col);
            c.line(x, y - r * 0.22, x + r * 0.42, y - r * 0.52, r * 0.2, col);
            c.line(x + r * 0.42, y - r * 0.52, x - r * 0.72, y - r * 0.95, (r * 0.12).max(1.2), col);
        }
        Pic::Staff => {
            // a rake at his side
            c.line(x + r * 0.62, y - r * 0.55, x + r * 0.62, y + r * 0.9, (r * 0.1).max(1.1), col);
            c.line(x + r * 0.36, y - r * 0.62, x + r * 0.88, y - r * 0.62, (r * 0.12).max(1.1), col);
            for k in 0..3 {
                let tx = x + r * (0.4 + 0.22 * k as f32);
                c.line(tx, y - r * 0.62, tx, y - r * 0.82, (r * 0.07).max(1.0), col);
            }
            // the man in a cap
            c.circle(x - r * 0.15, y - r * 0.42, r * 0.24, col);
            c.round_rect(x - r * 0.48, y - r * 0.72, r * 0.62, r * 0.18, r * 0.06, col);
            c.round_rect(x - r * 0.53, y - r * 0.12, r * 0.76, r * 0.98, r * 0.3, col);
            if let Some(h) = hole {
                c.circle(x - r * 0.15, y + r * 0.2, r * 0.09, h);
            }
        }
        Pic::Face => {
            c.circle(x, y, r * 0.82, col);
            if let Some(h) = hole {
                for s in [-1.0f32, 1.0] {
                    c.ellipse(x + s * r * 0.3, y - r * 0.2, r * 0.1, r * 0.14, h);
                }
                for k in 0..13 {
                    let u = k as f32 / 12.0 * 2.0 - 1.0;
                    c.circle(x + u * r * 0.38, y + r * 0.22 + (1.0 - u * u) * r * 0.2, r * 0.08, h);
                }
            }
        }
        Pic::Hire => {
            c.circle(x - r * 0.25, y - r * 0.42, r * 0.25, col);
            c.round_rect(x - r * 0.68, y - r * 0.1, r * 0.86, r * 0.96, r * 0.32, col);
            let (px, py) = (x + r * 0.5, y - r * 0.4);
            c.line(px - r * 0.33, py, px + r * 0.33, py, r * 0.2, col);
            c.line(px, py - r * 0.33, px, py + r * 0.33, r * 0.2, col);
        }
        Pic::Flag => {
            c.ellipse(x, y + r * 0.66, r * 0.78, r * 0.22, col);
            c.line(x - r * 0.15, y + r * 0.66, x - r * 0.15, y - r * 0.85, w * 0.9, col);
            c.poly(&[(x - r * 0.15, y - r * 0.88), (x + r * 0.7, y - r * 0.6), (x - r * 0.15, y - r * 0.3)], col);
        }
        Pic::Clubs => {
            for s in [-1.0f32, 1.0] {
                let (top, bot) = ((x - s * r * 0.55, y - r * 0.85), (x + s * r * 0.4, y + r * 0.55));
                c.line(top.0, top.1, bot.0, bot.1, w, col);
                c.round_rect(bot.0 - r * 0.2 + s * r * 0.1, bot.1 - r * 0.05, r * 0.42, r * 0.24, r * 0.1, col);
            }
            c.circle(x, y + r * 0.72, r * 0.15, col);
        }
        Pic::Trophy => {
            c.poly(
                &[
                    (x - r * 0.62, y - r * 0.85),
                    (x + r * 0.62, y - r * 0.85),
                    (x + r * 0.48, y - r * 0.15),
                    (x + r * 0.14, y + r * 0.15),
                    (x - r * 0.14, y + r * 0.15),
                    (x - r * 0.48, y - r * 0.15),
                ],
                col,
            );
            for s in [-1.0f32, 1.0] {
                c.fill(
                    (x - r, y - r, x + r, y + r * 0.2),
                    |px, py| (dist(px, py, x + s * r * 0.6, y - r * 0.5) - r * 0.27).abs() - r * 0.08,
                    |_, _| col,
                );
            }
            c.rect(x - r * 0.1, y + r * 0.1, r * 0.2, r * 0.45, col);
            c.round_rect(x - r * 0.48, y + r * 0.52, r * 0.96, r * 0.28, r * 0.08, col);
        }
        Pic::Fire => glyph(c, Glyph::Exit, x - r * 0.1, y, r, col),
        Pic::Move => {
            let a = r * 0.8;
            c.line(x - a * 0.7, y, x + a * 0.7, y, w, col);
            c.line(x, y - a * 0.7, x, y + a * 0.7, w, col);
            let h = r * 0.28;
            for (dx, dy) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                let (tx, ty) = (x + dx * a, y + dy * a);
                let (bx, by) = (x + dx * (a - h * 1.3), y + dy * (a - h * 1.3));
                c.poly(&[(tx, ty), (bx - dy * h, by + dx * h), (bx + dy * h, by - dx * h)], col);
            }
        }
        Pic::Pencil => {
            let (ax, ay, bx, by) = (x - r * 0.3, y + r * 0.3, x + r * 0.62, y - r * 0.62);
            c.line(ax, ay, bx, by, r * 0.34, col);
            c.poly(&[(x - r * 0.48, y + r * 0.12), (x - r * 0.12, y + r * 0.48), (x - r * 0.72, y + r * 0.72)], col);
            if let Some(h) = hole {
                c.line(x + r * 0.32, y - r * 0.56, x + r * 0.56, y - r * 0.32, (r * 0.07).max(1.0), h);
            }
        }
    }
}

/// A soft shadow under a shape (its signed distance), offset down and to the right.
fn drop_shadow(c: &mut Canvas, b: (f32, f32, f32, f32), sd: impl Fn(f32, f32) -> f32) {
    let (dx, dy) = (1.5, 2.5);
    c.fill(
        (b.0, b.1, b.2 + 5.0, b.3 + 5.0),
        |px, py| sd(px - dx, py - dy) - 1.5,
        |px, py| with_alpha(rgb(0, 0, 0), 0.42 * (-(sd(px - dx, py - dy) - 1.5) / 3.0).clamp(0.0, 1.0)),
    );
}

/// A glossy round button (uiart.py glossy_disc): a soft shadow, a rim, a body lit from the top left, a cap of light, the
/// symbol; pointed at it glows gold.
fn disc(c: &mut Canvas, x: f32, y: f32, r: f32, base: Col, p: Pic, st: St) {
    let col = state_col(base, st);
    let b = (x - r, y - r, x + r, y + r);
    drop_shadow(c, b, |px, py| dist(px, py, x, y) - r);
    c.circle(x, y, r, shade(col, -0.45));
    let ri = r * 0.88;
    let (lx, ly) = (x - r * 0.3, y - r * 0.35);
    c.fill(b, |px, py| dist(px, py, x, y) - ri, |px, py| mix(shade(col, 0.35), shade(col, -0.25), dist(px, py, lx, ly) / (r * 1.5)));
    c.fill(
        b,
        |px, py| sd_ellipse(px, py, x, y - r * 0.425, r * 0.62, r * 0.375).max(dist(px, py, x, y) - ri),
        |_, _| with_alpha(rgb(255, 255, 255), 0.28),
    );
    let gc = match st {
        St::Off => mix(ink_on(col), col, 0.35),
        St::Lit => ink_on(base),
        _ => ink_on(col),
    };
    let gr = r * 0.62;
    pic(c, p, x + 0.6, y + 0.9, gr, with_alpha(rgb(0, 0, 0), if gc == LIGHT { 0.45 } else { 0.25 }), None);
    pic(c, p, x, y, gr, gc, Some(shade(col, 0.05)));
    if st == St::Lit {
        c.fill(
            (x - r - 2.0, y - r - 2.0, x + r + 2.0, y + r + 2.0),
            |px, py| (dist(px, py, x, y) - r - 0.6).abs() - 1.1,
            |_, _| with_alpha(GOLD_HI, 0.85),
        );
    }
}

/// A light field the engine writes dark text on: a rim, a pale body, a shadow along its top.
fn field(c: &mut Canvas, x: f32, y: f32, w: f32, h: f32, r: f32, col: Col) {
    c.round_rect(x, y, w, h, r, shade(col, -0.55));
    let inner = |px: f32, py: f32| sd_rect(px, py, x + 1.5, y + 1.5, w - 3.0, h - 3.0, (r - 1.5).max(0.0));
    c.fill((x, y, x + w, y + h), inner, |_, py| shade(col, 0.08 - 0.12 * ((py - y) / h)));
    c.fill(
        (x + 2.0, y + 1.5, x + w - 2.0, y + 5.0),
        |px, py| inner(px, py).max(py - (y + 5.0)),
        |_, py| with_alpha(shade(col, -0.35), 0.5 * (1.0 - (py - y - 1.5) / 3.5).clamp(0.0, 1.0)),
    );
}

/// The round tabs to the three People panels on the left of each, at the centres the engine draws their looks about
/// (cuts of 33 x 33 at (270, 475), (240, 494) and (217, 522)), and the panel that is open in gold.
const TABS: [(f32, f32, Pic); 3] = [(286.5, 491.5, Pic::Group), (256.5, 510.5, Pic::Golfer), (233.5, 538.5, Pic::Staff)];
const TAB_R: f32 = 14.5;

/// The panel's body: the glass tray and the tabs, `open` the current one.
fn tray(c: &mut Canvas, open: usize) {
    glass_tray(c);
    for (k, &(x, y, p)) in TABS.iter().enumerate() {
        if k == open {
            disc(c, x, y, TAB_R, GOLD, p, St::Normal);
        } else {
            disc(c, x, y, TAB_R, BLUE, p, St::Normal);
        }
    }
}

/// A tab's lit look in the 33 x 33 cut at (sx, sy).
fn tab_lit(c: &mut Canvas, sx: f32, sy: f32, tab: usize) {
    disc(c, sx + 16.5, sy + 16.5, TAB_R, BLUE, TABS[tab].2, St::Lit);
}

/// The People panel sheets by file name.
pub(super) fn sheet(name: &str, w: usize, h: usize) -> Option<Canvas> {
    let draw: fn(&mut Canvas) = match name {
        "memberpanel.pcx" => member,
        "joecoolpanel.pcx" => player,
        "employeepanel.pcx" => employee,
        _ => return None,
    };
    let mut c = Canvas::new(w, h, [0.0; 4]);
    draw(&mut c);
    Some(c)
}

// ---- the Golfers panel -------------------------------------------------------------------------------------------------------

/// A double arrow for the list's scroll (36 x 13 at (x, y)), pointing left (-1) or right (1).
fn scroll_arrows(c: &mut Canvas, x: f32, y: f32, dir: f32, col: Col) {
    let cy = y + 6.5;
    for k in 0..2 {
        let (x0, x1) = if dir < 0.0 {
            (x + 4.0 + 14.0 * k as f32, x + 16.0 + 14.0 * k as f32)
        } else {
            (x + 32.0 - 14.0 * k as f32, x + 20.0 - 14.0 * k as f32)
        };
        let tri = [(x0, cy), (x1, cy - 5.0), (x1, cy + 5.0)];
        c.fill((x0.min(x1) - 2.0, cy - 7.0, x0.max(x1) + 2.0, cy + 7.0), |px, py| sd_poly(px, py, &tri) - 1.0, |_, _| shade(col, -0.6));
        c.poly(&tri, col);
    }
}

/// A golfer's mood face (k 1 cross .. 10 delighted), 16 x 16 at (x, y).
fn mood_face(c: &mut Canvas, x: f32, y: f32, k: i32) {
    let t = (k - 1) as f32 / 9.0;
    let col = if t < 0.5 { mix(RED, GOLD, t * 2.0) } else { mix(GOLD, GREEN, t * 2.0 - 1.0) };
    let (cx, cy) = (x + 8.0, y + 8.0);
    c.circle(cx, cy, 7.6, shade(col, -0.55));
    c.fill(
        (x, y, x + 16.0, y + 16.0),
        |px, py| dist(px, py, cx, cy) - 6.7,
        |px, py| mix(shade(col, 0.35), shade(col, -0.15), dist(px, py, cx - 2.0, cy - 2.5) / 10.0),
    );
    for s in [-1.0f32, 1.0] {
        c.ellipse(cx + s * 2.3, cy - 1.6, 0.9, 1.2, INK);
    }
    // the mouth: a frown for a cross golfer, a smile for a happy one
    let m = (t - 0.5) * 2.0 * 1.8;
    let mouth = |u: f32| (cx + u * 3.2, cy + 2.6 - m * 0.5 + m * (1.0 - u * u));
    for j in 0..8 {
        let (a, b) = (mouth(j as f32 / 8.0 * 2.0 - 1.0), mouth((j + 1) as f32 / 8.0 * 2.0 - 1.0));
        c.line(a.0, a.1, b.0, b.1, 1.1, INK);
    }
}

/// A heart of a pair's story, 7 x 7 at (x, y).
fn heart(c: &mut Canvas, x: f32, y: f32, full: bool) {
    let tri = [(x + 0.4, y + 2.9), (x + 6.6, y + 2.9), (x + 3.5, y + 6.9)];
    let sd = move |px: f32, py: f32| dist(px, py, x + 2.0, y + 2.3).min(dist(px, py, x + 5.0, y + 2.3)) - 1.9;
    let sd = move |px: f32, py: f32| sd(px, py).min(sd_poly(px, py, &tri));
    c.fill((x - 1.0, y - 1.0, x + 8.0, y + 8.0), sd, |_, _| rgb(110, 24, 30));
    let fill = if full { rgb(226, 58, 70) } else { rgb(236, 226, 228) };
    c.fill((x - 1.0, y - 1.0, x + 8.0, y + 8.0), |px, py| sd(px, py) + 0.9, |_, _| fill);
}

/// Interface/MemberPanel: the body with its list field and scroll track, a pair's card, the mood faces, the hearts, the tabs'
/// and the arrows' lit looks.
fn member(c: &mut Canvas) {
    tray(c, 0);
    // the list (the engine's cells from (310, 498), four rows of 21 or six of 14) and the scroll bar under it (the thumb is
    // drawn at y 590 from x 351 over 400)
    field(c, 304.0, 493.0, 493.0, 91.0, 10.0, FIELD);
    c.round_rect(347.0, 587.5, 408.0, 11.0, 5.5, shade(BASE, -0.6));
    c.round_rect(348.5, 589.0, 405.0, 8.0, 4.0, shade(BASE, -0.35));
    scroll_arrows(c, 309.0, 587.0, -1.0, SKY);
    scroll_arrows(c, 756.0, 587.0, 1.0, SKY);
    // the lit looks: the Player and Employee tabs, the arrows
    tab_lit(c, 0.0, 400.0, 1);
    tab_lit(c, 0.0, 450.0, 2);
    scroll_arrows(c, 0.0, 150.0, -1.0, GOLD_HI);
    scroll_arrows(c, 0.0, 250.0, 1.0, GOLD_HI);
    // a pair's card (121 x 44, drawn at the first golfer's row): two name bars, the hole badge between them on the left
    for y in [1.0, 22.0] {
        field(c, 13.0, y, 107.0, 19.0, 9.5, PAPER);
    }
    c.circle(11.0, 21.0, 10.0, shade(GOLD, -0.5));
    c.fill(
        (1.0, 11.0, 21.0, 31.0),
        |px, py| dist(px, py, 11.0, 21.0) - 8.6,
        |px, py| mix(shade(GOLD, 0.6), shade(GOLD, 0.1), dist(px, py, 8.0, 17.0) / 14.0),
    );
    // the mood faces, k at (594 - 16 (k - 1), 100)
    for k in 1..=10 {
        mood_face(c, 594.0 - 16.0 * (k - 1) as f32, 100.0, k);
    }
    heart(c, 450.0, 220.0, true);
    heart(c, 450.0, 230.0, false);
}

// ---- the Player panel --------------------------------------------------------------------------------------------------------

/// Customise Golfer: the centre of its look in the 42 x 44 cut drawn at (255, 542).
const CUSTOMISE: (f32, f32, f32) = (21.0, 22.0, 18.0);
/// Practice Round, Play and Begin Tournament: row on the sheet, the centre in the cut (drawn at (298, row)).
const ROUND_BUTTONS: [(f32, f32, Pic); 3] = [(500.0, 16.0, Pic::Flag), (535.0, 15.0, Pic::Clubs), (565.0, 16.0, Pic::Trophy)];
const ROUND_R: f32 = 14.0;
const ROUND_COL: Col = rgb(76, 134, 192);

/// A shot shape: 0 straight, 1 fade, 2 draw, 3 high backspin, 4 low punch; the ball's path about (x, y) with an arrow head.
/// The first three seen from above (the ball flying up the oval), the last two from the side over a line of ground.
fn shot_path(c: &mut Canvas, kind: usize, x: f32, y: f32, col: Col) {
    let f = |t: f32| -> (f32, f32) {
        match kind {
            0 => (x, y + 15.0 - 29.0 * t),
            1 => (x - 4.0 + 13.0 * t * t, y + 15.0 - 29.0 * t),
            2 => (x + 4.0 - 13.0 * t * t, y + 15.0 - 29.0 * t),
            3 => (x - 23.0 + 44.0 * t, y + 11.0 - 25.0 * 4.0 * t * (1.0 - t) * (1.0 - 0.25 * t)),
            _ => (x - 23.0 + 44.0 * t, y + 11.0 - 8.0 * 4.0 * t * (1.0 - t) * (1.0 - 0.25 * t)),
        }
    };
    if kind >= 3 {
        c.line(x - 27.0, y + 13.0, x + 27.0, y + 13.0, 1.4, with_alpha(col, 0.6));
    }
    let n = 18;
    let end = if kind >= 3 { 0.94 } else { 0.86 };
    let pts: Vec<(f32, f32)> = (0..=n).map(|k| f(end * k as f32 / n as f32)).collect();
    for s in pts.windows(2) {
        c.line(s[0].0, s[0].1, s[1].0, s[1].1, 2.4, col);
    }
    let (a, b) = (pts[n - 1], pts[n]);
    let len = dist(a.0, a.1, b.0, b.1).max(0.01);
    let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
    let tip = (b.0 + dx * 5.0, b.1 + dy * 5.0);
    c.poly(&[tip, (b.0 - dy * 4.0 - dx * 1.5, b.1 + dx * 4.0 - dy * 1.5), (b.0 + dy * 4.0 - dx * 1.5, b.1 - dx * 4.0 - dy * 1.5)], col);
    let start = f(0.0);
    c.circle(start.0, start.1, 2.6, col);
}

/// A shot shape's oval in its 80 x 50 cell at (x, y).
fn shot_oval(c: &mut Canvas, x: f32, y: f32, kind: usize, st: St) {
    let col = state_col(BLUE, st);
    let (cx, cy, rx, ry) = (x + 40.0, y + 24.0, 37.0, 21.0);
    let b = (cx - rx, cy - ry, cx + rx, cy + ry);
    drop_shadow(c, b, |px, py| sd_ellipse(px, py, cx, cy, rx, ry));
    c.ellipse(cx, cy, rx, ry, shade(col, -0.45));
    c.fill(
        b,
        |px, py| sd_ellipse(px, py, cx, cy, rx - 2.5, ry - 2.5),
        |_, py| mix(shade(col, 0.3), shade(col, -0.2), (py - (cy - ry)) / (2.0 * ry)),
    );
    c.fill(b, |px, py| sd_ellipse(px, py, cx, cy - ry * 0.45, rx * 0.72, ry * 0.36), |_, _| with_alpha(rgb(255, 255, 255), 0.2));
    let ink = match st {
        St::Off => mix(ink_on(col), col, 0.35),
        St::Lit => ink_on(BLUE),
        _ => ink_on(col),
    };
    shot_path(c, kind, cx + 0.6, cy + 0.9, with_alpha(rgb(0, 0, 0), 0.3));
    shot_path(c, kind, cx, cy, ink);
    if st == St::Lit {
        c.fill(
            (cx - rx - 2.0, cy - ry - 2.0, cx + rx + 2.0, cy + ry + 2.0),
            |px, py| sd_ellipse(px, py, cx, cy, rx + 0.6, ry + 0.6).abs() - 1.1,
            |_, _| with_alpha(GOLD_HI, 0.85),
        );
    }
}

/// Interface/JoeCoolPanel: the body (tabs, Customise, the round buttons, the aim's field), every control's four looks and
/// the round's scorecard.
fn player(c: &mut Canvas) {
    tray(c, 1);
    let (cx, cy, cr) = CUSTOMISE;
    disc(c, 255.0 + cx, 542.0 + cy, cr, CREAM, Pic::Face, St::Normal);
    for &(row, dy, p) in &ROUND_BUTTONS {
        disc(c, 298.0 + 19.0, row + dy, ROUND_R, ROUND_COL, p, St::Normal);
    }
    // the aim's lines are written centred on x 406 from y 542 in dark blue
    field(c, 336.0, 538.0, 146.0, 61.0, 10.0, FIELD);
    // the looks, in the order pointed at, selected, unavailable, idle
    let states = [St::Lit, St::Sel, St::Off, St::Normal];
    for (k, &st) in states.iter().enumerate() {
        let sx = 50.0 * k as f32;
        disc(c, sx + cx, 300.0 + cy, cr, CREAM, Pic::Face, st);
        for &(row, dy, p) in &ROUND_BUTTONS {
            disc(c, sx + 19.0, row + dy, ROUND_R, ROUND_COL, p, st);
        }
        // the shot shapes: one row each, the pointed-at column a pixel lower on the sheet
        for kind in 0..5 {
            shot_oval(c, 100.0 * k as f32, 50.0 * kind as f32 + if k == 0 { 1.0 } else { 0.0 }, kind, st);
        }
    }
    tab_lit(c, 0.0, 400.0, 0);
    tab_lit(c, 0.0, 450.0, 2);
    scorecard(c);
}

/// The round's scorecard, cut (297, 300, 503, 122) and drawn at (297, 493): the kind of round in black on the left, the
/// hole numbers and pars in grey after "Hole" and "Par" in white on a dark band (cells 18 apart centred from x 426), the
/// two golfers' name bars and their score cells, a wide column at the end for the totals.
fn scorecard(c: &mut Canvas) {
    let dy = 300.0 - 493.0;
    let card = shade(SLATE, 0.25);
    c.round_rect(300.0, 496.0 + dy, 500.0, 120.0, 14.0, shade(SLATE, -0.5));
    c.fill(
        (300.0, 496.0 + dy, 800.0, 616.0 + dy),
        |px, py| sd_rect(px, py, 301.5, 497.5 + dy, 497.0, 117.0, 12.5),
        |_, py| shade(card, 0.1 - 0.15 * ((py - 496.0 - dy) / 104.0).clamp(0.0, 1.0)),
    );
    // the band of hole numbers and pars
    let band = shade(BASE, -0.3);
    c.round_rect(403.0, 512.0 + dy, 395.0, 35.0, 7.0, shade(band, -0.4));
    c.round_rect(404.0, 513.0 + dy, 393.0, 33.0, 6.0, band);
    c.line(440.0, 529.5 + dy, 794.0, 529.5 + dy, 1.0, with_alpha(shade(band, 0.3), 0.6));
    for i in 1..=19 {
        let x = 426.0 + 18.0 * i as f32 - 9.0;
        let x = if i == 19 { 760.5 } else { x };
        c.line(x, 515.0 + dy, x, 544.0 + dy, 1.0, with_alpha(shade(band, 0.3), 0.6));
    }
    // the golfers' rows
    for y in [549.0, 570.0] {
        field(c, 316.0, y + dy, 114.0, 19.0, 9.5, PAPER);
        for i in 1..=18 {
            let x = 426.0 + 18.0 * i as f32 - 8.5;
            c.round_rect(x, y + dy, 17.0, 19.0, 2.5, shade(SLATE, -0.35));
            c.round_rect(x + 1.0, y + 1.0 + dy, 15.0, 17.0, 2.0, PAPER);
        }
        c.round_rect(761.0, y + dy, 36.0, 19.0, 3.0, shade(SLATE, -0.35));
        c.round_rect(762.0, y + 1.0 + dy, 34.0, 17.0, 2.5, PAPER);
    }
}

// ---- the Employee panel ------------------------------------------------------------------------------------------------------

/// Hire Employees: the centre in its 42 x 42 cut drawn at (254, 542).
const HIRE: (f32, f32, f32) = (21.0, 21.0, 18.0);
/// The eight portraits (63 x 43), as panels_ui.rs PORTRAITS.
const PORTRAITS: [(f32, f32); 8] =
    [(336.0, 507.0), (336.0, 550.0), (399.0, 507.0), (399.0, 550.0), (462.0, 507.0), (462.0, 550.0), (525.0, 507.0), (525.0, 550.0)];
/// Move, Fire and Rename: their symbol and colour; each look a disc about (26, 24) of its 52 x 48 cell.
const ACTIONS: [(Pic, Col); 3] = [(Pic::Move, BLUE), (Pic::Fire, RED), (Pic::Pencil, BLUE)];

/// A portrait's pad (63 x 43 at (x, y)): a rounded plate with a stage the figure stands on (its ground at (30, 24)).
fn pad(c: &mut Canvas, x: f32, y: f32, st: St) {
    let base = match st {
        St::Normal => shade(SLATE, -0.1),
        St::Lit => shade(SLATE, 0.35),
        St::Sel => GOLD,
        St::Off => mix(SLATE, GREY, 0.7),
    };
    let rim = if st == St::Lit { GOLD_HI } else { shade(base, -0.5) };
    c.round_rect(x + 1.0, y + 1.0, 61.0, 41.0, 9.0, rim);
    c.fill(
        (x, y, x + 63.0, y + 43.0),
        |px, py| sd_rect(px, py, x + 2.5, y + 2.5, 58.0, 38.0, 7.5),
        |_, py| shade(base, 0.15 - 0.25 * ((py - y) / 43.0)),
    );
    c.ellipse(x + 31.0, y + 27.0, 23.0, 8.0, shade(base, -0.25));
    c.ellipse(x + 31.0, y + 26.0, 21.5, 6.8, shade(base, 0.1));
}

/// A scroll arrow of the portraits (16 x 62 at (x, y)), pointing left (-1) or right (1).
fn tall_arrow(c: &mut Canvas, x: f32, y: f32, dir: f32, st: St) {
    let base = if st == St::Off { mix(BASE, GREY, 0.6) } else { state_col(BLUE, st) };
    c.round_rect(x + 1.0, y + 1.0, 14.0, 60.0, 7.0, shade(base, -0.5));
    c.fill(
        (x, y, x + 16.0, y + 62.0),
        |px, py| sd_rect(px, py, x + 2.5, y + 2.5, 11.0, 57.0, 5.5),
        |px, _| shade(base, 0.25 - 0.4 * ((px - x) / 16.0)),
    );
    let (cx, cy) = (x + 8.0, y + 31.0);
    let tri = [(cx + dir * 4.5, cy), (cx - dir * 3.5, cy - 8.0), (cx - dir * 3.5, cy + 8.0)];
    let ink = match st {
        St::Off => shade(GREY, 0.2),
        St::Lit => GOLD_HI,
        _ => LIGHT,
    };
    c.poly(&tri, ink);
}

/// Interface/EmployeePanel: the body (tabs, Hire, the portraits' pads, the scroll arrows unlit, the field for the selected
/// employee's lines), the pads' and arrows' looks, Hire's, and Move, Fire and Rename in their four looks.
fn employee(c: &mut Canvas) {
    tray(c, 2);
    let (hx, hy, hr) = HIRE;
    disc(c, 254.0 + hx, 542.0 + hy, hr, CREAM, Pic::Hire, St::Normal);
    for &(x, y) in &PORTRAITS {
        pad(c, x, y, St::Normal);
    }
    tall_arrow(c, 313.0, 519.0, -1.0, St::Off);
    tall_arrow(c, 596.0, 519.0, 1.0, St::Off);
    // the selected employee's name, hire date, pay and count are written centred on x 700 from y 534
    field(c, 614.0, 527.0, 182.0, 72.0, 12.0, FIELD);
    // the looks: the pads pointed at and selected (rows 0 and 50 for the upper and lower portraits)
    for row in [0.0, 50.0] {
        pad(c, 0.0, row, St::Lit);
        pad(c, 100.0, row, St::Sel);
        pad(c, 200.0, row, St::Off);
    }
    // the arrows pointed at (x 0) and usable (x 100)
    for (row, dir) in [(200.0, -1.0), (300.0, 1.0)] {
        tall_arrow(c, 0.0, row, dir, St::Lit);
        tall_arrow(c, 50.0, row, dir, St::Off);
        tall_arrow(c, 100.0, row, dir, St::Normal);
    }
    // Hire pointed at and unavailable (the employee limit)
    disc(c, hx, 500.0 + hy, hr, CREAM, Pic::Hire, St::Lit);
    disc(c, 50.0 + hx, 500.0 + hy, hr, CREAM, Pic::Hire, St::Sel);
    disc(c, 100.0 + hx, 500.0 + hy, hr, CREAM, Pic::Hire, St::Off);
    // Move, Fire, Rename: pointed at (x 400), armed (500), unavailable (600) and idle (700)
    for (k, &(p, col)) in ACTIONS.iter().enumerate() {
        let row = 50.0 * k as f32;
        for (j, st) in [St::Lit, St::Sel, St::Off, St::Normal].into_iter().enumerate() {
            disc(c, 400.0 + 100.0 * j as f32 + 26.0, row + 24.0, 17.0, col, p, st);
        }
    }
    tab_lit(c, 0.0, 400.0, 0);
    tab_lit(c, 0.0, 450.0, 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The idle look of every control the engine leaves to the body is drawn, opaque, at its place.
    #[test]
    fn idle_controls_are_in_the_bodies() {
        let solid = |c: &Canvas, x: i32, y: i32| c.get(x, y)[3] > 0.99;
        let m = sheet("memberpanel.pcx", 800, 600).unwrap();
        let p = sheet("joecoolpanel.pcx", 800, 600).unwrap();
        let e = sheet("employeepanel.pcx", 800, 600).unwrap();
        for c in [&m, &p, &e] {
            for &(x, y, _) in &TABS {
                assert!(solid(c, x as i32, y as i32));
            }
        }
        // Customise and the round buttons; Hire; the portraits; the scroll arrows
        for (x, y) in [(276, 564), (317, 519), (317, 550), (317, 581)] {
            assert!(solid(&p, x, y));
        }
        assert!(solid(&e, 275, 563) && solid(&e, 366, 530) && solid(&e, 321, 549));
        assert!(solid(&m, 318, 593) && solid(&m, 785, 593));
        // every mood face
        for k in 1..=10 {
            assert!(solid(&m, 602 - 16 * (k - 1), 108));
        }
    }
}
