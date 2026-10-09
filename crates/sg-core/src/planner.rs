//! Shot planning: how far a golfer can hit (0x422530), where to aim (the aim search 0x422fb0 with its trial shots 0x4226a0
//! and route risk 0x421fa0), and the shot itself: club, launch speed, curve and the random errors of the swing (0x424120).
//! Restated from the exe in our own words; integer behaviour, including its rounding and random draws, follows the exe.

use crate::course::{t, Course};
use crate::geom::{angle, clamp, cosr, dir8, isqrt_ru, len, sinr, tdist, DX, DY};
use crate::golfer::{flag, game, Club};
use crate::land::ExeRng;

/// Planner globals the exe keeps between calls.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    /// Simulation mode (0x5a9cc4): 0 real shot, 1 planning, 2 deterministic trial.
    pub mode: i32,
    /// The player's shot option for their own golfer (0x58f330): 0 normal, 1 draw, -1 fade, 3 high, 4 low.
    pub option: i32,
    /// Strategy hint from the aim search (0x5a7140): 1 and 2 two kinds of choice, 4 roll toward the pin.
    pub hint: u32,
    /// No safe landing found (0x5a9ce4).
    pub no_safe: bool,
    /// Aim at the tile corner, not its centre (0x5a9ce8).
    pub corner: bool,
    /// Expected landing point (0x5a8828/0x5a8830) and the last trial's rest point (0x56a78c/0x56a790).
    pub land: (i32, i32),
    pub end: (i32, i32),
    /// Skill class bits the trial shots use (0x4c2e1c; 7 outside an aim search).
    pub class_bits: u8,
    /// Game option bits (0x5a47e0): 2 easier putting, 4 wider rough spread.
    pub options: u32,
    pub(crate) cache: Vec<(i32, i32, i32)>,
    pub(crate) cache_at: usize,
    /// The ball trace of the next trial shot (the flight record 0x59fc60 the exe gives a golfer before 0x4226a0): when set,
    /// [`preview`] adds the ball's point at each apex, every eighth tick and each bounce, up to 31 points.
    #[serde(skip)]
    pub trace: Option<Vec<(i32, i32)>>,
}

/// Adds a point to a ball trace (0x409950): the record keeps 31 points.
fn trace_point(t: &mut Option<Vec<(i32, i32)>>, p: (i32, i32)) {
    if let Some(v) = t.as_mut().filter(|v| v.len() < 31) {
        v.push(p);
    }
}

/// Maximum range of the golfer's next shot in range units (0x422530).
pub fn max_range(cl: &Club, c: &Course, g: usize) -> i32 {
    let gg = &cl.g[g];
    let length = gg.class & 1 != 0;
    let mut r = 0x96 + if length { 0x32 } else { 0 };
    r += if cl.difficulty < 1 {
        if length {
            0x28
        } else {
            0x19
        }
    } else {
        (gg.level * 0x32) / 3
    };
    let mut lie = c.tile_type(gg.bx, gg.by) as i32;
    if g >= 0x98 {
        lie = if gg.strokes != 0 { 2 } else { 0 };
    }
    if gg.kind != 0 {
        if gg.skill_mask & 1 != 0 {
            r += gg.skills[0] as i32 * 4 - 0x14;
        }
        if gg.skill_mask & 2 != 0 && lie == 0 {
            r += (gg.skills[1] as i32 - 5) * 6;
        }
    }
    if gg.momentum > 0 {
        r += (clamp(gg.momentum, 0, 3) * r) / 0x18;
    }
    if length {
        r += c.oper[10] * 0xf;
    }
    let h = c.h(lie as u8);
    if h > 0 {
        r -= (clamp(h, 0, 3) * r) / 8;
    }
    if lie != 0 {
        let q = ((r as i64 * -0x6666_6667i64) >> 32) as i32;
        let q = (q >> 1) - (q >> 31);
        r += q;
    }
    r.min(0x14a)
}

/// Coarse carry of a launch (0x4223f0).
fn coarse(mut s: i32, mut v: i32) -> i32 {
    let (mut d, mut h) = (0, 0);
    loop {
        d += s / 8;
        h += v / 16;
        s -= s >> 4;
        v -= 0x80;
        if h <= 0 {
            break;
        }
    }
    d
}

/// Coarse roll of a putt on the green (0x4223c0).
fn roll(c: &Course, mut s: i32) -> i32 {
    let r = c.row(t::GREEN).roll;
    let mut d = 0;
    loop {
        d += s / 8;
        s -= s >> r;
        if s <= 0x3f {
            break;
        }
    }
    d
}

impl State {
    /// Launch speed for a carry of 4/5 of `rng_units` (or a putt's roll), by bisection with the exe's 10-entry cache (0x422430).
    fn solve(&mut self, c: &Course, rng_units: i32, v: i32, putt: bool) -> i32 {
        if let Some(&(_, _, s)) = self.cache.iter().find(|e| e.0 == rng_units && e.1 == v) {
            return s;
        }
        let u = rng_units * 20 / 25;
        let mut s = u * 33 - u * u / 48 + 64;
        let goal = (rng_units << 10) / 25;
        let mut step = s / 2;
        loop {
            let d = if putt { roll(c, s) } else { coarse(s, v) };
            if goal < d {
                s -= step;
            }
            if d < goal {
                s += step;
            }
            step /= 2;
            if step <= 2 {
                break;
            }
        }
        if self.cache.len() < 10 {
            self.cache.push((rng_units, v, s));
        } else {
            self.cache[self.cache_at] = (rng_units, v, s);
        }
        self.cache_at = (self.cache_at + 1) % 10;
        s
    }
}

/// The random hook or slice of a swing (0x405920): -50..50, peaked near 0.
fn curve_draw(rng: &mut ExeRng) -> i32 {
    let v = rng.below(101) - 50;
    let m = v.abs();
    let m = if m < 20 {
        m / 2
    } else if m < 40 {
        m - 10
    } else {
        2 * m - 50
    };
    v.signum() * m
}

fn h_at(c: &Course, a: i32, b: i32) -> i32 {
    let i = a * 50 + b;
    let ty = if (0..2500).contains(&i) { c.ty[i as usize] } else { t::OUT };
    c.h(ty)
}

fn ty_at_point(c: &Course, x: i32, y: i32) -> u8 {
    let i = (x >> 10) * 50 + (y >> 10);
    if (0..2500).contains(&i) {
        c.ty[i as usize]
    } else {
        t::OUT
    }
}

fn class_at_point(c: &Course, x: i32, y: i32) -> u8 {
    c.row(ty_at_point(c, x, y)).class
}

/// Plans golfer g's shot (0x424120): the target, distance and club, the launch, the swing errors and the golfer's thoughts
/// about the shot. `user` is the player's own golfer with the aim tile already set; (tx, ty) an explicit aim point in map units
/// for trial shots (-1 for a real shot); `shape` 1 draw, -1 fade.
#[allow(clippy::too_many_arguments)]
pub fn plan_shot(cl: &mut Club, c: &mut Course, rng: &mut ExeRng, g: usize, user: bool, tx: i32, ty: i32, shape: i32) {
    let mut shape = shape;
    let h = cl.g[g].hole.clamp(0, 19) as usize;
    let maxr = max_range(cl, c, g);
    if cl.g[g].flags & flag::PENALTY != 0 {
        c.hazard[17] = 0x20;
        c.hazard[20] = 0x20;
    }
    let (ba, bb) = (cl.g[g].bx >> 10, cl.g[g].by >> 10);
    let mut lie: i32 = h_index_type(c, ba, bb) as i32;
    if !user {
        cl.g[g].aim_a = cl.holes[h].pin.0;
        cl.g[g].aim_b = cl.holes[h].pin.1;
    } else {
        shape = if cl.planner.option < 2 { cl.planner.option } else { 0 };
    }
    let (bx, by) = (cl.g[g].bx, cl.g[g].by);
    let (mut ax, mut ay) = (cl.g[g].aim_a * 0x400 - bx + 0x200, cl.g[g].aim_b * 0x400 - by + 0x200);
    if tx != -1 {
        ax = tx - bx;
        ay = ty - by;
        cl.g[g].aim_a = tx >> 10;
        cl.g[g].aim_b = ty >> 10;
    }
    let mut dist = isqrt_ru(ax, ay);
    cl.g[g].heading = angle(ax, ay);
    let old_flags = cl.g[g].flags;
    cl.g[g].flags &= !flag::CORNER_AIM;
    if !user && tx == -1 {
        let lim = if old_flags & flag::PENALTY != 0 || cl.g[g].class & 4 != 0 { 0x19 } else { 0x4b };
        if lim < dist && lie != 1 {
            shape = aim_search(cl, c, rng, g);
            let (aa, ab) = (cl.g[g].aim_a, cl.g[g].aim_b);
            if !cl.planner.corner {
                ax = aa * 0x400 - bx + 0x200;
                ay = ab * 0x400 - by + 0x200;
                cl.g[g].flags &= !flag::CORNER_AIM;
            } else {
                ax = aa * 0x400 - bx;
                ay = ab * 0x400 - by;
                cl.g[g].flags |= flag::CORNER_AIM;
            }
            cl.g[g].heading = angle((ax * 25) >> 10, (ay * 25) >> 10);
            dist = isqrt_ru(ax, ay);
        } else {
            near_adjust(cl, c, g, lie, &mut dist);
        }
    }
    // Hazards along the line of the shot and around the target: how hard the shot looks.
    let sec = dir8(cl.g[g].heading);
    let (ta, tb) = (cl.g[g].aim_a, cl.g[g].aim_b);
    let n = dist / 25;
    let mut acc = 4 - (g as i32 & 3);
    let mut w = [0i32; 32];
    let mut wdir = [0i32; 32];
    let mut first_water = 0;
    let mut blocked = 0;
    let mut feature = 0;
    let mut eyesore = 0;
    let mut celeb = 0;
    let hd = cl.g[g].heading;
    for i in 0..n.max(0) {
        let r = 0x200 + i * 0x400;
        let pa = (sinr(hd, r) + bx) >> 10;
        let pb = (by - cosr(hd, r)) >> 10;
        if c.oob(pa, pb) {
            break;
        }
        let pi = (pa * 50 + pb) as usize;
        let pt = c.ty[pi];
        let hz = c.h(pt);
        w[pt as usize] += (i + 1) * hz * 2;
        wdir[pt as usize] = sec ^ 4;
        if pt == t::WATER && first_water == 0 {
            first_water = i;
        }
        if r != 0x200 && c.row(pt).class == 13 {
            blocked += 1;
        }
        if c.flags[pi] & 0x100 != 0 {
            feature = pt as i32;
        }
        let refh = if n / 2 < i { c.raw_corner(ta, tb) } else { c.raw_corner(ba, bb) };
        if refh + 1 < c.raw_corner(pa, pb) {
            blocked += 2;
        }
        let side = hd.wrapping_add(if rng.below(2) != 0 { 0x0aaa_aaaa } else { 0xf555_5556 });
        let sx = (sinr(side, r) + bx) >> 10;
        let sy = (by - cosr(side, r)) >> 10;
        acc += i * hz * 2 + h_at(c, sx, sy) * (i + 1);
    }
    for u in 0..8usize {
        let (qa, qb) = (ta + DX[u], tb + DY[u]);
        if c.oob(qa, qb) {
            acc += 4 + n * 4;
        } else {
            let qt = c.ty[(qa * 50 + qb) as usize];
            let v = (c.h(qt) * (n + 1)) / 2;
            acc += v;
            w[qt as usize] += v;
            wdir[qt as usize] = u as i32;
        }
    }
    let mut worst = 0usize;
    let mut best = -1;
    for (k, &v) in w.iter().enumerate() {
        if best < v {
            best = v;
            worst = k;
        }
    }
    let worst_dir = wdir[worst];
    acc /= n + 3;
    if blocked > 1 {
        acc += 4;
    }
    if h_index_type(c, bx >> 10, by >> 10) == t::GREEN {
        acc = 0;
    }
    if dist < 0x29 && acc > 10 {
        acc = 10;
    }
    cl.g[g].rating = acc as i8 as i32;
    if cl.g[g].class & 4 != 0 {
        let e = c.raw_corner(ta, tb) - c.raw_corner(ba, bb);
        dist += if e < 1 { (e * 25) / 10 } else { (e * 25) / 8 };
    }
    // Distance, club and launch.
    dist = clamp(dist, 0, maxr);
    let club = clamp(((maxr - dist) * 0x3c) / (maxr * 3).max(1), (lie != 0) as i32, 0xb);
    cl.g[g].club = club;
    if user && cl.planner.option == 3 && club < 5 {
        dist = (maxr * 0xd) / 0x12;
        cl.g[g].club = 5;
    }
    if lie == 1 && dist < 0x32 && cl.g[g].flags & flag::PENALTY == 0 {
        cl.g[g].club = 13;
    }
    let u = dist * 20 / 25;
    let est = u * 33 - u * u / 48 + 64;
    let mut vert = est / 8 + 0x200;
    cl.g[g].vz = vert;
    cl.g[g].speed = cl.planner.solve(c, (dist * 4) / 5, vert, false);
    // The putting "no-break" length and the curve.
    let mut e = if cl.planner.options & 2 != 0 { 12 } else { 20 };
    if cl.g[g].class & 4 != 0 && c.oper[6] != 0 && cl.g[g].vip() != 0x20 {
        e += e / (4 - c.oper[6]).max(1);
    }
    if cl.g[g].skill_mask & 0x10 != 0 {
        e += (cl.g[g].skills[4] as i32 * e) / 8;
    }
    let pin = cl.holes[h].pin;
    if crate::course::inside(pin.0, pin.1) && c.obj[crate::course::idx(pin.0, pin.1)] & 0x80 != 0 {
        e -= 10;
    }
    if cl.g[g].momentum < 2 {
        e /= 2;
    }
    let er = rng.below(e / 2);
    let e = er + 4 + e / 2;
    let max_speed = cl.planner.solve(c, (maxr * 4) / 5, cl.g[g].vz, false);
    let mode = cl.planner.mode;
    let diff = cl.difficulty;
    let mut cv: i32 = 0;
    if mode < 2 {
        cv = curve_draw(rng) * 0x50000;
    }
    if mode == 1 {
        cv /= 3;
    }
    if cl.g[g].momentum < 0 {
        cv += (clamp(-cl.g[g].momentum, 0, 3) * cv) / if lie != 1 { 3 } else { 8 };
    }
    if lie != 1 && cl.g[g].vip() != 0x20 && (cl.g[g].class & 2 != 0 || cl.g[g].flags & flag::PENALTY != 0) {
        cv /= c.oper[8] + 2;
        if diff < 2 {
            cv -= cv / (diff + 2);
        }
    }
    if cl.g[g].flags & flag::WASHED != 0 && cl.g[g].vip() != 0x20 {
        cv -= cv / 3;
    }
    let mut bonus = -3;
    cl.g[g].flags &= !flag::SWEET;
    let gary = cl.gary;
    if cl.g[g].club == 13 {
        cv = 0;
        let k = if cl.game & game::TOURNAMENT != 0 { 2 } else { 1 };
        if e < k * dist {
            let r = rng.below(2);
            if r == 0 {
                cv = (rng.below(0x96) + 0x96) * 0x40000;
            } else if r == 1 {
                cv = (-0x96 - rng.below(0x96)) * 0x40000;
            }
            if dist < 6 {
                cv = 0;
            }
            if dist > 0xf {
                cv /= 2;
            }
            if dist > 0x19 {
                cv /= 2;
            }
            if dist > 0x23 {
                cv /= 2;
            }
        }
    } else {
        let gg = &cl.g[g];
        if gg.kind != 0 {
            if g as i32 == gary {
                cv -= ((3 - diff) * cv) / 6;
            } else if gg.vip() == 0x20 {
                cv += ((3 - diff) * cv) / 3;
            } else {
                bonus = gg.momentum - 3;
            }
            if gg.club < 4 {
                if gg.strokes == 0 {
                    cv = (cv * 6) / (gg.skills[2] as i32 + 4);
                    bonus += gg.skills[2] as i32;
                }
            } else {
                cv = (cv * 6) / (gg.skills[3] as i32 + 4);
                bonus += gg.skills[3] as i32;
            }
            if shape == 1 {
                cv = (cv * 6) / (gg.skills[5] as i32 + 3);
                bonus += gg.skills[5] as i32;
            } else if shape == -1 {
                cv = (cv * 6) / (gg.skills[6] as i32 + 3);
                bonus += gg.skills[6] as i32;
            } else if shape == 0 && cl.planner.option != 3 {
                bonus += 3;
                cv = (cv * 6) / 7;
            }
        }
        if cl.game & game::AIM_SEARCH == 0 && dist > 0x4b {
            let r = rng.below(cl.g[g].speed);
            if (cv.unsigned_abs() as i32 >> 9) + 0x200 < r {
                bonus += 4;
                cl.g[g].flags |= flag::SWEET;
                if g as i32 != gary {
                    cv /= 2;
                } else {
                    cv = 0;
                }
            }
        }
    }
    let mut h_shape = cl.g[g].heading;
    if shape == 1 {
        h_shape = h_shape.wrapping_add(0x1555_5554);
    } else if shape == -1 {
        h_shape = h_shape.wrapping_sub(0x1555_5554);
    }
    if cl.g[g].club != 13 {
        let e5 = clamp(cv, -0x38e_38e3, 0x38e_38e3);
        cl.g[g].heading = cl.g[g].heading.wrapping_add((e5 * 3) as u32);
        cv -= e5 / 2;
        if mode == 0 && cl.g[g].vip() == 0x20 {
            // 32-bit arithmetic as in the exe: a long shot's product wraps
            cv = cv.wrapping_add((dist - 100).wrapping_mul(cv) / 256);
            let r = rng.below(0x71c6);
            cv += (r - 0x38e_38e3) / 0x32;
        }
    }
    cl.g[g].flags &= !(flag::BACKSPIN | flag::TO_SHOP);
    cl.g[g].shot_type = 0;
    // Special shots: a low punch under trees, a high backspin shot onto the green.
    let wf = if cl.g[g].flags & flag::PENALTY != 0 && !user { 1 } else { first_water };
    let gg = &cl.g[g];
    let punch_ok = (gg.class & 4 != 0 || gg.flags & flag::HIT_TREE != 0) && lie != 1 && wf != 1;
    let punch = punch_ok
        && if !user {
            class_at_point(c, sinr(h_shape, 0x80) + bx, by - cosr(h_shape, 0x80)) == 13
                || class_at_point(c, sinr(h_shape, 0x400) + bx, by - cosr(h_shape, 0x400)) == 13
        } else {
            cl.planner.option == 4
        };
    if punch {
        vert = clamp(cl.g[g].speed / 0xc, 0, 0x100);
        cl.g[g].vz = vert;
        let s3 = (dist * 3) / 4;
        cl.g[g].speed = cl.planner.solve(c, s3, vert, false);
        if wf != 0 {
            cl.g[g].speed = cl.planner.solve(c, (wf * 0x32) / 3, vert, false);
        }
        shape = 0;
        cl.g[g].shot_type = 4;
    } else {
        let gg = &cl.g[g];
        if gg.class & 4 != 0
            && shape == 0
            && lie != 1
            && c.hazard[(lie.clamp(0, 22)) as usize] == 0
            && gg.club > 3
            && dist > 0x19
            && if !user { h_index_type(c, gg.aim_a, gg.aim_b) == t::GREEN } else { cl.planner.option == 3 }
        {
            cl.g[g].flags |= flag::BACKSPIN;
            let v = cl.g[g].vz;
            let v = ((dist + 0x32) * v) / 400 + v;
            cl.g[g].vz = v;
            cl.g[g].speed = cl.planner.solve(c, dist, v, false);
            cl.g[g].shot_type = 3;
            cv = (cv * 6) / (cl.g[g].skills[7] as i32 + 3);
            bonus += cl.g[g].skills[7] as i32;
        }
    }
    let stamp0 = cl.g[g].stamps[0];
    if tx == -1 {
        if cl.g[g].club == 13 {
            if h_index_type(c, cl.g[g].aim_a, cl.g[g].aim_b) == t::GREEN {
                cl.g[g].speed = cl.planner.solve(c, dist + 2, 0, true);
            }
        } else {
            cl.holes[h].plans += 1;
            let mut k = 0;
            let (gx, gy) = (cl.g[g].x, cl.g[g].y);
            while k < (c.raw_corner(ba, bb) << 4) / (diff + 2) {
                let h2 = cl.g[g].heading.wrapping_add(((0xc - rng.below(0x19)) as u32) << 24);
                let r = (dist - rng.below(200)) + 100;
                if r >= 0 {
                    let rr = (r / 0x19) * 0x400;
                    let pa = (sinr(h2, rr) + gx) >> 10;
                    let pb = (gy - cosr(h2, rr)) >> 10;
                    let d = dir8(h2) as usize;
                    if !c.oob(pa, pb) {
                        let pi = (pa * 50 + pb) as usize;
                        let q_class = {
                            let qi = (pa - DX[d]) * 50 - DY[d] + pb;
                            if (0..2500).contains(&qi) {
                                c.row(c.ty[qi as usize]).class
                            } else {
                                c.row(t::OUT).class
                            }
                        };
                        if (c.flags[pi] & 0x100 != 0 || c.ty[pi] == t::MARSH)
                            && c.raw_corner(pa, pb) <= c.raw_corner(ba, bb) + 1
                            && q_class != 13
                        {
                            feature = pa + pb * 50;
                        }
                        if c.row(c.ty[pi]).group == 0x10 && q_class != 13 {
                            let o = c.object_at(pa, pb);
                            if c.ty[pi] == t::HOME {
                                if o == -1 || c.objects[o as usize].kind != 5 || c.objects[o as usize].sub == 0 {
                                    eyesore = pa + pb * 50;
                                } else {
                                    celeb = c.objects[o as usize].sub;
                                }
                            } else if o >= 0 && c.objects[o as usize].kind == 4 {
                                if c.objects[o as usize].sub < 0x10 {
                                    feature = pa + pb * 50;
                                } else {
                                    eyesore = pa + pb * 50;
                                }
                            }
                        }
                    }
                }
                k += 1;
            }
        }
        let de = c.raw_corner(ba, bb) - c.raw_corner(cl.g[g].aim_a, cl.g[g].aim_b);
        cl.g[g].rating = (cl.g[g].rating as i8).wrapping_add(de.unsigned_abs() as i8) as i32;
        shot_thoughts(cl, c, rng, g, user, lie, dist, sec, worst as i32, worst_dir, feature, eyesore, celeb, shape, cv, stamp0);
    }
    // Draw or fade, short-shot errors, the lie, and the final launch speed.
    cl.g[g].flags &= !flag::HIT_TREE;
    let mut extra = 0;
    let mut cap = max_speed;
    if shape == 1 || shape == -1 {
        let base = cl.g[g].heading;
        let turn: i32 = if cl.g[g].flags & flag::BACKSPIN != 0 { 0x0fff_ffff } else { 0x1555_5554 };
        cl.g[g].heading = base.wrapping_add((turn * shape) as u32);
        if g as i32 == gary && ((shape == 1 && cv > 0) || (shape == -1 && cv < 0)) {
            cv /= 2;
        }
        extra = -0x239_a955 * shape;
        let sp = cl.g[g].speed;
        cl.g[g].speed = if dist < 300 {
            if dist < 0xfa {
                (sp << 4) / (0x14a - dist) + sp
            } else {
                (sp << 4) / (400 - dist) + sp
            }
        } else {
            max_speed + (max_speed << 4) / (400 - dist).max(1)
        };
        cl.g[g].shot_type = shape;
        cap = 999_999;
    }
    let v0 = clamp(cl.g[g].speed, 0, cap);
    cl.g[g].speed = v0;
    let mut skip_short = false;
    if dist < 0x4b {
        if diff == 0 {
            if g as i32 == user as i32 {
                cv /= 3;
                skip_short = true;
            } else if g as i32 == (user as i32 | 1) {
                cv <<= 1;
            }
        }
        if !skip_short && lie != 1 {
            let k = if cl.g[g].class & 7 == 7 { diff + 4 } else { 9 - diff };
            cl.g[g].heading = cl.g[g].heading.wrapping_add((k * cv * 2) as u32);
        }
    }
    if cl.g[g].kind != 0 && c.h(lie as u8) > 0 {
        let r = rng.below(c.h(lie as u8) * 10);
        if r <= cl.g[g].skills[8] as i32 {
            lie = 2;
            cl.g[g].flags |= flag::SWEET;
        }
    }
    if mode == 0 && lie != 1 {
        let r = rng.below(0x5555);
        let hz = c.h(lie as u8);
        cl.g[g].heading = cl.g[g].heading.wrapping_add((cv.signum().wrapping_mul(r + 0x155_5555).wrapping_mul(hz)) as u32);
        cv = ((hz + 2) * cv) / 2;
    }
    if cl.g[g].flags & flag::PENALTY != 0 && cl.g[g].strokes > 6 {
        cl.g[g].speed = v0;
        lie = -1;
    }
    cv += extra;
    let s = cl.g[g].speed;
    let theme = c.theme;
    let ns = match lie {
        -1 => s,
        0 => (s / 8 - rng.below(s / 4)) + s,
        1 => {
            let r = rng.below(s / 8);
            if cl.g[g].club == 13 {
                cl.g[g].vz = 0;
            }
            (s / 0xc - r) + s
        }
        2 if flag_at(c, ba, bb) & 0x800 == 0 => {
            let r = rng.below(s / 3);
            (s / 6 - r) + s
        }
        4 | 5 | 8 | 0x15 | 0x16 => {
            let r = rng.below(s / 2);
            let mut ns = (s / 4 - r) + s;
            let r2 = rng.below(cv / 2);
            cv += r2;
            if lie == if theme != 1 { 5 } else { 4 } {
                cl.g[g].heading = cl.g[g].heading.wrapping_add((cv * 4) as u32);
            }
            if cl.planner.options & 4 != 0 {
                let r = rng.below(ns / 2);
                ns = (ns / 4 - r) + ns;
            }
            ns
        }
        10 | 13..=16 => {
            let r = rng.below(s / 2);
            (s / 4 - r) + s
        }
        9 => {
            let r = rng.below(s * 2);
            cl.g[g].heading = cl.g[g].heading.wrapping_add((cv * 0x10) as u32);
            r
        }
        11 | 12 => s / 2 + rng.below(s),
        _ => {
            let r = rng.below(s);
            (s / 2 - r) + s
        }
    };
    let mut s = ns;
    let hl = if lie >= 0 { c.h(lie as u8) } else { 0 };
    if (cl.g[g].class & 2 != 0 && hl < 1) || cl.g[g].flags & flag::PENALTY != 0 {
        s = (v0 + s) / 2;
    }
    if mode == 0 {
        s += (hl * (s - v0)) / 8;
    }
    if s < v0 / 3 {
        s = v0 / 3;
    }
    if mode > 1 {
        s = v0;
    }
    if mode == 1 {
        s = (s + v0 * 2) / 3;
    }
    s += (clamp(bonus, -10, 0x10) * (v0 - s)) / 0x10;
    cl.g[g].speed = s;
    cl.g[g].curve = clamp(cv, -0x1555_5555, 0x1555_5555);
    cl.g[g].flags &= !flag::PENALTY;
    c.hazard[17] = 8;
    c.hazard[20] = 8;
}

fn h_index_type(c: &Course, a: i32, b: i32) -> u8 {
    let i = a * 50 + b;
    if (0..2500).contains(&i) {
        c.ty[i as usize]
    } else {
        t::OUT
    }
}

fn flag_at(c: &Course, a: i32, b: i32) -> u16 {
    let i = a * 50 + b;
    if (0..2500).contains(&i) {
        c.flags[i as usize]
    } else {
        0
    }
}

/// Near shots: hazards just short of the target make the golfer play a little longer, hazards beyond it a little shorter.
fn near_adjust(cl: &mut Club, c: &Course, g: usize, lie: i32, dist: &mut i32) {
    let (ta, tb) = (cl.g[g].aim_a, cl.g[g].aim_b);
    cl.planner.hint = 0;
    cl.planner.land = (ta * 0x400 + 0x200, tb * 0x400 + 0x200);
    if lie != 1 && *dist > 0x19 {
        let sec = dir8(cl.g[g].heading) as usize;
        let s0 = sec & 6;
        let s1 = ((sec & 7) + 1) & 6;
        let front = h_at(c, ta - DX[s0], tb - DY[s0]) + h_at(c, ta - DX[s1], tb - DY[s1]);
        let back = h_at(c, ta + DX[s0], tb + DY[s0]) + h_at(c, ta + DX[s1], tb + DY[s1]);
        let s = front - back;
        if s > 3 {
            *dist += clamp(*dist / 4 - if c.h(lie as u8) < 1 { 6 } else { 0 }, 0, 0xc);
        }
        if s < -3 {
            *dist -= 6;
        }
    }
}

/// The golfer's thoughts while planning (scenery, the hole, hazards, the shot type).
#[allow(clippy::too_many_arguments)]
fn shot_thoughts(
    cl: &mut Club,
    c: &mut Course,
    rng: &mut ExeRng,
    g: usize,
    user: bool,
    lie: i32,
    dist: i32,
    sec: i32,
    worst: i32,
    worst_dir: i32,
    feature: i32,
    eyesore: i32,
    celeb: i32,
    shape: i32,
    cv: i32,
    stamp0: u8,
) {
    let diff = cl.difficulty;
    let hl = c.h(lie.clamp(0, 22) as u8);
    let (aa, ab) = (cl.g[g].aim_a, cl.g[g].aim_b);
    let (ba, bb) = (cl.g[g].bx >> 10, cl.g[g].by >> 10);
    let ball_flags = flag_at(c, ba, bb);
    let hole = cl.g[g].hole;
    let hu = hole.clamp(0, 19) as usize;
    if !cl.planner.no_safe || h_at(c, aa, ab) < 1 || hl > 0 || user || cl.g[g].flags & flag::PENALTY != 0 {
        if eyesore == 0 {
            if celeb != 0 {
                cl.event(c, rng, g, 0x16, celeb - 1);
            } else if ball_flags & 0x800 == 0 || cl.attitude(g) == 0 || ball_flags & 0x4000 != 0 {
                if dist < 100 || user {
                    cl.planner.hint &= !3;
                }
                if cl.planner.hint == 0 || cl.g[g].shot_type == 4 {
                    'chain: {
                        if hole > 1 && cl.g[g].strokes == (g as i32 & 1) + 1 {
                            let r = rng.below(3);
                            if cl.holes[hu].monotony <= r {
                                cl.event(c, rng, g, 0x1d, 0x14);
                                break 'chain;
                            }
                        }
                        let mut to_hazard_check = diff == 0;
                        if !to_hazard_check {
                            let skip_1e = hole < 2 || cl.g[g].strokes != 1 || {
                                let r = rng.below(3);
                                cl.holes[hu].monotony <= r + 3
                            };
                            if !skip_1e {
                                cl.event(c, rng, g, 0x1e, 0x14);
                                break 'chain;
                            }
                            let r4_fail = {
                                let r = rng.below(4);
                                r + diff * 2 < cl.g[g].rating
                            };
                            if r4_fail || dist < 0x65 || hl > 0 || feature != 0 {
                                to_hazard_check = true;
                            } else {
                                cl.event(c, rng, g, 4, 0x14);
                                cl.g[g].rating = 0;
                                break 'chain;
                            }
                        }
                        if to_hazard_check {
                            let r = rng.below(5);
                            if r + 6 + diff < cl.g[g].rating && dist > 0x28 && feature == 0 {
                                let id = if cl.g[g].rating < 0x18 {
                                    let rel = (worst_dir - sec) & 7;
                                    if !(2..=6).contains(&rel) {
                                        0x26
                                    } else if rel == 2 || rel == 6 {
                                        5
                                    } else {
                                        0x25
                                    }
                                } else {
                                    0x1f
                                };
                                cl.event(c, rng, g, id, worst);
                                cl.g[g].rating = if cl.g[g].rating < 0x18 { 10 } else { 20 };
                            } else {
                                cl.g[g].rating = 6;
                            }
                        }
                    }
                } else if cl.planner.hint & 3 == 0 {
                    if hl < 1 {
                        cl.event(c, rng, g, 6, 0x14);
                    }
                } else {
                    let id = (!cl.planner.hint & 1) | 0x20;
                    cl.event(c, rng, g, id, 0x14);
                }
            } else {
                cl.event(c, rng, g, 0x18, 0x14);
            }
        } else if celeb == 0 {
            cl.event(c, rng, g, 0x14, eyesore);
        } else {
            cl.event(c, rng, g, 0x16, celeb - 1);
        }
    } else {
        cl.event(c, rng, g, 8, 0x14);
    }
    cl.g[g].flags &= !0x60;
    let mut quiet = cl.g[g].stamps[0] == stamp0;
    if quiet {
        if cl.g[g].timer == 0 {
            'q: {
                if feature != 0 && hl < 1 {
                    cl.event(c, rng, g, 0x1c, feature);
                    break 'q;
                }
                if shape == 1 {
                    cl.event(c, rng, g, 0x37, 0x14);
                } else if shape == -1 {
                    cl.event(c, rng, g, 0x38, 0x14);
                }
                if cl.g[g].flags & flag::BACKSPIN != 0 && dist > 100 {
                    cl.event(c, rng, g, 0x39, 0x14);
                }
                if cl.g[g].shot_type == 4 {
                    let id = if cl.g[g].strokes != 0 { 0x3c } else { 9 };
                    cl.event(c, rng, g, id, lie);
                }
                if dist > 0x4b {
                    let k = 0x10 - (cl.g[g].class & 2 != 0) as u32;
                    if (0x1e_i32 << k) < cv {
                        cl.g[g].flags |= 0x20;
                    }
                    if cv < (-0x1e_i32 << k) {
                        cl.g[g].flags |= 0x40;
                    }
                }
                let r = cl.g[g].roster.clamp(0, cl.members.len() as i32 - 1) as usize;
                if cl.g[g].stamps[0] == stamp0
                    && cl.g[g].timer == 0
                    && cl.g[g].strokes == 0
                    && cl.members[r].card[hu.min(18)] != 0
                    && cl.g[g].kind == 0
                {
                    cl.event(c, rng, g, 0x3b, 0x14);
                }
                let p = (cl.g[g].partner.clamp(0, 151)) as usize;
                if cl.g[g].stamps[0] == stamp0
                    && cl.g[g].strokes == 3
                    && (hole + g as i32) & 1 == 0
                    && cl.g[g].kind == 0
                    && cl.g[p].kind == 0
                    && cl.male(g ^ 1) == cl.male(g)
                    && cl.g[p].timer == 0
                    && cl.g[g].timer == 0
                {
                    cl.event(c, rng, g, 0x30, 0x14);
                    cl.event(c, rng, g ^ 1, 0x31, 0x14);
                    cl.g[p].timer = cl.g[p].timer.wrapping_add(1);
                }
            }
            if cl.g[g].timer != 0 {
                quiet = false;
            }
        } else {
            quiet = false;
        }
    } else if cl.g[g].timer != 0 {
        quiet = false;
    }
    let still = |cl: &Club| quiet && cl.g[g].stamps[0] == stamp0;
    let club = cl.g[g].club;
    if still(cl) && hole > 1 && club < 13 && cl.g[g].clubs_used & (1 << club) == 0 {
        cl.event(c, rng, g, 0x36, club);
    }
    cl.g[g].clubs_used |= 1 << (club & 15);
    if still(cl) {
        let et = c.raw_corner(aa, ab);
        let eb = c.raw_corner(ba, bb);
        if et < eb {
            cl.event(c, rng, g, 0x2e, 0x14);
        }
        if eb < et {
            cl.event(c, rng, g, 0x2d, 0x14);
        }
    }
    if still(cl) && cl.g[g].kind == 0 && cl.g[g].timer == 0 && lie == 1 && (cl.g[g].strokes + g as i32 + hole) & 3 == 0 {
        cl.event(c, rng, g, 0x3e, 0x14);
    }
}

/// Risk of the next shot from a landing point toward the pin (0x421fa0).
#[allow(clippy::too_many_arguments)]
fn route_risk(c: &mut Course, sx: i32, sy: i32, pa: i32, pb: i32, reach: i32, shape: i32, trees_only: i32) -> i32 {
    let old = c.hazard[1];
    c.hazard[1] = 0;
    let vx = pa * 0x400 + 0x200 - sx;
    let vy = pb * 0x400 + 0x200 - sy;
    let h = angle(vx, vy);
    let s = dir8(h) as usize;
    let nt = (len(vx, vy) + 0x200) >> 10;
    if nt < 0 {
        c.hazard[1] = old;
        return 0;
    }
    let rt = clamp(nt, 0, reach / 25 + 1);
    let k = if rt < 6 {
        1
    } else if rt <= 8 {
        2
    } else {
        3
    };
    let mut risk = 0;
    let mut trees = 0;
    let mut acc: i32 = rt.wrapping_mul(shape).wrapping_mul(0x1555_5554);
    for i in 0..=nt {
        let mut off: i32 = 0;
        if shape != 0 {
            if i <= rt / 2 {
                off = shape.wrapping_mul(0x0fff_fffc);
            } else if i < rt {
                off = acc / ((rt + 1) / 2).max(1);
            }
        }
        let ha = h.wrapping_add(off as u32);
        let (mut px, mut py) = (sx + sinr(ha, i * 0x400), sy - cosr(ha, i * 0x400));
        if shape != 0 && rt / 2 < i && i < rt {
            let hb = h.wrapping_sub(shape.wrapping_mul(0x1555_5554) as u32);
            px = pa * 0x400 + 0x200 - sinr(hb, (rt - i) * 0x400);
            py = pb * 0x400 + 0x200 + cosr(hb, (rt - i) * 0x400);
        }
        let fx = px % 0x400;
        let fy = py % 0x400;
        let (ta, tb) = (px >> 10, py >> 10);
        let tt = h_index_type(c, ta, tb);
        if (i >= rt - k && trees_only == 0) || c.row(tt).class == 13 {
            if c.h(tt) > 0 {
                risk += c.h(tt);
            }
            if c.row(tt).class == 13 {
                risk += 0x20 / (i + 1);
            }
            if tt == t::WATER || c.oob(ta, tb) {
                risk += 0x10;
            }
            if DY[s] != 0 {
                if fx >> 4 < 0xf {
                    risk += h_at(c, ta - 1, tb) / 2;
                }
                if fx >> 4 > 0x30 {
                    risk += h_at(c, ta + 1, tb) / 2;
                }
            }
            if DX[s] != 0 {
                if fy >> 4 < 0xf {
                    risk += h_at(c, ta, tb - 1) / 2;
                }
                if fy >> 4 > 0x30 {
                    risk += h_at(c, ta, tb + 1) / 2;
                }
            }
        }
        let tr = 0x200 + i * 0x400;
        if class_at_point(c, sx + sinr(h, tr), sy - cosr(h, tr)) == 13 {
            trees += 1;
        }
        acc = acc.wrapping_sub(shape.wrapping_mul(0x1555_5554));
    }
    if trees > 1 {
        risk += 1;
    }
    c.hazard[1] = old;
    risk.max(0)
}

/// A trial shot (0x4226a0): plans a shot at (wx, wy) with the given shape, flies it with a simplified ball and returns where it
/// stops; the golfer's record is put back afterwards.
pub fn preview(cl: &mut Club, c: &mut Course, rng: &mut ExeRng, g: usize, wx: i32, wy: i32, shape: i32) -> (i32, i32) {
    let saved = cl.g[g].clone();
    let saved_events = cl.out.len();
    let mut trace = cl.planner.trace.take();
    plan_shot(cl, c, rng, g, false, wx, wy, shape);
    cl.out.truncate(saved_events);
    cl.g[g].curve /= 2;
    let mode = cl.planner.mode;
    let imag = cl.planner.class_bits & 4 != 0;
    let mut ticks = 0;
    let mut end = (cl.g[g].bx, cl.g[g].by);
    while cl.g[g].speed != 0 && ticks < 20_000 {
        let gg = &mut cl.g[g];
        let (a, b) = (gg.bx >> 10, gg.by >> 10);
        let lie = if c.oob(a, b) { t::OUT } else { c.ty[(a * 50 + b) as usize] };
        let h0 = c.ground(gg.bx, gg.by);
        let s = gg.speed / 16;
        let dx = sinr(gg.heading, s);
        let dy = cosr(gg.heading, s);
        gg.bx += dx;
        gg.by -= dy;
        gg.bz += gg.vz / 32;
        if gg.bz != 0 || gg.vz != 0 {
            gg.vz -= 0x40;
            if 0 < gg.vz && gg.vz < 0x40 {
                trace_point(&mut trace, (gg.bx, gg.by));
            }
        }
        ticks += 1;
        if ticks & 7 == 0 {
            trace_point(&mut trace, (gg.bx, gg.by));
        }
        let ux = (gg.bx >> 6) & 0xf;
        let uy = (gg.by >> 6) & 0xf;
        let tf = |a: i32, b: i32| h_index_type(c, a, b);
        let mut edge = false;
        if ux < 2 {
            edge = tf(a - 1, b) != lie;
        }
        if uy < 2 && tf(a, b - 1) != lie {
            edge = true;
        }
        if ux > 0xd && tf(a + 1, b) != lie {
            edge = true;
        }
        if uy > 0xd && tf(a, b + 1) != lie {
            edge = true;
        }
        let d8 = dir8(gg.heading);
        let row = *c.row(lie);
        if gg.bz < 2 {
            let mut fr = if !imag { row.roll } else { clamp(row.roll - c.rise(gg.bx, gg.by, d8), 0, 99) };
            let side = c.rise(gg.bx, gg.by, (d8 + 2) & 7);
            if fr < 2 && edge {
                fr = 2;
            } else if fr > 4 && ticks > 0x80 {
                fr = 4;
            }
            if imag {
                gg.heading = gg.heading.wrapping_add((side.wrapping_mul(-0x200_0000)) as u32);
            }
            if fr < 5 {
                gg.speed -= (gg.speed >> fr) / 2;
            } else {
                gg.speed = (gg.speed - (gg.speed >> 6)) + 0x20;
            }
            if lie == t::WATER {
                if !edge {
                    gg.speed /= 2;
                }
            } else if lie == t::RAVINE && (8 - ux).abs() < 3 && (8 - uy).abs() < 3 {
                gg.speed /= 2;
            }
            let w = flag_walls(c, a, b);
            if a != gg.bx >> 10 && w & if dx < 1 { 0x40 } else { 4 } != 0 {
                gg.heading = gg.heading.wrapping_neg();
            }
            if b != gg.by >> 10 && w & if dy < 1 { 0x10 } else { 1 } != 0 {
                gg.heading = 0x8000_0000u32.wrapping_sub(gg.heading);
            }
        } else {
            gg.bz += h0 - c.ground(gg.bx, gg.by);
            gg.speed -= gg.speed >> 5;
            gg.heading = gg.heading.wrapping_add(gg.curve as u32);
            let z = gg.bz;
            if mode != 2 && tree_hit_preview(c, rng, lie, z, a, b) {
                let gg = &cl.g[g];
                let mut dd = len(gg.bx - a * 0x400 - 0x200, gg.by - b * 0x400 - 0x200);
                if gg.kind != 0 && gg.skill_mask & 0x200 != 0 {
                    dd += (gg.skills[9] as i32 * dd) / 4;
                }
                if dd < rng.below(0x300) {
                    let turn = (rng.below(0x80) + 0x40) as u32;
                    cl.g[g].heading = cl.g[g].heading.wrapping_add(turn << 24);
                    let sp = cl.g[g].speed;
                    cl.g[g].speed -= rng.below(sp);
                    cl.g[g].flags |= flag::HIT_TREE;
                }
            }
        }
        let gg = &mut cl.g[g];
        if gg.bz < 1 && gg.vz < 0 {
            let centre = 4 < ux && ux < 0xb && 4 < uy;
            let mut bf = row.bounce;
            if mode == 0 {
                if bf < 2 && edge {
                    bf = 2;
                }
                if centre && flag_at(c, a, b) & 0x20 != 0 {
                    bf = 4;
                }
            }
            let mut vz = clamp(-0x40 - (bf * gg.vz) / 0xc, 0, 9999);
            if vz < 0x80 {
                vz = 0;
            }
            gg.vz = vz;
            gg.bz = 0;
            trace_point(&mut trace, (gg.bx, gg.by));
            if gg.flags & flag::TO_SHOP != 0 {
                gg.heading ^= 0x8000_0000;
                gg.flags &= !flag::TO_SHOP;
                let sp = gg.speed;
                if gg.kind == 0 {
                    gg.speed = rng.below(sp << 1);
                } else {
                    let r = rng.below(sp << 1);
                    gg.speed = ((r - sp) * 3) / (gg.skills[9] as i32 + 5) + sp;
                }
            }
            let gg = &mut cl.g[g];
            if gg.flags & flag::BACKSPIN != 0 {
                gg.speed /= 2;
                gg.flags = (gg.flags & !flag::BACKSPIN) | flag::TO_SHOP;
            }
            if imag {
                let dp = (((((gg.heading as i32) >> 29) + 1) & !1) + 2) & 7;
                let r1 = c.rise(gg.bx, gg.by, dp);
                gg.heading = gg.heading.wrapping_sub((r1.wrapping_mul(gg.vz).wrapping_mul(0xaec33)) as u32);
                let s3 = clamp(c.rise(gg.bx, gg.by, d8), -2, 2);
                gg.speed += s3 * gg.vz * -2;
                gg.vz += (s3 * gg.vz) / 2;
            }
            if lie == t::WATER {
                if !edge && (mode != 0 || !centre || flag_at(c, a, b) & 0x20 == 0) {
                    gg.vz = 0;
                    gg.speed = 0;
                }
            } else if lie == t::ROCKS && gg.speed > 0x100 && !edge {
                let turn = 0x50 - rng.below(0xa0);
                cl.g[g].heading = cl.g[g].heading.wrapping_add((turn as u32) << 24);
            }
        }
        let gg = &mut cl.g[g];
        if gg.speed < 0x40 && gg.bz == 0 && gg.vz == 0 {
            end = (gg.bx, gg.by);
            gg.speed = 0;
        }
    }
    cl.planner.end = end;
    cl.planner.trace = trace;
    cl.g[g] = saved;
    end
}

fn flag_walls(c: &Course, a: i32, b: i32) -> u8 {
    let i = a * 50 + b;
    if (0..2500).contains(&i) {
        c.walls[i as usize]
    } else {
        0
    }
}

fn tree_hit_preview(c: &Course, rng: &mut ExeRng, ty: u8, z: i32, a: i32, b: i32) -> bool {
    Club::tree_hit(c, rng, ty, z, a, b)
}

/// The aim search (0x422fb0): trial shots at landing tiles around the line to the pin, refined in rounds; returns the shot shape
/// and leaves the chosen tile in the golfer's aim.
pub fn aim_search(cl: &mut Club, c: &mut Course, rng: &mut ExeRng, g: usize) -> i32 {
    cl.game |= game::AIM_SEARCH;
    cl.plan_busy = true;
    cl.planner.no_safe = true;
    cl.planner.corner = false;
    cl.planner.class_bits = cl.g[g].class & 7;
    let maxr = max_range(cl, c, g);
    let (bx, by) = (cl.g[g].bx, cl.g[g].by);
    let (ba, bb) = (bx >> 10, by >> 10);
    let lie = h_index_type(c, ba, bb);
    let entry_mode = cl.planner.mode;
    if cl.planner.mode == 0 && cl.g[g].flags & flag::PENALTY == 0 && (c.h(lie) < 1 || cl.g[g].kind == 0) {
        cl.planner.mode = 1;
    }
    let mut mask = if c.h(lie) < 1 && cl.g[g].class & 4 != 0 { 3 } else { 0 };
    if cl.g[g].kind != 0 {
        if cl.g[g].skill_mask & 0x20 == 0 {
            mask &= !1;
        }
        if cl.g[g].skill_mask & 0x40 == 0 {
            mask &= !2;
        }
    }
    let prev = (cl.g[g].aim_a, cl.g[g].aim_b);
    let h = cl.g[g].hole.clamp(0, 19) as usize;
    let pin = cl.holes[h].pin;
    cl.g[g].aim_a = pin.0;
    cl.g[g].aim_b = pin.1;
    let (pcx, pcy) = (pin.0 * 0x400 + 0x200, pin.1 * 0x400 + 0x200);
    let d_pin = isqrt_ru(pcx - bx, pcy - by);
    let heading = angle(pcx - bx, pcy - by);
    let next_max = maxr;
    let far = d_pin > maxr + next_max;
    let reach_tiles = (clamp(maxr - 0x19, 0, d_pin) * 0x400 + 0x200) / 0x19;
    let pa0 = (sinr(heading, reach_tiles) + bx) >> 10;
    let pb0 = (by - cosr(heading, reach_tiles)) >> 10;
    // As coded, the divisor compares tile coordinates with a pin in map units, so it is 6 for nearly every pin.
    let x = tdist(ba, bb, pin.0, pin.1);
    let mut div = 4;
    if x < 100 {
        div = 2;
    }
    if x > 200 {
        div = 6;
    }
    let mut samples = 2;
    let mut sims = 0;
    let mut flag3 = 0;
    let mut grid = vec![[0i32; 6]; 21 * 21];
    let mut aim_to_pin = vec![[0i32; 6]; 21 * 21];
    let mut head_w = vec![[0u32; 6]; 21 * 21];
    let mut best_c: Option<(i32, i32)> = None;
    let mut best_pass = 0;
    let mut best_shape = 0;
    let mut best_roll = false;
    let diff = cl.difficulty;
    loop {
        let mut best = 99_999;
        for v in aim_to_pin.iter_mut() {
            *v = [0; 6];
        }
        for i in -10..=10 {
            for j in -10..=10 {
                let ci = ((i + 10) * 21 + (j + 10)) as usize;
                let (ca, cb) = (pa0 + i, pb0 + j);
                if grid[ci][1] >= 100_000 {
                    continue;
                }
                let d = isqrt_ru(ca * 0x400 + 0x200 - bx, cb * 0x400 + 0x200 - by);
                let hc = angle(ca * 0x400 + 0x200 - bx, cb * 0x400 + 0x200 - by);
                if grid[ci][1] == 0 {
                    grid[ci] = [100_000; 6];
                    let ct = h_index_type(c, ca, cb);
                    let mut valid = !(cl.g[g].flags & flag::PENALTY != 0 && (ca, cb) == prev)
                        && crate::course::inside(ca, cb)
                        && !c.oob(ca, cb)
                        && d <= maxr + 0x10
                        && (d >= maxr / 3 || d >= d_pin / 3 || cl.g[g].flags & flag::PENALTY != 0)
                        && (ca, cb) != (ba, bb)
                        && ct != t::TEE
                        && c.h(ct) <= 1;
                    if valid {
                        let ok1 = if c.h(ct) == 1 { (0..8).any(|u| h_at(c, ca + DX[u], cb + DY[u]) <= 0) } else { true };
                        let dc = len(ca - pin.0, cb - pin.1);
                        let db = len(ba - pin.0, bb - pin.1);
                        if ct == t::GREEN && dc > 3 {
                            valid = false;
                        }
                        if dc + if ok1 { 1 } else { db / 2 } > db {
                            valid = false;
                        }
                    }
                    if !valid {
                        continue;
                    }
                    grid[ci] = [0; 6];
                }
                for pass in 0..2 {
                    if pass == 1 && c.h(lie) > 0 {
                        break;
                    }
                    for shape in -1..=1i32 {
                        let e = (pass * 3 + 1 + shape) as usize;
                        if grid[ci][e] >= 99_999 {
                            continue;
                        }
                        if (shape == -1 && mask & 1 == 0) || (shape == 1 && mask & 2 == 0) || (shape != 0 && d <= 0x4b) {
                            grid[ci][e] = 100_000;
                            continue;
                        }
                        let mut acc = 0;
                        let mut good = 0;
                        for _ in 0..samples {
                            let (ax, ay) = if pass == 0 { (ca * 0x400 + 0x200, cb * 0x400 + 0x200) } else { (ca * 0x400, cb * 0x400) };
                            let end = preview(cl, c, rng, g, ax, ay, shape);
                            if pass == 0 {
                                sims += 1;
                            }
                            if cl.g[g].class & 4 != 0 {
                                for u in 0..8usize {
                                    let k = if u & 1 == 1 { 4 } else { 3 };
                                    let (qx, qy) = (end.0 + DX[u] * 0x400 / k, end.1 + DY[u] * 0x400 / k);
                                    let (qa, qb) = (qx >> 10, qy >> 10);
                                    acc += if c.oob(qa, qb) {
                                        c.h(t::OUT)
                                    } else {
                                        c.h(c.ty[(qa * 50 + qb) as usize]) - (c.flags[(qa * 50 + qb) as usize] & 0x80 != 0) as i32
                                    };
                                }
                            }
                            let (ea, eb) = (end.0 >> 10, end.1 >> 10);
                            let ef = flag_at(c, ea, eb);
                            let et = h_index_type(c, ea, eb);
                            let end_h = if c.oob(ea, eb) || ef & 0x400 != 0 {
                                c.h(t::OUT)
                            } else {
                                c.h(et)
                                    + if ef & 0x80 != 0 {
                                        if (ef & 0x1f) as i32 == cl.g[g].hole {
                                            -1
                                        } else {
                                            2
                                        }
                                    } else {
                                        0
                                    }
                            };
                            if c.h(et) <= 0 {
                                good += 1;
                            }
                            let mut hw = hc & !1;
                            let atp = tdist(ax, ay, pin.0, pin.1);
                            aim_to_pin[ci][e] = atp;
                            let epin = tdist(end.0, end.1, pin.0, pin.1);
                            if atp - epin > 25 && atp > 2 * epin {
                                hw |= 1;
                            }
                            head_w[ci][e] = hw;
                            acc += epin / div + end_h * 8;
                            if samples == 4 && cl.planner.mode != 2 && cl.g[g].class & 4 != 0 && !far {
                                if end_h > 0 {
                                    acc += route_risk(c, end.0, end.1, pin.0, pin.1, maxr, 0, 0) * 2;
                                } else {
                                    let mut reach = maxr;
                                    if cl.g[g].strokes == 0 {
                                        reach -= reach / 5;
                                    }
                                    if cl.g[g].class & 4 != 0
                                        && c.h(et) <= 0
                                        && epin >= 0x32
                                        && (reach - epin) * 0x3c / (reach * 3).max(1) >= 4
                                    {
                                        flag3 = 1;
                                    }
                                    let mut rr = route_risk(c, end.0, end.1, pin.0, pin.1, reach, 0, flag3);
                                    if mask & 1 != 0 {
                                        rr = rr.min(route_risk(c, end.0, end.1, pin.0, pin.1, reach, -1, flag3));
                                    }
                                    if mask & 2 != 0 {
                                        rr = rr.min(route_risk(c, end.0, end.1, pin.0, pin.1, reach, 1, flag3));
                                    }
                                    acc += rr / 2;
                                }
                            }
                        }
                        grid[ci][e] += acc;
                        if good >= ((diff + 4) * samples) / 8 {
                            cl.planner.no_safe = false;
                        }
                        if grid[ci][e] < best {
                            best = grid[ci][e];
                            best_c = Some((ca, cb));
                            best_pass = pass;
                            best_shape = shape;
                            cl.planner.land = cl.planner.end;
                            best_roll = head_w[ci][e] & 1 != 0;
                        }
                    }
                }
            }
        }
        if samples >= 8 || cl.planner.mode == 2 {
            break;
        }
        let mut tcut = 0x80;
        let mut kept;
        let (mut min_a, mut max_a, mut min_h, mut max_h);
        let hp = angle(pcx - bx, pcy - by);
        loop {
            kept = 0;
            (min_a, max_a, min_h, max_h) = (0xffff, -1000, 0x0fff_ffff, 0xe000_0000u32 as i32);
            for i in -10..=10 {
                for j in -10..=10 {
                    let ci = ((i + 10) * 21 + (j + 10)) as usize;
                    if grid[ci][1] >= 100_000 {
                        continue;
                    }
                    let mut dead = 0;
                    for e in 0..6 {
                        if grid[ci][e] >= 99_999 {
                            dead += 1;
                        } else if grid[ci][e] > best + tcut {
                            grid[ci][e] = 99_999;
                            dead += 1;
                        } else {
                            kept += 1;
                            if samples == 4 && aim_to_pin[ci][e] != 0 {
                                min_a = min_a.min(aim_to_pin[ci][e]);
                                max_a = max_a.max(aim_to_pin[ci][e]);
                                let (ca, cb) = (pa0 + i, pb0 + j);
                                if tdist(bx, by, ca, cb) > 100 {
                                    let v = head_w[ci][e].wrapping_sub(hp) as i32;
                                    min_h = min_h.min(v);
                                    max_h = max_h.max(v);
                                }
                            }
                        }
                    }
                    if dead == 6 {
                        grid[ci][1] = 100_000;
                    }
                }
            }
            if kept > 4 && tcut > 0x10 && sims + 3 * kept * samples > 250 {
                tcut /= 2;
                continue;
            }
            break;
        }
        if samples == 4 {
            cl.planner.hint = if max_a - min_a > 0x4b {
                1
            } else if max_h.wrapping_sub(min_h) > 0x1555_5555 {
                2
            } else {
                0
            };
        }
        samples *= 2;
        if kept < 2 {
            break;
        }
    }
    let shape = if let Some((ca, cb)) = best_c {
        cl.g[g].aim_a = ca;
        cl.g[g].aim_b = cb;
        cl.planner.corner = best_pass == 1;
        if best_roll {
            cl.planner.hint |= 4;
        }
        best_shape
    } else {
        cl.g[g].aim_a = pin.0;
        cl.g[g].aim_b = pin.1;
        cl.planner.corner = false;
        0
    };
    cl.game &= !game::AIM_SEARCH;
    cl.planner.class_bits = 7;
    if entry_mode != 2 {
        cl.planner.mode = 0;
    }
    shape
}
