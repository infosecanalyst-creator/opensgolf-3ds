//! Content packs (docs/MODDING.md, sections 1 and 2): `pack.toml`, finding packs in folders and .zip files, checking them,
//! putting them in load order, and loading the enabled ones (their files over the game folder through crate::vfs, their
//! `data/*.toml` merged over the built-in tables through crate::data).
//!
//! The load order: the base set first (the built-in `original`, which is the player's own copy of SimGolf, unless the
//! player chose another base pack), then the mods in the order the player gave them in Preferences > Content, then the
//! player's translation. Later packs win. Nothing here panics on a bad pack: every problem (bad toml, a missing
//! requirement, a conflict) is reported on the pack's row and the pack is left out.
use crate::vfs::{Layer, Source};
use crate::zip::Zip;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The id of the built-in base set: the player's own installed copy of SimGolf.
pub const ORIGINAL: &str = "original";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Base,
    Mod,
    Translation,
    Scenario,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Base => "base",
            Kind::Mod => "mod",
            Kind::Translation => "translation",
            Kind::Scenario => "scenario",
        }
    }
}

// ---- versions and requirements -----------------------------------------------------------------------------------------------

/// A version: numbers separated by dots ("1.2.0"); missing parts count as 0, so 1.2 == 1.2.0.
#[derive(Clone, Debug)]
pub struct Version(pub Vec<u64>);

impl Version {
    pub fn parse(s: &str) -> Result<Version, String> {
        let s = s.trim();
        // a pre-release or build suffix ("1.0.0-beta", "1.0+x") is ignored for ordering
        let core = s.split(['-', '+']).next().unwrap_or("");
        let parts: Result<Vec<u64>, _> = core.split('.').map(|p| p.parse::<u64>()).collect();
        match parts {
            Ok(v) if !v.is_empty() && v.len() <= 4 => Ok(Version(v)),
            _ => Err(format!("bad version \"{s}\" (expected numbers like 1.2.0)")),
        }
    }
    fn at(&self, i: usize) -> u64 {
        self.0.get(i).copied().unwrap_or(0)
    }
}

impl PartialEq for Version {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Version {}
impl PartialOrd for Version {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        (0..self.0.len().max(o.0.len())).map(|i| self.at(i).cmp(&o.at(i))).find(|c| c.is_ne()).unwrap_or(Ordering::Equal)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.iter().map(u64::to_string).collect::<Vec<_>>().join("."))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Ge,
    Gt,
    Le,
    Lt,
    Eq,
    Ne,
    /// ^1.2: at least 1.2 with the same first number.
    Caret,
    /// ~1.2: at least 1.2 with the same first two numbers.
    Tilde,
}

/// One entry of `requires` or `conflicts`: a pack id and optional version constraints ("foo", "foo >= 1.0",
/// "foo >= 1.0, < 2").
#[derive(Clone, Debug, PartialEq)]
pub struct Requirement {
    pub id: String,
    pub constraints: Vec<(Op, Version)>,
}

impl Requirement {
    pub fn parse(s: &str) -> Result<Requirement, String> {
        let s = s.trim();
        let end = s.find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')).unwrap_or(s.len());
        let id = &s[..end];
        if !valid_id(id) {
            return Err(format!("bad requirement \"{s}\" (expected a pack id, then optional constraints like >= 1.0)"));
        }
        let mut constraints = Vec::new();
        let rest = s[end..].trim();
        if !rest.is_empty() {
            for c in rest.split(',') {
                let c = c.trim();
                let ops = [
                    (">=", Op::Ge),
                    ("<=", Op::Le),
                    ("==", Op::Eq),
                    ("!=", Op::Ne),
                    (">", Op::Gt),
                    ("<", Op::Lt),
                    ("=", Op::Eq),
                    ("^", Op::Caret),
                    ("~", Op::Tilde),
                ];
                let Some(&(sym, op)) = ops.iter().find(|(sym, _)| c.starts_with(sym)) else {
                    return Err(format!("bad requirement \"{s}\": \"{c}\" needs an operator (>=, >, <=, <, =, !=, ^ or ~)"));
                };
                constraints.push((op, Version::parse(&c[sym.len()..]).map_err(|e| format!("requirement \"{s}\": {e}"))?));
            }
        }
        Ok(Requirement { id: id.to_string(), constraints })
    }

    /// Whether pack `id` at version `v` meets this requirement.
    pub fn matches(&self, id: &str, v: &Version) -> bool {
        id == self.id
            && self.constraints.iter().all(|(op, w)| match op {
                Op::Ge => v >= w,
                Op::Gt => v > w,
                Op::Le => v <= w,
                Op::Lt => v < w,
                Op::Eq => v == w,
                Op::Ne => v != w,
                Op::Caret => v >= w && v.at(0) == w.at(0),
                Op::Tilde => v >= w && v.at(0) == w.at(0) && v.at(1) == w.at(1),
            })
    }
}

/// A pack id: lower case letters, digits and dashes, starting with a letter or digit.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && !id.starts_with('-') && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

// ---- pack.toml --------------------------------------------------------------------------------------------------------------

/// The `[pack]` table of a pack.toml.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: Kind,
    #[serde(default)]
    pub authors: Vec<String>,
    /// An SPDX licence identifier.
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    pack: Manifest,
}

impl Manifest {
    /// Reads and checks a pack.toml.
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let f: ManifestFile = toml::from_str(text).map_err(|e| format!("pack.toml: {}", crate::data::toml_error(text, &e)))?;
        let m = f.pack;
        if !valid_id(&m.id) {
            return Err(format!("pack.toml: bad id \"{}\" (lower case letters, digits and dashes)", m.id));
        }
        if m.id == ORIGINAL {
            return Err(format!("pack.toml: the id \"{ORIGINAL}\" is the built-in base set's"));
        }
        if m.name.trim().is_empty() {
            return Err("pack.toml: name is empty".into());
        }
        Version::parse(&m.version).map_err(|e| format!("pack.toml: {e}"))?;
        for r in m.requires.iter().chain(&m.conflicts) {
            Requirement::parse(r).map_err(|e| format!("pack.toml: {e}"))?;
        }
        Ok(m)
    }

    pub fn version(&self) -> Version {
        Version::parse(&self.version).unwrap_or(Version(vec![0]))
    }
    fn requirements(&self) -> Vec<Requirement> {
        self.requires.iter().filter_map(|r| Requirement::parse(r).ok()).collect()
    }
    fn conflict_list(&self) -> Vec<Requirement> {
        self.conflicts.iter().filter_map(|r| Requirement::parse(r).ok()).collect()
    }

    /// The built-in base set: the art, sounds and data of the player's own copy of the game.
    pub fn original() -> Manifest {
        Manifest {
            id: ORIGINAL.into(),
            name: "Original SimGolf (your own copy)".into(),
            version: "1.0.0".into(),
            kind: Kind::Base,
            authors: vec!["Firaxis Games".into()],
            license: "your own copy".into(),
            description: "The art, sounds, music and text of the SimGolf installed on this computer.".into(),
            requires: Vec::new(),
            conflicts: Vec::new(),
        }
    }
}

// ---- finding packs ----------------------------------------------------------------------------------------------------------

/// A pack found on disk (or in the browser's store) with a readable pack.toml.
pub struct Pack {
    pub manifest: Manifest,
    pub source: Arc<Source>,
    /// Where it was found, for the player.
    pub location: String,
}

/// What [`discover`] found: the packs, and the folders or zips that look like packs but could not be read.
#[derive(Default)]
pub struct Found {
    pub packs: Vec<Pack>,
    /// (where, what is wrong)
    pub broken: Vec<(String, String)>,
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// A .zip pack: pack.toml at the archive's root, or in its one top folder.
pub fn open_zip(data: Vec<u8>) -> Result<(Manifest, Source), String> {
    let zip = Zip::open(data)?;
    let toml_name = zip
        .names()
        .filter(|n| n.to_lowercase().ends_with("pack.toml") && n.matches('/').count() <= 1)
        .filter(|n| n.eq_ignore_ascii_case("pack.toml") || n.to_lowercase().ends_with("/pack.toml"))
        .min_by_key(|n| n.len())
        .map(str::to_string)
        .ok_or("no pack.toml in the zip")?;
    let prefix = toml_name[..toml_name.len() - "pack.toml".len()].to_string();
    let text = zip.read(&toml_name).ok_or("cannot read pack.toml in the zip")?;
    let m = Manifest::parse(&String::from_utf8_lossy(&text))?;
    Ok((m, Source::Zip { zip: Arc::new(zip), prefix }))
}

/// Looks for packs in each folder of `dirs` (every sub-folder with a pack.toml, every .zip), in the order given; within a
/// folder by name. A pack whose id was already found is reported as a duplicate and left out.
pub fn discover(dirs: &[PathBuf]) -> Found {
    let mut found = Found::default();
    for dir in dirs {
        let mut entries = crate::fsutil::list_dir(dir);
        entries.sort();
        for p in entries {
            let name = file_name(&p);
            if name.starts_with('.') {
                continue;
            }
            let loc = p.display().to_string();
            let got = if crate::fsutil::is_dir(&p) {
                let toml = crate::fsutil::resolve(&p, "pack.toml");
                match crate::fsutil::read_raw(&toml) {
                    Some(t) => Manifest::parse(&String::from_utf8_lossy(&t)).map(|m| (m, Source::Dir(p.clone()))),
                    None => Err("no pack.toml in the folder".into()),
                }
            } else if name.to_lowercase().ends_with(".zip") {
                match crate::fsutil::read_raw(&p) {
                    Some(d) => open_zip(d),
                    None => Err("cannot read the file".into()),
                }
            } else {
                continue;
            };
            match got {
                Ok((m, src)) => {
                    if let Some(first) = found.packs.iter().find(|q| q.manifest.id == m.id) {
                        found.broken.push((loc, format!("the id \"{}\" is already used by {}", m.id, first.location)));
                    } else {
                        found.packs.push(Pack { manifest: m, source: Arc::new(src), location: loc });
                    }
                }
                Err(e) => found.broken.push((loc, e)),
            }
        }
    }
    found
}

// ---- the player's choices ---------------------------------------------------------------------------------------------------

/// One mod in the player's list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModChoice {
    pub id: String,
    pub enabled: bool,
}

/// The player's content choices, kept under "content" in settings.json.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// The base set (None: the built-in original).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// The mods in the player's order, with their switches. Mods not listed are new: they come last, switched off.
    #[serde(default)]
    pub mods: Vec<ModChoice>,
    /// The translation in use (None: the base set's own text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

impl Settings {
    /// The settings with every found mod listed: the player's order first, then new mods by id, switched off.
    pub fn complete(&self, found: &Found) -> Settings {
        let mut s = self.clone();
        let is_mod = |id: &str| found.packs.iter().any(|p| p.manifest.id == id && p.manifest.kind == Kind::Mod);
        s.mods.retain(|m| is_mod(&m.id));
        let mut seen = std::collections::HashSet::new();
        s.mods.retain(|m| seen.insert(m.id.clone()));
        let mut new: Vec<&str> = found
            .packs
            .iter()
            .filter(|p| p.manifest.kind == Kind::Mod && !s.mods.iter().any(|m| m.id == p.manifest.id))
            .map(|p| p.manifest.id.as_str())
            .collect();
        new.sort();
        s.mods.extend(new.into_iter().map(|id| ModChoice { id: id.into(), enabled: false }));
        s
    }
}

// ---- the load order ---------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Loaded.
    Active,
    /// Switched off by the player (or another base set or translation is chosen).
    Off,
    /// Not loaded because of a problem.
    Error(String),
    /// Not a kind that is loaded this way (scenarios are chosen when a game starts).
    NotLoaded(String),
}

/// One line of Preferences > Content.
#[derive(Clone, Debug)]
pub struct Row {
    pub manifest: Manifest,
    pub location: String,
    /// Index into Found::packs (None for the built-in base set and for broken entries).
    pub pack: Option<usize>,
    pub status: Status,
    /// Problems found while loading the pack's data (it is loaded; the entries concerned are not).
    pub warnings: Vec<String>,
}

/// The packs in load order and the status of every pack found.
#[derive(Clone, Debug)]
pub struct Plan {
    /// Indexes into Found::packs, in load order (the built-in base set is not listed: it is the game folder itself).
    pub order: Vec<usize>,
    /// Bases, then mods in the player's order, then translations, scenarios and broken packs.
    pub rows: Vec<Row>,
}

impl Plan {
    /// The ids of the loaded packs, the base set first.
    pub fn active_ids(&self) -> Vec<String> {
        self.rows.iter().filter(|r| r.status == Status::Active).map(|r| r.manifest.id.clone()).collect()
    }
}

/// Checks a pack's requirements and conflicts against the packs loaded before it (`active`, in load order) and the packs
/// that will be enabled after it (`later`).
fn check(m: &Manifest, active: &[(String, Version, Vec<Requirement>)], later: &[&str], found: &Found) -> Result<(), String> {
    for r in m.requirements() {
        if active.iter().any(|(id, v, _)| r.matches(id, v)) {
            continue;
        }
        if let Some((_, v, _)) = active.iter().find(|(id, _, _)| *id == r.id) {
            return Err(format!("needs {} but version {v} is loaded", r_text(&r)));
        }
        if later.contains(&r.id.as_str()) {
            return Err(format!("needs {} loaded before it: move it below {}", r.id, r.id));
        }
        return Err(match found.packs.iter().find(|p| p.manifest.id == r.id) {
            Some(p) if !r.matches(&p.manifest.id, &p.manifest.version()) => {
                format!("needs {}, but version {} is installed", r_text(&r), p.manifest.version)
            }
            Some(_) => format!("needs {}, which is switched off or not loaded", r.id),
            None => format!("needs {}, which is not installed", r_text(&r)),
        });
    }
    let v = m.version();
    for (id, av, conflicts) in active {
        if let Some(c) = m.conflict_list().iter().find(|c| c.matches(id, av)) {
            return Err(format!("conflicts with {} (its pack.toml says so)", c.id));
        }
        if conflicts.iter().any(|c| c.matches(&m.id, &v)) {
            return Err(format!("conflicts with {id} (that pack's pack.toml says so)"));
        }
    }
    Ok(())
}

fn r_text(r: &Requirement) -> String {
    let ops = |o: &Op| match o {
        Op::Ge => ">=",
        Op::Gt => ">",
        Op::Le => "<=",
        Op::Lt => "<",
        Op::Eq => "=",
        Op::Ne => "!=",
        Op::Caret => "^",
        Op::Tilde => "~",
    };
    let c: Vec<String> = r.constraints.iter().map(|(o, v)| format!("{} {v}", ops(o))).collect();
    if c.is_empty() {
        r.id.clone()
    } else {
        format!("{} {}", r.id, c.join(", "))
    }
}

/// Checks a pack chosen later (a scenario, when a game starts) against the packs a plan loads: its requirements must be
/// loaded and it must not conflict with them.
pub fn check_against(m: &Manifest, plan: &Plan, found: &Found) -> Result<(), String> {
    let active: Vec<(String, Version, Vec<Requirement>)> = plan
        .rows
        .iter()
        .filter(|r| r.status == Status::Active)
        .map(|r| (r.manifest.id.clone(), r.manifest.version(), r.manifest.conflict_list()))
        .collect();
    check(m, &active, &[], found)
}

/// Puts the packs found in load order following the player's settings, and works out every pack's status.
pub fn resolve(found: &Found, settings: &Settings) -> Plan {
    let s = settings.complete(found);
    let mut rows = Vec::new();
    let mut order = Vec::new();
    let mut active: Vec<(String, Version, Vec<Requirement>)> = Vec::new();
    let find = |id: &str| found.packs.iter().position(|p| p.manifest.id == id);

    // the base set
    let original = Manifest::original();
    let mut base_row =
        Row { manifest: original.clone(), location: "built in".into(), pack: None, status: Status::Active, warnings: Vec::new() };
    let mut other_bases = Vec::new();
    let chosen = s.base.as_deref().and_then(find).filter(|&i| found.packs[i].manifest.kind == Kind::Base);
    for (i, p) in found.packs.iter().enumerate().filter(|(_, p)| p.manifest.kind == Kind::Base) {
        let status = if Some(i) == chosen {
            match check(&p.manifest, &[], &[], found) {
                Ok(()) => Status::Active,
                Err(e) => Status::Error(e),
            }
        } else {
            Status::Off
        };
        other_bases.push(Row { manifest: p.manifest.clone(), location: p.location.clone(), pack: Some(i), status, warnings: Vec::new() });
    }
    match other_bases.iter().find(|r| r.status == Status::Active) {
        Some(r) => {
            base_row.status = Status::Off;
            order.push(r.pack.unwrap_or(0));
            active.push((r.manifest.id.clone(), r.manifest.version(), r.manifest.conflict_list()));
        }
        None => active.push((ORIGINAL.into(), original.version(), Vec::new())),
    }
    rows.push(base_row);
    rows.extend(other_bases);

    // the mods, in the player's order
    for (k, m) in s.mods.iter().enumerate() {
        let Some(i) = find(&m.id) else { continue };
        let p = &found.packs[i];
        let status = if !m.enabled {
            Status::Off
        } else {
            let later: Vec<&str> = s.mods[k + 1..].iter().filter(|m| m.enabled).map(|m| m.id.as_str()).collect();
            match check(&p.manifest, &active, &later, found) {
                Ok(()) => {
                    order.push(i);
                    active.push((p.manifest.id.clone(), p.manifest.version(), p.manifest.conflict_list()));
                    Status::Active
                }
                Err(e) => Status::Error(e),
            }
        };
        rows.push(Row { manifest: p.manifest.clone(), location: p.location.clone(), pack: Some(i), status, warnings: Vec::new() });
    }

    // the translation, then scenarios
    for (i, p) in found.packs.iter().enumerate().filter(|(_, p)| p.manifest.kind == Kind::Translation) {
        let status = if s.translation.as_deref() == Some(p.manifest.id.as_str()) {
            match check(&p.manifest, &active, &[], found) {
                Ok(()) => {
                    order.push(i);
                    Status::Active
                }
                Err(e) => Status::Error(e),
            }
        } else {
            Status::Off
        };
        rows.push(Row { manifest: p.manifest.clone(), location: p.location.clone(), pack: Some(i), status, warnings: Vec::new() });
    }
    for (i, p) in found.packs.iter().enumerate().filter(|(_, p)| p.manifest.kind == Kind::Scenario) {
        let status = Status::NotLoaded("chosen when a game starts: Start New Game, Scenario...".into());
        rows.push(Row { manifest: p.manifest.clone(), location: p.location.clone(), pack: Some(i), status, warnings: Vec::new() });
    }
    for (loc, e) in &found.broken {
        let name = Path::new(loc).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| loc.clone());
        let manifest = Manifest {
            id: String::new(),
            name,
            version: "?".into(),
            kind: Kind::Mod,
            authors: Vec::new(),
            license: String::new(),
            description: String::new(),
            requires: Vec::new(),
            conflicts: Vec::new(),
        };
        rows.push(Row { manifest, location: loc.clone(), pack: None, status: Status::Error(e.clone()), warnings: Vec::new() });
    }
    Plan { order, rows }
}

// ---- loading ----------------------------------------------------------------------------------------------------------------

/// The loaded packs: their override layers (in load order) and the merged data tables.
pub struct Loaded {
    pub layers: Vec<Layer>,
    pub data: crate::data::Data,
    /// Problems with the data files, as (pack id, message); "" for the merged tables as a whole.
    pub warnings: Vec<(String, String)>,
}

/// Reads the packs of a plan: indexes their override files and merges their data files in load order.
pub fn load(found: &Found, plan: &Plan) -> Loaded {
    let mut tables = crate::data::Tables::builtin();
    let mut layers = Vec::new();
    let mut warnings = Vec::new();
    for &i in &plan.order {
        let p = &found.packs[i];
        let id = &p.manifest.id;
        for rel in p.source.files() {
            let low = rel.to_lowercase();
            if let Some(f) = low.strip_prefix("data/").and_then(|f| f.strip_suffix(".toml")) {
                if !crate::data::FILES.contains(&f) {
                    warnings.push((id.clone(), format!("data/{f}.toml is not a data file the engine reads")));
                }
            }
        }
        for f in crate::data::FILES {
            if let Some(text) = p.source.read(&format!("data/{f}.toml")) {
                for w in tables.merge(f, &String::from_utf8_lossy(&text)) {
                    warnings.push((id.clone(), w));
                }
            }
        }
        layers.push(Layer::new(id, p.source.clone()));
    }
    let (data, w) = crate::data::Data::from_tables(&tables);
    warnings.extend(w.into_iter().map(|w| (String::new(), w)));
    Loaded { layers, data, warnings }
}

/// Puts the loaded packs in effect: their files over the game folder at `game_dir`, their data as the tables the game
/// reads. With no packs this restores the game folder alone and the built-in tables.
pub fn apply(game_dir: &Path, loaded: Loaded) {
    if loaded.layers.is_empty() {
        crate::vfs::unmount();
        crate::data::reset();
        return;
    }
    crate::vfs::mount(game_dir, loaded.layers);
    crate::data::install(loaded.data);
}

/// Copies a loaded set's data warnings onto the rows of a plan.
pub fn attach_warnings(plan: &mut Plan, warnings: &[(String, String)]) {
    for (id, w) in warnings.iter().filter(|(id, _)| !id.is_empty()) {
        if let Some(r) = plan.rows.iter_mut().find(|r| &r.manifest.id == id) {
            r.warnings.push(w.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str, kind: &str, version: &str, requires: &[&str], conflicts: &[&str]) -> String {
        let list = |v: &[&str]| v.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
        format!(
            "[pack]\nid = \"{id}\"\nname = \"Pack {id}\"\nversion = \"{version}\"\nkind = \"{kind}\"\nauthors = [\"Tester\"]\n\
             license = \"CC0-1.0\"\nrequires = [{}]\nconflicts = [{}]\n",
            list(requires),
            list(conflicts)
        )
    }

    fn found(packs: &[String]) -> Found {
        let mut f = Found::default();
        for (k, t) in packs.iter().enumerate() {
            let m = Manifest::parse(t).unwrap();
            let zip = Zip::open(crate::zip::write(&[("pack.toml", t.as_bytes())])).unwrap();
            f.packs.push(Pack {
                manifest: m,
                source: Arc::new(Source::Zip { zip: Arc::new(zip), prefix: String::new() }),
                location: format!("#{k}"),
            });
        }
        f
    }

    fn on(ids: &[&str]) -> Settings {
        Settings { mods: ids.iter().map(|id| ModChoice { id: id.to_string(), enabled: true }).collect(), ..Default::default() }
    }

    fn status(p: &Plan, id: &str) -> Status {
        p.rows.iter().find(|r| r.manifest.id == id).unwrap().status.clone()
    }

    #[test]
    fn manifest_parsing_and_errors() {
        let m = Manifest::parse(&manifest("my-desert-courses", "mod", "1.2.0", &["original >= 1.0"], &[])).unwrap();
        assert_eq!((m.id.as_str(), m.kind, m.authors.len()), ("my-desert-courses", Kind::Mod, 1));
        assert!(Manifest::parse("[pack\nid=").unwrap_err().contains("pack.toml"));
        assert!(Manifest::parse(&manifest("Bad_Id", "mod", "1.0", &[], &[])).unwrap_err().contains("bad id"));
        assert!(Manifest::parse(&manifest("a", "mod", "one", &[], &[])).unwrap_err().contains("version"));
        assert!(Manifest::parse(&manifest("a", "plugin", "1", &[], &[])).is_err());
        assert!(Manifest::parse(&manifest("a", "mod", "1", &["b >> 2"], &[])).unwrap_err().contains("requirement"));
        assert!(Manifest::parse(&manifest("original", "base", "1", &[], &[])).is_err());
        assert!(Manifest::parse("[pack]\nid = \"a\"\nname = \"A\"\nversion = \"1\"\nkind = \"mod\"\ncolour = 3\n").is_err());
    }

    #[test]
    fn versions_and_constraints() {
        let v = |s: &str| Version::parse(s).unwrap();
        assert!(v("1.2") == v("1.2.0") && v("1.10") > v("1.9") && v("2.0.0-beta") == v("2"));
        let r = Requirement::parse("foo >= 1.0, < 2").unwrap();
        assert!(r.matches("foo", &v("1.5")) && !r.matches("foo", &v("2.0")) && !r.matches("bar", &v("1.5")));
        assert!(Requirement::parse("foo").unwrap().matches("foo", &v("0.1")));
        assert!(Requirement::parse("foo ^1.2").unwrap().matches("foo", &v("1.9")));
        assert!(!Requirement::parse("foo ^1.2").unwrap().matches("foo", &v("2.0")));
        assert!(!Requirement::parse("foo ~1.2").unwrap().matches("foo", &v("1.3")));
        assert!(Requirement::parse("foo=1").unwrap().matches("foo", &v("1.0.0")));
    }

    #[test]
    fn load_order_follows_the_player_and_reports_problems() {
        let f = found(&[
            manifest("a", "mod", "1.0", &[], &[]),
            manifest("b", "mod", "2.0", &["a >= 1.0"], &[]),
            manifest("c", "mod", "1.0", &["a >= 3"], &[]),
            manifest("d", "mod", "1.0", &["zzz"], &[]),
            manifest("e", "mod", "1.0", &[], &["a"]),
            manifest("f", "mod", "1.0", &["d"], &[]),
            manifest("g", "mod", "1.0", &[], &[]),
            manifest("fr", "translation", "1.0", &[], &[]),
            manifest("s", "scenario", "1.0", &[], &[]),
        ]);
        let p = resolve(&f, &on(&["a", "b", "c", "d", "e", "f"]));
        let ids: Vec<&str> = p.order.iter().map(|&i| f.packs[i].manifest.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(p.active_ids(), ["original", "a", "b"]);
        assert!(matches!(status(&p, "c"), Status::Error(e) if e.contains("a >= 3") && e.contains("1.0")));
        assert!(matches!(status(&p, "d"), Status::Error(e) if e.contains("not installed")));
        assert!(matches!(status(&p, "e"), Status::Error(e) if e.contains("conflicts with a")));
        // f needs d, which failed: f fails too
        assert!(matches!(status(&p, "f"), Status::Error(_)));
        // a new mod comes last, switched off; translations are off unless chosen; scenarios are not loaded
        assert_eq!(status(&p, "g"), Status::Off);
        assert_eq!(p.rows.iter().filter(|r| r.manifest.kind == Kind::Mod).last().unwrap().manifest.id, "g");
        assert_eq!(status(&p, "fr"), Status::Off);
        assert!(matches!(status(&p, "s"), Status::NotLoaded(_)));
        assert_eq!(status(&p, ORIGINAL), Status::Active);

        // b before a: b asks to be moved
        let p = resolve(&f, &on(&["b", "a"]));
        assert!(matches!(status(&p, "b"), Status::Error(e) if e.contains("before")));
        assert_eq!(status(&p, "a"), Status::Active);
        // a switched off: b cannot load
        let mut s = on(&["a", "b"]);
        s.mods[0].enabled = false;
        assert!(matches!(status(&resolve(&f, &s), "b"), Status::Error(e) if e.contains("switched off")));
        // the translation chosen loads last
        let mut s = on(&["a"]);
        s.translation = Some("fr".into());
        let p = resolve(&f, &s);
        assert_eq!(p.order.iter().map(|&i| f.packs[i].manifest.id.as_str()).collect::<Vec<_>>(), ["a", "fr"]);
    }

    #[test]
    fn a_chosen_base_set_replaces_the_original() {
        let f = found(&[manifest("free", "base", "1.0", &[], &[]), manifest("m", "mod", "1", &["free"], &[])]);
        let p = resolve(&f, &on(&["m"]));
        assert!(matches!(status(&p, "m"), Status::Error(_)));
        let mut s = on(&["m"]);
        s.base = Some("free".into());
        let p = resolve(&f, &s);
        assert_eq!(p.active_ids(), ["free", "m"]);
        assert_eq!(status(&p, ORIGINAL), Status::Off);
    }

    #[test]
    fn discovery_reads_folders_and_zips_and_reports_bad_ones() {
        let dir = std::env::temp_dir().join(format!("sg-packs-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (one, two) = (dir.join("one"), dir.join("two"));
        std::fs::create_dir_all(one.join("folder-pack/data")).unwrap();
        std::fs::write(one.join("folder-pack/Pack.toml"), manifest("folder-pack", "mod", "1.0", &[], &[])).unwrap();
        std::fs::write(one.join("folder-pack/data/terrain.toml"), "[[terrain]]\nid = \"fairway\"\ncost = 100\n").unwrap();
        std::fs::create_dir_all(one.join("broken")).unwrap();
        std::fs::write(one.join("broken/pack.toml"), "not toml [").unwrap();
        std::fs::create_dir_all(one.join("empty")).unwrap();
        let z = crate::zip::write(&[
            ("zipped/pack.toml", manifest("zipped", "mod", "1.0", &["folder-pack"], &[]).as_bytes()),
            ("zipped/gfx/Interface/Panel.png", b"png"),
        ]);
        std::fs::write(one.join("zipped.zip"), z).unwrap();
        std::fs::write(one.join("readme.txt"), "not a pack").unwrap();
        std::fs::create_dir_all(two.join("again")).unwrap();
        std::fs::write(two.join("again/pack.toml"), manifest("zipped", "mod", "9.0", &[], &[])).unwrap();

        let f = discover(&[one.clone(), two.clone(), dir.join("missing")]);
        let ids: Vec<&str> = f.packs.iter().map(|p| p.manifest.id.as_str()).collect();
        assert_eq!(ids, ["folder-pack", "zipped"]);
        assert_eq!(f.broken.len(), 3, "{:?}", f.broken); // bad toml, no pack.toml, duplicate id
        assert!(f.broken.iter().any(|(_, e)| e.contains("already used")));

        // load both: the data merges, the zip's art becomes an override layer
        let plan = resolve(&f, &on(&["folder-pack", "zipped"]));
        assert_eq!(plan.order.len(), 2);
        let loaded = load(&f, &plan);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.data.paint_units[2], 1);
        assert_eq!(loaded.layers[1].index.get("interface/panel.png").map(String::as_str), Some("gfx/Interface/Panel.png"));

        // the virtual file system: a game folder whose Interface/Panel.pcx the zip replaces with a PNG
        let game = dir.join("game");
        std::fs::create_dir_all(game.join("Interface")).unwrap();
        std::fs::write(game.join("Interface/Panel.pcx"), b"pcx").unwrap();
        std::fs::write(game.join("Interface/Other.pcx"), b"other").unwrap();
        let panel = crate::fsutil::resolve(&game, "interface/panel.pcx");
        assert_eq!(crate::fsutil::read_file(&panel).unwrap(), b"pcx");
        crate::vfs::mount(&game, loaded.layers);
        assert_eq!(crate::fsutil::read_file(&panel).unwrap(), b"png");
        assert_eq!(crate::fsutil::read_raw(&panel).unwrap(), b"pcx");
        assert_eq!(crate::fsutil::read_file(game.join("Interface/Other.pcx")).unwrap(), b"other");
        assert_eq!(crate::vfs::overridden_by(&panel).as_deref(), Some("zipped"));
        crate::vfs::unmount();
        assert_eq!(crate::fsutil::read_file(&panel).unwrap(), b"pcx");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_example_pack_parses_and_makes_fairways_cheaper() {
        let m = Manifest::parse(include_str!("../../../packs/example-cheap-fairways/pack.toml")).unwrap();
        assert_eq!(m.kind, Kind::Mod);
        let mut t = crate::data::Tables::builtin();
        let w = t.merge("terrain", include_str!("../../../packs/example-cheap-fairways/data/terrain.toml"));
        assert!(w.is_empty(), "{w:?}");
        let (d, w) = crate::data::Data::from_tables(&t);
        assert!(w.is_empty(), "{w:?}");
        assert!(d.paint_units[2] < crate::economy::Economy::builtin_terrain_cost_units(2));
    }
}
