//! Touch controls for phones and tablets (the browser build). The original is a mouse and keyboard game, so this is the port's
//! own layer: it turns touches into the mouse clicks and keys the game already understands.
//!
//! * One finger: a tap is a left click; dragging pans the course (or paints with the current tool); holding still for half a
//!   second is a right click (undo, remove, close a screen).
//! * Two fingers: dragging pans, pinching steps through the three zoom levels, twisting turns the view a quarter.
//! * A few touch buttons down the right edge stand in for keys the screen has no button for (open the hole, turn the
//!   building, game speed, repeat the last message); they show only once the screen has been touched.
//! * When a name is typed (rename, save) the phone's keyboard is raised through the page (web/simgolf.js).

use crate::app::*;
use crate::gfx::Gfx;
use crate::ui::{rgb, rgba, Screen as Ui};
use crate::Stage;
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton, TouchPhase};

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
}

/// The touch buttons: label and the key each one presses.
const BUTTONS: [(&str, KeyCode); 4] =
    [("Open hole", KeyCode::H), ("Turn", KeyCode::Tab), ("Speed", KeyCode::RightBracket), ("Message", KeyCode::Slash)];

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
        let now = self.app.clock;
        match phase {
            TouchPhase::Started => {
                self.touch.fingers.retain(|f| f.0 != id);
                self.touch.fingers.push((id, (x, y), (x, y)));
                match self.touch.fingers.len() {
                    1 => {
                        if self.touch_button(x, y) {
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
            if !(0.74..=1.35).contains(&ratio) {
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
            if let Some(&(_, p, _)) = self.touch.fingers.first() {
                self.mouse_motion_event(p.0, p.1);
                self.touch.queued = Some((MouseButton::Right, p.0, p.1, 2));
            }
            self.touch.mode = Mode::Spent;
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

    fn touch_buttons_shown(&self) -> bool {
        self.touch.used && self.app.ui_ok && self.app.screen == Screen::Play
    }

    pub(crate) fn draw_touch_buttons(&mut self) {
        if !self.touch_buttons_shown() {
            return;
        }
        let s = Ui::new(self.app.draw_w, self.app.draw_h);
        let g: &mut Gfx = &mut self.g;
        for (i, &(label, _)) in BUTTONS.iter().enumerate() {
            let (x, y, w, h) = button_rect(i);
            s.fill(g, x, y, w, h, rgba(0.1, 0.1, 0.3, 0.7));
            s.fill(g, x, y, w, 1.0, rgba(0.7, 0.7, 1.0, 0.8));
            s.text_centered(g, x + w * 0.5, y + h * 0.5 + 4.0, label, 12.0, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }
}

impl App {
    /// Whether a name is being typed (rename, save, Customise Golfer).
    pub fn typing(&self) -> bool {
        self.rename.is_some() || self.title.save_name.is_some() || self.cust.as_ref().is_some_and(|c| c.typing.is_some())
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn sg_keyboard(show: u32);
}

/// Raises or lowers the phone's keyboard (a hidden text box in the page); nothing on the desktop.
fn show_keyboard(show: bool) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        sg_keyboard(show as u32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = show;
}
