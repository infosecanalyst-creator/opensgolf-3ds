//! The player's own golfer (the club's pro, "Gary Golf"): a round started from the pro panel (practice, or a match against a
//! famous golfer who has challenged the club), the player aiming each full shot with a shot type, the match money, and the
//! pro's ten skills, which the player sets with points and which grow or shrink with remarkable shots.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "The player's pro"), restated in our own words.

use crate::course::{idx, inside, Course};
use crate::economy::money_digits;
use crate::geom::{angle, clamp, cosr, dir8, len, sinr, tdist};
use crate::golfer::{flag, game, Club, Column, Event, Golfer, SLOTS};
use crate::land::ExeRng;
use crate::lang::{tr, trf};

/// A famous golfer's challenge is waiting to be played (game flag 0x2000); the pro's round starts next tick (0x4000).
pub const CHALLENGE: u32 = 0x2000;
pub const START_ROUND: u32 = 0x4000;

/// Skill names in the order of the skill bytes.
pub const SKILL_NAME_KEYS: [&str; 10] = [
    "STR_SKILL_POWER_HITTER",
    "STR_SKILL_LONG_DRIVER",
    "STR_SKILL_ACCURATE_DRIVER",
    "STR_SKILL_ACCURATE_IRONS",
    "STR_SKILL_ACCURATE_PUTTER",
    "STR_SKILL_DRAW_SHOT",
    "STR_SKILL_FADE_SHOT",
    "STR_SKILL_HIGH_BACKSPIN",
    "STR_SKILL_RECOVERY",
    "STR_SKILL_LUCK",
];

/// The exe's skill names (pointer table 0x4c2c3c), all twelve, as the great-shot message prints them.
pub const EXE_SKILL_KEYS: [&str; 12] = [
    "STR_SKILL_POWER_HITTER",
    "STR_SKILL_LONG_DRIVER",
    "STR_SKILL_ACCURATE_DRIVER",
    "STR_SKILL_ACCURATE_IRONS",
    "STR_SKILL_ACCURATE_PUTTER",
    "STR_SKILL_DRAW_SHOT_R_TO_L",
    "STR_SKILL_FADE_SHOT_L_TO_R",
    "STR_SKILL_HIGH_BACKSPIN",
    "STR_SKILL_RECOVERY",
    "STR_SKILL_LUCK",
    "STR_SKILL_POSITIVE_ATTITUDE",
    "STR_SKILL_MENTAL_TOUGHNESS",
];

/// The skill names as the stats cards print them (footage of the original: the skill points card and a pro's card read
/// "Draw Shot (R to L)" and "Fade Shot (L to R)", as Pick A Pro does).
pub const SKILL_LABEL_KEYS: [&str; 10] = [
    "STR_SKILL_POWER_HITTER",
    "STR_SKILL_LONG_DRIVER",
    "STR_SKILL_ACCURATE_DRIVER",
    "STR_SKILL_ACCURATE_IRONS",
    "STR_SKILL_ACCURATE_PUTTER",
    "STR_SKILL_DRAW_SHOT_R_TO_L",
    "STR_SKILL_FADE_SHOT_L_TO_R",
    "STR_SKILL_HIGH_BACKSPIN",
    "STR_SKILL_RECOVERY",
    "STR_SKILL_LUCK",
];

/// Skill k's name (`SKILL_NAME_KEYS`).
pub fn skill_name(k: usize) -> &'static str {
    tr(SKILL_NAME_KEYS[k])
}

/// Skill k's name as the exe prints it (`EXE_SKILL_KEYS`).
pub fn exe_skill(k: usize) -> &'static str {
    tr(EXE_SKILL_KEYS[k])
}

/// Skill k's name as the stats cards print it (`SKILL_LABEL_KEYS`).
pub fn skill_label(k: usize) -> &'static str {
    tr(SKILL_LABEL_KEYS[k])
}

/// Club k's name for the panel: the panel (0x41b62f) prints "Club: " and the golfer's club byte (+0x1c, 0..11 as the aim
/// picks it) through the exe's club name routine 0x40a9a0, the same table the thoughts and shot words use (Driver, 3 Wood,
/// 4 Wood, 2 to 9 Iron, Lob Wedge, Sand Wedge, Putter).
pub fn club_label(k: usize) -> &'static str {
    crate::thoughts::club_name(k.min(13))
}

/// The lie under the ball, for the panel: 0x41b759 prints "Lie: " and the name (+0x00) of the terrain table's record for
/// the type under the ball (0x40bfa0, out of bounds off the map), the singular lower-case names of the static table
/// 0x4c1a40 ("tees", "sand trap", "pot bunker", "out of bounds"); the exe's per-theme renaming of some records is not decoded.
pub fn lie_name(ty: u8) -> &'static str {
    crate::course::type_name(ty as i32)
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

/// How far up the screen each point of `aim_line` is drawn, in 800 x 600 pixels (0x41bbe1..0x41c0bd). With A = zoom x k x
/// distance (k = 3 for the high shot, 0 for option 4, else 2), the ball and the last two points stay on the ground, points 1
/// and 4 go up A / 3 / 8 and points 2 and 3 A / 2 / 8, each division toward zero; a black copy of the line runs one pixel
/// under the ground points (the shadow) and the white line joins the lifted points. `zoom` is the exe's zoom step (1..4).
pub fn aim_lifts(distance: i32, opt: i32, zoom: f32) -> Vec<f32> {
    let k = match opt {
        3 => 3.0,
        4 => 0.0,
        _ => 2.0,
    };
    let a = zoom * k * distance as f32;
    let d1 = ((a / 3.0).trunc() / 8.0).trunc();
    let d2 = ((a / 2.0).trunc() / 8.0).trunc();
    let mut v = vec![0.0, d1, d2, d2, d1, 0.0];
    if opt != 3 {
        v.push(0.0);
    }
    v
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
        -4 => tr("STR_ATTITUDE_FURIOUS"),
        -3 => tr("STR_ATTITUDE_MAD"),
        -2 => tr("STR_ATTITUDE_UPSET"),
        -1 => tr("STR_ATTITUDE_WORRIED"),
        0 => tr("STR_ATTITUDE_CALM"),
        1 => tr("STR_ATTITUDE_DETERMINED"),
        2 => tr("STR_ATTITUDE_PUMPED"),
        _ => tr("STR_ATTITUDE_INVINCIBLE"),
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
                r.job = tr("STR_JOB_GOLF_PRO").to_string();
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
                self.message(tr("STR_MSG_MATCH_CANCELED").to_string());
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
        let mut text =
            trf("STR_CHALLENGE_WAGER", &[("PRO", &pro.name), ("HOLE_MONEY", &((5 * l + 5) * 400)), ("MATCH_MONEY", &((5 * l + 5) * 800))]);
        if best > 0 {
            let names: Vec<&str> = (0..10).filter(|&k| pro.skills[k] == best).map(skill_name).collect();
            text += &trf(
                "STR_CHALLENGE_PROUD",
                &[
                    ("HE", &(if female { tr("STR_CHALLENGE_SHE") } else { tr("STR_CHALLENGE_HE") })),
                    ("HIS", &(if female { tr("STR_CHALLENGE_HER") } else { tr("STR_CHALLENGE_HIS") })),
                    ("PERCENT", &best),
                    ("SKILLS", &names.join(tr("STR_CHALLENGE_AND"))),
                ],
            );
        }
        let pro_name = self.roster.first().map(|p| p.name.clone()).unwrap_or_else(|| tr("STR_LOG_GARY_GOLF").to_string());
        text += &trf("STR_CHALLENGE_CLICK", &[("PRO", &pro_name)]);
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
            let text = trf(
                "STR_MATCH_WIN_HOLE",
                &[("GOLFER", &self.name(o)), ("HOLE", &h), ("SCORE", &so), ("OTHER_SCORE", &se), ("MONEY", &money_digits(a as i64 * 100))],
            );
            self.message_by(text, self.gary, 0);
            self.earn(a, Column::Other, at);
            self.sound(0x23, None);
        }
        if se < so {
            let text = trf(
                "STR_MATCH_LOSE_HOLE",
                &[("GOLFER", &self.name(e)), ("HOLE", &h), ("SCORE", &se), ("OTHER_SCORE", &so), ("MONEY", &money_digits(a as i64 * 100))],
            );
            self.message_by(text, self.gary, 0);
            self.earn(-a, Column::Other, at);
            self.sound(0x24, None);
        }
        if self.par(h + 1) == 0 {
            let total = |c: &Golfer| (1..=hu).map(|k| c.card[k] as i32).sum::<i32>();
            let (to, te) = (total(&self.g[o]), total(&self.g[e]));
            if te < to {
                // 0x4c72dc with 0x4c72d0, 0x4c72b4 or 0x4c72a8 (the exe goes on with the standings, not in the port)
                self.message(trf("STR_MATCH_END_LOSE", &[("MONEY", &money_digits(a as i64 * 100))]));
                self.earn(-a, Column::Other, at);
                self.sound(0x24, None);
                self.wager_level -= 1;
            } else if te == to {
                self.message(tr("STR_MATCH_END_TIED").to_string());
                self.wager_level -= 1;
            } else {
                self.message(trf("STR_MATCH_END_WIN", &[("MONEY", &money_digits(a as i64 * 100))]));
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
                    0 => tr("STR_MATCH_EVEN_PAR").to_string(),
                    d if d < 0 => trf("STR_MATCH_UNDER_PAR", &[("STROKES", &(-d))]),
                    d => trf("STR_MATCH_OVER_PAR", &[("STROKES", &d)]),
                };
                self.message_by(trf("STR_MATCH_TIED_AT", &[("GOLFER", &no), ("OTHER", &ne), ("SCORE", &at)]), self.gary, 1);
            } else {
                let n = (to - te).abs();
                let key = if to < te {
                    crate::lang::plural(n as i64, "STR_MATCH_LEADS_BY_ONE", "STR_MATCH_LEADS_BY")
                } else {
                    crate::lang::plural(n as i64, "STR_MATCH_TRAILS_BY_ONE", "STR_MATCH_TRAILS_BY")
                };
                self.message_by(trf(key, &[("GOLFER", &no), ("OTHER", &ne), ("SHOTS", &n)]), self.gary, 1);
                self.sound(if to < te { 0x2f } else { 0x30 }, None);
            }
            self.gary = -1;
        }
    }

    /// When a shot comes to rest or drops (0x409620): at the exe's closest zoom, for a shot struck on screen (it has a replay
    /// record), an ordinary golfer's or the pro's shot far beyond what its difficulty predicts is a great shot (famous pros
    /// other than the player's are left out); for the pro the bar moves with a random share of his skills, and a shot of
    /// his that ends in a hazard may be a bad one.
    pub(crate) fn shot_review(&mut self, c: &Course, rng: &mut ExeRng, g: usize) {
        if !self.view_close || self.g[g].hole < 1 || self.g[g].hole > 18 {
            return;
        }
        let Some(rec) = self.shot_recs.iter().find(|r| r.g == g).map(|r| r.pts.clone()) else { return };
        let pro = g as i32 == self.gary;
        if self.g[g].kind & 0xe0 == 0x20 && !pro {
            return;
        }
        // (the exe also passes over the golfer its new member message follows, 0x4c2e0c, which the port does not keep)
        let gg = &self.g[g];
        let l = len(gg.ox - gg.bx, gg.oy - gg.by) * 25 / 1024;
        let pin = self.holes[gg.hole as usize].pin;
        let d = tdist(gg.bx, gg.by, pin.0, pin.1);
        let s: i32 = if pro { gg.skills[..10].iter().map(|&v| v as i32).sum() } else { 0 };
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
            if pro {
                if c.h(to) > 0 {
                    base -= clamp(c.h(to), 0, 3) * base / 6;
                }
                base = (rng.below(s) + 5 + 4 * self.difficulty) * base / 32;
                if c.landmark_bits(gg.x, gg.y) & 4 == 4 {
                    base /= 2;
                }
            }
            if (dk + 1) * base < l {
                self.great_shot(c, rng, g, d, l, false, rec);
            }
        } else if pro && rng.below(((c.h(to) + 4) * 250) / (self.difficulty + 2)) < c.h(tr) * s {
            self.great_shot(c, rng, g, d, l, true, rec);
        }
    }

    /// A remarkable shot (0x407e00): the message, and for the player's pro the skill it grows (a great shot) or costs (a bad
    /// finish), chosen by what the shot showed. The message is "<name> launches / knocks / lofts / snakes a [tricky / tough]
    /// <club> shot / drive / putt <n> yards <where> at <hole>!", where is "safely onto the fairway", "to within N feet / yards
    /// of the pin", "into the hole" or "inches from the hole"; a bad one "<name> yanks / flubs / scuffs / fluffs a <club> into
    /// the <ground> at <hole>!". Then the partner's reaction (a good shot) and the skill line. The game shows it with the
    /// great-shot SimFoto, or on the ticker when the shot is off screen (`Event::GreatShot`); a bad shot whose skill cannot
    /// fall says nothing at all.
    #[allow(clippy::too_many_arguments)]
    fn great_shot(&mut self, c: &Course, rng: &mut ExeRng, g: usize, d: i32, l: i32, bad: bool, pts: Vec<(i32, i32, i32)>) {
        let pro = g as i32 == self.gary;
        let gg = self.g[g].clone();
        let p = gg.partner.clamp(0, SLOTS as i32 - 1) as usize;
        let lie_type = |x: i32, y: i32| if inside(x >> 10, y >> 10) { c.ty[idx(x >> 10, y >> 10)] } else { crate::course::t::OUT };
        let rest = lie_type(gg.bx, gg.by);
        let club = gg.club.clamp(0, 13);
        let name = self.vip_name(g);
        // the exe's pieces: 0x4c508c / 0x4c5080 / 0x4c5074 / 0x4c5068, 0x4c5060 / 0x4c5058, 0x4c5040 / 0x4c5050 / the club
        // and 0x4c5048, the yards and 0x4c5038, 0x4c4ff8 / 0x4c502c, 0x4c5024 or 0x4c501c and 0x4c5010 / 0x4c4fe8 /
        // 0x4c4fd0; a bad shot 0x4c50a8 / 0x4c50b4 / 0x4c50c0 / 0x4c50cc, the club, 0x4c509c and the ground's name
        let mut t = name.clone();
        if !bad {
            t += match club {
                0 => tr("STR_SHOT_LAUNCHES"),
                13 => tr("STR_SHOT_SNAKES"),
                1..=6 => tr("STR_SHOT_KNOCKS"),
                _ => tr("STR_SHOT_LOFTS"),
            };
            if gg.rating > 10 {
                t += tr("STR_SHOT_TRICKY");
            } else if gg.rating == 10 {
                t += tr("STR_SHOT_TOUGH");
            }
            match club {
                0 => t += tr("STR_SHOT_DRIVE"),
                13 => t += tr("STR_SHOT_PUTT"),
                k => t += &trf("STR_SHOT_CLUB", &[("CLUB", &crate::thoughts::club_name(k as usize))]),
            }
            t += &trf("STR_SHOT_YARDS", &[("YARDS", &l)]);
            let pin = self.holes[gg.hole.clamp(0, 18) as usize].pin;
            if d == 0 {
                let cup = gg.bx == pin.0 * 0x400 + 0x200 && gg.by == pin.1 * 0x400 + 0x200;
                t += if cup { tr("STR_SHOT_INTO_THE_HOLE") } else { tr("STR_SHOT_INCHES") };
            } else if rest == crate::course::t::GREEN {
                t += &if d < 8 {
                    trf("STR_SHOT_WITHIN_FEET", &[("FEET", &(d * 2))])
                } else {
                    trf("STR_SHOT_WITHIN_YARDS", &[("YARDS", &d)])
                };
            } else {
                t += tr("STR_SHOT_FAIRWAY");
            }
        } else {
            let odd = gg.hole & 1 != 0;
            t += match (club < 5, odd) {
                (true, false) => tr("STR_SHOT_YANKS"),
                (true, true) => tr("STR_SHOT_FLUBS"),
                (false, false) => tr("STR_SHOT_SCUFFS"),
                (false, true) => tr("STR_SHOT_FLUFFS"),
            };
            t += crate::thoughts::club_name(club as usize);
            t += tr("STR_SHOT_INTO_THE");
            t += &crate::tracts::type_name(rest.min(22), c.theme, false);
        }
        // 0x4c4fc8, the hole's name (0x407280), 0x4c4fc4
        let h = gg.hole.clamp(0, 18) as usize;
        t += &trf("STR_SHOT_AT_HOLE", &[("HOLE", &crate::ratings::hole_name(h, &self.holes[h]))]);
        if !bad && self.g[p].hole > 0 {
            // the partner's reaction by the golfer's +0xae: 0x4c4fac, 0x4c4f54, 0x4c4f48 with 0x4c4f38 and 0x4c4f30 /
            // 0x4c4f28, 0x4c4f78 with 0x4c4f94 / 0x4c4f90 and 0x4c4f6c, 0x4c4f98 with his / her and 0x4c4f88
            let pn = self.vip_name(p);
            let male = self.male(p) != 0;
            let his = if male { tr("STR_SHOT_HIS") } else { tr("STR_SHOT_HER") };
            match gg.field_ae {
                0 => t += &trf("STR_SHOT_PARTNER_AMAZED", &[("PARTNER", &pn)]),
                1 => t += &trf("STR_SHOT_PARTNER_DELIGHTED", &[("PARTNER", &pn)]),
                2 => {
                    t += &trf(
                        "STR_SHOT_PARTNER_RATS",
                        &[("PARTNER", &pn), ("HIMSELF", &(if male { tr("STR_SHOT_HIMSELF") } else { tr("STR_SHOT_HERSELF") }))],
                    )
                }
                3 | 5 => t += &trf("STR_SHOT_PARTNER_CONGRATULATES", &[("PARTNER", &pn), ("HIS", &his)]),
                4 => t += &trf("STR_SHOT_PARTNER_EYES", &[("PARTNER", &pn), ("HIS", &his)]),
                _ => {}
            }
        }
        // (the exe also grows the skills of the pro's partner slot outside tournaments from a second table, 0x5a5a14,
        // which the port does not keep)
        if pro {
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
            let origin = lie_type(gg.ox, gg.oy);
            let mut pick: u16 = 0;
            if origin == crate::course::t::GREEN && mask & 0x10 != 0 {
                pick = 0x10;
            } else {
                for n in 1..=100 {
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
                        // the exe counts its tries from 0 and tests the count before adding one: more than 50 tries made
                        9 if n > 51 || d == 0 => {
                            if rest == crate::course::t::GREEN && d != 0 {
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
            if pick == 0 && bad {
                return;
            }
            if pick != 0 && gg.kind != 0 {
                let k = pick.trailing_zeros() as usize;
                let skill = exe_skill(k.min(11));
                if !bad {
                    if self.pro_skill[k] < 10 {
                        if self.pro_skill[k] == 0 {
                            // 0x4c3f70, the name, 0x4c4e64, the skill and 0x4c4e60
                            self.pro_skill[k] = clamp(3 - self.difficulty, 1, 3) as u8;
                            self.pro_mask |= pick;
                            self.g[g].skill_mask |= pick;
                            t += &trf("STR_SHOT_NEW_SKILL", &[("GOLFER", &name), ("SKILL", &skill)]);
                        } else {
                            // 0x4c3f70, the name, 0x4c4ef0, the skill, 0x4c4ed8, the level, 0x4c4ed0; at the cap
                            // 0x4c4e88, the next class (0x4c2a1c by the class) and 0x4c4e7c
                            self.pro_skill[k] += 1;
                            t += &trf("STR_SHOT_SKILL_IMPROVES", &[("GOLFER", &name), ("SKILL", &skill), ("LEVEL", &(self.pro_skill[k]))]);
                            if self.pro_skill[k] as i32 == cap {
                                let next = crate::economy::rank_name((rank as usize + 1).min(3));
                                t += &trf("STR_SHOT_SKILL_CAP", &[("RANK", &next)]);
                            }
                        }
                        self.g[g].skills[k] = self.pro_skill[k];
                        self.award_at(2, (gg.x, gg.y));
                        self.sound(0x2f, None);
                    }
                } else {
                    if self.pro_skill[k] < 3 {
                        return;
                    }
                    // 0x4c3f70, the name, 0x4c4f14 / 0x4c4f00, the skill, 0x4c4ef8
                    self.pro_skill[k] -= 1;
                    self.g[g].skills[k] = self.pro_skill[k];
                    let his = if self.male(g) != 0 { tr("STR_SHOT_HIS") } else { tr("STR_SHOT_HER") };
                    t += &trf("STR_SHOT_SKILL_LOSS", &[("GOLFER", &name), ("HIS", &his), ("SKILL", &skill)]);
                }
            }
        }
        self.out.push(Event::GreatShot { g, partner: p, text: t, bad, pts });
    }

    /// After the great-shot SimFoto is let go (the end of 0x407e00): the golfer's member record notes the photo on this hole
    /// and the golfer takes flag 0x20000; a good shot's golfer, unless his newest thought is already the tough-shot one, has
    /// it now (0x467a00 type 1), cheers (animation 12, paused 99) and turns two steps back; a bad one's momentum turns
    /// negative.
    pub fn great_shot_after(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, bad: bool) {
        self.great_shot_noted(g);
        self.g[g].flags |= flag::BAD_LIE;
        if !bad {
            if self.g[g].thoughts[0] & 0x7f != 1 {
                self.event(c, rng, g, 1, 1);
                let gg = &mut self.g[g];
                gg.anim = crate::golfer::anim::HAPPY;
                gg.pause = -99;
                gg.facing = (gg.facing - 2) & 7;
            }
        } else if self.g[g].momentum >= 0 {
            self.g[g].momentum = -1;
        }
    }

    /// The golfer's member record notes a great-shot SimFoto on the current hole (bit 1 of +3 + hole), as the exe also does
    /// for an ordinary golfer's great shot made off screen.
    pub fn great_shot_noted(&mut self, g: usize) {
        let h = self.g[g].hole.clamp(0, 18) as usize;
        let r = self.g[g].roster.clamp(0, self.members.len() as i32 - 1) as usize;
        if let Some(m) = self.members.get_mut(r) {
            m.holes[h] |= 1;
        }
    }

    /// Test hook for a still of the great-shot SimFoto: golfer `g`'s ball is put most of the way from where he stands to
    /// his hole's pin, and that shot (a 4 Wood rated tough, its arc in nine points) is reviewed as a great one.
    pub fn great_shot_test(&mut self, c: &Course, rng: &mut ExeRng, g: usize) {
        let gg = &self.g[g];
        let h = gg.hole.clamp(1, 18) as usize;
        let pin = self.holes[h].pin;
        let (ox, oy) = (gg.x, gg.y);
        let (px, py) = if pin.0 > 0 { (pin.0 * 0x400 + 0x200, pin.1 * 0x400 + 0x200) } else { (ox + 0x2000, oy - 0x2000) };
        let (bx, by) = (ox + (px - ox) * 7 / 10, oy + (py - oy) * 7 / 10);
        let pts = (0..9).map(|k| (ox + (bx - ox) * k / 8, oy + (by - oy) * k / 8, 4 * 450 * k * (8 - k) / 64)).collect();
        let gg = &mut self.g[g];
        (gg.ox, gg.oy, gg.bx, gg.by, gg.club, gg.rating) = (ox, oy, bx, by, 2, 10);
        let l = len(ox - bx, oy - by) * 25 / 1024;
        let d = tdist(bx, by, pin.0, pin.1);
        self.great_shot(c, rng, g, d, l, false, pts);
    }

    /// No member has had a great-shot SimFoto on hole `h` yet (bit 1 of +3 + h in the 76 member records).
    pub fn first_great_shot(&self, h: i32) -> bool {
        let h = h.clamp(0, 18) as usize;
        self.members.iter().all(|m| m.holes[h] & 1 == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fairway with a green round (35, 25), hole 2's pin; the pro in slot 3 on hole 2 with his partner in slot 2.
    fn great_shot_club() -> (Course, Club) {
        use crate::course::t;
        let mut c = Course::default();
        c.ty.iter_mut().for_each(|v| *v = t::FAIRWAY);
        for a in 33..=37 {
            for b in 23..=27 {
                c.ty[idx(a, b)] = t::GREEN;
            }
        }
        c.refresh();
        let mut cl = Club::default();
        cl.holes[2].pin = (35, 25);
        cl.holes[2].par = 4;
        cl.next_hole = 3;
        cl.gary = 3;
        cl.g[3].kind = 0x24;
        (cl.g[3].partner, cl.g[2].partner) = (2, 3);
        (cl.g[3].hole, cl.g[2].hole) = (2, 2);
        (c, cl)
    }

    fn said(cl: &Club) -> Option<String> {
        cl.out.iter().find_map(|e| if let Event::GreatShot { text, .. } = e { Some(text.clone()) } else { None })
    }

    #[test]
    fn great_shot_message() {
        let (c, mut cl) = great_shot_club();
        let mut rng = ExeRng { state: 1 };
        let (n, p) = (cl.vip_name(3), cl.vip_name(2));
        // a 4 Wood rated 10 that stays on the fairway
        let g = &mut cl.g[3];
        (g.club, g.rating, g.ox, g.oy, g.bx, g.by) = (2, 10, 5 << 10, 25 << 10, 20 << 10, 25 << 10);
        cl.great_shot(&c, &mut rng, 3, 15, 174, false, vec![]);
        let t = said(&cl).unwrap();
        assert!(t.starts_with(&format!(
            "{n} knocks a tough 4 Wood shot 174 yards safely onto the fairway at Hole 2!  {p} looks on in amazement. {n}"
        )));
        // a putt from the green into the cup: the putter skill grows (the pick is certain from the green)
        cl.out.clear();
        cl.pro_skill[4] = 2;
        let g = &mut cl.g[3];
        (g.club, g.rating, g.ox, g.oy, g.bx, g.by) = (13, 0, 34 << 10, 25 << 10, (35 << 10) + 0x200, (25 << 10) + 0x200);
        g.field_ae = 4;
        cl.great_shot(&c, &mut rng, 3, 0, 12, false, vec![]);
        let his = if cl.male(2) != 0 { "his" } else { "her" };
        assert_eq!(
            said(&cl).unwrap(),
            format!(
                "{n} snakes a putt 12 yards into the hole at Hole 2!  {p} can not believe {his} eyes. {n}'s 'Accurate Putter' skill \
                 improves to 30%! "
            )
        );
        assert_eq!(cl.pro_skill[4], 3);
        // a bad finish with no skill above 3 to lose says nothing
        cl.out.clear();
        cl.pro_skill = [2; 16];
        cl.great_shot(&c, &mut rng, 3, 15, 174, true, vec![]);
        assert!(said(&cl).is_none());
        // an ordinary golfer's great shot: no skill line, and his partner on hole 0 has no reaction
        cl.out.clear();
        cl.g[2].hole = 0;
        cl.g[3].field_ae = 0;
        cl.gary = -1;
        cl.great_shot(&c, &mut rng, 3, 0, 12, false, vec![]);
        assert!(said(&cl).unwrap().ends_with("into the hole at Hole 2!  "));
    }

    #[test]
    fn aim_line_lifts() {
        // 100 range units at zoom 4: A = 800, A/3/8 = 33, A/2/8 = 50
        assert_eq!(aim_lifts(100, 0, 4.0), vec![0.0, 33.0, 50.0, 50.0, 33.0, 0.0, 0.0]);
        // the high shot: A = 1200 and one point fewer
        assert_eq!(aim_lifts(100, 3, 4.0), vec![0.0, 50.0, 75.0, 75.0, 50.0, 0.0]);
        assert!(aim_lifts(100, 4, 4.0).iter().all(|&v| v == 0.0));
        let p = AimPreview { x: 0, y: -40960, distance: 100, club: 1, max_range: 200 };
        assert_eq!(aim_lifts(100, 1, 2.0).len(), aim_line((0, 0), &p, 1).len());
        assert_eq!(aim_lifts(100, 3, 2.0).len(), aim_line((0, 0), &p, 3).len());
    }

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
        let said = |cl: &Club, t: &str| cl.out.iter().any(|e| matches!(e, crate::golfer::Event::Message { text, .. } if text.ends_with(t)));
        assert!(said(&cl, " wins hole #1 by a score of 4 to 5! Collect \u{a7}4,000."));
        // last hole: a tie on the hole, the pro wins the match by a stroke
        cl.out.clear();
        cl.g[2].card[2] = 3;
        cl.g[3].card[2] = 3;
        cl.g[2].hole = 2;
        cl.g[3].hole = 3;
        cl.match_money(2);
        let earned: i32 = cl.out.iter().map(|e| if let crate::golfer::Event::Earn { units, .. } = e { *units } else { 0 }).sum();
        assert_eq!(earned, 40);
        assert!(said(&cl, "At the end of the match, you win \u{a7}4,000."));
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
