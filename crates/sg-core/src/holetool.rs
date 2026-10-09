//! Building holes the way the exe does: painting a tee and a green records them for the hole being built (0x5685f0), a pass
//! of trial shots plans the hole while it is built (it sets the tee facing, the length, the dogleg and slope marks and the
//! yardage markers), and opening the hole gives it its par.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Building holes"), restated in our own words.

use crate::course::{f, idx, inside, t, Course};
use crate::geom::{angle, cosr, dir8, sinr, tdist, UNIT};
use crate::golfer::{game, Club, Event, Golfer, HOLE_RECORDS};
use crate::land::ExeRng;

/// The golfer record the layout pass plans with (record 0x9a, after the 152 golfers and two more the exe keeps).
pub const TRIAL: usize = 0x9a;

/// Sound the exe plays when a hole cannot be opened.
const REFUSED: i32 = 0x18;
/// Sound for a hole opening.
const OPENED: i32 = 0x2a;

fn centre(a: i32) -> i32 {
    a * UNIT + 0x200
}

impl Club {
    fn two_tees(&self) -> bool {
        self.game & game::TWO_TEES != 0
    }

    /// The hole bookkeeping of painting tile (a, b) from type `old` to type `new` (0x41fee2..0x42097e). `green_next` says
    /// whether an orthogonal neighbour is a green. Returns whether the paint goes ahead (the caller then charges and paints
    /// when the type changes); a refused tee or green can still have been recorded, as in the exe.
    pub fn paint_hole_tile(&mut self, c: &mut Course, a: i32, b: i32, old: u8, new: u8, green_next: bool) -> bool {
        if !inside(a, b) {
            return false;
        }
        let i = idx(a, b);
        let h = self.next_hole;
        // eighteen holes built: no more tees, nor greens that would start a new one
        if h >= 19 && (new == t::TEE || (new == t::GREEN && !green_next)) {
            return false;
        }
        let old_flags = c.flags[i];
        let cup = old == t::GREEN && old_flags & f::CUP != 0;
        if self.game & game::TOURNAMENT != 0 && (old == t::TEE || cup) {
            self.out.push(Event::Sound { slot: REFUSED, at: None, delay: 0 });
            return false;
        }
        let hu = h.clamp(0, HOLE_RECORDS as i32 - 1) as usize;
        let mut go = true;
        if new == t::TEE {
            if self.holes[hu].back.0 == 0 {
                if old == t::TEE {
                    go = false;
                } else if cup {
                    let k = (old_flags & f::LOW) as usize;
                    self.holes[k].par = 0;
                    self.holes[k].pin.0 = 0;
                }
                self.holes[hu].back = (a, b);
                c.flags[i] = h as u16;
                if !self.two_tees() {
                    self.holes[hu].fwd = (a, b);
                }
            } else {
                if self.two_tees() && self.holes[hu].fwd.0 == 0 && a != 0 {
                    self.holes[hu].fwd = (a, b);
                    c.flags[i] = h as u16;
                    c.growth[i] = 0;
                } else {
                    go = false;
                }
                self.message(format!("Hole {h} already has its tee; open it before starting the next hole."));
            }
        } else if new == t::GREEN {
            if self.holes[hu].pin.0 == 0 && a != 0 && (!green_next || self.holes[hu].back.0 != 0) {
                if old == t::TEE {
                    let k = (c.flags[i] & f::LOW) as usize;
                    self.holes[k].par = 0;
                    self.holes[k].back.0 = 0;
                } else if cup {
                    return false;
                }
                if h < 19 {
                    self.holes[hu].pin = (a, b);
                    c.flags[i] = h as u16 | f::CUP;
                }
            } else if old != t::GREEN {
                c.flags[i] &= !f::LOW;
            }
        }
        if !go {
            return false;
        }
        if new == old {
            return true;
        }
        // painting over a hole's tee or cup green takes the hole out of play until it is rebuilt
        let k = (old_flags & f::LOW) as i32;
        if k != 0 {
            let ku = k as usize;
            if old == t::TEE {
                self.next_hole = self.next_hole.min(k);
                let fwd = self.holes[ku].fwd;
                if (a, b) != fwd {
                    self.holes[ku].back.0 = 0;
                } else {
                    self.holes[ku].fwd.0 = 0;
                    if !self.two_tees() {
                        self.holes[ku].back.0 = 0;
                    }
                }
                self.holes[ku].par = 0;
                if !(new == t::GREEN && c.flags[i] & f::CUP != 0) {
                    c.flags[i] &= !f::LOW;
                }
            } else if old == t::GREEN {
                self.next_hole = self.next_hole.min(k);
                self.holes[ku].par = 0;
                self.holes[ku].pin.0 = 0;
                if new != t::TEE {
                    c.flags[i] &= !f::LOW;
                }
            }
        }
        true
    }

    /// Opens the hole being built (0x40e720): it needs a tee and a green (and the forward tee in two-tee games). Sets the
    /// length when the layout pass did not, the par from it, the heading from the forward tee to the pin, clears the hole's
    /// statistics, and moves on to the first hole number without a par. Returns the hole opened.
    pub fn open_hole(&mut self, c: &mut Course) -> Option<usize> {
        let h = self.next_hole;
        if !(1..19).contains(&h) {
            self.out.push(Event::Sound { slot: REFUSED, at: None, delay: 0 });
            return None;
        }
        let hu = h as usize;
        let two = self.two_tees();
        let hr = &self.holes[hu];
        if hr.back.0 == 0 || hr.pin.0 == 0 || (two && hr.fwd.0 == 0) {
            self.out.push(Event::Sound { slot: REFUSED, at: None, delay: 0 });
            return None;
        }
        self.top_rounds = [0; 10];
        self.top_names = Default::default();
        self.out.push(Event::Sound { slot: OPENED, at: None, delay: 0 });
        let level10 = c.level[10];
        let hr = &mut self.holes[hu];
        if hr.length == 0 {
            let (dx, dy) = ((hr.back.0 - hr.pin.0) as f64, (hr.back.1 - hr.pin.1) as f64);
            let mut l = ((dx * dx + dy * dy) * 625.0).sqrt() as i32 as i16 as i32;
            if l > 250 {
                l += (l - 250) / 4;
            }
            hr.length = l as i16 as i32;
        }
        let mut eff = hr.length;
        if hr.flags & 0x60 != 0 && eff > 250 {
            eff += 25;
        }
        if eff > 300 {
            eff -= 25 * level10;
        }
        hr.par = match eff {
            e if e > 625 => 6,
            e if e > 474 => 5,
            e if e > 249 => 4,
            e if e < 51 => 2,
            _ => 3,
        };
        hr.tee_shots = 0;
        hr.plans = 0;
        hr.fees = 0;
        hr.time = 0;
        hr.mood_sum = 0;
        hr.quits = 0;
        for cls in 0..8 {
            for col in 0..10 {
                if let Some(v) = hr.hist.get_mut(cls * 11 + col) {
                    *v = 0;
                }
            }
        }
        for v in hr.events.iter_mut().take(0x40) {
            *v = 0;
        }
        hr.fwd_facing = dir8(angle(hr.pin.0 - hr.fwd.0, hr.pin.1 - hr.fwd.1));
        let back = hr.back;
        if inside(back.0, back.1) {
            let i = idx(back.0, back.1);
            c.flags[i] |= f::GROWING;
            c.growth[i] = 0;
        }
        let par = hr.par;
        let len = hr.length;
        if h == 1 {
            let home = self.holes[19].back;
            self.first_post = Some(((home.0 + back.0 * 2) / 3, (home.1 + back.1 * 2) / 3));
        }
        self.next_hole = 1;
        while (self.next_hole as usize) < 19 && self.holes[self.next_hole as usize].par != 0 {
            self.next_hole += 1;
        }
        let mut msg = opening_text(h, par, len, eff, self.holes[hu].flags);
        let unlocked = self.unlocked;
        msg += &self.unlock_text(h);
        // a building unlocked with this hole speaks with its picture (minus its kind), otherwise nobody (0x40e720)
        let speaker = if self.unlocked != unlocked { -unlocked } else { -1 };
        self.message_by(msg, speaker, 1);
        self.log_event(crate::records::log::HOLE, h);
        let fl = self.holes[hu].flags;
        let pin = self.holes[hu].pin;
        let green = (pin.0 * 1024 + 512, pin.1 * 1024 + 512);
        if self.difficulty < 2 {
            if fl & 0x40 != 0 {
                self.award_at(0, green);
            }
            if fl & 0x20 != 0 {
                self.award_at(1, green);
            }
            if par == 5 {
                self.award_at(4, green);
            }
        }
        if h == 9 {
            self.award_at(6, green);
        }
        if h == 18 {
            self.award_at(14, green);
        }
        Some(hu)
    }

    /// The layout pass the exe's main frame runs over the hole being built once it has a tee and a green (0x41353d): a trial
    /// golfer of the Length and Accuracy classes plans the tee shot, then one of all three classes plays up to five shots
    /// toward the pin, stopping in a hazard. Their landings are kept and replayed until the course changes (game flag LAYOUT,
    /// cleared by every terrain refresh). Sets the tee facing toward the first landing, the length when the shots reach the
    /// pin, the dogleg and slope marks and the three yardage markers.
    pub fn layout(&mut self, c: &mut Course, rng: &mut ExeRng) {
        let h = self.next_hole;
        if !(1..=18).contains(&h) {
            return;
        }
        let hu = h as usize;
        let (back, pin) = (self.holes[hu].back, self.holes[hu].pin);
        if back.0 == 0 || pin.0 == 0 {
            return;
        }
        let replay = self.game & game::LAYOUT != 0;
        if !replay {
            self.layout_land.clear();
        }
        let mut shot = 0usize;
        let mut sum = 0;
        let mut first = back;
        let mut last = back;
        for m in 2..4 {
            sum = 0;
            let mut g = Golfer { class: if m == 2 { 3 } else { 7 }, level: 1, hole: h, ..Default::default() };
            let (x, y) = (centre(back.0), centre(back.1));
            g.x = x;
            g.ox = x;
            g.bx = x;
            g.y = y;
            g.oy = y;
            g.by = y;
            self.g[TRIAL] = g;
            loop {
                let land = if !replay {
                    self.planner.mode = 2;
                    crate::planner::plan_shot(self, c, rng, TRIAL, false, -1, 0, 0);
                    self.planner.mode = 0;
                    let l = (self.planner.land.0 >> 10, self.planner.land.1 >> 10);
                    self.layout_land.push(l);
                    l
                } else {
                    self.layout_land.get(shot).copied().unwrap_or(pin)
                };
                shot += 1;
                self.g[TRIAL].aim_a = land.0;
                self.g[TRIAL].aim_b = land.1;
                last = land;
                if self.g[TRIAL].strokes == 0 && m == 2 {
                    first = land;
                    break;
                }
                if c.h(c.tile_type(land.0, land.1)) > 1 {
                    break;
                }
                let gg = &self.g[TRIAL];
                sum += tdist(gg.ox, gg.oy, land.0, land.1);
                let (x, y) = (centre(land.0), centre(land.1));
                let gg = &mut self.g[TRIAL];
                gg.x = x;
                gg.ox = x;
                gg.bx = x;
                gg.y = y;
                gg.oy = y;
                gg.by = y;
                gg.strokes += 1;
                if gg.strokes >= 5 || land == pin {
                    break;
                }
            }
        }
        self.game |= game::LAYOUT;
        let hr = &mut self.holes[hu];
        // (the exe scales the sum by 25 and back)
        hr.length = if last == pin { sum } else { 0 };
        hr.tee_facing = dir8(angle(first.0 - back.0, first.1 - back.1));
        hr.flags &= !0x3000;
        let (hb, hp) = (c.raw_corner(back.0, back.1), c.raw_corner(pin.0, pin.1));
        if hp > hb + 1 {
            hr.flags |= 0x1000;
        }
        if hp < hb - 1 {
            hr.flags |= 0x2000;
        }
        if sum < 250 {
            first = back;
        }
        let mut bend = angle(pin.0 - back.0, pin.1 - back.1).wrapping_sub(angle(pin.0 - first.0, pin.1 - first.1)) as i32;
        if first == pin {
            bend = 0;
        }
        hr.flags &= !0x60;
        if bend > 0x071c_71c6 {
            hr.flags |= 0x20;
        }
        if bend < -0x071c_71c6 {
            hr.flags |= 0x40;
        }
        // yardage markers at 150, 200 and 250 from the pin, on fairway clear of its edges, swept around the line back
        // toward the first landing (the last fit in the sweep wins)
        let a0 = angle(first.0 - pin.0, first.1 - pin.1);
        hr.markers = [(-1, -1); 3];
        let fairway = |a: i32, b: i32| inside(a, b) && c.ty[idx(a, b)] == t::FAIRWAY;
        let mut off: i32 = 0x0aaa_aa9a;
        for count in (1..=25).rev() {
            let sign: i32 = if count & 1 != 0 { 1 } else { -1 };
            let ang = a0.wrapping_add(sign.wrapping_mul(off / 8) as u32);
            for (k, r) in [150, 200, 250].into_iter().enumerate() {
                if r >= sum {
                    continue;
                }
                let d = (0x19000 + k as i32 * 0xc800) / 25;
                let x = sinr(ang, d) + pin.0 * UNIT + 0x200;
                let y = pin.1 * UNIT - cosr(ang, d) + 0x200;
                let (a, b) = (x >> 10, y >> 10);
                if !fairway(a, b) {
                    continue;
                }
                let (sa, sb) = ((x % 1024) >> 4, (y % 1024) >> 4);
                if (sa < 0x18 && !fairway(a - 1, b))
                    || (sa >= 0x30 && !fairway(a + 1, b))
                    || (sb < 0x18 && !fairway(a, b - 1))
                    || (sb >= 0x30 && !fairway(a, b + 1))
                {
                    continue;
                }
                hr.markers[k] = (x, y);
            }
            off = off.wrapping_sub(0x0aaa_aaaa);
        }
    }
}

/// The announcement of a new hole (0x40e720): "Hole #3, a scenic 328 yard par 4, is now open for play." A par 3 under 125
/// yards (par 4 under 350, par 5 under 500) is short, one over 200 (425, 575) long; the dogleg and slope come from the
/// hole's flags.
pub fn opening_text(h: i32, par: i32, len: i32, eff: i32, flags: u32) -> String {
    let size = match par {
        3 if eff < 125 => "short ",
        3 if eff > 200 => "long ",
        4 if eff < 350 => "short ",
        4 if eff > 425 => "long ",
        5 if eff < 500 => "short ",
        5 if eff > 575 => "long ",
        _ => "scenic ",
    };
    let mut s = format!("Hole #{h}, a {size}{len} yard ");
    if flags & 0x1000 != 0 {
        s += "uphill ";
    }
    if flags & 0x2000 != 0 {
        s += "downhill ";
    }
    s += &format!("par {par}");
    if flags & 0x20 != 0 {
        s += " with a dogleg to the left";
    }
    if flags & 0x40 != 0 {
        s += " with a dogleg to the right";
    }
    s + ", is now open for play."
}

/// What each building does, as the unlock message tells it (kinds 6..14).
const UNLOCK_BLURB: [&str; 9] = [
    " allows your golfers with imagination ability to improve their putting.",
    " will feed golfers who get hungry and provide some extra revenue.",
    " allows your accurate golfers to improve their equipment.",
    " helps your players to begin play with a better attitude.",
    " allows your long hitters to improve the length of their shots.",
    " increases your players speed of play.",
    " increases the value of all homes and lots at your course.",
    " keeps your players from getting tired towards the end of their round.",
    " allows you to charge higher greens fees at your course.",
];

/// The course rank upgrade (0x40e5f0) on reaching 6, 10 and 18 holes.
fn rank_text(step: usize) -> String {
    let names = crate::economy::RANK_NAMES;
    let mut s = format!(
        " Your course is now upgraded from a {} to a {} course. This will DOUBLE your tournament prize monies if your course passes the more stringent SGA evaluation. ",
        names[step],
        names[step + 1]
    );
    match step {
        0 => s += "In addition you may now hire more experienced employees, for a slightly higher salary of course.",
        1 => s += "In addition you may build upgraded buildings which are more effective.",
        _ => {}
    }
    s
}

impl Club {
    /// What opening hole `h` adds to its announcement: the rank upgrade at 6, 10 and 18 holes, otherwise the next building
    /// kind while the course is growing (the counter only ever rises).
    fn unlock_text(&mut self, h: i32) -> String {
        let mut s = String::new();
        if h == 6 {
            s += &rank_text(0);
        } else if h == 10 {
            s += &rank_text(1);
        } else if h > self.unlocked - 5 && self.unlocked <= 14 && !self.sandbox {
            s += if h & 1 == 0 {
                " As your course grows you can add new facilities. "
            } else {
                " New players are flocking to your golf course. "
            };
            let k = self.unlocked;
            let name = crate::land::BUILDINGS[k as usize].0;
            s += &format!("You may now build a {name}. A {name}{}", UNLOCK_BLURB[(k - 6) as usize]);
            self.unlocked += 1;
        }
        if h == 18 {
            s += &rank_text(2);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::Course;

    fn course() -> Course {
        let mut c = Course::default();
        for v in c.ty.iter_mut() {
            *v = t::FAIRWAY;
        }
        c
    }

    #[test]
    fn paint_and_open() {
        let mut cl = Club::default();
        let mut c = course();
        let mut rng = ExeRng::from_clock(1);
        assert!(cl.paint_hole_tile(&mut c, 10, 10, t::FAIRWAY, t::TEE, false));
        c.ty[idx(10, 10)] = t::TEE;
        assert!(cl.open_hole(&mut c).is_none(), "a hole needs a green");
        assert!(cl.paint_hole_tile(&mut c, 10, 25, t::FAIRWAY, t::GREEN, false));
        c.ty[idx(10, 25)] = t::GREEN;
        assert_eq!(c.flags[idx(10, 25)] & (f::LOW | f::CUP), 1 | f::CUP);
        cl.layout(&mut c, &mut rng);
        assert!(cl.game & game::LAYOUT != 0);
        assert_eq!(cl.open_hole(&mut c), Some(1));
        assert!(cl.holes[1].par >= 3, "15 tiles is a par 4 or so, got {}", cl.holes[1].par);
        assert_eq!(cl.next_hole, 2);
        // painting over the tee closes the hole again
        assert!(cl.paint_hole_tile(&mut c, 10, 10, t::TEE, t::ROUGH, false));
        assert_eq!(cl.holes[1].par, 0);
        assert_eq!(cl.next_hole, 1);
    }

    #[test]
    fn short_hole_par() {
        let mut cl = Club::default();
        let mut c = course();
        cl.paint_hole_tile(&mut c, 10, 10, t::FAIRWAY, t::TEE, false);
        cl.paint_hole_tile(&mut c, 10, 11, t::FAIRWAY, t::GREEN, false);
        assert_eq!(cl.open_hole(&mut c), Some(1));
        assert_eq!(cl.holes[1].length, 25);
        assert_eq!(cl.holes[1].par, 2);
    }
}
