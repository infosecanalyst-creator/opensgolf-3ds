//! SGA tournaments in the app: the Begin Tournament button with the SGA report or offer, the preparation checklist and its TV
//! towers, the leaderboard over the course, and the results with the prizes. The rules live in sg_core::tournament; the
//! layout of these screens is our own.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{rgb, rgba, Screen as Ui};
use sg_core::golfer::game;
use sg_core::land;
use sg_core::staff;
use sg_core::tournament::{Prep, Results, SgaReport, CRITERIA};

const BOX: Rect = Rect::new(110.0, 40.0, 580.0, 470.0);
const YES: Rect = Rect::new(BOX.x + 40.0, BOX.y + 420.0, 260.0, 32.0);
const NO: Rect = Rect::new(BOX.x + 360.0, BOX.y + 420.0, 180.0, 32.0);

fn check_rect(i: usize) -> Rect {
    Rect::new(BOX.x + 24.0, BOX.y + 60.0 + 26.0 * i as f32, 20.0, 20.0)
}

/// What the SGA screen shows: the report on its own, or the offer to accept.
#[derive(Clone, Debug, Default)]
pub struct SgaScreen {
    pub report: SgaReport,
    pub offer: bool,
}

impl App {
    /// Building kinds 6..19 present and joined to the clubhouse (0x5a6370, bits 6..19).
    pub fn facilities(&self) -> i32 {
        let Some(l) = &self.land else { return 0 };
        let joined = l.joined_objects();
        let mut mask = 0u32;
        for (i, o) in l.objects.iter().enumerate() {
            if (6..20).contains(&o.kind) && joined.get(i).copied().unwrap_or(false) {
                mask |= 1 << o.kind;
            }
        }
        mask.count_ones() as i32
    }

    /// Panel button 3, Begin Tournament (main frame command 0x4a): the SGA evaluates the course again; a failing course shows
    /// the report, a passing one the offer.
    pub fn begin_tournament(&mut self) {
        if !self.club.can_begin_tournament() {
            self.show_toast("The SGA has not offered you a tournament yet");
            return;
        }
        self.club.purse = 0;
        let fac = self.facilities();
        let report = self.club.sga_evaluate(fac);
        let offer = report.passed();
        if !self.ui_ok {
            println!("SGA evaluation: score {}, {} (${},000)", report.score, report.event, report.purse);
            if offer {
                self.accept_tournament();
            }
            return;
        }
        self.sga = Some(SgaScreen { report, offer });
        self.screen = Screen::Sga;
    }

    /// "Great, let the games begin!": the pro's skills first if he has none, then the field.
    pub fn accept_tournament(&mut self) {
        if self.club.pro_skill.iter().all(|&v| v == 0) {
            let pts = self.club.first_skill_points();
            self.open_skills(pts, None);
        }
        let Some(e) = self.employees.iter().find(|e| e.job == staff::job::OWNER).copied() else { return };
        let cash = (self.econ.cash / sg_core::economy::Economy::UNIT) as i32;
        self.club.start_tournament(&self.course, &mut self.exe_rng, (e.x, e.y), e.dir as i32, cash);
        self.panel = 3;
        self.edit = false;
        println!(
            "[{:6.1}s] the tournament begins: purse ${},000, {} players",
            self.sim_time,
            self.club.purse,
            self.club.leaderboard().0.len()
        );
    }

    /// After the club's tick: the mid-year offer, the preparation checklist on the frame after the start, and the results.
    pub fn tourney_after_tick(&mut self) {
        let fac = self.facilities();
        let name = self.course_name.clone();
        self.club.tournament_offer(fac, &name);
        if self.club.game & game::TOURNAMENT != 0 && self.club.tourney_opts == -1 {
            let prep = self.club.tournament_prep();
            if self.ui_ok && self.auto_aim == 0 {
                let n = prep.lines().len();
                self.prep = Some((prep, (1 << n) - 1));
                self.screen = Screen::Prep;
            } else {
                self.finish_prep(prep, 0xfffff);
            }
        }
        if let Some(res) = self.club.tournament_tick(&mut self.exe_rng, &self.course) {
            self.remove_tv();
            for (i, r) in res.rows.iter().enumerate() {
                println!(
                    "  {:2}. {:<24} {:+3} ({} strokes) {}",
                    i + 1,
                    r.name,
                    r.score,
                    r.total,
                    if r.prize > 0 { format!("${},000", r.prize) } else { String::new() }
                );
            }
            match res.gary_rank {
                Some(r) => {
                    println!("[{:6.1}s] tournament over: {} finishes {r}, prize ${},000", self.sim_time, self.pro_name(), res.gary_prize)
                }
                None => println!("[{:6.1}s] tournament over: {} wins no prize", self.sim_time, self.pro_name()),
            }
            if self.ui_ok {
                self.results = Some(res);
                self.screen = Screen::Results;
            }
            self.panel = 0;
        }
    }

    fn finish_prep(&mut self, prep: Prep, mask: i32) {
        let sites = self.club.apply_prep(&self.course, &mut self.exe_rng, &prep, mask);
        for l in prep.lines() {
            println!("  prep: {l}");
        }
        if !sites.is_empty() {
            let theme = self.exe_theme();
            if let Some(l) = self.land.as_mut() {
                for (kind, a, b, dir) in sites {
                    let i = l.place(&mut self.exe_rng, a, b, kind, 0, theme);
                    if let Some(o) = l.objects.get_mut(i) {
                        o.dir = dir;
                        o.flags |= 0x40;
                    }
                    l.write_area(&mut self.terrain, a, b, a, b);
                }
            }
            self.after_object_change();
        }
    }

    /// The cleanup removes the TV towers and booths (0x46d0c0).
    pub fn remove_tv(&mut self) {
        let mut any = false;
        if let Some(l) = self.land.as_mut() {
            for i in 0..l.objects.len() {
                let o = l.objects[i];
                if o.kind == land::K_TV_TOWER || o.kind == land::K_TV_BOOTH {
                    l.remove_object(i);
                    l.write_area(&mut self.terrain, o.a, o.b, o.a, o.b);
                    any = true;
                }
            }
        }
        if any {
            self.after_object_change();
        }
    }

    /// 'n' during a tournament: it ends with no prizes.
    pub fn cancel_tournament(&mut self) {
        if self.club.cancel_tournament(&mut self.exe_rng, &self.course) {
            self.remove_tv();
            self.show_toast("The tournament has been canceled.");
            self.panel = 0;
        }
    }

    pub fn tourney_click(&mut self, vx: f32, vy: f32) {
        match self.screen {
            Screen::Sga => {
                let Some(s) = self.sga.take() else { return };
                if s.offer && YES.has(vx, vy) {
                    self.screen = Screen::Play;
                    self.accept_tournament();
                } else if NO.has(vx, vy) || (!s.offer && YES.has(vx, vy)) || vx < 0.0 {
                    self.club.purse = 0;
                    self.screen = Screen::Play;
                } else {
                    self.sga = Some(s);
                }
            }
            Screen::Prep => {
                let Some((prep, mut mask)) = self.prep.take() else { return };
                for i in 0..prep.lines().len() {
                    if check_rect(i).has(vx, vy) {
                        mask ^= 1 << i;
                    }
                }
                if YES.has(vx, vy) || vx < 0.0 {
                    // bits 0..2 are the three options, the par changes follow from bit 3
                    self.screen = Screen::Play;
                    self.finish_prep(prep, mask);
                } else {
                    self.prep = Some((prep, mask));
                }
            }
            _ => {
                self.results = None;
                self.screen = Screen::Play;
            }
        }
    }

    pub fn draw_tourney_screen(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.fill(g, BOX.x, BOX.y, BOX.w, BOX.h, rgba(0.08, 0.1, 0.28, 0.95));
        let white = rgb(1.0, 1.0, 1.0);
        let button = |g: &mut Gfx, r: Rect, t: &str| {
            s.fill(g, r.x, r.y, r.w, r.h, rgba(0.3, 0.55, 0.3, 0.9));
            s.text_centered(g, r.x + r.w / 2.0, r.y + 22.0, t, 15.0, white);
        };
        match self.screen {
            Screen::Sga => {
                let Some(sg) = self.sga.clone() else { return };
                let r = &sg.report;
                let class = sg_core::economy::RANK_NAMES[r.class.clamp(0, 3) as usize];
                s.text(
                    g,
                    BOX.x + 20.0,
                    BOX.y + 28.0,
                    &format!("SGA Report: {} course, {} holes", class, r.holes),
                    17.0,
                    rgb(1.0, 1.0, 0.8),
                );
                for (i, name) in CRITERIA.iter().enumerate() {
                    let y = BOX.y + 70.0 + 26.0 * i as f32;
                    let c = if r.scores[i] == 0 { rgb(1.0, 0.5, 0.45) } else { white };
                    s.text(g, BOX.x + 24.0, y, name, 15.0, c);
                    s.text(g, BOX.x + 250.0, y, &format!("{}", r.values[i]), 15.0, c);
                    s.text(g, BOX.x + 340.0, y, &format!("ideal {}", r.ideal[i]), 13.0, rgb(0.75, 0.75, 0.9));
                    s.text(g, BOX.x + 470.0, y, &format!("{}/10", r.scores[i]), 15.0, c);
                }
                let y = BOX.y + 70.0 + 26.0 * 10.0 + 14.0;
                if sg.offer {
                    s.text(
                        g,
                        BOX.x + 24.0,
                        y,
                        &format!("Score {}: the SGA offers to hold the {}", r.score, r.event),
                        15.0,
                        rgb(0.8, 1.0, 0.8),
                    );
                    s.text(
                        g,
                        BOX.x + 24.0,
                        y + 22.0,
                        &format!("at {}, first prize ${},000!", self.course_name, r.purse),
                        15.0,
                        rgb(0.8, 1.0, 0.8),
                    );
                    button(g, YES, "Great, let the games begin!");
                    button(g, NO, "Not now");
                } else {
                    s.text(g, BOX.x + 24.0, y, "Improvement Required", 17.0, rgb(1.0, 0.5, 0.45));
                    button(g, YES, "OK");
                }
            }
            Screen::Prep => {
                let Some((prep, mask)) = self.prep.clone() else { return };
                s.text(g, BOX.x + 20.0, BOX.y + 28.0, "In preparation for the tournament, the SGA asks you to:", 15.0, rgb(1.0, 1.0, 0.8));
                for (i, l) in prep.lines().iter().enumerate() {
                    let r = check_rect(i);
                    s.fill(g, r.x, r.y, r.w, r.h, rgba(0.9, 0.9, 1.0, 0.9));
                    if mask & (1 << i) != 0 {
                        s.fill(g, r.x + 4.0, r.y + 4.0, r.w - 8.0, r.h - 8.0, rgba(0.1, 0.4, 0.1, 1.0));
                    }
                    s.text(g, r.x + 32.0, r.y + 16.0, l, 14.0, white);
                }
                button(g, YES, "OK");
            }
            _ => {
                let Some(res) = self.results.clone() else { return };
                draw_results(&s, g, &res);
            }
        }
        g.flush();
    }

    /// The leaderboard box over the course while the tournament runs.
    pub fn draw_leaderboard(&self, g: &mut Gfx, s: &Ui) {
        if self.club.game & game::TOURNAMENT == 0 {
            return;
        }
        let (rows, _) = self.club.leaderboard();
        let n = rows.len().min(18);
        let h = 44.0 + 15.0 * n as f32;
        let (x, y) = (8.0, 84.0);
        s.fill(g, x, y, 220.0, h, rgba(0.1, 0.1, 0.25, 0.85));
        let purse = if self.club.purse == 0 { 20 * (self.club.next_hole - 1) } else { self.club.purse };
        s.text(g, x + 8.0, y + 16.0, &format!("LEADER BOARD  ${purse},000"), 13.0, rgb(1.0, 1.0, 0.7));
        s.text(g, x + 8.0, y + 32.0, &format!("{} {} Open", 2001 + self.econ.year_index(), self.course_name), 12.0, rgb(0.85, 0.85, 1.0));
        for (i, r) in rows.iter().take(n).enumerate() {
            let c = if r.gary { rgb(1.0, 0.85, 0.3) } else { rgb(1.0, 1.0, 1.0) };
            let yy = y + 48.0 + 15.0 * i as f32;
            s.text(g, x + 8.0, yy, &format!("{}. {}", i + 1, r.name), 12.0, c);
            s.text(g, x + 186.0, yy, &score_text(r.score), 12.0, c);
        }
    }
}

fn score_text(v: i32) -> String {
    match v {
        0 => "E".to_string(),
        v if v > 0 => format!("+{v}"),
        v => format!("{v}"),
    }
}

fn draw_results(s: &Ui, g: &mut Gfx, res: &Results) {
    s.text(g, BOX.x + 20.0, BOX.y + 28.0, "TOURNAMENT RESULTS", 18.0, rgb(1.0, 1.0, 0.8));
    s.text(g, BOX.x + 20.0, BOX.y + 50.0, "Ranking", 13.0, rgb(0.8, 0.8, 1.0));
    s.text(g, BOX.x + 330.0, BOX.y + 50.0, "Total", 13.0, rgb(0.8, 0.8, 1.0));
    s.text(g, BOX.x + 450.0, BOX.y + 50.0, "Prize", 13.0, rgb(0.8, 0.8, 1.0));
    for (i, r) in res.rows.iter().take(22).enumerate() {
        let y = BOX.y + 68.0 + 16.0 * i as f32;
        let c = if r.gary { rgb(1.0, 0.85, 0.3) } else { rgb(1.0, 1.0, 1.0) };
        s.text(g, BOX.x + 20.0, y, &format!("{}. {}", i + 1, r.name), 13.0, c);
        let sc = if r.score < 0 { rgb(1.0, 0.5, 0.45) } else { c };
        s.text(g, BOX.x + 330.0, y, &format!("{} ({})", r.total, score_text(r.score)), 13.0, sc);
        if r.prize > 0 {
            s.text(g, BOX.x + 450.0, y, &format!("${},000", r.prize), 13.0, c);
        }
    }
    s.text_centered(g, BOX.x + BOX.w / 2.0, BOX.y + BOX.h - 12.0, "Click to continue", 13.0, rgb(0.8, 0.8, 1.0));
}
