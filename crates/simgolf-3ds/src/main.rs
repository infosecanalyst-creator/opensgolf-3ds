//! OpenSGolf on the Nintendo 3DS (New 3DS XL first; see README.md).
//!
//! This binary is milestone 1 of the port: it proves the whole chain on real hardware without touching the game crate yet.
//!   1. sg-core builds for armv6k-nintendo-3ds and runs (free base set, terrain, decoders, mixer).
//!   2. gfx.rs draws sg-core's terrain mesh through citro3d with the original's lighting, on the top screen, in stereo.
//!   3. The bottom screen is the course as a plan you draw on with the stylus (plan.rs); the C-stick pans, L/R zoom, the
//!      circle pad moves a course cursor on the top screen.
//!   4. ndsp plays the mixer.
//! Milestone 2 replaces `Demo` below with the game crate's `App` + `Stage` (its `use miniquad::*` lines pointed at
//! `crate::shim`, its `Gfx` at `crate::gfx`), which is the port proper.
#![feature(allocator_api)]

mod audio3ds;
mod texcache;
mod gfx;
mod input;
mod plan;
mod hud;
mod text;
mod screens;
mod shim;

// ---- the game (crates/simgolf), compiled here unchanged ------------------------------------------------------------------
// Its modules, as its own main.rs lists them, but for the desktop-only ones (Android, controllers, the desktop touch layer,
// macOS release notes) and gfx.rs, whose place gfx.rs above takes (the same public surface, drawn with citro3d). Its
// `use miniquad::*` lines reach vendor/miniquad-3ds, a stand-in with miniquad's types and a GPU that draws nothing. What
// the modules take from the game's main.rs (Stage, load_art) is in compat.rs.
#[path = "../../simgolf/src/app.rs"]
mod app;
#[path = "../../simgolf/src/audio.rs"]
mod audio;
#[path = "../../simgolf/src/autosave.rs"]
mod autosave;
#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
#[path = "../../simgolf/src/autoupdate.rs"]
mod autoupdate;
#[path = "../../simgolf/src/build_tools.rs"]
mod build_tools;
#[path = "../../simgolf/src/champ_ui.rs"]
mod champ_ui;
#[path = "../../simgolf/src/cheats_ui.rs"]
mod cheats_ui;
#[path = "../../simgolf/src/comfort.rs"]
mod comfort;
#[path = "../../simgolf/src/content.rs"]
mod content;
#[path = "../../simgolf/src/content_profiles.rs"]
mod content_profiles;
#[path = "../../simgolf/src/controls.rs"]
mod controls;
#[path = "../../simgolf/src/controls_ui.rs"]
mod controls_ui;
#[path = "../../simgolf/src/cursor_ui.rs"]
mod cursor_ui;
#[path = "../../simgolf/src/cust_ui.rs"]
mod cust_ui;
#[path = "../../simgolf/src/edges.rs"]
mod edges;
#[path = "../../simgolf/src/editor.rs"]
mod editor;
#[path = "../../simgolf/src/enhanced.rs"]
mod enhanced;
#[path = "../../simgolf/src/extras.rs"]
mod extras;
#[path = "../../simgolf/src/extras_ui.rs"]
mod extras_ui;
#[path = "../../simgolf/src/files_ui.rs"]
mod files_ui;
#[path = "../../simgolf/src/framerate.rs"]
mod framerate;
#[path = "../../simgolf/src/fx.rs"]
mod fx;
#[path = "../../simgolf/src/fx_ball.rs"]
mod fx_ball;
#[path = "../../simgolf/src/fx_crowd.rs"]
mod fx_crowd;
#[path = "../../simgolf/src/fx_glsl.rs"]
mod fx_glsl;
#[path = "../../simgolf/src/fx_heat.rs"]
mod fx_heat;
#[path = "../../simgolf/src/fx_life.rs"]
mod fx_life;
#[path = "../../simgolf/src/fx_light.rs"]
mod fx_light;
#[path = "../../simgolf/src/fx_nature.rs"]
mod fx_nature;
#[path = "../../simgolf/src/fx_night.rs"]
mod fx_night;
#[path = "../../simgolf/src/fx_reflect.rs"]
mod fx_reflect;
#[path = "../../simgolf/src/fx_shadow.rs"]
mod fx_shadow;
#[path = "../../simgolf/src/fx_smooth.rs"]
mod fx_smooth;
#[path = "../../simgolf/src/fx_water.rs"]
mod fx_water;
#[path = "../../simgolf/src/fx_wet.rs"]
mod fx_wet;
#[path = "../../simgolf/src/fx_winter.rs"]
mod fx_winter;
#[path = "../../simgolf/src/gamefiles.rs"]
mod gamefiles;
#[path = "../../simgolf/src/glass_ui.rs"]
mod glass_ui;
#[path = "../../simgolf/src/graphs.rs"]
mod graphs;
#[path = "../../simgolf/src/hd.rs"]
mod hd;
#[path = "../../simgolf/src/history_ui.rs"]
mod history_ui;
#[path = "../../simgolf/src/holemarks.rs"]
mod holemarks;
#[path = "../../simgolf/src/homes_ui.rs"]
mod homes_ui;
#[path = "../../simgolf/src/hud_ui.rs"]
mod hud_ui;
#[path = "../../simgolf/src/import.rs"]
mod import;
#[path = "../../simgolf/src/info_ui.rs"]
mod info_ui;
#[path = "../../simgolf/src/insight.rs"]
mod insight;
#[path = "../../simgolf/src/iso.rs"]
mod iso;
#[path = "../../simgolf/src/keys_ui.rs"]
mod keys_ui;
#[path = "../../simgolf/src/language.rs"]
mod language;
#[path = "../../simgolf/src/member_panel.rs"]
mod member_panel;
#[path = "../../simgolf/src/message_ui.rs"]
mod message_ui;
#[path = "../../simgolf/src/net.rs"]
mod net;
#[path = "../../simgolf/src/panels_ui.rs"]
mod panels_ui;
#[path = "../../simgolf/src/paths_lamps.rs"]
mod paths_lamps;
#[path = "../../simgolf/src/pathview_ui.rs"]
mod pathview_ui;
#[path = "../../simgolf/src/people_ui.rs"]
mod people_ui;
#[path = "../../simgolf/src/perf.rs"]
mod perf;
#[path = "../../simgolf/src/photo.rs"]
mod photo;
#[path = "../../simgolf/src/player_panel.rs"]
mod player_panel;
#[path = "../../simgolf/src/popup_ui.rs"]
mod popup_ui;
#[path = "../../simgolf/src/prefs_search.rs"]
mod prefs_search;
#[path = "../../simgolf/src/pro_ui.rs"]
mod pro_ui;
#[path = "../../simgolf/src/profile_ui.rs"]
mod profile_ui;
#[path = "../../simgolf/src/render.rs"]
mod render;
#[path = "../../simgolf/src/report_tabs.rs"]
mod report_tabs;
#[path = "../../simgolf/src/reports_ui.rs"]
mod reports_ui;
#[path = "../../simgolf/src/scenario_ui.rs"]
mod scenario_ui;
#[path = "../../simgolf/src/scorecard_ui.rs"]
mod scorecard_ui;
#[path = "../../simgolf/src/screens_ui.rs"]
mod screens_ui;
#[path = "../../simgolf/src/share.rs"]
mod share;
#[path = "../../simgolf/src/staffview_ui.rs"]
mod staffview_ui;
#[path = "../../simgolf/src/sysdiag.rs"]
mod sysdiag;
#[path = "../../simgolf/src/tags_ui.rs"]
mod tags_ui;
#[path = "../../simgolf/src/thoughts_ui.rs"]
mod thoughts_ui;
#[path = "../../simgolf/src/tourney_ui.rs"]
mod tourney_ui;
#[path = "../../simgolf/src/ui.rs"]
mod ui;
#[path = "../../simgolf/src/undo.rs"]
mod undo;
#[path = "../../simgolf/src/update.rs"]
mod update;
#[path = "../../simgolf/src/views.rs"]
mod views;
#[path = "../../simgolf/src/wild_ui.rs"]
mod wild_ui;
#[path = "../../simgolf/src/world_ui.rs"]
mod world_ui;
#[path = "../../simgolf/src/year_ui.rs"]
mod year_ui;
mod compat;
pub use compat::{load_art, Stage};

use ctru::prelude::*;
use gfx::{Eye, Gfx, Mat4, Mode, TerrainMesh, TerrainUniforms, TerrainVert, TextureId, Uniforms, Vert};
use screens::Screen;
use sg_core::terrain::{build_tile_triangles, Lighting, Terrain, TextureCatalog, TileTri, DEPTH_RANGE, HEIGHT_STEP};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by `shim::window::order_quit`.
pub static QUIT: AtomicBool = AtomicBool::new(false);

/// Where the game's files live on the SD card: your own SimGolf copy (the folder with Data, Flics and Sounds) goes in
/// `sdmc:/3ds/opensgolf/game`; without one the free base set is written to `sdmc:/3ds/opensgolf/free` on first start.
const SD_ROOT: &str = "sdmc:/3ds/opensgolf";

/// The same pitch the game uses for a non-800x600 view (`app::pitch_for`), and the original's 45 degree yaw.
const PITCH: f64 = 40.541603;
/// World height at which things reach the full stereo separation: seven terrain levels (the tallest hills of a course). The
/// parallax in gfx comes from world height alone, so anything drawn in the world pass with a real height pops the same way:
/// raised land, buildings (upright quads at their footprint's height, not flat sprites), and the ball in flight (its sprite at
/// the ball's world y, its shadow on the ground at y = 0, so it lifts out of the screen along the shot's arc).
const STEREO_FULL_HEIGHT: f32 = 7.0 * HEIGHT_STEP;
/// Course chunk size in tiles (see `Demo::chunks`): small enough that an edit rebuilds little and off-screen chunks are
/// skipped, large enough to keep the draw calls few.
const CHUNK: i32 = 10;

/// A terrain picture as drawn: its texture and its rectangle in it (u0, v0, du, dv).
#[derive(Clone, Copy)]
struct TexRef {
    tex: TextureId,
    rect: [f32; 4],
}

/// Terrain pictures are 64 x 64; up to 256 of them are packed into one 1024 x 1024 atlas (64 into 512 x 512), so a course
/// chunk draws in a call or two instead of one per picture (285 pictures made ~3,800 draw calls a frame).
const CELL: u32 = 64;
/// Mipmap levels an atlas keeps: down to 8 x 8 texel cells; beyond that, neighbouring pictures would blend together.
const ATLAS_LEVELS: u8 = 3;

/// Where the clubhouse stand-in sits (world x, z): a little north-west of the course centre.
const CLUBHOUSE: (f32, f32) = (-350.0, -650.0);

/// A ball in flight (the demo of the ball in 3D): from where it was hit, along the ground direction "up the screen",
/// rising to a peak and coming down over a few seconds.
struct Ball {
    from: [f32; 3],
    dir: [f32; 2],
    t0: std::time::Instant,
    /// The landing sound has played.
    landed: bool,
}

impl Ball {
    // a short, high pitch, so the whole flight stays on screen at the starting zoom
    const DIST: f32 = 600.0; // world units (6 tiles)
    const APEX: f32 = 260.0; // world units above the line from where it was hit to where it lands
    const SECS: f32 = 2.5;

    fn hit(d: &Demo, from: [f32; 3]) -> Ball {
        // "up the screen" on the ground: the view direction with its vertical part removed
        let r = d.eye_rot.0;
        let (fx, fz) = (-r[2], -r[10]);
        let n = (fx * fx + fz * fz).sqrt().max(1e-6);
        Ball { from, dir: [fx / n, fz / n], t0: std::time::Instant::now(), landed: false }
    }
    fn done(&self) -> bool {
        self.t0.elapsed().as_secs_f32() > Self::SECS + 0.5
    }
    /// The ball's position and the land's height under it.
    fn at(&self, d: &Demo) -> ([f32; 3], f32) {
        let f = (self.t0.elapsed().as_secs_f32() / Self::SECS).min(1.0);
        let (x, z) = (self.from[0] + self.dir[0] * Self::DIST * f, self.from[2] + self.dir[1] * Self::DIST * f);
        let ground = d.terrain.height_at(x, z);
        let line = self.from[1] + (ground - self.from[1]) * f;
        let y = (line + 4.0 * Self::APEX * f * (1.0 - f)).max(ground);
        ([x, y, z], ground)
    }
}

/// A CHUNK x CHUNK block of the course: one mesh with its triangles grouped by texture, the vertex range of each texture,
/// and its world-space bounds (for skipping it when off screen).
#[derive(Default)]
struct Chunk {
    mesh: Option<TerrainMesh>,
    ranges: Vec<(TextureId, usize, usize)>,
    lo: [f32; 3],
    hi: [f32; 3],
}

/// What `Demo::prepare` loads before the GPU is started.
struct Prepared {
    catalog: TextureCatalog,
    light: Lighting,
    terrain: Terrain,
    images: BTreeMap<PathBuf, Option<sg_core::assets::Rgba>>,
    /// Per tile type, the average colour of its first texture: the bottom screen's plan (plan.rs).
    type_rgb: Vec<Option<[u8; 3]>>,
    /// The decoded-texture cache (texcache.rs) and its file: kept for the game's course (`Demo::adopt`).
    cache: std::collections::HashMap<String, sg_core::assets::Rgba>,
    cache_file: PathBuf,
}

/// The demo course and its camera: what milestone 2 swaps for the game crate's App.
struct Demo {
    terrain: Terrain,
    catalog: TextureCatalog,
    light: Lighting,
    /// Each terrain picture's texture and where it sits in it: (u0, v0, du, dv), the whole texture for a picture of its own,
    /// a cell for one packed into an atlas.
    textures: BTreeMap<PathBuf, Option<TexRef>>,
    /// The texture for (type, set, variation): `catalog.pick` + `texture_for` once per combination, not per triangle.
    tex_ids: std::collections::HashMap<(i32, i32, i32), Option<TexRef>>,
    /// The course in CHUNK x CHUNK-tile blocks, each its own per-texture meshes, so an edit rebuilds only the blocks
    /// around it (a whole-course rebuild is ~20,000 triangles: too slow per press on the console).
    chunks: Vec<Chunk>,
    /// The fixed lighting terms per vertex depend on the camera's rotation (the light is fixed in eye space): built for it.
    eye_rot: Mat4,
    cam_x: f32,
    cam_z: f32,
    /// World units per top-screen pixel.
    upp: f32,
    /// Decoded terrain pictures by file name (texcache.rs), and the cache file to write new ones to.
    cache: std::collections::HashMap<String, sg_core::assets::Rgba>,
    cache_file: PathBuf,
}

/// Where the game's files are. With a copy of SimGolf at `sdmc:/3ds/opensgolf/game` that folder is read directly. Without
/// one, the free base set is a content pack (gfx/, sfx/, text/, fonts/ with a pack.toml): it is mounted through sg-core's pack
/// loader and virtual file system, which maps the original's paths (Data/Textures/..., Sounds/...) onto the pack, exactly
/// as the desktop game does. The same loader is what mods on the SD card will go through.
///
/// A missing free set is written first, with progress on a text console on the bottom screen (the GPU takes that screen
/// over afterwards): drawing 5,465 files on the ARM11 takes minutes, so `make-free-set.ps1` makes the set on a PC instead.
fn game_dir(say: &mut dyn FnMut(&str)) -> PathBuf {
    let root = Path::new(SD_ROOT);
    let game = root.join("game");
    if sg_core::fsutil::is_dir(game.join("Data")) {
        return game;
    }
    let free = root.join("free");
    if !sg_core::fsutil::exists(free.join("pack.toml")) {
        write_free_set(say, &free);
    } else {
        repair_free_set(say, &free);
    }
    say("mounting the free set");
    // the game folder the pack's files are mapped under; it need not hold anything
    let _ = std::fs::create_dir_all(&game);
    let found = sg_core::packs::discover(&[root.to_path_buf()]);
    let settings = sg_core::packs::Settings { base: Some(sg_core::packs::FREE.to_string()), ..Default::default() };
    let plan = sg_core::packs::resolve(&found, &settings);
    let loaded = sg_core::packs::load(&found, &plan);
    for (id, w) in &loaded.warnings {
        say(&format!("content pack {}: {w}", if id.is_empty() { "data" } else { id }));
    }
    sg_core::packs::apply(&game, loaded);
    say(&format!("content packs: {}", plan.active_ids().join(", ")));
    game
}

/// Writes whatever the card lacks of the free set (a partial copy left the console with no sounds, and an interrupted repair
/// leaves folders half done). What is there is found by listing each of the set's folders once (one directory read each:
/// checking files one by one took minutes on the console), and the missing files are written. A complete set costs one
/// listing per folder.
fn repair_free_set(say: &mut dyn FnMut(&str), free: &Path) {
    use std::collections::{HashMap, HashSet};
    let files = sg_core::freeset::files();
    // every folder the set has, listed once: lower-case names of what it holds
    let mut have: HashMap<String, HashSet<String>> = HashMap::new();
    for f in &files {
        let dir = f.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        if !have.contains_key(dir) {
            let names = std::fs::read_dir(free.join(dir))
                .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_lowercase()).collect())
                .unwrap_or_default();
            have.insert(dir.to_string(), names);
        }
    }
    let todo: Vec<&String> = files
        .iter()
        .filter(|f| {
            let (dir, name) = f.rsplit_once('/').unwrap_or(("", f.as_str()));
            !have.get(dir).is_some_and(|s| s.contains(&name.to_lowercase()))
        })
        .collect();
    if todo.is_empty() {
        return;
    }
    say(&format!("free set: {} files missing on the card; writing them", todo.len()));
    say("(once; a few minutes on the console)");
    let (mut written, mut bytes) = (0usize, 0usize);
    for (i, f) in todo.iter().enumerate() {
        if let Some(d) = sg_core::freeset::read(f) {
            if sg_core::fsutil::write_file(free.join(f.as_str()), &d) {
                written += 1;
                bytes += d.len();
            }
        }
        if i % 25 == 0 || i + 1 == todo.len() {
            say(&format!("{:>5} / {} files, {:>3} MB", i + 1, todo.len(), bytes / 1_000_000));
        }
    }
    say(&format!("done: {written} files"));
}

/// Writes the free base set to `free` as a pack folder, with progress through `say`.
fn write_free_set(say: &mut dyn FnMut(&str), free: &Path) {
    say("No SimGolf files found; writing the free");
    say(&format!("art set to {}", free.display()));
    say("(once; a PC does this faster: make-free-set.ps1)");
    let files = sg_core::freeset::files();
    let total = files.len();
    let mut written = 0usize;
    let mut bytes = 0usize;
    if !sg_core::fsutil::write_file(free.join("pack.toml"), sg_core::freeset::PACK_TOML.as_bytes()) {
        say("cannot write to the SD card!");
    }
    for (i, f) in files.iter().enumerate() {
        if let Some(d) = sg_core::freeset::read(f) {
            if sg_core::fsutil::write_file(free.join(f), &d) {
                written += 1;
                bytes += d.len();
            }
        }
        if i % 100 == 0 || i + 1 == total {
            say(&format!("{:>5} / {} files, {:>3} MB", i + 1, total, bytes / 1_000_000));
        }
    }
    let index = files.join("\n");
    let _ = sg_core::fsutil::write_file(free.join(sg_core::vfs::INDEX_FILE), index.as_bytes());
    say(&format!("done: {written} files"));
}

impl Demo {
    /// The CPU side of loading, before the GPU exists: the theme's texture catalog, its lighting, the demo course and every
    /// texture the course needs decoded into RAM. `say` reports progress (the startup console).
    fn prepare(game: &Path, say: &mut dyn FnMut(&str)) -> Prepared {
        let theme = "Parkland";
        let theme_dir = sg_core::fsutil::resolve(game, &format!("Data/Textures/{theme}"));
        say(&format!("textures: {}", theme_dir.display()));
        let catalog = TextureCatalog::load(&theme_dir).unwrap_or_else(|e| panic!("textures in {}: {e}", theme_dir.display()));
        let light = sg_core::fsutil::read_file(sg_core::fsutil::resolve(game, &format!("{theme}Lighting.txt")))
            .map(|d| sg_core::terrain::parse_lighting(&sg_core::formats::latin1(&d)))
            .unwrap_or_default();
        say("course: demo 50 x 50");
        let terrain = Terrain::demo_course(50, 50, 7);
        // every texture the course's triangles pick, decoded once
        let mut wanted: BTreeMap<PathBuf, ()> = BTreeMap::new();
        let mut tris: Vec<TileTri> = Vec::new();
        for y in 0..terrain.h {
            for x in 0..terrain.w {
                if terrain.type_at(x, y) == 20 {
                    continue;
                }
                tris.clear();
                build_tile_triangles(&terrain, x, y, &mut tris);
                for t in &tris {
                    if let Some(p) = catalog.pick(t.tex_type, t.set, t.variation) {
                        wanted.entry(p.clone()).or_insert(());
                    }
                }
            }
        }
        // each type's first picture, for the plan's colours (most are among the course's already)
        for ty in 0..sg_core::terrain::TYPE_COUNT as i32 {
            if let Some(p) = catalog.pick(ty, 0, 0) {
                wanted.entry(p.clone()).or_insert(());
            }
        }
        // decoded textures are cached on the SD card after the first run: one read instead of hundreds of file opens
        // one cache per art set: the player's own copy of SimGolf and the free set draw the same names differently
        let art = if sg_core::fsutil::is_dir(Path::new(SD_ROOT).join("game/Data/Textures")) { "game" } else { "free" };
        let cache_file = PathBuf::from(format!("{SD_ROOT}/cache/{theme}-{art}-textures.bin"));
        let t = std::time::Instant::now();
        let mut cache = texcache::read(&cache_file);
        if !cache.is_empty() {
            say(&format!("texture cache: {} images in {:.1}s", cache.len(), t.elapsed().as_secs_f32()));
        }
        let mut cache_dirty = false;
        say(&format!("decoding {} textures", wanted.len()));
        let mut images: BTreeMap<PathBuf, Option<sg_core::assets::Rgba>> = BTreeMap::new();
        let (mut t_read, mut t_decode) = (0.0f32, 0.0f32);
        for (i, p) in wanted.keys().enumerate() {
            let key = texcache::key(p);
            if let Some(img) = cache.get(&key) {
                images.insert(p.clone(), Some(img.clone()));
                continue;
            }
            let ext = sg_core::fsutil::ext_lower(p);
            let t = std::time::Instant::now();
            let data = sg_core::fsutil::read_file(p);
            t_read += t.elapsed().as_secs_f32();
            let t = std::time::Instant::now();
            let img = data.and_then(|d| {
                match ext.as_str() {
                    ".tga" => sg_core::assets::decode_tga(&d),
                    ".pcx" => sg_core::assets::decode_pcx(&d),
                    _ => sg_core::assets::decode_bmp(&d),
                }
                .ok()
            });
            t_decode += t.elapsed().as_secs_f32();
            match &img {
                Some(img) => {
                    cache.insert(key, img.clone());
                    cache_dirty = true;
                }
                None => say(&format!("  cannot decode {}", p.display())),
            }
            images.insert(p.clone(), img);
            if i % 16 == 15 || i + 1 == wanted.len() {
                say(&format!("  {} / {}  (read {t_read:.1}s, decode {t_decode:.1}s)", i + 1, wanted.len()));
            }
        }
        if cache_dirty {
            let t = std::time::Instant::now();
            match texcache::write(&cache_file, &cache) {
                Ok(bytes) => say(&format!("texture cache written: {} MB in {:.1}s", bytes / 1_000_000, t.elapsed().as_secs_f32())),
                Err(e) => say(&format!("texture cache not written: {e}")),
            }
        }
        let type_rgb = (0..sg_core::terrain::TYPE_COUNT as i32)
            .map(|ty| catalog.pick(ty, 0, 0).and_then(|p| images.get(p)).and_then(|i| i.as_ref()).map(plan::average_rgb))
            .collect();
        Prepared { catalog, light, terrain, images, type_rgb, cache, cache_file }
    }

    /// Uploads what `prepare` loaded and builds the course meshes.
    fn new(g: &mut Gfx, p: Prepared) -> (Demo, plan::Plan) {
        let plan = plan::Plan::new(p.type_rgb.clone());
        let mut d = Demo {
            terrain: p.terrain,
            catalog: p.catalog,
            light: p.light,
            textures: BTreeMap::new(),
            tex_ids: std::collections::HashMap::new(),
            chunks: Vec::new(),
            eye_rot: Mat4::rotate(PITCH, 1.0, 0.0, 0.0).mul(&Mat4::rotate(45.0, 0.0, 1.0, 0.0)),
            cam_x: 0.0,
            cam_z: 0.0,
            upp: 8.0,
            cache: p.cache,
            cache_file: p.cache_file,
        };
        d.upload(g, p.images);
        d.rebuild(g);
        (d, plan)
    }

    /// Makes the terrain textures: the 64 x 64 pictures packed into atlases, anything else a texture of its own. Any made
    /// before are freed first.
    fn upload(&mut self, g: &mut Gfx, images: BTreeMap<PathBuf, Option<sg_core::assets::Rgba>>) {
        let mut old: Vec<TextureId> = self.textures.values().flatten().map(|t| t.tex).collect();
        old.sort();
        old.dedup();
        for t in old {
            g.delete_texture(t);
        }
        self.textures.clear();
        self.tex_ids.clear();
        let (cells, others): (Vec<_>, Vec<_>) =
            images.into_iter().partition(|(_, img)| img.as_ref().is_some_and(|i| i.w == CELL && i.h == CELL));
        for group in cells.chunks(256) {
            let size = if group.len() <= 64 { 512 } else { 1024 };
            let per_row = size / CELL;
            let mut px = vec![0u8; (size * size * 4) as usize];
            for (i, (_, img)) in group.iter().enumerate() {
                let img = img.as_ref().unwrap();
                let (ox, oy) = ((i as u32 % per_row) * CELL, (i as u32 / per_row) * CELL);
                for y in 0..CELL {
                    let src = (y * CELL * 4) as usize;
                    let dst = (((oy + y) * size + ox) * 4) as usize;
                    px[dst..dst + (CELL * 4) as usize].copy_from_slice(&img.px[src..src + (CELL * 4) as usize]);
                }
            }
            let tex = g.texture_levels(&sg_core::assets::Rgba { w: size, h: size, px }, ATLAS_LEVELS);
            let s = size as f32;
            for (i, (path, _)) in group.iter().enumerate() {
                let (ox, oy) = ((i as u32 % per_row) * CELL, (i as u32 / per_row) * CELL);
                // inset half a texel so bilinear filtering at a cell's edge does not reach into the next picture
                let rect = [(ox as f32 + 0.5) / s, (oy as f32 + 0.5) / s, (CELL as f32 - 1.0) / s, (CELL as f32 - 1.0) / s];
                self.textures.insert(path.clone(), Some(TexRef { tex, rect }));
            }
        }
        for (path, img) in others {
            let r = img.map(|img| TexRef { tex: g.texture(&img, true), rect: [0.0, 0.0, 1.0, 1.0] });
            self.textures.insert(path, r);
        }
    }

    /// Takes over the game's course after it started one (its land, theme textures and light): the pictures its tiles use
    /// (from the cache, decoding any it lacks and saving them to it), packed and uploaded, and every chunk built.
    fn adopt(&mut self, g: &mut Gfx, app: &app::App) {
        let t0 = std::time::Instant::now();
        self.terrain = app.terrain.clone();
        self.catalog = app.catalog.clone();
        self.light = app.light;
        let mut wanted: BTreeMap<PathBuf, ()> = BTreeMap::new();
        let mut tris: Vec<TileTri> = Vec::new();
        for y in 0..self.terrain.h {
            for x in 0..self.terrain.w {
                tris.clear();
                build_tile_triangles(&self.terrain, x, y, &mut tris);
                for t in &tris {
                    if let Some(p) = self.catalog.pick(t.tex_type, t.set, t.variation) {
                        wanted.entry(p.clone()).or_insert(());
                    }
                }
            }
        }
        let mut images = BTreeMap::new();
        let mut decoded = 0;
        for p in wanted.keys() {
            let key = texcache::key(p);
            let img = match self.cache.get(&key) {
                Some(img) => Some(img.clone()),
                None => {
                    let img = decode_picture(p);
                    if let Some(i) = &img {
                        self.cache.insert(key, i.clone());
                        decoded += 1;
                    }
                    img
                }
            };
            images.insert(p.clone(), img);
        }
        if decoded > 0 {
            let _ = texcache::write(&self.cache_file, &self.cache);
        }
        let n = images.len();
        self.upload(g, images);
        self.rebuild(g);
        println!("[game] course {}x{}: {n} pictures ({decoded} newly decoded) in {:.1}s", self.terrain.w, self.terrain.h, t0.elapsed().as_secs_f32());
    }

    /// The view as the game has it: its camera position, and its zoom as world units per top-screen pixel (render.rs
    /// render_world's sum, with the 400 x 240 screen against the original's 800 x 600).
    fn sync_view(&mut self, app: &app::App) {
        self.cam_x = app.cam_x;
        self.cam_z = app.cam_z;
        let vscale = (screens::TOP_W as f32 / 800.0).min(screens::TOP_H as f32 / 600.0);
        self.upp = 2.0 / (app.view_zoom() * vscale);
    }

    fn texture_for(&mut self, g: &mut Gfx, path: &Path) -> Option<TexRef> {
        if let Some(t) = self.textures.get(path) {
            return *t;
        }
        // not uploaded yet (a texture an edit newly needs): from the cache, else decoded now
        let key = texcache::key(path);
        let img = match self.cache.get(&key) {
            Some(i) => Some(i.clone()),
            None => decode_picture(path),
        };
        let id = img.map(|img| TexRef { tex: g.texture(&img, true), rect: [0.0, 0.0, 1.0, 1.0] });
        self.textures.insert(path.to_path_buf(), id);
        id
    }

    fn chunks_x(&self) -> i32 {
        (self.terrain.w + CHUNK - 1) / CHUNK
    }
    fn chunks_y(&self) -> i32 {
        (self.terrain.h + CHUNK - 1) / CHUNK
    }

    /// The whole course (at load): every chunk.
    fn rebuild(&mut self, g: &mut Gfx) {
        let n = (self.chunks_x() * self.chunks_y()) as usize;
        if self.chunks.len() != n {
            self.chunks.resize_with(n, Chunk::default);
        }
        for i in 0..n {
            self.rebuild_chunk(g, i);
        }
    }

    /// After a stroke changed tiles x0..=x1, y0..=y1 (their type or their corners): the chunks holding them and their
    /// neighbours (a tile's pictures blend with the tiles round it, and its normals use the corners one further out).
    fn rebuild_tiles(&mut self, g: &mut Gfx, [x0, y0, x1, y1]: [i32; 4]) {
        let t = std::time::Instant::now();
        let (ncx, ncy) = (self.chunks_x(), self.chunks_y());
        let (cx0, cx1) = ((x0 - 1).max(0) / CHUNK, (x1 + 1).min(self.terrain.w - 1).max(0) / CHUNK);
        let (cy0, cy1) = ((y0 - 1).max(0) / CHUNK, (y1 + 1).min(self.terrain.h - 1).max(0) / CHUNK);
        let mut n = 0;
        for j in cy0..=cy1.min(ncy - 1) {
            for i in cx0..=cx1.min(ncx - 1) {
                self.rebuild_chunk(g, (j * ncx + i) as usize);
                n += 1;
            }
        }
        if gfx::NET_LOG.load(Ordering::Relaxed) {
            println!("[edit] tiles ({x0},{y0})-({x1},{y1}): {n} chunk(s) in {:.1} ms", t.elapsed().as_secs_f32() * 1000.0);
        }
    }

    /// One chunk's tiles, grouped by texture, one mesh per texture (as `App::rebuild_batches` groups the course), each
    /// vertex carrying its lighting worked out here once (Terrain.dll's model, as `shader/terrain.v.pica` describes).
    fn rebuild_chunk(&mut self, g: &mut Gfx, ci: usize) {
        let ncx = self.chunks_x();
        let (bx, by) = ((ci as i32 % ncx) * CHUNK, (ci as i32 / ncx) * CHUNK);
        // the light, fixed in eye space, and the half vector for the specular term
        let l = sg_core::terrain::light_direction();
        let h = {
            let v = [l[0], l[1], l[2] + 1.0];
            let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / n, v[1] / n, v[2] / n]
        };
        let r = self.eye_rot.0;
        let light_of = |nx: f32, ny: f32, nz: f32| {
            // the normal in eye space (the camera's rotation; not renormalised, as the original does not)
            let n = [r[0] * nx + r[4] * ny + r[8] * nz, r[1] * nx + r[5] * ny + r[9] * nz, r[2] * nx + r[6] * ny + r[10] * nz];
            let d = n[0] * l[0] + n[1] * l[1] + n[2] * l[2];
            let s = if d > 0.0 { (n[0] * h[0] + n[1] * h[1] + n[2] * h[2]).max(0.0).powi(13) } else { 0.0 };
            TerrainVert::pack_light(d.max(0.0), s)
        };
        let mut map: BTreeMap<Option<TextureId>, Vec<TerrainVert>> = BTreeMap::new();
        let mut tris: Vec<TileTri> = Vec::new();
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for y in by..(by + CHUNK).min(self.terrain.h) {
            for x in bx..(bx + CHUNK).min(self.terrain.w) {
                if self.terrain.type_at(x, y) == 20 {
                    continue;
                }
                tris.clear();
                build_tile_triangles(&self.terrain, x, y, &mut tris);
                for t in &tris {
                    let key = (t.tex_type, t.set, t.variation);
                    let tex = match self.tex_ids.get(&key) {
                        Some(id) => *id,
                        None => {
                            let path = self.catalog.pick(t.tex_type, t.set, t.variation).cloned();
                            let id = path.and_then(|p| self.texture_for(g, &p));
                            self.tex_ids.insert(key, id);
                            id
                        }
                    };
                    let rect = tex.map(|t| t.rect).unwrap_or([0.0, 0.0, 1.0, 1.0]);
                    let out = map.entry(tex.map(|t| t.tex)).or_default();
                    for v in t.v.iter() {
                        let p = [v.x, v.y, v.z];
                        for i in 0..3 {
                            lo[i] = lo[i].min(p[i]);
                            hi[i] = hi[i].max(p[i]);
                        }
                        let uv = [rect[0] + v.u * rect[2], rect[1] + v.v * rect[3]];
                        out.push(TerrainVert { pos: p, uv, light: light_of(v.nx, v.ny, v.nz) });
                    }
                }
            }
        }
        if let Some(old) = self.chunks[ci].mesh.take() {
            g.delete_terrain_mesh(old);
        }
        let mut all: Vec<TerrainVert> = Vec::new();
        let mut ranges = Vec::new();
        for (tex, v) in map {
            ranges.push((tex.unwrap_or(g.white), all.len(), v.len()));
            all.extend_from_slice(&v);
        }
        let mesh = if all.is_empty() { None } else { Some(g.terrain_mesh(&all)) };
        self.chunks[ci] = Chunk { mesh, ranges, lo, hi };
    }

    /// The camera's rotation (pitch, then the 45 degree yaw) with the camera position: world to view.
    fn view(&self) -> Mat4 {
        self.eye_rot.mul(&Mat4::translate(-self.cam_x, 0.0, -self.cam_z))
    }
    /// The orthographic projection for the top screen at the current zoom.
    fn proj(&self) -> Mat4 {
        let (hw, hh) = (screens::TOP_W as f32 * self.upp * 0.5, screens::TOP_H as f32 * self.upp * 0.5);
        Mat4::ortho(-hw, hw, -hh, hh, -DEPTH_RANGE, DEPTH_RANGE)
    }

    /// The point of the land under top-screen pixel (sx, sy): (x, height, z) in world units. The view ray through the pixel
    /// is followed down onto the land (a few steps of "where does the ray reach this height"), so a hill in front is what
    /// gets picked, as you see it.
    fn pick(&self, sx: f32, sy: f32) -> [f32; 3] {
        let (w, h) = (screens::TOP_W as f32, screens::TOP_H as f32);
        let (vx, vy) = ((sx - w * 0.5) * self.upp, -(sy - h * 0.5) * self.upp);
        let r = self.eye_rot.0;
        // the ground (y = 0) point first: view x = r00 dx + r02 dz, view y = r10 dx + r12 dz (r[col*4 + row])
        let (a, b, c, d) = (r[0], r[8], r[1], r[9]);
        let det = a * d - b * c;
        if det.abs() < 1e-6 {
            return [self.cam_x, 0.0, self.cam_z];
        }
        let p0 = [self.cam_x + (vx * d - b * vy) / det, 0.0, self.cam_z + (a * vy - c * vx) / det];
        // along the view direction (world vector of view z) the pixel stays the same
        let dir = [r[2], r[6], r[10]];
        if dir[1].abs() < 1e-4 {
            return p0;
        }
        let mut p = p0;
        for _ in 0..6 {
            let hgt = self.terrain.height_at(p[0], p[2]);
            let t = hgt / dir[1];
            p = [p0[0] + dir[0] * t, hgt, p0[2] + dir[2] * t];
        }
        p
    }

    /// The lit uniforms for world geometry drawn through the main shader (the clubhouse): the course's own light.
    fn lit_uniforms(&self) -> Uniforms {
        let mut u = Uniforms::flat(&self.proj(), &self.view());
        u.lit = 1.0;
        const BRIGHTNESS: f32 = 1.74; // calibrated in the game crate's render.rs
        u.light_amb = self.light.ambient.map(|v| v * BRIGHTNESS);
        u.light_dif = self.light.diffuse.map(|v| v * BRIGHTNESS);
        u.light_spec = self.light.specular;
        u.light_dir = sg_core::terrain::light_direction();
        u
    }

    /// A clubhouse stand-in: a real 3D block with a pitched roof, so it stands out of the screen with the 3D slider up. The
    /// game's buildings are sprites; drawn the same way (upright at their footprint's height) they will pop the same way.
    fn render_clubhouse(&self, g: &mut Gfx) {
        let (cx, cz) = CLUBHOUSE;
        let base = self.terrain.height_at(cx, cz);
        let (hx, hz, wall, ridge) = (90.0, 60.0, 45.0, 75.0);
        let (x0, x1, z0, z1) = (cx - hx, cx + hx, cz - hz, cz + hz);
        let (y0, y1, y2) = (base - 5.0, base + wall, base + ridge);
        let walls = [0.92, 0.88, 0.76, 1.0];
        let roof = [0.62, 0.22, 0.16, 1.0];
        let v = |p: [f32; 3], n: [f32; 3], c: [f32; 4]| Vert { pos: p, uv: [0.0, 0.0], normal: n, color: c };
        let mut t: Vec<Vert> = Vec::new();
        let mut quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], n: [f32; 3], col: [f32; 4]| {
            t.extend_from_slice(&[v(a, n, col), v(b, n, col), v(c, n, col), v(a, n, col), v(c, n, col), v(d, n, col)]);
        };
        // walls
        quad([x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0], [0.0, 0.0, -1.0], walls);
        quad([x1, y0, z1], [x0, y0, z1], [x0, y1, z1], [x1, y1, z1], [0.0, 0.0, 1.0], walls);
        quad([x0, y0, z1], [x0, y0, z0], [x0, y1, z0], [x0, y1, z1], [-1.0, 0.0, 0.0], walls);
        quad([x1, y0, z0], [x1, y0, z1], [x1, y1, z1], [x1, y1, z0], [1.0, 0.0, 0.0], walls);
        // the roof: two slopes meeting along the long axis, and the gable ends
        let s = 1.0 / ((ridge - wall) * (ridge - wall) + hz * hz).sqrt();
        quad([x0, y1, z0], [x1, y1, z0], [x1, y2, cz], [x0, y2, cz], [0.0, hz * s, -(ridge - wall) * s], roof);
        quad([x1, y1, z1], [x0, y1, z1], [x0, y2, cz], [x1, y2, cz], [0.0, hz * s, (ridge - wall) * s], roof);
        t.extend_from_slice(&[v([x0, y1, z0], [-1.0, 0.0, 0.0], walls), v([x0, y1, z1], [-1.0, 0.0, 0.0], walls), v([x0, y2, cz], [-1.0, 0.0, 0.0], walls)]);
        t.extend_from_slice(&[v([x1, y1, z1], [1.0, 0.0, 0.0], walls), v([x1, y1, z0], [1.0, 0.0, 0.0], walls), v([x1, y2, cz], [1.0, 0.0, 0.0], walls)]);
        let u = self.lit_uniforms();
        g.stereo_world(true);
        g.tris(Mode::Solid, None, &u, &t);
        g.stereo_world(false);
    }

    /// Draws the course on the current screen: the original's camera, ortho +-5000 depth, pitch about X, then yaw 45. Chunks
    /// wholly off screen are skipped (with a margin for the eyes' stereo offsets).
    fn render_world(&self, g: &mut Gfx) {
        let (w, h) = g.screen_size();
        let (hw, hh) = (w * self.upp * 0.5, h * self.upp * 0.5);
        let proj = Mat4::ortho(-hw, hw, -hh, hh, -DEPTH_RANGE, DEPTH_RANGE);
        let mv = self.eye_rot.mul(&Mat4::translate(-self.cam_x, 0.0, -self.cam_z));
        let mvp = proj.mul(&mv);
        const BRIGHTNESS: f32 = 1.74; // calibrated in the game crate's render.rs
        let u = TerrainUniforms {
            mvp: mvp.0,
            ka: self.light.ambient.map(|v| 0.04 + 0.2 * v * BRIGHTNESS),
            kd: self.light.diffuse.map(|v| 0.8 * v * BRIGHTNESS),
            ks: self.light.specular,
        };
        let margin = 1.0 + g.stereo_margin() + 0.02;
        g.stereo_world(true);
        for c in &self.chunks {
            let Some(mesh) = &c.mesh else { continue };
            let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for i in 0..8 {
                let p = [
                    if i & 1 == 0 { c.lo[0] } else { c.hi[0] },
                    if i & 2 == 0 { c.lo[1] } else { c.hi[1] },
                    if i & 4 == 0 { c.lo[2] } else { c.hi[2] },
                ];
                let q = mvp.apply(p);
                x0 = x0.min(q[0]);
                x1 = x1.max(q[0]);
                y0 = y0.min(q[1]);
                y1 = y1.max(q[1]);
            }
            if x1 < -margin || x0 > margin || y1 < -1.02 || y0 > 1.02 {
                continue;
            }
            for &(tex, first, count) in &c.ranges {
                g.draw_terrain(mesh, first, count, tex, &u);
            }
        }
        g.stereo_world(false);
    }

    /// The course cursor: a white diamond lying on the land under the cursor, drawn in the world so it has the land's
    /// depth in 3D (a flat marker at the screen's surface floated above everything).
    fn render_cursor(&self, g: &mut Gfx, (sx, sy): (f32, f32)) {
        let p = self.pick(sx, sy);
        let s = 5.0 * self.upp;
        let u = Uniforms::flat(&self.proj(), &self.view());
        let y = p[1] + 2.0;
        let q = [
            Vert::new(p[0] - s, y, p[2] - s, 0.0, 0.0),
            Vert::new(p[0] + s, y, p[2] - s, 0.0, 0.0),
            Vert::new(p[0] + s, y, p[2] + s, 0.0, 0.0),
            Vert::new(p[0] - s, y, p[2] + s, 0.0, 0.0),
        ];
        g.stereo_world(true);
        g.quad(Mode::Flat, None, &u, q);
        g.stereo_world(false);
    }

    /// The ball in flight and its shadow. Both are drawn flat on the screen where the world puts them, then moved apart for
    /// each eye by the parallax of their height: the shadow's is the land's; the ball's is capped at 1.5 times the tallest
    /// hill, so a high shot comes out of the screen without splitting into two balls (twice was too much on the console).
    fn render_ball(&self, g: &mut Gfx, ball: &Ball) {
        let (p, ground) = ball.at(self);
        let mvp = self.proj().mul(&self.view());
        let (w, h) = g.screen_size();
        let ui = Uniforms::flat(&Mat4::ortho(0.0, w, h, 0.0, -1.0, 1.0), &Mat4::identity());
        let to_px = |q: [f32; 4], par: f32| ((q[0] + par + 1.0) * 0.5 * w, (1.0 - q[1]) * 0.5 * h);
        let blob = |g: &mut Gfx, (x, y): (f32, f32), rx: f32, ry: f32, c: [f32; 4]| {
            // an octagon as a fan of 6 triangles
            let pts: Vec<(f32, f32)> =
                (0..8).map(|i| (i as f32 * std::f32::consts::FRAC_PI_4).sin_cos()).map(|(s, c)| (x + c * rx, y + s * ry)).collect();
            let mut t = Vec::new();
            for i in 1..7 {
                for &(px, py) in &[pts[0], pts[i], pts[i + 1]] {
                    t.push(Vert::new(px, py, 0.0, 0.0, 0.0).col(c));
                }
            }
            g.tris(Mode::Flat, None, &ui, &t);
        };
        let sh = to_px(mvp.apply([p[0], ground, p[2]]), g.parallax_clip(&mvp, ground));
        blob(g, sh, 3.5, 2.0, [0.0, 0.0, 0.0, 0.45]);
        let bp = to_px(mvp.apply(p), g.parallax_clip(&mvp, p[1].min(1.5 * STEREO_FULL_HEIGHT)));
        blob(g, (bp.0, bp.1), 2.5, 2.5, [1.0, 1.0, 1.0, 1.0]);
    }

}

/// The game's ground batches (whole-course meshes, made by `App::rebuild_batches` when it starts a game or loads one) are
/// freed: the ground is drawn by this frontend's chunks, which an edit rebuilds a few of rather than the whole course.
fn drop_ground_batches(app: &mut app::App, g: &mut Gfx) {
    for b in app.batches.drain(..) {
        g.delete_mesh(b.mesh);
    }
}

/// Two tile rectangles (x0, y0, x1, y1) as the one round both.
fn merge(a: Option<[i32; 4]>, b: [i32; 4]) -> [i32; 4] {
    match a {
        None => b,
        Some(a) => [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
    }
}


/// A terrain picture from its file (bmp, tga or pcx).
fn decode_picture(path: &Path) -> Option<sg_core::assets::Rgba> {
    let ext = sg_core::fsutil::ext_lower(path);
    sg_core::fsutil::read_file(path).and_then(|d| {
        match ext.as_str() {
            ".tga" => sg_core::assets::decode_tga(&d),
            ".pcx" => sg_core::assets::decode_pcx(&d),
            _ => sg_core::assets::decode_bmp(&d),
        }
        .ok()
    })
}

fn main() {
    // New 3DS: the 804 MHz CPU clock and the L2 cache (three times the old 3DS's speed); a no-op on an old 3DS.
    unsafe { ctru_sys::osSetSpeedupEnable(true) };
    let mut apt = Apt::new().expect("apt");
    // Launched over the network from the Homebrew Launcher's NetLoader (3dslink), the console's console output goes
    // back to the PC live: `3dslink -a <console ip> simgolf-3ds.3dsx` from the Simgolf folder, after pressing Y in the
    // launcher. Without a network this does nothing.
    let soc = ctru::services::soc::Soc::new().ok().and_then(|mut s| s.redirect_to_3dslink(true, true).ok().map(|_| s));
    gfx::NET_LOG.store(soc.is_some(), Ordering::Relaxed);
    gfx::trace("main: start");
    let mut hid = Hid::new().expect("hid");
    // the graphics service lives for the whole program: the render targets borrow its screens (gfx.rs)
    let gfx_svc: &'static ctru::services::gfx::Gfx = Box::leak(Box::new(ctru::services::gfx::Gfx::new().expect("gfx")));
    // a panic writes its message to the SD card (readable on a PC or from an emulator's virtual card), then shows it in the
    // system's error dialog where that can run; never a silent black screen
    {
        let applet_hook = {
            ctru::set_panic_hook(true);
            std::panic::take_hook()
        };
        std::panic::set_hook(Box::new(move |info| {
            let _ = std::fs::write(format!("{SD_ROOT}/panic.txt"), format!("{info}\n"));
            applet_hook(info);
        }));
    }

    // Startup narrates itself on a text console on the top screen; the console is dropped before the GPU takes the screens.
    let (game, prepared) = {
        let mut console = Console::new(gfx_svc.top_screen.borrow_mut());
        let t0 = std::time::Instant::now();
        let mut say = |line: &str| {
            println!("[{:5.1}s] {line}", t0.elapsed().as_secs_f32());
            use ctru::services::gfx::{Flush, Swap};
            console.flush_buffers();
            console.swap_buffers();
        };
        say("OpenSGolf for the 3DS: starting");
        let game = game_dir(&mut say);
        say(&format!("game folder: {}", game.display()));
        let prepared = Demo::prepare(&game, &mut say);
        say("starting the GPU");
        (game, prepared)
    };
    let mut g = Gfx::new(gfx_svc);
    gfx::trace("main: Gfx::new done");
    let (mut demo, mut plan) = Demo::new(&mut g, prepared);
    gfx::trace("main: Demo::new done (textures + meshes uploaded)");
    let mut input = input::Input::new();
    // the C-stick (New 3DS) is read through the irrst service; on an old 3DS it stays at rest and the D-pad pans
    let cstick = unsafe { ctru_sys::irrstInit() } >= 0;

    let mixer = std::sync::Arc::new(sg_core::mixer::Mixer::new(&sg_core::fsutil::resolve(&game, "Sounds")));
    // sound mixes on its own core (audio.rs); without a DSP firmware dump the game runs silent
    let audio = audio3ds::AudioThread::start(mixer.clone(), &mut apt);
    match &audio {
        Some(a) => println!("[audio] on core {}", a.core),
        None => println!("[audio] off: ndsp did not start (dump the DSP firmware with DSP1 once)"),
    }
    // ---- the game (milestone 2): the real club, its money, rules, holes and golfers ----------------------------------------
    // A new game on a Parkland property (the theme the textures were prepared for), the normal game rather than a sandbox, so
    // building costs money. The ground is drawn by this frontend's chunked renderer (Demo); everything on it by the game.
    // saves (INFO's SAVE and LOAD, and the game's autosave) on the card beside the game's other files
    let saves = "sdmc:/3ds/opensgolf/saves";
    let _ = std::fs::create_dir_all(saves);
    std::env::set_var("OPENSGOLF_SAVE_DIR", saves);
    let text = text::Text::new(&mut g);
    let mut hud = hud::Hud::new();
    gfx::trace("game: App::new");
    let mut app = app::App::new(game.clone());
    gfx::trace("game: App::new done");
    app.mixer = Some(mixer.clone());
    app.draw_w = screens::TOP_W as f32;
    app.draw_h = screens::TOP_H as f32;
    // the cheapest parkland plot of this game's deal (the desktop's chooser lets the player pick one they can afford; a dear
    // one dealt first left the club deep in debt)
    let props = sg_core::data::properties();
    let prop = (0..props.len()).filter(|&i| props[i].theme == 0).min_by_key(|&i| app.offer_for(i).1).unwrap_or(0);
    let t_game = std::time::Instant::now();
    gfx::trace(&format!("game: start_game (property {prop})"));
    app.start_game(&mut g, prop, false);
    drop_ground_batches(&mut app, &mut g);
    let started = format!("game: started in {:.1}s: {} on a {}x{} map", t_game.elapsed().as_secs_f32(), app.course_name, app.terrain.w, app.terrain.h);
    gfx::trace(&started);
    // between the game's middle and near levels: the plan's tiles at the size the stylus design was tried at
    app.zoom = app::ZOOM_UNIT * 2f32.powf(1.5);
    // every golfer body's clips decoded ahead on a spare core (sg_core::preload), so a new kind of golfer arriving does not
    // stop the game while its 50 clips load
    let mut jobs = Vec::new();
    for prefix in app::GOLFER_BODIES.iter().filter(|p| !p.is_empty()) {
        for &(base, name) in app::GOLFER_CLIPS.iter() {
            let name = if base == 0x28 && prefix.starts_with("Female") { "NormalSwing" } else { name };
            jobs.push((app.game_path(&format!("Flics/{prefix}_{name}.flc")), false));
            jobs.push((app.game_path(&format!("Flics/{prefix}_{name}Shadow.flc")), true));
        }
    }
    let preload_core = start_preload(jobs);
    gfx::trace(&format!("preload: {}", preload_core.map(|c| format!("on core {c}")).unwrap_or_else(|| "off".into())));
    demo.adopt(&mut g, &app);
    gfx::trace("game: ground adopted");
    demo.sync_view(&app);
    plan.stale = true;
    gfx::trace("main: game started");
    let mut last_step = std::time::Instant::now();
    // Start's pause menu: whether the game was paused before it opened (it is put back on Resume)
    let mut menu_was_paused = false;
    // the tiles the strokes of this frame changed (the ground is rebuilt there when the game marks the course changed)
    let mut changed: Option<[i32; 4]> = None;
    gfx::trace(&format!("textures: {} in VRAM, {} in main memory", g.tex_vram, g.tex_linear));

    // Redraw only when something changed (input, the 3D slider, an edit): a still view leaves the CPU and GPU idle until the
    // next vertical blank, which is most of the time in a sim like this one, and saves battery. The demo has no animation;
    // the game's will mark the view dirty every frame while anything moves.
    let mut dirty = true;
    let mut last_slider = -1.0f32;
    // a once-every-two-seconds performance line for the PC log (3dslink): frames drawn, average CPU time per frame, draws
    let (mut perf_t, mut perf_frames, mut perf_cpu, mut perf_draws) = (std::time::Instant::now(), 0u32, 0.0f32, 0u32);
    let (mut perf_sim, mut perf_loops, mut perf_reports) = (0.0f32, 0u32, 0u32);
    while apt.main_loop() && !QUIT.load(Ordering::Relaxed) {
        hid.scan_input();
        if cstick {
            unsafe { ctru_sys::irrstScanInput() };
        }
        if hid.keys_held().contains(KeyPad::START) && hid.keys_held().contains(KeyPad::SELECT) {
            break;
        }
        demo.sync_view(&app);
        let mut strokes: Vec<plan::Stroke> = Vec::new();
        let mut menu_pick: Option<usize> = None;
        for ev in input.poll(&hid, cstick) {
            dirty = true;
            // Start's pause menu takes the controls while it is open: D-pad or a tap to choose, A or a tap to pick, B or
            // Start to go back
            if let Some(sel) = hud.menu {
                let n = hud::MENU.len();
                match ev {
                    input::Event::KeyDown(shim::KeyCode::Up, _) => hud.menu = Some((sel + n - 1) % n),
                    input::Event::KeyDown(shim::KeyCode::Down, _) => hud.menu = Some((sel + 1) % n),
                    input::Event::MouseDown(shim::MouseButton::Left, _, _) => menu_pick = Some(sel),
                    input::Event::MouseDown(shim::MouseButton::Right, _, _) | input::Event::KeyDown(shim::KeyCode::Escape, _) => {
                        menu_pick = Some(0)
                    }
                    input::Event::Pen(phase, x, y) => {
                        if let Some(i) = hud::Hud::menu_hit(x, y) {
                            hud.menu = Some(i);
                            if matches!(phase, shim::TouchPhase::Ended) {
                                menu_pick = Some(i);
                            }
                        }
                    }
                    _ => {}
                }
                if menu_pick.is_some() {
                    mixer.play("Interface/Button1.wav", 0.7, false);
                } else if matches!(ev, input::Event::KeyDown(shim::KeyCode::Up | shim::KeyCode::Down, _)) {
                    mixer.play("Interface/Button2.wav", 0.5, false);
                }
                continue;
            }
            match ev {
                input::Event::Pan(dx, dy) => {
                    // screen right is world +x+z along the 45 degree yaw; screen down is +z-x
                    let (sx, sz) = (dx * demo.upp, dy * demo.upp);
                    let r = std::f32::consts::FRAC_1_SQRT_2;
                    app.cam_x += (sx - sz) * r;
                    app.cam_z += (sx + sz) * r;
                    // the plan's camera too, or the line after this match copies it back and undoes the pan
                    demo.cam_x = app.cam_x;
                    demo.cam_z = app.cam_z;
                    plan.stale = true;
                }
                // L / R: the game's zoom levels
                input::Event::Zoom(z) => {
                    // half the game's steps (its levels double): the small screens want the in-between sizes too
                    // three steps closer than the PC game goes (its nearest is ZOOM_UNIT x 4), for a close look on the small screen
                    let (near, far) = (app::ZOOM_UNIT * 4.0 * 2f32.powf(1.5), app::ZOOM_UNIT * (app::far_level() as f32).exp2());
                    let f = if z > 0 { std::f32::consts::SQRT_2 } else { std::f32::consts::FRAC_1_SQRT_2 };
                    app.zoom = (app.zoom * f).clamp(far * 0.999, near * 1.001);
                }
                input::Event::Pen(phase, x, y) => {
                    // the bottom screen: the plan and its tools (plan.rs), editing through the game
                    // a press on the map puts away a panel or card on the top screen (it still does its thing on the map)
                    if matches!(phase, shim::TouchPhase::Started) && x < plan::PLAN_W {
                        hud.close();
                    }
                    strokes.push(plan.pen(&mut demo, &mut app, phase, x, y, input.shoulder()));
                }
                input::Event::KeyDown(shim::KeyCode::Space, _) => {
                    // X: pause, as Space in the original
                    app.paused = !app.paused;
                    mixer.play("Interface/Button2.wav", 0.6, false);
                }
                input::Event::KeyDown(shim::KeyCode::Escape, _) => {
                    // Start: the pause menu (the game stops while it is up)
                    menu_was_paused = app.paused;
                    app.paused = true;
                    hud.close();
                    hud.menu = Some(0);
                    mixer.play("Interface/Button1.wav", 0.7, false);
                }
                input::Event::KeyDown(shim::KeyCode::F1, _) => {
                    // Select: money, holes, golfers, staff, then none
                    hud.cycle();
                    mixer.play("Interface/Button2.wav", 0.5, false);
                }
                input::Event::KeyDown(shim::KeyCode::Tab, _) => {
                    // Y: the strip's next tab
                    plan.next_tab();
                    mixer.play("Interface/Button2.wav", 0.5, false);
                }
                input::Event::KeyDown(k @ (shim::KeyCode::Q | shim::KeyCode::E), _) => {
                    // ZL / ZR: slower / faster, through pause: paused, 1, 2, 4
                    let steps = [0u32, 1, 2, 4];
                    let at = if app.paused { 0 } else { steps.iter().position(|&s| s == app.speed as u32).unwrap_or(1) };
                    let to = if k == shim::KeyCode::Q { at.saturating_sub(1) } else { (at + 1).min(3) };
                    if to == 0 {
                        app.paused = true;
                    } else {
                        app.paused = false;
                        app.speed = steps[to] as _;
                    }
                    hud.note(match to {
                        0 => "Paused",
                        1 => "Speed: normal",
                        2 => "Speed: fast",
                        _ => "Speed: fastest",
                    });
                    mixer.play("Interface/Button2.wav", 0.5, false);
                }
                input::Event::MouseDown(shim::MouseButton::Right, _, _) => {
                    // B: the game's undo, as the UNDO button
                    strokes.push(plan::Stroke { action: Some(plan::Action::Undo), ..Default::default() });
                }
                input::Event::MouseDown(shim::MouseButton::Left, mx, my) => {
                    // A: the armed tool once at the top screen's cursor, as a tap on the plan there would
                    let p = demo.pick(mx, my);
                    strokes.push(plan.tap_world(&mut demo, &mut app, p[0], p[2]));
                }
                _ => {}
            }
            // the plan's hand and edge slide move the view: the game's camera follows
            app.cam_x = demo.cam_x;
            app.cam_z = demo.cam_z;
        }
        if let Some(pick) = menu_pick {
            hud.menu = None;
            app.paused = menu_was_paused;
            match pick {
                1 => strokes.push(plan::Stroke { action: Some(plan::Action::Save), ..Default::default() }),
                2 => strokes.push(plan::Stroke { action: Some(plan::Action::Load), ..Default::default() }),
                3 => hud.show(plan::Panel::Help),
                4 => QUIT.store(true, Ordering::Relaxed),
                _ => {}
            }
        }
        // a pen held at the plan's edge slides the view and paints on (plan.rs); not under the pause menu
        if hud.menu.is_none() {
            strokes.push(plan.tick(&mut demo, &mut app));
        }
        let mut full_rebuild = false;
        for stroke in strokes {
            if stroke.scrolled {
                dirty = true;
                app.cam_x = demo.cam_x;
                app.cam_z = demo.cam_z;
            }
            if let Some(snd) = stroke.sound {
                mixer.play(snd, 0.8, false);
            }
            if let Some(r) = stroke.changed {
                changed = Some(merge(changed, r));
            }
            full_rebuild |= stroke.rebuilt;
            let Some(action) = stroke.action else { continue };
            dirty = true;
            match action {
                plan::Action::Undo => {
                    // the course and the money as they were before the last stroke, up to 30 back
                    if app.undo_step(&mut g) {
                        drop_ground_batches(&mut app, &mut g);
                        app.dirty = true;
                        full_rebuild = true;
                        plan.stale = true;
                        mixer.play("Interface/Button2.wav", 0.8, false);
                    } else {
                        hud.note("Nothing to undo");
                    }
                }
                plan::Action::Pause => app.paused = !app.paused,
                plan::Action::Speed => {
                    // the game's speeds (Space on the desktop): 1, 2, 4; from a pause it simply plays on
                    if app.paused {
                        app.paused = false;
                    } else {
                        app.speed = match app.speed {
                            1 => 2,
                            2 => 4,
                            _ => 1,
                        };
                    }
                }
                plan::Action::Panel(p) => hud.toggle(p),
                plan::Action::Staffed => hud.show(plan::Panel::Staff),
                plan::Action::Save => {
                    app.quick_save();
                    // the game's message names the desktop's keys
                    if app.ticker.text.starts_with("Quick saved") {
                        app.ticker.text = "Game saved. INFO tab, LOAD brings it back.".into();
                    }
                }
                plan::Action::Load => {
                    let path = files_ui::saved_games_dir().join(comfort::QUICK_FILE);
                    match app.load_game(&mut g, &path) {
                        Ok(()) => {
                            drop_ground_batches(&mut app, &mut g);
                            demo.adopt(&mut g, &app);
                            demo.sync_view(&app);
                            plan.stale = true;
                            changed = None;
                            hud.note("Loaded the saved game");
                        }
                        Err(e) => hud.note(&format!("No save to load ({e})")),
                    }
                }
                plan::Action::Thoughts => hud.note(if plan.thoughts { "Golfers' thoughts on" } else { "Golfers' thoughts off" }),
                plan::Action::Inspect(wx, wz) => hud.inspect(&app, wx, wz),
                plan::Action::Note(s) => hud.note(s),
            }
        }
        if full_rebuild {
            app.dirty = true;
            changed = None;
        }
        // the view stays over the course: its centre may go to the map's edge, not past it
        {
            let (hx, hz) = (app.terrain.w as f32 * sg_core::terrain::TILE_SIZE * 0.5, app.terrain.h as f32 * sg_core::terrain::TILE_SIZE * 0.5);
            app.cam_x = app.cam_x.clamp(-hx, hx);
            app.cam_z = app.cam_z.clamp(-hz, hz);
            demo.cam_x = app.cam_x;
            demo.cam_z = app.cam_z;
        }
        // golfers on the plan, and the top screen's thoughts, money and messages
        plan.golfers.clear();
        for gg in app.club.g.iter().filter(|gg| gg.hole > 0 && gg.hole < 19) {
            plan.golfers.push(app.units_to_world(gg.x, gg.y));
        }
        hud.update(&app, plan.thoughts);
        if hud.animating() || app.ticker.showing {
            dirty = true;
        }
        if app.paused {
            app.ticker.step_wall(app.clock);
        }
        // the club's clock: golfers, staff, money (only on the course and not while paused, as the desktop runs it)
        perf_loops += 1;
        let dt = last_step.elapsed().as_secs_f32().clamp(0.0, 0.25);
        last_step = std::time::Instant::now();
        if app.screen == app::Screen::Play && !app.paused {
            let ts = std::time::Instant::now();
            app.step_game(dt);
            perf_sim += ts.elapsed().as_secs_f32();
            dirty = true; // the golfers move
        }
        // the course changed (a stroke, an undo): the ground mirrored and rebuilt where it changed, the trees and the holes
        // brought up to date (the desktop's rebuild_batches step, with this frontend drawing the ground)
        if app.dirty {
            app.dirty = false;
            drop_ground_batches(&mut app, &mut g);
            // paths, mud and retaining walls follow the edit (the ground itself is Demo's)
            app.rebuild_overlays(&mut g);
            demo.terrain = app.terrain.clone();
            match changed.take() {
                Some(r) => demo.rebuild_tiles(&mut g, r),
                None => demo.rebuild(&mut g),
            }
            app.refresh_trees();
            app.report_course(false);
            plan.stale = true;
            dirty = true;
        }
        changed = None;
        if miniquad::window::quit_requested() {
            break;
        }
        let slider = unsafe { ctru_sys::osGet3DSliderState() };
        if slider != last_slider {
            last_slider = slider;
            dirty = true;
        }
        if !dirty {
            gfx_svc.wait_for_vblank();
            continue;
        }
        dirty = false;

        let t = std::time::Instant::now();
        g.begin_frame();
        // the plan's picture, after an edit (now, while the GPU is not reading it)
        plan.refresh(&mut g, &demo, &app);
        g.set_stereo_span(STEREO_FULL_HEIGHT, PITCH as f32, DEPTH_RANGE);
        let eyes: &[Eye] = if g.slider() > 0.0 { &[Eye::Left, Eye::Right] } else { &[Eye::Left] };
        for &eye in eyes {
            g.begin_screen(Screen::Top, eye, [0x2c, 0x3e, 0x50]);
            demo.render_world(&mut g);
            // everything on the ground, drawn by the game (its ground batches are dropped: the ground is the chunks above):
            // the retaining walls, paths, water glints, trees, buildings, golfers, staff, the pro's aim line
            g.stereo_world(true);
            app.render_world(&mut g);
            g.stereo_world(false);
            if plan.pen.is_some() {
                plan.render_top(&mut g, &demo); // the brush under the pen, on the land
            } else {
                demo.render_cursor(&mut g, input.cursor);
            }
            // the information over it, at the screen's depth (hud.rs)
            g.stereo_world(false);
            hud.render(&mut g, &text, &app);
        }
        // a hole built (a tee and a green) but not open: the status line says how to open it
        plan.hint = {
            let h = app.club.next_hole;
            let ready = (1..19).contains(&h) && {
                let r = &app.club.holes[h as usize];
                r.par == 0 && r.back.0 != 0 && r.pin.0 != 0
            };
            ready.then(|| format!("HOLE {h} IS BUILT: TAP ITS GREEN TO OPEN IT"))
        };
        g.begin_screen(Screen::Bottom, Eye::Left, [0x10, 0x14, 0x10]);
        plan.render(&mut g, &demo, &app, &text, input.cursor);
        hud.render_menu_bottom(&mut g, &text);
        let cpu = t.elapsed().as_secs_f32();
        let draws = g.draws;
        g.end_frame();

        perf_frames += 1;
        perf_cpu += cpu;
        perf_draws += draws;
        if perf_t.elapsed().as_secs_f32() >= 2.0 {
            let line = format!(
                "perf: {} frames in 2 s, {:.1} ms drawing + {:.1} ms game per frame, {} draws per frame; {}",
                perf_frames,
                perf_cpu * 1000.0 / perf_frames.max(1) as f32,
                perf_sim * 1000.0 / perf_loops.max(1) as f32,
                perf_draws / perf_frames.max(1),
                gfx::mem_line()
            );
            perf_reports += 1;
            if gfx::NET_LOG.load(Ordering::Relaxed) || perf_reports <= 600 {
                gfx::trace(&line);
            }
            (perf_t, perf_frames, perf_cpu, perf_draws) = (std::time::Instant::now(), 0, 0.0, 0);
            (perf_sim, perf_loops) = (0.0, 0);
        }
    }
    sg_core::preload::STOP.store(true, Ordering::Relaxed);
    drop(audio);
    if cstick {
        unsafe { ctru_sys::irrstExit() };
    }
    // stdout/stderr still point at the 3dslink socket and are closed during process exit, after main returns; tearing the
    // socket service down here made that close fault (data abort in _close_r on quitting a network-launched run)
    std::mem::forget(soc);
}

/// The preload worker's entry: decodes its list (sg_core::preload::run), then ends.
unsafe extern "C" fn preload_main(arg: *mut core::ffi::c_void) {
    let jobs = unsafe { Box::from_raw(arg as *mut Vec<(std::path::PathBuf, bool)>) };
    sg_core::preload::run(*jobs);
}

/// Starts the sprite preloader on a spare core at a lower priority than the game (core 2 on a New 3DS, where the sound mixer
/// also lives but mostly sleeps; else the system core's application share). The core used, or None.
fn start_preload(jobs: Vec<(std::path::PathBuf, bool)>) -> Option<i32> {
    let mut new3ds = false;
    let _ = unsafe { ctru_sys::APT_CheckNew3DS(&mut new3ds) };
    let mut prio = 0x30;
    let _ = unsafe { ctru_sys::svcGetThreadPriority(&mut prio, 0xFFFF8000) };
    let arg = Box::into_raw(Box::new(jobs));
    for core in if new3ds { vec![2, 1] } else { vec![1] } {
        let t = unsafe { ctru_sys::threadCreate(Some(preload_main), arg as *mut _, 256 * 1024, (prio + 2).min(0x3f), core, true) };
        if !t.is_null() {
            return Some(core);
        }
    }
    drop(unsafe { Box::from_raw(arg) });
    None
}
