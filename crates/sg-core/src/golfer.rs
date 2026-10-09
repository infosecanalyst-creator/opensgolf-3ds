//! The golfers: 152 records of the exe's golfer table, their arrival and pairing, their walk round the course, their thoughts
//! and mood, the swing, the ball, the end of each hole and of the round. Each part follows the exe's routine named in its
//! comment (golfer update 0x4289e0, mood events 0x467a00, hole end 0x427380, round end 0x4266b0, golfer creation 0x421bc0,
//! the main frame's arrival code, the golfer animation in the drawing routine). docs/PUBLISHER_EXE_NOTES.md describes them.
//!
//! The simulation is integer and tick based, like the exe's: call `Club::tick` once per game tick (87 ms).

use crate::course::{f, idx, inside, t, Course, NN};
use crate::geom::{adiff, angle, cdir, clamp, dir8, len, tdist, DX, DY, UNIT};
use crate::land::ExeRng;
use crate::roster::Person;

pub const SLOTS: usize = 152;
/// Hole records 0..=19; 1..18 are holes, 19 is the walk back to the clubhouse.
pub const HOLE_RECORDS: usize = 20;

pub mod flag {
    pub const PENALTY: u32 = 0x1;
    pub const HIT_TREE: u32 = 0x2;
    pub const VISITED_RANGE: u32 = 0x4;
    pub const VISITED_SHOP: u32 = 0x8;
    pub const VISITED_PUTTING: u32 = 0x10;
    pub const TO_RANGE: u32 = 0x20;
    pub const TO_PUTTING: u32 = 0x40;
    pub const BACKSPIN: u32 = 0x80;
    pub const TO_SHOP: u32 = 0x100;
    pub const GARY: u32 = 0x200;
    pub const TEED_OFF: u32 = 0x400;
    pub const ASIDE: u32 = 0x800;
    pub const IMPATIENT: u32 = 0x1000;
    pub const TO_SNACK: u32 = 0x2000;
    pub const HURRIED: u32 = 0x4000;
    pub const CART: u32 = 0x8000;
    pub const MAY_CART: u32 = 0x10000;
    pub const BAD_LIE: u32 = 0x20000;
    pub const BALL_MOVING: u32 = 0x40000;
    pub const SLOW_STEP: u32 = 0x80000;
    pub const STORY: u32 = 0x100000;
    pub const STORY_BEAT: u32 = 0x200000;
    pub const SWEET: u32 = 0x400000;
    pub const STORY_STARTED: u32 = 0x800000;
    pub const THIRST_LOW: u32 = 0x1000000;
    pub const TO_BENCH: u32 = 0x2000000;
    pub const WASHED: u32 = 0x4000000;
    pub const ANIM_ALT: u32 = 0x8000000;
    pub const CORNER_AIM: u32 = 0x10000000;
    pub const LEAVING: u32 = 0x20000000;
    pub const OFF_PATH: u32 = 0x40000000;
    pub const UPGRADE: u32 = 0x80000000;
}

/// Game state flags (0x59e7b8) the golfers read.
pub mod game {
    pub const DEBUG_TELEPORT: u32 = 0x10;
    pub const TWO_TEES: u32 = 0x40;
    pub const SLOW: u32 = 0x20000;
    /// The hole layout pass has planned the hole being built.
    pub const LAYOUT: u32 = 0x40000;
    pub const BRIDGE: u32 = 0x80000;
    pub const TOURNAMENT: u32 = 0x200000;
    pub const AIM_SEARCH: u32 = 0x800000;
    pub const NO_EVENTS: u32 = 0x2000000;
    pub const EDITOR: u32 = 0x4000000;
    pub const REPEAT: u32 = 0x4200000;
}

/// Animation states (+0x1d).
pub mod anim {
    pub const WALK0: i32 = 7;
    pub const STAND: i32 = 0xb;
    pub const HAPPY: i32 = 0xc;
    pub const UPSET: i32 = 0xd;
    pub const POINT: i32 = 0xe;
    pub const ADDRESS: i32 = 0x10;
    pub const SIT_DOWN: i32 = 0x11;
    pub const SITTING: i32 = 0x12;
    pub const WASH: i32 = 0x13;
}

/// One golfer record (0x100 bytes at 0x5794b8 in the exe); field comments give the record offset.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Golfer {
    /// +0x00, +0x04 position in map units
    pub x: i32,
    pub y: i32,
    /// +0x08, +0x0c screen position, -1 when not on screen (set by the drawing code)
    pub sx: i32,
    pub sy: i32,
    /// +0x10
    pub flags: u32,
    /// +0x14 walk left on the current heading
    pub walk_left: i32,
    /// +0x16 special skill mask (bit k: skill byte k is valid)
    pub skill_mask: u16,
    /// +0x18 kind: 0 ordinary, 0x20 pro, 0x40 commissioner, 0x60 CEO, 0x80 heiress (low 5 bits an id)
    pub kind: u8,
    /// +0x19 skill class bits (1 Length, 2 Accuracy, 4 Imagination), high nibble a wager stake
    pub class: u8,
    /// +0x1a facing 0..7
    pub facing: i32,
    /// +0x1b planned shot difficulty
    pub rating: i32,
    /// +0x1c club 0..13
    pub club: i32,
    /// +0x1d animation state, +0x1e frame counter, +0x1f clip drawn
    pub anim: i32,
    pub frame: u8,
    pub clip: u8,
    /// +0x20 shot state
    pub sub: i32,
    /// +0x21 round state: 0 free, -1 waiting, 1..18 hole, 19 going home
    pub hole: i32,
    /// +0x22 strokes on this hole
    pub strokes: i32,
    /// +0x23 + h strokes taken on hole h
    pub card: [i8; 19],
    /// +0x36 momentum of pro golfers
    pub momentum: i32,
    /// +0x37 + h smoothed mood per hole (story pairs)
    pub hole_mood: [u8; 19],
    /// +0x4a + h story step per hole
    pub story_hole: [u8; 19],
    /// +0x5c + h mood at the end of each hole
    pub end_mood: [u8; 19],
    /// +0x70 thought history (newest first) and +0x7a their stamps
    pub thoughts: [u8; 10],
    pub stamps: [u8; 10],
    /// +0x84 thought timer, +0x85 thought id, +0x86 its stamp
    pub timer: u8,
    pub thought: u8,
    pub thought_stamp: u8,
    /// +0x88 argument history; bit 0x4000 positive change, 0xc000 negative
    pub args: [u16; 10],
    /// +0x9c argument of the thought shown
    pub thought_arg: i32,
    /// +0x9e pause: frozen while negative
    pub pause: i32,
    /// +0xa0 clubs used (bit per club)
    pub clubs_used: u16,
    /// +0xa2 partner
    pub partner: i32,
    /// +0xa4 mood -10..10, +0xa6 hunger, +0xa8 thirst, +0xaa fatigue
    pub mood: i32,
    pub hunger: i32,
    pub thirst: i32,
    pub fatigue: i32,
    /// +0xac shot type: 0, 1 draw, -1 fade, 3 high, 4 low
    pub shot_type: i32,
    pub field_ae: i32,
    /// +0xb0 story, +0xb2 its step, +0xb4 smoothed mood
    pub story: i32,
    pub story_step: i32,
    pub smooth_mood: i32,
    /// +0xb6 roster index, +0xb8 appearance, +0xba tee set / level, +0xbb famous golfer index
    pub roster: i32,
    pub looks: u16,
    pub level: i32,
    pub famous: i32,
    pub field_bc: u8,
    /// +0xbe tee order
    pub tee_order: i32,
    /// +0xc0 tick the hole started
    pub hole_tick: u32,
    /// +0xc4/+0xc8 shot origin
    pub ox: i32,
    pub oy: i32,
    /// +0xcc/+0xd0 aim tile (-1 waiting for the player)
    pub aim_a: i32,
    pub aim_b: i32,
    /// +0xd4/+0xd8 ball (bx == 0: no ball), +0xdc height, +0xe0 heading, +0xe4 speed, +0xe8 vertical speed, +0xec curve
    pub bx: i32,
    pub by: i32,
    pub bz: i32,
    pub heading: u32,
    pub speed: i32,
    pub vz: i32,
    pub curve: i32,
    /// +0xf0 special skill levels
    pub skills: [u8; 12],
}

impl Golfer {
    pub fn vip(&self) -> u8 {
        self.kind & 0xe0
    }
}

/// One hole record (0x208 bytes at 0x575ab0 + h*0x208).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Hole {
    /// +0x00 par (0: no hole), +0x01 tee facing (toward the planned tee shot), +0x02 heading from the forward tee to the pin
    pub par: i32,
    pub tee_facing: i32,
    pub fwd_facing: i32,
    /// +0x04 length in range units (25 per tile)
    pub length: i32,
    /// +0x08 back tee, +0x10 forward tee, +0x18 pin, all tiles
    pub back: (i32, i32),
    pub fwd: (i32, i32),
    pub pin: (i32, i32),
    /// +0x20 tee shots, +0x24 shots planned
    pub tee_shots: i32,
    pub plans: i32,
    /// +0x28 strokes histogram per skill class (11 per class)
    pub hist: Vec<i32>,
    /// +0xd8 + 2*event: events raised here, and +0x16c their last argument
    pub events: Vec<i32>,
    pub event_args: Vec<i32>,
    /// +0x158 mood changes, +0x15a golfers who quit here
    pub mood_sum: i32,
    pub quits: i32,
    /// +0x15c drive total, +0x15e fairways hit, +0x160 greens in regulation, +0x162 putts, +0x166 longest drive
    pub drive_sum: i32,
    pub fairways: i32,
    pub gir: i32,
    pub putts: i32,
    pub drive_max: i32,
    /// +0x1ec play time (ticks / 2), +0x1f4 green fees, +0x1fc likeness to the previous hole
    pub time: i32,
    pub fees: i32,
    pub monotony: i32,
    /// +0x1f0 money spent building it, +0x1f8 maintenance charged (units)
    pub build_cost: i32,
    pub maint: i32,
    /// +0x200: 1 Top 100 hole, 2 Top 18 hole, 4 / 8 hard / easy marks, 0x20 / 0x40 dogleg one way or the other,
    /// 0x1000 uphill, 0x2000 downhill
    pub flags: u32,
    /// The three yardage markers in map units ((-1, -1) none; 0x59ae80 + h*24)
    pub markers: [(i32, i32); 3],
}

/// Membership record per roster person (0x2c bytes at 0x5849e0).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Member {
    pub best: u8,
    pub avg: i8,
    /// bits 0..2 level: 0 not invited, 1 visitor, 2 member, 3 silver, 4 gold, 5 platinum
    pub level: u8,
    /// +3 + h: 1 story scene, 2 happy ending, 4 quit on hole h
    pub holes: [u8; 19],
    /// +0x16 + h: strokes on hole h in the last round
    pub card: [u8; 19],
    /// +0x29: 0xff gone for good
    pub gone: u8,
    pub rounds: i32,
}

/// Ledger columns money is booked to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Column {
    GreensFees,
    FoodDrink,
    Other,
}

/// What the golfers did this tick that the game around them must show or book.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Event {
    /// A sound slot; with a map point it is positional.
    Sound { slot: i32, at: Option<(i32, i32)> },
    /// Money earned (units of $100), shown as a floating number at the point.
    Earn { units: i32, column: Column, at: (i32, i32) },
    /// A thought shown above a golfer (event id and argument).
    Thought { g: usize, id: u32, arg: i32 },
    /// A message for the ticker.
    Message(String),
    /// A golfer finished a hole (employees serving them note it): the hole, strokes taken, mood and fee paid.
    HoleDone { g: usize, hole: i32, strokes: i32, mood: i32, fee: i32 },
}

/// The golfers and everything they share (the exe's globals near them).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Club {
    pub g: Vec<Golfer>,
    pub holes: Vec<Hole>,
    /// Number of the next hole to build (0x5685f0): holes open = next_hole - 1.
    pub next_hole: i32,
    pub members: Vec<Member>,
    pub roster: Vec<Person>,
    /// Tee order counter (0x56a51c), creation counter (0x59ae7c), last golfer made (0x5a5a24).
    pub tee_counter: i32,
    pub create_counter: i32,
    pub last_created: i32,
    /// The player's golfer while playing, or -1 (0x5a59f8).
    pub gary: i32,
    /// Game state flags (0x59e7b8).
    pub game: u32,
    /// An aim search ran this tick (0x59b738).
    pub plan_busy: bool,
    /// Tick each golfer's ball started moving (0x56962c).
    pub shot_start: Vec<u32>,
    pub tick: u32,
    pub difficulty: i32,
    /// Doubles green fees and changes repeated thoughts when 2 (0x543cf4; never set by the shipped game).
    pub fee_mode: i32,
    /// Fast mode (0x59b04c): walking doubles and golfers do not dawdle.
    pub turbo: bool,
    /// Kind balancing table (0x59dea0).
    pub kind_counts: [i32; 32],
    /// Frames per view of each golfer clip id + body (0x53f3e8), filled from the sprite files.
    pub clip_frames: Vec<i32>,
    /// Best rounds (0x56a524).
    pub top_rounds: [i32; 10],
    /// Land purchases so far (0x53a450).
    pub purchases: i32,
    /// Hole types already announced (0x5685f8), the club ratings of the last statistics pass, and the home sites standing.
    pub types_announced: u32,
    pub ratings: crate::ratings::ClubRatings,
    pub home_sites: i32,
    /// The theme's stories, Happy Endings so far (0x561258) and the tick of the last story beat (0x59b048).
    pub stories: crate::stories::Stories,
    pub happy_endings: i32,
    pub last_beat: u32,
    /// Landmark types available (0x543cfc) and free to place once (0x822c70).
    pub landmarks_owned: u32,
    pub free_landmarks: u32,
    /// CEO visits started, bit 0x80 while one is on the course (0x572cac); heiress donations (0x59aaf8); a land offer pending
    /// (0x567a1c bit 0); celebrity homes bought (0x4c284c).
    pub ceo_count: i32,
    pub donations: i32,
    pub land_offer: bool,
    pub celeb_homes: i32,
    /// The property is an island (no commissioner comes) and a sandbox game (no visitors come).
    pub island: bool,
    pub sandbox: bool,
    /// celebrities.dta and progolfers.dta of the theme.
    pub celebrities: Vec<crate::vips::Celebrity>,
    pub pros: Vec<crate::vips::Pro>,
    /// The pro's skill bytes (0x5a5a04) and which he has (0x4c2c9c), points left to hand out, the wager level L (0x59b730),
    /// the famous golfer who challenged the club (0x4c2e14) and matches won.
    pub pro_skill: [u8; 16],
    pub pro_mask: u16,
    pub skill_points: i32,
    pub wager_level: i32,
    pub challenge_pro: i32,
    pub trophies: i32,
    /// SGA tournaments: the last evaluation score (0x561250), the purse in thousands (0x567b04), the preparation state and
    /// options (0x5a47e0: -1 just started, -2 checklist pending, else the ticked mask) and the records the field borrows.
    pub sga_score: i32,
    pub purse: i32,
    pub tourney_opts: i32,
    pub backup: Option<crate::tournament::Backup>,
    /// Animals, fly-overs and the ball splash.
    pub wildlife: crate::wildlife::Wildlife,
    /// Celebrity residents of the vacation homes (0x56d1b8).
    pub residents: Vec<crate::celebs::Actor>,
    /// Accomplishments earned (bit per id, 0x4c15a0), each with its tick and course; the one waiting for the board, its
    /// map point and the frames since; the monthly event log; the monthly history (cash, fun, skill, members); the course
    /// name for the records.
    pub awards: u32,
    pub earned: Vec<Option<crate::records::Earned>>,
    pub award_pending: i32,
    pub award_point: (i32, i32),
    pub award_frames: i32,
    pub event_log: Vec<u16>,
    pub history: Vec<[i32; 4]>,
    pub course_name: String,
    /// The course's exe theme (0 Parkland, 1 Desert, 2 Tropical, 3 Links).
    pub course_theme: u8,
    /// Aiming: the hovered tile, whether the pointer was nearer its corner, and the frames a click is still refused.
    pub aim_tile: (i32, i32),
    pub aim_corner: bool,
    pub aim_lock: i32,
    /// Year number and the year's first fee flag, for the green fee tutorial.
    pub year: i32,
    pub fees_this_year: i32,
    /// Cash in units, for the membership rule that helps a poor club.
    pub cash: i32,
    /// Members minus home sites (0x56d1b0).
    pub homesite_demand: i32,
    pub planner: crate::planner::State,
    pub out: Vec<Event>,
    /// Landing tiles of the hole layout pass (0x542dfc/0x542e24), replayed while game flag LAYOUT is set.
    pub layout_land: Vec<(i32, i32)>,
    /// Post the first employee gets when hole 1 opens (0x585860).
    pub first_post: Option<(i32, i32)>,
    /// What the drawing code shows for each golfer this tick: sprite id (clip + body) and frame.
    pub drawn: Vec<(i32, i32)>,
}

impl Default for Club {
    fn default() -> Self {
        Club::new(crate::roster::fallback())
    }
}

/// Default clip lengths when the sprite files are missing (frames per view).
const DEFAULT_CLIP: i32 = 8;

impl Club {
    pub fn new(roster: Vec<Person>) -> Club {
        Club {
            g: vec![Golfer::default(); crate::holetool::TRIAL + 1],
            holes: (0..HOLE_RECORDS)
                .map(|_| Hole {
                    hist: vec![0; 11 * 16],
                    events: vec![0; 0x50],
                    event_args: vec![0; 0x50],
                    markers: [(-1, -1); 3],
                    ..Default::default()
                })
                .collect(),
            next_hole: 1,
            members: vec![Member::default(); crate::roster::PEOPLE],
            roster,
            tee_counter: 0,
            create_counter: 0,
            last_created: 0,
            gary: -1,
            game: 0,
            plan_busy: false,
            shot_start: vec![0; SLOTS],
            tick: 0,
            difficulty: 1,
            fee_mode: 0,
            turbo: false,
            kind_counts: [0; 32],
            clip_frames: vec![DEFAULT_CLIP; 0x100],
            top_rounds: [0; 10],
            purchases: 0,
            types_announced: 0,
            ratings: Default::default(),
            home_sites: 0,
            stories: Default::default(),
            happy_endings: 0,
            last_beat: 0,
            landmarks_owned: 0,
            free_landmarks: 0,
            ceo_count: 0,
            donations: 0,
            land_offer: false,
            celeb_homes: 0,
            island: false,
            sandbox: false,
            celebrities: Vec::new(),
            pro_skill: [0; 16],
            pro_mask: 0,
            skill_points: 0,
            wager_level: 0,
            challenge_pro: -1,
            trophies: 0,
            awards: 0,
            earned: vec![None; 22],
            award_pending: -1,
            award_point: (-1, -1),
            award_frames: 0,
            event_log: vec![0; 500],
            history: vec![[0; 4]; 500],
            course_name: String::new(),
            course_theme: 0,
            residents: Vec::new(),
            wildlife: Default::default(),
            sga_score: 0,
            purse: 0,
            tourney_opts: 0,
            backup: None,
            aim_tile: (0, 0),
            aim_corner: false,
            aim_lock: 0,
            pros: Vec::new(),
            year: 0,
            fees_this_year: 0,
            cash: 0,
            homesite_demand: 0,
            planner: crate::planner::State { class_bits: 7, ..Default::default() },
            out: Vec::new(),
            layout_land: Vec::new(),
            first_post: None,
            drawn: vec![(0, 0); SLOTS],
        }
    }

    fn person(&self, g: usize) -> Person {
        self.roster.get(self.g[g].roster.max(0) as usize).cloned().unwrap_or_default()
    }

    /// 1 for a man, 0 for a woman.
    pub fn male(&self, g: usize) -> i32 {
        self.person(g).male_bit()
    }

    pub fn name(&self, g: usize) -> String {
        self.person(g).name
    }

    fn partner(&self, g: usize) -> usize {
        (self.g[g].partner.clamp(0, SLOTS as i32 - 1)) as usize
    }

    pub(crate) fn member_mut(&mut self, g: usize) -> &mut Member {
        let r = self.g[g].roster.clamp(0, self.members.len() as i32 - 1) as usize;
        &mut self.members[r]
    }

    pub(crate) fn member(&self, g: usize) -> &Member {
        let r = self.g[g].roster.clamp(0, self.members.len() as i32 - 1) as usize;
        &self.members[r]
    }

    fn hole_rec(&mut self, h: i32) -> Option<&mut Hole> {
        if (0..HOLE_RECORDS as i32).contains(&h) {
            Some(&mut self.holes[h as usize])
        } else {
            None
        }
    }

    pub fn par(&self, h: i32) -> i32 {
        if (0..HOLE_RECORDS as i32).contains(&h) {
            self.holes[h as usize].par
        } else {
            0
        }
    }

    pub(crate) fn sound(&mut self, slot: i32, at: Option<(i32, i32)>) {
        self.out.push(Event::Sound { slot, at });
    }

    pub(crate) fn earn(&mut self, units: i32, column: Column, at: (i32, i32)) {
        self.cash += units;
        self.out.push(Event::Earn { units, column, at });
    }

    pub(crate) fn message(&mut self, s: String) {
        self.out.push(Event::Message(s));
    }

    fn his_her(&self, g: usize) -> &'static str {
        if self.male(g) == 1 {
            "his"
        } else {
            "her"
        }
    }

    /// Body / voice index 0..8 of a golfer (men 0..3, women 5..8), as the drawing code chooses it (and remembers it in the
    /// roster's +0x23 nibble, which the voice then uses).
    pub fn look(&self, g: usize) -> i32 {
        let gg = &self.g[g];
        let p = self.person(g);
        let mut look = if p.female() {
            let mut l = 7;
            if p.b21 & 1 != 0 {
                l = 6;
            }
            if p.traits & 8 != 0 {
                l = 8;
            }
            if p.b21 & 4 != 0 {
                l = 5;
            }
            if p.fixed != 0 {
                l = ((p.b23 >> 4) & 3) as i32 + 5;
            }
            l
        } else {
            let mut l = if gg.class & 4 == 0 { 1 } else { ((!gg.class & 2) | 4) as i32 >> 1 };
            if p.fixed != 0 {
                l = ((p.b23 >> 4) & 3) as i32;
            }
            l
        };
        if gg.kind == 0x20 {
            // Tournament pros draw from the famous golfer table, not decoded; keep the class body.
            look = look.clamp(0, 8);
        }
        look
    }

    /// Voice index for emotion sounds: the remembered body nibble, plus 5 for women (0x467a00).
    fn voice(&self, g: usize) -> i32 {
        let p = self.person(g);
        let mut v = ((p.b23 >> 4) & 0xf) as i32;
        if p.male_bit() == 0 {
            v += 5;
        }
        v
    }

    fn remember_look(&mut self, g: usize) {
        let l = self.look(g);
        let r = self.g[g].roster.clamp(0, self.roster.len() as i32 - 1) as usize;
        let within = if self.roster[r].female() { l - 5 } else { l };
        self.roster[r].b23 = (self.roster[r].b23 & 0xf) | ((within as u8) << 4);
    }

    /// 2 when the golfer's last change was negative, 0 when positive, else 1 (0x4675d0).
    pub fn attitude(&self, g: usize) -> i32 {
        let a = self.g[g].args[0];
        if a & 0x8000 != 0 {
            2
        } else {
            ((!a >> 14) & 1) as i32
        }
    }

    // ---- mood events (0x467a00) --------------------------------------------------------------------------------------------

    /// Raises mood event `id` for golfer g: the thought is shown, its history kept, the mood changed by the event's amount (with
    /// the exe's damping), a partner may catch the bad mood, and a weed may start where the golfer stands.
    pub fn event(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, id: u32, arg: i32) {
        if g >= SLOTS || self.g[g].strokes > 9 || self.game & game::NO_EVENTS != 0 {
            return;
        }
        let mut id = id;
        let h = self.g[g].hole;
        if id == 0x13 && self.g[g].vip() != 0x20 && (1..HOLE_RECORDS as i32).contains(&h) && self.holes[h as usize].tee_shots > 9 {
            let hr = &self.holes[h as usize];
            let over = self.g[g].card[h as usize] as i32 - hr.par;
            if (hr.flags & 4 != 0 && over > 1) || (hr.flags & 8 != 0 && over < 0) {
                id = 0x17;
            }
        }
        {
            let gg = &mut self.g[g];
            let stamp = (gg.hole * 11 + gg.strokes) as u8;
            gg.timer = 7;
            gg.thought = id as u8;
            gg.thought_stamp = stamp;
            gg.thought_arg = arg;
        }
        self.out.push(Event::Thought { g, id, arg });
        let (x, y) = (self.g[g].x, self.g[g].y);
        let male = self.male(g);
        if id == 0x23 {
            self.sound(male + 0x46, Some((x, y)));
            if self.g[g].thoughts[0] == 0x23 {
                return;
            }
        } else if id == 0x13 {
            return;
        }
        let saved = self.g[g].clone();
        {
            let gg = &mut self.g[g];
            for i in (1..10).rev() {
                gg.thoughts[i] = gg.thoughts[i - 1];
                gg.stamps[i] = gg.stamps[i - 1];
                gg.args[i] = gg.args[i - 1];
            }
            gg.thoughts[0] = id as u8;
            gg.stamps[0] = (gg.hole * 11 + gg.strokes) as u8 + (id > 3) as u8;
            gg.args[0] = arg as u16;
        }
        let voice = self.voice(g);
        let bad_lie = self.g[g].flags & flag::BAD_LIE != 0;
        let diff = self.difficulty;
        let d: i32 = match id {
            1 => {
                self.sound(if bad_lie { 0x14 } else { 0 } + 0xd2 + voice, Some((x, y)));
                1
            }
            2 | 3 | 8 => {
                self.sound(if bad_lie { 0x10 } else { 0xdc } + voice, Some((x, y)));
                match id {
                    2 => -1,
                    _ => -2,
                }
            }
            4 => {
                self.sound(male + 0x12, Some((x, y)));
                -2
            }
            5 | 0x1f => {
                self.sound(male + 0x10, Some((x, y)));
                0
            }
            6 => {
                self.sound(male + 0x14, Some((x, y)));
                1
            }
            7 => {
                if arg == 0 {
                    self.sound(male + 0x4c, Some((x, y)));
                    1
                } else {
                    0
                }
            }
            9 => {
                self.sound(male + 0x46, Some((x, y)));
                -3
            }
            10 | 0x2b => {
                self.sound(male + 0x50, Some((x, y)));
                -2
            }
            0xb => {
                self.sound(male + 0x4a, Some((x, y)));
                1
            }
            0xc | 0xd => {
                self.sound(male + 8, Some((x, y)));
                -1
            }
            0xe => {
                self.sound(male + 0x9e, Some((x, y)));
                -1
            }
            0xf => {
                self.sound(male + 0x9c, Some((x, y)));
                -1
            }
            0x12 => (self.g[g].hunger > 7) as i32,
            0x14 | 0x16 | 0x1c => {
                let off = match id {
                    0x14 => 0x56,
                    0x16 => 0x4e,
                    _ => 0x4c,
                };
                self.sound(male + off, Some((x, y)));
                self.g[g].anim = anim::POINT;
                self.g[g].pause = -24;
                if id == 0x14 {
                    -2
                } else {
                    1
                }
            }
            0x15 => {
                self.sound(male + 0x44, Some((x, y)));
                -2
            }
            0x17 => {
                self.sound(male + 0x9a, Some((x, y)));
                -1 - (diff != 0) as i32
            }
            0x18 => {
                self.sound(male + 0x42, Some((x, y)));
                -2
            }
            0x19 => {
                self.sound(male + 0x3c, Some((x, y)));
                (self.g[g].thirst > 7) as i32
            }
            0x1a => {
                self.sound(male + 0xa0, Some((x, y)));
                -1
            }
            0x1b => {
                self.sound(male + 0x48, Some((x, y)));
                (self.g[g].fatigue > 0x3b) as i32
            }
            0x1d => {
                self.sound(male + 0x96, Some((x, y)));
                1
            }
            0x1e => {
                self.sound(male + 0x98, Some((x, y)));
                -2
            }
            0x20 | 0x21 => {
                self.sound(male + 0xa2, Some((x, y)));
                1
            }
            0x22 => {
                self.sound(if male != 0 { 0xa7 } else { 0xa4 }, Some((x, y)));
                1
            }
            0x23 => -2,
            0x24 => {
                self.sound(male + 0x3e, Some((x, y)));
                -3
            }
            0x27 => {
                self.sound(male + 0x14, Some((x, y)));
                (diff < 2) as i32
            }
            0x2c | 0x2e => 1,
            0x2f => -2,
            0x33..=0x35 => (diff == 0) as i32,
            0x36 => (diff < 2) as i32,
            0x3a => {
                self.sound(if male != 0 { 0xa5 } else { 0x5a }, Some((x, y)));
                0
            }
            0x3b => {
                let h = self.g[g].hole.clamp(0, 18) as usize;
                (self.member(g).holes[h] != 0) as i32
            }
            0x41 => {
                self.sound(male + 0x16, Some((x, y)));
                if diff > 1 {
                    -2
                } else {
                    0
                }
            }
            _ => 0,
        };
        if id == 0x28 {
            // The special swing only shows its thought: the record goes back as it was.
            self.g[g] = saved;
        }
        if d == 0 && id > 0x2f {
            return;
        }
        let orig = d;
        let mut d = d;
        if self.g[g].kind != 0 {
            let gg = &mut self.g[g];
            if -d.signum() == gg.momentum.signum() {
                gg.momentum = 0;
            } else {
                gg.momentum += d;
            }
            if matches!(id, 2 | 3 | 0xc | 0xd) && gg.momentum < 0 && d < 0 && gg.momentum < d {
                gg.momentum = 1;
            }
            if g as i32 == self.gary && d < 0 && self.g[g].flags & flag::BALL_MOVING != 0 {
                self.sound(0x30, None);
            }
        }
        let mut negative = true;
        if d == -1 {
            if self.fee_mode != 2 && self.g[g].kind == 0 {
                // A small upset only counts when it repeats a recent one, or follows a bad one.
                d = 0;
                let n = if diff == 0 {
                    3
                } else if diff < 3 {
                    5
                } else {
                    10
                };
                for i in 1..n {
                    if self.g[g].thoughts[i] as u32 == id {
                        d = -1;
                    }
                }
                if self.g[g].args[1] & 0x8000 != 0 {
                    d = -1;
                }
                if self.fee_mode == 1 {
                    d = 0;
                    negative = false;
                } else if d >= 0 {
                    negative = false;
                }
            }
        } else if d >= 0 {
            negative = false;
        }
        if negative {
            if self.g[g].vip() != 0x40 {
                d = (d - 1) / 2;
            }
            if d < 0
                && self.g[g].vip() != 0x20
                && ((self.g[g].strokes + g as i32) & 1) + 2 <= diff
                && id != 0x2f
                && id != 0x15
                && id != 0x18
            {
                let p = self.partner(g);
                if self.g[p].hole > 0 && self.g[p].timer == 0 && self.g[p].mood > 0 {
                    self.event(c, rng, p, 0x2f, 0x14);
                    self.g[p].timer = self.g[p].timer.wrapping_add(2);
                }
            }
        }
        self.g[g].mood = clamp(self.g[g].mood + d, -10, 10);
        let h = self.g[g].hole;
        if let Some(hr) = self.hole_rec(h) {
            hr.mood_sum += d;
        }
        let r = rng.below(6);
        if (r <= diff || self.g[g].flags & flag::LEAVING != 0)
            && d < 0
            && self.game & game::EDITOR == 0
            && id != 0x2f
            && c.landmark_bits(self.g[g].x, self.g[g].y) & 2 == 0
        {
            let (a, b) = (self.g[g].x >> 10, self.g[g].y >> 10);
            if inside(a, b) {
                let i = idx(a, b);
                if c.flags[i] & 0xc00 == 0 && c.ty[i] != t::WATER {
                    c.growth[i] = 1;
                    c.flags[i] |= f::WEED | f::GROWING;
                }
            }
        }
        if d != 0 {
            let h = self.g[g].hole;
            if let Some(hr) = self.hole_rec(h) {
                if (id as usize) < hr.events.len() {
                    hr.events[id as usize] += 1;
                    hr.event_args[id as usize] = arg;
                }
            }
            if d > 0 {
                self.g[g].args[0] |= 0x4000;
            }
        }
        if orig < 0 {
            self.g[g].args[0] |= 0x8000;
        }
        if d < 0 {
            self.g[g].args[0] |= 0xc000;
        }
        let (a, b) = (self.g[g].x >> 10, self.g[g].y >> 10);
        if inside(a, b) {
            let i = idx(a, b);
            if d > 0 {
                c.happy[i] = c.happy[i].wrapping_add(1);
            }
            if d < 0 {
                c.unhappy[i] = c.unhappy[i].wrapping_add(1);
            }
        }
    }

    // ---- arrivals (main frame) and creation (0x421bc0) ----------------------------------------------------------------------

    /// New game: every membership cleared, then twelve people invited: three each of skill classes 5, 3, 6 and 7 (0x4315e0).
    pub fn new_game(&mut self, rng: &mut ExeRng) {
        for m in self.members.iter_mut() {
            *m = Member::default();
        }
        for g in self.g.iter_mut().take(SLOTS) {
            *g = Golfer::default();
        }
        self.gary = -1;
        self.tee_counter = 0;
        self.create_counter = 0;
        self.last_created = 0;
        let mut count = 0;
        for kind in [5u8, 3, 6, 7] {
            let mut got = 0;
            let mut tries = 0;
            while got < 3 && tries < 100_000 {
                tries += 1;
                let r = rng.below(76) as usize;
                if r % 3 != count % 3 || r == 0 || r >= self.roster.len() {
                    continue;
                }
                if self.roster[r].kind(r) != kind || self.members[r].level != 0 {
                    continue;
                }
                self.members[r].level = 1;
                got += 1;
                count += 1;
            }
        }
    }

    /// Invites one more person (level 0 -> 1), tried up to 1000 times.
    fn invite_friend(&mut self, rng: &mut ExeRng) {
        for _ in 0..1000 {
            let r = rng.below(75) as usize + 1;
            if r < self.members.len() && self.members[r].level & 7 == 0 {
                self.members[r].level = (self.members[r].level & !7) | 1;
                return;
            }
        }
    }

    /// Makes a golfer waiting at the clubhouse from a roster person, or returns -1 when nobody can come (0x421bc0).
    pub fn create(&mut self, c: &Course, rng: &mut ExeRng, member_only: bool) -> i32 {
        let s = (self.create_counter.rem_euclid(SLOTS as i32)) as usize;
        self.create_counter += 1;
        self.g[s] = Golfer::default();
        let (da, db) = c.door;
        self.g[s].x = da * UNIT + 0x600;
        self.g[s].y = db * UNIT + 0x600;
        self.g[s].facing = self.holes[1].tee_facing;
        self.g[s].looks = rng.below(0x7fff) as u16;
        // Vestigial balancing of skill classes: only its random draws and counts remain in effect.
        let kinds: &[usize] = &[3, 5, 6, 7];
        let row = (s & 3) * 8;
        let least = kinds.iter().map(|&k| self.kind_counts[k + row]).min().unwrap_or(0);
        let ties: Vec<usize> = kinds.iter().copied().filter(|&k| self.kind_counts[k + row] == least).collect();
        let pick = ties[rng.below(ties.len() as i32) as usize % ties.len()];
        self.kind_counts[pick] += 1;
        for _ in 0..999 {
            let r = rng.below(75) + 1;
            self.g[s].roster = r;
            let kind = self.roster.get(r as usize).map(|p| p.kind(s)).unwrap_or(7);
            self.g[s].class = kind;
            let mut ok = true;
            for o in 0..SLOTS {
                if o == s {
                    continue;
                }
                let og = &self.g[o];
                if og.hole == -1 && (og.roster % 19 == r % 19 || og.roster == r) {
                    ok = false;
                }
                if og.hole > 0 && og.roster == r {
                    ok = false;
                }
            }
            let m = &self.members[r as usize];
            if m.gone == 0xff {
                ok = false;
            }
            if member_only && m.level & 7 == 0 {
                ok = false;
            }
            if ok {
                let b21 = self.roster[r as usize].b21 as i8 as i32;
                self.g[s].looks = (rng.below(128) * 256 + (b21 & 0xff)) as u16;
                self.g[s].level = r % 3;
                self.g[s].hole = -1;
                self.g[s].anim = anim::STAND;
                self.g[s].hole_tick = self.tick;
                let mut mood = rng.below(3) + 3;
                if self.difficulty == 0 {
                    mood = 4;
                }
                if c.oper[9] != 0 {
                    mood = c.oper[9] + 4;
                }
                self.g[s].mood = mood;
                if c.oper[11] != 0 {
                    self.g[s].flags |= flag::MAY_CART;
                }
                self.g[s].sx = -1;
                self.g[s].sy = -1;
                self.remember_look(s);
                return s as i32;
            }
        }
        // The exe writes "Your membership is declining." to its message buffer here but never shows it.
        self.create_counter = (self.create_counter - 1).rem_euclid(SLOTS as i32);
        self.g[s] = Golfer::default();
        -1
    }

    /// Sends a waiting pair to the first tee (0x45de80).
    pub(crate) fn start_pair(&mut self, c: &mut Course, rng: &mut ExeRng, u: usize) {
        let v = u ^ 1;
        for (a, b) in [(u, v), (v, u)] {
            let gg = &mut self.g[a];
            gg.hole = 1;
            gg.partner = b as i32;
            gg.hole_tick = self.tick;
            if c.oper[11] != 0 {
                gg.flags |= flag::MAY_CART;
            }
        }
        self.g[u].tee_order = self.tee_counter;
        self.g[v].tee_order = self.tee_counter + 1;
        self.tee_counter += 2;
        self.g[u].pause = -6 - rng.below(6);
        self.g[v].pause = 0;
        self.pair_setup(rng, u, v);
        if self.tick & 0x40 != 0 {
            for gg in [u, v] {
                let m = self.g[gg].mood;
                self.event(c, rng, gg, 0x3d, m);
            }
        }
    }

    /// The pairing routine (0x45de80) for owner-to-be p1 and p2: a story, a possible visitor in p2's place, both moods from
    /// the pair's compatibility, the story kept only by chance once many have ended happily, the pair relationship.
    pub(crate) fn pair_setup(&mut self, rng: &mut ExeRng, p1: usize, p2: usize) {
        let compat = self.compatibility(p1, p2);
        self.pick_story(rng, p1, p2);
        if self.g[p2].flags & crate::stories::MANUAL_PAIR == 0 {
            let sandbox = self.sandbox;
            self.vip_check(rng, p2, sandbox);
        }
        for gg in [p1, p2] {
            self.g[gg].mood = compat + 2;
            self.g[gg].smooth_mood = 2 * compat;
        }
        let owner = if self.g[p1].flags & flag::STORY != 0 { p1 } else { p2 };
        self.g[owner].hole_mood[1] = (2 * compat) as u8;
        let r = rng.below(self.happy_endings);
        if (r > 6 && self.g[p1].flags & crate::stories::MANUAL_PAIR == 0) || self.g[p2].kind != 0 {
            for gg in [p1, p2] {
                self.g[gg].story = -1;
                self.g[gg].field_ae = 0;
            }
        }
        self.vip_skills(rng, p2);
        let female = |c: &Club, g: usize| c.male(g) == 0;
        let mut rel = 0;
        if compat < 2 && female(self, p1) == female(self, p2) {
            rel = 2;
        }
        if compat == 5 {
            rel = 3;
        }
        if compat >= 4 && female(self, p1) != female(self, p2) && self.g[p1].looks & 0x10 != 0 && self.g[p2].looks & 0x10 != 0 {
            rel = 1;
        }
        if rel != 0 {
            self.g[p1].field_ae = rel;
            self.g[p2].field_ae = rel;
        }
        for gg in [p1, p2] {
            self.g[gg].flags &= !crate::stories::MANUAL_PAIR;
        }
    }

    /// The main frame's arrivals: a waiting pair goes to the first tee when it is free, and a new golfer arrives whenever fewer
    /// than two are queued, or every 128 ticks, up to eight queued.
    pub fn arrivals(&mut self, c: &mut Course, rng: &mut ExeRng) {
        let mut queue = 0;
        let mut waiting_pair: i32 = -1;
        let mut on_hole1 = 0;
        let mut can_tee = self.holes[1].par != 0;
        for k in 0..SLOTS as i32 {
            let s = (self.last_created + SLOTS as i32 - k).rem_euclid(SLOTS as i32) as usize;
            if self.g[s].hole == -1 {
                queue += 1;
                if self.g[s ^ 1].hole == -1 {
                    waiting_pair = s as i32;
                }
            }
            if self.g[s].hole == 1 {
                on_hole1 += 1;
                if self.g[s].strokes == 0 {
                    can_tee = false;
                    queue += 1;
                }
            }
        }
        if self.game & 0x100000 == 0 {
            on_hole1 = 0;
        }
        let tournament = self.game & game::TOURNAMENT != 0;
        if !tournament && on_hole1 == 0 && can_tee && waiting_pair != -1 && self.g[waiting_pair as usize ^ 1].hole == -1 {
            self.start_pair(c, rng, waiting_pair as usize);
        }
        let mut want = self.tick & 0x7f == 0 || queue < 2;
        if queue >= 8 {
            want = false;
        }
        if want && !tournament && self.holes[1].par != 0 {
            let s = self.create(c, rng, true);
            if s != -1 {
                self.last_created = s;
                let su = s as usize;
                self.g[su].partner = s;
                self.g[su].pause = -6 - rng.below(6);
                self.pro_challenge_check(rng, su);
            }
        }
    }

    // ---- hole end (0x427380), round end (0x4266b0), removal (0x426670) ---------------------------------------------------

    /// The golfer has holed out (or picked up): statistics, the green fee, the score card, the next hole, the mood drop.
    pub fn hole_end(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize) {
        let h = self.g[g].hole;
        let hu = h.clamp(0, 19) as usize;
        if self.g[g].kind == 0 {
            let s = clamp(self.g[g].strokes, 0, 9) as usize;
            let cls = (self.g[g].class & 0xf) as usize;
            if let Some(v) = self.holes[hu].hist.get_mut(s + cls * 11) {
                *v += 1;
            }
        }
        self.g[g].card[hu.min(18)] = self.g[g].strokes as i8;
        let mut fee = self.g[g].mood;
        if self.fee_mode == 2 {
            fee *= 2;
        }
        if self.holes[hu].flags & 1 != 0 {
            fee += 2;
        }
        if self.holes[hu].flags & 2 != 0 {
            fee += 2;
        }
        fee += c.oper[14];
        let lvl = self.member(g).level & 7;
        let (x, y) = (self.g[g].x, self.g[g].y);
        if lvl > 3 {
            fee += if lvl != 4 { 5 } else { 2 };
            self.sound(0x19, Some((x, y)));
        }
        if self.game & game::TOURNAMENT == 0 {
            if self.year == 0 && self.fees_this_year == 0 {
                self.message(format!(
                    "{} has paid ${} in greens fees. Happy golfers pay more, so keep your golfers happy!",
                    self.name(g),
                    fee * 100
                ));
            }
            self.fees_this_year += fee;
            self.holes[hu].fees += fee;
            self.earn(fee, Column::GreensFees, (x, y));
        }
        if self.g[g].timer == 0 {
            let m = self.g[g].mood;
            self.event(c, rng, g, 0x13, m);
        }
        let strokes = self.g[g].strokes;
        self.member_mut(g).card[hu.min(18)] = strokes as u8;
        self.match_money(g);
        let mood = self.g[g].mood;
        self.out.push(Event::HoleDone { g, hole: h, strokes, mood, fee });
        self.g[g].bx = 0;
        self.g[g].strokes = 0;
        if self.g[g].hole_tick < self.tick {
            self.holes[hu].time += ((self.tick - self.g[g].hole_tick) / 2) as i32;
        }
        self.g[g].hole_tick = self.tick;
        self.g[g].flags &= 0xfbdf_bbff;
        self.g[g].club = 0;
        let last = h == 18 || self.par(h + 1) == 0;
        if self.game & game::TOURNAMENT == 0 && last {
            self.round_complete(c, g);
        }
        self.g[g].hole += 1;
        let nh = self.g[g].hole;
        let p = self.partner(g);
        if self.game & game::REPEAT == 0 {
            if self.par(nh) == 0 || self.g[p].hole == 0 || self.g[p].hole == 19 {
                self.round_end(rng, g);
                return;
            }
        } else {
            if self.par(nh) == 0 {
                self.g[g].hole = 1;
            }
            self.g[g].tee_order += self.next_hole * 2;
            let nh = self.g[g].hole.clamp(0, 18) as usize;
            if self.g[g].card[nh] != 0 {
                self.round_end(rng, g);
            }
        }
        let nh = self.g[g].hole;
        if self.par(nh + 1) == 0 {
            let who = match self.g[g].vip() {
                0x40 => Some("County commissioner"),
                0x60 => Some("Corporate CEO"),
                _ => None,
            };
            if let Some(w) = who {
                let r = crate::vips::mood_remark(self.g[g].mood);
                self.message(format!("{w} {} is playing the last hole on your course. \"{r}\"", self.vip_name(g)));
            }
        }
        let nhu = nh.clamp(0, 18) as usize;
        let gg = &mut self.g[g];
        gg.smooth_mood = gg.smooth_mood * 7 / 8 + gg.mood;
        if gg.flags & flag::STORY == 0 {
            gg.smooth_mood = gg.mood;
        } else {
            gg.hole_mood[nhu] = gg.smooth_mood as u8;
        }
        gg.end_mood[nhu] = gg.mood as u8;
        gg.momentum -= gg.momentum.signum();
        let diff = self.difficulty;
        if self.g[g].vip() == 0x40 {
            let v = (nh + 6) * self.g[g].mood * diff;
            self.g[g].mood -= v / 160;
        } else {
            let rounds = self.member(g).rounds;
            let hotels = c.oper[13];
            let gg = &mut self.g[g];
            gg.mood -= ((rounds + 6 + nh) * (gg.mood - 1 + diff) * (diff + 1)) / ((hotels * 5 + 15) * 8);
        }
    }

    /// End of a full round: best scores, the course record and membership points.
    fn round_complete(&mut self, c: &Course, g: usize) {
        let h = self.g[g].hole;
        let mut total = 0;
        let mut padded = 0;
        let mut over = 0;
        let mut pars = 0;
        let mut points = 0;
        let mut all = true;
        for k in 1..=18 {
            if h < k {
                padded += 5;
                over += 1;
            } else {
                let s = self.g[g].card[k as usize] as i32;
                total += s;
                padded += s;
                pars += self.holes[k as usize].par;
                over += s - self.holes[k as usize].par;
                points += (self.member(g).holes[k as usize] & 3) as i32;
            }
            if k < self.next_hole && self.g[g].card[k as usize] == 0 {
                all = false;
            }
        }
        if all {
            if let Some(i) = (0..10).find(|&i| self.top_rounds[i] == 0 || total < self.top_rounds[i]) {
                for j in (i + 1..10).rev() {
                    self.top_rounds[j] = self.top_rounds[j - 1];
                }
                self.top_rounds[i] = total;
            }
            // The exe then announces a course record when the round beats the best one, but it compares with the table it
            // has just updated, so the announcement (and its membership point) never happens. Kept as the exe has it.
            let best = self.top_rounds[0];
            if self.next_hole > 2 && self.g[g].card[1] != 0 && (best == 0 || total < best) && total <= pars {
                self.message(format!("{} has just set a new course record of {total}!", self.name(g)));
                points += 1;
            }
        }
        if self.difficulty * 2 + 6 <= self.g[g].mood {
            points += 1;
        }
        let diff = self.difficulty;
        let year = self.year;
        let poor = self.cash < 200 && self.homesite_demand < 1;
        let next = self.next_hole;
        let m = self.member_mut(g);
        if (padded as u8) < m.best || m.best == 0 {
            m.best = padded as u8;
        }
        m.avg = if m.avg == 0 { over as i8 } else { ((m.avg as i32 + over) / 2) as i8 };
        let last_flags = m.holes.get((next - 1).clamp(0, 18) as usize).copied().unwrap_or(0);
        if last_flags & 3 != 0 || diff == 0 || year == 0 || poor {
            points += 1;
        }
        points += c.oper[9];
        let lvl = (self.member(g).level & 7) as i32;
        if self.g[g].kind == 0 && (1 << lvl) <= points && lvl < 5 {
            self.g[g].flags |= flag::UPGRADE;
            self.g[g].pause = -8;
        }
    }

    /// Membership upgrade offer (0x406670): the level rises, the club is told, and a friend is invited.
    fn upgrade(&mut self, rng: &mut ExeRng, g: usize) {
        self.g[g].flags &= !flag::UPGRADE;
        let m = self.member_mut(g);
        m.level = (m.level & !7) | ((m.level + 1) & 7);
        let lvl = m.level & 7;
        let name = self.name(g);
        let text = match lvl {
            0..=2 => format!("{name} applies for membership."),
            3 => format!("{name} has decided to upgrade to a prestigious Silver membership."),
            4 => format!("{name} has decided to upgrade to a coveted Gold membership."),
            _ => format!("{name} has decided to upgrade to an exclusive Platinum membership."),
        };
        self.message(text);
        self.invite_friend(rng);
    }

    /// End of a golfer's round (0x4266b0): special visitors give their verdict, then the golfer heads home.
    pub fn round_end(&mut self, rng: &mut ExeRng, g: usize) {
        self.vip_verdict(rng, g);
        self.g[g].bx = 0;
        self.g[g].hole = 19;
        if self.g[g].flags & flag::GARY != 0 {
            self.gary = -1;
        }
    }

    /// Frees a golfer's slot (0x426670).
    fn remove(&mut self, g: usize) {
        self.g[g].strokes = 0;
        self.g[g].anim = 0;
        self.g[g].hole = 0;
    }
}

/// Per-golfer values the update works out once and its branches share.
struct Locals {
    /// Ball tile (old, at the start of the tick) and its type.
    ba: i32,
    bb: i32,
    bt: u8,
    /// Golfer tile and its type.
    ga: i32,
    gb: i32,
    gt: u8,
    /// Flags at the start of walking.
    old_flags: u32,
    /// I have teed off and my partner has not.
    wait_tee: bool,
    /// My ball is nearer the pin than my partner's.
    closer: bool,
}

/// Flat tile index the exe uses for neighbour reads (no bounds check; outside the map reads as out of bounds here).
fn flat(a: i32, b: i32) -> Option<usize> {
    let i = a * 50 + b;
    (0..NN as i32).contains(&i).then_some(i as usize)
}

impl Club {
    fn pin_c(&self, h: i32) -> (i32, i32) {
        let p = self.holes[h.clamp(0, 19) as usize].pin;
        (p.0 * UNIT + 512, p.1 * UNIT + 512)
    }

    /// One game tick for the golfers: arrivals, the animation pass of the drawing routine, then every golfer's update.
    pub fn tick(&mut self, c: &mut Course, rng: &mut ExeRng, tick: u32) {
        self.tick = tick;
        if tick > 0 && tick & 0x3ff == 0 {
            self.month_decay();
        }
        self.arrivals(c, rng);
        for g in 0..SLOTS {
            self.animate(rng, g);
        }
        for g in 0..SLOTS {
            self.update(c, rng, g);
        }
        // The main frame clears the aim-search lock once per frame.
        self.plan_busy = false;
    }

    /// The golfer update (0x4289e0) for one golfer.
    fn update(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize) {
        if self.g[g].hole == 0 {
            return;
        }
        let tick = self.tick;
        let diff = self.difficulty;
        if self.g[g].hole == -1 {
            if self.g[g].timer == 0 {
                if rng.below(100) == 0 {
                    let (a, b) = c.objects.first().map(|o| (o.a, o.b)).unwrap_or(c.door);
                    self.g[g].x = a * UNIT + 0x600;
                    self.g[g].y = b * UNIT + 0x600;
                }
                if self.g[g].timer == 0 {
                    return;
                }
            }
            if rng.below(2) != 0 {
                self.g[g].timer -= 1;
            }
            return;
        }
        // Thought timer: counts down every 8 ticks; at 1 the exe asks the golfer's attitude and drops the answer.
        if self.g[g].timer != 0 && tick & 7 == 0 {
            self.g[g].timer -= 1;
            if self.g[g].timer == 1 {
                let _ = rng.below(6) > diff;
            }
        }
        if self.g[g].flags & flag::STORY != 0 {
            self.story_trigger(c, rng, g);
        }
        self.scenery_glance(c, rng, g);
        // Ball tile and type, before the ball moves this tick.
        let (ba, bb) = (self.g[g].bx >> 10, self.g[g].by >> 10);
        let bt = if c.oob(ba, bb) { t::OUT } else { c.ty[idx(ba, bb)] };
        self.needs(c, rng, g);
        let (ga, gb) = (self.g[g].x >> 10, self.g[g].y >> 10);
        let gt = flat(ga, gb).map(|i| c.ty[i]).unwrap_or(t::OUT);
        if self.g[g].speed == 0 {
            self.g[g].flags &= !flag::BALL_MOVING;
        }
        let mut l = Locals { ba, bb, bt, ga, gb, gt, old_flags: 0, wait_tee: false, closer: false };
        if self.g[g].pause < 0 {
            if tick & 1 == 1 {
                self.g[g].pause += 1;
            }
            if self.g[g].flags & flag::BALL_MOVING != 0 {
                self.ball(c, rng, g, &l);
            }
            return;
        }
        if self.g[g].flags & flag::UPGRADE != 0 {
            self.upgrade(rng, g);
        }
        l.old_flags = self.g[g].flags;
        self.g[g].flags &= !flag::CART;
        if self.g[g].strokes > 9 {
            self.hole_end(c, rng, g);
            return;
        }
        let p = self.partner(g);
        if self.g[p].hole == 0 && self.g[g].strokes == 0 && self.g[g].hole != 19 {
            self.round_end(rng, g);
        }
        let (me, pp) = (&self.g[g], &self.g[p]);
        l.wait_tee = pp.hole != 0 && pp.hole == me.hole && me.flags & flag::TEED_OFF != 0 && pp.flags & flag::TEED_OFF == 0;
        if me.flags & flag::TEED_OFF != 0 && me.bx != 0 && pp.bx != 0 && me.hole == pp.hole {
            let pin = self.holes[me.hole.clamp(0, 19) as usize].pin;
            l.closer = tdist(me.bx, me.by, pin.0, pin.1) < tdist(pp.bx, pp.by, pin.0, pin.1);
        }
        let me = &self.g[g];
        if me.bx == 0 || me.sub == 0 || l.wait_tee || me.flags & flag::LEAVING != 0 {
            self.movement(c, rng, g, &l);
        } else {
            self.shot(c, rng, g, &l);
        }
    }

    /// The golfer glances at a neighbouring tile now and then: flowers and landmarks please, eyesores and weeds do not.
    fn scenery_glance(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize) {
        let gg = &self.g[g];
        let mut mask: u32 = if gg.args[0] & 0xc000 != 0 { 0x1ff } else { 0xff };
        if gg.args[0] & 0x4000 != 0 {
            mask >>= 2;
        }
        if gg.walk_left > 0 {
            mask >>= 1;
        }
        if (self.tick.wrapping_add(33 * g as u32)) & mask != 0 || gg.thoughts[0] & 0x7f == 0xb || gg.thoughts[1] & 0x7f == 0xb {
            return;
        }
        let d = ((gg.facing - 2) + rng.below(5)) & 7;
        let (na, nb) = ((gg.x >> 10) + DX[d as usize], (gg.y >> 10) + DY[d as usize]);
        let arg = na + nb * 50;
        let Some(i) = flat(na, nb) else { return };
        if c.ty[i] == t::BUILDING {
            let o = c.object_at(na, nb);
            if o >= 0 {
                let ob = c.objects[o as usize];
                if ob.kind == 2 {
                    self.event(c, rng, g, 0xb, arg);
                } else if ob.kind == 4 {
                    self.event(c, rng, g, if ob.sub < 0x10 { 0xb } else { 0x14 }, arg);
                }
            }
        }
        let fl = c.flags[i];
        if fl & f::FLOWERS != 0 && (self.attitude(g) == 2 || fl & f::WEED != 0) {
            self.event(c, rng, g, if fl & f::WEED != 0 { 0x14 } else { 0xb }, arg);
        }
    }

    /// Hunger and thirst grow every 160 ticks (120 while walking a leg); hunger near trees, water and marsh.
    fn needs(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize) {
        let period = if self.g[g].walk_left != 0 { 120 } else { 160 };
        if (self.tick as i32).wrapping_add(37 * g as i32) % period != 0 || self.g[g].hole == 19 {
            return;
        }
        if self.g[g].hole > 1 && self.g[g].card[0] == 0 && self.g[g].clubs_used == 0 && rng.below(self.next_hole / 2) == 0 {
            self.event(c, rng, g, 0x3f, 0x14);
        }
        let (a, b) = (self.g[g].x >> 10, self.g[g].y >> 10);
        let group = |i: Option<usize>| i.map(|i| c.row(c.ty[i]).group).unwrap_or(6);
        let mut near = matches!(group(flat(a, b)), 7 | 8 | 0xe);
        for d in 0..8 {
            if matches!(group(flat(a + DX[d], b + DY[d])), 7 | 8 | 0xe) {
                near = true;
            }
        }
        if (c.theme == 3 && rng.below(2) != 0) || near {
            if self.g[g].hole > 2 && rng.below(2) == 0 {
                self.g[g].hunger += 1;
                let h = self.g[g].hunger;
                if h > 15 && h % 4 == 0 {
                    if self.g[g].vip() == 0x20 {
                        self.g[g].hunger = 16;
                    } else {
                        self.event(c, rng, g, 0xf, 0x14);
                    }
                }
            }
        } else {
            self.g[g].thirst += 1;
            let th = self.g[g].thirst;
            if th > 15 && th % 4 == 0 {
                if self.g[g].vip() == 0x20 {
                    self.g[g].thirst = 16;
                } else {
                    self.event(c, rng, g, 0xe, 0x14);
                }
            }
        }
    }

    /// A golfer whose mood fell below zero quits: the club hears why, a member resigns, and the golfer storms off.
    fn quit(&mut self, g: usize) {
        let reason = match self.g[g].thought & 0x7f {
            4 => "will be looking for a tougher course".to_string(),
            8 | 0x17 | 0x1e => "thinks your course needs improvement".to_string(),
            9 => "has punched out another golfer".to_string(),
            0xc => format!("has wrapped {} club around a tree", self.his_her(g)),
            0xd => format!("has thrown {} clubs into the lake", self.his_her(g)),
            0xe => "is too thirsty to play any more".to_string(),
            0xf => "is too hungry to keep playing".to_string(),
            0x15 => "has insulted another golfer".to_string(),
            0x1a => "is too tired to continue".to_string(),
            _ => "is leaving the course in disgust".to_string(),
        };
        let h = self.g[g].hole;
        let at = if h < 19 { format!("after hole {h}") } else { "after today's round".to_string() };
        let name = self.name(g);
        self.message(format!("{name} {reason} {at}."));
        if self.member(g).level & 7 > 1 {
            let hh = self.his_her(g);
            self.message(format!("{name} resigns {hh} membership."));
            self.member_mut(g).level &= !7;
        }
        let hu = h.clamp(0, 18) as usize;
        self.member_mut(g).holes[hu] |= 4;
        self.member_mut(g).gone = 0xff;
        self.sound(0x29, None);
        let old = self.g[g].hole;
        if let Some(hr) = self.hole_rec(old) {
            hr.quits += 1;
        }
        let gg = &mut self.g[g];
        gg.pause = -99;
        gg.anim = anim::UPSET;
        gg.hole = 19;
        gg.bx = 0;
        gg.flags = (gg.flags & !flag::MAY_CART) | flag::LEAVING;
        gg.fatigue = 0;
        gg.momentum = -3;
        if gg.flags & flag::GARY != 0 {
            self.gary = -1;
        }
    }

    /// Walking: where to go next and the steps there (the movement branch of 0x4289e0).
    fn movement(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, l: &Locals) {
        let diff = self.difficulty;
        let tick = self.tick;
        if (self.game & game::TOURNAMENT != 0 || self.g[g].vip() == 0x20) && self.g[g].mood < 0 {
            self.g[g].mood = 0;
        }
        if self.g[g].timer == 0 && self.g[g].flags & flag::LEAVING == 0 && self.g[g].mood < 0 {
            self.quit(g);
        }
        let p = self.partner(g);
        let mut queue = 0;
        let mut building: i32 = -1;
        let mut can_start = true;
        let mut yield_ = false;
        let (mut tx, mut ty);
        let mut tee_pos = (0, 0);
        if self.g[g].flags & flag::LEAVING == 0 {
            if self.g[g].hole == 19 && ((l.ga, l.gb) == c.door || self.g[g].flags & flag::GARY != 0 || self.g[g].sx == -1) {
                self.g[g].timer = 0;
                self.g[g].hole = 0;
                if self.g[g].flags & flag::GARY != 0 || self.g[p].flags & flag::GARY != 0 {
                    if self.g[p].hole != 0 && self.g[p].hole != 19 {
                        return;
                    }
                    if g as i32 == self.gary {
                        self.gary = -1;
                    }
                }
                self.remove(g);
                return;
            }
            if self.par(self.g[g].hole) == 0 {
                self.g[g].hole = 19;
            }
            let h = self.g[g].hole;
            if self.g[g].strokes == 0 {
                for o in 0..SLOTS {
                    let og = &self.g[o];
                    if og.hole > 0 && o != g && og.hole == h && og.strokes == 0 && og.tee_order <= self.g[g].tee_order && o != p {
                        if (g as i32 == self.gary || p as i32 == self.gary) && self.game & game::REPEAT == 0 {
                            self.round_end(rng, o);
                            self.round_end(rng, o ^ 1);
                        } else {
                            queue += 1;
                        }
                    }
                }
            }
            let pin = self.holes[h.clamp(0, 19) as usize].pin;
            let pin_c = self.pin_c(h);
            let me = self.g[g].clone();
            let pg = self.g[p].clone();
            let pin_ang = if me.bx != 0 { angle(pin_c.0 - me.bx, pin_c.1 - me.by) } else { 0 };
            if me.bx != 0 && pg.bx != 0 && me.hole == pg.hole {
                let dp = tdist(pg.bx, pg.by, pin.0, pin.1);
                let dm = tdist(me.bx, me.by, pin.0, pin.1);
                if dm < dp {
                    let pt = c.tile_type(pg.bx, pg.by);
                    if (len(me.x - pg.bx, me.y - pg.by) < len(me.x - me.bx, me.y - me.by) || pt == t::GREEN)
                        && tdist(pg.bx, pg.by, pin.0, pin.1) < tdist(me.x, me.y, pin.0, pin.1)
                    {
                        yield_ = true;
                    }
                    if (c.h(l.bt) < 1 && c.h(pt) > 0) || pg.flags & flag::BALL_MOVING != 0 {
                        yield_ = false;
                    }
                    if l.bt != t::GREEN {
                        if len(me.bx - pg.bx, me.by - pg.by) < 0x600 {
                            yield_ = true;
                        }
                        if len(me.bx - pg.x, me.by - pg.y) < 0x200 {
                            yield_ = true;
                        }
                    }
                }
            }
            if me.flags & flag::HURRIED != 0 {
                yield_ = false;
            }
            if me.bx == 0 {
                yield_ = g & 1 == 1;
            }
            // Watch the partner's shot when standing near them.
            if l.bt != t::GREEN && pg.hole != 0 && (len(me.x - pg.x, me.y - pg.y).wrapping_mul(25) & !0x3ff) <= 0x9fff {
                let watching = pg.bz != 0
                    && (pg.flags & flag::BALL_MOVING != 0 || pg.anim <= 10 || matches!(pg.anim, 0x10 | 0xc | 0xd))
                    && adiff(angle(pg.bx - me.x, pg.by - me.y), pg.heading).unsigned_abs() <= 0x4aaa_aaa9;
                if watching {
                    let a = angle(pg.bx - me.x, pg.by - me.y);
                    self.g[g].anim = anim::STAND;
                    self.g[g].facing = dir8(a);
                    self.g[g].pause = -rng.below(4);
                    self.g[g].walk_left = 0;
                    return;
                }
            }
            if me.bx != 0 && !l.wait_tee {
                let d = dir8(pin_ang) as usize;
                if !yield_ {
                    let k = if l.bt == t::GREEN {
                        (!(d as i32) & 1) | 2
                    } else if d & 1 == 1 {
                        3
                    } else {
                        5
                    };
                    tx = me.bx - DX[d] * k * 0x40;
                    ty = me.by - DY[d] * k * 0x40;
                } else {
                    if l.bt == t::GREEN && c.tile_type(pg.bx, pg.by) == t::GREEN {
                        tx = me.bx - (DX[d] * 0x400 + 0x200) / 2;
                        ty = me.by - (DY[d] * 0x400 + 0x200) / 2;
                    } else {
                        let a = angle(pin_c.0 - pg.bx, pin_c.1 - pg.by);
                        let d2 = ((dir8(a) - ((g as i32 & 1) + 1)) & 7) as usize;
                        tx = pg.bx - DX[d2] * 0x400;
                        ty = pg.by - DY[d2] * 0x400;
                    }
                    if c.tile_type(tx, ty) == t::WATER {
                        tx = me.bx;
                        ty = me.by;
                    }
                }
            } else {
                let hr = &self.holes[h.clamp(0, 19) as usize];
                let tee = if me.class < 5 && me.class != 3 && me.kind == 0 { hr.fwd } else { hr.back };
                tee_pos = (tee.0 * UNIT + 512, tee.1 * UNIT + 512);
                let d = dir8(angle(pin_c.0 - tee_pos.0, pin_c.1 - tee_pos.1)) as usize;
                let k = if d & 1 == 1 { 3 } else { 5 } * 0x40;
                tx = tee_pos.0 - DX[d] * k;
                ty = tee_pos.1 - DY[d] * k;
                if queue != 0 {
                    let d3 = ((hr.tee_facing + (g as i32 & 1)) & 7) as usize;
                    tx -= DX[d3] * 0x400;
                    ty -= DY[d3] * 0x400;
                }
                let chosen = self.detours(c, g, queue, &mut building);
                if let Some((x, y)) = chosen {
                    tx = x;
                    ty = y;
                } else {
                    // Step aside at the tee while the partner plays first.
                    let gg = &self.g[g];
                    let pg = &self.g[p];
                    let hr = &self.holes[gg.hole.clamp(0, 19) as usize];
                    if pg.hole != 0
                        && pg.hole <= gg.hole
                        && (g as i32) < p as i32
                        && (pg.flags & flag::TEED_OFF == 0 || pg.hole < gg.hole || pg.anim == anim::HAPPY)
                    {
                        can_start = false;
                        let d = ((hr.tee_facing - 1) & 7) as usize;
                        tx += (DX[d] * -0x400) / 2;
                        ty += (DY[d] * -0x400) / 2;
                        self.g[g].flags |= flag::ASIDE;
                    }
                    if self.g[p].hole != 0 && l.wait_tee {
                        can_start = false;
                        let d = (self.holes[self.g[g].hole.clamp(0, 19) as usize].tee_facing & 7) as usize;
                        tx += (DX[d] * -0x400) / 2;
                        ty += (DY[d] * -0x400) / 2;
                        self.g[g].flags |= flag::ASIDE;
                    }
                }
            }
        } else {
            // A golfer leaving angry: every 100 ticks a grumble and a stamp of the feet; otherwise off to the clubhouse.
            if ((g as i32) * 35 + tick as i32) % 100 == 0 {
                self.event(c, rng, g, 0x23, 0x14);
                self.g[g].anim = anim::UPSET;
                if rng.below(2) == 0 {
                    self.g[g].flags &= !flag::BAD_LIE;
                } else {
                    self.g[g].flags |= flag::BAD_LIE;
                }
                self.g[g].pause = -99;
                self.g[g].mood -= 1;
                if self.g[g].mood < -10 {
                    self.g[g].hole = 0;
                }
                return;
            }
            tx = c.door.0 * UNIT + 512;
            ty = c.door.1 * UNIT + 512;
        }
        self.walk_to(c, rng, g, l, (tx, ty), building, queue, can_start, yield_, tee_pos);
        let _ = diff;
    }

    /// The detours before teeing off: snack bar, bench, ball washer and (before the first shot) the practice buildings.
    /// Returns the new target and sets `building` (-2 for a bench).
    fn detours(&mut self, c: &mut Course, g: usize, queue: i32, building: &mut i32) -> Option<(i32, i32)> {
        let p = self.partner(g);
        let (x, y) = (self.g[g].x, self.g[g].y);
        let gg = &self.g[g];
        let snack_need = !((gg.hunger < 8 - if gg.flags & flag::TO_SNACK != 0 { 4 } else { 0 } - 2 * queue || gg.hunger < 2)
            && (gg.thirst < if gg.flags & flag::THIRST_LOW != 0 { 8 } else { 12 } - 2 * queue || gg.thirst < 3));
        if !snack_need {
            self.g[g].flags &= !flag::TO_SNACK;
        } else {
            let (b, d) = c.nearest(7, x, y);
            let reach = if self.g[g].flags & flag::TO_SNACK != 0 { 10 } else { 8 };
            if b < 0 || reach * 0x400 <= d {
                self.g[g].flags &= !flag::TO_SNACK;
            } else {
                self.g[g].flags = (self.g[g].flags & !flag::TO_BENCH) | flag::TO_SNACK;
                let o = c.objects[b as usize];
                *building = b;
                self.g[g].flags &= !(flag::TO_PUTTING | flag::TO_SHOP | flag::TO_RANGE);
                return Some((o.a * 0x400 + 0x200, o.b * 0x400 + 0x200));
            }
        }
        // Bench: tired golfers sit before teeing off.
        let gg = &self.g[g];
        if (6 - queue <= gg.fatigue / 20 || gg.flags & flag::IMPATIENT != 0) && gg.fatigue != 0 && gg.strokes == 0 {
            let r = if gg.flags & flag::TO_BENCH != 0 { 4 } else { 2 };
            if let Some((a, b)) = c.nearest_bench(x, y, r) {
                *building = -2;
                self.g[g].flags |= flag::TO_BENCH;
                self.g[g].flags &= !(flag::TO_PUTTING | flag::TO_SHOP | flag::TO_RANGE);
                return Some((a * 0x400 + 0x200, b * 0x400 + 0x200));
            }
            self.g[g].flags &= !flag::TO_BENCH;
        }
        // Ball washer while the partner tees off first.
        let gg = &self.g[g];
        let pg = &self.g[p];
        if (pg.tee_order < gg.tee_order || pg.strokes != 0) && gg.flags & flag::WASHED == 0 {
            let (b, d) = c.nearest(3, x, y);
            if b >= 0 && d < 0xc00 {
                let o = c.objects[b as usize];
                *building = b;
                self.g[g].flags &= !(flag::TO_PUTTING | flag::TO_SHOP | flag::TO_RANGE);
                return Some((o.a * 0x400 + 0x200, o.b * 0x400 + 0x200));
            }
        }
        if self.g[g].strokes != 0 {
            return None;
        }
        self.g[g].club = 0;
        let gi = g as i32 & 1;
        let gg = &self.g[g];
        if gg.flags & flag::VISITED_PUTTING == 0 && gg.class & 4 != 0 && gg.kind == 0 && c.oper[6] != 0 {
            let (b, d) = c.nearest(6, x, y);
            let reach = if gg.flags & flag::TO_PUTTING != 0 { 7 } else { 5 };
            if b < 0 || reach * 0x400 <= d {
                self.g[g].flags &= !flag::TO_PUTTING;
            } else {
                let o = c.objects[b as usize];
                *building = b;
                self.g[g].flags = (self.g[g].flags | flag::TO_PUTTING) & !(flag::TO_SHOP | flag::TO_RANGE);
                return Some(((o.a + 1 + gi) * 0x400, (o.b + 1 + gi) * 0x400));
            }
        }
        let gg = &self.g[g];
        if gg.flags & flag::VISITED_SHOP == 0 && gg.class & 2 != 0 && gg.kind == 0 && c.oper[8] != 0 {
            let (b, d) = c.nearest(8, x, y);
            let reach = if gg.flags & flag::TO_SHOP != 0 { 7 } else { 5 };
            if b < 0 || reach * 0x400 <= d {
                self.g[g].flags &= !flag::TO_SHOP;
            } else {
                let o = c.objects[b as usize];
                *building = b;
                self.g[g].flags = (self.g[g].flags | flag::TO_SHOP) & !flag::TO_RANGE;
                return Some(((o.a + gi) * 0x400 + 0x200, (o.b + gi) * 0x400 + 0x200));
            }
        }
        let gg = &self.g[g];
        if gg.flags & flag::VISITED_RANGE == 0 && gg.class & 1 != 0 && gg.kind == 0 && c.oper[10] != 0 {
            let (b, d) = c.nearest(10, x, y);
            let reach = if gg.flags & flag::TO_RANGE != 0 { 9 } else { 6 };
            if b < 0 || reach * 0x400 <= d {
                self.g[g].flags &= !flag::TO_RANGE;
            } else {
                let o = c.objects[b as usize];
                *building = b;
                self.g[g].flags |= flag::TO_RANGE;
                return Some(((o.a + gi) * 0x400 + 0xa00, (o.b + gi) * 0x400 + 0xa00));
            }
        }
        None
    }

    /// Walks toward a target, or acts on arriving there (LAB_0042a818 onward).
    #[allow(clippy::too_many_arguments)]
    fn walk_to(
        &mut self,
        c: &mut Course,
        rng: &mut ExeRng,
        g: usize,
        l: &Locals,
        target: (i32, i32),
        building: i32,
        queue: i32,
        can_start: bool,
        yield_: bool,
        tee_pos: (i32, i32),
    ) {
        let (mut tx, mut ty) = target;
        let tick = self.tick;
        let diff = self.difficulty;
        let p = self.partner(g);
        if self.g[g].bx != 0 && c.tile_type(tx, ty) == t::WATER {
            tx = self.g[g].bx;
            ty = self.g[g].by;
        }
        let (x, y) = (self.g[g].x, self.g[g].y);
        let dist = len(tx - x, ty - y);
        // Arrival radius: 2/16 of a tile at a spot or a bench by the exe's reading, half a tile at a building.
        let radius = if building == -1 || (building >= 0 && c.objects.get(building as usize).map(|o| o.kind) == Some(1)) { 2 } else { 8 };
        if dist < radius * 0x40 {
            self.g[g].anim = anim::STAND;
            if self.g[g].bx != 0 {
                let pg = &self.g[p];
                self.g[g].facing = dir8(angle(pg.bx - x, pg.by - y));
            }
            if self.g[g].flags & flag::BALL_MOVING != 0 {
                self.ball(c, rng, g, l);
                return;
            }
            let h = self.g[g].hole;
            if h == 19 {
                self.g[g].hole = 0;
            } else if building == -2 {
                self.g[g].pause = (self.g[g].fatigue * -8) / 20;
                self.g[g].anim = anim::SIT_DOWN;
                self.g[g].flags &= !flag::IMPATIENT;
                self.event(c, rng, g, 0x1b, 0x14);
                for _ in 0..10_000 {
                    let d = rng.below(4);
                    if c.bench_ok(l.ga, l.gb, d * 2) {
                        let dd = (d * 2) as usize;
                        self.g[g].x = DX[dd] * 0x1ff + 0x200 + l.ga * 0x400;
                        self.g[g].y = DY[dd] * 0x1ff + 0x200 + l.gb * 0x400;
                        self.g[g].fatigue = 0;
                        self.g[g].facing = d * 2;
                    } else {
                        self.g[g].fatigue -= 1;
                    }
                    if self.g[g].fatigue == 0 {
                        break;
                    }
                }
                self.g[g].sub = 0;
            } else if building == -1 {
                let me = self.g[g].clone();
                let pg = self.g[p].clone();
                if pg.hole != 0 {
                    if me.bx == 0 && pg.bx != 0 && pg.hole < me.hole {
                        self.g[g].anim = anim::STAND;
                        self.g[g].facing = dir8(angle(pg.x - x, pg.y - y));
                        self.g[g].walk_left = 0;
                        return;
                    }
                    if me.hole == pg.hole
                        && me.speed == 0
                        && (((g as i32) < p as i32 && pg.strokes == 0 && me.flags & flag::HURRIED == 0)
                            || ((p as i32) < g as i32 && me.flags & flag::TEED_OFF != 0 && pg.strokes == 0))
                    {
                        self.g[g].anim = anim::STAND;
                        self.g[g].facing = dir8(angle(pg.x - x, pg.y - y));
                        self.g[g].walk_left = 0;
                        self.g[g].pause = -4 - rng.below(4);
                        return;
                    }
                }
                let pin = self.holes[me.hole.clamp(0, 19) as usize].pin;
                let pin_y = pin.1 * 0x400 + 0x200;
                // The exe compares the partner ball's y offset from the pin with my ball's distance (a quirk kept as is).
                if pg.hole != me.hole
                    || pg.strokes == 0
                    || me.bx == 0
                    || pg.by - pin_y <= len(me.bx - (pin.0 * 0x400 + 0x200), me.by - pin_y)
                {
                    self.g[g].aim_a = 0;
                    if !l.wait_tee && can_start && queue == 0 {
                        if self.g[g].bx == 0 {
                            self.g[g].bx = tee_pos.0;
                            self.g[g].by = tee_pos.1;
                        }
                        self.g[g].sub = 1;
                    }
                    self.g[g].walk_left = 0;
                    self.g[g].vz = 0;
                    self.g[g].speed = 0;
                    self.g[g].bz = 0;
                    return;
                }
                self.g[g].anim = anim::STAND;
                self.g[g].facing = dir8(angle(pg.x - x, pg.y - y));
                self.g[g].walk_left = 0;
                self.g[g].pause = -rng.below(4);
            } else {
                let o = c.objects[building as usize];
                let at = (x, y);
                match o.kind {
                    7 => {
                        self.event(c, rng, g, 0x12, 0x14);
                        self.g[g].thirst = 0;
                        self.g[g].hunger = 0;
                        self.g[g].pause = -48;
                        self.g[g].anim = anim::STAND;
                        self.sound(0x29, Some(at));
                        self.earn(5, Column::FoodDrink, at);
                    }
                    3 => {
                        self.g[g].pause = -12 - rng.below(8);
                        self.g[g].anim = anim::WASH;
                        self.g[g].flags |= flag::WASHED;
                        self.event(c, rng, g, 0x29, 0x14);
                        self.g[g].timer = self.g[g].timer.wrapping_add(3);
                        if inside(o.a, o.b) {
                            let i = idx(o.a, o.b);
                            c.flags[i] |= f::GROWING;
                            c.growth[i] = 0;
                        }
                    }
                    6 => {
                        self.g[g].pause = -16 - rng.below(9);
                        self.g[g].club = 13;
                        self.g[g].anim = 0;
                        self.g[g].flags |= flag::VISITED_PUTTING;
                        let fee = if c.level[6] < 2 { 4 } else { 8 };
                        self.earn(fee, Column::FoodDrink, at);
                        self.event(c, rng, g, 0x35, 0x14);
                    }
                    8 => {
                        self.g[g].pause = -16 - rng.below(9);
                        self.g[g].flags |= flag::VISITED_SHOP;
                        self.event(c, rng, g, 0x34, 0x14);
                        let fee = if c.level[8] < 2 { 6 } else { 10 };
                        self.earn(fee, Column::FoodDrink, at);
                    }
                    10 => {
                        self.g[g].pause = -16 - rng.below(9);
                        self.g[g].flags |= flag::VISITED_RANGE;
                        self.event(c, rng, g, 0x33, 0x14);
                        let fee = if c.level[10] < 2 { 8 } else { 12 };
                        self.earn(fee, Column::FoodDrink, at);
                    }
                    _ => {}
                }
            }
            return;
        }
        // Not there yet: choose a heading now and then.
        let old_facing = self.g[g].facing;
        if dist < 0x400 {
            self.g[g].facing = cdir((tx - x) + dist / 2, (ty - y) + dist / 2);
            self.g[g].walk_left = 0;
        } else if self.g[g].walk_left < 1 {
            if let Some(i) = flat(l.ga, l.gb) {
                let fl = c.flags[i];
                if c.ty[i] == t::WATER && fl & f::PATH != 0 && fl & f::SCENIC != 0 && self.g[g].thought & 0x7f != 7 {
                    self.event(c, rng, g, 7, 0);
                    let r = rng.below(5);
                    let bc = self.g[g].field_bc;
                    self.g[g].field_bc = (!(1u8 << r) & bc) | (1u8 << (fl & 7));
                }
            }
            self.game &= !game::BRIDGE;
            let f = self.path_dir(c, rng, g, (tx, ty), (x, y), false);
            self.g[g].facing = f;
            if l.gt != t::TEE && f == (old_facing ^ 4) {
                self.g[g].facing = (f + if tick & 0x10 != 0 { 1 } else { -1 }) & 7;
            }
            let fac = self.g[g].facing;
            let fi = (fac & 7) as usize;
            let ux = (x >> 6) & 0xf;
            let uy = (y >> 6) & 0xf;
            let mut s = if DX[fi] == 0 {
                if DY[fi] == 1 {
                    0x18 - uy
                } else {
                    uy + 8
                }
            } else if DX[fi] == 1 {
                0x18 - ux
            } else {
                ux + 8
            };
            if fac & 1 == 1 && s > 0x10 {
                s = 0x10;
            }
            s -= 1;
            if self.game & game::BRIDGE != 0 {
                s = 3;
            }
            self.g[g].walk_left = s << 6;
            // Off-path warning: the second straight step in a row across rough or worse ground with no path complains.
            let gg = &self.g[g];
            let on_path = flat(l.ga, l.gb).map(|i| c.flags[i] & f::PATH != 0).unwrap_or(false);
            if diff == 0
                || gg.strokes > 1
                || gg.hole == 19
                || c.h(l.bt) > 0
                || (gg.strokes != 0 && gg.flags & flag::ASIDE != 0)
                || on_path
                || l.gt == t::BUILDING
                || c.h(l.gt) < 2
            {
                self.g[g].flags &= !flag::OFF_PATH;
            } else if fac & 1 == 0 && l.gt != t::SAND {
                if self.g[g].flags & flag::OFF_PATH != 0 && self.g[g].thought != 10 {
                    self.event(c, rng, g, 10, l.gt as i32);
                    self.g[g].flags &= !flag::OFF_PATH;
                } else {
                    self.g[g].flags |= flag::OFF_PATH;
                }
            }
            let gg = &self.g[g];
            if gg.hole != 19 && gg.flags & flag::MAY_CART == 0 && gg.thought != 0x2b && fac & 1 == 0 && c.rise(x, y, fac) > 2 {
                let r = c.rise(x, y, fac);
                self.event(c, rng, g, 0x2b, r);
            }
        }
        if l.gt != t::TEE && self.g[g].facing == (old_facing ^ 4) {
            self.g[g].pause = -8 - rng.below(4);
            self.g[g].anim = anim::STAND;
        }
        if self.give_way(c, rng, g, yield_) {
            return;
        }
        if self.g[g].pause >= 0 && self.g[g].facing != -1 {
            self.step(c, rng, g, l, (tx, ty), dist);
            if self.g[g].speed != 0 {
                if !l.wait_tee {
                    self.shot(c, rng, g, l);
                } else {
                    self.g[g].pause = -rng.below(8);
                    self.g[g].sub = 0;
                    self.g[g].anim = anim::STAND;
                }
            }
            return;
        }
        self.g[g].walk_left = 0;
    }

    /// Golfers ahead on the path make this one wait; long queues on the tee frustrate (slow play), and a frustrated pair quits.
    /// Returns true when the golfer quit.
    fn give_way(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, yield_: bool) -> bool {
        if self.g[g].flags & 0x2100_2000 != 0 {
            return false;
        }
        let p = self.partner(g);
        let mut o = self.last_created;
        let mut ahead = 0;
        let mut blocked = false;
        loop {
            o = (o + 1).rem_euclid(SLOTS as i32);
            let ou = o as usize;
            let og = self.g[ou].clone();
            let me = self.g[g].clone();
            let mut check = false;
            if og.flags & flag::LEAVING == 0 || og.hole == 0 || ou == g {
                if og.hole > 0 {
                    if ou == g {
                        break;
                    }
                    let pg = &self.g[p];
                    if og.hole <= me.hole
                        && og.tee_order <= me.tee_order
                        && (ou != p || (me.facing == pg.facing && pg.anim != anim::STAND && (ou != p || yield_)))
                    {
                        if og.hole == me.hole && og.strokes == 0 {
                            ahead += 1;
                        }
                        check = true;
                    }
                }
            } else {
                check = true;
            }
            if check {
                let (dx, dy) = (og.x - me.x, og.y - me.y);
                let leaving = og.flags & flag::LEAVING != 0;
                if len(dx, dy) < ((if leaving { 4 } else { 1 }) * 0x400) / 2 {
                    let cd = cdir(dx, dy);
                    let f = self.g[g].facing;
                    if f == cd || ou == p || f == ((cd + 1) & 7) || f == ((cd - 1) & 7) {
                        self.g[g].pause = -4 - rng.below(4);
                        blocked = true;
                        self.g[g].anim = anim::STAND;
                    }
                    if self.g[g].thoughts[0] != 0x24 && leaving {
                        let male = self.male(ou);
                        self.event(c, rng, g, 0x24, male);
                        self.g[g].pause = -4 - rng.below(4);
                        self.g[g].anim = anim::STAND;
                        self.g[g].facing = cdir(dx, dy);
                        blocked = true;
                    }
                }
            }
            if o == g as i32 {
                break;
            }
        }
        let gg = &self.g[g];
        if !(blocked && gg.strokes == 0 && ahead > 3 && gg.flags & 0x6000 == 0) {
            return false;
        }
        if gg.flags & flag::IMPATIENT == 0 {
            self.g[g].flags |= flag::IMPATIENT;
        } else {
            self.event(c, rng, g, 0x15, 0x14);
            self.g[g].fatigue += 1;
        }
        self.g[g].pause = -0x20 - rng.below(0x40);
        if self.difficulty == 0 {
            self.g[g].pause = -100 - rng.below(0x1c);
            self.g[g].flags |= flag::HURRIED;
        }
        self.g[g].anim = anim::STAND;
        if self.g[g].mood < 0 && self.g[g].vip() != 0x20 {
            let h = self.g[g].hole;
            self.message(format!(
                "{} and {} are leaving the course at hole {h}. 'Arghh, I am so tired of waiting...'",
                self.name(g),
                self.name(p)
            ));
            self.sound(0x29, None);
            self.g[g].pause = 0;
            if let Some(hr) = self.hole_rec(h) {
                hr.quits += 1;
            }
            let hu = h.clamp(0, 18) as usize;
            self.member_mut(g).holes[hu] |= 4;
            self.member_mut(g).gone = 0xff;
            self.g[p].bx = 0;
            self.g[p].x = 0;
            self.round_end(rng, p);
            let gg = &mut self.g[g];
            gg.hole = 19;
            gg.flags = (gg.flags & !flag::MAY_CART) | flag::LEAVING;
            gg.fatigue = 0;
            gg.momentum = -3;
            return true;
        }
        false
    }

    /// One walking step: speed from the ground, paths and carts, tiredness.
    fn step(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, l: &Locals, target: (i32, i32), dist: i32) {
        let tick = self.tick;
        let p = self.partner(g);
        let gt_row = *c.row(l.gt);
        let mut speed = clamp(6 - gt_row.walk, 3, if self.g[g].fatigue < 0xa0 { 5 } else { 3 });
        if g as i32 == self.gary || p as i32 == self.gary || self.g[g].flags & flag::HURRIED != 0 || self.game & game::TOURNAMENT != 0 {
            speed += 1;
        }
        let fi = (self.g[g].facing & 7) as usize;
        let here_path = flat(l.ga, l.gb).map(|i| c.flags[i] & f::PATH != 0).unwrap_or(false);
        if here_path || l.old_flags & flag::CART != 0 {
            let ahead = flat(l.ga + DX[fi], l.gb + DY[fi]).map(|i| c.flags[i] & f::PATH != 0).unwrap_or(false);
            let behind = flat(l.ga - DX[fi], l.gb - DY[fi]).map(|i| c.flags[i] & f::PATH != 0).unwrap_or(false);
            if ahead || behind {
                speed = 6;
                if self.g[g].flags & flag::MAY_CART != 0 {
                    let limit = if self.g[g].strokes == 0 && l.old_flags & flag::CART != 0 { -1 } else { 0 };
                    if limit < c.h(l.gt) {
                        self.g[g].flags |= flag::CART;
                    }
                }
            }
        }
        if ((tick >> 4) & 3) as usize == (g & 3) {
            speed += 1;
        }
        if self.g[g].flags & flag::MAY_CART != 0 && self.g[g].strokes < 2 && self.g[p].strokes < 2 {
            self.g[g].flags |= flag::CART;
        }
        if l.gt == t::GREEN || l.gt == t::TEE || dist < 0x400 {
            self.g[g].flags &= !flag::CART;
        }
        if self.g[g].flags & flag::CART != 0 {
            speed = c.oper[11] * 4 + 8;
            if self.g[g].fatigue > 0x28 {
                self.g[g].fatigue = 0x28;
            }
            if l.old_flags & flag::CART == 0 {
                let at = (self.g[g].x, self.g[g].y);
                self.sound(0x31, Some(at));
            }
        }
        if self.game & game::SLOW != 0 {
            let t3 = ((speed as i64 * 0x5555_5555) >> 32) as i32 - speed;
            speed += t3 / 2;
        }
        let stepn = clamp(speed, 0, (dist * 7) / 128);
        let div = if self.difficulty == 0 {
            if fi & 1 == 1 {
                10
            } else {
                7
            }
        } else if fi & 1 == 1 {
            8
        } else {
            5
        };
        let units = ((self.turbo as i32 + 1) * 0x40 * stepn) / div;
        if units < 0x1e {
            self.g[g].flags |= flag::SLOW_STEP;
        } else {
            self.g[g].flags &= !flag::SLOW_STEP;
        }
        self.g[g].x += DX[fi] * units;
        self.g[g].y += DY[fi] * units;
        self.g[g].walk_left -= units;
        let gi = flat(l.ga, l.gb);
        if gi.map(|i| c.ty[i]) != Some(t::WATER) && c.tile_type(self.g[g].x, self.g[g].y) == t::WATER {
            self.g[g].walk_left = 0;
        }
        if (tick as i32).wrapping_add(g as i32 * 0xb) % 0x18 == 0 && self.g[g].hole != 19 {
            let old = self.g[g].fatigue;
            let add = if here_path { 1 } else { gt_row.walk };
            self.g[g].fatigue += add;
            self.g[g].fatigue += self.g[g].hole / 6;
            let fnew = self.g[g].fatigue;
            if fnew > 0x9f && fnew / 0x28 != old / 0x28 {
                if self.g[g].vip() == 0x20 {
                    self.g[g].fatigue = 0xa0;
                } else {
                    let arg = (self.g[g].field_ae == 3) as i32;
                    self.event(c, rng, g, 0x1a, arg);
                }
            }
        }
        if self.game & game::DEBUG_TELEPORT != 0 {
            self.g[g].x = target.0;
            self.g[g].y = target.1;
            self.g[g].walk_left = 0;
        }
        let a = self.g[g].anim;
        if a < 7 || a + 1 > 10 {
            self.g[g].anim = 7;
        } else {
            self.g[g].anim = a + 1;
        }
    }

    /// The path search as the golfers use it (0x42e7e0): a flood from the target over the walking costs, paths preferred,
    /// water and out of bounds avoided; returns the heading of the cheapest neighbour, -1 when there is none.
    pub fn path_dir(&mut self, c: &Course, rng: &mut ExeRng, g: usize, to: (i32, i32), from: (i32, i32), staff_mode: bool) -> i32 {
        let direct = cdir(to.0 - from.0, to.1 - from.1);
        let cl = |v: i32| clamp(v >> 10, 0, 0x31);
        let (ta, tb) = (cl(to.0), cl(to.1));
        let (fa, fb) = (cl(from.0), cl(from.1));
        let pg = &self.g[g ^ 1];
        let mut avoid = (cl(pg.x), cl(pg.y));
        if avoid == (ta, tb) {
            avoid = (-1, -1);
        }
        if (ta, tb) == (fa, fb) {
            return direct;
        }
        let (va, vb) = (ta - fa, tb - fb);
        let span = len(va, vb);
        let b5 = (((((angle(va, vb) as i32) >> 28) + 1) ^ -7) >> 1) as u8;
        let prefer: u32 = (1 << (((b5 & 7) + 1) & 6)) | (1 << (b5 & 6));
        let mut cost = vec![0u8; NN];
        let mut qa = [0i32; 1024];
        let mut qb = [0i32; 1024];
        let (mut head, mut tail) = (0usize, 1usize);
        qa[0] = ta;
        qb[0] = tb;
        cost[idx(ta, tb)] = 1;
        let staff_rule = staff_mode || self.g[g].hole == 19;
        let mut limit: i32 = if staff_mode { 0xa0 + 0x5a } else { 0xa0 };
        let path_rule = (((!self.tick & 0x40) | 0x80) >> 6) as i32;
        loop {
            let (a, b) = (qa[head], qb[head]);
            head = (head + 1) & 0x3ff;
            let here = idx(a, b);
            let cnow = cost[here] as i32;
            if cnow <= limit {
                for k in 0..8 {
                    let (na, nb) = (a + DX[k], b + DY[k]);
                    if !(0..50).contains(&na) || !(0..50).contains(&nb) {
                        continue;
                    }
                    let ni = idx(na, nb);
                    let old = cost[ni] as i32;
                    let mut stepc = c.walk[ni] + (k as i32 & 1);
                    if (na, nb) == avoid {
                        stepc += 2;
                    }
                    if k & 1 == 0 && path_rule < span && c.flags[ni] & 0x420 == 0x20 && c.flags[here] & f::PATH != 0 {
                        let away = along_axis(na - ta, nb - tb);
                        stepc = (k as i32 != away) as i32;
                    }
                    if staff_rule {
                        let ty = c.ty[ni];
                        if ty == 1 || ty == 2 || ty == 0 || c.row(ty).class == 7 {
                            stepc += 1;
                        } else {
                            stepc = (stepc + 1) >> 1;
                        }
                    }
                    if (c.ty[ni] == t::WATER || c.ty[ni] == t::OUT) && c.flags[ni] & f::PATH == 0 && cnow < 0x40 {
                        stepc += 0x10;
                    }
                    let total = stepc + cnow;
                    if (old == 0 || total < old) && total < 0x100 {
                        cost[ni] = total as u8;
                        qa[tail] = na;
                        qb[tail] = nb;
                        tail = (tail + 1) & 0x3ff;
                        if (na, nb) == (fa, fb) {
                            limit = total;
                        }
                    }
                }
            }
            if head == tail {
                break;
            }
        }
        let here = idx(fa, fb);
        let mut best = cost[here] as i32 * 2 + 1;
        let mut out: i32 = -1;
        let r = rng.below(8);
        let bridge = c.ty[here] == t::WATER && c.flags[here] & f::PATH != 0;
        for i in 0..8 {
            let k = ((i + r) & 7) as usize;
            let (na, nb) = (fa + DX[k], fb + DY[k]);
            if !(0..50).contains(&na) || !(0..50).contains(&nb) || c.oob(na, nb) || cost[idx(na, nb)] == 0 {
                continue;
            }
            let mut v = (k as i32 & 1) + cost[idx(na, nb)] as i32 * 2;
            if bridge && k & 1 == 1 {
                continue;
            }
            if prefer & (1 << (k ^ 4)) != 0 {
                v -= 1;
            }
            if v < best {
                out = k as i32;
                best = v;
            }
        }
        let mut res = out;
        if !staff_mode && bridge && (0..7).contains(&out) {
            let gx = self.g[g].x - fa * 0x400 - 0x200;
            let gy = self.g[g].y - fb * 0x400 - 0x200;
            let turn = match out {
                0 if gx.abs() >= 0x101 => Some(if gx < 1 { 1 } else { 7 }),
                2 if gy.abs() >= 0x101 => Some(if gy < 1 { 3 } else { 1 }),
                4 if gx.abs() >= 0x101 => Some(if gx < 1 { 3 } else { 5 }),
                6 if gy.abs() >= 0x101 => Some(if gy < 1 { 5 } else { 7 }),
                _ => None,
            };
            if let Some(tn) = turn {
                res = tn & 7;
                if res != out {
                    self.game |= game::BRIDGE;
                }
            }
        }
        if res == -1 && (ta, tb) != (fa, fb) {
            res = cdir(va, vb);
        }
        res
    }
}

/// Heading 0, 2, 4 or 6 along the longer axis of a vector (0x42e7a0).
fn along_axis(x: i32, y: i32) -> i32 {
    if y.abs() < x.abs() {
        if x < 1 {
            6
        } else {
            2
        }
    } else if y < 1 {
        0
    } else {
        4
    }
}

impl Club {
    /// Tile type at a flat index, as the exe reads neighbours (outside the table: out of bounds).
    fn ty_flat(c: &Course, a: i32, b: i32) -> u8 {
        flat(a, b).map(|i| c.ty[i]).unwrap_or(t::OUT)
    }

    /// The shot branch (LAB_0042bc8f): planning, waiting for the way to clear, the swing.
    fn shot(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, l: &Locals) {
        let p = self.partner(g);
        if self.g[g].speed == 0 && l.closer {
            self.g[g].pause = -rng.below(8);
            self.g[g].sub = 0;
            self.g[g].anim = anim::STAND;
            return;
        }
        if self.g[g].speed == 0 {
            self.g[g].flags &= !(flag::ASIDE | flag::IMPATIENT);
            let ready = self.g[p].hole == 0 || 7 - (self.g[g].strokes != 0) as i32 <= self.g[p].anim;
            if !ready {
                let pg = &self.g[p];
                let a = angle(pg.x - self.g[g].x, pg.y - self.g[g].y);
                self.g[g].facing = dir8(a);
                self.g[g].anim = anim::STAND;
                self.g[g].walk_left = 0;
                return;
            }
            let h = self.g[g].hole;
            let pin = self.holes[h.clamp(0, 19) as usize].pin;
            let (bx, by) = (self.g[g].bx, self.g[g].by);
            if self.g[g].vip() == 0x20 || len(bx - pin.0 * 0x400 - 0x200, by - pin.1 * 0x400 - 0x200) > 0xff {
                let user;
                if self.g[g].flags & flag::GARY == 0 || l.bt == t::GREEN || tdist(bx, by, pin.0, pin.1) < 0x19 {
                    if self.plan_busy {
                        return;
                    }
                    user = false;
                } else {
                    if self.g[g].aim_a == 0 {
                        self.g[g].aim_a = -1;
                        self.planner.option = 0;
                        self.aim_lock = 10;
                        if h == 1 && self.g[p].vip() == 0x20 && self.g[g].strokes == 0 && self.game & game::TOURNAMENT == 0 {
                            self.sound(0x2a, None);
                            self.message(format!("{} challenges {} to a match!", self.name(p), self.name(g)));
                        }
                    }
                    if self.g[g].aim_a == -1 {
                        self.gary = g as i32;
                        self.g[g].anim = anim::STAND;
                        return;
                    }
                    user = true;
                }
                crate::planner::plan_shot(self, c, rng, g, user, -1, 0, 0);
                let gg = &mut self.g[g];
                gg.facing = dir8(gg.heading);
                if l.bt == t::GREEN {
                    self.holes[h.clamp(0, 19) as usize].putts += 1;
                    let cup = flat(l.ba, l.bb).map(|i| c.flags[i] & f::CUP != 0).unwrap_or(false);
                    if !cup {
                        self.g[g].pause = -25;
                        self.g[g].frame = 0;
                    }
                }
                let gg = &mut self.g[g];
                gg.ox = gg.bx;
                gg.oy = gg.by;
                if gg.anim != anim::POINT {
                    gg.anim = anim::STAND;
                }
                gg.sub = 1;
            } else {
                // A gimme: the ball is within a quarter tile of the cup.
                self.g[g].strokes += 1;
                self.holes[h.clamp(0, 19) as usize].putts += 1;
                self.hole_end(c, rng, g);
            }
            return;
        }
        match self.g[g].sub {
            0 => self.ball(c, rng, g, l),
            1 => {
                if self.g[g].anim == anim::ADDRESS {
                    return;
                }
                self.g[g].anim = anim::STAND;
                let gg = &self.g[g];
                if (gg.strokes != 0 || g & 1 == 0 || self.g[p].pause < 0) && !self.turbo {
                    let n = if l.bt == t::GREEN { 0 } else { gg.hole + 4 };
                    if rng.below(n) != 0 && gg.flags & 0x4200 == 0 {
                        return;
                    }
                }
                let mut clear = true;
                self.g[g].facing = dir8(self.g[g].heading);
                let me = self.g[g].clone();
                let hr = &self.holes[me.hole.clamp(0, 19) as usize];
                let back = hr.back;
                for o in 0..SLOTS {
                    let og = &self.g[o];
                    if !(og.hole == me.hole
                        && og.tee_order <= me.tee_order
                        && o != p
                        && o != g
                        && og.flags & flag::TEED_OFF != 0
                        && me.strokes < og.strokes)
                    {
                        continue;
                    }
                    if tdist(og.x, og.y, back.0, back.1) > 99 {
                        if tdist(og.bx, og.by, me.aim_a, me.aim_b) < 0x25 {
                            clear = false;
                        }
                        if tdist(og.x, og.y, me.aim_a, me.aim_b) > 0x24 {
                            if clear && tdist(og.x, og.y, back.0, back.1) < tdist(me.x, me.y, me.aim_a, me.aim_b) {
                                let a1 = angle(og.x - me.x, og.y - me.y);
                                let a2 = angle(me.aim_a * 0x400 - me.x + 0x200, me.aim_b * 0x400 - me.y + 0x200);
                                if adiff(a1, a2).unsigned_abs() < 0x0aaa_aaaa {
                                    clear = false;
                                }
                            }
                            continue;
                        }
                    }
                    clear = false;
                }
                if me.flags & flag::GARY != 0 || clear {
                    let gg = &mut self.g[g];
                    gg.anim = anim::ADDRESS;
                    gg.frame = 0;
                    gg.x = gg.bx;
                    gg.y = gg.by;
                    gg.flags &= !flag::BAD_LIE;
                } else {
                    self.g[g].anim = anim::STAND;
                    self.g[g].walk_left = 0;
                    self.g[g].pause = -rng.below(4);
                }
            }
            _ => {
                if self.g[g].sub == 2 {
                    if self.g[g].club != 13 {
                        self.g[g].facing = (self.g[g].facing + 2) & 7;
                    }
                    self.g[g].flags |= flag::ASIDE;
                    self.g[g].frame = 0;
                }
                self.g[g].sub += 1;
                self.g[g].anim = clamp(self.g[g].sub - 3, 0, 6);
                if self.g[g].sub > 0x78 {
                    self.g[g].sub = 0x78;
                }
                if self.g[g].anim == 5 {
                    let cls = self.g[g].class;
                    let mut snd = 0xbe;
                    if cls & 1 == 0 {
                        snd = 0xc1;
                    }
                    if cls & 2 == 0 {
                        snd = 0xbf;
                    }
                    if cls & 4 == 0 {
                        snd = 0xc0;
                    }
                    if self.g[g].flags & flag::SWEET != 0 {
                        snd = 0xbd;
                        self.event(c, rng, g, 0x28, 0);
                        self.g[g].timer = 3;
                    }
                    if self.g[g].club > 9 {
                        snd = 2;
                    }
                    if l.bt == t::GREEN {
                        snd = 3;
                    }
                    let at = (self.g[g].x, self.g[g].y);
                    if self.g[g].flags & flag::BALL_MOVING == 0 {
                        self.sound(snd, Some(at));
                    }
                    self.g[g].flags |= flag::BALL_MOVING;
                    if self.g[g].class & 0xf0 != 0 && l.bt != t::GREEN && g as i32 == self.gary {
                        self.sound(0x2e, Some(at));
                    }
                    self.shot_start[g] = self.tick;
                }
                if self.g[g].anim > 4 || self.g[g].flags & flag::BALL_MOVING != 0 {
                    if self.g[g].sub == 0x20 && self.g[g].flags & flag::HURRIED != 0 {
                        self.g[g].flags |= flag::TEED_OFF;
                        self.g[g].sub = 0;
                    }
                    if self.g[g].sub == 0 {
                        let gg = &mut self.g[g];
                        let fac = gg.facing & 7;
                        let d = ((fac + 2) & 7) as usize;
                        let k = (!fac & 1) | 2;
                        let m = if l.bt == t::GREEN { 0x40 } else { 0x80 };
                        gg.x = gg.ox - DX[d] * k * m;
                        gg.y = gg.oy - DY[d] * k * m;
                    }
                    self.ball(c, rng, g, l);
                }
            }
        }
    }

    /// Does a ball at height z on tile (a, b) of type ty hit the tree or building there (0x4070b0)?
    pub(crate) fn tree_hit(c: &Course, rng: &mut ExeRng, ty: u8, z: i32, a: i32, b: i32) -> bool {
        let (mut lo, mut hi) = (ty as i32, ty as i32);
        if ty == 0x16 || ty == 0x15 {
            lo = 0;
            hi = 200;
            let low = flat(a, b).map(|i| (c.flags[i] & 0x1f) as i32).unwrap_or(0);
            if low - 1 < 4 {
                hi = 0;
            }
        }
        let r = |rng: &mut ExeRng, n: i32| rng.below(n);
        match (c.theme, ty) {
            (0 | 1, 13) | (2, 14) => {
                hi = r(rng, 100) + 400;
                lo = 0x32;
            }
            (0 | 1, 14) | (2, 13) => {
                hi = r(rng, 100) + 100;
                lo = 0x14;
            }
            (0 | 2, 15) => {
                hi = r(rng, 100) + 100;
                lo = 0x32;
            }
            (1, 15) => {
                hi = r(rng, 100) + 300;
                lo = 100;
            }
            (0..=2, 16) => {
                hi = r(rng, 200) + 400;
                lo = 0x4b;
            }
            (3, 13..=16) => {
                hi = r(rng, 100) + 400;
                lo = 0x14;
            }
            _ => {}
        }
        z < hi && lo < z
    }

    /// The ball in motion (LAB_0042c661): flight, bounce, roll, the cup, walls, trees, and where it stops.
    fn ball(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, l: &Locals) {
        if self.g[g].speed == 0 {
            return;
        }
        let tick = self.tick;
        if self.shot_start[g] == 0 {
            self.shot_start[g] = tick;
        }
        let (a, b, bt) = (l.ba, l.bb, l.bt);
        let gg = &mut self.g[g];
        let h0 = c.ground(gg.bx, gg.by);
        let s = gg.speed / 16;
        let dx = crate::geom::sinr(gg.heading, s);
        let dy = crate::geom::cosr(gg.heading, s);
        gg.bx += dx;
        gg.by -= dy;
        gg.bz += gg.vz / 32;
        if gg.bz != 0 || gg.vz != 0 {
            gg.vz -= 0x40;
        }
        let ux = (gg.bx >> 6) & 0xf;
        let uy = (gg.by >> 6) & 0xf;
        let mut edge = 0;
        if ux < 2 && Self::ty_flat(c, a - 1, b) != bt {
            edge = 8;
        }
        if uy < 2 && Self::ty_flat(c, a, b - 1) != bt {
            edge |= 1;
        }
        if ux > 0xd && Self::ty_flat(c, a + 1, b) != bt {
            edge |= 2;
        }
        if uy > 0xd && Self::ty_flat(c, a, b + 1) != bt {
            edge |= 4;
        }
        let d8 = dir8(gg.heading);
        let mut ravine = false;
        let row = *c.row(bt);
        let pin = self.holes[gg.hole.clamp(0, 19) as usize].pin;
        if gg.bz < 2 {
            let mut fr = clamp(row.roll - c.rise(gg.bx, gg.by, d8), 0, 99);
            let mut side = c.rise(gg.bx, gg.by, (d8 + 2) & 7);
            if fr < 2 && edge != 0 {
                fr = 2;
            }
            if Self::ty_flat(c, gg.ox >> 10, gg.oy >> 10) == t::GREEN {
                fr = row.roll;
                side = 0;
            }
            if fr > 4 && tick.wrapping_sub(self.shot_start[g]) as i32 > 0x80 {
                fr = 4;
            }
            gg.heading = gg.heading.wrapping_sub((side.wrapping_mul(0x400_0000) / 2) as u32);
            if fr < 5 {
                gg.speed -= (gg.speed >> fr) / 2;
            } else {
                gg.speed += (0x40 - (gg.speed >> 5)) / 2;
            }
            if Self::ty_flat(c, a, b) == t::GREEN {
                gg.heading = gg.heading.wrapping_add((gg.curve / 2) as u32);
                if tick & 7 == 0 && rng.below(8) == 0 {
                    gg.curve = -gg.curve;
                }
            }
            if bt == t::WATER {
                if edge == 0 {
                    gg.speed /= 2;
                }
            } else if bt == t::RAVINE {
                if (8 - ux).abs() < 3 {
                    ravine = (8 - uy).abs() < 3;
                    if uy < 8 && Self::ty_flat(c, a, b - 1) == t::RAVINE {
                        ravine = true;
                    }
                    if uy > 8 && Self::ty_flat(c, a, b + 1) == t::RAVINE {
                        ravine = true;
                    }
                }
                if (8 - uy).abs() < 3 {
                    if ux < 8 && Self::ty_flat(c, a - 1, b) == t::RAVINE {
                        ravine = true;
                    }
                    if ux > 8 && Self::ty_flat(c, a + 1, b) == t::RAVINE {
                        ravine = true;
                    }
                }
                if ravine {
                    gg.speed /= 2;
                }
            }
            let tournament = self.game & game::TOURNAMENT != 0;
            let cup_r = (0x400 / if gg.club != 13 { 3 } else { 1 }) / if tournament { 30 } else { 20 };
            if !c.oob(a, b)
                && c.flags[idx(a, b)] & f::CUP != 0
                && gg.speed < 0x140
                && len(gg.bx - a * 0x400 - 0x200, gg.by - b * 0x400 - 0x200) < cup_r
            {
                let at = (gg.bx, gg.by);
                gg.bx = a * 0x400 + 0x200;
                gg.by = b * 0x400 + 0x200;
                gg.strokes += 1;
                self.sound(4, Some(at));
                self.hole_end(c, rng, g);
                self.g[g].speed = 0;
                self.g[g].flags &= !flag::BALL_MOVING;
                return;
            }
            let w = flat(a, b).map(|i| c.walls[i]).unwrap_or(0);
            let gg = &mut self.g[g];
            let at = (gg.bx, gg.by);
            if a != gg.bx >> 10 && w & if dx < 1 { 0x40 } else { 4 } != 0 {
                gg.heading = gg.heading.wrapping_neg();
                self.sound(6, Some(at));
            }
            let gg = &mut self.g[g];
            if b != gg.by >> 10 && w & if dy < 1 { 0x10 } else { 1 } != 0 {
                gg.heading = 0x8000_0000u32.wrapping_sub(gg.heading);
                self.sound(6, Some(at));
            }
        } else {
            gg.bz += h0 - c.ground(gg.bx, gg.by);
            gg.speed -= (gg.speed >> 4) / 2;
            gg.heading = gg.heading.wrapping_add((gg.curve / 2) as u32);
            let z = gg.bz;
            if Self::tree_hit(c, rng, bt, z, a, b) && self.g[g].flags & flag::HIT_TREE == 0 {
                let gg = &self.g[g];
                let mut dd = len(gg.bx - a * 0x400 - 0x200, gg.by - b * 0x400 - 0x200);
                if gg.kind != 0 && gg.skill_mask & 0x200 != 0 {
                    dd += (gg.skills[9] as i32 * dd) / 4;
                }
                if dd < rng.below(0x180) {
                    let turn = (rng.below(0x80) + 0x40) as u32;
                    self.g[g].heading = self.g[g].heading.wrapping_add(turn << 24);
                    let sp = self.g[g].speed;
                    self.g[g].speed -= rng.below(sp);
                    let snd = rng.below(3) + 6;
                    let at = (self.g[g].bx, self.g[g].by);
                    self.sound(snd, Some(at));
                    if self.g[g].flags & flag::HIT_TREE == 0 {
                        self.event(c, rng, g, 0xc, bt as i32);
                    }
                    self.g[g].flags |= flag::HIT_TREE;
                }
            }
        }
        let gg = &self.g[g];
        if gg.bz < 1 && gg.vz < 0 {
            let mut bf = row.bounce;
            if bf < 2 && edge != 0 {
                bf = 2;
            }
            let at = (gg.bx, gg.by);
            if gg.vz < -0x100 {
                self.sound(if bt == t::SAND { 0x37 } else { 0x36 }, Some(at));
            }
            let gg = &mut self.g[g];
            let mut vz = clamp(-0x40 - (gg.vz * bf) / 0xc, 0, 9999);
            if vz < 0x80 {
                vz = 0;
            }
            gg.vz = vz;
            gg.bz = 0;
            if gg.flags & flag::TO_SHOP != 0 {
                gg.heading ^= 0x8000_0000;
                gg.flags &= !flag::TO_SHOP;
            }
            if gg.flags & flag::BACKSPIN != 0 {
                gg.speed /= 2;
                gg.flags = (gg.flags & !flag::BACKSPIN) | flag::TO_SHOP;
            }
            let dp = (((((gg.heading as i32) >> 29) + 1) & !1) + 2) & 7;
            let r1 = c.rise(gg.bx, gg.by, dp);
            gg.heading = gg.heading.wrapping_sub((r1.wrapping_mul(gg.vz).wrapping_mul(0xaec33)) as u32);
            let s3 = clamp(c.rise(gg.bx, gg.by, d8), -2, 2);
            gg.speed += s3 * gg.vz * -2;
            gg.vz += (s3 * gg.vz) / 2;
            let h = c.h(bt);
            if h > 0 {
                let k = clamp(h, 1, 3);
                let r = rng.below(k * 0xaaaa) as u32;
                let gg = &mut self.g[g];
                gg.heading = gg.heading.wrapping_add(r.wrapping_sub(((k * 0x0aaa_aaaa) / 2) as u32));
            }
            let gg = &mut self.g[g];
            if c.tile_type(gg.bx, gg.by) == t::WATER && edge == 0 {
                gg.vz = 0;
                gg.speed = 0;
            }
            let mut lucky = true;
            if bt == t::ROCKS {
                lucky = false;
                if self.g[g].speed > 0x100 {
                    if edge == 0 {
                        let gg = &self.g[g];
                        let turn = if gg.kind == 0 || gg.skill_mask & 0x200 == 0 { 0x50 - rng.below(0xa0) } else { 0x28 - rng.below(0x50) };
                        self.g[g].heading = self.g[g].heading.wrapping_add((turn as u32) << 24);
                        let snd = rng.below(3) + 6;
                        let at = (self.g[g].bx, self.g[g].by);
                        self.sound(snd, Some(at));
                    }
                    lucky = true;
                }
            }
            if lucky {
                let gg = &self.g[g];
                if gg.speed > 0x100
                    && gg.kind != 0
                    && rng.below(100) < gg.skills[9] as i32
                    && tdist(gg.bx, gg.by, pin.0, pin.1) < 0x4b
                    && c.h(bt) < 1
                {
                    let at = (gg.bx, gg.by);
                    let a2 = angle(pin.0 * 0x400 - gg.bx + 0x200, pin.1 * 0x400 - gg.by + 0x200);
                    let dh = adiff(a2, gg.heading).wrapping_mul(4) / 5;
                    self.sound(0x18, Some(at));
                    self.g[g].heading = self.g[g].heading.wrapping_add(dh as u32);
                    self.event(c, rng, g, 0x2a, 0x14);
                }
            }
            if self.g[g].vz > 200 {
                let my = self.g[g].hole;
                let (bx, by) = (self.g[g].bx, self.g[g].by);
                let r = ((self.difficulty + 2) * 0x400) / 2;
                for o in 0..SLOTS {
                    let og = &self.g[o];
                    if og.hole != 0 && og.bx != 0 && og.hole != my && og.hole != 19 && len(bx - og.x, by - og.y) < r {
                        self.event(c, rng, o, 9, 0x14);
                    }
                }
            }
        }
        let gg = &self.g[g];
        if gg.speed < 0x40 && gg.bz == 0 && gg.vz == 0 {
            self.rest(c, rng, g, ravine);
        }
    }

    /// The ball has stopped: statistics, the golfer's reaction, penalties and drops.
    fn rest(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, ravine: bool) {
        let gg = &mut self.g[g];
        gg.speed = 0;
        gg.pause = 0;
        gg.sub = 0;
        gg.flags &= !flag::BALL_MOVING;
        let (ra, rb) = (gg.bx >> 10, gg.by >> 10);
        let rt = Self::ty_flat(c, ra, rb);
        if rt != t::FAIRWAY {
            gg.flags &= !flag::WASHED;
        }
        let h = gg.hole.clamp(0, 19) as usize;
        if gg.strokes == 0 {
            let dl = len(gg.bx - gg.ox, gg.by - gg.oy) * 25 / 1024;
            let hr = &mut self.holes[h];
            hr.tee_shots += 1;
            hr.drive_sum += dl;
            if hr.drive_max < dl {
                hr.drive_max = dl;
            }
            if c.h(rt) < 1 {
                hr.fairways += 1;
            }
        }
        self.shot_review(c, rng, g);
        let gg = &mut self.g[g];
        gg.strokes += 1;
        if gg.strokes == self.holes[h].par - 2 && rt == t::GREEN {
            self.holes[h].gir += 1;
        }
        if !c.oob(ra, rb) {
            let i = idx(ra, rb);
            if c.landing[i] != -1 {
                c.landing[i] = c.landing[i].wrapping_add(1);
            }
        }
        let first_thought = self.g[g].thoughts[0];
        let pin = self.holes[h].pin;
        let hz = c.h(rt);
        let mut penalty = false;
        if hz < 1 && !c.oob(ra, rb) {
            let gg = &self.g[g];
            if gg.rating > 8 && len(ra - pin.0, rb - pin.1) < len((gg.ox >> 10) - pin.0, (gg.oy >> 10) - pin.1) {
                let gg = &mut self.g[g];
                gg.anim = anim::HAPPY;
                gg.pause = -99;
                gg.facing = (gg.facing - 2) & 7;
                self.event(c, rng, g, 1, rt as i32);
            }
        } else {
            let scenic = flat(ra, rb).map(|i| c.flags[i] & f::SCENIC != 0).unwrap_or(false);
            if self.g[g].rating < self.difficulty + 3 && self.g[g].flags & flag::HIT_TREE == 0 && !scenic {
                let gg = &mut self.g[g];
                gg.anim = anim::UPSET;
                gg.pause = -99;
                gg.facing = (gg.facing - 2) & 7;
                self.event(c, rng, g, 2, rt as i32);
            }
            let st = Self::ty_flat(c, self.g[g].ox >> 10, self.g[g].oy >> 10);
            if c.h(st) > 0 && c.h(st) <= hz && !scenic {
                if self.g[g].anim != anim::UPSET {
                    self.g[g].anim = anim::UPSET;
                    self.g[g].facing = (self.g[g].facing - 2) & 7;
                }
                self.g[g].pause = -99;
                self.event(c, rng, g, (rt != st) as u32 + 2, rt as i32);
                self.g[g].flags |= flag::BAD_LIE;
            }
            if rt == t::WATER || (!c.oob(ra, rb) && ravine) || (c.oob(ra, rb) && ravine) {
                penalty = true;
                let at = (self.g[g].bx, self.g[g].y);
                self.sound(5, Some(at));
                if rt == t::WATER {
                    self.wildlife.splash = Some((self.g[g].bx, self.g[g].by, 0));
                }
                self.event(c, rng, g, 0xd, rt as i32);
                let gg = &mut self.g[g];
                let hd = angle(gg.bx - gg.ox, gg.by - gg.oy);
                let lw = len(gg.bx - gg.ox, gg.by - gg.oy);
                let dw = tdist(gg.bx, gg.by, pin.0, pin.1);
                gg.bx = gg.ox;
                gg.by = gg.oy;
                let mut best = -99;
                let mut s = 0;
                while s < lw {
                    let px = crate::geom::sinr(hd, s) + gg.ox;
                    let py = gg.oy - crate::geom::cosr(hd, s);
                    let tp = c.tile_type(px, py);
                    if (s == 0 || dw <= tdist(px, py, pin.0, pin.1)) && tp != t::WATER && tp != t::RAVINE {
                        let sc = s - 4 * c.h(tp);
                        if best < sc {
                            gg.bx = px;
                            gg.by = py;
                            best = sc;
                            if s != 0 {
                                gg.flags |= flag::TEED_OFF;
                            }
                        }
                    }
                    s += 0x200;
                }
            } else if c.oob(ra, rb) {
                penalty = true;
                self.event(c, rng, g, 2, rt as i32);
                let gg = &mut self.g[g];
                gg.bx = gg.ox;
                gg.by = gg.oy;
            }
            if penalty {
                let gg = &mut self.g[g];
                if gg.anim != anim::UPSET {
                    gg.anim = anim::UPSET;
                    gg.facing = (gg.facing - 2) & 7;
                }
                gg.pause = -99;
                gg.flags |= flag::PENALTY;
                if gg.strokes < 8 {
                    gg.strokes += 1;
                }
            }
        }
        if rt != t::WATER && !c.oob(ra, rb) {
            self.g[g].flags = (self.g[g].flags & !flag::PENALTY) | flag::TEED_OFF;
        }
        let gg = &self.g[g];
        if hz < 1 && first_thought == gg.thoughts[0] && gg.hole < 4 {
            let d = len(gg.ox - gg.bx, gg.oy - gg.by);
            if gg.strokes == 1
                && gg.flags & flag::VISITED_RANGE != 0
                && crate::planner::max_range(self, c, g) < d * 25 / 1024
                && gg.hole < 6
            {
                self.event(c, rng, g, 0x33, 0x14);
            }
        }
        if self.g[g].timer == 0 && hz > 0 {
            if self.g[g].flags & flag::TO_RANGE != 0 {
                self.event(c, rng, g, 0x11, 0x14);
            }
            if self.g[g].flags & flag::TO_PUTTING != 0 {
                self.event(c, rng, g, 0x10, 0x14);
            }
        }
    }

    /// Frames per view of a golfer clip (clip id + body).
    pub fn clip_len(&self, clip: i32, look: i32) -> i32 {
        self.clip_frames.get((clip + look).max(0) as usize).copied().unwrap_or(DEFAULT_CLIP).max(1)
    }

    /// The golfer animation pass of the exe's drawing routine: chooses the clip and frame, and holds or ends states (the
    /// swing waits for its frames, celebrations end the pause, the address ends with the swing).
    fn animate(&mut self, rng: &mut ExeRng, g: usize) {
        let h = self.g[g].hole;
        if h == 0 || h == -1 {
            return;
        }
        self.remember_look(g);
        let look = self.look(g);
        let male = self.male(g) == 1;
        let tick = self.tick;
        let a = self.g[g].anim;
        let mut clip: i32;
        let mut fr: i32;
        let gg = &mut self.g[g];
        if a < 7 {
            clip = 0x28;
            if male {
                if gg.class & 2 == 0 {
                    clip = 0x32;
                } else if gg.class & 4 == 0 {
                    clip = 0x37;
                }
            }
            if gg.club > if gg.shot_type != 3 { 6 } else { 8 } {
                clip = 0x50;
            }
            if gg.sub < 3 {
                gg.frame = 0;
            }
            if gg.club == 13 {
                clip = 0x5a;
            }
            let n = self.clip_frames.get((clip + look) as usize).copied().unwrap_or(DEFAULT_CLIP).max(1);
            let gg = &mut self.g[g];
            if a != 6 {
                if gg.flags & flag::ANIM_ALT == 0 {
                    if gg.frame == 3 && rng.below(2) != 0 && gg.club < 4 && gg.flags & flag::SWEET == 0 {
                        gg.flags |= flag::ANIM_ALT;
                        fr = gg.frame as i32;
                    } else {
                        gg.frame = gg.frame.wrapping_add(1);
                        fr = gg.frame as i32;
                    }
                } else {
                    if gg.frame == 0 {
                        gg.flags &= !flag::ANIM_ALT;
                    } else {
                        gg.frame -= 1;
                    }
                    fr = clamp(gg.frame as i32, 0, 0x63);
                }
                if n <= fr {
                    fr = n - 1;
                    gg.frame = fr as u8;
                }
                let t = ((clip != 0x5a) as i32 + 5) * n;
                if fr < t / 8 && a == 4 {
                    gg.sub -= 1;
                }
                if fr < n - 1 && a == 5 {
                    gg.sub -= 1;
                }
            } else {
                let d = len(gg.bx - gg.x, gg.by - gg.y);
                let reach = if gg.flags & 0x60 != 0 { 3 } else { 5 } * 0x400;
                let mut n2 = n;
                let mut done = false;
                if reach < d {
                    clip = 0xaa;
                    gg.frame = gg.frame.wrapping_add(1);
                    fr = gg.frame as i32;
                    let fi = (gg.facing & 7) as usize;
                    let k = if gg.facing & 1 != 0 { 3 } else { 5 };
                    gg.x = gg.ox - DX[fi] * k * 0x40;
                    gg.y = gg.oy - DY[fi] * k * 0x40;
                    if gg.flags & 0x20 != 0 {
                        clip = 0xa0;
                        if fr == 0x14 {
                            fr = 6;
                            gg.frame = 6;
                        }
                    }
                    if gg.flags & 0x40 != 0 {
                        clip = 0x96;
                        if fr == 0x14 {
                            fr = 6;
                            gg.frame = 6;
                        }
                    }
                    n2 = self.clip_frames.get((clip + look) as usize).copied().unwrap_or(DEFAULT_CLIP).max(1);
                    done = fr < n2;
                } else {
                    fr = 0;
                }
                if !done {
                    fr = n2 - 1;
                }
            }
        } else if a < 11 {
            clip = 0;
            if gg.fatigue > 0xa0 {
                clip = 0xd2;
            }
            if gg.flags & flag::CART != 0 {
                clip = 0xe6;
            }
            let n = self.clip_len(clip, look);
            let gg = &mut self.g[g];
            gg.frame = gg.frame.wrapping_add(1);
            if gg.flags & flag::SLOW_STEP == 0 {
                gg.frame = gg.frame.wrapping_add(1);
            }
            fr = gg.frame as i32 % n;
        } else {
            clip = a;
            fr = 0;
            if a == anim::STAND {
                clip = if gg.club == 13 { 100 } else { 10 };
                let n = self.clip_len(clip, look);
                let fidget = self.clip_len(0x14, look);
                let gg = &mut self.g[g];
                gg.frame = gg.frame.wrapping_add(1);
                fr = gg.frame as i32 % n;
                let u = (tick.wrapping_add(g as u32 * 0x25) & 0x7f) as i32;
                if clip == 10 && gg.strokes == 0 && u < fidget {
                    clip = 0x14;
                    fr = u;
                    if gg.pause == 0 {
                        gg.pause = -2;
                    }
                }
                if gg.club == 13 && fr == 0 {
                    gg.pause = 0;
                }
            }
            if a == anim::HAPPY || a == anim::UPSET {
                let alt = self.g[g].flags & flag::BAD_LIE != 0;
                clip = if a == anim::HAPPY {
                    if alt {
                        0xb4
                    } else {
                        0x1e
                    }
                } else if alt {
                    0xc8
                } else {
                    0xbe
                };
                let n = self.clip_len(clip, look);
                let gg = &mut self.g[g];
                let hold = g & 1 == 1 && tick & 1 == 0 && gg.frame != 0 && n / 2 >= gg.frame as i32;
                if hold {
                    fr = gg.frame as i32 % n;
                } else {
                    gg.frame = gg.frame.wrapping_add(1);
                    fr = gg.frame as i32 % n;
                }
                if fr == 0 {
                    if gg.flags & flag::ANIM_ALT == 0 && g & 1 == 0 {
                        gg.flags |= flag::ANIM_ALT;
                        gg.frame = 0;
                        gg.flags ^= flag::BAD_LIE;
                    } else {
                        gg.pause = 0;
                    }
                }
            }
            if a == anim::POINT {
                let n = self.clip_len(0x6e, look);
                clip = 0x6e;
                let gg = &mut self.g[g];
                gg.frame = gg.frame.wrapping_add(1);
                fr = gg.frame as i32 % n;
                if fr == n / 2 {
                    let r = rng.below(4) as u8;
                    gg.frame = gg.frame.wrapping_sub(r * 2);
                }
                if fr == 0 {
                    gg.anim = anim::STAND;
                    gg.pause = 0;
                }
            }
            if a == anim::SITTING {
                let sit = self.clip_len(0x3c, look);
                let sq = self.clip_len(0x46, look);
                clip = 0x46;
                let gg = &mut self.g[g];
                gg.frame = gg.frame.wrapping_add(1);
                fr = gg.frame as i32 % sq;
                if 0xc - sit < gg.pause {
                    gg.clip = 0x3c;
                    clip = 0x3c;
                    fr = 0xb - gg.pause;
                    if fr < 0xc {
                        gg.anim = anim::STAND;
                        gg.x = (gg.x & !0x3ff) + 0x200;
                        gg.y = (gg.y & !0x3ff) + 0x200;
                    }
                }
            }
            if a == anim::SIT_DOWN {
                let n = self.clip_len(0x3c, look);
                clip = 0x3c;
                let gg = &mut self.g[g];
                gg.frame = gg.frame.wrapping_add(1);
                fr = gg.frame as i32 % n;
                if fr == 0 {
                    gg.anim = anim::SITTING;
                    gg.frame = 0;
                }
            }
            if a == anim::WASH {
                let n = self.clip_len(0xdc, look);
                clip = 0xdc;
                let gg = &mut self.g[g];
                gg.frame = gg.frame.wrapping_add(1);
                fr = gg.frame as i32 % n;
            }
            if a == anim::ADDRESS {
                let gg = &self.g[g];
                clip = if gg.club < 7 { 0x78 } else { 0x82 };
                if gg.club == 13 {
                    clip = 0x8c;
                }
                let n = self.clip_frames.get((clip + look) as usize).copied().unwrap_or(DEFAULT_CLIP);
                let gg = &mut self.g[g];
                if n == 0 {
                    clip = 10;
                    gg.sub = 2;
                } else {
                    gg.frame = gg.frame.wrapping_add(1);
                    fr = gg.frame as i32 % n;
                    if fr == n - 1 {
                        gg.sub = 2;
                    }
                }
            }
        }
        let gg = &mut self.g[g];
        if clip != gg.clip as i32 {
            gg.frame = 0;
            gg.clip = clip as u8;
            fr = 0;
            if gg.anim != anim::HAPPY && gg.anim != anim::UPSET {
                gg.flags &= !flag::ANIM_ALT;
            }
        }
        self.drawn[g] = (clip + look, fr.max(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::land::Object;

    /// A flat one-hole course: fairway with a tee, a green and a clubhouse.
    pub(crate) fn test_course() -> (Course, Club) {
        let mut c = Course::default();
        for a in 0..50 {
            for b in 0..50 {
                let i = idx(a, b);
                c.ty[i] = t::ROUGH;
                c.walk[i] = 2;
                if (18..=32).contains(&b) && (8..=40).contains(&a) {
                    c.ty[i] = t::FAIRWAY;
                    c.walk[i] = 1;
                }
            }
        }
        for a in 9..=11 {
            for b in 24..=26 {
                c.ty[idx(a, b)] = t::TEE;
            }
        }
        for a in 33..=37 {
            for b in 23..=27 {
                c.ty[idx(a, b)] = t::GREEN;
            }
        }
        c.flags[idx(35, 25)] |= f::CUP | 1;
        c.objects.push(Object { kind: 15, a: 3, b: 22, dir: 0, flags: 0x40, sub: 0, val: 0 });
        for a in 3..7 {
            for b in 22..26 {
                c.ty[idx(a, b)] = t::BUILDING;
            }
        }
        c.door = (3, 22);
        c.refresh();
        let mut club = Club::default();
        club.holes[1] = Hole { par: 4, tee_facing: 2, back: (10, 25), fwd: (10, 25), pin: (35, 25), ..club.holes[1].clone() };
        club.holes[19].back = (3, 22);
        club.holes[19].fwd = (3, 22);
        club.holes[19].pin = (3, 22);
        club.next_hole = 2;
        (c, club)
    }

    #[test]
    fn golfers_play_a_hole() {
        let (mut c, mut club) = test_course();
        let mut rng = ExeRng { state: 12345 };
        club.new_game(&mut rng);
        let mut holed = 0;
        let mut fees = 0;
        let mut strokes = Vec::new();
        for tick in 1..6000u32 {
            for g in club.g.iter_mut().take(SLOTS) {
                g.sx = 400;
                g.sy = 300;
            }
            club.tick(&mut c, &mut rng, tick);
            for e in club.out.drain(..) {
                match e {
                    Event::Earn { units, column: Column::GreensFees, .. } => {
                        holed += 1;
                        fees += units;
                    }
                    Event::HoleDone { strokes: s, .. } => strokes.push(s as i8),
                    _ => {}
                }
            }
        }
        println!("holes finished {holed}, fees {fees}, cards {strokes:?}");
        assert!(holed >= 4, "golfers should finish the hole: {holed}");
        assert!(strokes.iter().all(|&s| (2..=10).contains(&s)), "{strokes:?}");
    }
}

#[cfg(test)]
mod soak {
    use super::tests::test_course;
    use super::*;

    /// A long run on the test course at every difficulty: no panics, golfers keep coming and finishing.
    #[test]
    fn long_run() {
        for diff in 0..4 {
            let (mut c, mut club) = test_course();
            club.difficulty = diff;
            let mut rng = ExeRng { state: 777 + diff as u32 };
            club.new_game(&mut rng);
            let mut done = 0;
            for tick in 1..50_000u32 {
                for g in club.g.iter_mut().take(SLOTS) {
                    g.sx = 400;
                }
                club.tick(&mut c, &mut rng, tick);
                done += club.out.drain(..).filter(|e| matches!(e, Event::HoleDone { .. })).count();
            }
            let gone = club.members.iter().filter(|m| m.gone == 0xff).count();
            let invited = club.members.iter().filter(|m| m.level & 7 != 0 && m.gone != 0xff).count();
            let playing = club.g.iter().take(SLOTS).filter(|g| g.hole != 0).count();
            println!("difficulty {diff}: {done} holes, {gone} quit for good, {invited} invited left, {playing} on course");
            // On a bare course the harder settings drive the members away for good, as in the exe; the easier ones keep playing.
            assert!(done > 5, "difficulty {diff}: {done} holes finished");
            if diff < 2 {
                assert!(done > 50, "difficulty {diff}: {done} holes finished");
            } else {
                assert!(invited + gone >= 12, "members are either still invited or gone");
            }
        }
    }
}
