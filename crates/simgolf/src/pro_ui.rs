//! The player's pro in the app: the pro panel's buttons, the skill dialog, aiming with the pointer, and the round's start.
//! The rules live in sg_core::pro; the layout of the dialog and the panel text placement here are our own.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{rgb, rgba, Screen as Ui};
use sg_core::golfer::SLOTS;
use sg_core::pro::{self, SKILL_NAMES};
use sg_core::staff;
use sg_core::terrain::TILE_SIZE;

/// The skill dialog (0x45f0f0): the values on opening are the floor a click can't go below.
#[derive(Clone, Debug, Default)]
pub struct SkillDialog {
    pub floor: [u8; 16],
    pub points: i32,
    pub title: String,
    /// The round to ask for when the dialog closes (practice false, match true).
    pub then: Option<bool>,
    pub confirm: bool,
}

const DLG: Rect = Rect::new(190.0, 50.0, 420.0, 400.0);

fn plus_rect(k: usize) -> Rect {
    Rect::new(DLG.x + 260.0, DLG.y + 70.0 + 26.0 * k as f32, 30.0, 22.0)
}
fn minus_rect(k: usize) -> Rect {
    Rect::new(DLG.x + 296.0, DLG.y + 70.0 + 26.0 * k as f32, 30.0, 22.0)
}
const DONE: Rect = Rect::new(DLG.x + 160.0, DLG.y + 350.0, 100.0, 30.0);

impl App {
    /// The pro's name (roster person 0).
    pub fn pro_name(&self) -> String {
        self.club.roster.first().map(|p| p.name.clone()).unwrap_or_else(|| "Gary Golf".to_string())
    }

    /// The pro panel's buttons: 0 practice round, 1 match, 2 skills, 3 cancel the round.
    pub fn pro_button(&mut self, b: usize) {
        match b {
            0 | 1 => {
                let m = b == 1;
                if self.club.gary != -1 {
                    self.show_toast(&format!("{} is already playing", self.pro_name()));
                } else if self.club.next_hole < 2 {
                    self.show_toast("Your pro needs at least one hole to play");
                } else if m && self.club.game & pro::CHALLENGE == 0 {
                    self.show_toast("No famous golfer has challenged you yet");
                } else if self.club.pro_skill.iter().all(|&v| v == 0) {
                    // the first allocation comes before the first round (0x4065c0)
                    let pts = self.club.first_skill_points();
                    self.open_skills(pts, Some(m));
                } else if !self.club.request_pro_round(m) {
                    self.show_toast("The round can't start now");
                }
            }
            2 => {
                let pts = self.club.skill_points;
                self.open_skills(pts, None);
            }
            _ => {
                if !self.club.cancel_pro_round(&mut self.exe_rng) {
                    self.show_toast("There is no round to cancel");
                }
            }
        }
    }

    pub fn open_skills(&mut self, points: i32, then: Option<bool>) {
        if !self.ui_ok {
            // without the interface (scripted runs) the points go in order of the list
            let mut p = points;
            let mut k = 0;
            while p > 0 && self.club.pro_skill[..10].iter().any(|&v| v < 10) {
                if self.club.pro_skill[k % 10] < 10 {
                    self.club.pro_skill[k % 10] += 1;
                    p -= 1;
                }
                k += 1;
            }
            self.club.skill_points = 0;
            self.finish_skills(then);
            return;
        }
        let title = if then.is_some() {
            "Before you play your course you must choose your pro's skills.".to_string()
        } else if points > 0 {
            "Add skill points to your pro's skills.".to_string()
        } else {
            format!("{}'s skills", self.pro_name())
        };
        self.skill_dialog = Some(SkillDialog { floor: self.club.pro_skill, points, title, then, confirm: false });
        self.screen = Screen::Skills;
    }

    fn finish_skills(&mut self, then: Option<bool>) {
        let mut mask = 0u16;
        for k in 0..12 {
            if self.club.pro_skill[k] != 0 {
                mask |= 1 << k;
            }
        }
        self.club.pro_mask = mask;
        if let Some(m) = then {
            if !self.club.request_pro_round(m) {
                self.show_toast("The round can't start now");
            }
        }
    }

    /// A click in the skill dialog (virtual 800x600 coordinates).
    pub fn skills_click(&mut self, vx: f32, vy: f32) {
        let Some(mut d) = self.skill_dialog.take() else {
            self.screen = Screen::Play;
            return;
        };
        let interactive = d.points > 0 || d.then.is_some();
        let mut handled = false;
        if interactive {
            for k in 0..10 {
                let v = &mut self.club.pro_skill[k];
                if plus_rect(k).has(vx, vy) {
                    handled = true;
                    if d.points > 0 && *v < 10 {
                        *v += 1;
                        d.points -= 1;
                        d.confirm = false;
                    } else {
                        self.slot_sound(0x18, self.cam_x, self.cam_z);
                    }
                } else if minus_rect(k).has(vx, vy) {
                    handled = true;
                    if *v > d.floor[k] {
                        *v -= 1;
                        d.points += 1;
                    } else {
                        self.slot_sound(0x18, self.cam_x, self.cam_z);
                    }
                }
            }
        }
        if DONE.has(vx, vy) || !interactive {
            if d.points > 0 && !d.confirm {
                d.confirm = true;
                self.show_toast("You haven't used all your skill points. Click Done again to keep them for later.");
                self.skill_dialog = Some(d);
                return;
            }
            self.club.skill_points = d.points.max(0);
            self.screen = Screen::Play;
            self.finish_skills(d.then);
            return;
        }
        if !handled {
            self.slot_sound(0x18, self.cam_x, self.cam_z);
        }
        self.skill_dialog = Some(d);
    }

    pub fn draw_skills(&mut self, g: &mut Gfx) {
        let Some(d) = self.skill_dialog.clone() else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.fill(g, DLG.x, DLG.y, DLG.w, DLG.h, rgba(0.1, 0.12, 0.3, 0.94));
        s.text(g, DLG.x + 14.0, DLG.y + 24.0, &d.title, 13.0, rgb(1.0, 1.0, 0.8));
        let interactive = d.points > 0 || d.then.is_some();
        if interactive {
            let col = if d.points > 0 { rgb(1.0, 0.45, 0.4) } else { rgb(0.8, 1.0, 0.8) };
            s.text(g, DLG.x + 14.0, DLG.y + 50.0, &format!("{} skill points", d.points), 15.0, col);
        }
        for k in 0..10 {
            let v = self.club.pro_skill[k];
            let y = DLG.y + 86.0 + 26.0 * k as f32;
            let c = if v == 0 { rgb(0.55, 0.55, 0.65) } else { rgb(1.0, 1.0, 1.0) };
            s.text(g, DLG.x + 20.0, y, SKILL_NAMES[k], 15.0, c);
            s.text(g, DLG.x + 190.0, y, &format!("{}%", v as i32 * 10), 15.0, c);
            if interactive {
                // plus and minus signs drawn as bars (the interface font has no plus)
                let w = rgb(1.0, 1.0, 1.0);
                for (r, plus) in [(plus_rect(k), true), (minus_rect(k), false)] {
                    s.fill(g, r.x, r.y, r.w, r.h, rgba(0.4, 0.4, 0.8, 0.7));
                    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
                    s.fill(g, cx - 7.0, cy - 1.5, 14.0, 3.0, w);
                    if plus {
                        s.fill(g, cx - 1.5, cy - 7.0, 3.0, 14.0, w);
                    }
                }
            }
        }
        s.fill(g, DONE.x, DONE.y, DONE.w, DONE.h, rgba(0.3, 0.6, 0.3, 0.85));
        s.text_centered(g, DONE.x + DONE.w / 2.0, DONE.y + 21.0, "Done", 16.0, rgb(1.0, 1.0, 1.0));
        g.flush();
    }

    /// Before the club's tick: the requested round starts at the pro's employee place (main frame 0x40fd08).
    pub fn pro_round_tick(&mut self) {
        if self.club.game & pro::START_ROUND == 0 {
            return;
        }
        let Some(e) = self.employees.iter().find(|e| e.active && e.job == staff::job::OWNER).copied() else {
            self.club.game &= !pro::START_ROUND;
            return;
        };
        self.club.start_pro_round(&self.course, &mut self.exe_rng, (e.x, e.y), e.dir as i32);
        if self.club.gary >= 0 {
            println!("[{:6.1}s] {} starts a round (golfer {})", self.sim_time, self.pro_name(), self.club.gary);
            self.panel = 3;
            self.edit = false;
        }
    }

    /// After the club's tick: the pro's employee stands where the playing pro is, ready for when the round ends (4.1); and
    /// skill points won by an accomplishment are handed out.
    pub fn pro_after_tick(&mut self) {
        let gary = self.club.gary;
        if gary == -1 && self.pro_slot >= 0 {
            let gg = &self.club.g[self.pro_slot as usize];
            let total: i32 = gg.card[1..].iter().map(|&v| v as i32).sum();
            let holes = gg.card[1..].iter().filter(|&&v| v != 0).count();
            println!("[{:6.1}s] {}'s round is over: {total} strokes on {holes} holes (hole {}, strokes {})", self.sim_time, self.pro_name(), gg.hole, gg.strokes);
            self.panel = 0;
        }
        self.pro_slot = gary;
        if gary >= 0 && (gary as usize) < SLOTS {
            let gg = &self.club.g[gary as usize];
            let (x, y, f) = (gg.x, gg.y, gg.facing);
            if let Some(e) = self.employees.iter_mut().find(|e| e.job == staff::job::OWNER) {
                e.x = x;
                e.y = y;
                e.dir = f as i8;
            }
        }
        if self.auto_aim == 1 && self.club.gary == -1 && self.club.game & pro::CHALLENGE != 0 && self.club.request_pro_round(true) {
            println!("[{:6.1}s] the match is accepted", self.sim_time);
        }
        if self.auto_aim != 0 {
            if let Some(g) = self.club.pro_aiming() {
                let pin = self.club.holes[self.club.g[g].hole.clamp(0, 19) as usize].pin;
                self.aim = self.club.aim_frame(&self.course, pin, false);
                if self.auto_aim == 1 && self.club.commit_aim() {
                    let p = self.aim.unwrap_or_default();
                    println!(
                        "[{:6.1}s] {} aims at the pin of hole {}: {} yards, {}",
                        self.sim_time,
                        self.pro_name(),
                        self.club.g[g].hole,
                        p.distance,
                        pro::CLUB_NAMES[p.club.clamp(0, 11) as usize]
                    );
                }
            }
        }
        if self.club.skill_points > 0 && self.club.pro_mask != 0 && self.ui_ok && self.screen == Screen::Play {
            let pts = self.club.skill_points;
            self.show_toast("Add three skill points to your pro's skills.");
            self.open_skills(pts, None);
        }
    }

    /// Aiming: the pointer's ground point picks the tile, or its nearest corner, and the shot preview follows it.
    pub fn aim_pointer(&mut self, wx: f32, wz: f32) {
        if self.club.pro_aiming().is_none() {
            self.aim = None;
            return;
        }
        let k = TILE_SIZE / staff::UNIT as f32;
        let ux = (wx + self.terrain.w as f32 * TILE_SIZE * 0.5) / k;
        let uy = (wz + self.terrain.h as f32 * TILE_SIZE * 0.5) / k;
        let (a, b) = ((ux as i32) >> 10, (uy as i32) >> 10);
        let mut best = ((a * 1024 + 512) as f32 - ux).hypot((b * 1024 + 512) as f32 - uy);
        let mut tile = (a, b);
        let mut corner = false;
        for i in 0..2 {
            for j in 0..2 {
                let d = (((a + i) * 1024) as f32 - ux).hypot((((b + j) * 1024) as f32) - uy);
                if d < best {
                    best = d;
                    tile = (a + i, b + j);
                    corner = true;
                }
            }
        }
        self.aim = self.club.aim_frame(&self.course, tile, corner);
        self.panel = 3;
    }

    /// A click on the course while the pro waits: the aim is taken.
    pub fn aim_click(&mut self) -> bool {
        if self.club.pro_aiming().is_none() {
            return false;
        }
        if self.club.commit_aim() {
            self.aim = None;
        }
        true
    }

    /// The panel text while aiming (3.2): club, attitude, distance, lie and the skills that count for this shot.
    pub fn aim_text(&self) -> Vec<(String, bool)> {
        let Some(g) = self.club.pro_aiming() else { return Vec::new() };
        let gg = &self.club.g[g];
        let (bx, by) = (gg.bx, gg.by);
        let lie = if sg_core::course::inside(bx >> 10, by >> 10) {
            self.course.ty[sg_core::course::idx(bx >> 10, by >> 10)]
        } else {
            sg_core::course::t::OUT
        };
        let hazard = self.course.h(lie) > 0;
        let p = self.aim.unwrap_or_default();
        let opt = self.club.planner.option;
        let mut v = vec![
            (format!("Club: {}", pro::CLUB_NAMES[p.club.clamp(0, 11) as usize]), true),
            (format!("Attitude: {}", pro::attitude(gg.momentum)), true),
            (format!("Distance: {} yards", p.distance), true),
            (format!("Lie: {}", pro::lie_name(lie)), true),
        ];
        for k in 0..10 {
            let s = self.club.pro_skill[k];
            if s != 0 {
                v.push((format!("{}  {}%", SKILL_NAMES[k], s as i32 * 10), pro::skill_applies(k, p.club, gg.strokes, opt, hazard)));
            }
        }
        v
    }
}
