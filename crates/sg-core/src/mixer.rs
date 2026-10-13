//! Sound mixer. All of the game's sounds are plain PCM .wav files under Sounds/, so there is nothing to decode beyond the WAV
//! reader (a content pack may replace a .wav with an OGG Vorbis file, which `assets::decode_sound` decodes); clips are
//! converted to 44.1 kHz stereo and mixed in a handful of voices with looping. The mixer has no device dependency, so it
//! can render to a buffer for tests; the game feeds an output device from [`Mixer::mix`].
use crate::assets::decode_sound;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const RATE: u32 = 44100;
const MAX_VOICES: usize = 32;

struct Voice {
    id: i32,
    clip: Arc<Vec<i16>>,
    pos: usize,
    vol: f32,
    /// -1 left .. 1 right.
    pan: f32,
    /// Samples (interleaved) of silence before the clip starts.
    delay: usize,
    /// Volume lost per sample while fading out (0: not fading); the voice ends at zero.
    fade: f32,
    looping: bool,
}

/// The index key of a path under Sounds/: lower case, forward slashes, and underscores read as spaces (the exe asks for
/// "Golf sfx/..." and "Tropical Music/..."; copies of the game whose folder names had their spaces turned into underscores
/// still match).
fn key_of(rel: &str) -> String {
    rel.to_lowercase().replace('\\', "/").replace('_', " ")
}

struct State {
    clips: BTreeMap<String, Arc<Vec<i16>>>,
    voices: Vec<Voice>,
    next_id: i32,
    master: f32,
    last_error: String,
}

pub struct Mixer {
    /// lower-case relative path (forward slashes) -> real path
    index: BTreeMap<String, PathBuf>,
    /// The free base set's sounds (crate::freeset), by the same keys -> path in that set: played for a clip of the
    /// player's copy that cannot be decoded, or for every clip when the copy has no sounds at all ([`Mixer::with_fallback`]).
    free: BTreeMap<String, String>,
    /// True when the copy had no sounds at all, so every clip comes from `free`.
    all_free: bool,
    state: Mutex<State>,
}

/// The index key of a sound file found in a folder, or None for a file that is not a sound. A copy whose sounds were
/// converted to OGG Vorbis (`Applause.ogg`) is read as the WAVs the game asks for, as content packs are (crate::vfs).
fn sound_key(rel: &Path) -> Option<(String, bool)> {
    let key = key_of(&rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"));
    // macOS keeps resource forks in "._" files next to the real ones on drives that are not its own
    if key.rsplit('/').next().is_some_and(|n| n.starts_with("._")) {
        return None;
    }
    if key.ends_with(".wav") {
        Some((key, true))
    } else {
        key.strip_suffix(".ogg").map(|stem| (format!("{stem}.wav"), false))
    }
}

/// Converts PCM (8-bit unsigned or 16-bit signed, mono or stereo, any rate) to 44.1 kHz interleaved stereo i16, with linear
/// interpolation.
pub fn to_stereo_44k(channels: u16, bits: u16, rate: u32, pcm: &[u8]) -> Option<Vec<i16>> {
    if channels == 0 || channels > 2 || rate == 0 || (bits != 8 && bits != 16) {
        return None;
    }
    let ch = channels as usize;
    let bps = bits as usize / 8;
    let frames = pcm.len() / (ch * bps);
    let sample = |f: usize, c: usize| -> f32 {
        let o = (f * ch + c.min(ch - 1)) * bps;
        if bits == 8 {
            (pcm[o] as f32 - 128.0) * 256.0
        } else {
            i16::from_le_bytes([pcm[o], pcm[o + 1]]) as f32
        }
    };
    if frames == 0 {
        return Some(Vec::new());
    }
    let out_frames = (frames as u64 * RATE as u64 / rate as u64) as usize;
    let mut out = Vec::with_capacity(out_frames * 2);
    let step = rate as f64 / RATE as f64;
    for i in 0..out_frames {
        let src = i as f64 * step;
        let f0 = (src as usize).min(frames - 1);
        let f1 = (f0 + 1).min(frames - 1);
        let k = (src - f0 as f64) as f32;
        for c in 0..2 {
            let v = sample(f0, c) * (1.0 - k) + sample(f1, c) * k;
            out.push(v.clamp(-32768.0, 32767.0) as i16);
        }
    }
    Some(out)
}

impl Mixer {
    /// Indexes every .wav under `sounds_dir` (recursively).
    pub fn new(sounds_dir: &Path) -> Mixer {
        let mut m = Mixer {
            index: BTreeMap::new(),
            free: BTreeMap::new(),
            all_free: false,
            state: Mutex::new(State { clips: BTreeMap::new(), voices: Vec::new(), next_id: 1, master: 0.8, last_error: String::new() }),
        };
        m.add_dir("", sounds_dir);
        if m.index.is_empty() {
            m.set_error(format!("no sounds found in {}", sounds_dir.display()));
        }
        m
    }

    fn set_error(&self, e: String) {
        if let Ok(mut s) = self.state.lock() {
            s.last_error = e;
        }
    }

    /// Also indexes every sound under `dir` (.wav, or .ogg standing for one), keyed under `prefix/` (the exe reads its Sims
    /// voice clips from SimsFX). A WAV wins over an OGG of the same name.
    pub fn add_dir(&mut self, prefix: &str, dir: &Path) {
        let prefix = if prefix.is_empty() { String::new() } else { format!("{}/", key_of(prefix)) };
        for p in crate::fsutil::walk(dir) {
            let Some((key, wav)) = p.strip_prefix(dir).ok().and_then(sound_key) else { continue };
            let key = format!("{prefix}{key}");
            if wav {
                self.index.insert(key, p.clone());
            } else {
                self.index.entry(key).or_insert(p.clone());
            }
        }
    }

    /// Lets the free base set's sounds (crate::freeset) stand in where the player's copy fails: for a clip that cannot be
    /// decoded, and for every clip when the copy holds no sounds at all. The original's 1:1 sound stays as it is wherever
    /// its own file plays (a clip the original lacks stays silent).
    pub fn with_fallback(&mut self) {
        for f in crate::freeset::files() {
            let low = f.to_lowercase();
            let key = if let Some(r) = low.strip_prefix("sfx/sounds/") {
                key_of(r)
            } else if let Some(r) = low.strip_prefix("sfx/simsfx/") {
                format!("simsfx/{}", key_of(r))
            } else {
                continue;
            };
            self.free.insert(key, f);
        }
        self.all_free = self.index.is_empty() && !self.free.is_empty();
    }

    /// True when every clip comes from the free set (the player's copy has no sounds).
    pub fn all_free(&self) -> bool {
        self.all_free
    }

    /// Decodes a few of the game's most used clips and says whether the player's own sounds play: Err names what failed.
    /// Used once at start to tell the player when their copy's sounds cannot be played (an unreadable coding, no files).
    pub fn check(&self) -> Result<(), String> {
        if self.all_free || self.index.is_empty() {
            return Err(self.last_error());
        }
        let Ok(mut s) = self.state.lock() else { return Ok(()) };
        let probe: Vec<String> = ["golfambience122.wav", "applause.wav", "golf sfx/drive with ball.wav"]
            .iter()
            .map(|k| k.to_string())
            .filter(|k| self.index.contains_key(k))
            .chain(self.index.keys().take(3).cloned())
            .collect();
        let mut err = String::new();
        for k in probe {
            match self.decode_own(&k) {
                Ok(pcm) => {
                    s.clips.insert(k, Arc::new(pcm));
                    return Ok(());
                }
                Err(e) => err = e,
            }
        }
        Err(err)
    }

    /// The player's own file of a clip, decoded to 44.1 kHz stereo.
    fn decode_own(&self, key: &str) -> Result<Vec<i16>, String> {
        let path = self.index.get(key).ok_or_else(|| format!("unknown sound {key}"))?;
        crate::fsutil::read_file(path)
            .ok_or_else(|| "cannot read".to_string())
            .and_then(|d| decode_sound(&d))
            .and_then(|w| to_stereo_44k(w.channels, w.bits, w.sample_rate, &w.pcm).ok_or_else(|| "unsupported PCM layout".to_string()))
            .map_err(|e| format!("cannot load {}: {e}", path.display()))
    }

    /// The free set's clip for a key, decoded.
    fn decode_free(&self, key: &str) -> Option<Vec<i16>> {
        let w = decode_sound(&crate::freeset::read(self.free.get(key)?)?).ok()?;
        to_stereo_44k(w.channels, w.bits, w.sample_rate, &w.pcm)
    }

    /// The clips the game can play: the player's, or the free set's when they have none.
    fn keys(&self) -> Box<dyn Iterator<Item = &String> + '_> {
        if self.all_free {
            Box::new(self.free.keys())
        } else {
            Box::new(self.index.keys())
        }
    }
    pub fn is_empty(&self) -> bool {
        self.keys().next().is_none()
    }
    pub fn known(&self, rel: &str) -> bool {
        let k = key_of(rel);
        self.index.contains_key(&k) || (self.all_free && self.free.contains_key(&k))
    }
    /// Relative paths under a folder (lower case), sorted.
    pub fn list(&self, folder_prefix: &str) -> Vec<String> {
        let p = key_of(folder_prefix);
        self.keys().filter(|k| k.starts_with(&p)).cloned().collect()
    }
    pub fn last_error(&self) -> String {
        self.state.lock().map(|s| s.last_error.clone()).unwrap_or_default()
    }
    pub fn set_master(&self, v: f32) {
        if let Ok(mut s) = self.state.lock() {
            s.master = v;
        }
    }

    /// A clip, decoded the first time it is asked for. The decoding (a tenth of a second for the original's longest music)
    /// runs without the lock, so the output device's thread goes on mixing the voices already playing meanwhile instead of
    /// waiting for it (a gap in the sound).
    fn load(&self, key: &str) -> Option<Arc<Vec<i16>>> {
        if let Some(c) = self.state.lock().ok()?.clips.get(key) {
            return Some(c.clone());
        }
        let pcm = if self.all_free {
            self.decode_free(key)?
        } else if !self.index.contains_key(key) {
            self.set_error(format!("unknown sound {key}"));
            return None;
        } else {
            match self.decode_own(key) {
                Ok(pcm) => pcm,
                Err(e) => {
                    // said once per clip: the clip is remembered (as the free set's, or as silence) and not read again
                    eprintln!("sound: {e}");
                    self.set_error(e);
                    self.decode_free(key).unwrap_or_default()
                }
            }
        };
        let mut s = self.state.lock().ok()?;
        Some(s.clips.entry(key.to_string()).or_insert_with(|| Arc::new(pcm)).clone())
    }

    /// Finds a clip by path relative to Sounds/ (case-insensitive, forward slashes) and plays it. Returns a voice id, or -1.
    pub fn play(&self, rel: &str, volume: f32, looping: bool) -> i32 {
        self.play_at(rel, volume, 0.0, 0, looping)
    }

    /// Plays a clip panned (-1 left .. 1 right) and starting `delay_ms` from now. Returns a voice id, or -1.
    pub fn play_at(&self, rel: &str, volume: f32, pan: f32, delay_ms: u32, looping: bool) -> i32 {
        let Some(clip) = self.load(&key_of(rel)) else { return -1 };
        let Ok(mut s) = self.state.lock() else { return -1 };
        if clip.is_empty() {
            return -1;
        }
        if s.voices.len() >= MAX_VOICES {
            // steal the oldest voice that is not looping (music and ambience stay)
            let i = s.voices.iter().position(|v| !v.looping).unwrap_or(0);
            s.voices.remove(i);
        }
        let id = s.next_id;
        s.next_id += 1;
        let delay = (delay_ms as usize * RATE as usize / 1000) * 2;
        s.voices.push(Voice { id, clip, pos: 0, vol: volume, pan: pan.clamp(-1.0, 1.0), delay, fade: 0.0, looping });
        id
    }

    /// Fades a voice out over `ms` milliseconds, then stops it.
    pub fn fade_out(&self, voice: i32, ms: u32) {
        if let Ok(mut s) = self.state.lock() {
            for v in s.voices.iter_mut().filter(|v| v.id == voice) {
                let n = (ms.max(1) as f32 * RATE as f32 / 1000.0) * 2.0;
                v.fade = v.vol / n;
            }
        }
    }

    /// Whether a voice is still sounding (or waiting to start).
    pub fn playing(&self, voice: i32) -> bool {
        self.state.lock().map(|s| s.voices.iter().any(|v| v.id == voice)).unwrap_or(false)
    }

    pub fn stop(&self, voice: i32) {
        if let Ok(mut s) = self.state.lock() {
            s.voices.retain(|v| v.id != voice);
        }
    }
    pub fn stop_all(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.voices.clear();
        }
    }
    pub fn set_voice_volume(&self, voice: i32, volume: f32) {
        if let Ok(mut s) = self.state.lock() {
            s.voices.iter_mut().filter(|v| v.id == voice).for_each(|v| v.vol = volume);
        }
    }

    /// Interleaved stereo S16 at 44100 Hz; thread safe. `out.len()` is 2 x frames.
    pub fn mix(&self, out: &mut [i16]) {
        let mut acc = vec![0i32; out.len()];
        if let Ok(mut s) = self.state.lock() {
            let master = s.master;
            for v in s.voices.iter_mut() {
                let total = v.clip.len();
                let mut i = v.delay.min(acc.len()) & !1;
                v.delay -= i;
                let (pl, pr) = ((1.0 - v.pan).min(1.0), (1.0 + v.pan).min(1.0));
                while i < acc.len() {
                    if v.pos >= total {
                        if v.looping {
                            v.pos = 0;
                        } else {
                            break;
                        }
                    }
                    if v.fade > 0.0 && v.vol <= 0.0 {
                        break;
                    }
                    let n = (acc.len() - i).min(total - v.pos) & !1;
                    if n == 0 {
                        break;
                    }
                    // gains per channel, stepped every 64 frames while fading
                    let mut k = 0;
                    while k < n {
                        let m = (n - k).min(128);
                        let gl = (v.vol * master * pl * 256.0) as i32;
                        let gr = (v.vol * master * pr * 256.0) as i32;
                        for j in (0..m).step_by(2) {
                            acc[i + k + j] += (v.clip[v.pos + k + j] as i32 * gl) >> 8;
                            acc[i + k + j + 1] += (v.clip[v.pos + k + j + 1] as i32 * gr) >> 8;
                        }
                        if v.fade > 0.0 {
                            v.vol = (v.vol - v.fade * m as f32).max(0.0);
                        }
                        k += m;
                    }
                    v.pos += n;
                    i += n;
                }
            }
            s.voices.retain(|v| (v.looping || v.pos < v.clip.len() || v.delay > 0) && !(v.fade > 0.0 && v.vol <= 0.0));
        }
        for (o, a) in out.iter_mut().zip(acc) {
            *o = a.clamp(-32768, 32767) as i16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_22k_doubles_in_length() {
        let pcm: Vec<u8> = (0..100i16).flat_map(|v| (v * 100).to_le_bytes()).collect();
        let out = to_stereo_44k(1, 16, 22050, &pcm).unwrap();
        assert_eq!(out.len(), 200 * 2);
        assert_eq!(out[0], out[1]); // mono goes to both channels
        assert_eq!(out[2 * 2], 100); // frame 2 at 44.1k is source frame 1
    }

    #[test]
    fn underscores_read_as_spaces() {
        assert_eq!(key_of("Golf sfx/Ball In Hole.wav"), key_of("Golf_Sfx/Ball_In_Hole.wav"));
        assert_eq!(key_of("music/Tropical Music/x.wav"), "music/tropical music/x.wav");
    }

    #[test]
    fn mixing_a_missing_file_is_silent() {
        let m = Mixer::new(Path::new("/nonexistent"));
        assert_eq!(m.play("x.wav", 1.0, false), -1);
        let mut buf = [1i16; 8];
        m.mix(&mut buf);
        assert_eq!(buf, [0; 8]);
    }
}
