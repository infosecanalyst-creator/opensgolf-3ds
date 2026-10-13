//! Android: where the app keeps its files, and the hookup sound output needs.
//!
//! * Saves, settings, packs and championship files go in the app's internal data folder (`Context.getFilesDir()`, e.g.
//!   `/data/user/0/org.opensgolf.game/files`). Android removes it with the app; nothing else can read it.
//! * The game runs on the free base set with no files at all. A player who owns SimGolf can copy the original game's
//!   `Program_Files_(ENGLISH)` folder (or the whole installed folder) to the app's external files folder,
//!   `Android/data/org.opensgolf.game/files/game` on the phone's shared storage (the folder a USB cable or a file manager
//!   shows), and the game reads it from there (docs/PLAYING.md, "Android").
//! * cpal's Android sound output (oboe) asks Java for buffer sizes through `ndk_context`, which miniquad does not set up:
//!   [`init`] hands it miniquad's Java VM and activity before the sound device is opened.
//!
//! * Nothing the game prints would otherwise be seen (Android sends a native program's stdout and stderr nowhere):
//!   [`init`] passes both to the system log under the tag "OpenSGolf" (`adb logcat -s OpenSGolf`), and a panic is written
//!   there and to `crash.txt` in the external files folder (`Android/data/org.opensgolf.game/files/crash.txt`), which a
//!   player can send without any developer tools (docs/INSTALL.md, "Android").
//!
//! Everything else here is plain JNI calls on miniquad's activity (miniquad's own `quad_native` Java glue).

use miniquad::native::android::{self, ndk_sys};
use std::ffi::{c_char, c_int, CStr, CString};
use std::io::{BufRead, BufReader};
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The tag of everything the game writes to the system log.
const TAG: &CStr = c"OpenSGolf";
const LOG_INFO: c_int = 4;
const LOG_FATAL: c_int = 7;

#[link(name = "log")]
extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
}

/// One message to the system log (logcat cuts an entry at about 4 KB: long text goes line by line).
fn log(prio: c_int, text: &str) {
    for line in text.lines() {
        let c = CString::new(line.replace('\0', " ")).unwrap_or_default();
        unsafe {
            __android_log_write(prio, TAG.as_ptr(), c.as_ptr());
        }
    }
}

/// stdout and stderr into the system log: a pipe in their place, read by a thread that logs each line.
fn redirect_output() {
    let mut fds = [0 as c_int; 2];
    if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
        return;
    }
    unsafe {
        dup2(fds[1], 1);
        dup2(fds[1], 2);
    }
    // SAFETY: fds[0] is the pipe's read end, owned by the reader from here on
    let read = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let _ = std::thread::Builder::new().name("opensgolf-log".into()).spawn(move || {
        for line in BufReader::new(read).lines() {
            match line {
                Ok(l) => log(LOG_INFO, &l),
                Err(_) => break,
            }
        }
    });
}

/// Where a panic is written for the player to send: the external files folder, which a file manager or a USB cable shows.
static CRASH_FILE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Logs a panic (message, place and backtrace) and writes it to `crash.txt`, then closes the app: a panic on the game's
/// thread would otherwise leave a frozen picture. A panic in the download thread (net.rs) only ends the download.
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("unnamed");
        let text = format!(
            "OpenSGolf {} panicked on thread '{name}': {info}\n\nbacktrace:\n{}",
            env!("CARGO_PKG_VERSION"),
            std::backtrace::Backtrace::force_capture()
        );
        log(LOG_FATAL, &text);
        if let Some(Some(path)) = CRASH_FILE.get() {
            match std::fs::write(path, &text) {
                Ok(()) => log(LOG_FATAL, &format!("written to {}", path.display())),
                Err(e) => log(LOG_FATAL, &format!("could not write {}: {e}", path.display())),
            }
        }
        default(info);
        if name != crate::net::NET_THREAD {
            std::process::abort();
        }
    }));
}

/// The two folders, looked up once.
struct Dirs {
    files: Option<PathBuf>,
    external: Option<PathBuf>,
}

static DIRS: OnceLock<Dirs> = OnceLock::new();

fn dirs() -> &'static Dirs {
    DIRS.get_or_init(|| unsafe { Dirs { files: context_dir("getFilesDir", false), external: context_dir("getExternalFilesDir", true) } })
}

/// A `java.io.File` folder of the activity (`getFilesDir()` or `getExternalFilesDir(null)`) as a path.
unsafe fn context_dir(method: &str, takes_type: bool) -> Option<PathBuf> {
    let activity = android::ACTIVITY;
    if activity.is_null() {
        return None;
    }
    let env = android::attach_jni_env();
    let file = if takes_type {
        let null: ndk_sys::jobject = std::ptr::null_mut();
        miniquad::call_object_method!(env, activity, method, "(Ljava/lang/String;)Ljava/io/File;", null)
    } else {
        miniquad::call_object_method!(env, activity, method, "()Ljava/io/File;")
    };
    // a pending exception (no shared storage mounted) leaves no folder
    if (**env).ExceptionCheck.unwrap()(env) != 0 {
        (**env).ExceptionClear.unwrap()(env);
        return None;
    }
    if file.is_null() {
        return None;
    }
    let jpath = miniquad::call_object_method!(env, file, "getAbsolutePath", "()Ljava/lang/String;");
    if jpath.is_null() {
        return None;
    }
    let chars = (**env).GetStringUTFChars.unwrap()(env, jpath, std::ptr::null_mut());
    if chars.is_null() {
        return None;
    }
    let path = CStr::from_ptr(chars).to_string_lossy().into_owned();
    (**env).ReleaseStringUTFChars.unwrap()(env, jpath, chars);
    (**env).DeleteLocalRef.unwrap()(env, jpath);
    (**env).DeleteLocalRef.unwrap()(env, file);
    Some(PathBuf::from(path))
}

/// The app's internal data folder: the save folder.
pub fn files_dir() -> PathBuf {
    dirs().files.clone().unwrap_or_else(|| PathBuf::from("/data/data/org.opensgolf.game/files"))
}

/// Where a player's copy of the original game goes: `<external files folder>/game`, which may hold the game's
/// `Program_Files_(ENGLISH)` folder (the whole installed folder copied over) or be that folder itself. The folder is
/// created, empty, so the player finds it with a file manager.
pub fn game_dir() -> PathBuf {
    let base = dirs().external.clone().unwrap_or_else(|| PathBuf::from("/sdcard/Android/data/org.opensgolf.game/files"));
    let game = base.join("game");
    let _ = std::fs::create_dir_all(&game);
    let inner = game.join("Program_Files_(ENGLISH)");
    if inner.is_dir() {
        inner
    } else {
        game
    }
}

/// Hands miniquad's Java VM and activity to `ndk_context` (for cpal's oboe output). Call once, from `main`, which miniquad
/// runs on the activity's thread after setting the activity.
///
/// First of all it passes the game's output to the system log and sets up the panic hook, so that nothing after this can fail
/// without a word.
pub fn init() {
    redirect_output();
    install_panic_hook();
    let dirs = dirs();
    let _ = CRASH_FILE.set(dirs.external.as_ref().map(|d| d.join("crash.txt")));
    println!("OpenSGolf {} starting: data folder {:?}, external folder {:?}", env!("CARGO_PKG_VERSION"), dirs.files, dirs.external);
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return;
        }
        let env = android::attach_jni_env();
        let mut vm: *mut ndk_sys::JavaVM = std::ptr::null_mut();
        if (**env).GetJavaVM.unwrap()(env, &mut vm) == 0 && !vm.is_null() {
            ndk_context::initialize_android_context(vm as *mut _, activity as *mut _);
        }
    }
}
