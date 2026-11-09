//! The walk camera: a hidden test view of the course from a golfer's eye height, with a perspective projection, reached only
//! with the `--walk` command-line switch on the desktop (it is in no menu, setting or help text). F12 turns it on and off on
//! the course screen, Esc turns it off; W A S D or the arrows walk (Shift faster), a drag with a mouse button held turns
//! and looks up and down, and the eye follows the ground. G follows the golfers on the course one after another (Shift+G
//! back): the camera walks behind the one chosen and, when they hit, follows the ball's flight and roll, then goes back to
//! them; a walk key or Esc leaves the following.
//!
//! What it draws (each part in a module of its own):
//!
//! * the ground (walk_ground.rs): the course's tiles with the player's terrain art, mipmapped and anisotropically filtered,
//!   under a detail layer made by the game itself (walk_gl.rs `detail_image`) that gives each kind of ground its grain up
//!   close (grass blades, mown stripes, the green's close cut, rough clumps, sand, the desert's pebbles) while the art's
//!   painted features fade out near the eye, where they would smear; the water with moving ripples, the sky's reflection
//!   and the sun's glints; and the land beyond the property, out to the theme's hills or down into the sea;
//! * the sky (walk_gl.rs `sky_fs`): the theme's colours, the sun and drifting clouds, and a haze of the same colours over
//!   everything with distance;
//! * what stands on the ground (walk_objects.rs): buildings as solids with their own sprites projected onto them, trees and
//!   people as cards, flags, tee markers and balls as small solids, shadows on the ground.
//!
//! Nothing here runs without the switch, and nothing in the game's state changes: the walk only reads it, and leaving
//! puts the overhead camera back where it was.
//!
//! With the switch on a scripted still (`--png`), the still is taken from the walk camera; `SG_WALK_AT=TX,TY,YAW[,PITCH]`
//! (tiles and degrees) places it, `SG_WALK_FOLLOW=N` follows the golfer in slot N (`next`: whoever hits next) and
//! `SG_WALK_SHOT=S` waits for their ball to have flown S seconds, `SG_WALK_BENCH=1` prints the frame times (with
//! `SG_FX_BENCH`), `SG_WALK_ISO=Q` draws the scene through the overhead camera of quarter turn Q (there each building's
//! solid must look exactly as its sprite does), `SG_WALK_CARDS=1` draws every object as its card, and `SG_WALK_DUMP=1`
//! prints the fitted solids (`SG_WALK_DUMP_DIR` writes the frames fitted).
use crate::app::{App, Screen};
use crate::gfx::{Gfx, Mat4};
use crate::ui::{rgba, Screen as Ui, F_ARIAL10, F_INFO14};
use crate::walk_gl::{cat, mode, Pipe, SkyUniforms, WalkGpu, WalkUniforms};
use crate::walk_ground::{is_water, Ground};
use crate::walk_objects::{box_visible, frustum, Objects, Obstacle, Scene};
use crate::Stage;
use miniquad::{KeyCode, MouseButton, TextureId};
use sg_core::terrain::{Terrain, TILE_SIZE};
use std::sync::atomic::{AtomicBool, Ordering};

/// World units per metre: a tile's edge is 100 units and 25 yards (docs/DECODE_PLAYCORE.md).
pub const METRE: f32 = TILE_SIZE / (25.0 * 0.9144);
/// The eye's height over the ground.
pub const EYE: f32 = 1.7 * METRE;
/// Walking speed (metres a second), and how many times that with Shift.
const SPEED: f32 = 5.0;
const FAST: f32 = 4.0;
/// How quickly the walk gets up to speed and stops (per second; higher is quicker).
const ACCEL: f32 = 7.0;
const BRAKE: f32 = 9.0;
/// Degrees turned per drawable pixel of a drag (divided by the window's dpi scale).
const LOOK: f32 = 0.25;
/// The steepest look up or down, degrees.
const MAX_PITCH: f32 = 75.0;
/// Vertical field of view, degrees.
const FOV: f32 = 60.0;
/// The near plane (world units).
const NEAR: f32 = 0.3 * METRE;
/// The far plane: past the hills on the horizon.
const FAR: f32 = 26000.0;
/// How far a walker keeps from a wall or a trunk.
const BODY: f32 = 0.4 * METRE;
/// The head's bob while walking: its height and the length of a step.
const BOB: f32 = 0.03 * METRE;
const STEP: f32 = 0.75 * METRE;
/// The key that turns the walk camera on and off (one the game leaves unused).
pub const KEY: KeyCode = KeyCode::F12;
/// The key that follows the golfers.
pub const FOLLOW_KEY: KeyCode = KeyCode::G;

static ENABLED: AtomicBool = AtomicBool::new(false);

/// The `--walk` switch was given.
#[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Where the walk camera stands and looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cam {
    /// The ground point under the eye (world units).
    pub x: f32,
    pub z: f32,
    /// Heading in degrees: 0 looks towards world -Z, 90 towards +X (the overhead camera's heading is 45 plus its turn).
    pub yaw: f32,
    /// Degrees above level.
    pub pitch: f32,
}

impl Cam {
    /// Level ahead and to the right, on the ground plane.
    pub fn axes(&self) -> ([f32; 2], [f32; 2]) {
        let (s, c) = self.yaw.to_radians().sin_cos();
        ([s, -c], [c, s])
    }
    /// The way the eye looks.
    pub fn forward(&self) -> [f32; 3] {
        let ([fx, fz], _) = self.axes();
        let (sp, cp) = self.pitch.to_radians().sin_cos();
        [fx * cp, sp, fz * cp]
    }
    /// The eye, at its height over the ground here.
    #[cfg(test)]
    pub fn eye(&self, t: &Terrain) -> [f32; 3] {
        [self.x, eye_height(t, self.x, self.z), self.z]
    }
    #[cfg(test)]
    pub fn view(&self, t: &Terrain) -> Mat4 {
        self.view_from(self.eye(t))
    }
    /// The view from an eye placed elsewhere (eased up and down slopes, or following a golfer).
    pub fn view_from(&self, e: [f32; 3]) -> Mat4 {
        let f = self.forward();
        Mat4::look_at(e, [e[0] + f[0], e[1] + f[1], e[2] + f[2]], [0.0, 1.0, 0.0])
    }
    /// The quarter turn of the overhead camera nearest to this heading: its sprites' views are drawn.
    pub fn quarter(&self) -> i32 {
        (((self.yaw - 45.0) / 90.0).round() as i32).rem_euclid(4)
    }
    /// Moves `ahead` and `right` world units along the ground, kept on the map and out of the water (walk_update steps
    /// among the buildings and trees too, with `step`).
    #[cfg(test)]
    pub fn walk(&mut self, t: &Terrain, ahead: f32, right: f32) {
        let ([fx, fz], [rx, rz]) = self.axes();
        let (dx, dz) = (fx * ahead + rx * right, fz * ahead + rz * right);
        (self.x, self.z) = step(t, &[], (self.x, self.z), (dx, dz));
    }
    /// Turns by a drag of (dx, dy) degrees.
    pub fn look(&mut self, dx: f32, dy: f32) {
        self.yaw = (self.yaw + dx).rem_euclid(360.0);
        self.pitch = (self.pitch - dy).clamp(-MAX_PITCH, MAX_PITCH);
    }
}

/// Whether a walker can stand at a point: on the map, not in the water (a bridge's path carries them over it), not in a
/// building or a tree.
pub fn free_at(t: &Terrain, obstacles: &[Obstacle], x: f32, z: f32) -> bool {
    let (hw, hh) = (t.w as f32 * TILE_SIZE * 0.5, t.h as f32 * TILE_SIZE * 0.5);
    if x.abs() > hw - 1.0 || z.abs() > hh - 1.0 {
        return false;
    }
    let (tx, ty) = (((x + hw) / TILE_SIZE).floor() as i32, ((z + hh) / TILE_SIZE).floor() as i32);
    if is_water(t, tx, ty) && t.path_at(tx, ty) == 0 {
        return false;
    }
    obstacles.iter().all(|o| match *o {
        Obstacle::Box { cx, cz, hx, hz } => (x - cx).abs() >= hx + BODY || (z - cz).abs() >= hz + BODY,
        Obstacle::Circle { x: ox, z: oz, r } => (x - ox).powi(2) + (z - oz).powi(2) >= (r + BODY).powi(2),
    })
}

/// A step from `from` by `d`, blocked by what is not `free_at`: the whole step, else only its part along X or along Z
/// (sliding along a wall or a shore), else none. A walker who somehow stands where they cannot (placed there) may step
/// anywhere, so they can always walk out.
pub fn step(t: &Terrain, obstacles: &[Obstacle], from: (f32, f32), d: (f32, f32)) -> (f32, f32) {
    let (hw, hh) = (t.w as f32 * TILE_SIZE * 0.5, t.h as f32 * TILE_SIZE * 0.5);
    let clamp = |p: (f32, f32)| (p.0.clamp(-hw + 1.0, hw - 1.0), p.1.clamp(-hh + 1.0, hh - 1.0));
    let to = clamp((from.0 + d.0, from.1 + d.1));
    if !free_at(t, obstacles, from.0, from.1) {
        return to;
    }
    // a long step is taken in short ones, so it cannot jump over a thin thing
    let n = ((d.0.abs().max(d.1.abs())) / (BODY * 0.5)).ceil().max(1.0) as usize;
    let (sx, sz) = (d.0 / n as f32, d.1 / n as f32);
    let mut p = from;
    for _ in 0..n {
        let tries = [(p.0 + sx, p.1 + sz), (p.0 + sx, p.1), (p.0, p.1 + sz)];
        match tries.into_iter().map(clamp).find(|q| *q != p && free_at(t, obstacles, q.0, q.1)) {
            Some(q) => p = q,
            None => break,
        }
    }
    p
}

/// The eye's height at a ground point: the terrain's there plus `EYE`.
pub fn eye_height(t: &Terrain, x: f32, z: f32) -> f32 {
    t.height_at(x, z) + EYE
}

/// The heading (Cam::yaw) from one ground point to another.
pub fn heading(from: (f32, f32), to: (f32, f32)) -> f32 {
    (to.0 - from.0).atan2(-(to.1 - from.1)).to_degrees().rem_euclid(360.0)
}

/// The world point at a tile's centre.
fn tile_centre(t: &Terrain, tx: i32, ty: i32) -> (f32, f32) {
    ((tx as f32 + 0.5) * TILE_SIZE - t.w as f32 * TILE_SIZE * 0.5, (ty as f32 + 0.5) * TILE_SIZE - t.h as f32 * TILE_SIZE * 0.5)
}

/// Where a walk starts: the first hole's tee looking at its green, else the map's centre with the overhead camera's heading.
pub fn start(app: &App) -> Cam {
    let t = &app.terrain;
    if let Ok(v) = std::env::var("SG_WALK_AT") {
        let v: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if v.len() >= 3 {
            let (x, z) = tile_centre(t, v[0] as i32, v[1] as i32);
            return Cam { x, z, yaw: v[2], pitch: v.get(3).copied().unwrap_or(0.0) };
        }
    }
    if let Some(h) = app.club.holes.get(1).filter(|h| h.par > 0) {
        let tee = if h.back != (0, 0) { h.back } else { h.fwd };
        if tee != (0, 0) {
            let (x, z) = tile_centre(t, tee.0, tee.1);
            let pin = tile_centre(t, h.pin.0, h.pin.1);
            let yaw = if pin != (x, z) { heading((x, z), pin) } else { 45.0 + app.rot };
            return Cam { x, z, yaw, pitch: 0.0 };
        }
    }
    Cam { x: 0.0, z: 0.0, yaw: (45.0 + app.rot).rem_euclid(360.0), pitch: 0.0 }
}

/// A theme's sky and haze.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub zenith: [f32; 3],
    pub haze: [f32; 3],
    pub cloud: [f32; 3],
    /// How much of the sky the clouds cover (0..1).
    pub cover: f32,
    /// The haze's density per world unit.
    pub density: f32,
}

/// The sky of a theme (app::THEMES: 0 Parkland, 1 Links, 2 Desert, 3 Tropical): a soft blue over green country, a paler
/// cloudier northern sky by the sea, a deep clear blue over warm dusty haze, a bright sky with the tropics' tall clouds.
pub fn look(theme: usize) -> Look {
    match theme {
        1 => Look { zenith: [0.40, 0.53, 0.70], haze: [0.78, 0.82, 0.85], cloud: [0.95, 0.96, 0.97], cover: 0.60, density: 2.15e-4 },
        2 => Look { zenith: [0.20, 0.44, 0.82], haze: [0.91, 0.83, 0.71], cloud: [1.0, 0.98, 0.95], cover: 0.18, density: 1.45e-4 },
        3 => Look { zenith: [0.14, 0.47, 0.86], haze: [0.75, 0.87, 0.93], cloud: [1.0, 1.0, 1.0], cover: 0.40, density: 1.75e-4 },
        _ => Look { zenith: [0.29, 0.49, 0.83], haze: [0.74, 0.82, 0.90], cloud: [1.0, 1.0, 1.0], cover: 0.42, density: 1.7e-4 },
    }
}

/// Where the haze hides everything (its density's 99.5 %).
pub fn haze_end(density: f32) -> f32 {
    5.3f32.powf(1.0 / 1.5) / density
}

/// The sun's direction in the world: the overhead view's light (fixed in its eye space, sg_core::terrain) seen from its
/// first quarter turn, so the walk's shading and the art's shadows (drawn for that view) agree.
pub fn sun_direction() -> [f32; 3] {
    let m = Mat4::rotate(crate::walk_objects::art_pitch() as f64, 1.0, 0.0, 0.0).mul(&Mat4::rotate(45.0, 0.0, 1.0, 0.0)).0;
    let l = sg_core::terrain::light_direction();
    let v = [0, 1, 2].map(|c| m[c * 4] * l[0] + m[c * 4 + 1] * l[1] + m[c * 4 + 2] * l[2]);
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    v.map(|q| q / n)
}

/// Following a golfer (G).
#[derive(Clone, Copy, Debug)]
struct Follow {
    /// The golfer's slot.
    slot: usize,
    /// Watching the ball since this many seconds, or the golfer (None).
    ball: Option<f32>,
    /// How long the ball has lain still.
    still: f32,
    /// The camera's eased place and the point it looks at.
    pos: [f32; 3],
    at: [f32; 3],
    /// The ball's last place and its eased direction of travel (level).
    last_ball: Option<[f32; 3]>,
    dir: [f32; 2],
}

/// The frame times kept for SG_WALK_BENCH.
#[derive(Default)]
struct Bench {
    on: bool,
    frames: Vec<f32>,
    cpu: Vec<f32>,
    /// The time spent working out what stands on the ground and its triangles (CPU only, before anything is drawn).
    build: Vec<f32>,
    last: Option<std::time::Instant>,
}

/// The walk camera's state in the main loop.
#[derive(Default)]
pub struct Walking {
    /// On, and where it is; None in the overhead view.
    pub cam: Option<Cam>,
    /// The overhead camera's place, zoom and turn when the walk began, given back when it ends.
    saved: (f32, f32, f32, f32),
    /// Walk keys held: ahead, back, left, right.
    held: [bool; 4],
    /// A mouse button is down: moving turns the camera.
    looking: bool,
    /// The clock of the last step.
    last: f64,
    /// A scripted still's walk has begun.
    still_begun: bool,
    /// The walk's speed (world units a second, along X and Z), the eye's eased height and the head's bob.
    vel: [f32; 2],
    eye_y: Option<f32>,
    bob: f32,
    bob_amp: f32,
    follow: Option<Follow>,
    gpu: Option<WalkGpu>,
    gpu_failed: bool,
    ground: Option<Ground>,
    objs: Objects,
    /// The terrain textures given anisotropic filtering, given back their plain filtering when the walk ends.
    aniso: Vec<TextureId>,
    /// The seconds the walk has been shown (the water's and the clouds' clock), and the last frame's step.
    time: f32,
    dt: f32,
    bench: Bench,
}

fn held_index(k: KeyCode) -> Option<usize> {
    match k {
        KeyCode::W | KeyCode::Up => Some(0),
        KeyCode::S | KeyCode::Down => Some(1),
        KeyCode::A | KeyCode::Left => Some(2),
        KeyCode::D | KeyCode::Right => Some(3),
        _ => None,
    }
}

/// The golfers on the course, by slot.
fn on_course(app: &App) -> Vec<usize> {
    (0..app.club.g.len().min(sg_core::golfer::SLOTS))
        .filter(|&i| {
            let g = &app.club.g[i];
            g.hole > 0 && g.hole < 19 && g.x != 0
        })
        .collect()
}

/// The next golfer on the course after `from` (or before it, `back`), round the list.
fn next_golfer(list: &[usize], from: Option<usize>, back: bool) -> Option<usize> {
    if list.is_empty() {
        return None;
    }
    let Some(f) = from else { return Some(list[0]) };
    let pos = list.iter().position(|&s| s == f);
    let i = match (pos, back) {
        (Some(p), false) => (p + 1) % list.len(),
        (Some(p), true) => (p + list.len() - 1) % list.len(),
        (None, false) => list.iter().position(|&s| s > f).unwrap_or(0),
        (None, true) => list.iter().rposition(|&s| s < f).unwrap_or(list.len() - 1),
    };
    Some(list[i])
}

/// Eases `v` towards `to` at `rate` per second over `dt`.
fn ease(v: f32, to: f32, rate: f32, dt: f32) -> f32 {
    v + (to - v) * (1.0 - (-rate * dt).exp())
}

impl Stage {
    /// Whether the walk camera is drawn this frame.
    pub(crate) fn walk_shown(&self) -> bool {
        self.walk.cam.is_some() && walk_screen(&self.app)
    }

    fn walk_begin(&mut self) {
        let a = &self.app;
        self.walk.saved = (a.cam_x, a.cam_z, a.zoom, a.rot);
        self.walk.cam = Some(start(a));
        self.walk.held = [false; 4];
        self.walk.looking = false;
        self.walk.last = self.app.clock;
        self.walk.vel = [0.0; 2];
        self.walk.eye_y = None;
        self.walk.follow = None;
        self.walk.bench.on = std::env::var_os("SG_WALK_BENCH").is_some();
        if let Ok(v) = std::env::var("SG_WALK_FOLLOW") {
            self.walk_follow_start(v.trim().parse::<usize>().ok());
        }
    }

    fn walk_end(&mut self) {
        self.walk.cam = None;
        self.walk.follow = None;
        let a = &mut self.app;
        (a.cam_x, a.cam_z, a.zoom, a.rot) = self.walk.saved;
        // the overhead view's textures as they were; the walk's own are kept for the next walk
        for t in std::mem::take(&mut self.walk.aniso) {
            crate::walk_gl::set_anisotropy(&mut self.g, t, 1.0);
        }
        if let Some(gr) = self.walk.ground.take() {
            gr.free(&mut self.g);
        }
        self.walk.objs.free(&mut self.g);
        self.walk.objs = Objects::default();
    }

    /// Follows golfer `slot`, or the first on the course.
    fn walk_follow_start(&mut self, slot: Option<usize>) {
        let list = on_course(&self.app);
        let slot = match slot {
            Some(s) if list.contains(&s) => Some(s),
            _ => next_golfer(&list, slot, false),
        };
        let Some(slot) = slot else { return };
        let Some(c) = self.walk.cam else { return };
        let e = [c.x, self.walk.eye_y.unwrap_or_else(|| eye_height(&self.app.terrain, c.x, c.z)), c.z];
        let f = c.forward();
        self.walk.follow = Some(Follow {
            slot,
            ball: None,
            still: 0.0,
            pos: e,
            at: [e[0] + f[0] * 50.0, e[1] + f[1] * 50.0, e[2] + f[2] * 50.0],
            last_ball: None,
            dir: [0.0, -1.0],
        });
    }

    /// A key went down: true when the walk camera took it.
    pub(crate) fn walk_key(&mut self, k: KeyCode) -> bool {
        if !enabled() {
            return false;
        }
        if self.walk_shown() {
            if k == KEY {
                self.walk_end();
            } else if k == KeyCode::Escape {
                if self.walk.follow.take().is_none() {
                    self.walk_end();
                }
            } else if k == FOLLOW_KEY {
                let list = on_course(&self.app);
                let from = self.walk.follow.map(|f| f.slot);
                if let Some(s) = next_golfer(&list, from, self.shift) {
                    if self.walk.follow.is_some() {
                        if let Some(f) = self.walk.follow.as_mut() {
                            (f.slot, f.ball, f.last_ball, f.still) = (s, None, None, 0.0);
                        }
                    } else {
                        self.walk_follow_start(Some(s));
                    }
                }
            } else if let Some(i) = held_index(k) {
                self.walk.held[i] = true;
                // walking takes the camera back from the golfer it followed, where it is
                self.walk.follow = None;
            }
            return true;
        }
        if k == KEY && walk_screen(&self.app) {
            self.walk_begin();
            return true;
        }
        false
    }

    pub(crate) fn walk_key_up(&mut self, k: KeyCode) {
        if let Some(i) = held_index(k) {
            self.walk.held[i] = false;
        }
    }

    /// A mouse button: true when the walk camera took it.
    pub(crate) fn walk_button(&mut self, _button: MouseButton, down: bool) -> bool {
        if !self.walk_shown() {
            // the release of a button held when the walk ended is the walk's too
            return std::mem::take(&mut self.walk.looking) && !down;
        }
        self.walk.looking = down;
        true
    }

    /// The pointer moved by (dx, dy) drawable pixels: true when the walk camera took it.
    pub(crate) fn walk_motion(&mut self, dx: f32, dy: f32) -> bool {
        if !self.walk_shown() {
            return false;
        }
        if self.walk.looking && self.walk.follow.is_none() {
            let k = LOOK / self.app.dpi.max(1.0);
            if let Some(c) = self.walk.cam.as_mut() {
                c.look(dx * k, dy * k);
            }
        }
        true
    }

    /// Once a frame: the walk's steps, the eye's easing and the following, and a scripted still's start in the walk camera.
    pub(crate) fn walk_update(&mut self, t: f64) {
        if enabled() && self.png_out.is_some() && !self.walk.still_begun && walk_screen(&self.app) {
            self.walk.still_begun = true;
            self.walk_begin();
        }
        let dt = (t - self.walk.last).clamp(0.0, 0.1) as f32;
        self.walk.last = t;
        if !self.walk_shown() {
            return;
        }
        self.walk.time += dt;
        self.walk.dt = dt;
        if self.walk.bench.on {
            let now = std::time::Instant::now();
            if let Some(l) = self.walk.bench.last {
                self.walk.bench.frames.push(now.duration_since(l).as_secs_f32() * 1000.0);
            }
            self.walk.bench.last = Some(now);
        }
        if self.walk.follow.is_some() {
            self.walk_follow_tick(dt);
            self.walk_shot_hold();
            return;
        }
        let Some(mut c) = self.walk.cam else { return };
        let h = self.walk.held;
        let ahead = (h[0] as i32 - h[1] as i32) as f32;
        let right = (h[3] as i32 - h[2] as i32) as f32;
        let len = (ahead * ahead + right * right).sqrt().max(1.0);
        let speed = SPEED * METRE * if self.shift { FAST } else { 1.0 };
        let ([fx, fz], [rx, rz]) = c.axes();
        let want = [(fx * ahead + rx * right) / len * speed, (fz * ahead + rz * right) / len * speed];
        let rate = if ahead == 0.0 && right == 0.0 { BRAKE } else { ACCEL };
        let v = &mut self.walk.vel;
        v[0] = ease(v[0], want[0], rate, dt);
        v[1] = ease(v[1], want[1], rate, dt);
        if v[0].abs() + v[1].abs() < 0.02 {
            *v = [0.0; 2];
        }
        let terrain = &self.app.terrain;
        let before = (c.x, c.z);
        (c.x, c.z) = step(terrain, &self.walk.objs.obstacles, (c.x, c.z), (v[0] * dt, v[1] * dt));
        // what blocked the walk stops its speed that way
        let moved = ((c.x - before.0) / dt.max(1e-4), (c.z - before.1) / dt.max(1e-4));
        if dt > 0.0 {
            if moved.0.abs() < v[0].abs() * 0.5 {
                v[0] = moved.0;
            }
            if moved.1.abs() < v[1].abs() * 0.5 {
                v[1] = moved.1;
            }
        }
        // the head's bob, with the pace
        let pace = (v[0] * v[0] + v[1] * v[1]).sqrt();
        self.walk.bob += pace * dt / STEP * std::f32::consts::PI;
        self.walk.bob_amp = ease(self.walk.bob_amp, (pace / (SPEED * METRE)).min(1.0), 6.0, dt);
        let target = eye_height(terrain, c.x, c.z) + self.walk.bob.sin().abs() * BOB * self.walk.bob_amp;
        self.walk.eye_y = Some(match self.walk.eye_y {
            Some(y) if (y - target).abs() < 3.0 * METRE => ease(y, target, 14.0, dt),
            _ => target,
        });
        self.walk.cam = Some(c);
    }

    /// Test hook: with `SG_WALK_SHOT=S` a scripted still of a golfer followed waits (up to three minutes), the game
    /// playing on as it would on the screen, until their ball has been in the air or rolling for S seconds.
    fn walk_shot_hold(&mut self) {
        let Some(wait) = std::env::var("SG_WALK_SHOT").ok().and_then(|v| v.parse::<f32>().ok()) else { return };
        if self.png_out.is_none() || self.walk.time > 180.0 {
            return;
        }
        // (the first second's frames are still catching up with the still's --time; and only a shot whose ball is seen
        // to take off counts)
        let app = &self.app;
        if let Some(f) = self.walk.follow.as_mut() {
            let airborne = app.walk_ball(f.slot).is_some_and(|b| b[1] > app.terrain.height_at(b[0], b[2]) + 1.0 * METRE);
            if self.walk.time < 1.0 || f.ball.is_some_and(|t| t < 0.15 && !airborne) {
                f.ball = None;
            }
        }
        if self.walk.follow.is_some_and(|f| f.ball.is_none_or(|t| t < wait)) {
            self.frames = 0;
            self.app.time += self.walk.dt as f64;
        }
    }

    /// The camera behind the golfer followed, or after their ball.
    fn walk_follow_tick(&mut self, dt: f32) {
        use sg_core::golfer::flag;
        let Some(mut f) = self.walk.follow else { return };
        let app = &self.app;
        let list = on_course(app);
        if !list.contains(&f.slot) {
            // the golfer left the course: the next one
            match next_golfer(&list, Some(f.slot), false) {
                Some(s) => (f.slot, f.ball, f.last_ball) = (s, None, None),
                None => {
                    self.walk.follow = None;
                    return;
                }
            }
        }
        // test hook: SG_WALK_FOLLOW=next follows whoever hits next
        if f.ball.is_none() && std::env::var("SG_WALK_FOLLOW").is_ok_and(|v| v == "next") {
            let flying = |s: usize| {
                app.club.g[s].flags & flag::BALL_MOVING != 0
                    && app.walk_ball(s).is_some_and(|b| b[1] > app.terrain.height_at(b[0], b[2]) + 2.0 * METRE)
            };
            if let Some(&s) = list.iter().find(|&&s| flying(s)) {
                if s != f.slot {
                    (f.slot, f.last_ball) = (s, None);
                }
            }
        }
        let gl = &app.club.g[f.slot];
        let (gx, gz) = app.units_to_world(gl.x, gl.y);
        let ground = app.terrain.height_at(gx, gz);
        let pin = app.club.holes.get(gl.hole as usize).filter(|h| h.par > 0).map(|h| app.terrain.tile_centre(h.pin.0, h.pin.1));
        let ball = app.walk_ball(f.slot);
        let moving = gl.flags & flag::BALL_MOVING != 0 && ball.is_some();
        if ball.is_none() {
            // holed, or picked up
            f.still += dt;
        }
        if let (Some(b), Some(lb)) = (ball, f.last_ball) {
            let (dx, dz) = (b[0] - lb[0], b[2] - lb[2]);
            let l = (dx * dx + dz * dz).sqrt();
            if l > 0.05 {
                f.dir = [ease(f.dir[0], dx / l, 3.0, dt), ease(f.dir[1], dz / l, 3.0, dt)];
                f.still = 0.0;
            } else {
                f.still += dt;
            }
        }
        f.last_ball = ball;
        match f.ball {
            None if moving => {
                f.ball = Some(0.0);
                f.still = 0.0;
                // the ball sets off the way the golfer faces their target
                if let Some((px, pz)) = pin {
                    let (dx, dz) = (px - gx, pz - gz);
                    let l = (dx * dx + dz * dz).sqrt().max(1e-3);
                    f.dir = [dx / l, dz / l];
                }
            }
            Some(t) if !moving && f.still > 1.2 || t > 14.0 => f.ball = None,
            Some(t) => f.ball = Some(t + dt),
            None => {}
        }
        let (want_pos, want_at, rate) = match (f.ball, ball) {
            (Some(_), Some(b)) => {
                let d = f.dir;
                let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-3);
                let (dx, dz) = (d[0] / l, d[1] / l);
                let back = 9.0 * METRE;
                let p = [b[0] - dx * back, b[1].max(app.terrain.height_at(b[0], b[2])) + 2.5 * METRE, b[2] - dz * back];
                (p, b, 4.0)
            }
            _ => {
                // behind the golfer and a little to the side, looking past them down the hole
                let (dx, dz) = match pin {
                    Some((px, pz)) => {
                        let (dx, dz) = (px - gx, pz - gz);
                        let l = (dx * dx + dz * dz).sqrt();
                        if l > 1.0 {
                            (dx / l, dz / l)
                        } else {
                            (f.dir[0], f.dir[1])
                        }
                    }
                    None => (f.dir[0], f.dir[1]),
                };
                let back = 5.5 * METRE;
                let side = 1.6 * METRE;
                let p = [gx - dx * back - dz * side, ground + EYE, gz - dz * back + dx * side];
                let at = [gx + dx * 40.0 * METRE, ground + 0.6 * EYE, gz + dz * 40.0 * METRE];
                (p, at, 2.5)
            }
        };
        // a scripted still (its time run through at once) is taken from where the camera is going
        let still = self.png_out.is_some() && std::env::var_os("SG_WALK_SHOT").is_none();
        let teleport = still || ((f.pos[0] - want_pos[0]).powi(2) + (f.pos[2] - want_pos[2]).powi(2)).sqrt() > 120.0 * METRE;
        for k in 0..3 {
            if teleport {
                f.pos[k] = want_pos[k];
                f.at[k] = want_at[k];
            } else {
                f.pos[k] = ease(f.pos[k], want_pos[k], rate, dt);
                f.at[k] = ease(f.at[k], want_at[k], rate * 1.6, dt);
            }
        }
        // never under the ground
        f.pos[1] = f.pos[1].max(app.terrain.height_at(f.pos[0], f.pos[2]) + 0.6 * METRE);
        let (dx, dy, dz) = (f.at[0] - f.pos[0], f.at[1] - f.pos[1], f.at[2] - f.pos[2]);
        let level = (dx * dx + dz * dz).sqrt().max(1e-3);
        let cam = Cam {
            x: f.pos[0],
            z: f.pos[2],
            yaw: heading((f.pos[0], f.pos[2]), (f.at[0], f.at[2])),
            pitch: dy.atan2(level).to_degrees().clamp(-MAX_PITCH, MAX_PITCH),
        };
        if std::env::var_os("SG_WALK_DUMP").is_some() {
            println!(
                "walk follow: t {:.2} slot {} golfer {:.0},{:.0} hole {} flags {:x} ball {:?} moving {moving} phase {:?} cam {:.0},{:.0} at {:.0},{:.0}",
                self.walk.time, f.slot, gx, gz, gl.hole, gl.flags, ball, f.ball, f.pos[0], f.pos[2], f.at[0], f.at[2]
            );
        }
        self.walk.cam = Some(cam);
        self.walk.eye_y = Some(f.pos[1]);
        self.walk.follow = Some(f);
    }
}

/// Draws the walk camera's view (main.rs's frame).
pub(crate) fn draw(app: &mut App, g: &mut Gfx, walk: &mut Walking) {
    let t0 = std::time::Instant::now();
    app.render_walk(g, walk);
    if walk.bench.on {
        walk.bench.cpu.push(t0.elapsed().as_secs_f32() * 1000.0);
        if walk.bench.cpu.len().is_multiple_of(120) {
            walk.bench_report();
        }
    }
}

impl Walking {
    fn bench_report(&self) {
        let stats = |v: &[f32]| {
            let mut s: Vec<f32> = v.iter().copied().skip(10).collect();
            if s.is_empty() {
                return (0.0, 0.0, 0.0);
            }
            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mean = s.iter().sum::<f32>() / s.len() as f32;
            (mean, s[s.len() / 2], s[(s.len() * 95 / 100).min(s.len() - 1)])
        };
        let (fm, fmed, f95) = stats(&self.bench.frames);
        let (cm, cmed, c95) = stats(&self.bench.cpu);
        let (bm, _, b95) = stats(&self.bench.build);
        println!(
            "walk bench: {} frames; frame mean {fm:.2} ms, median {fmed:.2}, 95% {f95:.2}; walk drawing with its GL calls mean {cm:.2} ms, median {cmed:.2}, 95% {c95:.2}; of which the objects' geometry (CPU) mean {bm:.2} ms, 95% {b95:.2}; things drawn {}, left out {}",
            self.bench.cpu.len(),
            self.objs.drawn,
            self.objs.culled
        );
    }
}

/// The screens the walk camera shows on: the course, with no photo mode, fly-over or notice.
pub(crate) fn walk_screen(app: &App) -> bool {
    app.screen == Screen::Play && app.photo.is_none() && app.showcase.is_none() && app.editor.is_none() && app.notice.is_none()
}

/// The hole nearest a point: its number, par and the distance to its pin in yards.
pub fn nearest_hole(app: &App, x: f32, z: f32) -> Option<(usize, i32, f32)> {
    let t = &app.terrain;
    let mut best: Option<(f32, usize)> = None;
    for (n, h) in app.club.holes.iter().enumerate().skip(1).take(18) {
        if h.par <= 0 || h.pin == (0, 0) {
            continue;
        }
        let tee = if h.back != (0, 0) { h.back } else { h.fwd };
        let (px, pz) = tile_centre(t, h.pin.0, h.pin.1);
        let (tx, tz) = if tee != (0, 0) { tile_centre(t, tee.0, tee.1) } else { (px, pz) };
        // the distance to the line from tee to pin
        let (dx, dz) = (px - tx, pz - tz);
        let l2 = (dx * dx + dz * dz).max(1e-6);
        let k = (((x - tx) * dx + (z - tz) * dz) / l2).clamp(0.0, 1.0);
        let d = ((x - tx - dx * k).powi(2) + (z - tz - dz * k).powi(2)).sqrt();
        if best.is_none_or(|b| d < b.0) {
            best = Some((d, n));
        }
    }
    let (_, n) = best?;
    let h = &app.club.holes[n];
    let (px, pz) = tile_centre(t, h.pin.0, h.pin.1);
    let yards = ((px - x).powi(2) + (pz - z).powi(2)).sqrt() / TILE_SIZE * 25.0;
    Some((n, h.par, yards))
}

/// The compass's points, in degrees of heading.
const POINTS: [(&str, f32); 8] =
    [("N", 0.0), ("NE", 45.0), ("E", 90.0), ("SE", 135.0), ("S", 180.0), ("SW", 225.0), ("W", 270.0), ("NW", 315.0)];

impl App {
    /// Draws the course from the walk camera, and the walk's line of help.
    pub(crate) fn render_walk(&mut self, g: &mut Gfx, wk: &mut Walking) {
        let Some(cam) = wk.cam else { return };
        if wk.gpu.is_none() && !wk.gpu_failed {
            wk.gpu = WalkGpu::new(g);
            wk.gpu_failed = wk.gpu.is_none();
        }
        let key = crate::walk_ground::ground_key(self);
        if wk.ground.as_ref().is_none_or(|gr| gr.key != key) {
            if let Some(old) = wk.ground.take() {
                old.free(g);
            }
            let gr = self.walk_ground(g);
            // the terrain art filtered anisotropically while it is seen from the ground (Enhanced graphics' own filtering
            // already does)
            if !crate::enhanced::terrain_filter() {
                for c in &gr.chunks {
                    for p in &c.parts {
                        if !wk.aniso.contains(&p.tex) {
                            crate::walk_gl::set_anisotropy(g, p.tex, 8.0);
                            wk.aniso.push(p.tex);
                        }
                    }
                }
            }
            wk.ground = Some(gr);
        }
        let Some(gpu) = wk.gpu.as_mut() else { return };
        let Some(ground) = wk.ground.as_ref() else { return };
        let aspect = self.draw_w / self.draw_h.max(1.0);
        let proj = Mat4::perspective(FOV, aspect, NEAR, FAR);
        let eye = [cam.x, wk.eye_y.unwrap_or_else(|| eye_height(&self.terrain, cam.x, cam.z)), cam.z];
        let view = cam.view_from(eye);
        let mut vp = proj.mul(&view);
        let mut eye = eye;
        let mut lk = look(self.theme);
        // test hook: SG_WALK_ISO=Q draws the walk camera's scene through the overhead camera of quarter turn Q (orthographic,
        // one art pixel to a screen pixel, times SG_WALK_ISO_ZOOM), centred on the walk's place: there each object's solid,
        // with its sprite projected onto it, must look as the sprite itself does
        let iso_q = std::env::var("SG_WALK_ISO").ok().and_then(|v| v.parse::<i32>().ok());
        if let Some(q) = iso_q {
            let zoom = std::env::var("SG_WALK_ISO_ZOOM").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
            let upp = sg_core::sprites::SPRITE_UNITS_PER_PIXEL / zoom;
            let (hw, hh) = (self.draw_w * upp * 0.5, self.draw_h * upp * 0.5);
            let gy = self.terrain.height_at(cam.x, cam.z);
            let m = Mat4::rotate(crate::walk_objects::art_pitch() as f64, 1.0, 0.0, 0.0)
                .mul(&Mat4::rotate((45.0 + 90.0 * q as f32) as f64, 0.0, 1.0, 0.0))
                .mul(&Mat4::translate(-cam.x, -gy, -cam.z));
            vp = Mat4::ortho(-hw, hw, -hh, hh, -12000.0, 12000.0).mul(&m);
            let fwd = crate::walk_objects::iso(q).f;
            eye = [cam.x - fwd[0] * 6000.0, gy - fwd[1] * 6000.0, cam.z - fwd[2] * 6000.0];
            lk.density = 1e-9;
        }
        let sun = sun_direction();
        let far = haze_end(lk.density);
        let f = cam.forward();
        let (_, [rx, rz]) = cam.axes();
        let r = [rx, 0.0, rz];
        let up = [r[1] * f[2] - r[2] * f[1], r[2] * f[0] - r[0] * f[2], r[0] * f[1] - r[1] * f[0]];
        let ty = (FOV.to_radians() * 0.5).tan();
        let tx = ty * aspect;
        let time = wk.time;
        let v4 = |a: [f32; 3], w: f32| [a[0], a[1], a[2], w];
        let focal = self.draw_h * 0.5 / ty;
        let b = crate::render::BRIGHTNESS;
        let base = WalkUniforms {
            mvp: vp.0,
            pull: v4(eye, 0.0),
            eye: v4(eye, time),
            sun: v4(sun, mode::GROUND),
            haze: v4(lk.haze, lk.density),
            zenith: v4(lk.zenith, 0.5),
            amb: v4(self.light.ambient.map(|v| v * b), 0.55),
            dif: v4(self.light.diffuse.map(|v| v * b), 1.0),
            info: [64.0, 0.0, focal, 0.0],
            water: v4(ground.water_colour, 0.0),
        };
        let planes = frustum(&vp);
        let with_size = |u: WalkUniforms, size: f32| WalkUniforms { info: [size, u.info[1], u.info[2], u.info[3]], ..u };
        if let Some(p) = &ground.skirt {
            gpu.draw_mesh(g, Pipe::Solid, &p.mesh, p.tex, p.clean, &with_size(base, p.size));
        }
        let near_far = |lo: [f32; 3], hi: [f32; 3]| {
            // the nearest point of the box to the eye
            let d: f32 = (0..3).map(|k| (eye[k] - eye[k].clamp(lo[k], hi[k])).powi(2)).sum::<f32>().sqrt();
            d < far
        };
        for c in &ground.chunks {
            if !box_visible(&planes, c.lo, c.hi) || !near_far(c.lo, c.hi) {
                continue;
            }
            for p in &c.parts {
                gpu.draw_mesh(g, Pipe::Solid, &p.mesh, p.tex, p.clean, &with_size(base, p.size));
            }
        }
        let art = |c: u8, r: f32| {
            let mut u = base.with_mode(mode::ART);
            u.info[1] = c as f32;
            u.zenith[3] = r;
            u
        };
        for bt in &self.wall_batches {
            gpu.draw_mesh(g, Pipe::Solid, &bt.mesh, bt.tex, bt.tex, &art(cat::ROCK, 0.04));
        }
        let water = base.with_mode(mode::WATER);
        for c in &ground.water {
            if !box_visible(&planes, c.lo, c.hi) || !near_far(c.lo, c.hi) {
                continue;
            }
            for p in &c.parts {
                gpu.draw_mesh(g, Pipe::Solid, &p.mesh, p.tex, p.clean, &with_size(water, p.size));
            }
        }
        if let Some(p) = &ground.sea {
            gpu.draw_mesh(g, Pipe::Solid, &p.mesh, p.tex, p.clean, &with_size(water, p.size));
        }
        self.walk_far_trees(g, &mut wk.objs, ground);
        self.walk_draw_far_trees(g, gpu, &wk.objs, &base);
        let (right, quarter) = match iso_q {
            Some(q) => ([crate::walk_objects::iso(q).r[0], crate::walk_objects::iso(q).r[2]], q.rem_euclid(4)),
            None => ([rx, rz], cam.quarter()),
        };
        let sc = Scene { eye, yaw: cam.yaw, right, planes, u: base, far, time, quarter };
        let t0 = std::time::Instant::now();
        self.walk_props(g, gpu, &mut wk.objs, &sc, wk.dt);
        if wk.bench.on {
            wk.bench.build.push(t0.elapsed().as_secs_f32() * 1000.0);
        }
        gpu.submit(g);
        if iso_q.is_none() {
            gpu.draw_sky(
                g,
                &SkyUniforms {
                    cam_f: v4(f, 0.0),
                    cam_r: v4(r.map(|v| v * tx), 0.0),
                    cam_u: v4(up.map(|v| v * ty), 0.0),
                    eye: v4(eye, time),
                    sun: v4(sun, 0.0),
                    haze: v4(lk.haze, 0.0),
                    zenith: v4(lk.zenith, lk.cover),
                    cloud: v4(lk.cloud, 0.00012),
                },
            );
        }
        // what is blended over the ground, the things and the sky: the paths, the shadows, the balls' trails
        let mut paths = art(cat::PATH, 0.02);
        paths.pull[3] = 0.0008;
        for bt in self.path_batches.iter().chain(&self.mud_batches) {
            gpu.draw_mesh(g, Pipe::Blend, &bt.mesh, bt.tex, bt.tex, &paths);
        }
        gpu.submit_late(g);
        self.walk_hud(g, wk, &cam);
    }

    /// The walk's help line, its compass and the hole nearest.
    fn walk_hud(&self, g: &mut Gfx, wk: &Walking, cam: &Cam) {
        let s = Ui::new(self.draw_w, self.draw_h);
        let (l, tp, rt, bt) = s.extent();
        let white = rgba(1.0, 1.0, 1.0, 0.92);
        let shade = rgba(0.0, 0.0, 0.0, 0.42);
        // the compass: a strip of the headings round the one looked along
        let cw = 260.0;
        let cx = rt - 8.0 - cw * 0.5;
        let y = tp + 8.0;
        s.fill(g, cx - cw * 0.5, y, cw, 22.0, shade);
        for k in 0..24 {
            let hd = k as f32 * 15.0;
            let d = ((hd - cam.yaw + 540.0).rem_euclid(360.0)) - 180.0;
            if d.abs() > 62.0 {
                continue;
            }
            let x = cx + d / 62.0 * (cw * 0.5 - 8.0);
            match POINTS.iter().find(|p| p.1 == hd) {
                Some((name, _)) => {
                    let c = if *name == "N" { rgba(1.0, 0.55, 0.45, 1.0) } else { white };
                    s.put_centered(g, F_ARIAL10, x, y + 6.0, name, c);
                }
                None => s.line(g, x, y + 8.0, x, y + 14.0, 1.0, rgba(1.0, 1.0, 1.0, 0.6)),
            }
        }
        s.line(g, cx, y + 18.0, cx, y + 22.0, 2.0, rgba(1.0, 0.85, 0.3, 1.0));
        // the hole nearest, or the golfer followed
        let line = match wk.follow {
            Some(f) => {
                let list = on_course(self);
                let n = list.iter().position(|&x| x == f.slot).map(|p| p + 1).unwrap_or(0);
                let gl = &self.club.g[f.slot];
                let hole = self.club.holes.get(gl.hole as usize).map(|h| h.par).unwrap_or(0);
                let what = if f.ball.is_some() { "watching the ball" } else { "walking with them" };
                format!("Following golfer {n} of {}: hole {}, par {hole}, stroke {} - {what}", list.len(), gl.hole, gl.strokes + 1)
            }
            None => match nearest_hole(self, cam.x, cam.z) {
                Some((n, par, yards)) => format!("Hole {n}   Par {par}   {:.0} yards to the pin", yards),
                None => "No holes yet".to_string(),
            },
        };
        let w = F_INFO14.width(&line) + 16.0;
        s.fill(g, l + 8.0, tp + 8.0, w, 22.0, shade);
        s.put(g, F_INFO14, l + 16.0, tp + 11.0, &line, white);
        let help = "Walk camera: W A S D or arrows walk, Shift run, drag to look, G follow a golfer (Shift+G back), F12 or Esc back";
        let w = F_ARIAL10.width(help) + 12.0;
        s.fill(g, l + 6.0, bt - 20.0, w, 14.0, shade);
        s.put(g, F_ARIAL10, l + 12.0, bt - 18.0, help, rgba(1.0, 1.0, 1.0, 0.85));
        g.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hill() -> Terrain {
        let mut t = Terrain::demo_course(8, 8, 1);
        t.corner.iter_mut().for_each(|c| *c = 0);
        t.tile_corner.clear();
        // one corner raised four levels in the middle of the map
        t.corner[4 * 9 + 4] = 4;
        t
    }

    fn flat_grass() -> Terrain {
        let mut t = hill();
        t.corner.iter_mut().for_each(|c| *c = 0);
        t.ty.iter_mut().for_each(|v| *v = 4);
        t.path_kind = vec![0; t.ty.len()];
        t
    }

    #[test]
    fn the_eye_follows_the_ground() {
        let t = hill();
        // the corner (4, 4) is the map's centre: 4 levels up there, flat ground far from it
        assert!((eye_height(&t, 0.0, 0.0) - (4.0 * sg_core::terrain::HEIGHT_STEP + EYE)).abs() < 1e-3);
        assert!((eye_height(&t, -350.0, -350.0) - EYE).abs() < 1e-3);
        // half way up the slope, half the height
        let mid = eye_height(&t, 50.0, 0.0);
        assert!((mid - (2.0 * sg_core::terrain::HEIGHT_STEP + EYE)).abs() < 1e-3);
        // a step towards the hill climbs it, and the eye stays 1.7 m over the ground
        let mut c = Cam { x: 100.0, z: 0.0, yaw: 270.0, pitch: 0.0 };
        let before = c.eye(&t)[1];
        c.walk(&t, 50.0, 0.0);
        assert!((c.x - 50.0).abs() < 1e-3 && c.z.abs() < 1e-3);
        assert!(c.eye(&t)[1] > before);
        assert!((c.eye(&t)[1] - t.height_at(c.x, c.z) - 1.7 * METRE).abs() < 1e-3);
        // and the map's edge stops the walk
        c.walk(&t, 10_000.0, 0.0);
        assert!(c.x > -400.0 && c.x < -398.0);
    }

    #[test]
    fn eye_height_is_a_golfers() {
        // 25 yards a tile: 1.7 m is about 7.4 units
        assert!((EYE - 7.437).abs() < 0.01);
    }

    #[test]
    fn the_point_looked_at_is_at_the_screen_centre() {
        let t = hill();
        for (yaw, pitch) in [(0.0, 0.0), (45.0, 10.0), (200.0, -20.0)] {
            let c = Cam { x: -200.0, z: 150.0, yaw, pitch };
            let e = c.eye(&t);
            let f = c.forward();
            let p = [e[0] + f[0] * 300.0, e[1] + f[1] * 300.0, e[2] + f[2] * 300.0];
            let m = Mat4::perspective(FOV, 1.6, NEAR, 8000.0).mul(&c.view(&t));
            let q = m.apply(p);
            assert!((q[0] / q[3]).abs() < 1e-4 && (q[1] / q[3]).abs() < 1e-4, "{yaw} {pitch}");
            // the walk's right is on the screen's right
            let ([_, _], [rx, rz]) = c.axes();
            let q = m.apply([p[0] + rx * 50.0, p[1], p[2] + rz * 50.0]);
            assert!(q[0] / q[3] > 0.1);
        }
    }

    #[test]
    fn headings_and_quarters() {
        // world -Z is heading 0, +X 90
        assert!((heading((0.0, 0.0), (0.0, -10.0)) - 0.0).abs() < 1e-4);
        assert!((heading((0.0, 0.0), (10.0, 0.0)) - 90.0).abs() < 1e-4);
        assert!((heading((0.0, 0.0), (0.0, 10.0)) - 180.0).abs() < 1e-4);
        // the overhead camera's own headings give its quarter turns
        for q in 0..4 {
            let c = Cam { x: 0.0, z: 0.0, yaw: 45.0 + q as f32 * 90.0, pitch: 0.0 };
            assert_eq!(c.quarter(), q);
        }
        assert_eq!(Cam { x: 0.0, z: 0.0, yaw: 10.0, pitch: 0.0 }.quarter(), 0);
        assert_eq!(Cam { x: 0.0, z: 0.0, yaw: 350.0, pitch: 0.0 }.quarter(), 3);
        // the overhead camera's heading at turn 0 matches the eye's view direction of the overhead matrix
        let m = Mat4::rotate(40.0, 1.0, 0.0, 0.0).mul(&Mat4::rotate(45.0, 0.0, 1.0, 0.0)).0;
        let (vx, vz) = (-m[2], -m[10]);
        let ([fx, fz], _) = Cam { x: 0.0, z: 0.0, yaw: 45.0, pitch: 0.0 }.axes();
        let l = (vx * vx + vz * vz).sqrt();
        assert!((vx / l - fx).abs() < 1e-4 && (vz / l - fz).abs() < 1e-4);
    }

    #[test]
    fn looking_is_held_short_of_straight_up() {
        let mut c = Cam { x: 0.0, z: 0.0, yaw: 350.0, pitch: 0.0 };
        c.look(20.0, -200.0);
        assert!((c.yaw - 10.0).abs() < 1e-4);
        assert_eq!(c.pitch, MAX_PITCH);
        c.look(0.0, 400.0);
        assert_eq!(c.pitch, -MAX_PITCH);
    }

    #[test]
    fn walls_and_trunks_stop_the_walk_and_it_slides_along_them() {
        let t = flat_grass();
        let wall = [Obstacle::Box { cx: 0.0, cz: 0.0, hx: 50.0, hz: 50.0 }];
        // walking straight at the wall from the south stops at its face, a body's width off
        let p = step(&t, &wall, (0.0, 150.0), (0.0, -200.0));
        assert!((p.1 - (50.0 + BODY)).abs() < BODY * 0.6, "{p:?}");
        assert!(p.0.abs() < 1e-3);
        // walking at it slantwise slides along it
        let p = step(&t, &wall, (0.0, 60.0), (40.0, -40.0));
        assert!((p.0 - 40.0).abs() < 1e-3 && p.1 >= 50.0 + BODY, "{p:?}");
        // a trunk
        let tree = [Obstacle::Circle { x: 0.0, z: 0.0, r: 3.0 }];
        let p = step(&t, &tree, (0.0, 30.0), (0.0, -60.0));
        assert!(p.1 >= 3.0 + BODY - 1e-3 && p.1 < 3.0 + BODY + BODY, "{p:?}");
        // placed inside, a walker can walk out
        let p = step(&t, &wall, (0.0, 0.0), (0.0, 10.0));
        assert!((p.1 - 10.0).abs() < 1e-3);
    }

    #[test]
    fn water_stops_the_walk_but_a_bridge_does_not() {
        let mut t = flat_grass();
        // a column of water at x = 4
        for y in 0..t.h {
            let i = t.tile_index(4, y);
            t.ty[i] = 17;
        }
        let (wx, _) = tile_centre(&t, 4, 2);
        let (sx, sz) = tile_centre(&t, 2, 2);
        let p = step(&t, &[], (sx, sz), (300.0, 0.0));
        assert!(p.0 < wx - TILE_SIZE * 0.5 + 1e-3, "{p:?}");
        // a path over the water (a bridge) carries the walk across
        let i = t.tile_index(4, 2);
        t.path_kind[i] = 1;
        let p = step(&t, &[], (sx, sz), (300.0, 0.0));
        assert!((p.0 - (sx + 300.0)).abs() < 1e-3, "{p:?}");
    }

    #[test]
    fn following_cycles_round_the_golfers() {
        let list = [2, 5, 9];
        assert_eq!(next_golfer(&list, None, false), Some(2));
        assert_eq!(next_golfer(&list, Some(2), false), Some(5));
        assert_eq!(next_golfer(&list, Some(9), false), Some(2));
        assert_eq!(next_golfer(&list, Some(2), true), Some(9));
        // a golfer who left: the next one on
        assert_eq!(next_golfer(&list, Some(6), false), Some(9));
        assert_eq!(next_golfer(&list, Some(6), true), Some(5));
        assert_eq!(next_golfer(&[], Some(1), false), None);
    }

    #[test]
    fn the_haze_ends_where_it_is_thick() {
        for theme in 0..4 {
            let lk = look(theme);
            let d = haze_end(lk.density);
            let f = 1.0 - (-(d * lk.density).powf(1.5)).exp();
            assert!((f - 0.995).abs() < 1e-3);
            // the hills beyond the property show through it, faintly
            assert!(d > 12000.0 && d < FAR);
        }
    }

    #[test]
    fn the_sun_is_up() {
        let s = sun_direction();
        assert!(s[1] > 0.2 && s[1] < 0.95, "{s:?}");
        assert!(((s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn easing_gets_there() {
        let mut v = 0.0;
        for _ in 0..120 {
            v = ease(v, 10.0, ACCEL, 1.0 / 60.0);
        }
        assert!((v - 10.0).abs() < 0.01);
        // half a second gets most of the way, a frame only a little
        assert!(ease(0.0, 10.0, ACCEL, 1.0 / 60.0) < 1.5);
    }
}

#[cfg(test)]
mod cull_tests {
    use super::*;
    use crate::walk_objects::{box_visible, frustum};

    /// The chunks the culling keeps are exactly those with a point on the screen (sampled), for a camera looking down.
    #[test]
    fn culling_keeps_every_chunk_on_the_screen() {
        let cam = Cam { x: 1450.0, z: 1650.0, yaw: 350.0, pitch: -9.0 };
        let eye = [cam.x, 22.4, cam.z];
        let vp = Mat4::perspective(FOV, 1280.0 / 720.0, NEAR, FAR).mul(&cam.view_from(eye));
        let planes = frustum(&vp);
        for cz in 0..5 {
            for cx in 0..5 {
                let lo = [-2500.0 + cx as f32 * 1200.0, 0.0, -2500.0 + cz as f32 * 1200.0];
                let hi = [(lo[0] + 1200.0).min(2500.0), 45.0, (lo[2] + 1200.0).min(2500.0)];
                let mut seen = false;
                for i in 0..=40 {
                    for j in 0..=40 {
                        for y in [lo[1], hi[1]] {
                            let p = [lo[0] + (hi[0] - lo[0]) * i as f32 / 40.0, y, lo[2] + (hi[2] - lo[2]) * j as f32 / 40.0];
                            let c = vp.apply(p);
                            if c[3] > 0.0 && c[0].abs() <= c[3] && c[1].abs() <= c[3] && c[2].abs() <= c[3] {
                                seen = true;
                            }
                        }
                    }
                }
                if seen {
                    assert!(box_visible(&planes, lo, hi), "chunk {lo:?} {hi:?} is on the screen but culled");
                }
            }
        }
    }
}
