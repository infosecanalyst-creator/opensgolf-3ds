//! The main screen's messages, as the publisher's golf.exe shows them:
//!
//! * the ticker (posted by 0x40cb00, stepped and drawn once a frame by 0x40d6a0, drawn by 0x40d320): one message at a time
//!   at the top of the screen. With nobody speaking it is a translucent strip hanging from the top edge with a rail along
//!   its bottom, which slides open and shut over three frames; with a speaker it is the translucent dialog frame with the
//!   speaker's ball portrait, a building's picture or a PopUpIcons piece on its left;
//! * the advisor's line over the player's pro (main frame 0x416305 to 0x416835): Easy only, in the first year;
//! * the "Paused" caption (main frame 0x419956).
//!
//! Every value below is from the decompile unless marked DERIVED or PLACEHOLDER.

use crate::app::*;
use crate::gfx::Gfx;
use crate::screens_ui::{top, BODY, LARGE, SMALL};
use crate::ui::{rgb, rgba, text_width, wrap_text, Screen as Ui};
use sg_core::staff;

/// Where the ticker is drawn (0x40d6a0 passes (0xf0, -8) to 0x40d320).
const X: f32 = 240.0;
const Y: f32 = -8.0;
/// Text width of the box (0x1a0 = 416) and the line pitch of its height sum (15 per line plus 0x20).
const BOX_W: f32 = 416.0;
const LINE: f32 = 15.0;
/// The ticker font (0x51b360): the body face. Its pixel size is not in the decompile (PLACEHOLDER: the port's body size).
const FONT: f32 = BODY;
/// Speaker codes (0x4c2e08).
pub const NOBODY: i32 = -1;
const FRAME_S: f64 = sg_core::flight::TICK_MS as f64 / 1000.0;

/// The ticker's state (the exe's globals in brackets).
#[derive(Clone, Debug)]
pub struct Ticker {
    /// The message text (0x5a6d40); it stays after the message is gone, for Repeat Last Message.
    pub text: String,
    /// Who says it (0x4c2e08): see `sg_core::golfer::Event::Message`.
    pub speaker: i32,
    /// A message is up (0x569498).
    pub showing: bool,
    /// Frames before a delayed message appears (0x5694a4, set from a negative priority).
    pub delay: i32,
    /// Countdown to its end (0x5a7144).
    pub left: i32,
    /// The strip's opening state 0..3 (0x5a9ccc).
    pub slide: i32,
    /// What this frame draws: None nothing, Some(-1) the whole box, Some(1..3) the strip opened that many quarters.
    pub frame: Option<i32>,
    /// Frames counted, for the countdown that skips every third one (the exe tests its game tick).
    pub frames: u64,
    /// Wall clock of the last frame stepped while the game stands still.
    pub wall: f64,
}

impl Default for Ticker {
    fn default() -> Self {
        Ticker { text: String::new(), speaker: NOBODY, showing: false, delay: 0, left: 0, slide: 0, frame: None, frames: 0, wall: 0.0 }
    }
}

impl Ticker {
    /// 0x40cb00: a priority below 1 is refused while a message is up or the golfer card is open (`card`); otherwise the
    /// message replaces whatever is showing. A negative priority also delays it by that many frames. It stays for
    /// 16 frames plus one per 3 characters (per 4 above Easy), counted on two frames in three.
    pub fn post(&mut self, text: &str, priority: i32, speaker: i32, difficulty: i32, card: bool) -> bool {
        if (self.showing || card) && priority < 1 {
            return false;
        }
        // (the exe also refuses everything in its dock mode 3, which the port has no counterpart for)
        self.text = text.to_string();
        self.showing = true;
        self.speaker = speaker;
        self.left = text.len() as i32 / if difficulty != 0 { 4 } else { 3 } + 16;
        if priority < 0 {
            self.delay = -priority;
        }
        true
    }

    /// The ticker is taken, for the game's own priority 0 messages.
    pub fn busy(&self) -> bool {
        self.showing
    }

    /// '?' and Repeat Last Message (main frame key 0x3f): the last text comes back for 0x80 frames, said by the same speaker.
    pub fn repeat(&mut self) {
        if !self.text.is_empty() {
            self.showing = true;
            self.left = 0x80;
        }
    }

    /// One frame of 0x40d6a0 (run only while a message is up or the strip is still closing).
    pub fn step(&mut self) {
        self.frame = None;
        if !self.showing && self.slide == 0 {
            return;
        }
        if self.delay > 0 {
            self.delay -= 1;
            return;
        }
        let arg = if self.showing {
            if self.slide < 3 {
                self.slide += 1;
                self.slide
            } else {
                -1
            }
        } else {
            self.slide -= 1;
            self.slide + 1
        };
        self.frame = Some(arg);
        // (the exe holds the countdown while its game flag 0x8000 is set, which the port never sets)
        self.frames += 1;
        if !self.frames.is_multiple_of(3) {
            self.left -= 1;
            if self.left < 1 {
                self.showing = false;
            }
        }
    }

    /// While the game stands still (paused) the exe's frames go on: step at the frame rate of the wall clock.
    pub fn step_wall(&mut self, clock: f64) {
        if clock - self.wall > 1.0 {
            self.wall = clock;
        }
        while clock - self.wall >= FRAME_S {
            self.wall += FRAME_S;
            self.step();
        }
    }
}

/// The text broken into lines the way 0x45af30 does: at '\n' and '^', and before a word that would pass `width` pixels.
fn lines(text: &str, width: f32) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.replace('_', " ").split(['\n', '^']) {
        let w = wrap_text(para, FONT, width);
        if w.is_empty() {
            out.push(String::new());
        } else {
            out.extend(w);
        }
    }
    out
}

/// 0x40cef0 and 0x40cdd0 round a size that is not a multiple of 16 up to the next 15 mod 16 and move the box back by half
/// of what they added.
fn round16(pos: f32, size: f32) -> (f32, f32) {
    let r = size as i32 & 15;
    if r == 0 {
        (pos, size)
    } else {
        (pos - ((15 - r) / 2) as f32, size + (15 - r) as f32)
    }
}

/// Employee first names (pointer table 0x4c148c), by job and upgrade like the titles; the last has no space in the exe.
const FIRST_NAMES: [&str; 8] = ["Chuck ", "ChiChi ", "Randy ", "Mike ", "Joe ", "Tommy ", "Sally ", "Rita"];

impl App {
    /// An employee's name (0x467600): the first name and the title by job and experience ("Chuck Club Pro"; the last
    /// first name has no space in the exe), or the pro's own name.
    pub fn employee_name(&self, e: &staff::Employee) -> String {
        if e.job == staff::job::OWNER {
            return self.pro_name();
        }
        let job = (-2 - e.job as i32).clamp(0, 3) as usize;
        format!("{}{}", FIRST_NAMES[job * 2 + e.upgraded as usize], STAFF_NAMES[job][e.upgraded as usize])
    }

    /// Posts a ticker message (0x40cb00); see `Ticker::post`.
    pub fn post_message(&mut self, text: &str, priority: i32, speaker: i32) -> bool {
        let card = self.card.is_some();
        let ok = self.ticker.post(text, priority, speaker, self.difficulty, card);
        if ok {
            self.last_message = text.to_string();
        }
        ok
    }

    /// Repeat Last Message ('?' and the Information menu). With a golfer card open the exe's '?' opens Customise instead,
    /// which the port leaves to the card.
    pub fn repeat_message(&mut self) {
        if self.card.is_none() {
            self.ticker.repeat();
        }
    }

    /// The ticker box (0x40d320) for this frame.
    pub fn draw_ticker(&mut self, g: &mut Gfx, s: &Ui) {
        if self.paused {
            self.ticker.step_wall(self.clock);
        } else {
            self.ticker.wall = self.clock;
        }
        let Some(arg) = self.ticker.frame else { return };
        let t = &self.ticker;
        let speaker = t.speaker;
        let talk = speaker != NOBODY;
        // the text is wrapped to 0x25 * 8 pixels beside a speaker, 0x2c * 8 without
        let rows = lines(&t.text, if talk { 0x25 as f32 * 8.0 } else { 0x2c as f32 * 8.0 });
        let h0 = rows.len() as f32 * LINE + 32.0;
        let (mut x, mut y, mut h, mut arg) = (X, Y, h0, arg);
        if talk {
            h = h.max(96.0);
            x += 28.0;
            y += 44.0;
            arg = -2;
        }
        if arg >= 0 {
            // the strip opening or closing: a quarter of its height per frame, no text
            self.strip(g, s, x - 8.0, y - 8.0, BOX_W, ((h * arg as f32) / 4.0).floor());
            return;
        }
        if arg == -2 {
            let (bx, bw) = round16(x - 8.0, BOX_W);
            let (by, bh) = round16(y - 8.0, h + 16.0);
            self.art.trans_frame(g, s, bx, by, bw, bh);
        } else {
            self.strip(g, s, x - 8.0, y - 8.0, BOX_W, h + 16.0);
        }
        let white = rgb(1.0, 1.0, 1.0);
        if !talk {
            for (i, l) in rows.iter().enumerate() {
                s.text(g, x + 12.0, top(y + 16.0 + LINE * i as f32, FONT), l, FONT, white);
            }
            return;
        }
        self.speaker_picture(g, s, speaker, x, y);
        let mut ty = y + 16.0;
        if h0 < 96.0 {
            ty += ((96.0 - h0 - 16.0) / 2.0).trunc();
        }
        for (i, l) in rows.iter().enumerate() {
            s.text(g, x + 100.0, top(ty + LINE * i as f32, FONT), l, FONT, white);
        }
    }

    /// The ticker strip (0x40cdd0): the translucent fill (0x40ca10, its 16 x 16 shading tile darkens by half, DERIVED as for
    /// the dialog frame) and the rail along the bottom: its left end at x - 4, the middle piece every 16 pixels from x + 12,
    /// the right end at x + w - 12. The rail pieces are elements 12 to 14 of the TransPopups group, DERIVED to be the three
    /// 16 x 16 cuts at (300 + 17k, 34), whose shapes are a left curl, a bar and a right curl.
    fn strip(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
        let (x, w) = round16(x, w);
        let (y, h) = round16(y, h);
        s.fill(g, x, y - 1.0, w, h, rgba(0.0, 0.0, 0.0, 0.5));
        let im = &self.art.trans;
        if im.tex.is_none() {
            return;
        }
        let by = y + h - 16.0;
        let mut cx = 12.0;
        while cx < w - 16.0 {
            s.image_part(g, im, x + cx, by, 317.0, 34.0, 16.0, 16.0);
            cx += 16.0;
        }
        s.image_part(g, im, x + w - 12.0, by, 334.0, 34.0, 16.0, 16.0);
        s.image_part(g, im, x - 4.0, by, 300.0, 34.0, 16.0, 16.0);
    }

    /// The speaker's picture left of the text; (x, y) is the box origin after the speaker offset.
    fn speaker_picture(&self, g: &mut Gfx, s: &Ui, speaker: i32, x: f32, y: f32) {
        if speaker >= 0 {
            // the speaker's ball portrait: their head, gender and newest reaction (0x45c200 ball mode)
            let gi = speaker as usize;
            if let Some(gg) = self.club.g.get(gi) {
                let id = gg.roster.max(0) as usize;
                let p = self.club.roster.get(id).cloned().unwrap_or_default();
                self.art.ball_head(g, s, p.female(), p.head_index(id), self.club.attitude(gi) as usize, x - 50.0, y - 40.0);
            }
        } else if (-14..=-6).contains(&speaker) {
            // a building kind's picture (the sprite object 0x53df84 + 0xb0 * (kind - 6)): DERIVED to be the first cut of the
            // theme layout sheet's column for the kind; the colour cut (y 200) is a PLACEHOLDER choice among its four
            let col = (-speaker - 6) as f32;
            let im = &self.panel_art.layouts[(self.exe_theme() as usize).min(3)];
            s.image_part(g, im, x + 8.0, y - 20.0, 75.0 * col, 200.0, 75.0, 100.0);
        } else if speaker == -5 {
            s.image_part(g, &self.art.trans, x - 40.0, y - 40.0, 0.0, 300.0, 140.0, 140.0);
        } else if speaker == -4 {
            // the club emblem of the course's site (array 0x5791f8 by the site record's first byte)
            if let Some(&(sheet, cut)) = self.land.as_ref().and_then(|l| crate::files_ui::EMBLEM.get(l.slot.property)) {
                let sy = if sheet == 0 { 396.0 } else { 481.0 };
                s.image_part(g, &self.title.art.emblems[sheet], x + 8.0, y, 4.0 + 88.0 * cut as f32, sy, 80.0, 80.0);
            }
        } else {
            // the PopUpIcons pieces (90 x 80 at x = 90 k): -20 trophy, -3 star, -2 book, -22 exclamation, -21 laurel ball
            let k = match speaker {
                -20 => 0,
                -3 => 1,
                -2 => 2,
                -22 => 3,
                -21 => 4,
                _ => return,
            };
            s.image_part(g, &self.art.popup_icons, x, y + 16.0, 90.0 * k as f32, 0.0, 90.0, 80.0);
        }
    }

    /// The advisor's line for this frame (main frame 0x416305): Easy, the first year (ticks below 0x2000), the player's pro
    /// out on the course and on screen. Returns the text and the pro's map position.
    fn advisor_line(&self) -> Option<(String, i32, i32)> {
        let tick = self.game_tick;
        if self.difficulty != 0 || tick >= 0x2000 {
            return None;
        }
        // the exe reads staff record 0 (its own pro) and needs its byte +0x15 clear (PLACEHOLDER: read as "not going home")
        let pro = self.employees.iter().find(|e| e.job == staff::job::OWNER).filter(|e| e.active && !e.going_home)?;
        // the exe's dock mode: 0 Build Course, 1 Add Buildings, 2 People (the port's closed dock and Golfers panel: neither)
        let mode = match self.panel {
            1 => 0,
            2 => 1,
            3 => 2,
            _ => 3,
        };
        let holes = &self.club.holes;
        let h1 = &holes[1];
        let (tee, green) = (h1.back.0 != 0, h1.pin.0 != 0);
        let mut t = String::new();
        if h1.par == 0 {
            t = "Press 'h' to open the hole.".into();
            if !green {
                t = "Now we need to build a green.".into();
            }
            if !tee {
                t = "First we need to build a tee.".into();
            }
            if tick & 0x20 == 0 {
                t = "Let's build our first golf hole.".into();
                if tee && green {
                    t = "We might add some fairway or sand traps.".into();
                }
            } else if mode != 0 {
                t = "Click on the big 'Build Course' button.".into();
            }
            // shown on three frames in four of each 32
            return (tick & 0x18 != 0).then_some((t, pro.x, pro.y));
        }
        if holes[2].par == 0 {
            t = "We've built one hole, let's build more!".into();
            if self.club.g[1].hole == 1 && self.club.g[1].strokes == 0 {
                t = "Here come some golfers!".into();
            }
        }
        if holes[3].par != 0 {
            // the hole nearest the pro (0x407340): by its back tee, its pin and its last yardage marker
            let (px, py) = (pro.x >> 10, pro.y >> 10);
            let d = |dx: i32, dy: i32| ((dx * dx + dy * dy) as f32).sqrt() as i32;
            let (mut best, mut h) = (0xffff, 1usize);
            for (k, hr) in holes.iter().enumerate().take(19).skip(1) {
                let mut ds = vec![d(hr.back.0 - px, hr.back.1 - py), d(hr.pin.0 - px, hr.pin.1 - py)];
                if hr.markers[2].0 != -1 {
                    ds.push(d(px - (hr.markers[2].0 >> 10), py - (hr.markers[2].1 >> 10)));
                }
                for v in ds {
                    if v < best {
                        best = v;
                        h = k;
                    }
                }
            }
            let hr = &holes[h];
            let pct = hr.mood_sum * 100 / (hr.plans / 2 + hr.tee_shots + 4);
            t = format!("This hole has a {pct}% fun rating.");
            if self.club.pro_mask == 0 {
                t = "I'm ready for a practice round.".into();
            } else if self.club.game & sg_core::tournament::OFFERED != 0 {
                t = "I'm ready to play a tournament.".into();
            }
        }
        if tick & 0x40 != 0 {
            match mode {
                0 if !self.pstate.alt => t = "I like to make each hole different.".into(),
                0 if tick & 0x80 != 0 => t = "Some flowers and benches will cheer our golfers.".into(),
                0 => t = "Adding paths will help speed up play.".into(),
                1 if self.pstate.alt => t = "Slopes affect the bounce and roll of the ball.".into(),
                1 if self.difficulty != 0 => t = "Buildings need a path to the clubhouse.".into(),
                1 => t = "Buildings are one way to improve our course.".into(),
                2 => {
                    // staff record 1, the first employee hired: its first name and title (0x467600)
                    t = match self.employees.get(1).filter(|e| e.active && e.job != staff::job::OWNER) {
                        Some(e) => {
                            format!("{} is doing a great job.", self.employee_name(e))
                        }
                        None => "If you hire employees I won't have to do everything myself!".into(),
                    }
                }
                _ => {}
            }
        }
        if tick & 0x20 != 0 || self.club.gary != -1 || t.is_empty() {
            return None;
        }
        Some((t, pro.x, pro.y))
    }

    /// The advisor's line over the pro: `zoom * 10` pixels above him, white, centred on a black line 10 pixels thick and
    /// the text's width plus 8 long (Terrain::drawLine); the body font above the exe's zoom 4, the small one otherwise.
    /// The sprite the exe adds at (x + 4, y + 10) (object 0x5a4100) is not identified and is left out.
    pub fn draw_advisor(&self, g: &mut Gfx, s: &Ui) {
        let Some((text, mx, my)) = self.advisor_line() else { return };
        let Some((x, y)) = self.screen_of(mx, my) else { return };
        let exe_zoom = self.zoom * 4.0 / 0.905;
        let size = if exe_zoom > 4.0 { BODY } else { SMALL };
        let y0 = (y - exe_zoom.round() * 10.0).round();
        let half = ((text_width(&text, size) + 8.0) / 2.0).floor();
        s.fill(g, x - half, y0, 2.0 * half, 10.0, rgba(0.0, 0.0, 0.0, 1.0));
        s.text_centered(g, x, top(y0, size), &text, size, rgb(1.0, 1.0, 1.0));
    }

    /// "Paused" centred at (400, 10) in white (font 0x519948, its size a PLACEHOLDER: the port's large face), with " Fast"
    /// after it while the exe's fast flag (0x59b04c) is on, here while the game runs above normal speed.
    pub fn draw_paused(&self, g: &mut Gfx, s: &Ui) {
        let mut t = String::new();
        if self.paused {
            t += "Paused";
        }
        if self.speed > 1 {
            t += " Fast";
        }
        if !t.is_empty() {
            s.text_centered(g, 400.0, top(10.0, LARGE), &t, LARGE, rgb(1.0, 1.0, 1.0));
        }
    }
}
