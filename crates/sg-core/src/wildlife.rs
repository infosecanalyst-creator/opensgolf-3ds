//! Wildlife and the other ambient life: animals that move into a habitat when the player retypes the land (0x405e30), their
//! wandering, grazing and flight from golfers (0x430360), the four fly-overs (balloon, sailboat, blimp, bird of prey), the
//! water depth rule the water effects use, and the ball splash.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Wildlife and water"), restated in our own words.

use crate::course::{idx, inside, Course, TYPES};
use crate::geom::{cdir, len, DX, DY};
use crate::golfer::{Club, SLOTS};
use crate::land::ExeRng;

/// Animation states (sprite bases; add the kind): walk, stand, going down (to eat or drink), the eating / drinking loop.
pub const WALK: i32 = 0x103;
pub const STAND: i32 = 0x10c;
pub const DOWN: i32 = 0x115;
pub const LOOP: i32 = 0x11e;

/// One kind of animal: its name, its clips in Flics/Animals (walk, stand, down, loop), the directions of the non-walk clips
/// as the exe loads them, its call sound slot, and where it lives.
pub struct Kind {
    pub name: &'static str,
    pub clips: [&'static str; 4],
    pub dirs: i32,
    pub call: Option<i32>,
    /// Habitat tile type and theme mask (bit per exe theme), with the name, from the exe's kind table (0x4c1998, records of
    /// 18 bytes: the name in 16, the habitat type, the theme mask; EXACT, read from the publisher exe's data).
    pub habitat: u8,
    pub themes: u8,
}

pub const KINDS: [Kind; 9] = [
    Kind { name: "Elk", clips: ["ELK_Run", "ELK_Sq", "ELK_EatDown", "ELK_EatLoop"], dirs: 4, call: Some(59), habitat: 14, themes: 0b1001 },
    Kind {
        name: "Crane",
        clips: ["Crane_Walk", "Crane_Sq", "Crane_DrinkDown", "Crane_DrinkLoop"],
        dirs: 4,
        call: Some(58),
        habitat: 4,
        themes: 0b0001,
    },
    Kind {
        name: "Flamingo",
        clips: ["Flamingo_Walk", "Flamingo_Sq", "Flamingo_DrinkDown", "Flamingo_DrinkLoop"],
        dirs: 4,
        call: None,
        habitat: 15,
        themes: 0b0100,
    },
    Kind {
        name: "Sheep",
        clips: ["Sheep_Walk", "Sheep_SQ", "Sheep_EatDown", "Sheep_EatLoop"],
        dirs: 4,
        call: Some(195),
        habitat: 4,
        themes: 0b1000,
    },
    Kind {
        name: "Crocodile",
        clips: ["Croc_Walk", "Croc_SQ", "Croc_Turnover", "Croc_BackScratch"],
        dirs: 8,
        call: Some(196),
        habitat: 4,
        themes: 0b0100,
    },
    Kind {
        name: "Snake",
        clips: ["Snake_Walk", "Snake_SQ", "Snake_SQ", "Snake_Fidget"],
        dirs: 4,
        call: Some(198),
        habitat: 12,
        themes: 0b0110,
    },
    Kind { name: "Gila", clips: ["Gila_Walk", "Gila_SQ", "Gila_SQ", "Gila_Fidget"], dirs: 4, call: Some(197), habitat: 4, themes: 0b0010 },
    Kind {
        name: "Road runner",
        clips: ["RR_Walk", "RR_SQ", "RR_DrinkDown", "RR_DrinkLoop"],
        dirs: 4,
        call: None,
        habitat: 4,
        themes: 0b0010,
    },
    Kind {
        name: "Duck",
        clips: ["Duck_Walk", "Duck_SQ", "Duck_DrinkDown", "Duck_DrinkLoop"],
        dirs: 4,
        call: None,
        habitat: 4,
        themes: 0b1100,
    },
];

/// The drop-in offsets (times 4 pixels) over the first 20 frames: a fall from high up, then two bounces.
pub const DROP: [i32; 20] = [-55, -54, -52, -49, -45, -40, -34, -27, -19, -10, 0, -4, -7, -8, -7, -4, 0, -2, 0, 0];

/// One animal (record of 0x14 bytes at 0x572cb0).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Animal {
    pub x: i32,
    pub y: i32,
    /// Negative: dropping in (-20..-1); then bit 0 "getting up" (the down clip backwards), bit 1 fleeing.
    pub t: i32,
    pub leader: i32,
    pub state: i32,
    pub frame: i32,
    pub heading: i32,
    /// 0..8, or -1 for a free record.
    pub kind: i32,
}

impl Default for Animal {
    fn default() -> Self {
        Animal { x: 0, y: 0, t: 0, leader: -1, state: STAND, frame: 0, heading: 0, kind: -1 }
    }
}

impl Animal {
    pub fn clip(&self) -> &'static str {
        let k = &KINDS[self.kind.clamp(0, 8) as usize];
        match self.state {
            WALK => k.clips[0],
            DOWN => k.clips[2],
            LOOP => k.clips[3],
            _ => k.clips[1],
        }
    }

    /// The frame to draw: the down clip runs backwards while getting up.
    pub fn shown_frame(&self, frames: i32) -> i32 {
        if self.t < 0 {
            0
        } else if self.state == DOWN && self.t & 1 != 0 {
            (frames - self.frame - 1).max(0)
        } else {
            self.frame
        }
    }

    /// Drawn size in quarters: 4 for a leader, 3 for a follower (2 for a young crocodile).
    pub fn scale(&self) -> i32 {
        if self.leader == -1 {
            4
        } else if self.kind == 4 {
            2
        } else {
            3
        }
    }
}

/// A fly-over (4 fixed slots: 0 balloon, 1 sailboat, 2 bird of prey, 3 blimp).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Flyer {
    pub kind: i32,
    pub active: bool,
    pub x: i32,
    pub y: i32,
    pub h: i32,
    pub heading: i32,
}

pub const BALLOON: i32 = 0;
pub const SAILBOAT: i32 = 1;
pub const BLIMP: i32 = 2;
pub const BIRD: i32 = 3;

/// Bird of prey by exe theme.
pub const BIRDS: [&str; 4] = ["Eagle", "Vulture", "Pelican", "Hawk"];
/// Sailboat clip by exe theme.
pub const SAILBOATS: [&str; 4] = ["Sailboat_Move", "SailboatDesert", "SailboatTropical", "SailboatLinks"];

/// The world for wildlife: everything they look at.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Wildlife {
    pub animals: Vec<Animal>,
    pub flyers: [Flyer; 4],
    /// The one ball splash: position and frame (None when none plays).
    pub splash: Option<(i32, i32, i32)>,
    /// "<Name> habitat" labels to show: position, text and frames left.
    pub labels: Vec<(i32, i32, String, i32)>,
    /// Sounds to play: slot and position.
    #[serde(skip)]
    pub sounds: Vec<(i32, (i32, i32))>,
    /// Option bit 0x20 (wildlife on).
    pub enabled: bool,
}

impl Default for Wildlife {
    fn default() -> Self {
        Wildlife {
            animals: vec![Animal::default(); 128],
            flyers: [
                Flyer { kind: BALLOON, ..Default::default() },
                Flyer { kind: SAILBOAT, ..Default::default() },
                Flyer { kind: BIRD, ..Default::default() },
                Flyer { kind: BLIMP, ..Default::default() },
            ],
            splash: None,
            labels: Vec::new(),
            sounds: Vec::new(),
            enabled: true,
        }
    }
}

fn ty(c: &Course, x: i32, y: i32) -> u8 {
    let (a, b) = (x >> 10, y >> 10);
    if inside(a, b) {
        c.ty[idx(a, b)]
    } else {
        crate::course::t::OUT
    }
}

fn tile_ty(c: &Course, a: i32, b: i32) -> u8 {
    if inside(a, b) {
        c.ty[idx(a, b)]
    } else {
        crate::course::t::OUT
    }
}

/// The water depth of each tile (0x42f1c0): 0 at the shore, 1 next to it, 2 in open water; 255 off the water.
pub fn water_depth(c: &Course) -> Vec<u8> {
    let n = crate::course::N;
    let water = |a: i32, b: i32| inside(a, b) && TYPES[c.ty[idx(a, b)] as usize].class == 17;
    let mut d = vec![255u8; (n * n) as usize];
    for a in 0..n {
        for b in 0..n {
            if !water(a, b) {
                continue;
            }
            let shore = (0..8).any(|k| {
                let (na, nb) = (a + DX[k], b + DY[k]);
                inside(na, nb) && !water(na, nb)
            });
            d[idx(a, b)] = if shore { 0 } else { 2 };
        }
    }
    let first = d.clone();
    for a in 0..n {
        for b in 0..n {
            if first[idx(a, b)] == 2 && (0..8).any(|k| inside(a + DX[k], b + DY[k]) && first[idx(a + DX[k], b + DY[k])] == 0) {
                d[idx(a, b)] = 1;
            }
        }
    }
    d
}

impl Wildlife {
    fn alloc(&mut self, rng: &mut ExeRng, a: i32, b: i32, kind: i32) -> i32 {
        let Some(i) = self.animals.iter().position(|r| r.kind == -1) else { return -1 };
        let r = &mut self.animals[i];
        r.x = a * 1024 + 512;
        r.y = b * 1024 + 512;
        r.heading = rng.below(4);
        r.state = STAND;
        r.t = -20;
        r.kind = kind;
        r.leader = -1;
        i as i32
    }

    /// Puts an animal of `kind` and a follower on tile (a, b) (test hook; the game brings them through `retyped`).
    pub fn spawn(&mut self, rng: &mut ExeRng, a: i32, b: i32, kind: i32) {
        let l = self.alloc(rng, a, b, kind);
        if l >= 0 {
            let f = self.alloc(rng, a + 1, b, kind);
            if f >= 0 {
                self.animals[f as usize].leader = l;
            }
        }
    }

    /// The habitat check after a tile is retyped (0x405e30): one time in eight, if a random record is free, an animal whose
    /// habitat and theme fit a tile next to it moves in, with one young follower.
    pub fn retyped(&mut self, c: &Course, rng: &mut ExeRng, a: i32, b: i32, theme: u8) {
        let ta = a - 1 + rng.below(3);
        let tb = b - 1 + rng.below(3);
        if rng.below(8) != 0 {
            return;
        }
        let s = rng.below(128) as usize;
        if self.animals[s].kind != -1 || !self.enabled {
            return;
        }
        let here = tile_ty(c, ta, tb);
        let mut k = rng.below(9);
        let mut tries = 1;
        loop {
            let kd = &KINDS[k as usize];
            if here == kd.habitat && (kd.themes >> theme) & 1 != 0 && !c.oob(ta, tb) {
                break;
            }
            k = rng.below(9);
            tries += 1;
            if tries > 9 {
                return;
            }
        }
        let l = self.alloc(rng, ta, tb, k);
        let centre = (ta * 1024 + 512, tb * 1024 + 512);
        self.sounds.push((143, centre));
        let _group = rng.below(2);
        if l == -1 {
            return;
        }
        // the exe means to bring a small group but its loop always stops after one follower
        if self.animals.get(l as usize + 1).is_some_and(|r| r.kind == -1) {
            let fa = ta - 1 + rng.below(3);
            let fb = tb - 1 + rng.below(3);
            let f = self.alloc(rng, fa, fb, k);
            if f >= 0 {
                self.animals[f as usize].leader = l;
            }
        }
        self.labels.push((centre.0, centre.1, format!("{} habitat", KINDS[k as usize].name), 48));
        if let Some(s) = KINDS[k as usize].call {
            self.sounds.push((s, centre));
        }
    }
}

impl Club {
    /// One step of every animal (0x430360). `frames(kind, state)` gives a clip's frames per direction. Golfers, or balls in
    /// low flight, within two tiles of a calm leader scare it off; a golfer who sees that gets mood event 0x27.
    pub fn wildlife_tick(&mut self, c: &mut Course, rng: &mut ExeRng, frames: &dyn Fn(i32, i32) -> i32) {
        if !self.wildlife.enabled {
            return;
        }
        for r in 0..self.wildlife.animals.len() {
            let mut a = self.wildlife.animals[r];
            if a.kind == -1 {
                continue;
            }
            if a.kind > 8 {
                a.kind = 1;
            }
            if a.t < 0 {
                a.state = WALK;
                a.t += 1;
                if a.t == 0 {
                    if let Some(s) = KINDS[a.kind as usize].call {
                        self.wildlife.sounds.push((s, (a.x, a.y)));
                    }
                }
                self.wildlife.animals[r] = a;
                continue;
            }
            let n = frames(a.kind, a.state);
            a.frame = if n == 0 { 0 } else { (a.frame + 1) % n };
            let f = a.frame;
            if a.state == WALK {
                let px = a.x + DX[a.heading as usize] * 1024 / 3;
                let py = a.y + DY[a.heading as usize] * 1024 / 3;
                let pt = ty(c, px, py);
                if c.oob(px >> 10, py >> 10) || pt == crate::course::t::WATER || pt == crate::course::t::BUILDING {
                    a.heading = (a.heading + 1) & 7;
                    if rng.below(100) == 0 {
                        a.kind = -1;
                        self.wildlife.animals[r] = a;
                        continue;
                    }
                }
                let leader_flees = a.leader >= 0 && self.wildlife.animals[a.leader as usize].t & 2 != 0;
                let step = if a.t & 2 == 0 && !leader_flees { 64 / if a.kind == 0 { 2 } else { 4 } } else { 64 };
                a.x += DX[a.heading as usize] * step;
                a.y += DY[a.heading as usize] * step;
            }
            if f == 0 {
                let mut d;
                if a.leader == -1 {
                    if rng.below(3) == 0 {
                        a.t &= !2;
                    }
                    for g in 0..SLOTS {
                        let gg = &self.g[g];
                        if gg.hole == 0 {
                            continue;
                        }
                        let near = len(gg.x - a.x, gg.y - a.y) < 0x800
                            || (gg.speed > 0 && gg.bz < 150 && gg.bx != 0 && len(gg.bx - a.x, gg.by - a.y) < 0x800);
                        if near {
                            if a.t & 2 == 0 && gg.timer == 0 && gg.thought as u32 != 0x27 {
                                self.event(c, rng, g, 0x27, a.kind);
                            }
                            a.t |= 2;
                            let gg = &self.g[g];
                            a.heading = cdir(a.x - gg.x, a.y - gg.y);
                            if rng.below(3) == 0 {
                                if let Some(s) = KINDS[a.kind as usize].call {
                                    self.wildlife.sounds.push((s, (a.x, a.y)));
                                }
                            }
                        }
                    }
                    d = rng.below(8);
                    if a.state == WALK && rng.below(3) != 0 {
                        d = a.heading;
                    }
                    let hab = KINDS[a.kind as usize].habitat;
                    let next = ty(c, a.x + DX[d as usize] * 1024, a.y + DY[d as usize] * 1024);
                    if next != hab && next != crate::course::t::ROUGH && ty(c, a.x, a.y) == hab {
                        d = -1;
                    }
                    if a.t & 2 != 0 {
                        d = (a.heading - 1 + rng.below(3)) & 7;
                        a.heading = d;
                        a.state = WALK;
                    }
                } else {
                    let l = self.wildlife.animals[a.leader as usize];
                    let mut dist = len(l.x - a.x, l.y - a.y);
                    if l.t & 2 != 0 {
                        a.state = WALK;
                        dist = 0x2800;
                    }
                    let limit = ((a.state != WALK) as i32 + 2) * 1024 / ((a.kind != 0) as i32 + 2);
                    if dist > limit {
                        d = cdir(l.x - a.x, l.y - a.y);
                        if dist < 0x800 {
                            d = (d + (r as i32 % 3) - 1) & 7;
                        }
                        a.heading = d;
                    } else {
                        d = -1;
                    }
                }
                if a.state == LOOP && rng.below(2) != 0 {
                    a.state = DOWN;
                    a.t |= 1;
                } else if a.state == DOWN {
                    a.state = if a.t & 1 != 0 { STAND } else { LOOP };
                    a.t &= !1;
                } else {
                    if a.state == STAND && rng.below(10) == 0 {
                        a.state = DOWN;
                    }
                    if a.state == STAND && rng.below(2) == 0 && d != -1 {
                        a.state = WALK;
                        a.heading = d;
                    }
                    if a.state == WALK && (rng.below(5) == 0 || a.heading != d) {
                        a.state = STAND;
                    }
                }
            }
            if ty(c, a.x, a.y) == crate::course::t::OUT {
                a.kind = -1;
            }
            self.wildlife.animals[r] = a;
        }
        for l in self.wildlife.labels.iter_mut() {
            l.3 -= 1;
        }
        self.wildlife.labels.retain(|l| l.3 > 0);
    }

    /// The four fly-overs (second half of 0x430360). `sail_frames` is the sailboat clip's frame count.
    pub fn flyers_tick(&mut self, c: &Course, rng: &mut ExeRng, tick: u32) {
        let max = 0xcc00;
        let water = |a: i32, b: i32| inside(a, b) && TYPES[c.ty[idx(a, b)] as usize].class == 17;
        for i in 0..4 {
            let mut f = self.wildlife.flyers[i];
            match f.kind {
                BALLOON | BLIMP => {
                    if !f.active {
                        if rng.below(100) == 0 && tick > 0x2000 {
                            f = Flyer {
                                kind: f.kind,
                                active: true,
                                x: 0,
                                y: rng.below(50) << 10,
                                h: if f.kind == BALLOON { 40 } else { 80 },
                                heading: 2,
                            };
                        }
                    } else if f.kind == BALLOON {
                        f.x += f.h.clamp(0, 40) * 64 / 200;
                        if tick.is_multiple_of(4) {
                            let phase = (tick / (((f.y >> 10) * 4 + 256) as u32)) % 5;
                            let (a, b) = (f.x >> 10, f.y >> 10);
                            // the stored corner height (0x40c170, 3 off the map) times the height step
                            let ground = c.raw_corner(a, b) * crate::course::HEIGHT_STEP_PX;
                            match phase {
                                0 | 1 | 4 => {
                                    if f.h == 0 {
                                        self.wildlife.sounds.push((57, (f.x, f.y)));
                                    }
                                    let lift = if phase == 0 { 60 } else { 40 };
                                    if f.h < ground + lift {
                                        f.h += 1;
                                    }
                                }
                                _ => {
                                    let cls = |a: i32, b: i32| if inside(a, b) { TYPES[c.ty[idx(a, b)] as usize].class } else { 18 };
                                    let blocked = matches!(cls(a, b), 13 | 18 | 17)
                                        || matches!(cls(a + 1, b), 13 | 18)
                                        || (inside(a, b) && c.h(c.ty[idx(a, b)]) <= 0);
                                    if blocked {
                                        if f.h < 40 {
                                            f.h += 1;
                                        }
                                    } else if f.h > 0 {
                                        f.h -= 1;
                                    }
                                }
                            }
                        }
                        if f.x > max || f.y > max {
                            f.active = false;
                        }
                    } else {
                        f.x += f.h.clamp(0, 40) * 64 / 128;
                        if f.x > max || f.y > max {
                            f.active = false;
                        }
                    }
                }
                SAILBOAT => {
                    if !f.active {
                        if rng.below(100) == 0 {
                            let r = rng.below(50);
                            if water(r, 0) {
                                f = Flyer { kind: SAILBOAT, active: true, x: r * 1024 + 512, y: 512, h: 0, heading: 4 };
                            }
                        }
                    } else {
                        let speed = 128 / (((tick >> 8) & 3) as i32 + 3);
                        f.x += DX[f.heading as usize] * speed;
                        f.y += DY[f.heading as usize] * speed;
                        let ahead = |h: i32, k: i32| water((f.x + DX[h as usize] * k) >> 10, (f.y + DY[h as usize] * k) >> 10);
                        if tick & 7 == 0 && (f.heading != 0 || f.y > 0x800) && !ahead(f.heading, 1536) {
                            let mut best = (99, f.heading);
                            for e in [0, 2, 4, 6] {
                                if e == f.heading || !ahead(e, 1024) || !ahead(e, 2048) {
                                    continue;
                                }
                                let turn = ((f.heading - e).rem_euclid(8)).min((e - f.heading).rem_euclid(8));
                                let score = rng.below(15) + 16 * turn;
                                if score < best.0 {
                                    best = (score, e);
                                }
                            }
                            f.heading = best.1;
                        }
                        if (f.y < 0x200 && DY[f.heading as usize] == -1) || !inside(f.x >> 10, f.y >> 10) {
                            f.active = false;
                        }
                        if tick & 0x7f == 0 && f.active {
                            self.wildlife.sounds.push((209, (f.x, f.y)));
                        }
                    }
                }
                _ => {
                    if !f.active {
                        if rng.below(100) == 0 {
                            let r = rng.below(50) << 10;
                            let (x, y, h) = match rng.below(4) {
                                0 => (0, r, 2),
                                1 => (r, 0, 4),
                                2 => (0xc800, r, 6),
                                _ => (r, 0xc800, 0),
                            };
                            f = Flyer { kind: BIRD, active: true, x, y, h: 0, heading: h };
                        }
                    } else {
                        f.x += DX[f.heading as usize] * 128;
                        f.y += DY[f.heading as usize] * 128;
                        if !(0..=max).contains(&f.x) || !(0..=max).contains(&f.y) {
                            f.active = false;
                        }
                    }
                }
            }
            self.wildlife.flyers[i] = f;
        }
        if let Some(s) = self.wildlife.splash.as_mut() {
            s.2 += 1;
            if s.2 >= 13 {
                self.wildlife.splash = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_and_habitat() {
        let mut c = Course::default();
        for a in 10..16 {
            for b in 10..16 {
                c.ty[idx(a, b)] = crate::course::t::WATER;
            }
        }
        let d = water_depth(&c);
        assert_eq!(d[idx(10, 10)], 0);
        assert_eq!(d[idx(11, 11)], 1);
        assert_eq!(d[idx(12, 12)], 2);
        assert_eq!(d[idx(5, 5)], 255);
        // a habitat (the exe table at 0x4c1998): cranes on rough in parkland eventually move in when the land is retyped
        let mut w = Wildlife::default();
        for a in 20..30 {
            for b in 20..30 {
                c.ty[idx(a, b)] = crate::course::t::ROUGH;
            }
        }
        let mut rng = ExeRng::from_clock(3);
        for _ in 0..2000 {
            w.retyped(&c, &mut rng, 25, 25, 0);
        }
        let n = w.animals.iter().filter(|a| a.kind == 1).count();
        assert!(n >= 2, "cranes moved in: {n}");
        assert!(w.animals.iter().all(|a| a.kind == -1 || a.kind == 1));
    }
}
