//! Preferences > Keyboard controls > Change keys (the port's own screen, controls.rs): every action of the course screen
//! with its key; a click on one waits for its new key, Esc cancels the wait. Keys that answer to two actions are printed in
//! red with a warning; the WASD camera and Reset to defaults are buttons under the list. Drawn as the Keyboard controls screen
//! is (keys_ui.rs), on its box; the OK tick goes back to that screen.
//!
//! Also here: the interface hidden with U during play, and the hint on how to bring it back.
use crate::app::*;
use crate::controls::{self, Binding, ACTIONS};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{rgb, rgba, Fnt, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use miniquad::KeyCode;
use sg_core::lang::{tr, trf};

#[derive(Default)]
pub struct CtlState {
    /// The interface is hidden (U during play).
    pub hud_hidden: bool,
    /// The Change keys screen, while it is open over the Keyboard controls screen.
    pub remap: Option<Remap>,
}

#[derive(Default)]
pub struct Remap {
    pub page: usize,
    /// The action waiting for its new key.
    pub capture: Option<usize>,
    /// The last thing that happened (and whether it is a warning).
    pub status: Option<(String, bool)>,
}

const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
const COL_X: [f32; 2] = [BOX.x + 22.0, 412.0];
const COL_W: f32 = 330.0;
const TOP: f32 = BOX.y + 62.0;
const LINE: f32 = 18.0;
const PER_COL: usize = 22;
const PER_PAGE: usize = PER_COL * 2;
const KEY_W: f32 = 96.0;
const FONT: Fnt = F_MANUAL15;

fn pages() -> usize {
    ACTIONS.len().div_ceil(PER_PAGE)
}

/// Where action `i` is drawn on its page: (page, column, top).
fn place(i: usize) -> (usize, usize, f32) {
    let k = i % PER_PAGE;
    (i / PER_PAGE, k / PER_COL, TOP + (k % PER_COL) as f32 * LINE)
}

fn row_rect(i: usize) -> Rect {
    let (_, c, y) = place(i);
    Rect::new(COL_X[c] - 4.0, y - 1.0, COL_W + 4.0, LINE)
}

fn button(k: usize) -> Rect {
    // previous, next, WASD camera, reset
    let (x, w) = [(24.0, 80.0), (110.0, 80.0), (256.0, 170.0), (432.0, 150.0)][k];
    Rect::new(BOX.x + x, BOX.y + BOX.h - 54.0, w, 20.0)
}

fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

/// The Keyboard controls screen's Change keys button.
pub fn change_keys_button() -> Rect {
    Rect::new(BOX.x + BOX.w - 220.0, BOX.y + BOX.h - 52.0, 130.0, 20.0)
}

/// `t` cut to fit `w` pixels of `f`, with "..." when cut.
fn fit(f: Fnt, t: &str, w: f32) -> String {
    if f.width(t) <= w {
        return t.to_string();
    }
    let mut s = t.to_string();
    while !s.is_empty() && f.width(&format!("{s}...")) > w {
        s.pop();
    }
    format!("{s}...")
}

/// A key's label on the Change keys screen: the key, the WASD camera's second key after a comma.
fn keys_label(s: &controls::Settings, i: usize) -> String {
    let mut v: Vec<String> = s.keys_of(i).map(Binding::label).collect();
    if v.is_empty() {
        v.push(tr("STR_CTL_NO_KEY").to_string());
    }
    v.join(", ")
}

impl App {
    pub fn open_remap(&mut self) {
        self.ctl.remap = Some(Remap::default());
        self.hover = -1;
    }

    /// Change keys on action `i`'s page with the action waiting for its new key (the Preferences' search, prefs_search.rs).
    pub fn open_remap_on(&mut self, i: usize) {
        self.ctl.remap = Some(Remap { page: place(i).0, capture: Some(i), status: None });
        self.hover = -1;
    }

    /// A click on the Change keys screen.
    pub fn remap_click(&mut self, x: f32, y: f32, right: bool) {
        let Some(r) = self.ctl.remap.as_mut() else { return };
        // a click while an action waits for its key cancels the wait
        if r.capture.take().is_some() {
            r.status = None;
            return;
        }
        if right || ok_tick().has(x, y) {
            self.ctl.remap = None;
            return;
        }
        let n = pages();
        if n > 1 && button(0).has(x, y) {
            r.page = r.page.saturating_sub(1);
            return;
        }
        if n > 1 && button(1).has(x, y) {
            r.page = (r.page + 1).min(n - 1);
            return;
        }
        if button(2).has(x, y) {
            let mut s = controls::settings();
            s.wasd = !s.wasd;
            controls::save(s);
            // the player's own keys that the WASD camera now uses would clash: they are reported
            let t = self.remap_conflict_text();
            self.ctl.remap.as_mut().expect("open").status = Some(t);
            return;
        }
        if button(3).has(x, y) {
            controls::save(controls::Settings::default());
            r.status = Some((tr("STR_CTL_RESET_DONE").to_string(), false));
            return;
        }
        let page = r.page;
        if let Some(i) = (0..ACTIONS.len()).find(|&i| place(i).0 == page && row_rect(i).has(x, y)) {
            r.capture = Some(i);
            r.status = Some((trf("STR_CTL_PRESS", &[("ACTION", &tr(ACTIONS[i].text))]), false));
        }
    }

    /// The status line after a change: the clashes left, else that all is well.
    fn remap_conflict_text(&self) -> (String, bool) {
        let s = controls::settings();
        let clash: Vec<String> = (0..ACTIONS.len())
            .filter(|&i| !s.conflicts(i).is_empty())
            .map(|i| format!("{} ({})", tr(ACTIONS[i].text), keys_label(&s, i)))
            .collect();
        if clash.is_empty() {
            (tr(if s.wasd { "STR_CTL_WASD_ON" } else { "STR_CTL_SAVED" }).to_string(), false)
        } else {
            (trf("STR_CTL_CLASH", &[("ACTIONS", &clash.join(", "))]), true)
        }
    }

    /// Keys on the Change keys screen: while an action waits, the key pressed becomes its key (Esc cancels); otherwise Esc
    /// goes back, Left and Right turn the pages.
    pub fn remap_key(&mut self, k: KeyCode) {
        let shift = self.shift_held;
        let Some(r) = self.ctl.remap.as_mut() else { return };
        let Some(i) = r.capture else {
            match k {
                KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => self.ctl.remap = None,
                KeyCode::Right | KeyCode::PageDown => r.page = (r.page + 1).min(pages() - 1),
                KeyCode::Left | KeyCode::PageUp => r.page = r.page.saturating_sub(1),
                _ => {}
            }
            return;
        };
        use KeyCode::*;
        if matches!(k, LeftShift | RightShift | LeftControl | RightControl | LeftAlt | RightAlt | LeftSuper | RightSuper) {
            return;
        }
        r.capture = None;
        if k == Escape {
            r.status = None;
            return;
        }
        let b = Binding { key: k, shift };
        if controls::reserved(b) {
            r.status = Some((trf("STR_CTL_RESERVED", &[("KEY", &b.label())]), true));
            self.ui_sound(0x18);
            return;
        }
        let mut s = controls::settings();
        s.bind(i, Some(b));
        let clash: Vec<String> = s.conflicts(i).iter().map(|&j| tr(ACTIONS[j].text).to_string()).collect();
        controls::save(s);
        let r = self.ctl.remap.as_mut().expect("open");
        r.status = Some(if clash.is_empty() {
            (trf("STR_CTL_BOUND", &[("ACTION", &tr(ACTIONS[i].text)), ("KEY", &b.label())]), false)
        } else {
            (trf("STR_CTL_ALSO", &[("KEY", &b.label()), ("ACTIONS", &clash.join(", "))]), true)
        });
    }

    pub fn draw_remap(&mut self, g: &mut Gfx) {
        let Some(r) = self.ctl.remap.as_ref() else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, grey, yellow, navy) = (c15(0x7fff), c15(0x6318), c15(0x7ff0), c15(0x0018));
        let red = rgb(0.85, 0.1, 0.1);
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_CTL_TITLE"), yellow, true);
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, tr("STR_CTL_HELP"), navy);
        let set = controls::settings();
        let (px, py) = self.info.pointer;
        for i in (0..ACTIONS.len()).filter(|&i| place(i).0 == r.page) {
            let (_, c, y) = place(i);
            let x = COL_X[c];
            let rr = row_rect(i);
            let waiting = r.capture == Some(i);
            if waiting || (r.capture.is_none() && rr.has(px, py)) {
                s.fill(g, rr.x, rr.y, rr.w, rr.h, rgb(0.66, 0.66, 0.84));
            }
            s.put(g, FONT, x + 2.0, y, &fit(FONT, tr(ACTIONS[i].text), COL_W - KEY_W - 10.0), navy);
            let kx = x + COL_W - KEY_W;
            let (label, colour) = if waiting {
                (tr("STR_CTL_WAITING").to_string(), yellow)
            } else if !set.conflicts(i).is_empty() {
                (keys_label(&set, i), red)
            } else {
                (keys_label(&set, i), white)
            };
            s.fill(g, kx - 2.0, y, KEY_W, LINE - 2.0, rgba(0.0, 0.0, 0.3, if waiting { 0.6 } else { 0.25 }));
            s.put(g, FONT, kx + 2.0, y, &fit(FONT, &label, KEY_W - 6.0), colour);
        }
        let n = pages();
        let wasd = trf("STR_CTL_WASD", &[("STATE", &tr(if set.wasd { "STR_CTL_ON" } else { "STR_CTL_OFF" }))]);
        let page_ok = [r.page > 0, r.page + 1 < n];
        let labels = [tr("STR_CONTENT_PREVIOUS").to_string(), tr("STR_CONTENT_NEXT").to_string(), wasd, tr("STR_CTL_RESET").to_string()];
        for (k, label) in labels.iter().enumerate() {
            if k < 2 && n < 2 {
                continue;
            }
            let b = button(k);
            let ok = k >= 2 || page_ok[k];
            let lit = ok && b.has(px, py);
            s.fill(g, b.x, b.y, b.w, b.h, if lit { rgb(0.85, 0.85, 1.0) } else { rgb(0.47, 0.47, 0.70) });
            s.put_centered(g, F_ARIAL10, b.x + b.w / 2.0, b.y + 4.0, label, if ok { navy } else { grey });
        }
        let line = match &r.status {
            Some((t, warn)) => (t.clone(), if *warn { red } else { navy }),
            None => (tr("STR_CTL_FOOT").to_string(), navy),
        };
        for (k, l) in crate::ui::wrap_text(&line.0, 10.0, BOX.w - 120.0).into_iter().take(2).enumerate() {
            s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 28.0 + k as f32 * 11.0, &l, line.1);
        }
        if self.art.trans.tex.is_some() {
            let t = ok_tick();
            s.image_part(g, &self.art.trans, t.x, t.y, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }

    /// The interface hidden during play: a small note in the bottom left corner says how to bring it back.
    pub fn draw_hud_hint(&self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        let t = trf("STR_CTL_HUD_HINT", &[("KEY", &controls::hide_key_label())]);
        let w = F_ARIAL10.width(&t) + 12.0;
        s.fill(g, 6.0, 580.0, w, 14.0, rgba(0.0, 0.0, 0.0, 0.45));
        s.put(g, F_ARIAL10, 12.0, 582.0, &t, rgba(1.0, 1.0, 1.0, 0.85));
        g.flush();
    }

    /// True while the interface is hidden on the course screen (play, not photo mode).
    pub fn hud_hidden(&self) -> bool {
        self.ctl.hud_hidden && self.screen == Screen::Play && self.photo.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_has_a_row_inside_the_list() {
        for i in 0..ACTIONS.len() {
            let r = row_rect(i);
            assert!(r.y + r.h <= button(0).y, "{} runs into the buttons", ACTIONS[i].id);
        }
        for i in 0..ACTIONS.len() {
            for j in i + 1..ACTIONS.len() {
                if place(i).0 == place(j).0 {
                    let (a, b) = (row_rect(i), row_rect(j));
                    assert!(!(a.has(b.x + 1.0, b.y + 1.0) && a.has(b.x + b.w - 1.0, b.y + b.h - 1.0)));
                }
            }
        }
    }
}
