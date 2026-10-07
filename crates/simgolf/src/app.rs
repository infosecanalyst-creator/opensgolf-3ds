//! The game state and rules that sit above sg-core: the course and its scenery, the golfers on it, money, editing, the in-game
//! panels and the story banner. Drawing lives in render.rs, input and the window in main.rs.
use crate::gfx::{Gfx, Mesh, Vert};
use crate::ui::Image;
use miniquad::TextureId;
use sg_core::economy::{self, Economy};
use sg_core::fsutil::resolve;
use sg_core::holes::*;
use sg_core::land::{self, ExeRng, Land, Noise, Slot};
use sg_core::mixer::Mixer;
use sg_core::mood;
use sg_core::properties::{PROPERTIES, START_FUNDS};
use sg_core::rng::Rng;
use sg_core::shot::{GolferAnim, GolferSkills, ShotSim};
use sg_core::sprites::{load_sprite, Sprite};
use sg_core::terrain::*;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const THEMES: [&str; 4] = ["Parkland", "Links", "Desert", "Tropical"];
pub const MAX_GOLFERS: usize = 8;
pub const LOOKS: usize = 4;

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
    /// A pool golfer who is not on the course.
    pub hidden: bool,
    /// Scenery regenerated from the terrain after edits.
    pub tree: bool,
    /// An amenity placed by the player (kept when props are regenerated).
    pub building: bool,
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
            golfer: None,
            hidden: false,
            tree: false,
            building: false,
        }
    }
}

/// One golfer on the course. A fixed pool is allocated.
#[derive(Clone, Debug, Default)]
pub struct Golfer {
    pub sim: ShotSim,
    pub active: bool,
    pub look: usize,
    pub hole: usize,
    pub strokes_round: i32,
    pub last_stroke: i32,
    pub last_event: &'static str,
    pub hole_start: f64,
    /// The golfer's mood value, which sets the green fee (the exe keeps it per golfer as a small integer).
    pub mood: i32,
    /// Hunger and thirst counters, as the exe keeps them per golfer (docs/PUBLISHER_EXE_NOTES.md, "Golfer needs").
    pub hunger: i32,
    pub thirst: i32,
    /// Seconds until the next needs update.
    pub needs_clock: f64,
}

/// Per hole statistics for the course report (reset when the course changes shape).
#[derive(Clone, Copy, Debug, Default)]
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

/// Amenities. Sprite names are the level 1 files found per theme in Flics/Bldgs; costs are PLACEHOLDERS (the exe's building costs
/// are not decoded). A Snack Bar visit pays 5 units, which is the exe's figure for building type 7 (assumed to be the snack bar).
pub struct BuildDef {
    pub name: &'static str,
    pub sprite: [Option<&'static str>; 4],
    pub cost: i32,
    pub visit: i32,
}

pub const BUILD: [BuildDef; 6] = [
    BuildDef {
        name: "Snack Bar",
        sprite: [Some("Bldgs/Park/ParkSnackL1"), None, None, Some("Bldgs/Tropical/TROPsnackL1")],
        cost: 10,
        visit: 5,
    },
    BuildDef {
        name: "Pro Shop",
        sprite: [Some("Bldgs/Park/ProsL1"), None, Some("Bldgs/Desert/dproL1"), Some("Bldgs/Tropical/TROPproshopL1")],
        cost: 30,
        visit: 0,
    },
    BuildDef {
        name: "Cart Garage",
        sprite: [
            Some("Bldgs/Park/cartL1"),
            Some("Bldgs/links/Cart_garageL1"),
            Some("Bldgs/Desert/DEScartL1"),
            Some("Bldgs/Tropical/TROPcartL1"),
        ],
        cost: 20,
        visit: 0,
    },
    BuildDef {
        name: "Hotel",
        sprite: [
            Some("Bldgs/Park/HotelL1"),
            Some("Bldgs/links/HotelL1"),
            Some("Bldgs/Desert/DesHotelL1"),
            Some("Bldgs/Tropical/TROPhotelL1"),
        ],
        cost: 50,
        visit: 0,
    },
    BuildDef { name: "Tennis Court", sprite: [Some("Bldgs/Park/tenL1"), None, Some("Bldgs/Desert/tenL1"), None], cost: 25, visit: 0 },
    BuildDef { name: "Marina", sprite: [Some("Bldgs/Park/MarL1"), None, None, Some("Bldgs/Tropical/TROPmarL1")], cost: 40, visit: 0 },
];

#[derive(Clone, Copy, Debug)]
pub struct Placed {
    pub def: usize,
    pub tx: i32,
    pub ty: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Property,
    Play,
    Report,
}

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
    pub golfers: Vec<Golfer>,
    /// Found from the painted tees and greens.
    pub holes: Vec<HoleRoute>,
    pub sim_time: f64,
    pub spawn_timer: f64,
    /// [look][GolferAnim]
    pub look_body: [[Option<usize>; 6]; LOOKS],
    pub look_shadow: [[Option<usize>; 6]; LOOKS],
    pub look_ok: [bool; LOOKS],
    pub rounds_started: u32,
    /// 0..3 as in the exe (the standard game's value is not known yet; 1 is a guess).
    pub difficulty: i32,
    /// The exe's random number generator (land, offer) and the height noise it fills once at start-up.
    pub exe_rng: ExeRng,
    pub noise: Noise,
    /// This game's deal of properties to offer slots, and the slot each property is in.
    pub offer: [Slot; 16],
    pub slot_of: [usize; 16],
    /// The land of the current game, when it was generated (None for the demo course or a loaded course file).
    pub land: Option<Land>,
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
    pub story_pos: usize,
    pub story_next: f64,
    pub hole_stats: Vec<HoleStat>,
    pub ratings: Vec<HoleRating>,
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
    pub build_idx: usize,
    pub buildings: Vec<Placed>,
    /// = selects raising, - selects lowering (the original's hotkeys); shift flips it.
    pub raise_sign: i32,
    pub paused: bool,
    pub paint_idx: usize,
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
            golfers: Vec::new(),
            holes: Vec::new(),
            sim_time: 0.0,
            spawn_timer: 1e9,
            look_body: [[None; 6]; LOOKS],
            look_shadow: [[None; 6]; LOOKS],
            look_ok: [false; LOOKS],
            rounds_started: 0,
            difficulty: 1,
            exe_rng,
            noise,
            offer,
            slot_of,
            land: None,
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
            story_pos: 0,
            story_next: 0.0,
            hole_stats: Vec::new(),
            ratings: Vec::new(),
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
            build_idx: 0,
            buildings: Vec::new(),
            raise_sign: 1,
            paused: false,
            paint_idx: 0,
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
        let id = std::fs::read(path)
            .ok()
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

    fn add_trees(&mut self) {
        const TREES: [[&str; 6]; 4] = [
            [
                "Trees/TreeMapleLarge",
                "Trees/TreeMapleMedium",
                "Trees/TreePineLarge",
                "Trees/TreePineMedium",
                "Trees/TreePineFirLg",
                "Trees/TreeMapleSmall",
            ],
            [
                "Trees/Links/LinksPine_Tall",
                "Trees/Links/LinksPine_Med",
                "Trees/Links/LinksTree3_Tall",
                "Trees/Links/LinksTree3_Med",
                "Trees/Links/LinksTree4_Tall",
                "Trees/Links/LinksTree4_Med",
            ],
            [
                "Trees/Desert/JoshuaTree_Lg",
                "Trees/Desert/JoshuaTree_Md",
                "Trees/Desert/CactusA_Lg",
                "Trees/Desert/CactusB_Md",
                "Trees/Desert/TreeCactusLg",
                "Trees/Desert/CactusC_Lg",
            ],
            [
                "Trees/Tropic/TreePalm/TreePalmLg",
                "Trees/Tropic/TreePalm/TreePalmMed",
                "Trees/Tropic/Tree_Cerc/Cerc_Large",
                "Trees/Tropic/Tree_Drac/Drac_Large",
                "Trees/Tropic/Tree_Tall_Palm/TallPalm_Large",
                "Trees/Tropic/Tree_Cerc/Cerc_Med",
            ],
        ];
        for ty in 0..self.terrain.h {
            for tx in 0..self.terrain.w {
                // Tile types 13..16 (tree, pine, palm, elm) all draw the Woods texture and carry trees.
                if !(TT_WOODS..=16).contains(&self.terrain.ty[self.terrain.tile_index(tx, ty)]) {
                    continue;
                }
                let mut rng = Rng::new(
                    self.seed.wrapping_mul(2654435761).wrapping_add(((ty * 64 + tx) as u32).wrapping_mul(40503)).wrapping_add(12345),
                );
                rng.next();
                rng.next();
                let n = 1 + rng.range(2);
                for _ in 0..n {
                    let base = TREES[self.theme][rng.range(6) as usize];
                    let (cx, cz) = self.terrain.tile_centre(tx, ty);
                    let mut p = Prop { tree: true, x: cx, z: cz, ..Default::default() };
                    p.x += (rng.unit() - 0.5) * 80.0;
                    p.z += (rng.unit() - 0.5) * 80.0;
                    // Colour variants live in separate palette files next to the sprites.
                    let pal = if base.contains("Tropic/TreePalm/") {
                        Some("Trees/Tropic/TreePalm/PalGreenPalm.pcx")
                    } else if base.contains("Tree_Cerc") {
                        Some("Trees/Tropic/Tree_Cerc/PalCerc.pcx")
                    } else if base.contains("Tree_Drac") {
                        Some("Trees/Tropic/Tree_Drac/PalDrac.pcx")
                    } else if base.contains("Tree_Tall_Palm") {
                        Some("Trees/Tropic/Tree_Tall_Palm/PalTallPalm.pcx")
                    } else {
                        None
                    };
                    p.body = self.sprite_for(&format!("{base}.flc"), false, pal);
                    p.shadow = self.sprite_for(&format!("{base}Shadow.flc"), true, None);
                    let Some(b) = p.body else { continue };
                    p.frame = self.sprites[b].s.frames_per_view - 1; // fully grown
                    self.props.push(p);
                }
            }
        }
    }

    pub fn refresh_trees(&mut self) {
        self.props.retain(|p| !p.tree);
        // add_trees appends; keep trees first so painter's order ties stay as before.
        let before = self.props.len();
        self.add_trees();
        self.props.rotate_left(before);
    }

    fn add_building_prop(&mut self, b: Placed) {
        let Some(base) = BUILD[b.def].sprite[self.theme] else { return };
        let (x, z) = self.terrain.tile_centre(b.tx, b.ty);
        let mut p = Prop { building: true, x, z, ..Default::default() };
        p.body = self.sprite_for(&format!("{base}.flc"), false, None);
        p.shadow = self.sprite_for(&format!("{base}Shadow.flc"), true, None);
        if p.body.is_some() {
            self.props.push(p);
        }
    }

    pub fn build_available(&self, d: usize) -> bool {
        BUILD[d].sprite[self.theme].is_some()
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
        for b in self.buildings.clone() {
            self.add_building_prop(b);
        }
        if self.terrain.clubhouse_x >= 0 {
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
        // The pool of golfers (their sprites differ for variety). Golfers arrive over time, see step_game().
        const LOOK: [&str; LOOKS] = ["Male/MaleKLS", "Male/MalePLS", "Female/FemalePLS", "Female/FemaleSSS"];
        const ANIM: [&str; 6] = ["_NormalWalk", "_NormalAddress", "_PerfectSwing", "_PuttAddress", "_Putt", "_Happy"];
        for l in 0..LOOKS {
            self.look_ok[l] = true;
            for i in 0..6 {
                let anim = if i == 2 && l >= 2 { "_NormalSwing" } else { ANIM[i] }; // the women have no PerfectSwing clip
                self.look_body[l][i] = self.sprite_for(&format!("{}{anim}.flc", LOOK[l]), false, None);
                self.look_shadow[l][i] = self.sprite_for(&format!("{}{anim}Shadow.flc", LOOK[l]), true, None);
                if self.look_body[l][i].is_none() {
                    self.look_ok[l] = false;
                }
            }
        }
        self.holes = find_holes(&self.terrain);
        self.golfers = vec![Golfer::default(); MAX_GOLFERS];
        self.econ.init(&self.terrain);
        self.sim_time = 0.0;
        self.spawn_timer = 1e9;
        for i in 0..MAX_GOLFERS {
            self.props.push(Prop {
                golfer: Some(i),
                hidden: true,
                body: self.look_body[0][0],
                shadow: self.look_shadow[0][0],
                ..Default::default()
            });
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
        let lighting = std::fs::read(self.game_path(&format!("{}Lighting.txt", THEMES[theme])))
            .map(|d| sg_core::terrain::parse_lighting(&sg_core::formats::latin1(&d)))
            .unwrap_or_default();
        self.light = lighting;
        self.rebuild_batches(g);
        self.populate_props();
        true
    }

    // ---- golfers and the club -----------------------------------------------------------------------------------------------

    pub fn golfers_on_course(&self) -> usize {
        self.golfers.iter().filter(|g| g.active).count()
    }

    /// Starts a golfer on the first hole. Skills vary from golfer to golfer (PLACEHOLDER spread of 4 to 11 out of 15).
    fn spawn_golfer(&mut self) {
        if self.holes.is_empty() {
            return;
        }
        let Some(i) = self.golfers.iter().position(|g| !g.active) else { return };
        let mut r = self
            .seed
            .wrapping_mul(2654435761)
            .wrapping_add((self.sim_time * 1000.0) as u32)
            .wrapping_add((i as u32).wrapping_mul(97))
            .wrapping_add(self.rounds_started.wrapping_mul(7919))
            .wrapping_add(1);
        let mut next = || {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            r
        };
        let mut g = Golfer { active: true, ..Default::default() };
        g.look = (next() % LOOKS as u32) as usize;
        if !self.look_ok[g.look] {
            g.look = 0;
        }
        for k in 0..10 {
            g.sim.skills.v[k] = 4 + (next() % 8) as i32;
        }
        if self.rounds_started == 0 {
            g.sim.skills = self.skills; // the first golfer is the one picked with --golfer
        }
        g.sim.set_route(self.holes[0].route.clone());
        g.hole_start = self.sim_time;
        // The exe starts each golfer's mood at 3 plus a random 0 to 2, or at 4 on the easiest difficulty.
        g.mood = if self.difficulty == 0 { 4 } else { 3 + (next() % 3) as i32 };
        g.sim.looping = false;
        g.last_stroke = -1;
        let seed = next();
        g.sim.init(&self.terrain, seed);
        self.golfers[i] = g;
        self.rounds_started += 1;
    }

    fn golfer_sounds(&mut self, gi: usize) {
        let (ev, club) = (self.golfers[gi].sim.event, self.golfers[gi].sim.club);
        match ev {
            "drive" => {
                self.snd(if club == "iron" { "Golf_Sfx/Iron.wav" } else { "Golf_Sfx/Drive With Ball.wav" }, 0.8, false);
            }
            "putt" => {
                self.snd("Golf_Sfx/Putt.wav", 0.8, false);
            }
            "on the course" => {
                self.snd("Golf_Sfx/Ball Drop Fairway.wav", 0.5, false);
            }
            "in the sand" => {
                self.snd("Golf_Sfx/Ball Drop Sand.wav", 0.6, false);
            }
            "holed" => {
                // The exe's green fee routine plays no sound; applause belongs to the SGA rating announcements and the cash
                // register to "Other" income (docs/PUBLISHER_EXE_NOTES.md, "Sounds").
                self.snd("Golf_Sfx/Ball In Hole.wav", 0.8, false);
            }
            e if e.starts_with("splash") => {
                self.snd("Golf_Sfx/Ball Water.wav", 0.8, false);
            }
            e if e.starts_with("out of bounds") => {
                self.snd("Golf_Sfx/Ball Tree.wav", 0.8, false);
            }
            _ => {}
        }
    }

    /// Advances the club by dt seconds: arrivals, every golfer's round, fees, wages and the board's messages.
    pub fn step_game(&mut self, dt: f32) {
        let open = !self.holes.is_empty() && !self.econ.game_over;
        // Arrivals (PLACEHOLDER rates): the first golfer comes at once, then one every 25 s when golfers are happy, slower when not;
        // the course takes at most two golfers per hole, up to the size of the pool.
        self.spawn_timer += dt as f64;
        let every = 25.0 / (0.5 + self.econ.fun / 100.0);
        let capacity = MAX_GOLFERS.min(2 * self.holes.len());
        if open && self.spawn_timer >= every && self.golfers_on_course() < capacity {
            self.spawn_golfer();
            self.spawn_timer = 0.0;
        }
        self.econ.step(dt as f64);
        if !self.econ.notice.is_empty() {
            println!("[{:6.1}s] board: {}", self.sim_time, self.econ.notice);
            self.econ.notice.clear();
        }
        for gi in 0..self.golfers.len() {
            if !self.golfers[gi].active {
                continue;
            }
            {
                let ranger = self.econ.staff[economy::RANGER] > 0;
                let g = &mut self.golfers[gi];
                g.sim.pace_scale = if ranger { 1.2 } else { 1.0 }; // a Ranger speeds play up (PLACEHOLDER; the original works near one tee)
                g.sim.step(&self.terrain, dt);
                g.needs_clock -= dt as f64;
            }
            if self.golfers[gi].needs_clock <= 0.0 {
                // The exe updates a golfer's needs every 160 ticks (14 seconds at 87 ms a tick).
                self.golfers[gi].needs_clock += 160.0 / sg_core::flight::TICKS_PER_SECOND as f64;
                self.needs_tick(gi);
                if !self.golfers[gi].active {
                    continue;
                }
            }
            let (ev, stroke) = (self.golfers[gi].sim.event, self.golfers[gi].sim.stroke);
            if ev != self.golfers[gi].last_event || stroke != self.golfers[gi].last_stroke {
                self.golfers[gi].last_event = ev;
                self.golfers[gi].last_stroke = stroke;
                self.golfer_sounds(gi);
                self.shot_mood_event(gi, ev);
                if !self.golfers[gi].active {
                    continue;
                }
                if ev == "holed" {
                    self.hole_out(gi);
                }
            }
            if self.golfers[gi].sim.finished {
                let g = &mut self.golfers[gi];
                g.hole += 1;
                if g.hole < self.holes.len() {
                    g.sim.set_route(self.holes[g.hole].route.clone());
                    g.hole_start = self.sim_time;
                    let seed = (g.sim.stroke as u32).wrapping_mul(7919).wrapping_add(g.hole as u32).wrapping_add(gi as u32);
                    g.sim.init(&self.terrain, seed);
                } else {
                    println!("[{:6.1}s] golfer {gi} finished the round in {} strokes", self.sim_time, g.strokes_round);
                    g.active = false;
                }
            }
        }
    }

    /// Raises the exe's mood events for what just happened to a golfer's ball. Which shot outcome raises which event is our reading
    /// of the event meanings; the putt distances that count as easy or tough are PLACEHOLDERS.
    fn shot_mood_event(&mut self, gi: usize, ev: &'static str) {
        const EASY_PUTT: f32 = 60.0;
        const TOUGH_PUTT: f32 = 150.0;
        let putt = self.golfers[gi].sim.last_putt_dist;
        let event = match ev {
            "holed" if putt > TOUGH_PUTT => mood::ev::TOUGH_PUTT_MADE,
            "putt missed" if putt < EASY_PUTT => mood::ev::EASY_PUTT_MISSED,
            e if e.starts_with("splash") => mood::ev::IN_HAZARD,
            e if e.starts_with("out of bounds") => mood::ev::BAD_SHOT,
            _ => return,
        };
        self.mood_event(gi, event, 0);
    }

    /// Applies one mood event to a golfer: the mood changes by the exe's amount, the change is tallied for the hole's fun rating,
    /// and a golfer pushed below -10 leaves the course.
    pub fn mood_event(&mut self, gi: usize, event: u32, arg: i32) {
        let d = mood::delta(event, self.difficulty, arg, false);
        if self.hole_stats.len() != self.holes.len() {
            self.hole_stats = vec![HoleStat::default(); self.holes.len()];
        }
        self.mood_delta(gi, d);
    }

    /// One needs update, following the exe: one counter grows by one (hunger near some terrain kinds or, in the Tropical theme, half
    /// the time; thirst otherwise), and once a counter is above 15 the golfer complains every fourth update. A hired Soda Vendor
    /// serves a thirsty golfer: the club earns 2 units under Food/Drink and the golfer's thirst resets. The terrain test and the
    /// chance that a vendor reaches a golfer in time are PLACEHOLDERS (the exe sends the vendor walking to the golfer).
    fn needs_tick(&mut self, gi: usize) {
        let seed = (self.sim_time * 1000.0) as u32 ^ (gi as u32).wrapping_mul(2654435761);
        let mut rng = Rng::new(seed | 1);
        let hungry_side = self.theme == 3 && rng.range(2) == 0 || rng.range(4) == 0;
        let (event, value) = {
            let g = &mut self.golfers[gi];
            if hungry_side {
                if rng.range(2) == 0 {
                    g.hunger += 1;
                }
                (mood::ev::HUNGRY, g.hunger)
            } else {
                g.thirst += 1;
                (mood::ev::THIRSTY, g.thirst)
            }
        };
        if value > 15 && value % 4 == 0 {
            self.mood_event(gi, event, 0x14);
        }
        let vendors = self.econ.staff[economy::SODA_VENDOR];
        if vendors > 0 && self.golfers[gi].active && self.golfers[gi].thirst > 16 && rng.range(4) < vendors.min(4) {
            let counter_ok = self.golfers[gi].thirst > 7;
            self.golfers[gi].thirst = 0;
            self.econ.earn_to(economy::LEDGER_FOOD_DRINK, 2.0 * Economy::UNIT);
            let d = mood::delta(mood::ev::DRINK, self.difficulty, 0x14, counter_ok);
            self.mood_delta(gi, d);
        }
    }

    fn mood_delta(&mut self, gi: usize, d: i32) {
        let hole = self.golfers[gi].hole;
        if let Some(hs) = self.hole_stats.get_mut(hole) {
            hs.mood_sum += d;
        }
        let (m, leaves) = mood::apply(self.golfers[gi].mood, d);
        self.golfers[gi].mood = m;
        if leaves {
            println!("[{:6.1}s] golfer {gi} left the course unhappy", self.sim_time);
            self.golfers[gi].active = false;
        }
    }

    fn hole_out(&mut self, gi: usize) {
        let (hole, stroke, mood, hole_start) = {
            let g = &self.golfers[gi];
            (g.hole, g.sim.stroke, g.mood, g.hole_start)
        };
        let par = self.holes.get(hole).map(|h| h.par).unwrap_or(4);
        let before = self.econ.cash;
        // Green fee, from the exe's fee routine, in units of 100: the golfer's mood, plus 2 for a Creative class hole or 5 for a Heroic,
        // Strategic or Classic one (the exe adds 2 for the hole type values above 3, and 3 more unless the value is 4; here the type
        // index is L + 2A + 4I, which is an ASSUMPTION about the exe's numbering). The exe also adds 2 each for two hole flags (probably
        // Top 100 and Top 18) and an Airstrip bonus; none of those exist here yet.
        let mut bonus = 0;
        if let Some(r) = self.ratings.get(hole) {
            if r.type_index > 3 {
                bonus = 2 + if r.type_index != 4 { 3 } else { 0 };
            }
        }
        let fee_units = mood.max(0) + bonus;
        self.econ.hole_finished(stroke, par, fee_units as f64);
        // A hungry or thirsty golfer who holes out within 6 tiles of a snack bar visits it (PLACEHOLDER reach rule; the exe walks the
        // golfer there). As in the exe, the visit resets hunger and thirst and raises the snack event, worth +1 when hunger was above 7.
        let needy = self.golfers[gi].hunger > 7 || self.golfers[gi].thirst > 7;
        if let (Some(h), true) = (self.holes.get(hole), needy) {
            let (gx, gz) = (h.green_x, h.green_z);
            let visit = self.buildings.iter().find_map(|b| {
                let (bx, bz) = self.terrain.tile_centre(b.tx, b.ty);
                (BUILD[b.def].visit > 0 && (bx - gx).hypot(bz - gz) < 6.0 * TILE_SIZE).then_some(BUILD[b.def].visit)
            });
            if let Some(v) = visit {
                self.econ.earn_to(economy::LEDGER_FOOD_DRINK, v as f64 * Economy::UNIT);
                let counter_ok = self.golfers[gi].hunger > 7;
                self.golfers[gi].hunger = 0;
                self.golfers[gi].thirst = 0;
                let d = mood::delta(mood::ev::SNACK, self.difficulty, 0x14, counter_ok);
                self.mood_delta(gi, d);
            }
        }
        // After the hole the exe lowers mood by (hole field + 6 + holes played) * (mood - 1 + difficulty) * (difficulty + 1) /
        // ((course factor * 5 + 15) * 8), integer division. The hole field and the course factor are not decoded; both are taken as 0.
        let d = self.difficulty;
        let dec = ((6 + hole as i32) * (mood - 1 + d) * (d + 1)) / 120;
        let g = &mut self.golfers[gi];
        g.mood -= dec;
        g.strokes_round += stroke;
        if self.hole_stats.len() != self.holes.len() {
            self.hole_stats = vec![HoleStat::default(); self.holes.len()];
        }
        if let Some(hs) = self.hole_stats.get_mut(hole) {
            hs.plays += 1;
            hs.strokes += stroke as f64;
            hs.seconds += self.sim_time - hole_start;
            hs.revenue += self.econ.cash - before;
            hs.mood += self.econ.last_mood;
            hs.hist[(stroke - 3).clamp(0, 5) as usize] += 1;
        }
        println!(
            "[{:6.1}s] golfer {gi} holed hole {} in {stroke} (par {par}), fee ${:.0}, cash ${:.0}, fun {:.0}",
            self.sim_time,
            hole + 1,
            self.econ.cash - before,
            self.econ.cash,
            self.econ.fun
        );
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
            self.golfers.iter_mut().for_each(|g| g.active = false);
            self.spawn_timer = 1e9;
        }
        for pi in 0..self.props.len() {
            let Some(gi) = self.props[pi].golfer else { continue };
            let g = &self.golfers[gi];
            self.props[pi].hidden = !g.active;
            if !g.active {
                continue;
            }
            let s = &g.sim;
            let a = s.anim as usize;
            let Some(body) = self.look_body[g.look][a] else {
                self.props[pi].hidden = true;
                continue;
            };
            let sp = &self.sprites[body].s;
            let fr = s.anim_time * 1000.0 / sp.frame_ms.max(1) as f32;
            let looping = s.anim == GolferAnim::Walk || s.anim == GolferAnim::Happy;
            let n = sp.frames_per_view;
            let p = &mut self.props[pi];
            p.body = Some(body);
            p.shadow = self.look_shadow[g.look][a];
            p.x = s.golfer_x;
            p.z = s.golfer_z;
            p.heading = s.golfer_heading;
            p.frame = if looping { (fr as i32) % n } else { (fr as i32).min(n - 1) };
        }
    }

    /// Course Status Report (the original's F1): the hole's class and length, and how many paths are joined to the clubhouse.
    pub fn report_course(&mut self, force: bool) {
        self.hole = analyze_hole(&self.terrain);
        self.holes = find_holes(&self.terrain);
        if self.hole_stats.len() != self.holes.len() {
            self.hole_stats = vec![HoleStat::default(); self.holes.len()];
        }
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

    /// Club ratings as the exe computes them. Fun: each hole contributes 100 * (sum of mood changes) / (plays + 4), and the club's
    /// value is the sum over holes (the exe also divides by half of another per-hole counter that is not decoded; taken as 0).
    pub fn club_fun(&self) -> i32 {
        self.hole_stats.iter().filter(|h| h.plays > 0).map(|h| (100 * h.mood_sum as i64) / (h.plays as i64 + 4)).sum::<i64>() as i32
    }
    /// Skill: the sum over holes of the Length, Accuracy and Imagination differences in strokes.
    pub fn club_skill(&self) -> f64 {
        self.ratings.iter().map(|r| (r.len + r.acc + r.img) as f64).sum()
    }

    pub fn ensure_ratings(&mut self, samples: i32) {
        if self.ratings.len() != self.holes.len() {
            self.ratings = self.holes.iter().map(|r| rate_hole(&self.terrain, r, samples, self.difficulty)).collect();
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
        self.econ.start_cash = if sandbox { START_FUNDS as f64 } else { (START_FUNDS - price) as f64 };
        let slot_index = self.slot_of[prop_idx];
        let land = land::generate(slot_index, self.offer[slot_index], self.difficulty, sandbox, &mut self.exe_rng, &self.noise);
        self.terrain = land.to_terrain();
        self.land = Some(land);
        self.course_name = format!("{} GC", p.name);
        self.load_theme(g, p.theme);
        self.screen = Screen::Play;
        self.hover = -1;
        self.edit = false;
        self.econ.day = 1;
        self.panel = 0;
        self.buildings.clear();
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

    /// Loads one story script from the disc's Themes folder. Format (read from the files): a title line, then blocks separated by
    /// blank lines; the first line of a block is one golfer's line, the lines after it that start with a space are the other
    /// golfer's possible replies. PARTNER stands for the other golfer. What decides which story plays, and when, is not decoded;
    /// this picks one by seed and shows it when two golfers are on the course.
    pub fn load_story(&mut self) {
        self.story_lines.clear();
        self.story_title.clear();
        self.story_pos = 0;
        self.story_next = 0.0;
        let dir = self.game_path("Themes/Standard");
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| sg_core::fsutil::ext_lower(p) == ".txt").collect())
            .unwrap_or_default();
        if files.is_empty() {
            return;
        }
        files.sort();
        let Ok(d) = std::fs::read(&files[self.seed as usize % files.len()]) else { return };
        let text = sg_core::formats::latin1(&d);
        let (mut first, mut new_block) = (true, true);
        for raw in text.split('\n') {
            let mut line = raw.trim_end_matches(['\r', '\n']).to_string();
            if first {
                self.story_title = line.trim_start_matches(' ').to_string();
                first = false;
                continue;
            }
            if line.is_empty() {
                new_block = true;
                continue;
            }
            let indented = line.starts_with(' ');
            line = line.replace("PARTNER", "pal");
            let t = line.trim_start_matches(' ');
            if t.is_empty() {
                continue;
            }
            let who = if !indented || new_block { "A" } else { "B" };
            self.story_lines.push(format!("{who}|{t}"));
            new_block = false;
        }
    }

    /// Advisor text from what the club looks like now. These hints are our own words.
    pub fn advisor_text(&self) -> &'static str {
        if self.holes.is_empty() {
            "Welcome to your new club. Open Build Course (the big round button at the bottom left), then paint a tee and a green a good distance apart to make your first hole."
        } else if self.buildings.is_empty() {
            "Golfers are on the course. Open Add Buildings and put up a snack bar: pick a building lot next to a path that joins the clubhouse. Visitors spend money there."
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
                // tees use this byte as their look
                let vb = if pe.ty == 0 { ((x * 7 + y * 13) as u32 % 5) as i32 } else { pe.vbyte };
                if self.terrain.inside(x, y) && self.terrain.ty[self.terrain.tile_index(x, y)] as i32 != pe.ty {
                    self.econ.spend_to(economy::LEDGER_BUILD_COURSE, Economy::terrain_cost_units(pe.ty) as f64 * Economy::UNIT);
                }
                self.terrain.paint(x, y, pe.ty, vb);
                if matches!(pe.ty, 0 | 1 | 17 | 22) {
                    self.terrain.flatten_tile(x, y);
                }
            }
        }
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
                let i = self.terrain.tile_index(x, y);
                if !remove && self.terrain.path_kind[i] == 0 {
                    self.econ.spend_to(economy::LEDGER_BUILD_COURSE, Economy::PATH_TILE_COST);
                }
                self.terrain.path_kind[i] = if remove { 0 } else { self.path_kind as u8 };
            }
        }
        self.dirty = true;
        self.snd("Interface/Path.wav", 0.7, false);
    }

    /// Amenity placement. Rules from the game text: buildings go on a building lot and need a path to the clubhouse. The lot must
    /// touch a path tile that is connected to the clubhouse. Removing refunds the cost.
    pub fn edit_building(&mut self, tx: i32, ty: i32, remove: bool) {
        if !self.terrain.inside(tx, ty) {
            return;
        }
        if let Some(i) = self.buildings.iter().position(|b| b.tx == tx && b.ty == ty) {
            if remove {
                self.econ.earn_to(economy::LEDGER_FACILITIES, BUILD[self.buildings[i].def].cost as f64 * Economy::UNIT);
                self.buildings.remove(i);
                self.props.retain(|p| !p.building);
                for b in self.buildings.clone() {
                    self.add_building_prop(b);
                }
                self.show_toast("Building removed, money refunded");
                self.snd("Interface/Building.wav", 0.7, false);
            }
            return;
        }
        if remove {
            return;
        }
        if !self.build_available(self.build_idx) {
            return self.show_toast("Not available in this theme");
        }
        if self.terrain.ty[self.terrain.tile_index(tx, ty)] != TT_BUILDING {
            return self.show_toast("Buildings go on a building lot");
        }
        let conn = paths_connected_to_clubhouse(&self.terrain);
        const DX: [i32; 4] = [0, 1, 0, -1];
        const DY: [i32; 4] = [-1, 0, 1, 0];
        let ok = (0..4).any(|k| {
            let (nx, ny) = (tx + DX[k], ty + DY[k]);
            self.terrain.inside(nx, ny) && conn[self.terrain.tile_index(nx, ny)] != 0
        });
        if !ok {
            return self.show_toast("Buildings need a path to the clubhouse");
        }
        let cost = BUILD[self.build_idx].cost as f64 * Economy::UNIT;
        if !self.econ.sandbox && self.econ.cash < cost {
            return self.show_toast("Not enough money");
        }
        self.econ.spend_to(economy::LEDGER_FACILITIES, cost);
        let b = Placed { def: self.build_idx, tx, ty };
        self.buildings.push(b);
        self.add_building_prop(b);
        self.snd("Interface/Building.wav", 0.7, false);
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
            4 => BUILD[self.build_idx].name,
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
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u32).unwrap_or(0)
}

fn slots_of(offer: &[Slot; 16]) -> [usize; 16] {
    let mut out = [0; 16];
    for (i, s) in offer.iter().enumerate() {
        out[s.property] = i;
    }
    out
}
