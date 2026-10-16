//! Touch controls for phones and tablets (the browser build and the Android app). The original is a mouse and keyboard game, so this is the port's
//! own layer: it turns touches into the mouse clicks and keys the game already understands.
//!
//! * One finger: a tap is a left click; dragging pans the course (or paints with the current tool); holding still for half a
//!   second is a right click (undo, remove, close a screen). With the raise / lower tool armed, holding still over the course
//!   lowers the ground instead (the desktop's Shift+click, which a tablet has no key for), and dragging on from there keeps
//!   lowering.
//! * Two fingers: dragging pans, pinching steps through the three zoom levels, twisting turns the view a quarter.
//! * A few touch buttons down the right edge stand in for keys the screen has no button for (open the hole, turn the
//!   building, game speed, repeat the last message); they show only once the screen has been touched.
//! * When a name is typed (rename, save) the phone's keyboard is raised: through the page in a browser (web/opensgolf.js),
//!   through miniquad on Android.

use crate::app::*;
use crate::gfx::Gfx;
use crate::ui::{rgb, rgba, Screen as Ui};
use crate::Stage;
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton, TouchPhase};
use sg_core::lang::tr;

/// Drawable pixels a finger may wander before a press becomes a drag.
const SLOP: f32 = 12.0;
/// Seconds a still finger takes to become a right click.
const LONG_PRESS: f64 = 0.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Mode {
    /// No finger down.
    #[default]
    Idle,
    /// One finger down and still: a tap, a drag or a long press yet to be decided.
    Pending,
    /// One finger driving the mouse (its button is down).
    Mouse,
    /// Two fingers: pan, pinch and twist.
    Gesture,
    /// The touch has been used up (a long press, a touch button); the rest of it is ignored.
    Spent,
}

/// A finger: its id, where it went down and where it is now.
type Finger = (u64, (f32, f32), (f32, f32));

#[derive(Default)]
pub struct Touch {
    /// The screen has been touched: the touch buttons show from then on.
    pub used: bool,
    mode: Mode,
    /// Fingers down: id, start and current position.
    fingers: Vec<Finger>,
    started: f64,
    /// The two-finger gesture's last midpoint, the spread and the angle the next step is measured from.
    mid: (f32, f32),
    spread: f32,
    angle: f32,
    /// Whether the phone's keyboard is up.
    keyboard: bool,
    /// A click waiting for the pointer's move to reach the screens (they act on what is under the pointer as of the last
    /// frame): the button, where, and frames left.
    queued: Option<(MouseButton, f32, f32, u8)>,
    /// A long press lowering the ground: the Shift it holds down is let go with the finger.
    lowering: bool,
    /// The on-screen keyboard's Shift (capitals for the next letter).
    osk_shift: bool,
}

/// The on-screen keyboard (browser on a touch screen, while a name is typed; issue #50): Safari on an iPad raises its own
/// keyboard only for some taps, and a name box closes only with Enter or Esc. Rows of keys across the bottom of the
/// 800 x 600 screen, below every name box.
const OSK_ROWS: [&str; 4] = ["1234567890", "qwertyuiop", "asdfghjkl'", "zxcvbnm,.-"];
const OSK_TOP: f32 = 420.0;
const OSK_H: f32 = 30.0;
const OSK_GAP: f32 = 4.0;

/// What an on-screen key does.
#[derive(Clone, Copy, PartialEq, Debug)]
enum OskKey {
    Char(char),
    Shift,
    Back,
    Space,
    Cancel,
    Ok,
}

/// Every key and its box in virtual pixels.
fn osk_keys() -> Vec<(OskKey, (f32, f32, f32, f32))> {
    let mut v = Vec::new();
    let w = 66.0;
    for (r, row) in OSK_ROWS.iter().enumerate() {
        let y = OSK_TOP + r as f32 * (OSK_H + OSK_GAP);
        let n = row.chars().count() as f32;
        let x0 = 400.0 - (n * (w + OSK_GAP) - OSK_GAP) * 0.5 - if r == 3 { 40.0 } else { 0.0 };
        for (i, c) in row.chars().enumerate() {
            v.push((OskKey::Char(c), (x0 + i as f32 * (w + OSK_GAP), y, w, OSK_H)));
        }
        if r == 3 {
            v.push((OskKey::Back, (x0 + n * (w + OSK_GAP), y, 76.0, OSK_H)));
        }
    }
    let y = OSK_TOP + 4.0 * (OSK_H + OSK_GAP);
    v.push((OskKey::Shift, (30.0, y, 110.0, OSK_H)));
    v.push((OskKey::Cancel, (146.0, y, 110.0, OSK_H)));
    v.push((OskKey::Space, (262.0, y, 276.0, OSK_H)));
    v.push((OskKey::Ok, (544.0, y, 226.0, OSK_H)));
    v
}

fn osk_hit(vx: f32, vy: f32) -> Option<OskKey> {
    osk_keys().into_iter().find(|(_, (x, y, w, h))| vx >= *x && vx < x + w && vy >= *y && vy < y + h).map(|(k, _)| k)
}

/// The touch buttons: label (a text key) and the key each one presses.
const BUTTONS: [(&str, KeyCode); 4] = [
    ("STR_TOUCH_OPEN_HOLE", KeyCode::H),
    ("STR_TOUCH_TURN", KeyCode::Tab),
    ("STR_TOUCH_SPEED", KeyCode::RightBracket),
    ("STR_TOUCH_MESSAGE", KeyCode::Slash),
];

fn button_rect(i: usize) -> (f32, f32, f32, f32) {
    (734.0, 190.0 + 40.0 * i as f32, 62.0, 32.0)
}

fn two(f: &[Finger]) -> ((f32, f32), f32, f32) {
    let (a, b) = (f[0].2, f[1].2);
    let mid = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    (mid, (dx * dx + dy * dy).sqrt().max(1.0), dy.atan2(dx))
}

impl Stage {
    pub(crate) fn touch(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        self.touch.used = true;
        // no pointer to hover with: the extras' name tags stay off (tags_ui)
        self.app.xs.touched = true;
        let now = self.app.clock;
        match phase {
            TouchPhase::Started => {
                self.touch.fingers.retain(|f| f.0 != id);
                self.touch.fingers.push((id, (x, y), (x, y)));
                match self.touch.fingers.len() {
                    1 => {
                        if self.osk_press(x, y) || self.touch_button(x, y) {
                            self.touch.mode = Mode::Spent;
                        } else {
                            self.touch.mode = Mode::Pending;
                            self.touch.started = now;
                        }
                    }
                    2 => {
                        // a second finger ends whatever the first was doing
                        if self.touch.mode == Mode::Mouse {
                            let p = self.touch.fingers[0].2;
                            self.mouse_button_up_event(MouseButton::Left, p.0, p.1);
                        }
                        self.touch.mode = Mode::Gesture;
                        let (mid, spread, angle) = two(&self.touch.fingers);
                        (self.touch.mid, self.touch.spread, self.touch.angle) = (mid, spread, angle);
                    }
                    _ => {}
                }
            }
            TouchPhase::Moved => {
                let Some(f) = self.touch.fingers.iter_mut().find(|f| f.0 == id) else { return };
                f.2 = (x, y);
                let start = f.1;
                match self.touch.mode {
                    Mode::Pending => {
                        let (dx, dy) = (x - start.0, y - start.1);
                        if dx * dx + dy * dy > SLOP * SLOP * self.app.dpi * self.app.dpi {
                            self.mouse_motion_event(start.0, start.1);
                            self.mouse_button_down_event(MouseButton::Left, start.0, start.1);
                            self.touch.mode = Mode::Mouse;
                            self.mouse_motion_event(x, y);
                        }
                    }
                    Mode::Mouse => self.mouse_motion_event(x, y),
                    Mode::Gesture if self.touch.fingers.len() >= 2 => self.gesture(),
                    _ => {}
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let was = self.touch.mode;
                let last = self.touch.fingers.len() == 1;
                self.touch.fingers.retain(|f| f.0 != id);
                match was {
                    Mode::Pending if phase == TouchPhase::Ended => {
                        // a tap: point where the finger was, then press and release once the screen has seen the pointer
                        self.mouse_motion_event(x, y);
                        self.touch.queued = Some((MouseButton::Left, x, y, 2));
                    }
                    Mode::Mouse => self.mouse_button_up_event(MouseButton::Left, x, y),
                    _ => {}
                }
                if self.touch.lowering && was == Mode::Mouse {
                    self.touch.lowering = false;
                    self.shift = false;
                }
                if last || self.touch.fingers.is_empty() {
                    self.touch.mode = Mode::Idle;
                } else if was == Mode::Gesture {
                    // lifting one of two fingers ends the gesture; the other one does nothing more
                    self.touch.mode = Mode::Spent;
                }
            }
        }
    }

    /// Two fingers: the midpoint pans the course, a pinch past a third more or less steps the zoom, a twist past 50
    /// degrees turns the view a quarter.
    fn gesture(&mut self) {
        let (mid, spread, angle) = two(&self.touch.fingers);
        if self.app.screen == Screen::Play {
            let (dx, dy) = (mid.0 - self.touch.mid.0, mid.1 - self.touch.mid.1);
            let k = self.app.upp;
            let sin_p = (pitch_for(self.app.draw_w, self.app.draw_h).to_radians()).sin() as f32;
            self.app.pan(-dx * k, dy * k / sin_p);
            let ratio = spread / self.touch.spread;
            if crate::enhanced::smooth_camera() || self.app.photo.is_some() {
                // Enhanced (and its photo mode): the zoom follows the fingers continuously
                if ratio > 0.0 && ratio.is_finite() {
                    self.app.zoom_by(ratio.log2());
                    self.touch.spread = spread;
                }
            } else if !(0.74..=1.35).contains(&ratio) {
                self.app.zoom_step(ratio > 1.0);
                self.touch.spread = spread;
            }
            let mut turn = angle - self.touch.angle;
            while turn > std::f32::consts::PI {
                turn -= std::f32::consts::TAU;
            }
            while turn < -std::f32::consts::PI {
                turn += std::f32::consts::TAU;
            }
            if turn.abs() > 50f32.to_radians() {
                self.app.rotate_view(if turn > 0.0 { -1 } else { 1 });
                self.touch.angle = angle;
            }
        }
        self.touch.mid = mid;
    }

    /// Called every frame: a finger held still long enough becomes a right click; the phone keyboard follows the name boxes.
    pub(crate) fn touch_update(&mut self) {
        if self.touch.mode == Mode::Pending && self.app.clock - self.touch.started > LONG_PRESS {
            let p = self.touch.fingers.first().map(|f| f.1);
            match p {
                // the raise / lower tool over the course: the press lowers, as Shift+click does (issue #39), and the finger
                // drags on lowering until it lifts
                Some(p) if self.lowers_here(p) => {
                    self.mouse_motion_event(p.0, p.1);
                    self.shift = true;
                    self.touch.lowering = true;
                    self.mouse_button_down_event(MouseButton::Left, p.0, p.1);
                    self.touch.mode = Mode::Mouse;
                }
                Some(p) => {
                    self.mouse_motion_event(p.0, p.1);
                    self.touch.queued = Some((MouseButton::Right, p.0, p.1, 2));
                    self.touch.mode = Mode::Spent;
                }
                None => self.touch.mode = Mode::Spent,
            }
        }
        if let Some((b, x, y, n)) = self.touch.queued {
            if n > 0 {
                self.touch.queued = Some((b, x, y, n - 1));
            } else {
                self.touch.queued = None;
                self.mouse_button_down_event(b, x, y);
                self.mouse_button_up_event(b, x, y);
            }
        }
        let typing = self.app.typing();
        if self.touch.used && typing != self.touch.keyboard {
            self.touch.keyboard = typing;
            show_keyboard(typing);
        }
    }

    /// Whether a long press at this drawable-pixel point lowers the ground: the raise / lower tool is armed and the point is
    /// on the course, not on the dock or its panel.
    fn lowers_here(&self, p: (f32, f32)) -> bool {
        if self.app.screen != Screen::Play || !self.app.edit || self.app.tool != 1 || self.app.popup.is_some() {
            return false;
        }
        let (vx, vy) = self.app.view.to_virtual(p.0, p.1);
        vy < 452.0 && !(self.app.ui_ok && crate::render::dock_hit(vx, vy) >= 0)
    }

    /// A press on one of the touch buttons: its key, as if typed.
    fn touch_button(&mut self, x: f32, y: f32) -> bool {
        if !self.touch_buttons_shown() {
            return false;
        }
        let (vx, vy) = self.app.view.to_virtual(x, y);
        for (i, &(_, key)) in BUTTONS.iter().enumerate() {
            let (bx, by, bw, bh) = button_rect(i);
            if vx >= bx && vx < bx + bw && vy >= by && vy < by + bh {
                let shift = key == KeyCode::Slash;
                let mods = KeyMods { shift, ..Default::default() };
                self.key_down_event(key, mods, false);
                self.key_up_event(key, mods);
                return true;
            }
        }
        false
    }

    fn osk_shown(&self) -> bool {
        cfg!(target_arch = "wasm32") && self.touch.used && self.app.typing()
    }

    /// A press on the on-screen keyboard: a letter typed, or Backspace, Enter (OK) or Esc (Cancel) pressed.
    fn osk_press(&mut self, x: f32, y: f32) -> bool {
        if !self.osk_shown() {
            return false;
        }
        let (vx, vy) = self.app.view.to_virtual(x, y);
        let Some(k) = osk_hit(vx, vy) else {
            // the keyboard's band takes every press, so a miss between keys does not reach the screen under it
            return vy >= OSK_TOP - OSK_GAP;
        };
        let none = KeyMods::default();
        let mut key = |me: &mut Self, code| {
            me.key_down_event(code, none, false);
            me.key_up_event(code, none);
        };
        match k {
            OskKey::Char(c) => {
                let c = if std::mem::take(&mut self.touch.osk_shift) { c.to_ascii_uppercase() } else { c };
                self.char_event(c, none, false);
            }
            OskKey::Space => self.char_event(' ', none, false),
            OskKey::Shift => self.touch.osk_shift = !self.touch.osk_shift,
            OskKey::Back => key(self, KeyCode::Backspace),
            OskKey::Ok => key(self, KeyCode::Enter),
            OskKey::Cancel => key(self, KeyCode::Escape),
        }
        true
    }

    pub(crate) fn draw_osk(&mut self) {
        if !self.osk_shown() {
            return;
        }
        let s = Ui::new(self.app.draw_w, self.app.draw_h);
        let g: &mut Gfx = &mut self.g;
        let shift = self.touch.osk_shift;
        s.fill(g, 0.0, OSK_TOP - 8.0, 800.0, 600.0 - OSK_TOP + 8.0, rgba(0.05, 0.05, 0.15, 0.85));
        for (k, (x, y, w, h)) in osk_keys() {
            let lit = k == OskKey::Shift && shift;
            let fill = match k {
                OskKey::Ok => rgba(0.15, 0.45, 0.2, 0.95),
                OskKey::Cancel => rgba(0.45, 0.15, 0.15, 0.95),
                _ if lit => rgba(0.4, 0.4, 0.8, 0.95),
                _ => rgba(0.2, 0.2, 0.35, 0.95),
            };
            s.fill(g, x, y, w, h, fill);
            s.fill(g, x, y, w, 1.0, rgba(0.7, 0.7, 1.0, 0.8));
            let label = match k {
                OskKey::Char(c) if shift => c.to_ascii_uppercase().to_string(),
                OskKey::Char(c) => c.to_string(),
                OskKey::Shift => tr("STR_OSK_SHIFT").to_string(),
                OskKey::Back => tr("STR_OSK_BACK").to_string(),
                OskKey::Space => tr("STR_OSK_SPACE").to_string(),
                OskKey::Cancel => tr("STR_OSK_CANCEL").to_string(),
                OskKey::Ok => tr("STR_OSK_OK").to_string(),
            };
            s.text_centered(g, x + w * 0.5, y + h * 0.5 + 5.0, &label, 15.0, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }

    fn touch_buttons_shown(&self) -> bool {
        self.touch.used && self.app.ui_ok && self.app.screen == Screen::Play && self.app.photo.is_none()
    }

    pub(crate) fn draw_touch_buttons(&mut self) {
        if !self.touch_buttons_shown() {
            return;
        }
        let s = Ui::new(self.app.draw_w, self.app.draw_h);
        // Enhanced widescreen: at the window's right edge (nothing in Classic)
        s.anchor(crate::enhanced::Anchor::Right);
        let g: &mut Gfx = &mut self.g;
        for (i, &(label, _)) in BUTTONS.iter().enumerate() {
            let (x, y, w, h) = button_rect(i);
            s.fill(g, x, y, w, h, rgba(0.1, 0.1, 0.3, 0.7));
            s.fill(g, x, y, w, 1.0, rgba(0.7, 0.7, 1.0, 0.8));
            s.text_centered(g, x + w * 0.5, y + h * 0.5 + 4.0, tr(label), 12.0, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }
}

impl App {
    /// Whether a name is being typed (rename, save, Customise Golfer).
    pub fn typing(&self) -> bool {
        self.rename.is_some()
            || self.title.save_name.is_some()
            || self.content.profile_name.is_some()
            || self.cust.as_ref().is_some_and(|c| c.typing.is_some())
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn sg_keyboard(show: u32);
}

/// The version of web/opensgolf.js's miniquad plugin ("simgolf_fs", version 1) this build goes with: gl.js asks for it and
/// otherwise logs that the plugin is unused.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn simgolf_fs_crate_version() -> u32 {
    1
}

/// Raises or lowers the phone's keyboard: in a browser a hidden text box in the page, on Android the system keyboard through
/// miniquad (its typed letters come back as char events); nothing on the desktop.
fn show_keyboard(show: bool) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        sg_keyboard(show as u32)
    }
    #[cfg(target_os = "android")]
    miniquad::window::show_keyboard(show);
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    let _ = show;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osk_keys_fit_the_screen_below_the_name_boxes_and_do_not_overlap() {
        let keys = osk_keys();
        for (i, (k, (x, y, w, h))) in keys.iter().enumerate() {
            assert!(*x >= 0.0 && x + w <= 800.0 && *y >= 400.0 && y + h <= 600.0, "{k:?} off the screen");
            for (k2, (x2, y2, w2, h2)) in &keys[i + 1..] {
                let apart = x + w <= *x2 || x2 + w2 <= *x || y + h <= *y2 || y2 + h2 <= *y;
                assert!(apart, "{k:?} overlaps {k2:?}");
            }
            assert_eq!(osk_hit(x + w * 0.5, y + h * 0.5), Some(*k));
        }
        // every letter and digit is there, and the keys that close a box
        for c in "abcdefghijklmnopqrstuvwxyz0123456789".chars() {
            assert!(keys.iter().any(|(k, _)| *k == OskKey::Char(c)), "{c}");
        }
        assert!(keys.iter().any(|(k, _)| *k == OskKey::Ok) && keys.iter().any(|(k, _)| *k == OskKey::Cancel));
        // the rename box (240..330) and the save box stay clear of the keyboard
        assert!(keys.iter().all(|(_, (_, y, _, _))| *y > 340.0));
    }
}
