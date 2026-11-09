//! The built-in HD pack maker (Preferences > Graphics > Make HD graphics from my SimGolf files..., docs/HD.md): the same pack
//! tools/hd_pack/hd_pack.py writes, made inside the game from the player's own files with the xBRZ upscaler (crate::xbrz).
//! The port's own.
//!
//! What goes in the pack, where, and how each kind is prepared mirror hd_pack.py exactly (its `collect` and `process`), so
//! the game's loader (crates/simgolf/src/hd.rs) takes either: every PCX outside Flics and Bodies with the port's colour keys
//! made transparent (`images/<path>.png`), the alpha sheets as grey masks, the terrain textures as wrapping tiles
//! (`textures/<path>.png`), and the frames of every sprite animation but the people and the shadows
//! (`sprites/<path>/NNNN.png`, palette index 255 clear), each exactly `scale` times its source, listed in `manifest.json`.
//! Sprites drawn in another palette are recoloured from these frames at load time (hd.rs), as with a hd_pack.py pack.
//!
//! The pieces here do no threading and keep no clock: the game runs them on threads natively and a few pictures a frame in
//! a browser (crates/simgolf/src/hd_make.rs). Files are read as they are on disk, never through content packs.

use crate::assets::Rgba;
use crate::fsutil;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PACK_FORMAT: &str = "simgolf-hd-pack";
pub const PACK_VERSION: u32 = 1;
/// The manifest's `method` of a pack made with the built-in upscaler.
pub const METHOD_XBRZ: &str = "xbrz";

/// Colour keys the port's loaders apply besides magenta, by file (hd_pack.py EXTRA_KEYS).
const EXTRA_KEYS: [(&str, [u8; 3]); 7] = [
    ("interface/starsheartsetc.pcx", [0x00, 0xFF, 0x00]),
    ("interface/memberpanel.pcx", [0x6B, 0x6B, 0x9C]),
    ("gbubbles.pcx", [0x9B, 0xE7, 0xFF]),
    ("interface/chooseparklandbuttons.pcx", [0xFF, 0x00, 0x00]),
    ("interface/chooselinksbuttons.pcx", [0xFF, 0x00, 0x00]),
    ("interface/choosedesertbuttons.pcx", [0xFF, 0x00, 0x00]),
    ("interface/choosetropicalbuttons.pcx", [0xFF, 0x00, 0x00]),
];

/// Sprite folders the game recolours per person: not upscaled (hd_pack.py PEOPLE_DIRS).
const PEOPLE_DIRS: [&str; 4] = ["flics/male/", "flics/female/", "flics/celebs/", "flics/employee/"];

/// Pixels of context around a wrapping texture (more than xBRZ's reach of two).
const WRAP_PAD: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A PCX of the interface and the rest; `mask` for an alpha sheet (`*_a.pcx`, `*_alpha.pcx`).
    Image { mask: bool },
    /// A terrain texture (Data/Textures/**.bmp, .tga).
    Texture,
    /// An FLC's frames.
    Sprite,
}

/// A source picture of the pack.
#[derive(Clone, Debug)]
pub struct Source {
    /// The manifest key: the path relative to the game folder, lower case.
    pub key: String,
    /// The path relative to the game folder as the files name it ('/').
    pub rel: String,
    pub path: PathBuf,
    pub kind: Kind,
}

/// One picture to upscale and where it goes in the pack.
pub struct Picture {
    pub out: String,
    pub img: Rgba,
    pub wrap: bool,
    pub mask: bool,
}

/// A source decoded: its manifest entry and its pictures.
pub struct Prepared {
    pub key: String,
    pub entry: serde_json::Value,
    pub pictures: Vec<Picture>,
}

/// A top level folder of the game, matched without case.
fn find_dir(game: &Path, name: &str) -> Option<PathBuf> {
    fsutil::list_raw(game)
        .into_iter()
        .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name)) && fsutil::is_dir_raw(p))
}

/// Every file under `dir`, sorted, as on disk.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for p in fsutil::list_raw(&d) {
            if fsutil::is_dir_raw(&p) {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn rel_of(game: &Path, p: &Path) -> Option<String> {
    let r = p.strip_prefix(game).ok()?;
    Some(r.to_string_lossy().replace('\\', "/").trim_start_matches('/').to_string())
}

/// True when `game` looks like the game folder (Flics and Interface in it).
pub fn is_game_folder(game: &Path) -> bool {
    find_dir(game, "Flics").is_some() && find_dir(game, "Interface").is_some()
}

/// The pack's sources in the game folder, in hd_pack.py's order: images, textures, sprites. `only`: a part of the path the
/// sources must contain (any case; for trying things out, as hd_pack.py's --only).
pub fn collect(game: &Path, only: Option<&str>) -> Vec<Source> {
    let only = only.map(str::to_lowercase);
    let matches = |key: &str| only.as_deref().is_none_or(|o| key.contains(o));
    let flics = find_dir(game, "Flics");
    let bodies = find_dir(game, "Bodies");
    let mut out = Vec::new();
    for path in walk(game) {
        let Some(rel) = rel_of(game, &path) else { continue };
        let key = rel.to_lowercase();
        if !key.ends_with(".pcx") || !matches(&key) {
            continue;
        }
        if [&flics, &bodies].iter().any(|d| d.as_ref().is_some_and(|d| path.starts_with(d))) {
            continue;
        }
        let mask = key.ends_with("_a.pcx") || key.ends_with("_alpha.pcx");
        out.push(Source { key, rel, path, kind: Kind::Image { mask } });
    }
    if let Some(tex) = find_dir(game, "Data").and_then(|d| find_dir(&d, "Textures")) {
        for path in walk(&tex) {
            let Some(rel) = rel_of(game, &path) else { continue };
            let key = rel.to_lowercase();
            if (key.ends_with(".bmp") || key.ends_with(".tga")) && matches(&key) {
                out.push(Source { key, rel, path, kind: Kind::Texture });
            }
        }
    }
    if let Some(flics) = &flics {
        for path in walk(flics) {
            let Some(rel) = rel_of(game, &path) else { continue };
            let key = rel.to_lowercase();
            if !key.ends_with(".flc") || key.ends_with("shadow.flc") || !matches(&key) {
                continue;
            }
            if PEOPLE_DIRS.iter().any(|p| key.starts_with(p)) {
                continue;
            }
            out.push(Source { key, rel, path, kind: Kind::Sprite });
        }
    }
    out
}

/// Width, height and number of pictures of a source, from its header alone (for the time and space estimate).
pub fn header_size(s: &Source) -> Option<(u32, u32, u32)> {
    let d = fsutil::read_head_raw(&s.path, 128)?;
    let u16le = |o: usize| d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32);
    let u32le = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    match s.kind {
        Kind::Image { .. } => Some((u16le(8)?.checked_sub(u16le(4)?)? + 1, u16le(10)?.checked_sub(u16le(6)?)? + 1, 1)),
        Kind::Texture if s.key.ends_with(".tga") => Some((u16le(12)?, u16le(14)?, 1)),
        Kind::Texture => Some((u32le(18)?, (u32le(22)? as i32).unsigned_abs(), 1)),
        // an FLC's frame count less the ring frames, which are not kept, is its header's count either way (flc.rs)
        Kind::Sprite => Some((u16le(8)?, u16le(10)?, u16le(6)?)),
    }
}

/// RGB to RGBA with the port's key colours clear: magenta as the port tests it (255,0,255 and the near-magenta 248,0,248:
/// red and blue 240 or more, green 8 or less) and the file's own key colours (hd_pack.py keyed_rgba).
fn key_colours(img: &mut Rgba, key: &str) {
    let extra: Vec<[u8; 3]> = EXTRA_KEYS.iter().filter(|(k, _)| *k == key).map(|(_, c)| *c).collect();
    for p in img.px.chunks_exact_mut(4) {
        let magenta = p[0] >= 240 && p[1] <= 8 && p[2] >= 240;
        p[3] = if magenta || extra.iter().any(|c| p[..3] == c[..]) { 0 } else { 255 };
    }
}

/// Decodes a source into its pictures (None: nothing to pack, a 1x1 placeholder). Err: a file that cannot be read, which
/// is left out of the pack as hd_pack.py leaves it.
pub fn prepare(s: &Source) -> Result<Option<Prepared>, String> {
    let data = fsutil::read_raw(&s.path).ok_or("cannot read")?;
    let stem = &s.rel[..s.rel.len() - 4];
    let entry = |file: &str, w: u32, h: u32| serde_json::json!({"file": file, "w": w, "h": h});
    match s.kind {
        Kind::Image { mask } => {
            let mut img = crate::assets::decode_pcx(&data)?;
            if img.w == 1 && img.h == 1 {
                return Ok(None);
            }
            if !mask {
                key_colours(&mut img, &s.key);
            }
            let out = format!("images/{stem}.png");
            Ok(Some(Prepared {
                key: s.key.clone(),
                entry: entry(&out, img.w, img.h),
                pictures: vec![Picture { out, img, wrap: false, mask }],
            }))
        }
        Kind::Texture => {
            let img = if s.key.ends_with(".tga") { crate::assets::decode_tga(&data)? } else { crate::assets::decode_bmp(&data)? };
            let out = format!("textures/{stem}.png");
            Ok(Some(Prepared {
                key: s.key.clone(),
                entry: entry(&out, img.w, img.h),
                pictures: vec![Picture { out, img, wrap: true, mask: false }],
            }))
        }
        Kind::Sprite => {
            let f = crate::flc::decode_flc(&data)?;
            if f.frames.is_empty() {
                return Err("no frames".into());
            }
            let dir = format!("sprites/{stem}");
            let pictures = f
                .frames
                .iter()
                .enumerate()
                .map(|(i, fr)| Picture {
                    out: format!("{dir}/{i:04}.png"),
                    img: crate::assets::to_rgba(fr, Some(255)),
                    wrap: false,
                    mask: false,
                })
                .collect::<Vec<_>>();
            let entry = serde_json::json!({"dir": dir, "w": f.w, "h": f.h, "frames": pictures.len()});
            Ok(Some(Prepared { key: s.key.clone(), entry, pictures }))
        }
    }
}

/// Per pixel, the colour of the nearest opaque pixel (its own when it is not clear), found breadth first: what a clear pixel
/// carries so that the texture filtering at an outline mixes in the picture's own colours, never black or the key (as
/// hd_pack.py's push-pull fill). Black for a picture with no opaque pixel.
pub fn fill_colours(img: &Rgba) -> Vec<[u8; 3]> {
    let (w, h) = (img.w as usize, img.h as usize);
    let mut col = vec![[0u8; 3]; w * h];
    let mut seen = vec![false; w * h];
    let mut queue = std::collections::VecDeque::new();
    for (i, p) in img.px.chunks_exact(4).enumerate() {
        if p[3] != 0 {
            col[i] = [p[0], p[1], p[2]];
            seen[i] = true;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        let c = col[i];
        let mut visit = |j: usize| {
            if !seen[j] {
                seen[j] = true;
                col[j] = c;
                queue.push_back(j);
            }
        };
        if x > 0 {
            visit(i - 1);
        }
        if x + 1 < w {
            visit(i + 1);
        }
        if y > 0 {
            visit(i - w);
        }
        if y + 1 < h {
            visit(i + w);
        }
    }
    col
}

/// The picture with `p` pixels of itself around it, tiled (for a wrapping texture).
fn pad_wrap(img: &Rgba, p: u32) -> Rgba {
    let (w, h) = (img.w as i64, img.h as i64);
    let mut out = Rgba::new(img.w + 2 * p, img.h + 2 * p);
    let ow = out.w as i64;
    for y in 0..out.h as i64 {
        for x in 0..ow {
            let sx = (x - p as i64).rem_euclid(w);
            let sy = (y - p as i64).rem_euclid(h);
            let s = ((sy * w + sx) * 4) as usize;
            let o = ((y * ow + x) * 4) as usize;
            out.px[o..o + 4].copy_from_slice(&img.px[s..s + 4]);
        }
    }
    out
}

fn crop(img: &Rgba, x0: u32, y0: u32, w: u32, h: u32) -> Rgba {
    let mut out = Rgba::new(w, h);
    let (sw, row) = (img.w as usize * 4, w as usize * 4);
    for y in 0..h as usize {
        let s = (y + y0 as usize) * sw + x0 as usize * 4;
        out.px[y * row..(y + 1) * row].copy_from_slice(&img.px[s..s + row]);
    }
    out
}

/// A mask's grey (Pillow's "L": 299/587/114 per mille), opaque.
fn grey(img: &Rgba) -> Rgba {
    let mut out = img.clone();
    for p in out.px.chunks_exact_mut(4) {
        let l = ((p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000) as u8;
        p.copy_from_slice(&[l, l, l, 255]);
    }
    out
}

/// The input of an upscaler for a picture: a mask as grey, a texture with WRAP_PAD pixels of its own wrapped around it
/// (the upscaled border is cropped off again by [`finish`]).
pub fn upscaler_input(p: &Picture) -> Rgba {
    if p.mask {
        grey(&p.img)
    } else if p.wrap {
        pad_wrap(&p.img, WRAP_PAD)
    } else {
        p.img.clone()
    }
}

/// The upscaler's output for a picture back to the picture's own area, and every clear pixel given the colour of the
/// nearest opaque source pixel ([`fill_colours`]): the pack's PNG.
pub fn finish(p: &Picture, big: Rgba, scale: u32) -> Rgba {
    let (w, h) = (p.img.w * scale, p.img.h * scale);
    let mut out = if p.wrap && !p.mask { crop(&big, WRAP_PAD * scale, WRAP_PAD * scale, w, h) } else { big };
    if out.px.chunks_exact(4).any(|q| q[3] == 0) {
        let fill = fill_colours(&p.img);
        let (sw, s, ow) = (p.img.w as usize, scale as usize, w as usize);
        for (i, q) in out.px.chunks_exact_mut(4).enumerate() {
            if q[3] == 0 {
                q[..3].copy_from_slice(&fill[(i / ow / s) * sw + i % ow / s]);
            }
        }
    }
    out
}

/// A picture upscaled with xBRZ: the pack's PNG.
pub fn upscale(p: &Picture, scale: u32) -> Rgba {
    finish(p, crate::xbrz::scale(&upscaler_input(p), scale), scale)
}

/// The manifest's method of the built-in AI upscaler (aiscale).
pub const METHOD_AI: &str = "ai:realesr-general-wdn-x4v3";
/// The texture's own pixels wrapped round it for the AI upscaler (its reach is much longer than xBRZ's few pixels).
const AI_WRAP_PAD: u32 = 16;

/// A picture upscaled with the built-in AI upscaler (x2 or x4; other scales with xBRZ): the colours from the model, the
/// transparency from xBRZ's upscale of the same picture, as with Real-ESRGAN. A mask (grey levels, not a picture) keeps
/// xBRZ's.
pub fn upscale_ai(p: &Picture, scale: u32) -> Rgba {
    let mut out = upscale(p, scale);
    if p.mask || !(scale == 2 || scale == 4) {
        return out;
    }
    // the colours with the clear pixels filled from their opaque neighbours (as the model would see nothing there)
    let (src, pad) = if p.wrap { (pad_wrap(&p.img, AI_WRAP_PAD), AI_WRAP_PAD) } else { (p.img.clone(), 0) };
    let fill = fill_colours(&src);
    let (w, h) = (src.w as usize, src.h as usize);
    let rgb: Vec<u8> = fill.iter().flat_map(|c| *c).collect();
    // what is used: the opaque pixels (all of a texture's own, none of the wrapped border)
    let need: Vec<bool> = (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as u32, (i / w) as u32);
            let inside = x >= pad && y >= pad && x < pad + p.img.w && y < pad + p.img.h;
            inside && (p.wrap || src.px[i * 4 + 3] != 0)
        })
        .collect();
    if !need.iter().any(|&n| n) {
        return out;
    }
    let big = crate::aiscale::upscale4(&rgb, w, h, Some(&need));
    let bw = w * 4;
    let (ow, s) = (out.w as usize, scale as usize);
    let o = pad as usize * 4;
    for (i, q) in out.px.chunks_exact_mut(4).enumerate() {
        if q[3] == 0 {
            continue;
        }
        let (x, y) = (i % ow, i / ow);
        // x4: the model's pixel; x2: the mean of its 2 x 2
        let k = 4 / s;
        let mut sum = [0u32; 3];
        for dy in 0..k {
            for dx in 0..k {
                let b = ((o + y * k + dy) * bw + o + x * k + dx) * 3;
                for c in 0..3 {
                    sum[c] += big[b + c] as u32;
                }
            }
        }
        let n = (k * k) as u32;
        for c in 0..3 {
            q[c] = ((sum[c] + n / 2) / n) as u8;
        }
    }
    out
}

// ---- the manifest ------------------------------------------------------------------------------------------------------------

/// A pack's manifest: what hd_pack.py's write_manifest writes.
#[derive(Clone, Debug, Default)]
pub struct Manifest {
    pub scale: u32,
    pub method: String,
    pub files: BTreeMap<String, serde_json::Value>,
}

impl Manifest {
    /// The manifest in `dir`, if there is one that reads.
    pub fn read(dir: &Path) -> Option<Manifest> {
        let v: serde_json::Value = serde_json::from_slice(&fsutil::read_raw(dir.join("manifest.json"))?).ok()?;
        if v["format"] != PACK_FORMAT {
            return None;
        }
        Some(Manifest {
            scale: v["scale"].as_u64()? as u32,
            method: v["method"].as_str().unwrap_or("").to_string(),
            files: v["files"].as_object().map(|m| m.iter().map(|(k, e)| (k.clone(), e.clone())).collect()).unwrap_or_default(),
        })
    }

    pub fn to_json(&self) -> String {
        let v = serde_json::json!({
            "format": PACK_FORMAT,
            "version": PACK_VERSION,
            "scale": self.scale,
            "method": self.method,
            "note": "Made locally from the user's own copy of SimGolf. Derived from EA's art: not to be shared.",
            "files": self.files,
        });
        serde_json::to_string_pretty(&v).unwrap_or_default()
    }

    /// Writes the manifest into `dir` (natively through a temporary file, so a cut-off write never leaves half a manifest).
    pub fn write(&self, dir: &Path) -> bool {
        let path = dir.join("manifest.json");
        #[cfg(not(target_arch = "wasm32"))]
        {
            let tmp = dir.join("manifest.json.tmp");
            fsutil::write_file(&tmp, self.to_json().as_bytes()) && std::fs::rename(&tmp, &path).is_ok()
        }
        #[cfg(target_arch = "wasm32")]
        fsutil::write_file(&path, self.to_json().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32, h: u32, px: &[[u8; 4]]) -> Rgba {
        Rgba { w, h, px: px.concat() }
    }

    #[test]
    fn keys_are_cleared_as_the_port_keys_them() {
        let mut i = img(4, 1, &[[255, 0, 255, 255], [248, 0, 248, 255], [0, 255, 0, 255], [10, 20, 30, 255]]);
        key_colours(&mut i, "interface/other.pcx");
        assert_eq!(i.px.chunks(4).map(|p| p[3]).collect::<Vec<_>>(), [0, 0, 255, 255]);
        let mut i = img(2, 1, &[[0, 255, 0, 255], [0, 254, 0, 255]]);
        key_colours(&mut i, "interface/starsheartsetc.pcx");
        assert_eq!(i.px.chunks(4).map(|p| p[3]).collect::<Vec<_>>(), [0, 255]);
    }

    #[test]
    fn clear_pixels_take_the_nearest_opaque_colour() {
        let i = img(4, 1, &[[9, 9, 9, 0], [200, 0, 0, 255], [0, 0, 0, 0], [0, 0, 200, 255]]);
        assert_eq!(fill_colours(&i), [[200, 0, 0], [200, 0, 0], [200, 0, 0], [0, 0, 200]]);
        assert_eq!(fill_colours(&img(1, 1, &[[1, 2, 3, 0]])), [[0, 0, 0]]);
    }

    #[test]
    fn a_keyed_sprite_scales_without_pink_fringes() {
        // a 6x6 green blob on the magenta key, keyed then upscaled: every pixel keeps clear of the key colour, opaque or not
        let mut px = vec![[255, 0, 255, 255]; 36];
        for (i, p) in px.iter_mut().enumerate() {
            let (x, y) = (i % 6, i / 6);
            if (1..5).contains(&x) && (1..5).contains(&y) && !(x == 1 && y == 1) {
                *p = [40, 160, 60, 255];
            }
        }
        let mut i = img(6, 6, &px);
        key_colours(&mut i, "x.pcx");
        let p = Picture { out: String::new(), img: i, wrap: false, mask: false };
        for scale in [2, 4] {
            let out = upscale(&p, scale);
            assert_eq!((out.w, out.h), (6 * scale, 6 * scale));
            for q in out.px.chunks_exact(4) {
                assert_eq!(q[..3], [40, 160, 60], "a colour other than the blob's: {q:?}");
            }
            assert!(out.px.chunks_exact(4).any(|q| q[3] == 0) && out.px.chunks_exact(4).any(|q| q[3] == 255));
        }
    }

    #[test]
    fn textures_wrap_and_masks_stay_grey() {
        // a texture with a stripe at its left edge: the upscale of its right edge sees the stripe across the seam
        let mut px = vec![[100, 100, 100, 255]; 16];
        for y in 0..4 {
            px[y * 4] = [0, 0, 0, 255];
        }
        let t = Picture { out: String::new(), img: img(4, 4, &px), wrap: true, mask: false };
        let out = upscale(&t, 4);
        assert_eq!((out.w, out.h), (16, 16));
        assert!(out.px.chunks_exact(4).all(|q| q[3] == 255));
        let m = Picture { out: String::new(), img: img(2, 1, &[[255, 255, 255, 255], [0, 0, 0, 255]]), wrap: false, mask: true };
        let out = upscale(&m, 2);
        assert!(out.px.chunks_exact(4).all(|q| q[0] == q[1] && q[1] == q[2] && q[3] == 255));
    }

    #[test]
    fn the_manifest_reads_back() {
        let mut m = Manifest { scale: 4, method: METHOD_XBRZ.into(), files: BTreeMap::new() };
        m.files.insert("interface/a.pcx".into(), serde_json::json!({"file": "images/Interface/a.png", "w": 3, "h": 2}));
        let v: serde_json::Value = serde_json::from_str(&m.to_json()).unwrap();
        assert_eq!(v["format"], PACK_FORMAT);
        assert_eq!(v["version"], 1);
        assert_eq!(v["files"]["interface/a.pcx"]["w"], 3);
    }
}
