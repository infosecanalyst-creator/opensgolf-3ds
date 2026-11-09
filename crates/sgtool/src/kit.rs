//! `sgtool hd-kit` and `sgtool pack-check`: the starter kit for drawing your own high-resolution art, and the checker for
//! the pack you make with it (docs/MODDING.md, "High-resolution packs and your own art").
use sg_core::assets::{decode_bmp, decode_pcx, decode_tga, Rgba};
use sg_core::freeset::Reference;
use sg_core::hires::{picture_size, upscale_nearest, Grid, SheetJson};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The tEXt keyword the kit's reference pictures carry, so `pack-check` can tell when one ended up in a pack.
const MARK: &str = "opensgolf-reference";

/// What a pack file stands for.
#[derive(Clone, Debug, PartialEq)]
enum Item {
    Picture { w: u32, h: u32 },
    Sprite(Grid),
}

// ---- PNG text chunk ---------------------------------------------------------------------------------------------------------

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// A PNG with a tEXt chunk (keyword, text) after its header.
fn with_text(png: Vec<u8>, key: &str, text: &str) -> Vec<u8> {
    let mut body = key.as_bytes().to_vec();
    body.push(0);
    body.extend_from_slice(text.as_bytes());
    let mut chunk = (body.len() as u32).to_be_bytes().to_vec();
    let start = chunk.len();
    chunk.extend_from_slice(b"tEXt");
    chunk.extend_from_slice(&body);
    let crc = crc32(&chunk[start..]);
    chunk.extend_from_slice(&crc.to_be_bytes());
    // signature (8) + IHDR (4 + 4 + 13 + 4)
    let mut out = png[..33].to_vec();
    out.extend(chunk);
    out.extend_from_slice(&png[33..]);
    out
}

/// True when a PNG carries the kit's reference mark.
fn is_reference(d: &[u8]) -> bool {
    let mut p = 8;
    while p + 12 <= d.len() {
        let len = u32::from_be_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]) as usize;
        if &d[p + 4..p + 8] == b"tEXt" && d.get(p + 8..p + 8 + len).is_some_and(|b| b.starts_with(MARK.as_bytes())) {
            return true;
        }
        if &d[p + 4..p + 8] == b"IDAT" {
            return false;
        }
        p += 12 + len;
    }
    false
}

// ---- the originals ----------------------------------------------------------------------------------------------------------

/// Pictures a high-resolution pack cannot replace: the bodies and heads are recoloured from their palette indices.
fn fixed_folder(rel: &str) -> bool {
    let low = rel.to_lowercase();
    low.starts_with("bodies/") || low.starts_with("heads/")
}

/// The layout of an original sprite or picture file of the game folder (any reader of the file; None when it is not art a
/// pack can redraw).
fn original(path: &Path, rel: &str) -> Option<Item> {
    let low = rel.to_lowercase();
    match sg_core::freeset::reference(rel) {
        Some(Reference::Palette | Reference::Alpha { .. }) => return None,
        _ if fixed_folder(rel) => return None,
        _ => {}
    }
    if low.ends_with(".flc") {
        let shadow = low.ends_with("shadow.flc");
        let s = sg_core::sprites::load_sprite(path, shadow, None).ok()?;
        return Some(Item::Sprite(Grid::of(&s)));
    }
    if low.starts_with("flics/") {
        return None; // palettes
    }
    let d = sg_core::fsutil::read_file(path)?;
    let (w, h) = picture_size(&d)?;
    Some(Item::Picture { w, h })
}

/// The original's file for a pack picture or sheet (`rel`: its path under gfx/ without the extension), from the game folder
/// when given, else from the free set's layout manifest.
fn original_for(stem: &str, game: Option<&Path>) -> Option<(String, Item)> {
    for ext in [".flc", ".pcx", ".tga", ".bmp", ".png"] {
        let rel = format!("{stem}{ext}");
        if let Some(g) = game {
            let p = sg_core::fsutil::resolve(g, &rel);
            if p.exists() {
                return original(&p, &rel).map(|i| (rel, i));
            }
        }
        match sg_core::freeset::reference(&rel) {
            Some(Reference::Sprite { w, h, views, frames, ax, ay, .. }) => {
                return Some((rel, Item::Sprite(Grid { views, frames, w, h, ax, ay })));
            }
            Some(Reference::Picture { w, h } | Reference::Overlay { w, h }) => return Some((rel, Item::Picture { w, h })),
            Some(Reference::Texture) => return Some((rel, Item::Picture { w: 64, h: 64 })),
            Some(_) => return None,
            None => {}
        }
    }
    None
}

// ---- hd-kit -----------------------------------------------------------------------------------------------------------------

/// A sprite's frames as one sheet: a row per view, a column per frame.
fn sprite_sheet(s: &sg_core::sprites::Sprite) -> Rgba {
    let g = Grid::of(s);
    let (w, h) = g.sheet_size(1);
    let mut sheet = Rgba::new(w, h);
    for i in 0..g.count().min(s.frames.len()) {
        let f = s.frame(i);
        let (v, c) = (i as u32 / g.frames, i as u32 % g.frames);
        for y in 0..g.h {
            let src = (y * f.w * 4) as usize;
            let dst = (((v * g.h + y) * w + c * g.w) * 4) as usize;
            sheet.px[dst..dst + (g.w * 4) as usize].copy_from_slice(&f.px[src..src + (g.w * 4) as usize]);
        }
    }
    // the transparent pixels keep no colour (load_sprite spreads the outline's colours under them)
    for p in sheet.px.chunks_exact_mut(4).filter(|p| p[3] == 0) {
        p[..3].copy_from_slice(&[0, 0, 0]);
    }
    sheet
}

/// The guide layer of a sheet at `scale`: the frame grid in blue, each frame's ground point as a red cross.
pub fn guide(g: &Grid, scale: u32) -> Rgba {
    let (w, h) = g.sheet_size(scale);
    let mut img = Rgba::new(w, h);
    let mut put = |x: i64, y: i64, c: [u8; 4]| {
        if x >= 0 && y >= 0 && x < w as i64 && y < h as i64 {
            let o = ((y as u32 * w + x as u32) * 4) as usize;
            img.px[o..o + 4].copy_from_slice(&c);
        }
    };
    let (fw, fh) = ((g.w * scale) as i64, (g.h * scale) as i64);
    const LINE: [u8; 4] = [40, 120, 255, 200];
    const CROSS: [u8; 4] = [255, 30, 30, 255];
    for v in 0..g.views as i64 {
        for f in 0..g.frames as i64 {
            let (x0, y0) = (f * fw, v * fh);
            for x in 0..fw {
                put(x0 + x, y0, LINE);
                put(x0 + x, y0 + fh - 1, LINE);
            }
            for y in 0..fh {
                put(x0, y0 + y, LINE);
                put(x0 + fw - 1, y0 + y, LINE);
            }
            // the middle of the ground point's pixel
            let (cx, cy) = (x0 + g.ax as i64 * scale as i64 + scale as i64 / 2, y0 + g.ay as i64 * scale as i64 + scale as i64 / 2);
            let arm = 3 * scale as i64;
            for d in -arm..=arm {
                put(cx + d, cy, CROSS);
                put(cx, cy + d, CROSS);
            }
        }
    }
    img
}

fn write(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("cannot make {}: {e}", d.display()))?;
    }
    std::fs::write(path, data).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn write_png(path: &Path, img: &Rgba, reference: bool) -> Result<(), String> {
    let png = sg_core::png::encode_png(img).ok_or("empty picture")?;
    let png = if reference { with_text(png, MARK, "EA's art for personal reference only: never put it in a pack") } else { png };
    write(path, &png)
}

fn stem(rel: &str) -> &str {
    &rel[..rel.rfind('.').unwrap_or(rel.len())]
}

/// `sgtool hd-kit <game> <out> [--scale N] [--only DIR]...`. Returns a summary line.
pub fn hd_kit(game: &Path, out: &Path, scale: u32, only: &[String]) -> Result<String, String> {
    if !sg_core::packs::SCALES.contains(&scale) {
        return Err(format!("--scale {scale}: use 1, 2 or 4"));
    }
    if !game.is_dir() {
        return Err(format!("{} is not a folder", game.display()));
    }
    let only: Vec<String> = only.iter().map(|o| o.to_lowercase().replace('\\', "/").trim_matches('/').to_string()).collect();
    let mut files: Vec<(String, PathBuf)> = sg_core::fsutil::walk(game)
        .into_iter()
        .filter_map(|p| {
            let rel =
                p.strip_prefix(game).ok()?.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
            Some((rel, p))
        })
        .filter(|(rel, _)| {
            let low = rel.to_lowercase();
            [".pcx", ".tga", ".bmp", ".flc"].iter().any(|e| low.ends_with(e))
                && (only.is_empty() || only.iter().any(|o| low == *o || low.starts_with(&format!("{o}/"))))
        })
        .collect();
    files.sort();
    let mut csv = String::from("file,original,kind,width,height,frame_width,frame_height,frames,views,anchor_x,anchor_y\n");
    let mut dirs = BTreeSet::new();
    let (mut pictures, mut sprites) = (0, 0);
    for (rel, path) in &files {
        let Some(item) = original(path, rel) else { continue };
        let pack = format!("gfx/{}.png", stem(rel));
        let reference = out.join("reference").join(format!("{}.png", stem(rel)));
        match &item {
            Item::Picture { w, h } => {
                let d = std::fs::read(path).map_err(|e| format!("{rel}: {e}"))?;
                let low = rel.to_lowercase();
                let img = if low.ends_with(".tga") {
                    decode_tga(&d)
                } else if low.ends_with(".bmp") {
                    decode_bmp(&d)
                } else {
                    decode_pcx(&d)
                };
                let Ok(img) = img else { continue };
                write_png(&reference, &upscale_nearest(&img, scale), true)?;
                let kind = if low.starts_with("data/textures/") { "texture" } else { "picture" };
                let _ = writeln!(csv, "{pack},{rel},{kind},{},{},,,,,,", w * scale, h * scale);
                pictures += 1;
            }
            Item::Sprite(g) => {
                let shadow = rel.to_lowercase().ends_with("shadow.flc");
                let s = sg_core::sprites::load_sprite(path, shadow, None).map_err(|e| format!("{rel}: {e}"))?;
                write_png(&reference, &upscale_nearest(&sprite_sheet(&s), scale), true)?;
                write_png(&out.join("reference").join(format!("{}.guide.png", stem(rel))), &guide(g, scale), false)?;
                let (w, h) = g.sheet_size(scale);
                let kind = if shadow { "shadow" } else { "sprite" };
                let _ = writeln!(
                    csv,
                    "{pack},{rel},{kind},{w},{h},{},{},{},{},{},{}",
                    g.w * scale,
                    g.h * scale,
                    g.frames,
                    g.views,
                    g.ax * scale as i32 + scale as i32 / 2,
                    g.ay * scale as i32 + scale as i32 / 2
                );
                sprites += 1;
            }
        }
        if let Some((d, _)) = rel.rsplit_once('/') {
            dirs.insert(d.to_string());
        }
    }
    if pictures + sprites == 0 {
        return Err(format!("no pictures or sprites found in {} (is it the SimGolf folder?)", game.display()));
    }
    write(&out.join("CHECKLIST.csv"), csv.as_bytes())?;
    write(&out.join("README.md"), kit_readme(scale).as_bytes())?;
    write(&out.join("reference").join("DO-NOT-SHARE.txt"), DO_NOT_SHARE.as_bytes())?;
    let t = out.join("template");
    write(&t.join("pack.toml"), template_toml(scale).as_bytes())?;
    write(&t.join("README.md"), template_readme(scale).as_bytes())?;
    write(&t.join("LICENSE"), LICENSE_PLACEHOLDER.as_bytes())?;
    for d in &dirs {
        let dir = t.join("gfx").join(d);
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    }
    Ok(format!("{pictures} pictures and {sprites} sprites at {scale}x -> {} (start with template/ and CHECKLIST.csv)", out.display()))
}

const DO_NOT_SHARE: &str = "The pictures in this folder are Electronic Arts' art from your own copy of SimGolf, enlarged for\n\
reference. They are for your eyes only while you draw: never upload them, never put them (or anything traced or painted\n\
over them) in a pack, and delete them when you are done. `sgtool pack-check` refuses a pack that contains one.\n";

const LICENSE_PLACEHOLDER: &str = "REPLACE THIS FILE with the licence of your art.\n\n\
Packs in the community index (crspytopgn/opensgolf-content) must be under a free licence that lets anyone share them,\n\
for example CC-BY-4.0, CC-BY-SA-4.0 or CC0-1.0. Paste its full text here and put its SPDX identifier in pack.toml\n\
(`license = \"CC-BY-SA-4.0\"`).\n";

fn template_toml(scale: u32) -> String {
    format!(
        "[pack]\n\
         id = \"my-art\"                    # unique: lower case letters, digits and dashes\n\
         name = \"My art\"\n\
         version = \"0.1.0\"\n\
         kind = \"mod\"\n\
         authors = [\"Your Name\"]\n\
         license = \"CC-BY-SA-4.0\"          # the SPDX id of the licence in LICENSE\n\
         description = \"Hand-drawn trees and flags.\"\n\
         scale = {scale}                       # your pictures are {scale} times the original's size\n"
    )
}

fn rules(scale: u32) -> String {
    format!(
        "## The rules\n\n\
         * Put each picture under `gfx/` at the path of the original, as a PNG: `gfx/Interface/BuildPanel.png` replaces\n  \
         `Interface/BuildPanel.pcx`. Names are not case-sensitive. You may redraw only some files: the game uses the original\n  \
         (or the HD upscale) for the rest.\n\
         * PNG, 8 bits per channel, RGBA, not interlaced. Transparency is the alpha channel (no magenta key needed).\n\
         * Every picture is exactly {scale} times the original's size (CHECKLIST.csv lists them). The game draws it at the\n  \
         original's size on screen, so layouts and click areas stay the same; it is just sharper.\n\
         * A sprite is one sheet: one row per view (direction), one column per frame, every frame {scale} times the\n  \
         original's frame, all the same size. Keep the ground point where the guide's red cross is: that pixel stands on\n  \
         the ground. `<name>.json` next to the sheet may change the layout (see docs/MODDING.md).\n\
         * Shadows are separate sheets (`...Shadow.png`): black, with the alpha as the shadow's strength (about 50%).\n\
         * Colour variants: draw the sprite in the colours of the original. The game recolours each pixel by the change of\n  \
         the nearest original colour (tree and flower variants, golfers' clothes). `<name>.mask.png` limits that to the\n  \
         pixels it covers; for golfers its colours say what each pixel is: red shirt, green trousers, blue skin, yellow\n  \
         hat, magenta hair, cyan hands and arms. Or draw a variant yourself as `<name>~<palette file>.png`, for example\n  \
         `TreeMapleLarge~PalRedMaple.png`.\n\
         * Not in a high-resolution pack: the pictures under Bodies/ and Heads/ (recoloured from their palette indices) and\n  \
         the palette files (`*Pal*.pcx`).\n\n\
         ## Testing\n\n\
         Copy the pack folder into `<save folder>/packs/`, start the game, and switch it on in System Functions >\n\
         Preferences > Game files > Content packs. Run `sgtool pack-check <pack folder>` first: it checks sizes, sheets,\n\
         names and the licence.\n"
    )
}

fn kit_readme(scale: u32) -> String {
    format!(
        "# OpenSGolf starter kit for your own art ({scale}x)\n\n\
         **The pictures in `reference/` are Electronic Arts' art from your copy of SimGolf, enlarged with no smoothing.\n\
         They are for your personal reference only: never upload them, never put them, or anything traced or painted\n\
         over them, in a pack, and delete them when you are done.** Draw your own art from scratch; the references only\n\
         show the size, the framing and where the ground point is.\n\n\
         * `reference/<path>.png`: the original at {scale}x (sprites as sheets: a row per view, a column per frame).\n\
         * `reference/<path>.guide.png`: a separate guide layer for each sheet: the frame grid in blue and each frame's\n  \
         ground point as a red cross. Put it on a layer above your drawing and hide it before you export.\n\
         * `CHECKLIST.csv`: every file, with the PNG's size, the frame size, frames, views and the ground point (in the\n  \
         sheet's pixels, inside each frame).\n\
         * `template/`: your pack. Fill in pack.toml and LICENSE, and put your PNGs in its `gfx/` folders.\n\n\
         {}\n\
         ## Publishing\n\n\
         Choose a free licence for your art (CC-BY-4.0, CC-BY-SA-4.0, CC0-1.0...), zip the pack folder and open a pull\n\
         request to https://github.com/crspytopgn/opensgolf-content (see its README). Players then find it in\n\
         Preferences > Game files > Content packs > Find more.\n",
        rules(scale)
    )
}

fn template_readme(scale: u32) -> String {
    format!(
        "# My art (an OpenSGolf content pack)\n\n\
         Original art drawn for OpenSGolf at {scale}x the original game's size. Describe your pack here: what it changes,\n\
         who drew it, and the licence (see LICENSE).\n\n{}",
        rules(scale)
    )
}

// ---- pack-check -------------------------------------------------------------------------------------------------------------

/// What `pack-check` found.
#[derive(Default, Debug)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub checked: usize,
}

fn list(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = sg_core::fsutil::walk(dir)
        .into_iter()
        .filter_map(|p| {
            Some(p.strip_prefix(dir).ok()?.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"))
        })
        .collect();
    v.sort();
    v
}

/// Checks a pack folder: pack.toml, the licence, and every picture and sheet under gfx/ against the original's size (from
/// `game` when given, else the free set's layout manifest).
pub fn pack_check(dir: &Path, game: Option<&Path>) -> Report {
    let mut r = Report::default();
    let toml = match std::fs::read_to_string(dir.join("pack.toml")) {
        Ok(t) => t,
        Err(_) => {
            r.errors.push(format!("no pack.toml in {}: every pack needs one (see docs/MODDING.md)", dir.display()));
            return r;
        }
    };
    let m = match sg_core::packs::Manifest::parse(&toml) {
        Ok(m) => m,
        Err(e) => {
            r.errors.push(e);
            return r;
        }
    };
    let scale = m.scale;
    // the licence
    if m.license.trim().is_empty() {
        r.errors.push("pack.toml has no `license`: put the SPDX id of your licence there, e.g. license = \"CC-BY-SA-4.0\"".into());
    }
    let files = list(dir);
    let licence = files.iter().find(|f| {
        let low = f.to_lowercase();
        ["license", "license.txt", "license.md", "licence", "licence.txt", "licence.md", "copying"].contains(&low.as_str())
    });
    match licence.map(|f| std::fs::read_to_string(dir.join(f)).unwrap_or_default()) {
        None => r.errors.push("no LICENSE file: add the full text of your art's licence as LICENSE".into()),
        Some(t) if t.contains("REPLACE THIS FILE") || t.trim().is_empty() => {
            r.errors.push("LICENSE is still the kit's placeholder: replace it with the text of your licence".into())
        }
        _ => {}
    }
    if m.kind == sg_core::packs::Kind::Base && scale > 1 {
        r.errors.push("a base set cannot have scale > 1 (the engine reads its sizes from it): make it a mod".into());
    }
    // the art
    let lower: BTreeSet<String> = files.iter().map(|f| f.to_lowercase()).collect();
    for f in &files {
        let low = f.to_lowercase();
        let Some(rel) = f.get(4..).filter(|_| low.starts_with("gfx/")) else {
            if low.ends_with(".png") && std::fs::read(dir.join(f)).is_ok_and(|d| is_reference(&d)) {
                r.errors.push(format!("{f}: a reference picture of EA's art from the starter kit: it must not be in the pack"));
            }
            continue;
        };
        r.checked += 1;
        let rlow = rel.to_lowercase();
        if rlow.ends_with(".guide.png") {
            r.errors.push(format!("{f}: a guide layer from the starter kit: leave it out of the pack"));
            continue;
        }
        if rlow.ends_with(".json") {
            let base = format!("gfx/{}.png", stem(rel)).to_lowercase();
            if !lower.contains(&base) {
                r.warnings.push(format!("{f}: there is no sheet {base} for it"));
            }
            continue;
        }
        if !rlow.ends_with(".png") {
            if scale > 1 && [".pcx", ".tga", ".bmp", ".flc"].iter().any(|e| rlow.ends_with(e)) {
                r.errors.push(format!(
                    "{f}: a pack with scale = {scale} takes its pictures and sprites as PNG only (this file would be ignored)"
                ));
            }
            continue;
        }
        let Ok(data) = std::fs::read(dir.join(f)) else {
            r.errors.push(format!("{f}: cannot read it"));
            continue;
        };
        if is_reference(&data) {
            r.errors.push(format!("{f}: a reference picture of EA's art from the starter kit: it must not be in the pack"));
            continue;
        }
        let Some(img) = sg_core::png::decode_png(&data) else {
            r.errors.push(format!("{f}: not a PNG the game reads: save it as PNG, 8 bits per channel, RGBA, not interlaced"));
            continue;
        };
        if data.get(25) != Some(&6) {
            r.warnings.push(format!("{f}: not RGBA (the game reads it, but RGBA with an alpha channel is recommended)"));
        }
        // a mask or a hand-drawn variant goes with its sheet
        let s = stem(rel);
        let (sheet_stem, extra) = if let Some(b) = s.strip_suffix(".mask") {
            (b.to_string(), Some("mask"))
        } else if let Some((b, _)) = s.split_once('~') {
            (b.to_string(), Some("variant"))
        } else {
            (s.to_string(), None)
        };
        if fixed_folder(rel) && scale > 1 {
            r.errors.push(format!(
                "{f}: pictures under Bodies/ and Heads/ are recoloured from their palette indices and cannot be in a high-resolution pack"
            ));
            continue;
        }
        if extra.is_some() && !lower.contains(&format!("gfx/{sheet_stem}.png").to_lowercase()) {
            r.errors.push(format!("{f}: there is no sheet gfx/{sheet_stem}.png for it"));
            continue;
        }
        let Some((orig, item)) = original_for(&sheet_stem, game) else {
            r.warnings.push(format!(
                "{f}: no original picture or sprite is called {sheet_stem} (check the folder and the spelling; the game will not use it)"
            ));
            continue;
        };
        let json = if let Item::Sprite(_) = item {
            match std::fs::read_to_string(dir.join(format!("gfx/{sheet_stem}.json"))) {
                Ok(t) => match SheetJson::parse(&t) {
                    Ok(j) => Some(j),
                    Err(e) => {
                        r.errors.push(format!("gfx/{sheet_stem}.json: {e}"));
                        None
                    }
                },
                Err(_) => None,
            }
        } else {
            None
        };
        match item {
            Item::Picture { w, h } => {
                if extra.is_some() {
                    r.warnings.push(format!("{f}: masks and variants are for sprite sheets; {orig} is a picture"));
                } else if (img.w, img.h) != (w * scale, h * scale) {
                    r.errors.push(format!(
                        "{f} is {}x{}: it replaces {orig} ({w}x{h}), so at {scale}x it must be {}x{}",
                        img.w,
                        img.h,
                        w * scale,
                        h * scale
                    ));
                }
            }
            Item::Sprite(g) => {
                let g = json.as_ref().map_or(g, |j| j.apply(&g));
                let want = g.sheet_size(scale);
                if (img.w, img.h) != want {
                    r.errors.push(format!(
                        "{f} is {}x{}: {orig} has {} views of {} frames of {}x{}, so at {scale}x the sheet must be {}x{} ({} frames across, {} views down, each {}x{})",
                        img.w,
                        img.h,
                        g.views,
                        g.frames,
                        g.w,
                        g.h,
                        want.0,
                        want.1,
                        g.frames,
                        g.views,
                        g.w * scale,
                        g.h * scale
                    ));
                }
                if let (Some("variant"), Some((_, pal))) = (extra, s.split_once('~')) {
                    let dir_of = sheet_stem.rsplit_once('/').map_or("", |(d, _)| d);
                    let pal_rel = if dir_of.is_empty() { format!("{pal}.pcx") } else { format!("{dir_of}/{pal}.pcx") };
                    if sg_core::freeset::reference(&pal_rel) != Some(Reference::Palette) {
                        r.warnings.push(format!("{f}: there is no palette file {pal_rel} for this variant"));
                    }
                }
            }
        }
    }
    if r.checked == 0 {
        r.warnings.push("no files under gfx/: put your pictures there (gfx/<path of the original>.png)".into());
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sgtool-kit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn put(dir: &Path, rel: &str, data: &[u8]) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, data).unwrap();
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        sg_core::png::encode_png(&Rgba::new(w, h)).unwrap()
    }

    const TOML: &str = "[pack]\nid = \"t\"\nname = \"T\"\nversion = \"1.0.0\"\nkind = \"mod\"\nlicense = \"CC0-1.0\"\nscale = 4\n";

    #[test]
    fn a_good_pack_passes() {
        let d = tmp("good");
        put(&d, "pack.toml", TOML.as_bytes());
        put(&d, "LICENSE", b"CC0 1.0 Universal");
        // WillowTree: 4 views of 14 frames of 85 x 78 (freeset manifest)
        put(&d, "gfx/Flics/Trees/WillowTree.png", &png(85 * 14 * 4, 78 * 4 * 4));
        put(&d, "gfx/Flics/Trees/WillowTree.mask.png", &png(85 * 14 * 4, 78 * 4 * 4));
        put(&d, "gfx/Flics/Trees/WillowTree~WillowPink.png", &png(85 * 14 * 4, 78 * 4 * 4));
        put(&d, "gfx/BLDG.png", &png(3200, 2400));
        let r = pack_check(&d, None);
        assert!(r.errors.is_empty() && r.warnings.is_empty(), "{r:?}");
        assert_eq!(r.checked, 4);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_bad_pack_is_explained() {
        let d = tmp("bad");
        put(&d, "pack.toml", TOML.replace("license = \"CC0-1.0\"\n", "").as_bytes());
        put(&d, "LICENSE", LICENSE_PLACEHOLDER.as_bytes());
        put(&d, "gfx/BLDG.png", &png(800, 600));
        put(&d, "gfx/Flics/Trees/WillowTree.png", &png(85 * 14 * 4, 78 * 4));
        put(&d, "gfx/Flics/Trees/WilowTree.png", &png(4, 4));
        put(&d, "gfx/Flics/Trees/Other.mask.png", &png(4, 4));
        put(&d, "gfx/Interface/Old.pcx", b"pcx");
        put(&d, "gfx/Flics/Trees/WillowTree.guide.png", &png(4, 4));
        put(&d, "gfx/Interface/Bad.png", b"\x89PNG\r\n\x1a\nnot really");
        put(&d, "gfx/Bodies/MaleSwap01.png", &png(240, 240));
        let reference = with_text(png(4, 4), MARK, "x");
        put(&d, "gfx/Interface/BuildPanel.png", &reference);
        put(&d, "notes/ref.png", &reference);
        let r = pack_check(&d, None);
        let all = r.errors.join("\n");
        for want in [
            "no `license`",
            "placeholder",
            "BLDG.png is 800x600: it replaces BLDG.pcx (800x600), so at 4x it must be 3200x2400",
            "WillowTree.png is 4760x312",
            "4 views of 14 frames",
            "Old.pcx: a pack with scale = 4",
            "guide layer",
            "Bad.png: not a PNG",
            "Bodies/",
            "gfx/Interface/BuildPanel.png: a reference picture",
            "notes/ref.png: a reference picture",
            "Other.mask.png: there is no sheet",
        ] {
            assert!(all.contains(want), "missing {want:?} in\n{all}");
        }
        assert!(r.warnings.iter().any(|w| w.contains("WilowTree")), "{:?}", r.warnings);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_or_broken_manifest() {
        let d = tmp("none");
        assert!(pack_check(&d, None).errors[0].contains("no pack.toml"));
        put(&d, "pack.toml", b"[pack]\nid = \"t\"\nname = \"T\"\nversion = \"1\"\nkind = \"mod\"\nscale = 3\n");
        assert!(pack_check(&d, None).errors[0].contains("scale"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn guide_marks_grid_and_ground_point() {
        let g = Grid { views: 1, frames: 2, w: 10, h: 10, ax: 5, ay: 8 };
        let img = guide(&g, 4);
        assert_eq!((img.w, img.h), (80, 40));
        let at = |x: u32, y: u32| img.px[((y * 80 + x) * 4) as usize..((y * 80 + x) * 4 + 4) as usize].to_vec();
        assert_eq!(at(0, 20)[2], 255, "frame border");
        assert_eq!(at(39, 20)[2], 255, "frame border");
        assert_eq!(at(22, 34), [255, 30, 30, 255], "ground point of frame 0");
        assert_eq!(at(62, 34), [255, 30, 30, 255], "ground point of frame 1");
        assert_eq!(at(10, 10)[3], 0, "clear elsewhere");
    }

    #[test]
    fn reference_mark_round_trip() {
        let p = with_text(png(3, 2), MARK, "x");
        assert!(is_reference(&p));
        assert!(!is_reference(&png(3, 2)));
        assert_eq!(sg_core::png::decode_png(&p).map(|i| (i.w, i.h)), Some((3, 2)), "still a valid PNG");
    }
}
