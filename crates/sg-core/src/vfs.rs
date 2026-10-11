//! The virtual file system of content packs (docs/MODDING.md, section 2).
//!
//! Every read of an original game file goes through [`crate::fsutil::read_file`], which asks [`read_override`] first: when
//! an enabled pack has a file at the same path under its `gfx/`, `sfx/`, `music/` or `video/` folder, the pack's file is
//! returned instead of the game folder's. Paths match case-insensitively; later layers (packs later in the load order) win.
//! A picture may be replaced by a PNG of the same name (`Interface/BuildPanel.png` for `Interface/BuildPanel.pcx`) and a
//! sound by an OGG (`Sounds/Applause.ogg` for `Sounds/Applause.wav`); the decoders recognise the replacement by its
//! contents (crate::assets).
//!
//! Nothing is mounted until the game mounts its enabled packs, and with no packs every lookup returns at once, so the game
//! reads exactly what it read before.
use crate::zip::Zip;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// The folders of a pack that override game files, and the file types each accepts (lower case, with the dot).
pub const OVERRIDE_DIRS: [(&str, &[&str]); 4] = [
    ("gfx", &[".png", ".pcx", ".tga", ".bmp", ".flc"]),
    ("sfx", &[".wav", ".ogg"]),
    ("music", &[".wav", ".ogg"]),
    ("video", &[".bik", ".flc"]),
];

/// Where a pack's files are: a folder, or a folder inside a .zip (`prefix` is that folder's path in the archive, with a
/// trailing slash, or empty when pack.toml is at the archive's root).
pub enum Source {
    Dir(PathBuf),
    Zip { zip: Arc<Zip>, prefix: String },
}

impl Source {
    /// A file of the pack by its path relative to the pack's root (forward slashes, case-insensitive).
    pub fn read(&self, rel: &str) -> Option<Vec<u8>> {
        match self {
            Source::Dir(d) => crate::fsutil::read_raw(crate::fsutil::resolve(d, rel)),
            Source::Zip { zip, prefix } => {
                let want = format!("{prefix}{rel}").to_lowercase();
                let name = zip.names().find(|n| n.to_lowercase() == want)?.to_string();
                zip.read(&name)
            }
        }
    }

    /// Every file of the pack, as paths relative to its root with forward slashes, sorted.
    pub fn files(&self) -> Vec<String> {
        match self {
            Source::Dir(d) => crate::fsutil::walk(d).iter().filter_map(|p| p.strip_prefix(d).ok().map(rel_string)).collect(),
            Source::Zip { zip, prefix } => {
                zip.names().filter_map(|n| n.strip_prefix(prefix.as_str()).map(str::to_string)).filter(|n| !n.is_empty()).collect()
            }
        }
    }

    /// Where the pack is, for messages.
    pub fn describe(&self) -> String {
        match self {
            Source::Dir(d) => d.display().to_string(),
            Source::Zip { .. } => "(zip)".into(),
        }
    }
}

/// A path's components joined with '/'.
fn rel_string(p: &Path) -> String {
    p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// One pack's overriding files: lower-case path in the game folder -> the file's path in the pack.
pub struct Layer {
    pub id: String,
    pub source: Arc<Source>,
    pub index: HashMap<String, String>,
}

impl Layer {
    /// Indexes the files under the pack's override folders. Files of a type their folder does not take are left out.
    pub fn new(id: &str, source: Arc<Source>) -> Layer {
        let mut index = HashMap::new();
        for rel in source.files() {
            let Some((top, rest)) = rel.split_once('/') else { continue };
            let top = top.to_lowercase();
            let Some((_, exts)) = OVERRIDE_DIRS.iter().find(|(d, _)| *d == top) else { continue };
            let low = rest.to_lowercase();
            if exts.iter().any(|e| low.ends_with(e)) {
                index.insert(low, rel.clone());
            }
        }
        Layer { id: id.to_string(), source, index }
    }
}

struct Vfs {
    game_dir: PathBuf,
    layers: Vec<Layer>,
}

static MOUNTED: AtomicBool = AtomicBool::new(false);
static VFS: RwLock<Option<Vfs>> = RwLock::new(None);

/// Puts `layers` (in load order: later ones win) over the game folder. Replaces whatever was mounted.
pub fn mount(game_dir: &Path, layers: Vec<Layer>) {
    let any = layers.iter().any(|l| !l.index.is_empty());
    if let Ok(mut v) = VFS.write() {
        *v = any.then(|| Vfs { game_dir: game_dir.to_path_buf(), layers });
    }
    MOUNTED.store(any, Ordering::Release);
}

/// Takes every layer away: the game folder alone again.
pub fn unmount() {
    if let Ok(mut v) = VFS.write() {
        *v = None;
    }
    MOUNTED.store(false, Ordering::Release);
}

/// The names a replacement of `rel` (lower case) may have: the same name, then a PNG for a PCX/TGA/BMP picture or an OGG
/// for a WAV sound.
pub fn candidates(rel: &str) -> Vec<String> {
    let mut v = vec![rel.to_string()];
    if let Some((stem, ext)) = rel.rsplit_once('.') {
        match ext {
            "pcx" | "tga" | "bmp" => v.push(format!("{stem}.png")),
            "wav" => v.push(format!("{stem}.ogg")),
            _ => {}
        }
    }
    v
}

/// The path of `p` relative to the mounted game folder, lower case with '/', when it is inside it.
fn game_rel(v: &Vfs, p: &Path) -> Option<String> {
    let rel = p.strip_prefix(&v.game_dir).ok()?;
    let s = rel_string(rel).replace('\\', "/").to_lowercase();
    (!s.is_empty()).then_some(s)
}

/// Finds the layer and pack path that replace `rel` (lower case), searching from the last layer.
fn find<'a>(layers: &'a [Layer], rel: &str) -> Option<(&'a Layer, &'a str)> {
    let cands = candidates(rel);
    layers.iter().rev().find_map(|l| cands.iter().find_map(|c| l.index.get(c).map(|f| (l, f.as_str()))))
}

/// The bytes of the pack file that replaces the game file at `p`, if any.
pub fn read_override(p: &Path) -> Option<Vec<u8>> {
    if !MOUNTED.load(Ordering::Acquire) {
        return None;
    }
    let v = VFS.read().ok()?;
    let v = v.as_ref()?;
    let rel = game_rel(v, p)?;
    let (layer, file) = find(&v.layers, &rel)?;
    layer.source.read(file)
}

/// The id of the pack that replaces the game file at `p`, if any (the HD mode leaves such files to the pack).
pub fn overridden_by(p: &Path) -> Option<String> {
    if !MOUNTED.load(Ordering::Acquire) {
        return None;
    }
    let v = VFS.read().ok()?;
    let v = v.as_ref()?;
    let rel = game_rel(v, p)?;
    find(&v.layers, &rel).map(|(l, _)| l.id.clone())
}

/// True when an enabled pack replaces the game file at `p`.
pub fn overridden(p: &Path) -> bool {
    overridden_by(p).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(id: &str, files: &[(&str, &[u8])]) -> Layer {
        let zip = Zip::open(crate::zip::write(files)).unwrap();
        Layer::new(id, Arc::new(Source::Zip { zip: Arc::new(zip), prefix: String::new() }))
    }

    #[test]
    fn lookup_order_case_and_formats() {
        let a = layer("a", &[("gfx/Interface/BuildPanel.png", b"A-png"), ("sfx/Sounds/Applause.wav", b"A-wav"), ("data/x.toml", b"")]);
        let b = layer("b", &[("GFX/interface/buildpanel.PCX", b"B-pcx"), ("music/Sounds/Theme.ogg", b"B-ogg")]);
        assert_eq!(a.index.len(), 2, "data/ is not an override folder");
        let layers = vec![a, b];
        // the later layer wins, whatever the case of the names
        let (l, f) = find(&layers, "interface/buildpanel.pcx").unwrap();
        assert_eq!((l.id.as_str(), f), ("b", "GFX/interface/buildpanel.PCX"));
        assert_eq!(l.source.read(f).unwrap(), b"B-pcx");
        // a PNG replaces a BMP of the same name, an OGG a WAV
        assert_eq!(find(&layers, "interface/buildpanel.bmp").unwrap().0.id, "a");
        assert_eq!(find(&layers, "sounds/theme.wav").unwrap().0.id, "b");
        assert_eq!(find(&layers, "sounds/applause.wav").unwrap().0.id, "a");
        assert!(find(&layers, "interface/other.pcx").is_none());
        // an unlisted type in an override folder is ignored
        let c = layer("c", &[("gfx/readme.txt", b"")]);
        assert!(c.index.is_empty());
    }

    #[test]
    fn candidates_for_art_and_sound() {
        assert_eq!(candidates("a/b.pcx"), ["a/b.pcx", "a/b.png"]);
        assert_eq!(candidates("a/b.wav"), ["a/b.wav", "a/b.ogg"]);
        assert_eq!(candidates("a/b.flc"), ["a/b.flc"]);
    }
}
