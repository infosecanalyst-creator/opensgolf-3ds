//! Interface pictures: panels, buttons, screens and portraits in the free set's own look (a slate blue frame, cream fields,
//! gold for what is selected and green for what is on), drawn into the boxes where the engine cuts the original's pieces
//! (manifest.txt) so every cut finds a finished piece of art. Some sheets get their own drawing: the terrain buttons (a
//! small tile of each terrain), the hole signs (their numbers), the faces and the title screen (the OpenSGolf name).
use super::enc;
use super::paint::{alpha as with_alpha, mix, rgb, sd_ellipse, sd_poly, sd_rect, shade, smooth_noise, Canvas, Col};
use super::{hash, theme_of, Comp, Entry, Item};

pub const BASE: Col = rgb(66, 108, 150);
pub const GOLD: Col = rgb(228, 180, 64);
pub const GREEN: Col = rgb(84, 168, 92);
pub const GREY: Col = rgb(140, 146, 156);
pub const PAPER: Col = rgb(246, 238, 214);
pub const DARK: Col = rgb(36, 46, 58);
pub const RED: Col = rgb(200, 74, 62);
pub const OUTLINE: Col = rgb(26, 40, 56);

/// The colour of a tone class and lightness from the manifest.
pub fn tone(t: u8, light: u8) -> Col {
    let c = match t {
        b'g' => GOLD,
        b'e' => GREEN,
        b's' => GREY,
        b'w' => PAPER,
        b'k' => DARK,
        b'r' => RED,
        _ => BASE,
    };
    let k = match light {
        0 => -0.3,
        1 => -0.08,
        2 => 0.1,
        _ => 0.28,
    };
    if t == b'w' {
        shade(c, (k * 0.3f32).min(0.0))
    } else {
        shade(c, k)
    }
}

/// One piece of a sheet in its box: a bevelled panel, a round or oval button, or a diamond tile.
pub fn element(c: &mut Canvas, p: &Comp) {
    let (x, y, w, h) = (p.x, p.y, p.w, p.h);
    let col = tone(p.tone, p.light);
    let big = w * h > 20000.0;
    // a long oval is drawn as a pill (bars, tabs, the long panels with rounded ends)
    // and a big piece (a report's frame, a panel) as a plain panel: the engine writes into it
    let shape = if w * h > 40000.0 && !(p.tone == b'w' && p.shape == b'E') {
        b'R'
    } else if p.shape == b'E' && (w > 2.2 * h || h > 2.2 * w) {
        b'P'
    } else {
        p.shape
    };
    let sd = |px: f32, py: f32| -> f32 {
        match shape {
            b'P' => sd_rect(px, py, x, y, w, h, w.min(h) / 2.0),
            b'E' => sd_ellipse(px, py, x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0),
            b'D' => sd_poly(px, py, &[(x + w / 2.0, y), (x + w, y + h / 2.0), (x + w / 2.0, y + h), (x, y + h / 2.0)]),
            b'O' => sd_rect(px, py, x, y, w, h, (w.min(h) * 0.3).min(14.0)),
            _ => sd_rect(px, py, x, y, w, h, if big { 9.0 } else { (w.min(h) * 0.18).min(6.0) }),
        }
    };
    let round = shape == b'E';
    let paper = p.tone == b'w';
    c.fill((x, y, x + w, y + h), sd, |px, py| {
        let d = sd(px, py);
        let edge = (-d).clamp(0.0, 3.0) / 3.0;
        // a bevel: light from the top left, a dark outline
        let (u, v) = ((px - x) / w.max(1.0), (py - y) / h.max(1.0));
        let lit = if round {
            let (nx, ny) = (u * 2.0 - 1.0, v * 2.0 - 1.0);
            0.22 - 0.3 * (nx * 0.5 + ny * 0.8).clamp(-1.0, 1.0) - 0.15 * (nx * nx + ny * ny)
        } else if paper {
            0.0
        } else {
            0.12 - 0.22 * v
        };
        let inner = shade(col, lit);
        if edge < 0.34 {
            mix(OUTLINE, inner, edge * 2.0)
        } else if edge < 0.67 && !paper {
            shade(inner, 0.18)
        } else {
            inner
        }
    });
    if paper && w > 30.0 && h > 14.0 {
        // an inset shadow along the top of a field
        c.fill(
            (x + 2.0, y + 2.0, x + w - 2.0, y + 5.0),
            |px, py| (sd(px, py) + 2.0).max(py - (y + 4.5)),
            |_, _| with_alpha(shade(col, -0.25), 0.5),
        );
    }
}

/// The free set's background for a whole screen: a calm gradient of the theme colours with a faint golf ball dimple
/// pattern.
fn background(c: &mut Canvas, seed: u32, warm: bool) {
    let (w, h) = (c.w, c.h);
    let (top, bottom) = if warm { (rgb(150, 92, 60), rgb(92, 52, 36)) } else { (rgb(176, 200, 222), rgb(112, 146, 176)) };
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32, y as f32);
            let t = fy / h as f32;
            let n = smooth_noise(fx, fy, 90.0, seed, 0);
            let mut col = shade(mix(top, bottom, t), (n - 0.5) * 0.12);
            // dimples on a 24 pixel lattice, offset every other row
            let (gx, gy) = (fx + if (fy / 24.0) as i32 % 2 == 1 { 12.0 } else { 0.0 }, fy);
            let (dx, dy) = (gx % 24.0 - 12.0, gy % 24.0 - 12.0);
            let d = (dx * dx + dy * dy).sqrt();
            if d < 6.0 {
                col = shade(col, -0.05 * (1.0 - d / 6.0) + if dx + dy < 0.0 { 0.02 } else { 0.0 });
            }
            c.set(x as i32, y as i32, col);
        }
    }
}

/// The picture of a manifest entry.
pub fn picture(e: &Entry, low: &str) -> Option<Vec<u8>> {
    let Item::Pic { w, h, key, comps, full } = &e.item else { return None };
    let (w, h) = (*w as usize, *h as usize);
    let name = low.rsplit('/').next().unwrap_or(low);
    if low.starts_with("bodies/") {
        return Some(super::sprite::body_still(name, w as u32, h as u32));
    }
    let c = draw(low, name, w, h, comps, *full);
    let keyed = key.unwrap_or([255, 0, 255]);
    if low.starts_with("heads/") {
        // listed and read as 8-bit PCX by the engine
        return Some(enc::pcx(&c.to_rgba_keyed(keyed, 0.5), Some(keyed)));
    }
    let img = if *full { c.to_rgba_keyed(keyed, 0.02) } else { c.to_rgba_keyed(keyed, 0.5) };
    crate::png::encode_png(&img)
}

/// The alpha sheet that goes with picture `of` (white where it is drawn).
pub fn alpha(of: &Entry, _low: &str) -> Option<Vec<u8>> {
    let Item::Pic { w, h, comps, full, .. } = &of.item else { return None };
    let olow = of.path.to_lowercase();
    let name = olow.rsplit('/').next().unwrap_or(&olow).to_string();
    let c = draw(&olow, &name, *w as usize, *h as usize, comps, *full);
    crate::png::encode_png(&c.alpha_sheet())
}

fn draw(low: &str, name: &str, w: usize, h: usize, comps: &[Comp], full: bool) -> Canvas {
    let mut c = Canvas::new(w, h, [0.0; 4]);
    let seed = hash(low);
    if full {
        let warm = name.contains("top10") || name.contains("bulletin") || name.contains("credits");
        if name == "black.pcx" {
            c = Canvas::new(w, h, rgb(0, 0, 0));
            return c;
        }
        background(&mut c, seed, warm);
        // the fields and panels: even boxes and paper ovals (the original's textures and globes give nothing worth drawing)
        let decorative = warm || name == "splash.pcx" || name == "logo.pcx";
        // rectangles, and the oval fields (not round shapes, which are the original's pictures: balls, globes, faces)
        let field = |p: &&Comp| matches!(p.shape, b'R' | b'O') || matches!(p.tone, b'w' | b'k') && p.w > 1.6 * p.h;
        for p in comps.iter().filter(|_| !decorative).filter(field) {
            element(&mut c, p);
        }
        match name {
            "logo.pcx" | "splash.pcx" => title_logo(&mut c, 400.0, 250.0),
            "titleseldiffunsel.pcx" => {
                for (k, &(x, y)) in [(388.0f32, 113.0f32), (360.0, 227.0), (366.0, 346.0), (402.0, 464.0)].iter().enumerate() {
                    c.circle(x - 175.0, y + 8.0, 36.0, shade(BASE, 0.1));
                    face(&mut c, x - 175.0, y + 8.0, 30.0, [0, 4, 9, 13][k], [0, 1, 2, 1][k], false);
                }
            }
            "worldbase.pcx" => globe(&mut c, 150.0, 330.0, 140.0),
            _ => {}
        }
        if name.starts_with("choose") {
            theme_icons(&mut c, name);
        }
        if warm && name.contains("credits") {
            c.round_rect(220.0, 120.0, 360.0, 400.0, 6.0, rgb(250, 248, 240));
            c.round_rect(236.0, 136.0, 328.0, 230.0, 2.0, rgb(160, 200, 230));
            title_logo(&mut c, 400.0, 440.0);
        }
        return c;
    }
    if name == "3mainlowerleft.pcx" {
        dock(&mut c);
        return c;
    }
    if name == "titleunsel.pcx" || name == "titlemo.pcx" {
        title_buttons(&mut c, comps, name == "titlemo.pcx");
        return c;
    }
    if name.contains("terrainbuttons") {
        terrain_buttons(&mut c, comps, theme_of(low));
        return c;
    }
    if name.starts_with("golfballhalopage") || name.contains("expressionsflat") {
        faces(&mut c, name, low.contains("female"));
        return c;
    }
    if low.starts_with("heads/") {
        // a custom head: three faces stacked
        for e in 0..3 {
            face(&mut c, 70.0, 70.0 + e as f32 * 140.0, 55.0, (hash(name) % 19) as u8, e, name.starts_with("f_"));
        }
        return c;
    }
    for p in comps {
        element(&mut c, p);
    }
    if name == "holemark.pcx" {
        hole_numbers(&mut c, comps);
    }
    c
}

/// The dock in the lower left (render.rs DOCK, DOCK_LIT, DOCK_TRAIL): the assembled dock with its ten buttons in the box
/// (0, 430, 215, 170), the buttons' bright and gold looks at their cuts, and the gold trails.
fn dock(c: &mut Canvas) {
    // centre and radius of each button on the dock, its glyph
    const BTN: [(f32, f32, f32, Glyph); 10] = [
        (43.0, 473.0, 33.0, Glyph::Build),
        (117.0, 497.0, 30.0, Glyph::House),
        (177.0, 536.0, 26.0, Glyph::People),
        (32.0, 543.0, 11.0, Glyph::Plus),
        (31.0, 585.0, 11.0, Glyph::Minus),
        (17.0, 565.0, 11.0, Glyph::TurnRight),
        (47.0, 565.0, 11.0, Glyph::TurnLeft),
        (75.0, 552.0, 14.0, Glyph::Info),
        (107.0, 568.0, 14.0, Glyph::Pause),
        (133.0, 583.0, 13.0, Glyph::Gear),
    ];
    // the lit looks: (sheet x, y, w, h) per button (bright blue; gold 100 to the right for the big ones, 50 for the small)
    const LIT: [(f32, f32, f32, f32); 10] = [
        (0.0, 0.0, 78.0, 81.0),
        (0.0, 100.0, 78.0, 81.0),
        (0.0, 200.0, 78.0, 81.0),
        (600.0, 50.0, 36.0, 38.0),
        (600.0, 100.0, 36.0, 38.0),
        (600.0, 150.0, 36.0, 38.0),
        (600.0, 200.0, 36.0, 38.0),
        (600.0, 250.0, 36.0, 38.0),
        (600.0, 300.0, 36.0, 38.0),
        (600.0, 350.0, 36.0, 38.0),
    ];
    // the dock: a quarter disc in the corner with a rim
    let (ox, oy) = (0.0, 600.0);
    c.fill(
        (0.0, 430.0, 215.0, 600.0),
        |x, y| sd_ellipse(x, y, ox, oy, 212.0, 166.0),
        |x, y| {
            let d = -sd_ellipse(x, y, ox, oy, 212.0, 166.0);
            let base = shade(BASE, -0.05 - (600.0 - y) / 600.0);
            if d < 3.0 {
                OUTLINE
            } else if d < 7.0 {
                shade(BASE, 0.3)
            } else {
                base
            }
        },
    );
    for &(x, y, r, g) in &BTN {
        round_button(c, x, y, r, shade(BASE, 0.05), g);
    }
    for (k, &(sx, sy, w, h)) in LIT.iter().enumerate() {
        let (_, _, _, g) = BTN[k];
        let r = w.min(h) / 2.0 - 2.0;
        let gold_dx = if k < 3 { 100.0 } else { 50.0 };
        round_button(c, sx + w / 2.0, sy + h / 2.0, r, shade(BASE, 0.4), g);
        round_button(c, sx + gold_dx + w / 2.0, sy + h / 2.0, r, GOLD, g);
    }
    // the gold trails from the open mode's button towards the panel
    for m in 0..3 {
        let (x0, y0) = (300.0, 100.0 * m as f32);
        for k in 0..30 {
            let u = k as f32 / 29.0;
            let (x, y) = (x0 + 8.0 + u * 200.0, y0 + 10.0 + u * u * 70.0);
            c.circle(x, y, 3.0, GOLD);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Glyph {
    Build,
    House,
    People,
    Plus,
    Minus,
    TurnRight,
    TurnLeft,
    Info,
    Pause,
    Gear,
    Folder,
    BallOnTee,
    Shovel,
    Cards,
    Trophy,
    Exit,
}

/// A round button of colour `col` at (x, y) with a glyph.
fn round_button(c: &mut Canvas, x: f32, y: f32, r: f32, col: Col, g: Glyph) {
    c.fill(
        (x - r, y - r, x + r, y + r),
        |px, py| ((px - x).powi(2) + (py - y).powi(2)).sqrt() - r,
        |px, py| {
            let d = r - ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            let (nx, ny) = ((px - x) / r, (py - y) / r);
            let lit = 0.18 - 0.25 * (nx * 0.5 + ny * 0.8);
            if d < 1.6 {
                OUTLINE
            } else if d < 3.0 {
                shade(col, 0.35)
            } else {
                shade(col, lit)
            }
        },
    );
    glyph(c, g, x, y, r * 0.62, rgb(250, 250, 245));
}

/// A simple white symbol in a circle of radius r.
fn glyph(c: &mut Canvas, g: Glyph, x: f32, y: f32, r: f32, col: Col) {
    let w = (r * 0.22).max(1.5);
    match g {
        Glyph::Build => {
            // a flag on a green
            c.ellipse(x, y + r * 0.6, r * 0.8, r * 0.28, rgb(120, 200, 110));
            c.line(x, y + r * 0.6, x, y - r * 0.8, w * 0.6, col);
            c.poly(&[(x, y - r * 0.8), (x + r * 0.7, y - r * 0.55), (x, y - r * 0.3)], rgb(220, 60, 60));
        }
        Glyph::House => {
            c.poly(&[(x - r * 0.8, y), (x, y - r * 0.8), (x + r * 0.8, y)], col);
            c.rect(x - r * 0.55, y, r * 1.1, r * 0.75, col);
            c.rect(x - r * 0.15, y + r * 0.3, r * 0.3, r * 0.45, shade(BASE, -0.3));
        }
        Glyph::People => {
            for s in [-0.4f32, 0.4] {
                c.circle(x + s * r, y - r * 0.35, r * 0.28, col);
                c.round_rect(x + s * r - r * 0.32, y, r * 0.64, r * 0.75, r * 0.25, col);
            }
        }
        Glyph::Plus | Glyph::Minus => {
            c.line(x - r * 0.7, y, x + r * 0.7, y, w, col);
            if g == Glyph::Plus {
                c.line(x, y - r * 0.7, x, y + r * 0.7, w, col);
            }
        }
        Glyph::TurnRight | Glyph::TurnLeft => {
            let s = if g == Glyph::TurnRight { 1.0 } else { -1.0 };
            for k in 0..10 {
                let a = std::f32::consts::PI * (0.2 + k as f32 * 0.12);
                c.circle(x - s * a.cos() * r * 0.6, y - a.sin() * r * 0.6, w * 0.5, col);
            }
            c.poly(&[(x + s * r * 0.6, y - r * 0.1), (x + s * r * 0.95, y - r * 0.5), (x + s * r * 0.25, y - r * 0.5)], col);
        }
        Glyph::Info => {
            c.circle(x, y - r * 0.55, w * 0.7, col);
            c.line(x, y - r * 0.15, x, y + r * 0.7, w, col);
        }
        Glyph::Pause => {
            c.line(x - r * 0.3, y - r * 0.6, x - r * 0.3, y + r * 0.6, w, col);
            c.line(x + r * 0.3, y - r * 0.6, x + r * 0.3, y + r * 0.6, w, col);
        }
        Glyph::Gear => {
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::PI / 4.0;
                c.line(x, y, x + a.cos() * r * 0.8, y + a.sin() * r * 0.8, w, col);
            }
            c.circle(x, y, r * 0.5, col);
            c.circle(x, y, r * 0.2, shade(BASE, -0.2));
        }
        Glyph::Folder => {
            c.round_rect(x - r * 0.8, y - r * 0.45, r * 0.7, r * 0.3, 2.0, col);
            c.round_rect(x - r * 0.8, y - r * 0.3, r * 1.6, r * 1.0, 3.0, col);
        }
        Glyph::BallOnTee => {
            c.circle(x, y - r * 0.25, r * 0.5, col);
            c.poly(
                &[(x - r * 0.25, y + r * 0.3), (x + r * 0.25, y + r * 0.3), (x + r * 0.05, y + r * 0.9), (x - r * 0.05, y + r * 0.9)],
                col,
            );
        }
        Glyph::Shovel => {
            c.line(x - r * 0.6, y - r * 0.6, x + r * 0.2, y + r * 0.2, w, col);
            c.poly(
                &[(x + r * 0.05, y + r * 0.2), (x + r * 0.45, y - r * 0.1), (x + r * 0.85, y + r * 0.6), (x + r * 0.5, y + r * 0.85)],
                col,
            );
        }
        Glyph::Cards => {
            c.round_rect(x - r * 0.75, y - r * 0.6, r * 0.9, r * 0.9, 3.0, col);
            c.round_rect(x - r * 0.15, y - r * 0.2, r * 0.9, r * 0.9, 3.0, shade(col, -0.2));
        }
        Glyph::Trophy => {
            c.fill(
                (x - r, y - r, x + r, y + r),
                |px, py| sd_ellipse(px, py, x, y - r * 0.45, r * 0.6, r * 0.5).max(py - (y - r * 0.45)),
                |_, _| col,
            );
            c.line(x, y - r * 0.4, x, y + r * 0.4, w, col);
            c.rect(x - r * 0.45, y + r * 0.4, r * 0.9, r * 0.3, col);
        }
        Glyph::Exit => {
            c.rect(x - r * 0.6, y - r * 0.7, r * 0.6, r * 1.4, col);
            c.line(x - r * 0.1, y, x + r * 0.8, y, w, col);
            c.poly(&[(x + r * 0.8, y - r * 0.35), (x + r * 0.8, y + r * 0.35), (x + r * 1.05, y)], col);
        }
    }
}

/// The title screen's round buttons (TitleUnSel) in the label ovals' boxes (render.rs MENU_BTN), and their lit looks with
/// the OpenSGolf logo (TitleMO: the engine keeps the shapes touching each box, the logo's box is (170, 190, 480, 165)).
fn title_buttons(c: &mut Canvas, comps: &[Comp], lit: bool) {
    const MENU: [(f32, f32, f32, f32, Glyph); 6] = [
        (40.0, 25.0, 325.0, 120.0, Glyph::Folder),
        (415.0, 40.0, 360.0, 135.0, Glyph::BallOnTee),
        (12.0, 395.0, 285.0, 100.0, Glyph::Shovel),
        (508.0, 380.0, 270.0, 95.0, Glyph::Cards),
        (285.0, 478.0, 285.0, 105.0, Glyph::Trophy),
        (718.0, 528.0, 64.0, 64.0, Glyph::Exit),
    ];
    let unsel: Vec<Comp> = super::manifest()
        .entries
        .iter()
        .find(|e| e.path.eq_ignore_ascii_case("Interface/TitleUnSel.pcx"))
        .and_then(|e| if let Item::Pic { comps, .. } = &e.item { Some(comps.clone()) } else { None })
        .unwrap_or_else(|| comps.to_vec());
    for &(x, y, w, h, g) in &MENU {
        let icon = unsel.iter().filter(|p| p.w > 20.0).find(|p| {
            let (cx, cy) = (p.x + p.w / 2.0, p.y + p.h / 2.0);
            cx >= x && cx < x + w && cy >= y && cy < y + h
        });
        let (cx, cy, r) =
            icon.map(|p| (p.x + p.w / 2.0, p.y + p.h / 2.0, p.w.min(p.h) / 2.0 - 2.0)).unwrap_or((x + w - 30.0, y + 30.0, 26.0));
        if lit && g != Glyph::Exit {
            // the whole oval lights up in gold, the button over it
            c.fill(
                (x, y, x + w, y + h),
                |px, py| sd_ellipse(px, py, x + w / 2.0, y + h / 2.0, w / 2.0 - 4.0, h / 2.0 - 6.0),
                |_, py| shade(GOLD, 0.25 - (py - y) / h * 0.3),
            );
            c.fill(
                (x, y, x + w, y + h),
                |px, py| sd_ellipse(px, py, x + w / 2.0, y + h / 2.0, w / 2.0 - 16.0, h / 2.0 - 18.0),
                |_, _| shade(PAPER, 0.3),
            );
        }
        round_button(c, cx, cy, r, if lit { GOLD } else { BASE }, g);
    }
    if lit {
        title_logo(c, 410.0, 272.0);
    }
}

/// A globe of our own for the world map.
fn globe(c: &mut Canvas, cx: f32, cy: f32, r: f32) {
    c.fill(
        (cx - r, cy - r, cx + r, cy + r),
        |x, y| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r,
        |x, y| {
            let (nx, ny) = ((x - cx) / r, (y - cy) / r);
            let land = smooth_noise(x, y, 38.0, 4242, 0) > 0.56;
            let base = if land { rgb(96, 170, 84) } else { rgb(70, 130, 196) };
            shade(base, 0.2 - 0.35 * (nx * 0.5 + ny * 0.7) - 0.25 * (nx * nx + ny * ny))
        },
    );
}

/// The OpenSGolf name on a paper oval centred at (cx, cy).
fn title_logo(c: &mut Canvas, cx: f32, cy: f32) {
    c.ellipse(cx, cy, 234.0, 76.0, shade(BASE, -0.2));
    c.ellipse(cx, cy, 231.0, 73.0, PAPER);
    // a golf ball on a tee beside the name
    c.circle(cx - 170.0, cy - 6.0, 26.0, rgb(250, 250, 250));
    c.circle(cx - 178.0, cy - 14.0, 8.0, with_alpha(rgb(255, 255, 255), 0.9));
    for k in 0..6 {
        let a = k as f32;
        c.circle(cx - 170.0 + (a * 1.7).cos() * 12.0, cy - 6.0 + (a * 1.7).sin() * 12.0, 2.5, rgb(214, 218, 224));
    }
    c.poly(&[(cx - 178.0, cy + 20.0), (cx - 162.0, cy + 20.0), (cx - 168.0, cy + 46.0), (cx - 172.0, cy + 46.0)], rgb(200, 160, 90));
    let (wo, ws) = (Canvas::text_width("Open", 52.0), Canvas::text_width("SGolf", 52.0));
    let x0 = cx + 20.0 - (wo + ws) / 2.0;
    c.text("Open", x0, cy - 46.0, 52.0, rgb(40, 120, 70), 0);
    c.text("SGolf", x0 + wo, cy - 46.0, 52.0, rgb(40, 90, 150), 0);
    c.text("free golf course designer", cx + 30.0, cy + 22.0, 18.0, rgb(90, 100, 110), 1);
}

/// The theme pack buttons strip: a little scene per theme in the four states.
fn theme_icons(c: &mut Canvas, name: &str) {
    let ground = if name.contains("desert") {
        rgb(210, 170, 110)
    } else if name.contains("tropical") {
        rgb(80, 180, 120)
    } else if name.contains("links") {
        rgb(140, 160, 100)
    } else {
        rgb(90, 160, 70)
    };
    for k in 0..4 {
        let x = k as f32 * 200.0;
        c.rect(x, 0.0, 200.0, c.h as f32, rgb(255, 0, 0));
        let st = [0.0, 0.15, -0.1, -0.3][k];
        c.round_rect(x + 8.0, 8.0, 182.0, 73.0, 14.0, shade(BASE, st));
        c.ellipse(x + 50.0, 45.0, 34.0, 30.0, shade(ground, st));
        c.circle(x + 50.0, 30.0, 10.0, shade(rgb(60, 120, 60), st));
        c.round_rect(x + 92.0, 30.0, 92.0, 30.0, 8.0, shade(PAPER, st));
    }
}

/// The terrain buttons: a small tile of each terrain in its four states (normal, pointed at, selected, unavailable), the
/// tree buttons as a tree on a tile, and the flag icons.
fn terrain_buttons(c: &mut Canvas, comps: &[Comp], theme: usize) {
    // the slots' terrain, in the order the sheet holds them (docs/UI_ART_MAP.md: ids 0,1,7,4,9,10,17,2,3,5,8,11,12,...)
    let slot_col = |k: usize| -> Col {
        let sand = rgb(236, 220, 168);
        let grass = [rgb(96, 160, 66), rgb(164, 162, 86), rgb(70, 168, 72), rgb(122, 150, 86)][theme];
        match k {
            0 => shade(grass, 0.15),
            1 => shade(grass, 0.3),
            2 | 4 => sand,
            3 => shade(grass, -0.15),
            5 => rgb(110, 90, 70),
            6 => rgb(60, 120, 180),
            7 => shade(grass, 0.08),
            8 => mix(grass, sand, 0.3),
            9 => shade(grass, -0.35),
            10 => mix(sand, grass, 0.2),
            11 => rgb(90, 110, 60),
            _ => rgb(150, 145, 135),
        }
    };
    for p in comps {
        let (cx, cy) = (p.x + p.w / 2.0, p.y + p.h / 2.0);
        if p.x < 500.0 && p.y < 400.0 && p.w > 30.0 {
            let col = ((p.x + 4.0) / 62.0) as usize;
            let row = ((p.y + 4.0) / 54.0) as usize;
            let slot = row * 2 + col / 4;
            let state = col % 4;
            iso_button(c, p, slot_col(slot), state);
            // a mark on some tiles: a flag on the green, a tee peg on the tee, waves on water
            match slot {
                1 => {
                    c.line(cx, cy + 4.0, cx, cy - 12.0, 1.5, rgb(240, 240, 240));
                    c.poly(&[(cx, cy - 12.0), (cx + 9.0, cy - 9.0), (cx, cy - 6.0)], rgb(210, 50, 50));
                }
                0 => c.circle(cx, cy - 2.0, 3.0, rgb(250, 250, 250)),
                6 => {
                    for k in 0..2 {
                        let yy = cy - 3.0 + k as f32 * 6.0;
                        c.line(cx - 8.0, yy, cx + 8.0, yy, 1.5, with_alpha(rgb(220, 240, 255), 0.8));
                    }
                }
                _ => {}
            }
        } else if p.x >= 500.0 && p.w > 30.0 {
            let state = (((p.x - 500.0) / 80.0) as usize).min(3);
            let kind = ((p.y / 150.0) as usize).min(2);
            iso_button(c, p, rgb(120, 96, 70), state);
            let (tx, ty) = (cx, p.y + p.h * 0.55);
            c.line(tx, ty, tx, ty - p.h * 0.35, 3.0, rgb(110, 76, 46));
            match kind {
                1 => c.poly(&[(tx, ty - p.h * 0.62), (tx + 13.0, ty - p.h * 0.1), (tx - 13.0, ty - p.h * 0.1)], rgb(40, 110, 60)),
                2 => {
                    for a in 0..5 {
                        let ang = a as f32 * 1.25;
                        c.line(tx, ty - p.h * 0.36, tx + ang.cos() * 14.0, ty - p.h * 0.36 + ang.sin() * 7.0 + 3.0, 3.0, rgb(70, 150, 60));
                    }
                }
                _ => c.circle(tx, ty - p.h * 0.42, 15.0, rgb(60, 140, 60)),
            }
        } else {
            element(c, p);
        }
    }
}

/// A rounded diamond button holding a tile of `col`: state 0 normal, 1 pointed at (lighter), 2 selected (gold rim),
/// 3 unavailable (grey).
fn iso_button(c: &mut Canvas, p: &Comp, col: Col, state: usize) {
    let (x, y, w, h) = (p.x, p.y, p.w, p.h);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let diamond = |sx: f32, sy: f32| [(cx, cy - sy), (cx + sx, cy), (cx, cy + sy), (cx - sx, cy)];
    let (sx, sy) = (w / 2.0 - 1.0, h / 2.0 - 1.0);
    let frame = match state {
        2 => GOLD,
        3 => GREY,
        _ => shade(BASE, 0.25),
    };
    let outer = diamond(sx, sy);
    c.fill((x, y, x + w, y + h), |px, py| sd_poly(px, py, &outer) - 3.0, |_, py| shade(frame, 0.2 - (py - y) / h * 0.4));
    let inner = diamond(sx * 0.78, sy * 0.78);
    let tile = match state {
        1 => shade(col, 0.2),
        3 => mix(col, GREY, 0.75),
        _ => col,
    };
    c.fill(
        (x, y, x + w, y + h),
        |px, py| sd_poly(px, py, &inner),
        |px, py| shade(tile, (smooth_noise(px, py, 3.0, 7, 0) - 0.5) * 0.12 - (py - cy) / h * 0.15),
    );
}

/// The numbered hole signs (docs/UI_ART_MAP.md: 1..18 in three sizes, 9 to a row): each sign gets its number, the signs
/// counted in reading order within each size.
fn hole_numbers(c: &mut Canvas, comps: &[Comp]) {
    let mut signs: Vec<&Comp> = comps.iter().filter(|p| p.w >= 10.0 && p.h >= 10.0).collect();
    signs.sort_by_key(|p| ((p.y / 12.0) as i32, p.x as i32));
    for (k, p) in signs.iter().enumerate() {
        let n = k % 18 + 1;
        c.round_rect(p.x + 1.0, p.y + 1.0, p.w - 2.0, p.h * 0.7, 2.0, rgb(250, 248, 236));
        c.text(&n.to_string(), p.x + p.w / 2.0, p.y + 1.0, (p.h * 0.6).max(8.0), rgb(30, 60, 30), 1);
        c.line(p.x + p.w / 2.0, p.y + p.h * 0.7, p.x + p.w / 2.0, p.y + p.h, 1.5, rgb(120, 90, 60));
    }
}

/// Faces: the halo page (140 x 140 cells, two heads to a column, three expressions each) or the expressions page (90 x
/// 120 cells, 100 apart, rows 372 apart).
fn faces(c: &mut Canvas, name: &str, female: bool) {
    let halo = name.starts_with("golfballhalopage");
    for head in 0..19u8 {
        for e in 0..3 {
            let (cx, cy, r) = if halo {
                ((head / 2) as f32 * 140.0 + 70.0, (head & 1) as f32 * 420.0 + e as f32 * 140.0 + 72.0, 50.0)
            } else {
                ((head / 2) as f32 * 100.0 + 45.0, (head & 1) as f32 * 372.0 + [4.0, 128.0, 252.0][e] + 62.0, 36.0)
            };
            face(c, cx, cy, r, head, e, female);
        }
    }
}

/// A cartoon face: skin, hair, eyes and a mouth for the expression (0 happy, 1 neutral, 2 cross).
pub fn face(c: &mut Canvas, cx: f32, cy: f32, r: f32, head: u8, e: usize, female: bool) {
    const SKINS: [Col; 4] = [rgb(244, 206, 172), rgb(212, 158, 112), rgb(160, 108, 70), rgb(108, 72, 50)];
    const HAIR: [Col; 6] = [rgb(90, 60, 35), rgb(35, 30, 30), rgb(222, 186, 106), rgb(168, 72, 36), rgb(176, 176, 176), rgb(120, 82, 50)];
    let skin = SKINS[(head as usize * 7 / 3) % 4];
    let hair = HAIR[(head as usize * 5) % 6];
    let long = female;
    if long {
        c.round_rect(cx - r * 0.95, cy - r * 0.9, r * 1.9, r * 1.75, r * 0.7, shade(hair, -0.15));
    }
    // ears, face
    c.circle(cx - r * 0.8, cy + r * 0.05, r * 0.18, shade(skin, -0.1));
    c.circle(cx + r * 0.8, cy + r * 0.05, r * 0.18, shade(skin, -0.1));
    c.fill(
        (cx - r, cy - r, cx + r, cy + r),
        |x, y| sd_ellipse(x, y, cx, cy, r * 0.78, r * 0.9),
        |x, y| shade(skin, 0.1 - ((x - cx) * 0.3 + (y - cy) * 0.5) / r * 0.15),
    );
    // hair on top
    let hair_h = if head.is_multiple_of(3) && !female { 0.25 } else { 0.42 };
    c.fill(
        (cx - r, cy - r, cx + r, cy),
        |x, y| sd_ellipse(x, y, cx, cy - r * 0.45, r * 0.82, r * 0.55).max(y - (cy - r * (0.9 - hair_h * 1.6))),
        |_, _| hair,
    );
    // eyes
    let ey = cy - r * 0.08;
    for s in [-1.0f32, 1.0] {
        let ex = cx + s * r * 0.3;
        c.ellipse(ex, ey, r * 0.13, r * 0.15, rgb(255, 255, 255));
        c.circle(ex, ey + r * 0.02, r * 0.07, rgb(40, 40, 50));
        // brows: raised when happy, angled when cross
        let tilt = match e {
            2 => s * -0.12,
            0 => 0.06,
            _ => 0.0,
        } * r;
        c.line(ex - r * 0.14, ey - r * 0.24 + tilt, ex + r * 0.14, ey - r * 0.24 - tilt, r * 0.05, shade(hair, -0.2));
    }
    // nose and mouth
    c.line(cx, cy + r * 0.0, cx + r * 0.05, cy + r * 0.2, r * 0.05, shade(skin, -0.25));
    let my = cy + r * 0.45;
    let curve = match e {
        0 => 0.16,
        2 => -0.12,
        _ => 0.0,
    };
    for k in 0..12 {
        let u = k as f32 / 11.0 * 2.0 - 1.0;
        let (x0, y0) = (cx + u * r * 0.3, my + (1.0 - u * u) * curve * r);
        c.circle(x0, y0, r * 0.045, rgb(150, 50, 50));
    }
    let _ = female;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_fill_their_boxes() {
        let mut c = Canvas::new(40, 30, [0.0; 4]);
        element(&mut c, &Comp { x: 5.0, y: 5.0, w: 30.0, h: 20.0, shape: b'E', tone: b'b', light: 1 });
        assert!(c.get(20, 15)[3] > 0.99);
        assert!(c.get(1, 1)[3] < 0.01);
    }
}
