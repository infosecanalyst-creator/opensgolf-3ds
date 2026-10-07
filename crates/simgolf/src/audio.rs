//! Sound output: feeds the system's default output device from the Mixer (44.1 kHz stereo), resampling when the device runs at
//! another rate. Built with the `audio` feature (cpal: CoreAudio on macOS, WASAPI on Windows, ALSA on Linux).
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

    pub struct AudioOut {
        _stream: cpal::Stream,
    }

    fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
        dev: &cpal::Device,
        cfg: &cpal::StreamConfig,
        mut feed: Feed,
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
            |e| eprintln!("sound: {e}"),
            None,
        )
    }

    pub fn open(make_feed: impl FnOnce(u32) -> Feed) -> Option<AudioOut> {
        let host = cpal::default_host();
        let dev = host.default_output_device()?;
        let sup = dev.default_output_config().ok()?;
        let fmt = sup.sample_format();
        let cfg: cpal::StreamConfig = sup.into();
        let feed = make_feed(cfg.sample_rate.0);
        let stream = match fmt {
            cpal::SampleFormat::F32 => build::<f32>(&dev, &cfg, feed),
            cpal::SampleFormat::I16 => build::<i16>(&dev, &cfg, feed),
            cpal::SampleFormat::U16 => build::<u16>(&dev, &cfg, feed),
            cpal::SampleFormat::I32 => build::<i32>(&dev, &cfg, feed),
            _ => return None,
        }
        .ok()?;
        stream.play().ok()?;
        Some(AudioOut { _stream: stream })
    }
}

#[cfg(not(feature = "audio"))]
mod device {
    use super::Feed;
    pub struct AudioOut;
    pub fn open(_make_feed: impl FnOnce(u32) -> Feed) -> Option<AudioOut> {
        None
    }
}

pub use device::AudioOut;

/// Opens the default output device and feeds it from the mixer. None (and silence) when there is no device.
pub fn open(mixer: Arc<Mixer>) -> Option<AudioOut> {
    device::open(move |rate| Feed::new(mixer, rate))
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
