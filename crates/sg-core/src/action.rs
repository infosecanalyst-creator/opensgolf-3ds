//! What the player does to the game, as plain data (the port's own; CONTRIBUTING.md "Replays"): every change the player
//! makes to a game (painting, building, staff, the pro's shots, money, undo, the game speed ...) is an [`Action`], and the
//! game performs it in one place (simgolf's `App::apply`). The simulation is deterministic, so a game as it was when the log
//! began plus its actions, each stamped with the game tick it was applied at, plays that game again exactly: an exact replay
//! for a bug report (the report's files hold the log), and in time an exact undo.
//!
//! Besides the player's actions the log holds what the game around the simulation hands it, which no save keeps: the
//! pointer's tile, the flags set before each tick (the ticker's, the close view's), the view the golfers' screen places
//! are measured in, and the course brought up to date with the land on a redraw. A hash of the game every
//! [`CHECK_EVERY`] ticks lets a replay say the first tick at which it stopped matching.
//!
//! **When an action is applied:** between two game ticks, never inside one. An entry stamped `t` was applied after the
//! game had played `t` ticks (the game's tick counter read `t`) and before it played the next; a replay applies every
//! entry stamped `t`, in the order logged, just before it plays that tick. The flags and the pointer the simulation reads
//! during tick `t` are logged with the stamp `t` and only held until that tick reads them.
//!
//! **Format:** [`Log`] is serde data with a [`LOG_VERSION`]; every variant and field has an explicit short name, so a
//! later version can add variants and fields (with defaults) and still read older logs. Actions hold only game values
//! (tiles, hole numbers, employee and golfer records, terrain types, amounts), never screen positions or interface state.

use crate::extras::Extras;
use crate::land::ExeRng;
use crate::staff::StaffGolfer;
use serde::{Deserialize, Serialize};

/// The version of the log's format.
pub const LOG_VERSION: u32 = 1;

/// Game ticks between two hashes of the game in the log.
pub const CHECK_EVERY: u32 = 256;

/// The most entries a log holds; past it the log starts again from the game as it then is.
pub const MAX_ENTRIES: usize = 400_000;

/// The flags the game sets before each tick from what is on the screen (sg_core::golfer::Club's unsaved flags).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickFlags {
    /// The ticker is taken (a message showing or waiting, or the golfer card open).
    #[serde(rename = "b")]
    pub busy: bool,
    /// Preferences' advisor and first-time messages.
    #[serde(rename = "a")]
    pub advisor: bool,
    /// The view is at the closest zoom.
    #[serde(rename = "c")]
    pub close: bool,
    /// A ticker message is up.
    #[serde(rename = "m")]
    pub message: bool,
    /// Preferences' snapshots of great shots.
    #[serde(rename = "s")]
    pub snapshots: bool,
}

/// The view the golfers' places on the 800 x 600 screen are measured in (the original's drawing records them, and the
/// simulation reads them): the ground's projection, the drawable's size and the screen's scale and offset in it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct View {
    #[serde(rename = "m")]
    pub mv: [f32; 16],
    #[serde(rename = "u")]
    pub upp: f32,
    #[serde(rename = "w")]
    pub w: f32,
    #[serde(rename = "h")]
    pub h: f32,
    #[serde(rename = "k")]
    pub scale: f32,
    #[serde(rename = "x")]
    pub ox: f32,
    #[serde(rename = "y")]
    pub oy: f32,
}

/// The settings of the port's own that change the rules of play (they are the player's, kept in the settings, not the game's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    #[serde(rename = "fix_bugs")]
    pub fix_bugs: bool,
    #[serde(rename = "random_trees")]
    pub random_trees: bool,
}

/// One thing done to the game. The names in the log are the `rename`s, which never change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    // --- what the game around the simulation hands it
    /// The course brought up to date with the land (a redraw, a report): this plans the hole being built again after a
    /// terrain change.
    #[serde(rename = "sync")]
    Sync,
    /// The pointer's tile, which the pro's employee follows until the first hole opens.
    #[serde(rename = "ptr")]
    Pointer(Option<(i32, i32)>),
    /// The flags the next ticks are played with.
    #[serde(rename = "flags")]
    Flags(TickFlags),
    /// The view the next measures of the golfers' screen places use.
    #[serde(rename = "view")]
    View(View),
    /// The golfers' screen places measured (after a drawn frame).
    #[serde(rename = "screen")]
    Screen,
    /// A hash of the game as it stood (simgolf's `App::replay_hash`): a replay compares its own.
    #[serde(rename = "check")]
    Check(u64),
    /// The settings of the port's own that change the rules the game is played by: Fix original bugs, and the Random
    /// trees extra (the looks of planted trees).
    #[serde(rename = "rules")]
    Rules(Rules),
    /// The open holes' tees drawn with their arrows for the view turned `camera` (0, 2, 4, 6): the original keeps the
    /// arrow in the tile's variant byte, which the game reads back.
    #[serde(rename = "tee_arrows")]
    TeeArrows(i32),
    /// Planted trees and flower beds on these tiles grown to their last frame (the original's drawing ends the tile's mark
    /// for the staff then, on its own clock).
    #[serde(rename = "grown")]
    Grown(Vec<(i32, i32)>),
    /// The game speed: 1, 2 or 4.
    #[serde(rename = "speed")]
    Speed(u8),

    // --- building and the terrain
    /// A terrain brush picked (its type and variant byte); the original rolls the brush's look with the game's generator.
    #[serde(rename = "brush")]
    PickBrush {
        #[serde(rename = "ty")]
        ty: i32,
        #[serde(rename = "v")]
        vbyte: i32,
    },
    /// The terrain brush of type `ty` (variant byte `vbyte`) on these tiles, as one stroke; `turn` is the brush's look (Tab,
    /// none: rolled as it is used) and `look` the looks' generator (the Random trees extra).
    #[serde(rename = "paint")]
    Paint {
        #[serde(rename = "ty")]
        ty: i32,
        #[serde(rename = "v")]
        vbyte: i32,
        #[serde(rename = "turn", default)]
        turn: Option<i32>,
        #[serde(rename = "look", default)]
        look: Option<u32>,
        #[serde(rename = "at")]
        tiles: Vec<(i32, i32)>,
        /// The tee a tee brush places (the Multiple tees extra): 0 the hole's own, 1 forward, 2 middle.
        #[serde(rename = "tee", default)]
        tee: u8,
    },
    /// A rectangle or fill of the terrain brush (Shift or Alt), checked and charged as a whole.
    #[serde(rename = "area")]
    PaintArea {
        #[serde(rename = "ty")]
        ty: i32,
        #[serde(rename = "v")]
        vbyte: i32,
        #[serde(rename = "turn", default)]
        turn: Option<i32>,
        #[serde(rename = "look", default)]
        look: Option<u32>,
        #[serde(rename = "at")]
        tiles: Vec<(i32, i32)>,
        #[serde(rename = "big", default)]
        too_large: bool,
        #[serde(rename = "tee", default)]
        tee: u8,
    },
    /// The path tool on tile x, y with a brush of radius `brush` (`remove`: taken up).
    #[serde(rename = "path")]
    Path {
        x: i32,
        y: i32,
        #[serde(rename = "r")]
        brush: i32,
        #[serde(rename = "rm", default)]
        remove: bool,
        /// The path style (the Path styles and lamps extra).
        #[serde(rename = "k", default)]
        style: i32,
    },
    /// A building, amenity or landmark of `kind` on tile x, y, with its design (kind, design) and turn (`remove`: undone).
    #[serde(rename = "build")]
    Build {
        x: i32,
        y: i32,
        #[serde(rename = "k")]
        kind: i32,
        #[serde(rename = "d", default)]
        design: Option<(i32, i32)>,
        #[serde(rename = "turn", default)]
        turn: i32,
        #[serde(rename = "rm", default)]
        remove: bool,
        #[serde(rename = "s", default)]
        style: i32,
    },
    /// A row of `kind` from one tile to another (the Drag to place extra).
    #[serde(rename = "row")]
    Row {
        #[serde(rename = "a")]
        from: (i32, i32),
        #[serde(rename = "b")]
        to: (i32, i32),
        #[serde(rename = "k")]
        kind: i32,
        #[serde(rename = "d", default)]
        design: Option<(i32, i32)>,
        #[serde(rename = "turn", default)]
        turn: i32,
        #[serde(rename = "rm", default)]
        remove: bool,
        #[serde(rename = "s", default)]
        style: i32,
    },
    /// The Elevation panel's tool `tool` (0 a point, 1 2 x 2, 2 an area) on corner x, y, up or down.
    #[serde(rename = "elev")]
    Elevation {
        x: i32,
        y: i32,
        #[serde(rename = "t")]
        tool: i32,
        #[serde(rename = "up")]
        up: bool,
    },
    /// The raise tool on corner x, y with a round brush of radius `brush`, `delta` steps.
    #[serde(rename = "raise")]
    Raise {
        x: i32,
        y: i32,
        #[serde(rename = "r")]
        brush: i32,
        #[serde(rename = "d")]
        delta: i32,
    },
    /// What stands on tile x, y taken back, as the original's second right click does.
    #[serde(rename = "take")]
    TakeBack { x: i32, y: i32 },
    /// A press on the course: the course is kept in case the click changes it (the undo list).
    #[serde(rename = "press")]
    UndoPress,
    /// The button up: what the click changed goes on the undo list.
    #[serde(rename = "release")]
    UndoRelease,
    /// Undo: the last click that changed the course is put back.
    #[serde(rename = "undo")]
    Undo,
    /// The instant shot analysis from tile x, y (`nearest`: of the hole nearest it, else of the last one analysed).
    #[serde(rename = "analyse")]
    Analysis {
        x: i32,
        y: i32,
        #[serde(rename = "n")]
        nearest: bool,
    },
    /// The hole being built opened.
    #[serde(rename = "open")]
    OpenHole,
    /// Hole `from` moved to number `to` (the Routing screen).
    #[serde(rename = "move_hole")]
    MoveHole {
        #[serde(rename = "a")]
        from: usize,
        #[serde(rename = "b")]
        to: usize,
    },
    /// Hole `hole`'s flag word (the Hole Stats' flags).
    #[serde(rename = "hole_flags")]
    HoleFlags {
        #[serde(rename = "h")]
        hole: usize,
        #[serde(rename = "f")]
        flags: u32,
    },
    /// The Set par extra: hole `hole` takes par `par`.
    #[serde(rename = "par")]
    SetPar {
        #[serde(rename = "h")]
        hole: i32,
        #[serde(rename = "p")]
        par: i32,
    },

    // --- staff
    /// An employee hired: `kind` 0 groundskeeper .. 3, `skilled` the experienced one.
    #[serde(rename = "hire")]
    Hire {
        #[serde(rename = "k")]
        kind: usize,
        #[serde(rename = "s", default)]
        skilled: bool,
    },
    /// Employee record `employee` let go.
    #[serde(rename = "fire")]
    Fire {
        #[serde(rename = "e")]
        employee: usize,
    },
    /// Employee record `employee` posted to tile x, y.
    #[serde(rename = "post")]
    Post {
        #[serde(rename = "e")]
        employee: usize,
        x: i32,
        y: i32,
    },

    // --- the pro
    /// The pro's buttons: 0 a practice round, 1 the match, 3 cancel the round (or the tournament), 4 the tournament.
    #[serde(rename = "pro")]
    Pro {
        #[serde(rename = "b")]
        button: usize,
    },
    /// The shot's shape while the pro aims.
    #[serde(rename = "shot")]
    ShotOption(i32),
    /// The pro's aim at tile x, y (`corner`: at its corner).
    #[serde(rename = "aim")]
    Aim {
        x: i32,
        y: i32,
        #[serde(rename = "c", default)]
        corner: bool,
    },
    /// The aim taken: the pro swings.
    #[serde(rename = "swing")]
    Swing,
    /// One of the pro's skills set to `value` in the skill dialog.
    #[serde(rename = "skill")]
    Skill {
        #[serde(rename = "k")]
        skill: usize,
        #[serde(rename = "v")]
        value: u8,
    },
    /// The skill dialog closed with `points` left; `then` the round it was opened for (match or practice).
    #[serde(rename = "skills_done")]
    SkillsDone {
        #[serde(rename = "p")]
        points: i32,
        #[serde(rename = "then", default)]
        then: Option<bool>,
    },
    /// The player's pro left-handed or not (Customise).
    #[serde(rename = "lefty")]
    ProLeft(bool),

    // --- tournaments
    /// The SGA's report asked for (F7): its evaluation keeps figures in the club.
    #[serde(rename = "sga")]
    SgaReport,
    /// The SGA's tournament asked for (its evaluation).
    #[serde(rename = "tourney")]
    BeginTournament,
    /// The tournament accepted.
    #[serde(rename = "tourney_go")]
    AcceptTournament,
    /// The tournament cancelled.
    #[serde(rename = "tourney_cancel")]
    CancelTournament,
    /// The answer to the tournament's offer (its box's line `k`).
    #[serde(rename = "tourney_answer")]
    TourneyAnswer(usize),
    /// The tournament's preparation box closed with the options ticked (bits).
    #[serde(rename = "tourney_prep")]
    TourneyPrep(i32),
    /// The tournament's results closed.
    #[serde(rename = "tourney_done")]
    TourneyDone,

    // --- land and the club
    /// The land screen opened: the original rolls the tracts' prices with the game's generator.
    #[serde(rename = "land")]
    LandScreen,
    /// The pointer onto another of the land screen's places (`at`: 0..8 a tract, 9 cancel, -1 none): the original rolls the
    /// prices again each time.
    #[serde(rename = "land_at")]
    LandHover(i32),
    /// Tract `tract` (0..8, as the land screen numbers them) bought.
    #[serde(rename = "tract")]
    Tract {
        #[serde(rename = "n")]
        tract: usize,
    },
    /// Golfer record `golfer` held by the card's Move/Eject put down at map units x, y (`clubhouse`: on the clubhouse, so
    /// he goes home).
    #[serde(rename = "drop")]
    DropGolfer {
        #[serde(rename = "g")]
        golfer: usize,
        x: i32,
        y: i32,
        #[serde(rename = "home", default)]
        clubhouse: bool,
    },
    /// The two golfers paired up from the clubhouse's waiting list.
    #[serde(rename = "pair")]
    Pair { a: usize, b: usize },
    /// The golfer card's story goes on to its next chapter.
    #[serde(rename = "chapter")]
    Chapter {
        #[serde(rename = "g")]
        golfer: usize,
    },
    /// The great shot's photo let go (a click, or at once when it cannot be seen): the game goes on.
    #[serde(rename = "photo_done")]
    GreatShotDone,
    /// The golfer card's Take Snapshot: the golfer and his partner say something now.
    #[serde(rename = "snapshot")]
    Snapshot {
        #[serde(rename = "g")]
        golfer: usize,
    },
    /// The pro's round (or the tournament) called off from the System menu.
    #[serde(rename = "cancel_round")]
    CancelRound,
    /// Customise's Exit: person record `record` as edited (the roster's own fields), and the card's golfer `slot` taking
    /// its class toggles.
    #[serde(rename = "person")]
    Person {
        #[serde(rename = "r")]
        record: usize,
        #[serde(rename = "p")]
        person: serde_json::Value,
        #[serde(rename = "g", default)]
        slot: Option<usize>,
        #[serde(rename = "t", default)]
        toggles: u8,
    },
    /// The Top 10 table made afresh (no table file yet): the original draws it with the game's generator.
    #[serde(rename = "top10")]
    Top10,
    /// The hole being built measured where the course view shows its figures (the original stores the length as it draws).
    #[serde(rename = "measure")]
    MeasureHole,
    /// The Second course extra's A/B switch: Course B (`true`, started the first time) or Course A, said in a toast.
    #[serde(rename = "course_switch")]
    CourseSwitch(bool),
    /// The course the hole panel and the hole tools work on, set without a word (a flag of the other course clicked).
    #[serde(rename = "course_shown")]
    CourseShown(bool),
    /// The game's own extras changed while it is played.
    #[serde(rename = "extras")]
    Extras(Extras),
    /// The building whose footprint holds tile x, y turned a quarter (the Rotate extra's Tab).
    #[serde(rename = "turn")]
    Turn { x: i32, y: i32 },
    /// The next path style (the Path styles and lamps extra).
    #[serde(rename = "path_style")]
    PathStyle,
    /// A cheat of the Cheats box.
    #[serde(rename = "cheat")]
    Cheat(usize),
    /// The course renamed.
    #[serde(rename = "name")]
    Rename(String),
    /// The career over: the game goes on as a sandbox.
    #[serde(rename = "sandbox")]
    ContinueSandbox,
    /// A test hook of the scripted runs (the --edit items), as typed.
    #[serde(rename = "script")]
    Script(String),
}

impl Action {
    /// What the game hands the simulation, which the game logs itself: never the player's own doing.
    pub fn is_input(&self) -> bool {
        matches!(
            self,
            Action::Sync
                | Action::Rules(_)
                | Action::TeeArrows(_)
                | Action::Grown(_)
                | Action::Pointer(_)
                | Action::Flags(_)
                | Action::View(_)
                | Action::Screen
                | Action::Check(_)
        )
    }
}

/// One logged action: the game tick it was applied at (before that tick was played) and the action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    #[serde(rename = "t")]
    pub tick: u32,
    #[serde(rename = "a")]
    pub action: Action,
}

/// What the game held at the log's start that no save keeps: what the game around the simulation had handed it, and the
/// game speed (simgolf's `App::log_start`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Start {
    #[serde(rename = "tick")]
    pub tick: u32,
    #[serde(rename = "screen", default)]
    pub screen: Vec<(i32, i32)>,
    #[serde(rename = "flags", default)]
    pub flags: TickFlags,
    #[serde(rename = "snaps", default)]
    pub award_snaps: Vec<(usize, (i32, i32))>,
    #[serde(rename = "ptr", default)]
    pub pointer: Option<(i32, i32)>,
    #[serde(rename = "aim", default)]
    pub auto_aim: u8,
    #[serde(rename = "retyped", default)]
    pub retyped: Vec<(i32, i32)>,
    #[serde(rename = "staff_golfers", default)]
    pub staff_golfers: Vec<StaffGolfer>,
    #[serde(rename = "deco", default)]
    pub deco_rng: Option<ExeRng>,
    /// Whether the hole being built was planned on the course as it stood (else the next redraw plans it again).
    #[serde(rename = "planned", default)]
    pub planned: bool,
    #[serde(rename = "speed", default = "one")]
    pub speed: u8,
    #[serde(rename = "rules", default)]
    pub rules: Rules,
    /// The course the hole tools work on: Course B (the Second course extra) or Course A.
    #[serde(rename = "course_b", default)]
    pub course_b_view: bool,
    /// The weather the club had worked out (not saved: each tick sets it, and a load works it out at once): none yet
    /// (a game not played a tick since it began), and the last day announced and the planner's drag and bounce.
    #[serde(rename = "no_weather", default)]
    pub no_weather: bool,
    #[serde(rename = "announced", default)]
    pub announced: Option<u32>,
    #[serde(rename = "drag", default)]
    pub drag: i32,
    #[serde(rename = "bounce", default)]
    pub bounce_cut: i32,
    /// The first hash, of the game as the log starts, and the hashes of its sections and fields, which say where a replay's
    /// loaded game differs.
    #[serde(rename = "hash", default)]
    pub hash: u64,
    #[serde(rename = "parts", default)]
    pub parts: Vec<(String, u64)>,
}

fn one() -> u8 {
    1
}

/// The actions since the log's start.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Log {
    #[serde(rename = "version")]
    pub version: u32,
    #[serde(rename = "start")]
    pub start: Start,
    #[serde(rename = "entries")]
    pub entries: Vec<Entry>,
}

impl Log {
    pub fn new(start: Start) -> Log {
        Log { version: LOG_VERSION, start, entries: Vec::new() }
    }

    /// How many of the entries are the player's own actions (not what the game hands the simulation).
    pub fn player_actions(&self) -> usize {
        self.entries.iter().filter(|e| !e.action.is_input()).count()
    }
}

/// The first line of a replay file.
pub const REPLAY_MAGIC: &[u8] = b"OPENSGOLF-REPLAY\n";

/// A replay file: the magic line, the saved game the log starts from (its length as 8 bytes, little-endian, then the save
/// file's bytes) and the log as JSON compressed with zlib.
pub fn encode_replay(save: &[u8], log: &Log) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(log).map_err(|e| e.to_string())?;
    let mut out = REPLAY_MAGIC.to_vec();
    out.extend((save.len() as u64).to_le_bytes());
    out.extend_from_slice(save);
    out.extend(miniz_oxide::deflate::compress_to_vec_zlib(&json, 6));
    Ok(out)
}

/// A replay file read back: the save's bytes and the log. A log of a newer version than this build reads is refused.
pub fn decode_replay(data: &[u8]) -> Result<(Vec<u8>, Log), String> {
    let rest = data.strip_prefix(REPLAY_MAGIC).ok_or("not a replay file")?;
    if rest.len() < 8 {
        return Err("the replay file is cut short".into());
    }
    let n = u64::from_le_bytes(rest[..8].try_into().unwrap_or_default()) as usize;
    let rest = &rest[8..];
    if rest.len() < n {
        return Err("the replay file is cut short".into());
    }
    let (save, packed) = rest.split_at(n);
    let json = miniz_oxide::inflate::decompress_to_vec_zlib(packed).map_err(|e| format!("the replay's log cannot be unpacked: {e:?}"))?;
    let log: Log = serde_json::from_slice(&json).map_err(|e| format!("the replay's log cannot be read: {e}"))?;
    if log.version > LOG_VERSION {
        return Err(format!("the replay's log is version {}, newer than this build's {LOG_VERSION}", log.version));
    }
    Ok((save.to_vec(), log))
}

/// Where a replay stopped matching the game it was recorded from: the first logged hash its own game does not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divergence {
    pub tick: u32,
    pub logged: u64,
    pub replayed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logs_names_stay_as_they_are() {
        // the format: older logs must keep reading, so the names written never change
        let e = Entry { tick: 7, action: Action::Paint { ty: 2, vbyte: 0, turn: Some(1), look: None, tiles: vec![(3, 4)], tee: 0 } };
        assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"t":7,"a":{"paint":{"ty":2,"v":0,"turn":1,"look":null,"at":[[3,4]],"tee":0}}}"#);
        assert_eq!(serde_json::to_string(&Action::UndoPress).unwrap(), r#""press""#);
        assert_eq!(serde_json::to_string(&Action::Hire { kind: 1, skilled: true }).unwrap(), r#"{"hire":{"k":1,"s":true}}"#);
        // a field a later version adds is read as its default from an older log
        let old: Action = serde_json::from_str(r#"{"paint":{"ty":2,"v":0,"at":[]}}"#).unwrap();
        assert_eq!(old, Action::Paint { ty: 2, vbyte: 0, turn: None, look: None, tiles: vec![], tee: 0 });
    }

    #[test]
    fn a_replay_file_reads_back() {
        let mut log = Log::new(Start { tick: 12, speed: 2, ..Default::default() });
        log.entries.push(Entry { tick: 12, action: Action::OpenHole });
        log.entries.push(Entry { tick: 40, action: Action::Check(0xdead_beef) });
        let save = b"SIMGOLF-NATIVE-SAVE\nxyz".to_vec();
        let f = encode_replay(&save, &log).unwrap();
        let (s, l) = decode_replay(&f).unwrap();
        assert_eq!((s, l), (save, log.clone()));
        assert!(decode_replay(&f[..f.len() - 3]).is_err());
        assert!(decode_replay(b"something else").is_err());
        let mut newer = log;
        newer.version = LOG_VERSION + 1;
        assert!(decode_replay(&encode_replay(b"", &newer).unwrap()).is_err());
    }
}
