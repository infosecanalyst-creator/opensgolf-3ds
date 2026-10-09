//! One golfer playing one hole: the loop of walk, address, swing, ball flight, lie and putt.
//!
//! The range and flight follow the exe's rules (crate::flight); distances in yards, dispersion and penalties are still
//! placeholders (see docs/GAMELOGIC.md). The animation timings come from the sprite files.
use crate::flight;
use crate::terrain::{is_sand, is_water, Terrain, TILE_SIZE, TT_FIRM_FAIRWAY, TT_ROCK, TT_TEE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GolferAnim {
    Walk = 0,
    Address = 1,
    Swing = 2,
    PuttAddress = 3,
    Putt = 4,
    Happy = 5,
}

// Sprite timings, from the FLC files (frames per view x 83 ms): Male*_PerfectSwing 20 frames, ball struck near frame 11 (club
// horizontal in front of the golfer); *_NormalAddress 18 frames; *_Putt 33 frames.
pub const FRAME_SEC: f32 = 0.083;
pub const SWING_SEC: f32 = 20.0 * FRAME_SEC;
pub const SWING_IMPACT_SEC: f32 = 11.0 * FRAME_SEC;
pub const ADDRESS_SEC: f32 = 18.0 * FRAME_SEC;
pub const PUTT_SEC: f32 = 33.0 * FRAME_SEC;
pub const PUTT_IMPACT_SEC: f32 = 12.0 * FRAME_SEC;

/// Skill levels 0..15 in the order of progolfers.dta. The NAMES are from the data file; what each level DOES here is a placeholder
/// (see docs/GAMELOGIC.md). Draw, fade and backspin are not modelled yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GolferSkills {
    pub v: [i32; 10],
}

impl GolferSkills {
    pub const POWER: usize = 0;
    pub const LONG_DRIVER: usize = 1;
    pub const ACC_DRIVER: usize = 2;
    pub const ACC_IRONS: usize = 3;
    pub const ACC_PUTTER: usize = 4;
    pub const DRAW: usize = 5;
    pub const FADE: usize = 6;
    pub const BACKSPIN: usize = 7;
    pub const RECOVERY: usize = 8;
    pub const LUCK: usize = 9;
}

impl Default for GolferSkills {
    fn default() -> Self {
        GolferSkills { v: [7; 10] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Walk,
    Address,
    Swing,
    Flight,
    Settle,
    PuttAddress,
    Putt,
    Roll,
    Celebrate,
    Pause,
}

const PI: f32 = std::f32::consts::PI;
/// World units per tile; the exe uses 1024.
const TILE_SIZE_WORLD: f32 = TILE_SIZE;
/// World units per second (placeholder).
const WALK_SPEED: f32 = 130.0;
/// Placeholder carry for a full swing.
const DRIVE_DIST: f32 = 900.0;
/// Switch to the putter inside this distance of the hole.
const PUTT_RANGE: f32 = 230.0;

#[derive(Clone, Debug)]
pub struct ShotSim {
    /// Walking speed multiplier (a Ranger speeds play up).
    pub pace_scale: f32,
    /// Set before init(); init() keeps them.
    pub skills: GolferSkills,
    // Outputs, read by the renderer every frame.
    pub anim: GolferAnim,
    /// Seconds since the animation started.
    pub anim_time: f32,
    /// World position of the golfer's feet.
    pub golfer_x: f32,
    pub golfer_z: f32,
    /// Degrees, atan2(z, x), the direction the body faces.
    pub golfer_heading: f32,
    pub ball_x: f32,
    pub ball_z: f32,
    /// Height above the ground; -1 once holed.
    pub ball_h: f32,
    pub stroke: i32,
    /// Club of the latest stroke: "drive", "iron" or "putt".
    pub club: &'static str,
    /// Short text of the latest event (for the log).
    pub event: &'static str,
    /// Distance from the hole of the latest putt when it was struck (world units).
    pub last_putt_dist: f32,
    /// Start over after the hole (the game turns this off to move to the next hole).
    pub looping: bool,
    /// Set after the celebration when `looping` is false.
    pub finished: bool,
    /// The route of the hole being played (x,z pairs, first the tee, last the green). Defaults to Terrain::path.
    route: Option<Vec<f32>>,
    phase: Phase,
    rng: u32,
    phase_time: f32,
    shot_from_x: f32,
    shot_from_z: f32,
    land_x: f32,
    land_z: f32,
    flight_sec: f32,
    flight_peak: f32,
    aim_heading: f32,
    struck: bool,
    missed: bool,
}

impl Default for ShotSim {
    fn default() -> Self {
        ShotSim {
            pace_scale: 1.0,
            skills: GolferSkills::default(),
            anim: GolferAnim::Walk,
            anim_time: 0.0,
            golfer_x: 0.0,
            golfer_z: 0.0,
            golfer_heading: 0.0,
            ball_x: 0.0,
            ball_z: 0.0,
            ball_h: 0.0,
            stroke: 0,
            club: "",
            event: "",
            last_putt_dist: 0.0,
            looping: true,
            finished: false,
            route: None,
            phase: Phase::Pause,
            rng: 1,
            phase_time: 0.0,
            shot_from_x: 0.0,
            shot_from_z: 0.0,
            land_x: 0.0,
            land_z: 0.0,
            flight_sec: 1.0,
            flight_peak: 0.0,
            aim_heading: 0.0,
            struck: false,
            missed: false,
        }
    }
}

impl ShotSim {
    pub fn set_route(&mut self, r: Vec<f32>) {
        self.route = Some(r);
    }
    pub fn route(&self) -> &[f32] {
        self.route.as_deref().unwrap_or(&[])
    }

    fn rnd(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0xFF_FFFF) as f32 / 0x100_0000 as f32
    }
    fn hole_x(&self) -> f32 {
        let r = self.route();
        r[r.len() - 2]
    }
    fn hole_z(&self) -> f32 {
        let r = self.route();
        r[r.len() - 1]
    }
    pub fn dist_to_hole(&self) -> f32 {
        (self.hole_x() - self.ball_x).hypot(self.hole_z() - self.ball_z)
    }
    fn set_phase(&mut self, p: Phase) {
        self.phase = p;
        self.phase_time = 0.0;
    }

    pub fn init(&mut self, t: &Terrain, seed: u32) {
        if self.route.is_none() {
            self.route = Some(t.path.clone());
        }
        self.finished = false;
        self.rng = if seed != 0 { seed } else { 1 };
        self.stroke = 0;
        let r = self.route();
        let (bx, bz) = if r.len() >= 2 { (r[0], r[1]) } else { (0.0, 0.0) };
        self.ball_x = bx;
        self.ball_z = bz;
        self.ball_h = 0.0;
        self.golfer_x = bx;
        self.golfer_z = bz;
        self.event = "on the tee";
        self.set_phase(Phase::Walk);
        self.anim = GolferAnim::Walk;
    }

    /// Aim at the point on the course route about one drive further on than the ball.
    fn aim_at_green(&mut self) {
        let p = self.route();
        let n = p.len() / 2;
        let (mut best, mut bd) = (0usize, 1e30f32);
        for i in 0..n {
            let d = (p[2 * i] - self.ball_x).hypot(p[2 * i + 1] - self.ball_z);
            if d < bd {
                bd = d;
                best = i;
            }
        }
        let mut acc = 0.0f32;
        let mut i = best;
        while i + 1 < n && acc < DRIVE_DIST {
            acc += (p[2 * i + 2] - p[2 * i]).hypot(p[2 * i + 3] - p[2 * i + 1]);
            i += 1;
        }
        let (mut tx, mut tz) = (p[2 * i], p[2 * i + 1]);
        if self.dist_to_hole() < DRIVE_DIST {
            tx = self.hole_x();
            tz = self.hole_z();
        }
        self.aim_heading = (tz - self.ball_z).atan2(tx - self.ball_x) * 180.0 / PI;
    }

    /// Advances the simulation by dt seconds.
    pub fn step(&mut self, t: &Terrain, dt: f32) {
        if self.route().len() < 4 {
            return;
        }
        self.phase_time += dt;
        self.anim_time += dt;
        let rad = PI / 180.0;
        // The golfer stands beside the ball, on the left of the line of play.
        let stand_spot =
            |s: &ShotSim| (s.ball_x + ((s.aim_heading - 90.0) * rad).cos() * 16.0, s.ball_z + ((s.aim_heading - 90.0) * rad).sin() * 16.0);
        match self.phase {
            Phase::Pause => {
                if self.phase_time > 1.5 {
                    if self.looping {
                        let seed = self.rng;
                        self.init(t, seed);
                    } else {
                        self.finished = true;
                    }
                }
            }
            Phase::Walk => {
                self.aim_at_green();
                let (sx, sz) = stand_spot(self);
                let (dx, dz) = (sx - self.golfer_x, sz - self.golfer_z);
                let d = dx.hypot(dz);
                if self.anim != GolferAnim::Walk {
                    self.anim = GolferAnim::Walk;
                    self.anim_time = 0.0;
                }
                if d > 2.0 {
                    let s = d.min(WALK_SPEED * self.pace_scale * dt);
                    self.golfer_x += dx / d * s;
                    self.golfer_z += dz / d * s;
                    self.golfer_heading = dz.atan2(dx) / rad;
                } else {
                    self.golfer_x = sx;
                    self.golfer_z = sz;
                    let putt = self.dist_to_hole() < PUTT_RANGE;
                    self.anim = if putt { GolferAnim::PuttAddress } else { GolferAnim::Address };
                    self.anim_time = 0.0;
                    self.set_phase(if putt { Phase::PuttAddress } else { Phase::Address });
                }
            }
            Phase::Address | Phase::PuttAddress => {
                self.golfer_heading = self.aim_heading;
                if self.phase_time >= ADDRESS_SEC {
                    let putt = self.phase == Phase::PuttAddress;
                    self.anim = if putt { GolferAnim::Putt } else { GolferAnim::Swing };
                    self.anim_time = 0.0;
                    self.struck = false;
                    self.set_phase(if putt { Phase::Putt } else { Phase::Swing });
                }
            }
            Phase::Swing | Phase::Putt => {
                self.golfer_heading = self.aim_heading;
                let putt = self.phase == Phase::Putt;
                if !self.struck && self.phase_time >= if putt { PUTT_IMPACT_SEC } else { SWING_IMPACT_SEC } {
                    self.struck = true;
                    self.stroke += 1;
                    self.shot_from_x = self.ball_x;
                    self.shot_from_z = self.ball_z;
                    if putt {
                        self.strike_putt();
                    } else {
                        self.strike_full(t);
                    }
                }
            }
            Phase::Flight => {
                let u = (self.phase_time / self.flight_sec).min(1.0);
                self.ball_x = self.shot_from_x + (self.land_x - self.shot_from_x) * u;
                self.ball_z = self.shot_from_z + (self.land_z - self.shot_from_z) * u;
                self.ball_h = 4.0 * self.flight_peak * u * (1.0 - u);
                if self.anim_time > SWING_SEC {
                    self.anim_time = SWING_SEC - 0.001; // hold the follow through
                }
                if u >= 1.0 {
                    self.land(t);
                }
            }
            Phase::Settle => {
                if self.anim_time > SWING_SEC {
                    self.anim_time = SWING_SEC - 0.001;
                }
                if self.phase_time > 0.8 {
                    self.set_phase(Phase::Walk);
                }
            }
            Phase::Roll => {
                let u = (self.phase_time / self.flight_sec).min(1.0);
                self.ball_x = self.shot_from_x + (self.land_x - self.shot_from_x) * u;
                self.ball_z = self.shot_from_z + (self.land_z - self.shot_from_z) * u;
                if u >= 1.0 && self.missed {
                    self.event = "putt missed";
                    self.set_phase(Phase::Settle);
                } else if u >= 1.0 {
                    self.ball_h = -1.0;
                    self.event = "holed";
                    self.anim = GolferAnim::Happy;
                    self.anim_time = 0.0;
                    self.set_phase(Phase::Celebrate);
                }
            }
            Phase::Celebrate => {
                if self.phase_time > 3.0 {
                    self.set_phase(Phase::Pause);
                }
            }
        }
    }

    fn strike_putt(&mut self) {
        // PLACEHOLDER: chance to hole out grows with the putting skill, and a little with luck.
        let s = &self.skills.v;
        let chance = 0.40 + 0.55 * s[GolferSkills::ACC_PUTTER] as f32 / 15.0 + 0.05 * s[GolferSkills::LUCK] as f32 / 15.0;
        self.last_putt_dist = self.dist_to_hole();
        let far = self.dist_to_hole() > 60.0;
        let r = self.rnd();
        self.missed = if far { r > chance } else { r > (chance + 0.3).min(1.0) };
        self.land_x = self.hole_x();
        self.land_z = self.hole_z();
        if self.missed {
            let a = self.rnd() * 2.0 * PI;
            let off = 20.0 + 35.0 * self.rnd();
            self.land_x += a.cos() * off;
            self.land_z += a.sin() * off;
        }
        self.flight_sec = (self.dist_to_hole() / 160.0).max(0.6);
        self.flight_peak = 0.0;
        self.event = "putt";
        self.club = "putt";
        self.set_phase(Phase::Roll);
        self.anim = GolferAnim::Putt;
        self.anim_time = PUTT_IMPACT_SEC; // keep the putt animation running through the roll
    }

    fn strike_full(&mut self, t: &Terrain) {
        type S = GolferSkills;
        let approach = self.dist_to_hole() < DRIVE_DIST;
        // Range and flight follow the exe's rules (crate::flight). The skill levels here are 0..15 placeholders mapped onto the
        // exe's 0..9 digits; the base byte, the lie hazard and the range unit in yards are not decoded, so 3 and 0 are used.
        // The dispersion cone (accuracy, recovery from sand) is a PLACEHOLDER.
        let here0 = t.type_at_world(self.ball_x, self.ball_z);
        let on_tee = here0 == TT_TEE as i32;
        let sk = self.skills.v;
        let len_digit = sk[S::LONG_DRIVER] * 9 / 15;
        let acc_digit = sk[if approach { S::ACC_IRONS } else { S::ACC_DRIVER }] * 9 / 15;
        let max_r = flight::max_range(1, 3 + sk[S::POWER] * 6 / 15, len_digit, acc_digit, false, 0, on_tee);
        let units_per_range = TILE_SIZE_WORLD / flight::RANGE_UNITS_PER_TILE as f32;
        let carry = max_r as f32 * units_per_range * 0.8;
        let acc = sk[if approach { S::ACC_IRONS } else { S::ACC_DRIVER }] as f32 / 15.0;
        let mut spread = 24.0 - 20.0 * acc;
        let here = t.type_at_world(self.ball_x, self.ball_z);
        if is_sand(here) {
            spread *= 1.6 - 0.8 * sk[S::RECOVERY] as f32 / 15.0;
        }
        let dist = carry.min(self.dist_to_hole()) * (0.88 + 0.24 * self.rnd());
        let h = self.aim_heading + (self.rnd() - 0.5) * spread;
        let rad = PI / 180.0;
        self.land_x = self.ball_x + (h * rad).cos() * dist;
        self.land_z = self.ball_z + (h * rad).sin() * dist;
        let arc = flight::simulate(1.max((dist / (units_per_range * 0.8)).round() as i32));
        self.flight_sec = (arc.ticks as f32 / flight::TICKS_PER_SECOND).max(0.5);
        self.flight_peak = arc.peak / 1024.0 * TILE_SIZE_WORLD;
        self.club = if approach { "iron" } else { "drive" };
        self.event = "drive";
        self.set_phase(Phase::Flight);
        self.anim = GolferAnim::Swing;
        self.anim_time = SWING_IMPACT_SEC;
    }

    fn land(&mut self, t: &Terrain) {
        self.ball_h = 0.0;
        let mut ty = t.type_at_world(self.ball_x, self.ball_z);
        // The manual: firm fairway makes balls bounce higher and roll farther, and rocks deflect the ball at random.
        // The amounts are PLACEHOLDERS (a 12 percent run on, a 40 to 90 unit kick in a random direction).
        if ty == TT_FIRM_FAIRWAY as i32 {
            let (dx, dz) = (self.land_x - self.shot_from_x, self.land_z - self.shot_from_z);
            self.ball_x += dx * 0.12;
            self.ball_z += dz * 0.12;
            ty = t.type_at_world(self.ball_x, self.ball_z);
        } else if ty == TT_ROCK as i32 {
            let a = self.rnd() * 2.0 * PI;
            let off = 40.0 + 50.0 * self.rnd();
            self.ball_x += a.cos() * off;
            self.ball_z += a.sin() * off;
            ty = t.type_at_world(self.ball_x, self.ball_z);
        }
        if ty < 0 {
            self.event = "out of bounds, replay";
            self.stroke += 1;
            self.ball_x = self.shot_from_x;
            self.ball_z = self.shot_from_z;
        } else if is_water(ty) {
            self.event = "splash, replay with a penalty";
            self.stroke += 1;
            self.ball_x = self.shot_from_x;
            self.ball_z = self.shot_from_z;
        } else if is_sand(ty) {
            self.event = "in the sand";
        } else {
            self.event = "on the course";
        }
        self.set_phase(Phase::Settle);
    }
}
