//! Sprites decoded ahead of need on another thread (the 3DS frontend runs `run` on a spare CPU core): the game loads a golfer
//! body's 50 clips the moment its first golfer arrives, and a course's trees, buildings and staff when it opens, which held
//! the 3DS's main thread for many seconds. `take` hands the game's loader (simgolf's `App::sprite_for`) a sprite already
//! decoded; anything not ready yet it decodes itself as before, and marks claimed so the worker skips it. `asked` lists what
//! the game has asked for, in order, so a frontend can keep the list and decode those first next time. Desktop builds never
//! start the worker, so `take` always says no.

use crate::sprites::{load_sprite, Sprite};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// A sprite to decode: its file, whether it is a shadow, and the palette file it is drawn with (if not its own).
pub type Job = (PathBuf, bool, Option<PathBuf>);

type Key = (PathBuf, Option<PathBuf>);

struct State {
    ready: HashMap<Key, Result<Sprite, String>>,
    /// Sprites the game has asked for (and so loads itself, or has taken): the worker leaves them alone.
    claimed: HashSet<Key>,
    /// The same, in the order asked.
    asked: Vec<Job>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
/// Set to stop the worker at its next file (the program is quitting).
pub static STOP: AtomicBool = AtomicBool::new(false);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(|| State { ready: HashMap::new(), claimed: HashSet::new(), asked: Vec::new() }))
}

/// Decodes the sprites in order, skipping any the game has claimed or already queued twice. Runs until done or `STOP`.
pub fn run(jobs: Vec<Job>) {
    for (path, shadow, pal) in jobs {
        if STOP.load(Ordering::Relaxed) {
            break;
        }
        let key = (path, pal);
        if with(|s| s.claimed.contains(&key) || s.ready.contains_key(&key)) {
            continue;
        }
        let r = load_sprite(&key.0, shadow, key.1.as_deref());
        with(|s| {
            if !s.claimed.contains(&key) {
                s.ready.insert(key, r);
            }
        });
    }
}

/// The sprite for `path` (with palette `pal`) if the worker has it (the game then owns it); else None, and it is claimed.
pub fn take(path: &Path, shadow: bool, pal: Option<&Path>) -> Option<Result<Sprite, String>> {
    with(|s| {
        let key = (path.to_path_buf(), pal.map(Path::to_path_buf));
        if s.claimed.insert(key.clone()) {
            s.asked.push((key.0.clone(), shadow, key.1.clone()));
        }
        s.ready.remove(&key)
    })
}

/// Every sprite the game has asked for so far, in order.
pub fn asked() -> Vec<Job> {
    with(|s| s.asked.clone())
}
