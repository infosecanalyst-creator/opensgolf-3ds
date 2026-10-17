//! `opensgolf-free`: the free base set, drawn, synthesised and written by the engine itself so that OpenSGolf runs without
//! any file of the original game (docs/MODDING.md, section 1; packs/opensgolf-free/README.md).
//!
//! The set is a content pack whose files are made when they are read ([`read`]), like OpenGFX is a set of files for
//! OpenTTD; `sgtool freeset DIR` writes them all to a folder. Every file has the name, size and layout the engine expects of
//! the original's file (the same picture sizes, the same places for the pieces the engine cuts out of a sheet, the same
//! sprite views, frame counts and anchors), so everything lands where the engine puts the original's art. Those numbers
//! are the only thing taken from the original: they are in `manifest.txt`, measured by tools/freeset_layout.py. The trees,
//! buildings and golfers are sprite sheets rendered offline from our own models, and the sounds, voices and music are
//! synthesised and composed in code, by tools/freeset_art (baked.rs, packs/opensgolf-free/art); everything else is drawn here at
//! run time (paint.rs, tex.rs, sprite.rs, sound.rs), the text is our own (text.rs) and the two fonts are Liberation Sans
//! Bold (SIL Open Font License).
//!
//! Pictures are PNG files named after the original's PCX, TGA or BMP (the engine takes a PNG for any of those, crate::vfs),
//! except where the engine lists a folder by file type (terrain textures, heads) or reads a palette: those are BMP and PCX.
mod baked;
mod enc;
mod icon;
mod paint;
mod sound;
mod sprite;
mod tex;
mod text;
mod ui;

pub use icon::app_icon;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The version of the drawn set, shown on the Content screen.
pub const VERSION: &str = "0.1.0";

static MANIFEST: &str = include_str!("manifest.txt");

/// A part of a sprite sheet the engine cuts out: its box, a shape class (R rectangle, O rounded rectangle, E ellipse,
/// D diamond) and a tone class (b base, g gold, e green, s grey, w paper, k dark, r red) with a lightness 0..3.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Comp {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub shape: u8,
    pub tone: u8,
    pub light: u8,
}

#[derive(Clone, Debug)]
pub(crate) enum Item {
    /// A picture: size, colour key, the parts the engine cuts (empty with `full`: one picture filling the size).
    Pic {
        w: u32,
        h: u32,
        key: Option<[u8; 3]>,
        comps: Vec<Comp>,
        full: bool,
    },
    /// The alpha sheet of the picture `of` (an index into the entries).
    Alpha {
        w: u32,
        h: u32,
        of: usize,
    },
    /// A palette file (a PCX read for its palette only).
    Pal {
        w: u32,
        h: u32,
    },
    Flc(enc::FlcHead),
    /// A 64 x 64 terrain texture.
    Tex,
    /// A path overlay texture with alpha.
    Overlay {
        w: u32,
        h: u32,
    },
    /// A sound of about this many seconds.
    Wav(f32),
    /// The terrain descriptions file of a theme, with its keys.
    Keys(Vec<String>),
}

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    /// The path in the game folder, as the original spells it.
    pub path: String,
    pub item: Item,
}

pub(crate) struct Manifest {
    pub entries: Vec<Entry>,
    /// Lower-case pack path -> what makes it: an entry, or one of the set's own text files.
    pub by_pack: HashMap<String, Source>,
    pub files: Vec<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum Source {
    Entry(usize),
    Text(usize),
}

fn parse_key(s: &str) -> Option<[u8; 3]> {
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// The path, which may contain spaces, and the numbers after it: `n` fields are taken from the end of the line.
fn split_tail(line: &str, n: usize) -> Option<(String, Vec<String>)> {
    let mut parts: Vec<&str> = line.split(' ').collect();
    if parts.len() < n + 2 {
        return None;
    }
    let tail: Vec<String> = parts.split_off(parts.len() - n).into_iter().map(str::to_string).collect();
    Some((parts[1..].join(" "), tail))
}

fn num(s: &str) -> u32 {
    s.parse().unwrap_or(0)
}

pub(crate) fn manifest() -> &'static Manifest {
    static M: OnceLock<Manifest> = OnceLock::new();
    M.get_or_init(|| {
        let mut entries: Vec<Entry> = Vec::new();
        let mut current_pic: Option<usize> = None;
        for line in MANIFEST.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("  ") {
                let f: Vec<&str> = rest.split_whitespace().collect();
                if let (Some(k), 6) = (current_pic, f.len()) {
                    let tone = f[5].as_bytes();
                    let c = Comp {
                        x: num(f[0]) as f32,
                        y: num(f[1]) as f32,
                        w: num(f[2]) as f32,
                        h: num(f[3]) as f32,
                        shape: f[4].as_bytes()[0],
                        tone: tone[0],
                        light: tone.get(1).map(|d| d - b'0').unwrap_or(1),
                    };
                    if let Item::Pic { comps, .. } = &mut entries[k].item {
                        comps.push(c);
                    }
                }
                continue;
            }
            current_pic = None;
            let kind = line.as_bytes()[0];
            let parsed = match kind {
                b'P' => split_tail(line, 5).map(|(p, t)| {
                    let full = t[3] == "full";
                    (p, Item::Pic { w: num(&t[0]), h: num(&t[1]), key: parse_key(&t[2]), comps: Vec::new(), full })
                }),
                b'A' => split_tail(line, 2).map(|(p, t)| (p, Item::Alpha { w: num(&t[0]), h: num(&t[1]), of: usize::MAX })),
                b'L' => split_tail(line, 2).map(|(p, t)| (p, Item::Pal { w: num(&t[0]), h: num(&t[1]) })),
                b'F' => split_tail(line, 9).map(|(p, t)| {
                    let n: Vec<u32> = t.iter().map(|s| num(s)).collect();
                    let head = enc::FlcHead {
                        w: n[0],
                        h: n[1],
                        views: n[2],
                        fpv: n[3].max(1),
                        crop: (n[4], n[5]),
                        view_ms: n[6],
                        mask: n[7],
                        speed: n[8].max(1),
                    };
                    (p, Item::Flc(head))
                }),
                b'T' => Some((line[2..].to_string(), Item::Tex)),
                b'G' => split_tail(line, 2).map(|(p, t)| (p, Item::Overlay { w: num(&t[0]), h: num(&t[1]) })),
                b'W' => split_tail(line, 1).map(|(p, t)| (p, Item::Wav(t[0].parse().unwrap_or(1.0)))),
                b'K' => {
                    // K path keys: the path has no spaces in this case
                    let mut it = line[2..].splitn(2, ' ');
                    let p = it.next().unwrap_or("").to_string();
                    let keys = it.next().unwrap_or("").split('|').map(str::to_string).collect();
                    Some((p, Item::Keys(keys)))
                }
                _ => None,
            };
            if let Some((path, item)) = parsed {
                if matches!(item, Item::Pic { .. }) {
                    current_pic = Some(entries.len());
                }
                entries.push(Entry { path, item });
            }
        }
        // each alpha sheet belongs to the picture of the same name without _A / _alpha (SGAreport_alpha to SGA)
        let index: HashMap<String, usize> = entries.iter().enumerate().map(|(i, e)| (e.path.to_lowercase(), i)).collect();
        for i in 0..entries.len() {
            if let Item::Alpha { w, h, .. } = entries[i].item {
                let low = entries[i].path.to_lowercase();
                let base = low.replace("_alpha.pcx", ".pcx").replace("_a.pcx", ".pcx").replace("sgareport.pcx", "sga.pcx");
                let of = index.get(&base).copied().unwrap_or(usize::MAX);
                entries[i].item = Item::Alpha { w, h, of };
            }
        }
        let mut by_pack = HashMap::new();
        let mut files = Vec::new();
        for (i, e) in entries.iter().enumerate() {
            if matches!(e.item, Item::Alpha { of: usize::MAX, .. }) {
                continue;
            }
            let p = pack_path(e);
            by_pack.insert(p.to_lowercase(), Source::Entry(i));
            files.push(p);
        }
        for (i, rel) in text::FILES.iter().enumerate() {
            let p = format!("{}/{}", if rel.to_lowercase().ends_with(".ttf") { "fonts" } else { "text" }, rel);
            by_pack.insert(p.to_lowercase(), Source::Text(i));
            files.push(p);
        }
        files.sort();
        Manifest { entries, by_pack, files }
    })
}

/// Where an entry's file is in the pack: pictures as PNG under gfx/ (BMP, TGA and PCX where the engine needs the type),
/// sounds under sfx/, text under text/.
fn pack_path(e: &Entry) -> String {
    let p = &e.path;
    let low = p.to_lowercase();
    let png = |p: &str| format!("gfx/{}.png", &p[..p.rfind('.').unwrap_or(p.len())]);
    match &e.item {
        Item::Pic { .. } if low.starts_with("heads/") || low.starts_with("bodies/") => format!("gfx/{p}"),
        Item::Pic { .. } | Item::Alpha { .. } => png(p),
        Item::Wav(_) => format!("sfx/{p}"),
        Item::Keys(_) => format!("text/{p}"),
        _ => format!("gfx/{p}"),
    }
}

/// Every file of the set, as paths in the pack (gfx/..., sfx/..., text/..., fonts/...), sorted.
pub fn files() -> Vec<String> {
    manifest().files.clone()
}

/// The bytes of one file of the set, by its path in the pack (case-insensitive), made now.
pub fn read(rel: &str) -> Option<Vec<u8>> {
    let m = manifest();
    match *m.by_pack.get(&rel.to_lowercase())? {
        Source::Text(i) => text::make(text::FILES[i]),
        Source::Entry(i) => make(m, i),
    }
}

fn make(m: &Manifest, i: usize) -> Option<Vec<u8>> {
    let e = &m.entries[i];
    let low = e.path.to_lowercase();
    match &e.item {
        // the interface pictures drawn by tools/freeset_art/ui.py, else drawn here
        Item::Pic { .. } => baked::get(&pack_path(e)).map(<[u8]>::to_vec).or_else(|| ui::picture(e, &low)),
        Item::Alpha { of, .. } => baked::get(&pack_path(e)).map(<[u8]>::to_vec).or_else(|| ui::alpha(&m.entries[*of], &low)),
        Item::Pal { w, h } => Some(sprite::palette_file(&low, *w, *h)),
        Item::Flc(head) => {
            let shadow = low.ends_with("shadow.flc").then(sprite::shadow_palette);
            baked::flc(&e.path, head, shadow.as_ref()).or_else(|| Some(sprite::flc(&low, head)))
        }
        Item::Tex => Some(tex::texture(&e.path)),
        Item::Overlay { w, h } => Some(tex::overlay(&low, *w, *h)),
        Item::Wav(secs) => {
            // the recorded and generated sounds of tools/freeset_art (Ogg Vorbis, which the engine takes for a WAV)
            let stem = &e.path[..e.path.rfind('.').unwrap_or(e.path.len())];
            baked::get(&format!("{stem}.ogg"))
                .or_else(|| sound::alias(stem).and_then(|a| baked::get(&format!("{a}.ogg"))))
                .map(<[u8]>::to_vec)
                .or_else(|| Some(sound::make(&low, *secs)))
        }
        Item::Keys(keys) => Some(text::terrain_text(&low, keys)),
    }
}

/// The set's pack.toml (the same as packs/opensgolf-free/pack.toml in the repository).
pub const PACK_TOML: &str = include_str!("pack.toml");

/// Writes every file of the set under `dir` (the layout of a pack folder) with its pack.toml. Returns the number of files
/// and bytes written.
pub fn write_all(dir: &std::path::Path) -> Result<(usize, usize), String> {
    let mut n = 1;
    let mut bytes = PACK_TOML.len();
    if !crate::fsutil::write_file(dir.join("pack.toml"), PACK_TOML.as_bytes()) {
        return Err(format!("cannot write {}", dir.join("pack.toml").display()));
    }
    for f in files() {
        let d = read(&f).ok_or_else(|| format!("cannot make {f}"))?;
        let path = dir.join(&f);
        if !crate::fsutil::write_file(&path, &d) {
            return Err(format!("cannot write {}", path.display()));
        }
        n += 1;
        bytes += d.len();
    }
    Ok((n, bytes))
}

/// Theme of a path: 0 Parkland, 1 Desert, 2 Tropical, 3 Links (the exe's theme order), from the folder or file name.
pub(crate) fn theme_of(low: &str) -> usize {
    if low.contains("desert") || low.contains("/des") || low.contains("/d1") || low.contains("/d2") || low.contains("/d3") {
        1
    } else if low.contains("trop") {
        2
    } else if low.contains("links") || low.contains("/links/") {
        3
    } else {
        0
    }
}

/// A stable number from a string (picks variations by file name).
pub(crate) fn hash(s: &str) -> u32 {
    let mut h = 2166136261u32;
    for b in s.bytes() {
        h = (h ^ b.to_ascii_lowercase() as u32).wrapping_mul(16777619);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pack_folder_matches() {
        let m = crate::packs::Manifest::parse(PACK_TOML).unwrap();
        assert_eq!((m.id.as_str(), m.version.as_str()), (crate::packs::FREE, VERSION));
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/opensgolf-free/pack.toml");
        if let Ok(t) = std::fs::read_to_string(file) {
            assert_eq!(t.replace('\r', ""), PACK_TOML.replace('\r', ""));
        }
    }

    #[test]
    fn manifest_reads() {
        let m = manifest();
        assert!(m.entries.len() > 5000, "{}", m.entries.len());
        let alpha_ok = m.entries.iter().filter(|e| matches!(e.item, Item::Alpha { of, .. } if of != usize::MAX)).count();
        assert!(alpha_ok >= 25);
        assert!(m.by_pack.contains_key("gfx/interface/titlebase.png"));
        assert!(m.by_pack.contains_key("fonts/klepto__.ttf"));
    }

    #[test]
    fn a_file_of_each_kind_decodes() {
        let pic = read("gfx/Interface/TitleBASE.png").unwrap();
        assert_eq!(crate::assets::decode_pcx(&pic).map(|i| (i.w, i.h)), Ok((800, 600)));
        let tex = read("gfx/Data/Textures/Parkland/FairwayA0001.bmp").unwrap();
        assert_eq!(crate::assets::decode_bmp(&tex).map(|i| (i.w, i.h)), Ok((64, 64)));
        let f = crate::flc::decode_flc(&read("gfx/Flics/Male/MaleKLS_NormalWalk.flc").unwrap()).unwrap();
        assert_eq!((f.views, f.frames.len()), (8, 8 * f.frames_per_view as usize));
        let w = crate::assets::decode_sound(&read("sfx/Sounds/Applause.wav").unwrap()).unwrap();
        assert!(!w.pcm.is_empty());
        assert!(read("text/Themes/Standard/progolfers.dta").is_some());
    }
}
