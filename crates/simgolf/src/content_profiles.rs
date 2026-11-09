//! Content profiles and the packs a saved game was made with (the port's own; docs/MODDING.md, "Profiles").
//!
//! A profile is a named set of content choices: the base set, the mods with their order and switches, and the translation.
//! Preferences > Content saves the current choices as a profile and loads one back (the choices then apply at the next
//! start, as every change on that screen does). Profiles are kept under "content_profiles" in settings.json.
//!
//! A saved game records the packs that were loaded when it was saved (ids, versions and kinds, in load order; the save's
//! `packs` field, versioned on its own). Loading it from the Load screen while other packs are loaded asks first: switch
//! to the save's packs (the game starts again on them and loads the save), load it anyway, or not load it. Games saved
//! before this was recorded load without the question.
use crate::app::*;
use crate::content::fit;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{rgb, rgba, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use serde::{Deserialize, Serialize};
use sg_core::lang::{tr, trf};
use sg_core::packs::{self, Found, Kind, ModChoice, Plan, Settings, Status};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The settings.json key of the profiles.
const PROFILES_KEY: &str = "content_profiles";

/// One pack loaded when a game was saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackStamp {
    pub id: String,
    pub version: String,
    pub kind: Kind,
}

/// The save's record of its packs. `version` counts this record's own format (1 so far).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPacks {
    pub version: u32,
    /// The packs in load order, the base set (or "original") first.
    pub packs: Vec<PackStamp>,
}

/// The packs a plan loads (the rows marked active), in load order.
pub fn stamps(plan: &Plan) -> Vec<PackStamp> {
    plan.rows
        .iter()
        .filter(|r| r.status == Status::Active)
        .map(|r| PackStamp { id: r.manifest.id.clone(), version: r.manifest.version.clone(), kind: r.manifest.kind })
        .collect()
}

/// The content choices that load `want` (the other installed mods listed after them, switched off), and the packs of
/// `want` that are not installed (or not in that version), as "name version".
pub fn settings_for(want: &[PackStamp], found: &Found) -> (Settings, Vec<String>) {
    let installed = |p: &PackStamp| {
        p.id == packs::ORIGINAL
            || (p.id == packs::FREE && p.kind == Kind::Base)
            || found.packs.iter().any(|f| f.manifest.id == p.id && f.manifest.version == p.version)
    };
    let missing = want.iter().filter(|p| !installed(p)).map(|p| format!("{} {}", p.id, p.version)).collect();
    let mut s = Settings::default();
    for p in want {
        match p.kind {
            Kind::Base if p.id != packs::ORIGINAL => s.base = Some(p.id.clone()),
            Kind::Mod => s.mods.push(ModChoice { id: p.id.clone(), enabled: true }),
            Kind::Translation => s.translation = Some(p.id.clone()),
            _ => {}
        }
    }
    (s.complete(found), missing)
}

/// "name version, name version" for a message.
fn describe(list: &[PackStamp]) -> String {
    list.iter().map(|p| format!("{} {}", p.id, p.version)).collect::<Vec<_>>().join(", ")
}

/// The base pack of a list (its graphics), in plain words.
fn art_name(list: &[PackStamp]) -> Option<String> {
    let base = list.iter().find(|p| p.kind == Kind::Base)?;
    Some(match base.id.as_str() {
        sg_core::packs::ORIGINAL => tr("STR_SAVE_PACKS_ART_ORIGINAL").to_string(),
        sg_core::packs::FREE => tr("STR_SAVE_PACKS_ART_FREE").to_string(),
        id => trf("STR_SAVE_PACKS_ART_OTHER", &[("ID", &id)]),
    })
}

/// What Load Previous Game says of a save made with other packs (the port's own): the graphics when the base pack differs
/// ("saved with the original SimGolf art; you're using the free graphics"), else that other mods or a translation were
/// in use.
fn differ_text(saved: &[PackStamp], now: &[PackStamp]) -> String {
    match (art_name(saved), art_name(now)) {
        (Some(a), Some(b)) if a != b => trf("STR_SAVE_PACKS_DIFFER_ART", &[("SAVED", &a), ("NOW", &b)]),
        _ => tr("STR_SAVE_PACKS_DIFFER").to_string(),
    }
}

// ---- profiles ------------------------------------------------------------------------------------------------------------------

fn settings_path() -> PathBuf {
    save_dir().join(crate::hd::SETTINGS_FILE)
}

fn read_settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(settings_path())
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The saved profiles by name.
pub fn profiles() -> BTreeMap<String, Settings> {
    serde_json::from_value(read_settings_json()[PROFILES_KEY].clone()).unwrap_or_default()
}

fn save_profiles(p: &BTreeMap<String, Settings>) {
    let mut v = read_settings_json();
    if p.is_empty() {
        if let Some(o) = v.as_object_mut() {
            o.remove(PROFILES_KEY);
        }
    } else if let Ok(j) = serde_json::to_value(p) {
        v[PROFILES_KEY] = j;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(settings_path(), t.as_bytes());
    }
}

/// A profile's summary line: base set, mods switched on, translation.
fn summary(s: &Settings) -> String {
    let mods: Vec<&str> = s.mods.iter().filter(|m| m.enabled).map(|m| m.id.as_str()).collect();
    trf(
        "STR_PROFILE_LINE",
        &[
            ("BASE", &s.base.clone().unwrap_or_else(|| packs::ORIGINAL.to_string())),
            ("MODS", &if mods.is_empty() { tr("STR_PROFILE_NO_MODS").to_string() } else { mods.join(", ") }),
            ("TRANSLATION", &s.translation.clone().unwrap_or_else(|| "-".into())),
        ],
    )
}

/// The Content screen's profiles list, while it is open.
#[derive(Default)]
pub struct ProfilesView {
    pub names: Vec<String>,
    pub scroll: usize,
    pub picked: Option<usize>,
    pub status: String,
    pub failed: bool,
}

/// A saved game waiting for the player's answer: its path and the packs it was made with.
pub struct PendingLoad {
    pub path: PathBuf,
    pub want: Vec<PackStamp>,
}

const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
const ROWS: usize = 8;
const ROW_H: f32 = 46.0;
const TOP: f32 = 96.0;

/// The Content screen's Save profile... and Profiles... buttons (top right, beside the help line).
pub fn profile_buttons() -> [Rect; 2] {
    [Rect::new(BOX.x + BOX.w - 232.0, BOX.y + 34.0, 104.0, 18.0), Rect::new(BOX.x + BOX.w - 122.0, BOX.y + 34.0, 104.0, 18.0)]
}

/// The profiles list's Back button (where Find more... is on the list of packs).
fn back_button() -> Rect {
    Rect::new(BOX.x + BOX.w - 200.0, BOX.y + BOX.h - 50.0, 116.0, 22.0)
}

fn row_button(k: usize, which: usize) -> Rect {
    let y = TOP + k as f32 * ROW_H + 4.0;
    Rect::new(BOX.x + BOX.w - if which == 0 { 160.0 } else { 90.0 }, y, 64.0, 20.0)
}

fn name_box() -> Rect {
    Rect::new(220.0, 250.0, 360.0, 100.0)
}

impl App {
    /// The Content screen's top buttons: Save profile... asks for a name, Profiles... lists them. True when one was hit.
    pub fn profile_buttons_click(&mut self, x: f32, y: f32) -> bool {
        let [save, list] = profile_buttons();
        if save.has(x, y) {
            self.content.profile_name = Some(String::new());
            return true;
        }
        if list.has(x, y) {
            self.open_profiles(String::new(), false);
            return true;
        }
        false
    }

    pub fn open_profiles(&mut self, status: String, failed: bool) {
        let names = profiles().into_keys().collect();
        let status = if status.is_empty() { tr("STR_PROFILE_HELP").to_string() } else { status };
        self.content.profiles = Some(ProfilesView { names, status, failed, ..Default::default() });
    }

    /// Keys on the name box: Enter saves the profile (replacing one of that name), Esc cancels. True when the box took it.
    pub fn profile_name_key(&mut self, k: miniquad::KeyCode) -> bool {
        use miniquad::KeyCode;
        let Some(name) = self.content.profile_name.as_mut() else { return false };
        match k {
            KeyCode::Escape => self.content.profile_name = None,
            KeyCode::Backspace => {
                name.pop();
            }
            KeyCode::Enter | KeyCode::KpEnter => {
                let n = name.trim().to_string();
                if n.is_empty() {
                    self.ui_sound(0x18);
                    return true;
                }
                self.content.profile_name = None;
                let mut all = profiles();
                all.insert(n.clone(), self.content.settings.clone());
                save_profiles(&all);
                self.open_profiles(trf("STR_PROFILE_SAVED", &[("NAME", &n)]), false);
            }
            _ => {}
        }
        true
    }

    pub fn profile_char(&mut self, c: char) {
        if let Some(name) = self.content.profile_name.as_mut() {
            if !c.is_control() && name.chars().count() < 32 {
                name.push(c);
            }
        }
    }

    /// Loads profile `i`: its choices become the saved ones (they apply at the next start).
    fn load_profile(&mut self, i: usize) {
        let Some(v) = self.content.profiles.as_ref() else { return };
        let Some(name) = v.names.get(i).cloned() else { return };
        let Some(p) = profiles().remove(&name) else { return };
        // packs the profile names that are not installed are left out
        let gone: Vec<String> = p
            .base
            .iter()
            .chain(p.translation.iter())
            .chain(p.mods.iter().filter(|m| m.enabled).map(|m| &m.id))
            .filter(|id| *id != packs::FREE && !self.content.found.packs.iter().any(|f| &f.manifest.id == *id))
            .cloned()
            .collect();
        let mut s = p.complete(&self.content.found);
        if s.base.as_ref().is_some_and(|b| gone.contains(b)) {
            s.base = None;
        }
        if s.translation.as_ref().is_some_and(|t| gone.contains(t)) {
            s.translation = None;
        }
        self.content.settings = s;
        crate::content::save_choices(&self.content.settings);
        self.content.rediscover();
        let (msg, failed) = if gone.is_empty() {
            (trf("STR_PROFILE_LOADED", &[("NAME", &name)]), false)
        } else {
            (trf("STR_PROFILE_LOADED_MISSING", &[("NAME", &name), ("MISSING", &gone.join(", "))]), true)
        };
        let v = self.content.profiles.as_mut().expect("open");
        v.picked = Some(i);
        v.status = msg;
        v.failed = failed;
    }

    fn delete_profile(&mut self, i: usize) {
        let Some(v) = self.content.profiles.as_mut() else { return };
        let Some(name) = v.names.get(i).cloned() else { return };
        let mut all = profiles();
        all.remove(&name);
        save_profiles(&all);
        self.open_profiles(trf("STR_PROFILE_DELETED", &[("NAME", &name)]), false);
    }

    /// A click on the profiles list (or the name box).
    pub fn profiles_click(&mut self, x: f32, y: f32) {
        if self.content.profile_name.is_some() {
            // a click outside the box cancels it
            if !name_box().has(x, y) {
                self.content.profile_name = None;
            }
            return;
        }
        if back_button().has(x, y) {
            self.content.profiles = None;
            return;
        }
        let Some(v) = self.content.profiles.as_mut() else { return };
        if !(TOP..TOP + ROWS as f32 * ROW_H).contains(&y) {
            return;
        }
        let k = ((y - TOP) / ROW_H) as usize;
        let i = v.scroll + k;
        if i >= v.names.len() {
            return;
        }
        if row_button(k, 0).has(x, y) {
            return self.load_profile(i);
        }
        if row_button(k, 1).has(x, y) {
            return self.delete_profile(i);
        }
        v.picked = Some(i);
    }

    /// Keys on the profiles list: Up and Down pick, Enter loads, Delete deletes, Esc goes back to the packs.
    pub fn profiles_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let Some(v) = self.content.profiles.as_mut() else { return };
        let n = v.names.len();
        match k {
            KeyCode::Escape | KeyCode::Backspace => self.content.profiles = None,
            KeyCode::Up | KeyCode::Down if n > 0 => {
                let p = match (v.picked, k == KeyCode::Down) {
                    (None, _) => 0,
                    (Some(p), true) => (p + 1).min(n - 1),
                    (Some(p), false) => p.saturating_sub(1),
                };
                v.picked = Some(p);
                v.scroll = v.scroll.min(p).max((p + 1).saturating_sub(ROWS));
            }
            KeyCode::Enter | KeyCode::KpEnter => {
                if let Some(p) = v.picked {
                    self.load_profile(p);
                }
            }
            KeyCode::Delete => {
                if let Some(p) = v.picked {
                    self.delete_profile(p);
                }
            }
            _ => {}
        }
    }

    pub fn draw_profiles(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, yellow, navy, green) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x0180));
        let red = rgb(0.85, 0.1, 0.1);
        let all = profiles();
        let Some(v) = self.content.profiles.as_ref() else { return };
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_PROFILE_TITLE"), yellow, true);
        let (px, py) = self.info.pointer;
        if v.names.is_empty() {
            s.put(g, F_MANUAL15, BOX.x + 24.0, TOP + 4.0, tr("STR_PROFILE_NONE"), white);
        }
        for (k, name) in v.names.iter().enumerate().skip(v.scroll).take(ROWS) {
            let row = k - v.scroll;
            let y = TOP + row as f32 * ROW_H;
            if v.picked == Some(k) || Rect::new(BOX.x + 10.0, y, BOX.w - 20.0, ROW_H - 2.0).has(px, py) {
                s.fill(g, BOX.x + 12.0, y, BOX.w - 24.0, ROW_H - 2.0, rgb(0.66, 0.66, 0.84));
            }
            s.put(g, F_MANUAL15, BOX.x + 24.0, y + 4.0, &fit(name, BOX.w - 200.0), white);
            let line = all.get(name).map(summary).unwrap_or_default();
            s.put(g, F_ARIAL10, BOX.x + 24.0, y + 24.0, &fit(&line, BOX.w - 200.0), green);
            for (which, label) in [(0, tr("STR_PROFILE_LOAD")), (1, tr("STR_PROFILE_DELETE"))] {
                let r = row_button(row, which);
                crate::glass_ui::button_fill(g, &s, r.x, r.y, r.w, r.h, r.has(px, py));
                s.put_centered(g, F_ARIAL10, r.x + r.w / 2.0, r.y + 4.0, label, navy);
            }
        }
        let mut ty = TOP + ROWS as f32 * ROW_H + 30.0;
        for l in crate::ui::wrap_text(&v.status, 10.0, BOX.w - 260.0).into_iter().take(3) {
            s.put(g, F_ARIAL10, BOX.x + 24.0, ty, &l, if v.failed { red } else { navy });
            ty += 12.0;
        }
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 26.0, tr("STR_CONTENT_NOTE"), navy);
        let b = back_button();
        crate::glass_ui::button_fill(g, &s, b.x, b.y, b.w, b.h, b.has(px, py));
        s.put_centered(g, F_ARIAL10, b.x + b.w / 2.0, b.y + 5.0, tr("STR_INDEX_BACK"), navy);
        if self.art.trans.tex.is_some() {
            s.image_part(g, &self.art.trans, BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }

    /// The name box over the Content screen while a profile is being named.
    pub fn draw_profile_name(&self, g: &mut Gfx) {
        let Some(name) = &self.content.profile_name else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        let b = name_box();
        s.fill(g, b.x - 3.0, b.y - 3.0, b.w + 6.0, b.h + 6.0, rgba(0.55, 0.55, 0.85, 0.95));
        s.fill(g, b.x, b.y, b.w, b.h, rgba(0.92, 0.92, 1.0, 0.97));
        s.text_centered(g, 400.0, b.y + 26.0, tr("STR_PROFILE_NAME_PROMPT"), 16.0, rgb(0.15, 0.1, 0.4));
        s.fill(g, b.x + 20.0, b.y + 40.0, b.w - 40.0, 28.0, rgb(1.0, 1.0, 1.0));
        let caret = if (self.clock * 2.0) as i64 % 2 == 0 { "|" } else { "" };
        s.text(g, b.x + 26.0, b.y + 60.0, &format!("{name}{caret}"), 16.0, rgb(0.05, 0.05, 0.2));
        s.put_centered(g, F_ARIAL10, 400.0, b.y + 80.0, tr("STR_PROFILE_NAME_KEYS"), rgb(0.15, 0.1, 0.4));
        g.flush();
    }

    // ---- the save's packs ------------------------------------------------------------------------------------------------------

    /// The record of the packs in use, for a save.
    pub fn saved_packs(&self) -> Option<SavedPacks> {
        Some(SavedPacks { version: 1, packs: self.content.stamps.clone() })
    }

    /// Before a saved game is loaded from the Load screen: when it was made with other packs than the ones loaded, asks
    /// what to do and returns true (the load waits for the answer). A second call after "load it anyway" lets it through.
    pub fn save_packs_question(&mut self, path: &Path) -> bool {
        if self.ctl_load_anyway() {
            return false;
        }
        let Some(rec) = read_save(path).ok().and_then(|s| s.packs) else { return false };
        if rec.packs == self.content.stamps {
            return false;
        }
        let (_, missing) = settings_for(&rec.packs, &self.content.found);
        // plain words on screen; the packs' ids and versions go to the log
        println!(
            "content: {} was saved with {}; running with {}; not installed: {}",
            path.display(),
            describe(&rec.packs),
            describe(&self.content.stamps),
            if missing.is_empty() { "none".to_string() } else { missing.join(", ") }
        );
        let mut text = differ_text(&rec.packs, &self.content.stamps);
        if !missing.is_empty() {
            text.push(' ');
            text.push_str(tr("STR_SAVE_PACKS_MISSING"));
        }
        let mut lines = crate::gamefiles::wrap(&text, 560.0);
        use crate::gamefiles::{Action, Notice};
        let (confirm, alt) = if missing.is_empty() {
            lines.push(format!(
                " {}",
                tr(if cfg!(any(target_arch = "wasm32", target_os = "android")) {
                    "STR_SAVE_PACKS_SWITCH_LATER"
                } else {
                    "STR_SAVE_PACKS_SWITCH"
                })
            ));
            lines.push(format!(" {}", tr("STR_SAVE_PACKS_LOAD_ANYWAY")));
            (Action::SavePacksSwitch, Some(Action::SavePacksLoad))
        } else {
            lines.push(format!(" {}", tr("STR_SAVE_PACKS_LOAD_ANYWAY")));
            (Action::SavePacksLoad, None)
        };
        lines.push(format!(" {}", tr("STR_SAVE_PACKS_CANCEL")));
        self.notice = Some(Notice { lines, confirm: Some(confirm), alt, sel: None });
        self.content.pending_load = Some(PendingLoad { path: path.to_path_buf(), want: rec.packs });
        true
    }

    fn ctl_load_anyway(&mut self) -> bool {
        std::mem::take(&mut self.content.load_anyway)
    }

    /// The player's answer: switch to the save's packs (and start again to load it), or load it with the packs in use.
    pub fn save_packs_answer(&mut self, switch: bool) {
        let Some(p) = self.content.pending_load.take() else { return };
        if !switch {
            self.content.load_anyway = true;
            if let Some(f) = self.title.files.as_mut() {
                f.loading = 1;
            }
            return;
        }
        let (s, _) = settings_for(&p.want, &self.content.found);
        self.content.settings = s;
        crate::content::save_choices(&self.content.settings);
        self.content.rediscover();
        println!("content: switching to the packs of {}", p.path.display());
        if cfg!(any(target_arch = "wasm32", target_os = "android")) {
            self.show_notice(tr("STR_SAVE_PACKS_RESTART_YOURSELF"));
            return;
        }
        self.release_audio();
        let e = crate::gamefiles::restart_with(tr("STR_SAVE_PACKS_SWITCHED"), Some(&p.path));
        eprintln!("restart: {e}");
        self.show_notice(tr("STR_GAME_FILES_RESTART"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(id: &str, kind: Kind) -> PackStamp {
        PackStamp { id: id.into(), version: "1.0.0".into(), kind }
    }

    #[test]
    fn a_save_with_other_graphics_is_told_in_plain_words() {
        let orig = [stamp(packs::ORIGINAL, Kind::Base)];
        let free = [stamp(packs::FREE, Kind::Base), stamp("fr", Kind::Translation)];
        let t = differ_text(&orig, &free);
        assert!(t.contains(tr("STR_SAVE_PACKS_ART_ORIGINAL")) && t.contains(tr("STR_SAVE_PACKS_ART_FREE")), "{t}");
        assert!(!t.contains("1.0.0") && !t.contains("opensgolf-free"), "no ids or versions: {t}");
        // the same graphics with other mods or a translation
        assert_eq!(differ_text(&orig, &[stamp(packs::ORIGINAL, Kind::Base), stamp("m", Kind::Mod)]), tr("STR_SAVE_PACKS_DIFFER"));
        assert!(differ_text(&[stamp("my-art", Kind::Base)], &orig).contains("my-art"));
    }

    #[test]
    fn settings_for_a_save_list_its_packs_in_order() {
        let found = Found::default();
        let want = [stamp("free-art", Kind::Base), stamp("b-mod", Kind::Mod), stamp("a-mod", Kind::Mod), stamp("fr", Kind::Translation)];
        let (s, missing) = settings_for(&want, &found);
        assert_eq!(s.base.as_deref(), Some("free-art"));
        assert_eq!(s.translation.as_deref(), Some("fr"));
        // nothing is installed: the mods are dropped by complete(), and every pack is reported
        assert!(s.mods.is_empty());
        assert_eq!(missing.len(), 4);
        // the original base set is always there
        let (s, missing) = settings_for(&[stamp(packs::ORIGINAL, Kind::Base)], &found);
        assert_eq!(s, Settings::default());
        assert!(missing.is_empty());
    }

    #[test]
    fn the_record_reads_back() {
        let r = SavedPacks { version: 1, packs: vec![stamp(packs::ORIGINAL, Kind::Base), stamp("m", Kind::Mod)] };
        let j = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<SavedPacks>(&j).unwrap(), r);
        assert!(j.contains("\"kind\":\"mod\""));
    }
}
