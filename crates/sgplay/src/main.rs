//! Plays the game's Bink videos (Flics/SMSG_IntroFinal.bik, SMSG_ClosingFinal.bik).
//!
//! The Bink codec is RAD Game Tools' and the original plays it through binkw32.dll. This tool reads the file header itself but hands
//! the video and audio decoding to an installed `ffmpeg` program, which has an open implementation of both Bink codecs (macOS:
//! `brew install ffmpeg`; Windows: put ffmpeg.exe on the PATH). ffmpeg runs as two child processes (raw RGB video, raw PCM audio)
//! and the audio clock keeps the picture in step. Files that are not Bink are measured with ffprobe instead.
//!
//!   sgplay FILE.bik                          play, Esc or Space to quit
//!   sgplay FILE.bik --png out.png --at 12.5  save the frame at that time and exit
//!   sgplay FILE.bik --info                   print the header
use miniquad::*;
use std::collections::VecDeque;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

struct Info {
    width: u32,
    height: u32,
    fps: f64,
    frames: u32,
    audio: bool,
    label: String,
}

fn probe(file: &str) -> Option<Info> {
    if let Some(h) = std::fs::read(file).ok().and_then(|d| sg_core::bink::parse_bink_header(&d)) {
        return Some(Info {
            width: h.width,
            height: h.height,
            fps: h.fps(),
            frames: h.frames,
            audio: h.audio_tracks > 0,
            label: String::from_utf8_lossy(&h.magic).into_owned(),
        });
    }
    // Not Bink: ask ffprobe.
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate,nb_frames",
            "-of",
            "csv=p=0",
            file,
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let f: Vec<&str> = s.trim().split(',').collect();
    if f.len() < 3 {
        return None;
    }
    let (num, den) = f[2].split_once('/').unwrap_or((f[2], "1"));
    let fps = num.parse::<f64>().ok()? / den.parse::<f64>().ok().filter(|d| *d > 0.0).unwrap_or(1.0);
    let has_audio = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "a:0", "-show_entries", "stream=index", "-of", "csv=p=0", file])
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    Some(Info {
        width: f[0].parse().ok()?,
        height: f[1].parse().ok()?,
        fps,
        frames: f.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
        audio: has_audio,
        label: "ffprobe".into(),
    })
}

fn ffmpeg(args: &[&str]) -> Option<Child> {
    Command::new("ffmpeg").args(args).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()
}

/// Shared between the reader threads and the window.
struct Shared {
    frames: Mutex<VecDeque<Vec<u8>>>,
    video_done: AtomicBool,
    quit: AtomicBool,
    audio: Mutex<VecDeque<i16>>,
    /// Stereo frames handed to the device so far: the playback clock.
    audio_played: AtomicU64,
    audio_done: AtomicBool,
}

struct Player {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    info: Info,
    shared: Arc<Shared>,
    start: f64,
    next_frame: u64,
    shown: bool,
    children: Vec<Child>,
    _audio: Option<audio::Out>,
}

const VERTEX: &str = r#"#version 100
attribute vec2 in_pos;
attribute vec2 in_uv;
uniform vec2 scale;
varying highp vec2 uv;
void main() { gl_Position = vec4(in_pos * scale, 0.0, 1.0); uv = in_uv; }
"#;
const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying highp vec2 uv;
uniform sampler2D tex;
void main() { gl_FragColor = texture2D(tex, uv); }
"#;

#[repr(C)]
struct Uniforms {
    scale: [f32; 2],
}

impl Player {
    fn clock(&self) -> f64 {
        if self._audio.is_some() {
            self.shared.audio_played.load(Ordering::Relaxed) as f64 / 44100.0
        } else {
            date::now() - self.start
        }
    }
}

impl EventHandler for Player {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let clock = self.clock();
        let (mut current, finished) = {
            let mut q = self.shared.frames.lock().unwrap();
            let mut cur = None;
            // Take every frame that is due; show the newest of them.
            while !q.is_empty() && self.next_frame as f64 / self.info.fps <= clock {
                cur = q.pop_front();
                self.next_frame += 1;
            }
            (cur, self.shared.video_done.load(Ordering::Relaxed) && q.is_empty())
        };
        if let Some(rgb) = current.take() {
            let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
            for p in rgb.as_chunks::<3>().0 {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
            self.ctx.texture_update(self.bindings.images[0], &rgba);
            self.shown = true;
        }
        let (dw, dh) = window::screen_size();
        let (ar, wr) = (self.info.width as f32 / self.info.height as f32, dw / dh.max(1.0));
        // Letterbox to keep the aspect ratio.
        let scale = if wr > ar { [ar / wr, 1.0] } else { [1.0, wr / ar] };
        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        if self.shown {
            self.ctx.apply_pipeline(&self.pipeline);
            self.ctx.apply_bindings(&self.bindings);
            self.ctx.apply_uniforms(UniformsSource::table(&Uniforms { scale }));
            self.ctx.draw(0, 6, 1);
        }
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
        if finished {
            window::order_quit();
        }
    }

    fn key_down_event(&mut self, k: KeyCode, _: KeyMods, _: bool) {
        if k == KeyCode::Escape || k == KeyCode::Space {
            window::order_quit();
        }
    }

    fn quit_requested_event(&mut self) {
        self.shared.quit.store(true, Ordering::Relaxed);
        for c in self.children.iter_mut() {
            let _ = c.kill();
        }
    }
}

mod audio {
    use super::Shared;
    use std::sync::Arc;

    #[cfg(feature = "audio")]
    pub struct Out {
        _stream: cpal::Stream,
    }
    #[cfg(not(feature = "audio"))]
    pub struct Out;

    /// Plays the 44.1 kHz stereo queue on the default device (nearest-sample resampling to the device rate).
    #[cfg(feature = "audio")]
    pub fn open(shared: Arc<Shared>) -> Option<Out> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        use std::sync::atomic::Ordering;
        let dev = cpal::default_host().default_output_device()?;
        let sup = dev.default_output_config().ok()?;
        if sup.sample_format() != cpal::SampleFormat::F32 {
            return None;
        }
        let cfg: cpal::StreamConfig = sup.into();
        let ch = cfg.channels as usize;
        let step = 44100.0 / cfg.sample_rate.0 as f64;
        let mut frac = 0.0f64;
        let mut cur = (0i16, 0i16);
        let stream = dev
            .build_output_stream(
                &cfg,
                move |data: &mut [f32], _| {
                    let mut q = shared.audio.lock().unwrap();
                    for frame in data.chunks_mut(ch.max(1)) {
                        frac += step;
                        while frac >= 1.0 {
                            frac -= 1.0;
                            if q.len() >= 2 {
                                cur = (q.pop_front().unwrap(), q.pop_front().unwrap());
                                shared.audio_played.fetch_add(1, Ordering::Relaxed);
                            } else if shared.audio_done.load(Ordering::Relaxed) {
                                // Out of audio: keep the clock running so the last frames still show.
                                cur = (0, 0);
                                shared.audio_played.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        for (i, s) in frame.iter_mut().enumerate() {
                            *s = match (ch, i) {
                                (1, _) => (cur.0 as f32 + cur.1 as f32) / 65536.0,
                                (_, 0) => cur.0 as f32 / 32768.0,
                                (_, 1) => cur.1 as f32 / 32768.0,
                                _ => 0.0,
                            };
                        }
                    }
                },
                |e| eprintln!("sound: {e}"),
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(Out { _stream: stream })
    }
    #[cfg(not(feature = "audio"))]
    pub fn open(_shared: Arc<Shared>) -> Option<Out> {
        None
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut file, mut png_out, mut at, mut info_only) = (String::new(), None, 0.0f64, false);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--png" if i + 1 < args.len() => {
                png_out = Some(args[i + 1].clone());
                i += 1;
            }
            "--at" if i + 1 < args.len() => {
                at = args[i + 1].parse().unwrap_or(0.0);
                i += 1;
            }
            "--info" => info_only = true,
            a => file = a.to_string(),
        }
        i += 1;
    }
    if file.is_empty() {
        eprintln!("usage: sgplay FILE.bik [--info] [--png out.png --at SECONDS]");
        std::process::exit(2);
    }
    let Some(info) = probe(&file) else {
        eprintln!("error: cannot read a Bink header from {file}");
        std::process::exit(1);
    };
    println!(
        "{file}: {}, {}x{}, {} frames at {:.3} fps ({:.1} s), {} audio track(s)",
        info.label,
        info.width,
        info.height,
        info.frames,
        info.fps,
        info.frames as f64 / info.fps,
        info.audio as i32
    );
    if info_only {
        return;
    }
    if Command::new("ffmpeg").arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| !s.success()).unwrap_or(true) {
        eprintln!(
            "error: ffmpeg is not installed (macOS: brew install ffmpeg; Windows: ffmpeg.exe on the PATH); it decodes the Bink codec"
        );
        std::process::exit(1);
    }
    let frame_bytes = (info.width * info.height * 3) as usize;
    let mut video = ffmpeg(&["-v", "quiet", "-i", &file, "-f", "rawvideo", "-pix_fmt", "rgb24", "-"]).unwrap_or_else(|| {
        eprintln!("error: cannot start ffmpeg");
        std::process::exit(1)
    });
    let mut vout = video.stdout.take().unwrap();

    if let Some(out) = png_out {
        // Decode up to the requested time and save one frame.
        let target = (at * info.fps) as u64;
        let mut buf = vec![0u8; frame_bytes];
        let mut n = 0u64;
        while vout.read_exact(&mut buf).is_ok() {
            n += 1;
            if n > target {
                break;
            }
        }
        let _ = video.kill();
        let _ = video.wait();
        let mut img = sg_core::assets::Rgba::new(info.width, info.height);
        for (o, p) in img.px.as_chunks_mut::<4>().0.iter_mut().zip(buf.as_chunks::<3>().0) {
            o.copy_from_slice(&[p[0], p[1], p[2], 255]);
        }
        if !sg_core::png::write_png(&out, &img) {
            eprintln!("error: cannot write {out}");
            std::process::exit(1);
        }
        println!("saved frame {} to {out}", n.saturating_sub(1));
        return;
    }

    let shared = Arc::new(Shared {
        frames: Mutex::new(VecDeque::new()),
        video_done: AtomicBool::new(false),
        quit: AtomicBool::new(false),
        audio: Mutex::new(VecDeque::new()),
        audio_played: AtomicU64::new(0),
        audio_done: AtomicBool::new(false),
    });
    let mut children = vec![];
    {
        // Video frames are read on a thread into a small queue.
        let sh = shared.clone();
        std::thread::spawn(move || {
            let mut f = vec![0u8; frame_bytes];
            while !sh.quit.load(Ordering::Relaxed) && vout.read_exact(&mut f).is_ok() {
                loop {
                    if sh.quit.load(Ordering::Relaxed) {
                        break;
                    }
                    let mut q = sh.frames.lock().unwrap();
                    if q.len() < 6 {
                        q.push_back(f.clone());
                        break;
                    }
                    drop(q);
                    std::thread::sleep(std::time::Duration::from_millis(4));
                }
            }
            sh.video_done.store(true, Ordering::Relaxed);
        });
    }
    children.push(video);
    let mut audio_child = None;
    if info.audio {
        if let Some(mut a) = ffmpeg(&["-v", "quiet", "-i", &file, "-vn", "-f", "s16le", "-ar", "44100", "-ac", "2", "-"]) {
            let mut aout = a.stdout.take().unwrap();
            let sh = shared.clone();
            std::thread::spawn(move || {
                let mut buf = vec![0u8; 44100 * 4 / 10]; // 100 ms chunks
                while !sh.quit.load(Ordering::Relaxed) {
                    if sh.audio.lock().unwrap().len() > 44100 {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    match aout.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let mut q = sh.audio.lock().unwrap();
                            q.extend(buf[..n & !1].as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b)));
                        }
                    }
                }
                sh.audio_done.store(true, Ordering::Relaxed);
            });
            audio_child = Some(a);
        }
    }
    children.extend(audio_child);

    let conf = conf::Conf {
        window_title: "SimGolf native: video".into(),
        window_width: info.width as i32,
        window_height: info.height as i32,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    };
    miniquad::start(conf, move || {
        let mut ctx = window::new_rendering_backend();
        #[rustfmt::skip]
        let verts: [f32; 16] = [-1.0, -1.0, 0.0, 1.0,  1.0, -1.0, 1.0, 1.0,  1.0, 1.0, 1.0, 0.0,  -1.0, 1.0, 0.0, 0.0];
        let vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&verts));
        let ib = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&[0u16, 1, 2, 0, 2, 3]));
        let tex = ctx.new_texture(
            TextureAccess::Static,
            TextureSource::Empty,
            TextureParams { width: info.width, height: info.height, format: TextureFormat::RGBA8, ..Default::default() },
        );
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                ShaderMeta {
                    images: vec!["tex".into()],
                    uniforms: UniformBlockLayout { uniforms: vec![UniformDesc::new("scale", UniformType::Float2)] },
                },
            )
            .expect("shader");
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("in_pos", VertexFormat::Float2), VertexAttribute::new("in_uv", VertexFormat::Float2)],
            shader,
            PipelineParams::default(),
        );
        let out = if info.audio { audio::open(shared.clone()) } else { None };
        if info.audio && out.is_none() {
            eprintln!("sound: no audio device, playing silent video");
        }
        Box::new(Player {
            ctx,
            pipeline,
            bindings: Bindings { vertex_buffers: vec![vb], index_buffer: ib, images: vec![tex] },
            info,
            shared,
            start: date::now(),
            next_frame: 0,
            shown: false,
            children,
            _audio: out,
        })
    });
}
