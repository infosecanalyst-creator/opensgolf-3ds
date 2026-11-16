//! The course review (the port's own; the Information menu's line, with the comments by hole on, extras.rs
//! `hole_comments`): one page of what to look at next, put
//! together from what the other helpers already know. Buildings off the paths and holes with no path near the tee (the
//! path view's marks), the course tiles no groundskeeper reaches (the staff areas'), the holes golfers find dull (Hole
//! Stats' fun factor) and each hole's commonest complaint this year (the comments by hole). Each item is a line with a
//! mark: a tick when all is well, an exclamation mark when there is something to do. It only reads; nothing changes.
use crate::app::*;
use crate::extras::{draw_arrow, draw_button, wrap};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{Fnt, Screen as Ui, F_INFO14, F_MANUAL20};
use sg_core::lang::{tr, trf};

const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
const TOP_Y: f32 = BOX.y + 50.0;
const BOTTOM_Y: f32 = BOX.y + BOX.h - 66.0;
const MARK_X: f32 = BOX.x + 24.0;
const TEXT_X: f32 = BOX.x + 56.0;
const TEXT_W: f32 = BOX.w - 120.0;
const UP: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: TOP_Y, w: 20.0, h: 20.0 };
const DOWN: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: BOTTOM_Y - 20.0, w: 20.0, h: 20.0 };
/// A hole whose fun factor is under this is called dull (Hole Stats' "poor" and below).
const DULL: i32 = 40;

fn font() -> Fnt {
    crate::textsize::big(F_INFO14)
}
fn line() -> f32 {
    crate::textsize::px(16.0)
}
fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

/// One item of the review: well (a tick) or to do (an exclamation mark), and its words.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub ok: bool,
    pub text: String,
}

/// "1, 4 and 7": hole numbers or names as a list.
fn list(v: &[String]) -> String {
    crate::screens_ui::and_list(&v.iter().map(|s| s.as_str()).collect::<Vec<_>>())
}

impl App {
    /// The Information menu's line: the review (closing to the course).
    pub fn open_review(&mut self) {
        self.screen = Screen::Review;
        self.xs.review_off = 0;
        self.hover = -1;
    }

    fn close_review(&mut self) {
        self.screen = Screen::Play;
        self.hover = -1;
    }

    /// The review's items, worked out now.
    pub fn review_items(&self) -> Vec<Item> {
        let mut out = Vec::new();
        let holes: Vec<usize> = (1..19).filter(|&h| self.club.holes[h].par != 0).collect();
        if holes.is_empty() {
            out.push(Item { ok: false, text: tr("STR_REVIEW_NO_HOLES").to_string() });
            return out;
        }
        // the paths (the path view's marks: buildings off the paths, then holes with no path near the tee)
        let marks = self.path_marks();
        let buildings: Vec<String> = self
            .course
            .objects
            .iter()
            .skip(1)
            .filter(|o| (6..=14).contains(&o.kind) && o.flags & 0x40 == 0)
            .map(|o| sg_core::land::building_name(o.kind).to_string())
            .collect();
        out.push(if buildings.is_empty() {
            Item { ok: true, text: tr("STR_REVIEW_BUILDINGS_OK").to_string() }
        } else {
            Item { ok: false, text: trf("STR_REVIEW_BUILDINGS_OFF", &[("LIST", &list(&buildings))]) }
        });
        let reach = |h: usize| {
            let (tx, ty) = self.club.holes[h].back;
            marks.tiles.iter().any(|&((x, y), j)| j && (x - tx).abs() <= 3 && (y - ty).abs() <= 3)
        };
        let far: Vec<String> = holes.iter().filter(|&&h| !reach(h)).map(|h| h.to_string()).collect();
        out.push(if far.is_empty() {
            Item { ok: true, text: tr("STR_REVIEW_TEES_OK").to_string() }
        } else {
            Item { ok: false, text: trf("STR_REVIEW_TEES_FAR", &[("LIST", &list(&far))]) }
        });
        // upkeep (the staff areas')
        if self.land.is_some() {
            let bare = self.staff_marks().bare.len();
            out.push(if bare == 0 {
                Item { ok: true, text: tr("STR_REVIEW_KEPT_OK").to_string() }
            } else {
                Item { ok: false, text: trf("STR_REVIEW_KEPT_BARE", &[("COUNT", &bare)]) }
            });
        }
        // fun (Hole Stats')
        let dull: Vec<String> = holes
            .iter()
            .filter_map(|&h| sg_core::ratings::hole_stats(&self.club.holes[h]).fun.filter(|&f| f < DULL).map(|f| format!("{h} ({f}%)")))
            .collect();
        out.push(if dull.is_empty() {
            Item { ok: true, text: tr("STR_REVIEW_FUN_OK").to_string() }
        } else {
            Item { ok: false, text: trf("STR_REVIEW_FUN_DULL", &[("LIST", &list(&dull))]) }
        });
        // each hole's commonest complaint this year (the comments by hole)
        let tally = self.tally_now();
        let mut heard = false;
        for &h in &holes {
            let worst = tally.top(h as u8, crate::insight::THIS_YEAR, 8).into_iter().find_map(|(id, n, arg)| {
                let (text, tone) = self.thought_label(id, arg);
                (tone == sg_core::thoughts::Tone::Bad && !text.is_empty()).then_some((text, n))
            });
            if let Some((text, n)) = worst {
                heard = true;
                out.push(Item { ok: false, text: trf("STR_REVIEW_COMPLAINT", &[("HOLE", &h), ("WHAT", &text), ("COUNT", &n)]) });
            }
        }
        if !heard {
            out.push(Item { ok: true, text: tr("STR_REVIEW_NO_COMPLAINTS").to_string() });
        }
        out
    }

    fn review_scroll(&mut self, d: i32) {
        let n = self.review_items().len() as i32;
        self.xs.review_off = (self.xs.review_off as i32 + d).clamp(0, (n - 1).max(0)) as usize;
    }

    pub fn review_wheel(&mut self, y: f32) {
        self.review_scroll(if y > 0.0 { -1 } else { 1 });
    }

    pub fn review_click(&mut self, x: f32, y: f32, right: bool) {
        if right || ok_tick().has(x, y) || !BOX.has(x, y) {
            return self.close_review();
        }
        if UP.has(x, y) {
            self.review_scroll(-1);
        } else if DOWN.has(x, y) {
            self.review_scroll(1);
        }
    }

    pub fn review_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        match k {
            KeyCode::Up => self.review_scroll(-1),
            KeyCode::Down => self.review_scroll(1),
            KeyCode::PageUp => self.review_scroll(-8),
            KeyCode::PageDown | KeyCode::Space => self.review_scroll(8),
            KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => self.close_review(),
            _ => {}
        }
    }

    pub fn draw_review(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(&mut *g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, yellow, navy, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x6318));
        let (good, bad) = (c15(0x01e0), c15(0x5800));
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_REVIEW_TITLE"), yellow, true);
        let items = self.review_items();
        let mut y = TOP_Y;
        let mut shown = 0;
        for it in items.iter().skip(self.xs.review_off) {
            let ls = wrap(font(), &it.text, TEXT_W);
            let h = line() * ls.len() as f32 + 6.0;
            if y + h > BOTTOM_Y && shown > 0 {
                break;
            }
            s.put(g, font(), MARK_X, y, if it.ok { "OK" } else { "!" }, if it.ok { good } else { bad });
            for (k, l) in ls.iter().enumerate() {
                s.put(g, font(), TEXT_X, y + line() * k as f32, l, if it.ok { navy } else { white });
            }
            y += h;
            shown += 1;
        }
        let (px, py) = self.info.pointer;
        let (can_up, can_down) = (self.xs.review_off > 0, self.xs.review_off + shown < items.len());
        for (r, up, ok) in [(UP, true, can_up), (DOWN, false, can_down)] {
            draw_button(g, &s, r, "", ok, r.has(px, py));
            draw_arrow(g, &s, r.x + r.w / 2.0, r.y + r.h / 2.0, up, if ok { navy } else { grey });
        }
        let room = ok_tick().x - 6.0 - (BOX.x + 24.0);
        let help = tr("STR_REVIEW_HELP");
        s.put_fit(g, crate::textsize::fit(F_INFO14, 1.25, help, room), BOX.x + 24.0, BOX.y + BOX.h - 28.0, room, help, navy);
        if self.art.trans.tex.is_some() {
            let r = ok_tick();
            s.image_part(g, &self.art.trans, r.x, r.y, 350.0, 140.0, 50.0, 50.0);
        } else {
            draw_button(g, &s, ok_tick(), "OK", true, ok_tick().has(px, py));
        }
        g.flush();
    }
}
