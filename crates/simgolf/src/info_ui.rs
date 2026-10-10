//! Information screens from the disc's infoscreens art: the Course Report and the Hole Stats dialog behind its rows, Best
//! Scores, Top 10 Designers, and the round OK tick (OkStates.pcx) the info screens share. Layouts follow the exe's 800 x 600
//! screens (docs/UI_SCREENS.md, docs/DECODE_HOLE_STATS2.md, docs/DECODE_TOP10_PAIR.md); the exe's text positions are the
//! tops of the text, ours are baselines, so most rows add a small offset to sit on the art's pills.

use crate::app::*;
use crate::gfx::Gfx;
use crate::ui::{load_pcx, load_pcx_alpha, rgb, rgba, text_width, Image, Screen as Ui};
use sg_core::ratings;
use sg_core::top10;

/// The art of the screens in this module and of the tournament, land and world screens.
#[derive(Default)]
pub struct InfoArt {
    pub ok: Image,
    pub report: Image,
    pub holestat: Image,
    pub lowscore: Image,
    pub top10_blank: Image,
    pub top10_cups: Image,
    pub sga: Image,
    pub result: Image,
    pub buy_land: Image,
    pub land_buttons: Image,
    pub world_buttons: Image,
    pub tacks: Image,
}

impl InfoArt {
    pub fn load(g: &mut Gfx, app: &App) -> InfoArt {
        let p = |rel: &str| app.game_path(&format!("Interface/{rel}"));
        let alpha = |g: &mut Gfx, c: &str, a: &str| load_pcx_alpha(g, &p(c), &p(a)).unwrap_or_default();
        let keyed = |g: &mut Gfx, c: &str| load_pcx(g, &p(c), true, None).unwrap_or_default();
        let plain = |g: &mut Gfx, c: &str| load_pcx(g, &p(c), false, None).unwrap_or_default();
        InfoArt {
            ok: keyed(g, "infoscreens/OkStates.pcx"),
            report: alpha(g, "infoscreens/coursereport.pcx", "infoscreens/coursereport_alpha.pcx"),
            holestat: alpha(g, "infoscreens/HoleSTAT.pcx", "infoscreens/HoleSTAT_alpha.pcx"),
            lowscore: alpha(g, "infoscreens/lowscore.pcx", "infoscreens/lowscore_alpha.pcx"),
            top10_blank: plain(g, "Top10_Blank.pcx"),
            top10_cups: plain(g, "Top10_Trophies.pcx"),
            sga: alpha(g, "infoscreens/SGA.pcx", "infoscreens/SGAreport_alpha.pcx"),
            result: alpha(g, "infoscreens/tournament result.pcx", "infoscreens/tournament result_alpha.pcx"),
            buy_land: plain(g, "infoscreens/buy_land.pcx"),
            land_buttons: keyed(g, "infoscreens/buy_land_buttons.pcx"),
            world_buttons: keyed(g, "WorldButton.pcx"),
            tacks: alpha(g, "TacksandArrow.pcx", "TacksandArrow_A.pcx"),
        }
    }
}

/// The state of the screens in this module (and the pointer the OK ticks light up under).
#[derive(Default)]
pub struct Info {
    pub art: InfoArt,
    /// The pointer in virtual coordinates.
    pub pointer: (f32, f32),
    /// The hole the Hole Stats dialog shows.
    pub stats_hole: usize,
    /// The Top 10 table on screen and the rank of the new entry (drawn in white and underlined).
    pub top10: Vec<top10::Entry>,
    pub top10_new: Option<usize>,
    /// The year end put the course in the Top 10: shown when the year-end report closes.
    pub top10_due: Option<usize>,
    /// The property chooser: the dark red message box, the purchase being announced ("We're off to ..."), with the frames
    /// it has been shown, and the button under the pointer.
    pub world_msg: Option<String>,
    pub world_go: Option<(usize, u32)>,
    /// The world map's button under the pointer and for how many frames (its label shows after 20).
    pub world_tip: (i32, u32),
}

/// 15-bit exe colour (5-5-5) to RGBA.
pub fn c15(v: u32) -> [f32; 4] {
    rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0)
}

pub fn black() -> [f32; 4] {
    rgb(0.0, 0.0, 0.0)
}

/// The exe's dim over the course before an info screen (strength 0.25).
pub fn dim(g: &mut Gfx, s: &Ui) {
    s.fill(g, -400.0, -400.0, 1600.0, 1400.0, rgba(0.0, 0.0, 0.0, 0.25));
}

/// Text right aligned at x.
pub fn text_right(s: &Ui, g: &mut Gfx, x: f32, y: f32, t: &str, size: f32, c: [f32; 4]) {
    s.text(g, x - text_width(t, size), y, t, size, c);
}

/// "1.23" in hundredths, "-" for negatives (0x42dd50).
pub fn hundredths(v: i32) -> String {
    format!("{}{}.{:02}", if v < 0 { "-" } else { "" }, v.abs() / 100, v.abs() % 100)
}

/// Rows of the Course Report: one per open hole, 17 px apart below the 104 px header.
const REPORT_TOP: f32 = 104.0;
const REPORT_PITCH: f32 = 17.0;
/// The report's cells on the art (left edge and width), in column order Yds .. Profit.
const CX: [f32; 12] = [112.0, 157.0, 190.0, 233.0, 283.0, 330.0, 375.0, 422.0, 468.0, 571.0, 641.0, 716.0];
const CW: [f32; 12] = [41.0, 29.0, 39.0, 46.0, 43.0, 41.0, 43.0, 42.0, 99.0, 66.0, 71.0, 74.0];
const HEAD: [&str; 12] = ["Yds", "Par", "Avg", "Time", "Fun", "+Len", "+Acc", "+Img", "Type", "Avg.Fee", "Revenue", "Profit"];

impl App {
    /// The OK tick at (x, y): the idle cut unless the art has it already, and the lit cut under the pointer.
    pub fn ok_tick(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, baked: bool) {
        let ok = &self.info.art.ok;
        if ok.tex.is_none() {
            return;
        }
        if !baked {
            s.image_part(g, ok, x, y, 1.0, 1.0, 44.0, 42.0);
        }
        if self.over_ok(x, y) {
            s.image_part(g, ok, x, y, 46.0, 1.0, 44.0, 42.0);
        }
    }

    pub fn over_ok(&self, x: f32, y: f32) -> bool {
        let (px, py) = self.info.pointer;
        px >= x && px < x + 44.0 && py >= y && py < y + 44.0
    }

    /// Closes an info screen; the year-end report hands over to the Top 10 when the course made it.
    pub fn close_info(&mut self) {
        if self.screen == Screen::YearEnd {
            if let Some(rank) = self.info.top10_due.take() {
                self.open_top10(Some(rank));
                return;
            }
        }
        self.screen = Screen::Play;
        self.hover = -1;
    }

    // ---- Course Report ----------------------------------------------------------------------------------------------------

    /// The open holes in report order.
    fn report_holes(&self) -> Vec<usize> {
        (1..19).filter(|&h| self.club.holes[h].par != 0).collect()
    }

    /// A click on the Course Report: a hole's row opens its Hole Stats, anything else closes the report.
    pub fn report_click(&mut self, vx: f32, vy: f32) {
        let holes = self.report_holes();
        let row = ((vy - REPORT_TOP) / REPORT_PITCH).floor();
        if vy >= REPORT_TOP && (0.0..holes.len() as f32).contains(&row) && (0.0..800.0).contains(&vx) {
            self.info.stats_hole = holes[row as usize];
            self.screen = Screen::HoleStats;
            self.ui_sound(0x38);
            return;
        }
        self.screen = Screen::Play;
        self.hover = -1;
    }

    /// The Course Report (0x44fb30 mode 0): header, a 17 px strip per open hole with its numbers and the art's highlight cells,
    /// the totals strip and the legend.
    pub fn draw_report(&mut self, g: &mut Gfx) {
        let art = &self.info.art.report;
        if !self.ui_ok || art.tex.is_none() {
            return;
        }
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        dim(g, &s);
        let ink = black();
        s.image_part(g, art, 0.0, 0.0, 0.0, 0.0, 800.0, REPORT_TOP);
        s.text_centered(g, 316.0, 50.0, "COURSE REPORT", 24.0, ink);
        for c in 0..12 {
            s.text_centered(g, CX[c] + CW[c] / 2.0, 86.0, HEAD[c], 12.0, ink);
        }
        let all = self.club.game & sg_core::golfer::game::TWO_TEES != 0;
        let holes = self.report_holes();
        let rows: Vec<_> = holes.iter().map(|&h| ratings::report_row(h, &self.club.holes[h], self.difficulty, all)).collect();
        let open = rows.len() as i32;
        let (mut t_yds, mut t_par, mut t_avg, mut t_min, mut t_fun, mut t_len, mut t_acc, mut t_img, mut t_fee, mut t_rev, mut t_prof) =
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        let mut m_last = 0;
        let digits = |v: i32| crate::screens_ui::digits(v as i64);
        for (i, r) in rows.iter().enumerate() {
            let y = REPORT_TOP + REPORT_PITCH * i as f32;
            s.image_part(g, art, 0.0, y, 0.0, if i % 2 == 1 { 169.0 } else { 129.0 }, 800.0, REPORT_PITCH);
            t_yds += r.yards;
            t_par += r.par;
            t_avg += r.avg;
            t_min += r.minutes;
            m_last = r.minutes;
            t_fun += r.fun;
            t_len += r.len;
            t_acc += r.acc;
            t_img += r.img;
            t_fee += r.avg_fee;
            t_rev += r.revenue;
            t_prof += r.profit;
            // highlight cells: 0 red, 1 bright green, 2 dark green (the three rows of cells on the sheet)
            let mark = |g: &mut Gfx, c: usize, k: Option<usize>| {
                if let Some(k) = k {
                    s.image_part(g, art, CX[c], y, CX[c], [205.0, 241.0, 275.0][k], CW[c], 15.0);
                }
            };
            let demand = |v: i32| match v {
                v if v < 0 => Some(0),
                50..=99 => Some(2),
                v if v >= 100 => Some(1),
                _ => None,
            };
            mark(g, 2, (r.flags & 0xc != 0).then_some(0));
            mark(
                g,
                3,
                if r.minutes >= 5 * r.par {
                    Some(0)
                } else if r.minutes <= 3 * r.par {
                    Some(1)
                } else {
                    None
                },
            );
            mark(
                g,
                4,
                if r.fun < 10 {
                    Some(0)
                } else if r.fun >= 50 {
                    Some(1)
                } else {
                    None
                },
            );
            mark(g, 5, demand(r.len));
            mark(g, 6, demand(r.acc));
            mark(g, 7, demand(r.img));
            mark(
                g,
                8,
                if r.variety < 2 {
                    Some(1)
                } else if r.variety >= 3 {
                    Some(0)
                } else {
                    None
                },
            );
            mark(g, 11, (r.profit < 0).then_some(0));
            let ty = y + 12.0;
            let hole = &self.club.holes[r.hole];
            s.text(g, 15.0, ty, &ratings::hole_name(r.hole, hole), 11.0, ink);
            // Top 100 / Top 18 / scenic marker (index 3..7 on the sheet's icon row), and the dogleg arrow in the type cell
            let tops = if r.flags & 1 != 0 { 1 + (r.flags >> 1 & 1) as usize } else { 0 };
            let m = 2 + tops + if r.scenic { 3 } else { 0 };
            if m > 2 {
                s.image_part(g, art, 90.0, y + 1.0, 194.0 + 15.0 * (m - 3) as f32, 557.0, 14.0, 15.0);
            }
            let dog = if r.flags & 0x60 == 0 { 1 } else { (r.flags >> 4 & 2) as usize };
            s.image_part(g, art, 555.0, y + 1.0, 547.0, 549.0 + 14.0 * dog as f32, 12.0, 13.0);
            text_right(&s, g, 151.0, ty, &r.yards.to_string(), 11.0, ink);
            s.text_centered(g, 172.0, ty, &r.par.to_string(), 11.0, ink);
            text_right(&s, g, 227.0, ty, &hundredths(r.avg), 11.0, ink);
            text_right(&s, g, 278.0, ty, &format!("{}m", r.minutes), 11.0, ink);
            text_right(&s, g, 324.0, ty, &format!("{}%", r.fun), 11.0, ink);
            text_right(&s, g, 369.0, ty, &hundredths(r.len), 11.0, ink);
            text_right(&s, g, 415.0, ty, &hundredths(r.acc), 11.0, ink);
            text_right(&s, g, 462.0, ty, &hundredths(r.img), 11.0, ink);
            s.text(g, 471.0, ty, r.type_name, 11.0, ink);
            text_right(&s, g, 635.0, ty, &digits(r.avg_fee), 11.0, ink);
            text_right(&s, g, 709.0, ty, &digits(r.revenue), 11.0, ink);
            text_right(&s, g, 786.0, ty, &digits(r.profit), 11.0, ink);
        }
        let total_y = REPORT_TOP + REPORT_PITCH * rows.len() as f32;
        s.image_part(g, art, 0.0, total_y, 0.0, 420.0, 800.0, 29.0);
        let ty = total_y + 21.0;
        if open > 0 {
            s.text(g, 15.0, ty, "Total", 11.0, ink);
            text_right(&s, g, 151.0, ty, &digits(t_yds), 11.0, ink);
            s.text_centered(g, 172.0, ty, &t_par.to_string(), 11.0, ink);
            text_right(&s, g, 227.0, ty, &hundredths(t_avg), 11.0, ink);
            // the exe shows the last hole's minutes past the hour, not the total's
            let time = if t_min >= 60 { format!("{}h {}m", t_min / 60, m_last % 60) } else { format!("{}m", m_last % 60) };
            text_right(&s, g, 278.0, ty, &time, 11.0, ink);
            text_right(&s, g, 324.0, ty, &format!("{}%", t_fun / open), 11.0, ink);
            for (c, (x, v)) in [(369.0, t_len / open), (415.0, t_acc / open), (462.0, t_img / open)].into_iter().enumerate() {
                // the totals light up only from 50 (the +Img total never does: the exe's own slip)
                let k = match v {
                    50..=99 => Some(2.0),
                    v if v >= 100 => Some(1.0),
                    _ => None,
                };
                if let (Some(k), true) = (k, c < 2) {
                    s.image_part(g, art, CX[5 + c], total_y + 9.0, CX[5 + c], [205.0, 241.0, 275.0][k as usize], CW[5 + c], 15.0);
                }
                text_right(&s, g, x, ty, &hundredths(v), 11.0, ink);
            }
            text_right(&s, g, 635.0, ty, &format!("\u{a7}{}", digits(t_fee / open)), 11.0, ink);
            text_right(&s, g, 709.0, ty, &format!("\u{a7}{}", digits(t_rev)), 11.0, ink);
            if t_prof < 0 {
                s.image_part(g, art, CX[11], total_y + 9.0, CX[11], 205.0, CW[11], 15.0);
            }
            text_right(&s, g, 786.0, ty, &format!("\u{a7}{}", digits(t_prof)), 11.0, ink);
        }
        let by = total_y + 29.0;
        s.image_part(g, art, 0.0, by, 0.0, 468.0, 800.0, 72.0);
        // the legend's tops at the bottom piece's y - 63 + its height (72), the tick 15 below them (0x4520f0)
        for (x, t) in [(178.0, "Top 100 Hole"), (335.0, "Top 18 Hole"), (487.0, "Scenic Hole")] {
            s.text_centered(g, x, crate::screens_ui::top(by + 9.0, 11.0), t, 11.0, ink);
        }
        self.ok_tick(g, &s, 734.0, by + 24.0, false);
        g.flush();
    }

    // ---- Hole Stats -------------------------------------------------------------------------------------------------------

    /// The Hole Stats dialog (0x453330) for the hole picked on the Course Report. It closes on any click.
    pub fn draw_hole_stats(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        let art = &self.info.art.holestat;
        if art.tex.is_none() {
            return;
        }
        dim(g, &s);
        let h = self.info.stats_hole.clamp(1, 18);
        let hole = &self.club.holes[h];
        let st = ratings::hole_stats(hole);
        let ink = black();
        s.image_part(g, art, 0.0, 0.0, 0.0, 0.0, 800.0, 220.0);
        let mut title = format!("HOLE STATS for {}", ratings::hole_name(h, hole));
        if hole.flags & 0x81 != 0 {
            title += &format!(" ({h})");
        }
        s.text_centered(g, 385.0, 64.0, &title, 18.0, ink);
        // left column: label at 190, value centred at 356
        let rows = [(83.0, "Fun Factor"), (104.0, "Length"), (125.0, "Accuracy"), (146.0, "Imagination")];
        for (y, l) in rows {
            s.text(g, 190.0, y + 10.0, l, 12.0, ink);
        }
        if let Some(f) = st.fun {
            s.text_centered(g, 356.0, 93.0, &format!("{f}%-({})", ratings::fun_word(f)), 12.0, ink);
        }
        for (k, v) in [st.len, st.acc, st.img].into_iter().enumerate() {
            let t = format!("{}{} ({})", if v >= 0 { "+" } else { "" }, hundredths(v), ratings::demand_word(v));
            s.text_centered(g, 356.0, rows[k + 1].0 + 10.0, &t, 12.0, ink);
        }
        // right column: label at 441, values centred at 536 or 591
        s.text(g, 441.0, 93.0, "Yards", 12.0, ink);
        s.text_centered(g, 536.0, 93.0, &hole.length.to_string(), 12.0, ink);
        let rounds = hole.tee_shots;
        if rounds > 0 {
            // the exe's clock picks one of five figures, a new one every 1.024 seconds
            let k = ((self.clock * 1000.0) as u64 >> 10) % 5;
            let on_hole = self.club.g.iter().filter(|g| g.hole == h as i32 && g.strokes > 0).count() as i32;
            let r = (rounds - on_hole).max(1);
            let (label, value) = match k {
                0 => ("Avg. Drive: ", format!("{} yds", hole.drive_sum / rounds)),
                1 => ("Longest Drive", format!("{} yds", hole.drive_max)),
                2 => ("Fairways hit", format!("{}%", hole.fairways * 100 / rounds)),
                3 => ("Greens in Reg", format!("{}%", hole.gir * 100 / r)),
                _ => ("Average Putts", hundredths(hole.putts * 100 / r)),
            };
            s.text(g, 441.0, 114.0, label, 12.0, ink);
            s.text_centered(g, 591.0, 114.0, &value, 12.0, ink);
        }
        s.text(g, 441.0, 135.0, "Par ", 12.0, ink);
        s.text_centered(g, 536.0, 135.0, &hole.par.to_string(), 12.0, ink);
        s.text(g, 441.0, 156.0, "Stroke average", 12.0, ink);
        if let Some(a) = st.avg {
            s.text_centered(g, 591.0, 156.0, &hundredths(a), 12.0, ink);
        }
        s.text(g, 190.0, 188.0, "Average shots on this hole", 12.0, ink);
        for (k, &(v, n)) in st.cols.iter().enumerate() {
            let x = 445.0 + 34.0 * k as f32;
            let label = if k == 5 { format!("{v}+") } else { v.to_string() };
            s.text_centered(g, x, 178.0, &label, 11.0, ink);
            if n != 0 {
                s.text_centered(g, x, 197.0, &n.to_string(), 11.0, ink);
            }
        }
        s.text(g, 190.0, 213.0, "Comments", 12.0, ink);
        let mut y = 220.0;
        if hole.par == 0 {
            s.image_part(g, art, 0.0, y, 0.0, 282.0, 800.0, 16.0);
            if (self.clock * 1000.0) as u64 & 0x200 != 0 {
                s.text_centered(g, 400.0, y + 12.0, "Under Construction!", 12.0, c15(0x7d08));
            }
            y += 16.0;
        }
        for &(e, pct, arg) in &st.comments {
            s.image_part(g, art, 0.0, y, 0.0, 282.0, 800.0, 16.0);
            let line = self.club.thought(&self.course, e, arg, 0x98);
            let c = match line.tone {
                sg_core::thoughts::Tone::Good => c15(0x1284),
                sg_core::thoughts::Tone::Bad => c15(0x7d08),
                _ => ink,
            };
            s.text_centered(g, 400.0, y + 12.0, &format!("{pct}%   '{}'", line.text), 11.0, c);
            y += 16.0;
        }
        s.image_part(g, art, 0.0, y, 0.0, 347.0, 800.0, 55.0);
        self.ok_tick(g, &s, 574.0, y + 8.0, false);
        g.flush();
    }

    // ---- Best Scores ------------------------------------------------------------------------------------------------------

    /// Best N Hole Scores (0x455a30): the ten best full rounds and who played them.
    pub fn draw_best_scores(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        let art = &self.info.art.lowscore;
        dim(g, &s);
        let ink = black();
        s.image_part(g, art, 195.0, 45.0, 195.0, 45.0, 411.0, 79.0);
        let n = (1..19).filter(|&h| self.club.holes[h].par != 0).count();
        s.text_centered(g, 413.0, 76.0, &format!("Best {n} Hole Scores"), 20.0, ink);
        s.text(g, 236.0, 106.0, "Golfer", 12.0, ink);
        s.text_centered(g, 551.0, 106.0, "Score", 12.0, ink);
        let mut y = 124.0;
        for (score, name) in self.club.top_rounds.iter().zip(&self.club.top_names).take_while(|(v, _)| **v != 0) {
            s.image_part(g, art, 195.0, y, 195.0, 224.0, 411.0, 17.0);
            s.text(g, 246.0, y + 12.0, name, 12.0, ink);
            s.text_centered(g, 551.0, y + 12.0, &score.to_string(), 12.0, ink);
            y += 17.0;
        }
        s.image_part(g, art, 195.0, y, 195.0, 321.0, 411.0, 61.0);
        if self.over_ok(544.0, y + 14.0) {
            s.image_part(g, art, 544.0, y + 14.0, 593.0, 434.0, 44.0, 44.0);
        }
        g.flush();
    }

    // ---- Top 10 Designers -------------------------------------------------------------------------------------------------

    fn top10_file(&self) -> std::path::PathBuf {
        save_dir().join("top10.sve")
    }

    /// The table from top10.sve, or the exe's default table (written out) when there is none.
    pub fn load_top10(&mut self) -> Vec<top10::Entry> {
        let f = self.top10_file();
        if let Some(t) = sg_core::fsutil::read_file(&f).and_then(|d| top10::parse(&d)) {
            return t;
        }
        let t = top10::defaults(&mut self.exe_rng);
        sg_core::fsutil::write_file(&f, &top10::to_bytes(&t));
        t
    }

    pub fn open_top10(&mut self, new: Option<usize>) {
        self.info.top10 = self.load_top10();
        self.info.top10_new = new;
        self.screen = Screen::Top10;
        self.hover = -1;
    }

    /// The year end puts this course's result in the table; when it makes the list the table follows the year-end report.
    pub fn top10_year_end(&mut self) {
        if self.club.championship() {
            return;
        }
        let mut t = self.load_top10();
        // the course's id (0x822c78, drawn when the game began); a game saved before the id was kept falls back on a hash of
        // its name, which only has to tell this course from the others in the table
        let id = self.course_id.unwrap_or_else(|| {
            self.course_name.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193)) as i32 & 0x7fff_ffff
        });
        let e = top10::Entry {
            name: self.pro_name(),
            course: self.course_name.clone(),
            fun: self.club.ratings.fun,
            skill: self.club.ratings.skill,
            cash: (self.econ.cash / sg_core::economy::Economy::UNIT) as i32,
            extra: 0,
            difficulty: self.difficulty.clamp(0, 3) as i16,
            course_id: id,
        };
        if let Some(rank) = top10::insert(&mut t, e) {
            sg_core::fsutil::write_file(self.top10_file(), &top10::to_bytes(&t));
            self.info.top10_due = Some(rank);
        }
    }

    /// Top 10 Designers (0x473470): the shelf, a trophy for each rank held, the designer's name on its plate and the
    /// numbers on the cup.
    pub fn draw_top10(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        let a = &self.info.art;
        s.fill(g, 0.0, 0.0, 800.0, 600.0, black());
        s.image(g, &a.top10_blank, 0.0, 0.0);
        for (rank, e) in self.info.top10.iter().enumerate().take(top10::COUNT) {
            let slot = top10::RANK_SLOT[rank];
            let (col, row) = ((slot % 5) as f32, slot / 5);
            let (cy, ch) = if row == 0 { (40.0, 304.0) } else { (344.0, 256.0) };
            s.image_part(g, &a.top10_cups, col * 160.0, cy, col * 160.0, cy, 160.0, ch);
            let x0 = col * 160.0 + 80.0;
            // the name plate: drawn dark, light and then in its colour, white for the new entry, which is underlined
            let ny = if row == 0 { 321.0 } else { 574.0 } + 11.0;
            let new = self.info.top10_new == Some(rank);
            s.text_centered(g, x0, ny, &e.name, 12.0, black());
            s.text_centered(g, x0, ny + 2.0, &e.name, 12.0, c15(0x7ff0));
            s.text_centered(g, x0, ny + 1.0, &e.name, 12.0, c15(if new { 0x7fff } else { 0x4206 }));
            if new {
                let w = text_width(&e.name, 12.0) + 4.0;
                s.fill(g, x0 - w / 2.0, ny + 3.0, w, 1.0, c15(0x4200));
                s.fill(g, x0 - w / 2.0, ny + 4.0, w, 1.0, c15(0x7ff0));
            }
            let r = rank as f32;
            let (cash_y, p) = if row == 0 { (164.0 + 10.0 * r, 12.0) } else { (443.0 + 4.0 * r, 10.0) };
            let ink = c15(0x4206);
            let fun_y = cash_y - 2.0 * p;
            s.text_centered(g, x0, cash_y + 9.0, &format!("Cash: {}", crate::ui::money(e.cash as i64 * 100)), 10.0, ink);
            s.text_centered(g, x0, cash_y - p + 9.0, &format!("Skill: {}", hundredths(e.skill)), 10.0, ink);
            s.text_centered(g, x0, fun_y + 9.0, &format!("Fun: {}", e.fun), 10.0, ink);
            let total_y = fun_y - if row == 0 { 8.0 + p } else { 2.0 + p } + 6.0;
            let total = e.score().to_string();
            s.text_centered(g, x0, total_y - 1.0, &total, 16.0, black());
            s.text_centered(g, x0, total_y + 1.0, &total, 16.0, c15(0x7ff0));
            s.text_centered(g, x0, total_y, &total, 16.0, c15(0x5288));
            s.text_centered(g, x0, total_y - 16.0, &format!("Total Score (x{})", e.difficulty + 1), 10.0, ink);
            if rank == 0 {
                s.text_centered(g, x0, total_y - 30.0, &e.course, 10.0, ink);
            }
        }
        g.flush();
    }
}
