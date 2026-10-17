//! Sound output: feeds the system's default output device from the Mixer (44.1 kHz stereo), resampling when the device runs at
//! another rate. Built with the `audio` feature (cpal: CoreAudio on macOS, WASAPI on Windows, ALSA on Linux, oboe/AAudio on
//! Android).
// a build without the `audio` feature (the desktop's silent build, or the Android app when its sound could not be built)
// has no device to feed
#![cfg_attr(all(not(feature = "audio"), not(target_arch = "wasm32")), allow(dead_code))]
use sg_core::mixer::{Mixer, RATE};
use std::sync::Arc;

/// Pulls 44.1 kHz stereo frames from the mixer and resamples them linearly to the device rate.
pub struct Feed {
    mixer: Arc<Mixer>,
    buf: Vec<i16>,
    idx: usize,
    frac: f64,
    step: f64,
}

impl Feed {
    pub fn new(mixer: Arc<Mixer>, device_rate: u32) -> Feed {
        Feed { mixer, buf: Vec::new(), idx: 0, frac: 0.0, step: RATE as f64 / device_rate.max(1) as f64 }
    }

    /// Follows an output that now runs at another rate (a browser's sound context made again after it was closed).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn set_device_rate(&mut self, device_rate: u32) {
        self.step = RATE as f64 / device_rate.max(1) as f64;
    }

    /// Next stereo frame, -1..1.
    pub fn next_frame(&mut self) -> (f32, f32) {
        while self.idx + 1 >= self.buf.len() / 2 {
            let keep = self.buf.split_off((self.idx * 2).min(self.buf.len()));
            self.buf = keep;
            self.idx = 0;
            let mut chunk = vec![0i16; 1024 * 2];
            self.mixer.mix(&mut chunk);
            self.buf.extend_from_slice(&chunk);
        }
        let (a, b) = (self.idx * 2, self.idx * 2 + 2);
        let k = self.frac as f32;
        let l = self.buf[a] as f32 * (1.0 - k) + self.buf[b] as f32 * k;
        let r = self.buf[a + 1] as f32 * (1.0 - k) + self.buf[b + 1] as f32 * k;
        self.frac += self.step;
        while self.frac >= 1.0 {
            self.frac -= 1.0;
            self.idx += 1;
        }
        (l / 32768.0, r / 32768.0)
    }
}

#[cfg(feature = "audio")]
mod device {
    use super::Feed;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use sg_core::mixer::Mixer;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// The stream to the default output device. It follows the system: when the default device changes (headphones or
    /// AirPods connected, another output chosen in the system's settings) or the stream fails (the device went away),
    /// [`AudioOut::keep_alive`] opens the new default device. A stream left on the old device played to a device the
    /// player no longer listens to, or nowhere, with no word.
    pub struct AudioOut {
        stream: cpal::Stream,
        mixer: Arc<Mixer>,
        device: String,
        failed: Arc<AtomicBool>,
        checked: Instant,
    }

    fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
        dev: &cpal::Device,
        cfg: &cpal::StreamConfig,
        mut feed: Feed,
        failed: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, cpal::BuildStreamError> {
        let ch = cfg.channels as usize;
        dev.build_output_stream(
            cfg,
            move |data: &mut [T], _| {
                for frame in data.chunks_mut(ch.max(1)) {
                    let (l, r) = feed.next_frame();
                    for (i, s) in frame.iter_mut().enumerate() {
                        let v = match (ch, i) {
                            (1, _) => (l + r) * 0.5,
                            (_, 0) => l,
                            (_, 1) => r,
                            _ => 0.0,
                        };
                        *s = T::from_sample(v);
                    }
                }
            },
            move |e| {
                super::log(&format!("sound: the output stopped: {e}"));
                failed.store(true, Ordering::Relaxed);
            },
            None,
        )
    }

    /// The default output device's name ("" when there is none).
    fn default_name(host: &cpal::Host) -> String {
        host.default_output_device().and_then(|d| d.name().ok()).unwrap_or_default()
    }

    /// A stream to the default output device, playing. Err says why there is none (for the log).
    fn start(mixer: &Arc<Mixer>, failed: &Arc<AtomicBool>) -> Result<(cpal::Stream, String), String> {
        let host = cpal::default_host();
        let dev = host.default_output_device().ok_or_else(|| format!("no default output device ({:?})", host.id()))?;
        let name = dev.name().unwrap_or_default();
        let sup = dev.default_output_config().map_err(|e| format!("{name}: no output format: {e}"))?;
        let fmt = sup.sample_format();
        let cfg: cpal::StreamConfig = sup.into();
        let feed = Feed::new(mixer.clone(), cfg.sample_rate.0);
        let f = failed.clone();
        let stream = match fmt {
            cpal::SampleFormat::F32 => build::<f32>(&dev, &cfg, feed, f),
            cpal::SampleFormat::I16 => build::<i16>(&dev, &cfg, feed, f),
            cpal::SampleFormat::U16 => build::<u16>(&dev, &cfg, feed, f),
            cpal::SampleFormat::I32 => build::<i32>(&dev, &cfg, feed, f),
            other => return Err(format!("{name}: sample format {other:?} not supported")),
        }
        .map_err(|e| format!("{name}: cannot open the output: {e}"))?;
        stream.play().map_err(|e| format!("{name}: cannot start the output: {e}"))?;
        super::log(&format!("sound: playing on \"{name}\" ({} Hz, {} channels, {fmt:?})", cfg.sample_rate.0, cfg.channels));
        Ok((stream, name))
    }

    pub fn open(mixer: Arc<Mixer>) -> Option<AudioOut> {
        let failed = Arc::new(AtomicBool::new(false));
        match start(&mixer, &failed) {
            Ok((stream, device)) => Some(AudioOut { stream, mixer, device, failed, checked: Instant::now() }),
            Err(e) => {
                super::log(&format!("sound: {e}"));
                None
            }
        }
    }

    impl AudioOut {
        /// Every two seconds: a stream that failed, or a default output device that changed, is opened again on the
        /// current default device.
        pub fn keep_alive(&mut self) {
            if self.checked.elapsed() < Duration::from_secs(2) {
                return;
            }
            self.checked = Instant::now();
            let failed = self.failed.load(Ordering::Relaxed);
            // (Android routes a stream to the device in use by itself: only a failed stream is opened again)
            let now = if cfg!(target_os = "android") { self.device.clone() } else { default_name(&cpal::default_host()) };
            if !failed && (now == self.device || now.is_empty()) {
                return;
            }
            println!(
                "sound: the output device changed (\"{}\" to \"{now}\"{}): opening it",
                self.device,
                if failed { ", the old one failed" } else { "" }
            );
            let flag = Arc::new(AtomicBool::new(false));
            match start(&self.mixer, &flag) {
                Ok((stream, device)) => {
                    self.stream = stream;
                    self.device = device;
                    self.failed = flag;
                }
                Err(e) => {
                    eprintln!("sound: {e}");
                    // try again later, quietly, with the same name (no message every two seconds)
                    self.device = now;
                }
            }
        }
    }
}

/// In a browser the page pulls sound through Web Audio (web/opensgolf.js): it calls `sg_audio_fill` with its output rate and a
/// frame count and gets back interleaved stereo f32 samples. It asks a little ahead of what is playing and queues the blocks
/// on the browser's audio thread; at its usual 44.1 kHz the samples pass through unchanged.
#[cfg(target_arch = "wasm32")]
mod device {
    use super::Feed;
    pub struct AudioOut;
    impl AudioOut {
        pub fn keep_alive(&mut self) {}
    }
    static mut FEED: Option<(Box<dyn FnOnce(u32) -> Feed>, Option<Feed>, Vec<f32>)> = None;

    pub fn open(make_feed: impl FnOnce(u32) -> Feed + 'static) -> Option<AudioOut> {
        // SAFETY: the browser build is single threaded; the page only calls back between frames.
        unsafe { FEED = Some((Box::new(make_feed), None, Vec::new())) };
        Some(AudioOut)
    }

    #[no_mangle]
    pub extern "C" fn sg_audio_fill(rate: u32, frames: u32) -> *const f32 {
        // SAFETY: as above.
        #[allow(static_mut_refs)]
        let Some((make, feed, buf)) = (unsafe { FEED.as_mut() }) else {
            return std::ptr::null();
        };
        if feed.is_none() {
            let m = std::mem::replace(make, Box::new(|r| panic!("audio opened twice at {r}")));
            *feed = Some(m(rate));
        }
        let f = feed.as_mut().unwrap();
        f.set_device_rate(rate);
        buf.clear();
        for _ in 0..frames {
            let (l, r) = f.next_frame();
            buf.push(l);
            buf.push(r);
        }
        buf.as_ptr()
    }
}

#[cfg(all(not(feature = "audio"), not(target_arch = "wasm32")))]
mod device {
    pub struct AudioOut;
    impl AudioOut {
        pub fn keep_alive(&mut self) {}
    }
    pub fn open(_mixer: std::sync::Arc<sg_core::mixer::Mixer>) -> Option<AudioOut> {
        None
    }
}

pub use device::AudioOut;

/// The sound's lines so far, for the sound report (`write_report`).
static LINES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// A line about the sound for the log, and in a browser for its console as well (the page shows nothing of the game's
/// standard output, so a player asked "what does the console say?" could not tell why a game was silent). The lines also
/// go into the sound report.
pub fn log(msg: &str) {
    println!("{msg}");
    if let Ok(mut l) = LINES.lock() {
        if l.len() < 50 {
            l.push(msg.to_string());
        }
    }
    #[cfg(target_arch = "wasm32")]
    log_console(msg);
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn log_console(msg: &str) {
    #[link(wasm_import_module = "env")]
    extern "C" {
        // miniquad's gl.js: console.log of a nul-terminated UTF-8 string
        fn console_log(msg: *const std::ffi::c_char);
    }
    if let Ok(c) = std::ffi::CString::new(msg) {
        // SAFETY: a valid nul-terminated string, read before the call returns
        unsafe { console_log(c.as_ptr()) };
    }
}

/// The sound report: what the sound plays from and on, and the clips the player's copy lacked (Mixer::report). Written to
/// `sound.txt` in the save folder on the desktop and Android, and to the console in a browser, for a player to send with a
/// "no sound" report (several came in, all from copies other than the one the port is tested with).
pub fn write_report(mixer: Option<&Mixer>, muted: bool) {
    let mut r = format!("OpenSGolf {} ({}) sound report\n", env!("CARGO_PKG_VERSION"), std::env::consts::OS);
    if muted {
        r += "sound is switched off (Preferences or --mute)\n";
    }
    if let Ok(l) = LINES.lock() {
        for line in l.iter() {
            r += line;
            r += "\n";
        }
    }
    match mixer {
        Some(m) => r += &m.report(),
        None => r += "no sounds loaded\n",
    }
    #[cfg(target_arch = "wasm32")]
    for line in r.lines() {
        log_console(line);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dir = crate::app::save_dir();
        let _ = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join("sound.txt"), &r));
    }
}

/// Called every frame: the sound report is written 20 seconds after the start (when the first sounds have been asked for),
/// then every two minutes, so a report holds the clips a game has asked for so far.
pub fn report_tick(clock: f64, mixer: Option<&Mixer>, muted: bool) {
    static NEXT: std::sync::Mutex<f64> = std::sync::Mutex::new(-1.0);
    let Ok(mut next) = NEXT.lock() else { return };
    if *next < 0.0 {
        *next = clock + 20.0;
    } else if clock >= *next {
        *next = clock + 120.0;
        drop(next);
        write_report(mixer, muted);
    }
}

/// Opens the default output device and feeds it from the mixer. None (and silence) when there is no device; why is logged.
#[allow(clippy::arc_with_non_send_sync)]
pub fn open(mixer: Arc<Mixer>) -> Option<AudioOut> {
    #[cfg(target_arch = "wasm32")]
    {
        device::open(move |rate| Feed::new(mixer, rate))
    }
    #[cfg(not(target_arch = "wasm32"))]
    device::open(mixer)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mixer playing one looping square wave from a WAV written to a temp folder.
    fn tone_mixer(tag: &str) -> Arc<Mixer> {
        let d = std::env::temp_dir().join(format!("sg_feed_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let n = 22050u32;
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36 + n * 2).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        for v in [16u32] {
            w.extend_from_slice(&v.to_le_bytes());
        }
        for v in [1u16, 1] {
            w.extend_from_slice(&v.to_le_bytes());
        }
        w.extend_from_slice(&22050u32.to_le_bytes());
        w.extend_from_slice(&44100u32.to_le_bytes());
        for v in [2u16, 16] {
            w.extend_from_slice(&v.to_le_bytes());
        }
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(n * 2).to_le_bytes());
        for i in 0..n {
            w.extend_from_slice(&(if i % 50 < 25 { 12000i16 } else { -12000 }).to_le_bytes());
        }
        std::fs::write(d.join("tone.wav"), w).unwrap();
        let mut m = Mixer::new(&d);
        m.with_fallback();
        let m = Arc::new(m);
        assert!(m.play("tone.wav", 1.0, true) > 0);
        m
    }

    #[test]
    fn feed_plays_at_every_device_rate() {
        // the rates output devices really run at (Windows and Macs mostly 48 kHz, some 96 kHz; browsers follow the device)
        for rate in [22050u32, 32000, 44100, 48000, 88200, 96000, 192000] {
            let mut f = Feed::new(tone_mixer(&rate.to_string()), rate);
            let frames = rate as usize * 2;
            let (mut loud, mut peak) = (0usize, 0f32);
            for _ in 0..frames {
                let (l, r) = f.next_frame();
                assert!(l.is_finite() && r.is_finite());
                peak = peak.max(l.abs()).max(r.abs());
                if l.abs() > 0.05 {
                    loud += 1;
                }
            }
            assert!(peak > 0.2, "{rate} Hz: peak {peak}");
            assert!(loud > frames * 8 / 10, "{rate} Hz: only {loud} of {frames} frames sound");
        }
    }

    #[test]
    fn feed_resamples_silence() {
        let m = Arc::new(Mixer::new(std::path::Path::new("/nonexistent")));
        let mut f = Feed::new(m, 48000);
        for _ in 0..5000 {
            assert_eq!(f.next_frame(), (0.0, 0.0));
        }
    }
}
