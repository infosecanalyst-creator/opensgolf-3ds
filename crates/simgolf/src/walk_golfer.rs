//! Golfers in the round for the walk camera: jointed bodies that act out what the game has them doing.
//!
//! A golfer's pictures are drawn for eight headings and a few dozen pixels tall; seen from the ground at a few paces they
//! are flat and blocky, and no single solid can stand for them because they change with every step and every swing. So a
//! golfer here is a small figure of the project's own design: hips, a body, a head with a cap (or hair), arms and legs
//! jointed at the knees and elbows, shoes, and a club, coloured from the golfer's own outfit (the shirt, trousers, skin,
//! hat, hair and hand colours the game composes for them from the swap palettes, `sg_core::bodies`).
//!
//! The game says what each golfer is doing by the clip it plays (`app::GOLFER_CLIPS`: walking, addressing the ball,
//! swinging, chipping, putting, lining up a putt, cheering, sulking, sitting, pointing...) and how far through it they
//! are. Each clip is a pose here, moving with the clip's progress: the walk's stride, the swing's backswing, downswing and
//! follow-through, the putter's pendulum. Feet and hands are placed where the pose wants them and the knees and elbows
//! found between (two-bone reaching), so a pose is a handful of targets rather than every joint's angle.

use crate::gfx::Vert;
use std::f32::consts::{PI, TAU};

type V = [f32; 3];

fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V, k: f32) -> V {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V, b: V) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: V) -> f32 {
    dot(a, a).sqrt()
}
fn norm(a: V) -> V {
    mul(a, 1.0 / len(a).max(1e-6))
}
/// `a` turned about the vertical by `t` (radians; +x towards +z).
fn yaw(a: V, t: f32) -> V {
    let (s, c) = t.sin_cos();
    [a[0] * c - a[2] * s, a[1], a[0] * s + a[2] * c]
}
fn mix(a: V, b: V, t: f32) -> V {
    add(mul(a, 1.0 - t), mul(b, t))
}
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// A golfer's colours (linear 0..1).
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub shirt: [f32; 3],
    pub pants: [f32; 3],
    pub skin: [f32; 3],
    pub hat: [f32; 3],
    pub hair: [f32; 3],
    pub hands: [f32; 3],
    pub female: bool,
}

impl Look {
    /// From a composed golfer palette (`Swaps::compose`): the middle of each range's ramp (shirt 0..20, trousers 20..40,
    /// skin 40..60, hat 60..80, then hair 80..90 and arms 90..97 for a woman, arms 80..90 for a man).
    pub fn from_palette(pal: &[u8; 768], female: bool) -> Look {
        let ramp = |first: usize, n: usize| {
            // the ramp's entries by brightness, and the middle third's mean: neither its highlight nor its shadow
            let mut c: Vec<[f32; 3]> =
                (first..first + n).map(|i| [pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]].map(|v| v as f32 / 255.0)).collect();
            c.sort_by(|a, b| (a[0] + a[1] + a[2]).partial_cmp(&(b[0] + b[1] + b[2])).unwrap_or(std::cmp::Ordering::Equal));
            let mid = &c[n / 3..(2 * n / 3).max(n / 3 + 1)];
            let s = mid.iter().fold([0.0; 3], |a, v| [a[0] + v[0], a[1] + v[1], a[2] + v[2]]);
            s.map(|v| v / mid.len() as f32)
        };
        let skin = ramp(40, 20);
        Look {
            shirt: ramp(0, 20),
            pants: ramp(20, 20),
            skin,
            hat: ramp(60, 20),
            hair: if female { ramp(80, 10) } else { [0.22, 0.15, 0.1] },
            hands: if female { ramp(90, 7) } else { ramp(80, 10) },
            female,
        }
    }

    /// When the game has no swap palettes: colours from the outfit's numbers.
    pub fn plain(o: &sg_core::bodies::Outfit) -> Look {
        const CLOTH: [[f32; 3]; 10] = [
            [0.92, 0.92, 0.9],
            [0.75, 0.15, 0.15],
            [0.15, 0.3, 0.7],
            [0.2, 0.55, 0.25],
            [0.95, 0.8, 0.2],
            [0.5, 0.25, 0.6],
            [0.15, 0.15, 0.18],
            [0.9, 0.5, 0.65],
            [0.55, 0.45, 0.3],
            [0.4, 0.75, 0.85],
        ];
        const SKIN: [[f32; 3]; 4] = [[0.93, 0.76, 0.62], [0.8, 0.6, 0.45], [0.6, 0.42, 0.3], [0.38, 0.25, 0.18]];
        let skin = SKIN[o.skin as usize % 4];
        Look {
            shirt: CLOTH[o.shirt as usize % 10],
            pants: CLOTH[(o.pants as usize + 6) % 10],
            skin,
            hat: CLOTH[o.hat as usize % 10],
            hair: [0.25, 0.17, 0.1],
            hands: if o.alt_skin == 4 { [0.95, 0.95, 0.92] } else { skin },
            female: o.female,
        }
    }
}

/// What a golfer is doing: the game's clip, by its place in `app::GOLFER_CLIPS`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    Stand,
    Walk {
        tired: bool,
    },
    Fidget,
    Happy,
    /// A fist pumped (a great shot).
    Pump,
    Sad,
    /// The club raised and slammed down in a temper, then hands on hips (a bad shot).
    Fail,
    Swing {
        reach: f32,
    },
    Putt,
    Address {
        putt: bool,
    },
    LineUp,
    Point,
    Lean(f32),
    LookUp,
    Sit,
    Shake,
}

impl Act {
    /// The act of a clip by its name in `app::GOLFER_CLIPS`.
    pub fn of_clip(name: &str) -> Act {
        match name {
            "NormalWalk" => Act::Walk { tired: false },
            "TiredWalk" => Act::Walk { tired: true },
            "Fidget" => Act::Fidget,
            "Happy" => Act::Happy,
            "SuccessA" => Act::Pump,
            "Sad" => Act::Sad,
            "FailureA" => Act::Fail,
            "PerfectSwing" | "NoAccSwing" | "NoimagSwing" | "NormalSwing" => Act::Swing { reach: 1.0 },
            "Pitch" => Act::Swing { reach: 0.55 },
            "Putt" => Act::Putt,
            "NormalAddress" | "PitchAddress" => Act::Address { putt: false },
            "PuttAddress" => Act::Address { putt: true },
            "LineUpPutt" => Act::LineUp,
            "PointAt" => Act::Point,
            "LeanRight" => Act::Lean(1.0),
            "LeanLeft" => Act::Lean(-1.0),
            "LookUp" => Act::LookUp,
            "Sitting" | "SitSq" | "Cart" => Act::Sit,
            "HandShake" => Act::Shake,
            _ => Act::Stand,
        }
    }

    /// Whether the golfer stands side on to the heading (the shot's line), as at the ball; otherwise they face it.
    pub fn at_ball(self) -> bool {
        matches!(self, Act::Swing { .. } | Act::Putt | Act::Address { .. })
    }
}

/// The body's targets for a pose, in metres for a figure 1.78 m tall: x to the golfer's right, y up, z forward.
struct Pose {
    /// The middle of the hips.
    hips: V,
    /// The hips' turn (radians, about the vertical, + turns the right hip back) and the shoulders' turn on top of it.
    hip_turn: f32,
    chest_turn: f32,
    /// The spine's bend forward and lean to the right (radians).
    bend: f32,
    lean: f32,
    /// The head's nod (radians, + looks down).
    nod: f32,
    /// The feet (ankles) and the hands (right, left).
    feet: [V; 2],
    hands: [V; 2],
    /// The club: where it is held (the top hand) and its direction down the shaft; none when it isn't in hand.
    club: Option<(V, V, f32)>,
}

/// The two-bone reach from `a` towards `t` with bone lengths `l1`, `l2`, bending towards `hint`: the middle joint and
/// the end (at `t` when it is in reach).
fn reach(a: V, t: V, l1: f32, l2: f32, hint: V) -> (V, V) {
    let d = sub(t, a);
    let dist = len(d).clamp(0.05, (l1 + l2) * 0.999);
    let dir = norm(d);
    let ca = ((l1 * l1 + dist * dist - l2 * l2) / (2.0 * l1 * dist)).clamp(-1.0, 1.0);
    let sa = (1.0 - ca * ca).sqrt();
    let mut h = sub(hint, mul(dir, dot(hint, dir)));
    if len(h) < 1e-4 {
        h = cross(dir, [1.0, 0.0, 0.0]);
    }
    let h = norm(h);
    let mid = add(a, add(mul(dir, l1 * ca), mul(h, l1 * sa)));
    (mid, add(a, mul(dir, dist)))
}

/// The pose for an act `t` of the way through its clip (0..1), `time` in seconds for idle movement.
fn pose(act: Act, t: f32, time: f32, club_len: f32) -> Pose {
    let stand = Pose {
        hips: [0.0, 0.95, 0.0],
        hip_turn: 0.0,
        chest_turn: 0.0,
        bend: 0.04,
        lean: 0.0,
        nod: 0.05,
        feet: [[0.12, 0.08, 0.0], [-0.12, 0.08, 0.0]],
        hands: [[0.27, 0.68, 0.04], [-0.27, 0.68, 0.04]],
        club: None,
    };
    let breathe = (time * 1.7).sin() * 0.008;
    match act {
        Act::Stand | Act::Fidget | Act::LookUp => {
            let shift = if act == Act::Fidget { (t * TAU).sin() * 0.04 } else { 0.0 };
            let mut p = Pose { hips: [shift, 0.95 + breathe, 0.0], lean: shift * 1.2, ..stand };
            if act == Act::LookUp {
                // watching the ball fly, a hand shading the eyes, the club in the other
                p.nod = -0.45;
                p.hands = [[0.27, 0.68, 0.06], [-0.05, 1.72, 0.2]];
                p.club = Some(([0.27, 0.68, 0.06], norm([0.08, -1.0, 0.18]), club_len));
            }
            if act == Act::Fidget {
                // a hand to the cap's peak at the clip's middle
                let k = smooth(1.0 - ((t - 0.5).abs() * 3.0));
                p.hands[0] = [lerp(0.27, 0.1, k), lerp(0.68, 1.66, k), lerp(0.04, 0.12, k)];
            }
            p
        }
        Act::Walk { tired } => {
            let ph = t * TAU;
            let stride = if tired { 0.22 } else { 0.32 };
            let foot = |k: f32| {
                let s = (ph + k).sin();
                let lift = (ph + k).cos().max(0.0) * 0.09;
                (s * stride, 0.08 + lift)
            };
            let (zr, yr) = foot(0.0);
            let (zl, yl) = foot(PI);
            let bob = (ph * 2.0).cos().abs() * 0.025;
            let swing = ph.sin() * 0.22;
            Pose {
                hips: [0.0, 0.93 + bob, 0.0],
                hip_turn: -swing * 0.25,
                chest_turn: swing * 0.35,
                bend: if tired { 0.2 } else { 0.06 },
                nod: if tired { 0.3 } else { 0.05 },
                feet: [[0.1, yr, zr], [-0.1, yl, zl]],
                hands: [[0.25, 0.7, -swing], [-0.25, 0.7, swing]],
                // a tired golfer drags the club behind him
                club: tired.then(|| ([0.25, 0.7, -swing], norm([0.12, -0.7, -0.7]), club_len)),
                ..stand
            }
        }
        Act::Happy => {
            let jump = ((t * TAU * 2.0).sin()).max(0.0) * 0.07;
            Pose {
                hips: [0.0, 0.95 + jump, 0.0],
                nod: -0.15,
                feet: [[0.13, 0.08 + jump, 0.0], [-0.13, 0.08 + jump, 0.0]],
                hands: [[0.3, 1.95 + jump, 0.08], [-0.3, 1.95 + jump, 0.08]],
                ..stand
            }
        }
        Act::Sad => Pose { bend: 0.28, nod: 0.55, hips: [0.0, 0.94, 0.0], hands: [[0.17, 0.95, 0.08], [-0.17, 0.95, 0.08]], ..stand },
        Act::Pump => {
            // the right fist pumped down twice, a little crouch with each; the club in the left hand
            let k = 0.5 + 0.5 * (t * TAU * 2.0).cos();
            let fist = [lerp(0.16, 0.26, k), lerp(0.95, 1.6, k), lerp(0.2, 0.18, k)];
            Pose {
                hips: [0.0, 0.93 - (1.0 - k) * 0.05, 0.0],
                bend: 0.1 + (1.0 - k) * 0.12,
                nod: -0.1,
                feet: [[0.17, 0.08, 0.02], [-0.17, 0.08, 0.0]],
                hands: [fist, [-0.26, 0.7, 0.05]],
                club: Some(([-0.26, 0.7, 0.05], norm([-0.05, -0.6, 0.8]), club_len)),
                ..stand
            }
        }
        Act::Fail => {
            // the club up over the head, slammed down in front, then hands on hips glaring at it
            let (grip, shaft, hands, bend) = if t < 0.3 {
                let k = smooth(t / 0.3);
                let g = [0.0, lerp(0.85, 1.95, k), lerp(0.3, -0.05, k)];
                (g, norm([0.0, lerp(-0.9, 0.6, k), lerp(0.4, -0.8, k)]), [add(g, [0.05, -0.05, 0.0]), g], lerp(0.1, -0.1, k))
            } else if t < 0.4 {
                let k = smooth((t - 0.3) / 0.1);
                let g = [0.0, lerp(1.95, 0.75, k), lerp(-0.05, 0.4, k)];
                (g, norm([0.0, lerp(0.6, -0.8, k), lerp(-0.8, 0.6, k)]), [add(g, [0.05, -0.05, 0.0]), g], lerp(-0.1, 0.45, k))
            } else {
                // the club left lying where it was thrown
                ([0.12, 0.03, 0.4], norm([0.2, 0.0, 1.0]), [[0.2, 0.98, -0.04], [-0.2, 0.98, -0.04]], 0.15)
            };
            Pose { bend, nod: 0.35, hands, club: Some((grip, shaft, club_len)), feet: [[0.16, 0.08, 0.0], [-0.16, 0.08, 0.0]], ..stand }
        }
        Act::Point => Pose { nod: -0.05, hands: [[0.12, 1.5, 0.62], [-0.27, 0.68, 0.04]], ..stand },
        Act::Shake => Pose { hands: [[0.08, 1.0, 0.45], [-0.27, 0.68, 0.04]], ..stand },
        Act::Lean(side) => {
            let k = smooth(1.0 - ((t - 0.5).abs() * 2.2)) * side;
            Pose { lean: k * 0.3, hips: [-k * 0.05, 0.95, 0.0], nod: 0.15, ..stand }
        }
        Act::Sit => Pose {
            hips: [0.0, 0.5, -0.05],
            bend: -0.05,
            feet: [[0.14, 0.08, 0.42], [-0.14, 0.08, 0.42]],
            hands: [[0.18, 0.6, 0.3], [-0.18, 0.6, 0.3]],
            ..stand
        },
        Act::LineUp => Pose {
            hips: [0.0, 0.48, -0.15],
            bend: 0.55,
            nod: 0.1,
            feet: [[0.2, 0.08, 0.15], [-0.2, 0.08, -0.05]],
            hands: [[0.2, 0.55, 0.42], [-0.15, 0.55, 0.3]],
            club: Some(([-0.15, 0.55, 0.3], norm([0.1, -0.45, 0.6]), club_len)),
            ..stand
        },
        Act::Swing { reach } => full_swing(reach, t, club_len),
        Act::Address { putt: false } => {
            // at the ball, the club waggling a little
            let mut p = full_swing(1.0, 0.0, club_len);
            if let Some((g, d, l)) = p.club {
                let w = (time * 2.2).sin() * 0.06;
                p.club = Some((g, norm(add(d, [w, 0.0, 0.0])), l));
                p.hands[0] = add(g, mul(p.club.unwrap().1, 0.09));
            }
            p
        }
        Act::Putt | Act::Address { .. } => {
            let putt = matches!(act, Act::Putt | Act::Address { putt: true });
            // the club's turn round the swing (radians): 0 at the ball, + back over the right shoulder, - through
            let theta = match act {
                Act::Swing { reach } => {
                    let top = 2.4 * reach;
                    let end = -2.6 * reach;
                    if t < 0.12 {
                        0.0
                    } else if t < 0.48 {
                        top * smooth((t - 0.12) / 0.36)
                    } else if t < 0.56 {
                        top * (1.0 - ((t - 0.48) / 0.08).powf(1.6))
                    } else if t < 0.78 {
                        end * smooth((t - 0.56) / 0.22)
                    } else {
                        end
                    }
                }
                Act::Putt => {
                    if t < 0.15 {
                        0.0
                    } else if t < 0.5 {
                        0.32 * (smooth((t - 0.15) / 0.35))
                    } else if t < 0.62 {
                        0.32 * (1.0 - smooth((t - 0.5) / 0.12))
                    } else {
                        -0.35 * smooth((t - 0.62) / 0.2)
                    }
                }
                _ => (time * 2.2).sin() * 0.05,
            };
            let bend: f32 = if putt { 0.62 } else { 0.45 };
            let hips = [0.0, if putt { 0.92 } else { 0.91 }, -0.04];
            let spine = 0.5;
            let shoulders = add(hips, [0.0, spine * bend.cos(), spine * bend.sin()]);
            // the hands' circle round the shoulders, in the plane of the target line and the club's line at the ball
            let down = norm([0.0, -0.9, if putt { 0.3 } else { 0.48 }]);
            let r = if putt { 0.5 } else { 0.6 };
            let side = [1.0, 0.0, 0.0];
            // the backswing lifts the hands up and round, the follow-through round to the left
            let grip = add(shoulders, mul(add(mul(side, theta.sin()), mul(down, theta.cos())), r));
            let grip = if theta.abs() > 1.4 { add(grip, [0.0, (theta.abs() - 1.4) * 0.12, 0.0]) } else { grip };
            // the wrists cock in the backswing and release through the ball
            let cock = if putt {
                theta
            } else if theta > 0.0 {
                theta + (theta / 2.4).min(1.0) * 1.3
            } else {
                theta * 1.25
            };
            let shaft = norm(add(mul(side, cock.sin()), mul(down, cock.cos())));
            let shaft = norm(add(shaft, [0.0, if theta.abs() > 1.6 { 0.2 } else { 0.0 }, 0.0]));
            let right = add(grip, mul(shaft, 0.09));
            let turn = theta.clamp(-2.6, 2.4);
            let finish = (-theta - 1.2).max(0.0);
            Pose {
                hips: add(hips, [turn * -0.03, 0.0, 0.0]),
                hip_turn: turn * 0.18,
                chest_turn: turn * 0.38,
                bend: bend - finish * 0.12,
                lean: 0.06 + turn * -0.02,
                nod: if putt { 0.9 } else { 0.75 - finish * 0.35 },
                feet: [[0.23, 0.08, 0.0], [-0.23, 0.08 + finish * 0.03, 0.0]],
                hands: [right, grip],
                club: Some((grip, shaft, club_len)),
            }
        }
    }
}

/// A full swing's keys through its clip (the game strikes the ball three quarters of the way through a swing or chip clip,
/// golfer.rs, so impact is keyed there): (t, the top hand's place, the club's direction down the shaft, the shoulders'
/// turn), in metres with x to the golfer's right, y up, z towards the ball; the target is on their left (-x).
const SWING: [(f32, V, V, f32); 9] = [
    // at the ball, still
    (0.0, [-0.04, 0.84, 0.3], [0.04, -0.87, 0.48], 0.0),
    (0.1, [-0.04, 0.84, 0.3], [0.04, -0.87, 0.48], 0.0),
    // the takeaway: hands back to the right hip, the club along the target line, toe up
    (0.3, [0.3, 0.86, 0.3], [1.0, 0.12, -0.05], 0.55),
    // halfway back: the wrists cocked, the club upright
    (0.45, [0.42, 1.18, 0.14], [0.35, 1.0, -0.2], 1.0),
    // the top: the hands over the right shoulder, the club across the back of the neck towards the target
    (0.6, [0.26, 1.52, -0.04], [-1.0, -0.08, -0.3], 1.45),
    // the downswing: the club lagging behind the hands
    (0.7, [0.34, 1.02, 0.22], [0.55, 0.95, -0.05], 0.75),
    // impact: back at the ball, the hands a little ahead
    (0.77, [-0.1, 0.84, 0.31], [0.1, -0.87, 0.47], 0.15),
    // through: the arms out towards the target, the club along the line
    (0.85, [-0.4, 0.95, 0.28], [-1.0, 0.2, 0.05], -0.85),
    // the finish: the hands high by the left ear, the club down the back, the chest to the target
    (0.96, [-0.2, 1.48, 0.02], [0.65, -0.45, -0.62], -1.6),
];

/// A chip's keys: the full swing's, back only to halfway and through only to the arms out along the line.
fn chip_keys() -> [(f32, V, V, f32); 6] {
    let k = |i: usize, t: f32| (t, SWING[i].1, SWING[i].2, SWING[i].3);
    [k(0, 0.0), k(1, 0.15), k(2, 0.38), k(3, 0.58), k(6, 0.77), k(7, 0.93)]
}

/// The pose of a full swing (`reach` 1) or a chip (less) `t` of the way through its clip.
fn full_swing(reach: f32, t: f32, club_len: f32) -> Pose {
    let chip;
    let keys: &[(f32, V, V, f32)] = if reach < 1.0 {
        chip = chip_keys();
        &chip
    } else {
        &SWING
    };
    let k = keys.iter().rposition(|key| key.0 <= t).unwrap_or(0);
    let (a, b) = (keys[k], keys[(k + 1).min(keys.len() - 1)]);
    let f = if b.0 > a.0 { smooth((t - a.0) / (b.0 - a.0)) } else { 0.0 };
    let mix = |p: V, q: V| add(p, mul(sub(q, p), f));
    let (mut grip, shaft, turn) = (mix(a.1, b.1), norm(mix(a.2, b.2)), lerp(a.3, b.3, f));
    // a longer club (the driver) held a little higher at the ball, its head on the ground
    let near = 1.0 - (turn.abs() / 0.6).min(1.0);
    grip[1] += (club_len - 0.96) * 0.87 * near;
    let finish = (-turn - 0.6).max(0.0);
    let bend: f32 = 0.45 - finish * 0.22;
    Pose {
        hips: [turn * -0.025, 0.91 + finish * 0.02, -0.04],
        hip_turn: turn * 0.45,
        chest_turn: turn * 0.55,
        bend,
        lean: 0.06 - turn * 0.02,
        nod: 0.75 - finish * 0.55,
        // the right heel up and the knee turned through at the finish
        feet: [[0.23, 0.08 + finish * 0.07, finish * 0.05], [-0.23, 0.08, 0.0]],
        hands: [add(grip, mul(shaft, 0.09)), grip],
        club: Some((grip, shaft, club_len)),
    }
}

thread_local! {
    /// How finely the figure being built is divided (`Who::detail`).
    static DETAIL: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
    /// The cloth the parts being built are made of (`Cloth`), carried to the shader in their texture coordinates.
    static CLOTH: std::cell::Cell<Cloth> = const { std::cell::Cell::new(Cloth::Plain) };
}

/// What a part is made of: the shader (walk_gl.rs) draws its surface from it, so a shirt has its stripes or checks and
/// trousers their weave without a picture of their own. The number is the kind the shader reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cloth {
    Plain = 0,
    Stripes = 1,
    Check = 2,
    Pique = 3,
    Twill = 4,
    Leather = 5,
    Skin = 6,
    Hair = 7,
}

/// The parts built from here on are of `c`.
fn cloth(c: Cloth) {
    CLOTH.with(|k| k.set(c));
}

/// A point's texture coordinates for the shader: `u` round the part and `v` along it, in metres, the cloth's kind in
/// `v`'s 64s (walk_gl.rs).
fn cloth_uv(u: f32, v: f32) -> [f32; 2] {
    let k = CLOTH.with(|k| k.get()) as i32 as f32;
    [u, v.rem_euclid(64.0) + k * 64.0]
}

/// `n` divisions at the figure's detail for a part `size` times the size `n` is meant for: a small part (a pupil, a
/// glint, a finger of the moustache) needs far fewer to look round, so it costs far less.
fn sized(n: usize, least: usize, size: f32) -> usize {
    let d = DETAIL.with(|d| d.get());
    // far off (a golfer a few dozen pixels tall), the fewest that still read as round; nearer, a big part (a limb, the
    // head, the body) never fewer than eight, so it is round rather than boxy at any size it is seen at
    let least = if d < 0.12 {
        least.min(4) - 1
    } else if size >= 0.5 && d >= 0.3 {
        least.max(8)
    } else {
        least
    };
    ((n as f32 * d * size.clamp(0.0, 1.0).sqrt()).round() as usize).max(least)
}

/// Whether the figure is drawn big enough on the screen for its small touches (a glint in the eye, the ends of the
/// mouth) to show; far off they are left out.
fn fine() -> bool {
    DETAIL.with(|d| d.get()) >= 0.3
}

/// The detail a figure is built at for how tall it stands on the screen, in pixels: rounder the bigger it is drawn, so a
/// golfer close up on a 4K screen is smooth and a distant one, or one on a small screen, costs little.
pub fn detail_for(pixels: f32) -> f32 {
    (pixels / 450.0).clamp(0.05, 1.4)
}

/// A tube from `a` to `b` (radii `ra`, `rb`), elliptical: `flat` times as deep (along `across` × the tube) as wide.
#[allow(clippy::too_many_arguments)]
fn tube(out: &mut Vec<Vert>, a: V, b: V, ra: f32, rb: f32, col: [f32; 3], across: V, flat: f32, sides: usize) {
    let sides = sized(sides, 4, ra.max(rb) / 0.08);
    let ax = norm(sub(b, a));
    let mut e1 = sub(across, mul(ax, dot(across, ax)));
    if len(e1) < 1e-4 {
        e1 = cross(ax, [0.0, 0.0, 1.0]);
        if len(e1) < 1e-4 {
            e1 = cross(ax, [1.0, 0.0, 0.0]);
        }
    }
    let e1 = norm(e1);
    let e2 = cross(ax, e1);
    let c = [col[0], col[1], col[2], 1.0];
    let ring = |i: usize| {
        let t = TAU * i as f32 / sides as f32;
        let (s, k) = t.sin_cos();
        (add(mul(e1, k), mul(e2, s * flat)), norm(add(mul(e1, k * flat), mul(e2, s))))
    };
    let v = |p: V, n: V, t: [f32; 2]| {
        let mut v = Vert::new(p[0], p[1], p[2], t[0], t[1]).col(c);
        v.normal = n;
        v
    };
    // round the tube and along it, in metres, for the cloth's pattern
    let round = TAU * (ra + rb) * 0.5 * (1.0 + flat) * 0.5;
    let long = len(sub(b, a));
    let at = |i: usize, along: f32| cloth_uv(round * i as f32 / sides as f32, along);
    for i in 0..sides {
        let (o0, n0) = ring(i);
        let (o1, n1) = ring(i + 1);
        let (p0, p1, p2, p3) = (add(a, mul(o0, ra)), add(a, mul(o1, ra)), add(b, mul(o1, rb)), add(b, mul(o0, rb)));
        let (t0, t1, t2, t3) = (at(i, 0.0), at(i + 1, 0.0), at(i + 1, long), at(i, long));
        out.extend([v(p0, n0, t0), v(p1, n1, t1), v(p2, n1, t2), v(p0, n0, t0), v(p2, n1, t2), v(p3, n0, t3)]);
        // the ends closed
        let (ta, tb) = (at(0, 0.0), at(0, long));
        out.extend([v(a, mul(ax, -1.0), ta), v(p1, mul(ax, -1.0), t1), v(p0, mul(ax, -1.0), t0)]);
        out.extend([v(b, ax, tb), v(p3, ax, t3), v(p2, ax, t2)]);
    }
}

/// A ball at `c`, its radii `r` along x, y and z.
fn ball(out: &mut Vec<Vert>, c: V, r: V, col: [f32; 3]) {
    let big = r[0].max(r[1]).max(r[2]) / 0.08;
    let (seg, rings) = (sized(22, 6, big), sized(14, 4, big));
    grid(out, seg, rings, true, true, col, r[0].max(r[2]), |i, j| {
        let (th, ph) = (TAU * i as f32 / seg as f32, PI * j as f32 / rings as f32);
        let n = [ph.sin() * th.cos(), ph.cos(), ph.sin() * th.sin()];
        ([c[0] + n[0] * r[0], c[1] + n[1] * r[1], c[2] + n[2] * r[2]], norm([n[0] / r[0], n[1] / r[1], n[2] / r[2]]))
    });
}

/// Part of an ellipsoid at `c` in the frame `ax` (right, up, forward) with radii `r` along them: from the top down to
/// polar angle `ph1` (PI for the whole of it) and round from `th0` to `th1` (0 the right, PI / 2 the front). Smooth: the
/// head and what fits on it.
#[allow(clippy::too_many_arguments)]
fn shell(out: &mut Vec<Vert>, c: V, ax: [V; 3], r: V, ph0: f32, ph1: f32, th: (f32, f32), col: [f32; 3]) {
    let big = r[0].max(r[1]).max(r[2]) / 0.16;
    let (seg, rings) = (sized(30, 6, big), sized(17, 4, big));
    grid(out, seg, rings, ph0 <= 1e-4, ph1 >= PI - 1e-4, col, r[0].max(r[2]), |i, j| {
        let th = th.0 + (th.1 - th.0) * i as f32 / seg as f32;
        let ph = ph0 + (ph1 - ph0) * j as f32 / rings as f32;
        let n = [ph.sin() * th.cos(), ph.cos(), ph.sin() * th.sin()];
        let p = add(c, add(add(mul(ax[0], n[0] * r[0]), mul(ax[1], n[1] * r[1])), mul(ax[2], n[2] * r[2])));
        (p, norm(add(add(mul(ax[0], n[0] / r[0]), mul(ax[1], n[1] / r[1])), mul(ax[2], n[2] / r[2]))))
    });
}

/// A surface from a (seg + 1) x (rings + 1) grid of points and normals, each worked out once; where the first or last
/// row is a pole (`top`, `bottom`) its quads are single triangles.
#[allow(clippy::too_many_arguments)]
fn grid(
    out: &mut Vec<Vert>,
    seg: usize,
    rings: usize,
    top: bool,
    bottom: bool,
    col: [f32; 3],
    span: f32,
    at: impl Fn(usize, usize) -> (V, V),
) {
    let col = [col[0], col[1], col[2], 1.0];
    let w = seg + 1;
    let pts: Vec<Vert> = (0..=rings)
        .flat_map(|j| (0..=seg).map(move |i| (i, j)))
        .map(|(i, j)| {
            let (p, n) = at(i, j);
            let t = cloth_uv(TAU * span * i as f32 / seg as f32, PI * span * j as f32 / rings as f32);
            let mut v = Vert::new(p[0], p[1], p[2], t[0], t[1]).col(col);
            v.normal = n;
            v
        })
        .collect();
    out.reserve(seg * rings * 6);
    for j in 0..rings {
        for i in 0..seg {
            let (a, b, cc, d) = (pts[j * w + i], pts[j * w + i + 1], pts[(j + 1) * w + i + 1], pts[(j + 1) * w + i]);
            if j == 0 && top {
                out.extend([a, cc, d]);
            } else if j + 1 == rings && bottom {
                out.extend([a, b, cc]);
            } else {
                out.extend([a, b, cc, a, cc, d]);
            }
        }
    }
}

/// How much smaller than life the body is drawn below the head (the club with it).
const BODY: f32 = 0.88;
/// Below the hips, how much shorter still.
const LEGS: f32 = 0.93;
/// Where the legs end, before they are shortened.
const HIP_Y: f32 = 0.92;
/// How much bigger than its modelled size the head is drawn.
const HEAD: f32 = 1.15;

/// A height of the body as drawn: the legs shortened, the rest moved down onto them (the ground stays where it is).
fn stubby(y: f32) -> f32 {
    if y < HIP_Y {
        y * LEGS
    } else {
        y - HIP_Y * (1.0 - LEGS)
    }
}

/// Where the ball lies for a golfer at the ball (x to their right, z forward, metres): under the club's head at address.
/// The game keeps the ball at the golfer's own place until it is struck, so the body is drawn back from it by this much.
pub fn ball_spot(act: Act, club: i32, female: bool) -> [f32; 2] {
    let a = match act {
        Act::Putt | Act::Address { putt: true } => Act::Address { putt: true },
        Act::Swing { reach } => Act::Swing { reach },
        _ => Act::Address { putt: false },
    };
    let p = pose(a, 0.0, 0.0, club_length(club));
    let Some((grip, shaft, l)) = p.club else { return [0.0, 0.6] };
    let h = add(grip, mul(shaft, l));
    let tall = if female { 0.94 } else { 1.0 } * BODY;
    [h[0] * tall, h[2] * tall]
}

fn club_length(club: i32) -> f32 {
    match club {
        13 => 0.86,
        0 => 1.12,
        1..=3 => 1.05,
        _ => 0.96,
    }
}

/// A golfer's facial hair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Beard {
    None,
    Moustache,
    Goatee,
    /// A goatee and a moustache.
    Both,
    /// A short beard: a full one, its colour half the skin's.
    Stubble,
    Full,
}

impl Beard {
    /// The facial hair of each of the game's stock men's heads (0..18), as their portraits show it.
    pub fn of_stock_head(head: u8) -> Beard {
        use Beard::*;
        const HEADS: [Beard; 19] = [
            Full, Full, Moustache, Goatee, None, Both, Stubble, Full, Goatee, None, Moustache, None, None, None, Goatee, Goatee, None,
            None, Goatee,
        ];
        HEADS.get(head as usize).copied().unwrap_or(None)
    }
}

/// Who a golfer is, for their figure: their colours, their build (from their body's pictures: `height` and `girth` about
/// 1), what kind of golfer they are (`Golfer::kind`: 0x20 a pro, 0x40 the commissioner, 0x60 a CEO, 0x80 the heiress)
/// and their mood (-10..10).
#[derive(Clone, Copy, Debug)]
pub struct Who {
    pub look: Look,
    pub height: f32,
    pub girth: f32,
    pub kind: u8,
    pub mood: i32,
    /// The facial hair the golfer's own portrait shows, and its colour.
    pub beard: Beard,
    pub whiskers: [f32; 3],
    /// The player's own pro: drawn as the star of the course (aviators on his visor, a gold star on his shirt, a white
    /// glove, a confident grin), so he stands out from every other golfer.
    pub star: bool,
    /// How finely the figure is divided: 1 for a golfer about 360 pixels tall on the screen (`detail_for`).
    pub detail: f32,
}

impl Who {
    #[cfg(test)]
    pub fn plain(look: Look) -> Who {
        Who { look, height: 1.0, girth: 1.0, kind: 0, mood: 0, beard: Beard::None, whiskers: [0.3, 0.2, 0.1], detail: 1.0, star: false }
    }
}

/// A golfer's body in metres, standing at the origin, as `figure`, for someone of ordinary build and mood.
#[cfg(test)]
pub fn body(look: &Look, act: Act, t: f32, time: f32, club: i32) -> Vec<Vert> {
    figure(&Who::plain(*look), act, t, time, club)
}

/// A golfer's figure in metres, standing at the origin: x to their right, y up, z forward (`at_ball` acts: towards the
/// ball, the target on their left). `t` is how far through the clip, `time` seconds for idle movement, `club` the club
/// in hand (0..13, 13 the putter). Drawn as a cartoon is: a big head with a face that shows the mood, big hands and
/// feet, and the build of the golfer's own pictures.
pub fn figure(who: &Who, act: Act, t: f32, time: f32, club: i32) -> Vec<Vert> {
    DETAIL.with(|d| d.set(who.detail.clamp(0.05, 3.0)));
    let look = &who.look;
    let mut p = pose(act, t, time, club_length(club));
    // a club held at the side or dragged rests on the ground rather than through it
    if let Some((grip, shaft, l)) = p.club {
        if !act.at_ball() && grip[1] + shaft[1] * l < 0.02 {
            let dy = ((0.02 - grip[1]) / l).clamp(-0.999, 0.999);
            let h = (shaft[0] * shaft[0] + shaft[2] * shaft[2]).sqrt().max(1e-4);
            let k = (1.0 - dy * dy).sqrt() / h;
            p.club = Some((grip, [shaft[0] * k, dy, shaft[2] * k], l));
        }
    }
    let mut out = Vec::with_capacity(6000);
    let o = &mut out;
    let up = [0.0, 1.0, 0.0];
    let tall = if look.female { 0.94 } else { 1.0 } * who.height.clamp(0.85, 1.15);
    let g = who.girth.clamp(0.8, 1.35);
    let kind = if who.star { 0x20 } else { who.kind & 0xe0 };
    // brown leather shoes and belt, a steel buckle
    let shoe = if who.star { [0.95, 0.95, 0.93] } else { [0.36, 0.17, 0.1] };
    let sole = [0.2, 0.12, 0.08];
    let belt = [0.42, 0.26, 0.14];
    let steel = [0.72, 0.73, 0.76];
    // the shirt's cloth: a business shirt striped, the pro's polo a knit, the rest by their colour stripes, checks or knit
    let shirt_cloth = if who.star {
        Cloth::Pique
    } else if matches!(kind, 0x40 | 0x60) {
        Cloth::Stripes
    } else {
        let h = (look.shirt[0] * 7.3 + look.shirt[1] * 13.1 + look.shirt[2] * 29.7 + look.pants[0] * 3.7) * 10.0;
        [Cloth::Stripes, Cloth::Check, Cloth::Pique][(h as i32).rem_euclid(3) as usize]
    };

    // the hips and the spine: bent forward and leant, the shoulders turned on the hips
    let hip_right = yaw([1.0, 0.0, 0.0], -p.hip_turn);
    let chest_turn = p.hip_turn + p.chest_turn;
    let chest_right = yaw([1.0, 0.0, 0.0], -chest_turn);
    let chest_fwd = yaw([0.0, 0.0, 1.0], -chest_turn);
    let spine = norm(add(add(mul(up, p.bend.cos()), mul(chest_fwd, p.bend.sin())), mul(chest_right, p.lean.sin())));
    let hips = p.hips;
    let waist = add(hips, mul(spine, 0.12));
    let shoulders = add(hips, mul(spine, 0.5));
    let neck = add(shoulders, mul(spine, 0.07));
    let face = norm(add(mul(chest_fwd, p.nod.cos()), mul(up, -p.nod.sin())));

    // legs: from the hip joints to the feet, the knees forward; big shoes
    for (k, s) in [(0usize, 1.0f32), (1, -1.0)] {
        let hip = add(hips, mul(hip_right, s * 0.1 * g));
        let foot = p.feet[k];
        let hint = add(yaw([0.0, 0.0, 1.0], -p.hip_turn * 0.5), [s * 0.1, 0.0, 0.0]);
        let (knee, ankle) = reach(hip, foot, 0.45, 0.43, hint);
        // trousers: straight, loose legs falling to a hem over the shoe
        cloth(Cloth::Twill);
        ball(o, hip, [0.11 * g; 3], look.pants);
        tube(o, hip, knee, 0.11 * g, 0.088 * g.sqrt(), look.pants, hip_right, 0.92, 22);
        ball(o, knee, [0.088 * g.sqrt(); 3], look.pants);
        tube(o, knee, add(ankle, [0.0, -0.02, 0.0]), 0.086 * g.sqrt(), 0.084, look.pants, hip_right, 0.92, 22);
        // leather shoes: a rounded upper on a darker sole, the toe a little long
        cloth(Cloth::Leather);
        let toe = add(ankle, [s * 0.015, -0.02, 0.19]);
        let heel = add(ankle, [0.0, -0.02, -0.06]);
        tube(o, heel, toe, 0.06, 0.055, shoe, [0.0, 1.0, 0.0], 0.75, 12);
        ball(o, toe, [0.058, 0.045, 0.065], shoe);
        ball(o, heel, [0.058, 0.05, 0.05], shoe);
        let drop = [0.0, -0.04, 0.0];
        tube(o, add(heel, drop), add(toe, add(drop, [0.0, 0.0, 0.03])), 0.066, 0.066, sole, [1.0, 0.0, 0.0], 0.25, 10);
    }
    // the body: trousers to the belt, the shirt to the shoulders, wider than deep
    cloth(Cloth::Twill);
    tube(o, sub(hips, mul(spine, 0.06)), waist, 0.19 * g, 0.19 * g, look.pants, hip_right, 0.72, 20);
    cloth(Cloth::Leather);
    tube(o, waist, add(waist, mul(spine, 0.035)), 0.197 * g, 0.197 * g, belt, chest_right, 0.72, 20);
    cloth(Cloth::Plain);
    if fine() {
        // the buckle, square on the belt's front
        let at = add(add(waist, mul(spine, 0.018)), mul(chest_fwd, 0.142 * g));
        let flat = norm(cross(chest_right, spine));
        tube(o, sub(at, mul(chest_right, 0.03)), add(at, mul(chest_right, 0.03)), 0.024, 0.024, steel, flat, 0.3, 4);
    }
    cloth(shirt_cloth);
    tube(o, add(waist, mul(spine, 0.03)), shoulders, 0.2 * g, 0.22 * g.sqrt(), look.shirt, chest_right, 0.72, 20);
    ball(o, add(shoulders, mul(spine, -0.02)), [0.2 * g.sqrt(), 0.08, 0.14], look.shirt);
    if fine() {
        // the collar: two flaps folded down either side of the neck, and the placket's buttons down the front
        let collar = mix(look.shirt, [1.0; 3], 0.12);
        for s in [-1.0f32, 1.0] {
            let a = add(add(shoulders, mul(spine, 0.06)), add(mul(chest_right, s * 0.07), mul(chest_fwd, -0.01)));
            let b = add(add(shoulders, mul(spine, 0.0)), add(mul(chest_right, s * 0.04), mul(chest_fwd, 0.105)));
            tube(o, a, b, 0.03, 0.012, collar, chest_fwd, 0.3, 6);
        }
        cloth(Cloth::Plain);
        let button = mix(look.shirt, [1.0; 3], 0.55);
        for i in 0..3 {
            let at = add(add(shoulders, mul(spine, -0.06 - 0.1 * i as f32)), mul(chest_fwd, (0.135 + 0.01 * i as f32) * g.sqrt()));
            ball(o, at, [0.011, 0.011, 0.006], button);
        }
    }
    // a businessman's belly and tie
    cloth(shirt_cloth);
    let belly = if matches!(kind, 0x40 | 0x60) { 1.0 } else { (g - 1.0).max(0.0) * 3.0 };
    if belly > 0.0 {
        ball(o, add(add(waist, mul(spine, 0.12)), mul(chest_fwd, 0.06 * g)), [0.16 * g, 0.16, 0.12 + 0.03 * belly], look.shirt);
    }
    cloth(Cloth::Plain);
    if matches!(kind, 0x40 | 0x60) {
        let tie = if kind == 0x60 { [0.12, 0.16, 0.45] } else { [0.6, 0.1, 0.12] };
        let a = add(add(shoulders, mul(spine, -0.03)), mul(chest_fwd, 0.125 * g.sqrt()));
        let b = add(add(waist, mul(spine, 0.14)), mul(chest_fwd, 0.2 * g));
        tube(o, a, b, 0.028, 0.045, tie, chest_right, 0.25, 6);
        ball(o, a, [0.03, 0.028, 0.02], tie);
    }
    cloth(Cloth::Skin);
    tube(o, shoulders, add(neck, mul(spine, 0.05)), 0.075, 0.068, look.skin, chest_right, 1.0, 22);
    cloth(Cloth::Plain);
    if who.star {
        // a big gold star on the pro's chest
        let at = add(add(add(shoulders, mul(spine, -0.14)), mul(chest_fwd, 0.16 * g.sqrt())), mul(chest_right, -0.075));
        for i in 0..5 {
            let a = TAU * i as f32 / 5.0;
            let tip = add(add(at, mul(chest_right, a.sin() * 0.075)), mul(spine, a.cos() * 0.075));
            tube(o, at, tip, 0.03, 0.006, [0.98, 0.78, 0.18], chest_fwd, 0.35, 6);
        }
        ball(o, at, [0.03, 0.03, 0.012], [0.98, 0.78, 0.18]);
    }
    if kind == 0x80 {
        // the heiress's pearls
        for i in 0..11 {
            let a = (i as f32 / 10.0 - 0.5) * 2.6;
            let at = add(add(add(neck, mul(spine, -0.035)), mul(chest_fwd, a.cos() * 0.075)), mul(chest_right, a.sin() * 0.075));
            ball(o, at, [0.014; 3], [0.97, 0.96, 0.92]);
        }
    }

    // arms: short sleeves, then the arm and a big hand; the elbows down and out
    for (k, s) in [(0usize, 1.0f32), (1, -1.0)] {
        let shoulder = add(add(shoulders, mul(chest_right, s * 0.2 * g.sqrt())), mul(spine, -0.02));
        let hint = add(add([0.0, -1.0, 0.0], mul(chest_right, s * 0.6)), mul(chest_fwd, -0.3));
        let (elbow, wrist) = reach(shoulder, p.hands[k], 0.3, 0.28, hint);
        let sleeve = add(shoulder, mul(sub(elbow, shoulder), 0.5));
        cloth(shirt_cloth);
        ball(o, shoulder, [0.098; 3], look.shirt);
        tube(o, shoulder, sleeve, 0.098, 0.092, look.shirt, chest_right, 1.0, 22);
        cloth(Cloth::Skin);
        tube(o, sleeve, elbow, 0.07, 0.066, look.skin, chest_right, 1.0, 22);
        ball(o, elbow, [0.066; 3], look.skin);
        tube(o, elbow, wrist, 0.064, 0.056, look.skin, chest_right, 1.0, 22);
        // the pro's white golf glove on his leading (left) hand
        // (bare hands in the skin's colour: the outfit's arm colour reads as a dark lump on the grip)
        let hand = if who.star && k == 1 { [0.97, 0.97, 0.95] } else { look.skin };
        ball(o, wrist, [0.084, 0.09, 0.084], hand);
    }
    cloth(Cloth::Plain);
    // the club: grip, shaft and head
    if let Some((grip, shaft, l)) = p.club {
        let top = sub(grip, mul(shaft, 0.06));
        let end = add(grip, mul(shaft, l));
        tube(o, top, add(grip, mul(shaft, 0.22)), 0.018, 0.015, [0.1, 0.1, 0.1], up, 1.0, 6);
        tube(o, add(grip, mul(shaft, 0.22)), end, 0.011, 0.008, [0.72, 0.73, 0.75], up, 1.0, 6);
        let face_dir = norm(cross(shaft, [1.0, 0.0, 0.0]));
        let toe = norm(cross(face_dir, shaft));
        let head_col = if club <= 2 { [0.15, 0.15, 0.17] } else { [0.78, 0.79, 0.82] };
        let size = if club <= 2 { [0.07, 0.042, 0.06] } else { [0.055, 0.025, 0.016] };
        let c = add(end, mul(toe, -size[0] * 0.8));
        tube(o, add(c, mul(toe, -size[0])), add(c, mul(toe, size[0])), size[1], size[1], head_col, face_dir, size[2] / size[1], 8);
    }
    // the body drawn smaller than life under the big head
    for v in out.iter_mut() {
        v.pos = mul([v.pos[0], stubby(v.pos[1]), v.pos[2]], BODY);
    }
    let neck = mul([neck[0], stubby(neck[1]), neck[2]], BODY);
    let at = out.len();
    head(&mut out, who, act, neck, spine, face);
    for v in out[at..].iter_mut() {
        v.pos = add(neck, mul(sub(v.pos, neck), HEAD));
    }
    if tall != 1.0 {
        for v in out.iter_mut() {
            v.pos = mul(v.pos, tall);
        }
    }
    out
}

/// The head, as a cartoon's: big and round, with big eyes, a nose, ears, brows and a mouth that show the mood, the hair
/// and the hat fitted to it. `neck` is the top of the neck, `spine` the way up the neck, `face` the way the face looks.
fn head(o: &mut Vec<Vert>, who: &Who, act: Act, neck: V, spine: V, face: V) {
    cloth(Cloth::Skin);
    let look = &who.look;
    let kind = if who.star { 0x20 } else { who.kind & 0xe0 };
    let dark = [0.06, 0.05, 0.05];
    let white = [0.98, 0.98, 0.96];
    let mood = match act {
        Act::Happy | Act::Pump => 1.0,
        Act::Sad => -1.0,
        Act::Fail => -2.0,
        _ if who.star => ((who.mood as f32 / 6.0) + 0.6).clamp(-1.0, 1.0),
        _ => (who.mood as f32 / 6.0).clamp(-1.0, 1.0),
    };
    // the head's frame: right, up (along the neck, a little back), forward
    let side = norm(cross(spine, face));
    let face = norm(cross(side, spine));
    let crown = spine;
    let ax = [side, crown, face];
    let r = [0.15, 0.165, 0.15];
    // (a little neck showing under the chin)
    let c = add(add(neck, mul(crown, r[1] * 0.92 + 0.025)), mul(face, 0.015));
    // a point by the head's frame: `x` right, `y` up, `z` forward, from the head's middle
    let at = |x: f32, y: f32, z: f32| add(add(add(c, mul(side, x)), mul(crown, y)), mul(face, z));
    // on the head's surface (ahead), at `x` right and `y` up, `out` beyond it
    let on = |x: f32, y: f32, out: f32| {
        let (u, v) = (x / r[0], y / r[1]);
        let z = r[2] * (1.0 - u * u - v * v).max(0.0).sqrt();
        at(x, y, z + out)
    };
    let full = (0.0, TAU);
    shell(o, c, ax, r, 0.0, PI, full, look.skin);
    // ears and a big round nose
    for sx in [-1.0f32, 1.0] {
        shell(o, at(sx * r[0] * 0.96, -0.01, -0.005), ax, [0.03, 0.045, 0.035], 0.0, PI, full, look.skin);
    }
    shell(o, on(0.0, -0.03, 0.0), ax, [0.027, 0.025, 0.03], 0.0, PI, full, look.skin);
    // full cheeks, a little rosy, and a rounded chin: a soft, friendly face
    for sx in [-1.0f32, 1.0] {
        shell(o, at(sx * 0.07, -0.055, 0.06), ax, [0.055, 0.05, 0.055], 0.0, PI, full, look.skin);
        if fine() {
            shell(o, on(sx * 0.085, -0.045, -0.004), ax, [0.026, 0.017, 0.008], 0.0, PI, full, mix(look.skin, [0.95, 0.45, 0.45], 0.28));
        }
    }
    shell(o, at(0.0, -0.11, 0.06), ax, [0.07, 0.05, 0.07], 0.0, PI, full, look.skin);
    cloth(Cloth::Plain);
    // eyes: white ovals with dark brown irises and a glint, looking where the head looks
    let iris = [0.16, 0.1, 0.07];
    for sx in [-1.0f32, 1.0] {
        let e = on(sx * 0.05, 0.02, -0.01);
        shell(o, e, ax, [0.027, 0.033, 0.018], 0.0, PI, full, white);
        let pupil = add(e, add(mul(face, 0.014), mul(side, sx * -0.003)));
        shell(o, pupil, ax, [0.015, 0.019, 0.008], 0.0, PI, full, iris);
        shell(o, add(pupil, mul(face, 0.004)), ax, [0.008, 0.01, 0.005], 0.0, PI, full, dark);
        if fine() {
            shell(o, add(add(pupil, mul(face, 0.008)), add(mul(side, 0.006), mul(crown, 0.008))), ax, [0.005; 3], 0.0, PI, full, white);
        }
        // the brows: raised when happy, down at the middle when cross, down at the outside when sad
        let tilt = if mood < -1.5 {
            -0.016
        } else if mood < -0.3 {
            0.012
        } else {
            0.0
        };
        let lift = if mood > 0.3 { 0.008 } else { 0.0 };
        let brow = if look.female { look.hair } else { mul(look.hair, 0.6) };
        tube(o, on(sx * 0.025, 0.07 + lift + tilt, 0.004), on(sx * 0.08, 0.068 + lift - tilt, 0.002), 0.008, 0.007, brow, crown, 0.7, 6);
    }
    // the facial hair, shaped to the head: a moustache under the nose, a goatee on the chin, a beard along the jaw
    let whisk = if who.beard == Beard::Stubble { mix(who.whiskers, look.skin, 0.45) } else { who.whiskers };
    let jaw = matches!(who.beard, Beard::Full | Beard::Stubble);
    if jaw {
        // from ear to ear round the front of the jaw and under the chin, the sideburns up to the ears
        let k = if who.beard == Beard::Full { 1.07 } else { 1.025 };
        shell(o, c, ax, [r[0] * k, r[1] * (k - 0.01), r[2] * k], 1.95, 2.75, (-0.15, PI + 0.15), whisk);
        for sx in [-1.0f32, 1.0] {
            let a = at(sx * r[0] * 0.93, 0.0, -0.01);
            tube(o, a, at(sx * r[0] * 0.86, -0.075, 0.035), 0.022, 0.03, whisk, face, 0.6, 8);
        }
    }
    if matches!(who.beard, Beard::Moustache | Beard::Both) || who.beard == Beard::Full {
        // two rounded lobes, drooping a little at the ends
        for sx in [-1.0f32, 1.0] {
            shell(o, on(sx * 0.024, -0.054, 0.006), ax, [0.03, 0.014, 0.016], 0.0, PI, full, whisk);
            shell(o, on(sx * 0.046, -0.062, 0.002), ax, [0.014, 0.014, 0.012], 0.0, PI, full, whisk);
        }
    }
    if matches!(who.beard, Beard::Goatee | Beard::Both) {
        shell(o, on(0.0, -0.118, 0.004), ax, [0.03, 0.026, 0.02], 0.0, PI, full, whisk);
    }
    // the mouth: a smile, a line or a frown, one smooth stroke, rounded at its ends
    let mouth_out = if jaw { 0.014 } else { 0.008 };
    let mouth = |u: f32| {
        let bend = mood.clamp(-1.0, 1.0) * 0.022 * (1.0 - u * u * 4.0);
        on(u * 0.08, -0.075 - bend, mouth_out)
    };
    let lip = [0.4, 0.1, 0.1];
    let strokes = if fine() { 8 } else { 3 };
    for i in 0..strokes {
        let (u0, u1) = (i as f32 / strokes as f32 - 0.5, (i + 1) as f32 / strokes as f32 - 0.5);
        tube(o, mouth(u0), mouth(u1), 0.0085, 0.0085, lip, face, 0.8, 8);
    }
    if fine() {
        for u in [-0.5, 0.5] {
            shell(o, mouth(u), ax, [0.0085; 3], 0.0, PI, full, lip);
        }
    }
    if kind == 0x60 {
        // the CEO's cigar, in the corner of the mouth
        let a = on(0.03, -0.075, 0.0);
        let b = add(add(a, mul(face, 0.1)), add(mul(side, 0.03), mul(crown, -0.015)));
        tube(o, a, b, 0.013, 0.013, [0.38, 0.22, 0.1], side, 1.0, 12);
        ball(o, b, [0.014; 3], [1.0, 0.45, 0.1]);
    }
    // the hair, then the hat: fitted to the head, the brim above the brows
    cloth(Cloth::Hair);
    let rim = 0.085 / r[1];
    let hr = |k: f32| [r[0] * k, r[1] * k, r[2] * k];
    if look.female {
        // over the crown down to the brows in front, and long at the back
        shell(o, c, ax, hr(1.06), 0.0, rim.acos() + 0.08, full, look.hair);
        shell(o, add(c, mul(face, -0.02)), ax, [r[0] * 1.12, r[1] * 1.08, r[2] * 1.1], 0.0, 2.4, (PI, TAU), look.hair);
        shell(o, at(0.0, -0.12, -0.12), ax, [0.07, 0.09, 0.06], 0.0, PI, full, look.hair);
    } else {
        // short at the sides and back
        shell(o, c, ax, hr(1.06), 0.0, 1.95, (PI * 1.05, TAU - PI * 0.05), look.hair);
    }
    cloth(Cloth::Plain);
    match kind {
        0x80 => {
            // the heiress's sun hat: a low crown and a wide brim
            let base = at(0.0, r[1] * 0.62, 0.0);
            shell(o, add(c, mul(crown, 0.012)), ax, hr(1.1), 0.0, 0.95, full, look.hat);
            tube(o, base, add(base, mul(crown, 0.012)), r[0] * 2.0, r[0] * 1.95, look.hat, face, 1.0, 20);
            tube(o, add(base, mul(crown, 0.012)), add(base, mul(crown, 0.035)), r[0] * 1.1, r[0] * 1.08, [0.9, 0.3, 0.45], face, 1.0, 16);
        }
        0x20 => {
            // a pro's visor: a band and a long peak, the hair showing over it
            if !look.female {
                shell(o, c, ax, hr(1.05), 0.0, rim.acos(), full, look.hair);
            }
            let y = r[1] * rim;
            let band = r[0] * (1.0 - rim * rim).sqrt() * 1.08;
            let band_col = if who.star { [0.95, 0.76, 0.25] } else { look.hat };
            tube(o, at(0.0, y, 0.0), at(0.0, y + 0.03, 0.0), band, band, band_col, face, 1.0, 18);
            tube(o, on(0.0, y + 0.012, -0.02), add(on(0.0, y + 0.012, 0.0), mul(face, 0.12)), 0.12, 0.1, look.hat, side, 0.1, 12);
            if who.star {
                // aviators pushed up on the visor's band: two dark teardrop lenses in gold rims and the bridge between
                let gold = [0.95, 0.76, 0.25];
                for sx in [-1.0f32, 1.0] {
                    let l = on(sx * 0.055, y + 0.045, 0.012);
                    shell(o, l, ax, [0.04, 0.03, 0.012], 0.0, PI, full, gold);
                    shell(o, add(l, mul(face, 0.006)), ax, [0.034, 0.025, 0.01], 0.0, PI, full, [0.1, 0.14, 0.2]);
                }
                tube(o, on(-0.02, y + 0.052, 0.018), on(0.02, y + 0.052, 0.018), 0.005, 0.005, gold, crown, 1.0, 6);
            }
        }
        _ if look.female => {}
        _ => {
            // a cap: its crown fitted over the head down to just above the brows, a button on top and a peak
            shell(o, c, ax, hr(1.07), 0.0, rim.acos(), full, look.hat);
            shell(o, at(0.0, r[1] * 1.07, 0.0), ax, [0.018; 3], 0.0, PI, full, look.hat);
            let y = r[1] * rim + 0.004;
            tube(o, on(0.0, y, -0.03), add(on(0.0, y, 0.0), mul(face, 0.11)), 0.125, 0.105, look.hat, side, 0.1, 12);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> Look {
        Look::plain(&sg_core::bodies::Outfit { female: false, shirt: 1, pants: 2, skin: 0, hat: 3, hair: 0, alt_skin: 0 })
    }

    fn top(v: &[Vert]) -> f32 {
        v.iter().map(|p| p.pos[1]).fold(f32::MIN, f32::max)
    }

    /// How long a figure takes to build, and how big it is, at each detail (cargo test --release -- --ignored cost).
    #[test]
    #[ignore]
    fn cost() {
        for d in [0.05f32, 0.15, 0.3, 0.6, 1.0, 1.6, 2.0] {
            let who = Who { detail: d, beard: Beard::Full, ..Who::plain(look()) };
            let n = 200;
            let t0 = std::time::Instant::now();
            let mut verts = 0;
            for i in 0..n {
                let mut v = figure(&who, Act::Swing { reach: 1.0 }, i as f32 / n as f32, 0.0, 4);
                let mut ink = Vec::new();
                crate::walk_toon::outline(&v, 0.01, &mut ink);
                crate::walk_toon::cel(&mut v, [0.3, 0.8, 0.5]);
                verts = v.len() + ink.len();
            }
            let us = t0.elapsed().as_secs_f64() * 1e6 / n as f64;
            println!("detail {d}: {verts} vertices, {us:.0} us a golfer");
        }
    }

    #[test]
    fn a_standing_golfer_is_a_persons_height_on_the_ground() {
        let v = body(&look(), Act::Stand, 0.0, 0.0, 0);
        assert!(v.len() % 3 == 0);
        let (lo, hi) = (v.iter().map(|p| p.pos[1]).fold(f32::MAX, f32::min), top(&v));
        assert!(lo.abs() < 0.04, "feet at {lo}");
        // a person's height, the head a little big
        assert!((1.55..1.8).contains(&hi), "height {hi}");
        let wide = v.iter().map(|p| p.pos[0].abs()).fold(0.0, f32::max);
        assert!(wide < 0.45, "wide {wide}");
    }

    #[test]
    fn every_act_keeps_the_golfer_together_through_its_clip() {
        let acts = [
            Act::Stand,
            Act::Walk { tired: false },
            Act::Walk { tired: true },
            Act::Fidget,
            Act::Happy,
            Act::Sad,
            Act::Pump,
            Act::Fail,
            Act::Swing { reach: 1.0 },
            Act::Swing { reach: 0.55 },
            Act::Putt,
            Act::Address { putt: false },
            Act::Address { putt: true },
            Act::LineUp,
            Act::Point,
            Act::Lean(1.0),
            Act::LookUp,
            Act::Sit,
            Act::Shake,
        ];
        for a in acts {
            for k in 0..=10 {
                let v = body(&look(), a, k as f32 / 10.0, k as f32, 4);
                assert!(v.iter().all(|p| p.pos.iter().all(|c| c.is_finite())), "{a:?}");
                let far = v.iter().map(|p| len(p.pos)).fold(0.0, f32::max);
                assert!(far < 3.0, "{a:?} at {k}: {far}");
                assert!(v.iter().all(|p| p.pos[1] > -0.1), "{a:?} at {k} below the ground");
            }
        }
    }

    #[test]
    fn the_swing_goes_back_over_the_right_shoulder_and_through_to_the_left() {
        // the club's head: furthest point of the club's colour from the body's middle
        let head = |t: f32| {
            let v = body(&look(), Act::Swing { reach: 1.0 }, t, 0.0, 4);
            v.iter().filter(|p| (p.color[0] - 0.78).abs() < 0.005 && (p.color[2] - 0.82).abs() < 0.005).map(|p| p.pos).fold(
                [0.0; 3],
                |a, p| {
                    if len(p) > len(a) {
                        p
                    } else {
                        a
                    }
                },
            )
        };
        let (address, top, finish) = (head(0.0), head(0.6), head(0.98));
        assert!(address[1] < 0.3, "at the ball {address:?}");
        // (at the top, across the back of the neck towards the target)
        assert!(top[1] > 0.85 && top[0] < 0.0 && top[2] < 0.0, "at the top {top:?}");
        // (at the finish, hanging down the back)
        assert!(finish[2] < -0.2 && finish[1] > 0.6, "at the finish {finish:?}");
    }

    #[test]
    fn colours_come_from_the_palettes_ranges() {
        let mut pal = [0u8; 768];
        for i in 0..20 {
            pal[i * 3] = 200; // the shirt red
            pal[(20 + i) * 3 + 2] = 200; // the trousers blue
        }
        let l = Look::from_palette(&pal, false);
        assert!(l.shirt[0] > 0.7 && l.shirt[2] < 0.1);
        assert!(l.pants[2] > 0.7 && l.pants[0] < 0.1);
    }
}

#[cfg(test)]
mod dump {
    use super::*;

    /// Writes swing frames' triangles for looking at outside the game: SG_GOLFER_DUMP=DIR cargo test dump_swing -- --ignored
    #[test]
    #[ignore]
    fn dump_swing() {
        let Ok(dir) = std::env::var("SG_GOLFER_DUMP") else { return };
        let look = Look::plain(&sg_core::bodies::Outfit { female: false, shirt: 0, pants: 2, skin: 1, hat: 1, hair: 0, alt_skin: 1 });
        let o = |female: bool, shirt: u8, pants: u8, skin: u8, hat: u8| sg_core::bodies::Outfit {
            female,
            shirt,
            pants,
            skin,
            hat,
            hair: 2,
            alt_skin: skin,
        };
        let cast = [
            ("joe", Who { mood: 8, ..Who::plain(Look::plain(&o(false, 1, 2, 0, 3))) }, Act::Happy, 0.25),
            ("ceo", Who { kind: 0x60, girth: 1.2, ..Who::plain(Look::plain(&o(false, 0, 6, 1, 6))) }, Act::Stand, 0.0),
            ("heiress", Who { kind: 0x80, mood: 5, ..Who::plain(Look::plain(&o(true, 7, 0, 0, 4))) }, Act::Stand, 0.0),
            ("pro", Who { kind: 0x20, mood: 3, ..Who::plain(Look::plain(&o(false, 2, 0, 2, 0))) }, Act::Pump, 0.0),
            ("cross", Who { mood: -8, ..Who::plain(Look::plain(&o(false, 4, 3, 1, 2))) }, Act::Fail, 0.8),
            ("sad", Who { mood: -6, girth: 0.85, height: 1.1, ..Who::plain(Look::plain(&o(true, 3, 5, 3, 1))) }, Act::Sad, 0.5),
            ("star", Who { star: true, mood: 4, ..Who::plain(Look::plain(&o(false, 2, 0, 1, 0))) }, Act::Stand, 0.0),
            (
                "full",
                Who { beard: Beard::Full, whiskers: [0.55, 0.42, 0.2], mood: 3, ..Who::plain(Look::plain(&o(false, 5, 1, 0, 0))) },
                Act::Stand,
                0.0,
            ),
            (
                "goatee",
                Who { beard: Beard::Both, whiskers: [0.3, 0.18, 0.1], ..Who::plain(Look::plain(&o(false, 6, 2, 2, 1))) },
                Act::Stand,
                0.0,
            ),
            (
                "tache",
                Who { beard: Beard::Moustache, whiskers: [0.2, 0.12, 0.08], mood: -2, ..Who::plain(Look::plain(&o(false, 2, 3, 1, 4))) },
                Act::Stand,
                0.0,
            ),
            (
                "stubble",
                Who { beard: Beard::Stubble, whiskers: [0.25, 0.15, 0.1], ..Who::plain(Look::plain(&o(false, 1, 0, 0, 5))) },
                Act::Stand,
                0.0,
            ),
        ];
        for (k, (name, who, act, t)) in cast.iter().enumerate() {
            let v = figure(who, *act, *t, 0.0, 4);
            let s: String = v
                .iter()
                .map(|p| {
                    format!(
                        "{} {} {} {} {} {} {} {} {}\n",
                        p.pos[0], p.pos[1], p.pos[2], p.normal[0], p.normal[1], p.normal[2], p.color[0], p.color[1], p.color[2]
                    )
                })
                .collect();
            std::fs::write(format!("{dir}/cast-{k:02}-{name}.txt"), s).unwrap();
        }
        let acts = [("swing", Act::Swing { reach: 1.0 }, 4), ("chip", Act::Swing { reach: 0.55 }, 9), ("putt", Act::Putt, 13)];
        for (name, act, club) in acts {
            for k in 0..=20 {
                let v = body(&look, act, k as f32 / 20.0, 0.0, club);
                let s: String = v
                    .iter()
                    .map(|p| {
                        format!(
                            "{} {} {} {} {} {} {} {} {}\n",
                            p.pos[0], p.pos[1], p.pos[2], p.normal[0], p.normal[1], p.normal[2], p.color[0], p.color[1], p.color[2]
                        )
                    })
                    .collect();
                std::fs::write(format!("{dir}/{name}-{k:02}.txt"), s).unwrap();
            }
        }
    }
}
