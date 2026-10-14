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
                eprintln!("sound: the output stopped: {e}");
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
        println!("sound: playing on \"{name}\" ({} Hz, {} channels, {fmt:?})", cfg.sample_rate.0, cfg.channels);
        Ok((stream, name))
    }

    pub fn open(mixer: Arc<Mixer>) -> Option<AudioOut> {
        let failed = Arc::new(AtomicBool::new(false));
        match start(&mixer, &failed) {
            Ok((stream, device)) => Some(AudioOut { stream, mixer, device, failed, checked: Instant::now() }),
            Err(e) => {
                eprintln!("sound: {e}");
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

    #[test]
    fn feed_resamples_silence() {
        let m = Arc::new(Mixer::new(std::path::Path::new("/nonexistent")));
        let mut f = Feed::new(m, 48000);
        for _ in 0..5000 {
            assert_eq!(f.next_frame(), (0.0, 0.0));
        }
    }
}
