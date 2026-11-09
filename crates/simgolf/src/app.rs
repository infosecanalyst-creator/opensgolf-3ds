//! The game state and rules that sit above sg-core: the course and its scenery, the golfers on it, money, editing, the in-game
//! panels and the story banner. Drawing lives in render.rs, input and the window in main.rs.
use crate::gfx::{Gfx, Mesh, Vert};
use crate::render::THEME_PACKS;
use crate::ui::{money, Image};
use miniquad::TextureId;
use sg_core::action::Action;
use sg_core::course::Course;
use sg_core::economy::{self, Economy};
use sg_core::fsutil::resolve;
use sg_core::golfer::{self as golf, Club};
use sg_core::land::{self, ExeRng, Land, Noise, Slot};
use sg_core::lang::{tr, trf};
use sg_core::mixer::Mixer;
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
    /// A ball washer on this tile (x, y) that plays its frames while a golfer washes (the tile's counter).
    pub grow_tile: Option<(i32, i32)>,
    /// A newly planted tree, scenic tree or flower bed on this tile (x, y) that shows its growth as its frame
    /// (`App::sprouts`); true for the sprite whose last frame ends the growing (the tree or the bed piece).
    pub sprout: Option<(i32, i32, bool)>,
    /// A flag or tee marker, rebuilt from the hole records every update.
    pub decor: bool,
    /// A celebrity resident (index into Club::residents).
    pub resident: Option<usize>,
    /// Ambient life and water effects (animals, fly-overs, splash, ripples, rocks, dolphins), rebuilt every update.
    pub ambient: bool,
    /// Lies level rather than standing up (the blimp): Enhanced graphics' water reflection draws it the right way up at its
    /// mirrored height, as a level shape's reflection runs along the same line on the ground (fx_reflect.rs; issue #147).
    pub level: bool,
    /// Drawn size (1 = the sprite's own) and height of the body above the ground in world units (the shadow stays down).
    pub scale: f32,
    pub lift: f32,
    /// A golfer's colours: the clip is drawn in the palette composed for this outfit.
    pub outfit: Option<sg_core::bodies::Outfit>,
    /// The exe's view of the sprite whatever the camera (sprites the exe queues in a fixed view).
    pub view: Option<i32>,
    /// The sprite moved this many 800 x 600 pixels right and down on the screen from its ground point (the exe's screen
    /// offsets); the prop still sorts by its ground point.
    pub shift: (f32, f32),
    /// A flower bed piece: its sprite id, palette id (0xbb while weedy, drawn in the piece's last palette, see
    /// `App::bed_palette`) and tile.
    pub bed: Option<(u16, u8, i32, i32)>,
    /// The animated layer of a TV tower or booth standing at this tile: its frame follows `sg_core::objects::tv_frame`.
    pub tv: Option<(i32, i32)>,
    /// An ambient animal's or fly-over's identity from tick to tick (0: none), for Enhanced smooth motion (enhanced.rs).
    pub track: u32,
    /// Added to the game tick for an animated prop's frame (a driving range's flags wave out of step).
    pub phase: u32,
    /// Drawn mirrored left to right about its anchor (a left-handed golfer's swing, the Left-handed golfers extra): an 8 view
    /// sprite then takes the view whose mirror image faces the prop's heading (`prop_view`).
    pub mirror: bool,
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
            sprout: None,
            decor: false,
            resident: None,
            ambient: false,
            level: false,
            scale: 1.0,
            lift: 0.0,
            outfit: None,
            view: None,
            shift: (0.0, 0.0),
            bed: None,
            tv: None,
            track: 0,
            phase: 0,
            mirror: false,
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
    /// The brush's name (a text key).
    pub name_key: &'static str,
    pub ty: i32,
    pub vbyte: i32,
}

pub const PAINT: [PaintEntry; 24] = [
    PaintEntry { name_key: "STR_PAINT_FAIRWAY", ty: 2, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_FIRM_FAIRWAY", ty: 3, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_GREEN", ty: 1, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_TEE", ty: 0, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_ROUGH", ty: 4, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_DEEP_ROUGH", ty: 5, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_WOODS", ty: 13, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_SAND_BUNKER", ty: 7, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_POT_BUNKER", ty: 9, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_WATER_SHALLOW", ty: 17, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_WATER_MIDDLE", ty: 17, vbyte: 1 },
    PaintEntry { name_key: "STR_PAINT_WATER_DEEP", ty: 17, vbyte: 2 },
    PaintEntry { name_key: "STR_PAINT_ROCK", ty: 12, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_BRUSH", ty: 11, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_BUILDING_LOT", ty: 22, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_CLIFF", ty: 31, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_RAVINE", ty: 32, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_FLOWER_BED", ty: 33, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_ZEN_SAND", ty: 34, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_GRASS_BUNKER", ty: 35, vbyte: 0 },
    // the rest of the Build Course panel's buttons, which paint these exe tile ids directly
    PaintEntry { name_key: "STR_PAINT_WASTE_BUNKER", ty: 8, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_STREAM", ty: 10, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_PINE_TREE", ty: 14, vbyte: 0 },
    PaintEntry { name_key: "STR_PAINT_PALM_TREE", ty: 15, vbyte: 0 },
];

/// Building kinds the Add Buildings panel offers, in the exe's order (kind numbers of `sg_core::data::buildings()`): benches, flower beds,
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
    /// Select Difficulty, between Start New Game (or Sandbox Mode) and the property chooser.
    Difficulty,
    /// A popup menu over the game (Information, System Functions, Preferences).
    Popup,
    /// The pro's skill dialog.
    Skills,
    /// The SGA report or tournament offer, the preparation checklist, the tournament results.
    Sga,
    Prep,
    Results,
    /// Load Previous Game, and the championship course and Pick A Pro choosers built on it (files_ui).
    Files,
    /// Select a Theme Pack.
    Themes,
    /// The credits, from the title logo.
    Credits,
    /// The Hole Stats dialog behind a Course Report row, Best Scores and Top 10 Designers.
    HoleStats,
    BestScores,
    Top10,
    /// SELECT THE NEXT PAIR OF GOLFERS, the accomplishments board, the year-end report, the Membership Roster.
    Pair,
    Board,
    YearEnd,
    Roster,
    /// Customise Golfer, the player's own character editor.
    Customise,
    /// F2 Player Comments, F3 Histograph, F4 Financial Report, F5 Routing Map, F8 Keyboard Shortcuts.
    Comments,
    Histograph,
    Finance,
    Routing,
    Shortcuts,
    /// Preferences > Content: the installed content packs (the port's own screen, docs/MODDING.md).
    Content,
    /// Start New Game with scenario packs installed: a free game or a scenario (the port's own, docs/MODDING.md 5).
    Scenarios,
    /// Information > Scenario goals: the goals of the game and how they stand.
    Goals,
    /// Preferences > Keyboard controls: every key of the game (the port's own screen, keys_ui.rs).
    Keys,
    /// OpenSGolf extras (extras.rs): the People screen, the message history and a golfer's profile page.
    People,
    History,
    Profile,
    /// OpenSGolf extras: comments by hole (insight.rs).
    HoleComments,
    /// OpenSGolf extras: history graphs (graphs.rs).
    Graphs,
    /// The course editor's box over the course (editor.rs; the port's own).
    Editor,
}

/// Where saved games, championship courses and pros, accomplishment snapshots, packs and settings go. The browser keeps them
/// in its own storage under plain names; Android in the app's internal data folder; on the desktop they go in the user's data folder (OPENSGOLF_SAVE_DIR, or the older
/// SIMGOLF_SAVE_DIR, overrides it), not wherever the game happened to be started from. A player who already has a folder from
/// before the project became OpenSGolf (named SimGolf / simgolf) keeps using it.
pub fn save_dir() -> PathBuf {
    #[cfg(target_arch = "wasm32")]
    {
        PathBuf::new()
    }
    // Android: the app's internal data folder (android.rs)
    #[cfg(target_os = "android")]
    {
        crate::android::files_dir()
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        if let Some(d) = env("OPENSGOLF_SAVE_DIR").or_else(|| env("SIMGOLF_SAVE_DIR")) {
            return d;
        }
        let (base, new, old) = if cfg!(target_os = "windows") {
            (env("APPDATA"), "OpenSGolf", "SimGolf")
        } else if cfg!(target_os = "macos") {
            (env("HOME").map(|h| h.join("Library/Application Support")), "OpenSGolf", "SimGolf")
        } else {
            (env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share"))), "opensgolf", "simgolf")
        };
        let Some(base) = base else { return PathBuf::new() };
        let (new, old) = (base.join(new), base.join(old));
        if !new.exists() && old.is_dir() {
            return old;
        }
        new
    }
}

/// A whole game as saved: the land and terrain, the golfers and holes, the staff, the money and calendar, and the exe's random
/// generator, so a loaded game goes on exactly as it would have. Stored as JSON compressed with zlib, after a magic line.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveGame {
    /// 1: the original's 50 x 50 map; 2: a larger course size, in `size`.
    pub version: u32,
    /// The map's edge in tiles (sg_core::mapsize; absent in version 1 saves, which are 50).
    #[serde(default = "original_size", skip_serializing_if = "is_original_size")]
    pub size: i32,
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
    /// The theme pack folder the game was played with (older saves: Standard).
    #[serde(default)]
    pub theme_pack: String,
    /// The Top 10 course id (older saves: none).
    #[serde(default)]
    pub course_id: Option<i32>,
    /// The year each property was bought (older saves: none recorded).
    #[serde(default)]
    pub bought: Vec<i32>,
    /// The goals of the game: its scenario's and its scripts' (docs/MODDING.md 5 and 6; older saves: none).
    #[serde(default, skip_serializing_if = "sg_core::scenario::Goals::is_empty")]
    pub goals: sg_core::scenario::Goals,
    /// The state of each pack's scripts (`this` and the script's random generator), by pack id.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub scripts: std::collections::BTreeMap<String, sg_core::script::PackState>,
    /// What the OpenSGolf extras keep (extras.rs): the message history, employees' names, what golfers said, bios.
    #[serde(default, skip_serializing_if = "crate::extras::Kept::is_empty")]
    pub extras: crate::extras::Kept,
    /// A course shared by course code or file (share.rs): its course record is the challenge to beat.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shared: bool,
    /// The content packs loaded when the game was saved (content_profiles.rs; a record with its own version; older saves:
    /// none, and they load without asking).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packs: Option<crate::content_profiles::SavedPacks>,
    /// A game brought over from a saved game of the original SimGolf (share.rs, sg_core::sve): what came over and what did
    /// not, told to the player when the game is loaded. The game's own saves do not write it again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported: Option<String>,
    /// The cosmetic generator (`App::deco_rng`), which steps the celebrity residents' walk: saved only while the club has
    /// residents, so they walk on after a load as they would have (older saves: none, and the generator goes on as it is).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deco_rng: Option<ExeRng>,
}

const SAVE_MAGIC: &[u8] = b"SIMGOLF-NATIVE-SAVE\n";

/// The newest save version this build reads.
const SAVE_VERSION: u32 = 2;

fn original_size() -> i32 {
    sg_core::mapsize::ORIGINAL
}

fn is_original_size(n: &i32) -> bool {
    *n == sg_core::mapsize::ORIGINAL
}

/// Terrain.dll's normals for the faces on a tile's sides (0x10063c40: north (y - 1), south, west, east, 0x18 bytes each), the
/// first for views 0 and 2, the second for 1 and 3. Shared by the property's cliff and the retaining walls; not unit length.
const EDGE_NORMALS: [[[f32; 3]; 2]; 4] = [
    [[0.707, 0.0, 0.707], [1.0, 1.0, 0.0]],
    [[-0.707, 0.0, -0.707], [-1.0, 1.0, 0.0]],
    [[-1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
    [[1.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
];

/// Reads a whole game saved by `App::save_game` (also used for the Load screen's preview of a save).
/// A timing line for the 3DS log (simgolf-3ds gfx.rs `trace`); nothing on the other platforms.
#[cfg(target_os = "horizon")]
fn stage(s: &str) {
    crate::gfx::trace(s);
}
#[cfg(not(target_os = "horizon"))]
fn stage(_s: &str) {}

pub fn read_save(path: &Path) -> Result<SaveGame, String> {
    let data = sg_core::fsutil::read_file(path).ok_or_else(|| trf("STR_ERROR_FILE_NOT_FOUND", &[("FILE", &path.display())]))?;
    read_save_bytes(&data, &path.display().to_string())
}

/// Reads a whole game from the bytes of a save file (`name` for the messages): a scenario pack's course may be in a zip.
/// A save as a saved file's bytes: the magic line, then the JSON compressed with zlib at `level`.
pub fn encode_save(save: &SaveGame, level: u8) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(save).map_err(|e| e.to_string())?;
    let mut out = SAVE_MAGIC.to_vec();
    out.extend(miniz_oxide::deflate::compress_to_vec_zlib(&json, level));
    Ok(out)
}

/// Whether bytes start as a saved game does.
pub fn is_save(data: &[u8]) -> bool {
    data.starts_with(SAVE_MAGIC)
}

/// A game read from a saved game of the original SimGolf (sg_core::sve) as a save of the port's own, at the original's
/// size. The people of the save keep their own heads; a custom portrait (head 19 and up) is the theme pack's when the same
/// person is in it, else the person's stock head. What came over and what did not goes in `imported`.
pub fn original_save(g: sg_core::sve::Game, game_dir: &Path) -> SaveGame {
    let _size = sg_core::mapsize::scoped(sg_core::mapsize::ORIGINAL);
    let pack = THEME_PACKS
        .iter()
        .find(|p| p.replace(' ', "_").eq_ignore_ascii_case(&g.theme_pack.replace(' ', "_")))
        .copied()
        .unwrap_or("Standard")
        .replace(' ', "_");
    let mut club = g.club;
    let theirs = sg_core::roster::load(game_dir, &pack);
    for (i, p) in club.roster.iter_mut().enumerate() {
        if p.head.is_some_and(|h| h >= 19) {
            match theirs.get(i).filter(|q| q.name == p.name && q.portrait.is_some()) {
                Some(q) => {
                    p.head = q.head;
                    p.portrait = q.portrait.clone();
                }
                None => p.head = None,
            }
        }
    }
    let terrain = g.land.to_terrain();
    let mut staff_tiles = TileState::new(terrain.w, terrain.h);
    let n = land::mn();
    for a in 0..n {
        for b in 0..n {
            let i = (a * n + b) as usize;
            let t = (b * terrain.w + a) as usize;
            staff_tiles.flags[t] |= g.weeds[i] & (staff::WEEDS | staff::WORKED);
            if g.land.flags[i] & land::flag::CREEK != 0 {
                staff_tiles.flags[t] |= staff::SEEDS_WEEDS;
            }
        }
    }
    let mut rng = ExeRng::from_clock(g.tick ^ g.course_id as u32);
    let noise = g.noise.unwrap_or_else(|| Noise::new(&mut rng));
    let mut bought = vec![-1; sg_core::data::properties().len()];
    if let Some(b) = bought.get_mut(g.land.slot.property) {
        *b = (g.tick >> 13) as i32;
    }
    let list = |keys: &[&'static str]| keys.iter().map(|k| tr(k)).collect::<Vec<_>>().join(", ");
    let note = trf("STR_SVE_IMPORTED_NOTE", &[("CARRIED", &list(&g.carried)), ("DROPPED", &list(&g.dropped))]);
    SaveGame {
        version: 1,
        size: sg_core::mapsize::ORIGINAL,
        // the exe's theme (0 Parkland, 1 Desert, 2 Tropical, 3 Links) as the port's theme index (`App::exe_theme`)
        theme: [0, 2, 3, 1][g.theme.min(3) as usize],
        course_name: g.course_name,
        difficulty: g.difficulty,
        terrain,
        land: Some(g.land),
        club,
        course: g.course,
        econ: g.econ,
        exe_rng: rng,
        noise,
        employees: g.employees,
        staff_tiles,
        game_tick: g.tick,
        hole_stats: Vec::new(),
        theme_pack: pack,
        course_id: Some(g.course_id & 0xffff),
        bought,
        goals: Default::default(),
        scripts: Default::default(),
        extras: Default::default(),
        shared: false,
        packs: None,
        imported: Some(note),
        deco_rng: None,
    }
}

pub fn read_save_bytes(data: &[u8], name: &str) -> Result<SaveGame, String> {
    let body = data.strip_prefix(SAVE_MAGIC).ok_or_else(|| trf("STR_ERROR_NOT_A_SAVED_GAME", &[("FILE", &name)]))?;
    let json = miniz_oxide::inflate::decompress_to_vec_zlib(body).map_err(|e| format!("{name}: {e:?}"))?;
    let s: SaveGame = serde_json::from_slice(&json).map_err(|e| format!("{name}: {e}"))?;
    // a save from a newer version, or one whose map does not match its size, is refused rather than loaded wrong
    let n = s.size;
    let tiles = (n * n) as usize;
    let fits = sg_core::mapsize::valid(n) == n
        && s.land.as_ref().is_none_or(|l| {
            l.n == n && l.ty.len() == tiles && l.height.len() == ((n + 1) * (n + 1)) as usize && s.course.ty.len() == tiles
        });
    if s.version > SAVE_VERSION || !fits {
        return Err(trf("STR_ERROR_NOT_A_SAVED_GAME", &[("FILE", &name)]));
    }
    Ok(s)
}

/// One texture-batched mesh of the course.
pub struct Batch {
    pub tex: TextureId,
    pub mesh: Mesh,
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
    /// The optional HD graphics (off by default; see hd.rs).
    pub hd: crate::hd::HdState,
    /// The Enhanced graphics effects' state (fx.rs); unused in Classic.
    pub fx: crate::fx::FxWorld,
    /// The optional Enhanced graphics' camera and motion state (off by default; see enhanced.rs).
    pub enh: crate::enhanced::EnhState,
    /// Enhanced graphics' photo mode while it is open (photo.rs); always None in Classic.
    pub photo: Option<Box<crate::photo::Photo>>,
    /// The showcase fly-over while it is under way (showcase.rs; the port's own, Classic too).
    pub showcase: Option<crate::showcase::Showcase>,
    pub batches: Vec<Batch>,
    pub path_batches: Vec<Batch>,
    /// Paths not joined to the clubhouse, drawn as mud tracks (manual p. 18).
    pub mud_batches: Vec<Batch>,
    pub wall_batches: Vec<Batch>,
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
    /// The art files of the player's copy that could not be read (Flics/..., shadows aside), and whether this run said so
    /// (main.rs: once, in a box until the list changes, and in the ticker).
    pub unreadable: Vec<String>,
    pub unreadable_said: bool,
    /// When the last of them was found (the game clock): the note waits until the art has finished loading.
    pub unreadable_at: f64,
    /// Sprites drawn from a content pack's high-resolution or person's sheet, by sprite index (packart.rs).
    pub pack_art: HashMap<usize, sg_core::hires::SheetSprite>,
    pub props: Vec<Prop>,
    pub show_props: bool,
    /// Seconds, drives animation.
    pub time: f64,
    /// The golfers and the course as they see it (the exe's golfer table and tile arrays).
    pub club: Club,
    pub course: Course,
    /// The numbers of the open holes (par set), from the golfers' hole records.
    pub hole_numbers: Vec<i32>,
    /// The terrain came from outside the hole tool (demo course, saved terrain, scripted edits): its painted tee and green
    /// pairs become holes at the next course sync.
    pub adopt_holes: bool,
    /// The Second course extra (sg_core::second): the hole panel's A/B switch is on Course B, so the hole tools and the hole
    /// reports work on it (`App::hole_b`). Not saved: a loaded game starts on Course A.
    pub hole_b_view: bool,
    /// What the plan of the hole being built was last made on (`plan_inputs`): a course sync plans the hole again only when
    /// that changed, as the exe plans it again only after its terrain refresh 0x42f7a0 clears game flag 0x40000 (0x42f929),
    /// which neither the Course Report (0x44fb30) nor the instant shot analysis (0x41d0e3..0x41d8f3) runs. Not saved: a load
    /// clears it, as the exe's load runs the refresh (`load_save_into`).
    pub plan_made_on: Option<u64>,
    /// A replay's load keeps the plan of the hole being built as saved (replay.rs), when the game it was saved from had it
    /// planned on the course as it stood.
    pub keep_plan_on_load: bool,
    /// The Top 10 table the Top10 action drew (info_ui.rs).
    pub top10_drawn: Option<Vec<sg_core::top10::Entry>>,
    skip_plan: bool,
    /// The action log and a replay under way (replay.rs).
    pub rec: crate::replay::Recorder,
    pub sim_time: f64,
    /// Golfer clips by sprite id (clip + body): body and shadow sprites.
    pub golfer_clips: HashMap<i32, (Option<usize>, Option<usize>)>,
    /// 0..3 as in the exe (the standard game's value is not known yet; 1 is a guess).
    pub difficulty: i32,
    /// The course's id in the Top 10 table (0x822c78): rand(0xffff) drawn when the difficulty is picked for a new game.
    /// None for a game saved before it was kept.
    pub course_id: Option<i32>,
    /// The year (0 = 2001) each property was bought in this game, -1 for one not bought (the land records' byte +6).
    pub bought: Vec<i32>,
    /// The exe's random number generator (land, offer) and the height noise it fills once at start-up.
    pub exe_rng: ExeRng,
    /// A second generator for things only drawn (celebrity residents, wildlife steps, water sparkle), so the club's draws do
    /// not depend on the camera or the frame rate.
    pub deco_rng: ExeRng,
    /// A generator for what only the pointer and the frame show: the port's pick of a garden item's design and the water
    /// glints' coin (the exe tosses that with its own generator, 0x822d9c, every frame).
    pub look_rng: ExeRng,
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
    /// The instant shot analysis on screen ('/' and '.'), how many of its shots are drawn so far, and the hole it analyses
    /// (0x56c790, kept for '.').
    pub analysis: Option<sg_core::analysis::Analysis>,
    pub analysis_shown: usize,
    pub analysis_hole: i32,
    /// The pro's skill dialog while open, and the shot preview while he waits for the player's aim.
    pub skill_dialog: Option<crate::pro_ui::SkillDialog>,
    pub aim: Option<sg_core::pro::AimPreview>,
    /// The pro was waiting for the player's aim at the last tick (a new shot drops an armed tool, pro_after_tick).
    pub was_aiming: bool,
    /// Aiming by touch (touch.rs, the port's own): the target a finger set, shot with the Swing touch button.
    pub touch_target: crate::touch::TouchTarget,
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
    /// The title's side screens (Load Previous Game, championship choosers, Theme Packs, credits) and the Save dialog.
    pub title: crate::files_ui::TitleScreens,
    /// Tiles the player retyped since the last tick (each may draw wildlife), and the water depth of each tile.
    pub retyped: Vec<(i32, i32)>,
    /// The clubhouse and record screens: their art, the pair screen's picks, the golfers list page, the golfer card open,
    /// the board's notice for the year-end report, the roster's scroll.
    pub art: crate::screens_ui::Art,
    pub pair_picks: Vec<usize>,
    pub golfer_page: usize,
    /// The golfers drawn in the Golfers panel's cells this frame, in cell order (the exe's 0x5a9cf0).
    pub member_cells: Vec<usize>,
    pub card: Option<usize>,
    pub card_ui: crate::screens_ui::CardUi,
    /// Take Snapshot's photo while it is being taken or held on screen.
    pub simfoto: Option<crate::screens_ui::SimFoto>,
    /// The great-shot SimFoto while it is on screen (0x407e00).
    pub great_shot: Option<crate::screens_ui::GreatShot>,
    /// The Preferences' "Show snapshots of great shots." (bit 2 of 0x5a5a00, on by default).
    pub show_snapshots: bool,
    /// The --screen greatshot test hook is waiting for the golfers' screen points.
    pub test_great_shot: bool,
    /// A character file was loaded whose portrait still has to be registered (needs the graphics context).
    pub portraits_due: bool,
    /// Customise Golfer, while open.
    pub cust: Option<crate::cust_ui::Customise>,
    pub year_notice: String,
    /// The tick the End of Year report reads (0x44cff0 reads the date of the moment it opens: the year index of the tick
    /// that ends the year, so 2000 + n after the n-th season, the year just finished).
    pub year_end_tick: u32,
    pub roster_offset: usize,
    /// The accomplishment snapshots still to take (id and map point), and the snapshots taken this session.
    pub snapshot_due: Vec<(usize, (i32, i32))>,
    pub snapshots: HashMap<usize, crate::ui::Image>,
    /// The report screens' art, the routing map's tab and selected hole.
    pub reports: crate::reports_ui::ReportArt,
    /// The dock panels' sheets and their state (panels_ui).
    pub panel_art: crate::panels_ui::PanelArt,
    pub pstate: crate::panels_ui::PanelState,
    /// The info screens' art and state (OK ticks, report, hole stats, best scores, top 10, land and world screens).
    pub info: crate::info_ui::Info,
    pub route_tab: usize,
    pub route_hole: usize,
    /// The routing map's employee list: rows scrolled off the top (eight show at a time).
    pub route_scroll: usize,
    /// The property chooser was opened from the course (F6) to move the club.
    pub world_move: bool,
    /// Draw the course only (for snapshots).
    pub no_hud: bool,
    /// Option "show golfer thoughts" (option bit 0x10).
    pub show_thoughts: bool,
    /// Option bit 0, "Display golfer names on screen" (Shift+N).
    pub show_names: bool,
    /// The dock button under the pointer and for how many frames (its tooltip waits for 11).
    pub dock_tip: (i32, u32),
    /// Frames drawn while paused (0x5a9f5c): the dock's Pause button blinks gold every fourth frame.
    pub dock_blink: u32,
    /// Floating money (0x40c890): an 8-entry ring of (amount in $100 units, map x, map y, ticks left, the clock when it goes
    /// at the latest).
    pub floats: [(i32, i32, i32, u8, f64); 8],
    pub float_next: usize,
    /// Words over map points (0x40c720, drawn by 0x40c7a0): (map x, map y, text, clock when they go), at most 8. The right
    /// click's undo preview is the one user.
    pub map_labels: Vec<(i32, i32, String, f64)>,
    /// The tile of the last right click on the course (0x4c2e24, 0x5a6368): a right click there again undoes.
    pub undo_mark: Option<(i32, i32)>,
    /// The popup menu shown, and the last ticker message (Repeat Last Message).
    pub popup: Option<crate::popup_ui::Popup>,
    /// The content packs found and loaded (docs/MODDING.md), for Preferences > Content.
    pub content: crate::content::ContentState,
    /// Preferences > Keyboard controls (keys_ui.rs): the page shown and the screen to go back to.
    pub keys_page: usize,
    pub keys_back: Screen,
    /// The Preferences boxes to go back to (prefs_ui.rs): a box, screen or search opened from Preferences or one of its
    /// boxes returns to it when done.
    pub prefs_stack: Vec<crate::popup_ui::PopupKind>,
    /// The OpenSGolf extras' state (extras.rs).
    pub xs: crate::extras::State,
    /// Quick save and load, the comfort pauses and the staff areas (comfort.rs).
    pub comfort: crate::comfort::State,
    /// The support reminder's play time and asks (support.rs; the port's own, kept in settings.json, never in saves).
    pub support: crate::support::State,
    /// The welcome boxes (welcome.rs).
    pub welcome: crate::welcome::State,
    /// The launcher, while it is up (the port's own screen before the title menu, launcher.rs).
    pub launcher: Option<crate::launcher::State>,
    /// The game folder held the original's files when the game started, before any pack was put over it (gamefiles.rs
    /// `files_report`).
    pub game_files_found: bool,
    /// The play-time count (playcount.rs; the port's own, kept in settings.json, never in saves).
    pub playcount: crate::playcount::State,
    /// The Change keys screen, the hidden interface and the save's content packs question (controls_ui.rs).
    pub ctl: crate::controls_ui::CtlState,
    /// The scenario's goals and the packs' scripts of the game in progress (scenario_ui).
    pub scen: crate::scenario_ui::ScenarioState,
    pub last_message: String,
    /// The main screen's message window (the exe's ticker).
    pub ticker: crate::message_ui::Ticker,
    /// The course name being typed (Rename Course...), while the prompt is open.
    pub rename: Option<String>,
    pub water_depth: Vec<u8>,
    /// An employee picked up to be moved: the next click on the course becomes their post (the exe's "Move this employee").
    pub moving_employee: Option<usize>,
    /// Employee clips by sprite set (0 Greeter, 1 Ranger, 2 Groundskeeper, 3 Tray Girl, 4 Golf Celebrity, 5 Marshall,
    /// 6 Lawn Technician, 7 Soda Vendor; 8 the player's pro) and state (walk, stand, action): body and shadow.
    pub staff_clips: [[(Option<usize>, Option<usize>); 3]; 9],
    pub weed_sprite: Option<usize>,
    /// The palette id each flower bed piece sprite (0x1a2..0x1b3) was last drawn in, 0 for none yet. The exe draws a bed
    /// through jgl's paletted blit (sprite vtable +0x78, 0x100098c0 in jgl.dll), which keeps the palette it is given in the
    /// sprite object and, given none, draws in the one it kept. A weedy bed's palette object (0xbb) is never loaded (its
    /// "flics\bldgs\flowers\...IckyPal" files are not on the disc), so the exe hands the blit no palette and the bed shows in
    /// the colour the same piece sprite was last drawn in: the previous bed of that piece in the tile loop, or the design
    /// strip's last icon of that shape, or before any of those the FLC's own palette.
    pub bed_palette: [u8; 18],
    pub course_name: String,
    pub screen: Screen,
    pub title_base: Image,
    pub title_un: Image,
    pub title_mo: Image,
    /// The highlight art split per title menu button (0..5) and for the logo (6).
    pub title_mo_parts: Vec<crate::ui::Image>,
    /// The swap palettes golfers are recoloured from, the palettes composed so far and the recoloured frames (sprite, frame,
    /// outfit), dropped wholesale when the cache grows large.
    pub swaps: sg_core::bodies::Swaps,
    pub outfit_pals: HashMap<sg_core::bodies::Outfit, [u8; 768]>,
    pub outfit_tex: HashMap<(usize, usize, sg_core::bodies::Outfit), miniquad::TextureId>,
    /// Classic's sprite atlas (atlas.rs): where each frame (sprite, frame) and each recoloured frame (sprite, frame,
    /// outfit) was put.
    pub sprite_atlas: HashMap<(usize, usize), (crate::atlas::STex, crate::atlas::Slot)>,
    pub outfit_atlas: HashMap<(usize, usize, sg_core::bodies::Outfit), (crate::atlas::STex, crate::atlas::Slot)>,
    /// Red silhouettes of sprite frames (sprite, frame), for a building preview that will not fit.
    pub red_tex: HashMap<(usize, usize), miniquad::TextureId>,
    /// The terrain brushes' tile pictures under the pointer (Data/parkland.pcx, desert, tropical, links: the exe's theme order).
    pub tool_tiles: [Image; 4],
    /// The rocks the frame stands along water banks (cliffs01.pcx, cut by 0x445970; see wild_ui's bank rocks).
    pub cliffs: Image,
    pub world_base: Image,
    /// The main screen's course badge and rating pills (Interface/courseinfo.pcx with its alpha sheet).
    pub hud_art: Image,
    /// The rest of the HUD's art and its face strip (hud_ui).
    pub hud: crate::hud_ui::HudArt,
    /// Shift is held (the face strip then shows the back nine only).
    pub shift_held: bool,
    /// Ctrl is held (the Change keys screen takes it with the key it waits for).
    pub ctrl_held: bool,
    /// The Select Difficulty art: the screen and the sheet of its lit items.
    pub diff_base: Image,
    pub diff_mo: Image,
    pub theme_icons: [Image; 4],
    pub report_art: Image,
    pub dock_art: Image,
    /// The Preferences' "Advisor and first-time messages" option.
    pub show_advisor: bool,
    /// Open dock panel: 0 none, 1 terrain (and Amenities), 2 buildings (and Elevation), 3 employees, 4 golfers, 5 player.
    pub panel: i32,
    /// Dock button under the mouse, -1 none.
    pub dock_hover: i32,
    /// List row under the mouse in an open panel.
    pub panel_hover: i32,
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
    /// The course editor (editor.rs; the port's own): Some while the game in hand is the editor's, never in a normal game.
    pub editor: Option<Box<crate::editor::Editor>>,
    /// Select Difficulty and the property chooser were opened by the title's "Course editor..." button.
    pub editor_start: bool,
    /// The course size (map edge in tiles, sg_core::mapsize) the next new game is dealt and played at.
    pub course_size: i32,
    pub toast: String,
    pub toast_until: f64,
    /// The toast is about the original's saved games found in the Saved Games folder (share.rs): the Load list shows it
    /// even without the port's buttons.
    pub sve_toast: bool,
    /// A message in the game's box over the screen, with an OK or a question's two options (gamefiles.rs).
    pub notice: Option<crate::gamefiles::Notice>,
    /// "Check for updates" on the title menu: the request under way (update.rs).
    pub update_check: Option<crate::net::Request>,
    /// The check was the start up one (the interface extra): it speaks only of a newer version (update.rs).
    pub update_auto: bool,
    /// The newest release the last check found, with its files (update.rs).
    pub update_release: Option<crate::update::Release>,
    /// "Update now": the download, check and install under way (autoupdate.rs).
    #[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
    pub update_job: Option<crate::autoupdate::Job>,
    /// The title screen says that the free graphics are in use and how to use the original files (once, gamefiles.rs).
    pub free_hint: bool,
    /// Camera follows the golfer.
    pub follow: bool,
    // Course editing
    pub edit: bool,
    /// 0 paint, 1 raise (shift lowers), 2 path (shift removes), 4 building, 5 undo. (The original has no wall tool: walls
    /// follow the ground, see build_walls.)
    pub tool: i32,
    /// 1 gravel, 2 paved.
    pub path_kind: i32,
    /// Building kind the building tool places (an exe kind, see OFFERED_KINDS).
    pub build_idx: usize,
    /// Tab's turn for the next building (the exe's 0x5a34f0; the object faces `turn & 3`).
    pub turn: i32,
    /// = selects raising, - selects lowering (the original's hotkeys); shift flips it.
    pub raise_sign: i32,
    pub paused: bool,
    /// Game speed (Space cycles 1, 2, 4): above 1 the exe's fast mode is on as well (golfers walk twice as far a tick).
    pub speed: u8,
    pub paint_idx: usize,
    /// The paint tool's variant (0x5a34f0, drawn when a terrain brush is picked) and the brush it was drawn for.
    pub paint_variant: Option<(usize, i32)>,
    /// The Tees tool's step under the Multiple tees extra (Tab): 0 the hole's own tee (the original's tool), 1 a forward
    /// tee, 2 a middle tee (sg_core::extras `Tee::of_step`; tees.rs).
    pub tee_step: u8,
    /// While a paint action is applied: the tee it places (replay.rs), whatever the tool in hand.
    pub tee_forced: Option<sg_core::extras::Tee>,
    /// The design the next bench, flower bed, willow, scenic bridge or landmark takes and the kind it is for (the exe's
    /// 0x56a514 and 0x5a9f60): set when the amenity is picked, then chosen from the strip of designs over the panel; it stays
    /// after a placement. None for a landmark not chosen yet.
    pub design: Option<(i32, i32)>,
    /// The design last under the pointer on that strip (0x5a9f50), which a click in the panel makes the design; -1 none.
    pub design_hover: i32,
    /// The port's extras (extras_ui): what new games start with, and the landmark strip's page shown.
    pub extras_next: sg_core::extras::Extras,
    /// The Set par extra's change waiting for its confirmation box: hole and new par (extras_ui.rs).
    pub par_change: Option<(usize, i32)>,
    /// The words the Preferences' search box opens on (prefs_search.rs).
    pub pref_query: String,
    /// Undo for building and terrain (undo.rs).
    pub undo: crate::undo::Undo,
    /// The port's measuring tape and the brush's rectangle and fill (build_tools.rs).
    pub bt: crate::build_tools::BuildTools,
    /// Seconds of play since the last autosave, and the clock at the last frame (autosave.rs).
    pub autosave_due: f64,
    pub autosave_clock: f64,
    /// `autosave_due` when the window going to the background last saved the game, since the last five-minute autosave
    /// (autosave.rs autosave_on_hide); None when there was none.
    pub autosave_hidden_at: Option<f64>,
    /// Until when a pasted course code is waited for (share.rs; 0: not waiting).
    pub paste_wait: f64,
    pub strip_page: u8,
    /// Where each golfer slot's ball was drawn last frame (the exe keeps the screen point, 0x567b34 / 0x567d94), for the
    /// sweet shot's streak.
    pub ball_trail: Vec<Option<[f32; 3]>>,
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
    // Sound
    pub mixer: Option<Arc<Mixer>>,
    pub audio: Option<crate::audio::AudioOut>,
    pub mute: bool,
    pub sound_log: bool,
    pub music_on: bool,
    pub ambience: i32,
    /// The title tune (slot 0x80) while it plays, and whether the last frame was on the title screens (None: not yet known).
    pub title_music: i32,
    pub music_screen: Option<bool>,
    /// The playing screen jingle and the screen it belongs to.
    pub jingle: Option<(i32, Screen)>,
    /// The voice each sound slot last started: the exe keeps one sound buffer per slot, so a slot never sounds twice at once.
    pub slot_voices: std::collections::HashMap<i32, i32>,
    /// The painting tune's note counter (0x5a9cdc): counts up for every fairway tile painted on screen, never reset.
    pub tune_note: i32,
    pub music: i32,
    pub music_idx: usize,
    /// Wall clock (seconds) used for toasts and the story timer.
    pub clock: f64,
    /// The exe's frames (one per 87 ms game tick) that have passed on the wall clock since the last drawn frame, and the
    /// count they are taken from. The exe drew one frame per tick, so its hover delays and blinks count these; our frames
    /// come at the display's refresh rate (60 to 144 a second), which would run them two to five times too fast.
    pub exe_steps: u32,
    pub exe_frame: u64,
    /// Trees, scenic trees and flower beds planted and still growing, by tile: the frames drawn since planting. The exe
    /// keeps this as the tile's counter 0x578804 under flag 0x4000 and advances it in its drawing (sg_core::decor::
    /// sprout_frame); the port keeps a copy for the drawing only, so the simulation's tiles stay as they were.
    pub sprouts: HashMap<(i32, i32), u32>,
    /// The game tick the sprouts were last advanced at.
    pub sprout_tick: u32,
}

/// The exe's frame: one per game tick (sg_core::flight::TICK_MS).
pub const EXE_FRAME_S: f64 = sg_core::flight::TICK_MS as f64 / 1000.0;

/// Test hook for stills of planting: SG_SPROUT=k holds every tree, scenic tree and flower bed planted in this run at its
/// growth frame k (App::sprouts).
fn sprout_hold() -> Option<u32> {
    static HOLD: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    *HOLD.get_or_init(|| std::env::var("SG_SPROUT").ok().and_then(|v| v.parse().ok()))
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
            hd: crate::hd::HdState { pack_dir: save_dir().join("HD"), ..Default::default() },
            fx: Default::default(),
            enh: Default::default(),
            photo: None,
            showcase: None,
            batches: Vec::new(),
            path_batches: Vec::new(),
            mud_batches: Vec::new(),
            wall_batches: Vec::new(),
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
            unreadable: Vec::new(),
            unreadable_said: false,
            unreadable_at: 0.0,
            pack_art: HashMap::new(),
            props: Vec::new(),
            show_props: true,
            time: 0.0,
            club: Club::default(),
            course: Course::default(),
            hole_numbers: Vec::new(),
            adopt_holes: true,
            hole_b_view: false,
            plan_made_on: None,
            keep_plan_on_load: false,
            top10_drawn: None,
            skip_plan: false,
            rec: Default::default(),
            sim_time: 0.0,
            golfer_clips: HashMap::new(),
            difficulty: 1,
            course_id: None,
            bought: Vec::new(),
            exe_rng,
            deco_rng: ExeRng::from_clock(0x5167),
            look_rng: ExeRng::from_clock(0x6c1),
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
            analysis: None,
            analysis_shown: 0,
            analysis_hole: 0,
            skill_dialog: None,
            aim: None,
            was_aiming: false,
            touch_target: Default::default(),
            cancel_until: 0,
            auto_aim: 0,
            pro_slot: -1,
            sga: None,
            prep: None,
            results: None,
            title: Default::default(),
            retyped: Vec::new(),
            show_thoughts: true,
            show_names: true,
            dock_tip: (-1, 0),
            dock_blink: 0,
            floats: [(0, 0, 0, 0, 0.0); 8],
            map_labels: Vec::new(),
            undo_mark: None,
            float_next: 0,
            popup: None,
            content: Default::default(),
            keys_page: 0,
            keys_back: Screen::Play,
            prefs_stack: Vec::new(),
            xs: Default::default(),
            comfort: Default::default(),
            support: Default::default(),
            welcome: Default::default(),
            launcher: None,
            game_files_found: false,
            playcount: Default::default(),
            ctl: Default::default(),
            scen: Default::default(),
            last_message: String::new(),
            ticker: Default::default(),
            rename: None,
            art: Default::default(),
            pair_picks: Vec::new(),
            golfer_page: 0,
            member_cells: Vec::new(),
            card: None,
            card_ui: Default::default(),
            simfoto: None,
            great_shot: None,
            show_snapshots: true,
            test_great_shot: false,
            portraits_due: false,
            cust: None,
            year_notice: String::new(),
            year_end_tick: 0,
            roster_offset: 0,
            snapshot_due: Vec::new(),
            snapshots: HashMap::new(),
            no_hud: false,
            reports: Default::default(),
            panel_art: Default::default(),
            pstate: Default::default(),
            info: Default::default(),
            route_tab: 0,
            route_hole: 1,
            route_scroll: 0,
            world_move: false,
            water_depth: Vec::new(),
            moving_employee: None,
            staff_clips: [[(None, None); 3]; 9],
            weed_sprite: None,
            bed_palette: [0; 18],
            course_name: tr("STR_DEMO_COURSE").into(),
            screen: Screen::Play,
            title_base: Image::default(),
            title_un: Image::default(),
            title_mo: Image::default(),
            title_mo_parts: Vec::new(),
            swaps: Default::default(),
            outfit_pals: HashMap::new(),
            outfit_tex: HashMap::new(),
            sprite_atlas: HashMap::new(),
            outfit_atlas: HashMap::new(),
            red_tex: HashMap::new(),
            tool_tiles: [Image::default(); 4],
            cliffs: Image::default(),
            world_base: Image::default(),
            hud_art: Image::default(),
            hud: Default::default(),
            shift_held: false,
            ctrl_held: false,
            diff_base: Image::default(),
            diff_mo: Image::default(),
            theme_icons: [Image::default(); 4],
            report_art: Image::default(),
            dock_art: Image::default(),
            show_advisor: true,
            panel: 0,
            dock_hover: -1,
            panel_hover: -1,
            hole_stats: Vec::new(),
            ui_ok: false,
            view: crate::ui::View::default(),
            hover: -1,
            theme_pack: 0,
            reset_clock: false,
            sandbox_choice: false,
            editor: None,
            editor_start: false,
            course_size: sg_core::mapsize::ORIGINAL,
            toast: String::new(),
            toast_until: 0.0,
            sve_toast: false,
            notice: None,
            update_check: None,
            update_auto: false,
            update_release: None,
            #[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
            update_job: None,
            free_hint: false,
            follow: false,
            edit: false,
            tool: 0,
            path_kind: 1,
            build_idx: 7,
            raise_sign: 1,
            turn: 0,
            paused: false,
            speed: 1,
            paint_idx: 0,
            paint_variant: None,
            tee_step: 0,
            tee_forced: None,
            design: None,
            design_hover: -1,
            extras_next: crate::extras_ui::saved(),
            par_change: None,
            pref_query: String::new(),
            undo: Default::default(),
            bt: Default::default(),
            autosave_due: 0.0,
            autosave_clock: 0.0,
            autosave_hidden_at: None,
            paste_wait: 0.0,
            strip_page: 0,
            ball_trail: Vec::new(),
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
            course_file: save_dir().join("course.sgc"),
            mixer: None,
            audio: None,
            mute: false,
            sound_log: false,
            music_on: false,
            ambience: -1,
            title_music: -1,
            music_screen: None,
            jingle: None,
            slot_voices: Default::default(),
            tune_note: 0,
            music: -1,
            music_idx: 0,
            clock: 0.0,
            exe_steps: 0,
            exe_frame: 0,
            sprouts: HashMap::new(),
            sprout_tick: 0,
        }
    }

    /// Counts the exe's frames that have passed since the last drawn frame (exe_steps), for the per-frame delays and blinks.
    pub fn count_exe_frames(&mut self) {
        let f = (self.clock / EXE_FRAME_S) as u64;
        self.exe_steps = if self.exe_frame == 0 { 0 } else { f.saturating_sub(self.exe_frame).min(1000) as u32 };
        self.exe_frame = f;
    }

    /// A file in the game folder, matched case-insensitively.
    pub fn game_path(&self, rel: &str) -> PathBuf {
        resolve(&self.game_dir, rel)
    }

    /// A message for the player: on the course it goes to the ticker as the exe's messages do (priority 1, nobody
    /// speaking); the title screens, which have no ticker, show it as a note at the bottom for 3 seconds.
    /// The art files that could not be read, said once a run (in the ticker) and in a box until the list changes; None
    /// when there is nothing (new) to say.
    pub fn take_unreadable_note(&mut self) -> Option<(String, bool)> {
        if self.unreadable_said || self.unreadable.is_empty() || self.clock - self.unreadable_at < 2.0 {
            return None;
        }
        self.unreadable_said = true;
        let file = format!("Flics/{}", self.unreadable[0]);
        let msg = match self.unreadable.len() {
            1 => trf("STR_FILES_UNREADABLE_1", &[("FILE", &file)]),
            n => trf("STR_FILES_UNREADABLE", &[("N", &n), ("FILE", &file)]),
        };
        let mut list = self.unreadable.clone();
        list.sort();
        let boxed = crate::gamefiles::take_once("unreadable_files_shown", &list.join("|"));
        Some((msg, boxed))
    }

    /// The port's own game speed one step faster or slower (normal, x2, x4), stopping at either end; says the speed.
    pub fn step_speed(&mut self, faster: bool) {
        let s = match (self.speed, faster) {
            (1, true) => 2,
            (_, true) => 4,
            (4, false) => 2,
            (_, false) => 1,
        };
        self.act(sg_core::action::Action::Speed(s));
        self.say_speed();
    }

    /// The game speed's single button (a controller's Y, the touch button): normal, x2, x4, then normal again.
    pub fn cycle_speed(&mut self) {
        let s = match self.speed {
            1 => 2,
            2 => 4,
            _ => 1,
        };
        self.act(sg_core::action::Action::Speed(s));
        self.say_speed();
    }

    fn say_speed(&mut self) {
        let name = match self.speed {
            1 => tr("STR_TOAST_SPEED_NORMAL"),
            2 => tr("STR_TOAST_SPEED_FAST"),
            _ => tr("STR_TOAST_SPEED_FASTEST"),
        };
        self.show_toast(name);
    }

    pub fn show_toast(&mut self, msg: &str) {
        self.toast = msg.to_string();
        self.toast_until = self.clock + 3.0;
        self.sve_toast = false;
        self.post_message(msg, 1, crate::message_ui::NOBODY);
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

    /// Plays one of the exe's sound slots (the file each slot holds after the exe's loader, sg_core::sounds) at a world point,
    /// as the exe's positional call does: silent off screen, panned by the screen x, louder towards the edges (the exe's own
    /// odd volume curve, 50 at the centre to 74 of 127 at the edge).
    pub fn slot_sound(&mut self, slot: i32, x: f32, z: f32) {
        self.slot_sound_after(slot, Some((x, z)), 0);
    }

    /// An interface sound: centred, never gated (the exe's position -1).
    pub fn ui_sound(&mut self, slot: i32) {
        self.slot_sound_after(slot, None, 0);
    }

    /// A screen's jingle (year end 0x7f, accomplishments 0x7e): it fades when the player leaves that screen.
    pub fn screen_jingle(&mut self, slot: i32, screen: Screen) {
        let id = self.slot_sound_after(slot, None, 0);
        self.jingle = Some((id, screen));
    }

    pub fn slot_sound_after(&mut self, slot: i32, at: Option<(f32, f32)>, delay_ms: u32) -> i32 {
        self.slot_play(slot, at, delay_ms, false)
    }

    /// The sound of a painted tile (0x420660, right after the tile's growth byte is set): by the terrain's class byte
    /// (record +0x26 of the table at 0x4c1a40) tee and green play slot 0x105 (Test 2), fairway and firm fairway 0x101
    /// (Test 7) as the next note of the painting tune, the rough class 0x100 (Test 9) or 0x102 (Place Rocks 1) for rocks,
    /// the sand class 0x104 (Test 10), water 0x103 (Place Water); trees play their own sound when they start growing and
    /// buildings nothing. EXACT. The port's own brushes past the exe's 23 terrain ids keep their earlier sounds.
    pub fn paint_sound(&mut self, ty: u8, x: i32, y: i32) {
        let at = Some(self.units_to_world(x * 1024 + 512, y * 1024 + 512));
        let slot = match ty {
            0 | 1 => 0x105,
            2 | 3 => {
                self.slot_play(0x101, at, 0, true);
                return;
            }
            12 => 0x102,
            4 | 5 | 6 | 10 | 11 | 18 | 19 => 0x100,
            7..=9 => 0x104,
            17 => 0x103,
            13..=16 | 20..=22 => return,
            31 | 32 => 0x100,
            34 | 35 => 0x104,
            33 => {
                self.snd("Effects/Flower Bed.wav", 0.7, false);
                return;
            }
            _ => 0x100,
        };
        self.slot_play(slot, at, 0, false);
    }

    /// The exe's play of a slot (0x40c500 for a world point, else a centred interface play). A positional play sets a
    /// random pitch of -299..300 cents (`300 - rand(600)` from the generator at 0x822d9c), or with `tune` the next note
    /// of the painting tune: a counter that only ever counts up (0x5a9cdc) picks one of 36 notes (the table at 0x4c2fa4,
    /// "Jesu, Joy of Man's Desiring"), `(note - 5) * 100` cents, at volume sy / 12 + 50. EXACT.
    fn slot_play(&mut self, slot: i32, at: Option<(f32, f32)>, delay_ms: u32, tune: bool) -> i32 {
        const TUNE: [i8; 36] =
            [0, 2, 4, 7, 5, 5, 9, 7, 7, 12, 11, 12, 7, 4, 0, 2, 4, 5, 7, 9, 7, 5, 4, 2, 4, 0, -1, 0, 2, -5, -1, 2, 5, 4, 2, 4];
        let Some(file) = sg_core::sounds::slot_file(slot) else { return -1 };
        let (sx, sy) = match at {
            Some((x, z)) => match self.screen_of_world(x, z) {
                Some((vx, vy)) if (0.0..800.0).contains(&vx) && (0.0..=500.0).contains(&vy) => (vx, vy),
                _ => return -1,
            },
            None => (400.0, 300.0),
        };
        let mut vol = (((sx - 400.0).abs() as i32 >> 4) + 50) as f32 / 127.0 * 1.3;
        let pan = (sx * 127.0 / 800.0 - 64.0) / 64.0;
        let cents = if tune {
            let note = TUNE[(self.tune_note % 36) as usize] as i32;
            self.tune_note = self.tune_note.wrapping_add(1) & 0x7fff_ffff;
            vol = ((sy as i32 / 12) + 50) as f32 / 127.0 * 1.3;
            (note - 5) * 100
        } else if at.is_some() {
            300 - self.look_rng.below(600)
        } else {
            0
        };
        let (Some(m), false) = (self.mixer.clone(), self.mute) else { return -1 };
        // One buffer per slot (0x4481b0 sets up and plays the slot's own sound object, 0x484940): a slot asked for again
        // while its clip still sounds (or waits for its delayed start) keeps playing it rather than starting a second copy
        // over it, as DirectSound's Play does on a buffer that is already playing; the pitch, volume and pan set before
        // the play still change the sounding buffer. Two pros' applause, a skill rise and a Top 18 hole in one tick, or a
        // burst of refused clicks, were otherwise heard as overlapping copies.
        if let Some(&old) = self.slot_voices.get(&slot) {
            if m.playing(old) {
                if tune {
                    // the painting tune restarts its note so each painted tile is heard as one note of the tune
                    // (DERIVED: the sound object's play is in sound.dll, not read)
                    m.stop(old);
                } else {
                    m.retune(old, vol, pan, cents);
                    return old;
                }
            }
        }
        let id = m.play_pitched(file, vol, pan, delay_ms, cents, false);
        if id >= 0 {
            self.slot_voices.insert(slot, id);
        }
        if self.sound_log {
            println!("  sound: {file}{}", if id < 0 { "  (missing)" } else { "" });
        }
        id
    }

    /// Music and ambience by screen, as the exe's main loop has them: the title tune (Music Idea, slot 0x80) plays once when
    /// the title menu opens and fades over a second when a game starts; the course ambience (slot 0x2d) loops during a game
    /// and fades when the player goes back to the title. There is no music during play.
    pub fn screen_audio(&mut self) {
        if let Some((id, sc)) = self.jingle {
            if sc != self.screen {
                if let Some(m) = &self.mixer {
                    m.fade_out(id, 1000);
                }
                self.jingle = None;
            }
        }
        let title = self.on_title();
        if self.music_screen == Some(title) {
            return;
        }
        self.music_screen = Some(title);
        let Some(m) = self.mixer.clone() else { return };
        if self.mute {
            return;
        }
        if title {
            if self.ambience >= 0 {
                m.fade_out(self.ambience, 1000);
                self.ambience = -1;
            }
            if self.screen == Screen::Menu && !m.playing(self.title_music) {
                self.title_music = sg_core::sounds::slot_file(0x80).map(|f| m.play(f, 0.8, false)).unwrap_or(-1);
            }
        } else {
            if self.title_music >= 0 {
                m.fade_out(self.title_music, 1000);
                self.title_music = -1;
            }
            if self.ambience < 0 {
                self.start_ambience();
            }
        }
    }

    /// The title menu and the screens it leads to before a game (the title tune's screens in `screen_audio`), as against a
    /// game's screens: the course, its reports, menus and boxes, and Load Previous Game opened from it.
    pub fn on_title(&self) -> bool {
        matches!(
            self.screen,
            Screen::Menu | Screen::Scenarios | Screen::Difficulty | Screen::Property | Screen::Themes | Screen::Credits
        ) || (self.screen == Screen::Files && !self.title.over_game())
            // the support box over the title menu (support.rs)
            || self.popup_over_title()
    }

    pub fn start_ambience(&mut self) {
        if let (Some(m), true) = (&self.mixer, self.ambience >= 0) {
            m.stop(self.ambience);
        }
        // slot 0x2d at the exe's full volume (100 of 127)
        self.ambience = self.snd("GolfAmbience122.wav", 0.6, true);
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
        if crate::packart::active() {
            if let Some(t) = crate::packart::terrain_texture(g, path) {
                self.textures.insert(path.to_path_buf(), Some(t));
                return Some(t);
            }
        }
        if crate::hd::active() {
            if let Some(t) = crate::hd::texture(g, path) {
                self.textures.insert(path.to_path_buf(), Some(t));
                return Some(t);
            }
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

    fn upload(g: &mut Gfx, map: BTreeMap<usize, (TextureId, Vec<Vert>)>) -> Vec<Batch> {
        map.into_values().map(|(tex, v)| Batch { tex, mesh: g.mesh(&v) }).collect()
    }

    fn clear_batches(&mut self, g: &mut Gfx) {
        for b in
            self.batches.drain(..).chain(self.path_batches.drain(..)).chain(self.mud_batches.drain(..)).chain(self.wall_batches.drain(..))
        {
            g.delete_mesh(b.mesh);
        }
    }

    /// The overlays on the ground (paths, mud, retaining walls) rebuilt without the ground itself: the 3DS frontend draws the
    /// ground with its own renderer and needs only these after an edit.
    pub fn rebuild_overlays(&mut self, g: &mut Gfx) {
        for b in self.path_batches.drain(..).chain(self.mud_batches.drain(..)).chain(self.wall_batches.drain(..)) {
            g.delete_mesh(b.mesh);
        }
        self.sync_tile_corners();
        self.build_paths(g);
        self.build_walls(g);
        self.build_edge_walls(g);
    }

    pub fn rebuild_batches(&mut self, g: &mut Gfx) {
        self.clear_batches(g);
        self.terrain.desert = self.theme == 2; // the original swaps shallow water for its desert variant in this theme
        self.terrain.sand_phase = ((self.rot / 90.0).round() as i32).rem_euclid(4);
        // the free base set draws all its water in one look with a shore against land (sg_core::terrain::Terrain::water_one)
        self.terrain.water_one = self.content.loaded.first().is_some_and(|b| b == sg_core::packs::FREE);
        // Classic shades the ground with Terrain.dll's normals, smooth across tile edges (issue #112); Enhanced graphics
        // keeps the per-tile normals its look was made with (sg_core::terrain::vertex_normals)
        self.terrain.own_tile_normals = crate::enhanced::settings().enabled || crate::fx::active();
        self.sync_tile_corners();
        // the 3DS draws the ground with its own chunked meshes from its own texture cache (simgolf-3ds main.rs): the whole-
        // course batches, and every ground texture decoded and uploaded for them, would be thrown away (most of the time a
        // new or loaded game took there)
        if cfg!(target_os = "horizon") {
            self.build_paths(g);
            self.build_walls(g);
            self.build_edge_walls(g);
            return;
        }
        let mut map: BTreeMap<usize, (TextureId, Vec<Vert>)> = BTreeMap::new();
        let mut tris = Vec::new();
        let mut order: HashMap<Option<TextureId>, usize> = HashMap::new();
        // Enhanced graphics only (fx.rs): the water goes into batches of its own for the water shader
        let fx_water = crate::fx::water_on();
        let mut water: BTreeMap<usize, (TextureId, Vec<Vert>)> = BTreeMap::new();
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
                    let next = order.len();
                    let key = *order.entry(tex).or_insert(next);
                    let m = if fx_water && crate::fx::is_water_tex(t.tex_type) { &mut water } else { &mut map };
                    let e = m.entry(key).or_insert_with(|| (tex.unwrap_or(g.white), Vec::new()));
                    e.1.extend(t.v.iter().map(Self::to_vert));
                }
            }
        }
        self.batches = Self::upload(g, map);
        if crate::fx::active() || self.fx.has_water() {
            let water = Self::upload(g, water);
            self.fx_terrain_built(g, water);
        }
        self.build_paths(g);
        self.build_walls(g);
        self.build_edge_walls(g);
    }

    /// Path overlay, as Terrain.dll draws a path tile (0x100108f0): an arm toward each neighbour with a path, a third of the
    /// tile wide from its centre to the edge, cut from the cross shaped `Path.tga` (texture 0.33..0.67 across, the third at
    /// the arm's end along), then the centre's four triangles as `sg_core::terrain::path_centre` picks them (`PathInside.tga`
    /// between two arms, `PathCurve.tga` rounding the sides without one). A path joined to the clubhouse uses the `X` art,
    /// one that isn't the plain art (the tile's +0x209 byte, which layPath and updatePath set from the exe's flag 0x40,
    /// wrapper 0x44a410; the texture slot is 0x21 with it, 0x20 without).
    fn build_paths(&mut self, g: &mut Gfx) {
        // the Path styles extra (paths_lamps.rs) draws both arts in its material: there an unjoined path keeps its muddy tint
        let styled = self.club.extras.path_material() != 0;
        let t = &self.terrain;
        if t.path_kind.is_empty() {
            return;
        }
        // joined to the clubhouse: the exe's tile flag 0x40 from the clubhouse flood, which the terrain renderer gets with
        // each path (no land yet: every path counts as joined)
        let connected: Option<Vec<bool>> = self.land.as_ref().map(|l| {
            let mut l = l.clone();
            l.sync_from_terrain(t);
            l.joined_tiles()
        });
        let joined_at =
            |tx: i32, ty: i32| connected.as_ref().is_none_or(|j| !sg_core::course::inside(tx, ty) || j[sg_core::course::idx(tx, ty)]);
        let (ox, oz) = (-t.w as f32 * TILE_SIZE * 0.5, -t.h as f32 * TILE_SIZE * 0.5);
        // (rank, joined, file, triangles): the arms go down before the centres, as in the DLL's tile draw
        let mut jobs: Vec<(u8, bool, &'static str, Vec<Vert>)> = Vec::new();
        for ty in 0..t.h {
            for tx in 0..t.w {
                let kind = t.path_at(tx, ty);
                // nothing is drawn out of bounds: a property's paths beyond the purchased land (round a landmark the
                // land generator placed there) show once the tract is bought
                if kind == 0 || t.type_at(tx, ty) == 20 {
                    continue;
                }
                // Terrain.dll keeps a draw flag beside the path flag (Tile+0x20a beside +0x208) and clears it on tees,
                // greens, fairways, types 7 and 9 and buildings (setPath 0x10013400, setType 0x10014020 via 0x1000d480,
                // 0x1000d540, 0x10015230); the tile's path draw (0x100108f0) returns at once without it, so the ground
                // shows. Neighbours still read the path flag, so their arms run up to the tile's edge.
                // water too: the bridge on it carries the path (Terrain.dll's type set, 0x1000d480, clears a water
                // tile's path drawing; issue #53)
                if matches!(t.type_at(tx, ty), 0 | 1 | 2 | 3 | 7 | 9 | 17 | 22) {
                    continue;
                }
                let n = t.path_at(tx, ty - 1) != 0;
                let s = t.path_at(tx, ty + 1) != 0;
                let w = t.path_at(tx - 1, ty) != 0;
                let e = t.path_at(tx + 1, ty) != 0;
                let joined = joined_at(tx, ty);
                let [arm_file, inside_file, curve_file] = if joined {
                    ["PathX.tga", "PathInsideX.tga", "PathCurveX.tga"]
                } else {
                    ["Path.tga", "PathInside.tga", "PathCurve.tga"]
                };
                let vert = |wx: f32, wz: f32, u: f32, v: f32| {
                    let y = t.height_at(wx, wz) + 1.5;
                    let e = 12.0;
                    let nx = t.height_at(wx - e, wz) - t.height_at(wx + e, wz);
                    let nz = t.height_at(wx, wz - e) - t.height_at(wx, wz + e);
                    let ny = 2.0 * e;
                    let l = (nx * nx + ny * ny + nz * nz).sqrt();
                    let mut vv = Vert::new(wx, y, wz, u, v);
                    vv.normal = [nx / l, ny / l, nz / l];
                    if styled && !joined {
                        vv.color = [0.45, 0.32, 0.22, 1.0];
                    }
                    vv
                };
                // a point of the tile (u, v in 0..1, v = 0 the north edge) with its texture coordinates
                let q = |u: f32, v: f32, tu: f32, tv: f32| vert(ox + (tx as f32 + u) * TILE_SIZE, oz + (ty as f32 + v) * TILE_SIZE, tu, tv);
                let (a, b) = (1.0f32 / 3.0, 2.0f32 / 3.0);
                // arms: (u0, v0, u1, v1) on the tile; the texture's third at that end along the arm, the same across
                // (and whether it runs along v, and whether it is on the far side, south or east)
                let arms = [
                    (n, [a, 0.0, b, 0.5], true, false),
                    (s, [a, 0.5, b, 1.0], true, true),
                    (w, [0.0, a, 0.5, b], false, false),
                    (e, [0.5, a, 1.0, b], false, true),
                ];
                for (on, r, along_v, far) in arms {
                    if !on {
                        continue;
                    }
                    // the half tile from the centre to the edge takes the texture's outer third: 0.5 -> 1/3 or 2/3, edge -> 0 or 1
                    let tex = |c: f32| if far { a + c * b } else { c * b };
                    let mut v = Vec::new();
                    const D: i32 = 2;
                    for j in 0..D {
                        for i in 0..D {
                            let u0 = r[0] + (r[2] - r[0]) * i as f32 / D as f32;
                            let u1 = r[0] + (r[2] - r[0]) * (i + 1) as f32 / D as f32;
                            let v0 = r[1] + (r[3] - r[1]) * j as f32 / D as f32;
                            let v1 = r[1] + (r[3] - r[1]) * (j + 1) as f32 / D as f32;
                            let p = |u: f32, vv: f32| {
                                let (tu, tv) = if along_v { (u, tex(vv)) } else { (tex(u), vv) };
                                q(u, vv, tu, tv)
                            };
                            let (q00, q10, q11, q01) = (p(u0, v0), p(u1, v0), p(u1, v1), p(u0, v1));
                            v.extend_from_slice(&[q00, q10, q11, q00, q11, q01]);
                        }
                    }
                    jobs.push((0, joined, arm_file, v));
                }
                // the centre's triangles (NW, NE, SW, SE): two edge midpoints and the centre, the same part of the texture,
                // split once more so they follow the ground
                let centre = sg_core::terrain::path_centre(sg_core::terrain::path_mask(n, s, e, w));
                let tris = [[(0.0, 0.5), (0.5, 0.0)], [(0.5, 0.0), (1.0, 0.5)], [(0.0, 0.5), (0.5, 1.0)], [(0.5, 1.0), (1.0, 0.5)]];
                for (piece, [p1, p2]) in centre.into_iter().zip(tris) {
                    let file = match piece {
                        2 => inside_file,
                        3 => curve_file,
                        _ => continue,
                    };
                    let c = (0.5f32, 0.5f32);
                    let mid = |p: (f32, f32), r: (f32, f32)| ((p.0 + r.0) * 0.5, (p.1 + r.1) * 0.5);
                    let (m1, m2, m3) = (mid(c, p1), mid(p1, p2), mid(p2, c));
                    let p = |pt: (f32, f32)| q(pt.0, pt.1, pt.0, pt.1);
                    let mut v = Vec::new();
                    for tri in [[c, m1, m3], [m1, p1, m2], [m3, m2, p2], [m1, m2, m3]] {
                        v.extend(tri.map(p));
                    }
                    jobs.push((1, joined, file, v));
                }
            }
        }
        jobs.sort_by_key(|j| j.0);
        let mut paths: BTreeMap<usize, (TextureId, Vec<Vert>)> = BTreeMap::new();
        let mut mud: BTreeMap<usize, (TextureId, Vec<Vert>)> = BTreeMap::new();
        let mut order: HashMap<&'static str, usize> = HashMap::new();
        for (_, joined, file, v) in jobs {
            // (the Path styles and lamps extra's materials: paths_lamps.rs)
            let Some(tex) = self.path_texture(g, file) else { continue };
            let next = order.len();
            let key = *order.entry(file).or_insert(next);
            let m = if joined { &mut paths } else { &mut mud };
            m.entry(key).or_insert_with(|| (tex, Vec::new())).1.extend(v);
        }
        self.path_batches = Self::upload(g, paths);
        self.mud_batches = Self::upload(g, mud);
    }

    /// Each tile's own corners, as golf.exe hands them to Terrain.dll (0x449540): every tile but water (type 17, left at sea
    /// level) has each corner raised to the course's corner as the ground uses it (0x40bfe0: tees at their highest corner,
    /// pot bunkers and water at their lowest, buildings at their level, wetlands at sea level), so flattened tiles step away
    /// from their neighbours. Without land (the demo course) the tiles share their corners.
    fn sync_tile_corners(&mut self) {
        self.terrain.tile_corner.clear();
        if self.land.is_none() || self.terrain.w != sg_core::course::n() || self.terrain.h != sg_core::course::n() {
            return;
        }
        self.sync_course();
        let c = &self.course;
        let level = |a: i32, b: i32, k: i32| (c.corner(a, b, k) - sg_core::terrain::EXE_SEA).max(0) as i8;
        let mut out = Vec::with_capacity(self.terrain.ty.len());
        for b in 0..sg_core::course::n() {
            for a in 0..sg_core::course::n() {
                out.push(if c.ty[sg_core::course::idx(a, b)] == sg_core::course::t::WATER {
                    // The original's load path (0x449540) leaves water at sea level and only lifts it when it is edited,
                    // so a stream or lake above sea level shows black slits and triangles along its banks and at its
                    // waterfalls, where the ground's walls were made for the water's real level (course::refresh). "Fix
                    // original bugs" (docs/FIXES.md 1) gives water its level: flat at its lowest corner, as the walls have it.
                    if crate::enhanced::fix_bugs() {
                        [level(a, b, 7); 4]
                    } else {
                        [0; 4]
                    }
                } else {
                    // corners 7 (-x -y), 1 (+x -y), 5 (-x +y), 3 (+x +y)
                    [level(a, b, 7), level(a, b, 1), level(a, b, 5), level(a, b, 3)]
                });
            }
        }
        self.terrain.tile_corner = out;
    }

    /// Retaining walls (Terrain.dll 0x1000f7f0, drawn with every tile). golf.exe sets them where the ground steps (0x449540): a
    /// tile gets a wall on each side whose neighbour stands higher there (the course's wall bits, 0x42f530: the tile's two
    /// corners on that side, flattened as its type flattens them, against the neighbour's), with the style of the tile's type
    /// (terrain table +0x28: 1 for water, 2 for every other type). The DLL draws a side whose flag is set and whose neighbour
    /// exists as four triangles between the tile's three vertices on that side (bottom, t 0) and the neighbour's three on the
    /// same side (top, t 1), u 0, 0.5, 1 along it: the whole picture stretched over the step, whatever its height. Style 1
    /// takes `CliffTest.bmp` (texture 35), any other `RetainWallA.bmp` (34). The normal is the cliff's for that side and the
    /// view's parity (0x10063c40). The face looks towards the lower tile and back faces are culled (GL_CULL_FACE, 0x100034a3),
    /// so only the sides facing the camera show: the port keeps those for the current view.
    fn build_walls(&mut self, g: &mut Gfx) {
        self.terrain.wall_mask = vec![0; self.terrain.ty.len()];
        if self.land.is_none() {
            return;
        }
        self.sync_course();
        use sg_core::course;
        for ty in 0..self.terrain.h {
            for tx in 0..self.terrain.w {
                if course::inside(tx, ty) {
                    let w = self.course.walls[course::idx(tx, ty)];
                    let i = self.terrain.tile_index(tx, ty);
                    self.terrain.wall_mask[i] = (0..4).fold(0, |m, d| m | (((w >> (2 * d)) & 1) << d));
                }
            }
        }
        let (Some(ground), Some(water)) = (self.theme_texture(g, "RetainWallA.bmp"), self.theme_texture(g, "CliffTest.bmp")) else {
            return;
        };
        let t = &self.terrain;
        let view = ((self.rot / 90.0).round() as i32).rem_euclid(4);
        let odd = (view & 1) as usize;
        let (ox, oz) = (-t.w as f32 * TILE_SIZE * 0.5, -t.h as f32 * TILE_SIZE * 0.5);
        let (mut on_ground, mut on_water) = (Vec::new(), Vec::new());
        // per port side (0 north = -y, 1 east, 2 south, 3 west): the views that see its face, the cliff side whose normal it
        // takes (EDGE_NORMALS: north, south, west, east), and its three vertices in the DLL's order as (row, column) of the
        // tile's 3 x 3 patch (the neighbour's are the facing ones across the side)
        type Side = ([i32; 2], usize, [(usize, usize); 3]);
        const SIDES: [Side; 4] = [
            ([0, 3], 0, [(0, 0), (0, 1), (0, 2)]),
            ([0, 1], 3, [(0, 2), (1, 2), (2, 2)]),
            ([1, 2], 1, [(2, 2), (2, 1), (2, 0)]),
            ([2, 3], 2, [(2, 0), (1, 0), (0, 0)]),
        ];
        for ty in 0..t.h {
            for tx in 0..t.w {
                for (dir, (views, normal, verts)) in SIDES.iter().enumerate() {
                    let d = 2 * dir;
                    let (na, nb) = (tx + sg_core::geom::DX[d], ty + sg_core::geom::DY[d]);
                    if !t.wall_at(tx, ty, dir as i32) || !t.inside(na, nb) || !views.contains(&view) {
                        continue;
                    }
                    let (own, theirs) = (t.patch_heights(tx, ty), t.patch_heights(na, nb));
                    let n = EDGE_NORMALS[*normal][odd];
                    // the neighbour's vertex on the same spot: across a north or south side the other row, else the other column
                    let facing = |(r, c): (usize, usize)| if dir % 2 == 0 { (2 - r, c) } else { (r, 2 - c) };
                    let vtx = |(r, c): (usize, usize), y: f32, u: f32, v: f32| {
                        let x = ox + tx as f32 * TILE_SIZE + c as f32 * TILE_SIZE * 0.5;
                        let z = oz + ty as f32 * TILE_SIZE + r as f32 * TILE_SIZE * 0.5;
                        let mut q = Vert::new(x, y, z, u, v);
                        q.normal = n;
                        q
                    };
                    // the port's pictures are top-down, so the DLL's t 0 (the picture's bottom row) is v 1
                    let p: Vec<Vert> = verts.iter().enumerate().map(|(i, &(r, c))| vtx((r, c), own[r][c], i as f32 * 0.5, 1.0)).collect();
                    let q: Vec<Vert> = verts
                        .iter()
                        .enumerate()
                        .map(|(i, &rc)| {
                            let (fr, fc) = facing(rc);
                            vtx(rc, theirs[fr][fc], i as f32 * 0.5, 0.0)
                        })
                        .collect();
                    let out = if t.type_at(tx, ty) == 17 { &mut on_water } else { &mut on_ground };
                    out.extend_from_slice(&[q[0], p[0], p[1], q[0], p[1], q[1], q[1], p[1], p[2], q[1], p[2], q[2]]);
                }
            }
        }
        for (tex, batch) in [(ground, on_ground), (water, on_water)] {
            if !batch.is_empty() {
                self.wall_batches.push(Batch { tex, mesh: g.mesh(&batch) });
            }
        }
    }

    /// The cliff round the property (Terrain.dll 0x1000ea30, every frame for every tile that is not out of bounds): on each
    /// side whose neighbour tile exists and is out of bounds (type 20), a face from the side's two end vertices straight down
    /// to height -75, two triangles textured once with the theme's `strata.bmp` (u 0 to 1 along the side, its grass cap at
    /// the top). Only the sides facing the camera are drawn: the -y side ("N" in the DLL) in views 1 and 2, +y in views 0
    /// and 3, -x in views 0 and 1, +x in views 2 and 3, the view being the camera's quarter turns. Each side's normal comes
    /// from a table at 0x10063c40, one of two by the view's parity, and is not unit length (OpenGL does not normalise it).
    /// The map's own border has no neighbour, so no cliff. Built for the current view: a quarter turn rebuilds the batches.
    fn build_edge_walls(&mut self, g: &mut Gfx) {
        let Some(tex) = self.theme_texture(g, "strata.bmp") else { return };
        let t = &self.terrain;
        let view = ((self.rot / 90.0).round() as i32).rem_euclid(4);
        let odd = (view & 1) as usize;
        // (neighbour step, the views that draw it, the side's normals for even and odd views)
        type Side = ((i32, i32), [i32; 2], [[f32; 3]; 2]);
        const SIDES: [Side; 4] = [
            ((0, -1), [1, 2], EDGE_NORMALS[0]),
            ((0, 1), [0, 3], EDGE_NORMALS[1]),
            ((-1, 0), [0, 1], EDGE_NORMALS[2]),
            ((1, 0), [2, 3], EDGE_NORMALS[3]),
        ];
        const BOTTOM: f32 = -75.0;
        let (ox, oz) = (-t.w as f32 * TILE_SIZE * 0.5, -t.h as f32 * TILE_SIZE * 0.5);
        let mut batch = Vec::new();
        for ((dx, dy), views, normals) in SIDES {
            if !views.contains(&view) {
                continue;
            }
            let n = normals[odd];
            let vn = |x: f32, y: f32, z: f32, u: f32, v: f32| {
                let mut q = Vert::new(x, y, z, u, v);
                q.normal = n;
                q
            };
            for ty in 0..t.h {
                for tx in 0..t.w {
                    let (nx, ny) = (tx + dx, ty + dy);
                    if t.type_at(tx, ty) == 20 || !(0..t.w).contains(&nx) || !(0..t.h).contains(&ny) || t.type_at(nx, ny) != 20 {
                        continue;
                    }
                    let (x, z) = (ox + tx as f32 * TILE_SIZE, oz + ty as f32 * TILE_SIZE);
                    let ((x0, z0), (x1, z1)) = match (dx, dy) {
                        (0, -1) => ((x, z), (x + TILE_SIZE, z)),
                        (0, _) => ((x, z + TILE_SIZE), (x + TILE_SIZE, z + TILE_SIZE)),
                        (-1, _) => ((x, z), (x, z + TILE_SIZE)),
                        _ => ((x + TILE_SIZE, z), (x + TILE_SIZE, z + TILE_SIZE)),
                    };
                    // the side's end vertices are this tile's own corners
                    let l = t.tile_levels(tx, ty).map(|v| v as f32 * sg_core::terrain::HEIGHT_STEP);
                    let (y0, y1) = match (dx, dy) {
                        (0, -1) => (l[0], l[1]),
                        (0, _) => (l[2], l[3]),
                        (-1, _) => (l[0], l[2]),
                        _ => (l[1], l[3]),
                    };
                    // the bitmaps are bottom-up in OpenGL, so v 1 is the picture's top row (the port's images are top-down)
                    let top0 = vn(x0, y0, z0, 0.0, 0.0);
                    let bot0 = vn(x0, BOTTOM, z0, 0.0, 1.0);
                    let bot1 = vn(x1, BOTTOM, z1, 1.0, 1.0);
                    let top1 = vn(x1, y1, z1, 1.0, 0.0);
                    batch.extend_from_slice(&[top0, bot0, bot1, top0, bot1, top1]);
                }
            }
        }
        if !batch.is_empty() {
            self.wall_batches.push(Batch { tex, mesh: g.mesh(&batch) });
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
        // decoded ahead on another thread (the 3DS: a course's sprites and the golfer bodies, sg_core::preload), else now
        let loaded = sg_core::preload::take(&path, shadow, pal_path.as_deref());
        let r = match loaded.unwrap_or_else(|| load_sprite(&path, shadow, pal_path.as_deref())) {
            Ok(s) => {
                let s = self.pack_sheet(&path, pal_path.as_deref(), s);
                let n = s.frames.len();
                if crate::hd::active() && !shadow {
                    if let Some(h) = crate::hd::sprite(&path, pal_path.as_deref(), &s) {
                        self.hd.sprites.insert(self.sprites.len(), h);
                    }
                }
                self.sprites.push(GlSprite { s, tex: vec![None; n] });
                Some(self.sprites.len() - 1)
            }
            Err(e) => {
                if !shadow {
                    // (many objects simply have no shadow file, so those are not said)
                    crate::bugreport::warn(&format!("sprite: {e}"));
                    // a whole copy reads every one: a file it cannot is missing or damaged (the free set's art is the
                    // game's own and is not counted)
                    if !self.on_free_set() && !self.unreadable.iter().any(|f| f == rel) {
                        self.unreadable.push(rel.to_string());
                        self.unreadable_at = self.clock;
                    }
                }
                None
            }
        };
        self.sprite_index.insert(key, r);
        r
    }

    /// A decoration sprite of the exe (id and palette id) for the loaded theme, body and shadow.
    pub(crate) fn decor_sprite(&mut self, id: u16, pal: u8) -> (Option<usize>, Option<usize>) {
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
        for a in 0..land::mn() {
            for b in 0..land::mn() {
                let i = (a * land::mn() + b) as usize;
                if !(13..=15).contains(&ty[i]) {
                    continue;
                }
                // a weed on the tile shows the tree grown (the weed branch 0x41404e drops the counter before the tree)
                let grown = !self.sprouting(a, b) || self.staff_weed(a, b);
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
            let frame = match self.sprouts.get(&(a, b)) {
                Some(&c) if !grown => sg_core::decor::sprout_frame(c, n, true).0,
                _ => d.frame.unwrap_or(n - 1).min(n - 1),
            };
            self.props.push(Prop {
                tree: true,
                x: cx + d.da * TILE_SIZE,
                z: cz + d.db * TILE_SIZE,
                body,
                shadow,
                frame,
                facing: d.view,
                sprout: if grown { None } else { Some((a, b, true)) },
                ..Default::default()
            });
        }
    }

    /// Pushes a decoration sprite as a prop on tile (a, b).
    pub(crate) fn push_decor(&mut self, a: i32, b: i32, d: sg_core::decor::Draw, object: bool, decor: bool) {
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

    /// The pictures of the holes' tee tiles (sg_core::decor::tee_arrow): the exe rewrites a tee's variant byte as it draws the
    /// tile each frame (0x4120ae) and the terrain is redrawn when one changes (opening a hole, turning the view). The port keeps
    /// the picture in the renderer's terrain only, so the land and its saves are as before.
    fn update_tee_arrows(&mut self) {
        // the arrows are kept in the tiles' variant bytes, which the game reads: logged when they change (replay.rs), and a
        // replay's are its log's
        if self.rec.replaying() {
            return;
        }
        let camera = 2 * ((self.rot / 90.0).round() as i32).rem_euclid(4);
        if self.tee_arrows_change(camera) {
            self.log_tee_arrows(camera);
            self.tee_arrows(camera);
        }
    }

    /// Whether the tees' arrows for the view turned `camera` differ from those the tiles hold.
    fn tee_arrows_change(&self, camera: i32) -> bool {
        self.tee_arrow_tiles(camera).iter().any(|&(o, v)| self.terrain.variation[o] != v)
    }

    /// The open holes' tee tiles (terrain indices) and the arrow each takes for the view turned `camera`.
    fn tee_arrow_tiles(&self, camera: i32) -> Vec<(usize, u8)> {
        use sg_core::course::{f, idx, inside, t};
        let c = &self.course;
        let mut out = Vec::new();
        for a in 0..self.terrain.w {
            for b in 0..self.terrain.h {
                if !inside(a, b) {
                    continue;
                }
                let i = idx(a, b);
                let h = (c.flags[i] & f::LOW) as usize;
                if c.ty[i] != t::TEE || h == 0 || h >= self.club.holes.len() {
                    continue;
                }
                let o = self.terrain.tile_index(a, b);
                if self.terrain.ty[o] != sg_core::terrain::TT_TEE {
                    continue;
                }
                // (the record of the course the tee belongs to: sg_core::second)
                let Some(r) = self.club.course_holes(self.club.tile_course(h, a, b)).get(h) else { continue };
                // a forward or middle tee (the Multiple tees extra, tees.rs) points toward its hole's cup
                let facing = r.extra_tee_facing((a, b)).unwrap_or(r.tee_facing);
                out.push((o, sg_core::decor::tee_arrow(r.par, facing, r.flags, camera)));
            }
        }
        out
    }

    /// The TeeArrows action (replay.rs): the open holes' tees take their arrows for the view turned `camera`.
    pub(crate) fn tee_arrows(&mut self, camera: i32) {
        let mut changed = false;
        for (o, v) in self.tee_arrow_tiles(camera) {
            if self.terrain.variation[o] != v {
                self.terrain.variation[o] = v;
                changed = true;
            }
        }
        self.dirty |= changed;
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
        self.push_extra_tee_markers();
        self.push_holemarks();
    }

    /// The tile's growth counter is running (flag 0x4000).
    fn tile_growing(&self, a: i32, b: i32) -> bool {
        let t = &self.staff_tiles;
        a < t.w && b < t.h && t.flags.get((b * t.w + a) as usize).is_some_and(|f| f & staff::WORKED != 0)
    }

    /// A weed stands on this tile (the staff's tile flags).
    fn staff_weed(&self, a: i32, b: i32) -> bool {
        let t = &self.staff_tiles;
        a < t.w && b < t.h && t.flags.get((b * t.w + a) as usize).is_some_and(|f| f & staff::WEEDS != 0)
    }

    /// A tree, scenic tree or flower bed planted on this tile is still growing (`sprouts`).
    fn sprouting(&self, a: i32, b: i32) -> bool {
        self.sprouts.contains_key(&(a, b))
    }

    /// A tree, scenic tree or flower bed was just planted on this tile: it starts growing from its first frame (the exe sets
    /// the tile's flag 0x4000 and its counter to 0, 0x420608 for the brushes).
    pub(crate) fn start_sprout(&mut self, a: i32, b: i32) {
        self.sprouts.insert((a, b), 0);
    }

    /// The Grown action (replay.rs): the exe drops the tile's flag 0x4000 when a tree reaches its last frame (0x4142cd), which
    /// ends the weeds' growing counter's mark too; nothing else reads it on a tile without a weed.
    pub(crate) fn sprouts_grown(&mut self, tiles: &[(i32, i32)]) {
        let t = &mut self.staff_tiles;
        for &(a, b) in tiles {
            if a < t.w && b < t.h {
                if let Some(f) = t.flags.get_mut((b * t.w + a) as usize) {
                    if *f & staff::WEEDS == 0 {
                        *f &= !staff::WORKED;
                    }
                }
            }
        }
    }

    /// Newly planted trees, scenic trees and flower beds grow by a frame for each frame the exe would have drawn: its 87 ms
    /// frames on the clock, which go on while the game is paused (the tile loop 0x413da3 runs on every pass of the main
    /// loop; only the tick stops), or the game's ticks when they come faster (the fast mode draws a frame per tick). The
    /// last frame of the tree or bed piece ends the growing (sg_core::decor::sprout_frame), and the plants are then drawn
    /// again as grown ones, exactly as they are drawn after a load.
    fn update_tree_growth(&mut self) {
        let ticks = self.game_tick.wrapping_sub(self.sprout_tick);
        self.sprout_tick = self.game_tick;
        let hold = sprout_hold();
        if !self.sprouts.is_empty() {
            let steps = self.exe_steps.max(ticks.min(1000));
            for c in self.sprouts.values_mut() {
                *c = hold.unwrap_or(c.saturating_add(steps));
            }
        }
        let mut done: Vec<(i32, i32)> = Vec::new();
        let (mut trees, mut objects) = (false, false);
        for pi in 0..self.props.len() {
            let p = &self.props[pi];
            let (Some((a, b, lead)), Some(bi), tree) = (p.sprout, p.body, p.tree) else { continue };
            let n = self.sprites[bi].s.frames_per_view;
            let (frame, grown) = match self.sprouts.get(&(a, b)) {
                Some(&c) => {
                    let (f, g) = sg_core::decor::sprout_frame(c, n, hold.is_some() || self.tile_on_screen(a, b).is_some());
                    (f, g && hold.is_none() && lead)
                }
                None => (n - 1, true),
            };
            self.props[pi].frame = frame;
            if grown {
                done.push((a, b));
                trees |= tree;
                objects |= !tree;
            }
        }
        for &(a, b) in &done {
            self.sprouts.remove(&(a, b));
        }
        // what the staff read of it is logged (replay.rs): the drawing's clock is not the game's, and a replay's are its log's
        if !done.is_empty() && !self.rec.replaying() {
            self.log_grown(&done);
            self.sprouts_grown(&done);
        }
        // a plant whose tile was painted over before it was drawn
        if hold.is_none() {
            self.sprouts.retain(|_, c| *c <= 0x40);
        }
        if trees {
            self.refresh_trees();
        }
        if objects {
            self.props.retain(|p| !p.object);
            self.add_land_objects();
        }
        // a ball washer plays its frames while a golfer washes (0x463180 case 3: the tile's counter while the tile is
        // flagged and the counter is short of the frame count, frame 0 otherwise)
        for p in self.props.iter_mut().filter(|p| p.object) {
            let (Some((a, b)), Some(bi)) = (p.grow_tile, p.body) else { continue };
            let t = &self.staff_tiles;
            let i = (b * t.w + a) as usize;
            let n = self.sprites[bi].s.frames_per_view;
            let c = t.counter.get(i).copied().unwrap_or(0) as i32;
            let washing = a < t.w && t.flags.get(i).is_some_and(|f| f & staff::WORKED != 0);
            p.frame = if washing && c < n { c } else { 0 };
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
    /// Building kinds unlocked: the club's counter, never below what the open holes have earned (saves from before the counter
    /// was kept), all of them in sandbox and with the Unlock all extra (sg_core::extras).
    pub fn unlocked(&self) -> i32 {
        // the course editor has every kind (editor.rs)
        if self.econ.sandbox || self.club.extras.unlock_all || self.econ.free {
            return 17;
        }
        self.club.unlocked.max(land::unlocked_kinds(self.hole_numbers.len(), false))
    }

    pub fn build_available(&self, kind: usize) -> bool {
        // The garden page (kinds up to 5, the willow and the bridge) is always open; buildings unlock with holes.
        let k = kind as i32;
        // landmarks once the club owns a type (the property's own, donations, Happy Endings), or the extras' pages are on
        if k == land::K_LANDMARK && self.strip_pages().is_empty() {
            return false;
        }
        // home sites only while Silver and better members outnumber them (0x56d1b0; open in sandbox, and with the Unlock all
        // extra, as there)
        if k == land::K_HOME_SITE && self.club.homesite_demand < 1 && !self.econ.sandbox && !self.club.extras.unlock_all && !self.econ.free
        {
            return false;
        }
        OFFERED_KINDS.contains(&k) && (k <= 5 || k == land::K_WILLOW || k == land::K_BRIDGE || k < self.unlocked())
    }

    /// Scenery: trees on Woods tiles, the placed buildings, a clubhouse, and the pool of golfers.
    pub fn populate_props(&mut self) {
        self.props.clear();
        // trees on a course just generated or loaded stand grown
        self.sprouts.clear();
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
        let pack = THEME_PACKS.get(self.theme_pack).copied().unwrap_or("Standard").replace(' ', "_");
        self.club = Club::new(if roster.is_empty() { sg_core::roster::load(&self.game_dir, &pack) } else { roster });
        self.club.stories = stories;
        self.club.celebrities = celebrities;
        self.club.pros = pros;
        // the port's extras for the new game (a loaded game takes its own)
        self.club.extras = self.extras_next;
        self.econ.extras = self.club.extras;
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
        // what was showing of the old course: its floating money, a golfer's card, the shot analysis
        self.floats = [(0, 0, 0, 0, 0.0); 8];
        self.map_labels.clear();
        self.undo_mark = None;
        self.card = None;
        self.analysis = None;
        // and its messages: one showing would carry over, and Repeat Last Message would bring it back
        self.ticker = Default::default();
        self.last_message.clear();
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
                crate::bugreport::warn(&format!("error: {e}"));
                return false;
            }
        };
        self.clear_batches(g);
        for t in self.textures.drain().filter_map(|(_, t)| t) {
            g.ctx.delete_texture(t);
        }
        self.catalog = cat;
        self.theme = theme;
        sg_core::land::set_name_theme(self.exe_theme());
        for s in self.sprites.drain(..) {
            for t in s.tex.into_iter().flatten() {
                g.ctx.delete_texture(t);
            }
        }
        // the recoloured frames and the atlas were made from these sprites (by their indices)
        for (_, t) in self.outfit_tex.drain() {
            g.ctx.delete_texture(t);
        }
        self.forget_atlas(g);
        self.sprite_index.clear();
        self.hd.sprites.clear();
        self.pack_art.clear();
        for (_, t) in self.red_tex.drain() {
            g.ctx.delete_texture(t);
        }
        let lighting = sg_core::fsutil::read_file(self.game_path(&format!("{}Lighting.txt", THEMES[theme])))
            .map(|d| sg_core::terrain::parse_lighting(&sg_core::formats::latin1(&d)))
            .unwrap_or_default();
        self.light = lighting;
        let t0 = std::time::Instant::now();
        self.rebuild_batches(g);
        stage(&format!("load_theme: batches {:.1}s", t0.elapsed().as_secs_f32()));
        self.populate_props();
        stage(&format!("load_theme: props {:.1}s", t0.elapsed().as_secs_f32()));
        self.register_portraits(g);
        stage(&format!("load_theme: portraits {:.1}s", t0.elapsed().as_secs_f32()));
        true
    }

    /// Empties Classic's sprite atlas (atlas.rs) and where the frames were put in it: the art or how it is drawn changed.
    pub fn forget_atlas(&mut self, g: &mut Gfx) {
        self.sprite_atlas.clear();
        self.outfit_atlas.clear();
        g.atlas_clear();
    }

    // ---- golfers and the club -----------------------------------------------------------------------------------------------

    /// Brings the golfers' view of the course up to date: tiles, objects and levels from the land, weeds from the staff's tiles,
    /// the clubhouse door, and one hole record per tee and green pair (back and forward tee on the tee, pin and cup on the green).
    pub fn sync_course(&mut self) {
        // outside an action or a tick it is logged (replay.rs), and during a replay only the log's count
        if !self.sync_hook() {
            return;
        }
        self.ensure_land();
        if let Some(land) = self.land.as_mut() {
            land.sync_from_terrain(&self.terrain);
        }
        let Some(land) = &self.land else { return };
        self.course.sync(land, self.difficulty == 0 || self.econ.sandbox);
        self.water_depth = sg_core::wildlife::water_depth(&self.course);
        self.course.door = self.club_anchor();
        self.course.theme = self.exe_theme();
        land::set_name_theme(self.course.theme);
        if self.course.objects.first().map(|o| o.kind) != Some(land::K_CLUBHOUSE) {
            let (a, b) = self.course.door;
            self.course.objects.insert(0, land::Object { kind: land::K_CLUBHOUSE, a, b, dir: 0, flags: 0x40, sub: 0, val: 0 });
        }
        self.weeds_to_course();
        for f in self.course.flags.iter_mut() {
            *f &= !(sg_core::course::f::CUP | if *f & sg_core::course::f::FOOTPRINT == 0 { sg_core::course::f::LOW } else { 0 });
        }
        // the hole numbers on tees and cup greens, from the golfers' hole records (Course B's too, numbered 1 to 18 as
        // well: sg_core::second)
        self.club.show_course(false);
        for b in [false, true] {
            if b && self.club.course_b.is_none() {
                break;
            }
            for h in 1..19 {
                let Some(rec) = self.club.course_holes(b).get(h) else { continue };
                for (k, (a, b)) in [rec.back, rec.fwd, rec.pin].into_iter().enumerate() {
                    if a != 0 && sg_core::course::inside(a, b) {
                        let i = sg_core::course::idx(a, b);
                        let cup = if k == 2 { sg_core::course::f::CUP } else { 0 };
                        self.course.flags[i] = (self.course.flags[i] & !0x1f) | cup | h as u16;
                    }
                }
            }
        }
        // the Multiple tees extra's forward and middle tees (nothing on a course without them)
        self.club.sync_extra_tees(&mut self.course);
        let door = self.course.door;
        let home = &mut self.club.holes[19];
        home.par = 0;
        home.back = door;
        home.fwd = door;
        home.pin = door;
        if std::mem::take(&mut self.adopt_holes) {
            self.adopt_painted_holes();
        }
        // every terrain change plans the hole being built again; a sync with nothing changed (a redraw, the Course Report, the
        // shot analysis) keeps the plan (see `plan_made_on`)
        // (the hole being built on the course the hole panel's A/B switch shows: sg_core::second)
        let inputs = self.plan_inputs();
        if self.plan_made_on != Some(inputs) && std::mem::take(&mut self.skip_plan) {
            // a replay's load: the plan stays as the game it was saved from had it (replay.rs)
            self.plan_made_on = Some(inputs);
        }
        if self.plan_made_on != Some(inputs) {
            self.club.game &= !golf::game::LAYOUT;
            self.on_hole_course(|a| a.club.layout(&mut a.course, &mut a.exe_rng));
            self.plan_made_on = Some(inputs);
        }
        self.rebuild_hole_routes();
    }

    /// What the plan of the hole being built is made on, as a hash: the golfers' course as the terrain and the buildings make
    /// it (not the weeds and work marks, the counters of landings and moods, nor the planner's scratch hazard bytes, which
    /// change as the game runs without a terrain refresh), the hole being built and its tees and cup, and the two tees mode.
    fn plan_inputs(&self) -> u64 {
        use sg_core::course::f;
        let c = &self.course;
        let mut h = sg_core::fingerprint::Fnv64::new();
        h.u64(sg_core::fingerprint::hash(&(&c.ty, &c.obj, &c.height, &c.walls, c.theme, c.door, c.editor)));
        for &v in &c.flags {
            h.u64((v & !(f::WEED | f::GROWING)) as u64);
        }
        for o in &c.objects {
            h.u64(sg_core::fingerprint::hash(&(o.kind, o.a, o.b, o.dir)));
        }
        let b = self.hole_b();
        let n = self.club.course_next_hole(b);
        let r = &self.club.course_holes(b)[n.clamp(0, 19) as usize];
        h.u64(sg_core::fingerprint::hash(&(n, r.back, r.fwd, r.pin, self.club.game & golf::game::TWO_TEES)));
        if b {
            h.u64(1);
        }
        h.finish()
    }

    /// Whether the hole being built is planned on the course as it stands (a sync now would not plan it again).
    pub fn plan_is_current(&self) -> bool {
        self.plan_made_on == Some(self.plan_inputs())
    }

    /// The Second course extra (sg_core::second): the hole panel's A/B switch is on Course B and there is one.
    pub fn hole_b(&self) -> bool {
        self.hole_b_view && self.club.course_b.is_some() && self.club.extras.second_course
    }

    /// Runs `f` with the course the hole panel's A/B switch shows in the club's hole records (Course A's, as always,
    /// without the Second course extra), then puts back what was there.
    pub fn on_hole_course<R>(&mut self, f: impl FnOnce(&mut App) -> R) -> R {
        let was = self.club.b_shown();
        let b = self.hole_b();
        self.club.show_course(b);
        let r = f(self);
        self.club.show_course(was);
        r
    }

    /// Hole `h` of Course B (`b`) or Course A as the screens name it: "B3", or the number as the original has it.
    pub fn hole_text(&self, b: bool, h: i32) -> String {
        if b {
            trf("STR_COURSE_B_HOLE", &[("HOLE", &h)])
        } else {
            h.to_string()
        }
    }

    /// The A/B switch (the Second course extra): flips between Course A and Course B, starting Course B the first time.
    pub fn toggle_hole_course(&mut self) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            self.act(Action::CourseSwitch(!self.hole_b_view));
            return;
        }
        if !self.hole_b_view && !self.club.start_course_b() {
            return;
        }
        self.hole_b_view = !self.hole_b_view;
        self.show_toast(tr(if self.hole_b() { "STR_TOAST_COURSE_B" } else { "STR_TOAST_COURSE_A" }));
        self.dirty = true;
        self.sync_course();
    }

    /// The numbers of the open holes, from the golfers' hole records.
    fn rebuild_hole_routes(&mut self) {
        let numbers: Vec<i32> = (1..19).filter(|&h| self.club.holes[h as usize].par != 0).collect();
        if numbers != self.hole_numbers {
            self.hole_numbers = numbers;
            self.hole_stats = vec![HoleStat::default(); self.hole_numbers.len()];
        }
    }

    /// Terrain from outside the hole tool: each painted tee and green pair not yet a hole is built and opened, in the order
    /// the pairs are found (tees top row first), as if made with the tool (sg_core::holetool::painted_pairs, the port's own
    /// tooling).
    fn adopt_painted_holes(&mut self) {
        for (tee, pin) in sg_core::holetool::painted_pairs(&self.course) {
            let h = self.club.next_hole;
            if !(1..19).contains(&h) {
                break;
            }
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
        self.club.out.retain(|e| !matches!(e, golf::Event::Sound { .. } | golf::Event::Message { .. }));
    }

    /// The exe's open-hole command (key H, or a click on the new hole's cup green): the hole being built needs a tee and a
    /// green; its par comes from its length.
    pub fn open_hole(&mut self) -> bool {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            return self.act(Action::OpenHole);
        }
        self.sync_course();
        let Some(h) = self.on_hole_course(|a| a.club.open_hole(&mut a.course)) else { return false };
        if let Some(post) = self.club.first_post.take() {
            if let Some(e) = self.employees.first_mut().filter(|e| e.active) {
                e.post = Some(post);
            }
        }
        let rec = &self.club.course_holes(self.hole_b())[h];
        let label = if self.hole_b() { "B" } else { "" };
        println!("hole {label}{h} open: par {}, length {}", rec.par, rec.length);
        self.sync_course();
        true
    }

    /// The staff's weed tiles into the golfers' tile flags (the exe keeps both in one array).
    fn weeds_to_course(&mut self) {
        weeds_into(&self.staff_tiles, &mut self.course);
    }

    /// New weeds the golfers started, back into the staff's tiles.
    fn weeds_from_course(&mut self) {
        let t = &mut self.staff_tiles;
        if t.flags.len() != (t.w * t.h) as usize {
            return;
        }
        for a in 0..t.w.min(sg_core::course::n()) {
            for b in 0..t.h.min(sg_core::course::n()) {
                let si = (b * t.w + a) as usize;
                let ci = sg_core::course::idx(a, b);
                let bits = self.course.flags[ci] & (staff::WEEDS | staff::WORKED);
                t.flags[si] = (t.flags[si] & !(staff::WEEDS | staff::WORKED)) | bits;
                t.counter[si] = self.course.growth[ci];
            }
        }
    }

    /// Sprite id (clip + body) of a golfer clip, loaded on first use: (body, shadow).
    pub(crate) fn golfer_clip(&mut self, id: i32) -> (Option<usize>, Option<usize>) {
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

    /// The length of a golfer clip from its file header alone, without decoding its frames (a loaded course has many
    /// bodies, and decoding all their clips at once took seconds in the browser); the clip itself loads when drawn. False
    /// when the header cannot tell (no header extension, a content pack sheet that may differ), and the clip is loaded.
    fn golfer_clip_length(&mut self, id: i32) -> bool {
        if crate::packart::active() || self.golfer_clips.contains_key(&id) {
            return false;
        }
        let Some(&(base, name)) = GOLFER_CLIPS.iter().rev().find(|(b, _)| *b <= id && id - *b <= 8) else { return false };
        let body = (id - base) as usize;
        let Some(prefix) = GOLFER_BODIES.get(body).filter(|p| !p.is_empty()) else { return false };
        let name = if base == 0x28 && body >= 5 { "NormalSwing" } else { name };
        let path = self.game_path(&format!("Flics/{prefix}_{name}.flc"));
        let Some(fpv) = sg_core::fsutil::read_file(&path).and_then(|d| sg_core::flc::header_frames_per_view(&d)) else {
            return false;
        };
        match self.club.clip_frames.get_mut(id as usize) {
            Some(slot) => *slot = fpv.max(1) as _,
            None => return false,
        }
        true
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
                if !self.golfer_clip_length(base + look) {
                    self.golfer_clip(base + look);
                }
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
        for f in self.floats.iter_mut() {
            f.3 = f.3.saturating_sub(1);
        }
        if self.ui_ok {
            // the opening story and the first design tip (0x45fd80), told by the course itself; a sandbox game skips only
            // the opening story (the exe's sandbox test returns from the tick 0x20 branch, the tip at 0x1400 has none)
            if tick == 0x20 && !self.econ.sandbox {
                let text = self.welcome_text();
                self.post_message(&text, 1, -4);
            } else if tick == 0x1400 {
                self.post_message(tr(DESIGN_TIP), 0, -4);
            }
        }
        if self.econ.sandbox {
            // the exe holds a sandbox's cash at 10000 units every frame
            self.econ.cash = 10000.0 * Economy::UNIT;
        }
        self.club.island = self.land.as_ref().map(|l| l.slot.record().coast == 2).unwrap_or(false);
        if std::mem::take(&mut self.club.land_offer) {
            // the commissioner approved an expansion: sound 56 and the question; yes shows the tracts
            self.ui_sound(56);
            if self.ui_ok {
                self.open_popup(crate::popup_ui::PopupKind::LandOffer);
            } else {
                self.open_land_screen();
            }
        }
        self.club.home_sites = sites;
        self.club.homesite_demand = self.club.ratings.waitlist;
        if tick.is_multiple_of(1024 / (self.difficulty.clamp(0, 3) as u32 + 2)) {
            // hole maintenance (+0x1f8): one unit per open hole each charge interval
            for h in self.club.holes.iter_mut().chain(self.club.course_b.iter_mut().flat_map(|b| b.holes.iter_mut())) {
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
                // the course's copy of the objects (made from the land's at each terrain change) takes the new values and
                // owners too, as a loaded game's copy has them (the port's own: it kept those of the last terrain change)
                for o in self.course.objects.iter_mut() {
                    if let Some(n) = l.objects.iter().find(|n| (n.kind, n.a, n.b) == (o.kind, o.a, o.b)) {
                        o.val = n.val;
                        o.sub = n.sub;
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
        // where the course is, for the Weather that matters extra (sg_core::weather; unused while it is off)
        self.club.place = self.weather_place();
        self.pro_round_tick();
        // what the screen hands the simulation before the tick (logged, and a replay's from the log: replay.rs)
        let flags = self.tick_flags();
        self.club.ticker_busy = flags.busy;
        self.club.advisor = flags.advisor;
        self.club.view_close = flags.close;
        self.club.message_up = flags.message;
        self.club.show_snapshots = flags.snapshots;
        self.club.tick(&mut self.course, &mut self.exe_rng, tick);
        // test hook: SG_GOLFER_LOG=N prints the golfers every N game ticks (how many play, queue and are frozen, their moods
        // and a hash of where they all are), to compare runs with and without the weather
        static GOLFER_LOG: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
        let every = *GOLFER_LOG.get_or_init(|| std::env::var("SG_GOLFER_LOG").ok().and_then(|v| v.parse().ok()).unwrap_or(0));
        if every > 0 && tick.is_multiple_of(every) {
            println!("{}", self.golfer_log_line(tick));
        }
        // test hook: SG_STUCK_LOG=T prints every golfer who has been on one hole for T game ticks or more (each further T)
        static STUCK_LOG: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
        let long = *STUCK_LOG.get_or_init(|| std::env::var("SG_STUCK_LOG").ok().and_then(|v| v.parse().ok()).unwrap_or(0));
        if long > 0 {
            for (i, g) in self.club.g.iter().take(golf::SLOTS).enumerate() {
                let on = tick.wrapping_sub(g.hole_tick);
                if (1..19).contains(&g.hole) && on >= long && on.is_multiple_of(long) {
                    let bt = self.course.ty.get(((g.bx >> 10) * sg_core::course::n() + (g.by >> 10)).max(0) as usize);
                    println!(
                        "stuck tick {tick} golfer {i} partner {} hole {} for {on} strokes {} sub {} pause {} flags {:#x} anim {} \
                         at ({},{}) ball ({},{}) tile {:?} speed {} mood {} partner_hole {} partner_strokes {} partner_sub {}",
                        g.partner,
                        g.hole,
                        g.strokes,
                        g.sub,
                        g.pause,
                        g.flags,
                        g.anim,
                        g.x >> 10,
                        g.y >> 10,
                        g.bx >> 10,
                        g.by >> 10,
                        bt,
                        g.speed,
                        g.mood,
                        self.club.g[i ^ 1].hole,
                        self.club.g[i ^ 1].strokes,
                        self.club.g[i ^ 1].sub,
                    );
                }
            }
        }
        sg_core::ratings::pass(&mut self.club, self.difficulty);
        self.pro_after_tick();
        self.board_tick();
        // the month's history entry (the year-end report reads it): written at each charge and again at every month start
        // (the main frame's tick & 0x3ff == 0 block, 0x40f5c0); on Medium and Expert the charge interval (341, 204 ticks)
        // never lands on a month start, so without the second write the year-end slot stayed empty (all zero, in red)
        // (the month start goes by the date, sg_core::extras::Calendar; the charges by the game tick)
        let (new_month, new_year) = (self.club.cal.new_month(tick), self.club.cal.new_year(tick));
        if tick.is_multiple_of(1024 / (self.difficulty.clamp(0, 3) as u32 + 2)) || new_month {
            let members = self.club.member_count();
            self.club.record_history(self.club.cash, members);
        }
        if new_month && self.difficulty != 0 && !self.econ.sandbox && !self.club.championship() {
            // the monthly reminder for buildings with no path to the clubhouse
            let cut: Vec<(String, (i32, i32))> = self
                .course
                .objects
                .iter()
                .skip(1)
                .filter(|o| (6..=14).contains(&o.kind) && o.flags & 0x40 == 0)
                .map(|o| (land::building_name(o.kind).to_string(), (o.a * 1024 + 1024, o.b * 1024 + 1024)))
                .collect();
            for (name, at) in cut {
                // the extras' Show button and history take the building's place (extras.rs)
                self.xs.next_place = Some(at);
                self.show_toast(&trf("STR_TOAST_NOT_OPERATIONAL", &[("BUILDING", &name)]));
            }
        }
        if new_year && !self.club.championship() {
            println!("[{:6.1}s] end of year {}", self.sim_time, 2000 + (self.club.date() >> 13));
            self.open_year_end();
            self.top10_year_end();
        }
        self.wildlife_tick();
        self.tourney_after_tick();
        self.weeds_from_course();
        let events: Vec<golf::Event> = self.club.out.drain(..).collect();
        for e in events {
            match e {
                golf::Event::Sound { slot, at, delay } => {
                    let at = at.map(|(x, y)| self.units_to_world(x, y));
                    self.slot_sound_after(slot, at, delay);
                }
                golf::Event::Earn { units, column, at } => {
                    self.float_money(units, at);
                    if crate::extras::people() {
                        self.count_spent(units, at);
                    }
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
                golf::Event::Message { text, speaker, priority } => {
                    if !text.is_empty() {
                        // the club already refused what the busy ticker would refuse; a negative priority still delays it
                        println!("[{:6.1}s] {text}", self.sim_time);
                        self.post_message(&text, if priority < 0 { priority } else { 1 }, speaker);
                    }
                }
                // a hole of Course B (sg_core::second) counts in the takings, not in Course A's hole figures and graphs
                golf::Event::HoleDone { g, .. } if self.club.g.get(g).is_some_and(|gg| gg.course_b) => {}
                golf::Event::HoleDone { g, hole, strokes, mood, fee } => {
                    self.script_hole_done(g, hole, strokes);
                    if crate::extras::graphs() {
                        self.graphs_hole_done(hole, strokes, mood, fee);
                    }
                    if self.hole_stats.len() != self.hole_numbers.len() {
                        self.hole_stats = vec![HoleStat::default(); self.hole_numbers.len()];
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
                golf::Event::Thought { g, id: 0x32, arg } => {
                    if crate::extras::people() {
                        self.keep_thought(g, 0x32, arg);
                    }
                    if let Some(line) = self.club.story_line(g) {
                        let title = self.club.stories.title(self.club.g[g].story).trim().to_string();
                        let who = self.club.name(g);
                        println!("[{:6.1}s] story '{title}': {who}: {line}", self.sim_time);
                        // drawn over the golfer while the thought lasts (thoughts_ui.rs); the message history keeps it
                        self.keep_story_line(g, &format!("{who} ({title}): {line}"));
                    }
                }
                golf::Event::Thought { g, id, arg } => {
                    // test hook: SG_THOUGHTS=1 prints every golfer thought as it is raised, with the golfer's mood
                    if std::env::var_os("SG_THOUGHTS").is_some() {
                        let line = self.club.thought(&self.course, id, arg & 0x3fff, g);
                        println!("thought golfer {g} type {id:#x} arg {arg} mood {}: {}", self.club.g[g].mood, line.text);
                    }
                    if crate::extras::people() {
                        self.keep_thought(g, id, arg);
                    }
                    if crate::extras::hole_comments() {
                        self.tally_thought(g, id, arg);
                    }
                    // the landing map's heat map of complaints (insight.rs)
                    if crate::extras::landing_map() {
                        self.heat_thought(g, id, arg);
                    }
                }
                golf::Event::GreatShot { g, partner, text, bad, pts } => self.great_shot_event(g, partner, text, bad, &pts),
            }
        }
        // the OpenSGolf extras' history graphs move on to a new month with the date (graphs.rs)
        if crate::extras::graphs() {
            self.graphs_tick();
        }
        // the scenario's goals and the packs' scripts (docs/MODDING.md 5 and 6)
        self.scenario_tick(tick);
    }

    /// Advances the club by dt seconds: one golfer and staff update per game tick (87 ms), fees, wages and the board's messages.
    pub fn step_game(&mut self, dt: f32) {
        self.tick_acc += dt as f64;
        let tick_s = sg_core::flight::TICK_MS as f64 / 1000.0;
        // the great-shot SimFoto holds the game where it is (0x407e00 waits for a click inside the golfers' tick)
        while self.tick_acc >= tick_s && self.great_shot.is_none() && !self.comfort.hold {
            self.tick_acc -= tick_s;
            // the Pause on big news extra (comfort.rs) compares the club before and after the tick (the money's own tick
            // is the staff's)
            // the action log's and a replay's point between two ticks (replay.rs)
            self.tick_begin();
            let before = (crate::extras::pause_events() && self.ui_ok).then(|| self.news());
            if !self.econ.game_over {
                self.club_tick();
            }
            // Enhanced graphics only: the balls' trails and impacts read the simulation after its tick (fx_ball.rs)
            if crate::fx::active() {
                self.fx_ball_tick();
            }
            // the OpenSGolf extras' landing map reads the balls after the tick too (insight.rs)
            if crate::extras::landing_map() {
                self.insight_tick();
            }
            self.ticker.step();
            self.staff_tick();
            self.staff_animate();
            self.resident_tick();
            // test hooks: SG_FINGERPRINT and SG_SAVE_AT (fingerprint.rs)
            self.fingerprint_note("");
            self.save_at_hook();
            self.tick_end();
            if before.is_some_and(|b| crate::comfort::big_news(&b, &self.news())) {
                self.pause_for_news();
            }
        }
    }

    /// The sizes of the collections that could grow over a long game, for the performance log (perf.rs).
    /// The `SG_GOLFER_LOG` line of tick `tick`: the weather the simulation plays (Weather that matters), the golfers on
    /// the holes, queued and pausing, their moods, the quits so far, the cash and a hash of every golfer's place.
    fn golfer_log_line(&self, tick: u32) -> String {
        let c = &self.club;
        let live = || c.g.iter().take(golf::SLOTS).filter(|g| g.hole != 0);
        let on = live().filter(|g| (1..19).contains(&g.hole)).count();
        let queued = live().filter(|g| g.hole == -1).count();
        let pausing = live().filter(|g| (1..19).contains(&g.hole) && g.pause < 0).count();
        let low = live().filter(|g| g.mood < 0).count();
        let mood: i32 = live().map(|g| g.mood).sum();
        let quits: i32 = c.holes.iter().map(|h| h.quits).sum();
        let pos =
            c.g.iter().take(golf::SLOTS).fold(0u64, |h, g| {
                (h ^ ((g.x as u32 as u64) << 32 | g.y as u32 as u64) ^ g.hole as u32 as u64).wrapping_mul(0x100_0000_01b3)
            });
        let w = c.today.map(|t| format!("rain {:.2} storm {}", t.rain, t.storm as u8)).unwrap_or_else(|| "off".into());
        format!(
            "golfers tick {tick} weather {w} on {on} queued {queued} pausing {pausing} mood {mood} low {low} quits {quits} cash {} pos {pos:016x}",
            c.cash
        )
    }

    pub fn perf_sizes(&self) -> String {
        let c = &self.club;
        let lazy: usize = self.sprites.iter().map(|s| s.tex.iter().filter(|t| t.is_some()).count()).sum();
        format!(
            "props={} (weeds {} ambient {} objects {} trees {} shown {}) sprite_tex={lazy} outfit_tex={} atlas={}+{} red_tex={} textures={} snapshots={} batches={}/{}/{}/{} members={} roster={} \
             event_log={} history={} out={} shot_recs={} ledger={} hole_stats={} ball_trail={} voices={}",
            self.props.len(),
            self.props.iter().filter(|p| p.weed).count(),
            self.props.iter().filter(|p| p.ambient).count(),
            self.props.iter().filter(|p| p.object).count(),
            self.props.iter().filter(|p| p.tree).count(),
            self.props.iter().filter(|p| !p.hidden).count(),
            self.outfit_tex.len(),
            self.sprite_atlas.len(),
            self.outfit_atlas.len(),
            self.red_tex.len(),
            self.textures.len(),
            self.snapshots.len(),
            self.batches.len(),
            self.path_batches.len(),
            self.mud_batches.len(),
            self.wall_batches.len(),
            c.members.len(),
            c.roster.len(),
            c.event_log.len(),
            c.history.len(),
            c.out.len(),
            c.shot_recs.len(),
            self.econ.ledger.len(),
            self.hole_stats.len(),
            self.ball_trail.len(),
            self.slot_voices.len(),
        )
    }

    /// Advances the simulation to the current time and points every golfer prop at its golfer.
    pub fn update_props(&mut self) {
        let perf = crate::perf::on().then(|| (std::time::Instant::now(), self.game_tick));
        // no time passes in the course editor (editor.rs): no golfers, no calendar
        if self.editor.is_some() {
            self.sim_time = self.time;
        }
        while self.sim_time < self.time && self.great_shot.is_none() && !self.comfort.hold {
            let t0 = now();
            self.step_game(1.0 / 60.0);
            tick_bench_add(now() - t0);
            self.sim_time += 1.0 / 60.0;
        }
        // paused for news (comfort.rs): the time not played yet is let go, so the game goes on from that tick
        if std::mem::take(&mut self.comfort.hold) {
            self.time = self.sim_time;
            self.tick_acc = 0.0;
        }
        if let Some((t, tick)) = perf {
            crate::perf::sim(t.elapsed().as_secs_f64() * 1000.0, self.game_tick.wrapping_sub(tick) as u64);
        }
        if self.time < self.sim_time - 1.0 {
            // clock went backwards
            self.sim_time = 0.0;
        }
        // where each golfer is on screen, or -1 when off it (the exe's drawing records this; a golfer who has finished and is
        // out of sight goes straight home)
        self.screen_hook();
        // a replay applies what is due before the next tick as soon as it can (the log is played through at its last tick)
        self.replay_due();
        if std::mem::take(&mut self.test_great_shot) {
            self.test_great_shot_now();
        }
        let track = crate::enhanced::smooth_motion();
        const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
        const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];
        for pi in 0..self.props.len() {
            let Some(gi) = self.props[pi].golfer else { continue };
            let (h, x, y, facing, anim, club) = {
                let g = &self.club.g[gi];
                (g.hole, g.x, g.y, g.facing, g.anim, g.club)
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
            // The swing clips are drawn toward the shot, as the address is: in the original's art the address's last frame and
            // the swing's first are the same picture in every view, and each view sends the ball the way a golfer walking in
            // that view faces (MalePSS_NormalAddress.flc against MalePSS_PerfectSwing.flc). The swing's facing is a quarter
            // turn round from the shot for every club but the putter (golfer.rs, the shot's start), so it is turned back
            // here; OpenSGolf up to 0.6.4 drew it one view round from the facing, so the golfer turned 45 degrees as he
            // started his swing and swung 45 degrees off the ball's line. The bench clips have four views and take half the
            // eight-way view. A left-hander (the Left-handed golfers extra) is drawn mirrored while he addresses the ball and
            // swings, in the view whose mirror image sends the ball the same way (prop_view).
            let mirror = (anim < 7 || anim == sg_core::golfer::anim::ADDRESS) && self.club.left_handed(gi);
            let mut d = facing & 7;
            if anim < 7 && club != 13 {
                d = (d - 2) & 7;
            }
            let (wx, wz) = self.units_to_world(x, y);
            let outfit = self.club.outfit(gi);
            let p = &mut self.props[pi];
            p.hidden = false;
            p.body = Some(b);
            p.shadow = shadow;
            p.x = wx;
            p.z = wz;
            p.frame = frame;
            p.heading = DY[d as usize].atan2(DX[d as usize]).to_degrees();
            p.facing = -((d + 2 + (d & 1)) / 2);
            p.outfit = Some(outfit);
            p.mirror = mirror;
        }
        self.update_staff_props();
        self.update_resident_props();
        self.update_ambient_props();
        self.update_weed_props();
        self.update_tree_growth();
        self.update_decor_props();
        self.update_tee_arrows();
        for p in self.props.iter_mut() {
            if p.animated {
                if let Some(b) = p.body {
                    let n = self.sprites[b].s.frames_per_view.max(1);
                    p.frame = match p.tv {
                        Some((a, tb)) => sg_core::objects::tv_frame(self.game_tick, a, tb, n),
                        None => (self.game_tick.wrapping_add(p.phase) % n as u32) as i32,
                    };
                }
            }
        }
        if track {
            self.track_motion();
        }
    }

    /// The console's course summary (scripted runs): each open hole's par and yards from the golfers' hole records, and how
    /// many path tiles the clubhouse flood reaches (the exe's tile flag 0x40; the others are drawn as unjoined paths).
    pub fn report_course(&mut self, force: bool) {
        self.sync_course();
        let holes: Vec<String> = self
            .hole_numbers
            .iter()
            .map(|&h| {
                let r = &self.club.holes[h as usize];
                format!("{h}: par {} {} yd", r.par, r.length)
            })
            .collect();
        let joined = self.land.as_ref().map(|l| l.joined_tiles()).unwrap_or_default();
        let (mut paths, mut on) = (0, 0);
        for (i, &k) in self.terrain.path_kind.iter().enumerate() {
            if k == 0 {
                continue;
            }
            paths += 1;
            let (x, y) = (i as i32 % self.terrain.w, i as i32 / self.terrain.w);
            if !sg_core::course::inside(x, y) || joined.get(sg_core::course::idx(x, y)).copied().unwrap_or(true) {
                on += 1;
            }
        }
        let r = format!(
            "{} holes [{}]; paths: {paths} tiles, {on} joined to the clubhouse, {} not joined",
            self.hole_numbers.len(),
            holes.join(", "),
            paths - on
        );
        if force || r != self.last_report {
            println!("course: {r}");
            self.last_report = r;
        }
    }

    // ---- new games, stories --------------------------------------------------------------------------------------------------

    /// Deals the properties out to the offer slots for a new game, as the exe does before showing the property chooser.
    pub fn deal_offer(&mut self, sandbox: bool) {
        // the acres follow the size the game will be played at
        let _size = sg_core::mapsize::scoped(self.course_size);
        self.offer = land::deal_offer(&mut self.exe_rng, sandbox);
        self.slot_of = slots_of(&self.offer);
    }

    /// Swaps property `k` with the one in the cheapest slot it may hold (0 for Parkland, else 1).
    pub fn cheap_slot(&mut self, k: usize) {
        let to = if sg_core::data::properties()[k].theme == 0 { 0 } else { 1 };
        let from = self.slot_of[k];
        if from == to {
            return;
        }
        let other = self.offer[to].property;
        let sandbox = self.econ.sandbox;
        let _size = sg_core::mapsize::scoped(self.course_size);
        self.offer[to] = Slot { property: k, acres: land::slot_acres(to as i32, k, sandbox) };
        self.offer[from] = Slot { property: other, acres: land::slot_acres(from as i32, other, sandbox) };
        self.slot_of = slots_of(&self.offer);
    }

    /// Acres and price (in money) of a property in this game's offer. On a larger course size the price grows with the map's
    /// edge (sg_core::mapsize::scale_money), the acres with its area.
    pub fn offer_for(&self, prop_idx: usize) -> (i32, i32) {
        let slot = self.slot_of[prop_idx];
        let price = sg_core::mapsize::scale_money_at(self.course_size, sg_core::data::economy().offer_units[slot] as i64 * 100);
        // the Hardcore extra's dearer land (nothing when it is off)
        (self.offer[slot].acres, self.extras_next.land(price as f64) as i32)
    }

    pub fn start_game(&mut self, g: &mut Gfx, prop_idx: usize, sandbox: bool) {
        let t0 = std::time::Instant::now();
        self.undo.clear();
        // the last game's point under the pointer (issue #146)
        self.has_hit = false;
        // a new game is never the course editor's (editor.rs makes it the editor's afterwards)
        self.editor = None;
        self.econ.free = false;
        self.autosave_restart();
        let p = sg_core::data::properties()[prop_idx];
        // a game started without the difficulty screen (scripted runs) still gets its Top 10 id
        if self.course_id.is_none() {
            self.course_id = Some(self.exe_rng.below(0xffff));
        }
        let (acres, price) = self.offer_for(prop_idx);
        // the map in play takes the chosen course size (sg_core::mapsize; the original's 50 x 50 by default)
        sg_core::mapsize::set(self.course_size);
        self.econ.sandbox = sandbox;
        // The property is paid for out of the starting funds.
        // The exe starts with 1000 units (10000 in sandbox) and takes the property's price off a normal game. A larger course
        // size starts with funds grown with the map's edge, as its prices are.
        let funds = |f: i32| sg_core::mapsize::scale_money_at(self.course_size, f as i64);
        self.econ.start_cash = if sandbox {
            funds(sg_core::data::economy().sandbox_funds) as f64
        } else {
            (funds(sg_core::data::economy().start_funds) - price as i64) as f64
        };
        if let Some(m) = self.scen.start_money.take() {
            // a scenario's own starting cash (docs/MODDING.md 5)
            self.econ.start_cash = m as f64;
        }
        let slot_index = self.slot_of[prop_idx];
        let land = land::generate(slot_index, self.offer[slot_index], self.difficulty, sandbox, &mut self.exe_rng, &self.noise);
        stage(&format!("start_game: land generated {:.1}s", t0.elapsed().as_secs_f32()));
        self.terrain = land.to_terrain();
        self.land = Some(land);
        self.course_name = p.course().to_string();
        self.load_theme(g, p.theme);
        stage(&format!("start_game: theme loaded {:.1}s", t0.elapsed().as_secs_f32()));
        self.club.course_name = self.course_name.clone();
        self.club.course_theme = self.exe_theme();
        self.bought.clear();
        self.mark_bought(prop_idx);
        self.xs = Default::default();
        self.screen = Screen::Play;
        self.hover = -1;
        self.edit = false;
        // a new game starts in dock mode 0 (0x567afc is 0): the Build Course panel is open, as in footage of the original,
        // with no tool armed
        self.panel = 1;
        self.pstate.alt = false;
        self.reset_staff();
        self.load_story();
        self.reset_clock = true;
        // the game opens on the clubhouse
        let (a, b) = self.club_anchor();
        let (x, z) = self.units_to_world(a * 1024 + 1024, b * 1024 + 1024);
        self.cam_x = x;
        self.cam_z = z;
        println!(
            "new game: {}, {} acres, price {}, cash left {:.0}{}",
            p.name(),
            acres,
            price,
            self.econ.start_cash,
            if sandbox { " (sandbox)" } else { "" }
        );
        // the scenario's goals (none for a free game) and the packs' scripts
        self.begin_game_scripts();
        stage(&format!("start_game: done {:.1}s", t0.elapsed().as_secs_f32()));
        // the action log starts again from the new game (replay.rs)
        self.log_restart();
    }

    /// "Return to Main Menu." at the end of a career, the way the exe leaves a game (0x420e0f): the game is saved as
    /// "&QuitSave" (an autosave in the Load list), the course ambience fades (slot 0x2d over a second) and the title comes
    /// back. Loading that save brings the same question back, as its debt counter is still past 2.
    pub fn end_career(&mut self) {
        let path = crate::files_ui::saved_games_dir().join("&QuitSave.sgs");
        match self.save_game(&path) {
            Ok(()) => println!("saved game {}", path.display()),
            Err(e) => crate::bugreport::warn(&format!("could not save {}: {e}", path.display())),
        }
        self.screen = Screen::Menu;
        self.hover = -1;
    }

    /// The whole-game save file, next to the course file.
    pub fn game_file(&self) -> PathBuf {
        self.course_file.with_extension("sgs")
    }

    /// Where the accomplishment snapshots (accomp<id>.png) are written and the board reads them back.
    pub fn snapshot_dir(&self) -> PathBuf {
        self.course_file.parent().map(|p| p.to_path_buf()).unwrap_or_default().join("snapshots")
    }

    /// Saves the whole game.
    pub fn save_game(&self, path: &Path) -> Result<(), String> {
        let out = self.save_bytes(6)?;
        if sg_core::fsutil::write_file(path, &out) {
            self.fingerprint_note("save");
            // the save self-check reads the file back over the next frames (selfcheck.rs)
            self.selfcheck_after_save(&out);
            Ok(())
        } else {
            Err(trf("STR_ERROR_COULD_NOT_WRITE", &[("FILE", &path.display())]))
        }
    }

    /// The whole game as a saved file's bytes, compressed at zlib `level` (undo.rs keeps them in memory at a fast level).
    pub fn save_bytes(&self, level: u8) -> Result<Vec<u8>, String> {
        let size = sg_core::mapsize::n();
        let save = SaveGame {
            // version 2: a course size other than the original's, recorded in `size` (version 1 saves are all 50 x 50)
            version: if size == sg_core::mapsize::ORIGINAL { 1 } else { 2 },
            size,
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
            theme_pack: THEME_PACKS.get(self.theme_pack).copied().unwrap_or("Standard").replace(' ', "_"),
            course_id: self.course_id,
            bought: self.bought.clone(),
            goals: self.scen.goals.clone(),
            scripts: self.scripts_for_save(),
            extras: crate::extras::Kept {
                // the insight extras' part only while their ticks are on (insight.rs)
                insight: crate::insight::Kept::for_save(
                    &self.xs.kept.insight,
                    crate::extras::landing_map(),
                    crate::extras::hole_comments(),
                ),
                // the history graphs' months only while their tick is on (graphs.rs)
                graphs: self.xs.kept.graphs.clone().filter(|k| crate::extras::graphs() && !k.is_empty()),
                ..self.xs.kept.clone()
            },
            packs: self.saved_packs(),
            shared: false,
            imported: None,
            deco_rng: (!self.club.residents.is_empty()).then_some(self.deco_rng),
        };
        encode_save(&save, level)
    }

    /// Loads a whole game saved by `save_game` and goes on from where it was.
    pub fn load_game(&mut self, g: &mut Gfx, path: &Path) -> Result<(), String> {
        let s = read_save(path)?;
        self.load_save(g, s)?;
        self.fingerprint_note("load");
        Ok(())
    }

    /// Goes on with a game read by `read_save` (or `read_save_bytes`).
    pub fn load_save(&mut self, g: &mut Gfx, s: SaveGame) -> Result<(), String> {
        self.undo.clear();
        self.has_hit = false;
        // a loaded game is never the course editor's (editor.rs makes it the editor's afterwards; undo keeps it)
        self.editor = None;
        self.autosave_restart();
        self.load_save_with(g, s, false)?;
        // the action log starts again from the game loaded (replay.rs)
        self.log_restart();
        Ok(())
    }

    /// `load_save`; with `keep_art` (undo.rs) a save of the theme already loaded keeps its textures and only rebuilds the
    /// course's meshes and props.
    pub fn load_save_with(&mut self, g: &mut Gfx, s: SaveGame, keep_art: bool) -> Result<(), String> {
        self.load_save_into(Some(g), s, keep_art)
    }

    /// `load_save_with`; without `g` the game is loaded with no art, no portraits, no scripts run and no message: the save
    /// self-check's scratch copy of the game (selfcheck.rs), which only reads the simulation back.
    pub(crate) fn load_save_into(&mut self, mut g: Option<&mut Gfx>, mut s: SaveGame, keep_art: bool) -> Result<(), String> {
        let imported = s.imported.take();
        // the save's course size (older saves: the original's), before anything reads the map
        sg_core::mapsize::set(s.size);
        self.course_size = sg_core::mapsize::n();
        if let Some(k) = THEME_PACKS.iter().position(|p| p.replace(' ', "_").eq_ignore_ascii_case(&s.theme_pack)) {
            self.theme_pack = k;
        }
        self.terrain = s.terrain;
        self.land = s.land;
        if let Some(l) = &self.land {
            l.green_looks_to(&mut self.terrain);
        }
        self.course_name = s.course_name;
        self.difficulty = s.difficulty;
        self.course_id = s.course_id;
        self.bought = s.bought;
        if self.bought.iter().all(|&y| y < 0) {
            // a game saved before the purchases were kept: its own property, bought in a year not recorded (shown as 2001)
            if let Some(p) = self.land.as_ref().map(|l| l.slot.property) {
                self.bought = vec![-1; sg_core::data::properties().len()];
                if let Some(b) = self.bought.get_mut(p) {
                    *b = 0;
                }
            }
        }
        self.adopt_holes = false;
        self.hole_b_view = false;
        match g.as_deref_mut() {
            Some(g) if keep_art && s.theme == self.theme => {
                self.rebuild_batches(g);
                self.populate_props();
            }
            Some(g) => {
                if !self.load_theme(g, s.theme) {
                    return Err(tr("STR_ERROR_THEME_NOT_LOADED").into());
                }
            }
            // the scratch copy: only the theme the simulation reads (exe_theme)
            None => self.theme = s.theme,
        }
        self.club = s.club;
        if self.club.roster.is_empty() {
            // a course made in the course editor (editor.rs) leaves the theme's people to the player's own files, as a
            // new game reads them (every game saved by the game itself has them)
            let pack = THEME_PACKS.get(self.theme_pack).copied().unwrap_or("Standard").replace(' ', "_");
            self.club.roster = sg_core::roster::load(&self.game_dir, &pack);
        }
        // (the scratch copy keeps the portraits' heads as saved, which are the live game's)
        if let Some(g) = g.as_deref_mut() {
            self.register_portraits(g);
        }
        let tutorial = self.club.stories.tutorial;
        self.load_story();
        self.club.stories.tutorial = tutorial;
        self.course = s.course;
        self.econ = s.econ;
        self.econ.extras = self.club.extras;
        self.exe_rng = s.exe_rng;
        if let Some(r) = s.deco_rng {
            self.deco_rng = r;
        }
        self.noise = s.noise;
        self.employees = s.employees;
        self.staff_tiles = s.staff_tiles;
        self.game_tick = s.game_tick;
        self.hole_numbers.clear();
        // As the original's load: a hole being built is planned again here, with the generator as loaded, even when the save
        // has it planned (game flag LAYOUT, the landings and the hole record are all in the save). The exe's load 0x40b9b0
        // restores the flag word with 0x40000 as saved (0x40bae7..0x40bb08) but then runs 0x42f7a0, which clears 0x40000
        // (0x42f929), and the landing table 0x542dfc is not a saved block (docs/DECODE_SAVE.md section 1). Keeping the plan
        // would not match the original.
        self.plan_made_on = None;
        self.skip_plan = std::mem::take(&mut self.keep_plan_on_load);
        self.sync_course();
        self.skip_plan = false;
        self.hole_stats = s.hole_stats;
        // the Weather that matters extra's day as the save left it (sg_core::weather; nothing with the extra off)
        self.club.place = self.weather_place();
        self.club.weather_restore();
        self.sim_time = 0.0;
        self.tick_acc = 0.0;
        self.reset_clock = true;
        self.screen = Screen::Play;
        // the exe's load (0x40b9b0) sets the dock mode to 0: the Build Course panel opens, no tool armed
        self.panel = 1;
        self.pstate.alt = false;
        self.edit = false;
        self.dirty = true;
        if g.is_some() {
            self.restore_game_scripts(s.goals, &s.scripts);
        } else {
            // the scratch copy runs no scripts: the self-check compares their saved state with the live game's itself
            self.scen.goals = s.goals;
        }
        self.xs = crate::extras::State { kept: s.extras, ..Default::default() };
        // an undo in the course editor stays in the editor: free building, no scripts (editor.rs)
        self.econ.free = self.editor.is_some();
        if self.editor.is_some() {
            self.scen.host = Default::default();
        }
        // a game brought over from the original's saved game says what came over (share.rs)
        if let Some(note) = imported.filter(|_| g.is_some()) {
            self.post_message(&note, 1, crate::message_ui::NOBODY);
        }
        Ok(())
    }

    /// Gives every person whose head is a character file's own portrait the custom head that portrait takes this session.
    pub fn register_portraits(&mut self, g: &mut Gfx) {
        for i in 0..self.club.roster.len() {
            let Some(rel) = self.club.roster[i].portrait.clone() else { continue };
            let female = self.club.roster[i].female();
            if let Some(h) = self.art.portrait_head(g, &self.game_path(&rel), female) {
                self.club.roster[i].head = Some(h);
            }
        }
    }

    /// Loads the theme pack's stories (every *.txt in its Themes folder, or the Standard folder's when it has none, listed in
    /// case-insensitive name order as the original's file system does), its celebrities and its pro golfers.
    pub fn load_story(&mut self) {
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
        let mut pros = read("progolfers.dta").map(|t| sg_core::vips::parse_pros(&t)).unwrap_or_default();
        // the pack's own pro files join the table (0x4659a0: Themes\<pack>\*.pro in directory order, which the port takes
        // as case-insensitive name order)
        let mut pro_files: Vec<PathBuf> = sg_core::fsutil::list_dir(self.game_path(&format!("Themes/{pack}")))
            .into_iter()
            .filter(|p| sg_core::fsutil::ext_lower(p) == ".pro")
            .collect();
        pro_files.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
        let parsed: Vec<_> =
            pro_files.iter().filter_map(sg_core::fsutil::read_file).filter_map(|b| sg_core::championship::parse_pro(&b)).collect();
        sg_core::vips::append_pro_files(&mut pros, &parsed);
        self.club.celebrities = celebs;
        self.club.pros = pros;
    }

    // ---- course editing ----------------------------------------------------------------------------------------------------

    pub fn edit_paint(&mut self, tx: i32, ty: i32) {
        let r = self.brush;
        let mut tiles = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + r {
                    tiles.push((tx + dx, ty + dy));
                }
            }
        }
        self.paint_tiles(&tiles);
    }

    /// The terrain brush on each of these tiles in turn, as one stroke (one sound, one message), each tile checked and charged
    /// as the exe's paint does it; the brush's own tiles, or the port's rectangle and fill (build_tools.rs). The wetlands'
    /// warning sounds once a stroke.
    pub fn paint_tiles(&mut self, tiles: &[(i32, i32)]) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            let a = self.paint_action(tiles.to_vec());
            self.act(a);
            return;
        }
        let pe = &PAINT[self.paint_idx];
        // the green brush's byte: a plain or a tricky green (Tab)
        // and the sand trap's: its shape, the one the pointer shows (issue #104)
        let want = if pe.ty == 1 {
            self.paint_brush_variant();
            self.green_brush_byte()
        } else if pe.ty == 7 {
            self.sand_brush_byte()
        } else {
            pe.vbyte
        };
        self.ensure_land();
        if let Some(land) = self.land.as_mut() {
            land.sync_from_terrain(&self.terrain);
        }
        let tournament = self.club.game & golf::game::TOURNAMENT != 0;
        // the Multiple tees extra's Tees tool stepped to a forward or middle tee (tees.rs; the hole's own tee otherwise)
        let extra_tee = self.tee_tool();
        let mut tee_said: Option<String> = None;
        let mut refused = false;
        let mut broke = false;
        let mut warned = false;
        // an object, flower bed or the scenery of a retyped tile went: the objects are drawn again (#136: what was painted
        // over stayed on the screen)
        let mut scenery = false;
        let retyped = self.retyped.len();
        for &(x, y) in tiles {
            if !self.terrain.inside(x, y) {
                continue;
            }
            // the exe's checks before a tile is painted (0x41fee2): on the property, affordable, not a permanent
            // obstacle or a landmark, no tee or cup taken away during a tournament
            let old = self.terrain.ty[self.terrain.tile_index(x, y)];
            let new = pe.ty as u8;
            let li = (x * land::mn() + y) as usize;
            let (in_play, obstacle, on) = match self.land.as_ref() {
                Some(l) => (l.in_play(x, y), l.flags[li] & land::flag::OBSTACLE != 0, l.object_on(x, y)),
                None => (true, false, None),
            };
            let clear = Economy::terrain_clear_units(old as i32);
            let cost = clear + Economy::terrain_cost_units(pe.ty);
            if !in_play || obstacle {
                refused = true;
                continue;
            }
            if clear >= 50 && !warned {
                warned = true;
                self.ui_sound(24); // a warning only: wetlands and marsh are dear to clear
            }
            // nothing to change, nothing charged: the exe leaves at the same-type exit (0x4205ef), and only the green and sand
            // trap brushes set the tile's byte before it (below); water keeps its depth there, so painting water over the
            // sea no longer turns it shallow and light (#132). The course editor's water brushes set the depth they name.
            let byte_only = new == 1 || new == 7 || self.editor.is_some();
            // a flower bed or a garden border on the tile goes, even under the same type (0x42057b, before that exit)
            if let Some(l) = self.land.as_mut() {
                if l.flags[li] & 0x1000 != 0 {
                    l.flags[li] &= !0x1000;
                    scenery = true;
                }
            }
            if old == new && (!byte_only || want == self.terrain.variation[self.terrain.tile_index(x, y)] as i32) {
                continue;
            }
            let cup = self.course.flags.get(sg_core::course::idx(x, y)).is_some_and(|&f| f & sg_core::course::f::CUP != 0);
            if tournament && (old == 0 || cup) {
                refused = true;
                continue;
            }
            if !self.econ.affordable(cost as f64 * Economy::UNIT, self.hole_numbers.len()) {
                broke = true;
                continue;
            }
            if let Some(o) = on {
                let obj = self.land.as_ref().map(|l| l.objects[o]).unwrap_or_default();
                let edge = sg_core::data::buildings().get(obj.kind as usize).map(|b| b.1).unwrap_or(1);
                if (obj.kind == land::K_LANDMARK && obj.sub <= 16) || obj.kind == land::K_CLUBHOUSE || edge > 1 {
                    // landmarks are permanent; for a building the exe first asks whether to demolish it (the question
                    // is not drawn yet: such tiles are left alone)
                    refused = true;
                    continue;
                }
                scenery = true;
                if let Some(l) = self.land.as_mut() {
                    let size = l.footprint_size(&obj);
                    l.remove_object(o);
                    l.write_area(&mut self.terrain, obj.a, obj.b, obj.a + size - 1, obj.b + size - 1);
                }
            }
            // tees and greens go through the hole tool's bookkeeping (the hole being built gets its tee or cup)
            let mut extra_hole = None;
            if new == 0 && extra_tee != sg_core::extras::Tee::Back {
                // a forward or middle tee of the hole it is in front of
                // (on the course the A/B switch shows: sg_core::second)
                match self.on_hole_course(|a| a.club.place_extra_tee(&mut a.course, extra_tee, x, y, old)) {
                    Ok(h) => {
                        extra_hole = Some(h);
                        let course_b = self.hole_b();
                        let yards = self.yards_to_cup(course_b, h, (x, y));
                        let (hole, tee) = (self.hole_text(course_b, h as i32), tr(extra_tee.key()));
                        tee_said = Some(trf("STR_TOAST_TEE_PLACED", &[("HOLE", &hole), ("TEE", &tee), ("YARDS", &yards)]));
                    }
                    Err(e) => {
                        // a tee placed in the same stroke says so rather than the refusals of the brush's other tiles
                        if tee_said.is_none() {
                            tee_said = self.tee_refusal_text(extra_tee, e);
                        }
                        refused = true;
                        continue;
                    }
                }
            } else if old <= 1 || new <= 1 {
                let green_next = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(ex, ey)| self.green_joins(x + ex, y + ey));
                if !self.on_hole_course(|a| a.club.paint_hole_tile(&mut a.course, x, y, old, new, green_next)) {
                    // the hole being built has its tee: with the extra on, Tab is how to add another to a hole
                    if new == 0 && self.club.extras.tees && tee_said.is_none() {
                        tee_said = Some(tr("STR_TOAST_TEE_TAB").to_string());
                    }
                    continue;
                }
            }
            // the green brush over a green only turns it plain or tricky: the exe sets the variant byte at 0x420561 and
            // leaves before the charge and the new-tile bookkeeping, since the type stays the same
            // a sand trap over sand turns it the same way, free (0x42059d writes the byte before the same-type exit at
            // 0x4205ef), unless the tile's flag 0x200 keeps its byte
            let keep = new == 7 && self.land.as_ref().is_some_and(|l| l.flags[li] & 0x200 != 0);
            if old == new && (new == 1 || new == 7) {
                if !keep {
                    let ti = self.terrain.tile_index(x, y);
                    self.terrain.variation[ti] = want as u8;
                    if let Some(l) = self.land.as_mut() {
                        l.var[li] = want as u8;
                    }
                }
                continue;
            }
            // a new tee is drawn with the round mark (0x420760 clears its variant byte) until its hole opens (tee_arrows)
            let vb = match self.land.as_ref() {
                _ if pe.ty == 0 => 0,
                Some(l) if keep => l.var[li] as i32,
                _ => want,
            };
            let cash = self.econ.cash;
            self.econ.spend_to(economy::LEDGER_BUILD_COURSE, cost as f64 * Economy::UNIT);
            self.float_change(cash, (x, y));
            let hb = extra_hole.unwrap_or(self.club.course_next_hole(self.hole_b()) as usize).min(18);
            if !self.econ.free {
                if let Some(h) = self.club.course_hole_mut(self.hole_b(), hb) {
                    h.build_cost += cost;
                }
            }
            if old > 1 {
                if let Some(l) = self.land.as_mut() {
                    l.record_paint(x, y, old, cost);
                }
            }
            let changed = old as i32 != pe.ty;
            self.terrain.paint(x, y, pe.ty, vb);
            if changed {
                self.painted_tile(x, y, pe.ty as u8);
            }
        }
        if broke {
            self.ui_sound(24);
            self.show_toast(tr("STR_TOAST_NOT_ENOUGH_MONEY"));
        } else if refused {
            self.ui_sound(24);
        }
        if let Some(t) = tee_said.filter(|_| !broke) {
            self.show_toast(&t);
        }
        // one "already has its tee" message per stroke, not per tile
        self.club.out.dedup();
        self.dirty = true;
        if scenery || self.retyped.len() != retyped {
            if let Some(l) = self.land.as_mut() {
                l.sync_from_terrain(&self.terrain);
            }
            self.props.retain(|p| !p.object);
            self.add_land_objects();
        }
    }

    /// Opens the land screen (after the County Commissioner approves an expansion and the player says yes).
    pub fn open_land_screen(&mut self) {
        // the original rolls the prices with the game's generator: an action (replay.rs)
        if self.rec.depth_zero() {
            self.act(sg_core::action::Action::LandScreen);
            return;
        }
        self.ensure_land();
        self.ui_sound(56);
        let Some(land) = self.land.as_ref() else { return };
        self.tracts = sg_core::tracts::roll(land, self.club.purchases, &mut self.exe_rng);
        self.land_hover = -1;
        self.screen = Screen::Land;
    }

    /// Pointer over the land screen (virtual 800 x 600 coordinates): which tract or the cancel button. The exe redraws on
    /// every change and rolls the prices again each time (keeping the first ones), which draws from the generator.
    pub fn land_pointer(&mut self, x: f32, y: f32) {
        let hit = self.land_hit(x, y);
        if hit != self.land_hover {
            self.act(sg_core::action::Action::LandHover(hit));
        }
    }

    /// The commissioner's question answered No (or closed), or the land screen left without buying: with the Keep the land
    /// offer extra the approval waits (`Club::land_kept`), and a toast says where to use it.
    pub fn land_put_off(&mut self) {
        if self.club.extras.keep_land && self.club.land_kept {
            self.show_toast(tr("STR_TOAST_LAND_KEPT"));
        }
    }

    /// The Information menu's Buy land line (the Keep the land offer extra): shown while an approval waits.
    pub fn land_waiting(&self) -> bool {
        self.club.extras.keep_land && self.club.land_kept
    }

    /// A click on the land screen: buy the tract under the pointer, or leave on the cancel button or a right click.
    pub fn land_click(&mut self, right: bool) {
        if right || self.land_hover == 9 {
            self.screen = Screen::Play;
            self.land_put_off();
            return;
        }
        if !(0..9).contains(&self.land_hover) {
            self.ui_sound(24);
            return;
        }
        self.act(sg_core::action::Action::Tract { tract: self.land_hover as usize });
    }

    /// Buys tract `i` (0..8) at the price the land screen shows: the Tract action (replay.rs).
    pub(crate) fn buy_tract(&mut self, i: usize) -> bool {
        let Some(price) = self.tracts.get(i).map(|t| t.price) else { return false };
        if price == 0 {
            return false;
        }
        let cost = self.club.extras.land(price as f64 * Economy::UNIT);
        if !self.econ.affordable(cost, self.hole_numbers.len()) {
            self.show_toast(&trf(
                "STR_TOAST_CHANGE_COSTS",
                &[("COST", &crate::ui::money(cost as i64)), ("CASH", &crate::ui::money(self.econ.cash as i64))],
            ));
            return false;
        }
        self.econ.spend_to(economy::LEDGER_OTHER, cost);
        if let Some(land) = self.land.as_mut() {
            sg_core::tracts::buy(land, i);
            self.club.log_event(sg_core::records::log::LAND, i as i32);
            // the terrain the renderer and editor use follows
            // the whole tract, which is larger than the original's 16 tiles on a larger course (issue #54)
            let (a0, b0) = sg_core::tracts::origin(i);
            let side = sg_core::tracts::side();
            for a in a0..a0 + side {
                for b in b0..b0 + side {
                    let t = (a * land::mn() + b) as usize;
                    let o = self.terrain.tile_index(a, b);
                    if self.terrain.ty[o] == land::T_OUT {
                        self.terrain.ty[o] = land.ty[t];
                        self.terrain.variation[o] = if land.ty[t] == land::T_WATER { land.var[t] } else { 0 };
                    }
                }
            }
        }
        self.club.purchases += 1;
        // the approval the Keep the land offer extra kept is used
        self.club.land_kept = false;
        println!("bought tract {} for {}", i + 1, crate::ui::money(cost as i64));
        self.screen = Screen::Play;
        self.dirty = true;
        self.refresh_trees();
        true
    }

    /// Arms terrain brush `k` (PAINT): picking a brush, from its button or its key, rolls its variant at once, a random 0..2
    /// (0x41eac4 `rand(3)` into 0x5a34f0), the green's 0 (0x41eade), so the shape the pointer shows is the one painted and
    /// Tab turns it from the start (issue #104: the roll waited for the first tile, so the first click changed the shape and
    /// Tabs before it were lost).
    pub fn pick_brush(&mut self, k: usize) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            self.act(Action::PickBrush { ty: PAINT[k].ty, vbyte: PAINT[k].vbyte });
            return;
        }
        self.tool = 0;
        self.paint_idx = k;
        self.edit = true;
        let v = if PAINT[k].ty == 1 { 0 } else { self.exe_rng.below(3) };
        self.paint_variant = Some((k, v));
    }

    /// The variant byte a sand trap tile takes from the brush (0x42059d: variant % 12 + 12, written on every tile the brush
    /// touches while its flag 0x200 is clear); Terrain.dll draws SandBunker1..4 by its low two bits, turned by the view.
    pub(crate) fn sand_brush_byte(&mut self) -> i32 {
        self.paint_brush_variant().rem_euclid(12) + 12
    }

    /// The brush's variant (0x5a34f0): rolled when the brush is picked (`pick_brush`), or when first used by a test hook that
    /// armed it directly (the green always starts at 0, 0x41eaa0), one up with each Tab.
    fn paint_brush_variant(&mut self) -> i32 {
        match self.paint_variant {
            Some((k, v)) if k == self.paint_idx => v,
            _ => {
                let v = if PAINT[self.paint_idx].ty == 1 { 0 } else { self.exe_rng.below(3) };
                self.paint_variant = Some((self.paint_idx, v));
                v
            }
        }
    }

    /// The variant byte the green brush gives a green (0x420561): 0xff for the tricky green (an odd variant, Tab), else 0.
    /// Terrain.dll draws a green whose byte has bit 0x80 with the TrickyGreen textures, and the golfers read the same bit.
    /// Whether tile (x, y) is a green that a green painted beside it joins, for the hole tool's rule that a green next to
    /// another starts no hole without a tee (0x41fee2). The exe counts a putting green's tiles too, so a green painted against
    /// one got no flag (#128); with "Fix original bugs" a putting green is a building of its own and is not counted.
    fn green_joins(&self, x: i32, y: i32) -> bool {
        self.terrain.inside(x, y)
            && self.terrain.ty[self.terrain.tile_index(x, y)] == 1
            && !(crate::enhanced::fix_bugs() && self.terrain.on_putting_green(x, y))
    }

    pub(crate) fn green_brush_byte(&self) -> i32 {
        let v = match self.paint_variant {
            Some((k, v)) if k == self.paint_idx => v,
            _ => 0,
        };
        if v & 1 != 0 {
            0xff
        } else {
            0
        }
    }

    /// The exe's bookkeeping for a tile painted to a new type (0x420561..): weeds go, the tile starts its growth counter, and
    /// its variant byte becomes the brush's variant (tees 0, greens 0 or 0xff), which sizes a planted tree.
    fn painted_tile(&mut self, x: i32, y: i32, new: u8) {
        let v = self.paint_brush_variant();
        self.ensure_land();
        self.retyped.push((x, y));
        if let Some(land) = self.land.as_mut() {
            if (0..land::mn()).contains(&x) && (0..land::mn()).contains(&y) {
                let i = (x * land::mn() + y) as usize;
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
        self.paint_sound(new, x, y);
        if let Some((slot, _)) = sg_core::decor::tree_sound(new) {
            self.ui_sound(slot);
        }
        // a planted tree grows from its first frame (update_tree_growth)
        if (13..=15).contains(&new) {
            self.start_sprout(x, y);
        } else {
            self.sprouts.remove(&(x, y));
        }
        // the Random trees extra (OpenSGolf extras): the next tree this brush plants takes a random look of its kind, any
        // of the variant byte's twelve (v % 3 the size, (v / 3) & 1 a pine's species; decor::trees), drawn from the
        // looks' own generator so the game's is untouched; the brush's look under the pointer shows it
        if crate::extras::random_trees() && (13..=15).contains(&new) {
            self.paint_variant = Some((self.paint_idx, self.look_rng.below(12)));
        }
    }

    pub fn edit_path(&mut self, tx: i32, ty: i32, remove: bool) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            self.act(Action::Path { x: tx, y: ty, brush: self.brush, remove, style: self.path_kind });
            return;
        }
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
        let holes = self.hole_numbers.len();
        let variant = self.item_design(kind);
        let Some(land) = self.land.as_mut() else { return false };
        land.sync_from_terrain(&self.terrain);
        let cost = match land.tile_item_cost(x, y, kind, theme) {
            Ok(c) => c,
            Err(m) => {
                if !m.is_empty() {
                    self.show_toast(m);
                }
                self.ui_sound(24);
                return false;
            }
        };
        let amount = cost as f64 * Economy::UNIT;
        if !self.econ.affordable(amount, holes) {
            self.show_toast(&trf("STR_TOAST_CHANGE_COSTS", &[("COST", &money(amount as i64)), ("CASH", &money(self.econ.cash as i64))]));
            self.ui_sound(24);
            return false;
        }
        let Some(land) = self.land.as_mut() else { return false };
        let new_bed = land.flags.get((x * land::mn() + y) as usize).is_some_and(|f| f & 0x1000 == 0);
        if land.place_tile_item(x, y, kind, variant, theme).is_err() {
            return false;
        }
        // a scenic tree and a new flower bed grow from their first frame (the building tool sets flag 0x4000 and the
        // counter to 0; update_tree_growth)
        if kind == land::K_WILLOW || (kind == land::K_FLOWERS && new_bed) {
            self.start_sprout(x, y);
        }
        let Some(land) = self.land.as_mut() else { return false };
        let i = self.terrain.tile_index(x, y);
        if (kind == land::K_PATH || kind == land::K_BENCH || kind == land::K_BRIDGE) && self.terrain.path_kind[i] == 0 {
            self.terrain.path_kind[i] = self.path_kind.max(1) as u8;
        }
        land.write_area(&mut self.terrain, x, y, x, y);
        if cost != 0 {
            let cash = self.econ.cash;
            self.econ.spend_to(economy::LEDGER_FACILITIES, amount);
            self.float_change(cash, (x, y));
        }
        match kind {
            land::K_BENCH => self.ui_sound(262),
            land::K_FLOWERS => self.ui_sound(148),
            land::K_BRIDGE => self.ui_sound(263),
            // the scenic tree plays the broadleaf's planting sound as it starts growing (0x412c9c: slot 0x93, volume 100)
            land::K_WILLOW => self.ui_sound(0x93),
            _ => {}
        }
        // the Random trees extra (OpenSGolf extras): the next scenic tree is a random one of the seven, as picking the
        // tool chooses one (0x434ac0); the strip and the look under the pointer show it
        if kind == land::K_WILLOW && crate::extras::random_trees() {
            self.arm_design(kind);
        }
        self.after_object_change();
        true
    }

    /// Picking a garden amenity sets its design as the exe's amenity click does (0x434ac0): the first bench, a flower bed of
    /// the first shape in one of its first four colours, one of the seven scenic trees or eight bridges at random (the
    /// generator at 0x822d9c), and no landmark until one is chosen from the strip.
    pub fn arm_design(&mut self, kind: i32) {
        let v = match kind {
            land::K_BENCH => 0,
            land::K_FLOWERS => self.look_rng.below(4),
            land::K_WILLOW => self.look_rng.below(7),
            land::K_BRIDGE => self.look_rng.below(8),
            _ => -1,
        };
        self.design = (v >= 0).then_some((kind, v));
        self.design_hover = v;
    }

    /// The design of the next tile item of a kind (bench 0..4, flowers 0..14, willow 0..6, bridge 0..7); the amenity's first
    /// design when the tool was armed some other way.
    pub fn item_design(&mut self, kind: i32) -> i32 {
        match self.design {
            Some((k, v)) if k == kind => v,
            _ => {
                self.arm_design(kind);
                self.design.map(|d| d.1).unwrap_or(0)
            }
        }
    }

    /// The landmark the landmark tool places: the one chosen from the strip, while the club still owns it.
    pub fn chosen_landmark(&self) -> Option<i32> {
        match self.design {
            Some((k, t)) if k == land::K_LANDMARK && self.landmark_allowed(t) => Some(t),
            _ => None,
        }
    }

    /// A right click on a course tile, as the main frame takes it (0x41f1bc): the first names what an undo there would do,
    /// over the tile's middle in green for 48 frames (0x40a160, then 0x40c720 with 0x23e8; any words already there go,
    /// 0x40c860); a second right click on the same tile, however much later, undoes it (0x40a4e0) and the words go.
    pub fn right_click_undo(&mut self, a: i32, b: i32) {
        let at = (a * 1024 + 512, b * 1024 + 512);
        self.map_labels.retain(|l| (l.0, l.1) != at);
        if self.undo_mark == Some((a, b)) {
            self.undo_mark = None;
            // one step of the port's own Ctrl+Z (undo.rs), as a click on the course is
            self.undo_press();
            self.undo_tile(a, b);
            self.undo_release();
            return;
        }
        self.ensure_land();
        let Some(land) = self.land.as_mut() else { return };
        land.sync_from_terrain(&self.terrain);
        let (text, _) = land.undo_label(a, b);
        self.map_label(at, text);
        self.undo_mark = Some((a, b));
    }

    /// Words over a map point for 48 of the exe's frames (0x40c720: a ring of 8).
    pub fn map_label(&mut self, at: (i32, i32), text: String) {
        let now = self.clock;
        self.map_labels.retain(|l| l.3 > now);
        if self.map_labels.len() >= 8 {
            self.map_labels.remove(0);
        }
        self.map_labels.push((at.0, at.1, text, now + 0x30 as f64 * sg_core::flight::TICK_MS as f64 / 1000.0));
    }

    /// Undo on a tile, as the exe's second right-click does: an item comes off with exactly what it cost back (booked to Build
    /// course), a building is demolished with no refund. The money back (or a home site's price to buy its lot back) floats
    /// over the tile.
    pub fn undo_tile(&mut self, x: i32, y: i32) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            self.act(Action::TakeBack { x, y });
            return;
        }
        let cash = self.econ.cash;
        self.take_back_tile(x, y);
        self.float_change(cash, (x, y));
    }

    fn take_back_tile(&mut self, x: i32, y: i32) {
        // a lamp post of the Path styles and lamps extra comes off first (paths_lamps.rs)
        if self.remove_lamp(x, y) {
            return;
        }
        self.ensure_land();
        let Some(land) = self.land.as_mut() else { return };
        land.sync_from_terrain(&self.terrain);
        let li = (x * land::mn() + y) as usize;
        let old = land.ty.get(li).copied().unwrap_or(land::T_ROUGH);
        // a hole's tee or cup green undone is the same as painting over it (0x41fee2): refused during a tournament
        let cup =
            old == land::T_GREEN && self.course.flags.get(sg_core::course::idx(x, y)).is_some_and(|&f| f & sg_core::course::f::CUP != 0);
        let hole_tile = old == land::T_TEE || cup;
        if hole_tile
            && self.club.game & golf::game::TOURNAMENT != 0
            && land.undo.get(li).is_some_and(|&c| c < 0x7d || (c != land::UNDO_NONE && c & 0x7f == 0x10))
        {
            return self.ui_sound(24);
        }
        let undone = land.undo_tile(x, y);
        let new = land.ty.get(li).copied().unwrap_or(old);
        if matches!(undone, land::Undone::Refund(_)) && new != old && (old <= land::T_GREEN || new <= land::T_GREEN) {
            // taking a tee or a green away goes through the hole tool's bookkeeping, as painting over it does: the hole it
            // belonged to is out of play (par 0) until it is rebuilt
            let green_next = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(ex, ey)| self.green_joins(x + ex, y + ey));
            self.on_hole_course(|a| a.club.paint_hole_tile(&mut a.course, x, y, old, new, green_next));
        }
        let Some(land) = self.land.as_mut() else { return };
        match undone {
            land::Undone::Refused => self.ui_sound(24),
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
                    // the home-site details' record of it (homes_ui.rs)
                    self.home_removed(o.a, o.b);
                    // buying the lot back (0x40e400): its smoothed value / 50 plus half its value now
                    let c = o.val / 50 + sg_core::homes::lot_value(&self.course, &self.club.holes, self.difficulty, o.a, o.b) / 2;
                    self.econ.spend_to(economy::LEDGER_HOME_SITES, c as f64 * Economy::UNIT);
                }
                self.after_object_change();
                self.show_toast(tr("STR_TOAST_BUILDING_DEMOLISHED"));
                self.snd("Interface/Building.wav", 0.7, false);
            }
            land::Undone::Refund(r) => {
                let i = self.terrain.tile_index(x, y);
                let f = land.flags[(x * land::mn() + y) as usize];
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
    /// and moves it there, which needs a Country Club (10 holes); the Snack Bar has no levels, so any number may stand. It only earns money once a path joins it to the clubhouse.
    /// Benches, flower beds, willows and bridges are tile items (see place_tile_item). Removing goes through undo: items give
    /// back what they cost, buildings are demolished with no refund.
    pub fn edit_building(&mut self, tx: i32, ty: i32, remove: bool) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            let a = self.build_action(tx, ty, remove);
            self.act(a);
            return;
        }
        if !self.terrain.inside(tx, ty) {
            return;
        }
        self.ensure_land();
        let theme = self.exe_theme();
        let holes = self.hole_numbers.len();
        if remove {
            return self.undo_tile(tx, ty);
        }
        let kind = self.build_idx as i32;
        if kind == land::K_LANDMARK && self.chosen_landmark().is_none() {
            // a click on the course before a landmark is chosen puts the tool down (0x41fc7c)
            self.edit = false;
            return;
        }
        // the Path styles and lamps extra's lamp post (paths_lamps.rs)
        if self.chosen_landmark().is_some_and(|t| kind == land::K_LANDMARK && t == sg_core::extras::LAMP_TYPE) {
            return self.place_lamp(tx, ty);
        }
        // the Buyable animals extra's page of the landmark strip
        if let Some(t) = self.chosen_landmark().filter(|&t| kind == land::K_LANDMARK && t >= sg_core::objects::ANIMAL_BASE) {
            return self.buy_animal(tx, ty, t);
        }
        if matches!(kind, land::K_PATH | land::K_BENCH | land::K_FLOWERS | land::K_WILLOW | land::K_BRIDGE) {
            self.place_tile_item(tx, ty, kind);
            return;
        }
        let Some(land) = self.land.as_mut() else { return };
        land.sync_from_terrain(&self.terrain);
        if !self.build_available(kind as usize) {
            return self.show_toast(tr("STR_TOAST_BUILD_MORE_HOLES"));
        }
        let Some(land) = self.land.as_mut() else { return };
        // a Snack Bar never upgrades the one standing: any number may be built (land::kind_has_levels, 0x41f926)
        let existing = if sg_core::land::kind_has_levels(kind) { land.objects.iter().position(|o| o.kind == kind) } else { None };
        if existing.is_some() && economy::rank(holes) < 2 {
            self.ui_sound(24);
            return self.show_toast(tr("STR_TOAST_NO_UPGRADES_YET"));
        }
        if existing.is_some_and(|i| land.objects[i].sub >= 1) {
            self.ui_sound(24);
            return self.show_toast(tr("STR_TOAST_NO_FURTHER_UPGRADE"));
        }
        // an upgrade is placed one tile larger
        let size = sg_core::data::buildings()[kind as usize].1 + existing.is_some() as i32;
        let Some(mut clear) = land.fits(tx, ty, size, kind, theme) else {
            return self.show_toast(tr("STR_TOAST_NO_ROOM"));
        };
        // "Fix original bugs" (docs/FIXES.md 2): a new lot never goes on top of a standing one (the original charged again)
        if existing.is_none() && crate::enhanced::fix_bugs() && land.covers_object(tx, ty, size) {
            return self.show_toast(tr("STR_TOAST_NO_ROOM"));
        }
        if kind == 0 {
            clear /= 2;
        }
        let level = existing.map(|i| land.objects[i].sub + 1).unwrap_or(0);
        let mut cost = (sg_core::data::buildings()[kind as usize].2 * (level + 2) / 2 + clear) as f64 * Economy::UNIT;
        // a landmark: the one chosen from the strip; (type * 5 + 25) * 2, or nothing the first time a donated type is placed
        // (clearing is not charged)
        let landmark = self.chosen_landmark();
        if kind == land::K_LANDMARK {
            let Some(t) = landmark else { return };
            cost = if t >= sg_core::objects::CUT_BASE {
                // a cut landmark (the Cut content extra): the port's price. Tiles keep an object's index in a byte, so the
                // extra never takes the object table past 255 records in use (the exe's placement, 0x40e000, stops at 0xff).
                if self.land.as_ref().is_some_and(|l| l.objects.iter().filter(|o| o.kind >= 0).count() >= 255) {
                    self.ui_sound(24);
                    return self.show_toast(tr("STR_TOAST_NO_ROOM"));
                }
                App::extra_landmark_units(t) as f64 * Economy::UNIT
            } else if self.club.free_landmarks & (1 << t) != 0 {
                0.0
            } else {
                ((t * 5 + 25) * 2) as f64 * Economy::UNIT
            };
        }
        if !self.econ.affordable(cost, holes) {
            return self
                .show_toast(&trf("STR_TOAST_CHANGE_COSTS", &[("COST", &money(cost as i64)), ("CASH", &money(self.econ.cash as i64))]));
        }
        let lot = if kind == land::K_HOME_SITE {
            let v = sg_core::homes::lot_value(&self.course, &self.club.holes, self.difficulty, tx, ty);
            if v < 50 {
                return self.show_toast(tr("STR_TOAST_LOT_NOT_ATTRACTIVE"));
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
        let n = land.place_sized(&mut self.exe_rng, tx, ty, kind, 0, theme, existing.is_some() as i32);
        if kind >= 6 && existing.is_none() {
            // the exe logs the first building of each kind
            self.club.log_event(sg_core::records::log::BUILT, kind);
        }
        land.objects[n].sub = level;
        land.objects[n].dir = (self.turn & 3) as u8;
        if kind == land::K_LANDMARK {
            let t = landmark.unwrap_or(0);
            land.objects[n].sub = t;
            self.club.free_landmarks &= !(1 << t);
        }
        land.write_area(&mut self.terrain, tx, ty, tx + size - 1, ty + size - 1);
        let cash = self.econ.cash;
        self.econ.spend_to(economy::LEDGER_FACILITIES, cost);
        if lot > 0 {
            // the lot is sold at once
            self.econ.earn_to(economy::LEDGER_HOME_SITES, lot as f64 * Economy::UNIT);
            // OpenSGolf extras (homes_ui.rs): what the home site cost and sold for
            self.home_placed(tx, ty, cost.round() as i64, (lot as f64 * Economy::UNIT) as i64);
        }
        // what it cost, less a home site's lot sold, floats over its middle tile (docs/DECODE_HOMES.md)
        self.float_change(cash, (tx + (size - 1) / 2, ty + (size - 1) / 2));
        self.after_object_change();
        self.snd("Interface/Building.wav", 0.7, false);
        let kind = sg_core::data::object_id(kind as usize).to_string();
        self.script_hook(sg_core::script::Hook::BuildingPlaced { kind, x: tx, y: ty });
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
        // a course without land (the demo course, a course file) is played at the original's size, or at its own edge when
        // it is a square larger than that (a course saved from a larger course size)
        let (w, h) = (self.terrain.w, self.terrain.h);
        sg_core::mapsize::set(if w == h && w > sg_core::mapsize::ORIGINAL { w } else { sg_core::mapsize::ORIGINAL });
        let mut land = Land::from_terrain(&self.terrain, self.exe_theme());
        if self.terrain.clubhouse_x >= 0 {
            let (a, b) = (self.terrain.clubhouse_x - 2, self.terrain.clubhouse_y - 2);
            land.objects.push(land::Object { kind: land::K_CLUBHOUSE, a, b, dir: 0, flags: 0x40, sub: 0, val: 0 });
            land.clubhouse = (a, b);
            for r in 0..4 {
                for c in 0..4 {
                    let (ta, tb) = (a + r, b + c);
                    if (0..land::mn()).contains(&ta) && (0..land::mn()).contains(&tb) {
                        let i = (ta * land::mn() + tb) as usize;
                        land.flags[i] |= land::flag::FOOTPRINT;
                        land.var[i] = 0;
                    }
                }
            }
        }
        self.land = Some(land);
    }

    pub fn edit_raise(&mut self, cx: i32, cy: i32, delta: i32) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            self.act(Action::Raise { x: cx, y: cy, brush: self.brush, delta });
            return;
        }
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
        // a Shift+drag rectangle paints, and a drag to place's row is placed, when the button comes up (build_tools.rs)
        if !self.has_hit || (self.tool == 0 && self.bt.rect_from.is_some()) || self.bt.row_from.is_some() {
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
            4 => self.edit_building(a, b, lower),
            5 => self.undo_tile(a, b),
            _ if self.pstate.elev_tool == 3 && self.panel == 2 && self.pstate.alt => {
                if !dragging {
                    let t = self.terrain.tile_of(self.hit_x, self.hit_z);
                    self.start_analysis(Some(t), true);
                }
            }
            _ => {
                let down = lower != (self.raise_sign < 0);
                let delta = if down { -1 } else { 1 };
                // the Elevation panel's tools edit as the exe does; the keyboard's raise tool keeps the round brush
                if self.panel == 2 && self.pstate.alt && self.panel_art.ready() {
                    self.edit_elevation(a, b, delta)
                } else {
                    self.edit_raise(a, b, delta)
                }
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

    /// z / x and the dock's zoom buttons: the next of the exe's three levels (1, 2, 4) in or out.
    pub fn zoom_step(&mut self, inward: bool) {
        if crate::enhanced::smooth_camera() {
            // Enhanced: from a zoom between levels, the next of the three levels that way
            let levels: Vec<f32> = (far_level()..=2).map(|l| ZOOM_UNIT * (l as f32).exp2()).collect();
            let z = self.zoom;
            self.zoom = if inward {
                levels.iter().copied().find(|&l| l > z * 1.001).unwrap_or(ZOOM_UNIT * 4.0)
            } else {
                levels.iter().rev().copied().find(|&l| l < z / 1.001).unwrap_or(levels[0])
            };
            return;
        }
        let far = far_level();
        let level = (self.zoom / ZOOM_UNIT).log2().round().clamp(far as f32, 2.0) as i32;
        let level = if inward { (level + 1).min(2) } else { (level - 1).max(far) };
        self.zoom = ZOOM_UNIT * (level as f32).exp2();
    }

    /// PageUp / Home and the Rotate Map buttons: a quarter turn of the view (the exe's rotation steps by 2 of 8).
    pub fn rotate_view(&mut self, quarters: i32) {
        self.rot = ((self.rot / 90.0).round() + quarters as f32).rem_euclid(4.0) * 90.0;
        self.turned(quarters as f32 * 90.0);
        // the sand traps' pictures turn with the view (Terrain.dll's phase)
        self.dirty |= self.terrain.ty.contains(&7);
        // so does the cliff round the property (only the sides facing the camera are built)
        self.dirty |= self.terrain.ty.contains(&20);
        // the flower beds pick their pieces by how their neighbours lie on the screen (decor::flower_bed), and landmark 3
        // turns twice as fast as the view
        if self.land.is_some() {
            self.props.retain(|p| !p.object);
            self.add_land_objects();
        }
    }

    /// The welcome for the course's theme (0x45fd80 at tick 0x20), with the course name and the starting cash in thousands,
    /// in the exe's words (its slips included).
    pub fn welcome_text(&self) -> String {
        let n = &self.course_name;
        let k = (self.econ.cash / Economy::UNIT) as i64 * 100 / 1000;
        match self.exe_theme() {
            1 => trf("STR_WELCOME_DESERT", &[("COURSE", &n), ("THOUSANDS", &k)]),
            2 => trf("STR_WELCOME_TROPICAL", &[("COURSE", &n)]),
            3 => trf("STR_WELCOME_LINKS", &[("COURSE", &n)]),
            _ => trf("STR_WELCOME_PARKLAND", &[("COURSE", &n), ("THOUSANDS", &k)]),
        }
    }

    /// Queues a floating amount at a map point for 24 ticks (0x40c890); none in a sandbox. The port's own: it also goes once
    /// 24 ticks' time has passed on the clock, so an amount shown while the game is paused (building) does not stay.
    pub fn float_money(&mut self, units: i32, at: (i32, i32)) {
        if units == 0 || self.econ.sandbox {
            return;
        }
        let until = self.clock + 0x18 as f64 * sg_core::flight::TICK_MS as f64 / 1000.0;
        self.floats[self.float_next] = (units, at.0, at.1, 0x18, until);
        self.float_next = (self.float_next + 1) % 8;
    }

    /// What a building or terrain action did to the money on one tile, floating over it in green (given back) or red (paid):
    /// `before` is the cash before it. Every tile painted, item or building placed and tile undone shows its own, as in
    /// footage of the original (each tile of a stroke, a path, a water tile undone); the exe also floats a home site's net
    /// cost at its tile (docs/DECODE_HOMES.md). Its callers of 0x40c890 for these are not read yet.
    pub fn float_change(&mut self, before: f64, tile: (i32, i32)) {
        let units = ((self.econ.cash - before) / Economy::UNIT).round() as i32;
        let at = (tile.0 * 1024 + 512, tile.1 * 1024 + 512);
        self.float_money(units, at);
        // a click's amounts float again, given back, when it is undone (undo.rs)
        self.undo.note_float(units, at);
    }

    pub fn pan(&mut self, right: f32, up: f32) {
        let yaw = (45.0 + self.rot) * std::f32::consts::PI / 180.0;
        self.cam_x += yaw.cos() * right + yaw.sin() * up;
        self.cam_z += yaw.sin() * right - yaw.cos() * up;
        self.clamp_cam();
    }

    /// The view's centre kept on the map, as the exe keeps it: after every move its tile is clamped to 0..49 on each axis
    /// (0x467130 from 0x41df51..0x41e249), whatever the zoom or the land owned (#138: the port's view could be scrolled away
    /// from the course and lost). On a larger course, its own edge.
    pub fn clamp_cam(&mut self) {
        if self.terrain.w <= 0 || self.terrain.h <= 0 {
            return;
        }
        let (lo, hi) = (self.terrain.tile_centre(0, 0), self.terrain.tile_centre(self.terrain.w - 1, self.terrain.h - 1));
        self.cam_x = self.cam_x.clamp(lo.0, hi.0);
        self.cam_z = self.cam_z.clamp(lo.1, hi.1);
    }
}

/// Original pitch angles, chosen per resolution in Terrain::initSystem (38.68, 40.54, 40.83 degrees).
/// The design tip five months in (0x45fd80 at tick 0x1400), a text key.
pub const DESIGN_TIP: &str = "STR_MSG_DESIGN_TIP";

/// Our zoom at the exe's zoom level 1 (a tile 16 pixels wide on the 800 x 600 screen); its levels are 1, 2 and 4.
pub const ZOOM_UNIT: f32 = 0.905 / 4.0;

/// The farthest zoom level as a power of two of ZOOM_UNIT: 0 (the exe's level 1) on the original's 50 x 50 map, where it
/// shows the whole course; a larger course size (sg_core::mapsize) gets a level or two further out (1/2 up to 100 tiles,
/// 1/4 beyond), so its whole map fits on the screen as the original's does.
pub fn far_level() -> i32 {
    let n = sg_core::mapsize::n().max(sg_core::mapsize::ORIGINAL);
    let mut level = 0;
    while sg_core::mapsize::ORIGINAL << -level < n {
        level -= 1;
    }
    level
}

/// The farthest zoom (ZOOM_UNIT on the original's map, see far_level).
pub fn far_zoom() -> f32 {
    ZOOM_UNIT * (far_level() as f32).exp2()
}

pub fn pitch_for(w: f32, h: f32) -> f64 {
    if w == 800.0 && h == 600.0 {
        38.682186
    } else if w == 1280.0 && h == 1024.0 {
        40.832218
    } else {
        40.541603
    }
}

/// SG_TICK_BENCH: the simulation steps' times, (steps, total seconds, slowest), printed by `tick_bench_report`.
static TICK_BENCH: std::sync::Mutex<(u64, f64, f64)> = std::sync::Mutex::new((0, 0.0, 0.0));

fn tick_bench_add(dt: f64) {
    if let Ok(mut b) = TICK_BENCH.lock() {
        b.0 += 1;
        b.1 += dt;
        b.2 = b.2.max(dt);
    }
}

/// Prints the simulation steps' mean and slowest time (SG_TICK_BENCH set; for measuring a course size's cost).
pub fn tick_bench_report(app: &App) {
    if std::env::var_os("SG_TICK_BENCH").is_none() {
        return;
    }
    let (n, total, max) = TICK_BENCH.lock().map(|b| *b).unwrap_or_default();
    let playing = app.club.g.iter().take(golf::SLOTS).filter(|g| g.hole > 0).count();
    let (size, mean, max, holes) = (sg_core::course::n(), total * 1000.0 / n.max(1) as f64, max * 1000.0, app.club.next_hole - 1);
    println!(
        "tick bench: map {size}x{size}, {n} steps, {mean:.3} ms a step, slowest {max:.2} ms, {playing} golfers on the course, {holes} holes"
    );
}

/// Milliseconds from the system clock, wrapped to 32 bits like the Windows tick count.
/// The staff's weed tiles into a course's tile flags and growth (`App::weeds_to_course`, at the start of every game tick).
pub fn weeds_into(t: &TileState, course: &mut Course) {
    if t.flags.len() != (t.w * t.h) as usize {
        return;
    }
    for a in 0..t.w.min(sg_core::course::n()) {
        for b in 0..t.h.min(sg_core::course::n()) {
            let si = (b * t.w + a) as usize;
            let ci = sg_core::course::idx(a, b);
            let bits = t.flags[si] & (staff::WEEDS | staff::WORKED);
            course.flags[ci] = (course.flags[ci] & !(staff::WEEDS | staff::WORKED)) | bits;
            course.growth[ci] = t.counter[si];
        }
    }
}

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
            for a in 0..land::mn() {
                for b in 0..land::mn() {
                    if l.flags[(a * land::mn() + b) as usize] & land::flag::CREEK != 0 {
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
        self.hire_employee(kind, false)
    }

    /// Hires an employee of kind 0..3, the experienced version (Celebrity, Marshall, Technician, Refresher) when `skilled`.
    pub fn hire_employee(&mut self, kind: usize, skilled: bool) -> bool {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            return self.act(Action::Hire { kind, skilled });
        }
        // the club's employee limit (the original's 20; the Hardcore and Higher staff limits extras change it)
        let hired = self.employees.iter().filter(|e| e.active && (-5..=-2).contains(&e.job)).count();
        if hired >= crate::panels_ui::max_employees(self.club.extras.staff_limit()) {
            return false;
        }
        if !self.econ.hire(kind) {
            return false;
        }
        let club = self.club_anchor();
        let i = staff::hire(&mut self.employees, kind, skilled, club, &mut self.exe_rng);
        self.employees[i].hired = (self.club.date() >> 10) as i32;
        true
    }

    /// Fires one employee (an index into `employees`), as the Employee panel's Fire button does: 25 units under Salaries.
    pub fn fire_employee(&mut self, i: usize) -> bool {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            return self.act(Action::Fire { employee: i });
        }
        let Some(e) = self.employees.get(i).filter(|e| e.active && (-5..=-2).contains(&e.job)) else { return false };
        if !self.econ.fire((-2 - e.job as i32) as usize) {
            return false;
        }
        self.employees[i].active = false;
        if self.moving_employee == Some(i) {
            self.moving_employee = None;
        }
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
        // the date moves with the game tick, or slower with the Slower seasons and Pause the calendar extras (sg_core::extras)
        let extras = self.club.extras;
        self.club.cal.step(self.game_tick, &extras);
        // no wages in a championship
        let staffed: Vec<usize> = (0..self.employees.len())
            .filter(|&i| {
                let e = &self.employees[i];
                e.active && e.job != staff::job::OWNER && !self.club.championship()
            })
            .collect();
        let payroll: Vec<economy::Payroll> = staffed
            .iter()
            .map(|&i| {
                let e = &self.employees[i];
                economy::Payroll { kind: (-2 - e.job as i32).clamp(0, 3) as usize, experienced: e.upgraded }
            })
            .collect();
        // (Course B's open holes are kept up too: sg_core::second)
        let holes = self.hole_numbers.len() + self.club.course_b_open();
        let paid = self.econ.on_tick_dated(self.game_tick, &self.club.cal, self.difficulty, holes, &payroll, &mut self.exe_rng);
        if self.club.cal.new_year(self.game_tick) {
            // the year's debt lines for the year-end report (none when the cash is not negative)
            self.year_notice = std::mem::take(&mut self.econ.notice);
            if !self.year_notice.is_empty() {
                println!("[{:6.1}s] board: {}", self.sim_time, self.year_notice.replace('\n', " "));
            }
        }
        for (&i, units) in staffed.iter().zip(paid) {
            self.employees[i].paid = (self.employees[i].paid + units).min(0x7fff);
        }
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
                clicked: self.hover_tile,
                follow_pointer: self.club.holes[1].par == 0,
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
            // OpenSGolf extras: Sandbox: no new dandelions (issue #105)
            if !self.club.extras.no_weeds {
                staff::spread_weeds(&self.terrain, &mut self.staff_tiles, self.game_tick, self.difficulty, &mut self.exe_rng, &mut out);
            }
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
        self.owner_clips();
        let weed = match self.theme {
            2 => "Flowers/OilSlick.flc",
            3 => "Flowers/DryGrass.flc",
            _ => "Flowers/dandelion_01.flc",
        };
        self.weed_sprite = self.sprite_for(weed, false, None);
    }

    /// The player's pro uses a golfer's clips: walk, stand and the happy gesture. Those of MaleKLS until the record has been
    /// edited on the Customise screen; then the body chosen there, as the pro's golfer on the course (issue #79).
    fn owner_clips(&mut self) {
        match self.club.roster.first().filter(|p| p.fixed != 0).map(crate::cust_ui::person_look) {
            Some(look) => {
                for (m, base) in [0x00, 0x0a, 0x1e].into_iter().enumerate() {
                    if let found @ (Some(_), _) = self.golfer_clip(base + look as i32) {
                        self.staff_clips[8][m] = found;
                    }
                }
            }
            None => {
                for (m, name) in ["Male/MaleKLS_NormalWalk", "Male/MaleKLS_Sq", "Male/MaleKLS_Happy"].iter().enumerate() {
                    let body = self.sprite_for(&format!("{name}.flc"), false, None);
                    let shadow = self.sprite_for(&format!("{name}Shadow.flc"), true, None);
                    self.staff_clips[8][m] = (body, shadow);
                }
            }
        }
    }

    /// The clip set and state of an employee: (set, state) into staff_clips (state 0 walking, 1 standing, 2 working).
    pub(crate) fn staff_clip_of(&self, e: &staff::Employee) -> (usize, usize) {
        let set =
            if e.job == staff::job::OWNER { 8 } else { ((-(e.job as i32) - 2) + if e.upgraded { 4 } else { 0 }).clamp(0, 7) as usize };
        let state = if e.anim < staff::ANIM_STAND {
            0
        } else if e.anim == staff::ANIM_STAND {
            1
        } else {
            2
        };
        (set, state)
    }

    /// One game tick of the employees' animation, as the exe's: two frames a tick walking, one otherwise; a finished action
    /// clip goes back to standing.
    fn staff_animate(&mut self) {
        self.owner_clips();
        for i in 0..self.employees.len() {
            let e = self.employees[i];
            if !e.active {
                continue;
            }
            let (set, state) = self.staff_clip_of(&e);
            let n = self.staff_clips[set][state].0.map(|b| self.sprites[b].s.frames_per_view.max(1)).unwrap_or(1);
            let e = &mut self.employees[i];
            e.frame = e.frame.wrapping_add(if state == 0 { 2 } else { 1 });
            if e.frame as i32 % n == 0 && e.anim == staff::ANIM_ACTION {
                e.anim = staff::ANIM_STAND;
            }
        }
    }

    /// Points the employee props at the employees: clip by job and state, frame advanced like the exe's (two a frame when
    /// walking), facing from the employee's direction.
    pub fn update_staff_props(&mut self) {
        self.owner_clips();
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
            let (set, state) = self.staff_clip_of(&e);
            let (body, shadow) = self.staff_clips[set][state];
            let Some(b) = body else {
                self.props[pi].hidden = true;
                continue;
            };
            // the frame steps with the game tick (staff_tick), not with the screen's refresh
            let n = self.sprites[b].s.frames_per_view.max(1);
            let f = e.frame as i32 % n;
            let e = &self.employees[ei];
            const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
            const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];
            let k = (e.dir as i32 & 7) as usize;
            let (x, z) = (e.x, e.y);
            let (wx, wz) = self.units_to_world(x, z);
            let outfit = (e.job == staff::job::OWNER).then(|| {
                let mut o = self.club.roster.first().map(|p| p.outfit(0)).unwrap_or_default();
                o.alt_skin = 4; // the pro's hands are drawn pale, as for every pro
                o
            });
            let p = &mut self.props[pi];
            p.hidden = false;
            p.body = Some(b);
            p.shadow = shadow;
            p.x = wx;
            p.z = wz;
            p.frame = f;
            p.outfit = outfit;
            p.heading = DY[k].atan2(DX[k]).to_degrees();
        }
    }
}

/// A resident celebrity's colours as the exe composes them (0x46212f): skin and hair from the male swap palettes skin + 1
/// and hair + 6 (the hair in the hat's entries 60..79), shirt and trousers as a golfer's, the hands always swap 5; the male
/// swaps for every type. A celebrity record that is missing reads as the first colours.
fn resident_outfit(c: Option<&sg_core::vips::Celebrity>) -> sg_core::bodies::Outfit {
    let (skin, hair, shirt, pants) = c.map(|c| (c.skin, c.hair, c.shirt, c.pants)).unwrap_or_default();
    sg_core::bodies::Outfit { female: false, shirt, pants, skin, hat: hair.saturating_add(5), hair: 0, alt_skin: 4 }
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
        // (measured and logged as in update_props: replay.rs)
        self.screen_hook();
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
            let outfit = resident_outfit(self.club.celebrities.get((r.owner - 1).max(0) as usize));
            let p = &mut self.props[pi];
            // the clips carry the golfers' template palette (blue skin, #141): the exe recolours each resident from its
            // celebrity's colours as it draws it (0x4017d0, palette slot 0x9c + celebrity, composed at 0x46212f)
            p.outfit = Some(outfit);
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
        let exe_theme = self.exe_theme();
        // Tile items: benches, ornamental trees and flower beds as the exe draws them (sg_core::decor)
        let noise = self.noise.clone();
        let field = |x: i32, y: i32| noise.field(x, y);
        let mut draws: Vec<(i32, i32, sg_core::decor::Draw)> = Vec::new();
        for a in 0..land::mn() {
            for b in 0..land::mn() {
                let i = (a * land::mn() + b) as usize;
                // nothing stands on land outside the property (the generator's benches round a landmark beyond the border)
                if land.ty[i] == land::T_OUT {
                    continue;
                }
                let f = land.flags[i];
                if f & 0x20 != 0 && land.ty[i] == land::T_WATER {
                    // bridge pieces on a path over water
                    let (mut path, mut dry, mut water_path) = (0u8, 0u8, false);
                    for k in 0..4 {
                        let (na, nb) = (a + sg_core::geom::DX[2 * k], b + sg_core::geom::DY[2 * k]);
                        if !(0..land::mn()).contains(&na) || !(0..land::mn()).contains(&nb) {
                            continue;
                        }
                        let j = (na * land::mn() + nb) as usize;
                        if land.flags[j] & 0x20 != 0 {
                            path |= 1 << k;
                            if land.ty[j] == land::T_WATER {
                                water_path = true;
                            } else {
                                dry |= 1 << k;
                            }
                        }
                    }
                    let rot = ((self.rot / 90.0).round() as i32).rem_euclid(4);
                    for d in sg_core::decor::bridge(path, dry, water_path, f & 0x100 != 0, (f & 0x1f) as u8, 0, 1, rot) {
                        draws.push((a, b, d));
                    }
                }
                if f & 0x200 != 0 {
                    let course = &self.course;
                    for d in sg_core::decor::benches(land.var[i], &|d| course.bench_ok(a, b, d as i32)) {
                        draws.push((a, b, d));
                    }
                } else if land.ty[i] == land::T_ELM && f & 0x100 != 0 {
                    let grown = !self.sprouting(a, b);
                    for d in sg_core::decor::ornamental(land.var[i], a, b, grown, &field) {
                        draws.push((a, b, d));
                    }
                } else if f & 0x1000 != 0 && land.ty[i] != land::T_OUT {
                    let course = &self.course;
                    let bed = |a: i32, b: i32| {
                        let j = (a * land::mn() + b) as usize;
                        ((0..land::mn()).contains(&a) && (0..land::mn()).contains(&b) && land.flags[j] & 0x1000 != 0)
                            .then(|| (land.var[j], course.level_table.get(j).copied().unwrap_or(0)))
                    };
                    let weedy = course.flags.get(i).is_some_and(|f| f & sg_core::course::f::WEED != 0);
                    let growth = self.sprouts.get(&(a, b)).map(|&c| c as i32);
                    let rot = ((self.rot / 90.0).round() as i32).rem_euclid(4);
                    for d in sg_core::decor::flower_bed(a, b, &bed, weedy, growth, &|a, b| course.raw_corner(a, b), rot) {
                        draws.push((a, b, d));
                    }
                }
            }
        }
        for (a, b, d) in draws {
            let n = self.props.len();
            self.push_decor(a, b, d, true, false);
            // the beds go straight onto the frame in the exe's tile loop, under the queued sprites
            if (0x1a2..=0x1b3).contains(&d.sprite) && self.props.len() > n {
                self.props[n].flat = true;
                self.props[n].bed = Some((d.sprite, d.pal, a, b));
            }
            // a scenic tree or flower bed still growing shows its counter (update_tree_growth); the tree or the bed piece
            // ends it, a gazebo or topiary in the bed follows
            if d.frame.is_some() && self.props.len() > n && self.sprouting(a, b) {
                let lead = d.sprite != 0x1b4 && d.sprite != 0x1b5;
                self.props[n].sprout = Some((a, b, lead));
            }
        }
        let big_course = self.hole_numbers.len() > 10;
        for (oi, o) in land.objects.iter().enumerate().filter(|(_, o)| o.kind >= 0) {
            // The art level the exe's object drawing (0x463180) gets: the number built of the kind less one (0x5a8c38, one
            // more for an upgraded building, i.e. the object's upgrade level), except the snack bar, which takes its second
            // art once the course has more than 10 holes. The hole count used for every kind here made the art change
            // whenever the objects were redrawn on a course of 11+ holes (a tournament placing its TV towers, after a load).
            let level = if o.kind == land::K_SNACK_BAR { big_course as u16 } else { o.sub.clamp(0, 1) as u16 };
            // land outside the property is not drawn, nor what stands on it (an obstacle placed before the border was
            // taken away shows again when the tract is bought)
            if self.terrain.type_at(o.a, o.b) == 20 {
                continue;
            }
            if o.kind == land::K_DRIVING_RANGE {
                // the range is drawn tile by tile (0x463180 case 10): shop, platforms, fences, flags on its target greens
                let green = |i: i32, j: i32| {
                    let (ta, tb) = (o.a + i, o.b + j);
                    (0..land::mn()).contains(&ta)
                        && (0..land::mn()).contains(&tb)
                        && land.ty[(ta * land::mn() + tb) as usize] == land::T_GREEN
                };
                for pc in sg_core::objects::driving_range(level, theme, &green) {
                    let Some(body) = self.sprite_for(&format!("{}.flc", pc.file), false, None) else { continue };
                    let shadow = if pc.flat { None } else { self.sprite_for(&format!("{}Shadow.flc", pc.file), true, None) };
                    let (x, z) = self.units_to_world(o.a * 1024 + pc.at.0, o.b * 1024 + pc.at.1);
                    self.props.push(Prop {
                        x,
                        z,
                        body: Some(body),
                        shadow,
                        flat: pc.flat,
                        animated: pc.animated,
                        phase: pc.phase,
                        view: pc.view,
                        object: true,
                        ..Default::default()
                    });
                }
                continue;
            }
            let size = sg_core::data::buildings().get(o.kind as usize).map(|b| b.1).unwrap_or(1);
            let (cx, cz) = self.terrain.tile_centre(o.a, o.b);
            let off = (size - 1) as f32 * TILE_SIZE * 0.5;
            let (x, z) = (cx + off, cz + off);
            // a landmark in its own palette, 100 + type, facing its way plus the view's quarter turns (one more for type 3,
            // whose view the exe turns by the full eighth count, 0x463180 case 4)
            let (mut pal, mut facing) = (None, o.dir as i32);
            if o.kind == land::K_WASHER {
                pal = sg_core::decor::palette_file(0x3d, exe_theme);
            }
            if o.kind == land::K_LANDMARK {
                pal = sg_core::objects::landmark_art(o.sub, exe_theme).and_then(|a| a.1);
                if o.sub == 3 {
                    facing += ((self.rot / 90.0).round() as i32).rem_euclid(4);
                }
            }
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
            } else if o.kind == land::K_WASHER {
                // the ball washer (0x463180 case 3): sprite 0x183 "bldgs/upgrade bwasher" in palette 0x3d, in every theme,
                // facing its way; frame 0, or while a golfer washes (the tile's 0x4000 flag) its counter (update_tree_growth)
                vec![("bldgs/upgrade bwasher".to_string(), false, false)]
            } else if o.kind == land::K_LANDMARK {
                // the original's types and the Cut content extra's (sg_core::objects::landmark_art)
                match sg_core::objects::landmark_art(o.sub, exe_theme).filter(|_| o.sub < sg_core::objects::ANIMAL_BASE) {
                    Some((f, _)) => vec![(f.to_string(), false, true)],
                    None => continue,
                }
            } else {
                sg_core::objects::building_layers(o.kind, level, theme).iter().map(|l| (l.file.to_string(), l.flat, l.animated)).collect()
            };
            for (file, flat, animated) in layers {
                let body = self.sprite_for(&format!("{file}.flc"), false, pal);
                if body.is_none() {
                    continue;
                }
                let shadow = if flat { None } else { self.sprite_for(&format!("{file}Shadow.flc"), true, None) };
                let tv = (animated && matches!(o.kind, land::K_TV_TOWER | land::K_TV_BOOTH)).then_some((o.a, o.b));
                let grow_tile = (o.kind == land::K_WASHER).then_some((o.a, o.b));
                self.props.push(Prop { x, z, body, shadow, flat, facing, animated, object: true, tv, grow_tile, ..Default::default() });
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

/// The name of employee kind k (0 Club Pro, 1 Ranger, 2 Groundskeeper, 3 Soda Vendor), basic or experienced.
pub fn staff_title(k: usize, experienced: bool) -> &'static str {
    tr(STAFF_NAME_KEYS[k][experienced as usize])
}

/// Names of the employee kinds, basic and experienced, as the exe shows them (text keys).
pub const STAFF_NAME_KEYS: [[&str; 2]; 4] = [
    ["STR_STAFF_CLUB_PRO", "STR_STAFF_CELEBRITY"],
    ["STR_STAFF_RANGER", "STR_STAFF_MARSHALL"],
    ["STR_STAFF_GROUNDSKEEPER", "STR_STAFF_TECHNICIAN"],
    ["STR_STAFF_SODA_VENDOR", "STR_STAFF_REFRESHER"],
];

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
            // (on the course the tile belongs to: sg_core::second)
            let b = self.club.tile_course(h, t.0, t.1);
            if fl & sg_core::course::f::CUP != 0 && h > 0 && h < 19 && self.club.course_holes(b).get(h).is_some_and(|r| r.par == 0) {
                if b == self.hole_b() {
                    self.open_hole();
                }
                return;
            }
        }
        if self.moving_employee.is_none() && sg_core::course::inside(t.0, t.1) {
            let o = self.course.object_at(t.0, t.1);
            let clubhouse = o >= 0 && self.course.objects.get(o as usize).is_some_and(|ob| ob.kind == land::K_CLUBHOUSE);
            // a golfer under the click (or one held by the card's Move/Eject) comes before the clubhouse
            if self.golfer_click(wx, wz, clubhouse) || clubhouse && self.open_pair_screen() {
                return;
            }
        }
        if let Some(i) = self.moving_employee.take() {
            if self.act(Action::Post { employee: i, x: t.0, y: t.1 }) {
                let e = self.employees[i];
                let name = if e.job == staff::job::OWNER {
                    None
                } else {
                    Some(staff_title((-2 - e.job as i32).clamp(0, 3) as usize, e.upgraded).to_string())
                };
                let name = name.unwrap_or_else(|| self.pro_name());
                self.show_toast(&trf("STR_TOAST_EMPLOYEE_POSTED", &[("EMPLOYEE", &name)]));
            }
            return;
        }
        let mut pick = None;
        for (i, e) in self.employees.iter().enumerate() {
            // any employee record, the player's pro included (0x41eff5), but not while he is out playing a round
            if !e.active || (e.job == staff::job::OWNER && self.club.gary != -1) {
                continue;
            }
            let (ex, ez) = self.units_to_world(e.x, e.y);
            if (ex - wx).hypot(ez - wz) < 70.0 {
                let owner = e.job == staff::job::OWNER;
                pick = Some((i, (!owner).then(|| staff_title((-2 - e.job as i32).clamp(0, 3) as usize, e.upgraded).to_string())));
                break;
            }
        }
        if let Some((i, name)) = pick {
            let name = name.unwrap_or_else(|| self.pro_name());
            self.moving_employee = Some(i);
            self.show_toast(&trf("STR_TOAST_EMPLOYEE_CLICK", &[("EMPLOYEE", &name)]));
            return;
        }
        self.clicked_tile = Some(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game on a flat fairway with hole 1's tee at 10,10 and its cup green at 10,30, planned but not opened.
    fn building_game() -> App {
        let mut a = App::with_clock(PathBuf::new(), 4242);
        let n = sg_core::mapsize::ORIGINAL;
        let tiles = (n * n) as usize;
        a.terrain = Terrain { w: n, h: n, ty: vec![2; tiles], variation: vec![0; tiles], set: vec![0; tiles], ..Default::default() };
        a.terrain.corner = vec![1; ((n + 1) * (n + 1)) as usize];
        a.terrain.path_kind = vec![0; tiles];
        a.terrain.wall_mask = vec![0; tiles];
        a.terrain.paint(10, 10, 0, 0);
        a.terrain.paint(10, 30, 1, 0);
        let h = &mut a.club.holes[1];
        h.back = (10, 10);
        h.fwd = (10, 10);
        h.pin = (10, 30);
        a.sync_course();
        a.game_tick = 1;
        a
    }

    /// What the plan of the hole being built leaves in the game.
    fn plan_of(a: &App) -> String {
        let c = &a.club;
        serde_json::to_string(&(a.exe_rng, c.game, &c.layout_land, &c.holes[1], &c.g[sg_core::holetool::TRIAL], &c.planner)).unwrap()
    }

    #[test]
    fn analyze_golf_shot_from_the_elevation_panel_analyses_the_clicked_tile() {
        let mut a = building_game();
        a.panel = 2;
        a.pstate.alt = true;
        a.pstate.elev_tool = 0;
        assert!(a.elevation_click(3));
        assert!(a.edit && a.tool == 1 && a.pstate.elev_tool == 3, "the button arms the tool");
        let heights = a.terrain.corner.clone();
        // a left click on tile (10, 20) shows the analysis of the hole nearest it, as / there does, and leaves the ground
        let ts = sg_core::terrain::TILE_SIZE;
        a.has_hit = true;
        a.hit_x = (10.5 - a.terrain.w as f32 * 0.5) * ts;
        a.hit_z = (20.5 - a.terrain.h as f32 * 0.5) * ts;
        assert_eq!(a.terrain.tile_of(a.hit_x, a.hit_z), (10, 20));
        a.apply_tool(false, false);
        assert!(a.analysis.is_some(), "the click analyses");
        assert_eq!(a.analysis_hole, a.club.nearest_hole(10, 20));
        assert_eq!(a.terrain.corner, heights, "no ground is raised or lowered");
    }

    #[test]
    fn analyze_golf_shot_is_still_armed_after_going_to_the_buildings_panel_and_back() {
        let mut a = building_game();
        a.panel = 2;
        a.pstate.alt = true;
        assert!(a.elevation_click(3));
        // the Elevation panel's Buildings button, then the Buildings panel's Elevation button
        assert!(a.elevation_click(-2));
        assert!(!a.edit && !a.pstate.alt);
        assert!(a.buildings_click(-2));
        assert!(a.edit && a.tool == 1 && a.pstate.alt && a.pstate.elev_tool == 3, "the tool drawn as selected is armed");
        let ts = sg_core::terrain::TILE_SIZE;
        a.has_hit = true;
        a.hit_x = (10.5 - a.terrain.w as f32 * 0.5) * ts;
        a.hit_z = (20.5 - a.terrain.h as f32 * 0.5) * ts;
        a.apply_tool(false, false);
        assert!(a.analysis.is_some(), "a click on the course analyses");
    }

    #[test]
    fn the_hole_being_built_is_planned_again_only_after_a_terrain_change() {
        let mut a = building_game();
        assert!(a.club.game & golf::game::LAYOUT != 0 && !a.club.layout_land.is_empty(), "the hole is planned");
        let planned = plan_of(&a);
        // a redraw's sync and the Course Report change nothing (the exe's 0x44fb30 does not run the refresh 0x42f7a0)
        a.sync_course();
        a.report_course(true);
        assert_eq!(plan_of(&a), planned);
        // the instant shot analysis draws only its own shots (0x41d0e3..0x41d8f3 runs no refresh either)
        let mut b = building_game();
        a.start_analysis(Some((10, 20)), true);
        let hole = b.club.nearest_hole(10, 20);
        b.club.shot_analysis(&mut b.course, &mut b.exe_rng, 10, 20, hole);
        assert_eq!(a.exe_rng.state, b.exe_rng.state);
        assert_eq!(a.club.layout_land, b.club.layout_land);
        // a terrain change plans it again
        let before = a.exe_rng.state;
        a.terrain.paint(20, 20, 4, 0);
        a.sync_course();
        assert_ne!(a.exe_rng.state, before);
    }

    #[test]
    fn painting_over_a_flower_bed_takes_it_away() {
        // issue #136: the exe clears the bed's flag on every tile it paints (0x42057b), and what was painted over is drawn
        // no more
        let mut a = building_game();
        a.econ.cash = 1e7;
        a.ensure_land();
        let li = (20 * land::mn() + 20) as usize;
        a.land.as_mut().unwrap().flags[li] |= 0x1000;
        a.pick_brush(PAINT.iter().position(|p| p.ty == 17 && p.vbyte == 0).unwrap());
        a.paint_tiles(&[(20, 20)]);
        assert_eq!(a.land.as_ref().unwrap().flags[li] & 0x1000, 0);
        assert!(!a.props.iter().any(|p| p.bed.is_some_and(|b| (b.2, b.3) == (20, 20))), "the bed is not drawn");
    }

    #[test]
    fn a_resident_wears_its_celebritys_colours() {
        // issue #141: the exe recolours each resident from its record (skin + 1, hair + 6 in the hat's entries, hands 5)
        let c = sg_core::vips::Celebrity { skin: 3, hair: 2, shirt: 7, pants: 1, ..Default::default() };
        let o = super::resident_outfit(Some(&c));
        assert_eq!((o.female, o.shirt, o.pants, o.skin, o.hat, o.alt_skin), (false, 7, 1, 3, 7, 4));
    }

    #[test]
    fn every_property_keeps_its_card() {
        // issue #143: card k is property k whatever the deal (0x4703f0); a sandbox's cards differ in nothing
        let mut a = building_game();
        for sandbox in [false, true] {
            for _ in 0..5 {
                a.deal_offer(sandbox);
                for (k, &(cx, cy)) in crate::world_ui::CARD_POS.iter().enumerate() {
                    assert_eq!(a.world_hit(cx + 25.0, cy + 25.0), k as i32);
                }
            }
        }
        let first: Vec<_> = (0..16).map(|k| a.offer_for(k).0).collect();
        a.deal_offer(true);
        assert_eq!(first, (0..16).map(|k| a.offer_for(k).0).collect::<Vec<_>>());
    }

    #[test]
    fn the_view_stays_on_the_map() {
        // issue #138: the view's centre stops at the map's edge tiles, as the exe's clamp to 0..49
        let mut a = building_game();
        let (lo, hi) = (a.terrain.tile_centre(0, 0), a.terrain.tile_centre(a.terrain.w - 1, a.terrain.h - 1));
        for _ in 0..200 {
            a.pan(-1000.0, 1000.0);
        }
        assert!((lo.0..=hi.0).contains(&a.cam_x) && (lo.1..=hi.1).contains(&a.cam_z));
        a.cam_x = 1e6;
        a.cam_z = -1e6;
        a.clamp_cam();
        assert_eq!((a.cam_x, a.cam_z), (hi.0, lo.1));
    }

    #[test]
    fn a_games_extras_do_not_carry_over_into_new_games() {
        // issue #137: ticked in a game, an extra is that game's; ticked on the title screens, it is the next games'
        let mut a = building_game();
        a.screen = Screen::Play;
        let next = a.extras_next;
        let mut e = a.club.extras;
        e.no_weeds = !e.no_weeds;
        a.set_extras(e);
        assert_eq!(a.club.extras.no_weeds, e.no_weeds);
        assert_eq!(a.extras_next, next, "new games start as before");
        a.screen = Screen::Menu;
        assert_eq!(a.extras_shown(), next, "the title screens show the next games' extras");
        let game = a.club.extras;
        let mut n = next;
        n.no_weeds = !n.no_weeds;
        a.set_extras(n);
        assert_eq!(a.extras_next.no_weeds, n.no_weeds);
        assert_eq!(a.club.extras, game, "the game left on the title menu keeps its own");
        crate::extras_ui::save_next(&next);
    }

    #[test]
    fn water_over_the_sea_keeps_its_depth() {
        // issue #132: the water brush over deep water left it deep and charged nothing, as the exe's same-type exit does
        let mut a = building_game();
        a.econ.cash = 1e7;
        let ti = a.terrain.tile_index(20, 20);
        a.terrain.paint(20, 20, 17, 2);
        a.pick_brush(PAINT.iter().position(|p| p.ty == 17 && p.vbyte == 0).unwrap());
        let cash = a.econ.cash;
        a.paint_tiles(&[(20, 20)]);
        assert_eq!(a.terrain.variation[ti], 2);
        assert_eq!(a.econ.cash, cash);
    }

    #[test]
    fn undo_puts_back_the_click_and_its_money_and_leaves_the_time() {
        // undoing a click no longer goes back in time: only what it painted, and what it cost, come back
        let mut a = building_game();
        a.econ.cash = 1e7;
        a.ensure_land();
        let ti = a.terrain.tile_index(20, 20);
        a.pick_brush(PAINT.iter().position(|p| p.ty == 17 && p.vbyte == 0).unwrap());
        let cash = a.econ.cash;
        a.undo_press();
        a.paint_tiles(&[(20, 20)]);
        a.undo_release();
        let cost = cash - a.econ.cash;
        assert!(cost > 0.0 && a.terrain.ty[ti] == 17);
        // the game goes on: time passes, a green fee comes in
        a.game_tick += 500;
        a.econ.earn_to(economy::LEDGER_GREEN_FEES, 3000.0);
        assert!(a.undo_click_step());
        assert_eq!(a.terrain.ty[ti], 2);
        assert_eq!(a.land.as_ref().unwrap().ty[(20 * land::mn() + 20) as usize], 2);
        assert_eq!(a.game_tick, 501);
        assert_eq!(a.econ.cash, cash + 3000.0);
        let y = &a.econ.ledger[a.econ.year_index()];
        assert_eq!((y[economy::LEDGER_BUILD_COURSE], y[economy::LEDGER_GREEN_FEES]), (0.0, 3000.0));
        assert!(a.floats.iter().any(|f| f.0 == (cost / Economy::UNIT) as i32), "the money back floats over the tile");
        assert!(!a.undo.can_undo());
    }

    #[test]
    fn undo_leaves_what_a_tournament_built_since() {
        // review of #149: a tournament's preparation puts its TV towers on the end of the objects after a click (tourney_ui.rs
        // finish_prep); undoing the click leaves them, and the tile one of them stands on
        let mut a = building_game();
        a.econ.cash = 1e7;
        a.ensure_land();
        a.pick_brush(PAINT.iter().position(|p| p.ty == 17 && p.vbyte == 0).unwrap());
        a.undo_press();
        a.paint_tiles(&[(20, 20), (26, 20)]);
        a.undo_release();
        let theme = a.exe_theme();
        let l = a.land.as_mut().unwrap();
        let n = l.objects.len();
        let tower = l.place(&mut a.exe_rng, 20, 20, land::K_TV_TOWER, 0, theme);
        l.write_area(&mut a.terrain, 20, 20, 20, 20);
        let li = |a: i32, b: i32| (a * land::mn() + b) as usize;
        let under = l.ty[li(20, 20)];
        assert_eq!(tower, n, "the tower goes on the end");
        assert!(a.undo_click_step());
        let l = a.land.as_ref().unwrap();
        assert_eq!(l.objects.len(), n + 1);
        assert_eq!(l.objects[tower].kind, land::K_TV_TOWER);
        assert_eq!((l.ty[li(20, 20)], l.var[li(20, 20)]), (under, tower as u8), "the tower's tile stays");
        assert_eq!(a.terrain.ty[a.terrain.tile_index(20, 20)], under);
        assert_eq!((l.ty[li(26, 20)], a.terrain.ty[a.terrain.tile_index(26, 20)]), (2, 2), "the other tile goes back");
    }

    #[test]
    fn undo_brings_back_a_building_whose_record_was_used_since() {
        // a building taken away by a click, whose record a TV tower took (Land::place reuses the first free one), comes back
        // in another record, and its tiles point there
        let mut a = building_game();
        a.econ.cash = 1e7;
        a.ensure_land();
        let theme = a.exe_theme();
        let li = |a: i32, b: i32| (a * land::mn() + b) as usize;
        let l = a.land.as_mut().unwrap();
        let mark = l.place(&mut a.exe_rng, 24, 24, land::K_LANDMARK, 0, theme);
        l.write_area(&mut a.terrain, 24, 24, 24, 24);
        a.undo_press();
        let l = a.land.as_mut().unwrap();
        l.remove_object(mark);
        l.write_area(&mut a.terrain, 24, 24, 24, 24);
        a.undo_release();
        let l = a.land.as_mut().unwrap();
        let tower = l.place(&mut a.exe_rng, 30, 30, land::K_TV_TOWER, 0, theme);
        assert_eq!(tower, mark, "the tower takes the landmark's record");
        assert!(a.undo_click_step());
        let l = a.land.as_ref().unwrap();
        assert_eq!(l.objects[tower].kind, land::K_TV_TOWER);
        let back = l.objects.len() - 1;
        assert_ne!(back, mark);
        assert_eq!((l.objects[back].kind, l.objects[back].a, l.objects[back].b), (land::K_LANDMARK, 24, 24));
        assert_eq!(l.var[li(24, 24)], back as u8);
        assert_eq!(l.var[li(30, 30)], tower as u8);
    }

    #[test]
    fn undo_puts_back_a_holes_tee() {
        let mut a = building_game();
        a.econ.cash = 1e7;
        a.pick_brush(PAINT.iter().position(|p| p.ty == 4).unwrap());
        a.undo_press();
        a.paint_tiles(&[(10, 10)]);
        a.undo_release();
        assert_eq!(a.club.holes[1].back.0, 0, "painting over the tee takes it from the hole");
        assert!(a.undo_click_step());
        assert_eq!(a.club.holes[1].back, (10, 10));
        assert_eq!(a.terrain.ty[a.terrain.tile_index(10, 10)], 0);
        assert_eq!(a.course.flags[sg_core::course::idx(10, 10)] & sg_core::course::f::LOW, 1, "the tee is hole 1's again");
    }

    #[test]
    fn placing_and_removing_float_their_money() {
        let mut a = building_game();
        a.econ.cash = 1e7;
        a.ensure_land();
        a.edit = true;
        a.tool = 2;
        a.has_hit = true;
        (a.hit_x, a.hit_z) = a.terrain.tile_centre(20, 20);
        a.apply_tool(false, false);
        let paid = a.floats.iter().find(|f| f.3 > 0).map(|f| f.0).unwrap();
        assert!(paid < 0, "a path's cost floats in red");
        a.apply_tool(true, false);
        assert!(a.floats.iter().any(|f| f.3 > 0 && f.0 == -paid), "removing it floats the money back in green");
    }

    #[test]
    fn each_tile_painted_floats_its_own_cost() {
        // footage of the original: every tile of a stroke shows its own amount over it
        let mut a = building_game();
        a.econ.cash = 1e7;
        a.pick_brush(PAINT.iter().position(|p| p.ty == 12).unwrap());
        a.undo_press();
        a.paint_tiles(&[(20, 20), (21, 20), (22, 20)]);
        a.undo_release();
        let shown: Vec<_> = a.floats.iter().filter(|f| f.3 > 0).map(|f| (f.0, f.1 >> 10, f.2 >> 10)).collect();
        let each = -Economy::terrain_cost_units(12);
        assert_eq!(shown, [(each, 20, 20), (each, 21, 20), (each, 22, 20)]);
        // undone, each tile's money floats back over it
        assert!(a.undo_click_step());
        let back: Vec<_> = a.floats.iter().filter(|f| f.3 > 0 && f.0 > 0).map(|f| (f.0, f.1 >> 10)).collect();
        assert_eq!(back, [(-each, 20), (-each, 21), (-each, 22)]);
    }

    #[test]
    fn the_trial_golfers_ball_is_not_a_golfers() {
        // the layout pass leaves the trial golfer's ball on the planned hole; it lies past the golfers the game draws
        let a = building_game();
        let t = &a.club.g[sg_core::holetool::TRIAL];
        assert!(t.bx != 0 && (1..19).contains(&t.hole));
        assert!(sg_core::holetool::TRIAL >= sg_core::golfer::SLOTS);
    }

    #[test]
    fn a_sand_trap_is_painted_the_way_the_pointer_shows_it_turned() {
        // issue #104: the shape picked with Tab must be the one placed
        let mut a = building_game();
        a.econ.cash = 1e7;
        let k = PAINT.iter().position(|p| p.ty == 7).unwrap();
        a.pick_brush(k);
        let (_, v0) = a.paint_variant.expect("picking the brush rolls its variant");
        assert!((0..3).contains(&v0));
        // two Tabs (main.rs), then a tile
        for _ in 0..2 {
            let (k, v) = a.paint_variant.unwrap();
            a.paint_variant = Some((k, v + 1));
        }
        a.paint_tiles(&[(20, 20)]);
        let ti = a.terrain.tile_index(20, 20);
        let li = (20 * land::mn() + 20) as usize;
        assert_eq!(a.terrain.ty[ti], 7);
        assert_eq!(a.terrain.variation[ti] as i32, (v0 + 2) % 12 + 12, "the terrain draws the brush's shape");
        assert_eq!(a.land.as_ref().unwrap().var[li], a.terrain.variation[ti], "and the land keeps it");
        assert_eq!(a.paint_variant, Some((k, v0 + 2)), "painting keeps the turn");
        // one more Tab and the same tile again: turned, free (0x42059d before the same-type exit)
        a.paint_variant = Some((k, v0 + 3));
        let cash = a.econ.cash;
        a.paint_tiles(&[(20, 20)]);
        assert_eq!(a.terrain.variation[ti] as i32 & 3, (v0 + 3) & 3);
        assert_eq!(a.econ.cash, cash);
        // the green still starts plain
        let g = PAINT.iter().position(|p| p.ty == 1).unwrap();
        a.pick_brush(g);
        assert_eq!(a.paint_variant, Some((g, 0)));
    }

    #[test]
    fn a_load_plans_the_hole_being_built_again_as_the_original_does() {
        let a = building_game();
        let bytes = a.save_bytes(1).unwrap();
        let load = || {
            let mut b = App::with_clock(PathBuf::new(), 7);
            b.load_save_into(None, read_save_bytes(&bytes, "test").unwrap(), false).unwrap();
            b
        };
        let (mut b, c) = (load(), load());
        // 0x40b9b0 then 0x42f7a0 (0x42f929): planned again, with the generator as saved, the same way each time
        assert_ne!(b.exe_rng.state, a.exe_rng.state);
        assert!(b.club.game & golf::game::LAYOUT != 0);
        assert_eq!(plan_of(&b), plan_of(&c));
        // and only once: the load's redraw keeps that plan
        let loaded = plan_of(&b);
        b.sync_course();
        assert_eq!(plan_of(&b), loaded);
    }
}
