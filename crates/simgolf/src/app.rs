//! The game state and rules that sit above sg-core: the course and its scenery, the golfers on it, money, editing, the in-game
//! panels and the story banner. Drawing lives in render.rs, input and the window in main.rs.
use crate::gfx::{Gfx, Mesh, Vert};
use crate::render::THEME_PACKS;
use crate::ui::{money, Image};
use miniquad::TextureId;
use sg_core::course::Course;
use sg_core::economy::{self, Economy};
use sg_core::fsutil::resolve;
use sg_core::golfer::{self as golf, Club};
use sg_core::holes::*;
use sg_core::land::{self, ExeRng, Land, Noise, Slot};
use sg_core::mixer::Mixer;
use sg_core::properties::{PROPERTIES, START_FUNDS};
use sg_core::shot::GolferSkills;
use sg_core::sprites::{load_sprite, Sprite};
use sg_core::staff::{self, Employee, StaffEvent, StaffGolfer, TileState};
use sg_core::terrain::*;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const THEMES: [&str; 4] = ["Parkland", "Links", "Desert", "Tropical"];
/// Golfer bodies by the exe's body index (0..3 men, 5..8 women), and the clips by the exe's clip id; a sprite id is clip + body.
pub const GOLFER_BODIES: [&str; 9] = [
    "Male/MalePLS",
    "Male/MaleKLS",
    "Male/MalePSS",
    "Male/MaleSSS",
    "",
    "Female/FemalePLS",
    "Female/FemaleSSS",
    "Female/FemalePSS",
    "Female/FemaleSkTT",
];
pub const GOLFER_CLIPS: [(i32, &str); 25] = [
    (0x00, "NormalWalk"),
    (0x0a, "sq"),
    (0x14, "Fidget"),
    (0x1e, "Happy"),
    (0x28, "PerfectSwing"),
    (0x32, "NoAccSwing"),
    (0x37, "NoimagSwing"),
    (0x3c, "Sitting"),
    (0x46, "SitSq"),
    (0x50, "Pitch"),
    (0x5a, "Putt"),
    (0x64, "LineUpPutt"),
    (0x6e, "PointAt"),
    (0x78, "NormalAddress"),
    (0x82, "PitchAddress"),
    (0x8c, "PuttAddress"),
    (0x96, "LeanRight"),
    (0xa0, "LeanLeft"),
    (0xaa, "LookUp"),
    (0xb4, "SuccessA"),
    (0xbe, "Sad"),
    (0xc8, "FailureA"),
    (0xd2, "TiredWalk"),
    (0xdc, "HandShake"),
    (0xe6, "Cart"),
];

/// A sprite plus its lazily created textures (one per view/frame).
pub struct GlSprite {
    pub s: Sprite,
    pub tex: Vec<Option<TextureId>>,
}

/// A placed object. Sprites are billboards anchored on the ground at (x, z).
#[derive(Clone, Debug)]
pub struct Prop {
    pub x: f32,
    pub z: f32,
    pub body: Option<usize>,
    pub shadow: Option<usize>,
    /// Ground overlay (building base), drawn under everything else.
    pub flat: bool,
    /// Frame within a view.
    pub frame: i32,
    /// World heading in degrees (atan2(z, x)); only used by 8 view sprites.
    pub heading: f32,
    /// Index into App::golfers, None for other props.
    pub golfer: Option<usize>,
    /// Index into App::employees.
    pub employee: Option<usize>,
    /// Facing 0..3 of a 4-view object (added to the camera's quarter turn, as the exe does).
    pub facing: i32,
    /// Frames advance with the game tick (landmarks and building animations).
    pub animated: bool,
    /// A weed, rebuilt from the tile flags every update.
    pub weed: bool,
    /// A land object's sprite (building, landmark, obstacle), rebuilt when objects change.
    pub object: bool,
    /// A pool golfer who is not on the course.
    pub hidden: bool,
    /// Scenery regenerated from the terrain after edits.
    pub tree: bool,
    /// A tree on this tile (x, y) that shows its growth counter while the tile grows.
    pub grow_tile: Option<(i32, i32)>,
    /// A flag or tee marker, rebuilt from the hole records every update.
    pub decor: bool,
    /// A celebrity resident (index into Club::residents).
    pub resident: Option<usize>,
    /// Ambient life and water effects (animals, fly-overs, splash, ripples, rocks, dolphins), rebuilt every update.
    pub ambient: bool,
    /// Drawn size (1 = the sprite's own) and height of the body above the ground in world units (the shadow stays down).
    pub scale: f32,
    pub lift: f32,
}

impl Default for Prop {
    fn default() -> Self {
        Prop {
            x: 0.0,
            z: 0.0,
            body: None,
            shadow: None,
            flat: false,
            frame: 0,
            heading: -1000.0,
            employee: None,
            facing: 0,
            animated: false,
            weed: false,
            object: false,
            golfer: None,
            hidden: false,
            tree: false,
            grow_tile: None,
            decor: false,
            resident: None,
            ambient: false,
            scale: 1.0,
            lift: 0.0,
        }
    }
}

/// Per hole statistics for the course report (reset when the course changes shape).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HoleStat {
    pub plays: i32,
    pub strokes: f64,
    pub seconds: f64,
    pub revenue: f64,
    pub mood: f64,
    pub hist: [i32; 6],
    /// Total of the mood changes golfers had on this hole (the exe's per-hole counter).
    pub mood_sum: i32,
}

pub struct PaintEntry {
    pub name: &'static str,
    pub ty: i32,
    pub vbyte: i32,
}

pub const PAINT: [PaintEntry; 20] = [
    PaintEntry { name: "Fairway", ty: 2, vbyte: 0 },
    PaintEntry { name: "Firm fairway", ty: 3, vbyte: 0 },
    PaintEntry { name: "Green", ty: 1, vbyte: 0 },
    PaintEntry { name: "Tee", ty: 0, vbyte: 0 },
    PaintEntry { name: "Rough", ty: 4, vbyte: 0 },
    PaintEntry { name: "Deep rough", ty: 5, vbyte: 0 },
    PaintEntry { name: "Woods", ty: 13, vbyte: 0 },
    PaintEntry { name: "Sand bunker", ty: 7, vbyte: 0 },
    PaintEntry { name: "Pot bunker", ty: 9, vbyte: 0 },
    PaintEntry { name: "Water shallow", ty: 17, vbyte: 0 },
    PaintEntry { name: "Water middle", ty: 17, vbyte: 1 },
    PaintEntry { name: "Water deep", ty: 17, vbyte: 2 },
    PaintEntry { name: "Rock", ty: 12, vbyte: 0 },
    PaintEntry { name: "Brush", ty: 11, vbyte: 0 },
    PaintEntry { name: "Building lot", ty: 22, vbyte: 0 },
    PaintEntry { name: "Cliff (editor id)", ty: 31, vbyte: 0 },
    PaintEntry { name: "Ravine (editor id)", ty: 32, vbyte: 0 },
    PaintEntry { name: "Flower bed (editor id)", ty: 33, vbyte: 0 },
    PaintEntry { name: "Zen sand (editor id)", ty: 34, vbyte: 0 },
    PaintEntry { name: "Grass bunker (editor id)", ty: 35, vbyte: 0 },
];

/// Building kinds the Add Buildings panel offers, in the exe's order (kind numbers of `land::BUILDINGS`): benches, flower beds,
/// ball washers, then the amenities as holes unlock them. Paths have their own tool; landmarks and home sites need donations and
/// members first.
pub const OFFERED_KINDS: [i32; 16] = [1, 2, 3, 16, 19, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Property,
    Play,
    Report,
    /// The exe's TRACTS FOR SALE screen.
    Land,
    /// The pro's skill dialog.
    Skills,
    /// The SGA report or tournament offer, the preparation checklist, the tournament results.
    Sga,
    Prep,
    Results,
    /// Play a Championship: the course and pro choosers.
    Champ,
    /// SELECT THE NEXT PAIR OF GOLFERS, the accomplishments board, the year-end report, the Membership Roster.
    Pair,
    Board,
    YearEnd,
    Roster,
    /// F2 Player Comments, F3 Histograph, F4 Financial Report, F5 Routing Map, F8 Keyboard Shortcuts.
    Comments,
    Histograph,
    Finance,
    Routing,
    Shortcuts,
}

/// A whole game as saved: the land and terrain, the golfers and holes, the staff, the money and calendar, and the exe's random
/// generator, so a loaded game goes on exactly as it would have. Stored as JSON compressed with zlib, after a magic line.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveGame {
    pub version: u32,
    pub theme: usize,
    pub course_name: String,
    pub difficulty: i32,
    pub terrain: Terrain,
    pub land: Option<Land>,
    pub club: Club,
    pub course: Course,
    pub econ: Economy,
    pub exe_rng: ExeRng,
    pub noise: Noise,
    pub employees: Vec<Employee>,
    pub staff_tiles: TileState,
    pub game_tick: u32,
    pub hole_stats: Vec<HoleStat>,
}

const SAVE_MAGIC: &[u8] = b"SIMGOLF-NATIVE-SAVE\n";

/// One texture-batched mesh of the course.
pub struct Batch {
    pub tex: TextureId,
    pub mesh: Mesh,
    pub water: bool,
}

pub struct App {
    pub game_dir: PathBuf,
    pub theme: usize,
    pub seed: u32,
    pub terrain: Terrain,
    pub catalog: TextureCatalog,
    pub light: Lighting,
    /// Loaded textures by file (None when the file could not be decoded).
    pub textures: HashMap<PathBuf, Option<TextureId>>,
    pub batches: Vec<Batch>,
    pub path_batches: Vec<Batch>,
    /// Paths not joined to the clubhouse, drawn as mud tracks (manual p. 18).
    pub mud_batches: Vec<Batch>,
    pub wall_batches: Vec<Batch>,
    pub hole: HoleInfo,
    pub last_report: String,
    pub econ: Economy,
    pub cam_x: f32,
    pub cam_z: f32,
    pub zoom: f32,
    pub rot: f32,
    pub draw_w: f32,
    pub draw_h: f32,
    /// Drawable pixels per window point (2 on Retina).
    pub dpi: f32,
    pub sprites: Vec<GlSprite>,
    pub sprite_index: HashMap<String, Option<usize>>,
    pub props: Vec<Prop>,
    pub show_props: bool,
    /// Seconds, drives animation.
    pub time: f64,
    /// The golfers and the course as they see it (the exe's golfer table and tile arrays).
    pub club: Club,
    pub course: Course,
    /// The open holes (par set), drawn and reported from the golfers' hole records, and their hole numbers.
    pub holes: Vec<HoleRoute>,
    pub hole_numbers: Vec<i32>,
    /// The terrain came from outside the hole tool (demo course, saved terrain, scripted edits): its painted tee and green
    /// pairs become holes at the next course sync.
    pub adopt_holes: bool,
    pub sim_time: f64,
    /// Golfer clips by sprite id (clip + body): body and shadow sprites.
    pub golfer_clips: HashMap<i32, (Option<usize>, Option<usize>)>,
    /// 0..3 as in the exe (the standard game's value is not known yet; 1 is a guess).
    pub difficulty: i32,
    /// The exe's random number generator (land, offer) and the height noise it fills once at start-up.
    pub exe_rng: ExeRng,
    /// A second generator for things only drawn (celebrity residents, wildlife steps, water sparkle), so the club's draws do
    /// not depend on the camera or the frame rate.
    pub deco_rng: ExeRng,
    pub noise: Noise,
    /// This game's deal of properties to offer slots, and the slot each property is in.
    pub offer: [Slot; 16],
    pub slot_of: [usize; 16],
    /// The land of the current game, when it was generated (None for the demo course or a loaded course file).
    pub land: Option<Land>,
    /// Course staff, the tile flags they work on (weeds), what they know of each golfer, and the game tick that drives them.
    pub employees: Vec<Employee>,
    pub staff_tiles: TileState,
    pub staff_golfers: Vec<StaffGolfer>,
    pub game_tick: u32,
    pub tick_acc: f64,
    /// Last ground tile the player clicked (the player's own pro walks there).
    pub clicked_tile: Option<(i32, i32)>,
    /// The tile under the pointer (the pro's employee follows it until a hole exists).
    pub hover_tile: Option<(i32, i32)>,
    /// The pro's skill dialog while open, and the shot preview while he waits for the player's aim.
    pub skill_dialog: Option<crate::pro_ui::SkillDialog>,
    pub aim: Option<sg_core::pro::AimPreview>,
    /// The first 'n' while the pro plays asks; a second one within this game tick cancels the round.
    pub cancel_until: u32,
    /// Scripted runs: 1 the pro aims each full shot at the pin by himself, 2 he only shows that aim (for stills).
    pub auto_aim: u8,
    /// The pro's golfer slot while he plays (to notice the round's end).
    pub pro_slot: i32,
    /// Tournament screens: the SGA report or offer, the preparation checklist with its ticks, the results.
    pub sga: Option<crate::tourney_ui::SgaScreen>,
    pub prep: Option<(sg_core::tournament::Prep, i32)>,
    pub results: Option<sg_core::tournament::Results>,
    pub champ: Option<crate::champ_ui::ChampScreen>,
    /// Tiles the player retyped since the last tick (each may draw wildlife), and the water depth of each tile.
    pub retyped: Vec<(i32, i32)>,
    /// The clubhouse and record screens: their art, the pair screen's picks, the golfers list page, the golfer card open,
    /// the board's notice for the year-end report, the roster's scroll.
    pub art: crate::screens_ui::Art,
    pub pair_picks: Vec<usize>,
    pub golfer_page: usize,
    pub card: Option<usize>,
    pub year_notice: String,
    pub roster_offset: usize,
    /// The accomplishment snapshots still to take (id and map point), and the snapshots taken this session.
    pub snapshot_due: Vec<(usize, (i32, i32))>,
    pub snapshots: HashMap<usize, crate::ui::Image>,
    /// The report screens' art, the routing map's tab and selected hole.
    pub reports: crate::reports_ui::ReportArt,
    pub route_tab: usize,
    pub route_hole: usize,
    /// The property chooser was opened from the course (F6) to move the club.
    pub world_move: bool,
    /// Draw the course only (for snapshots).
    pub no_hud: bool,
    /// Option "show golfer thoughts" (option bit 0x10).
    pub show_thoughts: bool,
    pub water_depth: Vec<u8>,
    /// An employee picked up to be moved: the next click on the course becomes their post (the exe's "Move this employee").
    pub moving_employee: Option<usize>,
    /// Employee clips by sprite set (0 Greeter, 1 Ranger, 2 Groundskeeper, 3 Tray Girl, 4 Golf Celebrity, 5 Marshall,
    /// 6 Lawn Technician, 7 Soda Vendor; 8 the player's pro) and state (walk, stand, action): body and shadow.
    pub staff_clips: [[(Option<usize>, Option<usize>); 3]; 9],
    pub weed_sprite: Option<usize>,
    pub course_name: String,
    pub screen: Screen,
    pub title_base: Image,
    pub title_un: Image,
    pub title_mo: Image,
    pub world_base: Image,
    pub theme_icons: [Image; 4],
    pub report_art: Image,
    pub dock_art: Image,
    /// H toggles the advisor and story banners.
    pub show_advisor: bool,
    /// Open dock panel: 0 none, 1 terrain, 2 buildings, 3 people.
    pub panel: i32,
    /// Dock button under the mouse, -1 none.
    pub dock_hover: i32,
    /// List row under the mouse in an open panel.
    pub panel_hover: i32,
    /// Lines of a story file from the disc (read at run time, never stored in the project). "A|text" or "B|text".
    pub story_lines: Vec<String>,
    pub story_title: String,
    pub story_until: u32,
    pub hole_stats: Vec<HoleStat>,
    pub ui_ok: bool,
    pub view: crate::ui::View,
    /// Button or card under the mouse, -1 none.
    pub hover: i32,
    /// Chosen on the title screen; its text and golfers are not used yet.
    pub theme_pack: usize,
    pub reset_clock: bool,
    /// The property chooser was opened from Sandbox Mode.
    pub sandbox_choice: bool,
    pub toast: String,
    pub toast_until: f64,
    /// Camera follows the golfer.
    pub follow: bool,
    // Course editing
    pub edit: bool,
    /// 0 paint, 1 raise (shift lowers), 2 path (shift removes), 3 wall, 4 building.
    pub tool: i32,
    /// 1 gravel, 2 paved.
    pub path_kind: i32,
    /// Building kind the building tool places (an exe kind, see OFFERED_KINDS).
    pub build_idx: usize,
    /// = selects raising, - selects lowering (the original's hotkeys); shift flips it.
    pub raise_sign: i32,
    pub paused: bool,
    /// Game speed (Space cycles 1, 2, 4): above 1 the exe's fast mode is on as well (golfers walk twice as far a tick).
    pub speed: u8,
    pub paint_idx: usize,
    /// The paint tool's variant (0x5a34f0, drawn when a terrain brush is picked) and the brush it was drawn for.
    pub paint_variant: Option<(usize, i32)>,
    /// The land screen's tracts and the one under the pointer (9 = the cancel button).
    pub tracts: [sg_core::tracts::Tract; 9],
    pub land_hover: i32,
    /// Radius in tiles.
    pub brush: i32,
    pub has_hit: bool,
    /// Ground point under the mouse.
    pub hit_x: f32,
    pub hit_z: f32,
    /// Last tile/corner edited during a drag.
    pub last_cell: i64,
    pub dirty: bool,
    /// Modelview of the last render.
    pub mv: [f32; 16],
    /// World units per drawable pixel (last render).
    pub upp: f32,
    pub course_file: PathBuf,
    pub skills: GolferSkills,
    // Sound
    pub mixer: Option<Arc<Mixer>>,
    pub audio: Option<crate::audio::AudioOut>,
    pub mute: bool,
    pub sound_log: bool,
    pub music_on: bool,
    pub ambience: i32,
    pub music: i32,
    pub music_idx: usize,
    /// Wall clock (seconds) used for toasts and the story timer.
    pub clock: f64,
}

pub fn now() -> f64 {
    miniquad::date::now()
}

impl App {
    pub fn new(game_dir: PathBuf) -> App {
        Self::with_clock(game_dir, clock_ms())
    }

    /// `clock` stands in for the Windows millisecond clock the exe seeds its generator with.
    pub fn with_clock(game_dir: PathBuf, clock: u32) -> App {
        let mut exe_rng = ExeRng::from_clock(clock);
        let noise = Noise::new(&mut exe_rng);
        let offer = land::deal_offer(&mut exe_rng, false);
        let slot_of = slots_of(&offer);
        App {
            game_dir,
            theme: 0,
            seed: 7,
            terrain: Terrain::default(),
            catalog: TextureCatalog::default(),
            light: Lighting::default(),
            textures: HashMap::new(),
            batches: Vec::new(),
            path_batches: Vec::new(),
            mud_batches: Vec::new(),
            wall_batches: Vec::new(),
            hole: HoleInfo::default(),
            last_report: String::new(),
            econ: Economy::default(),
            cam_x: 0.0,
            cam_z: 0.0,
            zoom: 0.36,
            rot: 0.0,
            draw_w: 1024.0,
            draw_h: 768.0,
            dpi: 1.0,
            sprites: Vec::new(),
            sprite_index: HashMap::new(),
            props: Vec::new(),
            show_props: true,
            time: 0.0,
            club: Club::default(),
            course: Course::default(),
            holes: Vec::new(),
            hole_numbers: Vec::new(),
            adopt_holes: true,
            sim_time: 0.0,
            golfer_clips: HashMap::new(),
            difficulty: 1,
            exe_rng,
            deco_rng: ExeRng::from_clock(0x5167),
            noise,
            offer,
            slot_of,
            land: None,
            employees: Vec::new(),
            staff_tiles: TileState::new(0, 0),
            staff_golfers: Vec::new(),
            game_tick: 0,
            tick_acc: 0.0,
            clicked_tile: None,
            hover_tile: None,
            skill_dialog: None,
            aim: None,
            cancel_until: 0,
            auto_aim: 0,
            pro_slot: -1,
            sga: None,
            prep: None,
            results: None,
            champ: None,
            retyped: Vec::new(),
            show_thoughts: true,
            art: Default::default(),
            pair_picks: Vec::new(),
            golfer_page: 0,
            card: None,
            year_notice: String::new(),
            roster_offset: 0,
            snapshot_due: Vec::new(),
            snapshots: HashMap::new(),
            no_hud: false,
            reports: Default::default(),
            route_tab: 0,
            route_hole: 1,
            world_move: false,
            water_depth: Vec::new(),
            moving_employee: None,
            staff_clips: [[(None, None); 3]; 9],
            weed_sprite: None,
            course_name: "Demo Course".into(),
            screen: Screen::Play,
            title_base: Image::default(),
            title_un: Image::default(),
            title_mo: Image::default(),
            world_base: Image::default(),
            theme_icons: [Image::default(); 4],
            report_art: Image::default(),
            dock_art: Image::default(),
            show_advisor: true,
            panel: 0,
            dock_hover: -1,
            panel_hover: -1,
            story_lines: Vec::new(),
            story_title: String::new(),
            story_until: 0,
            hole_stats: Vec::new(),
            ui_ok: false,
            view: crate::ui::View::default(),
            hover: -1,
            theme_pack: 0,
            reset_clock: false,
            sandbox_choice: false,
            toast: String::new(),
            toast_until: 0.0,
            follow: false,
            edit: false,
            tool: 0,
            path_kind: 1,
            build_idx: 7,
            raise_sign: 1,
            paused: false,
            speed: 1,
            paint_idx: 0,
            paint_variant: None,
            tracts: Default::default(),
            land_hover: -1,
            brush: 0,
            has_hit: false,
            hit_x: 0.0,
            hit_z: 0.0,
            last_cell: -1,
            dirty: false,
            mv: [0.0; 16],
            upp: 1.0,
            course_file: PathBuf::from("course.sgc"),
            skills: GolferSkills::default(),
            mixer: None,
            audio: None,
            mute: false,
            sound_log: false,
            music_on: false,
            ambience: -1,
            music: -1,
            music_idx: 0,
            clock: 0.0,
        }
    }

    /// A file in the game folder, matched case-insensitively.
    pub fn game_path(&self, rel: &str) -> PathBuf {
        resolve(&self.game_dir, rel)
    }

    pub fn show_toast(&mut self, msg: &str) {
        self.toast = msg.to_string();
        self.toast_until = self.clock + 3.0;
    }

    // ---- sound ------------------------------------------------------------------------------------------------------------

    pub fn snd(&mut self, rel: &str, vol: f32, looping: bool) -> i32 {
        let Some(m) = &self.mixer else { return -1 };
        if self.mute {
            return -1;
        }
        let id = m.play(rel, vol, looping);
        if self.sound_log {
            println!("  sound: {rel}{}", if id < 0 { "  (missing)" } else { "" });
        }
        id
    }

    /// Plays one of the exe's sound slots (the file each slot holds after the exe's loader, sg_core::sounds). The exe pans and
    /// scales these by screen position; here they play centred.
    pub fn slot_sound(&mut self, slot: i32, _x: f32, _z: f32) {
        if let Some(file) = sg_core::sounds::slot_file(slot) {
            self.snd(file, 0.7, false);
        }
    }

    pub fn start_ambience(&mut self) {
        if let (Some(m), true) = (&self.mixer, self.ambience >= 0) {
            m.stop(self.ambience);
        }
        self.ambience = self.snd("GolfAmbience122.wav", 0.22, true);
    }

    pub fn toggle_music(&mut self) {
        let Some(m) = self.mixer.clone() else { return };
        if self.music >= 0 {
            m.stop(self.music);
        }
        self.music = -1;
        self.music_on = !self.music_on;
        if !self.music_on {
            return;
        }
        const FOLDERS: [&str; 4] = ["music/misc_music/", "music/links_music/", "music/desert_music/", "music/tropical_music/"];
        let tracks = m.list(FOLDERS[self.theme]);
        if tracks.is_empty() {
            return;
        }
        let t = tracks[self.music_idx % tracks.len()].clone();
        self.music_idx += 1;
        self.music = self.snd(&t, 0.35, true);
    }

    // ---- textures, meshes and scenery ---------------------------------------------------------------------------------------

    pub fn texture_for(&mut self, g: &mut Gfx, path: &Path) -> Option<TextureId> {
        if let Some(t) = self.textures.get(path) {
            return *t;
        }
        let tga = sg_core::fsutil::ext_lower(path) == ".tga";
        let id = sg_core::fsutil::read_file(path)
            .and_then(|d| if tga { sg_core::assets::decode_tga(&d).ok() } else { sg_core::assets::decode_bmp(&d).ok() })
            .map(|img| g.texture(&img, true));
        self.textures.insert(path.to_path_buf(), id);
        id
    }

    fn theme_texture(&mut self, g: &mut Gfx, file: &str) -> Option<TextureId> {
        let p = self.game_path(&format!("Data/Textures/{}/{}", THEMES[self.theme], file));
        self.texture_for(g, &p)
    }

    fn to_vert(v: &Vertex) -> Vert {
        Vert { pos: [v.x, v.y, v.z], uv: [v.u, v.v], normal: [v.nx, v.ny, v.nz], color: [1.0; 4] }
    }

    fn upload(g: &mut Gfx, map: BTreeMap<usize, (TextureId, Vec<Vert>, bool)>) -> Vec<Batch> {
        map.into_values().map(|(tex, v, water)| Batch { tex, mesh: g.mesh(&v), water }).collect()
    }

    fn clear_batches(&mut self, g: &mut Gfx) {
        for b in
            self.batches.drain(..).chain(self.path_batches.drain(..)).chain(self.mud_batches.drain(..)).chain(self.wall_batches.drain(..))
        {
            g.delete_mesh(b.mesh);
        }
    }

    pub fn rebuild_batches(&mut self, g: &mut Gfx) {
        self.clear_batches(g);
        self.terrain.desert = self.theme == 2; // the original swaps shallow water for its desert variant in this theme
        let mut map: BTreeMap<usize, (TextureId, Vec<Vert>, bool)> = BTreeMap::new();
        let mut tris = Vec::new();
        let mut order: HashMap<Option<TextureId>, usize> = HashMap::new();
        for y in 0..self.terrain.h {
            for x in 0..self.terrain.w {
                tris.clear();
                // Terrain.dll does not draw out-of-bounds tiles (type 20): the land outside the property is left empty.
                if self.terrain.type_at(x, y) == 20 {
                    continue;
                }
                build_tile_triangles(&self.terrain, x, y, &mut tris);
                for t in &tris {
                    let path = self.catalog.pick(t.tex_type, t.set, t.variation).cloned();
                    let tex = path.and_then(|p| self.texture_for(g, &p));
                    let water =
                        tex.is_some() && (t.tex_type == TT_WATER_SHALLOW as i32 || t.tex_type == TT_MARSH as i32 || is_water(t.tex_type));
                    let next = order.len();
                    let key = *order.entry(tex).or_insert(next);
                    let e = map.entry(key).or_insert_with(|| (tex.unwrap_or(g.white), Vec::new(), water));
                    e.2 |= water;
                    e.1.extend(t.v.iter().map(Self::to_vert));
                }
            }
        }
        self.batches = Self::upload(g, map);
        self.build_paths(g);
        self.build_walls(g);
    }

    /// Path overlay (our own model): every path tile gets a centre piece, plus an arm toward each path neighbour, cut out of the
    /// game's cross shaped Path.tga (arms 0..0.33 / 0.66..1 of the tile, centre between).
    fn build_paths(&mut self, g: &mut Gfx) {
        let t = &self.terrain;
        if t.path_kind.is_empty() {
            return;
        }
        let connected = paths_connected_to_clubhouse(t);
        let (ox, oz) = (-t.w as f32 * TILE_SIZE * 0.5, -t.h as f32 * TILE_SIZE * 0.5);
        struct Piece {
            r: [f32; 4],
            disc: bool,
        }
        let mut jobs: Vec<(bool, &'static str, Vec<Vert>)> = Vec::new();
        for ty in 0..t.h {
            for tx in 0..t.w {
                let kind = t.path_at(tx, ty);
                if kind == 0 {
                    continue;
                }
                let n = t.path_at(tx, ty - 1) != 0;
                let s = t.path_at(tx, ty + 1) != 0;
                let w = t.path_at(tx - 1, ty) != 0;
                let e = t.path_at(tx + 1, ty) != 0;
                let links = n as i32 + s as i32 + w as i32 + e as i32;
                let file = if kind == 2 { "PathX.tga" } else { "Path.tga" };
                let disc_file = if kind == 2 { "PathCurveX.tga" } else { "PathCurve.tga" };
                let (a, b) = (0.33f32, 0.67f32);
                let straight = (n && s && !w && !e) || (w && e && !n && !s);
                let mut pieces = Vec::new();
                if links >= 3 || straight {
                    pieces.push(Piece { r: [a, a, b, b], disc: false });
                } else {
                    pieces.push(Piece { r: [0.22, 0.22, 0.78, 0.78], disc: true });
                }
                if n {
                    pieces.push(Piece { r: [a, 0.0, b, a], disc: false });
                }
                if s {
                    pieces.push(Piece { r: [a, b, b, 1.0], disc: false });
                }
                if w {
                    pieces.push(Piece { r: [0.0, a, a, b], disc: false });
                }
                if e {
                    pieces.push(Piece { r: [b, a, 1.0, b], disc: false });
                }
                let joined = connected[t.tile_index(tx, ty)] != 0;
                let vert = |wx: f32, wz: f32, u: f32, v: f32| {
                    let y = t.height_at(wx, wz) + 1.5;
                    let e = 12.0;
                    let nx = t.height_at(wx - e, wz) - t.height_at(wx + e, wz);
                    let nz = t.height_at(wx, wz - e) - t.height_at(wx, wz + e);
                    let ny = 2.0 * e;
                    let l = (nx * nx + ny * ny + nz * nz).sqrt();
                    let mut vv = Vert::new(wx, y, wz, u, v);
                    vv.normal = [nx / l, ny / l, nz / l];
                    if !joined {
                        vv.color = [0.45, 0.32, 0.22, 1.0]; // unconnected paths: the same artwork in a muddy brown
                    }
                    vv
                };
                for p in &pieces {
                    let mut v = Vec::new();
                    const D: i32 = 2;
                    for j in 0..D {
                        for i in 0..D {
                            let u0 = p.r[0] + (p.r[2] - p.r[0]) * i as f32 / D as f32;
                            let u1 = p.r[0] + (p.r[2] - p.r[0]) * (i + 1) as f32 / D as f32;
                            let v0 = p.r[1] + (p.r[3] - p.r[1]) * j as f32 / D as f32;
                            let v1 = p.r[1] + (p.r[3] - p.r[1]) * (j + 1) as f32 / D as f32;
                            // Disc pieces map the whole texture onto the rectangle.
                            let tu = |u: f32| if p.disc { (u - p.r[0]) / (p.r[2] - p.r[0]) } else { u };
                            let tv = |v: f32| if p.disc { (v - p.r[1]) / (p.r[3] - p.r[1]) } else { v };
                            let q =
                                |u: f32, vv: f32| vert(ox + (tx as f32 + u) * TILE_SIZE, oz + (ty as f32 + vv) * TILE_SIZE, tu(u), tv(vv));
                            let (q00, q10, q11, q01) = (q(u0, v0), q(u1, v0), q(u1, v1), q(u0, v1));
                            v.extend_from_slice(&[q00, q10, q11, q00, q11, q01]);
                        }
                    }
                    jobs.push((joined, if p.disc { disc_file } else { file }, v));
                }
            }
        }
        let mut paths: BTreeMap<usize, (TextureId, Vec<Vert>, bool)> = BTreeMap::new();
        let mut mud: BTreeMap<usize, (TextureId, Vec<Vert>, bool)> = BTreeMap::new();
        let mut order: HashMap<&'static str, usize> = HashMap::new();
        for (joined, file, v) in jobs {
            let Some(tex) = self.theme_texture(g, file) else { continue };
            let next = order.len();
            let key = *order.entry(file).or_insert(next);
            let m = if joined { &mut paths } else { &mut mud };
            m.entry(key).or_insert_with(|| (tex, Vec::new(), false)).1.extend(v);
        }
        self.path_batches = Self::upload(g, paths);
        self.mud_batches = Self::upload(g, mud);
    }

    /// Retaining walls: PLACEHOLDER look, a vertical strip standing on the tile edge, 30 units tall, plus a thin cap.
    fn build_walls(&mut self, g: &mut Gfx) {
        if self.terrain.wall_mask.is_empty() {
            return;
        }
        let Some(tex) = self.theme_texture(g, "RetainingWallA.bmp") else { return };
        let t = &self.terrain;
        const WALL_HEIGHT: f32 = 30.0;
        const CAP: f32 = 5.0;
        let (ox, oz) = (-t.w as f32 * TILE_SIZE * 0.5, -t.h as f32 * TILE_SIZE * 0.5);
        let mut batch = Vec::new();
        const DX: [i32; 4] = [0, 1, 0, -1];
        const DY: [i32; 4] = [-1, 0, 1, 0];
        let vn = |x: f32, y: f32, z: f32, u: f32, v: f32, n: [f32; 3]| {
            let mut q = Vert::new(x, y, z, u, v);
            q.normal = n;
            q
        };
        for ty in 0..t.h {
            for tx in 0..t.w {
                for dir in 0..4 {
                    if !t.wall_at(tx, ty, dir) {
                        continue;
                    }
                    // A shared edge is drawn by the north / west tile only, unless it lies on the map border.
                    if (dir == 1 || dir == 2) && t.wall_at(tx + DX[dir as usize], ty + DY[dir as usize], (dir + 2) & 3) {
                        continue;
                    }
                    let (x, z) = (ox + tx as f32 * TILE_SIZE, oz + ty as f32 * TILE_SIZE);
                    let (x0, z0, x1, z1, ix, iz) = match dir {
                        0 => (x, z, x + TILE_SIZE, z, 0.0, CAP),
                        2 => (x, z + TILE_SIZE, x + TILE_SIZE, z + TILE_SIZE, 0.0, -CAP),
                        3 => (x, z, x, z + TILE_SIZE, CAP, 0.0),
                        _ => (x + TILE_SIZE, z, x + TILE_SIZE, z + TILE_SIZE, -CAP, 0.0),
                    };
                    let nx = if dir == 3 {
                        -1.0
                    } else if dir == 1 {
                        1.0
                    } else {
                        0.0
                    };
                    let nz = if dir == 0 {
                        -1.0
                    } else if dir == 2 {
                        1.0
                    } else {
                        0.0
                    };
                    const N: i32 = 4;
                    for i in 0..N {
                        let (a, b) = (i as f32 / N as f32, (i + 1) as f32 / N as f32);
                        let (xa, za, xb, zb) = (x0 + (x1 - x0) * a, z0 + (z1 - z0) * a, x0 + (x1 - x0) * b, z0 + (z1 - z0) * b);
                        let (ya, yb) = (t.height_at(xa, za), t.height_at(xb, zb));
                        let side = [nx, 0.0, nz];
                        let (p0, p1) = (vn(xa, ya, za, a, 0.3, side), vn(xb, yb, zb, b, 0.3, side));
                        let (p2, p3) = (vn(xb, yb + WALL_HEIGHT, zb, b, 0.0, side), vn(xa, ya + WALL_HEIGHT, za, a, 0.0, side));
                        batch.extend_from_slice(&[p0, p1, p2, p0, p2, p3]);
                        let up = [0.0, 1.0, 0.0];
                        let (c0, c1) = (vn(xa, ya + WALL_HEIGHT, za, a, 0.3, up), vn(xb, yb + WALL_HEIGHT, zb, b, 0.3, up));
                        let (c2, c3) =
                            (vn(xb + ix, yb + WALL_HEIGHT, zb + iz, b, 0.4, up), vn(xa + ix, ya + WALL_HEIGHT, za + iz, a, 0.4, up));
                        batch.extend_from_slice(&[c0, c1, c2, c0, c2, c3]);
                    }
                }
            }
        }
        if !batch.is_empty() {
            self.wall_batches.push(Batch { tex, mesh: g.mesh(&batch), water: false });
        }
    }

    /// Loads a sprite from Flics/ (cached; None when the file is missing).
    pub fn sprite_for(&mut self, rel: &str, shadow: bool, pal: Option<&str>) -> Option<usize> {
        let key = format!("{rel}|{}", pal.unwrap_or(""));
        if let Some(v) = self.sprite_index.get(&key) {
            return *v;
        }
        let path = self.game_path(&format!("Flics/{rel}"));
        let pal_path = pal.map(|p| self.game_path(&format!("Flics/{p}")));
        let r = match load_sprite(&path, shadow, pal_path.as_deref()) {
            Ok(s) => {
                let n = s.frames.len();
                self.sprites.push(GlSprite { s, tex: vec![None; n] });
                Some(self.sprites.len() - 1)
            }
            Err(e) => {
                if !shadow {
                    eprintln!("sprite: {e}"); // many objects simply have no shadow file
                }
                None
            }
        };
        self.sprite_index.insert(key, r);
        r
    }

    /// A decoration sprite of the exe (id and palette id) for the loaded theme, body and shadow.
    fn decor_sprite(&mut self, id: u16, pal: u8) -> (Option<usize>, Option<usize>) {
        let theme = self.exe_theme();
        let Some(file) = sg_core::decor::sprite_file(id, theme) else { return (None, None) };
        let pal = sg_core::decor::palette_file(pal, theme);
        (self.sprite_for(&format!("{file}.flc"), false, pal), self.sprite_for(&format!("{file}Shadow.flc"), true, None))
    }

    /// The trees on the tree, pine and palm tiles, chosen and placed the way the exe's tile loop does (sg_core::decor).
    fn add_trees(&mut self) {
        self.ensure_land();
        let Some(land) = self.land.as_ref() else { return };
        let (ty, var, flags) = (land.ty.clone(), land.var.clone(), land.flags.clone());
        let theme = self.exe_theme();
        let noise = &self.noise;
        let field = |x: i32, y: i32| noise.field(x, y);
        let mut draws = Vec::new();
        for a in 0..land::N {
            for b in 0..land::N {
                let i = (a * land::N + b) as usize;
                if !(13..=15).contains(&ty[i]) {
                    continue;
                }
                let grown = !self.tile_growing(a, b);
                for d in sg_core::decor::trees(&ty, var[i], flags[i] & 0x100 != 0, a, b, theme, grown, &field) {
                    draws.push((a, b, grown, d));
                }
            }
        }
        for (a, b, grown, d) in draws {
            let (body, shadow) = self.decor_sprite(d.sprite, d.pal);
            let Some(bi) = body else { continue };
            let (cx, cz) = self.terrain.tile_centre(a, b);
            let n = self.sprites[bi].s.frames_per_view;
            let frame = d.frame.unwrap_or(n - 1).min(n - 1);
            self.props.push(Prop {
                tree: true,
                x: cx + d.da * TILE_SIZE,
                z: cz + d.db * TILE_SIZE,
                body,
                shadow,
                frame,
                facing: d.view,
                grow_tile: if grown { None } else { Some((a, b)) },
                ..Default::default()
            });
        }
    }

    /// Pushes a decoration sprite as a prop on tile (a, b).
    fn push_decor(&mut self, a: i32, b: i32, d: sg_core::decor::Draw, object: bool, decor: bool) {
        let (body, shadow) = self.decor_sprite(d.sprite, d.pal);
        let Some(bi) = body else { return };
        let n = self.sprites[bi].s.frames_per_view;
        let (cx, cz) = self.terrain.tile_centre(a, b);
        self.props.push(Prop {
            x: cx + d.da * TILE_SIZE,
            z: cz + d.db * TILE_SIZE,
            body,
            shadow,
            frame: d.frame.unwrap_or(n - 1).clamp(0, (n - 1).max(0)),
            facing: d.view,
            object,
            decor,
            ..Default::default()
        });
    }

    /// Flags on the cup tiles and tee markers on the open holes' tees, as the exe draws them every frame.
    fn update_decor_props(&mut self) {
        self.props.retain(|p| !p.decor);
        let theme = self.exe_theme();
        let mut draws = Vec::new();
        for h in 1..19 {
            let rec = &self.club.holes[h];
            let (pa, pb) = rec.pin;
            if pa != 0 && sg_core::course::inside(pa, pb) {
                let growing = self.tile_growing(pa, pb).then(|| self.staff_tiles.counter[(pb * self.staff_tiles.w + pa) as usize] as i32);
                draws.push((pa, pb, h, true, growing));
            }
            if rec.par != 0 && rec.back.0 != 0 {
                draws.push((rec.back.0, rec.back.1, h, false, None));
            }
        }
        for (a, b, h, cup, growing) in draws {
            let rec = self.club.holes[h].clone();
            if cup {
                let pop = self.decor_sprite(0x185 + theme as u16, 0x63).0.map(|s| self.sprites[s].s.frames_per_view).unwrap_or(1);
                let open = self.decor_sprite(0x189 + theme as u16, 0x63).0.map(|s| self.sprites[s].s.frames_per_view).unwrap_or(1);
                let d = sg_core::decor::flag(
                    theme,
                    growing,
                    pop,
                    rec.par != 0,
                    rec.mood_sum,
                    rec.tee_shots,
                    rec.plans,
                    self.game_tick as i32,
                    open,
                );
                self.push_decor(a, b, d, false, true);
            } else {
                for d in sg_core::decor::tee_markers(rec.par, rec.tee_facing, None) {
                    self.push_decor(a, b, d, false, true);
                }
            }
        }
    }

    /// The tile's growth counter is running (flag 0x4000).
    fn tile_growing(&self, a: i32, b: i32) -> bool {
        let t = &self.staff_tiles;
        a < t.w && b < t.h && t.flags.get((b * t.w + a) as usize).is_some_and(|f| f & staff::WORKED != 0)
    }

    /// Newly planted trees show their growth counter as the frame and stop growing at their last frame.
    fn update_tree_growth(&mut self) {
        let mut done = Vec::new();
        for p in self.props.iter_mut().filter(|p| p.tree) {
            let (Some((a, b)), Some(bi)) = (p.grow_tile, p.body) else { continue };
            let t = &self.staff_tiles;
            let i = (b * t.w + a) as usize;
            let n = self.sprites[bi].s.frames_per_view;
            if t.flags.get(i).is_some_and(|f| f & staff::WORKED != 0) {
                let c = t.counter[i] as i32;
                p.frame = c.min(n - 1);
                if c >= n - 1 {
                    done.push(i);
                    p.grow_tile = None;
                }
            } else {
                p.frame = n - 1;
                p.grow_tile = None;
            }
        }
        for i in done {
            self.staff_tiles.flags[i] &= !staff::WORKED;
        }
    }

    pub fn refresh_trees(&mut self) {
        self.props.retain(|p| !p.tree);
        // add_trees appends; keep trees first so painter's order ties stay as before.
        let before = self.props.len();
        self.add_trees();
        self.props.rotate_left(before);
    }

    /// Whether the building tool may place this kind now: unlocked by the course's holes (sandbox: all).
    pub fn build_available(&self, kind: usize) -> bool {
        // The garden page (kinds up to 5, the willow and the bridge) is always open; buildings unlock with holes.
        let k = kind as i32;
        // landmarks once the club owns a type (the property's own, donations, Happy Endings)
        if k == land::K_LANDMARK && self.club.landmarks_owned & 0x3fff == 0 {
            return false;
        }
        // home sites only while Silver and better members outnumber them (0x56d1b0)
        if k == land::K_HOME_SITE && self.club.homesite_demand < 1 && !self.econ.sandbox {
            return false;
        }
        OFFERED_KINDS.contains(&k)
            && (k <= 5 || k == land::K_WILLOW || k == land::K_BRIDGE || k < land::unlocked_kinds(self.holes.len(), self.econ.sandbox))
    }

    /// Scenery: trees on Woods tiles, the placed buildings, a clubhouse, and the pool of golfers.
    pub fn populate_props(&mut self) {
        self.props.clear();
        const CLUB: [[&str; 2]; 4] = [
            ["Bldgs/Park/clubL2", "Bldgs/Park/clubL2_base"],
            ["Bldgs/links/clubL2", "Bldgs/links/clubL2_dirt"],
            ["Bldgs/Desert/DESclubL2", "Bldgs/Desert/DesClubL1base"],
            ["Bldgs/Tropical/TROPclubL2", "Bldgs/Tropical/TROPclubL2_base"],
        ];
        self.add_trees();
        self.ensure_land();
        if self.land.is_some() {
            self.add_land_objects();
        } else if self.terrain.clubhouse_x >= 0 {
            let (mut x, mut z) = self.terrain.tile_centre(self.terrain.clubhouse_x, self.terrain.clubhouse_y);
            if self.terrain.clubhouse_size > 0 && self.terrain.clubhouse_size % 2 == 0 {
                x -= TILE_SIZE * 0.5;
                z -= TILE_SIZE * 0.5;
            }
            let [b, gr] = CLUB[self.theme];
            let body = Prop {
                x,
                z,
                body: self.sprite_for(&format!("{b}.flc"), false, None),
                shadow: self.sprite_for(&format!("{b}Shadow.flc"), true, None),
                ..Default::default()
            };
            let base = Prop {
                x,
                z,
                flat: true,
                body: self.sprite_for(&format!("{gr}.flc"), false, None),
                shadow: self.sprite_for(&format!("{gr}Shadow.flc"), true, None),
                ..Default::default()
            };
            if base.body.is_some() {
                self.props.push(base);
            }
            if body.body.is_some() {
                self.props.push(body);
            }
        }
        self.load_staff_sprites();
        self.adopt_holes = true;
        self.golfer_clips.clear();
        let roster = std::mem::take(&mut self.club.roster);
        let stories = std::mem::take(&mut self.club.stories);
        let celebrities = std::mem::take(&mut self.club.celebrities);
        let pros = std::mem::take(&mut self.club.pros);
        self.club = Club::new(if roster.is_empty() { sg_core::roster::load(&self.game_dir) } else { roster });
        self.club.stories = stories;
        self.club.celebrities = celebrities;
        self.club.pros = pros;
        // the property's own landmark is available from the start; a sandbox has them all
        self.club.landmarks_owned = if self.econ.sandbox {
            0x3fff
        } else {
            self.land
                .as_ref()
                .map(|l| l.objects.iter().filter(|o| o.kind == land::K_LANDMARK && o.sub < 16).fold(0, |m, o| m | 1 << o.sub))
                .unwrap_or(0)
        };
        self.club.new_game(&mut self.exe_rng);
        self.course = Course::default();
        self.staff_golfers.clear();
        self.econ.init();
        self.sim_time = 0.0;
        self.sync_course();
        for i in 0..golf::SLOTS {
            self.props.push(Prop { golfer: Some(i), hidden: true, ..Default::default() });
        }
    }

    pub fn load_theme(&mut self, g: &mut Gfx, theme: usize) -> bool {
        let dir = self.game_path(&format!("Data/Textures/{}", THEMES[theme]));
        let cat = match TextureCatalog::load(&dir) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: {e}");
                return false;
            }
        };
        self.clear_batches(g);
        for t in self.textures.drain().filter_map(|(_, t)| t) {
            g.ctx.delete_texture(t);
        }
        self.catalog = cat;
        self.theme = theme;
        for s in self.sprites.drain(..) {
            for t in s.tex.into_iter().flatten() {
                g.ctx.delete_texture(t);
            }
        }
        self.sprite_index.clear();
        let lighting = sg_core::fsutil::read_file(self.game_path(&format!("{}Lighting.txt", THEMES[theme])))
            .map(|d| sg_core::terrain::parse_lighting(&sg_core::formats::latin1(&d)))
            .unwrap_or_default();
        self.light = lighting;
        self.rebuild_batches(g);
        self.populate_props();
        true
    }

    // ---- golfers and the club -----------------------------------------------------------------------------------------------

    /// Golfers playing or walking home (the waiting ones are inside the clubhouse).
    pub fn golfers_on_course(&self) -> usize {
        self.club.g.iter().take(golf::SLOTS).filter(|g| g.hole > 0).count()
    }

    /// Brings the golfers' view of the course up to date: tiles, objects and levels from the land, weeds from the staff's tiles,
    /// the clubhouse door, and one hole record per tee and green pair (back and forward tee on the tee, pin and cup on the green).
    pub fn sync_course(&mut self) {
        self.ensure_land();
        if let Some(land) = self.land.as_mut() {
            land.sync_from_terrain(&self.terrain);
        }
        let Some(land) = &self.land else { return };
        self.course.sync(land);
        self.water_depth = sg_core::wildlife::water_depth(&self.course);
        self.course.door = self.club_anchor();
        self.course.theme = self.exe_theme();
        if self.course.objects.first().map(|o| o.kind) != Some(land::K_CLUBHOUSE) {
            let (a, b) = self.course.door;
            self.course.objects.insert(0, land::Object { kind: land::K_CLUBHOUSE, a, b, dir: 0, flags: 0x40, sub: 0, val: 0 });
        }
        self.weeds_to_course();
        for f in self.course.flags.iter_mut() {
            *f &= !(sg_core::course::f::CUP | if *f & sg_core::course::f::FOOTPRINT == 0 { sg_core::course::f::LOW } else { 0 });
        }
        // the hole numbers on tees and cup greens, from the golfers' hole records
        for h in 1..19 {
            let rec = &self.club.holes[h];
            for (k, (a, b)) in [rec.back, rec.fwd, rec.pin].into_iter().enumerate() {
                if a != 0 && sg_core::course::inside(a, b) {
                    let i = sg_core::course::idx(a, b);
                    let cup = if k == 2 { sg_core::course::f::CUP } else { 0 };
                    self.course.flags[i] = (self.course.flags[i] & !0x1f) | cup | h as u16;
                }
            }
        }
        let door = self.course.door;
        let home = &mut self.club.holes[19];
        home.par = 0;
        home.back = door;
        home.fwd = door;
        home.pin = door;
        if std::mem::take(&mut self.adopt_holes) {
            self.adopt_painted_holes();
        }
        // every terrain change plans the hole being built again
        self.club.game &= !golf::game::LAYOUT;
        self.club.layout(&mut self.course, &mut self.exe_rng);
        self.rebuild_hole_routes();
    }

    /// Forgets every hole (new terrain loaded); its painted tees and greens are adopted at the next sync.
    pub fn reset_holes(&mut self) {
        for h in 1..19 {
            let rec = &mut self.club.holes[h];
            rec.par = 0;
            rec.back = (0, 0);
            rec.fwd = (0, 0);
            rec.pin = (0, 0);
            rec.length = 0;
            rec.flags = 0;
            rec.markers = [(-1, -1); 3];
        }
        self.club.next_hole = 1;
        self.adopt_holes = true;
    }

    /// The open holes as routes for drawing and the course report.
    fn rebuild_hole_routes(&mut self) {
        let numbers: Vec<i32> = (1..19).filter(|&h| self.club.holes[h as usize].par != 0).collect();
        let p = &self.terrain.path;
        self.holes = numbers
            .iter()
            .map(|&h| {
                let rec = &self.club.holes[h as usize];
                let (tx, tz) = self.terrain.tile_centre(rec.back.0, rec.back.1);
                let (gx, gz) = self.terrain.tile_centre(rec.pin.0, rec.pin.1);
                let near = |x: f32, z: f32, px: f32, pz: f32| (x - px).hypot(z - pz) < 300.0;
                let route = if p.len() >= 4 && near(p[0], p[1], tx, tz) && near(p[p.len() - 2], p[p.len() - 1], gx, gz) {
                    p.clone()
                } else {
                    vec![tx, tz, gx, gz]
                };
                HoleRoute { tee_x: tx, tee_z: tz, green_x: gx, green_z: gz, length: (gx - tx).hypot(gz - tz), par: rec.par, route }
            })
            .collect();
        if numbers != self.hole_numbers {
            self.hole_numbers = numbers;
            self.hole_stats = vec![HoleStat::default(); self.holes.len()];
        }
    }

    /// Terrain from outside the hole tool: each painted tee and green pair not yet a hole is built and opened, in the order
    /// the pairs are found (tees top row first), as if made with the tool.
    fn adopt_painted_holes(&mut self) {
        for r in find_holes(&self.terrain) {
            let h = self.club.next_hole;
            if !(1..19).contains(&h) {
                break;
            }
            let tee = self.terrain.tile_of(r.tee_x, r.tee_z);
            let pin = self.terrain.tile_of(r.green_x, r.green_z);
            let numbered =
                |c: &Course, (a, b): (i32, i32)| sg_core::course::inside(a, b) && c.flags[sg_core::course::idx(a, b)] & 0x1f != 0;
            if numbered(&self.course, tee) || numbered(&self.course, pin) {
                continue;
            }
            let rec = &mut self.club.holes[h as usize];
            rec.back = tee;
            rec.fwd = tee;
            rec.pin = pin;
            for (k, (a, b)) in [tee, pin].into_iter().enumerate() {
                if sg_core::course::inside(a, b) {
                    let i = sg_core::course::idx(a, b);
                    let cup = if k == 1 { sg_core::course::f::CUP } else { 0 };
                    self.course.flags[i] = (self.course.flags[i] & !0x1f) | cup | h as u16;
                }
            }
            self.club.game &= !golf::game::LAYOUT;
            // adoption is not part of the exe's game, so it plans with a copy and leaves the game's generator alone
            let mut rng = self.exe_rng;
            self.club.layout(&mut self.course, &mut rng);
            self.club.open_hole(&mut self.course);
        }
        // adopted holes open quietly
        self.club.out.retain(|e| !matches!(e, golf::Event::Sound { .. } | golf::Event::Message(_)));
    }

    /// The exe's open-hole command (key H, or a click on the new hole's cup green): the hole being built needs a tee and a
    /// green; its par comes from its length.
    pub fn open_hole(&mut self) -> bool {
        self.sync_course();
        let Some(h) = self.club.open_hole(&mut self.course) else { return false };
        if let Some(post) = self.club.first_post.take() {
            if let Some(e) = self.employees.first_mut().filter(|e| e.active) {
                e.post = Some(post);
            }
        }
        println!("hole {h} open: par {}, length {}", self.club.holes[h].par, self.club.holes[h].length);
        self.sync_course();
        true
    }

    /// The staff's weed tiles into the golfers' tile flags (the exe keeps both in one array).
    fn weeds_to_course(&mut self) {
        let t = &self.staff_tiles;
        if t.flags.len() != (t.w * t.h) as usize {
            return;
        }
        for a in 0..t.w.min(50) {
            for b in 0..t.h.min(50) {
                let si = (b * t.w + a) as usize;
                let ci = sg_core::course::idx(a, b);
                let bits = t.flags[si] & (staff::WEEDS | staff::WORKED);
                self.course.flags[ci] = (self.course.flags[ci] & !(staff::WEEDS | staff::WORKED)) | bits;
                self.course.growth[ci] = t.counter[si];
            }
        }
    }

    /// New weeds the golfers started, back into the staff's tiles.
    fn weeds_from_course(&mut self) {
        let t = &mut self.staff_tiles;
        if t.flags.len() != (t.w * t.h) as usize {
            return;
        }
        for a in 0..t.w.min(50) {
            for b in 0..t.h.min(50) {
                let si = (b * t.w + a) as usize;
                let ci = sg_core::course::idx(a, b);
                let bits = self.course.flags[ci] & (staff::WEEDS | staff::WORKED);
                t.flags[si] = (t.flags[si] & !(staff::WEEDS | staff::WORKED)) | bits;
                t.counter[si] = self.course.growth[ci];
            }
        }
    }

    /// Sprite id (clip + body) of a golfer clip, loaded on first use: (body, shadow).
    fn golfer_clip(&mut self, id: i32) -> (Option<usize>, Option<usize>) {
        if let Some(c) = self.golfer_clips.get(&id) {
            return *c;
        }
        let mut found = (None, None);
        if let Some(&(base, name)) = GOLFER_CLIPS.iter().rev().find(|(b, _)| *b <= id && id - *b <= 8) {
            let body = (id - base) as usize;
            if let Some(prefix) = GOLFER_BODIES.get(body).filter(|p| !p.is_empty()) {
                let name = if base == 0x28 && body >= 5 { "NormalSwing" } else { name };
                found = (
                    self.sprite_for(&format!("{prefix}_{name}.flc"), false, None),
                    self.sprite_for(&format!("{prefix}_{name}Shadow.flc"), true, None),
                );
            }
        }
        if let Some(b) = found.0 {
            if let Some(slot) = self.club.clip_frames.get_mut(id as usize) {
                *slot = self.sprites[b].s.frames_per_view.max(1);
            }
        }
        self.golfer_clips.insert(id, found);
        found
    }

    /// Makes sure the clip lengths of every body on the course are known to the golfer animation.
    fn load_golfer_bodies(&mut self) {
        for g in 0..golf::SLOTS {
            let h = self.club.g[g].hole;
            if h == 0 || h == -1 {
                continue;
            }
            let look = self.club.look(g);
            if self.golfer_clips.contains_key(&(1000 + look)) {
                continue;
            }
            self.golfer_clips.insert(1000 + look, (None, None));
            for &(base, _) in GOLFER_CLIPS.iter() {
                self.golfer_clip(base + look);
            }
        }
    }

    /// One game tick of the golfers (the exe updates them just before the staff).
    fn club_tick(&mut self) {
        self.load_golfer_bodies();
        self.weeds_to_course();
        self.club.difficulty = self.difficulty;
        self.club.cash = (self.econ.cash / Economy::UNIT) as i32;
        self.club.year = self.econ.year_index() as i32;
        let tick = self.game_tick;
        let sites = self.land.as_ref().map(|l| l.objects.iter().filter(|o| o.kind == land::K_HOME_SITE).count()).unwrap_or(0) as i32;
        self.club.sandbox = self.econ.sandbox;
        self.club.island = self.land.as_ref().map(|l| l.slot.record().coast == 2).unwrap_or(false);
        if std::mem::take(&mut self.club.land_offer) {
            // the commissioner approved an expansion: the exe asks whether to buy land, then shows the tracts
            self.show_toast("Do you wish to purchase additional land to expand your course?");
            self.open_land_screen();
        }
        self.club.home_sites = sites;
        self.club.homesite_demand = self.club.ratings.waitlist;
        if tick.is_multiple_of(1024 / (self.difficulty.clamp(0, 3) as u32 + 2)) {
            // hole maintenance (+0x1f8): one unit per open hole each charge interval
            for h in self.club.holes.iter_mut() {
                if h.par != 0 {
                    h.maint += 1;
                }
            }
        }
        if sites > 0 && tick.is_multiple_of(1024 / (self.difficulty.clamp(0, 3) as u32 + 2)) {
            let sizes = |l: &land::Land| l.objects.iter().map(|o| sg_core::homes::house_size(o.val)).collect::<Vec<_>>();
            if let Some(l) = self.land.as_mut() {
                let before = (sizes(l), l.objects.iter().map(|o| o.sub).collect::<Vec<_>>());
                let visited = sg_core::homes::revalue(&mut l.objects, &self.course, &self.club.holes, self.difficulty, tick);
                for i in visited {
                    let (val, owner) = (l.objects[i].val, l.objects[i].sub);
                    let home = (l.objects[i].a, l.objects[i].b);
                    if let Some(o) = self.club.celebrity_home(&mut self.exe_rng, val, owner, home) {
                        l.objects[i].sub = o;
                    }
                }
                if (sizes(l), l.objects.iter().map(|o| o.sub).collect::<Vec<_>>()) != before {
                    self.props.retain(|p| !p.object);
                    self.add_land_objects();
                }
            }
        }
        self.club.course_name = self.course_name.clone();
        self.club.course_theme = self.exe_theme();
        self.pro_round_tick();
        self.club.tick(&mut self.course, &mut self.exe_rng, tick);
        sg_core::ratings::pass(&mut self.club, self.difficulty);
        self.pro_after_tick();
        self.board_tick();
        if tick.is_multiple_of(1024 / (self.difficulty.clamp(0, 3) as u32 + 2)) {
            let members = self.club.member_count();
            self.club.record_history(self.club.cash, members);
        }
        if tick > 0 && tick & 0x1fff == 0 && !self.club.championship() {
            println!("[{:6.1}s] end of year {}", self.sim_time, 2000 + (tick >> 13));
            self.open_year_end();
        }
        self.wildlife_tick();
        self.tourney_after_tick();
        self.weeds_from_course();
        let events: Vec<golf::Event> = self.club.out.drain(..).collect();
        for e in events {
            match e {
                golf::Event::Sound { slot, at } => {
                    let (wx, wz) = match at {
                        Some((x, y)) => self.units_to_world(x, y),
                        None => (self.cam_x, self.cam_z),
                    };
                    self.slot_sound(slot, wx, wz);
                }
                golf::Event::Earn { units, column, .. } => {
                    let col = match column {
                        golf::Column::GreensFees => economy::LEDGER_GREEN_FEES,
                        golf::Column::FoodDrink => economy::LEDGER_FOOD_DRINK,
                        golf::Column::Other => economy::LEDGER_OTHER,
                    };
                    if col == economy::LEDGER_GREEN_FEES {
                        self.econ.hole_completed(units as f64);
                    } else {
                        self.econ.earn_to(col, units as f64 * Economy::UNIT);
                    }
                }
                golf::Event::Message(m) => {
                    if !m.is_empty() {
                        println!("[{:6.1}s] {m}", self.sim_time);
                        self.show_toast(&m);
                    }
                }
                golf::Event::HoleDone { hole, strokes, mood, fee, .. } => {
                    if self.hole_stats.len() != self.holes.len() {
                        self.hole_stats = vec![HoleStat::default(); self.holes.len()];
                    }
                    let k = self.hole_numbers.iter().position(|&n| n == hole).unwrap_or(usize::MAX);
                    let time = self.club.holes.get(hole as usize).map(|h| h.time).unwrap_or(0);
                    let mood_sum = self.club.holes.get(hole as usize).map(|h| h.mood_sum).unwrap_or(0);
                    if let Some(hs) = self.hole_stats.get_mut(k) {
                        hs.plays += 1;
                        hs.strokes += strokes as f64;
                        hs.seconds = time as f64 * 2.0 * sg_core::flight::TICK_MS as f64 / 1000.0;
                        hs.revenue += fee as f64 * Economy::UNIT;
                        hs.mood += mood as f64 * 10.0;
                        hs.hist[(strokes - 3).clamp(0, 5) as usize] += 1;
                        hs.mood_sum = mood_sum;
                    }
                }
                golf::Event::Thought { g, id: 0x32, .. } => {
                    if let Some(line) = self.club.story_line(g) {
                        let story = self.club.g[g].story;
                        self.story_title = self.club.stories.title(story).trim().to_string();
                        let who = self.club.name(g);
                        println!("[{:6.1}s] story '{}': {who}: {line}", self.sim_time, self.story_title);
                        self.story_lines.push(format!("{who}|{line}"));
                        if self.story_lines.len() > 2 {
                            self.story_lines.remove(0);
                        }
                        self.story_until = self.game_tick + 90;
                    }
                }
                golf::Event::Thought { .. } => {}
            }
        }
    }

    /// Advances the club by dt seconds: one golfer and staff update per game tick (87 ms), fees, wages and the board's messages.
    pub fn step_game(&mut self, dt: f32) {
        if !self.econ.notice.is_empty() {
            println!("[{:6.1}s] board: {}", self.sim_time, self.econ.notice);
            self.year_notice = self.econ.notice.clone();
            self.econ.notice.clear();
        }
        self.tick_acc += dt as f64;
        let tick_s = sg_core::flight::TICK_MS as f64 / 1000.0;
        while self.tick_acc >= tick_s {
            self.tick_acc -= tick_s;
            if !self.econ.game_over {
                self.club_tick();
            }
            self.staff_tick();
            self.resident_tick();
        }
    }

    /// Advances the simulation to the current time and points every golfer prop at its golfer.
    pub fn update_props(&mut self) {
        while self.sim_time < self.time {
            self.step_game(1.0 / 60.0);
            self.sim_time += 1.0 / 60.0;
        }
        if self.time < self.sim_time - 1.0 {
            // clock went backwards
            self.sim_time = 0.0;
        }
        // where each golfer is on screen, or -1 when off it (the exe's drawing records this; a golfer who has finished and is
        // out of sight goes straight home)
        for gi in 0..golf::SLOTS {
            if self.club.g[gi].hole == 0 {
                continue;
            }
            let (x, y) = (self.club.g[gi].x, self.club.g[gi].y);
            let (sx, sy) = self.screen_of(x, y).map(|(a, b)| (a as i32, b as i32)).unwrap_or((-1, -1));
            self.club.g[gi].sx = sx;
            self.club.g[gi].sy = sy;
        }
        const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
        const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];
        for pi in 0..self.props.len() {
            let Some(gi) = self.props[pi].golfer else { continue };
            let (h, x, y, facing, anim, clip) = {
                let g = &self.club.g[gi];
                (g.hole, g.x, g.y, g.facing, g.anim, g.clip as i32)
            };
            if h == 0 || h == -1 {
                self.props[pi].hidden = true;
                continue;
            }
            let (id, frame) = self.club.drawn[gi];
            let (body, shadow) = self.golfer_clip(id);
            let Some(b) = body else {
                self.props[pi].hidden = true;
                continue;
            };
            // The exe draws the swing clips one view round from the golfer's facing (not the putt); the bench clips have four
            // views and take half the eight-way view.
            let mut d = facing & 7;
            if anim < 7 && clip != 0x5a {
                d = (d - 1) & 7;
            }
            let (wx, wz) = self.units_to_world(x, y);
            let p = &mut self.props[pi];
            p.hidden = false;
            p.body = Some(b);
            p.shadow = shadow;
            p.x = wx;
            p.z = wz;
            p.frame = frame;
            p.heading = DY[d as usize].atan2(DX[d as usize]).to_degrees();
            p.facing = -((d + 2 + (d & 1)) / 2);
        }
        self.update_staff_props();
        self.update_resident_props();
        self.update_ambient_props();
        self.update_weed_props();
        self.update_tree_growth();
        self.update_decor_props();
        for p in self.props.iter_mut() {
            if p.animated {
                if let Some(b) = p.body {
                    p.frame = (self.game_tick % self.sprites[b].s.frames_per_view.max(1) as u32) as i32;
                }
            }
        }
    }

    /// Course Status Report (the original's F1): the hole's class and length, and how many paths are joined to the clubhouse.
    pub fn report_course(&mut self, force: bool) {
        self.hole = analyze_hole(&self.terrain);
        self.sync_course();
        let conn = paths_connected_to_clubhouse(&self.terrain);
        let paths = self.terrain.path_kind.iter().filter(|&&k| k != 0).count();
        let joined = self.terrain.path_kind.iter().zip(&conn).filter(|(&k, &c)| k != 0 && c != 0).count();
        let r = format!(
            "{}; {} holes (tee to green pairs); paths: {paths} tiles, {joined} joined to the clubhouse, {} shown as mud",
            self.hole.report(),
            self.holes.len(),
            paths - joined
        );
        if force || r != self.last_report {
            println!("course: {r}");
            self.last_report = r;
        }
    }

    // ---- new games, stories --------------------------------------------------------------------------------------------------

    /// Deals the properties out to the offer slots for a new game, as the exe does before showing the property chooser.
    pub fn deal_offer(&mut self, sandbox: bool) {
        self.offer = land::deal_offer(&mut self.exe_rng, sandbox);
        self.slot_of = slots_of(&self.offer);
    }

    /// Acres and price (in money) of a property in this game's offer.
    pub fn offer_for(&self, prop_idx: usize) -> (i32, i32) {
        let slot = self.slot_of[prop_idx];
        (self.offer[slot].acres, land::SLOT_PRICE_UNITS[slot] * 100)
    }

    pub fn start_game(&mut self, g: &mut Gfx, prop_idx: usize, sandbox: bool) {
        let p = PROPERTIES[prop_idx];
        let (acres, price) = self.offer_for(prop_idx);
        self.econ.sandbox = sandbox;
        // The property is paid for out of the starting funds.
        // The exe starts with 1000 units (10000 in sandbox) and takes the property's price off a normal game.
        self.econ.start_cash = if sandbox { (START_FUNDS * 10) as f64 } else { (START_FUNDS - price) as f64 };
        let slot_index = self.slot_of[prop_idx];
        let land = land::generate(slot_index, self.offer[slot_index], self.difficulty, sandbox, &mut self.exe_rng, &self.noise);
        self.terrain = land.to_terrain();
        self.land = Some(land);
        self.course_name = format!("{} GC", p.name);
        self.load_theme(g, p.theme);
        self.club.course_name = self.course_name.clone();
        self.club.course_theme = self.exe_theme();
        self.screen = Screen::Play;
        self.hover = -1;
        self.edit = false;
        self.panel = 0;
        self.reset_staff();
        self.load_story();
        self.reset_clock = true;
        println!(
            "new game: {}, {} acres, price {}, cash left {:.0}{}",
            p.name,
            acres,
            price,
            self.econ.start_cash,
            if sandbox { " (sandbox)" } else { "" }
        );
    }

    /// The whole-game save file, next to the course file.
    pub fn game_file(&self) -> PathBuf {
        self.course_file.with_extension("sgs")
    }

    /// Saves the whole game.
    pub fn save_game(&self, path: &Path) -> Result<(), String> {
        let save = SaveGame {
            version: 1,
            theme: self.theme,
            course_name: self.course_name.clone(),
            difficulty: self.difficulty,
            terrain: self.terrain.clone(),
            land: self.land.clone(),
            club: self.club.clone(),
            course: self.course.clone(),
            econ: self.econ.clone(),
            exe_rng: self.exe_rng,
            noise: self.noise.clone(),
            employees: self.employees.clone(),
            staff_tiles: self.staff_tiles.clone(),
            game_tick: self.game_tick,
            hole_stats: self.hole_stats.clone(),
        };
        let json = serde_json::to_vec(&save).map_err(|e| e.to_string())?;
        let mut out = SAVE_MAGIC.to_vec();
        out.extend(miniz_oxide::deflate::compress_to_vec_zlib(&json, 6));
        if sg_core::fsutil::write_file(path, &out) {
            Ok(())
        } else {
            Err(format!("{}: could not write", path.display()))
        }
    }

    /// Loads a whole game saved by `save_game` and goes on from where it was.
    pub fn load_game(&mut self, g: &mut Gfx, path: &Path) -> Result<(), String> {
        let data = sg_core::fsutil::read_file(path).ok_or_else(|| format!("{}: not found", path.display()))?;
        let body = data.strip_prefix(SAVE_MAGIC).ok_or_else(|| format!("{}: not a saved game", path.display()))?;
        let json = miniz_oxide::inflate::decompress_to_vec_zlib(body).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let s: SaveGame = serde_json::from_slice(&json).map_err(|e| format!("{}: {e}", path.display()))?;
        self.terrain = s.terrain;
        self.land = s.land;
        self.course_name = s.course_name;
        self.difficulty = s.difficulty;
        self.adopt_holes = false;
        if !self.load_theme(g, s.theme) {
            return Err("could not load the theme".into());
        }
        self.club = s.club;
        let tutorial = self.club.stories.tutorial;
        self.load_story();
        self.club.stories.tutorial = tutorial;
        self.course = s.course;
        self.econ = s.econ;
        self.exe_rng = s.exe_rng;
        self.noise = s.noise;
        self.employees = s.employees;
        self.staff_tiles = s.staff_tiles;
        self.game_tick = s.game_tick;
        self.hole_numbers.clear();
        self.sync_course();
        self.hole_stats = s.hole_stats;
        self.sim_time = 0.0;
        self.tick_acc = 0.0;
        self.reset_clock = true;
        self.screen = Screen::Play;
        self.dirty = true;
        Ok(())
    }

    /// Loads the theme pack's stories (every *.txt in its Themes folder, or the Standard folder's when it has none, listed in
    /// case-insensitive name order as the original's file system does), its celebrities and its pro golfers.
    pub fn load_story(&mut self) {
        self.story_lines.clear();
        self.story_title.clear();
        let pack = THEME_PACKS.get(self.theme_pack).copied().unwrap_or("Standard").replace(' ', "_");
        let list = |dir: &Path| -> Vec<(String, String)> {
            let mut files: Vec<PathBuf> = sg_core::fsutil::list_dir(dir)
                .into_iter()
                .filter(|p| sg_core::fsutil::ext_lower(p) == ".txt")
                .filter(|p| !p.file_name().map(|n| n.to_string_lossy().to_lowercase().contains("shadow")).unwrap_or(false))
                .collect();
            files.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
            files
                .iter()
                .filter_map(|p| {
                    let name = p.file_name()?.to_string_lossy().into_owned();
                    let text = sg_core::formats::latin1(&sg_core::fsutil::read_file(p)?);
                    Some((name, text))
                })
                .collect()
        };
        let mut files = list(&self.game_path(&format!("Themes/{pack}")));
        if files.is_empty() {
            files = list(&self.game_path("Themes/Standard"));
        }
        self.club.stories = sg_core::stories::Stories::build(&files, self.club.stories.tutorial);
        let read = |name: &str| -> Option<String> {
            sg_core::fsutil::read_file(self.game_path(&format!("Themes/{pack}/{name}")))
                .or_else(|| sg_core::fsutil::read_file(self.game_path(&format!("Themes/Standard/{name}"))))
                .map(|d| sg_core::formats::latin1(&d))
        };
        let celebs = read("celebrities.dta").map(|t| sg_core::vips::parse_celebrities(&t)).unwrap_or_default();
        let pros = read("progolfers.dta").map(|t| sg_core::vips::parse_pros(&t)).unwrap_or_default();
        self.club.celebrities = celebs;
        self.club.pros = pros;
    }

    /// Advisor text from what the club looks like now. These hints are our own words.
    pub fn advisor_text(&self) -> &'static str {
        if self.holes.is_empty() {
            "Welcome to your new club. Open Build Course (the big round button at the bottom left), then paint a tee and a green a good distance apart to make your first hole."
        } else if !self.land.as_ref().map(|l| l.objects.iter().any(|o| (6..=14).contains(&o.kind))).unwrap_or(false) {
            "Golfers are on the course. Open Add Buildings and put up an amenity, then lay a path from it to the clubhouse: buildings only work once a path joins them to the clubhouse."
        } else if self.holes.len() < 3 {
            "More holes bring more golfers and more money. Build another tee and green, and make the holes different: long, narrow and tricky shots raise the club's skill rating."
        } else if self.econ.staff_count() == 0 {
            "Your course is growing. The People button lets you hire a club pro, ranger, groundskeeper or soda vendor, who cost wages but keep golfers happy."
        } else {
            "Watch the fun and skill numbers at the top right. Press the information button for the course report, and keep cash above zero so the board stays calm."
        }
    }

    // ---- course editing ----------------------------------------------------------------------------------------------------

    pub fn edit_paint(&mut self, tx: i32, ty: i32) {
        let pe = &PAINT[self.paint_idx];
        let r = self.brush;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r + r {
                    continue;
                }
                let (x, y) = (tx + dx, ty + dy);
                if !self.terrain.inside(x, y) {
                    continue;
                }
                // tees and greens go through the hole tool's bookkeeping (the hole being built gets its tee or cup)
                let old = self.terrain.ty[self.terrain.tile_index(x, y)];
                let new = pe.ty as u8;
                if old <= 1 || new <= 1 {
                    let green_next = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(ex, ey)| {
                        self.terrain.inside(x + ex, y + ey) && self.terrain.ty[self.terrain.tile_index(x + ex, y + ey)] == 1
                    });
                    if !self.club.paint_hole_tile(&mut self.course, x, y, old, new, green_next) {
                        continue;
                    }
                }
                // tees use this byte as their look
                let vb = if pe.ty == 0 { ((x * 7 + y * 13) as u32 % 5) as i32 } else { pe.vbyte };
                if self.terrain.ty[self.terrain.tile_index(x, y)] as i32 != pe.ty {
                    self.econ.spend_to(economy::LEDGER_BUILD_COURSE, Economy::terrain_cost_units(pe.ty) as f64 * Economy::UNIT);
                }
                let changed = self.terrain.ty[self.terrain.tile_index(x, y)] as i32 != pe.ty;
                self.terrain.paint(x, y, pe.ty, vb);
                if changed {
                    self.painted_tile(x, y, pe.ty as u8);
                }
                if matches!(pe.ty, 0 | 1 | 17 | 22) {
                    self.terrain.flatten_tile(x, y);
                }
            }
        }
        // one "already has its tee" message per stroke, not per tile
        self.club.out.dedup();
        self.dirty = true;
        let s = match pe.ty {
            2 | 3 => "Interface/Place Fairway.wav",
            0 | 1 => "Interface/Place GreenTee.wav",
            7 | 9 | 34 | 35 => "Interface/Place Bunker.wav",
            17 => "Interface/Place Water.wav",
            12 | 31 | 32 => "Interface/Place Rocks.wav",
            22 => "Interface/Building.wav",
            33 => "Effects/Flower Bed.wav",
            _ => "Interface/Place Rough.wav",
        };
        self.snd(s, 0.7, false);
    }

    /// Opens the land screen (after the County Commissioner approves an expansion and the player says yes).
    pub fn open_land_screen(&mut self) {
        self.ensure_land();
        self.slot_sound(56, 0.0, 0.0);
        let Some(land) = self.land.as_ref() else { return };
        self.tracts = sg_core::tracts::roll(land, self.club.purchases, &mut self.exe_rng);
        self.land_hover = -1;
        self.screen = Screen::Land;
    }

    /// Pointer over the land screen (virtual 800 x 600 coordinates): which tract or the cancel button. The exe redraws on
    /// every change and rolls the prices again each time (keeping the first ones), which draws from the generator.
    pub fn land_pointer(&mut self, x: f32, y: f32) {
        let mut hit = -1;
        if (54.0..258.0).contains(&y) {
            let row = ((y - 54.0) / 68.0) as i32;
            let col = if (15.0..262.0).contains(&x) {
                0
            } else if (272.0..524.0).contains(&x) {
                1
            } else if (538.0..775.0).contains(&x) {
                2
            } else {
                -1
            };
            if col >= 0 && row < 3 {
                hit = row + 3 * col;
            }
        }
        if (662.0..726.0).contains(&x) && (533.0..597.0).contains(&y) {
            hit = 9;
        }
        if hit != self.land_hover {
            self.land_hover = hit;
            if let Some(land) = self.land.as_ref() {
                let _ = sg_core::tracts::roll(land, self.club.purchases, &mut self.exe_rng);
            }
        }
    }

    /// A click on the land screen: buy the tract under the pointer, or leave on the cancel button or a right click.
    pub fn land_click(&mut self, right: bool) {
        if right || self.land_hover == 9 {
            self.screen = Screen::Play;
            return;
        }
        if !(0..9).contains(&self.land_hover) {
            self.slot_sound(24, 0.0, 0.0);
            return;
        }
        let i = self.land_hover as usize;
        let price = self.tracts[i].price;
        if price == 0 {
            return;
        }
        let cost = price as f64 * Economy::UNIT;
        if !self.econ.affordable(cost, self.holes.len()) {
            self.show_toast(&format!(
                "This change costs {}. You have only {}.",
                crate::ui::money(cost as i64),
                crate::ui::money(self.econ.cash as i64)
            ));
            return;
        }
        self.econ.spend_to(economy::LEDGER_OTHER, cost);
        if let Some(land) = self.land.as_mut() {
            sg_core::tracts::buy(land, i);
            self.club.log_event(sg_core::records::log::LAND, i as i32);
            // the terrain the renderer and editor use follows
            let (a0, b0) = sg_core::tracts::origin(i);
            for a in a0..a0 + 16 {
                for b in b0..b0 + 16 {
                    let t = (a * land::N + b) as usize;
                    let o = self.terrain.tile_index(a, b);
                    if self.terrain.ty[o] == land::T_OUT {
                        self.terrain.ty[o] = land.ty[t];
                        self.terrain.variation[o] = if land.ty[t] == land::T_WATER { land.var[t] } else { 0 };
                    }
                }
            }
        }
        self.club.purchases += 1;
        println!("bought tract {} for {}", i + 1, crate::ui::money(cost as i64));
        self.screen = Screen::Play;
        self.dirty = true;
        self.refresh_trees();
    }

    /// The exe's bookkeeping for a tile painted to a new type (0x420561..): weeds go, the tile starts its growth counter, and
    /// its variant byte becomes the brush's variant (tees 0, greens 0 or 0xff), which sizes a planted tree.
    fn painted_tile(&mut self, x: i32, y: i32, new: u8) {
        let v = match self.paint_variant {
            Some((k, v)) if k == self.paint_idx => v,
            _ => {
                let v = self.exe_rng.below(3);
                self.paint_variant = Some((self.paint_idx, v));
                v
            }
        };
        self.ensure_land();
        self.retyped.push((x, y));
        if let Some(land) = self.land.as_mut() {
            if (0..land::N).contains(&x) && (0..land::N).contains(&y) {
                let i = (x * land::N + y) as usize;
                land.flags[i] &= 0xd6ff;
                land.growth[i] = 0;
                if land.flags[i] & 0x200 == 0 {
                    land.var[i] = (v % 12 + 12) as u8;
                }
                if new == 0 {
                    land.var[i] = 0;
                } else if new == 1 {
                    land.var[i] = if v & 1 != 0 { 0xff } else { 0 };
                }
            }
        }
        let t = &mut self.staff_tiles;
        if x < t.w && y < t.h {
            let i = (y * t.w + x) as usize;
            if let Some(f) = t.flags.get_mut(i) {
                *f = (*f & !staff::WEEDS) | staff::WORKED;
                t.counter[i] = 0;
            }
        }
        if let Some((slot, _)) = sg_core::decor::tree_sound(new) {
            self.slot_sound(slot, 0.0, 0.0);
        }
    }

    pub fn edit_path(&mut self, tx: i32, ty: i32, remove: bool) {
        let r = self.brush;
        if self.terrain.path_kind.len() != self.terrain.ty.len() {
            self.terrain.path_kind = vec![0; self.terrain.ty.len()];
        }
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (tx + dx, ty + dy);
                if dx * dx + dy * dy > r * r + r || !self.terrain.inside(x, y) {
                    continue;
                }
                if remove {
                    self.undo_tile(x, y);
                } else {
                    self.place_tile_item(x, y, land::K_PATH);
                }
            }
        }
        self.dirty = true;
        self.snd("Interface/Path.wav", 0.7, false);
    }

    /// One tile item (path, bench, flower bed, willow, scenic bridge) through the exe's building tool: the item's price (half
    /// the clearing for paths; willows 25 and bridges 100 flat), the affordability test, Facilities in the ledger, an undo record.
    pub fn place_tile_item(&mut self, x: i32, y: i32, kind: i32) -> bool {
        self.ensure_land();
        let theme = self.exe_theme();
        let holes = self.holes.len();
        let variant = match kind {
            land::K_BENCH => self.exe_rng.below(5),
            land::K_FLOWERS => self.exe_rng.below(4),
            land::K_WILLOW => self.exe_rng.below(7),
            land::K_BRIDGE => self.exe_rng.below(8),
            _ => 0,
        };
        let Some(land) = self.land.as_mut() else { return false };
        land.sync_from_terrain(&self.terrain);
        let cost = match land.tile_item_cost(x, y, kind, theme) {
            Ok(c) => c,
            Err(m) => {
                if !m.is_empty() {
                    self.show_toast(m);
                }
                self.slot_sound(24, 0.0, 0.0);
                return false;
            }
        };
        let amount = cost as f64 * Economy::UNIT;
        if !self.econ.affordable(amount, holes) {
            self.show_toast(&format!("This change costs {}. You have only {}.", money(amount as i64), money(self.econ.cash as i64)));
            self.slot_sound(24, 0.0, 0.0);
            return false;
        }
        let Some(land) = self.land.as_mut() else { return false };
        if land.place_tile_item(x, y, kind, variant, theme).is_err() {
            return false;
        }
        let i = self.terrain.tile_index(x, y);
        if (kind == land::K_PATH || kind == land::K_BENCH || kind == land::K_BRIDGE) && self.terrain.path_kind[i] == 0 {
            self.terrain.path_kind[i] = self.path_kind.max(1) as u8;
        }
        land.write_area(&mut self.terrain, x, y, x, y);
        if cost != 0 {
            self.econ.spend_to(economy::LEDGER_FACILITIES, amount);
        }
        match kind {
            land::K_BENCH => self.slot_sound(262, 0.0, 0.0),
            land::K_FLOWERS => self.slot_sound(148, 0.0, 0.0),
            land::K_BRIDGE => self.slot_sound(263, 0.0, 0.0),
            _ => {}
        }
        self.after_object_change();
        true
    }

    /// Undo on a tile, as the exe's second right-click does: an item comes off with exactly what it cost back (booked to Build
    /// course), a building is demolished with no refund.
    pub fn undo_tile(&mut self, x: i32, y: i32) {
        self.ensure_land();
        let Some(land) = self.land.as_mut() else { return };
        land.sync_from_terrain(&self.terrain);
        match land.undo_tile(x, y) {
            land::Undone::Refused => self.slot_sound(24, 0.0, 0.0),
            land::Undone::Building(i) => {
                if land.objects[i].kind == land::K_CLUBHOUSE {
                    return;
                }
                let o = land.objects[i];
                let size = land.footprint_size(&o);
                land.remove_object(i);
                land.write_area(&mut self.terrain, o.a, o.b, o.a + size - 1, o.b + size - 1);
                if o.kind == land::K_HOME_SITE {
                    self.club.evict(o.a, o.b);
                    // buying the lot back (0x40e400): its smoothed value / 50 plus half its value now
                    let c = o.val / 50 + sg_core::homes::lot_value(&self.course, &self.club.holes, self.difficulty, o.a, o.b) / 2;
                    self.econ.spend_to(economy::LEDGER_HOME_SITES, c as f64 * Economy::UNIT);
                }
                self.after_object_change();
                self.show_toast("Building demolished");
                self.snd("Interface/Building.wav", 0.7, false);
            }
            land::Undone::Refund(r) => {
                let i = self.terrain.tile_index(x, y);
                let f = land.flags[(x * land::N + y) as usize];
                if f & land::flag::PATH == 0 {
                    self.terrain.path_kind[i] = 0;
                }
                land.write_area(&mut self.terrain, x, y, x, y);
                if r != 0 {
                    let amount = r as f64 * Economy::UNIT;
                    self.econ.earn(amount);
                    self.econ.book(economy::LEDGER_BUILD_COURSE, amount);
                }
                self.after_object_change();
            }
        }
    }

    /// Placing and removing buildings, as the exe does it. A building goes wherever its footprint fits (no water, cliffs, tees,
    /// other buildings or land outside the property under it); it costs its price times (level + 2) / 2 plus a clearing charge for
    /// trees, rocks and water under it, booked to Facilities. Building a kind that already stands again upgrades it (level + 1)
    /// and moves it there, which needs a Country Club (10 holes). It only earns money once a path joins it to the clubhouse.
    /// Benches, flower beds, willows and bridges are tile items (see place_tile_item). Removing goes through undo: items give
    /// back what they cost, buildings are demolished with no refund.
    pub fn edit_building(&mut self, tx: i32, ty: i32, remove: bool) {
        if !self.terrain.inside(tx, ty) {
            return;
        }
        self.ensure_land();
        let theme = self.exe_theme();
        let holes = self.holes.len();
        if remove {
            return self.undo_tile(tx, ty);
        }
        let kind = self.build_idx as i32;
        if matches!(kind, land::K_PATH | land::K_BENCH | land::K_FLOWERS | land::K_WILLOW | land::K_BRIDGE) {
            self.place_tile_item(tx, ty, kind);
            return;
        }
        let Some(land) = self.land.as_mut() else { return };
        land.sync_from_terrain(&self.terrain);
        if !self.build_available(kind as usize) {
            return self.show_toast("Build more holes to unlock this building");
        }
        let Some(land) = self.land.as_mut() else { return };
        let size = land::BUILDINGS[kind as usize].1;
        let existing = if kind >= 6 { land.objects.iter().position(|o| o.kind == kind) } else { None };
        if existing.is_some() && economy::rank(holes) < 2 {
            return self.show_toast("Sorry, upgraded buildings are not available until you build beyond 9 holes.");
        }
        let Some(mut clear) = land.fits(tx, ty, size, kind, theme) else {
            return self.show_toast("There is no room for that building there");
        };
        if kind == 0 {
            clear /= 2;
        }
        let level = existing.map(|i| land.objects[i].sub + 1).unwrap_or(0);
        let mut cost = (land::BUILDINGS[kind as usize].2 * (level + 2) / 2 + clear) as f64 * Economy::UNIT;
        // a landmark: the first owned type still free to place, else the first owned one; (type * 5 + 25) * 2, or nothing
        // the first time a donated type is placed (clearing is not charged)
        let landmark = (0..14)
            .find(|t| self.club.free_landmarks & (1 << t) != 0)
            .or_else(|| (0..14).find(|t| self.club.landmarks_owned & (1 << t) != 0));
        if kind == land::K_LANDMARK {
            let Some(t) = landmark else { return };
            cost = if self.club.free_landmarks & (1 << t) != 0 { 0.0 } else { ((t * 5 + 25) * 2) as f64 * Economy::UNIT };
        }
        if !self.econ.affordable(cost, holes) {
            return self.show_toast(&format!("This change costs {}. You have only {}.", money(cost as i64), money(self.econ.cash as i64)));
        }
        let lot = if kind == land::K_HOME_SITE {
            let v = sg_core::homes::lot_value(&self.course, &self.club.holes, self.difficulty, tx, ty);
            if v < 50 {
                return self.show_toast(
                    "This is not a very attractive location for a building lot. Home buyers like water, woods, and grass near a golf hole with a good fun factor.",
                );
            }
            v / 4
        } else {
            0
        };
        let Some(land) = self.land.as_mut() else { return };
        if let Some(i) = existing {
            let o = land.objects[i];
            let s = land.footprint_size(&o);
            self.club.evict(o.a, o.b);
            land.remove_object(i);
            land.write_area(&mut self.terrain, o.a, o.b, o.a + s - 1, o.b + s - 1);
        }
        let n = land.place(&mut self.exe_rng, tx, ty, kind, 0, theme);
        self.club.log_event(sg_core::records::log::BUILT, kind);
        land.objects[n].sub = level;
        if kind == land::K_LANDMARK {
            let t = landmark.unwrap_or(0);
            land.objects[n].sub = t;
            self.club.free_landmarks &= !(1 << t);
        }
        land.write_area(&mut self.terrain, tx, ty, tx + size - 1, ty + size - 1);
        self.econ.spend_to(economy::LEDGER_FACILITIES, cost);
        if lot > 0 {
            // the lot is sold at once
            self.econ.earn_to(economy::LEDGER_HOME_SITES, lot as f64 * Economy::UNIT);
        }
        self.after_object_change();
        self.snd("Interface/Building.wav", 0.7, false);
    }

    /// After objects change: terrain meshes, trees and object sprites are rebuilt.
    pub(crate) fn after_object_change(&mut self) {
        self.dirty = true;
        self.props.retain(|p| !p.object);
        self.add_land_objects();
        self.sync_course();
    }

    /// The exe's theme number (0 Parkland, 1 Desert, 2 Tropical, 3 Links) of the loaded theme.
    pub fn exe_theme(&self) -> u8 {
        [0, 3, 1, 2][self.theme.min(3)]
    }

    /// Every course gets the exe's object model: generated land has it already; for the demo course or a loaded course file it is
    /// made from the terrain, with the clubhouse as its first object.
    pub fn ensure_land(&mut self) {
        if self.land.is_some() {
            return;
        }
        let mut land = Land::from_terrain(&self.terrain, self.exe_theme());
        if self.terrain.clubhouse_x >= 0 {
            let (a, b) = (self.terrain.clubhouse_x - 2, self.terrain.clubhouse_y - 2);
            land.objects.push(land::Object { kind: land::K_CLUBHOUSE, a, b, dir: 0, flags: 0x40, sub: 0, val: 0 });
            land.clubhouse = (a, b);
            for r in 0..4 {
                for c in 0..4 {
                    let (ta, tb) = (a + r, b + c);
                    if (0..land::N).contains(&ta) && (0..land::N).contains(&tb) {
                        let i = (ta * land::N + tb) as usize;
                        land.flags[i] |= land::flag::FOOTPRINT;
                        land.var[i] = 0;
                    }
                }
            }
        }
        self.land = Some(land);
    }

    pub fn edit_wall(&mut self, remove: bool) {
        // Nearest tile edge to the picked point.
        let (tx, ty) = self.terrain.tile_of(self.hit_x, self.hit_z);
        let fx = (self.hit_x + self.terrain.w as f32 * TILE_SIZE * 0.5) / TILE_SIZE - tx as f32;
        let fz = (self.hit_z + self.terrain.h as f32 * TILE_SIZE * 0.5) / TILE_SIZE - ty as f32;
        let d = [fz, 1.0 - fx, 1.0 - fz, fx]; // distances to N, E, S, W edges
        let dir = (0..4).fold(0, |best, i| if d[i] < d[best] { i } else { best });
        self.terrain.set_wall(tx, ty, dir as i32, !remove);
        self.dirty = true;
        self.snd("Interface/Place Rocks Generic.wav", 0.7, false);
    }

    pub fn edit_raise(&mut self, cx: i32, cy: i32, delta: i32) {
        let r = self.brush;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + r {
                    self.terrain.raise_corner(cx + dx, cy + dy, delta);
                }
            }
        }
        self.dirty = true;
        self.snd(if delta > 0 { "Interface/Bass Up 2.wav" } else { "Interface/Bass Down 2.wav" }, 0.6, false);
    }

    /// Applies the current tool at the picked ground point. During a drag it only fires when the cell changes.
    pub fn apply_tool(&mut self, lower: bool, dragging: bool) {
        if !self.has_hit {
            return;
        }
        let (a, b) =
            if self.tool != 1 { self.terrain.tile_of(self.hit_x, self.hit_z) } else { self.terrain.corner_of(self.hit_x, self.hit_z) };
        let cell = ((b as i64) << 12) | (a as i64 & 0xFFF) | ((self.tool as i64) << 24);
        if dragging && cell == self.last_cell {
            return;
        }
        self.last_cell = cell;
        match self.tool {
            0 => self.edit_paint(a, b),
            2 => self.edit_path(a, b, lower),
            3 => self.edit_wall(lower),
            4 => self.edit_building(a, b, lower),
            _ => {
                let down = lower != (self.raise_sign < 0);
                self.edit_raise(a, b, if down { -1 } else { 1 })
            }
        }
    }

    /// Ground point under a drawable-pixel position: ortho ray against the heightfield.
    pub fn pick_ground(&self, mx: f32, my: f32) -> Option<(f32, f32)> {
        let m: Vec<f64> = self.mv.iter().map(|&v| v as f64).collect();
        let upp = self.upp as f64;
        let ex = (mx as f64 - self.draw_w as f64 * 0.5) * upp;
        let ey = (self.draw_h as f64 * 0.5 - my as f64) * upp;
        let v = [ex - m[12], ey - m[13], -m[14]];
        let mut o = [0f64; 3];
        let mut d = [0f64; 3];
        for j in 0..3 {
            o[j] = m[4 * j] * v[0] + m[4 * j + 1] * v[1] + m[4 * j + 2] * v[2];
            d[j] = -m[4 * j + 2];
        }
        if d[1].abs() < 1e-6 {
            return None;
        }
        // March from above the highest possible ground down along the ray, then bisect the first crossing.
        let top = MAX_LEVEL as f64 * HEIGHT_STEP as f64 + 1.0;
        let at = |s: f64| {
            let (x, z) = (o[0] + d[0] * s, o[2] + d[2] * s);
            (o[1] + d[1] * s - self.terrain.height_at(x as f32, z as f32) as f64, x, z)
        };
        let mut s0 = (top - o[1]) / d[1];
        let mut s1 = s0;
        let span = top / d[1].abs();
        const STEPS: i32 = 120;
        let mut hit = false;
        for i in 1..=STEPS {
            s1 = s0 + span * i as f64 / STEPS as f64;
            if at(s1).0 <= 0.0 {
                hit = true;
                break;
            }
            s0 = s1;
        }
        // As in the C++ port, each step starts from the previous sample, so the step length grows along the ray.
        if !hit {
            return None;
        }
        for _ in 0..24 {
            let mid = 0.5 * (s0 + s1);
            if at(mid).0 > 0.0 {
                s0 = mid;
            } else {
                s1 = mid;
            }
        }
        let (_, x, z) = at(s1);
        if x.abs() > self.terrain.w as f64 * TILE_SIZE as f64 * 0.5 || z.abs() > self.terrain.h as f64 * TILE_SIZE as f64 * 0.5 {
            return None;
        }
        Some((x as f32, z as f32))
    }

    pub fn pan(&mut self, right: f32, up: f32) {
        let yaw = (45.0 + self.rot) * std::f32::consts::PI / 180.0;
        self.cam_x += yaw.cos() * right + yaw.sin() * up;
        self.cam_z += yaw.sin() * right - yaw.cos() * up;
    }

    /// The edit tool line (the C++ port showed it in the window title; Tab exits, T tool, [ ] type, , . brush, shift lowers or removes).
    pub fn edit_status(&self) -> String {
        let tool = match self.tool {
            0 => "Paint",
            1 => "Raise/Lower",
            2 => "Path",
            3 => "Wall",
            _ => "Building",
        };
        let what = match self.tool {
            0 => PAINT[self.paint_idx].name,
            2 => {
                if self.path_kind == 1 {
                    "gravel"
                } else {
                    "paved"
                }
            }
            3 => "edge",
            4 => land::BUILDINGS.get(self.build_idx).map(|b| b.0).unwrap_or("?"),
            _ => "terrain",
        };
        format!("Edit: {tool}, {what}, brush {}", self.brush * 2 + 1)
    }
}

/// Original pitch angles, chosen per resolution in Terrain::initSystem (38.68, 40.54, 40.83 degrees).
pub fn pitch_for(w: f32, h: f32) -> f64 {
    if w == 800.0 && h == 600.0 {
        38.682186
    } else if w == 1280.0 && h == 1024.0 {
        40.832218
    } else {
        40.541603
    }
}

/// Milliseconds from the system clock, wrapped to 32 bits like the Windows tick count.
pub fn clock_ms() -> u32 {
    (miniquad::date::now() * 1000.0) as u64 as u32
}

fn slots_of(offer: &[Slot; 16]) -> [usize; 16] {
    let mut out = [0; 16];
    for (i, s) in offer.iter().enumerate() {
        out[s.property] = i;
    }
    out
}

impl App {
    pub fn units_to_world(&self, x: i32, y: i32) -> (f32, f32) {
        let k = TILE_SIZE / staff::UNIT as f32;
        (x as f32 * k - self.terrain.w as f32 * TILE_SIZE * 0.5, y as f32 * k - self.terrain.h as f32 * TILE_SIZE * 0.5)
    }

    /// Clubhouse anchor tile (the exe's first object), from the generated land or the demo course's clubhouse.
    pub fn club_anchor(&self) -> (i32, i32) {
        match &self.land {
            Some(l) => l.clubhouse,
            None => (self.terrain.clubhouse_x - 2, self.terrain.clubhouse_y - 2),
        }
    }

    /// Resets the staff for a new course: the player's own pro starts at the clubhouse, posted on its anchor tile, and the
    /// generator's creek and garden tiles can start weeds.
    pub fn reset_staff(&mut self) {
        self.employees.clear();
        self.staff_tiles = TileState::new(self.terrain.w, self.terrain.h);
        if let Some(l) = &self.land {
            for a in 0..land::N {
                for b in 0..land::N {
                    if l.flags[(a * land::N + b) as usize] & land::flag::CREEK != 0 {
                        let i = (b * self.terrain.w + a) as usize;
                        self.staff_tiles.flags[i] |= staff::SEEDS_WEEDS;
                    }
                }
            }
        }
        let club = self.club_anchor();
        if self.terrain.clubhouse_x >= 0 {
            let i = staff::create(&mut self.employees, staff::job::OWNER, club, &mut self.exe_rng);
            self.employees[i].post = Some(club);
        }
        self.game_tick = 0;
        self.tick_acc = 0.0;
    }

    /// Hires an employee of kind 0..3 (Club Pro, Ranger, Groundskeeper, Soda Vendor).
    pub fn hire_staff(&mut self, kind: usize) -> bool {
        if !self.econ.hire(kind) {
            return false;
        }
        let club = self.club_anchor();
        staff::hire(&mut self.employees, kind, false, club, &mut self.exe_rng);
        true
    }

    /// Fires the most recently hired employee of a kind: the exe removes the employee at once and charges 25 units under
    /// Salaries.
    pub fn fire_staff(&mut self, kind: usize) -> bool {
        if !self.econ.fire(kind) {
            return false;
        }
        let job = -2 - kind as i8;
        if let Some(e) = self.employees.iter_mut().rev().find(|e| e.active && e.job == job) {
            e.active = false;
        }
        true
    }

    /// One game tick of staff work and weeds.
    fn staff_tick(&mut self) {
        self.game_tick = self.game_tick.wrapping_add(1);
        let payroll: Vec<economy::Payroll> = self
            .employees
            .iter()
            // no wages in a championship
            .filter(|e| e.active && e.job != staff::job::OWNER && !self.club.championship())
            .map(|e| economy::Payroll { kind: (-2 - e.job as i32).clamp(0, 3) as usize, experienced: e.upgraded })
            .collect();
        let holes = self.holes.len();
        self.econ.on_tick(self.game_tick, self.difficulty, holes, &payroll, &mut self.exe_rng);
        if self.staff_tiles.flags.len() != (self.terrain.w * self.terrain.h) as usize {
            self.staff_tiles = TileState::new(self.terrain.w, self.terrain.h);
        }
        // What the staff see of each golfer: the exe's staff routines read the golfer records directly.
        self.staff_golfers.resize(golf::SLOTS, StaffGolfer { face: -1, ..Default::default() });
        for gi in 0..golf::SLOTS {
            let g = &self.club.g[gi];
            let sg = &mut self.staff_golfers[gi];
            if g.hole <= 0 {
                *sg = StaffGolfer { face: -1, ..Default::default() };
                continue;
            }
            sg.present = true;
            sg.x = g.x;
            sg.y = g.y;
            sg.hunger = g.hunger;
            sg.thirst = g.thirst;
            sg.fatigue = g.fatigue;
            sg.hurried = g.flags & golf::flag::HURRIED != 0;
            sg.busy = g.strokes > 1;
            sg.last_pro_event = g.thoughts.iter().copied().find(|&t| t == 0x22 || t == 0x3a).unwrap_or(0) as u32;
            sg.pause = g.pause;
            sg.face = -1;
        }
        let mut out = Vec::new();
        {
            let weed_frames = self.weed_sprite.map(|w| self.sprites[w].s.frames_per_view).unwrap_or(1);
            let club = self.club_anchor();
            let mut world = staff::World {
                terrain: &self.terrain,
                tiles: &mut self.staff_tiles,
                golfers: &mut self.staff_golfers,
                club,
                clicked: if self.club.holes[1].par == 0 { self.hover_tile } else { None },
                tick: self.game_tick,
                difficulty: self.difficulty,
                weed_frames,
            };
            // the pro's employee is away while he plays a round (staff routine 0x402a40)
            let away = self.club.gary != -1;
            if away {
                for e in self.employees.iter_mut().filter(|e| e.job == staff::job::OWNER) {
                    e.active = false;
                }
            }
            staff::tick(&mut self.employees, &mut world, &mut self.exe_rng, &mut out);
            if away {
                for e in self.employees.iter_mut().filter(|e| e.job == staff::job::OWNER) {
                    e.active = true;
                }
            }
            staff::spread_weeds(&self.terrain, &mut self.staff_tiles, self.game_tick, self.difficulty, &mut self.exe_rng, &mut out);
            staff::grow_weeds(&mut self.staff_tiles, &mut self.exe_rng);
        }
        for gi in 0..golf::SLOTS {
            if self.club.g[gi].hole <= 0 {
                continue;
            }
            let sg = self.staff_golfers[gi];
            let g = &mut self.club.g[gi];
            g.pause = sg.pause;
            if sg.face >= 0 {
                g.facing = sg.face as i32;
            }
            if sg.hurried {
                g.flags |= golf::flag::HURRIED;
            }
        }
        for ev in out {
            match ev {
                StaffEvent::Mood { golfer, event, arg, counter_ok } => {
                    if self.club.g.get(golfer).map(|g| g.hole > 0).unwrap_or(false) {
                        if event == sg_core::mood::ev::DRINK {
                            // the drink's bonus reads the thirst at the sale (the experienced vendor's always counts);
                            // the vendor's zero is copied back below
                            self.club.g[golfer].thirst = if counter_ok { 99 } else { 0 };
                        }
                        self.club.event(&mut self.course, &mut self.exe_rng, golfer, event, arg);
                    }
                }
                StaffEvent::DrinkSold { golfer } => {
                    self.econ.earn_to(economy::LEDGER_FOOD_DRINK, 2.0 * Economy::UNIT);
                    println!("[{:6.1}s] soda vendor sold golfer {golfer} a drink", self.sim_time);
                }
                StaffEvent::Sound { slot, x, y } => {
                    let (wx, wz) = self.units_to_world(x, y);
                    self.slot_sound(slot, wx, wz);
                }
                StaffEvent::WeedPulled { a, b } => println!("[{:6.1}s] groundskeeper pulled a weed at {a},{b}", self.sim_time),
                StaffEvent::Left { .. } => {}
            }
        }
        // thirst last, after the drink's mood event
        for gi in 0..golf::SLOTS {
            if self.club.g[gi].hole > 0 {
                self.club.g[gi].thirst = self.staff_golfers[gi].thirst;
            }
        }
    }

    /// Loads the employee clips (the exe's sprite sets, in its order) and the theme's weed.
    pub fn load_staff_sprites(&mut self) {
        const SETS: [&str; 8] = [
            "GreeterWalk|GreeterSQ|GreeterAction2",
            "RangerWalk|RangerSQ|RangerAction",
            "GKWalk|GKSQ|GKAction",
            "TrayGirl_Walk|TrayGirl_SQ|TrayGirl_Action",
            "GolfCeleb_Walk|GolfCeleb_SQ|GolfCeleb_Action",
            "Marshall_Walk|Marshall_SQ|Marshall_Action",
            "LawnTech_Walk|LawnTech_Sq|LawnTech_Action",
            "SodaVendorWalk|SodaVendorSQ|SodaVendorAction",
        ];
        const PALS: [&str; 8] =
            ["GreeterPal", "RangerPal", "GKPal", "TrayGirlPal", "GolfCelebPal", "MarshallPal", "LawnTechPal", "SodaVendorPal"];
        for (k, set) in SETS.iter().enumerate() {
            let pal = format!("Employee/{}.pcx", PALS[k]);
            for (m, name) in set.split('|').enumerate() {
                let body = self.sprite_for(&format!("Employee/{name}.flc"), false, Some(&pal));
                let shadow = self.sprite_for(&format!("Employee/{name}Shadow.flc"), true, None);
                self.staff_clips[k][m] = (body, shadow);
            }
        }
        // The player's pro uses a golfer's clips: walk, stand and the happy gesture.
        for (m, name) in ["Male/MaleKLS_NormalWalk", "Male/MaleKLS_Sq", "Male/MaleKLS_Happy"].iter().enumerate() {
            let body = self.sprite_for(&format!("{name}.flc"), false, None);
            let shadow = self.sprite_for(&format!("{name}Shadow.flc"), true, None);
            self.staff_clips[8][m] = (body, shadow);
        }
        let weed = match self.theme {
            2 => "Flowers/OilSlick.flc",
            3 => "Flowers/DryGrass.flc",
            _ => "Flowers/dandelion_01.flc",
        };
        self.weed_sprite = self.sprite_for(weed, false, None);
    }

    /// Points the employee props at the employees: clip by job and state, frame advanced like the exe's (two a frame when
    /// walking), facing from the employee's direction.
    pub fn update_staff_props(&mut self) {
        let want = self.employees.len();
        let have = self.props.iter().filter(|p| p.employee.is_some()).count();
        for i in have..want {
            self.props.push(Prop { employee: Some(i), hidden: true, ..Default::default() });
        }
        for pi in 0..self.props.len() {
            let Some(ei) = self.props[pi].employee else { continue };
            let Some(e) = self.employees.get(ei).copied() else {
                self.props[pi].hidden = true;
                continue;
            };
            if !e.active || (e.job == staff::job::OWNER && self.club.gary != -1) {
                self.props[pi].hidden = true;
                continue;
            }
            let set =
                if e.job == staff::job::OWNER { 8 } else { ((-(e.job as i32) - 2) + if e.upgraded { 4 } else { 0 }).clamp(0, 7) as usize };
            let state = if e.anim < staff::ANIM_STAND {
                0
            } else if e.anim == staff::ANIM_STAND {
                1
            } else {
                2
            };
            let (body, shadow) = self.staff_clips[set][state];
            let Some(b) = body else {
                self.props[pi].hidden = true;
                continue;
            };
            let n = self.sprites[b].s.frames_per_view.max(1);
            let step = if state == 0 { 2 } else { 1 };
            let e = &mut self.employees[ei];
            e.frame = e.frame.wrapping_add(step);
            let f = e.frame as i32 % n;
            if f == 0 && e.anim == staff::ANIM_ACTION {
                e.anim = staff::ANIM_STAND;
            }
            const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
            const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];
            let k = (e.dir as i32 & 7) as usize;
            let (x, z) = (e.x, e.y);
            let (wx, wz) = self.units_to_world(x, z);
            let p = &mut self.props[pi];
            p.hidden = false;
            p.body = Some(b);
            p.shadow = shadow;
            p.x = wx;
            p.z = wz;
            p.frame = f;
            p.heading = DY[k].atan2(DX[k]).to_degrees();
        }
    }
}

impl App {
    /// The celebrity residents walk about their lots, one step a game tick (the exe steps them every frame), with the
    /// cosmetic generator so the club's own draws stay as they are.
    fn resident_tick(&mut self) {
        let tournament = self.club.game & golf::game::TOURNAMENT != 0;
        for i in 0..self.club.residents.len() {
            let r = self.club.residents[i];
            let name = sg_core::celebs::TYPES[r.kind.clamp(0, 11) as usize];
            let suffix = match r.state {
                sg_core::celebs::WALK => "Walk",
                sg_core::celebs::ACTION => "Char",
                _ => "SQ",
            };
            let frames = self
                .sprite_for(&format!("Celebs/{name}_{suffix}.flc"), false, None)
                .map(|s| self.sprites[s].s.frames_per_view)
                .unwrap_or(1);
            let voice = self.club.residents[i].step(&mut self.deco_rng, frames, true, self.paused, tournament);
            if let Some(slot) = voice {
                let (wx, wz) = self.units_to_world(r.world().0, r.world().1);
                self.slot_sound(slot, wx, wz);
            }
        }
    }

    fn update_resident_props(&mut self) {
        let want = self.club.residents.len();
        self.props.retain(|p| p.resident.is_none_or(|i| i < want));
        let have = self.props.iter().filter(|p| p.resident.is_some()).count();
        for i in have..want {
            self.props.push(Prop { resident: Some(i), hidden: true, ..Default::default() });
        }
        // where each golfer is on screen, or -1 when off it (the exe's drawing records this; a golfer who has finished and is
        // out of sight goes straight home)
        for gi in 0..golf::SLOTS {
            if self.club.g[gi].hole == 0 {
                continue;
            }
            let (x, y) = (self.club.g[gi].x, self.club.g[gi].y);
            let (sx, sy) = self.screen_of(x, y).map(|(a, b)| (a as i32, b as i32)).unwrap_or((-1, -1));
            self.club.g[gi].sx = sx;
            self.club.g[gi].sy = sy;
        }
        const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
        const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];
        for pi in 0..self.props.len() {
            let Some(ri) = self.props[pi].resident else { continue };
            let r = self.club.residents[ri];
            let name = sg_core::celebs::TYPES[r.kind.clamp(0, 11) as usize];
            let suffix = match r.state {
                sg_core::celebs::WALK => "Walk",
                sg_core::celebs::ACTION => "Char",
                _ => "SQ",
            };
            let body = self.sprite_for(&format!("Celebs/{name}_{suffix}.flc"), false, None);
            let shadow = self.sprite_for(&format!("Celebs/{name}_{suffix}Shadow.flc"), true, None);
            let (wx, wz) = self.units_to_world(r.world().0, r.world().1);
            let k = (r.facing & 7) as usize;
            let p = &mut self.props[pi];
            p.hidden = body.is_none() || r.hold >= 128;
            p.body = body;
            p.shadow = shadow;
            p.x = wx;
            p.z = wz;
            p.frame = r.frame;
            p.heading = DY[k].atan2(DX[k]).to_degrees();
        }
    }

    /// The generated land's objects as the exe draws them: landmarks (sprite 0x168 + type, animated, facing one of four ways)
    /// and buildings (layers by kind, theme and level; level 2 once the course has more than 10 holes), each standing in the
    /// middle of its footprint.
    fn add_land_objects(&mut self) {
        let Some(land) = self.land.clone() else { return };
        let theme = land.slot.record().theme;
        // Tile items: benches and ornamental trees as the exe draws them (sg_core::decor); flower beds INTERIM: the single-bed
        // sprite of the theme (the exe picks the bed's shape from its neighbours through a table not decoded here).
        let beds = match theme {
            1 => "flowers/DesFlowers_Single",
            2 => "flowers/TropFlowers_Single",
            _ => "flowers/Flowers_Single",
        };
        let noise = self.noise.clone();
        let field = |x: i32, y: i32| noise.field(x, y);
        let mut draws: Vec<(i32, i32, sg_core::decor::Draw)> = Vec::new();
        for a in 0..land::N {
            for b in 0..land::N {
                let i = (a * land::N + b) as usize;
                let f = land.flags[i];
                if f & 0x20 != 0 && land.ty[i] == land::T_WATER {
                    // bridge pieces on a path over water
                    let (mut path, mut dry, mut water_path) = (0u8, 0u8, false);
                    for k in 0..4 {
                        let (na, nb) = (a + sg_core::geom::DX[2 * k], b + sg_core::geom::DY[2 * k]);
                        if !(0..land::N).contains(&na) || !(0..land::N).contains(&nb) {
                            continue;
                        }
                        let j = (na * land::N + nb) as usize;
                        if land.flags[j] & 0x20 != 0 {
                            path |= 1 << k;
                            if land.ty[j] == land::T_WATER {
                                water_path = true;
                            } else {
                                dry |= 1 << k;
                            }
                        }
                    }
                    for d in sg_core::decor::bridge(path, dry, water_path, f & 0x100 != 0, (f & 0x1f) as u8, 0, 1) {
                        draws.push((a, b, d));
                    }
                }
                if f & 0x200 != 0 {
                    let course = &self.course;
                    for d in sg_core::decor::benches(land.var[i], &|d| course.bench_ok(a, b, d as i32)) {
                        draws.push((a, b, d));
                    }
                } else if land.ty[i] == land::T_ELM && f & 0x100 != 0 {
                    let grown = !self.tile_growing(a, b);
                    for d in sg_core::decor::ornamental(land.var[i], a, b, grown, &field) {
                        draws.push((a, b, d));
                    }
                } else if f & 0x1000 != 0 && land.ty[i] != land::T_OUT {
                    let body = self.sprite_for(&format!("{beds}.flc"), false, None);
                    if body.is_some() {
                        let shadow = self.sprite_for(&format!("{beds}Shadow.flc"), true, None);
                        let (x, z) = self.terrain.tile_centre(a, b);
                        self.props.push(Prop { x, z, body, shadow, facing: (land.var[i] & 3) as i32, object: true, ..Default::default() });
                    }
                }
            }
        }
        for (a, b, d) in draws {
            self.push_decor(a, b, d, true, false);
        }
        let level = (self.holes.len() > 10) as u16;
        for (oi, o) in land.objects.iter().enumerate().filter(|(_, o)| o.kind >= 0) {
            // land outside the property is not drawn, nor what stands on it (an obstacle placed before the border was
            // taken away shows again when the tract is bought)
            if self.terrain.type_at(o.a, o.b) == 20 {
                continue;
            }
            let size = land::BUILDINGS.get(o.kind as usize).map(|b| b.1).unwrap_or(1);
            let (cx, cz) = self.terrain.tile_centre(o.a, o.b);
            let off = (size - 1) as f32 * TILE_SIZE * 0.5;
            let (x, z) = (cx + off, cz + off);
            let layers: Vec<(String, bool, bool)> = if o.kind == land::K_HOME_SITE && o.sub != 0 {
                // a celebrity's vacation home: one of two houses by the object slot's parity (0x4012d0)
                let (house, dirt) = sg_core::celebs::HOUSES[oi & 1];
                vec![(dirt.to_string(), true, false), (house.to_string(), false, false)]
            } else if o.kind == land::K_HOME_SITE {
                let id = match sg_core::homes::house_size(o.val) {
                    0 => 0x1c8,
                    1 => 0x1c9,
                    _ => 0x1ca,
                };
                match sg_core::objects::sprite(id, theme) {
                    Some(f) => vec![(f.to_string(), false, false)],
                    None => continue,
                }
            } else if o.kind == land::K_LANDMARK {
                match sg_core::objects::LANDMARKS.get(o.sub as usize) {
                    Some(f) => vec![(f.to_string(), false, true)],
                    None => continue,
                }
            } else {
                sg_core::objects::building_layers(o.kind, level, theme).iter().map(|l| (l.file.to_string(), l.flat, l.animated)).collect()
            };
            for (file, flat, animated) in layers {
                let body = self.sprite_for(&format!("{file}.flc"), false, None);
                if body.is_none() {
                    continue;
                }
                let shadow = if flat { None } else { self.sprite_for(&format!("{file}Shadow.flc"), true, None) };
                self.props.push(Prop { x, z, body, shadow, flat, facing: o.dir as i32, animated, object: true, ..Default::default() });
            }
        }
    }
}

impl App {
    /// Weed sprites on the weed tiles: a growing weed shows the frame of its counter, a grown one the last frame.
    fn update_weed_props(&mut self) {
        self.props.retain(|p| !p.weed);
        let t = &self.staff_tiles;
        let mut add = Vec::new();
        for b in 0..t.h {
            for a in 0..t.w {
                let i = (b * t.w + a) as usize;
                if t.flags[i] & staff::WEEDS == 0 {
                    continue;
                }
                let counter = if t.flags[i] & staff::WORKED != 0 { Some(t.counter[i] as i32) } else { None };
                add.push((a, b, counter));
            }
        }
        for (a, b, counter) in add {
            let ty = if sg_core::course::inside(a, b) { self.course.ty[sg_core::course::idx(a, b)] } else { 4 };
            let probe = sg_core::decor::weed(ty, a, b, 1, None);
            let (body, _) = self.decor_sprite(probe.sprite, probe.pal);
            let Some(w) = body else { continue };
            let n = self.sprites[w].s.frames_per_view.max(1);
            let d = sg_core::decor::weed(ty, a, b, n, counter);
            let (x, z) = self.terrain.tile_centre(a, b);
            self.props.push(Prop {
                x,
                z,
                body: Some(w),
                frame: d.frame.unwrap_or(n - 1).clamp(0, n - 1),
                weed: true,
                ..Default::default()
            });
        }
    }
}

/// Names of the employee kinds, basic and experienced, as the exe shows them.
pub const STAFF_NAMES: [[&str; 2]; 4] =
    [["Club Pro", "Celebrity"], ["Ranger", "Marshall"], ["Groundskeeper", "Technician"], ["Soda Vendor", "Refresher"]];

impl App {
    /// A left click on the course: drop an employee being moved on that tile, or pick up a hired employee standing at the click,
    /// or else send the player's own pro there.
    pub fn course_click(&mut self, wx: f32, wz: f32) {
        let t = self.terrain.tile_of(wx, wz);
        if !self.terrain.inside(t.0, t.1) {
            return;
        }
        // a click on the cup green of a hole not yet open opens it, as in the exe
        if self.moving_employee.is_none() && sg_core::course::inside(t.0, t.1) {
            let fl = self.course.flags[sg_core::course::idx(t.0, t.1)];
            let h = (fl & 0x1f) as usize;
            if fl & sg_core::course::f::CUP != 0 && h > 0 && h < 19 && self.club.holes[h].par == 0 {
                self.open_hole();
                return;
            }
        }
        if self.moving_employee.is_none() && sg_core::course::inside(t.0, t.1) {
            let o = self.course.object_at(t.0, t.1);
            if o >= 0 && self.course.objects.get(o as usize).is_some_and(|ob| ob.kind == land::K_CLUBHOUSE) && self.open_pair_screen() {
                return;
            }
        }
        if let Some(i) = self.moving_employee.take() {
            if let Some(e) = self.employees.get_mut(i) {
                e.post = Some(t);
                e.steps = 0;
                let name = STAFF_NAMES[(-2 - e.job as i32).clamp(0, 3) as usize][e.upgraded as usize];
                self.show_toast(&format!("{name} will work around here"));
            }
            return;
        }
        let mut pick = None;
        for (i, e) in self.employees.iter().enumerate() {
            if !e.active || e.job == staff::job::OWNER {
                continue;
            }
            let (ex, ez) = self.units_to_world(e.x, e.y);
            if (ex - wx).hypot(ez - wz) < 70.0 {
                pick = Some((i, STAFF_NAMES[(-2 - e.job as i32).clamp(0, 3) as usize][e.upgraded as usize]));
                break;
            }
        }
        if let Some((i, name)) = pick {
            self.moving_employee = Some(i);
            self.show_toast(&format!("Click the course where the {name} should work"));
            return;
        }
        self.clicked_tile = Some(t);
    }
}
