//! The virtual file system of content packs (docs/MODDING.md, section 2).
//!
//! Every read of an original game file goes through [`crate::fsutil::read_file`], which asks [`read_override`] first: when
//! an enabled pack has a file at the same path under its `gfx/`, `sfx/`, `music/` or `video/` folder, the pack's file is
//! returned instead of the game folder's. Paths match case-insensitively; later layers (packs later in the load order) win.
//! A picture may be replaced by a PNG of the same name (`Interface/BuildPanel.png` for `Interface/BuildPanel.pcx`) and a
//! sound by an OGG (`Sounds/Applause.ogg` for `Sounds/Applause.wav`); the decoders recognise the replacement by its
//! contents (crate::assets).
//!
//! A pack with `scale` > 1 (high-resolution art, crate::hires) keeps its pictures apart: only the high-resolution loaders
//! read them at full size ([`art`]); every other read gets them scaled down to the original's size. A PNG sheet in place
//! of a sprite's .flc is likewise read only by the sprite loader (crate::hires::sheet_sprite); other reads of the .flc get
//! the file under the pack.
//!
//! Nothing is mounted until the game mounts its enabled packs, and with no packs every lookup returns at once, so the game
//! reads exactly what it read before.
use crate::zip::Zip;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// The folders of a pack that override game files, and the file types each accepts (lower case, with the dot). `text` holds
/// the game folder's text and character files (stories, golfer lists, terrain descriptions, credits), `fonts` its TrueType
/// fonts; a base set needs both, a mod may replace single files.
pub const OVERRIDE_DIRS: [(&str, &[&str]); 6] = [
    ("gfx", &[".png", ".pcx", ".tga", ".bmp", ".flc"]),
    ("sfx", &[".wav", ".ogg"]),
    ("music", &[".wav", ".ogg"]),
    ("video", &[".bik", ".flc"]),
    ("text", &[".txt", ".dta", ".chr", ".glf", ".pro", ".sve"]),
    ("fonts", &[".ttf"]),
];

/// A pack folder's optional file list (one pack-relative path per line), read instead of walking the folder.
pub const INDEX_FILE: &str = "pack-index.txt";

/// Where a pack's files are: a folder, or a folder inside a .zip (`prefix` is that folder's path in the archive, with a
/// trailing slash, or empty when pack.toml is at the archive's root).
pub enum Source {
    Dir(PathBuf),
    Zip {
        zip: Arc<Zip>,
        prefix: String,
    },
    /// The free base set drawn by the engine itself (crate::freeset): its files are made when they are read.
    Generated,
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
            Source::Generated => crate::freeset::read(rel),
        }
    }

    /// Every file of the pack, as paths relative to its root with forward slashes, sorted.
    pub fn files(&self) -> Vec<String> {
        match self {
            // OpenSGolf (3DS): a pack folder may carry `pack-index.txt`, one pack-relative path per line, which stands in for
            // walking the folder (a stat per file through the console's file service: minutes for the free set on an
            // SD card). `sgtool freeset` and the 3DS's own first-run write produce it.
            Source::Dir(d) => match crate::fsutil::read_raw(d.join(INDEX_FILE)) {
                Some(t) => {
                    let mut v: Vec<String> = String::from_utf8_lossy(&t)
                        .lines()
                        .map(|l| l.trim().replace('\\', "/"))
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect();
                    v.sort();
                    v
                }
                None => {
                    let v: Vec<String> = crate::fsutil::walk(d).iter().filter_map(|p| p.strip_prefix(d).ok().map(rel_string)).collect();
                    // write the index so the walk only ever happens once (ignored where the folder is read-only)
                    if !v.is_empty() {
                        let _ = std::fs::write(d.join(INDEX_FILE), v.join("\n"));
                    }
                    v
                }
            },
            Source::Zip { zip, prefix } => {
                zip.names().filter_map(|n| n.strip_prefix(prefix.as_str()).map(str::to_string)).filter(|n| !n.is_empty()).collect()
            }
            Source::Generated => crate::freeset::files(),
        }
    }

    /// Where the pack is, for messages.
    pub fn describe(&self) -> String {
        match self {
            Source::Dir(d) => d.display().to_string(),
            Source::Zip { .. } => "(zip)".into(),
            Source::Generated => "built in".into(),
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
    /// How many times the original's size the pack's pictures are (pack.toml `scale`).
    pub scale: u32,
    /// The pictures and sheets of a pack with `scale` > 1 (same keys as `index`), which the classic loaders cannot read
    /// as they are.
    pub art: HashMap<String, String>,
}

impl Layer {
    /// Indexes the files under the pack's override folders. Files of a type their folder does not take are left out.
    pub fn new(id: &str, source: Arc<Source>) -> Layer {
        Layer::with_scale(id, source, 1)
    }

    /// A pack whose art is `scale` times the original's size: its gfx/ PNGs go to `art`. A PCX, TGA, BMP or FLC there
    /// cannot be at another size and is left out (`sgtool pack-check` says so).
    pub fn with_scale(id: &str, source: Arc<Source>, scale: u32) -> Layer {
        let scale = scale.max(1);
        let mut index = HashMap::new();
        let mut art = HashMap::new();
        for rel in source.files() {
            let Some((top, rest)) = rel.split_once('/') else { continue };
            let top = top.to_lowercase();
            let Some((_, exts)) = OVERRIDE_DIRS.iter().find(|(d, _)| *d == top) else { continue };
            let low = rest.to_lowercase();
            if exts.iter().any(|e| low.ends_with(e)) {
                if scale > 1 && top == "gfx" {
                    if low.ends_with(".png") {
                        art.insert(low, rel.clone());
                    }
                    continue;
                }
                index.insert(low, rel.clone());
            }
        }
        Layer { id: id.to_string(), source, index, scale, art }
    }
}

struct Vfs {
    game_dir: PathBuf,
    layers: Vec<Layer>,
    /// The folders the layers' files make inside the game folder: lower-case folder path ("" for the game folder itself) ->
    /// lower-case entry name -> (the entry's name as the pack spells it, whether it is a folder).
    dirs: HashMap<String, HashMap<String, (String, bool)>>,
}

/// The folder index of the layers' files, so that a game folder made (partly or wholly) of pack files can be listed.
fn dir_index(layers: &[Layer]) -> HashMap<String, HashMap<String, (String, bool)>> {
    let mut dirs: HashMap<String, HashMap<String, (String, bool)>> = HashMap::new();
    dirs.entry(String::new()).or_default();
    for l in layers {
        for file in l.index.values() {
            // the path under the override folder, with the pack's spelling
            let Some((_, rest)) = file.split_once('/') else { continue };
            let parts: Vec<&str> = rest.split('/').filter(|c| !c.is_empty()).collect();
            let mut dir = String::new();
            for (k, part) in parts.iter().enumerate() {
                let last = k + 1 == parts.len();
                dirs.entry(dir.clone()).or_default().entry(part.to_lowercase()).or_insert_with(|| (part.to_string(), !last));
                if !dir.is_empty() {
                    dir.push('/');
                }
                dir.push_str(&part.to_lowercase());
            }
        }
    }
    dirs
}

static MOUNTED: AtomicBool = AtomicBool::new(false);
/// Some layer has high-resolution pictures or sprite sheets ([`art`]).
static ART: AtomicBool = AtomicBool::new(false);
/// High-resolution pictures scaled down for the classic loaders, by layer and lower-case game path.
static SCALED: RwLock<Option<HashMap<String, Arc<Vec<u8>>>>> = RwLock::new(None);

/// Tests that mount layers take this first: the mount is global.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static VFS: RwLock<Option<Vfs>> = RwLock::new(None);

/// Puts `layers` (in load order: later ones win) over the game folder. Replaces whatever was mounted.
pub fn mount(game_dir: &Path, layers: Vec<Layer>) {
    let any = layers.iter().any(|l| !l.index.is_empty() || !l.art.is_empty());
    let art = layers.iter().any(|l| !l.art.is_empty() || l.index.keys().any(|k| k.starts_with("flics/") && k.ends_with(".png")));
    if let Ok(mut c) = SCALED.write() {
        *c = None;
    }
    ART.store(any && art, Ordering::Release);
    if let Ok(mut v) = VFS.write() {
        *v = any.then(|| Vfs { game_dir: game_dir.to_path_buf(), dirs: dir_index(&layers), layers });
    }
    MOUNTED.store(any, Ordering::Release);
}

/// Takes every layer away: the game folder alone again.
pub fn unmount() {
    if let Ok(mut v) = VFS.write() {
        *v = None;
    }
    MOUNTED.store(false, Ordering::Release);
    ART.store(false, Ordering::Release);
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
    if !ART.load(Ordering::Acquire) {
        let (layer, file) = find(&v.layers, &rel)?;
        return layer.source.read(file);
    }
    classic_bytes(v, &rel, p, v.layers.len())
}

/// What one layer has for the game file `rel`.
enum Hit<'a> {
    /// A file the classic loaders read.
    Classic(&'a str),
    /// A high-resolution picture.
    Art(&'a str),
    /// A PNG sheet for a sprite.
    Sheet(&'a str),
}

/// Pictures a high-resolution pack may replace: not the palette files and sprites under Flics/, nor the bodies and heads,
/// which are recoloured from their palette indices.
fn hires_picture(rel: &str) -> bool {
    let picture = [".pcx", ".png", ".tga", ".bmp"].iter().any(|e| rel.ends_with(e));
    picture && !["flics/", "bodies/", "heads/"].iter().any(|d| rel.starts_with(d))
}

fn layer_hit<'a>(l: &'a Layer, rel: &str) -> Option<Hit<'a>> {
    if let Some(stem) = rel.strip_suffix(".flc") {
        let png = format!("{stem}.png");
        if let Some(f) = l.art.get(&png).or_else(|| l.index.get(&png)) {
            return Some(Hit::Sheet(f));
        }
        return l.index.get(rel).map(|f| Hit::Classic(f.as_str()));
    }
    let cands = candidates(rel);
    if let Some(f) = cands.iter().find_map(|c| l.index.get(c)) {
        return Some(Hit::Classic(f));
    }
    if hires_picture(rel) {
        return cands.iter().find_map(|c| l.art.get(c)).map(|f| Hit::Art(f.as_str()));
    }
    None
}

/// What the classic loaders read for `rel` from the layers below `below` (None: the game folder's file): the topmost
/// classic file, or a high-resolution picture scaled down.
fn classic_bytes(v: &Vfs, rel: &str, p: &Path, below: usize) -> Option<Vec<u8>> {
    for i in (0..below).rev() {
        let l = &v.layers[i];
        match layer_hit(l, rel) {
            Some(Hit::Classic(f)) => return l.source.read(f),
            Some(Hit::Art(f)) => {
                if let Some(d) = scaled_down(v, rel, p, i, f) {
                    return Some(d);
                }
            }
            _ => {}
        }
    }
    None
}

/// The original's size of the high-resolution picture `data` of layer `i`: its size over the scale, which must be whole
/// and match the picture under it (when there is one). None when it does not fit.
fn art_fits(v: &Vfs, rel: &str, p: &Path, i: usize, data: &[u8]) -> Option<(u32, u32)> {
    let s = v.layers[i].scale;
    let (w, h) = crate::hires::picture_size(data)?;
    if w % s != 0 || h % s != 0 || w == 0 || h == 0 {
        return None;
    }
    let size = (w / s, h / s);
    let under = classic_bytes(v, rel, p, i).or_else(|| crate::fsutil::read_raw(p));
    match under.and_then(|d| crate::hires::picture_size(&d)) {
        Some(u) if u != size => None,
        _ => Some(size),
    }
}

fn scaled_down(v: &Vfs, rel: &str, p: &Path, i: usize, file: &str) -> Option<Vec<u8>> {
    let key = format!("{i}:{rel}");
    if let Some(d) = SCALED.read().ok().and_then(|c| c.as_ref().and_then(|c| c.get(&key).cloned())) {
        return Some(d.to_vec());
    }
    let data = v.layers[i].source.read(file)?;
    art_fits(v, rel, p, i, &data)?;
    let img = crate::png::decode_image(&data)?;
    let small = crate::png::encode_png(&crate::hires::downscale(&img, v.layers[i].scale))?;
    if let Ok(mut c) = SCALED.write() {
        c.get_or_insert_with(HashMap::new).insert(key, Arc::new(small.clone()));
    }
    Some(small)
}

/// A pack's high-resolution picture or sprite sheet for a game file.
pub struct Art {
    /// The file's bytes (a PNG).
    pub data: Vec<u8>,
    /// How many times the original's size it is.
    pub scale: u32,
    /// For a picture, the original's size (the size every layout and cut works in).
    pub size: (u32, u32),
    /// A sprite sheet (for a .flc).
    pub sheet: bool,
    /// The layer it comes from ([`layer_file`]) and the game file's lower-case path.
    pub layer: usize,
    pub rel: String,
}

/// True when a mounted pack has high-resolution pictures or sprite sheets (the loaders' cheap first check).
pub fn art_active() -> bool {
    ART.load(Ordering::Acquire)
}

/// The high-resolution picture or sprite sheet that replaces the game file at `p`: from the topmost pack that has anything
/// for it, when that is such art (a picture only when its size fits, see `art_fits`). None when there is none, or when the
/// topmost replacement is a classic file.
pub fn art(p: &Path) -> Option<Art> {
    if !ART.load(Ordering::Acquire) {
        return None;
    }
    let v = VFS.read().ok()?;
    let v = v.as_ref()?;
    let rel = game_rel(v, p)?;
    for i in (0..v.layers.len()).rev() {
        let l = &v.layers[i];
        match layer_hit(l, &rel) {
            Some(Hit::Classic(_)) => return None,
            Some(Hit::Sheet(f)) => {
                let data = l.source.read(f)?;
                return Some(Art { data, scale: l.scale, size: (0, 0), sheet: true, layer: i, rel });
            }
            Some(Hit::Art(f)) => {
                let data = l.source.read(f)?;
                if let Some(size) = art_fits(v, &rel, p, i, &data) {
                    return Some(Art { data, scale: l.scale, size, sheet: false, layer: i, rel });
                }
            }
            None => {}
        }
    }
    None
}

/// A file of layer `layer` by its lower-case path in the game folder (a sheet's `.json`, `.mask.png` or variants).
pub fn layer_file(layer: usize, rel: &str) -> Option<Vec<u8>> {
    let v = VFS.read().ok()?;
    let l = v.as_ref()?.layers.get(layer)?;
    let rel = rel.to_lowercase();
    match l.index.get(&rel).or_else(|| l.art.get(&rel)) {
        Some(f) => l.source.read(f),
        // a file the override folders do not index (a sheet's .json)
        None => l.source.read(&format!("gfx/{rel}")),
    }
}

/// The id of the pack that replaces the game file at `p`, if any (the HD mode leaves such files to the pack).
pub fn overridden_by(p: &Path) -> Option<String> {
    if !MOUNTED.load(Ordering::Acquire) {
        return None;
    }
    let v = VFS.read().ok()?;
    let v = v.as_ref()?;
    let rel = game_rel(v, p)?;
    if ART.load(Ordering::Acquire) {
        return v.layers.iter().rev().find(|l| layer_hit(l, &rel).is_some()).map(|l| l.id.clone());
    }
    find(&v.layers, &rel).map(|(l, _)| l.id.clone())
}

/// What the layers put at `p` inside the game folder: 0 nothing, 1 a file, 2 a folder.
pub fn virtual_kind(p: &Path) -> u8 {
    if !MOUNTED.load(Ordering::Acquire) {
        return 0;
    }
    let Ok(v) = VFS.read() else { return 0 };
    let Some(v) = v.as_ref() else { return 0 };
    if p == v.game_dir {
        return 2;
    }
    let Some(rel) = game_rel(v, p) else { return 0 };
    if v.dirs.contains_key(&rel) {
        return 2;
    }
    if find(&v.layers, &rel).is_some() {
        return 1;
    }
    0
}

/// The names (as the packs spell them) of the files and folders the layers put directly inside the folder `p` of the game
/// folder, with whether each is a folder.
pub fn virtual_entries(p: &Path) -> Vec<(String, bool)> {
    if !MOUNTED.load(Ordering::Acquire) {
        return Vec::new();
    }
    let Ok(v) = VFS.read() else { return Vec::new() };
    let Some(v) = v.as_ref() else { return Vec::new() };
    let rel = if p == v.game_dir { Some(String::new()) } else { game_rel(v, p) };
    let Some(rel) = rel else { return Vec::new() };
    v.dirs.get(&rel).map(|m| m.values().cloned().collect()).unwrap_or_default()
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
    fn folders_of_the_layers() {
        let a = layer("a", &[("gfx/Interface/BuildPanel.png", b"A"), ("text/Themes/Standard/Story.txt", b"t"), ("fonts/Font.TTF", b"f")]);
        let d = dir_index(&[a]);
        assert_eq!(d[""]["interface"], ("Interface".to_string(), true));
        assert_eq!(d[""]["font.ttf"], ("Font.TTF".to_string(), false));
        assert_eq!(d["themes/standard"]["story.txt"], ("Story.txt".to_string(), false));
        assert_eq!(d["interface"]["buildpanel.png"], ("BuildPanel.png".to_string(), false));
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        crate::png::encode_png(&crate::assets::Rgba::new(w, h)).unwrap()
    }

    fn scaled(id: &str, scale: u32, files: &[(&str, &[u8])]) -> Layer {
        let zip = Zip::open(crate::zip::write(files)).unwrap();
        Layer::with_scale(id, Arc::new(Source::Zip { zip: Arc::new(zip), prefix: String::new() }), scale)
    }

    #[test]
    fn high_resolution_art() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let game = std::env::temp_dir().join(format!("sg-vfs-art-{}", std::process::id()));
        std::fs::create_dir_all(game.join("Interface")).unwrap();
        std::fs::write(game.join("Interface/Pic.pcx"), png(4, 2)).unwrap();
        let (big, odd, sheet, small) = (png(16, 8), png(10, 8), png(8, 8), png(4, 2));
        let files: &[(&str, &[u8])] = &[
            ("gfx/Interface/Pic.png", &big),
            ("gfx/Interface/Odd.png", &odd),
            ("gfx/Flics/Trees/T.png", &sheet),
            ("gfx/Flics/Trees/T.mask.png", &sheet),
            ("gfx/Flics/Trees/T.json", b"{}"),
            ("gfx/Flics/Trees/Old.flc", b"x"),
            ("sfx/Sounds/a.wav", b"w"),
        ];
        let hi = scaled("hi", 4, files);
        assert_eq!(hi.index.len(), 1, "only the sound is read the classic way");
        assert_eq!(hi.art.len(), 4, "the .flc of a high-resolution pack is left out");
        mount(&game, vec![hi]);
        assert!(art_active());
        let pic = game.join("Interface/Pic.pcx");
        let a = art(&pic).unwrap();
        assert_eq!((a.scale, a.size, a.sheet), (4, (4, 2), false));
        // every other reader gets the picture at the original's size
        assert_eq!(crate::hires::picture_size(&read_override(&pic).unwrap()), Some((4, 2)));
        assert_eq!(overridden_by(&pic).as_deref(), Some("hi"));
        // a size that is not a whole multiple is not used at all
        assert!(art(&game.join("Interface/Odd.pcx")).is_none());
        assert!(read_override(&game.join("Interface/Odd.pcx")).is_none());
        // a sheet goes to the sprite loader only
        let t = game.join("Flics/Trees/T.flc");
        assert!(art(&t).unwrap().sheet);
        assert!(read_override(&t).is_none());
        assert!(layer_file(0, "flics/trees/t.mask.png").is_some());
        assert!(layer_file(0, "flics/trees/t.json").is_some(), "found in the pack though not indexed");
        // a picture at the original's size in a later pack wins over the high-resolution one
        let classic = scaled("classic", 1, &[("gfx/Interface/Pic.png", &small)]);
        mount(&game, vec![scaled("hi", 4, files), classic]);
        assert!(art(&pic).is_none());
        assert_eq!(read_override(&pic).unwrap(), small);
        // and a high-resolution pack over a classic one: the classic one is what it is checked against
        let classic = scaled("classic", 1, &[("gfx/Interface/Odd.png", &png(5, 2))]);
        mount(&game, vec![classic, scaled("hi", 4, files)]);
        assert!(art(&game.join("Interface/Odd.pcx")).is_none(), "16 x 8 over 5 x 2");
        assert_eq!(crate::hires::picture_size(&read_override(&game.join("Interface/Odd.pcx")).unwrap()), Some((5, 2)));
        unmount();
        assert!(!art_active());
        let _ = std::fs::remove_dir_all(&game);
    }

    #[test]
    fn candidates_for_art_and_sound() {
        assert_eq!(candidates("a/b.pcx"), ["a/b.pcx", "a/b.png"]);
        assert_eq!(candidates("a/b.wav"), ["a/b.wav", "a/b.ogg"]);
        assert_eq!(candidates("a/b.flc"), ["a/b.flc"]);
    }
}
