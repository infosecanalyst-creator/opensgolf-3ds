//! The player's pro in the app: the pro panel's buttons, the skill dialog, aiming with the pointer, and the round's start.
//! The rules live in sg_core::pro. The skill dialog follows the exe's wide stats card (0x45f0f0 with x offset 200,
//! docs/DECODE_GOLFERCARD.md 3, DECODE_CARDS2.md 1, DECODE_CARDS3.md 1); the panel text placement here is our own.

use crate::app::*;
use crate::gfx::Gfx;
use crate::popup_ui::ChoiceBox;
use crate::screens_ui::{c15, skill_value, top, BODY, LARGE};
use crate::ui::{rgb, Screen as Ui};
use sg_core::golfer::SLOTS;
use sg_core::pro::{self, SKILL_LABELS, SKILL_NAMES};
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
    /// The confirm box over the dialog (OK with points left).
    pub confirm: bool,
    /// Points were given on opening: the pads and the points line show, and OK closes it; otherwise any click does.
    pub editable: bool,
    /// Opened for an accomplishment's three points: footage of the original shows the card over the trophy room under a
    /// heading "Add three skill points to your player...".
    pub award: bool,
}

/// The x offset the exe passes for the player's own card.
const X0: f32 = 200.0;

/// Row r's top.
fn row_y(r: usize) -> f32 {
    90.0 + 24.0 * r as f32
}

/// The pad under the pointer: (row, true for the top half that adds).
fn pad_hit(vx: f32, vy: f32) -> Option<(usize, bool)> {
    if !(X0 + 50.0..=X0 + 82.0).contains(&vx) {
        return None;
    }
    (0..10).find_map(|r| {
        let y = row_y(r);
        if vy > y && vy <= y + 12.0 {
            Some((r, true))
        } else if vy > y + 12.0 && vy <= y + 24.0 {
            Some((r, false))
        } else {
            None
        }
    })
}

/// The OK ball: within 20 of (x0 + 350, 334) both ways.
fn ok_hit(vx: f32, vy: f32) -> bool {
    (vx - X0 - 350.0).abs() < 20.0 && (vy - 334.0).abs() < 20.0
}

/// The confirm box (the generic popup 0x46d6e0 at (400, 200)), worded as footage of the original shows it: the first
/// answer leaves with the points kept, the second goes back to the card.
fn confirm_box() -> ChoiceBox {
    let lines = [
        "You haven't used all your skill points!",
        "Do you really want to exit?",
        " Yea, I don't need no stinkin' skill points.",
        " Whoops, my bad.",
    ];
    ChoiceBox::new(lines.iter().map(|l| l.to_string()).collect(), 400.0, 200.0)
}

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
            4 => self.begin_tournament(),
            _ if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 => self.cancel_tournament(),
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
        // the card's title is the pro's name
        let title = self.pro_name();
        let editable = points > 0;
        self.skill_dialog = Some(SkillDialog { floor: self.club.pro_skill, points, title, then, confirm: false, editable, award: false });
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
        // a pro already out (a tournament started before the first allocation) plays with the new values
        let gary = self.club.gary;
        if gary >= 0 && (gary as usize) < sg_core::golfer::SLOTS {
            let gg = &mut self.club.g[gary as usize];
            gg.skills[..12].copy_from_slice(&self.club.pro_skill[..12]);
            gg.skill_mask = mask;
        }
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
        if d.confirm {
            // the confirm box: its first answer keeps the points for later, anything else goes back to the dialog
            d.confirm = false;
            if confirm_box().option_at(vx, vy) == Some(0) {
                self.close_skills(d);
            } else {
                self.skill_dialog = Some(d);
            }
            return;
        }
        if !d.editable {
            // read only: any click leaves
            self.close_skills(d);
            return;
        }
        if let Some((k, add)) = pad_hit(vx, vy) {
            let v = &mut self.club.pro_skill[k];
            if add && d.points > 0 && *v < 10 {
                *v += 1;
                d.points -= 1;
            } else if !add && *v > 0 && *v > d.floor[k] {
                *v -= 1;
                d.points += 1;
            } else {
                self.ui_sound(0x18);
            }
        } else if ok_hit(vx, vy) {
            if d.points > 0 {
                d.confirm = true;
            } else {
                self.close_skills(d);
                return;
            }
        }
        self.skill_dialog = Some(d);
    }

    /// Keys in the skill dialog: Enter is OK (or the confirm box's choice), Esc leaves the confirm box (or a read-only card).
    pub fn skills_key(&mut self, enter: bool) {
        let Some(d) = self.skill_dialog.as_mut() else { return };
        let (cx, cy) = if d.confirm {
            if enter {
                let b = confirm_box();
                let r = b.rect();
                (r.x + 50.0, b.line_tops()[2] + 5.0)
            } else {
                (-1.0, -1.0)
            }
        } else if !d.editable || enter {
            (X0 + 350.0, 334.0)
        } else {
            return;
        };
        self.skills_click(cx, cy);
    }

    fn close_skills(&mut self, d: SkillDialog) {
        self.club.skill_points = d.points.max(0);
        self.screen = Screen::Play;
        self.finish_skills(d.then);
    }

    pub fn draw_skills(&mut self, g: &mut Gfx) {
        let Some(d) = self.skill_dialog.clone() else { return };
        if d.award {
            // the trophy room behind, and the heading in a thin light frame: cyan, left at 212 with its capitals' top at 23
            // (footage of the original; the frame's foot is under the card)
            crate::ui::set_face(Some(crate::ui::Face::Info));
            self.draw_board(g);
            crate::ui::set_face(None);
        }
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        if d.award {
            let edge = rgb(0.78, 0.76, 0.74);
            s.fill(g, 200.0, 11.0, 330.0, 1.0, edge);
            s.fill(g, 200.0, 11.0, 1.0, 40.0, edge);
            s.fill(g, 529.0, 11.0, 1.0, 40.0, edge);
            s.text(g, 212.0, 23.0 + 15.0, "Add three skill points to your player...", 20.0, c15(0x03fc));
        }
        let (mx, my) = self.card_ui.mouse;
        self.art.solid_frame(g, &s, X0 + 46.0, 50.0, 320.0, 316.0);
        s.text_centered(g, X0 + 210.0, top(58.0, LARGE), &d.title, LARGE, rgb(1.0, 1.0, 1.0));
        // the pro on the sky and grass window, head over body
        s.image(g, &self.art.head_body, X0 + 247.0, 86.0);
        let me = self.club.roster.first().cloned().unwrap_or_default();
        let look = crate::cust_ui::person_look(&me);
        let mut o = me.outfit(0);
        o.alt_skin = 4; // the pro's hands are drawn pale
        let swaps = std::mem::take(&mut self.swaps);
        self.art.head_over_body(g, &s, &swaps, &me, o, 0, look, 1, X0 + 255.0, 102.0);
        self.swaps = swaps;
        if d.editable {
            // footage of the original: "Add 16 skill points." in white with points left (the decoded red test is not
            // borne out)
            s.text_centered(g, X0 + 210.0, top(80.0, BODY), &format!("Add {} skill points.", d.points), BODY, c15(0x7fff));
        }
        let hover = if d.editable && !d.confirm { pad_hit(mx, my) } else { None };
        let t = &self.art.trans;
        for r in 0..10 {
            let y = row_y(r);
            s.image_part(g, t, X0 + 82.0, y, 32.0, 100.0, 191.0, 24.0);
            if d.editable {
                s.image_part(g, t, X0 + 50.0, y, 0.0, 150.0, 40.0, 24.0);
                match hover {
                    Some((k, true)) if k == r => s.image_part(g, t, X0 + 50.0, y, 0.0, 200.0, 35.0, 12.0),
                    Some((k, false)) if k == r => s.image_part(g, t, X0 + 50.0, y + 12.0, 0.0, 220.0, 35.0, 12.0),
                    _ => {}
                }
            }
            let v = self.club.pro_skill[r];
            if v != 0 {
                s.text(g, X0 + 87.0, top(y + 7.0, BODY), &skill_value(v), BODY, rgb(0.0, 0.0, 0.0));
            }
            let c = if v != 0 { rgb(0.0, 0.0, 0.0) } else { c15(0x4210) };
            s.text(g, X0 + 138.0, top(y + 7.0, BODY), SKILL_LABELS[r], BODY, c);
        }
        if d.editable {
            // the OK ball: footage of the original shows the yellow cut with the pointer away from it
            s.image_part(g, t, X0 + 326.0, 318.0, 350.0, 140.0, 50.0, 50.0);
        }
        if d.confirm {
            let b = confirm_box();
            self.draw_choice_box(g, &s, &b, b.option_at(mx, my));
        }
        g.flush();
    }

    /// Before the club's tick: the requested round starts at the pro's employee place (main frame 0x40fd08).
    pub fn pro_round_tick(&mut self) {
        if self.club.game & pro::START_ROUND == 0 {
            return;
        }
        if self.club.game & sg_core::golfer::game::REPEAT != 0 {
            self.club.game &= !pro::START_ROUND;
            return;
        }
        let Some(e) = self.employees.iter().find(|e| e.active && e.job == staff::job::OWNER).copied() else {
            self.club.game &= !pro::START_ROUND;
            return;
        };
        self.club.start_pro_round(&self.course, &mut self.exe_rng, (e.x, e.y), e.dir as i32);
        if self.club.gary >= 0 {
            println!("[{:6.1}s] {} starts a round (golfer {})", self.sim_time, self.pro_name(), self.club.gary);
            self.panel = 5;
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
            println!(
                "[{:6.1}s] {}'s round is over: {total} strokes on {holes} holes (hole {}, strokes {})",
                self.sim_time,
                self.pro_name(),
                gg.hole,
                gg.strokes
            );
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
        if self.club.skill_points > 0 && self.club.pro_mask != 0 && self.ui_ok && self.screen == Screen::Play && self.club.award_pending < 0
        {
            let pts = self.club.skill_points;
            self.open_skills(pts, None);
            if let Some(d) = self.skill_dialog.as_mut() {
                d.award = true;
            }
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
        self.panel = 5;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_and_ok() {
        // row 0 spans y 90..114: the top half adds, the lower half removes
        assert_eq!(pad_hit(260.0, 95.0), Some((0, true)));
        assert_eq!(pad_hit(260.0, 110.0), Some((0, false)));
        assert_eq!(pad_hit(260.0, 310.0), Some((9, true)));
        assert_eq!(pad_hit(300.0, 95.0), None);
        assert!(ok_hit(550.0, 334.0) && ok_hit(565.0, 350.0) && !ok_hit(575.0, 334.0));
    }
}
