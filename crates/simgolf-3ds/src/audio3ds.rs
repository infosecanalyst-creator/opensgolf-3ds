//! Sound on the 3DS: the DSP (ndsp) plays what `sg_core::mixer::Mixer` mixes, 44.1 kHz stereo 16-bit, through two wave
//! buffers that are refilled as each finishes. The desktop's `audio.rs` resamples for the device; ndsp takes the mixer's
//! own rate, so the samples go straight through.
//!
//! The DSP firmware has to be dumped once per console (DSP1 from the Homebrew Launcher) or ndsp fails to start; `open`
//! then returns None and the game runs silent, as the desktop build does without the `audio` feature.
use ctru::linear::LinearAllocator;
use ctru::services::ndsp::{wave::Status, wave::Wave, AudioFormat, AudioMix, InterpolationType, Ndsp, OutputMode};
use sg_core::mixer::{Mixer, RATE};
use std::sync::Arc;

/// Frames per wave buffer: ~46 ms at 44.1 kHz, so a slow frame does not starve it.
const CHUNK: usize = 2048;

type Buf = Vec<u8, LinearAllocator>;

pub struct AudioOut {
    ndsp: Ndsp,
    mixer: Arc<Mixer>,
    /// Boxed: ndsp keeps pointers to the queued wave buffers' headers, so a queued `Wave` must never move (this struct is
    /// returned by value from `open`, which would move them).
    waves: [Box<Wave<Buf>>; 2],
}

impl AudioOut {
    pub fn open(mixer: Arc<Mixer>) -> Option<AudioOut> {
        let mut ndsp = Ndsp::new().ok()?;
        ndsp.set_output_mode(OutputMode::Stereo);
        let mut waves = [Box::new(Self::wave()), Box::new(Self::wave())];
        {
            // API CHECK: Ndsp::channel(id) -> Channel; its setters and queue_wave.
            let mut ch = ndsp.channel(0).ok()?;
            ch.set_interpolation(InterpolationType::Linear);
            ch.set_sample_rate(RATE as f32);
            ch.set_format(AudioFormat::PCM16Stereo);
            // both front speakers at full volume (set rather than left to the channel's reset values)
            ch.set_mix(&AudioMix::default());
            for w in waves.iter_mut() {
                Self::fill(&mixer, w);
                ch.queue_wave(w).ok()?;
            }
        }
        Some(AudioOut { ndsp, mixer, waves })
    }

    fn wave() -> Wave<Buf> {
        let mut buf: Buf = Vec::with_capacity_in(CHUNK * 2 * 2, LinearAllocator);
        buf.resize(CHUNK * 2 * 2, 0);
        Wave::new(buf, AudioFormat::PCM16Stereo, false)
    }

    fn fill(mixer: &Mixer, w: &mut Wave<Buf>) {
        let mut pcm = vec![0i16; CHUNK * 2];
        mixer.mix(&mut pcm);
        let bytes = w.get_buffer_mut().expect("wave buffer");
        for (d, s) in bytes.chunks_exact_mut(2).zip(pcm.iter()) {
            d.copy_from_slice(&s.to_le_bytes());
        }
        // The DSP reads RAM directly, not through the CPU's cache: without this it keeps playing whatever the buffer held
        // when ctru-rs first flushed it (silence; `Wave::new` flushes once, `queue_wave` never does).
        unsafe {
            let _ = ctru_sys::DSP_FlushDataCache(bytes.as_ptr().cast(), bytes.len() as u32);
        }
    }

    /// Refills and requeues any wave the DSP has finished with (called every few milliseconds by the audio thread).
    pub fn keep_alive(&mut self) {
        let Ok(mut ch) = self.ndsp.channel(0) else {
            return;
        };
        for w in self.waves.iter_mut() {
            if w.status() == Status::Done {
                Self::fill(&self.mixer, w);
                let _ = ch.queue_wave(w);
            }
        }
    }
}

/// Sound on its own CPU core, so mixing never competes with drawing: core 2 on a New 3DS (free for games), else the system
/// core (core 1) with 30% of its time. The thread owns the DSP service; the game only calls `Mixer::play`. Created with
/// libctru's `threadCreate` (the standard library's 3DS thread options, core and priority, are gone from current nightlies).
pub struct AudioThread {
    ctx: *mut ThreadCtx,
    thread: ctru_sys::Thread,
    /// Which core it runs on, for the log.
    pub core: i32,
}

struct ThreadCtx {
    mixer: Arc<Mixer>,
    stop: std::sync::atomic::AtomicBool,
    /// 0 starting, 1 running, 2 the DSP did not start
    state: std::sync::atomic::AtomicU8,
}

unsafe extern "C" fn audio_main(arg: *mut core::ffi::c_void) {
    use std::sync::atomic::Ordering;
    let ctx = unsafe { &*(arg as *const ThreadCtx) };
    let Some(mut out) = AudioOut::open(ctx.mixer.clone()) else {
        ctx.state.store(2, Ordering::Release);
        return;
    };
    ctx.state.store(1, Ordering::Release);
    while !ctx.stop.load(Ordering::Relaxed) {
        out.keep_alive();
        // a wave is ~46 ms of sound: checking every 5 ms keeps the next one queued with room to spare
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

impl AudioThread {
    /// None when no core could be had or the DSP did not start (no DSP firmware dump): the game runs silent.
    pub fn start(mixer: Arc<Mixer>, apt: &mut ctru::services::apt::Apt) -> Option<AudioThread> {
        use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
        let mut new3ds = false;
        unsafe { ctru_sys::APT_CheckNew3DS(&mut new3ds) };
        let mut cores = Vec::new();
        if new3ds {
            cores.push(2);
        }
        if apt.set_app_cpu_time_limit(30).is_ok() {
            cores.push(1);
        }
        let mut prio = 0x30;
        unsafe { ctru_sys::svcGetThreadPriority(&mut prio, 0xFFFF8000) }; // CUR_THREAD_HANDLE
        for core in cores {
            let ctx = Box::into_raw(Box::new(ThreadCtx { mixer: mixer.clone(), stop: AtomicBool::new(false), state: AtomicU8::new(0) }));
            let thread = unsafe { ctru_sys::threadCreate(Some(audio_main), ctx as *mut _, 64 * 1024, prio - 1, core, false) };
            if thread.is_null() {
                drop(unsafe { Box::from_raw(ctx) });
                continue;
            }
            let t0 = std::time::Instant::now();
            let state = loop {
                match unsafe { &*ctx }.state.load(Ordering::Acquire) {
                    0 if t0.elapsed().as_secs() < 3 => std::thread::sleep(std::time::Duration::from_millis(2)),
                    s => break s,
                }
            };
            let t = AudioThread { ctx, thread, core };
            return match state {
                1 => Some(t),
                0 => {
                    // never got going (the core is not being scheduled): leave it be rather than wait on it forever
                    unsafe { &*ctx }.stop.store(true, Ordering::Relaxed);
                    std::mem::forget(t);
                    None
                }
                _ => None, // dropping `t` joins the finished thread
            };
        }
        None
    }
}

impl Drop for AudioThread {
    fn drop(&mut self) {
        unsafe {
            (*self.ctx).stop.store(true, std::sync::atomic::Ordering::Relaxed);
            ctru_sys::threadJoin(self.thread, u64::MAX);
            ctru_sys::threadFree(self.thread);
            drop(Box::from_raw(self.ctx));
        }
    }
}
