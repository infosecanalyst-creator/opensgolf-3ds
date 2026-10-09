//! The player's own golfer (the club's pro, "Gary Golf"): a round started from the pro panel (practice, or a match against a
//! famous golfer who has challenged the club), the player aiming each full shot with a shot type, the match money, and the
//! pro's ten skills, which the player sets with points and which grow or shrink with remarkable shots.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "The player's pro"), restated in our own words.

use crate::course::{idx, inside, Course};
use crate::economy::money_digits;
use crate::geom::{angle, clamp, cosr, dir8, len, sinr, tdist};
use crate::golfer::{flag, game, Club, Column, Golfer, SLOTS};
use crate::land::ExeRng;

/// A famous golfer's challenge is waiting to be played (game flag 0x2000); the pro's round starts next tick (0x4000).
pub const CHALLENGE: u32 = 0x2000;
pub const START_ROUND: u32 = 0x4000;

/// Skill names in the order of the skill bytes.
pub const SKILL_NAMES: [&str; 10] = [
    "Power Hitter",
    "Long Driver",
    "Accurate Driver",
    "Accurate Irons",
    "Accurate Putter",
    "Draw Shot",
    "Fade Shot",
    "High Backspin Shot",
    "Recovery Skills",
    "Luck",
];

/// Club names for the panel. APPROXIMATION: the exe's club name routine (0x40a9a0) was not decoded; these follow the
/// club numbers 0 (longest) to 11 (shortest) the planner uses.
pub const CLUB_NAMES: [&str; 12] =
    ["Driver", "3 Wood", "5 Wood", "2 Iron", "3 Iron", "4 Iron", "5 Iron", "6 Iron", "7 Iron", "8 Iron", "9 Iron", "Wedge"];

/// The lie under the ball, for the panel. APPROXIMATION: the names in the exe's terrain records are not available.
pub fn lie_name(ty: u8) -> &'static str {
    use crate::course::t;
    match ty {
        t::TEE => "Tee",
        t::GREEN => "Green",
        t::FAIRWAY | 3 => "Fairway",
        t::SAND | 8 => "Sand Trap",
        t::POT => "Pot Bunker",
        t::RAVINE => "Ravine",
        t::ROCKS => "Rocks",
        t::WATER => "Water",
        t::MARSH => "Marsh",
        t::OUT => "Out of Bounds",
        _ => "Rough",
    }
}

/// Whether skill k counts for the shot being aimed (the panel draws it white, else grey).
pub fn skill_applies(k: usize, club: i32, strokes: i32, opt: i32, hazard: bool) -> bool {
    match k {
        0 => true,
        1 | 2 => club <= 3 && strokes == 0,
        3 => club > 3,
        5 => opt == 1,
        6 => opt == -1,
        7 => opt == 3,
        8 => hazard,
        _ => false,
    }
}

/// The aim line's points in map units from the ball (0x41bbe1): fractions 5, 9, 12, 15, 16 (and 20 except for the high
/// shot) of the distance over 16 or 20, bending right for a draw and left for a fade and coming back to the heading.
/// The second point's bend is a medium-confidence reading; the screen heights are left to the caller.
pub fn aim_line(ball: (i32, i32), p: &AimPreview, opt: i32) -> Vec<(i32, i32)> {
    let h = angle(p.x - ball.0, p.y - ball.1);
    let d = if opt == 3 { 16 } else { 20 };
    let s = if opt.abs() <= 1 { 2 * opt } else { 0 };
    let c = s.wrapping_mul(0x0aaa_aaaa);
    let bends = [c, s.wrapping_mul(0x0800_0000), c / 2, c / 3, c / 4, 0];
    let fr: &[i32] = if opt == 3 { &[5, 9, 12, 15, 16] } else { &[5, 9, 12, 15, 16, 20] };
    let mut out = vec![ball];
    for (k, &f) in fr.iter().enumerate() {
        let r = ((p.distance * f / d) << 10) / 25;
        let a = h.wrapping_add(if k + 1 == fr.len() { 0 } else { bends[k] } as u32);
        out.push((ball.0 + sinr(a, r), ball.1 - cosr(a, r)));
    }
    out
}

/// What the aiming frame shows: the shot as the planner would take it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AimPreview {
    /// Aim point in map units (tile centre, or its corner when the pointer is nearer a corner).
    pub x: i32,
    pub y: i32,
    /// Distance in range units, clamped to the shot's maximum, and the club the preview picks.
    pub distance: i32,
    pub club: i32,
    pub max_range: i32,
}

/// The "Attitude" word for the pro's momentum.
pub fn attitude(m: i32) -> &'static str {
    match clamp(m, -4, 4) {
        -4 => "furious",
        -3 => "mad",
        -2 => "upset",
        -1 => "worried",
        0 => "calm",
        1 => "determined",
        2 => "pumped",
        _ => "invincible",
    }
}

impl Club {
    /// Panel button 1 (practice round) or 2 (the match): asks for the pro's round to start next tick.
    pub fn request_pro_round(&mut self, match_play: bool) -> bool {
        let holes = self.next_hole >= 2;
        let special = self.game & 0x4200000 != 0;
        if !holes || special || self.gary != -1 {
            return false;
        }
        if match_play {
            if self.game & CHALLENGE == 0 || self.challenge_pro == -1 {
                return false;
            }
            self.game |= START_ROUND;
        } else {
            self.game = (self.game & !CHALLENGE) | START_ROUND;
        }
        true
    }

    /// An accomplishment (0x46e7b0): each is earned once; once the pro has skills, each brings three skill points to hand
    /// out. Returns whether it was new.
    pub fn award(&mut self, id: u32) -> bool {
        // nothing is earned in a championship (0x46e7b0)
        if id >= 32 || self.awards & (1 << id) != 0 || self.game & crate::championship::CHAMPIONSHIP != 0 {
            return false;
        }
        self.awards |= 1 << id;
        if self.pro_mask != 0 {
            self.skill_points += 3;
        }
        true
    }

    /// Points for the first allocation (0x4065c0): ten plus three for each accomplishment already earned.
    pub fn first_skill_points(&self) -> i32 {
        10 + 3 * self.awards.count_ones() as i32
    }

    /// Whether the pro panel's skill points must be handed out before the round can start.
    pub fn needs_skill_points(&self) -> bool {
        self.pro_mask == 0
    }

    /// Starts the pro's round (0x40f190): golfers waiting or not yet started on hole 1 go home, a partner is made (the
    /// challenger in a match) and the pro joins at his employee's place.
    pub fn start_pro_round(&mut self, c: &Course, rng: &mut ExeRng, pos: (i32, i32), facing: i32) {
        let with = self.game & CHALLENGE != 0;
        self.sound(0x2f, None);
        self.pro_mask = 0;
        for k in 0..12 {
            if self.pro_skill[k] != 0 {
                self.pro_mask |= 1 << k;
            }
        }
        let fp = self.challenge_pro;
        if self.create_counter & 2 != 0 {
            self.create_counter = (self.create_counter + 2) % SLOTS as i32;
        }
        for s in 0..SLOTS {
            if (self.g[s].hole == 1 && self.g[s].strokes == 0) || self.g[s].hole == -1 {
                self.g[s].strokes = 0;
                self.g[s].anim = 0;
                self.g[s].hole = 0;
            }
        }
        let mut a = self.create(c, rng, false);
        if a & 1 != 0 {
            a = self.create(c, rng, false);
        }
        if a != -1 {
            self.last_created = a;
        }
        let p = self.create(c, rng, false);
        if a < 0 || p < 0 {
            return;
        }
        self.last_created = p;
        let (a, p) = (a as usize, p as usize);
        self.g[a].partner = p as i32;
        self.g[p].partner = a as i32;
        self.g[a].pause = 0;
        if with && fp >= 0 {
            let pro = self.pros.get(fp as usize).cloned().unwrap_or_default();
            let ga = &mut self.g[a];
            ga.kind = 0x20;
            ga.famous = fp;
            ga.roster = crate::vips::ROSTER_CHALLENGER;
            ga.flags &= !0x1c;
            ga.level = clamp(self.difficulty, 0, 2);
            ga.class = ((self.wager_level * 16 + 7) & 0xff) as u8;
            ga.skill_mask = 0xe0;
            for k in 0..12 {
                ga.skills[k] = pro.skills[k];
                if pro.skills[k] != 0 {
                    ga.skill_mask |= 1 << k;
                }
            }
            if let Some(r) = self.roster.get_mut(crate::vips::ROSTER_CHALLENGER as usize) {
                r.name = pro.name.clone();
                r.job = "Golf Pro".to_string();
            }
            self.challenge_pro = -1;
        }
        self.game &= !(CHALLENGE | START_ROUND);
        let tick = self.tick;
        for gg in [a, p] {
            let g = &mut self.g[gg];
            g.hole = 1;
            g.hole_tick = tick;
            g.story = -1;
            g.field_ae = 0;
        }
        self.g[a].flags &= !flag::STORY;
        let gp = &mut self.g[p];
        gp.flags &= !0x10_001c;
        gp.kind = 0x24;
        gp.roster = 0;
        gp.level = 2;
        gp.class = 0x47;
        gp.skill_mask = self.pro_mask;
        for k in 0..12 {
            if self.pro_mask & (1 << k) != 0 {
                gp.skills[k] = self.pro_skill[k];
            }
        }
        gp.flags |= flag::GARY;
        gp.x = pos.0;
        gp.y = pos.1;
        gp.pause = 6;
        gp.facing = facing;
        self.gary = p as i32;
    }

    /// The pro waits for the player to aim (+0xcc == -1, the panel in aiming mode).
    pub fn pro_aiming(&self) -> Option<usize> {
        let g = self.gary;
        if g < 0 {
            return None;
        }
        let gg = &self.g[g as usize];
        (gg.aim_a == -1 && gg.flags & flag::GARY != 0).then_some(g as usize)
    }

    /// Picks the shot type (straight 0, fade -1, draw 1, high 3, low 4).
    pub fn set_shot_option(&mut self, opt: i32) {
        self.planner.option = opt;
    }

    /// One aiming frame (0x41b20f): the hovered tile (or its nearest corner), the shot type's limits, the facing and the
    /// preview club. Refuses a curved or high shot from a hazard.
    pub fn aim_frame(&mut self, c: &Course, tile: (i32, i32), corner: bool) -> Option<AimPreview> {
        let g = self.pro_aiming()?;
        self.aim_tile = tile;
        self.aim_corner = corner;
        let (px, py) = if corner { (tile.0 * 1024, tile.1 * 1024) } else { (tile.0 * 1024 + 512, tile.1 * 1024 + 512) };
        let mut maxr = crate::planner::max_range(self, c, g);
        let opt = self.planner.option;
        if opt == 3 {
            maxr = 13 * maxr / 18;
        } else if opt == 4 {
            maxr /= 2;
        }
        let (bx, by) = (self.g[g].bx, self.g[g].by);
        let lie = if inside(bx >> 10, by >> 10) { c.ty[idx(bx >> 10, by >> 10)] } else { crate::course::t::OUT };
        if matches!(opt, 3 | 1 | -1) && c.h(lie) > 0 {
            self.planner.option = 0;
            self.sound(0x18, None);
        }
        if self.aim_lock != 0 {
            self.aim_lock -= 1;
        }
        let d = len(bx - px, by - py) * 25 / 1024;
        let cd = clamp(d, 0, maxr);
        let gg = &mut self.g[g];
        gg.facing = dir8(angle(px - bx, py - by));
        let club = clamp(((maxr - cd) * 60) / (maxr * 3).max(1), (gg.strokes != 0) as i32, 11);
        gg.club = club;
        Some(AimPreview { x: px, y: py, distance: cd, club, max_range: maxr })
    }

    /// A click while aiming (0x41f29d): the hovered tile becomes the aim and the pro swings next tick.
    pub fn commit_aim(&mut self) -> bool {
        let Some(g) = self.pro_aiming() else { return false };
        if self.aim_lock >= 1 {
            return false;
        }
        self.g[g].aim_a = self.aim_tile.0;
        self.g[g].aim_b = self.aim_tile.1;
        true
    }

    /// Key 'n': the pro's round is cancelled (not in a tournament); his partner plays on alone.
    pub fn cancel_pro_round(&mut self, rng: &mut ExeRng) -> bool {
        if self.game & game::TOURNAMENT != 0 || self.gary <= 0 {
            return false;
        }
        for g in 0..SLOTS {
            if self.g[g].flags & flag::GARY != 0 {
                self.g[g].flags &= !flag::GARY;
                let k = rng.below(3);
                self.g[g].class = (self.g[g].class & 7).wrapping_add(0xffu8.wrapping_shl(k as u32));
                self.message("Match canceled.".to_string());
                self.round_end(rng, g);
                self.g[g].strokes = 0;
                self.g[g].anim = 0;
                self.g[g].hole = 0;
            }
        }
        self.gary = -1;
        true
    }

    /// The famous-golfer challenge (main frame 0x40fe97), checked when golfer s has just arrived: a newcomer with all three
    /// skill classes from the second year on, with L + 3 holes built and no pro on the course, brings a challenge from a
    /// famous golfer whose skills suit the wager level L.
    pub(crate) fn pro_challenge_check(&mut self, rng: &mut ExeRng, s: usize) {
        let ok = self.challenge_pro == -1 && !self.g.iter().take(SLOTS).any(|o| o.vip() == 0x20 && o.hole != 0);
        let l = self.wager_level;
        if !ok || self.g[s].class != 7 || self.tick <= 0x2000 || self.par(l + 3) == 0 || self.pros.is_empty() {
            return;
        }
        let mut tries = 0;
        let r = loop {
            let r = loop {
                let r = rng.below(100);
                tries += 1;
                if self.pros.get(r as usize).is_some() {
                    break r;
                }
            };
            let s12: i32 = self.pros[r as usize].skills.iter().map(|&v| v as i32).sum::<i32>() - 20;
            let q = tries / 4;
            if 5 * l - q <= s12 && s12 <= 5 * l + q {
                break r;
            }
        };
        let pro = self.pros[r as usize].clone();
        let best = pro.skills.iter().copied().max().unwrap_or(0);
        let female = pro.body >= 4;
        // the exe's pieces (0x4c6ba4, 0x4c6b68, 0x4c6b58, 0x4c6b44, 0x4c6b2c / 0x4c6b18, 0x4c6afc, 0x4c6af4, 0x4c6ad0,
        // 0x4c6aa0); the amounts are plain numbers after the section sign
        let mut text = format!(
            "Famous golfer {} challenges you to a match at your course with a wager of \u{a7}{} per hole and \u{a7}{} for the match! ",
            pro.name,
            (5 * l + 5) * 400,
            (5 * l + 5) * 800
        );
        if best > 0 {
            let names: Vec<&str> = (0..10).filter(|&k| pro.skills[k] == best).map(|k| SKILL_NAMES[k]).collect();
            text += &format!(
                "{} is proud of {} {}0 percent skill rating in {}",
                if female { "She" } else { "He" },
                if female { "her" } else { "his" },
                best,
                names.join(" and ")
            );
        }
        let pro_name = self.roster.first().map(|p| p.name.clone()).unwrap_or_else(|| "Gary Golf".to_string());
        text += &format!(". Click on the match icon on the {pro_name} panel when you are ready to start the match.");
        self.message(text);
        self.game |= CHALLENGE;
        self.sound(0x26, None);
        self.wager_level = l + 1;
        self.challenge_pro = r;
    }

    /// Match money at the end of a hole (0x427380), for the later finisher of a pair that both carry a stake: 20 * L units
    /// for each hole won or lost, and again for the match on the last hole.
    pub(crate) fn match_money(&mut self, g: usize) {
        let p = self.g[g].partner.clamp(0, SLOTS as i32 - 1) as usize;
        if self.game & game::TOURNAMENT != 0 || self.g[g].class & 0xf0 == 0 || self.g[p].class & 0xf0 == 0 {
            return;
        }
        let h = self.g[g].hole;
        if !(self.g[g].hole < self.g[p].hole || self.g[p].hole == 0) {
            return;
        }
        let (e, o) = if g & 1 == 0 { (g, p) } else { (p, g) };
        let hu = h.clamp(0, 18) as usize;
        let a = self.wager_level * 20;
        let (so, se) = (self.g[o].card[hu] as i32, self.g[e].card[hu] as i32);
        let at = (self.g[o].x, self.g[o].y);
        if so < se {
            // the exe's pieces (0x4c7324, 0x4c7314, 0x4c730c, 0x4c7300 or 0x4c72f8, 0x4c4944)
            let text =
                format!("{} wins hole #{h} by a score of {so} to {se}! Collect \u{a7}{}.", self.name(o), money_digits(a as i64 * 100));
            self.message_by(text, self.gary, 0);
            self.earn(a, Column::Other, at);
            self.sound(0x23, None);
        }
        if se < so {
            let text = format!("{} wins hole #{h} by a score of {se} to {so}. Pay \u{a7}{}.", self.name(e), money_digits(a as i64 * 100));
            self.message_by(text, self.gary, 0);
            self.earn(-a, Column::Other, at);
            self.sound(0x24, None);
        }
        if self.par(h + 1) == 0 {
            let total = |c: &Golfer| (1..=hu).map(|k| c.card[k] as i32).sum::<i32>();
            let (to, te) = (total(&self.g[o]), total(&self.g[e]));
            if te < to {
                // 0x4c72dc with 0x4c72d0, 0x4c72b4 or 0x4c72a8 (the exe goes on with the standings, not in the port)
                self.message(format!("At the end of the match, you lose \u{a7}{}.", money_digits(a as i64 * 100)));
                self.earn(-a, Column::Other, at);
                self.sound(0x24, None);
                self.wager_level -= 1;
            } else if te == to {
                self.message("At the end of the match, the score is tied.".to_string());
                self.wager_level -= 1;
            } else {
                self.message(format!("At the end of the match, you win \u{a7}{}.", money_digits(a as i64 * 100)));
                self.earn(a, Column::Other, at);
                self.sound(0x23, None);
                self.trophies += 1;
                let at = (self.g[o].x, self.g[o].y);
                self.award_at(5, at);
                let famous = self.g[e].famous;
                self.log_event(crate::records::log::MATCH, famous);
            }
            let par: i32 = (1..=h).map(|k| self.par(k)).sum();
            let (no, ne) = (self.name(o), self.name(e));
            if to == te {
                let d = to - par;
                let at = match d {
                    0 => "even par".to_string(),
                    d if d < 0 => format!("{} under par", -d),
                    d => format!("{d} over par"),
                };
                self.message_by(format!("{no} and {ne} are tied at {at}."), self.gary, 1);
            } else {
                let n = (to - te).abs();
                let word = if to < te { "leads" } else { "trails" };
                self.message_by(format!("{no} {word} {ne} by {n} shot{}.", if n == 1 { "" } else { "s" }), self.gary, 1);
                self.sound(if to < te { 0x2f } else { 0x30 }, None);
            }
            self.gary = -1;
        }
    }

    /// When a shot comes to rest (the exe does it when the shot's replay ends, 0x409620): an ordinary golfer's or the pro's
    /// shot far beyond what its difficulty predicts is a great shot; for the pro a skill may grow, and a shot that ends in a
    /// hazard may cost him one.
    pub(crate) fn shot_review(&mut self, c: &Course, rng: &mut ExeRng, g: usize) {
        if g as i32 != self.gary || self.g[g].hole < 1 || self.g[g].hole > 18 {
            return;
        }
        let gg = &self.g[g];
        let l = len(gg.ox - gg.bx, gg.oy - gg.by) * 25 / 1024;
        let pin = self.holes[gg.hole as usize].pin;
        let d = tdist(gg.bx, gg.by, pin.0, pin.1);
        let s: i32 = gg.skills[..10].iter().map(|&v| v as i32).sum();
        let ty = |x: i32, y: i32| if inside(x >> 10, y >> 10) { c.ty[idx(x >> 10, y >> 10)] } else { crate::course::t::OUT };
        let (to, tr) = (ty(gg.ox, gg.oy), ty(gg.bx, gg.by));
        if c.h(tr) < 1 {
            let r = gg.rating;
            let mut base = 20;
            if r < 5 {
                base = 25;
            }
            if r < 10 {
                base += 5;
            }
            if r > 10 {
                base -= 5;
            }
            let mut dk = d;
            if d == 0 {
                base /= 2;
            } else if d > 25 && r > 0 {
                dk = 25;
                base = 200 / r;
            }
            if c.h(to) > 0 {
                base -= clamp(c.h(to), 0, 3) * base / 6;
            }
            base = (rng.below(s) + 5 + 4 * self.difficulty) * base / 32;
            if c.landmark_bits(gg.x, gg.y) & 4 == 4 {
                base /= 2;
            }
            if (dk + 1) * base < l {
                self.skill_change(c, rng, g, d, l, false);
            }
        } else if rng.below(((c.h(to) + 4) * 250) / (self.difficulty + 2)) < c.h(tr) * s {
            self.skill_change(c, rng, g, d, l, true);
        }
    }

    /// A skill of the pro grows (a great shot) or shrinks (a bad finish) (0x407e00), chosen by what the shot showed.
    fn skill_change(&mut self, c: &Course, rng: &mut ExeRng, g: usize, d: i32, l: i32, bad: bool) {
        let rank = crate::economy::rank((self.next_hole - 1).max(0) as usize);
        let cap = if rank == 3 { 99 } else { rank * 2 + 6 };
        let mut mask: u16 = 0xffff;
        for k in 0..12 {
            if self.pro_skill[k] as i32 >= cap {
                mask &= !(1 << k);
            }
        }
        if bad {
            mask = 0;
            for k in 0..12 {
                if self.pro_skill[k] > 3 {
                    mask |= 1 << k;
                }
            }
        }
        let gg = self.g[g].clone();
        let lie_type = |x: i32, y: i32| if inside(x >> 10, y >> 10) { c.ty[idx(x >> 10, y >> 10)] } else { crate::course::t::OUT };
        let origin = lie_type(gg.ox, gg.oy);
        let mut pick: u16 = 0;
        if origin == crate::course::t::GREEN && mask & 0x10 != 0 {
            pick = 0x10;
        } else {
            for t in 1..=100 {
                pick = match rng.below(12) {
                    0 if c.h(origin) > 0 => 0x100,
                    1 if gg.flags & flag::HIT_TREE != 0 || d == 0 => 0x200,
                    2 if gg.strokes == 0 && gg.club < 2 => 0x2,
                    3 if gg.strokes == 0 && gg.club < 4 => 0x4,
                    4 if gg.shot_type == 1 => 0x20,
                    5 if gg.shot_type == -1 => 0x40,
                    6 if gg.shot_type == 3 => 0x80,
                    7 if crate::planner::max_range(self, c, g) < l => 0x1,
                    8 if (3..=10).contains(&gg.club) => 0x8,
                    9 if t > 50 || d == 0 => {
                        if lie_type(gg.bx, gg.by) == crate::course::t::GREEN && d != 0 {
                            0x10
                        } else {
                            0x200
                        }
                    }
                    _ => 0,
                };
                if pick != 0 && pick & mask != 0 {
                    break;
                }
                pick = 0;
            }
        }
        if pick == 0 {
            return;
        }
        let k = pick.trailing_zeros() as usize;
        let name = self.name(g);
        if !bad {
            if self.pro_skill[k] >= 10 {
                return;
            }
            if self.pro_skill[k] == 0 {
                self.pro_skill[k] = clamp(3 - self.difficulty, 1, 3) as u8;
                self.pro_mask |= pick;
                self.g[g].skill_mask |= pick;
                self.message_by(format!("{name} acquires a new skill: {}!", SKILL_NAMES[k.min(9)]), g as i32, 1);
            } else {
                self.pro_skill[k] += 1;
                let mut t = format!("{name}'s {} skill improves to {}0%.", SKILL_NAMES[k.min(9)], self.pro_skill[k]);
                if self.pro_skill[k] as i32 == cap {
                    let next = crate::economy::RANK_NAMES[(rank as usize + 1).min(3)];
                    t += &format!(" To further improve this skill you will need a {next} course.");
                }
                self.message_by(t, g as i32, 1);
            }
            self.g[g].skills[k] = self.pro_skill[k];
            let at = (self.g[g].bx, self.g[g].by);
            self.award_at(2, at);
            self.sound(0x2f, None);
        } else {
            if self.pro_skill[k] <= 2 {
                return;
            }
            self.pro_skill[k] -= 1;
            self.g[g].skills[k] = self.pro_skill[k];
            let his = if self.male(g) != 0 { "his" } else { "her" };
            self.message_by(format!("{name} loses 10% of {his} {} skill.", SKILL_NAMES[k.min(9)]), g as i32, 1);
            if self.g[g].momentum >= 0 {
                self.g[g].momentum = -1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests() {
        let mut cl = Club::default();
        assert!(!cl.request_pro_round(false), "needs a hole");
        cl.next_hole = 3;
        assert!(cl.request_pro_round(false));
        assert!(cl.game & START_ROUND != 0);
        cl.game = 0;
        assert!(!cl.request_pro_round(true), "no challenge pending");
        assert_eq!(attitude(2), "pumped");
    }

    #[test]
    fn match_money() {
        let mut cl = Club::default();
        cl.holes[1].par = 4;
        cl.holes[2].par = 3;
        cl.next_hole = 3;
        cl.wager_level = 2;
        cl.gary = 3;
        // challenger in slot 2, the pro in slot 3, both with a stake
        cl.g[2].class = 0x27;
        cl.g[3].class = 0x47;
        cl.g[2].partner = 3;
        cl.g[3].partner = 2;
        cl.g[2].card[1] = 5;
        cl.g[3].card[1] = 4;
        // the pro finished hole 1 first and is on hole 2; the challenger is the later finisher
        cl.g[2].hole = 1;
        cl.g[3].hole = 2;
        cl.match_money(2);
        let earned: i32 = cl.out.iter().map(|e| if let crate::golfer::Event::Earn { units, .. } = e { *units } else { 0 }).sum();
        assert_eq!(earned, 40);
        // last hole: a tie on the hole, the pro wins the match by a stroke
        cl.out.clear();
        cl.g[2].card[2] = 3;
        cl.g[3].card[2] = 3;
        cl.g[2].hole = 2;
        cl.g[3].hole = 3;
        cl.match_money(2);
        let earned: i32 = cl.out.iter().map(|e| if let crate::golfer::Event::Earn { units, .. } = e { *units } else { 0 }).sum();
        assert_eq!(earned, 40);
        assert_eq!(cl.trophies, 1);
        assert_eq!(cl.gary, -1);
        assert!(cl.awards & (1 << 5) != 0);
    }

    #[test]
    fn aim_line_bends() {
        let p = AimPreview { x: 0, y: -20 * 1024, distance: 200, club: 0, max_range: 200 };
        let straight = aim_line((0, 0), &p, 0);
        assert!(straight.iter().all(|&(x, _)| x.abs() <= 2));
        let draw = aim_line((0, 0), &p, 1);
        assert!(draw[1].0 > 0, "a draw starts out to the right");
        assert_eq!(draw.last().map(|q| q.0.abs() <= 2), Some(true));
    }
}
