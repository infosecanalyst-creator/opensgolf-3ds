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
//! * Since Android 11 a file manager no longer shows that folder, so the panic is also kept in the internal data folder
//!   (`last_crash.txt`), and the next start shows it in a box with a button that copies it ([`take_crash_report`],
//!   App::show_crash_report). A crash in native code (no panic) is found there through Android's record of how the
//!   app's last run ended (`ApplicationExitInfo`, Android 11 and newer).
//!
//! * "Use my SimGolf files..." opens Android's own picker for a .zip of the game folder or the folder itself
//!   ([`pick_game_files`]); the activity's Java side copies the pick into the app's folders and the game unpacks it into
//!   `Android/data/org.opensgolf.game/files/game` (gamefiles.rs, import.rs), then starts again ([`restart`]). No
//!   permission is needed: the picker grants access to what the player picked.
//!
//! Everything else here is plain JNI calls on miniquad's activity (miniquad's own `quad_native` Java glue).

use miniquad::native::android::{self, ndk_sys};
use std::ffi::{c_char, c_int, CStr, CString};
use std::io::{BufRead, BufReader};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
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

/// The panic kept for the next start to show (the internal data folder, which needs no file manager).
static PENDING_FILE: OnceLock<PathBuf> = OnceLock::new();

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
        if let Some(path) = PENDING_FILE.get() {
            let _ = std::fs::write(path, &text);
        }
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

/// `<external files folder>/game`: where "Use my SimGolf files..." puts the files the player picked (gamefiles.rs).
pub fn game_root() -> PathBuf {
    let base = dirs().external.clone().unwrap_or_else(|| PathBuf::from("/sdcard/Android/data/org.opensgolf.game/files"));
    base.join("game")
}

/// Where a player's copy of the original game goes: `<external files folder>/game`, which may hold the game's
/// `Program_Files_(ENGLISH)` folder (the whole installed folder copied over) or be that folder itself. The folder is
/// created, empty, so the player finds it with a file manager.
pub fn game_dir() -> PathBuf {
    let game = game_root();
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
    let _ = PENDING_FILE.set(files_dir().join("last_crash.txt"));
    println!("OpenSGolf {} starting: data folder {:?}, external folder {:?}", env!("CARGO_PKG_VERSION"), dirs.files, dirs.external);
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return;
        }
        pick_test_hook();
        let env = android::attach_jni_env();
        let mut vm: *mut ndk_sys::JavaVM = std::ptr::null_mut();
        // once per process: an activity made again in the same process (Android may do that) runs `main` again, and
        // ndk_context panics when set twice
        static CONTEXT_SET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if (**env).GetJavaVM.unwrap()(env, &mut vm) == 0 && !vm.is_null() && !CONTEXT_SET.swap(true, std::sync::atomic::Ordering::SeqCst) {
            ndk_context::initialize_android_context(vm as *mut _, activity as *mut _);
        }
    }
}

/// A Java string as a Rust one (None for null).
unsafe fn rust_string(env: *mut ndk_sys::JNIEnv, s: ndk_sys::jobject) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let chars = (**env).GetStringUTFChars.unwrap()(env, s, std::ptr::null_mut());
    if chars.is_null() {
        return None;
    }
    let out = CStr::from_ptr(chars).to_string_lossy().into_owned();
    (**env).ReleaseStringUTFChars.unwrap()(env, s, chars);
    (**env).DeleteLocalRef.unwrap()(env, s);
    Some(out)
}

/// True when the last JNI call threw (the exception is logged and cleared).
unsafe fn threw(env: *mut ndk_sys::JNIEnv) -> bool {
    if (**env).ExceptionCheck.unwrap()(env) == 0 {
        return false;
    }
    (**env).ExceptionDescribe.unwrap()(env);
    (**env).ExceptionClear.unwrap()(env);
    true
}

/// "Use my SimGolf files...": opens Android's own picker (MainActivity.pickGameFiles, vendor/miniquad/java), for the game
/// folder (`folder`, ACTION_OPEN_DOCUMENT_TREE) or a .zip of it (ACTION_OPEN_DOCUMENT). The Java side copies what was
/// picked into the app's folders on a thread of its own: a .zip to `import.zip` in the internal data folder, a folder to
/// `import` in the external files folder; [`import_state`] says how far it is. False when the picker could not be asked
/// for.
pub fn pick_game_files(folder: bool) -> bool {
    let dest = if folder { game_root().with_file_name("import") } else { files_dir().join("import.zip") };
    pick_to(folder, &dest)
}

/// Where Load Previous Game's *Import a save file...* has Android's picker copy the file picked ([`pick_save_file`]).
pub fn picked_save_path() -> PathBuf {
    files_dir().join("picked_save")
}

/// Load Previous Game's *Import a save file...* (files_ui.rs): the same picker as [`pick_game_files`] for one file, copied
/// to [`picked_save_path`]; [`import_state`] says when it is there ("zip:PATH"). Android does not hand the file's name
/// over, so the game tells what it is from its bytes.
pub fn pick_save_file() -> bool {
    pick_to(false, &picked_save_path())
}

fn pick_to(folder: bool, dest: &Path) -> bool {
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return false;
        }
        let env = android::attach_jni_env();
        let Ok(c) = CString::new(dest.to_string_lossy().into_owned()) else { return false };
        let jdest = (**env).NewStringUTF.unwrap()(env, c.as_ptr());
        miniquad::call_void_method!(env, activity, "pickGameFiles", "(ZLjava/lang/String;)V", folder as c_int, jdest);
        (**env).DeleteLocalRef.unwrap()(env, jdest);
        println!("asked Android's picker for {}", if folder { "a folder" } else { "a .zip" });
        !threw(env)
    }
}

/// How the import [`pick_game_files`] started is going (MainActivity.importState): "picking" (the picker is up),
/// "cancelled", "copying:FILES:BYTES", "zip:PATH" or "dir:PATH" (copied: the game goes on from there), "failed:WHY".
pub fn import_state() -> String {
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return "failed:no activity".into();
        }
        let env = android::attach_jni_env();
        let s = miniquad::call_object_method!(env, activity, "importState", "()Ljava/lang/String;");
        if threw(env) {
            return "failed:importState".into();
        }
        rust_string(env, s).unwrap_or_default()
    }
}

/// Starts the app again (MainActivity.restartApp): the original's files load from the start on the new run, as a desktop
/// program starts itself again (gamefiles.rs).
pub fn restart() {
    println!("starting the app again on the player's files");
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return;
        }
        let env = android::attach_jni_env();
        miniquad::call_void_method!(env, activity, "restartApp", "()V");
        threw(env);
    }
}

/// "Update now" (autoupdate.rs): hands the downloaded APK, its signature and checksum checked, to Android's package
/// installer (MainActivity.installApk: a PackageInstaller session, so no file needs sharing through a FileProvider).
/// Android asks the player to confirm, and installs it only over an app signed with the same key. False when the Java
/// call failed.
pub fn install_apk(path: &std::path::Path) -> bool {
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return false;
        }
        let env = android::attach_jni_env();
        let Ok(c) = CString::new(path.to_string_lossy().into_owned()) else { return false };
        let jpath = (**env).NewStringUTF.unwrap()(env, c.as_ptr());
        miniquad::call_void_method!(env, activity, "installApk", "(Ljava/lang/String;)V", jpath);
        (**env).DeleteLocalRef.unwrap()(env, jpath);
        println!("handed {} to Android's installer", path.display());
        !threw(env)
    }
}

/// How the install [`install_apk`] started is going (MainActivity.installState): "installing", "waiting", "confirm"
/// (Android's box is up), "done", or "failed:WHY".
pub fn install_state() -> String {
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return "failed:no activity".into();
        }
        let env = android::attach_jni_env();
        let s = miniquad::call_object_method!(env, activity, "installState", "()Ljava/lang/String;");
        if threw(env) {
            return "failed:installState".into();
        }
        rust_string(env, s).unwrap_or_default()
    }
}

/// "Check for updates" (update.rs): opens `url` in the phone's browser (`startActivity` with an ACTION_VIEW intent for
/// it). False when that failed (no browser, or a JNI call threw).
pub fn open_url(url: &str) -> bool {
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return false;
        }
        let env = android::attach_jni_env();
        let Ok(c) = CString::new(url) else { return false };
        let f = &**env;
        let uri_class = f.FindClass.unwrap()(env, c"android/net/Uri".as_ptr());
        if threw(env) || uri_class.is_null() {
            return false;
        }
        let parse = f.GetStaticMethodID.unwrap()(env, uri_class, c"parse".as_ptr(), c"(Ljava/lang/String;)Landroid/net/Uri;".as_ptr());
        if threw(env) || parse.is_null() {
            return false;
        }
        let jurl = f.NewStringUTF.unwrap()(env, c.as_ptr());
        let uri = f.CallStaticObjectMethod.unwrap()(env, uri_class, parse, jurl);
        f.DeleteLocalRef.unwrap()(env, jurl);
        if threw(env) || uri.is_null() {
            return false;
        }
        let intent_class = f.FindClass.unwrap()(env, c"android/content/Intent".as_ptr());
        if threw(env) || intent_class.is_null() {
            return false;
        }
        let init = f.GetMethodID.unwrap()(env, intent_class, c"<init>".as_ptr(), c"(Ljava/lang/String;Landroid/net/Uri;)V".as_ptr());
        if threw(env) || init.is_null() {
            return false;
        }
        let action = f.NewStringUTF.unwrap()(env, c"android.intent.action.VIEW".as_ptr());
        let intent = f.NewObject.unwrap()(env, intent_class, init, action, uri);
        f.DeleteLocalRef.unwrap()(env, action);
        f.DeleteLocalRef.unwrap()(env, uri);
        if threw(env) || intent.is_null() {
            return false;
        }
        miniquad::call_void_method!(env, activity, "startActivity", "(Landroid/content/Intent;)V", intent);
        let ok = !threw(env);
        f.DeleteLocalRef.unwrap()(env, intent);
        println!("opening {url} in the browser: {}", if ok { "asked" } else { "failed" });
        ok
    }
}

/// Test hook: the intent's `sg_pick` extra (`adb shell am start -n org.opensgolf.game/.MainActivity --es sg_pick PATH`)
/// is taken as what was picked in "Use my SimGolf files..." (a .zip or a folder the app can read), as SG_PICK_FOLDER does
/// on a desktop (gamefiles.rs). Read once, from `init`.
unsafe fn pick_test_hook() {
    let activity = android::ACTIVITY;
    let env = android::attach_jni_env();
    let intent = miniquad::call_object_method!(env, activity, "getIntent", "()Landroid/content/Intent;");
    if threw(env) || intent.is_null() {
        return;
    }
    let key = (**env).NewStringUTF.unwrap()(env, c"sg_pick".as_ptr());
    let v = miniquad::call_object_method!(env, intent, "getStringExtra", "(Ljava/lang/String;)Ljava/lang/String;", key);
    (**env).DeleteLocalRef.unwrap()(env, key);
    (**env).DeleteLocalRef.unwrap()(env, intent);
    if threw(env) {
        return;
    }
    if let Some(p) = rust_string(env, v).filter(|p| !p.is_empty()) {
        println!("test hook: {p} taken as picked");
        std::env::set_var(crate::gamefiles::PICK_ENV, p);
    }
}

extern "C" {
    fn __system_property_get(name: *const c_char, value: *mut c_char) -> c_int;
}

/// A system property (`ro.build.version.sdk` ...), empty when not set.
fn property(name: &CStr) -> String {
    // PROP_VALUE_MAX is 92
    let mut buf = [0 as c_char; 96];
    let n = unsafe { __system_property_get(name.as_ptr(), buf.as_mut_ptr()) };
    if n <= 0 {
        return String::new();
    }
    unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned()
}

/// The phone, for a report: maker, model, chip and Android version.
pub(crate) fn device() -> String {
    format!(
        "{} {} ({}), Android {} (API {}), {}",
        property(c"ro.product.manufacturer"),
        property(c"ro.product.model"),
        property(c"ro.soc.model"),
        property(c"ro.build.version.release"),
        property(c"ro.build.version.sdk"),
        std::env::consts::ARCH
    )
}

/// How the app's last run ended, when that was a crash (Android 11 and newer): Android's `ApplicationExitInfo` of the
/// last exit, as (its time in ms, a few lines for the report). None on older Android or after a normal end.
fn last_exit_crash() -> Option<(i64, String)> {
    const REASON_CRASH: i32 = 4;
    const REASON_CRASH_NATIVE: i32 = 5;
    if property(c"ro.build.version.sdk").parse::<i32>().unwrap_or(0) < 30 {
        return None;
    }
    unsafe {
        let activity = android::ACTIVITY;
        if activity.is_null() {
            return None;
        }
        let env = android::attach_jni_env();
        let failed = || {
            if (**env).ExceptionCheck.unwrap()(env) != 0 {
                (**env).ExceptionClear.unwrap()(env);
                true
            } else {
                false
            }
        };
        let name = (**env).NewStringUTF.unwrap()(env, c"activity".as_ptr());
        let am = miniquad::call_object_method!(env, activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", name);
        if failed() || am.is_null() {
            return None;
        }
        let null: ndk_sys::jobject = std::ptr::null_mut();
        let list = miniquad::call_object_method!(
            env,
            am,
            "getHistoricalProcessExitReasons",
            "(Ljava/lang/String;II)Ljava/util/List;",
            null,
            0 as c_int,
            1 as c_int
        );
        if failed() || list.is_null() {
            return None;
        }
        let n = miniquad::call_int_method!(env, list, "size", "()I");
        if failed() || n < 1 {
            return None;
        }
        let info = miniquad::call_object_method!(env, list, "get", "(I)Ljava/lang/Object;", 0 as c_int);
        if failed() || info.is_null() {
            return None;
        }
        let reason = miniquad::call_int_method!(env, info, "getReason", "()I");
        let status = miniquad::call_int_method!(env, info, "getStatus", "()I");
        let time: i64 = miniquad::call_method!(CallLongMethod, env, info, "getTimestamp", "()J");
        if failed() || (reason != REASON_CRASH && reason != REASON_CRASH_NATIVE) {
            return None;
        }
        let jdesc = miniquad::call_object_method!(env, info, "getDescription", "()Ljava/lang/String;");
        let desc = if failed() || jdesc.is_null() {
            String::new()
        } else {
            let chars = (**env).GetStringUTFChars.unwrap()(env, jdesc, std::ptr::null_mut());
            let d = CStr::from_ptr(chars).to_string_lossy().into_owned();
            (**env).ReleaseStringUTFChars.unwrap()(env, jdesc, chars);
            d
        };
        let kind = if reason == REASON_CRASH_NATIVE { format!("native crash (signal {status})") } else { "crash".to_string() };
        Some((
            time,
            format!(
                "last run ended by a {kind} at {time} ms (Unix time){}",
                if desc.is_empty() { String::new() } else { format!(": {desc}") }
            ),
        ))
    }
}

/// The report of a crash in the last run, once: the panic the hook kept (taken, so it is shown once), or else Android's
/// record of a crash without a panic (shown once per crash, by its time in settings.json). None after a normal end.
pub fn take_crash_report() -> Option<String> {
    let exit = last_exit_crash().filter(|(time, _)| crate::gamefiles::take_once("last_crash_seen", &time.to_string()));
    let panic = PENDING_FILE.get().and_then(|p| {
        let text = std::fs::read_to_string(p).ok();
        let _ = std::fs::remove_file(p);
        text
    });
    if panic.is_none() && exit.is_none() {
        return None;
    }
    let mut text = format!("OpenSGolf {} on {}\n", env!("CARGO_PKG_VERSION"), device());
    if let Some((_, e)) = exit {
        text += &e;
        text.push('\n');
    }
    if let Some(p) = panic {
        text.push('\n');
        text += &p;
    }
    println!("crash report from the last run:\n{text}");
    Some(text)
}
