//! The 3DS's controls as the events the game crate's `Stage` already handles (mouse, touch, keys) plus two it does not have
//! (camera pan and zoom as analogue values, which main.rs applies to the camera directly).
//!
//! Mapping (change the tables, not the code):
//!   touch screen         the bottom screen: the course plan to draw on and its tools (plan.rs), as `Pen` events in its
//!                        pixels; also as touch events in the original's 800 x 600 UI coordinates for the game's screens
//!   circle pad           moves the course cursor on the top screen (the mouse pointer of the original)
//!   L / R held           a stylus drag on the plan slides the view instead of painting; a quick tap of L / R zooms
//!   A / B                left / right click at the course cursor (A uses the armed tool there, as a pen stroke would;
//!                        B undoes strokes, up to 20 back)
//!   X                    pause (Space, as the original)
//!   Y                    the bottom strip's next tab (Tab)
//!   C-stick / D-pad      pan the camera; in the pause menu the D-pad's up and down choose (Up / Down keys)
//!   L / R (tapped)       zoom out / in (on release, and only when no drag used them as a modifier)
//!   ZL / ZR (New 3DS)    game speed down / up (Q / E)
//!   Start                the pause menu: resume, save, load, controls, quit (Escape)
//!   Select               steps the top screen's panels: money, holes, golfers, staff, none (F1)
//!   3D slider            read by gfx.rs each frame; nothing to map
use crate::screens::{self, TOP_H, TOP_W};
use crate::shim::{KeyCode, KeyMods, MouseButton, TouchPhase};
use ctru::services::hid::{Hid, KeyPad};

#[derive(Clone, Copy, Debug)]
pub enum Event {
    KeyDown(KeyCode, KeyMods),
    KeyUp(KeyCode, KeyMods),
    MouseMove(f32, f32),
    MouseDown(MouseButton, f32, f32),
    MouseUp(MouseButton, f32, f32),
    /// On the bottom screen, in the original's 800 x 600 UI space.
    Touch(TouchPhase, u64, f32, f32),
    /// The same touch in bottom-screen pixels (320 x 240): the course plan and its tool strip (plan.rs).
    Pen(TouchPhase, f32, f32),
    /// Camera pan in top-screen pixels per frame.
    Pan(f32, f32),
    /// Zoom steps this frame (+ in, - out).
    Zoom(i32),
}

const BUTTON_KEYS: &[(KeyPad, KeyCode)] = &[
    (KeyPad::X, KeyCode::Space),
    (KeyPad::Y, KeyCode::Tab),
    (KeyPad::START, KeyCode::Escape),
    (KeyPad::SELECT, KeyCode::F1),
    (KeyPad::ZL, KeyCode::Q),
    (KeyPad::ZR, KeyCode::E),
    (KeyPad::DPAD_UP, KeyCode::Up),
    (KeyPad::DPAD_DOWN, KeyCode::Down),
];

pub struct Input {
    /// The course cursor on the top screen, in its pixels.
    pub cursor: (f32, f32),
    touching: bool,
    /// Where the pen last was on the bottom screen, in its pixels.
    pen: (f32, f32),
    held: [bool; 2],
    /// L or R is down now, and whether a stylus drag has used it as the scroll modifier since it went down.
    shoulder_now: bool,
    shoulder_used: bool,
}

impl Input {
    pub fn new() -> Input {
        Input { cursor: (TOP_W as f32 * 0.5, TOP_H as f32 * 0.5), touching: false, pen: (0.0, 0.0), held: [false; 2], shoulder_now: false, shoulder_used: false }
    }

    /// L or R is held: the plan scrolls under a drag (plan.rs).
    pub fn shoulder(&self) -> bool {
        self.shoulder_now
    }

    /// Polls the controls once (call after `hid.scan_input()`) and returns this frame's events.
    pub fn poll(&mut self, hid: &Hid, cstick: bool) -> Vec<Event> {
        let mut ev = Vec::new();
        let down = hid.keys_down();
        let up = hid.keys_up();
        let held = hid.keys_held();
        let mods = KeyMods::default();

        for &(pad, key) in BUTTON_KEYS {
            if down.contains(pad) {
                ev.push(Event::KeyDown(key, mods));
            }
            if up.contains(pad) {
                ev.push(Event::KeyUp(key, mods));
            }
        }

        // circle pad: the course cursor. Dead zone, then a quadratic curve for fine placement near the centre.
        let (cx, cy) = hid.circlepad_position();
        let axis = |v: i16| {
            let f = v as f32 / 156.0;
            if f.abs() < 0.12 {
                0.0
            } else {
                f * f.abs() * 6.0
            }
        };
        let (dx, dy) = (axis(cx), -axis(cy));
        if dx != 0.0 || dy != 0.0 {
            self.cursor.0 = (self.cursor.0 + dx).clamp(0.0, TOP_W as f32 - 1.0);
            self.cursor.1 = (self.cursor.1 + dy).clamp(0.0, TOP_H as f32 - 1.0);
            ev.push(Event::MouseMove(self.cursor.0, self.cursor.1));
        }
        for (i, (pad, button)) in [(KeyPad::A, MouseButton::Left), (KeyPad::B, MouseButton::Right)].into_iter().enumerate() {
            if down.contains(pad) {
                self.held[i] = true;
                ev.push(Event::MouseDown(button, self.cursor.0, self.cursor.1));
            }
            if up.contains(pad) && self.held[i] {
                self.held[i] = false;
                ev.push(Event::MouseUp(button, self.cursor.0, self.cursor.1));
            }
        }

        // camera: C-stick (New 3DS, through libctru's irrst service; ctru-rs has no wrapper for it) or the D-pad
        let (sx, sy) = if cstick {
            let mut p = ctru_sys::circlePosition { dx: 0, dy: 0 };
            unsafe { ctru_sys::irrstCstickRead(&mut p) };
            (p.dx, p.dy)
        } else {
            (0, 0)
        };
        let mut pan = (sx as f32 / 146.0 * 4.0, -(sy as f32) / 146.0 * 4.0);
        if pan.0.abs() < 0.3 {
            pan.0 = 0.0;
        }
        if pan.1.abs() < 0.3 {
            pan.1 = 0.0;
        }
        if held.contains(KeyPad::DPAD_LEFT) {
            pan.0 -= 3.0;
        }
        if held.contains(KeyPad::DPAD_RIGHT) {
            pan.0 += 3.0;
        }
        if held.contains(KeyPad::DPAD_UP) {
            pan.1 -= 3.0;
        }
        if held.contains(KeyPad::DPAD_DOWN) {
            pan.1 += 3.0;
        }
        if pan != (0.0, 0.0) {
            ev.push(Event::Pan(pan.0, pan.1));
        }
        // L / R: a tap zooms (on release); held, they turn a stylus drag into scrolling the view, and then zoom nothing
        let shoulder = held.contains(KeyPad::L) || held.contains(KeyPad::R);
        if shoulder && held.contains(KeyPad::TOUCH) {
            self.shoulder_used = true;
        }
        if !self.shoulder_used {
            let zoom = up.contains(KeyPad::R) as i32 - up.contains(KeyPad::L) as i32;
            if zoom != 0 {
                ev.push(Event::Zoom(zoom));
            }
        }
        if !shoulder {
            self.shoulder_used = false;
        }
        self.shoulder_now = shoulder;

        // the touch screen: the bottom screen's interface
        let (tx, ty) = hid.touch_position();
        let touching = held.contains(KeyPad::TOUCH);
        let (vx, vy) = screens::touch_to_virtual(tx as f32, ty as f32);
        let phase = match (self.touching, touching) {
            (false, true) => Some(TouchPhase::Started),
            (true, true) => Some(TouchPhase::Moved),
            (true, false) => Some(TouchPhase::Ended),
            (false, false) => None,
        };
        if let Some(phase) = phase {
            // the position is read while the pen is down; on release the last one is repeated (hid reports 0, 0 then)
            if touching {
                self.pen = (tx as f32, ty as f32);
            }
            ev.push(Event::Touch(phase, 0, vx, vy));
            ev.push(Event::Pen(phase, self.pen.0, self.pen.1));
        }
        self.touching = touching;
        ev
    }
}
