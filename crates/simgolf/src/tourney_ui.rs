//! SGA tournaments in the app: the Begin Tournament button with the SGA report or offer, the preparation checklist and its TV
//! towers, the leaderboard over the course, and the results with the prizes. The rules live in sg_core::tournament; the SGA
//! report, the leaderboard, the results and the recommendations list follow the exe's layouts (docs/UI_SCREENS.md 4 and 6,
//! docs/DECODE_TOURNAMENTS.md 6 and 9) on the disc's art.

use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::{black, c15, dim};
use crate::popup_ui::ChoiceBox;
use crate::render::Rect;
use crate::ui::{rgb, Screen as Ui};
use sg_core::golfer::game;
use sg_core::land;
use sg_core::staff;
use sg_core::tournament::{Prep, Results, SgaReport};

/// The recommendations list (0x46d200: the generic popup at (400, 100) in its checkbox mode, every box ticked to begin
/// with): the two-line heading, then one option a line, the TV towers (bit 0), the greens (bit 1), the rough (bit 2) and
/// each par change (bit 3 on). The exe builds the text with the TV towers' line always in it and places the towers only
/// when that box stays ticked. (Footage of the original, p1 4626-4646, shows a list without the towers' line and a tower
/// already standing while it is open: a different build of the game from the publisher's exe, which the port follows.)
fn prep_box(prep: &Prep, mask: i32) -> ChoiceBox {
    let mut lines = vec!["In preparation for the tournament the".to_string(), "following changes have been recommended:".to_string()];
    lines.extend(prep.lines().into_iter().map(|l| format!(" {l}")));
    let mut b = ChoiceBox::new(lines, 400.0, 100.0);
    b.checks = Some(mask as u32);
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
                    mask ^= 1 << k;
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

    /// The leaderboard box over the course while the tournament runs (0x45a090 while a golfer is still out), as the exe
    /// draws it: the translucent dialog frame at (0, 8), 144 wide and 22 (H + 1) + 16 high (rounded by the frame, so from y 3
    /// with five holes), the title "LEADER BOARD of" / "the §120,000" / "2004 San Diego Open" centred on x 72 with tops at 9,
    /// 21 and 33 in 0x7ff0, then from 45, 11 apart, "N. Name (E)" per golfer in white, the pro's own row (slot 1) in 0x23e8;
    /// places 1 to 9 start at x 7 and two-digit places at x 1 (footage: "10. Gary Golf" starts at the frame's edge). All in
    /// Arial Bold 10 (0x519fd8) at full size: the exe never shrinks a line, a long one runs over the frame. The black shadow
    /// one pixel below is from footage of the original. Footage (p1 4700-6200) shows these positions 2 pixels higher, the
    /// registration's error at the top of the screen (the cash pill is 2 pixels high there too).
    pub fn draw_leaderboard(&self, g: &mut Gfx, s: &Ui) {
        if self.club.game & game::TOURNAMENT == 0 {
            return;
        }
        let (rows, _) = self.club.leaderboard();
        let next = self.club.next_hole.max(1);
        self.art.trans_frame(g, s, 0.0, 8.0, 144.0, (22 * next + 16) as f32);
        let f = crate::ui::F_ARIAL10;
        let pale = c15(0x7ff0);
        // the exe writes the default first prize (H x 20) after this frame's title, so a zero purse shows for one frame
        let purse = if self.club.purse == 0 { 20 * (next - 1) } else { self.club.purse };
        let lines = if self.club.championship() {
            // championship play: the event by the difficulty (0x822c88), then "at " and the course's name without its class
            [
                "LEADER BOARD of the".to_string(),
                sg_core::championship::EVENTS[self.difficulty.clamp(0, 3) as usize].to_string(),
                format!("at {}", self.base_course_name()),
            ]
        } else {
            // the prize in thousands as a plain number (itoa) then ",000": no grouping
            ["LEADER BOARD of".to_string(), format!("the \u{a7}{purse},000"), self.open_name()]
        };
        let shadow = rgb(0.0, 0.0, 0.0);
        for (t, y) in lines.iter().zip([9.0, 21.0, 33.0]) {
            s.put_centered(g, f, 72.0, y + 1.0, t, shadow);
            s.put_centered(g, f, 72.0, y, t, pale);
        }
        let mut y = 45.0;
        for (i, r) in rows.iter().take(36).enumerate() {
            let place = i + 1;
            let c = c15(if r.gary { 0x23e8 } else { 0x7fff });
            let line = format!("{place}. {} ({})", r.name, score_text(r.score));
            let x = if place < 10 { 7.0 } else { 1.0 };
            s.put(g, f, x, y + 1.0, &line, shadow);
            s.put(g, f, x, y, &line, c);
            y += 11.0;
            if y > 548.0 {
                break;
            }
        }
    }

    /// TOURNAMENT RESULTS (0x45a090 once nobody is out): the header band, a band per row (paid places, the cut line row, the
    /// rest), the strokes per hole coloured against par, the total strokes and the prize, then the closing band with the OK
    /// tick. The bands are the loader's cuts of the sheet (0x44c2f0: (0, 0, 800, 106), (0, 180, 800, 26), (0, 125, 800,
    /// 22), (0, 224, 800, 18), (0, 357, 800, 51), (0, 274, 800, 51)), cut with the transparent edges trimmed and drawn
    /// without the trim: every band lands 2 pixels left of its place on the sheet (its first 2 columns are transparent)
    /// and the header 10 pixels higher too (its first 10 rows). Footage of the original (p2 86-92) matches this to a pixel.
    fn draw_results(&self, g: &mut Gfx, s: &Ui, res: &Results) {
        let a = &self.info.art.result;
        dim(g, s);
        let has = a.tex.is_some();
        if has {
            s.image_part(g, a, -TRIM_X, 0.0, 0.0, HEADER_TRIM_Y, 800.0, 106.0 - HEADER_TRIM_Y);
        }
        let ink = black();
        // fonts (0x45a090): the title in 0x821020 (Klepto 24) centred at (320, 16), the rest in 0x821ee8 (Manual SSi 14),
        // every y below the text's top
        use crate::ui::F_INFO14 as F;
        s.put_centered(g, crate::ui::F_INFO_TITLE, 320.0, 16.0, "TOURNAMENT RESULTS", ink);
        let nh = (1..19).filter(|&h| res.pars[h] != 0).count();
        // the headings at y 50: "Ranking" from x 25, each open hole's number centred on its column (175 + 27 (h - 1), by
        // hole number), "F" on 667 and "Prize" from 700
        s.put(g, F, 25.0, 50.0, "Ranking", ink);
        for h in (1..19).filter(|&h| res.pars[h] != 0) {
            s.put_centered(g, F, hole_x(h), 50.0, &h.to_string(), ink);
        }
        s.put_centered(g, F, 667.0, 50.0, "F", ink);
        s.put(g, F, 700.0, 50.0, "Prize", ink);
        let (ys, end) = result_rows(res);
        for ((i, r), &y) in res.rows.iter().enumerate().zip(&ys) {
            let place = i + 1;
            // the band under the row: a paid place's at y - 7, the first unpaid place's (the cut line) at y - 8, the rest's
            // at y - 4
            let (sy, sh, dy) = band(place, nh);
            if has {
                s.image_part(g, a, -TRIM_X, y - dy, 0.0, sy, 800.0, sh);
            }
            // the place and name from x 25, never shrunk: the pro's own row in 0x1284, a paid place in black, the rest grey
            let row_c = if r.gary {
                c15(0x1284)
            } else if place <= nh {
                ink
            } else {
                c15(0x4210)
            };
            s.put(g, F, 25.0, y, &format!("{place}. {}", r.name), row_c);
            // the strokes per hole in black at par, 0x6000 (red) under par and 0x0018 (blue) over it (footage of the
            // original shows a birdie red and a bogey blue)
            for h in 1..19 {
                let v = r.card[h] as i32;
                if v != 0 {
                    let c = match v.cmp(&res.pars[h]) {
                        std::cmp::Ordering::Less => c15(0x6000),
                        std::cmp::Ordering::Greater => c15(0x0018),
                        std::cmp::Ordering::Equal => ink,
                    };
                    s.put_centered(g, F, hole_x(h), y, &v.to_string(), c);
                }
            }
            // "F" is the round's total strokes, centred on 669, coloured by the score against par the other way round:
            // 0x6000 over par, 0x0018 under (footage: every total over par in red)
            let fc = match r.score.cmp(&0) {
                std::cmp::Ordering::Greater => c15(0x6000),
                std::cmp::Ordering::Less => c15(0x0018),
                std::cmp::Ordering::Equal => ink,
            };
            s.put_centered(g, F, 669.0, y, &r.total.to_string(), fc);
            // a paid place's prize, "§108,000", right aligned at 780 in 0x1284 whoever won it
            if place <= nh && r.prize > 0 {
                s.put_right(g, F, 780.0, y, &format!("\u{a7}{},000", crate::ui::group(r.prize as u64)), c15(0x1284));
            }
        }
        let (sy, dy) = if ys.len() == 18 && nh + 1 == 19 { (357.0, 7.0) } else { (274.0, 4.0) };
        if has {
            s.image_part(g, a, -TRIM_X, end - dy, 0.0, sy, 800.0, 51.0);
        }
        self.ok_tick(g, s, OK_X, results_ok_y(res), false);
    }
}

const OK_X: f32 = 732.0;
/// The transparent columns at the left of every results band and rows at the top of the header, which the exe's sprite
/// cutter trims (0x492000) and its draw does not put back.
const TRIM_X: f32 = 2.0;
const HEADER_TRIM_Y: f32 = 10.0;

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

/// The band a results row is drawn on (its y on the sheet, its height, and how far above the row's text it starts): the
/// paid places, the first place after them (the cut line), the rest.
fn band(place: usize, holes: usize) -> (f32, f32, f32) {
    if place <= holes {
        (180.0, 26.0, 7.0)
    } else if place == holes + 1 {
        (125.0, 22.0, 8.0)
    } else {
        (224.0, 18.0, 4.0)
    }
}

/// The x a hole's column is centred on in the results (0x45a090: 175 for hole 1, 27 apart, by hole number).
fn hole_x(h: usize) -> f32 {
    175.0 + 27.0 * (h as f32 - 1.0)
}

/// The text tops of the results' rows and the y after the last one (0x45a090): from 77, a paid place steps by the paid
/// band's 26 and every other place by the unpaid band's 18 (the cut line's band is 22 high but the step is 18); the rows
/// stop after the 18th or once the next row would start below 548.
fn result_rows(res: &Results) -> (Vec<f32>, f32) {
    let nh = (1..19).filter(|&h| res.pars[h] != 0).count();
    let mut y = 77.0;
    let mut ys = Vec::new();
    for i in 0..res.rows.len() {
        ys.push(y);
        y += if i < nh { 26.0 } else { 18.0 };
        if y > 548.0 || ys.len() >= 18 {
            break;
        }
    }
    (ys, y)
}

/// Where the results' OK tick sits (x 732): 4 below the closing band's top, which is 7 above the next row's y for the long
/// banner (18 rows on 18 holes) and 4 above it otherwise.
fn results_ok_y(res: &Results) -> f32 {
    let nh = (1..19).filter(|&h| res.pars[h] != 0).count();
    let (ys, end) = result_rows(res);
    if ys.len() == 18 && nh + 1 == 19 {
        end - 3.0
    } else {
        end
    }
}

fn score_text(v: i32) -> String {
    match v {
        0 => "E".to_string(),
        v if v > 0 => format!("+{v}"),
        v => format!("{v}"),
    }
}
