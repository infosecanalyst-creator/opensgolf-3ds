//! SGA tournaments in the app: the Begin Tournament button with the SGA report or offer, the preparation checklist and its TV
//! towers, the leaderboard over the course, and the results with the prizes. The rules live in sg_core::tournament; the SGA
//! report, the leaderboard and the results follow the exe's layouts (docs/UI_SCREENS.md 4 and 6, docs/DECODE_TOURNAMENTS.md 6)
//! on the disc's art; the preparation checklist's layout is our own.

use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::{black, c15, dim, text_right};
use crate::popup_ui::ChoiceBox;
use crate::render::Rect;
use crate::ui::{rgb, text_width, Screen as Ui};
use sg_core::golfer::game;
use sg_core::land;
use sg_core::staff;
use sg_core::tournament::{Prep, Results, SgaReport};

/// The recommendations list (0x46d200: the generic popup at (400, 100) in its checkbox mode). Footage of the original lists
/// only the greens, the rough and the par changes, ticked, under a two-line heading, and the TV towers stand during the
/// tournament all the same: the towers' bit 0 is not a line of the list and stays set.
fn prep_box(prep: &Prep, mask: i32) -> ChoiceBox {
    let mut lines = vec!["In preparation for the tournament the".to_string(), "following changes have been recommended:".to_string()];
    lines.extend(prep.lines().into_iter().skip(1).map(|l| format!(" {l}")));
    let mut b = ChoiceBox::new(lines, 400.0, 100.0);
    b.checks = Some((mask >> 1) as u32);
    b
}

/// What the SGA screen shows: the report on its own, or the offer to accept.
#[derive(Clone, Debug, Default)]
pub struct SgaScreen {
    pub report: SgaReport,
    pub offer: bool,
    /// The offer's answer picked: 0 "Great, let the games begin!", 1 "I think I need more practice."
    pub sel: usize,
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
        self.sga = Some(SgaScreen { report, offer, sel: 0 });
        self.screen = Screen::Sga;
    }

    /// F7: the SGA evaluation as a report (a passing course is offered a tournament, as in the exe).
    pub fn sga_report(&mut self) {
        let fac = self.facilities();
        let report = self.club.sga_evaluate(fac);
        if self.ui_ok {
            self.sga = Some(SgaScreen { report, offer: false, sel: 0 });
            self.screen = Screen::Sga;
        }
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
        self.show_player_panel();
        self.edit = false;
        // a tournament started without the SGA's offer has no purse yet; the prize money is set when it ends
        let purse = if self.club.purse > 0 { format!("purse ${},000", self.club.purse) } else { "purse set at the end".into() };
        println!("[{:6.1}s] the tournament begins: {purse}, {} players", self.sim_time, self.club.leaderboard().0.len());
    }

    /// After the club's tick: the mid-year offer, the preparation checklist on the frame after the start, and the results.
    pub fn tourney_after_tick(&mut self) {
        let fac = self.facilities();
        let name = self.course_name.clone();
        self.club.tournament_offer(fac, &name);
        if self.club.game & game::TOURNAMENT != 0 && self.club.tourney_opts == -1 {
            let prep = self.club.tournament_prep();
            // a championship takes every item without asking
            if self.ui_ok && self.auto_aim == 0 && !self.club.championship() {
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
            } else if self.club.championship() {
                self.club.game = 0;
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

    /// The offer box over the SGA report (the generic popup 0x46d6e0 at (400, 360), main loop 0x41ce8b): its lines (the
    /// year's Open, the first prize, the two answers in quotes) and its rectangle.
    /// (Footage of the original: "2004 San Diego Open tournament".)
    fn offer_box(&self, sg: &SgaScreen) -> ChoiceBox {
        let lines = vec![
            "The SGA offers to hold the".to_string(),
            format!("{} tournament", self.open_name()),
            "at your course with a".to_string(),
            format!("first prize of \u{a7}{},000.", crate::ui::group(sg.report.purse.max(0) as u64)),
            " 'Great, let the games begin.'".to_string(),
            " 'I think I need more practice.'".to_string(),
        ];
        ChoiceBox::new(lines, 400.0, 360.0)
    }

    /// The offer's answer under (x, y): 0 yes, 1 no.
    fn offer_option_at(&self, sg: &SgaScreen, x: f32, y: f32) -> Option<usize> {
        self.offer_box(sg).option_at(x, y)
    }

    /// Keys on the tournament screens: Up and Down pick an answer to the offer, Enter takes it, Esc says no.
    pub fn tourney_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        if let (Screen::Sga, Some(sg)) = (self.screen, self.sga.as_mut()) {
            if sg.offer {
                match k {
                    KeyCode::Up | KeyCode::Down => {
                        sg.sel ^= 1;
                        return;
                    }
                    KeyCode::Escape => sg.sel = 1,
                    KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => {}
                    _ => return,
                }
                let sel = sg.sel;
                return self.offer_answer(sel);
            }
        }
        if matches!(k, KeyCode::Enter | KeyCode::KpEnter | KeyCode::Escape | KeyCode::Space) {
            self.tourney_click(-1.0, -1.0);
        }
    }

    /// "Great, let the games begin!" starts the tournament; "I think I need more practice." only drops the prize (the offer
    /// stays and the button can be pressed again).
    fn offer_answer(&mut self, k: usize) {
        self.sga = None;
        self.screen = Screen::Play;
        if k == 0 {
            self.accept_tournament();
        } else {
            self.club.purse = 0;
        }
    }

    pub fn tourney_click(&mut self, vx: f32, vy: f32) {
        match self.screen {
            Screen::Sga => {
                let Some(s) = self.sga.clone() else { return };
                if !s.offer {
                    // the SGA report closes on any click
                    self.sga = None;
                    self.screen = Screen::Play;
                } else if let Some(k) = self.offer_option_at(&s, vx, vy) {
                    self.offer_answer(k);
                } else if vx < 0.0 {
                    self.offer_answer(1);
                }
            }
            Screen::Prep => {
                let Some((prep, mut mask)) = self.prep.take() else { return };
                let b = prep_box(&prep, mask);
                if let Some(k) = b.option_at(vx, vy) {
                    mask ^= 2 << k;
                }
                if b.ok_tick().has(vx, vy) || vx < 0.0 {
                    // bits 0..2 are the three options, the par changes follow from bit 3
                    self.screen = Screen::Play;
                    self.finish_prep(prep, mask);
                } else {
                    self.prep = Some((prep, mask));
                }
            }
            _ => {
                // the results wait for the OK tick (or a key)
                let ok = self.results.as_ref().map(results_ok_y).unwrap_or(0.0);
                if vx >= 0.0 && !Rect::new(OK_X, ok, 44.0, 44.0).has(vx, vy) {
                    return;
                }
                self.results = None;
                self.screen = Screen::Play;
                if self.club.championship() {
                    // the championship is over: back to the title menu, nothing saved
                    self.club.game = 0;
                    self.screen = Screen::Menu;
                    self.hover = -1;
                }
            }
        }
    }

    pub fn draw_tourney_screen(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        match self.screen {
            Screen::Sga => {
                let Some(mut sg) = self.sga.clone() else { return };
                self.draw_sga(g, &s, &sg.report, !sg.offer);
                if sg.offer {
                    let (px, py) = self.info.pointer;
                    if let Some(k) = self.offer_option_at(&sg, px, py) {
                        sg.sel = k;
                        if let Some(o) = self.sga.as_mut() {
                            o.sel = k;
                        }
                    }
                    let b = self.offer_box(&sg);
                    self.draw_choice_box(g, &s, &b, Some(sg.sel));
                }
            }
            Screen::Prep => {
                let Some((prep, mask)) = self.prep.clone() else { return };
                let b = prep_box(&prep, mask);
                let (px, py) = self.info.pointer;
                self.draw_choice_box(g, &s, &b, b.option_at(px, py));
            }
            _ => {
                let Some(res) = self.results.clone() else { return };
                self.draw_results(g, &s, &res);
            }
        }
        g.flush();
    }

    /// The SGA report (0x44fb30 with mode 1, or mode 2 under the offer): the ten criteria with their values, score pips and
    /// ideal (minimum), and the committee's recommendation.
    fn draw_sga(&self, g: &mut Gfx, s: &Ui, r: &SgaReport, ok: bool) {
        let a = &self.info.art;
        dim(g, s);
        if a.sga.tex.is_some() {
            s.image_part(g, &a.sga, 35.0, 26.0, 35.0, 26.0, 730.0, 419.0);
        } else {
            s.fill(g, 35.0, 26.0, 730.0, 419.0, rgb(0.6, 0.6, 0.8));
        }
        let ink = black();
        let red = c15(0x7d08);
        // fonts (0x44fb30 in its SGA modes): the title in 0x821020 (Klepto 24) left at (190, 42); the lines in 0x821ee8
        // (Manual SSi 14): the headings' tops at 77 ("0/100" at 80), the rows' at 104 + 17 i, the recommendation's label at
        // (174, 288); then centred on 400 at 310 the verdict in 0x821f28 (Manual SSi 16) or the event in 0x821020, and the
        // purse in 0x821f28 at 338
        use crate::ui::{F_INFO14, F_INFO16, F_INFO_TITLE};
        s.put(g, F_INFO_TITLE, 190.0, 42.0, "REPORT of the SIM GOLF ASSOCIATION", ink);
        let c = r.class.clamp(0, 3);
        let class = sg_core::economy::RANK_NAMES[c as usize];
        s.put(g, F_INFO14, 75.0, 77.0, &format!("Selection Criteria: {class}"), ink);
        // "Grade:" at (306, 77); the score out of 100 right-aligned at 396, or a failed course's red "0/100" at (370, 80)
        s.put(g, F_INFO14, 306.0, 77.0, "Grade:", ink);
        if r.score <= 0 {
            s.put(g, F_INFO14, 370.0, 80.0, "0/100", red);
        } else {
            s.put_right(g, F_INFO14, 396.0, 77.0, &format!("{}/100", r.score), ink);
        }
        s.put(g, F_INFO14, 497.0, 77.0, "Ideal  (Minimum)", ink);
        // the ideal, then the minimum in brackets (0x44fb30): the length grouped by thousands with a "+"
        let k = r.ideal[4];
        let ideal_len = r.ideal[0];
        let min_len = if r.ideal[1] == 18 { ideal_len - 1000 } else { ideal_len - ideal_len * 10 / (c * 5 + 20) };
        let min_k = (k - 9).clamp(0, 99);
        let num = |v: i32| if v < 0 { format!("-{}", crate::ui::group(v.unsigned_abs() as u64)) } else { crate::ui::group(v as u64) };
        let ideals = [
            format!("{}+  ({})", num(ideal_len), num(min_len)),
            format!("{}  ({})", r.ideal[1], r.ideal[1] - 9 / (4 - c)),
            "4 hours or less  (max: 5 hrs)".to_string(),
            "100%+".to_string(),
            format!("{k}  ({min_k})"),
            format!("{k}  ({min_k})"),
            format!("{k}  ({min_k})"),
            format!("{k}  ({min_k})"),
            format!("{k}  ({min_k})"),
            format!("{}  ({})", k / 2 + 1, (k / 2 - 3).clamp(0, 99)),
        ];
        for (i, label) in SGA_ROWS.iter().enumerate() {
            let y = 104.0 + 17.0 * i as f32;
            let ty = y + 10.0;
            s.text_centered(g, 119.0, ty, label, 14.0, ink);
            let v = r.values[i];
            let value = match i {
                0 => format!("{} yds.", num(v)),
                2 if v > 59 => format!("{} h {} m", v / 60, v % 60),
                2 => format!("{v} m"),
                3 => format!("{v}%"),
                _ => v.to_string(),
            };
            s.text_centered(g, 238.0, ty, &value, 14.0, ink);
            if r.scores[i] == 0 {
                s.text_centered(g, 386.0, ty, "- not acceptable -", 14.0, red);
            } else {
                for j in 0..r.scores[i] {
                    // the pip is GBUBBLES cut 17 (object 0x59b33c), the small gold star, at (303 + 14 j, row top)
                    let x = 303.0 + 14.0 * j as f32;
                    if self.hud.icons.tex.is_some() {
                        s.image_part(g, &self.hud.icons, x, y, 272.0, 324.0, 16.0, 16.0);
                    } else {
                        s.fill(g, x + 2.0, y + 4.0, 7.0, 7.0, c15(0x1284));
                    }
                }
            }
            s.text(g, 490.0, ty, &ideals[i], 14.0, ink);
        }
        s.put(g, F_INFO14, 174.0, 288.0, "Committee recommendation", ink);
        if r.score <= 0 {
            s.put_centered(g, F_INFO16, 400.0, 310.0, "Improvement Required.", red);
        } else {
            s.put_centered(g, F_INFO_TITLE, 400.0, 310.0, r.event, ink);
            s.put_centered(g, F_INFO16, 400.0, 338.0, &format!("\u{a7}{},000 first prize.", crate::ui::group(r.purse.max(0) as u64)), ink);
        }
        // footage of the original: the art's gold tick is covered by the blue idle cut, the offer's box up or not
        let _ = ok;
        self.ok_tick(g, s, 701.0, 398.0, false);
    }

    /// The year's tournament as the SGA names it: "2004 San Diego Open" in footage of the original (the offer box and the
    /// leader board), the year and the property's name, not the course's ("Dolphin Coast MC" there).
    pub fn open_name(&self) -> String {
        let place = self
            .land
            .as_ref()
            .and_then(|l| sg_core::properties::PROPERTIES.get(l.slot.property))
            .map(|p| p.name.to_string())
            .unwrap_or_else(|| self.course_name.clone());
        format!("{} {place} Open", 2001 + self.econ.year_index())
    }

    /// The leaderboard box over the course while the tournament runs (0x45a090), as footage of the original shows it: the
    /// translucent dialog frame from the top left corner (its right line at x 141, its bottom line at 158 with ten rows), the
    /// title "LEADER BOARD of" / "the §120,000" / "2004 San Diego Open" in pale yellow, then "N. Name (E)" per golfer,
    /// white, the pro's own row in green. All in Arial Bold 10 (0x519fd8).
    pub fn draw_leaderboard(&self, g: &mut Gfx, s: &Ui) {
        if self.club.game & game::TOURNAMENT == 0 {
            return;
        }
        let (rows, _) = self.club.leaderboard();
        let holes = (self.club.next_hole - 1).max(0);
        let shown = rows.len().min(36);
        // DERIVED from the footage: 143 wide and 11 pixels a row plus 49 (159 with ten rows), rounded by the frame
        let w = 143.0;
        self.art.trans_frame(g, s, 0.0, 0.0, w, 11.0 * shown as f32 + 49.0);
        let pale = c15(0x7ff0);
        let purse = if self.club.purse == 0 { 20 * holes } else { self.club.purse };
        let money = format!("\u{a7}{},000", crate::ui::group(purse.max(0) as u64));
        let lines = if self.club.championship() {
            // (no footage of a championship's board: the event's name in place of the year's Open)
            ["LEADER BOARD of the".to_string(), sg_core::championship::EVENTS[self.difficulty.clamp(0, 3) as usize].to_string(), money]
        } else {
            ["LEADER BOARD of".to_string(), format!("the {money}"), self.open_name()]
        };
        // PLACEHOLDER: a line wider than the box is drawn smaller (the exe's handling is not decoded)
        let fit = |t: &str, room: f32| {
            let tw = text_width(t, 10.0);
            if tw > room {
                10.0 * room / tw
            } else {
                10.0
            }
        };
        crate::ui::set_face(Some(crate::ui::Face::Arial));
        // the title lines centred on 72 with their tops at 7, 19 and 31, an 11 pixel row per golfer from 44, all over a black
        // shadow one pixel below (footage)
        let shadow = rgb(0.0, 0.0, 0.0);
        for (t, top) in lines.iter().zip([7.0, 19.0, 31.0]) {
            let size = fit(t, w - 10.0);
            s.text_centered(g, 72.0, crate::ui::top(top, size) + 1.0, t, size, shadow);
            s.text_centered(g, 72.0, crate::ui::top(top, size), t, size, pale);
        }
        for (i, r) in rows.iter().take(shown).enumerate() {
            let c = c15(if r.gary { 0x43f0 } else { 0x7fff });
            let line = format!("{}. {} ({})", i + 1, r.name, score_text(r.score));
            let size = fit(&line, w - 10.0).min(10.0);
            let y = crate::ui::top(44.0 + 11.0 * i as f32, size);
            s.text(g, 5.0, y + 1.0, &line, size, shadow);
            s.text(g, 5.0, y, &line, size, c);
        }
        crate::ui::set_face(None);
    }

    /// TOURNAMENT RESULTS: the header band, a band per row (paid places, the cut line row, the rest), the strokes per hole
    /// coloured against par, the score to par and the prize, then the closing band with the OK tick.
    fn draw_results(&self, g: &mut Gfx, s: &Ui, res: &Results) {
        let a = &self.info.art.result;
        dim(g, s);
        let has = a.tex.is_some();
        if has {
            s.image_part(g, a, 0.0, 0.0, 0.0, 0.0, 800.0, 106.0);
        }
        let ink = black();
        // fonts (0x45a090): the title in 0x821020 (Klepto 24) centred at (320, 16), the rest in 0x821ee8 (Manual SSi 14)
        s.put_centered(g, crate::ui::F_INFO_TITLE, 320.0, 16.0, "TOURNAMENT RESULTS", ink);
        let holes = (1..19).filter(|&h| res.pars[h] != 0).collect::<Vec<_>>();
        let nh = holes.len();
        // the headings' tops are at y 50; ours draws from the baseline
        let hy = 50.0 + 10.0;
        s.text(g, 25.0, hy, "Ranking", 14.0, ink);
        for (k, h) in holes.iter().enumerate() {
            s.text_centered(g, 176.0 + 27.0 * k as f32, hy, &h.to_string(), 14.0, ink);
        }
        s.text_centered(g, 667.0, hy, "F", 14.0, ink);
        s.text(g, 700.0, hy, "Prize", 14.0, ink);
        let mut top = 70.0;
        let mut drawn = 0;
        for (i, r) in res.rows.iter().enumerate().take(18) {
            let place = i + 1;
            let (sy, sh) = band(place, nh);
            if top + sh > 548.0 {
                break;
            }
            if has {
                s.image_part(g, a, 0.0, top, 0.0, sy, 800.0, sh);
            }
            let ty = top + sh / 2.0 + 4.0;
            let row_c = if r.gary {
                c15(0x1284)
            } else if place <= nh {
                ink
            } else {
                c15(0x4210)
            };
            let name = format!("{place}. {}", r.name);
            let tw = text_width(&name, 14.0);
            s.text(g, 18.0, ty, &name, if tw > 136.0 { 14.0 * 136.0 / tw } else { 14.0 }, row_c);
            let by_par = |d: i32| match d {
                0 => row_c,
                d if d > 0 => c15(0x6000),
                _ => c15(0x0018),
            };
            for (k, &h) in holes.iter().enumerate() {
                let v = r.card[h] as i32;
                if v > 0 {
                    s.text_centered(g, 176.0 + 27.0 * k as f32, ty, &v.to_string(), 14.0, by_par(v - res.pars[h]));
                }
            }
            s.text_centered(g, 669.0, ty, &score_text(r.score), 14.0, by_par(r.score));
            if place <= nh && r.prize > 0 {
                text_right(s, g, 780.0, ty, &format!("{},000", crate::ui::group(r.prize as u64)), 14.0, row_c);
            }
            top += sh;
            drawn += 1;
        }
        let banner_a = drawn == 18 && nh + 1 == 19;
        if has {
            s.image_part(g, a, 0.0, top, 0.0, if banner_a { 357.0 } else { 274.0 }, 800.0, 51.0);
        }
        // the tick sits 4 below the closing band's top (0x452ec0: band at y - 7 or y - 4, tick at y - 54 or y - 51 plus
        // the band's 51 height)
        self.ok_tick(g, s, OK_X, top + 4.0, false);
    }
}

const OK_X: f32 = 732.0;

const SGA_ROWS: [&str; 10] = [
    "Length of Course",
    "Number of Holes",
    "Time to Play",
    "Fun Factor",
    "Holes with Variety",
    "Scenic Holes",
    "Length Holes",
    "Accuracy Holes",
    "Imagination Holes",
    "Facilities on Site",
];

/// The band a results row is drawn on (its y on the sheet and height): the paid places, the row after them, the rest.
fn band(place: usize, holes: usize) -> (f32, f32) {
    if place <= holes {
        (180.0, 26.0)
    } else if place == holes + 1 {
        (125.0, 22.0)
    } else {
        (224.0, 18.0)
    }
}

/// Where the results' OK tick sits: on the closing band under the last row.
fn results_ok_y(res: &Results) -> f32 {
    let nh = (1..19).filter(|&h| res.pars[h] != 0).count();
    let mut top = 70.0;
    for i in 0..res.rows.len().min(18) {
        let sh = band(i + 1, nh).1;
        if top + sh > 548.0 {
            break;
        }
        top += sh;
    }
    top + 4.0
}

fn score_text(v: i32) -> String {
    match v {
        0 => "E".to_string(),
        v if v > 0 => format!("+{v}"),
        v => format!("{v}"),
    }
}
