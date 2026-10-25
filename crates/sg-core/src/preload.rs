//! Sprites decoded ahead of need on another thread (the 3DS frontend runs `run` on a spare CPU core): the game loads a golfer
//! body's 50 clips the moment its first golfer arrives, which held the 3DS's main thread for about ten seconds. `take` hands
//! the game's loader (simgolf's `App::sprite_for`) a sprite already decoded; anything not ready yet it decodes itself as
//! before, and marks claimed so the worker skips it. Desktop builds never start the worker, so `take` always says no.

use crate::sprites::{load_sprite, Sprite};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

struct State {
    ready: HashMap<PathBuf, Result<Sprite, String>>,
    /// Paths the game has asked for (and so loads itself, or has taken): the worker leaves them alone.
    claimed: HashSet<PathBuf>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
/// Set to stop the worker at its next file (the program is quitting).
pub static STOP: AtomicBool = AtomicBool::new(false);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(|| State { ready: HashMap::new(), claimed: HashSet::new() }))
}

/// Decodes the sprites in order (path, is a shadow), skipping any the game has claimed. Runs until done or `STOP`.
pub fn run(jobs: Vec<(PathBuf, bool)>) {
    for (path, shadow) in jobs {
        if STOP.load(Ordering::Relaxed) {
            break;
        }
        if with(|s| s.claimed.contains(&path) || s.ready.contains_key(&path)) {
            continue;
        }
        let r = load_sprite(&path, shadow, None);
        with(|s| {
            if !s.claimed.contains(&path) {
                s.ready.insert(path, r);
            }
        });
    }
}

/// The sprite for `path` if the worker has it (the game then owns it); else None, and the path is claimed.
pub fn take(path: &Path) -> Option<Result<Sprite, String>> {
    with(|s| {
        s.claimed.insert(path.to_path_buf());
        s.ready.remove(path)
    })
}
