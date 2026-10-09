//! The optional HD graphics mode: the game's own art upscaled by the player, on their machine, from their own copy of the game
//! (tools/hd_pack, docs/HD.md). Nothing of it ships with the port.
//!
//! Classic is the default and stays exactly as it is: every hook into the art loaders is guarded by [`active`], which is only
//! true after the player chose HD and a pack was found, so with HD off none of this module's loading or drawing code runs.
//!
//! A pack is a folder with a `manifest.json` and PNGs, each exactly `scale` times its source picture. The loaders here create
//! the textures at the higher resolution but report the source's size, so every layout, cut (`Screen::image_part` works in
//! texture coordinates relative to that size) and hit test of the game is unchanged. Whatever the pack does not have is
//! loaded the classic way.
use crate::app::App;
use crate::gfx::Gfx;
use crate::ui::Image;
use miniquad::TextureId;
use sg_core::assets::{decode_pcx, Rgba};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The settings file in the save folder: `{"graphics": "hd"}` or `{"graphics": "classic"}`.
pub const SETTINGS_FILE: &str = "settings.json";

/// What the player sees when they ask for HD and there is no pack.
pub const NO_PACK: &str =
    "No HD art pack found. Make one from your own copy of the game with tools/hd_pack (see docs/HD.md). Staying with Classic graphics.";

/// The App's HD state.
#[derive(Default)]
pub struct HdState {
    /// HD graphics are on (a pack is loaded).
    pub enabled: bool,
    /// Where the pack is looked for: <save folder>/HD unless --hd-pack says otherwise.
    pub pack_dir: PathBuf,
    /// HD frames of the loaded sprites, by sprite index.
    pub sprites: HashMap<usize, HdSprite>,
    /// The anti-aliased frame target.
    pub msaa: Option<MsaaTarget>,
}

struct Entry {
    file: Option<String>,
    dir: Option<String>,
    w: u32,
    h: u32,
    frames: usize,
}

pub struct Pack {
    root: PathBuf,
    scale: u32,
    files: HashMap<String, Entry>,
}

impl Pack {
    /// Reads the pack's manifest.
    pub fn open(root: &Path) -> Result<Pack, String> {
        let data = sg_core::fsutil::read_file(root.join("manifest.json")).ok_or_else(|| format!("{}: no manifest.json", root.display()))?;
        let v: serde_json::Value = serde_json::from_slice(&data).map_err(|e| format!("manifest.json: {e}"))?;
        if v["format"] != "simgolf-hd-pack" {
            return Err("manifest.json is not a SimGolf HD pack".into());
        }
        let scale = v["scale"].as_u64().filter(|s| (2..=8).contains(s)).ok_or("manifest.json: bad scale")? as u32;
        let mut files = HashMap::new();
        if let Some(m) = v["files"].as_object() {
            for (k, e) in m {
                let num = |n: &str| e[n].as_u64().unwrap_or(0);
                files.insert(
                    k.to_lowercase(),
                    Entry {
                        file: e["file"].as_str().map(str::to_string),
                        dir: e["dir"].as_str().map(str::to_string),
                        w: num("w") as u32,
                        h: num("h") as u32,
                        frames: num("frames") as usize,
                    },
                );
            }
        }
        if files.is_empty() {
            return Err("the HD pack is empty".into());
        }
        Ok(Pack { root: root.to_path_buf(), scale, files })
    }

    fn path(&self, rel: &str) -> PathBuf {
        rel.split('/').filter(|c| !c.is_empty()).fold(self.root.clone(), |p, c| p.join(c))
    }

    /// The pack's picture for a source file, checked to be exactly `scale` times the source size.
    fn image(&self, key: &str) -> Option<(Rgba, u32, u32)> {
        let e = self.files.get(key)?;
        let img = decode_png(&sg_core::fsutil::read_file(self.path(e.file.as_deref()?))?)?;
        (img.w == e.w * self.scale && img.h == e.h * self.scale).then_some((img, e.w, e.h))
    }
}

struct State {
    pack: Option<Rc<Pack>>,
    game_dir: PathBuf,
    /// Interface textures made by the loaders, deleted when the art is reloaded for a change of mode.
    tracked: Vec<TextureId>,
}

thread_local! {
    static STATE: RefCell<State> = const { RefCell::new(State { pack: None, game_dir: PathBuf::new(), tracked: Vec::new() }) };
}

/// True when HD art is in use. Every HD hook checks this first.
pub fn active() -> bool {
    STATE.with(|s| s.borrow().pack.is_some())
}

fn pack() -> Option<Rc<Pack>> {
    STATE.with(|s| s.borrow().pack.clone())
}

fn set_pack(p: Option<Pack>, game_dir: &Path) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pack = p.map(Rc::new);
        s.game_dir = game_dir.to_path_buf();
    });
}

/// Remembers a texture made by an interface loader (so a change of mode can free it).
pub fn track(t: TextureId) {
    STATE.with(|s| s.borrow_mut().tracked.push(t));
}

fn take_tracked() -> Vec<TextureId> {
    STATE.with(|s| std::mem::take(&mut s.borrow_mut().tracked))
}

/// The manifest key of a file in the game folder: its path relative to the folder, lower case, with '/'.
fn key_of(path: &Path) -> Option<String> {
    let game = STATE.with(|s| s.borrow().game_dir.clone());
    let rel = path.strip_prefix(&game).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/").trim_start_matches('/').to_lowercase())
}

// ---- the settings file ------------------------------------------------------------------------------------------------------

/// The graphics mode saved in the settings file (false = Classic, also when there is no file).
pub fn saved_choice() -> bool {
    sg_core::fsutil::read_file(crate::app::save_dir().join(SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .map(|v| v["graphics"] == "hd")
        .unwrap_or(false)
}

fn save_choice(hd: bool) {
    let path = crate::app::save_dir().join(SETTINGS_FILE);
    let mut v = sg_core::fsutil::read_file(&path)
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    v["graphics"] = serde_json::Value::from(if hd { "hd" } else { "classic" });
    if let Ok(s) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(&path, s.as_bytes());
    }
}

impl App {
    /// Turns HD on or off before any art is loaded (start up). Returns an error message when HD was asked for and no pack
    /// could be opened (the game then stays Classic).
    pub fn init_hd(&mut self, on: bool) -> Option<String> {
        if !on {
            return None;
        }
        match Pack::open(&self.hd.pack_dir) {
            Ok(p) => {
                println!("HD: pack {} ({}x, {} pictures)", self.hd.pack_dir.display(), p.scale, p.files.len());
                set_pack(Some(p), &self.game_dir);
                self.hd.enabled = true;
                None
            }
            Err(e) => {
                eprintln!("HD: {e}");
                Some(NO_PACK.to_string())
            }
        }
    }

    /// The player's choice from the Preferences: switches the mode, saves it, and reloads the course's textures and sprites.
    /// The interface art is reloaded by the caller (main.rs, load_art). Returns true when the mode changed.
    pub fn switch_hd(&mut self, g: &mut Gfx, on: bool) -> bool {
        if on == self.hd.enabled {
            return false;
        }
        if on {
            if let Some(msg) = self.init_hd(true) {
                self.show_toast(&msg);
                return false;
            }
        } else {
            set_pack(None, &self.game_dir);
            self.hd.enabled = false;
        }
        save_choice(on);
        for t in take_tracked() {
            g.ctx.delete_texture(t);
        }
        // terrain: drop the textures and rebuild the meshes, which load them again
        for t in self.textures.drain().filter_map(|(_, t)| t) {
            g.ctx.delete_texture(t);
        }
        self.rebuild_batches(g);
        // sprites: kept in place (props, clips and queues hold their indices); only their textures and HD frames change
        self.hd.sprites.clear();
        let keys: Vec<(String, usize)> = self.sprite_index.iter().filter_map(|(k, v)| v.map(|i| (k.clone(), i))).collect();
        for (key, i) in keys {
            for t in self.sprites[i].tex.iter_mut().filter_map(Option::take) {
                g.ctx.delete_texture(t);
            }
            if on && !self.sprites[i].s.shadow {
                if let Some((rel, pal)) = key.split_once('|').filter(|(r, _)| !r.starts_with("holemark")) {
                    let path = self.game_path(&format!("Flics/{rel}"));
                    let pal = (!pal.is_empty()).then(|| self.game_path(&format!("Flics/{pal}")));
                    if let Some(h) = sprite(&path, pal.as_deref(), &self.sprites[i].s) {
                        self.hd.sprites.insert(i, h);
                    }
                }
            }
        }
        for (_, t) in self.red_tex.drain() {
            g.ctx.delete_texture(t);
        }
        true
    }
}

// ---- anti-aliasing (HD only) --------------------------------------------------------------------------------------------------

/// MSAA samples of the HD frame.
const SAMPLES: i32 = 4;

/// The multisampled frame HD draws into, resolved into a plain texture that is then copied to the window.
pub struct MsaaTarget {
    size: (u32, u32),
    pass: miniquad::RenderPass,
    color: TextureId,
    depth: TextureId,
    resolved: TextureId,
}

/// A multisampled pass of w x h resolving into `resolve`, plus its two textures to delete after use; None in Classic or
/// when the renderer cannot resolve.
pub fn msaa_pass(g: &mut Gfx, resolve: TextureId, w: u32, h: u32) -> Option<(miniquad::RenderPass, [TextureId; 2])> {
    use miniquad::{TextureFormat, TextureParams};
    if !active() || !g.ctx.info().features.resolve_attachments {
        return None;
    }
    let p = |format| TextureParams { width: w, height: h, format, sample_count: SAMPLES, ..Default::default() };
    let color = g.ctx.new_render_texture(p(TextureFormat::RGBA8));
    let depth = g.ctx.new_render_texture(p(TextureFormat::Depth));
    let pass = g.ctx.new_render_pass_mrt(&[color], Some(&[resolve]), Some(depth));
    Some((pass, [color, depth]))
}

impl App {
    /// In HD, the pass to draw the frame into (multisampled, window sized); None in Classic, which draws straight to the
    /// window as it always has.
    pub fn hd_frame_pass(&mut self, g: &mut Gfx) -> Option<miniquad::RenderPass> {
        if !active() {
            if let Some(t) = self.hd.msaa.take() {
                free_target(g, t);
            }
            return None;
        }
        let size = (self.draw_w as u32, self.draw_h as u32);
        if let Some(t) = self.hd.msaa.take() {
            if t.size == size {
                let pass = t.pass;
                self.hd.msaa = Some(t);
                return Some(pass);
            }
            free_target(g, t);
        }
        let resolved = g.ctx.new_render_texture(miniquad::TextureParams {
            width: size.0,
            height: size.1,
            format: miniquad::TextureFormat::RGBA8,
            ..Default::default()
        });
        let Some((pass, [color, depth])) = msaa_pass(g, resolved, size.0, size.1) else {
            g.ctx.delete_texture(resolved);
            return None;
        };
        self.hd.msaa = Some(MsaaTarget { size, pass, color, depth, resolved });
        Some(pass)
    }

    /// Copies the resolved HD frame to the window, pixel for pixel.
    pub fn hd_present(&mut self, g: &mut Gfx) {
        use crate::gfx::{Mat4, Mode, Uniforms, Vert};
        let Some(t) = &self.hd.msaa else { return };
        let (w, h) = (t.size.0 as f32, t.size.1 as f32);
        let u = Uniforms::flat(&Mat4::ortho(0.0, w, h, 0.0, -1.0, 1.0), &Mat4::identity());
        g.ctx.begin_default_pass(miniquad::PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        // render textures have their first row at the bottom
        let q = [
            Vert::new(0.0, 0.0, 0.0, 0.0, 1.0),
            Vert::new(w, 0.0, 0.0, 1.0, 1.0),
            Vert::new(w, h, 0.0, 1.0, 0.0),
            Vert::new(0.0, h, 0.0, 0.0, 0.0),
        ];
        g.quad(Mode::Solid, Some(t.resolved), &u, q);
        g.flush();
        g.ctx.end_render_pass();
    }
}

fn free_target(g: &mut Gfx, t: MsaaTarget) {
    g.ctx.delete_render_pass(t.pass);
    for x in [t.color, t.depth, t.resolved] {
        g.ctx.delete_texture(x);
    }
}

// ---- loaders (only called while HD is active) --------------------------------------------------------------------------------

fn finish(g: &mut Gfx, img: &Rgba, w: u32, h: u32) -> Image {
    let tex = g.texture_hd(img);
    track(tex);
    Image { tex: Some(tex), w: w as f32, h: h as f32 }
}

/// ui::load_pcx from the pack. The pack's transparency is the port's magenta key plus the file's own key colour; a load
/// without any key gets the picture fully opaque, as the classic load does.
pub fn load_pcx(g: &mut Gfx, path: &Path, magenta_key: bool, key_rgb: Option<u32>) -> Option<Image> {
    let (mut img, w, h) = pack()?.image(&key_of(path)?)?;
    if !magenta_key && key_rgb.is_none() {
        img.px.chunks_exact_mut(4).for_each(|p| p[3] = 255);
    }
    Some(finish(g, &img, w, h))
}

/// ui::load_pcx_alpha from the pack: the picture's own key, then the upscaled alpha sheet (white opaque, black clear).
pub fn load_pcx_alpha(g: &mut Gfx, path: &Path, alpha: &Path) -> Option<Image> {
    let p = pack()?;
    let (mut img, w, h) = p.image(&key_of(path)?)?;
    if let Some((a, aw, ah)) = key_of(alpha).and_then(|k| p.image(&k)) {
        if aw == w && ah == h {
            for (c, m) in img.px.chunks_exact_mut(4).zip(a.px.chunks_exact(4)) {
                c[3] = c[3].min(m[0].max(m[1]).max(m[2]));
            }
        }
    }
    Some(finish(g, &img, w, h))
}

/// ui::split_overlay from the pack: the pieces are found on the classic picture, and each HD pixel is kept when a pixel of
/// a kept piece is at or next to its place in the classic picture.
pub fn split_overlay(g: &mut Gfx, path: &Path, rects: &[(f32, f32, f32, f32)]) -> Option<Vec<Image>> {
    let p = pack()?;
    let (img, w, h) = p.image(&key_of(path)?)?;
    let mut low = decode_pcx(&sg_core::fsutil::read_file(path)?).ok()?;
    if low.w != w || low.h != h {
        return None;
    }
    for q in low.px.chunks_exact_mut(4) {
        q[3] = if q[0] == 255 && q[1] == 0 && q[2] == 255 { 0 } else { 255 };
    }
    let label = label_pieces(&low);
    let (wu, hu, s) = (w as usize, h as usize, p.scale as usize);
    let mut out = Vec::new();
    for keep in pieces_in_rects(&label, wu, hu, rects) {
        let near: Vec<bool> = (0..wu * hu)
            .map(|i| {
                let (x, y) = ((i % wu) as i32, (i / wu) as i32);
                (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let (nx, ny) = (x + dx, y + dy);
                        nx >= 0 && ny >= 0 && nx < wu as i32 && ny < hu as i32 && keep[label[ny as usize * wu + nx as usize] as usize]
                    })
                })
            })
            .collect();
        let mut part = img.clone();
        let pw = wu * s;
        for (i, q) in part.px.chunks_exact_mut(4).enumerate() {
            let (x, y) = (i % pw / s, i / pw / s);
            if !near[y * wu + x] {
                q[3] = 0;
            }
        }
        out.push(finish(g, &part, w, h));
    }
    Some(out)
}

/// The 8-connected runs of opaque pixels, numbered from 1 (0 = transparent), as ui::split_overlay finds them.
fn label_pieces(img: &Rgba) -> Vec<u32> {
    let (w, h) = (img.w as usize, img.h as usize);
    let mut label = vec![0u32; w * h];
    let mut next = 0u32;
    let mut stack = Vec::new();
    for start in 0..w * h {
        if img.px[start * 4 + 3] == 0 || label[start] != 0 {
            continue;
        }
        next += 1;
        label[start] = next;
        stack.push(start);
        while let Some(i) = stack.pop() {
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if label[j] == 0 && img.px[j * 4 + 3] != 0 {
                        label[j] = next;
                        stack.push(j);
                    }
                }
            }
        }
    }
    label
}

/// Per rectangle, which pieces have a pixel inside it (indexed by label).
fn pieces_in_rects(label: &[u32], w: usize, h: usize, rects: &[(f32, f32, f32, f32)]) -> Vec<Vec<bool>> {
    let n = label.iter().copied().max().unwrap_or(0) as usize;
    rects
        .iter()
        .map(|&(rx, ry, rw, rh)| {
            let mut keep = vec![false; n + 1];
            for y in (ry.max(0.0) as usize)..((ry + rh) as usize).min(h) {
                for x in (rx.max(0.0) as usize)..((rx + rw) as usize).min(w) {
                    keep[label[y * w + x] as usize] = true;
                }
            }
            keep[0] = false;
            keep
        })
        .collect()
}

/// A terrain texture (BMP or TGA under Data/Textures) from the pack, mipmapped like the classic one.
pub fn texture(g: &mut Gfx, path: &Path) -> Option<TextureId> {
    let (img, _, _) = pack()?.image(&key_of(path)?)?;
    Some(g.texture_hd(&img))
}

// ---- sprites ------------------------------------------------------------------------------------------------------------------

/// HD frames of a sprite: the pack's PNGs, decoded when the frame's texture is first needed.
pub struct HdSprite {
    files: Vec<PathBuf>,
    w: u32,
    h: u32,
    scale: u32,
    /// For a sprite drawn in another palette than its file's (colour variants of trees, buildings...): per frame, the colour
    /// change of each source pixel (variant palette minus the file's palette), spread smoothly over the HD pixels.
    delta: Option<Vec<Vec<[i16; 3]>>>,
}

/// The pack's frames for a sprite loaded from `flc` (in the palette of `pal` when given), or None when the pack has no
/// matching frames (the classic frames are drawn then).
pub fn sprite(flc: &Path, pal: Option<&Path>, s: &sg_core::sprites::Sprite) -> Option<HdSprite> {
    let p = pack()?;
    let e = p.files.get(&key_of(flc)?)?;
    let dir = e.dir.as_deref()?;
    if e.w != s.w || e.h != s.h || e.frames != s.frames.len() || e.frames == 0 {
        return None;
    }
    let files = (0..e.frames).map(|i| p.path(&format!("{dir}/{i:04}.png"))).collect();
    let delta = match pal {
        Some(pal) => Some(palette_delta(flc, pal)?),
        None => None,
    };
    Some(HdSprite { files, w: e.w, h: e.h, scale: p.scale, delta })
}

fn palette_delta(flc: &Path, pal: &Path) -> Option<Vec<Vec<[i16; 3]>>> {
    let f = sg_core::flc::decode_flc(&sg_core::fsutil::read_file(flc)?).ok()?;
    let over = sg_core::fsutil::read_file(pal).and_then(|d| sg_core::assets::read_pcx_palette(&d))?;
    let (w, h) = (f.w as usize, f.h as usize);
    Some(
        f.frames
            .iter()
            .map(|fr| {
                let mut d: Vec<Option<[i16; 3]>> = fr
                    .idx
                    .iter()
                    .map(|&c| {
                        let c = c as usize;
                        (c != 255).then(|| [0, 1, 2].map(|k| over[c * 3 + k] as i16 - fr.pal[c * 3 + k] as i16))
                    })
                    .collect();
                // transparent pixels take a neighbour's change, so the smoothing at the outline mixes no zero change in
                let snapshot = d.clone();
                for (i, v) in d.iter_mut().enumerate() {
                    if v.is_none() {
                        let (x, y) = ((i % w) as i32, (i / w) as i32);
                        *v = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (1, -1), (-1, 1)].iter().find_map(|&(dx, dy)| {
                            let (nx, ny) = (x + dx, y + dy);
                            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                                return None;
                            }
                            snapshot[ny as usize * w + nx as usize]
                        });
                    }
                }
                d.into_iter().map(|v| v.unwrap_or([0; 3])).collect()
            })
            .collect(),
    )
}

impl HdSprite {
    /// Frame `i` at the pack's resolution.
    pub fn frame(&self, i: usize) -> Option<Rgba> {
        let mut img = decode_png(&sg_core::fsutil::read_file(self.files.get(i)?)?)?;
        if img.w != self.w * self.scale || img.h != self.h * self.scale {
            return None;
        }
        if let Some(d) = self.delta.as_ref().and_then(|d| d.get(i)) {
            let (w, h, s) = (self.w as usize, self.h as usize, self.scale as f32);
            let at = |x: usize, y: usize| d[y.min(h - 1) * w + x.min(w - 1)];
            let iw = img.w as usize;
            for (j, q) in img.px.chunks_exact_mut(4).enumerate() {
                let fx = ((j % iw) as f32 + 0.5) / s - 0.5;
                let fy = ((j / iw) as f32 + 0.5) / s - 0.5;
                let (x0, y0) = (fx.floor().max(0.0) as usize, fy.floor().max(0.0) as usize);
                let (tx, ty) = ((fx - x0 as f32).clamp(0.0, 1.0), (fy - y0 as f32).clamp(0.0, 1.0));
                let (a, b, c, e) = (at(x0, y0), at(x0 + 1, y0), at(x0, y0 + 1), at(x0 + 1, y0 + 1));
                for k in 0..3 {
                    let top = a[k] as f32 * (1.0 - tx) + b[k] as f32 * tx;
                    let bot = c[k] as f32 * (1.0 - tx) + e[k] as f32 * tx;
                    q[k] = (q[k] as f32 + top * (1.0 - ty) + bot * ty).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        Some(img)
    }
}

// ---- PNG decoding -------------------------------------------------------------------------------------------------------------

/// Decodes an 8-bit, non-interlaced PNG (greyscale, RGB, palette, grey + alpha or RGBA) to RGBA. Enough for the packs, which
/// tools/hd_pack writes as RGBA.
pub fn decode_png(d: &[u8]) -> Option<Rgba> {
    if d.len() < 8 || d[..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return None;
    }
    let be = |o: usize| u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let (mut w, mut h, mut ct) = (0usize, 0usize, 0u8);
    let mut pal: Vec<u8> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
    let mut idat = Vec::new();
    let mut p = 8;
    while p + 8 <= d.len() {
        let len = be(p) as usize;
        let ty = &d[p + 4..p + 8];
        let body = d.get(p + 8..p + 8 + len)?;
        match ty {
            b"IHDR" => {
                if len < 13 || body[8] != 8 || body[12] != 0 {
                    return None; // only 8 bits per channel, not interlaced
                }
                w = u32::from_be_bytes(body[0..4].try_into().ok()?) as usize;
                h = u32::from_be_bytes(body[4..8].try_into().ok()?) as usize;
                ct = body[9];
            }
            b"PLTE" => pal = body.to_vec(),
            b"tRNS" => trns = body.to_vec(),
            b"IDAT" => idat.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        p += 12 + len;
    }
    let ch = match ct {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => return None,
    };
    if w == 0 || h == 0 || w > 16384 || h > 16384 {
        return None;
    }
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&idat).ok()?;
    let stride = w * ch;
    if raw.len() < (stride + 1) * h {
        return None;
    }
    let mut cur = vec![0u8; stride];
    let mut prev = vec![0u8; stride];
    let mut out = Rgba::new(w as u32, h as u32);
    for y in 0..h {
        let line = &raw[y * (stride + 1)..(y + 1) * (stride + 1)];
        let f = line[0];
        for i in 0..stride {
            let x = line[i + 1];
            let a = if i >= ch { cur[i - ch] } else { 0 };
            let b = prev[i];
            let c = if i >= ch { prev[i - ch] } else { 0 };
            cur[i] = match f {
                0 => x,
                1 => x.wrapping_add(a),
                2 => x.wrapping_add(b),
                3 => x.wrapping_add(((a as u16 + b as u16) / 2) as u8),
                4 => {
                    let pp = a as i16 + b as i16 - c as i16;
                    let (pa, pb, pc) = ((pp - a as i16).abs(), (pp - b as i16).abs(), (pp - c as i16).abs());
                    x.wrapping_add(if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    })
                }
                _ => return None,
            };
        }
        let row = &mut out.px[y * w * 4..(y + 1) * w * 4];
        for (x, o) in row.chunks_exact_mut(4).enumerate() {
            let s = &cur[x * ch..x * ch + ch];
            let px = match ct {
                0 => [s[0], s[0], s[0], 255],
                2 => [s[0], s[1], s[2], 255],
                3 => {
                    let i = s[0] as usize;
                    let c = pal.get(i * 3..i * 3 + 3)?;
                    [c[0], c[1], c[2], trns.get(i).copied().unwrap_or(255)]
                }
                4 => [s[0], s[0], s[0], s[1]],
                _ => [s[0], s[1], s[2], s[3]],
            };
            o.copy_from_slice(&px);
        }
        std::mem::swap(&mut cur, &mut prev);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trip() {
        let mut img = Rgba::new(5, 3);
        for (i, v) in img.px.iter_mut().enumerate() {
            *v = (i * 37 % 251) as u8;
        }
        let png = sg_core::png::encode_png(&img).unwrap();
        let back = decode_png(&png).unwrap();
        assert_eq!((back.w, back.h), (5, 3));
        assert_eq!(back.px, img.px);
    }

    #[test]
    fn png_filters() {
        // a 2x2 RGB image, row 0 with the Sub filter, row 1 with Paeth
        let raw = [1u8, 10, 20, 30, 5, 5, 5, 4, 1, 2, 3, 4, 5, 6];
        let z = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut chunk = |ty: &[u8], body: &[u8]| {
            png.extend_from_slice(&(body.len() as u32).to_be_bytes());
            png.extend_from_slice(ty);
            png.extend_from_slice(body);
            png.extend_from_slice(&[0; 4]); // the decoder does not check CRCs
        };
        chunk(b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 2, 8, 2, 0, 0, 0]);
        chunk(b"IDAT", &z);
        chunk(b"IEND", &[]);
        let img = decode_png(&png).unwrap();
        assert_eq!(&img.px[0..8], &[10, 20, 30, 255, 15, 25, 35, 255]);
        // Paeth: first pixel predicts from above (b), second from the left/above/upper-left
        assert_eq!(&img.px[8..11], &[11, 22, 33]);
    }
}
