//! "Make HD graphics from my SimGolf files..." (Preferences > Graphics; docs/HD.md): the HD pack tools/hd_pack/hd_pack.py
//! makes, made by the game itself from the player's own files, with the built-in xBRZ upscaler (sg_core::xbrz) or, on a
//! desktop that has it, Real-ESRGAN (realesrgan-ncnn-vulkan on PATH or next to the game; never downloaded). The port's own.
//!
//! The pack goes where the game looks for one (<save folder>/HD, or --hd-pack): on threads natively, so the game plays on
//! while it is made; in a browser a few pictures a frame, the files kept by the page with the game's own (web/opensgolf.js).
//! A run that stops (cancelled, the game closed) carries on later: the manifest lists the pictures done, and settings.json's
//! `hd_make` remembers the run until it is complete. When it is, HD graphics are switched on.
//!
//! Test hook: `--make-hd` makes the pack from the --game folder and exits (docs/PLAYING.md).
use crate::app::App;
use crate::gamefiles::{wrap, Action, Notice};
use sg_core::hdgen::{self, Manifest, Prepared, Source};
use sg_core::lang::{tr, trf};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(target_has_atomic = "64")]
use std::sync::atomic::AtomicU64;

/// A 64-bit counter where the CPU has no 64-bit atomics (the 3DS's ARM11): a lock around it.
#[cfg(not(target_has_atomic = "64"))]
#[derive(Default, Debug)]
pub struct AtomicU64(std::sync::Mutex<u64>);
#[cfg(not(target_has_atomic = "64"))]
impl AtomicU64 {
    pub fn load(&self, _: Ordering) -> u64 {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn store(&self, v: u64, _: Ordering) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = v;
    }    pub fn fetch_add(&self, v: u64, _: Ordering) -> u64 {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let old = *g;
        *g = old.wrapping_add(v);
        old
    }
}
use std::sync::{Arc, Mutex};

/// The settings.json key of a run not finished yet: {"scale": n, "method": "..."}.
const SETTINGS_KEY: &str = "hd_make";

/// What the player chose in the boxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// x4 with the AI upscaler.
    Big,
    /// x2 with the AI upscaler.
    Small,
    /// Carry on with the run not finished.
    Resume,
    /// Start again instead of carrying on: the first box.
    Restart,
    /// Stop the run under way.
    Stop,
}

/// How the pictures are upscaled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    Xbrz,
    /// The built-in AI upscaler (sg_core::aiscale): every system, on the processor.
    Ai,
    /// Real-ESRGAN's executable (desktop).
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    Esrgan(PathBuf),
}

/// The Real-ESRGAN model used (as hd_pack.py's default).
const ESRGAN_MODEL: &str = "realesrgan-x4plus";

impl Method {
    /// The manifest's `method` (hd_pack.py's names for Real-ESRGAN, so either tool carries on the other's run).
    pub fn name(&self) -> String {
        match self {
            Method::Xbrz => hdgen::METHOD_XBRZ.to_string(),
            Method::Ai => hdgen::METHOD_AI.to_string(),
            Method::Esrgan(_) => format!("realesrgan:{ESRGAN_MODEL}"),
        }
    }
}

/// What to make.
#[derive(Clone, Debug)]
pub struct Spec {
    pub game: PathBuf,
    pub out: PathBuf,
    pub scale: u32,
    pub method: Method,
    /// Keep the pictures an earlier run of the same scale and method made.
    pub resume: bool,
    /// Only sources whose path contains this (a test hook).
    pub only: Option<String>,
    /// The course's theme (app::THEMES index): its pictures are made first, then those of no theme, then the other themes'.
    pub first: Option<usize>,
    /// Worker threads (natively).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub threads: usize,
}

/// Microseconds a thread takes per source pixel with xBRZ, PNG writing included, for the estimate: a fixed part (decoding,
/// the corner pre-pass) plus a part per upscaled pixel. Measured on the whole game (about 245 million source pixels): 33 s
/// at x2 and 50 s at x4 on four threads of a 2020s laptop core.
const XBRZ_US_PER_PX: (f64, f64) = (0.44, 0.0235);
/// PNG bytes of the AI upscaler's pack for one of xBRZ's (measured on the whole game: 614 MB at x2 for 392 MB).
const AI_BYTES: f64 = 1.57;
/// The share of the pictures' pixels the AI upscaler works out (the opaque ones and the clear ones near them), measured on
/// the whole game.
const AI_WORKED_SHARE: f64 = 0.49;

/// Source pixels a second one thread of this device upscales with the AI upscaler: timed once, on a small picture (about a
/// tenth of a second on a laptop), so the estimates are this device's own.
pub fn ai_speed() -> f64 {
    static SPEED: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *SPEED.get_or_init(|| {
        let (w, h) = (24, 24);
        let rgb: Vec<u8> = (0..w * h * 3).map(|i| (i * 37 % 251) as u8).collect();
        // the first run widens the model's weights: not timed
        sg_core::aiscale::upscale4(&rgb[..8 * 8 * 3], 8, 8, None);
        let t = crate::app::now();
        sg_core::aiscale::upscale4(&rgb, w, h, None);
        let s = (crate::app::now() - t).max(1e-3);
        let speed = (w * h) as f64 / s;
        println!("HD: this device upscales {speed:.0} pixels a second a thread with the AI upscaler");
        speed
    })
}

/// PNG bytes per source pixel and unit of scale, for the estimate (measured likewise: 0.40 GB at x2, 0.79 GB at x4).
const BYTES_PER_PX_SCALE: f64 = 0.8;

/// The sources and what the pack will hold.
pub struct Plan {
    pub sources: Vec<Source>,
    /// Source pixels per source (all pictures), from the headers.
    pub px: Vec<u64>,
    pub pictures: u64,
}

impl Plan {
    pub fn new(game: &Path, only: Option<&str>) -> Plan {
        let sources = hdgen::collect(game, only);
        let sizes: Vec<(u64, u64)> =
            sources.iter().map(|s| hdgen::header_size(s).map_or((0, 0), |(w, h, n)| (w as u64 * h as u64 * n as u64, n as u64))).collect();
        Plan { px: sizes.iter().map(|s| s.0).collect(), pictures: sizes.iter().map(|s| s.1).sum(), sources }
    }

    fn total_px(&self) -> u64 {
        self.px.iter().sum()
    }

    /// Bytes on disk at `scale`.
    pub fn bytes(&self, scale: u32) -> f64 {
        self.total_px() as f64 * scale as f64 * BYTES_PER_PX_SCALE
    }

    /// Seconds with xBRZ on `threads` threads (a browser: one, and slower).
    pub fn seconds(&self, scale: u32, threads: usize) -> f64 {
        let slow = if cfg!(target_arch = "wasm32") { 2.0 } else { 1.0 };
        let (fixed, per) = XBRZ_US_PER_PX;
        self.total_px() as f64 * (fixed + per * (scale * scale) as f64) / 1e6 * slow / threads.max(1) as f64
    }

    /// Seconds with the AI upscaler on `threads` threads of this device, from its own speed ([`ai_speed`]): the model's
    /// work on the share of the pictures' pixels it works out, plus xBRZ's for the transparency.
    pub fn ai_seconds(&self, scale: u32, threads: usize) -> f64 {
        self.total_px() as f64 * AI_WORKED_SHARE / ai_speed() / threads.max(1) as f64 + self.seconds(scale, threads)
    }
}

/// How far a run is, shared with its threads.
#[derive(Default)]
pub struct Shared {
    pub total_px: AtomicU64,
    pub done_px: AtomicU64,
    pub pictures: AtomicUsize,
    pub cancel: AtomicBool,
    manifest: Mutex<Manifest>,
    dirty: AtomicBool,
    error: Mutex<Option<String>>,
}

impl Shared {
    fn fail(&self, e: String) {
        self.error.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(e);
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// The share done, 0 to 1.
    pub fn fraction(&self) -> f64 {
        let t = self.total_px.load(Ordering::Relaxed).max(1);
        (self.done_px.load(Ordering::Relaxed) as f64 / t as f64).min(1.0)
    }

    fn save_manifest(&self, out: &Path) {
        if self.dirty.swap(false, Ordering::Relaxed) {
            let m = self.manifest.lock().unwrap_or_else(|e| e.into_inner()).clone();
            m.write(out);
        }
    }
}

/// The run's sources still to do (those an earlier run made are kept when resuming) and its manifest to add to.
fn start_manifest(spec: &Spec, plan: Plan, shared: &Shared) -> Vec<Source> {
    let mut m = Manifest { scale: spec.scale, method: spec.method.name(), ..Default::default() };
    if let Some(old) = Manifest::read(&spec.out).filter(|o| o.scale == spec.scale) {
        // as hd_pack.py: a pack of the same scale is added to; only a run of the same method is carried on
        let same = old.method == m.method && spec.resume;
        m.files = old.files;
        if !same {
            m.files.retain(|_, _| false);
        }
    }
    let mut todo = Vec::new();
    let (mut total, mut done) = (0, 0);
    for (s, px) in plan.sources.into_iter().zip(plan.px) {
        total += px;
        if m.files.contains_key(&s.key) {
            done += px;
        } else {
            todo.push((s, px));
        }
    }
    if let Some(t) = spec.first {
        todo.sort_by_key(|(s, _)| theme_order(&s.rel, t));
    }
    shared.total_px.store(total, Ordering::Relaxed);
    shared.done_px.store(done, Ordering::Relaxed);
    *shared.manifest.lock().unwrap_or_else(|e| e.into_inner()) = m;
    shared.dirty.store(true, Ordering::Relaxed);
    todo.into_iter().map(|(s, _)| s).collect()
}

/// Where a source comes in a run that does the theme `first` first: 0 that theme's, 1 no theme's, 2 another theme's.
fn theme_order(rel: &str, first: usize) -> u8 {
    // the words the game files' folders and names give each theme (app::THEMES' order)
    const KEYS: [&str; 4] = ["park", "link", "des", "trop"];
    let rel = rel.to_lowercase();
    match KEYS.iter().position(|k| rel.contains(k)) {
        Some(k) if k == first => 0,
        Some(_) => 2,
        None => 1,
    }
}

/// Writes a picture of the pack. Err: the file could not be written (a full disk).
fn write_picture(out: &Path, rel: &str, img: &sg_core::assets::Rgba) -> Result<(), String> {
    let path = rel.split('/').fold(out.to_path_buf(), |p, c| p.join(c));
    let png = sg_core::png::encode_png(img).ok_or("empty picture")?;
    if !sg_core::fsutil::write_file(&path, &png) {
        return Err(trf("STR_HD_MAKE_WRITE_FAILED", &[("FILE", &path.display())]));
    }
    Ok(())
}

/// A source's pictures upscaled and written, then its entry added to the manifest.
fn finish_source(spec: &Spec, shared: &Shared, p: Prepared, pics: Vec<sg_core::assets::Rgba>, px: u64) -> Result<(), String> {
    for (pic, img) in p.pictures.iter().zip(&pics) {
        write_picture(&spec.out, &pic.out, img)?;
    }
    shared.manifest.lock().unwrap_or_else(|e| e.into_inner()).files.insert(p.key, p.entry);
    shared.dirty.store(true, Ordering::Relaxed);
    shared.done_px.fetch_add(px, Ordering::Relaxed);
    shared.pictures.fetch_add(pics.len(), Ordering::Relaxed);
    Ok(())
}

fn source_px(p: &Prepared) -> u64 {
    p.pictures.iter().map(|q| q.img.w as u64 * q.img.h as u64).sum()
}

// ---- natively: threads -----------------------------------------------------------------------------------------------------

/// Makes the pack on `spec.threads` threads (returns when done or cancelled; Err: what stopped it).
#[cfg(not(target_arch = "wasm32"))]
pub fn run(spec: &Spec, plan: Plan, shared: &Shared) -> Result<(), String> {
    let todo = start_manifest(spec, plan, shared);
    shared.save_manifest(&spec.out);
    let next = AtomicUsize::new(0);
    // Real-ESRGAN runs on the graphics card: one runner, in batches
    let threads = if matches!(spec.method, Method::Esrgan(_)) { 1 } else { spec.threads.max(1) };
    std::thread::scope(|sc| {
        let workers: Vec<_> = (0..threads).map(|_| sc.spawn(|| worker(spec, shared, &todo, &next))).collect();
        let mut last = std::time::Instant::now();
        while !workers.iter().all(|w| w.is_finished()) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if last.elapsed().as_secs_f64() >= 2.0 {
                shared.save_manifest(&spec.out);
                last = std::time::Instant::now();
            }
        }
    });
    shared.save_manifest(&spec.out);
    match shared.error.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn worker(spec: &Spec, shared: &Shared, todo: &[Source], next: &AtomicUsize) {
    let batch_size = if matches!(spec.method, Method::Esrgan(_)) { 64 } else { 1 };
    let mut batch: Vec<Prepared> = Vec::new();
    loop {
        let i = next.fetch_add(1, Ordering::Relaxed);
        let more = i < todo.len() && !shared.cancel.load(Ordering::Relaxed);
        if more {
            match hdgen::prepare(&todo[i]) {
                Ok(Some(p)) => batch.push(p),
                Ok(None) => {}
                Err(e) => println!("  skip {}: {e}", todo[i].rel),
            }
        }
        let n: usize = batch.iter().map(|p| p.pictures.len()).sum();
        if (n >= batch_size || !more) && !batch.is_empty() {
            let pics: Vec<Vec<sg_core::assets::Rgba>> = match &spec.method {
                Method::Xbrz => batch.iter().map(|p| p.pictures.iter().map(|q| hdgen::upscale(q, spec.scale)).collect()).collect(),
                Method::Ai => batch.iter().map(|p| p.pictures.iter().map(|q| hdgen::upscale_ai(q, spec.scale)).collect()).collect(),
                #[cfg(not(target_os = "android"))]
                Method::Esrgan(exe) => match esrgan::run(exe, &batch, spec.scale) {
                    Ok(v) => v,
                    Err(e) => return shared.fail(e),
                },
                #[cfg(target_os = "android")]
                Method::Esrgan(_) => return shared.fail("Real-ESRGAN".into()),
            };
            for (p, pics) in batch.drain(..).zip(pics) {
                let px = source_px(&p);
                if let Err(e) = finish_source(spec, shared, p, pics, px) {
                    return shared.fail(e);
                }
            }
        }
        if !more {
            return;
        }
    }
}

/// Real-ESRGAN (desktop): found on PATH or next to the game, run on batches of pictures as hd_pack.py runs it. The colours
/// come from Real-ESRGAN and the transparency from the xBRZ upscale of the same picture.
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub mod esrgan {
    use super::*;
    use sg_core::assets::Rgba;
    use sg_core::hdgen::Picture;

    const NAMES: [&str; 2] = ["realesrgan-ncnn-vulkan", "realesrgan-ncnn-vulkan.exe"];
    /// Context pixels around each picture (hd_pack.py PAD), cropped off again.
    const PAD: u32 = 8;

    /// The executable, if there is one on PATH or next to the game (or in a `realesrgan` folder beside it).
    pub fn find() -> Option<PathBuf> {
        let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
        if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
            dirs.push(d.join("realesrgan"));
            dirs.push(d);
        }
        dirs.iter().flat_map(|d| NAMES.iter().map(move |n| d.join(n))).find(|p| p.is_file())
    }

    /// The picture's colours, the clear pixels filled from their opaque neighbours, opaque, with PAD pixels of edge (or of
    /// itself, a wrapping texture) around.
    fn input(p: &Picture) -> Rgba {
        let fill = hdgen::fill_colours(&p.img);
        let (w, h) = (p.img.w as i64, p.img.h as i64);
        let mut out = Rgba::new(p.img.w + 2 * PAD, p.img.h + 2 * PAD);
        for y in 0..out.h as i64 {
            for x in 0..out.w as i64 {
                let (sx, sy) = (x - PAD as i64, y - PAD as i64);
                let (sx, sy) = if p.wrap { (sx.rem_euclid(w), sy.rem_euclid(h)) } else { (sx.clamp(0, w - 1), sy.clamp(0, h - 1)) };
                let c = fill[(sy * w + sx) as usize];
                let o = ((y * out.w as i64 + x) * 4) as usize;
                out.px[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        out
    }

    /// Halves a picture (2x2 boxes): a x4 model's output for a x2 pack.
    fn halve(img: &Rgba) -> Rgba {
        let mut out = Rgba::new(img.w / 2, img.h / 2);
        let w = img.w as usize;
        for y in 0..out.h as usize {
            for x in 0..out.w as usize {
                for k in 0..4 {
                    let at = |dx: usize, dy: usize| img.px[((2 * y + dy) * w + 2 * x + dx) * 4 + k] as u32;
                    out.px[(y * out.w as usize + x) * 4 + k] = ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
                }
            }
        }
        out
    }

    pub fn run(exe: &Path, batch: &[Prepared], scale: u32) -> Result<Vec<Vec<Rgba>>, String> {
        let tmp = std::env::temp_dir().join(format!("opensgolf-hd-{}", std::process::id()));
        let (src, dst) = (tmp.join("in"), tmp.join("out"));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&src).and_then(|_| std::fs::create_dir_all(&dst)).map_err(|e| e.to_string())?;
        let pics: Vec<&Picture> = batch.iter().flat_map(|p| &p.pictures).collect();
        for (i, p) in pics.iter().enumerate() {
            if !sg_core::png::write_png(src.join(format!("{i:06}.png")), &input(p)) {
                return Err(trf("STR_HD_MAKE_WRITE_FAILED", &[("FILE", &src.display())]));
            }
        }
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("-i").arg(&src).arg("-o").arg(&dst).args(["-n", ESRGAN_MODEL, "-s", "4", "-f", "png"]);
        if let Some(models) = exe.parent().map(|d| d.join("models")).filter(|m| m.is_dir()) {
            cmd.arg("-m").arg(models);
        }
        let r = cmd.output().map_err(|e| format!("{}: {e}", exe.display()))?;
        if !r.status.success() {
            let _ = std::fs::remove_dir_all(&tmp);
            let log = format!("{}{}", String::from_utf8_lossy(&r.stdout), String::from_utf8_lossy(&r.stderr));
            let tail: String = log.chars().skip(log.chars().count().saturating_sub(400)).collect();
            return Err(format!("realesrgan-ncnn-vulkan failed: {tail}"));
        }
        let mut out = Vec::new();
        let mut k = 0;
        for p in batch {
            let mut v = Vec::new();
            for pic in &p.pictures {
                // the transparency (and a fallback) from xBRZ
                let mut img = hdgen::upscale(pic, scale);
                let big = sg_core::fsutil::read_raw(dst.join(format!("{k:06}.png"))).and_then(|d| sg_core::png::decode_png(&d));
                k += 1;
                let (pw, ph) = ((pic.img.w + 2 * PAD) * 4, (pic.img.h + 2 * PAD) * 4);
                if let Some(big) = big.filter(|b| (b.w, b.h) == (pw, ph) && !pic.mask) {
                    let big = if scale == 2 { halve(&big) } else { big };
                    let (bw, o) = (big.w as usize, (PAD * scale) as usize);
                    let w = img.w as usize;
                    for (i, q) in img.px.chunks_exact_mut(4).enumerate() {
                        if q[3] != 0 {
                            let s = ((i / w + o) * bw + i % w + o) * 4;
                            q[..3].copy_from_slice(&big.px[s..s + 3]);
                        }
                    }
                }
                v.push(img);
            }
            out.push(v);
        }
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(out)
    }
}

// ---- in a browser: a few pictures a frame ------------------------------------------------------------------------------------

/// A run made a step at a time on the game's own thread (the browser's way; natively the --make-hd --hd-step test hook).
pub struct Stepper {
    spec: Spec,
    todo: Vec<Source>,
    next: usize,
    cur: Option<(Prepared, Vec<sg_core::assets::Rgba>)>,
    last_save: f64,
}

impl Stepper {
    pub fn new(spec: Spec, plan: Plan, shared: &Shared) -> Stepper {
        let todo = start_manifest(&spec, plan, shared);
        shared.save_manifest(&spec.out);
        Stepper { spec, todo, next: 0, cur: None, last_save: crate::app::now() }
    }

    /// Works until `until` (seconds, crate::app::now) has passed, a picture at least. True when the run is over (done,
    /// cancelled or failed).
    pub fn step(&mut self, shared: &Shared, until: f64) -> bool {
        loop {
            if shared.cancel.load(Ordering::Relaxed) {
                shared.save_manifest(&self.spec.out);
                return true;
            }
            if self.cur.is_none() {
                let Some(s) = self.todo.get(self.next) else {
                    shared.save_manifest(&self.spec.out);
                    return true;
                };
                self.next += 1;
                match hdgen::prepare(s) {
                    Ok(Some(p)) => self.cur = Some((p, Vec::new())),
                    Ok(None) => {}
                    Err(e) => println!("  skip {}: {e}", s.rel),
                }
                continue;
            }
            let Some((p, done)) = self.cur.as_mut() else { continue };
            if let Some(pic) = p.pictures.get(done.len()) {
                done.push(match self.spec.method {
                    Method::Ai => hdgen::upscale_ai(pic, self.spec.scale),
                    _ => hdgen::upscale(pic, self.spec.scale),
                });
            }
            if done.len() == p.pictures.len() {
                let Some((p, pics)) = self.cur.take() else { continue };
                let px = source_px(&p);
                if let Err(e) = finish_source(&self.spec, shared, p, pics, px) {
                    shared.fail(e);
                }
            }
            let t = crate::app::now();
            if t - self.last_save > 5.0 {
                shared.save_manifest(&self.spec.out);
                self.last_save = t;
            }
            if t >= until {
                return false;
            }
        }
    }
}

// ---- the run under way ---------------------------------------------------------------------------------------------------------

enum Runner {
    #[cfg(not(target_arch = "wasm32"))]
    Thread(std::thread::JoinHandle<Result<(), String>>),
    Steps(Box<Stepper>),
}

struct Job {
    shared: Arc<Shared>,
    runner: Runner,
    spec: Spec,
    started: f64,
    /// The progress box is up (closing it lets the game be played; Preferences' line brings it back).
    visible: bool,
    shown: Vec<String>,
}

thread_local! {
    static JOB: RefCell<Option<Job>> = const { RefCell::new(None) };
}

/// The share done of the run under way, if there is one.
pub fn running() -> Option<f64> {
    JOB.with(|j| j.borrow().as_ref().map(|j| j.shared.fraction()))
}

fn settings_path() -> PathBuf {
    crate::app::save_dir().join(crate::hd::SETTINGS_FILE)
}

fn read_settings() -> serde_json::Value {
    sg_core::fsutil::read_file(settings_path())
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The run not finished, from settings.json: (scale, method name).
fn unfinished() -> Option<(u32, String)> {
    let v = read_settings();
    let m = &v[SETTINGS_KEY];
    Some((m["scale"].as_u64()? as u32, m["method"].as_str()?.to_string()))
}

fn set_unfinished(spec: Option<&Spec>) {
    let mut v = read_settings();
    match spec {
        Some(s) => v[SETTINGS_KEY] = serde_json::json!({"scale": s.scale, "method": s.method.name()}),
        None => _ = v.as_object_mut().map(|m| m.remove(SETTINGS_KEY)),
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(settings_path(), t.as_bytes());
    }
}

/// What Preferences > Graphics offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer {
    Make,
    Again,
    Resume,
    /// A run under way, percent done.
    Running(u8),
}

/// Worker threads natively: all cores but one for the game (at least one).
fn threads() -> usize {
    if cfg!(target_arch = "wasm32") {
        return 1;
    }
    std::thread::available_parallelism().map_or(2, |n| n.get()).saturating_sub(1).max(1)
}

fn mb(bytes: f64) -> String {
    if bytes >= 1e9 {
        format!("{:.1} GB", bytes / 1e9)
    } else {
        format!("{} MB", ((bytes / 1e6).ceil() as u64).max(1))
    }
}

fn minutes(s: f64) -> String {
    let m = (s / 60.0).ceil().max(1.0) as u64;
    trf(if m == 1 { "STR_HD_MAKE_MINUTE" } else { "STR_HD_MAKE_MINUTES" }, &[("N", &m)])
}

impl App {
    /// The Preferences > Graphics line: only with the original files in use (not on the free set).
    pub fn hd_make_offer(&self) -> Option<Offer> {
        if let Some(f) = running() {
            return Some(Offer::Running((f * 100.0) as u8));
        }
        // not in a browser yet: the AI upscaler runs there on one thread without the vector unit, hours for the game's art
        if cfg!(target_arch = "wasm32") || self.on_free_set() || !hdgen::is_game_folder(&self.game_dir) {
            return None;
        }
        let pack = sg_core::fsutil::exists(self.hd.pack_dir.join("manifest.json"));
        Some(if pack && unfinished().is_some() {
            Offer::Resume
        } else if pack {
            Offer::Again
        } else {
            Offer::Make
        })
    }

    /// The line chosen: the run's progress box, or the question what to make (with the time and space it takes).
    pub fn hd_make_open(&mut self) {
        if running().is_some() {
            JOB.with(|j| {
                if let Some(j) = j.borrow_mut().as_mut() {
                    j.visible = true;
                    j.shown.clear();
                }
            });
            return self.poll_hd_progress();
        }
        match self.hd_make_offer() {
            Some(Offer::Resume) => {
                let mut lines = wrap(tr("STR_HD_MAKE_ASK_RESUME"), 560.0);
                lines.extend(["STR_HD_MAKE_RESUME", "STR_HD_MAKE_RESTART", "STR_GAME_FILES_NOT_NOW"].map(|k| format!(" {}", tr(k))));
                self.notice = Some(Notice {
                    lines,
                    confirm: Some(Action::HdMake(Choice::Resume)),
                    alt: Some(Action::HdMake(Choice::Restart)),
                    sel: Some(0),
                });
            }
            Some(o) => self.hd_make_ask(o == Offer::Again),
            None => {}
        }
    }

    fn hd_make_ask(&mut self, again: bool) {
        let plan = Plan::new(&self.game_dir, None);
        let n = threads();
        // the AI's pictures carry more detail and compress less than xBRZ's (measured on the whole game: 614 MB at x2)
        let est = |scale: u32| (mb(plan.bytes(scale) * AI_BYTES), minutes(plan.ai_seconds(scale, n)));
        let (b4, t4) = est(4);
        let (b2, t2) = est(2);
        let mut text = String::new();
        if again {
            text += tr("STR_HD_MAKE_AGAIN_NOTE");
            text += " ";
        }
        let browser = cfg!(target_arch = "wasm32");
        text += &trf(
            if browser { "STR_HD_MAKE_ASK_BROWSER" } else { "STR_HD_MAKE_ASK" },
            &[("PICTURES", &plan.pictures), ("SIZE", &if browser { &b2 } else { &b4 }), ("TIME", &if browser { &t2 } else { &t4 })],
        );
        let mut lines = wrap(&text, 560.0);
        let (confirm, alt) = if browser {
            lines.push(format!(" {}", trf("STR_HD_MAKE_X2", &[("SIZE", &b2), ("TIME", &t2)])));
            (Choice::Small, None)
        } else {
            lines.push(format!(" {}", trf("STR_HD_MAKE_BIG", &[("SIZE", &b4), ("TIME", &t4)])));
            lines.push(format!(" {}", trf("STR_HD_MAKE_SMALL", &[("SIZE", &b2), ("TIME", &t2)])));
            (Choice::Big, Some(Choice::Small))
        };
        lines.push(format!(" {}", tr("STR_GAME_FILES_NOT_NOW")));
        self.notice = Some(Notice { lines, confirm: Some(Action::HdMake(confirm)), alt: alt.map(Action::HdMake), sel: Some(0) });
    }

    /// The answer of one of the boxes.
    pub fn hd_make_answer(&mut self, c: Choice) {
        if c == Choice::Stop {
            JOB.with(|j| {
                if let Some(j) = j.borrow().as_ref() {
                    j.shared.cancel.store(true, Ordering::Relaxed);
                }
            });
            return;
        }
        if c == Choice::Restart {
            return self.hd_make_ask(true);
        }
        if running().is_some() {
            return;
        }
        let (scale, method, resume) = match c {
            Choice::Big => (4, Method::Ai, false),
            Choice::Small => (2, Method::Ai, false),
            _ => match unfinished() {
                Some((scale, name)) => {
                    let m = if name.starts_with("realesrgan") {
                        self.esrgan().unwrap_or(Method::Ai)
                    } else if name == hdgen::METHOD_XBRZ {
                        Method::Xbrz
                    } else {
                        Method::Ai
                    };
                    (scale, m, true)
                }
                None => (4, Method::Ai, true),
            },
        };
        if !resume {
            // a new pack in place of the one there (the old pictures are written over)
            sg_core::fsutil::remove_file(self.hd.pack_dir.join("manifest.json"));
        }
        let spec = Spec {
            game: self.game_dir.clone(),
            out: self.hd.pack_dir.clone(),
            scale,
            method,
            resume,
            only: None,
            first: Some(self.theme),
            threads: threads(),
        };
        set_unfinished(Some(&spec));
        println!("HD: making a x{scale} pack with {} into {}", spec.method.name(), spec.out.display());
        start(spec, None);
        self.poll_hd_progress();
    }

    fn esrgan(&self) -> Option<Method> {
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        {
            esrgan::find().map(Method::Esrgan)
        }
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        None
    }

    /// The progress box, while it is up.
    fn poll_hd_progress(&mut self) {
        let Some((frac, pics, elapsed, visible, shown)) = JOB.with(|j| {
            j.borrow().as_ref().map(|j| {
                let f = j.shared.fraction();
                (f, j.shared.pictures.load(Ordering::Relaxed), crate::app::now() - j.started, j.visible, j.shown.clone())
            })
        }) else {
            return;
        };
        if !visible {
            return;
        }
        // the box was closed (Keep playing, Esc, a click outside it): the game plays on, the run goes on
        let ours = self.notice.as_ref().map(|n| n.lines == shown);
        if !shown.is_empty() && ours != Some(true) {
            if ours.is_none() {
                JOB.with(|j| j.borrow_mut().as_mut().map(|j| j.visible = false));
            }
            return;
        }
        let pct = (frac * 100.0) as u32;
        let left =
            if frac > 0.02 && elapsed > 3.0 { minutes(elapsed * (1.0 - frac) / frac) } else { tr("STR_HD_MAKE_LEFT_UNKNOWN").to_string() };
        let mut lines = wrap(&trf("STR_HD_MAKE_PROGRESS", &[("PCT", &pct), ("PICTURES", &pics), ("LEFT", &left)]), 560.0);
        lines.push(format!(" {}", tr("STR_HD_MAKE_STOP")));
        lines.push(format!(" {}", tr("STR_HD_MAKE_HIDE")));
        // first shown with Keep playing lit, so one click or Enter goes back to the game; after that the pointer's choice
        let sel = if shown.is_empty() { Some(1) } else { self.notice.as_ref().and_then(|n| n.sel) };
        self.notice = Some(Notice { lines: lines.clone(), confirm: Some(Action::HdMake(Choice::Stop)), alt: None, sel });
        JOB.with(|j| j.borrow_mut().as_mut().map(|j| j.shown = lines));
    }

    /// At start: a run the player began and did not finish (the game was closed) carries on by itself in the background,
    /// with only the corner note showing (a click on it brings the box back, with Stop).
    pub fn hd_make_carry_on(&mut self) {
        if self.ui_ok && self.hd_make_offer() == Some(Offer::Resume) {
            self.hd_make_answer(Choice::Resume);
            JOB.with(|j| j.borrow_mut().as_mut().map(|j| j.visible = false));
            if self.notice.as_ref().is_some_and(|n| n.confirm == Some(Action::HdMake(Choice::Stop))) {
                self.notice = None;
            }
        }
    }

    /// Whether Preferences > Graphics offers Remove HD graphics: a pack is there and no run is making one.
    pub fn hd_remove_offer(&self) -> bool {
        running().is_none() && sg_core::fsutil::exists(self.hd.pack_dir.join("manifest.json"))
    }

    /// The pack's files and their bytes (0 where the size cannot be read, as in a browser).
    fn pack_files(&self) -> (Vec<PathBuf>, u64) {
        let files = sg_core::fsutil::walk(&self.hd.pack_dir);
        #[cfg(not(target_arch = "wasm32"))]
        let bytes = files.iter().filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum();
        #[cfg(target_arch = "wasm32")]
        let bytes = 0;
        (files, bytes)
    }

    /// Remove HD graphics...: what it does and the space it frees, then OK or Cancel.
    pub fn hd_remove_ask(&mut self) {
        let (_, bytes) = self.pack_files();
        let text =
            if bytes > 0 { trf("STR_HD_REMOVE_ASK", &[("SIZE", &mb(bytes as f64))]) } else { tr("STR_HD_REMOVE_ASK_PLAIN").to_string() };
        let mut lines = wrap(&text, 560.0);
        lines.extend(["STR_HD_REMOVE_OK", "STR_GAME_FILES_NOT_NOW"].map(|k| format!(" {}", tr(k))));
        self.notice = Some(Notice { lines, confirm: Some(Action::HdRemove), alt: None, sel: Some(1) });
    }

    /// Deletes the pack (HD is already off) and any run not finished; says so.
    pub fn hd_remove(&mut self) {
        let (files, _) = self.pack_files();
        for f in &files {
            sg_core::fsutil::remove_file(f);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = std::fs::remove_dir_all(&self.hd.pack_dir);
        set_unfinished(None);
        println!("HD: pack removed ({} files)", files.len());
        self.show_notice(tr("STR_HD_REMOVED"));
    }

    /// The corner note while a run goes on with its box closed: its text and where it is (800 x 600 units), or None.
    fn hd_badge(&self) -> Option<(String, crate::render::Rect)> {
        let (frac, elapsed, visible) =
            JOB.with(|j| j.borrow().as_ref().map(|j| (j.shared.fraction(), crate::app::now() - j.started, j.visible)))?;
        if visible || self.notice.is_some() || !self.ui_ok {
            return None;
        }
        let pct = (frac * 100.0) as u32;
        let left =
            if frac > 0.02 && elapsed > 3.0 { minutes(elapsed * (1.0 - frac) / frac) } else { tr("STR_HD_MAKE_LEFT_UNKNOWN").to_string() };
        let t = trf("STR_HD_MAKE_BADGE", &[("PCT", &pct), ("LEFT", &left)]);
        let w = crate::ui::text_width(&t, 15.0) + 20.0;
        Some((t, crate::render::Rect { x: 400.0 - w / 2.0, y: 8.0, w, h: 22.0 }))
    }

    /// Draws the corner note (over the game, under a box).
    pub fn draw_hd_badge(&mut self, g: &mut crate::gfx::Gfx) {
        let Some((t, r)) = self.hd_badge() else { return };
        let s = crate::ui::Screen::new(self.draw_w, self.draw_h);
        s.fill(g, r.x, r.y, r.w, r.h, crate::ui::rgba(0.1, 0.1, 0.3, 0.85));
        s.text_centered(g, 400.0, r.y + 4.0, &t, 15.0, crate::ui::rgb(1.0, 1.0, 0.8));
        g.flush();
    }

    /// A click on the corner note brings the progress box back. True when it took the click.
    pub fn hd_badge_click(&mut self, vx: f32, vy: f32) -> bool {
        match self.hd_badge() {
            Some((_, r)) if r.has(vx, vy) => {
                self.hd_make_open();
                true
            }
            _ => false,
        }
    }

    /// Each frame: the run's progress (a browser's next pictures), and once it is over its outcome. True when a pack was
    /// completed: the caller switches HD on (and reloads the art).
    pub fn poll_hd_make(&mut self) -> bool {
        let over = JOB.with(|j| {
            let mut j = j.borrow_mut();
            let Some(job) = j.as_mut() else { return false };
            match &mut job.runner {
                #[cfg(not(target_arch = "wasm32"))]
                Runner::Thread(h) => h.is_finished(),
                // a browser: a slice of each frame (more while the box is up and the game waits)
                Runner::Steps(s) => s.step(&job.shared, crate::app::now() + if job.visible { 0.05 } else { 0.012 }),
            }
        });
        if !over {
            self.poll_hd_progress();
            return false;
        }
        let Some(job) = JOB.with(|j| j.borrow_mut().take()) else { return false };
        let result = match job.runner {
            #[cfg(not(target_arch = "wasm32"))]
            Runner::Thread(h) => h.join().unwrap_or_else(|_| Err("panic".into())),
            Runner::Steps(_) => match job.shared.error.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                Some(e) => Err(e),
                None => Ok(()),
            },
        };
        if job.visible && self.notice.as_ref().is_some_and(|n| n.lines == job.shown) {
            self.notice = None;
        }
        let secs = crate::app::now() - job.started;
        match result {
            Err(e) => {
                eprintln!("HD: making the pack failed: {e}");
                self.show_notice(&trf("STR_HD_MAKE_FAILED", &[("ERR", &e)]));
                false
            }
            Ok(()) if job.shared.cancel.load(Ordering::Relaxed) => {
                println!("HD: stopped at {:.0}%", job.shared.fraction() * 100.0);
                self.show_toast(tr("STR_HD_MAKE_STOPPED"));
                false
            }
            Ok(()) => {
                println!(
                    "HD: pack made: {} pictures x{} with {} in {secs:.0} s",
                    job.shared.pictures.load(Ordering::Relaxed),
                    job.spec.scale,
                    job.spec.method.name()
                );
                set_unfinished(None);
                // a box that stays until it is read: what happened, that nothing else is needed, and how to undo it
                self.show_notice(tr("STR_HD_MAKE_DONE"));
                true
            }
        }
    }
}

/// Starts a run (on a thread natively, in steps in a browser or with `steps`).
fn start(spec: Spec, steps: Option<bool>) {
    let shared = Arc::new(Shared::default());
    let plan = Plan::new(&spec.game, spec.only.as_deref());
    let step = steps.unwrap_or(cfg!(target_arch = "wasm32"));
    let runner = if step {
        Runner::Steps(Box::new(Stepper::new(spec.clone(), plan, &shared)))
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (s, sh) = (spec.clone(), shared.clone());
            match std::thread::Builder::new().name("opensgolf-hd".into()).spawn(move || run(&s, plan, &sh)) {
                Ok(h) => Runner::Thread(h),
                Err(e) => {
                    eprintln!("HD: {e}");
                    return;
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        unreachable!()
    };
    let job = Job { shared, runner, spec, started: crate::app::now(), visible: true, shown: Vec::new() };
    JOB.with(|j| *j.borrow_mut() = Some(job));
}

/// The --make-hd test hook: makes the pack and says how long it took. Returns the exit code.
#[cfg(not(target_arch = "wasm32"))]
pub fn cli(spec: Spec, steps: bool) -> i32 {
    if !hdgen::is_game_folder(&spec.game) {
        eprintln!("{}: not the SimGolf game folder (no Flics and Interface)", spec.game.display());
        return 1;
    }
    let t0 = std::time::Instant::now();
    let plan = Plan::new(&spec.game, spec.only.as_deref());
    println!(
        "making a x{} HD pack with {} on {} thread(s) from {} into {}: {} sources, {} pictures (estimate: {}, {:.0} s)",
        spec.scale,
        spec.method.name(),
        if steps { 1 } else { spec.threads },
        spec.game.display(),
        spec.out.display(),
        plan.sources.len(),
        plan.pictures,
        mb(plan.bytes(spec.scale)),
        plan.seconds(spec.scale, if steps { 1 } else { spec.threads })
    );
    if !spec.resume {
        sg_core::fsutil::remove_file(spec.out.join("manifest.json"));
    }
    let shared = Arc::new(Shared::default());
    let result = if steps {
        let mut s = Stepper::new(spec.clone(), plan, &shared);
        while !s.step(&shared, crate::app::now() + 0.016) {}
        match shared.error.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    } else {
        let sh = shared.clone();
        let s = spec.clone();
        let h = std::thread::spawn(move || run(&s, plan, &sh));
        let mut last = 0;
        while !h.is_finished() {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let p = (shared.fraction() * 100.0) as u32;
            if p / 10 != last / 10 {
                println!("  {p}% ({} pictures, {:.0} s)", shared.pictures.load(Ordering::Relaxed), t0.elapsed().as_secs_f64());
                last = p;
            }
        }
        h.join().unwrap_or_else(|_| Err("panic".into()))
    };
    match result {
        Ok(()) => {
            println!(
                "done: {} pictures upscaled x{} with {} into {} ({:.1} s)",
                shared.pictures.load(Ordering::Relaxed),
                spec.scale,
                spec.method.name(),
                spec.out.display(),
                t0.elapsed().as_secs_f64()
            );
            0
        }
        Err(e) => {
            eprintln!("making the HD pack failed: {e}");
            1
        }
    }
}
