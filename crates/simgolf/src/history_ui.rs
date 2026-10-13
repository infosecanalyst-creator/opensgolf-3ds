//! OpenSGolf extras: the message history (extras.rs). Every message the ticker shows is kept with the game date, who said
//! it (the golfer's ball portrait, or the ticker's picture for the club's own messages) and the place it is about when
//! known; the history keeps the last thousand and is saved with the game. The screen lists them newest first in the look of
//! the port's other own screens (keys_ui: the System Functions box's frame and fill, its fonts and colours, the OK tick);
//! a click on a message with a place (or a golfer still on the course) closes the screen with the camera there.
//!
//! Opened by Shift+M, the Information menu's "Message history" line, or a click on the ticker. Keys: Up, Down, PageUp,
//! PageDown, Home, End and the mouse wheel scroll; Esc, Enter, a right click or the OK tick close it.
use crate::app::*;
use crate::extras::{draw_arrow, draw_button, wrap, Message};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{rgba, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use sg_core::lang::{tr, trf};

/// The box: the Keyboard controls screen's.
const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
const TOP: f32 = BOX.y + 58.0;
const BOTTOM: f32 = BOX.y + BOX.h - 62.0;
/// The text's left edge (after the portrait) and its width.
const TEXT_X: f32 = BOX.x + 64.0;
const TEXT_W: f32 = 540.0;
const LINE: f32 = 15.0;
/// The scroll arrows, right of the list.
const UP: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: TOP, w: 20.0, h: 20.0 };
const DOWN: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: BOTTOM - 20.0, w: 20.0, h: 20.0 };

fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

/// The screen's state: the newest message shown at the top (0 the newest of all) and the screen to go back to.
#[derive(Default)]
pub struct HistoryUi {
    pub off: usize,
    pub back: Option<Screen>,
}

/// A message laid out: its index in the history, its top and height, and its lines.
struct Row {
    i: usize,
    y: f32,
    h: f32,
    lines: Vec<String>,
}

fn row_lines(m: &Message) -> Vec<String> {
    m.text.split('\n').flat_map(|p| wrap(F_MANUAL15, &p.replace(['^', '_'], " "), TEXT_W)).collect()
}

fn row_height(lines: usize) -> f32 {
    (14.0 + LINE * lines as f32).max(36.0) + 6.0
}

impl App {
    pub fn open_history(&mut self) {
        if self.screen != Screen::History {
            self.xs.history.back = Some(if self.screen == Screen::Popup { Screen::Play } else { self.screen });
        }
        self.xs.history.off = 0;
        self.screen = Screen::History;
        self.hover = -1;
    }

    fn close_history(&mut self) {
        self.screen = self.xs.history.back.take().unwrap_or(Screen::Play);
        if !matches!(self.screen, Screen::People | Screen::Profile) {
            self.screen = Screen::Play;
        }
        self.hover = -1;
    }

    /// The messages on screen from the current offset, newest first.
    fn history_rows(&self) -> Vec<Row> {
        let all = &self.xs.kept.messages;
        let mut rows = Vec::new();
        let mut y = TOP;
        for i in (0..all.len()).rev().skip(self.xs.history.off) {
            let lines = row_lines(&all[i]);
            let h = row_height(lines.len());
            if y + h > BOTTOM && !rows.is_empty() {
                break;
            }
            rows.push(Row { i, y, h, lines });
            y += h;
        }
        rows
    }

    /// The camera's target for a message: the golfer when still on the course, else the place kept.
    fn message_place(&self, m: &Message) -> Option<(i32, i32)> {
        self.golfer_of(m.roster).map(|g| (self.club.g[g].x, self.club.g[g].y)).or(m.at)
    }

    fn history_scroll(&mut self, d: i32) {
        let n = self.xs.kept.messages.len() as i32;
        self.xs.history.off = (self.xs.history.off as i32 + d).clamp(0, (n - 1).max(0)) as usize;
    }

    pub fn history_wheel(&mut self, y: f32) {
        self.history_scroll(if y > 0.0 { -2 } else { 2 });
    }

    pub fn history_click(&mut self, x: f32, y: f32, right: bool) {
        if right || ok_tick().has(x, y) || !BOX.has(x, y) {
            return self.close_history();
        }
        if UP.has(x, y) {
            return self.history_scroll(-3);
        }
        if DOWN.has(x, y) {
            return self.history_scroll(3);
        }
        let rows = self.history_rows();
        if let Some(r) = rows.iter().find(|r| (r.y..r.y + r.h).contains(&y) && x < UP.x) {
            let m = self.xs.kept.messages[r.i].clone();
            if let Some(at) = self.message_place(&m) {
                self.xs.history.back = None;
                self.close_history();
                self.look_at(at);
                if let Some(g) = self.golfer_of(m.roster) {
                    self.card = Some(g);
                }
            } else {
                self.ui_sound(0x18);
            }
        }
    }

    pub fn history_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let page = self.history_rows().len().max(1) as i32;
        match k {
            KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => self.close_history(),
            KeyCode::Up => self.history_scroll(-1),
            KeyCode::Down => self.history_scroll(1),
            KeyCode::PageUp => self.history_scroll(-page),
            KeyCode::PageDown | KeyCode::Space => self.history_scroll(page),
            KeyCode::Home => self.xs.history.off = 0,
            KeyCode::End => self.history_scroll(i32::MAX / 2),
            KeyCode::M if self.shift_held => self.close_history(),
            _ => {}
        }
    }

    pub fn draw_history(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(&mut *g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, yellow, navy, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x6318));
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_HISTORY_TITLE"), yellow, true);
        let n = self.xs.kept.messages.len();
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, &trf("STR_HISTORY_HELP", &[("COUNT", &n)]), navy);
        if n == 0 {
            s.put_centered(g, F_MANUAL15, 400.0, 280.0, tr("STR_HISTORY_EMPTY"), white);
        }
        let (px, py) = self.info.pointer;
        let rows = self.history_rows();
        for r in &rows {
            let m = &self.xs.kept.messages[r.i];
            let place = self.message_place(m).is_some();
            if place && (r.y..r.y + r.h).contains(&py) && (BOX.x..UP.x).contains(&px) {
                s.fill(g, BOX.x + 16.0, r.y - 2.0, UP.x - BOX.x - 24.0, r.h - 2.0, rgba(1.0, 1.0, 1.0, 0.18));
            }
            // who said it: a golfer's ball portrait, else the ticker's own picture shrunk
            if m.roster >= 0 {
                self.small_head(g, &s, m.roster, m.face as usize, BOX.x + 20.0, r.y, 36.0);
            } else if m.speaker != crate::message_ui::NOBODY {
                self.small_picture(g, &s, m.speaker, BOX.x + 20.0, r.y, 36.0);
            }
            s.put(g, F_ARIAL10, TEXT_X, r.y, &crate::extras::date(m.tick), navy);
            if place {
                let t = tr("STR_HISTORY_SHOW");
                s.put(g, F_ARIAL10, UP.x - 12.0 - F_ARIAL10.width(t), r.y, t, navy);
            }
            for (k, l) in r.lines.iter().enumerate() {
                s.put(g, F_MANUAL15, TEXT_X, r.y + 12.0 + LINE * k as f32, l, white);
            }
            s.fill(g, BOX.x + 20.0, r.y + r.h - 4.0, UP.x - BOX.x - 32.0, 1.0, rgba(0.0, 0.0, 0.3, 0.25));
        }
        // the scroll arrows and where the page is in the list
        let can_up = self.xs.history.off > 0;
        let can_down = rows.last().is_some_and(|r| r.i > 0);
        for (b, up, ok) in [(UP, true, can_up), (DOWN, false, can_down)] {
            draw_button(g, &s, b, "", ok, b.has(px, py));
            draw_arrow(g, &s, b.x + b.w / 2.0, b.y + b.h / 2.0, up, if ok { navy } else { grey });
        }
        if n > 0 {
            let track = DOWN.y - UP.y - UP.h - 8.0;
            let shown = rows.len().max(1) as f32 / n as f32;
            let th = (track * shown).max(8.0).min(track);
            let ty =
                UP.y + UP.h + 4.0 + (track - th) * (self.xs.history.off as f32 / (n.saturating_sub(rows.len()).max(1)) as f32).min(1.0);
            s.fill(g, UP.x + 7.0, ty, 6.0, th, white);
        }
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 24.0, tr("STR_HISTORY_CLOSE"), navy);
        if self.art.trans.tex.is_some() {
            let r = ok_tick();
            s.image_part(g, &self.art.trans, r.x, r.y, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }

    /// The ticker's picture for a speaker that is not a golfer, shrunk to `size` (the PopUpIcons pieces, the bare ball).
    pub fn small_picture(&self, g: &mut Gfx, s: &Ui, speaker: i32, x: f32, y: f32, size: f32) {
        let k = match speaker {
            -20 => 0,
            -3 => 1,
            -2 => 2,
            -22 => 3,
            -21 => 4,
            _ => {
                s.image_scaled(g, &self.art.trans, (x, y, size, size), (0.0, 300.0, 140.0, 140.0));
                return;
            }
        };
        s.image_scaled(g, &self.art.popup_icons, (x, y + size * 0.06, size, size * 0.88), (90.0 * k as f32, 0.0, 90.0, 80.0));
    }

    /// The extras' part of posting a ticker message (message_ui::post_message): the history keeps it, and the ticker
    /// remembers the place it is about for its Show button.
    pub fn keep_message(&mut self, text: &str, speaker: i32, shown: bool) {
        let place = self.xs.next_place.take();
        if shown {
            self.xs.ticker_place = place.map(|p| (speaker, p));
        }
        if !shown || !crate::extras::history() || text.trim().is_empty() {
            return;
        }
        let golfer = (speaker >= 0).then_some(speaker as usize).filter(|&g| g < sg_core::golfer::SLOTS);
        let roster = golfer.map(|g| self.club.g[g].roster).unwrap_or(-1);
        let face = golfer.map(|g| self.club.attitude(g) as u8).unwrap_or(0);
        let at = place.or(golfer.map(|g| (self.club.g[g].x, self.club.g[g].y)));
        let tick = self.game_tick;
        // the same message again at once (a monthly reminder for each building, a refused repeat) is kept once
        if self.xs.kept.messages.last().is_some_and(|m| m.text == text && tick.saturating_sub(m.tick) < 40 && m.at == at) {
            return;
        }
        self.xs.kept.push_message(Message { tick, text: text.to_string(), speaker, roster, face, at });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_at_least_a_portrait_high() {
        assert!(row_height(1) >= 36.0);
        assert_eq!(row_height(4), 14.0 + 60.0 + 6.0);
    }
}
