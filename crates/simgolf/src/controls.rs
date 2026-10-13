//! The player's key bindings (the port's own; docs/PLAYING.md, "Changing the keys"): every key of the course screen that the
//! original or the port gives an action can be moved to another key, an optional WASD camera adds W, A, S and D for scrolling,
//! and U hides the interface. The choices are kept under "controls" in settings.json (only the keys that differ from the
//! defaults) and are honoured wherever the course screen's keys are read: natively and in the browser, which runs the same
//! code (web/opensgolf.js only keeps the browser from acting on keys the game uses).
//!
//! The course screen's key handling (main.rs play_key) is left as it is: a key pressed is first turned into the *default*
//! key of the action bound to it ([`translate`]), so play_key sees the original's keys whatever the bindings are. A key
//! that is the default of an action moved elsewhere, and bound to nothing now, does nothing. Keys no action owns (the
//! digits for the holes, the keypad's + and -, keys other parts of the port add) pass through untouched.
//!
//! With the defaults (no "controls" in settings.json) every key does exactly what it did before.
use miniquad::KeyCode;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// The settings.json key of the bindings.
pub const SETTINGS_KEY: &str = "controls";

/// A key with or without Shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub key: KeyCode,
    pub shift: bool,
}

const fn plain(key: KeyCode) -> Binding {
    Binding { key, shift: false }
}
const fn shifted(key: KeyCode) -> Binding {
    Binding { key, shift: true }
}

/// The keys that can be bound, by their settings.json names. Modifier keys, Esc (it cancels and opens the pause menu
/// everywhere) and the Android back button are left out.
const KEY_NAMES: &[(KeyCode, &str)] = {
    use KeyCode::*;
    &[
        (A, "A"),
        (B, "B"),
        (C, "C"),
        (D, "D"),
        (E, "E"),
        (F, "F"),
        (G, "G"),
        (H, "H"),
        (I, "I"),
        (J, "J"),
        (K, "K"),
        (L, "L"),
        (M, "M"),
        (N, "N"),
        (O, "O"),
        (P, "P"),
        (Q, "Q"),
        (R, "R"),
        (S, "S"),
        (T, "T"),
        (U, "U"),
        (V, "V"),
        (W, "W"),
        (X, "X"),
        (Y, "Y"),
        (Z, "Z"),
        (Key0, "0"),
        (Key1, "1"),
        (Key2, "2"),
        (Key3, "3"),
        (Key4, "4"),
        (Key5, "5"),
        (Key6, "6"),
        (Key7, "7"),
        (Key8, "8"),
        (Key9, "9"),
        (Space, "Space"),
        (Apostrophe, "'"),
        (Comma, ","),
        (Minus, "-"),
        (Period, "."),
        (Slash, "/"),
        (Semicolon, ";"),
        (Equal, "="),
        (LeftBracket, "["),
        (Backslash, "\\"),
        (RightBracket, "]"),
        (GraveAccent, "`"),
        (Enter, "Enter"),
        (Tab, "Tab"),
        (Backspace, "Backspace"),
        (Insert, "Insert"),
        (Delete, "Delete"),
        (Right, "Right"),
        (Left, "Left"),
        (Down, "Down"),
        (Up, "Up"),
        (PageUp, "PageUp"),
        (PageDown, "PageDown"),
        (Home, "Home"),
        (End, "End"),
        (Pause, "Pause"),
        (F1, "F1"),
        (F2, "F2"),
        (F3, "F3"),
        (F4, "F4"),
        (F5, "F5"),
        (F6, "F6"),
        (F7, "F7"),
        (F8, "F8"),
        (F9, "F9"),
        (F10, "F10"),
        (F11, "F11"),
        (F12, "F12"),
        (Kp0, "Num0"),
        (Kp1, "Num1"),
        (Kp2, "Num2"),
        (Kp3, "Num3"),
        (Kp4, "Num4"),
        (Kp5, "Num5"),
        (Kp6, "Num6"),
        (Kp7, "Num7"),
        (Kp8, "Num8"),
        (Kp9, "Num9"),
        (KpDecimal, "Num."),
        (KpDivide, "Num/"),
        (KpMultiply, "Num*"),
        (KpSubtract, "Num-"),
        (KpAdd, "Num+"),
        (KpEnter, "NumEnter"),
    ]
};

fn key_name(k: KeyCode) -> Option<&'static str> {
    KEY_NAMES.iter().find(|(c, _)| *c == k).map(|(_, n)| *n)
}

impl Binding {
    /// The settings.json form: "Shift+" and the key's name ("W", "Shift+F6", "PageUp").
    pub fn name(self) -> String {
        let n = key_name(self.key).unwrap_or("?");
        if self.shift {
            format!("Shift+{n}")
        } else {
            n.to_string()
        }
    }

    /// Reads the settings.json form (case of a letter does not matter).
    pub fn parse(s: &str) -> Option<Binding> {
        let s = s.trim();
        let (shift, rest) = match s.get(..6) {
            Some(p) if p.eq_ignore_ascii_case("shift+") && s.len() > 6 => (true, &s[6..]),
            _ => (false, s),
        };
        let key = KEY_NAMES.iter().find(|(_, n)| n.eq_ignore_ascii_case(rest)).map(|(c, _)| *c)?;
        Some(Binding { key, shift })
    }

    /// As the Keyboard controls screen prints keys: a letter alone in lower case ("g"), with Shift in upper case ("Shift+G");
    /// Shift+/ is "?".
    pub fn label(self) -> String {
        if self == shifted(KeyCode::Slash) {
            return "?".into();
        }
        let n = key_name(self.key).unwrap_or("?");
        let n = if n.len() == 1 && !self.shift { n.to_lowercase() } else { n.to_string() };
        if self.shift {
            format!("Shift+{n}")
        } else {
            n
        }
    }
}

/// When an action's key counts: on the course, or only while the player's pro aims (the shot shapes, which share keys with
/// the course's tools).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctx {
    Course,
    Aim,
}

/// An action that can be given another key.
pub struct Action {
    /// The settings.json name.
    pub id: &'static str,
    /// What it does (a text key in lang/en.txt).
    pub text: &'static str,
    /// The original's key (or the port's, for the port's own actions).
    pub default: Binding,
    pub ctx: Ctx,
    /// With the WASD camera on: a second key (scrolling), or the key that replaces the default (the tools W and S had).
    pub wasd: Wasd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wasd {
    None,
    /// A key added beside the binding.
    Also(KeyCode),
    /// The default moves to this key.
    Moves(KeyCode),
}

macro_rules! act {
    ($id:literal, $text:literal, $b:expr) => {
        Action { id: $id, text: $text, default: $b, ctx: Ctx::Course, wasd: Wasd::None }
    };
    ($id:literal, $text:literal, $b:expr, $w:expr) => {
        Action { id: $id, text: $text, default: $b, ctx: Ctx::Course, wasd: $w }
    };
}
macro_rules! aim {
    ($id:literal, $text:literal, $k:ident) => {
        Action { id: $id, text: $text, default: plain(KeyCode::$k), ctx: Ctx::Aim, wasd: Wasd::None }
    };
}

/// Every action that can be given another key, in the order the Change keys screen lists them (main.rs play_key has their
/// defaults: the exe's two key switches at 0x41c5b2 and 0x41de29, and the port's own keys).
pub const ACTIONS: &[Action] = {
    use KeyCode::*;
    &[
        act!("scroll_up", "STR_CTL_SCROLL_UP", plain(Up), Wasd::Also(W)),
        act!("scroll_down", "STR_CTL_SCROLL_DOWN", plain(Down), Wasd::Also(S)),
        act!("scroll_left", "STR_CTL_SCROLL_LEFT", plain(Left), Wasd::Also(A)),
        act!("scroll_right", "STR_CTL_SCROLL_RIGHT", plain(Right), Wasd::Also(D)),
        act!("zoom_in", "STR_CTL_ZOOM_IN", plain(Z)),
        act!("zoom_out", "STR_CTL_ZOOM_OUT", plain(X)),
        act!("turn_left", "STR_CTL_TURN_LEFT", plain(PageUp)),
        act!("turn_right", "STR_CTL_TURN_RIGHT", plain(Home)),
        act!("turn_back", "STR_CTL_TURN_BACK", plain(Key0)),
        act!("photo_mode", "STR_CTL_PHOTO_MODE", plain(C)),
        act!("hide_interface", "STR_CTL_HIDE_INTERFACE", plain(U)),
        act!("pause", "STR_CTL_PAUSE", shifted(P)),
        act!("speed", "STR_CTL_SPEED", plain(RightBracket)),
        act!("green", "STR_CTL_GREEN", plain(G)),
        act!("fairway", "STR_CTL_FAIRWAY", plain(F)),
        act!("rough", "STR_CTL_ROUGH", plain(R)),
        act!("sand", "STR_CTL_SAND", plain(S), Wasd::Moves(K)),
        act!("trees", "STR_CTL_TREES", plain(T)),
        act!("water", "STR_CTL_WATER", plain(W), Wasd::Moves(Q)),
        act!("ravine", "STR_CTL_RAVINE", plain(V)),
        act!("path", "STR_CTL_PATH", plain(P)),
        act!("bench", "STR_CTL_BENCH", plain(B)),
        act!("home_site", "STR_CTL_HOME_SITE", shifted(B)),
        act!("elevation", "STR_CTL_ELEVATION", plain(E)),
        act!("raise", "STR_CTL_RAISE", plain(Equal)),
        act!("lower", "STR_CTL_LOWER", plain(Minus)),
        act!("turn_building", "STR_CTL_TURN_BUILDING", plain(Tab)),
        act!("put_down", "STR_CTL_PUT_DOWN", plain(Space)),
        act!("open_hole", "STR_CTL_OPEN_HOLE", plain(H)),
        act!("analysis", "STR_CTL_ANALYSIS", plain(Slash)),
        act!("analysis_again", "STR_CTL_ANALYSIS_AGAIN", plain(Period)),
        act!("trees_off", "STR_CTL_TREES_OFF", shifted(T)),
        act!("names", "STR_CTL_NAMES", shifted(N)),
        act!("message", "STR_CTL_MESSAGE", shifted(Slash)),
        act!("music", "STR_CTL_MUSIC", plain(M)),
        aim!("shot_straight", "STR_CTL_SHOT_STRAIGHT", S),
        aim!("shot_fade", "STR_CTL_SHOT_FADE", F),
        aim!("shot_draw", "STR_CTL_SHOT_DRAW", D),
        aim!("shot_high", "STR_CTL_SHOT_HIGH", H),
        aim!("shot_low", "STR_CTL_SHOT_LOW", L),
        act!("cancel_round", "STR_CTL_CANCEL_ROUND", plain(N)),
        act!("tournament", "STR_CTL_TOURNAMENT", shifted(J)),
        act!("save", "STR_CTL_SAVE", shifted(S)),
        act!("load", "STR_CTL_LOAD", shifted(L)),
        act!("champ_course", "STR_CTL_CHAMP_COURSE", shifted(C)),
        act!("screenshot", "STR_CTL_SCREENSHOT", shifted(F6)),
        act!("routing", "STR_CTL_ROUTING", shifted(R)),
        act!("world", "STR_CTL_WORLD", shifted(W)),
        act!("report_f1", "STR_KEYS_F1", plain(F1)),
        act!("report_f2", "STR_KEYS_F2", plain(F2)),
        act!("report_f3", "STR_KEYS_F3", plain(F3)),
        act!("report_f4", "STR_KEYS_F4", plain(F4)),
        act!("report_f5", "STR_KEYS_F5", plain(F5)),
        act!("report_f6", "STR_KEYS_F6", plain(F6)),
        act!("report_f7", "STR_KEYS_F7", plain(F7)),
        act!("report_f8", "STR_KEYS_F8", plain(F8)),
        act!("report_f9", "STR_KEYS_F9", plain(F9)),
        act!("report_f10", "STR_KEYS_F10", plain(F10)),
        // the OpenSGolf extras' keys (extras.rs; each does nothing while its extra is off)
        act!("people", "STR_CTL_PEOPLE", shifted(G)),
        act!("history", "STR_CTL_HISTORY", shifted(M)),
        act!("name_tags", "STR_CTL_NAME_TAGS", shifted(H)),
        act!("path_view", "STR_CTL_PATH_VIEW", shifted(V)),
    ]
};

/// The keys no action may take: the digits jump to holes (Shift too), Esc cancels, the modifiers are modifiers.
pub fn reserved(b: Binding) -> bool {
    use KeyCode::*;
    matches!(b.key, Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7 | Key8 | Key9) || key_name(b.key).is_none()
}

/// The player's choices as kept in settings.json.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The WASD camera: W, A, S and D scroll as the arrow keys do; the water and sand tools move to Q and K.
    pub wasd: bool,
    /// Keys moved from their defaults, by action id ("water": "Shift+Q"). An action set to "" has no key.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub keys: BTreeMap<String, String>,
}

thread_local! {
    static SETTINGS: RefCell<Settings> = RefCell::new(Settings::default());
}

/// The bindings in use.
pub fn settings() -> Settings {
    SETTINGS.with(|s| s.borrow().clone())
}

/// Puts bindings in use (not saved).
pub fn set(s: Settings) {
    SETTINGS.with(|c| *c.borrow_mut() = s);
}

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// At start up: the saved bindings.
pub fn init() {
    let s: Settings = serde_json::from_value(settings_json()[SETTINGS_KEY].clone()).unwrap_or_default();
    set(s);
}

/// Puts the bindings in use and saves them into settings.json, keeping its other keys (the key is removed when everything
/// is back to the defaults).
pub fn save(s: Settings) {
    let mut v = settings_json();
    if s == Settings::default() {
        if let Some(o) = v.as_object_mut() {
            o.remove(SETTINGS_KEY);
        }
    } else if let Ok(c) = serde_json::to_value(&s) {
        v[SETTINGS_KEY] = c;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE), t.as_bytes());
    }
    set(s);
}

impl Settings {
    /// Action `i`'s key before the player's own changes: its default, or with the WASD camera the key it moves to.
    pub fn base(&self, i: usize) -> Option<Binding> {
        let a = &ACTIONS[i];
        match a.wasd {
            Wasd::Moves(k) if self.wasd => Some(plain(k)),
            _ => Some(a.default),
        }
    }

    /// Action `i`'s key (None: no key).
    pub fn binding(&self, i: usize) -> Option<Binding> {
        match self.keys.get(ACTIONS[i].id) {
            Some(s) if s.is_empty() => None,
            Some(s) => Binding::parse(s).or_else(|| self.base(i)),
            None => self.base(i),
        }
    }

    /// The WASD camera's second key for action `i`.
    pub fn extra(&self, i: usize) -> Option<Binding> {
        match ACTIONS[i].wasd {
            Wasd::Also(k) if self.wasd => Some(plain(k)),
            _ => None,
        }
    }

    /// Every key of action `i`.
    pub fn keys_of(&self, i: usize) -> impl Iterator<Item = Binding> {
        self.binding(i).into_iter().chain(self.extra(i))
    }

    /// Gives action `i` key `b` (None: no key); a key equal to the action's own default is not kept as a change.
    pub fn bind(&mut self, i: usize, b: Option<Binding>) {
        let id = ACTIONS[i].id.to_string();
        if b == self.base(i) {
            self.keys.remove(&id);
        } else {
            self.keys.insert(id, b.map(Binding::name).unwrap_or_default());
        }
    }

    /// The other actions that answer to a key of action `i` in the same situation (the shot shapes only clash with each
    /// other: while the pro aims they come first).
    pub fn conflicts(&self, i: usize) -> Vec<usize> {
        let mine: Vec<Binding> = self.keys_of(i).collect();
        (0..ACTIONS.len()).filter(|&j| j != i && ACTIONS[j].ctx == ACTIONS[i].ctx && self.keys_of(j).any(|b| mine.contains(&b))).collect()
    }

    /// The key a key press stands for on the course screen: the default key of the action bound to it, with that action;
    /// the key itself when no action owns it; None when it is the default of an action that was moved and nothing is bound
    /// to it now. While the pro aims, the shot shapes are looked up first.
    pub fn translate(&self, key: KeyCode, shift: bool, aiming: bool) -> Option<(KeyCode, bool, Option<usize>)> {
        let b = Binding { key, shift };
        let order = |ctx: Ctx| (0..ACTIONS.len()).filter(move |&i| ACTIONS[i].ctx == ctx);
        let looked: Vec<usize> = if aiming { order(Ctx::Aim).chain(order(Ctx::Course)).collect() } else { order(Ctx::Course).collect() };
        if let Some(&i) = looked.iter().find(|&&i| self.keys_of(i).any(|k| k == b)) {
            let d = ACTIONS[i].default;
            return Some((d.key, d.shift, Some(i)));
        }
        if looked.iter().any(|&i| ACTIONS[i].default == b) {
            return None;
        }
        Some((key, shift, None))
    }
}

/// [`Settings::translate`] with the bindings in use.
pub fn translate(key: KeyCode, shift: bool, aiming: bool) -> Option<(KeyCode, bool, Option<usize>)> {
    SETTINGS.with(|s| s.borrow().translate(key, shift, aiming))
}

/// The index of the action with settings.json name `id`.
pub fn action(id: &str) -> usize {
    ACTIONS.iter().position(|a| a.id == id).expect("a known action")
}

/// True when action `i` is a shot shape (only while the pro aims).
pub fn is_aim(i: usize) -> bool {
    ACTIONS[i].ctx == Ctx::Aim
}

/// The Keyboard controls screen's rows (keys_ui.rs) that print a key of these actions: when the player changed any of them,
/// the row prints the keys in use instead of its fixed text.
const ROW_ACTIONS: &[(&str, &[&str])] = &[
    ("STR_KEYS_K_ARROWS", &["scroll_up", "scroll_down", "scroll_left", "scroll_right"]),
    ("STR_KEYS_K_ZOOM", &["zoom_in", "zoom_out"]),
    ("STR_KEYS_K_TURN", &["turn_left", "turn_right"]),
    ("STR_KEYS_K_TURN_BACK", &["turn_back"]),
    ("STR_KEYS_K_PHOTO_MODE", &["photo_mode"]),
    ("STR_KEYS_K_HIDE_INTERFACE", &["hide_interface"]),
    ("STR_KEYS_K_PAUSE", &["pause"]),
    ("STR_KEYS_K_SPEED", &["speed"]),
    ("STR_KEYS_K_GREEN", &["green"]),
    ("STR_KEYS_K_FAIRWAY", &["fairway"]),
    ("STR_KEYS_K_ROUGH", &["rough"]),
    ("STR_KEYS_K_SAND", &["sand"]),
    ("STR_KEYS_K_TREES", &["trees"]),
    ("STR_KEYS_K_WATER", &["water"]),
    ("STR_KEYS_K_RAVINE", &["ravine"]),
    ("STR_KEYS_K_PATH", &["path"]),
    ("STR_KEYS_K_BENCH", &["bench"]),
    ("STR_KEYS_K_HOME_SITE", &["home_site"]),
    ("STR_KEYS_K_ELEVATION", &["elevation"]),
    ("STR_KEYS_K_RAISE", &["raise", "lower"]),
    ("STR_KEYS_K_TAB", &["turn_building"]),
    ("STR_KEYS_K_SPACE", &["put_down"]),
    ("STR_KEYS_K_OPEN_HOLE", &["open_hole"]),
    ("STR_KEYS_K_ANALYSIS", &["analysis"]),
    ("STR_KEYS_K_ANALYSIS_AGAIN", &["analysis_again"]),
    ("STR_KEYS_K_TREES_OFF", &["trees_off"]),
    ("STR_KEYS_K_NAMES", &["names"]),
    ("STR_KEYS_K_MESSAGE", &["message"]),
    ("STR_KEYS_K_MUSIC", &["music"]),
    ("STR_KEYS_K_F1", &["report_f1"]),
    ("STR_KEYS_K_F2", &["report_f2"]),
    ("STR_KEYS_K_F3", &["report_f3"]),
    ("STR_KEYS_K_F4", &["report_f4"]),
    ("STR_KEYS_K_F5", &["report_f5"]),
    ("STR_KEYS_K_F6", &["report_f6"]),
    ("STR_KEYS_K_F7", &["report_f7"]),
    ("STR_KEYS_K_F8", &["report_f8"]),
    ("STR_KEYS_K_F9", &["report_f9"]),
    ("STR_KEYS_K_F10", &["report_f10"]),
    ("STR_KEYS_K_ROUTING", &["routing"]),
    ("STR_KEYS_K_WORLD", &["world"]),
    ("STR_KEYS_K_SHOTS", &["shot_straight", "shot_fade", "shot_draw", "shot_high", "shot_low"]),
    ("STR_KEYS_K_CANCEL_ROUND", &["cancel_round"]),
    ("STR_KEYS_K_TOURNAMENT", &["tournament"]),
    ("STR_KEYS_K_SAVE", &["save"]),
    ("STR_KEYS_K_LOAD", &["load"]),
    ("STR_KEYS_K_CHAMP_COURSE", &["champ_course"]),
    ("STR_KEYS_K_SCREENSHOT", &["screenshot"]),
    ("STR_KEYS_K_PEOPLE", &["people"]),
    ("STR_KEYS_K_HISTORY", &["history"]),
    ("STR_KEYS_K_TAGS", &["name_tags"]),
    ("STR_KEYS_K_PATHVIEW", &["path_view"]),
];

/// The text a Keyboard controls row prints for its key: None (the row's own text) unless the player moved one of its keys.
pub fn row_label(row: &str) -> Option<String> {
    let (_, ids) = ROW_ACTIONS.iter().find(|(r, _)| *r == row)?;
    let s = settings();
    let idx: Vec<usize> = ids.iter().map(|id| action(id)).collect();
    if idx.iter().all(|&i| s.binding(i) == Some(ACTIONS[i].default)) {
        return None;
    }
    let sep = if idx.len() > 2 { " " } else { " / " };
    Some(idx.iter().map(|&i| s.binding(i).map(Binding::label).unwrap_or_else(|| "-".into())).collect::<Vec<_>>().join(sep))
}

/// The key that shows the interface again, as printed (the hint while it is hidden).
pub fn hide_key_label() -> String {
    settings().binding(action("hide_interface")).map(Binding::label).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_pass_every_key_through_unchanged() {
        let s = Settings::default();
        for a in ACTIONS.iter().filter(|a| a.ctx == Ctx::Course) {
            let (k, sh, _) = s.translate(a.default.key, a.default.shift, false).unwrap();
            assert_eq!((k, sh), (a.default.key, a.default.shift), "{}", a.id);
        }
        // keys no action owns: holes, the keypad, a free letter
        for k in [KeyCode::Key1, KeyCode::KpAdd, KeyCode::Y] {
            assert_eq!(s.translate(k, false, false), Some((k, false, None)));
            assert_eq!(s.translate(k, true, false), Some((k, true, None)));
        }
        // while the pro aims, s is the straight shot (its default stays S)
        let (k, _, i) = s.translate(KeyCode::S, false, true).unwrap();
        assert_eq!(k, KeyCode::S);
        assert!(is_aim(i.unwrap()));
    }

    #[test]
    fn a_moved_key_stands_for_its_default_and_frees_the_old_one() {
        let mut s = Settings::default();
        let water = action("water");
        s.bind(water, Some(plain(KeyCode::Q)));
        assert_eq!(s.translate(KeyCode::Q, false, false), Some((KeyCode::W, false, Some(water))));
        assert_eq!(s.translate(KeyCode::W, false, false), None);
        // Shift+W is still the world map
        assert_eq!(s.translate(KeyCode::W, true, false).map(|t| t.0), Some(KeyCode::W));
        // back to its default: no change is kept
        s.bind(water, Some(plain(KeyCode::W)));
        assert!(s.keys.is_empty());
    }

    #[test]
    fn wasd_scrolls_and_moves_water_and_sand() {
        let s = Settings { wasd: true, ..Default::default() };
        assert_eq!(s.translate(KeyCode::W, false, false).map(|t| t.0), Some(KeyCode::Up));
        assert_eq!(s.translate(KeyCode::A, false, false).map(|t| t.0), Some(KeyCode::Left));
        assert_eq!(s.translate(KeyCode::S, false, false).map(|t| t.0), Some(KeyCode::Down));
        assert_eq!(s.translate(KeyCode::D, false, false).map(|t| t.0), Some(KeyCode::Right));
        // the arrows still scroll
        assert_eq!(s.translate(KeyCode::Up, false, false).map(|t| t.0), Some(KeyCode::Up));
        assert_eq!(s.translate(KeyCode::Q, false, false).map(|t| t.0), Some(KeyCode::W));
        assert_eq!(s.translate(KeyCode::K, false, false).map(|t| t.0), Some(KeyCode::S));
        // the shot shapes keep s and d while the pro aims
        assert!(is_aim(s.translate(KeyCode::D, false, true).unwrap().2.unwrap()));
        for i in 0..ACTIONS.len() {
            assert!(s.conflicts(i).is_empty(), "{} clashes", ACTIONS[i].id);
        }
    }

    #[test]
    fn conflicts_are_found_in_the_same_situation_only() {
        let mut s = Settings::default();
        assert!((0..ACTIONS.len()).all(|i| s.conflicts(i).is_empty()));
        s.bind(action("zoom_in"), Some(plain(KeyCode::G)));
        assert_eq!(s.conflicts(action("zoom_in")), vec![action("green")]);
        // a shot shape on the trees' key: different situations
        s.bind(action("shot_low"), Some(plain(KeyCode::T)));
        assert!(s.conflicts(action("shot_low")).is_empty());
    }

    #[test]
    fn names_read_back() {
        for (k, _) in KEY_NAMES {
            for shift in [false, true] {
                let b = Binding { key: *k, shift };
                assert_eq!(Binding::parse(&b.name()), Some(b));
            }
        }
        assert_eq!(Binding::parse("shift+f6"), Some(shifted(KeyCode::F6)));
        assert_eq!(plain(KeyCode::G).label(), "g");
        assert_eq!(shifted(KeyCode::P).label(), "Shift+P");
        assert_eq!(shifted(KeyCode::Slash).label(), "?");
    }
}
