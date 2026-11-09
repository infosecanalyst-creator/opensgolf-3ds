//! Touch controls for phones and tablets (the browser build and the Android app). The original is a mouse and keyboard game, so this is the port's
//! own layer: it turns touches into the mouse clicks and keys the game already understands.
//!
//! * One finger: a tap is a left click; dragging pans the course (or paints with the current tool); holding still for half a
//!   second is a right click (undo, remove, close a screen). With the raise / lower tool armed, holding still over the course
//!   lowers the ground instead (the desktop's Shift+click, which a tablet has no key for), and dragging on from there keeps
//!   lowering.
//! * Two fingers: dragging pans, pinching steps through the three zoom levels, twisting turns the view a quarter.
//! * A few round touch buttons down the right edge stand in for keys the screen has no button for (game speed, repeat the
//!   last message, turn the building, open the hole, the showcase fly-over, undo); they show only once the screen has been
//!   touched, Turn and Open hole only while they have something to do, Showcase once a hole is open, Undo (left of Speed)
//!   while building with a click to undo.
//! * Aiming the player's pro (the port's own): a mouse aims by hovering and shoots with a click, but a finger has no hover,
//!   so its first tap both aimed and shot. With a finger, a tap on the course sets the target and a one-finger drag moves it
//!   (held clear of the finger, with its distance and club beside it); lifting the finger does not shoot. A round **Swing**
//!   button then takes the shot at the target, exactly the shot a click there takes. Two fingers still move the camera and
//!   a long press is still a right click; the shot shapes stay on the Player panel.
//! * Looking round while aiming (the port's own, issue #75): a one-finger drag moves the target only when it starts on or
//!   near it; a drag anywhere else on the course pans the view (it never shoots), a target dragged to the screen's edge
//!   scrolls the view, a small round button centres the view on the pro's ball, and the first touch aim shows a short hint.
//! * Mouse aim and swing (an OpenSGolf extra, off by default; extras.rs mouse_swing): the mouse aims the pro's shot through
//!   this same path, as a finger whose press is the left button's: a click sets the target, a drag from it moves it (lifted
//!   above the pointer, with the line and the plate), a drag elsewhere looks round, a target dragged to the edge scrolls,
//!   and the Swing and Centre buttons show and take a click. Only the shot: buildings, the touch buttons' column, the long
//!   press and the phone's keyboard stay a finger's; the right button, the wheel and the keys are untouched.
//! * When a name is typed (rename, save) the phone's keyboard is raised: through the page in a browser (web/opensgolf.js),
//!   through miniquad on Android.

use crate::app::*;
use crate::gfx::Gfx;
use crate::ui::{rgb, rgba, Screen as Ui};
use crate::Stage;
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton, TouchPhase};
use sg_core::lang::{tr, trf};

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
    /// One finger moving the pro's aim target (TouchTarget).
    Aim,
    /// One finger panning the course while a touch target is set (a drag away from the target; nothing is clicked).
    Pan,
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
    /// The touch button under the finger: it acts when the finger lifts on it (sliding off lets it be).
    held: Option<Btn>,
    /// The button last let go and when, lit for a moment.
    flash: Option<(Btn, f64)>,
    /// The mouse events under way come from a finger (the click a tap makes, a drag): the course aims instead of shooting.
    feeding: bool,
    /// The look-around hint has been shown (once a run, the first time the pro is aimed by touch).
    hinted: bool,
    /// The clock at the last edge scroll of a dragged target (the next step's time is measured from it).
    edge_at: f64,
    /// The finger down is the mouse's left button (Mouse aim and swing), from its press to its release.
    mouse: bool,
}

/// The finger id the mouse's left button goes by (Mouse aim and swing); a real touch's ids are small.
const MOUSE_ID: u64 = u64::MAX;

/// How far above the finger a target it picks up is held, in device-independent pixels (a fingertip and a half), so the
/// finger does not hide it.
const AIM_LIFT: f32 = 64.0;
/// How near the target a one-finger drag must start to move it, in device-independent pixels (the ring and the plate above
/// it, with room for a fingertip); a drag that starts further away pans the view.
const AIM_GRAB: f32 = 80.0;
/// The band along the course's edges, in device-independent pixels, where a dragged target scrolls the view.
const EDGE_BAND: f32 = 48.0;
/// The edge scroll's top speed, in screen heights a second (at the very edge).
const EDGE_SPEED: f32 = 0.9;

/// Whether a one-finger drag starting at `finger` moves the target (drawn at `target_px`, None when there is none or it is
/// off the screen) rather than panning the view: only when it starts within `grab` of it.
fn grabs_target(finger: (f32, f32), target_px: Option<(f32, f32)>, grab: f32) -> bool {
    target_px.is_some_and(|t| (t.0 - finger.0).hypot(t.1 - finger.1) <= grab)
}

/// How hard a dragged target at `p` pushes the view towards each edge of the course's area `(left, top, right, bottom)`:
/// -1 to 1 on each axis (negative left and up), 0 outside the band of `band` pixels along the edges, growing to 1 at the
/// edge and past it.
fn edge_push(p: (f32, f32), area: (f32, f32, f32, f32), band: f32) -> (f32, f32) {
    let (l, t, r, b) = area;
    let side = |v: f32, lo: f32, hi: f32| {
        if v < lo + band {
            -((lo + band - v) / band).min(1.0)
        } else if v > hi - band {
            ((v - (hi - band)) / band).min(1.0)
        } else {
            0.0
        }
    };
    (side(p.0, l, r), side(p.1, t, b))
}

/// Whether the segment from `p` to `q` touches the rectangle `(left, top, width, height)` (Liang-Barsky).
fn crosses((x, y, w, h): (f32, f32, f32, f32), p: (f32, f32), q: (f32, f32)) -> bool {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (num, den) in [(p.0 - x, -dx), (x + w - p.0, dx), (p.1 - y, -dy), (y + h - p.1, dy)] {
        if den == 0.0 {
            if num < 0.0 {
                return false;
            }
        } else if den < 0.0 {
            t0 = t0.max(num / den);
        } else {
            t1 = t1.min(num / den);
        }
    }
    t0 <= t1
}

/// Where the shot's plate (`w` x `h`, its left and top) goes round the aimed spot `at`, `gap` clear of it (side to side, up
/// and down): above, where a finger dragging the target (below it) does not cover it, unless the shot's lines (`lines`,
/// 800 x 600 points from the ball) run through it there, as the arc of a high shot coming down on the spot does; then
/// beside the spot on the side away from the ball (`ball_left`: the ball is left of the spot), the other side or below,
/// whichever the lines cross least. Kept on the screen side to side.
fn plate_spot(at: (f32, f32), (w, h): (f32, f32), gap: (f32, f32), lines: &[Vec<(f32, f32)>], ball_left: bool) -> (f32, f32) {
    let (vx, vy) = at;
    let (above, below) = ((vx - w * 0.5, vy - gap.1 - h), (vx - w * 0.5, vy + gap.1));
    let (right, left) = ((vx + gap.0, vy - h * 0.5), (vx - gap.0 - w, vy - h * 0.5));
    let order = if ball_left { [above, right, left, below] } else { [above, left, right, below] };
    let hits = |&(x, y): &(f32, f32)| {
        let r = (x - 2.0, y - 2.0, w + 4.0, h + 4.0);
        lines.iter().flat_map(|l| l.windows(2)).filter(|s| crosses(r, s[0], s[1])).count()
    };
    // (the first of the least crossed, in that order)
    order.into_iter().map(|(x, y)| (x.clamp(2.0, (798.0 - w).max(2.0)), y)).min_by_key(hits).unwrap_or(above)
}

/// What a touch target is for: the pro's aim, or the building tool's next placement (with its kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetFor {
    Aim,
    Place(usize),
}

/// Aiming the player's pro and placing buildings by touch (the port's own). The original previews under the hovering
/// pointer (the aim at the hovered tile, 0x41b20f every frame; the building's footprint) and acts on a click (the shot,
/// 0x41f29d; the placement); a finger cannot hover, so its first tap did both at once. Here a tap sets a target, a drag
/// moves it, lifting the finger leaves it, and the Swing or Place button acts on it through the calls a click on the same
/// ground point makes. A mouse moved since puts hovering back.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TouchTarget {
    /// The last pointer was a finger.
    pub on: bool,
    /// Mouse aim and swing is on (extras.rs, set every frame): the mouse aims the pro's shot as a finger does (not buildings).
    pub mouse: bool,
    /// The ground point (world x, z): it stays on the ground while the camera moves.
    pub target: Option<(f32, f32)>,
    /// What the target is for: it goes when that changes (a new shot, another tool or kind).
    pub what: Option<TargetFor>,
    /// A drag under way: where the finger went down, and where the target stood on the screen then (drawable pixels).
    drag: Option<((f32, f32), (f32, f32))>,
}

impl TouchTarget {
    /// A finger touched the screen.
    pub fn touched(&mut self) {
        self.on = true;
    }

    /// A mouse moved (not a finger's): hovering again, no target (Mouse aim and swing stays as it is).
    pub fn pointer_moved(&mut self) {
        *self = Self { mouse: self.mouse, ..Self::default() };
    }

    /// Whether targets are set by touch for `what` (None: nothing to target now): after a touch, or for the shot with Mouse
    /// aim and swing.
    pub fn active(&self, what: Option<TargetFor>) -> bool {
        (self.on && what.is_some()) || (self.mouse && what == Some(TargetFor::Aim))
    }

    /// A tap on the course: the target is that ground point.
    pub fn tap(&mut self, at: (f32, f32)) {
        self.target = Some(at);
        self.drag = None;
    }

    /// A drag begins from `finger`; `target_px` is where the target is on the screen (None when there is none or it is off
    /// the screen). A target away from the finger moves with it from where it is, as on a trackpad; one under the finger, or
    /// none, is picked up `lift` pixels above the finger, where the finger does not cover it.
    pub fn drag_start(&mut self, finger: (f32, f32), target_px: Option<(f32, f32)>, lift: f32) {
        let away = |t: (f32, f32)| (t.0 - finger.0).hypot(t.1 - finger.1) >= lift * 0.75;
        let at = target_px.filter(|&t| away(t)).unwrap_or((finger.0, finger.1 - lift));
        self.drag = Some((finger, at));
    }

    /// Where the target goes with the finger at `finger` during a drag (drawable pixels).
    pub fn drag_point(&self, finger: (f32, f32)) -> Option<(f32, f32)> {
        self.drag.map(|(f0, at)| (at.0 + finger.0 - f0.0, at.1 + finger.1 - f0.1))
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The finger lifted (or a second one came): the target stays where it is, and nothing happens.
    pub fn drag_end(&mut self) {
        self.drag = None;
    }

    /// The Swing or Place button shows: targeting by touch for `what` with a target set.
    pub fn ready(&self, what: Option<TargetFor>) -> bool {
        self.active(what) && self.what == what && self.target.is_some()
    }

    /// Every frame, with what a target would be for now: when that changes (the shot taken, a new shot, another tool or
    /// kind) the target goes, so the next one starts without one.
    pub fn sync(&mut self, what: Option<TargetFor>) {
        if what != self.what {
            self.what = what;
            self.target = None;
            self.drag = None;
        }
    }
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

/// The touch buttons, top to bottom down the right edge: what each stands in for (an action of controls.rs, run as its key
/// would be whatever key the player gave it) and its name, shown beside it while a finger is on it. Speed and Message are
/// always there and come first, so the two that come and go (Turn with something to turn, Open hole with a hole ready)
/// never move the others.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Btn {
    Speed,
    Message,
    Turn,
    OpenHole,
    /// The showcase fly-over (showcase.rs), last, below the others.
    Showcase,
    /// Aiming by touch: the shot at the target (TouchTarget). Bigger, and on its own to the left of the column's foot.
    Swing,
    /// Placing by touch: the building at the target. Bigger, to the left of Turn.
    Place,
    /// Aiming by touch: the view back on the pro's ball (issue #75). Above Swing, shown all the time the pro waits.
    Centre,
    /// Ctrl+Z (undo.rs), which a phone has no key for: while building, with a click to undo. Left of Speed, clear of Place.
    Undo,
}
/// The column's buttons: those before Swing.
#[cfg(test)]
const COLUMN: usize = 5;
const BUTTONS: [Btn; 9] =
    [Btn::Speed, Btn::Message, Btn::Turn, Btn::OpenHole, Btn::Showcase, Btn::Swing, Btn::Place, Btn::Centre, Btn::Undo];

impl Btn {
    fn action(self) -> &'static str {
        match self {
            Btn::Speed => "speed",
            Btn::Message => "message",
            Btn::Turn => "turn_building",
            Btn::OpenHole => "open_hole",
            Btn::Showcase => "showcase",
            // no key: the original shoots with a click (touch_button_press)
            Btn::Swing | Btn::Place | Btn::Centre => "",
            // Ctrl+Z is not one of the keys to set (main.rs key_down_event; touch_button_press)
            Btn::Undo => "",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Btn::Speed => "STR_TOUCH_SPEED",
            Btn::Message => "STR_TOUCH_MESSAGE",
            Btn::Turn => "STR_TOUCH_TURN",
            Btn::OpenHole => "STR_TOUCH_OPEN_HOLE",
            Btn::Showcase => "STR_TOUCH_SHOWCASE",
            Btn::Swing => "STR_TOUCH_SWING",
            Btn::Place => "STR_TOUCH_PLACE",
            Btn::Centre => "STR_TOUCH_CENTRE",
            Btn::Undo => "STR_TOUCH_UNDO",
        }
    }
}

/// The column's top: under the money, fun and rating gauges (to y 130) and Enhanced graphics' camera button (140 to 164).
const COLUMN_TOP: f32 = 176.0;
/// The buttons' radius in 800 x 600 units: about 36 pixels across on the device's screen (CSS pixels in a browser), kept
/// between the size of the dock's Information button and a size that still leaves the column clear of the build panel.
const R_MIN: f32 = 15.0;
const R_MAX: f32 = 23.0;
const R_SCREEN: f32 = 18.0;
/// The room round a button that still counts as on it (fingers are wider than the art).
const HIT_PAD: f32 = 5.0;
/// Seconds a button stays lit after it is let go.
const FLASH: f64 = 0.25;

/// A button's radius for this many device-independent pixels to an 800 x 600 unit.
fn button_radius(px_per_unit: f32) -> f32 {
    (R_SCREEN / px_per_unit.max(0.01)).clamp(R_MIN, R_MAX)
}

/// The left edge of the OpenSGolf extras' legends at the top right (the path view, the staff areas, the landing map: x 566,
/// y 140 to 430, further left with bigger text): while one is up the column stands to its left instead of over it.
const LEGEND_LEFT: f32 = 566.0;

/// Slot `i`'s centre for radius `r` with the column's right edge at `right`, in the 800 x 600 layout (held to the right edge
/// with Enhanced widescreen).
fn button_centre(i: usize, r: f32, right: f32) -> (f32, f32) {
    (right - 6.0 - r, COLUMN_TOP + r + i as f32 * (2.0 * (r + HIT_PAD)))
}

/// The Swing and Place buttons' radius for the column's radius `r`.
fn big_radius(r: f32) -> f32 {
    r * 1.6
}

/// The Swing button's centre for the column's radius `r` and right edge `right`: left of the column's foot, just above the
/// dock's panel (y 452), where the thumb of the hand that holds that side reaches it.
fn swing_centre(r: f32, right: f32) -> (f32, f32) {
    let rs = big_radius(r);
    (right - 6.0 - 2.0 * r - 14.0 - rs, 452.0 - 12.0 - rs)
}

/// The Place button's centre: to the left of Turn (slot 2), which turns the building it places.
fn place_centre(r: f32, right: f32) -> (f32, f32) {
    let (x, y) = button_centre(2, r, right);
    (x - r - 14.0 - big_radius(r), y)
}

/// The Centre button's centre: above Swing, level with its right side.
fn centre_centre(r: f32, right: f32) -> (f32, f32) {
    let (x, y) = swing_centre(r, right);
    let rs = big_radius(r);
    (x + rs - r, y - rs - 14.0 - r)
}

/// The Undo button's centre: to the left of Speed (slot 0), two rows above Place, so the two never touch.
fn undo_centre(r: f32, right: f32) -> (f32, f32) {
    let (x, y) = button_centre(0, r, right);
    (x - 2.0 * r - 14.0, y)
}

/// Whether (vx, vy) is on the button centred at `c`.
fn on_button(c: (f32, f32), r: f32, vx: f32, vy: f32) -> bool {
    let h = r + HIT_PAD;
    (vx - c.0).abs() <= h && (vy - c.1).abs() <= h
}

fn two(f: &[Finger]) -> ((f32, f32), f32, f32) {
    let (a, b) = (f[0].2, f[1].2);
    let mid = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    (mid, (dx * dx + dy * dy).sqrt().max(1.0), dy.atan2(dx))
}

impl Stage {
    pub(crate) fn touch(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        self.touch.feeding = true;
        self.touch_finger(phase, id, x, y);
        self.touch.feeding = false;
    }

    /// Whether the mouse event under way comes from a finger (main.rs: a tap on the course aims instead of shooting).
    pub(crate) fn by_finger(&self) -> bool {
        self.touch.feeding
    }

    fn touch_finger(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        // the mouse's left button (Mouse aim and swing) is no touch: the touch buttons, the name tags and the touch targets
        // for buildings stay as they are
        let finger = !self.touch.mouse;
        if finger {
            self.touch.used = true;
            // no pointer to hover with: the extras' name tags stay off (tags_ui)
            self.app.xs.touched = true;
        }
        let now = self.app.clock;
        match phase {
            TouchPhase::Started => {
                if finger {
                    self.app.touch_target.touched();
                }
                self.touch.fingers.retain(|f| f.0 != id);
                self.touch.fingers.push((id, (x, y), (x, y)));
                match self.touch.fingers.len() {
                    1 => {
                        if self.osk_press(x, y) {
                            self.touch.mode = Mode::Spent;
                        } else if let Some(b) = self.touch_button_at(x, y) {
                            // the button takes the whole touch: nothing reaches the course or the interface under it
                            self.touch.held = Some(b);
                            self.touch.mode = Mode::Spent;
                        } else {
                            self.touch.mode = Mode::Pending;
                            self.touch.started = now;
                        }
                    }
                    2 => {
                        // a second finger ends whatever the first was doing
                        self.touch.held = None;
                        // a target being dragged stays where it is, a one-finger pan ends
                        self.app.touch_target.drag_end();
                        if self.touch.mode == Mode::Pan {
                            self.app.drag_state(false, now);
                        }
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
                let prev = f.2;
                f.2 = (x, y);
                let start = f.1;
                if self.touch.held.is_some() && self.touch_button_at(x, y) != self.touch.held {
                    self.touch.held = None;
                }
                match self.touch.mode {
                    Mode::Pending => {
                        let (dx, dy) = (x - start.0, y - start.1);
                        let far = dx * dx + dy * dy > SLOP * SLOP * self.app.dpi * self.app.dpi;
                        if far && self.targets_here(start) {
                            let px = self.app.touch_target.target.map(|(wx, wz)| self.app.pixel_of_world(wx, wz));
                            let on_screen = px.filter(|p| (0.0..self.app.draw_w).contains(&p.0) && (0.0..self.app.draw_h).contains(&p.1));
                            if grabs_target(start, on_screen, AIM_GRAB * self.app.dpi) {
                                // the drag moves the target, and nothing is clicked
                                self.app.touch_target.drag_start(start, on_screen, AIM_LIFT * self.app.dpi);
                                self.touch.mode = Mode::Aim;
                                self.touch.edge_at = now;
                                self.target_drag(x, y);
                            } else {
                                // away from the target (or before one is set): the view pans, and nothing is clicked
                                // (a left press would shoot; issue #75)
                                self.app.drag_state(true, now);
                                self.touch.mode = Mode::Pan;
                                self.finger_pan(x - start.0, y - start.1);
                            }
                        } else if far && (self.picks_spot(start) || self.pans_here(start)) {
                            // an employee or a golfer waits for the tap that says where to go: a drag looks around for the
                            // spot and posts nobody (a left press at the drag's start did, issue #86); and with no tool in
                            // hand a drag only looks round: as a mouse drag it began with a left press, which picked up
                            // the employee under the finger, or opened the golfer's card or the flag's Hole Stats
                            self.app.drag_state(true, now);
                            self.touch.mode = Mode::Pan;
                            self.finger_pan(x - start.0, y - start.1);
                        } else if far {
                            self.mouse_motion_event(start.0, start.1);
                            self.mouse_button_down_event(MouseButton::Left, start.0, start.1);
                            self.touch.mode = Mode::Mouse;
                            self.mouse_motion_event(x, y);
                        }
                    }
                    Mode::Mouse => self.mouse_motion_event(x, y),
                    Mode::Aim => self.target_drag(x, y),
                    Mode::Pan => self.finger_pan(x - prev.0, y - prev.1),
                    Mode::Gesture if self.touch.fingers.len() >= 2 => self.gesture(),
                    _ => {}
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let was = self.touch.mode;
                let last = self.touch.fingers.len() == 1;
                self.touch.fingers.retain(|f| f.0 != id);
                if let Some(b) = self.touch.held.take() {
                    if phase == TouchPhase::Ended && self.touch_button_at(x, y) == Some(b) {
                        self.touch.flash = Some((b, now));
                        self.touch_button_press(b);
                    }
                }
                match was {
                    Mode::Pending if phase == TouchPhase::Ended => {
                        // a tap: point where the finger was, then press and release once the screen has seen the pointer
                        self.mouse_motion_event(x, y);
                        self.touch.queued = Some((MouseButton::Left, x, y, 2));
                    }
                    Mode::Mouse => self.mouse_button_up_event(MouseButton::Left, x, y),
                    // lifting the finger leaves the target where it is: the Swing button shoots
                    Mode::Aim => self.app.touch_target.drag_end(),
                    Mode::Pan => self.app.drag_state(false, now),
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

    /// Mouse aim and swing (extras.rs mouse_swing): a left press on the Swing or Centre button, or (`course`, asked once the
    /// dock, the panel, the card and the ticker have had the press) on the course while the pro waits for the shot, goes the
    /// touch path as a finger going down there. True when it was taken so; its moves and release follow it there.
    pub(crate) fn mouse_swing_down(&mut self, x: f32, y: f32, course: bool) -> bool {
        // a press whose release never came (let go outside the window) is let go first
        if self.touch.mouse && !self.by_finger() {
            self.touch(TouchPhase::Cancelled, MOUSE_ID, x, y);
            self.touch.mouse = false;
        }
        if !crate::extras::mouse_swing()
            || self.by_finger()
            || self.touch.mode != Mode::Idle
            || !self.touch.fingers.is_empty()
            || self.app.touch_target_for() != Some(TargetFor::Aim)
        {
            return false;
        }
        let take = if course { self.targets_here((x, y)) } else { matches!(self.touch_button_at(x, y), Some(Btn::Swing | Btn::Centre)) };
        if take {
            self.touch.mouse = true;
            self.touch(TouchPhase::Started, MOUSE_ID, x, y);
        }
        take
    }

    /// The mouse moved with its left button down after a press `mouse_swing_down` took: the finger moves.
    pub(crate) fn mouse_swing_move(&mut self, x: f32, y: f32) {
        if self.touch.mouse && !self.by_finger() {
            self.touch(TouchPhase::Moved, MOUSE_ID, x, y);
        }
    }

    /// The left button let go after a press `mouse_swing_down` took: the finger lifts (a click that did not move is a
    /// tap). True when it was so.
    pub(crate) fn mouse_swing_up(&mut self, x: f32, y: f32) -> bool {
        if !self.touch.mouse || self.by_finger() {
            return false;
        }
        self.touch(TouchPhase::Ended, MOUSE_ID, x, y);
        self.touch.mouse = false;
        true
    }

    /// One finger panning the view while a touch target is set: the course follows the finger as a mouse drag moves it
    /// (main.rs mouse_motion_event), with the smooth camera's inertia.
    fn finger_pan(&mut self, dx: f32, dy: f32) {
        let k = self.app.upp;
        let sin_p = (pitch_for(self.app.draw_w, self.app.draw_h).to_radians()).sin() as f32;
        self.app.xs.follow = None;
        self.app.drag_pan(-dx * k, dy * k / sin_p, self.app.clock);
    }

    /// A target dragged into the band along the course's edges scrolls the view that way, faster nearer the edge, and stays
    /// under the finger (it moves over the ground): long shots can be aimed past the screen. Called every frame.
    fn edge_scroll(&mut self) {
        let now = self.app.clock;
        let dt = (now - self.touch.edge_at).clamp(0.0, 0.05) as f32;
        self.touch.edge_at = now;
        if self.touch.mode != Mode::Aim || dt <= 0.0 {
            return;
        }
        let Some(finger) = self.touch.fingers.first().map(|f| f.2) else { return };
        let Some(p) = self.app.touch_target.drag_point(finger) else { return };
        let a = &self.app;
        // the course's area: the window above the dock's panel (y 452 of the 800 x 600 screen)
        let bottom = (a.view.oy + 452.0 * a.view.scale).min(a.draw_h);
        let (px, py) = edge_push(p, (0.0, 0.0, a.draw_w, bottom), EDGE_BAND * a.dpi);
        if px == 0.0 && py == 0.0 {
            return;
        }
        let step = EDGE_SPEED * a.draw_h * dt;
        let sin_p = (pitch_for(a.draw_w, a.draw_h).to_radians()).sin() as f32;
        let k = a.upp;
        self.app.pan(px * step * k, -py * step * k / sin_p);
        self.target_drag(finger.0, finger.1);
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
                    // Zoom towards the pointer (views.rs): the ground between the fingers keeps its place
                    self.app.zoom_about(mid, |a| a.zoom_by(ratio.log2()));
                    self.touch.spread = spread;
                }
            } else if !(0.74..=1.35).contains(&ratio) {
                self.app.zoom_about(mid, |a| a.zoom_step(ratio > 1.0));
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
        self.edge_scroll();
        // the first touch aim: how to look round (once a run; issue #75)
        if !self.touch.hinted && self.app.touch_target.active(Some(TargetFor::Aim)) && self.app.touch_target_for() == Some(TargetFor::Aim) {
            self.touch.hinted = true;
            // (the mouse's own words with Mouse aim and swing, before any touch)
            self.app.show_toast(tr(if self.app.touch_target.on { "STR_TOUCH_LOOK_HINT" } else { "STR_MOUSE_SWING_HINT" }));
        }
        // a mouse button held still is no right click: the right button is there for that
        if self.touch.mode == Mode::Pending && !self.touch.mouse && self.app.clock - self.touch.started > LONG_PRESS {
            let p = self.touch.fingers.first().map(|f| f.1);
            match p {
                // the raise / lower tool over the course: the press lowers, as Shift+click does (issue #39), and the finger
                // drags on lowering until it lifts
                Some(p) if self.lowers_here(p) => {
                    self.touch.feeding = true;
                    self.mouse_motion_event(p.0, p.1);
                    self.shift = true;
                    self.touch.lowering = true;
                    self.mouse_button_down_event(MouseButton::Left, p.0, p.1);
                    self.touch.feeding = false;
                    self.touch.mode = Mode::Mouse;
                }
                Some(p) => {
                    self.touch.feeding = true;
                    self.mouse_motion_event(p.0, p.1);
                    self.touch.feeding = false;
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
                self.touch.feeding = true;
                self.mouse_button_down_event(b, x, y);
                self.mouse_button_up_event(b, x, y);
                self.touch.feeding = false;
            }
        }
        let typing = self.app.typing();
        if self.touch.used && typing != self.touch.keyboard {
            self.touch.keyboard = typing;
            show_keyboard(typing);
        }
    }

    /// Whether a one-finger drag from this drawable-pixel point moves a touch target (TouchTarget): there is something to
    /// target and the point is on the course where the pointer would preview it (above the panel, off the dock).
    fn targets_here(&self, p: (f32, f32)) -> bool {
        let a = &self.app;
        if a.popup.is_some() || a.notice.is_some() || a.touch_target_for().is_none() {
            return false;
        }
        let (vx, vy) = a.view.to_virtual(p.0, p.1);
        vy < 452.0 && !(a.ui_ok && crate::render::dock_hit(vx, vy) >= 0)
    }

    /// Whether a one-finger drag from this drawable-pixel point is on the course while an employee (the Employee panel's or
    /// the People screen's Move, or one picked up on the course) or a golfer (the card's Move or Eject) waits for the tap
    /// on its new spot.
    fn picks_spot(&self, p: (f32, f32)) -> bool {
        let a = &self.app;
        if a.screen != Screen::Play || a.popup.is_some() || a.notice.is_some() {
            return false;
        }
        if a.moving_employee.is_none() && a.card_ui.held.is_none() {
            return false;
        }
        let (vx, vy) = a.view.to_virtual(p.0, p.1);
        vy < 452.0 && !(a.ui_ok && crate::render::dock_hit(vx, vy) >= 0)
    }

    /// Whether a one-finger drag from this drawable-pixel point only pans the view: on the course with no tool in hand (a
    /// tool's drag paints) and nothing over it that a press there closes or works (the photo mode, the showcase, a great
    /// shot's SimFoto, the shot analysis, the measuring tape, a name being typed). A tap still clicks there.
    fn pans_here(&self, p: (f32, f32)) -> bool {
        let a = &self.app;
        let over = a.photo.is_some()
            || a.showcase.is_some()
            || a.great_shot.is_some()
            || a.simfoto.is_some()
            || a.analysis.is_some()
            || a.bt.tape.is_some()
            || a.typing();
        if a.screen != Screen::Play || a.edit || a.popup.is_some() || a.notice.is_some() || over {
            return false;
        }
        let (vx, vy) = a.view.to_virtual(p.0, p.1);
        vy < 452.0 && !(a.ui_ok && crate::render::dock_hit(vx, vy) >= 0)
    }

    /// The target is at ground point `w`, and the preview with it: the shot's (aim_pointer, as the pointer's hover does) or
    /// the building's footprint (the hover's ground point, main.rs update).
    fn set_target(&mut self, w: (f32, f32)) {
        let what = self.app.touch_target_for();
        self.app.touch_target.sync(what);
        self.app.touch_target.target = Some(w);
        match what {
            Some(TargetFor::Aim) => self.app.aim_pointer(w.0, w.1),
            Some(TargetFor::Place(_)) => {
                (self.app.has_hit, self.app.hit_x, self.app.hit_z) = (true, w.0, w.1);
                if !self.app.rec.replaying() {
                    self.app.hover_tile = Some(self.app.terrain.tile_of(w.0, w.1));
                }
            }
            None => {}
        }
    }

    /// A finger dragging the target is at (x, y): the target follows it, held where the drag picked it up.
    fn target_drag(&mut self, x: f32, y: f32) {
        let Some(p) = self.app.touch_target.drag_point((x, y)) else { return };
        if let Some(w) = self.app.pick_ground(p.0, p.1) {
            self.set_target(w);
        }
    }

    /// A tap on the course with something to target (main.rs, in place of the click's shot or placement): the target is
    /// there. True when it was taken so.
    pub(crate) fn target_tap(&mut self, x: f32, y: f32) -> bool {
        if !self.touch.feeding || !self.app.touch_target.active(self.app.touch_target_for()) {
            return false;
        }
        if let Some(w) = self.app.pick_ground(x, y) {
            self.set_target(w);
            self.app.touch_target.tap(w);
        }
        true
    }

    /// The Swing or Place button: the shot or the placement at the target, through what a left click on that ground point
    /// does (main.rs course_press and course_release; for the shot aim_pointer, then aim_click). A shot the game refuses
    /// (0x41f29d's aim lock) keeps the target; a placement lets it go, as the spot is taken.
    fn confirm(&mut self) {
        let what = self.app.touch_target_for();
        if !self.app.touch_target.ready(what) {
            return;
        }
        let Some(w) = self.app.touch_target.target else { return };
        self.set_target(w);
        match what {
            Some(TargetFor::Aim) => {
                self.app.aim_click();
                if self.app.aim.is_none() {
                    self.app.touch_target.target = None;
                }
            }
            Some(TargetFor::Place(_)) => {
                self.course_press(Some(w));
                self.course_release();
                self.app.touch_target.target = None;
            }
            None => {}
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

    /// The touch buttons' radius for this window, in 800 x 600 units.
    fn button_radius(&self) -> f32 {
        button_radius(self.app.view.scale / self.app.dpi.max(0.5))
    }

    /// Button `i`'s centre and radius on this frame.
    fn slot(&self, i: usize) -> ((f32, f32), f32) {
        let r = self.button_radius();
        let right = if self.app.right_legend_up() { self.app.legend_left() } else { 800.0 };
        match BUTTONS[i] {
            Btn::Swing => (swing_centre(r, right), big_radius(r)),
            Btn::Place => (place_centre(r, right), big_radius(r)),
            Btn::Centre => (centre_centre(r, right), r),
            Btn::Undo => (undo_centre(r, right), r),
            _ => (button_centre(i, r, right), r),
        }
    }

    /// Whether button `b` is there to press: the column with the interface up, Swing whenever a touch aim is set (the
    /// interface hidden with u still aims).
    fn button_shown(&self, b: Btn) -> bool {
        if matches!(b, Btn::Swing | Btn::Place | Btn::Centre) {
            let a = &self.app;
            let what = a.touch_target_for();
            let mine = match what {
                Some(TargetFor::Aim) => b == Btn::Swing || b == Btn::Centre,
                Some(TargetFor::Place(_)) => b == Btn::Place,
                None => false,
            };
            return (self.touch.used || a.touch_target.mouse)
                && a.screen == Screen::Play
                && a.photo.is_none()
                && a.notice.is_none()
                && a.great_shot.is_none()
                && a.simfoto.is_none()
                && a.analysis.is_none()
                && a.popup.is_none()
                && !a.typing()
                && mine
                // Centre whenever the pro waits for a touch aim, the others once their target is set
                && if b == Btn::Centre { a.touch_target.active(what) } else { a.touch_target.ready(what) };
        }
        self.touch_buttons_shown() && self.app.touch_button_wanted(b)
    }

    /// The touch button shown at this drawable-pixel point, if any.
    fn touch_button_at(&self, x: f32, y: f32) -> Option<Btn> {
        // in the column's own layout: with Enhanced widescreen it is drawn at the window's right edge, and a press in the room
        // round a button (or on one that has just appeared) is not on anything drawn the last frame for unshift to find
        let s = Ui::new(self.app.draw_w, self.app.draw_h);
        let (vx, vy) = s.view.to_virtual_raw(x, y);
        let (dx, dy) = s.anchor_shift(crate::enhanced::Anchor::Right);
        let (vx, vy) = (vx - dx, vy - dy);
        BUTTONS
            .iter()
            .enumerate()
            .find(|&(i, &b)| {
                self.button_shown(b) && {
                    let (c, r) = self.slot(i);
                    on_button(c, r, vx, vy)
                }
            })
            .map(|(_, &b)| b)
    }

    /// A touch button let go: its action, as its key does on the course (play_key after the player's own keys are read, so a
    /// key moved elsewhere, a shot shape on the same key or a Shift left down cannot change what the button does).
    fn touch_button_press(&mut self, b: Btn) {
        if matches!(b, Btn::Swing | Btn::Place) {
            self.confirm();
            return;
        }
        if b == Btn::Centre {
            self.app.centre_on_pro();
            return;
        }
        // the speed's one button cycles normal, x2 and x4 (the keys step and stop: App::step_speed)
        if b == Btn::Speed {
            self.app.cycle_speed();
            return;
        }
        if b == Btn::Undo {
            // as Ctrl+Z does (main.rs key_down_event)
            if !self.editing && self.app.undo_step(&mut self.g) {
                self.app.ui_sound(0x18);
            }
            return;
        }
        let i = crate::controls::action(b.action());
        let d = crate::controls::ACTIONS[i].default;
        self.play_action(d.key, d.shift, Some(i));
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
        let key = |me: &mut Self, code| {
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
            // bigger with the text size, as far as the key holds it (textsize.rs)
            let size = crate::textsize::fit_px(15.0, 2.0, &label, w - 6.0).min(h - 4.0).max(15.0);
            s.text_centered(g, x + w * 0.5, y + h * 0.5 + 5.0 + ((size - 15.0) / 3.0).floor(), &label, size, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }

    /// Test hooks for scripted stills (after SG_CLICK, before SG_PRESS): SG_TOUCH=1 counts the screen as touched (the touch
    /// buttons show); SG_TAP="x,y;hold:x,y;down:x,y;key:Tab;staff;..." taps, holds for a long press, or puts a finger down and
    /// leaves it there (for stills of a pressed button) at window pixels, through the same path a finger takes, or presses a
    /// key (settings.json's names), with the frames a tap's click waits for drawn in between. Each step prints the button
    /// it pressed and the state the buttons change.
    pub(crate) fn touch_test_hook(&mut self) {
        if std::env::var_os("SG_TOUCH").is_some() {
            self.touch.used = true;
            self.app.xs.touched = true;
        }
        for t in std::env::var("SG_TAP").unwrap_or_default().split(';').map(str::trim).filter(|t| !t.is_empty()) {
            if let Some(k) = t.strip_prefix("key:") {
                let Some(b) = crate::controls::Binding::parse(k) else { continue };
                let mods = KeyMods { shift: b.shift, ctrl: b.ctrl, ..Default::default() };
                self.key_down_event(b.key, mods, false);
                self.key_up_event(b.key, KeyMods::default());
                self.draw_frame(None);
                continue;
            }
            // run:S lets the game play S seconds (the shot after Swing); pro centres the view on the pro's ball
            if let Some(secs) = t.strip_prefix("run:").and_then(|v| v.parse::<f64>().ok()) {
                self.app.time += secs;
                self.draw_frame(None);
                self.touch_target_report("run");
                continue;
            }
            // frames:N draws N frames with the previews updated (the aim's lock runs out after 10)
            if let Some(n) = t.strip_prefix("frames:").and_then(|v| v.parse::<u32>().ok()) {
                for _ in 0..n {
                    // a thirtieth of a second each (the clock stands still in a scripted run): a target held at the edge
                    // scrolls the view
                    self.app.clock += 1.0 / 30.0;
                    self.touch_update();
                    self.pointer_preview();
                    self.draw_frame(None);
                }
                continue;
            }
            // click:x,y is a mouse click there (to compare a touch with the mouse)
            if let Some(p) = t.strip_prefix("click:") {
                let v = crate::ints(p);
                let [x, y] = v[..] else { continue };
                let (x, y) = (x as f32, y as f32);
                self.mouse_motion_event(x, y);
                self.pointer_preview();
                self.mouse_button_down_event(MouseButton::Left, x, y);
                self.mouse_button_up_event(MouseButton::Left, x, y);
                self.draw_frame(None);
                self.touch_target_report("click");
                continue;
            }
            if t == "pro" {
                if let Some(gi) = self.app.club.pro_aiming().or((self.app.club.gary >= 0).then_some(self.app.club.gary as usize)) {
                    let gg = &self.app.club.g[gi];
                    (self.app.cam_x, self.app.cam_z) = self.app.units_to_world(gg.bx, gg.by);
                }
                self.draw_frame(None);
                continue;
            }
            // staff centres the view on the employee hired last, and prints the window pixel it stands at
            if t == "staff" {
                let last =
                    self.app.employees.iter().rev().find(|e| e.active && e.job != sg_core::staff::job::OWNER).map(|e| (e.job, e.x, e.y));
                if let Some((_, x, y)) = last {
                    (self.app.cam_x, self.app.cam_z) = self.app.units_to_world(x, y);
                }
                self.draw_frame(None);
                if let Some((job, x, y)) = last {
                    let (wx, wz) = self.app.units_to_world(x, y);
                    let (px, py) = self.app.pixel_of_world(wx, wz);
                    println!("staff job {job} at window pixel {px:.0},{py:.0}");
                }
                continue;
            }
            // pan2:x0,y0,x1,y1 moves two fingers (80 pixels apart, side by side) from x0,y0 to x1,y1 and lifts them
            if let Some(p) = t.strip_prefix("pan2:") {
                let v = crate::ints(p);
                let [x0, y0, x1, y1] = v[..] else { continue };
                let (x0, y0, x1, y1) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32);
                let cam = (self.app.cam_x, self.app.cam_z);
                self.touch(TouchPhase::Started, 1, x0 - 40.0, y0);
                self.touch(TouchPhase::Started, 2, x0 + 40.0, y0);
                for k in 1..=8 {
                    let f = k as f32 / 8.0;
                    let (x, y) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                    self.touch(TouchPhase::Moved, 1, x - 40.0, y);
                    self.touch(TouchPhase::Moved, 2, x + 40.0, y);
                }
                self.touch(TouchPhase::Ended, 2, x1 + 40.0, y1);
                self.touch(TouchPhase::Ended, 1, x1 - 40.0, y1);
                self.draw_frame(None);
                println!("two fingers moved the view by {:.1},{:.1}", self.app.cam_x - cam.0, self.app.cam_z - cam.1);
                self.touch_target_report("pan2");
                continue;
            }
            // lift takes the finger left down by slide: off the screen
            if t == "lift" {
                if let Some(&(id, _, (x, y))) = self.touch.fingers.first() {
                    self.touch(TouchPhase::Ended, id, x, y);
                }
                self.draw_frame(None);
                self.touch_target_report("lift");
                continue;
            }
            // btn:centre / btn:swing taps that touch button where it is drawn
            if let Some(name) = t.strip_prefix("btn:") {
                let Some(i) = BUTTONS.iter().position(|b| format!("{b:?}").eq_ignore_ascii_case(name)) else { continue };
                let s = Ui::new(self.app.draw_w, self.app.draw_h);
                let (dx, dy) = s.anchor_shift(crate::enhanced::Anchor::Right);
                let ((cx, cy), _) = self.slot(i);
                let v = s.view;
                let (x, y) = ((cx + dx) * v.scale + v.ox, (cy + dy) * v.scale + v.oy);
                let cam = (self.app.cam_x, self.app.cam_z);
                self.touch(TouchPhase::Started, 1, x, y);
                let held = self.touch.held;
                self.touch(TouchPhase::Ended, 1, x, y);
                for _ in 0..4 {
                    self.touch_update();
                    self.pointer_preview();
                    self.draw_frame(None);
                }
                println!(
                    "touch btn:{name} at {x:.0},{y:.0}: {held:?} pressed, the view moved by {:.1},{:.1}",
                    self.app.cam_x - cam.0,
                    self.app.cam_z - cam.1
                );
                self.touch_target_report("btn");
                continue;
            }
            // drag:x0,y0,x1,y1 puts a finger down, moves it in steps and lifts it; slide: leaves it down at the end
            if let Some((how, p)) = t.split_once(':').filter(|(h, _)| matches!(*h, "drag" | "slide")) {
                let v = crate::ints(p);
                let [x0, y0, x1, y1] = v[..] else { continue };
                let (x0, y0, x1, y1) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32);
                self.touch(TouchPhase::Started, 1, x0, y0);
                for k in 1..=8 {
                    let f = k as f32 / 8.0;
                    self.touch(TouchPhase::Moved, 1, x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                }
                if how == "drag" {
                    self.touch(TouchPhase::Ended, 1, x1, y1);
                    for _ in 0..4 {
                        self.touch_update();
                        self.pointer_preview();
                        self.draw_frame(None);
                    }
                } else {
                    self.draw_frame(None);
                }
                self.touch_target_report(how);
                continue;
            }
            let (how, p) = t.split_once(':').filter(|(h, _)| matches!(*h, "tap" | "hold" | "down")).unwrap_or(("tap", t));
            let v = crate::ints(p);
            let [x, y] = v[..] else { continue };
            let (x, y) = (x as f32, y as f32);
            self.touch(TouchPhase::Started, 1, x, y);
            let held = self.touch.held;
            if how == "down" {
                // the finger stays down: the frame is drawn with the button pressed
                self.draw_frame(None);
                continue;
            }
            if how == "hold" {
                self.touch.started = self.app.clock - LONG_PRESS - 0.1;
                self.touch_update();
            }
            self.touch(TouchPhase::Ended, 1, x, y);
            for _ in 0..4 {
                self.touch_update();
                self.pointer_preview();
                self.draw_frame(None);
            }
            let a = &self.app;
            println!(
                "touch {how} {x},{y}: {} (speed {}, turn {}, open holes {}, shift {}, tool {} {})",
                held.map_or("no button".to_string(), |b| format!("{b:?} button")),
                a.speed,
                a.turn,
                a.club.holes.iter().filter(|h| h.par != 0).count(),
                self.shift,
                a.edit,
                a.tool
            );
            self.touch_target_report(how);
        }
    }

    /// The touch target's state after a test step (SG_TAP): what it is for, its tile, the aim's distance and club, and the
    /// pro's strokes on the hole.
    fn touch_target_report(&self, step: &str) {
        let a = &self.app;
        let tile = a.touch_target.target.map(|(x, z)| a.terrain.tile_of(x, z));
        let aim = a.aim.map(|p| format!("{} yds club {} at {},{}", p.distance, p.club, p.x, p.y));
        let strokes = a
            .club
            .pro_aiming()
            .or((a.club.gary >= 0).then_some(a.club.gary as usize))
            .map(|g| (a.club.g[g].hole, a.club.g[g].strokes, a.club.g[g].bx, a.club.g[g].by));
        println!(
            "touch target after {step}: for {:?}, tile {tile:?}, dragging {}, aim {aim:?}, pro (hole, strokes, ball) {strokes:?}, objects {}, cash {:.0}, view at {:.0},{:.0}, moving employee {:?}, card {:?}",
            a.touch_target_for(),
            a.touch_target.dragging(),
            a.land.as_ref().map_or(0, |l| l.objects.len()),
            a.econ.cash,
            a.cam_x,
            a.cam_z,
            a.moving_employee,
            a.card
        );
    }

    /// The touch buttons show on the course once the screen has been touched, with the interface up and nothing over the
    /// course that takes the keys (a notice, a SimFoto, the shot analysis, a name being typed).
    fn touch_buttons_shown(&self) -> bool {
        let a = &self.app;
        self.touch.used
            && a.ui_ok
            && a.screen == Screen::Play
            && a.photo.is_none()
            && !a.no_hud
            && !a.hud_hidden()
            && a.notice.is_none()
            && a.great_shot.is_none()
            && a.simfoto.is_none()
            && a.analysis.is_none()
            && !a.typing()
    }

    /// The touch buttons, drawn like the original's small round dock buttons (Interface/3mainLowerLeft.pcx: + - and the
    /// turns, a navy face in a pale rim with a cream picture, the picture gold while lit), by code so that they look the
    /// same with the free base set. They stay see-through until pressed; the button under a finger shows its name.
    pub(crate) fn draw_touch_buttons(&mut self) {
        self.draw_touch_target();
        let shown: Vec<bool> = BUTTONS.iter().map(|&b| self.button_shown(b)).collect();
        if !shown.contains(&true) {
            return;
        }
        let s = Ui::new(self.app.draw_w, self.app.draw_h);
        // Enhanced widescreen: at the window's right edge (nothing in Classic)
        s.anchor(crate::enhanced::Anchor::Right);
        let now = self.app.clock;
        let flash = self.touch.flash.filter(|f| now - f.1 < FLASH).map(|f| f.0);
        let slots: Vec<((f32, f32), f32)> = (0..BUTTONS.len()).map(|i| self.slot(i)).collect();
        let r_col = self.button_radius();
        let g: &mut Gfx = &mut self.g;
        for (i, &b) in BUTTONS.iter().enumerate() {
            if !shown[i] {
                continue;
            }
            let ((cx, cy), r) = slots[i];
            let lit = self.touch.held == Some(b) || flash == Some(b);
            // Swing and Place stand out: they act on the game
            draw_button(g, &s, cx, cy, r, lit, matches!(b, Btn::Swing | Btn::Place));
            let ink = if lit { GOLD } else { with_alpha(CREAM, 0.9) };
            match b {
                Btn::Speed => icon_speed(g, &s, cx, cy, r, self.app.speed, ink),
                Btn::Message => icon_message(g, &s, cx, cy, r, ink),
                Btn::Turn => icon_turn(g, &s, cx, cy, r, ink),
                Btn::OpenHole => icon_flag(g, &s, cx, cy, r, ink, lit),
                Btn::Showcase => icon_film(g, &s, cx, cy, r, ink),
                Btn::Place => icon_tick(g, &s, cx, cy, r, ink),
                Btn::Centre => icon_centre(g, &s, cx, cy, r, ink),
                Btn::Undo => icon_undo(g, &s, cx, cy, r, ink),
                // its name on its face: the one button that acts on the game, it says what it does
                Btn::Swing => {
                    let name = tr(b.label());
                    let w = 2.0 * r - 14.0;
                    let mut size = (r * 0.5).round();
                    while size > 9.0 && crate::ui::text_width(name, size) > w {
                        size -= 1.0;
                    }
                    let size = crate::textsize::fit_px(size, 2.0, name, w);
                    let y = cy + size * 0.35;
                    s.text_centered(g, cx + 0.8, y + 0.8, name, size, with_alpha(NAVY_DARK, ink[3]));
                    s.text_centered(g, cx, y, name, size, ink);
                }
            }
            if self.touch.held == Some(b) && b != Btn::Swing {
                // its name to the left, as the dock's tooltips are (the camera button's plate, photo.rs)
                let name = tr(b.label());
                // (as large on the device as the buttons are, so readable on a phone too)
                // and bigger again with the text size (textsize.rs)
                let k = r_col / R_MIN * crate::textsize::scale();
                let size = 13.0 * k;
                let tw = crate::ui::text_width(name, size);
                let (x, y) = (cx - r - 8.0 - tw - 12.0 * k, cy - 10.0 * k);
                s.fill(g, x, y, tw + 12.0 * k, 20.0 * k, rgba(0.0, 0.0, 0.0, 0.7));
                s.text(g, x + 6.0 * k, y + 15.0 * k, name, size, rgb(1.0, 1.0, 1.0));
            }
        }
        g.flush();
    }

    /// The touch target (TouchTarget): for the shot a ring on the aimed spot with its distance and club above it (where a
    /// finger below does not cover them), or beside it when the shot's line comes down there (plate_spot); a building shows
    /// its footprint and price as with the pointer; and while a finger drags the target a thin line from the finger to it.
    fn draw_touch_target(&mut self) {
        let a = &self.app;
        let what = a.touch_target_for();
        if !a.touch_target.ready(what) || a.photo.is_some() {
            return;
        }
        let Some((wx, wz)) = a.touch_target.target else { return };
        // the shot's spot is the aimed tile or corner (aim_frame), the building's the tile it goes on
        let (wx, wz, label) = match what {
            Some(TargetFor::Aim) => {
                let Some(p) = a.aim else { return };
                let (x, z) = a.units_to_world(p.x, p.y);
                let club = sg_core::pro::club_label(p.club.clamp(0, 11) as usize);
                (x, z, trf("STR_TOUCH_AIM_LABEL", &[("YARDS", &p.distance), ("CLUB", &club)]))
            }
            _ => (wx, wz, String::new()),
        };
        let Some((vx, vy)) = a.screen_of_world(wx, wz) else { return };
        // the shot's lines as render.rs draws them: on the ground, and the arc lifted up the screen
        let lines: Vec<Vec<(f32, f32)>> = a
            .aim_path()
            .filter(|_| !label.is_empty())
            .map(|(line, lifts)| {
                let ground: Vec<((f32, f32), f32)> = line
                    .iter()
                    .zip(&lifts)
                    .filter_map(|(&(x, y), &l)| {
                        let (wx, wz) = a.units_to_world(x, y);
                        a.screen_of_world(wx, wz).map(|p| (p, l))
                    })
                    .collect();
                vec![ground.iter().map(|&(p, _)| p).collect(), ground.iter().map(|&((x, y), l)| (x, y - l)).collect()]
            })
            .unwrap_or_default();
        let ball_left = lines.first().and_then(|l| l.first()).is_some_and(|b| b.0 < vx);
        // how far the box on the aimed tile (render.rs draw_aim) reaches from the spot, side to side
        let r = sg_core::terrain::TILE_SIZE * 0.5;
        let box_half = [(-r, -r), (r, -r), (r, r), (-r, r)]
            .iter()
            .filter_map(|&(dx, dz)| a.screen_of_world(wx + dx, wz + dz))
            .fold(0.0f32, |m, (x, _)| m.max((x - vx).abs()));
        let s = Ui::new(a.draw_w, a.draw_h);
        let k = self.button_radius() / R_MIN;
        let finger = if a.touch_target.dragging() { self.touch.fingers.first().map(|f| a.view.to_virtual(f.2 .0, f.2 .1)) } else { None };
        let g: &mut Gfx = &mut self.g;
        if let Some((fx, fy)) = finger {
            s.line(g, fx, fy, vx, vy, 1.0, with_alpha(CREAM, 0.6));
        }
        if !label.is_empty() {
            // the shot's spot (a building shows its own footprint, cursor_ui.rs)
            ring(g, &s, vx, vy, 7.0 * k, 9.0 * k, |_| with_alpha(NAVY_DARK, 0.8));
            ring(g, &s, vx, vy, 5.5 * k, 7.0 * k, |_| GOLD);
            s.fill(g, vx - 0.75, vy - 0.75, 1.5, 1.5, GOLD);
            let size = 13.0 * k * crate::textsize::scale();
            let tw = crate::ui::text_width(&label, size);
            let gap = ((14.0 * k).max(box_half + 4.0), 14.0 * k);
            let (x, y) = plate_spot((vx, vy), (tw + 12.0 * k, 20.0 * k), gap, &lines, ball_left);
            s.fill(g, x, y, tw + 12.0 * k, 20.0 * k, rgba(0.0, 0.0, 0.0, 0.7));
            s.text(g, x + 6.0 * k, y + 15.0 * k, &label, size, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }
}

// ---- the touch buttons' look --------------------------------------------------------------------------------------------
// The colours are the 15-bit palette of the dock's small buttons (3mainLowerLeft.pcx cuts at x 600 and 650): rim 0x101830
// and 0x707098 / 0x8888b8, face 0x303870 to 0x182040, picture 0xf0e0c0 with its shade 0xd8c070, lit 0xf8b800.

const NAVY_DARK: [f32; 4] = [16.0 / 255.0, 24.0 / 255.0, 48.0 / 255.0, 1.0];
const RIM_LIGHT: [f32; 4] = [136.0 / 255.0, 136.0 / 255.0, 184.0 / 255.0, 1.0];
const RIM_SHADE: [f32; 4] = [32.0 / 255.0, 40.0 / 255.0, 80.0 / 255.0, 1.0];
const FACE_MID: [f32; 4] = [56.0 / 255.0, 64.0 / 255.0, 120.0 / 255.0, 1.0];
const FACE_EDGE: [f32; 4] = [24.0 / 255.0, 32.0 / 255.0, 64.0 / 255.0, 1.0];
const CREAM: [f32; 4] = [240.0 / 255.0, 224.0 / 255.0, 192.0 / 255.0, 1.0];
const GOLD: [f32; 4] = [248.0 / 255.0, 184.0 / 255.0, 0.0, 1.0];
const FLAG_RED: [f32; 4] = [216.0 / 255.0, 48.0 / 255.0, 32.0 / 255.0, 1.0];
/// How see-through a button is until pressed.
const IDLE_ALPHA: f32 = 0.65;
/// Pieces of a circle.
const SEGMENTS: usize = 32;

fn with_alpha(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

fn mix(a: [f32; 4], b: [f32; 4], k: f32) -> [f32; 4] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k)
}

/// A ring from radius `r0` out to `r1` round (cx, cy) (r0 = 0 for a disc), each piece coloured by `col` of its middle angle
/// (radians, 0 to the right, growing clockwise on the screen). The pieces do not overlap, so a see-through ring is even.
fn ring(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r0: f32, r1: f32, col: impl Fn(f32) -> [f32; 4]) {
    let step = std::f32::consts::TAU / SEGMENTS as f32;
    for k in 0..SEGMENTS {
        let (a0, a1) = (k as f32 * step, (k + 1) as f32 * step);
        let p = |r: f32, a: f32| (cx + r * a.cos(), cy + r * a.sin());
        s.quad(g, [p(r0, a0), p(r1, a0), p(r1, a1), p(r0, a1)], col((a0 + a1) * 0.5));
    }
}

/// A round button: a dark outline, a bevel lit from the top left, and a navy face lighter towards its middle.
fn draw_button(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, lit: bool, solid: bool) {
    let a = if lit || solid { 1.0 } else { IDLE_ALPHA };
    // a soft shadow below and to the right, outside the button
    ring(g, s, cx + 1.0, cy + 2.0, r, r + 1.5, |ang| rgba(0.0, 0.0, 0.0, 0.3 * a * (ang - 0.8).cos().max(0.0)));
    ring(g, s, cx, cy, r - 1.5, r, |_| with_alpha(NAVY_DARK, a));
    // the bevel: light where it faces the top left (angle 225 degrees), dark where it faces away
    let light = std::f32::consts::PI * 1.25;
    ring(g, s, cx, cy, r - 4.0, r - 1.5, |ang| with_alpha(mix(RIM_SHADE, RIM_LIGHT, ((ang - light).cos() + 1.0) * 0.5), a));
    ring(g, s, cx, cy, r - 5.0, r - 4.0, |_| with_alpha(NAVY_DARK, a));
    let face = r - 5.0;
    let mid = if lit { mix(FACE_MID, RIM_LIGHT, 0.25) } else { FACE_MID };
    let bands = 10;
    for k in 0..bands {
        let (r0, r1) = (face * k as f32 / bands as f32, face * (k + 1) as f32 / bands as f32);
        let c = mix(mid, FACE_EDGE, (k as f32 + 0.5) / bands as f32);
        ring(g, s, cx, cy, r0, r1, |_| with_alpha(c, a));
    }
}

/// A picture drawn twice: a dark shade a little down and right, then the picture, as the dock's pictures are.
fn shaded(g: &mut Gfx, ink: [f32; 4], draw: impl Fn(&mut Gfx, f32, f32, [f32; 4])) {
    draw(g, 0.8, 0.8, with_alpha(NAVY_DARK, ink[3]));
    draw(g, 0.0, 0.0, ink);
}

/// Speed: one, two or three play arrows for normal, x2 and x4.
fn icon_speed(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, speed: u8, ink: [f32; 4]) {
    let n = match speed {
        1 => 1,
        2 => 2,
        _ => 3,
    };
    let h = r * 0.42;
    let w = r * 0.38;
    let total = w * n as f32 + (n - 1) as f32 * r * 0.04;
    let x0 = cx - total * 0.5;
    shaded(g, ink, |g, dx, dy, c| {
        for k in 0..n {
            let x = x0 + k as f32 * (w + r * 0.04) + dx;
            let y = cy + dy;
            s.quad(g, [(x, y - h), (x + w, y), (x + w, y), (x, y + h)], c);
        }
    });
}

/// Message: a speech bubble with three dots.
fn icon_message(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, ink: [f32; 4]) {
    let (bw, bh) = (r * 0.5, r * 0.36);
    let by = cy - r * 0.07;
    shaded(g, ink, |g, dx, dy, c| {
        let (x, y) = (cx + dx, by + dy);
        // the bubble: an ellipse in pieces that meet at its middle, and its tail to the lower left
        let step = std::f32::consts::TAU / SEGMENTS as f32;
        for k in 0..SEGMENTS {
            let (a0, a1) = (k as f32 * step, (k + 1) as f32 * step);
            let p = |a: f32| (x + bw * a.cos(), y + bh * a.sin());
            s.quad(g, [(x, y), p(a0), p(a1), (x, y)], c);
        }
        let t = (x - bw * 0.45, y + bh * 0.7);
        s.quad(g, [t, (t.0 + bw * 0.35, t.1), (x - bw * 0.75, y + bh * 1.55), t], c);
    });
    let d = r * 0.07;
    for k in -1..=1 {
        let x = cx + k as f32 * bw * 0.48;
        s.quad(g, [(x - d, by - d), (x + d, by - d), (x + d, by + d), (x - d, by + d)], with_alpha(NAVY_DARK, ink[3]));
    }
}

/// Turn: a thick arrow bending round to the right (the dock's turn buttons' picture, with more of the turn).
fn icon_turn(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, ink: [f32; 4]) {
    let rad = r * 0.36;
    let w = r * 0.13;
    let (oy, from, to) = (cy + r * 0.04, std::f32::consts::PI * 0.75, std::f32::consts::PI * 2.05);
    shaded(g, ink, |g, dx, dy, c| {
        let n = 14;
        let p = |a: f32, rr: f32| (cx + dx + rr * a.cos(), oy + dy + rr * a.sin());
        for k in 0..n {
            let a0 = from + (to - from) * k as f32 / n as f32;
            let a1 = from + (to - from) * (k + 1) as f32 / n as f32;
            s.quad(g, [p(a0, rad - w), p(a0, rad + w), p(a1, rad + w), p(a1, rad - w)], c);
        }
        // the head, pointing on round the turn
        let tip_a = to + 0.55;
        let (bx, by) = p(to, rad);
        let (ox, oy2) = (to.cos(), to.sin());
        let tip = p(tip_a, rad);
        let hw = w * 2.4;
        s.quad(g, [(bx - ox * hw, by - oy2 * hw), (bx + ox * hw, by + oy2 * hw), tip, tip], c);
    });
}

/// Undo: a hooked arrow pointing back to the left (the usual undo picture), so it does not read as Turn's arrow going round.
fn icon_undo(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, ink: [f32; 4]) {
    let rad = r * 0.32;
    let w = r * 0.15;
    let (ox, oy, from, to) = (cx - r * 0.06, cy + r * 0.1, std::f32::consts::PI * 1.5, std::f32::consts::PI * 2.5);
    shaded(g, ink, |g, dx, dy, c| {
        let n = 12;
        let p = |a: f32, rr: f32| (ox + dx + rr * a.cos(), oy + dy + rr * a.sin());
        // the hook: round the right from the top to the bottom
        for k in 0..n {
            let a0 = from + (to - from) * k as f32 / n as f32;
            let a1 = from + (to - from) * (k + 1) as f32 / n as f32;
            s.quad(g, [p(a0, rad - w), p(a0, rad + w), p(a1, rad + w), p(a1, rad - w)], c);
        }
        // a short tail back to the left along the bottom
        let (bx, by) = (ox + dx, oy + dy + rad);
        let tail = r * 0.28;
        s.quad(g, [(bx - tail, by - w), (bx, by - w), (bx, by + w), (bx - tail, by + w)], c);
        // the head at the top, pointing left
        let (tx, ty) = (ox + dx, oy + dy - rad);
        let (hw, len) = (w * 2.3, r * 0.36);
        s.quad(g, [(tx, ty - hw), (tx, ty + hw), (tx - len, ty), (tx - len, ty)], c);
    });
}

/// Open hole: the flagstick in its cup, the flag red (as the build panel's green shows it) and the stick cream.
fn icon_flag(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, ink: [f32; 4], lit: bool) {
    let px = cx - r * 0.12;
    let (top, foot) = (cy - r * 0.48, cy + r * 0.42);
    let flag = if lit { GOLD } else { with_alpha(FLAG_RED, ink[3]) };
    shaded(g, ink, |g, dx, dy, c| {
        s.line(g, px + dx, top + dy, px + dx, foot + dy, r * 0.09, c);
        // the cup: a flat ring at the stick's foot
        s.quad(
            g,
            [
                (px - r * 0.3 + dx, foot + dy),
                (px + dx, foot - r * 0.07 + dy),
                (px + r * 0.3 + dx, foot + dy),
                (px + dx, foot + r * 0.07 + dy),
            ],
            c,
        );
    });
    let fl = [(px, top), (px + r * 0.5, top + r * 0.16), (px + r * 0.5, top + r * 0.16), (px, top + r * 0.34)];
    s.quad(g, fl.map(|(x, y)| (x + 0.8, y + 0.8)), with_alpha(NAVY_DARK, ink[3]));
    s.quad(g, fl, flag);
}

/// Place: a tick.
fn icon_tick(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, ink: [f32; 4]) {
    let w = r * 0.13;
    shaded(g, ink, |g, dx, dy, c| {
        let (x0, y0) = (cx - r * 0.38 + dx, cy + dy);
        let (x1, y1) = (cx - r * 0.1 + dx, cy + r * 0.3 + dy);
        let (x2, y2) = (cx + r * 0.42 + dx, cy - r * 0.32 + dy);
        s.line(g, x0, y0, x1, y1, w * 2.0, c);
        s.line(g, x1, y1, x2, y2, w * 2.0, c);
    });
}

/// The showcase's picture: a film camera, a box with its lens hood to the right.
fn icon_film(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, ink: [f32; 4]) {
    shaded(g, ink, |g, dx, dy, c| {
        let (x0, x1, h) = (cx - r * 0.44 + dx, cx + r * 0.12 + dx, r * 0.24);
        let y = cy + dy;
        s.quad(g, [(x0, y - h), (x1, y - h), (x1, y + h), (x0, y + h)], c);
        s.quad(g, [(x1, y - r * 0.06), (cx + r * 0.44 + dx, y - h), (cx + r * 0.44 + dx, y + h), (x1, y + r * 0.06)], c);
    });
}

/// The Centre button's picture: a ball in a sight (a ring with four ticks).
fn icon_centre(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, ink: [f32; 4]) {
    shaded(g, ink, |g, dx, dy, c| {
        let (x, y) = (cx + dx, cy + dy);
        ring(g, s, x, y, r * 0.36, r * 0.46, |_| c);
        ring(g, s, x, y, 0.0, r * 0.14, |_| c);
        let (a, b, w) = (r * 0.3, r * 0.62, r * 0.05);
        s.fill(g, x - b, y - w, b - a, 2.0 * w, c);
        s.fill(g, x + a, y - w, b - a, 2.0 * w, c);
        s.fill(g, x - w, y - b, 2.0 * w, b - a, c);
        s.fill(g, x - w, y + a, 2.0 * w, b - a, c);
    });
}

impl App {
    /// The touch Centre button (issue #75): the view on the ball of the pro waiting for the player's shot; it ends People >
    /// Follow and any scroll under way. False when the pro is not waiting.
    pub fn centre_on_pro(&mut self) -> bool {
        let Some(gi) = self.club.pro_aiming() else { return false };
        let gg = &self.club.g[gi];
        (self.cam_x, self.cam_z) = self.units_to_world(gg.bx, gg.by);
        self.xs.follow = None;
        self.enh.pan_left = (0.0, 0.0);
        self.enh.pan_vel = (0.0, 0.0);
        true
    }

    /// What a touch target would be for now (TouchTarget): the pro's aim while he waits for the player's shot; the building
    /// tool's placement of a building, home site, landmark or single garden item (a stray tap there costs money). Paths and
    /// the terrain brushes (tees and greens among them) paint, so a finger stays direct on them, as it does for a row kind
    /// with the Drag to place extra (the drag lays the row).
    pub fn touch_target_for(&self) -> Option<TargetFor> {
        if self.screen != Screen::Play || self.auto_aim != 0 || self.editor.is_some() {
            return None;
        }
        if !self.edit {
            return self.club.pro_aiming().map(|_| TargetFor::Aim);
        }
        let row = crate::extras::drag_place() && crate::build_tools::row_kind(self.build_idx as i32);
        (self.tool == 4 && self.build_idx as i32 != sg_core::land::K_PATH && !row).then_some(TargetFor::Place(self.build_idx))
    }

    /// Whether a touch button has something to do now: Speed and Message always; Turn while Tab turns something (a building
    /// or a green's look in hand, the Path styles extra's style with the Pathway tool, a placed building with the Rotate
    /// extra); Open hole once the hole being built has its tee and green; Showcase once a hole is open; Undo while building
    /// (a tool in hand, or the Build Course or Add Buildings panel open) with a click to undo, so it is not left on the
    /// course for the rest of the game.
    fn touch_button_wanted(&self, b: Btn) -> bool {
        match b {
            Btn::Speed | Btn::Message => true,
            Btn::Turn if self.edit => match self.tool {
                4 => true,
                0 => self.paint_variant.is_some_and(|(k, _)| k == self.paint_idx),
                2 => self.club.extras.lamps(),
                _ => false,
            },
            Btn::Turn => self.club.extras.rotate,
            Btn::OpenHole => self.club.can_open_course_hole(self.hole_b()),
            Btn::Showcase => self.club.holes.iter().skip(1).take(18).any(|h| h.par != 0),
            Btn::Undo => self.undo.can_undo() && (self.edit || matches!(self.panel, 1 | 2)),
            // shown on their own terms (Stage::button_shown)
            Btn::Swing | Btn::Place | Btn::Centre => false,
        }
    }

    /// Whether one of the extras' legends is up at the top right, under the gauges (pathview_ui, staffview_ui, insight).
    fn right_legend_up(&self) -> bool {
        (self.comfort.staff_view && self.land.is_some())
            || (self.xs.path_view && crate::extras::path_help())
            || (self.xs.insight.map && crate::extras::landing_map())
    }

    /// The left edge of the legends up at the top right: x 566, or the landing map's when bigger text has widened it
    /// (textsize.rs).
    fn legend_left(&self) -> f32 {
        let (k, _, _) = self.legend_place(self.legend_below());
        (790.0 - (224.0 * k).round()).min(LEGEND_LEFT)
    }

    /// Whether a name is being typed (rename, save, Customise Golfer).
    pub fn typing(&self) -> bool {
        self.rename.is_some()
            || self.editor_typing()
            || self.title.save_name.is_some()
            || self.content.profile_name.is_some()
            || self.cust.as_ref().is_some_and(|c| c.typing.is_some())
            || self.popup.as_ref().is_some_and(|p| p.kind == crate::popup_ui::PopupKind::Search)
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

    #[test]
    fn touch_buttons_fit_between_the_gauges_and_the_panel_at_every_size() {
        // a phone in landscape (0.65 pixels to the unit), tablets, a desktop, a 4K screen
        for px in [0.3, 0.65, 1.0, 1.37, 1.71, 2.4, 3.6] {
            let r = button_radius(px);
            assert!((R_MIN..=R_MAX).contains(&r));
            for right in [800.0, LEGEND_LEFT] {
                let c: Vec<(f32, f32)> = (0..COLUMN).map(|i| button_centre(i, r, right)).collect();
                for (i, &(x, y)) in c.iter().enumerate() {
                    // under the gauges and the camera button (to y 164), above the dock's panel (y 452), on the screen
                    assert!(y - r - HIT_PAD > 164.0 && y + r + HIT_PAD < 452.0, "slot {i} at {px}: {y}");
                    assert!(x + r + HIT_PAD <= right + 0.01 && x - r > 0.0);
                    // no press is on two buttons
                    if let Some(&(_, y2)) = c.get(i + 1) {
                        assert!(y2 - y >= 2.0 * (r + HIT_PAD) - 0.01);
                    }
                    assert!(on_button((x, y), r, x + r, y - r) && !on_button((x, y), r, x, y + r + HIT_PAD + 1.0));
                }
            }
        }
        // about 36 device pixels across where that fits
        assert_eq!(button_radius(1.0), 18.0);
    }

    #[test]
    fn swing_and_place_sit_clear_of_the_column_above_the_panel() {
        for px in [0.3, 0.65, 1.0, 1.37, 1.71, 2.4, 3.6] {
            let r = button_radius(px);
            let big = big_radius(r);
            for right in [800.0, LEGEND_LEFT] {
                let col_left = button_centre(0, r, right).0 - r - HIT_PAD;
                for (name, (x, y)) in [("swing", swing_centre(r, right)), ("place", place_centre(r, right))] {
                    // left of the column, under the gauges, above the dock's panel, on the screen
                    assert!(x + big + HIT_PAD < col_left, "{name} at {px}");
                    assert!(y - big - HIT_PAD > 164.0 && y + big + HIT_PAD < 452.0, "{name} at {px}: {y}");
                    assert!(x - big > 0.0);
                }
                // Place stands level with Turn, which turns what it places
                assert_eq!(place_centre(r, right).1, button_centre(2, r, right).1);
            }
        }
    }

    #[test]
    fn undo_sits_left_of_speed_clear_of_every_other_button() {
        for px in [0.3, 0.65, 1.0, 1.37, 1.71, 2.4, 3.6] {
            let r = button_radius(px);
            let big = big_radius(r);
            for right in [800.0, LEGEND_LEFT] {
                let (x, y) = undo_centre(r, right);
                // under the gauges and the camera button, on the screen, level with Speed
                assert!(y - r - HIT_PAD > 164.0 && x - r > 0.0, "undo at {px}");
                assert_eq!(y, button_centre(0, r, right).1);
                let apart = |c: (f32, f32), rc: f32| (x - c.0).abs() > r + rc + 2.0 * HIT_PAD || (y - c.1).abs() > r + rc + 2.0 * HIT_PAD;
                for i in 0..COLUMN {
                    assert!(apart(button_centre(i, r, right), r), "undo on slot {i} at {px}");
                }
                for (name, c, rc) in
                    [("swing", swing_centre(r, right), big), ("place", place_centre(r, right), big), ("centre", centre_centre(r, right), r)]
                {
                    assert!(apart(c, rc), "undo on {name} at {px}");
                }
            }
        }
    }

    #[test]
    fn a_drag_moves_the_target_only_from_near_it_and_pans_the_view_elsewhere() {
        // issue #75: on or near the target (its ring and plate) the drag aims, anywhere else it looks round
        let t = Some((400.0, 200.0));
        assert!(grabs_target((400.0, 200.0), t, 80.0));
        assert!(grabs_target((400.0, 270.0), t, 80.0) && grabs_target((450.0, 250.0), t, 80.0));
        assert!(!grabs_target((400.0, 290.0), t, 80.0) && !grabs_target((100.0, 100.0), t, 80.0));
        // no target yet, or one off the screen: the drag pans (a tap sets the target)
        assert!(!grabs_target((400.0, 200.0), None, 80.0));
    }

    #[test]
    fn the_shot_plate_stays_above_the_spot_unless_the_shot_comes_down_there() {
        let (spot, size, gap) = ((400.0, 200.0), (120.0, 20.0), (30.0, 14.0));
        // nothing in the way: above the spot, centred, as before
        assert_eq!(plate_spot(spot, size, gap, &[], true), (340.0, 166.0));
        // a ball below the spot: its line stays clear of the plate above
        let from_below = vec![vec![(300.0, 330.0), (400.0, 200.0)]];
        assert_eq!(plate_spot(spot, size, gap, &from_below, true), (340.0, 166.0));
        // a high shot from the left coming down on the spot: beside it, away from the ball
        let lob = vec![vec![(250.0, 260.0), (330.0, 120.0), (390.0, 150.0), (400.0, 200.0)]];
        assert_eq!(plate_spot(spot, size, gap, &lob, true), (430.0, 190.0));
        // the same shot from the right: on the left
        let mirrored: Vec<Vec<(f32, f32)>> = lob.iter().map(|l| l.iter().map(|&(x, y)| (800.0 - x, y)).collect()).collect();
        assert_eq!(plate_spot(spot, size, gap, &mirrored, false), (250.0, 190.0));
        // kept on the screen at its edge
        assert_eq!(plate_spot((790.0, 200.0), size, gap, &[], true), (678.0, 166.0));
        // a segment through, into, or past the rectangle
        let r = (0.0, 0.0, 10.0, 10.0);
        assert!(crosses(r, (-5.0, 5.0), (15.0, 5.0)) && crosses(r, (5.0, 5.0), (5.0, 6.0)) && crosses(r, (-5.0, -5.0), (5.0, 5.0)));
        assert!(!crosses(r, (-5.0, -5.0), (-1.0, 20.0)) && !crosses(r, (11.0, 0.0), (11.0, 10.0)) && !crosses(r, (-5.0, 6.0), (6.0, 17.0)));
    }

    #[test]
    fn a_target_at_the_edge_scrolls_the_view_that_way_harder_nearer_the_edge() {
        let area = (0.0, 0.0, 800.0, 450.0);
        // in the middle, and just inside the band: nothing
        assert_eq!(edge_push((400.0, 200.0), area, 48.0), (0.0, 0.0));
        assert_eq!(edge_push((48.0, 48.0), area, 48.0), (0.0, 0.0));
        // half way into the band, at the edge, and past it (a target lifted above the top)
        assert_eq!(edge_push((24.0, 200.0), area, 48.0), (-0.5, 0.0));
        assert_eq!(edge_push((800.0, 200.0), area, 48.0), (1.0, 0.0));
        assert_eq!(edge_push((400.0, -30.0), area, 48.0), (0.0, -1.0));
        assert_eq!(edge_push((400.0, 426.0), area, 48.0), (0.0, 0.5));
        // a corner pushes both ways
        assert_eq!(edge_push((0.0, 0.0), area, 48.0), (-1.0, -1.0));
    }

    #[test]
    fn centre_sits_above_swing_clear_of_the_column_and_the_gauges() {
        for px in [0.3, 0.65, 1.0, 1.37, 1.71, 2.4, 3.6] {
            let r = button_radius(px);
            let big = big_radius(r);
            for right in [800.0, LEGEND_LEFT] {
                let (x, y) = centre_centre(r, right);
                let (sx, sy) = swing_centre(r, right);
                let col_left = button_centre(0, r, right).0 - r - HIT_PAD;
                // no press is on both Centre and Swing
                assert!(sy - big - HIT_PAD >= y + r + HIT_PAD, "centre over swing at {px}");
                assert!(x + r + HIT_PAD < col_left && x - r > 0.0, "centre at {px}");
                assert!(y - r - HIT_PAD > 164.0, "centre under the gauges at {px}: {y}");
                assert!(x <= sx + big);
            }
        }
    }

    #[test]
    fn mouse_aim_and_swing_targets_the_shot_only_and_survives_the_mouse_moving() {
        let (aim, bench) = (Some(TargetFor::Aim), Some(TargetFor::Place(1)));
        let mut t = TouchTarget::default();
        t.sync(aim);
        // off: the mouse hovers and clicks as in the original
        assert!(!t.active(aim));
        t.mouse = true;
        // on: the shot is targeted as by touch, from the first frame (no tap needed to start), but no building is
        assert!(t.active(aim) && !t.ready(aim) && !t.active(bench) && !t.active(None));
        t.tap((3.0, 4.0));
        assert!(t.ready(aim));
        // the same lift and drag as a finger's
        t.drag_start((200.0, 200.0), Some((200.0, 200.0)), 64.0);
        assert_eq!(t.drag_point((210.0, 200.0)), Some((210.0, 136.0)));
        t.drag_end();
        // a building in hand: no target, and the mouse places as before
        t.sync(bench);
        t.tap((1.0, 1.0));
        assert!(!t.ready(bench));
        // a pointer move outside the shot (main.rs) clears the target but not the extra's switch
        t.pointer_moved();
        assert!(t.mouse && !t.on && t.target.is_none());
        t.sync(aim);
        assert!(t.active(aim));
        // with a touch as well, buildings are targeted too, as for any finger
        t.touched();
        assert!(t.active(bench));
    }

    #[test]
    fn a_touch_target_is_set_by_a_tap_moved_by_a_drag_and_never_acts_by_itself() {
        let aim = Some(TargetFor::Aim);
        let mut t = TouchTarget::default();
        t.sync(aim);
        // a mouse user never gets a target or the Swing button
        assert!(!t.active(aim) && !t.ready(aim));
        t.touched();
        assert!(t.active(aim) && !t.ready(aim), "no target before a tap");
        t.tap((10.0, 20.0));
        assert_eq!(t.target, Some((10.0, 20.0)));
        assert!(t.ready(aim));
        // a drag from the target itself lifts it above the finger, then it follows the finger
        t.drag_start((100.0, 300.0), Some((100.0, 300.0)), 64.0);
        assert_eq!(t.drag_point((100.0, 300.0)), Some((100.0, 236.0)));
        assert_eq!(t.drag_point((130.0, 290.0)), Some((130.0, 226.0)));
        // lifting the finger leaves the target, still ready (nothing is shot)
        t.drag_end();
        assert!(!t.dragging() && t.ready(aim) && t.drag_point((0.0, 0.0)).is_none());
        // a drag away from the target moves it from where it stands, as a trackpad does
        t.drag_start((400.0, 400.0), Some((200.0, 150.0)), 64.0);
        assert_eq!(t.drag_point((410.0, 380.0)), Some((210.0, 130.0)));
        // with no target on the screen it starts above the finger
        t.drag_start((50.0, 300.0), None, 64.0);
        assert_eq!(t.drag_point((50.0, 300.0)), Some((50.0, 236.0)));
        // the shot taken (no shot waiting): the target goes, and the next shot starts without one
        t.sync(None);
        assert!(t.target.is_none() && !t.dragging() && !t.ready(None));
        t.sync(aim);
        assert!(!t.ready(aim));
        // a building's target goes when another kind is picked; Swing's target never stands for Place
        let bench = Some(TargetFor::Place(1));
        t.sync(bench);
        t.tap((1.0, 2.0));
        assert!(t.ready(bench) && !t.ready(aim));
        t.sync(Some(TargetFor::Place(5)));
        assert!(t.target.is_none());
        // a mouse moved: hover previews again
        t.tap((1.0, 2.0));
        t.pointer_moved();
        assert!(!t.on && t.target.is_none());
    }
}
