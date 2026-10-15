//! Controllers (the port's own; docs/PLAYING.md "Controllers and the Steam Deck"): an Xbox, PlayStation, Switch or Steam
//! Deck controller plays the game through the mouse and keys it already has. On Windows, macOS and Linux the controller is
//! read through gilrs; in the browser the page reads the Gamepad API (web/opensgolf.js) and hands each frame's state to
//! `sg_gamepad`. Not on Android yet.
//!
//! * left stick: the pointer (drawn by the game while the controller is in use); right stick: scrolls the course;
//! * A: click (hold to drag or paint); B: on the course a right click (undo, remove), elsewhere Esc (close, back);
//! * X: turn (Tab); Y: game speed (]); LB / RB: turn the view; LT / RT: zoom out / in;
//! * d-pad: the arrow keys; Start: Esc (System Functions); Back / Select: pause; left stick click: open hole (h); right
//!   stick click: repeat the last message (?).
//!
//! A real mouse movement gives the pointer back to the mouse.

use crate::Stage;
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton};

/// The W3C standard gamepad's buttons, as bits of `State::buttons`.
pub mod btn {
    pub const A: u32 = 1 << 0;
    pub const B: u32 = 1 << 1;
    pub const X: u32 = 1 << 2;
    pub const Y: u32 = 1 << 3;
    pub const LB: u32 = 1 << 4;
    pub const RB: u32 = 1 << 5;
    pub const LT: u32 = 1 << 6;
    pub const RT: u32 = 1 << 7;
    pub const BACK: u32 = 1 << 8;
    pub const START: u32 = 1 << 9;
    pub const LS: u32 = 1 << 10;
    pub const RS: u32 = 1 << 11;
    pub const UP: u32 = 1 << 12;
    pub const DOWN: u32 = 1 << 13;
    pub const LEFT: u32 = 1 << 14;
    pub const RIGHT: u32 = 1 << 15;
}

/// One frame of the first controller: the buttons held and the sticks (x right, y down, -1 to 1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub buttons: u32,
    pub left: (f32, f32),
    pub right: (f32, f32),
}

/// The buttons that stand for a key, and the key.
const KEYS: [(u32, KeyCode); 10] = [
    (btn::X, KeyCode::Tab),
    (btn::Y, KeyCode::RightBracket),
    (btn::LB, KeyCode::Home),
    (btn::RB, KeyCode::PageUp),
    (btn::LT, KeyCode::X),
    (btn::RT, KeyCode::Z),
    (btn::BACK, KeyCode::P),
    (btn::START, KeyCode::Escape),
    (btn::LS, KeyCode::H),
    (btn::RS, KeyCode::Slash),
];
/// The d-pad: the arrow keys, repeating while held.
const DPAD: [(u32, KeyCode); 4] =
    [(btn::UP, KeyCode::Up), (btn::DOWN, KeyCode::Down), (btn::LEFT, KeyCode::Left), (btn::RIGHT, KeyCode::Right)];

/// Stick travel ignored around the centre.
const DEAD: f32 = 0.18;
/// The pointer's top speed, in window heights a second.
const POINTER_SPEED: f32 = 1.1;
/// The course's scroll speed at full tilt, in tiles a second.
const SCROLL_TILES: f32 = 14.0;
/// A held d-pad repeats after this long, this often (seconds).
const REPEAT: (f64, f64) = (0.4, 0.12);

#[derive(Default)]
pub struct Pad {
    #[cfg(all(feature = "gamepad", not(any(target_arch = "wasm32", target_os = "android"))))]
    gilrs: Option<gilrs::Gilrs>,
    #[cfg(all(feature = "gamepad", not(any(target_arch = "wasm32", target_os = "android"))))]
    started: bool,
    last: State,
    /// The controller has the pointer (the game draws it).
    pub active: bool,
    /// The pointer, in window pixels.
    pub pointer: (f32, f32),
    /// The last frame's clock.
    t: f64,
    /// When each held d-pad direction repeats next.
    repeat_at: [f64; 4],
    /// B went down off the course: it is Esc until it comes up.
    b_esc: bool,
}

#[cfg(target_arch = "wasm32")]
static WEB_STATE: std::sync::Mutex<Option<State>> = std::sync::Mutex::new(None);

/// The page's controller each frame (web/opensgolf.js): the buttons as `btn` bits and the two sticks.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn sg_gamepad(buttons: u32, lx: f32, ly: f32, rx: f32, ry: f32) {
    if let Ok(mut s) = WEB_STATE.lock() {
        *s = Some(State { buttons, left: (lx, ly), right: (rx, ry) });
    }
}

/// A stick with its dead zone taken out and a gentle curve (fine moves near the centre, fast at full tilt).
fn shape(v: (f32, f32)) -> (f32, f32) {
    let m = (v.0 * v.0 + v.1 * v.1).sqrt();
    if m < DEAD {
        return (0.0, 0.0);
    }
    let k = ((m - DEAD) / (1.0 - DEAD)).min(1.0);
    let k = k * k / m;
    (v.0 * k, v.1 * k)
}

impl Pad {
    /// This frame's state of the first controller, if there is one.
    fn read(&mut self) -> Option<State> {
        #[cfg(target_arch = "wasm32")]
        {
            WEB_STATE.lock().ok().and_then(|s| *s)
        }
        #[cfg(all(feature = "gamepad", not(any(target_arch = "wasm32", target_os = "android"))))]
        {
            if !self.started {
                self.started = true;
                self.gilrs = gilrs::Gilrs::new().map_err(|e| eprintln!("controllers: {e}")).ok();
            }
            let gil = self.gilrs.as_mut()?;
            while let Some(ev) = gil.next_event() {
                if let gilrs::EventType::Connected = ev.event {
                    println!("controller connected: {}", gil.gamepad(ev.id).name());
                }
            }
            let (_, pad) = gil.gamepads().find(|(_, p)| p.is_connected())?;
            use gilrs::{Axis, Button};
            let map = [
                Button::South,
                Button::East,
                Button::West,
                Button::North,
                Button::LeftTrigger,
                Button::RightTrigger,
                Button::LeftTrigger2,
                Button::RightTrigger2,
                Button::Select,
                Button::Start,
                Button::LeftThumb,
                Button::RightThumb,
                Button::DPadUp,
                Button::DPadDown,
                Button::DPadLeft,
                Button::DPadRight,
            ];
            let mut buttons = 0;
            for (i, b) in map.iter().enumerate() {
                if pad.is_pressed(*b) {
                    buttons |= 1 << i;
                }
            }
            // gilrs' y axes point up
            let axis = |a: Axis| pad.axis_data(a).map(|d| d.value()).unwrap_or(0.0);
            Some(State {
                buttons,
                left: (axis(Axis::LeftStickX), -axis(Axis::LeftStickY)),
                right: (axis(Axis::RightStickX), -axis(Axis::RightStickY)),
            })
        }
        #[cfg(not(any(target_arch = "wasm32", all(feature = "gamepad", not(target_os = "android")))))]
        {
            None
        }
    }
}

impl Stage {
    /// Each frame: the controller's sticks and buttons become the pointer, clicks and keys.
    pub(crate) fn gamepad_update(&mut self, t: f64) {
        let dt = if self.pad.t > 0.0 { (t - self.pad.t).clamp(0.0, 0.1) as f32 } else { 0.0 };
        self.pad.t = t;
        let Some(s) = self.pad.read() else { return };
        let prev = std::mem::replace(&mut self.pad.last, s);
        let pressed = s.buttons & !prev.buttons;
        let released = prev.buttons & !s.buttons;
        let (w, h) = (self.app.draw_w, self.app.draw_h);
        let none = KeyMods::default();
        // the left stick moves the pointer
        let (lx, ly) = shape(s.left);
        if (lx != 0.0 || ly != 0.0 || pressed != 0) && !self.pad.active {
            self.pad.active = true;
            self.pad.pointer = self.mouse;
            miniquad::window::show_mouse(false);
        }
        if lx != 0.0 || ly != 0.0 {
            let v = POINTER_SPEED * h * dt;
            let p = &mut self.pad.pointer;
            p.0 = (p.0 + lx * v).clamp(0.0, w - 1.0);
            p.1 = (p.1 + ly * v).clamp(0.0, h - 1.0);
            let (x, y) = self.pad.pointer;
            self.mouse_motion_event(x, y);
        }
        // the right stick scrolls the course
        let (rx, ry) = shape(s.right);
        if (rx != 0.0 || ry != 0.0) && self.app.screen == crate::app::Screen::Play {
            let d = SCROLL_TILES * sg_core::terrain::TILE_SIZE * dt;
            self.app.xs.follow = None;
            self.app.pan(rx * d, -ry * d);
        }
        let (x, y) = self.pad.pointer;
        // B: a right click on the course, Esc (close, back) everywhere else; let go where it went down
        if pressed & btn::B != 0 {
            self.pad.b_esc = self.app.screen != crate::app::Screen::Play;
        }
        if self.pad.b_esc {
            if pressed & btn::B != 0 {
                self.key_down_event(KeyCode::Escape, none, false);
            }
            if released & btn::B != 0 {
                self.key_up_event(KeyCode::Escape, none);
            }
        }
        let b_button = if self.pad.b_esc { 0 } else { btn::B };
        for (b, button) in [(btn::A, MouseButton::Left), (b_button, MouseButton::Right)] {
            if pressed & b != 0 {
                self.mouse_button_down_event(button, x, y);
            }
            if released & b != 0 {
                self.mouse_button_up_event(button, x, y);
            }
        }
        for &(b, k) in &KEYS {
            if pressed & b != 0 {
                self.key_down_event(k, none, false);
            }
            if released & b != 0 {
                self.key_up_event(k, none);
            }
        }
        // the d-pad repeats while held, for scrolling and lists
        for (i, &(b, k)) in DPAD.iter().enumerate() {
            if pressed & b != 0 {
                self.pad.repeat_at[i] = t + REPEAT.0;
                self.key_down_event(k, none, false);
            } else if s.buttons & b != 0 && t >= self.pad.repeat_at[i] {
                self.pad.repeat_at[i] = t + REPEAT.1;
                self.key_down_event(k, none, true);
            }
            if released & b != 0 {
                self.key_up_event(k, none);
            }
        }
    }

    /// A real mouse moved: it takes the pointer back.
    pub(crate) fn gamepad_mouse_moved(&mut self, x: f32, y: f32) {
        if self.pad.active && (x, y) != self.pad.pointer {
            self.pad.active = false;
            miniquad::window::show_mouse(true);
        }
    }

    /// The controller's pointer: a white arrow with a dark edge, while the controller has it.
    pub(crate) fn draw_gamepad_pointer(&mut self) {
        if !self.pad.active {
            return;
        }
        let s = crate::ui::Screen::new(self.app.draw_w, self.app.draw_h);
        let (vx, vy) = s.view.to_virtual(self.pad.pointer.0, self.pad.pointer.1);
        let g = &mut self.g;
        // an arrow 11 wide and 16 tall, row by row: (first column, width)
        const ROWS: [(f32, f32); 16] = [
            (0.0, 1.0),
            (0.0, 2.0),
            (0.0, 3.0),
            (0.0, 4.0),
            (0.0, 5.0),
            (0.0, 6.0),
            (0.0, 7.0),
            (0.0, 8.0),
            (0.0, 9.0),
            (0.0, 10.0),
            (0.0, 6.0),
            (0.0, 3.0),
            (4.0, 3.0),
            (0.0, 2.0),
            (5.0, 3.0),
            (5.0, 3.0),
        ];
        let px = 1.2;
        for (r, &(x0, wd)) in ROWS.iter().enumerate() {
            let y = vy + r as f32 * px;
            s.fill(g, vx + x0 * px - px, y - px, (wd + 2.0) * px, px * 3.0, crate::ui::rgba(0.05, 0.05, 0.1, 0.9));
        }
        for (r, &(x0, wd)) in ROWS.iter().enumerate() {
            let y = vy + r as f32 * px;
            s.fill(g, vx + x0 * px, y, wd * px, px, crate::ui::rgb(1.0, 1.0, 1.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_shape() {
        assert_eq!(shape((0.1, 0.1)), (0.0, 0.0));
        let (x, y) = shape((1.0, 0.0));
        assert!((x - 1.0).abs() < 1e-5 && y == 0.0);
        let (x, _) = shape((0.5, 0.0));
        assert!(x > 0.0 && x < 0.5);
    }
}
