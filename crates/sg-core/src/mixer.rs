//! Sound mixer. All of the game's sounds are plain PCM .wav files under Sounds/, so there is nothing to decode beyond the WAV
//! reader; clips are converted to 44.1 kHz stereo and mixed in a handful of voices with looping. The mixer has no device
//! dependency, so it can render to a buffer for tests; the game feeds an output device from [`Mixer::mix`].
use crate::assets::decode_wav;
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
    looping: bool,
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
    state: Mutex<State>,
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
        let mut index = BTreeMap::new();
        for p in crate::fsutil::walk(sounds_dir) {
            if let Ok(rel) = p.strip_prefix(sounds_dir) {
                let key = rel.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect::<Vec<_>>().join("/");
                if key.ends_with(".wav") {
                    index.insert(key, p.clone());
                }
            }
        }
        let last_error = if index.is_empty() { format!("no sounds found in {}", sounds_dir.display()) } else { String::new() };
        Mixer { index, state: Mutex::new(State { clips: BTreeMap::new(), voices: Vec::new(), next_id: 1, master: 0.8, last_error }) }
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
    pub fn known(&self, rel: &str) -> bool {
        self.index.contains_key(&rel.to_lowercase())
    }
    /// Relative paths under a folder (lower case), sorted.
    pub fn list(&self, folder_prefix: &str) -> Vec<String> {
        let p = folder_prefix.to_lowercase();
        self.index.keys().filter(|k| k.starts_with(&p)).cloned().collect()
    }
    pub fn last_error(&self) -> String {
        self.state.lock().map(|s| s.last_error.clone()).unwrap_or_default()
    }
    pub fn set_master(&self, v: f32) {
        if let Ok(mut s) = self.state.lock() {
            s.master = v;
        }
    }

    fn load(&self, s: &mut State, key: &str) -> Option<Arc<Vec<i16>>> {
        if let Some(c) = s.clips.get(key) {
            return Some(c.clone());
        }
        let Some(path) = self.index.get(key) else {
            s.last_error = format!("unknown sound {key}");
            return None;
        };
        let clip = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|d| decode_wav(&d))
            .and_then(|w| to_stereo_44k(w.channels, w.bits, w.sample_rate, &w.pcm).ok_or_else(|| "unsupported PCM layout".to_string()));
        match clip {
            Ok(pcm) => {
                let c = Arc::new(pcm);
                s.clips.insert(key.to_string(), c.clone());
                Some(c)
            }
            Err(e) => {
                s.last_error = format!("cannot load {}: {e}", path.display());
                None
            }
        }
    }

    /// Finds a clip by path relative to Sounds/ (case-insensitive, forward slashes) and plays it. Returns a voice id, or -1.
    pub fn play(&self, rel: &str, volume: f32, looping: bool) -> i32 {
        let Ok(mut s) = self.state.lock() else { return -1 };
        let Some(clip) = self.load(&mut s, &rel.to_lowercase()) else { return -1 };
        if clip.is_empty() {
            return -1;
        }
        if s.voices.len() >= MAX_VOICES {
            s.voices.remove(0); // steal the oldest
        }
        let id = s.next_id;
        s.next_id += 1;
        s.voices.push(Voice { id, clip, pos: 0, vol: volume, looping });
        id
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
                let gain = (v.vol * master * 256.0) as i32;
                let mut i = 0;
                while i < acc.len() {
                    if v.pos >= total {
                        if v.looping {
                            v.pos = 0;
                        } else {
                            break;
                        }
                    }
                    let n = (acc.len() - i).min(total - v.pos);
                    for k in 0..n {
                        acc[i + k] += (v.clip[v.pos + k] as i32 * gain) >> 8;
                    }
                    v.pos += n;
                    i += n;
                }
            }
            s.voices.retain(|v| v.looping || v.pos < v.clip.len());
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
    fn mixing_a_missing_file_is_silent() {
        let m = Mixer::new(Path::new("/nonexistent"));
        assert_eq!(m.play("x.wav", 1.0, false), -1);
        let mut buf = [1i16; 8];
        m.mix(&mut buf);
        assert_eq!(buf, [0; 8]);
    }
}
