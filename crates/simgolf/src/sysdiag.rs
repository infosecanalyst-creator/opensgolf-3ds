//! The system around the game: the graphics driver's check at start, a stop when the window is gone, a message box for a
//! start that cannot go on, and SG_DIAG's log (driver, frame times, memory, window events) for reports from other machines.
//!
//! SG_DIAG=N (seconds, default 60) prints the OpenGL driver, then every 2 seconds the frames drawn, their mean and longest
//! time, the memory in use and the window events seen; N/2 seconds after the first frame it sends the window a mouse move
//! and a click (Windows), after N seconds it closes the window as the close button does (the program should end at once).
//! Error messages go to the output only, no message box waits for a click.
// the browser and Android have neither the driver check nor the end of the program
#![cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
use std::sync::Mutex;
use std::time::Instant;

struct Diag {
    secs: f64,
    start: Instant,
    last_log: f64,
    first_frame: f64,
    last_frame: f64,
    frames: u32,
    sum_ms: f64,
    max_ms: f64,
    events: [u32; 2],
    clicked: bool,
    closed: bool,
}

static DIAG: Mutex<Option<Diag>> = Mutex::new(None);

/// Window events counted by SG_DIAG.
#[derive(Clone, Copy)]
pub enum Ev {
    Mouse = 0,
    Click = 1,
}

/// SG_DIAG set: logging on from here.
pub fn init() {
    let Some(v) = std::env::var_os("SG_DIAG") else { return };
    let secs = v.to_str().and_then(|s| s.parse::<f64>().ok()).filter(|s| *s > 0.0).unwrap_or(60.0);
    println!("diag: {} {} {}, logging for {secs} s", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH);
    *DIAG.lock().unwrap() = Some(Diag {
        secs,
        start: Instant::now(),
        last_log: 0.0,
        first_frame: 0.0,
        last_frame: 0.0,
        frames: 0,
        sum_ms: 0.0,
        max_ms: 0.0,
        events: [0; 2],
        clicked: false,
        closed: false,
    });
}

pub fn event(e: Ev) {
    if let Some(d) = DIAG.lock().unwrap().as_mut() {
        d.events[e as usize] += 1;
    }
}

/// A line of the log (when SG_DIAG is on).
pub fn note(msg: &str) {
    if let Some(d) = DIAG.lock().unwrap().as_ref() {
        println!("diag {:6.1}s: {msg}", d.start.elapsed().as_secs_f64());
    }
}

/// After each frame: the log's figures, and its scripted click and close.
pub fn frame(w: f32, h: f32) {
    let mut g = DIAG.lock().unwrap();
    let Some(d) = g.as_mut() else { return };
    let t = d.start.elapsed().as_secs_f64();
    if d.frames == 0 {
        d.first_frame = t;
    } else {
        let ms = (t - d.last_frame) * 1000.0;
        d.sum_ms += ms;
        d.max_ms = d.max_ms.max(ms);
    }
    d.last_frame = t;
    d.frames += 1;
    if t - d.last_log >= 2.0 || d.frames == 1 {
        let (ws, private) = memory_mb();
        println!(
            "diag {t:6.1}s: frames {} mean {:.1} ms max {:.1} ms, memory {ws} MB (private {private} MB), mouse {} click {}, \
             {w}x{h}",
            d.frames,
            d.sum_ms / (d.frames - 1).max(1) as f64,
            d.max_ms,
            d.events[0],
            d.events[1]
        );
        d.last_log = t;
        d.max_ms = 0.0;
    }
    // the scripted click and close count from the first frame
    let t = t - d.first_frame;
    if !d.clicked && t >= d.secs / 2.0 {
        d.clicked = true;
        println!("diag first frame + {t:.1}s: sending the window a mouse move and a click");
        post_click();
    }
    if !d.closed && t >= d.secs {
        d.closed = true;
        println!("diag first frame + {t:.1}s: closing the window");
        drop(g);
        post_close();
    }
}

/// The driver's own names, on the first frame (any platform) and in the check at start.
pub fn gl_strings() -> [String; 4] {
    use miniquad::gl;
    let get = |name: u32| {
        // SAFETY: glGetString with a valid name on the current context; the result is a static NUL-ended string or null
        unsafe {
            let p = gl::glGetString(name);
            if p.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(p as *const std::ffi::c_char).to_string_lossy().into_owned()
            }
        }
    };
    [get(gl::GL_VENDOR), get(0x1F01 /* GL_RENDERER */), get(gl::GL_VERSION), get(0x8B8C)]
    // GL_SHADING_LANGUAGE_VERSION
}

/// The check at start: the driver must give OpenGL 2.1 or later in hardware. Windows' own stand-in, "GDI Generic" (OpenGL
/// 1.1 in software, what a Windows program gets without a real driver, e.g. a 64-bit ARM program on a PC whose graphics
/// driver has no OpenGL for ARM programs) cannot run the game: a message says what to do and the program ends.
pub fn check_gl() {
    let [vendor, renderer, version, glsl] = gl_strings();
    println!("GL: {vendor} / {renderer} / {version} / GLSL {glsl}");
    let num = version
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .find(|s| !s.is_empty())
        .and_then(|s| {
            let mut it = s.split('.').map(|n| n.parse::<u32>().unwrap_or(0));
            Some((it.next()?, it.next().unwrap_or(0)))
        })
        .unwrap_or((0, 0));
    // OpenGL ES (Android, the browser's WebGL 2) reports "OpenGL ES 3.0 ...": the check is for desktop OpenGL only
    let es = version.contains("ES");
    let unusable = renderer.contains("GDI Generic") || (!es && !version.is_empty() && num < (2, 1));
    if unusable {
        fatal(&format!(
            "OpenSGolf needs OpenGL 2.1 or later, but this PC's graphics driver gives only {version} ({renderer}) to this \
             program.\n\n{}",
            advice()
        ));
    }
}

/// What a player can do when the graphics driver cannot run the game.
fn advice() -> &'static str {
    if cfg!(all(windows, target_arch = "aarch64")) {
        "Your graphics driver doesn't support OpenGL for ARM apps. Please use the regular Windows (x64) download \
         (opensgolf-windows.zip), which runs on ARM PCs too, or install the \"OpenCL, OpenGL, and Vulkan Compatibility \
         Pack\" from the Microsoft Store."
    } else if cfg!(windows) {
        "Please install the latest graphics driver for your PC from its maker (Intel, AMD, NVIDIA or Qualcomm)."
    } else {
        "Please install or update your system's OpenGL graphics driver."
    }
}

/// Ends the program with a message: a message box on Windows (the program has no console there), the error output elsewhere.
pub fn fatal(msg: &str) -> ! {
    eprintln!("{msg}");
    #[cfg(windows)]
    win::message_box("OpenSGolf", msg);
    std::process::exit(1);
}

/// Windows: a panic before or during start (miniquad's "WGL: ..." when the driver has no usable OpenGL) shows a message box
/// instead of a window that silently vanishes, and ends the program (sound included).
///
/// On every desktop the panic (message, place, backtrace) is also written to `crash.txt` in the save folder (app.rs
/// `save_dir`), as Android writes its own: a game started from Finder or a desktop menu has nowhere to show its standard
/// error, so a player reporting a crash had nothing to send.
pub fn install_panic_hook() {
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let name = std::thread::current().name().unwrap_or("unnamed").to_string();
            let text = format!(
                "OpenSGolf {} ({}) panicked on thread '{name}': {info}\n\nbacktrace:\n{}",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::backtrace::Backtrace::force_capture()
            );
            let dir = crate::app::save_dir();
            let path = dir.join("crash.txt");
            if std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &text)).is_ok() {
                eprintln!("the crash is written to {}", path.display());
            }
            default(info);
        }));
    }
    #[cfg(windows)]
    {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            default(info);
            let what = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_default();
            let gl = what.contains("WGL") || what.contains("OpenGL") || what.contains("opengl32");
            let msg = if gl {
                format!("OpenSGolf could not start its graphics ({what}).\n\n{}", advice())
            } else {
                format!("OpenSGolf stopped with an error: {what}\n\n{}", info.location().map(|l| l.to_string()).unwrap_or_default())
            };
            win::message_box("OpenSGolf", &msg);
            std::process::exit(1);
        }));
    }
}

/// Called each frame: the game's window is gone though the main loop still runs (a window procedure that never saw the
/// close, as on 64-bit ARM Windows before the miniquad fix): the program ends rather than play on without a window.
pub fn window_gone() -> bool {
    #[cfg(windows)]
    return win::window_gone();
    #[cfg(not(windows))]
    false
}

fn post_click() {
    #[cfg(windows)]
    win::post_click();
}

fn post_close() {
    #[cfg(windows)]
    win::post_close();
    #[cfg(not(windows))]
    miniquad::window::order_quit();
}

/// Working set and private memory in MB.
fn memory_mb() -> (u64, u64) {
    #[cfg(windows)]
    return win::memory_mb();
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
        let pages: Vec<u64> = statm.split_whitespace().filter_map(|n| n.parse().ok()).collect();
        let mb = |p: u64| p * 4096 / (1024 * 1024);
        (pages.get(1).copied().map(mb).unwrap_or(0), pages.get(5).copied().map(mb).unwrap_or(0))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    (0, 0)
}

#[cfg(windows)]
mod win {
    use std::sync::atomic::{AtomicIsize, Ordering};

    type Hwnd = isize;
    #[repr(C)]
    struct MemCounters {
        cb: u32,
        page_faults: u32,
        peak_ws: usize,
        ws: usize,
        quota: [usize; 4],
        pagefile: usize,
        peak_pagefile: usize,
    }
    #[link(name = "user32")]
    extern "system" {
        fn EnumThreadWindows(thread: u32, f: extern "system" fn(Hwnd, isize) -> i32, l: isize) -> i32;
        fn GetClassNameW(w: Hwnd, buf: *mut u16, n: i32) -> i32;
        fn IsWindow(w: Hwnd) -> i32;
        fn PostMessageW(w: Hwnd, msg: u32, wp: usize, lp: isize) -> i32;
        fn MessageBoxW(w: Hwnd, text: *const u16, title: *const u16, kind: u32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThreadId() -> u32;
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(p: isize, c: *mut MemCounters, cb: u32) -> i32;
    }

    /// The game's window (miniquad's class "MINIQUADAPP" on this, the main, thread), found once.
    static WND: AtomicIsize = AtomicIsize::new(0);

    extern "system" fn each(w: Hwnd, _: isize) -> i32 {
        let mut buf = [0u16; 32];
        // SAFETY: a window handle Windows gave us and a buffer of the size passed
        let n = unsafe { GetClassNameW(w, buf.as_mut_ptr(), buf.len() as i32) };
        if String::from_utf16_lossy(&buf[..n.max(0) as usize]) == "MINIQUADAPP" {
            WND.store(w, Ordering::Relaxed);
            return 0;
        }
        1
    }

    fn wnd() -> Hwnd {
        if WND.load(Ordering::Relaxed) == 0 {
            // SAFETY: enumerates this thread's windows with a callback that only reads their class names
            unsafe { EnumThreadWindows(GetCurrentThreadId(), each, 0) };
        }
        WND.load(Ordering::Relaxed)
    }

    pub fn window_gone() -> bool {
        let w = WND.load(Ordering::Relaxed);
        let w = if w == 0 { wnd() } else { w };
        // SAFETY: IsWindow takes any handle value
        w != 0 && unsafe { IsWindow(w) } == 0
    }

    pub fn post_click() {
        let lp = (300 << 16) | 400;
        // SAFETY: posting plain messages to our own window
        unsafe {
            PostMessageW(wnd(), 0x0200, 0, lp); // WM_MOUSEMOVE
            PostMessageW(wnd(), 0x0201, 1, lp); // WM_LBUTTONDOWN
            PostMessageW(wnd(), 0x0202, 0, lp); // WM_LBUTTONUP
        }
    }

    pub fn post_close() {
        // SAFETY: as above
        unsafe { PostMessageW(wnd(), 0x0010, 0, 0) }; // WM_CLOSE
    }

    pub fn memory_mb() -> (u64, u64) {
        // SAFETY: a zeroed counters struct with its size set, filled in by Windows
        unsafe {
            let mut c: MemCounters = std::mem::zeroed();
            c.cb = std::mem::size_of::<MemCounters>() as u32;
            if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) == 0 {
                return (0, 0);
            }
            ((c.ws >> 20) as u64, (c.pagefile >> 20) as u64)
        }
    }

    pub fn message_box(title: &str, text: &str) {
        // SG_DIAG runs unattended (a test machine): no box to wait on
        if std::env::var_os("SG_DIAG").is_some() {
            return;
        }
        let w = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let (t, b) = (w(title), w(text));
        // SAFETY: NUL-ended UTF-16 strings that outlive the call; MB_OK | MB_ICONERROR
        unsafe { MessageBoxW(0, b.as_ptr(), t.as_ptr(), 0x10) };
    }
}
