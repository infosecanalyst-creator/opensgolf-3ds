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
//! Everything else here is plain JNI calls on miniquad's activity (miniquad's own `quad_native` Java glue).

use miniquad::native::android::{self, ndk_sys};
use std::ffi::CStr;
use std::path::PathBuf;
use std::sync::OnceLock;

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
pub fn init() {
    dirs();
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
