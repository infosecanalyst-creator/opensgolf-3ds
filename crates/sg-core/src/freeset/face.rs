//! The golfers' faces in the free set: a cast of 19 club members of each sex drawn as cartoon caricatures, each in three
//! expressions (0 happy, 1 neutral, 2 cross). A member is a head shape, a skin and a hairstyle with the features that make
//! them who they are (a nose, eyes, brows, the odd moustache, beard, pair of glasses, hat or earrings), all in flat colours
//! with an ink outline and a cel shadow on the lower right. The shapes are signed distances in units of the face radius r
//! about the face's centre (u to the right, v down), so one drawing serves the halo page and the smaller expressions page.
use super::paint::{alpha, mix, noise, rgb, sd_ellipse, sd_poly, sd_rect, sd_segment, shade, Canvas, Col};

const INK: Col = rgb(62, 38, 34);
const SKINS: [Col; 6] =
    [rgb(252, 224, 198), rgb(244, 200, 164), rgb(224, 170, 126), rgb(194, 138, 96), rgb(152, 102, 68), rgb(110, 72, 50)];
const IRIS: [Col; 5] = [rgb(70, 130, 200), rgb(84, 150, 92), rgb(128, 84, 50), rgb(150, 116, 58), rgb(72, 50, 40)];
const BLACK: Col = rgb(44, 36, 40);
const DARK: Col = rgb(86, 56, 36);
const BROWN: Col = rgb(140, 92, 52);
const AUBURN: Col = rgb(164, 70, 40);
const GINGER: Col = rgb(222, 118, 50);
const BLONDE: Col = rgb(240, 200, 104);
const PLATINUM: Col = rgb(248, 234, 186);
const GREY: Col = rgb(166, 166, 172);
const WHITE: Col = rgb(240, 240, 244);
const LILAC: Col = rgb(200, 172, 226);
const PINK: Col = rgb(240, 104, 160);
const RED_LIPS: Col = rgb(204, 44, 60);
const GOLD: Col = rgb(236, 190, 64);

/// Hairstyles: each has a front (over the head) and some a back (behind it).
#[derive(Clone, Copy, PartialEq)]
enum Hair {
    Bald,
    /// Bald on top with a fringe round the sides.
    Fringe,
    /// The fringe with three brave strands across the top.
    CombOver,
    Buzz,
    Side,
    Quiff,
    Curls,
    /// Tight curls, the older ladies' set.
    Perm,
    Flat,
    Spiky,
    Slick,
    /// A middle parting.
    Curtains,
    Bowl,
    Shaggy,
    Mullet,
    /// A short swept pixie cut.
    Crop,
    Bob,
    Long,
    Pony,
    Bun,
    Bunches,
    Beehive,
    Afro,
    /// Long big curls.
    Glam,
}
use Hair::*;

/// One member of the cast.
#[derive(Clone, Copy)]
struct Look {
    /// 0 round, 1 long, 2 square-jawed, 3 pear (jowly), 4 heart (pointed chin), 5 wide, 6 egg.
    shape: u8,
    skin: u8,
    hair: Col,
    style: Hair,
    /// 0 thin, 1 plain, 2 bushy, 3 one brow across.
    brow: u8,
    /// 0 button, 1 red bulb, 2 long, 3 hook, 4 a line, 5 upturned, 6 round.
    nose: u8,
    /// 0 big and round, 1 beady, 2 big and round but always screwed shut when happy.
    eyes: u8,
    /// 0 hidden by hair, 1 plain, 2 sticking out.
    ears: u8,
    /// 0 none, 1 round, 2 thick square, 3 half-moon reading glasses, 4 sunglasses.
    specs: u8,
    frame: Col,
    /// 0 none, 1 walrus, 2 handlebar, 3 pencil, 4 brush.
    tache: u8,
    /// 0 none, 1 full, 2 goatee, 3 stubble.
    beard: u8,
    /// 0 none, 1 visor, 2 cap, 3 flat cap, 4 headband, 5 sunglasses pushed up.
    hat: u8,
    hat_col: Col,
    /// 0 none, 1 gold hoops, 2 pearls, 3 studs.
    rings: u8,
    lips: Option<Col>,
    /// 0 plain, 1 a gap, 2 buck teeth, 3 braces.
    teeth: u8,
    freckles: bool,
    rosy: bool,
    old: bool,
    lashes: bool,
    mole: bool,
    cleft: bool,
    /// The happy (0 open grin, 1 closed smile, 2 toothy), neutral (0 slight smile, 1 flat, 2 smirk, 3 pursed) and cross
    /// (0 frown, 1 gritted teeth, 2 shouting) mouths.
    mouths: [u8; 3],
}

const L: Look = Look {
    shape: 0,
    skin: 1,
    hair: BROWN,
    style: Side,
    brow: 1,
    nose: 0,
    eyes: 0,
    ears: 1,
    specs: 0,
    frame: rgb(52, 40, 40),
    tache: 0,
    beard: 0,
    hat: 0,
    hat_col: rgb(210, 60, 50),
    rings: 0,
    lips: None,
    teeth: 0,
    freckles: false,
    rosy: false,
    old: false,
    lashes: false,
    mole: false,
    cleft: false,
    mouths: [0, 0, 0],
};

/// The women of the club.
const WOMEN: [Look; 19] = [
    Look { shape: 0, skin: 0, hair: GINGER, style: Curls, freckles: true, rosy: true, lashes: true, ..L },
    Look {
        shape: 1,
        skin: 2,
        hair: BLACK,
        style: Bob,
        ears: 0,
        nose: 4,
        rings: 1,
        lips: Some(RED_LIPS),
        lashes: true,
        mouths: [2, 3, 1],
        ..L
    },
    Look {
        shape: 3,
        skin: 0,
        hair: LILAC,
        style: Beehive,
        nose: 2,
        eyes: 1,
        specs: 3,
        frame: GOLD,
        rings: 2,
        rosy: true,
        old: true,
        mouths: [1, 1, 0],
        ..L
    },
    Look {
        shape: 4,
        skin: 1,
        hair: BLONDE,
        style: Pony,
        hat: 1,
        hat_col: rgb(40, 170, 170),
        freckles: true,
        lashes: true,
        mouths: [0, 0, 1],
        ..L
    },
    Look {
        shape: 5,
        skin: 5,
        hair: BLACK,
        style: Afro,
        ears: 0,
        nose: 6,
        rings: 1,
        lips: Some(rgb(150, 50, 80)),
        lashes: true,
        mouths: [2, 2, 2],
        ..L
    },
    Look { shape: 6, skin: 1, hair: BROWN, style: Bun, nose: 4, specs: 1, mole: true, lashes: true, mouths: [1, 1, 0], ..L },
    Look { shape: 1, skin: 0, hair: GREY, style: Crop, nose: 3, eyes: 2, rings: 2, rosy: true, old: true, mouths: [1, 2, 1], ..L },
    Look { shape: 6, skin: 3, hair: BLACK, style: Long, ears: 0, nose: 4, specs: 4, lips: Some(RED_LIPS), mouths: [2, 3, 0], ..L },
    Look {
        shape: 0,
        skin: 1,
        hair: AUBURN,
        style: Long,
        ears: 0,
        hat: 2,
        hat_col: rgb(214, 58, 52),
        freckles: true,
        lashes: true,
        mouths: [0, 0, 2],
        ..L
    },
    Look {
        shape: 4,
        skin: 0,
        hair: PLATINUM,
        style: Glam,
        ears: 0,
        eyes: 2,
        rings: 2,
        lips: Some(rgb(226, 70, 120)),
        lashes: true,
        mole: true,
        mouths: [1, 3, 1],
        ..L
    },
    Look { shape: 0, skin: 2, hair: DARK, style: Bunches, nose: 5, teeth: 3, freckles: true, lashes: true, ..L },
    Look {
        shape: 1,
        skin: 1,
        hair: BROWN,
        style: Bob,
        ears: 0,
        nose: 2,
        eyes: 1,
        specs: 2,
        frame: rgb(200, 50, 64),
        mouths: [1, 1, 1],
        ..L
    },
    Look { shape: 1, skin: 4, hair: BLACK, style: Crop, rings: 1, lips: Some(rgb(170, 60, 70)), lashes: true, mouths: [2, 2, 2], ..L },
    Look { shape: 3, skin: 1, hair: WHITE, style: Perm, nose: 6, specs: 1, frame: rgb(120, 70, 150), rosy: true, old: true, ..L },
    Look { shape: 4, skin: 3, hair: DARK, style: Bun, hat: 1, hat_col: rgb(240, 120, 170), rings: 3, lashes: true, mouths: [2, 0, 1], ..L },
    Look {
        shape: 6,
        skin: 0,
        hair: rgb(232, 146, 90),
        style: Long,
        ears: 0,
        nose: 5,
        hat: 4,
        hat_col: rgb(90, 190, 90),
        freckles: true,
        lashes: true,
        mouths: [0, 2, 2],
        ..L
    },
    Look { shape: 2, skin: 2, hair: GREY, style: Bob, ears: 0, hat: 5, lips: Some(rgb(200, 80, 90)), lashes: true, mouths: [1, 2, 0], ..L },
    Look { shape: 5, skin: 1, hair: PINK, style: Spiky, rings: 3, lashes: true, mouths: [2, 2, 2], ..L },
    Look { shape: 2, skin: 4, hair: BROWN, style: Pony, nose: 6, eyes: 1, hat: 2, hat_col: rgb(50, 72, 140), mouths: [0, 1, 1], ..L },
];

/// The men of the club.
const MEN: [Look; 19] = [
    Look {
        shape: 3,
        skin: 1,
        hair: WHITE,
        style: Fringe,
        brow: 2,
        nose: 1,
        eyes: 1,
        tache: 1,
        rosy: true,
        old: true,
        mouths: [0, 0, 1],
        ..L
    },
    Look { shape: 2, skin: 1, hair: BLACK, style: Quiff, brow: 0, nose: 2, cleft: true, mouths: [2, 2, 1], ..L },
    Look { shape: 0, skin: 0, hair: GINGER, style: Buzz, ears: 2, freckles: true, teeth: 1, ..L },
    Look { shape: 1, skin: 5, hair: BLACK, style: Flat, nose: 6, eyes: 2, beard: 2, mouths: [2, 1, 1], ..L },
    Look { shape: 6, skin: 1, hair: BROWN, style: Bowl, nose: 2, specs: 1, teeth: 2, mouths: [1, 1, 0], ..L },
    Look {
        shape: 3,
        skin: 0,
        hair: GREY,
        style: Slick,
        brow: 2,
        nose: 3,
        eyes: 1,
        hat: 3,
        hat_col: rgb(150, 118, 80),
        old: true,
        mouths: [1, 2, 0],
        ..L
    },
    Look { shape: 1, skin: 2, hair: BLONDE, style: Shaggy, ears: 0, nose: 4, specs: 4, beard: 3, mouths: [2, 2, 1], ..L },
    Look { shape: 0, skin: 2, hair: DARK, style: Curls, brow: 2, nose: 6, eyes: 1, beard: 1, mouths: [0, 0, 2], ..L },
    Look { shape: 2, skin: 1, hair: BROWN, style: Side, hat: 2, hat_col: rgb(214, 58, 52), tache: 3, mouths: [2, 1, 1], ..L },
    Look { shape: 3, skin: 0, hair: GREY, style: CombOver, nose: 1, eyes: 1, specs: 3, frame: GOLD, old: true, mouths: [1, 1, 0], ..L },
    Look { shape: 0, skin: 4, hair: BLACK, style: Curls, nose: 6, mouths: [2, 0, 2], ..L },
    Look { shape: 1, skin: 0, hair: WHITE, style: Slick, brow: 2, nose: 2, eyes: 1, tache: 4, beard: 2, old: true, mouths: [1, 1, 1], ..L },
    Look { shape: 4, skin: 1, hair: AUBURN, style: Spiky, teeth: 3, freckles: true, mouths: [0, 2, 2], ..L },
    Look { shape: 2, skin: 5, hair: GREY, style: Bald, nose: 6, beard: 1, rings: 3, mouths: [2, 1, 1], ..L },
    Look { shape: 6, skin: 1, hair: GINGER, style: Side, nose: 2, hat: 1, hat_col: rgb(70, 160, 80), tache: 2, mouths: [0, 2, 0], ..L },
    Look { shape: 6, skin: 2, hair: DARK, style: Curtains, nose: 0, eyes: 2, ears: 2, teeth: 2, mouths: [1, 1, 2], ..L },
    Look { shape: 2, skin: 1, hair: GREY, style: Side, nose: 2, specs: 2, beard: 3, mouths: [2, 1, 1], ..L },
    Look {
        shape: 1,
        skin: 1,
        hair: BROWN,
        style: Long,
        ears: 0,
        eyes: 2,
        specs: 1,
        frame: GOLD,
        tache: 1,
        hat: 4,
        hat_col: rgb(230, 150, 50),
        mouths: [1, 0, 0],
        ..L
    },
    Look { shape: 5, skin: 3, hair: BLACK, style: Mullet, brow: 3, nose: 1, tache: 4, mouths: [0, 2, 2], ..L },
];

/// Faces: the halo page (140 x 140 cells, two heads to a column, three expressions each) or the expressions page (90 x
/// 120 cells, 100 apart, rows 372 apart).
pub(super) fn faces(c: &mut Canvas, name: &str, female: bool) {
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

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

fn circ(u: f32, v: f32, cu: f32, cv: f32, rad: f32) -> f32 {
    ((u - cu).powi(2) + (v - cv).powi(2)).sqrt() - rad
}

/// The head of a shape.
fn head_sd(shape: u8, u: f32, v: f32) -> f32 {
    match shape {
        1 => sd_ellipse(u, v, 0.0, 0.0, 0.66, 0.94),
        2 => smin(sd_ellipse(u, v, 0.0, -0.2, 0.76, 0.7), sd_rect(u, v, -0.68, -0.1, 1.36, 1.0, 0.3), 0.15),
        3 => smin(sd_ellipse(u, v, 0.0, -0.26, 0.62, 0.62), sd_ellipse(u, v, 0.0, 0.34, 0.86, 0.56), 0.2),
        4 => smin(sd_ellipse(u, v, 0.0, -0.1, 0.8, 0.72), sd_ellipse(u, v, 0.0, 0.38, 0.5, 0.5), 0.25),
        5 => sd_ellipse(u, v, 0.0, 0.06, 0.9, 0.78),
        6 => smin(sd_ellipse(u, v, 0.0, -0.22, 0.64, 0.68), sd_ellipse(u, v, 0.0, 0.26, 0.74, 0.6), 0.2),
        _ => sd_ellipse(u, v, 0.0, 0.0, 0.8, 0.86),
    }
}

/// Where `f` crosses zero between a (inside, negative) and b (outside), by halving.
fn edge(f: impl Fn(f32) -> f32, mut a: f32, mut b: f32) -> f32 {
    for _ in 0..16 {
        let m = (a + b) / 2.0;
        if f(m) < 0.0 {
            a = m;
        } else {
            b = m;
        }
    }
    (a + b) / 2.0
}

/// Points along a parabola from u0 to u1: v = v0 + bend (1 - t^2) + tilt t, t from -1 to 1 (bend > 0 sags the middle).
fn curve(u0: f32, u1: f32, v0: f32, bend: f32, tilt: f32) -> Vec<(f32, f32)> {
    (0..=8)
        .map(|k| {
            let t = k as f32 / 4.0 - 1.0;
            (u0 + (u1 - u0) * (t + 1.0) / 2.0, v0 + bend * (1.0 - t * t) + tilt * t)
        })
        .collect()
}

/// A closed shape between a top and a bottom edge that meet at u = mu -+ w.
fn lens(mu: f32, w: f32, top: impl Fn(f32) -> f32, bot: impl Fn(f32) -> f32) -> Vec<(f32, f32)> {
    let mut p: Vec<(f32, f32)> = (0..=8).map(|k| k as f32 / 4.0 - 1.0).map(|t| (mu + t * w, top(t))).collect();
    p.extend((1..8).rev().map(|k| k as f32 / 4.0 - 1.0).map(|t| (mu + t * w, bot(t))));
    p
}

/// Circles of radius `rad` round an ellipse (centre cu, cv, radii ru, rv) from angle a0 to a1 (degrees, anticlockwise from
/// the right as seen): a curly outline.
#[allow(clippy::too_many_arguments)]
fn ring(n: usize, cu: f32, cv: f32, ru: f32, rv: f32, a0: f32, a1: f32, rad: f32) -> Vec<(f32, f32, f32)> {
    (0..n)
        .map(|k| {
            let a = (a0 + (a1 - a0) * k as f32 / (n - 1) as f32).to_radians();
            (cu + ru * a.cos(), cv - rv * a.sin(), rad * (1.0 - 0.12 * (k % 2) as f32))
        })
        .collect()
}

fn blobs(u: f32, v: f32, b: &[(f32, f32, f32)]) -> f32 {
    b.iter().fold(9.0, |d, &(cu, cv, rad)| d.min(circ(u, v, cu, cv, rad)))
}

/// A face being drawn: the canvas, the centre and radius, and the outline width.
struct Pen<'a> {
    c: &'a mut Canvas,
    cx: f32,
    cy: f32,
    r: f32,
    lw: f32,
}

impl Pen<'_> {
    fn px(&self, u: f32, v: f32) -> (f32, f32) {
        (self.cx + u * self.r, self.cy + v * self.r)
    }

    fn bx(&self, b: [f32; 4], pad: f32) -> (f32, f32, f32, f32) {
        let (x0, y0) = self.px(b[0], b[1]);
        let (x1, y1) = self.px(b[2], b[3]);
        (x0 - pad, y0 - pad, x1 + pad, y1 + pad)
    }

    /// Fills the shape `sd` (in pixels, negative inside) within box b with the colour `col` gives: Canvas::fill, but by
    /// tiles, 8 x 8 then 2 x 2, asking the shape only near its edge (the shapes here are mostly empty boxes or flat
    /// insides). With `shadow` (a colour and an offset) the pixels whose offset place is outside the shape take that
    /// colour: a cel shadow along the edges the offset points to. The tiles' margins allow for distances a little long.
    fn raster(
        &mut self,
        b: (f32, f32, f32, f32),
        sd: impl Fn(f32, f32) -> f32,
        col: impl Fn(f32, f32) -> Col,
        shadow: Option<(Col, f32, f32)>,
    ) {
        let (w, h) = (self.c.w as i32, self.c.h as i32);
        let (x0, y0) = ((b.0.floor() as i32).max(0), (b.1.floor() as i32).max(0));
        let (x1, y1) = ((b.2.ceil() as i32).min(w - 1), (b.3.ceil() as i32).min(h - 1));
        // a tile of size n at (tx, ty): its distance and its shadow's, at its centre
        let at = |tx: i32, ty: i32, n: i32| {
            let (mx, my) = (tx as f32 + n as f32 / 2.0, ty as f32 + n as f32 / 2.0);
            (sd(mx, my), shadow.map_or(-99.0, |(_, ox, oy)| sd(mx + ox, my + oy)))
        };
        let (far8, far2) = (8.0, 3.2);
        for ty in (y0..=y1).step_by(8) {
            for tx in (x0..=x1).step_by(8) {
                let (d8, s8) = at(tx, ty, 8);
                if d8 > far8 {
                    continue;
                }
                for sy in (ty..(ty + 8).min(y1 + 1)).step_by(2) {
                    for sx in (tx..(tx + 8).min(x1 + 1)).step_by(2) {
                        let (d2, s2) = if d8 < -far8 && s8.abs() > far8 { (d8, s8) } else { at(sx, sy, 2) };
                        if d2 > far2 {
                            continue;
                        }
                        for y in sy..(sy + 2).min(y1 + 1) {
                            for x in sx..(sx + 2).min(x1 + 1) {
                                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                                let cov = if d2 < -far2 { 1.0 } else { (0.5 - sd(px, py)).clamp(0.0, 1.0) };
                                if cov <= 0.0 {
                                    continue;
                                }
                                let c = match shadow {
                                    Some((dark, _, _)) if s2 > far2 => dark,
                                    Some((dark, ox, oy)) if s2 > -far2 => mix(col(px, py), dark, sd(px + ox, py + oy) + 0.5),
                                    _ => col(px, py),
                                };
                                self.c.blend(x, y, c, cov);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Shape `sd` (unit coordinates, within box b) in `col` with an outline in `ink`; with `sh` a cel shadow on its lower
    /// right.
    fn shape_in(&mut self, b: [f32; 4], sd: impl Fn(f32, f32) -> f32, col: Col, sh: Option<Col>, ink: Col) {
        let (cx, cy, r, lw) = (self.cx, self.cy, self.r, self.lw);
        let s = |x: f32, y: f32| sd((x - cx) / r, (y - cy) / r) * r;
        // the outline is a ring under the shape's edge, so the inside is painted once
        let b = self.bx(b, lw + 2.0);
        self.raster(
            b,
            |x, y| {
                let d = s(x, y);
                (d - lw).max(-d - 1.5)
            },
            |_, _| ink,
            None,
        );
        self.raster(b, s, |_, _| col, sh.map(|d| (d, r * 0.11, r * 0.08)));
    }

    fn shape(&mut self, b: [f32; 4], sd: impl Fn(f32, f32) -> f32, col: Col, sh: Option<Col>) {
        self.shape_in(b, sd, col, sh, INK);
    }

    /// Shape `sd` in `col` without an outline; `col` may vary with the place (unit coordinates).
    fn flat(&mut self, b: [f32; 4], sd: impl Fn(f32, f32) -> f32, col: Col) {
        self.tint(b, sd, |_, _| col);
    }

    fn tint(&mut self, b: [f32; 4], sd: impl Fn(f32, f32) -> f32, col: impl Fn(f32, f32) -> Col) {
        let (cx, cy, r) = (self.cx, self.cy, self.r);
        self.raster(self.bx(b, 2.0), |x, y| sd((x - cx) / r, (y - cy) / r) * r, col, None);
    }

    /// A line through points (unit coordinates), `w` pixels wide.
    fn stroke(&mut self, pts: &[(f32, f32)], w: f32, col: Col) {
        let p: Vec<(f32, f32)> = pts.iter().map(|&(u, v)| self.px(u, v)).collect();
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in &p {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        let h = w / 2.0;
        self.raster(
            (x0 - h - 1.0, y0 - h - 1.0, x1 + h + 1.0, y1 + h + 1.0),
            |x, y| p.windows(2).fold(f32::MAX, |d, s| d.min(sd_segment(x, y, s[0].0, s[0].1, s[1].0, s[1].1))) - h,
            |_, _| col,
            None,
        );
    }

    fn dot(&mut self, u: f32, v: f32, rad: f32, col: Col) {
        let (x, y) = self.px(u, v);
        self.c.circle(x, y, (rad * self.r).max(0.8), col);
    }
}

/// A cartoon face at (cx, cy) of radius r: member `head` of the women or the men in expression `e` (0 happy, 1 neutral, 2
/// cross).
pub fn face(c: &mut Canvas, cx: f32, cy: f32, r: f32, head: u8, e: usize, female: bool) {
    let l = if female { WOMEN[head as usize % 19] } else { MEN[head as usize % 19] };
    let mut p = Pen { c, cx, cy, r, lw: (r * 0.045).max(1.3) };
    let sh = l.shape;
    let hs = move |u: f32, v: f32| head_sd(sh, u, v);
    let skin = SKINS[l.skin as usize];
    let skin_sh = mix(shade(skin, -0.16), rgb(200, 70, 80), 0.1);
    let hair = l.hair;
    let hair_sh = shade(hair, -0.24);
    // the head's half width at the eyes, its top and its chin
    let hw = edge(|u| hs(u, 0.0), 0.0, 1.2);
    let top = -edge(|v| hs(0.0, -v), 0.0, 1.4);
    let chin = edge(|v| hs(0.0, v), 0.0, 1.4);
    let all = [-1.25, -1.4, 1.25, 1.3];
    let side = if head.is_multiple_of(2) { 1.0 } else { -1.0 };

    // ---- behind the head: long hair, buns, ponytails
    let back = back_hair(l.style, side);
    if let Some(sd) = &back {
        p.shape(all, sd, hair, Some(hair_sh));
        if l.style == Pony {
            p.dot(side * 0.68, -0.32, 0.075, l.hat_col);
        }
        if l.style == Bunches {
            for s in [-1.0, 1.0] {
                p.dot(s * 0.74, -0.05, 0.07, rgb(230, 80, 120));
            }
        }
    }

    // ---- ears, the head and its shading
    let ear_u = hw + if l.ears == 2 { 0.06 } else { -0.02 };
    let (ear_ru, ear_rv) = if l.ears == 2 { (0.2, 0.25) } else { (0.15, 0.2) };
    if l.ears > 0 {
        for s in [-1.0f32, 1.0] {
            p.shape(
                [s * ear_u - 0.3, -0.3, s * ear_u + 0.3, 0.4],
                |u, v| sd_ellipse(u, v, s * ear_u, 0.04, ear_ru, ear_rv),
                skin,
                Some(skin_sh),
            );
            p.stroke(&curve(s * (ear_u - ear_ru * 0.1), s * (ear_u + ear_ru * 0.25), -0.06, 0.12, 0.0), p.lw * 0.9, skin_sh);
        }
    }
    p.shape(all, hs, skin, Some(skin_sh));
    if l.shape == 3 {
        p.stroke(&curve(-0.26, 0.26, chin - 0.17, 0.06, 0.0), p.lw * 0.9, skin_sh);
    }
    if l.cleft {
        p.stroke(&[(0.0, chin - 0.13), (0.0, chin - 0.05)], p.lw * 0.9, skin_sh);
    }

    // ---- the jaw's hair
    let beard_top = |u: f32| 0.3 - 0.55 * u * u;
    match l.beard {
        1 => {
            let sd = move |u: f32, v: f32| (hs(u, v) - 0.06).max(beard_top(u) - v).min(sd_ellipse(u, v, 0.0, chin - 0.04, 0.48, 0.26));
            p.shape(all, sd, hair, Some(hair_sh));
        }
        3 => {
            let sd = move |u: f32, v: f32| hs(u, v).max(beard_top(u) + 0.08 - v);
            p.tint(
                all,
                |u, v| sd(u, v) + 0.01,
                |x, y| {
                    let n = noise(x as i32, y as i32, 7);
                    alpha(mix(skin, hair_sh, 0.6), if n > 0.62 { 0.7 } else { 0.22 })
                },
            );
        }
        _ => {}
    }

    // ---- eyes' place and size
    let ev = -0.06;
    let es = (hw * 0.42).max(0.27);
    let (erx, ery) = match l.eyes {
        1 => (0.07, 0.08),
        _ => (0.13, 0.155),
    };
    let mv = (chin * 0.56).max(0.44);
    let mw = (hw * 0.38).clamp(0.24, 0.34);

    // ---- cheeks, freckles, lines, the cross flush
    let blush = if e == 0 || l.rosy { 0.45 } else { 0.0 };
    if blush > 0.0 {
        for s in [-1.0f32, 1.0] {
            let cu = s * (es + 0.1);
            p.flat(
                [cu - 0.15, ev + 0.15, cu + 0.15, ev + 0.4],
                |u, v| sd_ellipse(u, v, cu, ev + 0.28, 0.13, 0.075),
                alpha(rgb(240, 110, 120), blush),
            );
        }
    }
    if l.freckles {
        let fc = mix(skin, rgb(170, 84, 40), 0.55);
        for s in [-1.0f32, 1.0] {
            for (du, dv) in [(0.0, 0.0), (0.07, 0.04), (0.13, -0.01), (0.05, -0.05), (0.11, 0.07), (0.17, 0.04)] {
                p.dot(s * (es - 0.06 + du), ev + 0.22 + dv, 0.021, fc);
            }
        }
    }
    let line_col = shade(skin, -0.3);
    if l.old {
        for s in [-1.0f32, 1.0] {
            let (ou, w) = (s * (es + erx + 0.04), p.lw * 0.8);
            p.stroke(&[(ou, ev - 0.02), (ou + s * 0.08, ev - 0.07)], w, line_col);
            p.stroke(&[(ou, ev + 0.03), (ou + s * 0.08, ev + 0.07)], w, line_col);
            p.stroke(&curve(s * 0.15, s * (mw + 0.08), mv - 0.18, 0.0, 0.1), w, line_col);
        }
        for k in 0..2 {
            p.stroke(&curve(-0.24, 0.24, top + 0.32 + k as f32 * 0.08, -0.03, 0.0), p.lw * 0.8, line_col);
        }
    }
    if l.mole {
        p.dot(side * (mw + 0.04), mv - 0.12, 0.024, rgb(90, 50, 40));
    }
    if e == 2 {
        let (cy, r) = (p.cy, p.r);
        p.tint(all, hs, |_, y| alpha(rgb(230, 40, 40), 0.08 + 0.22 * ((0.4 - (y - cy) / r) / 1.2).clamp(0.0, 1.0)));
    }

    // ---- the hair over the head
    front_hair(&mut p, &l, hs, top, hw, side);

    // ---- eyes and brows
    let iris = if l.skin >= 3 { IRIS[2 + head as usize % 3] } else { IRIS[(head as usize * 3 + female as usize) % 5] };
    for s in [-1.0f32, 1.0] {
        let eu = s * es;
        if l.specs != 4 {
            eye(&mut p, &l, e, head, eu, ev, erx, ery, s, iris);
        }
        // brows: raised when happy, the inner ends down when cross
        let bv = ev - if l.eyes == 1 { 0.18 } else { 0.27 };
        let (din, dout, arch) = match e {
            0 => (-0.05, -0.02, -0.05),
            2 => (0.09, -0.06, 0.0),
            _ => (0.0, 0.0, -0.035),
        };
        let (i, o) = ((eu - s * 0.13, bv + din), (eu + s * 0.15, bv + dout));
        let m = ((i.0 + o.0) / 2.0, (i.1 + o.1) / 2.0 + arch);
        let w = p.lw * [1.4, 2.3, 3.8, 3.0][l.brow as usize];
        let bc = if matches!(l.style, Bald) { rgb(40, 30, 30) } else { shade(hair, if luma(hair) > 170.0 { -0.12 } else { -0.3 }) };
        p.stroke(&[i, m, o], w, bc);
        if l.brow == 2 {
            p.stroke(&[(o.0 - s * 0.04, o.1 - 0.02), (o.0 + s * 0.03, o.1 - 0.05)], w * 0.6, bc);
        }
        if l.brow == 3 {
            p.stroke(&[i, (0.0, (i.1 + 0.02).min(bv + 0.06))], w, bc);
        }
    }

    // ---- the mouth (on a patch of lip through a full beard), then the moustache and the nose over it
    if l.beard == 1 {
        p.flat([-0.4, mv - 0.2, 0.4, mv + 0.3], |u, v| sd_ellipse(u, v, 0.0, mv + 0.04, mw * 0.95, 0.15), shade(skin, -0.1));
    }
    mouth(&mut p, &l, e, mv, mw, side);
    tache(&mut p, &l, e, mv, chin, hair, hair_sh);
    nose(&mut p, &l, skin, skin_sh, side);

    // ---- glasses
    specs(&mut p, &l, es, ev, hw);

    // ---- hats
    hat(&mut p, &l, hs, top, hw);

    // ---- earrings
    let lobe = (ear_u.min(hw + 0.02), 0.24);
    for s in [-1.0f32, 1.0] {
        let (u, v) = (s * lobe.0, lobe.1);
        match l.rings {
            1 => {
                let (ru, rv) = (0.1, 0.12);
                p.flat([u - 0.2, v, u + 0.2, v + 0.35], |x, y| sd_ellipse(x, y, u, v + rv, ru, rv).abs() - 0.04, INK);
                p.flat([u - 0.2, v, u + 0.2, v + 0.35], |x, y| sd_ellipse(x, y, u, v + rv, ru, rv).abs() - 0.02, GOLD);
            }
            2 => {
                p.shape(
                    [u - 0.1, v - 0.05, u + 0.1, v + 0.15],
                    |x, y| circ(x, y, u, v + 0.05, 0.05),
                    rgb(250, 248, 240),
                    Some(rgb(200, 196, 200)),
                );
            }
            3 => {
                p.shape([u - 0.1, v - 0.05, u + 0.1, v + 0.1], |x, y| circ(x, y, u, v + 0.02, 0.035), GOLD, None);
            }
            _ => {}
        }
    }

    // ---- a popping anger vein on a hot head: four arcs, each bowed in towards the middle
    if e == 2 && head % 3 == 1 {
        let (au, av) = (-side * 0.5, top + 0.32);
        let (q, w) = (0.035, 0.16);
        for (a, b) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            // the arc round the quadrant's outer corner, from one gap to the next
            let (cu, cv, rad) = (au + a * w, av + b * w, w - q);
            let pts: Vec<(f32, f32)> = (0..=6)
                .map(|k| (k as f32 / 6.0 * std::f32::consts::FRAC_PI_2).sin_cos())
                .map(|(sn, cs)| (cu - a * rad * cs, cv - b * rad * sn))
                .collect();
            p.stroke(&pts, p.lw * 2.8, rgb(255, 255, 255));
            p.stroke(&pts, p.lw * 1.5, rgb(196, 28, 36));
        }
    }
}

impl Look {
    fn skin_col(&self) -> Col {
        SKINS[self.skin as usize]
    }
}

fn luma(c: Col) -> f32 {
    c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11
}

/// The hair behind the head of a style, if it has any.
#[allow(clippy::type_complexity)]
fn back_hair(style: Hair, side: f32) -> Option<Box<dyn Fn(f32, f32) -> f32>> {
    Some(match style {
        Bob => Box::new(|u, v| smin(sd_ellipse(u, v, 0.0, -0.16, 0.98, 0.86), sd_ellipse(u, v, 0.0, 0.3, 0.96, 0.24), 0.12)),
        Long => Box::new(|u, v| {
            // straight down to the shoulders, the ends a little ragged
            let ends = 0.03 * (u * 16.0).sin().abs();
            smin(sd_ellipse(u, v, 0.0, -0.14, 0.96, 0.88), sd_rect(u, v, -0.9, -0.2, 1.8, 1.08 - ends, 0.3), 0.12)
        }),
        Pony => {
            let tail =
                [(0.66, -0.34, 0.15), (0.84, -0.18, 0.17), (0.94, 0.04, 0.17), (0.96, 0.26, 0.15), (0.9, 0.48, 0.12), (0.8, 0.64, 0.08)];
            Box::new(move |u, v| blobs(u * side, v, &tail))
        }
        Bun => Box::new(|u, v| circ(u, v, 0.0, -0.98, 0.27)),
        Bunches => {
            let b = [(-0.92, 0.06, 0.24), (-0.86, 0.3, 0.18), (0.92, 0.06, 0.24), (0.86, 0.3, 0.18)];
            Box::new(move |u, v| blobs(u, v, &b))
        }
        Beehive => Box::new(|u, v| smin(sd_ellipse(u, v, 0.0, -0.66, 0.72, 0.64), sd_ellipse(u, v, 0.0, 0.0, 0.88, 0.5), 0.1)),
        Afro => {
            let b = ring(13, 0.0, -0.16, 0.9, 0.82, -40.0, 220.0, 0.27);
            Box::new(move |u, v| blobs(u, v, &b).min(sd_ellipse(u, v, 0.0, -0.16, 0.92, 0.84)))
        }
        Glam => {
            let b = ring(14, 0.0, -0.06, 0.86, 0.92, -68.0, 248.0, 0.24);
            Box::new(move |u, v| blobs(u, v, &b).min(sd_ellipse(u, v, 0.0, -0.06, 0.88, 0.94)))
        }
        Mullet => Box::new(|u, v| sd_rect(u, v, -0.84, -0.3, 1.68, 1.0, 0.3) + 0.03 * (u * 18.0).sin().abs()),
        Shaggy => Box::new(|u, v| smin(sd_ellipse(u, v, 0.0, -0.16, 0.9, 0.84), sd_rect(u, v, -0.86, -0.2, 1.72, 0.8, 0.3), 0.1)),
        _ => return None,
    })
}

/// The hair over the head: a cap down to a hairline, with each style's own fringe, curls or spikes.
fn front_hair(p: &mut Pen, l: &Look, hs: impl Fn(f32, f32) -> f32 + Copy, top: f32, hw: f32, side: f32) {
    let hair = l.hair;
    let hair_sh = shade(hair, -0.24);
    let all = [-1.25, -1.4, 1.25, 1.3];
    // the cap: the head grown by t, above the hairline hl
    let cap = move |t: f32, hl: &dyn Fn(f32) -> f32, u: f32, v: f32| {
        // (well below the hairline the head need not be asked)
        let below = v - hl(u);
        if below > 0.2 {
            below
        } else {
            below.max(hs(u, v) - t)
        }
    };
    let shine = alpha(rgb(255, 255, 255), 0.45);
    let tufts = move |u: f32, v: f32| {
        let a = u.abs();
        circ(a, v, hw * 0.9, -0.26, 0.12).min(circ(a, v, hw * 0.98, -0.1, 0.14)).min(circ(a, v, hw * 0.94, 0.06, 0.11))
    };
    let gloss = |p: &mut Pen| {
        p.flat(all, |u, v| sd_ellipse(u, v, -0.26, top + 0.18, 0.2, 0.08), shine);
    };
    match l.style {
        Bald => gloss(p),
        Fringe | CombOver => {
            p.shape(all, tufts, hair, Some(hair_sh));
            gloss(p);
            if l.style == CombOver {
                for k in 0..3 {
                    let v = top + 0.14 + k as f32 * 0.09;
                    p.stroke(&curve(-hw * 0.8, hw * 0.8, v + 0.12, -0.14, 0.04), p.lw * 1.2, shade(hair, -0.15));
                }
            }
        }
        Buzz => {
            // cropped close: the hair's colour over the skin, speckled, inside the head's own outline
            let hl = |u: f32| -0.54 + 0.3 * u * u + 0.9 * (u.abs() - 0.5).max(0.0);
            let (lo, hi) = (mix(l.skin_col(), hair, 0.7), hair);
            p.tint(all, move |u, v| cap(-0.02, &hl, u, v), move |x, y| if noise(x as i32, y as i32, 11) > 0.6 { hi } else { lo });
        }
        Side | Slick | Pony | Bun => {
            let hl: &dyn Fn(f32) -> f32 = match l.style {
                Side => &|u: f32| -0.46 + 0.3 * u * u - 0.12 * (u + 0.3).max(0.0) + 0.8 * (u.abs() - 0.55).max(0.0),
                // swept back to a widow's peak
                Slick => &|u: f32| -0.6 + 0.36 * u * u + 0.5 * (0.14 - u.abs()).max(0.0),
                _ => &|u: f32| -0.52 + 0.34 * u * u,
            };
            let lift = if l.style == Side { 0.1 } else { 0.06 };
            p.shape(all, |u, v| cap(lift, hl, u * side, v), hair, Some(hair_sh));
            if l.style == Side {
                let pu = -0.34 * side;
                p.stroke(&[(pu, top - 0.04), (pu - 0.02 * side, top + 0.2)], p.lw, hair_sh);
                p.stroke(&curve(pu + 0.1 * side, pu + 0.5 * side, top + 0.12, -0.04, 0.03), p.lw * 1.6, shade(hair, 0.3));
            } else {
                // combed lines, bold on a slicked head, faint where it is only pulled back
                let (n, k_sh) = if l.style == Slick { (5, -0.32) } else { (3, -0.14) };
                for k in 0..n {
                    let u = (k as f32 - (n - 1) as f32 / 2.0) * 0.14;
                    p.stroke(&[(u * 1.2, top + 0.03), (u, top + 0.26 - 0.1 * u.abs())], p.lw * 0.8, shade(hair, k_sh));
                }
            }
        }
        Quiff => {
            // sideburns and a pompadour: a tilted roll of hair standing up off the brow
            let hl = |u: f32| -0.62 + 0.3 * u * u + 0.06 * u + 0.9 * (u.abs() - 0.5).max(0.0);
            let lock = [(-0.16, -0.66), (0.1, -0.66), (-0.1, -0.44)];
            let sd = move |u: f32, v: f32| {
                let u = u * side;
                let (ru, rv) = (u * 0.95 + (v - top) * 0.3, (v - top) * 0.95 - u * 0.3);
                let roll = sd_ellipse(ru, rv, 0.12, -0.02, 0.56, 0.26).min(sd_poly(u, v, &lock));
                smin(cap(0.06, &hl, u, v), roll, 0.12)
            };
            p.shape(all, sd, hair, Some(hair_sh));
            for k in 0..2 {
                let u0 = (-0.3 + k as f32 * 0.22) * side;
                p.stroke(&[(u0, top + 0.04), (u0 + 0.16 * side, top - 0.1 - k as f32 * 0.04)], p.lw * 1.4, shade(hair, 0.35));
            }
        }
        Curls | Perm => {
            let (n, rad, rv) = if l.style == Perm { (10, 0.22, 0.8) } else { (9, 0.19, 0.84) };
            let b = ring(n, 0.0, -0.08, hw * 1.0, rv, -16.0, 196.0, rad);
            let hl = |u: f32| -0.46 + 0.3 * u * u;
            p.shape(all, |u, v| blobs(u, v, &b).min(cap(0.05, &hl, u, v)), hair, Some(hair_sh));
            for &(u, v, rad) in &b {
                p.stroke(&curve(u - rad * 0.45, u + rad * 0.45, v - rad * 0.1, -rad * 0.4, 0.0), p.lw * 0.9, hair_sh);
            }
        }
        Afro | Glam => {
            let hl: &dyn Fn(f32) -> f32 =
                if l.style == Glam { &|u: f32| -0.42 + 0.3 * u * u + 0.1 * u } else { &|u: f32| -0.48 + 0.3 * u * u };
            if l.style == Glam {
                let sd = |u: f32, v: f32| cap(0.12, hl, u * side, v).min(sd_ellipse(u * side, v, -0.3, top + 0.22, 0.42, 0.2));
                p.shape(all, sd, hair, Some(hair_sh));
                p.stroke(&curve(-0.55 * side, -0.05 * side, top + 0.2, -0.06, 0.0), p.lw * 1.6, shade(hair, 0.4));
            } else {
                p.shape(all, |u, v| cap(0.1, hl, u, v), hair, Some(hair_sh));
                let b = ring(9, 0.0, -0.16, 0.92, 0.84, 20.0, 160.0, 0.2);
                for &(u, v, rad) in &b {
                    p.stroke(&curve(u - rad * 0.4, u + rad * 0.4, v, -rad * 0.4, 0.0), p.lw * 0.9, shade(hair, 0.25));
                }
            }
        }
        Flat => {
            let hl = |u: f32| -0.58 + 0.3 * u * u + 0.9 * (u.abs() - 0.45).max(0.0);
            // straight up from the temples and flat across the top
            let w = hw + 0.03;
            p.shape(all, move |u, v| sd_rect(u, v, -w, top - 0.16, 2.0 * w, 1.2, 0.08).max(v - hl(u)), hair, Some(hair_sh));
        }
        Spiky => {
            let hl = |u: f32| -0.44 + 0.3 * u * u;
            let mut pts: Vec<(f32, f32)> = Vec::new();
            for k in 0..=10 {
                let a = (165.0 - k as f32 * 15.0).to_radians();
                let k_r = if k % 2 == 1 { 1.3 } else { 0.96 };
                pts.push((a.cos() * hw * k_r, -0.08 - a.sin() * (-top - 0.08) * k_r));
            }
            pts.push((hw * 0.6, -0.2));
            pts.push((-hw * 0.6, -0.2));
            let sd = move |u: f32, v: f32| {
                let below = v - hl(u);
                if below > 0.2 {
                    below
                } else {
                    sd_poly(u, v, &pts).min(hs(u, v) - 0.05).max(below)
                }
            };
            p.shape(all, sd, hair, Some(hair_sh));
        }
        Curtains => {
            let hl = |u: f32| (-0.62 + 0.8 * u.abs()).min(-0.12 + 0.05 * u * u);
            p.shape(all, move |u, v| cap(0.1, &hl, u, v), hair, Some(hair_sh));
            p.stroke(&[(0.0, top - 0.06), (0.0, top + 0.2)], p.lw, hair_sh);
        }
        Bowl => {
            let hl = |u: f32| -0.28 + 0.1 * u.powi(4);
            p.shape(all, move |u, v| cap(0.13, &hl, u, v), hair, Some(hair_sh));
        }
        Shaggy | Mullet => {
            let hl: &dyn Fn(f32) -> f32 = if l.style == Shaggy {
                &|u: f32| -0.34 + 0.2 * u * u + 0.08 * ((u * 4.0).fract().abs() - 0.5).abs()
            } else {
                &|u: f32| -0.46 + 0.32 * u * u
            };
            p.shape(all, |u, v| cap(0.1, hl, u, v), hair, Some(hair_sh));
        }
        Crop => {
            let hl = |u: f32| -0.54 + 0.3 * u * u + 0.26 * (-u).clamp(-0.2, 0.7);
            p.shape(all, move |u, v| cap(0.08, &hl, u * side, v), hair, Some(hair_sh));
            p.stroke(&curve(-0.55 * side, -0.1 * side, top + 0.2, -0.05, 0.04), p.lw * 1.4, shade(hair, 0.3));
        }
        Bob | Long | Bunches | Beehive => {
            let hl: &dyn Fn(f32) -> f32 = match l.style {
                Bob | Bunches => &|u: f32| -0.32 + 0.035 * (u * 14.0).sin().abs() + 0.12 * u.powi(4),
                Beehive => &|u: f32| -0.42 + 0.3 * u * u,
                _ => &|u: f32| -0.46 + 0.36 * u * u - 0.1 * u,
            };
            p.shape(all, |u, v| cap(0.1, hl, u * side, v), hair, Some(hair_sh));
            if l.style == Beehive {
                p.stroke(&curve(-0.36, 0.3, top - 0.18, -0.1, 0.02), p.lw * 1.2, hair_sh);
                p.stroke(&curve(-0.24, 0.42, top - 0.38, -0.08, -0.02), p.lw * 1.2, hair_sh);
            } else {
                p.stroke(&curve(-0.5 * side, 0.0, top + 0.12, -0.05, 0.03), p.lw * 1.6, shade(hair, 0.32));
            }
        }
    }
}

/// One eye at (eu, ev), side s (-1 the left): open with its iris, shut in a happy squint, or under a cross lid.
#[allow(clippy::too_many_arguments)]
fn eye(p: &mut Pen, l: &Look, e: usize, head: u8, eu: f32, ev: f32, rx: f32, ry: f32, s: f32, iris: Col) {
    let shut = e == 0 && (l.eyes != 0 || head.is_multiple_of(2));
    if shut {
        let pts = curve(eu - rx * 1.05, eu + rx * 1.05, ev + ry * 0.35, -ry * 0.95, 0.0);
        p.stroke(&pts, p.lw * 1.9, INK);
        if l.lashes {
            p.stroke(&[(eu + s * rx * 0.95, ev + ry * 0.25), (eu + s * rx * 1.35, ev + ry * 0.05)], p.lw * 1.3, INK);
        }
        return;
    }
    if l.eyes == 1 {
        let rad = if e == 2 { 0.055 } else { 0.065 };
        p.dot(eu, ev, rad, INK);
        p.dot(eu + 0.02, ev - 0.022, 0.02, rgb(255, 255, 255));
        return;
    }
    // the lid: none at rest (open and bright), slanted down to the nose (cross), or the cheek pushed up (happy)
    let lid: Box<dyn Fn(f32, f32) -> f32> = match e {
        2 => Box::new(move |u, v| ev - ry * 0.45 - s * (u - eu) * 0.42 * ry / rx - v),
        0 => Box::new(move |u, v| v - (ev + ry * 0.6 - ry * 0.55 * (1.0 - ((u - eu) / rx).powi(2)).max(0.0))),
        _ => Box::new(|_, _| -9.0),
    };
    let b = [eu - rx - 0.05, ev - ry - 0.05, eu + rx + 0.05, ev + ry + 0.05];
    let ball = |u: f32, v: f32| sd_ellipse(u, v, eu, ev, rx, ry).max(lid(u, v));
    p.shape(b, ball, rgb(255, 255, 255), None);
    let look = if e == 2 { 0.0 } else { 0.15 * if head.is_multiple_of(3) { -1.0 } else { 1.0 } };
    let (iu, iv) = (eu + rx * look, ev + ry * 0.12);
    let ir = rx * if e == 2 { 0.5 } else { 0.66 };
    p.flat(b, |u, v| circ(u, v, iu, iv, ir).max(ball(u, v)), iris);
    p.flat(b, |u, v| circ(u, v, iu, iv, ir * 0.5).max(ball(u, v)), rgb(30, 24, 30));
    p.flat(b, |u, v| circ(u, v, iu + ir * 0.35, iv - ir * 0.4, ir * 0.36).max(ball(u, v)), rgb(255, 255, 255));
    // the upper lid's line
    if e == 2 {
        let lv = |u: f32| ev - ry * 0.45 - s * (u - eu) * 0.42 * ry / rx;
        p.stroke(&[(eu - rx * 1.05, lv(eu - rx * 1.05)), (eu + rx * 1.05, lv(eu + rx * 1.05))], p.lw * 1.6, INK);
    } else {
        p.stroke(&curve(eu - rx * 1.02, eu + rx * 1.02, ev - ry * 0.55, -ry * 0.45, 0.0), p.lw * 1.4, INK);
    }
    if l.lashes {
        let (ou, ov) = (eu + s * rx * 0.92, ev - ry * if e == 2 { 0.75 } else { 0.5 });
        p.stroke(&[(ou, ov), (ou + s * rx * 0.45, ov - ry * 0.3)], p.lw * 1.3, INK);
        p.stroke(&[(ou - s * rx * 0.3, ov - ry * 0.3), (ou + s * rx * 0.0, ov - ry * 0.75)], p.lw * 1.2, INK);
    }
}

/// The mouth of the expression, `mw` its half width, at height mv.
fn mouth(p: &mut Pen, l: &Look, e: usize, mv: f32, mw: f32, side: f32) {
    let ink = l.lips.map(|c| shade(c, -0.3)).unwrap_or(INK);
    let lip_line = |p: &mut Pen, pts: &[(f32, f32)]| {
        if let Some(c) = l.lips {
            p.stroke(pts, p.lw * 3.4, c);
        }
        p.stroke(pts, p.lw * 1.7, ink);
    };
    let b = [-mw - 0.1, mv - 0.25, mw + 0.1, mv + 0.35];
    let dark = rgb(112, 32, 42);
    let teeth_col = rgb(255, 252, 244);
    let kind = l.mouths[e.min(2)];
    match (e, kind) {
        (0, 0) => {
            // an open grin with teeth and a tongue
            let top = move |t: f32| mv - 0.06 + 0.05 * (1.0 - t * t);
            let bot = move |t: f32| mv - 0.06 + 0.3 * (1.0 - t * t);
            let pts = lens(0.0, mw, top, bot);
            let m = |u: f32, v: f32| sd_poly(u, v, &pts);
            p.shape_in(b, m, dark, None, ink);
            p.flat(b, |u, v| m(u, v).max(sd_ellipse(u, v, 0.0, mv + 0.24, mw * 0.55, 0.12)), rgb(232, 112, 124));
            p.flat(b, |u, v| m(u, v).max(v - top((u / mw).clamp(-1.0, 1.0)) - 0.075), teeth_col);
            teeth_extras(p, l, mv - 0.01, mv + 0.07);
            if let Some(c) = l.lips {
                p.stroke(&pts, p.lw * 1.6, c);
            }
        }
        (0, 1) => {
            let pts = curve(-mw, mw, mv - 0.05, 0.14, 0.0);
            lip_line(p, &pts);
            for s in [-1.0f32, 1.0] {
                p.stroke(&[(s * (mw - 0.02), mv - 0.08), (s * (mw + 0.04), mv - 0.02)], p.lw * 1.3, ink);
            }
            if l.teeth == 2 {
                teeth_extras(p, l, mv + 0.08, mv + 0.16);
            }
        }
        (0, _) => {
            // a wide toothy grin
            let w = mw * 1.1;
            let top = move |t: f32| mv - 0.07 + 0.06 * (1.0 - t * t);
            let bot = move |t: f32| mv - 0.07 + 0.24 * (1.0 - t * t);
            let pts = lens(0.0, w, top, bot);
            p.shape_in(b, |u, v| sd_poly(u, v, &pts), teeth_col, None, ink);
            let line = |t: f32| (t * w, mv - 0.07 + 0.14 * (1.0 - t * t));
            p.stroke(&(0..=8).map(|k| line(k as f32 / 4.0 - 1.0)).collect::<Vec<_>>(), p.lw * 0.8, rgb(150, 130, 130));
            for t in [-0.62f32, -0.22, 0.22, 0.62] {
                p.stroke(&[(t * w, top(t) + 0.01), (t * w, bot(t) - 0.01)], p.lw * 0.7, rgb(150, 130, 130));
            }
            teeth_extras(p, l, mv - 0.03, mv + 0.06);
            if let Some(c) = l.lips {
                p.stroke(&pts, p.lw * 1.6, c);
            }
        }
        (1, 0) => lip_line(p, &curve(-mw * 0.72, mw * 0.72, mv, 0.04, 0.0)),
        (1, 1) => lip_line(p, &curve(-mw * 0.6, mw * 0.6, mv + 0.01, 0.025, 0.0)),
        (1, 2) => {
            lip_line(p, &[(-mw * 0.62 * side, mv + 0.03), (0.0, mv + 0.035), (mw * 0.5 * side, mv + 0.01), (mw * 0.7 * side, mv - 0.04)])
        }
        (1, _) => {
            let c = l.lips.unwrap_or(rgb(210, 110, 110));
            p.shape_in(b, |u, v| sd_ellipse(u, v, 0.0, mv, mw * 0.42, 0.075), c, Some(shade(c, -0.2)), ink);
            p.stroke(&curve(-mw * 0.3, mw * 0.3, mv, 0.015, 0.0), p.lw * 0.9, ink);
        }
        (_, 0) => lip_line(p, &curve(-mw * 0.7, mw * 0.7, mv + 0.07, -0.11, 0.0)),
        (_, 1) => {
            // gritted teeth
            let w = mw * 0.86;
            let top = move |t: f32| mv - 0.05 + 0.05 * t * t;
            let bot = move |t: f32| mv + 0.09 + 0.02 * t * t;
            let pts = lens(0.0, w, top, bot);
            p.shape_in(b, |u, v| sd_poly(u, v, &pts), teeth_col, None, ink);
            p.stroke(&curve(-w, w, mv + 0.03, -0.0, 0.0), p.lw * 0.9, rgb(130, 110, 110));
            for t in [-0.6f32, -0.2, 0.2, 0.6] {
                p.stroke(&[(t * w, top(t) + 0.01), (t * w, bot(t) - 0.01)], p.lw * 0.8, rgb(130, 110, 110));
            }
        }
        _ => {
            // shouting
            let w = mw * 0.8;
            let top = move |t: f32| mv + 0.06 - 0.1 * (1.0 - t * t);
            let bot = move |t: f32| mv + 0.06 + 0.12 * (1.0 - t * t);
            let pts = lens(0.0, w, top, bot);
            let m = |u: f32, v: f32| sd_poly(u, v, &pts);
            p.shape_in(b, m, dark, None, ink);
            p.flat(b, |u, v| m(u, v).max(v - top((u / w).clamp(-1.0, 1.0)) - 0.05), teeth_col);
            p.flat(b, |u, v| m(u, v).max(sd_ellipse(u, v, 0.0, mv + 0.17, w * 0.5, 0.07)), rgb(232, 112, 124));
            if let Some(c) = l.lips {
                p.stroke(&pts, p.lw * 1.4, c);
            }
        }
    }
}

/// A gap, buck teeth or braces on the teeth between heights v0 and v1.
fn teeth_extras(p: &mut Pen, l: &Look, v0: f32, v1: f32) {
    match l.teeth {
        1 => p.flat([-0.1, v0 - 0.05, 0.1, v1 + 0.05], |u, v| sd_rect(u, v, -0.022, v0, 0.044, v1 - v0, 0.01), rgb(112, 32, 42)),
        2 => {
            for s in [-1.0f32, 1.0] {
                let u0 = if s < 0.0 { -0.075 } else { 0.0 };
                p.shape(
                    [u0 - 0.05, v0 - 0.08, u0 + 0.13, v1 + 0.1],
                    |u, v| sd_rect(u, v, u0 + 0.004, v0 - 0.02, 0.067, v1 - v0 + 0.07, 0.02),
                    rgb(255, 252, 244),
                    None,
                );
            }
        }
        3 => {
            let v = (v0 + v1) / 2.0;
            p.stroke(&curve(-0.24, 0.24, v - 0.01, 0.02, 0.0), p.lw * 1.1, rgb(150, 160, 176));
            for k in 0..5 {
                let u = (k as f32 - 2.0) * 0.1;
                p.flat(
                    [u - 0.05, v - 0.05, u + 0.05, v + 0.05],
                    |x, y| sd_rect(x, y, u - 0.018, v - 0.015 + 0.02 * (1.0 - (u / 0.24).powi(2)) - 0.01, 0.036, 0.036, 0.005),
                    rgb(130, 140, 160),
                );
            }
        }
        _ => {}
    }
}

/// The moustache (drooping when cross, lifted when happy) and the goatee.
#[allow(clippy::too_many_arguments)]
fn tache(p: &mut Pen, l: &Look, e: usize, mv: f32, chin: f32, hair: Col, hair_sh: Col) {
    let tilt = [-0.12, 0.0, 0.16][e.min(2)];
    let ty = mv - 0.11;
    let all = [-0.7, ty - 0.35, 0.7, ty + 0.4];
    match l.tache {
        1 => {
            let b = [(-0.15, ty, 0.2), (0.15, ty, 0.2), (-0.3, ty + 0.08, 0.13), (0.3, ty + 0.08, 0.13)];
            let sd = move |u: f32, v: f32| {
                let v = v - tilt * u.abs();
                b.iter().fold(9.0f32, |d, &(cu, cv, r)| d.min(sd_ellipse(u, v, cu, cv, r, r * 0.6)))
            };
            p.shape(all, sd, hair, Some(hair_sh));
            for k in [-2.0f32, -1.0, 0.0, 1.0, 2.0] {
                let u = k * 0.09;
                p.stroke(&[(u, ty + 0.0 + tilt * u.abs()), (u * 1.15, ty + 0.08 + tilt * u.abs())], p.lw * 0.7, hair_sh);
            }
        }
        2 => {
            let mut b = vec![(-0.1, ty, 0.08), (0.1, ty, 0.08)];
            for s in [-1.0f32, 1.0] {
                for &(u, v, r) in &[
                    (0.2, ty + 0.01, 0.065),
                    (0.3, ty + 0.0, 0.05),
                    (0.38, ty - 0.05, 0.04),
                    (0.4, ty - 0.12, 0.035),
                    (0.35, ty - 0.16, 0.03),
                ] {
                    b.push((s * u, v, r));
                }
            }
            let sd = move |u: f32, v: f32| blobs(u, v - tilt * 0.6 * u.abs(), &b);
            p.shape(all, sd, hair, Some(hair_sh));
        }
        3 => p.stroke(&curve(-0.2, 0.2, ty + 0.04 + tilt * 0.2, -0.03, 0.0), p.lw * 1.6, shade(hair, -0.2)),
        4 => {
            let sd = move |u: f32, v: f32| sd_rect(u, v - tilt * 0.5 * u.abs(), -0.25, ty - 0.06, 0.5, 0.13, 0.06);
            p.shape(all, sd, hair, Some(hair_sh));
        }
        _ => {}
    }
    if l.beard == 2 {
        // a pointed tuft from under the lip past the chin
        let pts = [(-0.15, mv + 0.13), (0.15, mv + 0.13), (0.1, chin - 0.02), (0.0, chin + 0.12), (-0.1, chin - 0.02)];
        p.shape([-0.4, mv, 0.4, chin + 0.3], |u, w| sd_poly(u, w, &pts) - 0.03, hair, Some(hair_sh));
        p.stroke(&[(0.0, mv + 0.2), (0.0, chin + 0.02)], p.lw * 0.8, hair_sh);
        if l.tache == 0 {
            for s in [-1.0f32, 1.0] {
                p.stroke(&[(s * 0.07, ty + 0.0), (s * 0.24, ty + 0.06 + tilt * 0.2), (s * 0.25, ty + 0.16)], p.lw * 1.7, hair);
            }
        }
    }
}

fn nose(p: &mut Pen, l: &Look, skin: Col, skin_sh: Col, side: f32) {
    let nv = 0.16;
    let b = [-0.3, -0.12, 0.3, 0.36];
    let col = shade(skin, -0.03);
    match l.nose {
        1 => {
            let c = mix(skin, rgb(230, 70, 70), 0.32);
            p.shape(b, |u, v| sd_ellipse(u, v, 0.0, nv + 0.01, 0.16, 0.13), c, Some(shade(c, -0.15)));
            p.dot(-0.06, nv - 0.04, 0.035, alpha(rgb(255, 255, 255), 0.7));
        }
        2 | 3 => {
            let pts: Vec<(f32, f32)> = if l.nose == 2 {
                vec![(-0.03, -0.06), (0.04, -0.06), (0.2, 0.2), (0.15, 0.26), (-0.05, 0.25), (-0.07, 0.2)]
            } else {
                vec![(-0.02, -0.08), (0.05, -0.08), (0.14, 0.08), (0.15, 0.26), (0.08, 0.3), (0.06, 0.24), (-0.07, 0.25), (-0.07, 0.19)]
            };
            let pts: Vec<(f32, f32)> = pts.into_iter().map(|(u, v)| (u * side, v)).collect();
            p.shape(b, |u, v| sd_poly(u, v, &pts) - 0.02, col, Some(skin_sh));
            p.dot(0.02 * side, 0.22, 0.022, shade(skin, -0.45));
        }
        4 => {
            let pts = [(0.03 * side, 0.02), (0.08 * side, 0.18), (0.02 * side, 0.22), (-0.04 * side, 0.2)];
            p.stroke(&pts, p.lw * 1.4, shade(skin, -0.38));
        }
        5 => {
            p.shape(b, |u, v| sd_ellipse(u, v, 0.0, nv, 0.11, 0.075), col, Some(skin_sh));
            for s in [-1.0f32, 1.0] {
                p.dot(s * 0.035, nv + 0.02, 0.018, shade(skin, -0.5));
            }
        }
        6 => p.shape(b, |u, v| sd_ellipse(u, v, 0.0, nv, 0.13, 0.1), col, Some(skin_sh)),
        _ => p.shape(b, |u, v| sd_ellipse(u, v, 0.0, nv, 0.085, 0.07), col, Some(skin_sh)),
    }
}

fn specs(p: &mut Pen, l: &Look, es: f32, ev: f32, hw: f32) {
    let fc = l.frame;
    let glint = alpha(rgb(255, 255, 255), 0.75);
    let tint = alpha(rgb(220, 236, 255), 0.22);
    let b = [-hw - 0.1, ev - 0.4, hw + 0.1, ev + 0.4];
    let o = p.lw / p.r;
    match l.specs {
        1 | 2 => {
            let lens_sd = move |u: f32, v: f32, s: f32| {
                if l.specs == 1 {
                    circ(u, v, s * es, ev, 0.2)
                } else {
                    sd_rect(u, v, s * es - 0.22, ev - 0.17, 0.44, 0.33, 0.07)
                }
            };
            let t = if l.specs == 1 { 0.028 } else { 0.05 };
            for s in [-1.0f32, 1.0] {
                p.flat(b, |u, v| lens_sd(u, v, s), tint);
                p.flat(b, |u, v| lens_sd(u, v, s).abs() - t - o * 0.5, INK);
                p.flat(b, |u, v| lens_sd(u, v, s).abs() - t, fc);
                let gu = s * es - 0.08;
                p.stroke(&[(gu, ev - 0.02), (gu + 0.07, ev - 0.1)], p.lw * 1.2, glint);
                let edge_u = s * (es + if l.specs == 1 { 0.2 } else { 0.22 });
                p.stroke(&[(edge_u, ev - 0.04), (s * (hw + 0.01), ev - 0.07)], p.lw * 1.6, fc);
            }
            let bridge = curve(-es + 0.2, es - 0.2, ev - 0.03, -0.04, 0.0);
            p.stroke(&bridge, p.lw * 1.6, fc);
        }
        3 => {
            for s in [-1.0f32, 1.0] {
                let lv = ev + 0.1;
                let half = move |u: f32, v: f32| sd_ellipse(u, v, s * es, lv, 0.18, 0.12).max(lv - v);
                p.flat(b, half, tint);
                p.flat(b, |u, v| half(u, v).abs() - 0.02 - o * 0.4, INK);
                p.flat(b, |u, v| half(u, v).abs() - 0.02, fc);
                p.stroke(&[(s * (es + 0.18), lv), (s * (hw + 0.01), ev - 0.04)], p.lw * 1.2, fc);
            }
            p.stroke(&curve(-es + 0.18, es - 0.18, ev + 0.1, -0.04, 0.0), p.lw * 1.2, fc);
        }
        4 => {
            let dark = rgb(36, 40, 58);
            for s in [-1.0f32, 1.0] {
                let lens_sd = move |u: f32, v: f32| {
                    smin(sd_ellipse(u, v, s * es, ev, 0.21, 0.13), sd_ellipse(u, v, s * (es - 0.04), ev + 0.08, 0.15, 0.12), 0.05)
                };
                p.shape(b, lens_sd, dark, None);
                p.flat(
                    b,
                    |u, v| lens_sd(u, v).max(sd_ellipse(u, v, s * es - 0.06, ev - 0.05, 0.12, 0.05)),
                    alpha(rgb(130, 170, 220), 0.55),
                );
                let gu = s * es + 0.06;
                p.stroke(&[(gu - 0.04, ev + 0.08), (gu + 0.06, ev - 0.02)], p.lw * 1.2, glint);
                p.stroke(&[(s * (es + 0.21), ev - 0.04), (s * (hw + 0.01), ev - 0.07)], p.lw * 1.8, INK);
            }
            p.stroke(&[(-es + 0.18, ev - 0.06), (es - 0.18, ev - 0.06)], p.lw * 2.0, INK);
        }
        _ => {}
    }
}

fn hat(p: &mut Pen, l: &Look, hs: impl Fn(f32, f32) -> f32 + Copy, top: f32, hw: f32) {
    let hc = l.hat_col;
    let hc_sh = shade(hc, -0.25);
    let all = [-1.25, -1.4, 1.25, 1.3];
    match l.hat {
        1 => {
            let bv = top + 0.32;
            p.shape(all, move |u, v| (hs(u, v) - 0.05).max((v - bv).abs() - 0.07), hc, Some(hc_sh));
            p.flat(all, move |u, v| hs(u, v).max((v - bv - 0.12).abs() - 0.05), alpha(rgb(80, 40, 30), 0.18));
            p.shape(all, move |u, v| sd_ellipse(u, v, 0.0, bv + 0.05, hw * 1.02, 0.2).max(bv + 0.05 - v), shade(hc, 0.18), Some(hc));
        }
        2 => {
            let bv = top + 0.4;
            p.shape(all, move |u, v| (hs(u, v) - 0.06).max(v - bv), hc, Some(hc_sh));
            p.stroke(&[(0.0, top - 0.04), (0.0, bv - 0.02)], p.lw * 0.9, hc_sh);
            p.dot(0.0, top - 0.04, 0.05, hc_sh);
            p.dot(-0.24, bv - 0.15, 0.07, rgb(255, 255, 255));
            p.flat(all, move |u, v| hs(u, v).max((v - bv - 0.13).abs() - 0.05), alpha(rgb(80, 40, 30), 0.2));
            p.shape(all, move |u, v| sd_ellipse(u, v, 0.0, bv, hw * 1.06, 0.22).max(bv - v), shade(hc, -0.08), Some(hc_sh));
        }
        3 => {
            let bv = top + 0.36;
            p.shape(all, move |u, v| sd_ellipse(u, v, 0.04, top + 0.12, hw * 1.12, 0.3).max(v - bv), hc, Some(hc_sh));
            p.shape(all, move |u, v| sd_ellipse(u, v, 0.04, bv - 0.02, hw * 0.88, 0.1).max(bv - 0.04 - v), hc_sh, None);
            for k in 0..3 {
                let u = (k as f32 - 1.0) * 0.3;
                p.stroke(&[(u, top - 0.1), (u * 1.2, bv - 0.06)], p.lw * 0.7, shade(hc, -0.15));
            }
        }
        4 => {
            let bv = top + 0.24;
            p.shape(all, move |u, v| (hs(u, v) - 0.11).max((v - bv).abs() - 0.06), hc, Some(hc_sh));
        }
        5 => {
            let v = top + 0.1;
            for s in [-1.0f32, 1.0] {
                p.shape(all, |x, y| sd_ellipse(x, y, s * 0.24, v, 0.2, 0.11), rgb(36, 40, 58), None);
                p.stroke(&[(s * 0.3, v - 0.05), (s * 0.2, v + 0.04)], p.lw, alpha(rgb(255, 255, 255), 0.7));
            }
            p.stroke(&[(-0.06, v - 0.03), (0.06, v - 0.03)], p.lw * 1.8, INK);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_stay_in_their_cells() {
        // each face drawn alone in its cell (the halo page's 140 x 140, the expressions page's 90 x 120) leaves the cell's
        // border clear: nothing is cut off and nothing reaches a neighbour
        for (w, h, cx, cy, r) in [(140usize, 140usize, 70.0, 72.0, 50.0), (90, 120, 45.0, 62.0, 36.0)] {
            for female in [false, true] {
                for head in 0..19u8 {
                    for e in 0..3 {
                        let mut c = Canvas::new(w, h, [0.0; 4]);
                        face(&mut c, cx, cy, r, head, e, female);
                        for y in 0..h {
                            for x in 0..w {
                                if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                                    assert!(c.px[y * w + x][3] < 0.02, "head {head} ({female}) {e}: drawn at ({x}, {y})");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
