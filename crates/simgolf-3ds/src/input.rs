//! The 3DS's controls as the events the game crate's `Stage` already handles (mouse, touch, keys) plus two it does not have
//! (camera pan and zoom as analogue values, which main.rs applies to the camera directly).
//!
//! Mapping (change the tables, not the code), the DS building games' layout:
//!   touch screen         the course (bottom screen), played with the stylus: `Pen` events in its pixels (plan.rs); also as
//!                        touch events in the original's 800 x 600 UI coordinates for the game's screens
//!   L / R                the top screen's tool category, previous / next (PageUp / PageDown)
//!   D-pad                left / right a tool, up / down the brush size (arrow keys); menus: choose
//!   A                    the chosen button acts (hire, fire, save, a page); menus: pick
//!   B                    clears a placed tee, else undo (menus: back)
//!   X                    pause (Space, as the original)
//!   Y                    game speed 1x, 2x, 4x (Tab)
//!   Circle Pad, C-stick  slide the view
//!   ZL (New 3DS)         swap the screens
//!   ZR (New 3DS)         tap: zoom in (E); hold: zoom out (Q)
//!   Start                the pause menu (Escape)
//!   Select               the top screen's pages: money, holes, golfers, staff, then the overview (F1)
//!   3D slider            read by gfx.rs each frame; nothing to map
use crate::screens::{self, TOP_H, TOP_W};
use crate::shim::{KeyCode, KeyMods, MouseButton, TouchPhase};
use ctru::services::hid::{Hid, KeyPad};
use std::sync::atomic::{AtomicU8, Ordering};

/// Left-handed play: the D-pad and the face buttons swap places (Y<->Up, A<->Right, B<->Down, X<->Left).
/// Toggled from the manual's touch screen, kept as a marker file, applied where the pad is read.
const LEFTY_FILE: &str = "sdmc:/3ds/opensgolf/.lefty";
static LEFTY: AtomicU8 = AtomicU8::new(2); // 2 = not read yet

pub fn lefty() -> bool {
    let v = LEFTY.load(Ordering::Relaxed);
    if v != 2 {
        return v == 1;
    }
    let on = std::fs::metadata(LEFTY_FILE).is_ok();
    LEFTY.store(on as u8, Ordering::Relaxed);
    on
}

/// Left-handed mode swaps the D-pad and A/B/X/Y: hint text names the button the player presses.
pub fn lh(s: &'static str) -> &'static str {
    const SW: &[(&str, &str)] = &[
        ("D-pad choose   A select   B back", "A/B/X/Y choose   D-Right select   D-Down back"),
        ("D-pad left / right", "X / A"),
        ("D-pad up / down", "Y / B"),
        ("choose it, then A", "choose it, then D-Right"),
        ("B", "D-Down"),
        ("X / Y", "D-Left / D-Up"),
        ("Circle Pad: scroll    L / R: section    B: close", "Circle Pad: scroll    L / R: section    D-Down: close"),
        ("B: back", "D-Down: back"),
        ("A", "D-Right"),
    ];
    if !lefty() {
        return s;
    }
    SW.iter().find(|(a, _)| *a == s).map(|(_, b)| *b).unwrap_or(s)
}

pub fn set_lefty(on: bool) {
    LEFTY.store(on as u8, Ordering::Relaxed);
    if on {
        let _ = std::fs::write(LEFTY_FILE, b"1");
    } else {
        let _ = std::fs::remove_file(LEFTY_FILE);
    }
}

fn remap(k: KeyPad) -> KeyPad {
    if !lefty() {
        return k;
    }
    let mut o = k;
    for (a, b) in [(KeyPad::Y, KeyPad::DPAD_UP), (KeyPad::A, KeyPad::DPAD_RIGHT), (KeyPad::B, KeyPad::DPAD_DOWN), (KeyPad::X, KeyPad::DPAD_LEFT)] {
        o.remove(a);
        o.remove(b);
        if k.contains(a) {
            o.insert(b);
        }
        if k.contains(b) {
            o.insert(a);
        }
    }
    o
}

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
    (KeyPad::L, KeyCode::PageUp),
    (KeyPad::R, KeyCode::PageDown),
    (KeyPad::DPAD_UP, KeyCode::Up),
    (KeyPad::DPAD_DOWN, KeyCode::Down),
    (KeyPad::DPAD_LEFT, KeyCode::Left),
    (KeyPad::DPAD_RIGHT, KeyCode::Right),
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
    /// ZL swapped the screens (the touchscreen is then not read).
    pub swapped: bool,
    /// Frames ZR has been held (tap = zoom in, hold = zoom out).
    zr_frames: u32,
}

impl Input {
    pub fn new() -> Input {
        Input { cursor: (TOP_W as f32 * 0.5, TOP_H as f32 * 0.5), touching: false, pen: (0.0, 0.0), held: [false; 2], shoulder_now: false, shoulder_used: false, swapped: false, zr_frames: 0 }
    }

    /// L or R is held: the plan scrolls under a drag (plan.rs).
    pub fn shoulder(&self) -> bool {
        self.shoulder_now
    }

    /// Polls the controls once (call after `hid.scan_input()`) and returns this frame's events.
    pub fn poll(&mut self, hid: &Hid, cstick: bool) -> Vec<Event> {
        let mut ev = Vec::new();
        let down = remap(hid.keys_down());
        let up = remap(hid.keys_up());
        let held = remap(hid.keys_held());
        let mods = KeyMods::default();

        // ZL swaps the screens. ZR: a tap zooms in, holding it zooms out step by step.
        if down.contains(KeyPad::ZL) {
            self.swapped = !self.swapped;
        }
        if held.contains(KeyPad::ZR) {
            self.zr_frames += 1;
            if self.zr_frames >= 20 && (self.zr_frames - 20) % 12 == 0 {
                ev.push(Event::KeyDown(KeyCode::Q, mods));
            }
        } else {
            if up.contains(KeyPad::ZR) && self.zr_frames > 0 && self.zr_frames < 20 {
                ev.push(Event::KeyDown(KeyCode::E, mods));
            }
            self.zr_frames = 0;
        }

        for &(pad, key) in BUTTON_KEYS {
            if down.contains(pad) {
                ev.push(Event::KeyDown(key, mods));
            }
            if up.contains(pad) {
                ev.push(Event::KeyUp(key, mods));
            }
        }

        // circle pad: slides the view (below, with the C-stick). Dead zone, then a quadratic curve for fine moves near the
        // centre.
        let (cx, cy) = hid.circlepad_position();
        let axis = |v: i16| {
            let f = v as f32 / 156.0;
            if f.abs() < 0.12 {
                0.0
            } else {
                f * f.abs() * 6.0
            }
        };
        let (pdx, pdy) = (axis(cx), -axis(cy));
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
        pan.0 += pdx;
        pan.1 += pdy;
        if pan != (0.0, 0.0) {
            ev.push(Event::Pan(pan.0, pan.1));
        }
        // L / R are the tool categories now (BUTTON_KEYS); nothing turns a stylus drag into scrolling (the MOVE tool does)
        self.shoulder_now = false;
        self.shoulder_used = false;

        // the touch screen: the bottom screen's interface
        let (tx, ty) = hid.touch_position();
        let touching = held.contains(KeyPad::TOUCH) && !self.swapped;
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
