//! The simulation fingerprint of the whole game (sg_core::fingerprint; the port's own), and its test hook:
//! `SG_FINGERPRINT=FILE` writes one line per game tick, "TICK FINGERPRINT", so two runs of the same scripted game can be
//! compared tick by tick (a bug report reproduced exactly) and a saved game shown to go on exactly as it would have
//! (save fidelity). With `SG_FINGERPRINT_PARTS=1` each line also gives every section's own hash ("club=..."), which says
//! where two runs part, and with `SG_FINGERPRINT_PARTS=2` the club's, the economy's and the course's fields one by one
//! too. A save writes "TICK FINGERPRINT save" and a load "TICK FINGERPRINT load" (docs/PLAYING.md).
//!
//! The sections are what decides how the game plays on: the club (golfers, holes, members, roster, the pro, stories,
//! tournaments, wildlife, the gameplay extras), the course's tiles and flags, the terrain and land, the money, the
//! generators, the staff and their tiles (weeds and growth), the tick and calendar, the scenario's goals and scripts, and
//! the few things the game hands the simulation each tick (the pointer's tile, which the pro's employee follows before
//! the first hole opens, and the scripted runs' aiming). Camera, props, sprites, textures, sounds, the ticker, toasts,
//! the drawing's sprouts and Enhanced graphics are not in it.

use crate::app::App;
use sg_core::fingerprint::{club_unsaved, fields, hash, Fingerprint, Fnv64};

impl App {
    /// The fingerprint of the game as it stands.
    pub fn fingerprint(&self) -> Fingerprint {
        let mut f = Fingerprint::new();
        f.add("club", &self.club);
        f.add_hash("club_unsaved", club_unsaved(&self.club));
        f.add("course", &self.course);
        f.add("terrain", &self.terrain);
        f.add("land", &self.land);
        f.add("econ", &self.econ);
        f.add("econ_unsaved", &(self.econ.extras, self.econ.free));
        f.add("exe_rng", &self.exe_rng);
        // the cosmetic generator steps the celebrity residents, whose walk is the club's (resident_tick)
        f.add("deco_rng", &self.deco_rng);
        f.add("noise", &self.noise);
        f.add("employees", &self.employees);
        f.add("staff_tiles", &self.staff_tiles);
        f.add("staff_golfers", &self.staff_golfers);
        f.add("tick", &(self.game_tick, self.difficulty, self.course_id, &self.bought, &self.hole_numbers, self.adopt_holes));
        f.add("hole_stats", &self.hole_stats);
        f.add("scripts", &(&self.scen.goals, &self.scripts_for_save()));
        // what the game around the simulation hands it each tick: the pointer's tile while the pro's employee follows it
        // (until the first hole opens, staff::World::follow_pointer), the scripted runs' aiming (the A and j edit specs)
        // and the tiles retyped since the wildlife last looked
        let mut h = Fnv64::new();
        let follow = self.club.holes.get(1).is_some_and(|h| h.par == 0);
        let tile = self.hover_tile.filter(|_| follow);
        h.bool(tile.is_some());
        let (a, b) = tile.unwrap_or_default();
        h.i32(a);
        h.i32(b);
        h.u8(self.auto_aim);
        h.u64(hash(&self.retyped));
        f.add_hash("inputs", h.finish());
        f
    }

    /// The club's, the economy's and the course's fields one by one ("club.next_hole"), and each golfer field over all the
    /// golfers ("club.g.mood"): `SG_FINGERPRINT_PARTS=2` and the save self-check's report (selfcheck.rs).
    pub fn fingerprint_field_list(&self) -> Vec<(String, u64)> {
        self.fingerprint_field_list_with(&self.course)
    }

    /// `fingerprint_field_list` with another copy of the course (the save self-check's, selfcheck.rs).
    pub fn fingerprint_field_list_with(&self, course: &sg_core::course::Course) -> Vec<(String, u64)> {
        let mut out = Vec::new();
        for (part, list) in [("club", fields(&self.club)), ("econ", fields(&self.econ)), ("course", fields(course))] {
            for (k, v) in list {
                out.push((format!("{part}.{k}"), v));
            }
        }
        // each golfer field over all the golfer records
        let mut keys: Vec<(&'static str, Fnv64)> = Vec::new();
        for g in &self.club.g {
            for (i, (k, v)) in fields(g).into_iter().enumerate() {
                if keys.len() <= i {
                    keys.push((k, Fnv64::new()));
                }
                keys[i].1.u64(v);
            }
        }
        for (k, h) in keys {
            out.push((format!("club.g.{k}"), h.finish()));
        }
        out
    }

    /// `SG_FINGERPRINT_PARTS=2`: the fields after the sections.
    fn fingerprint_fields(&self) -> String {
        self.fingerprint_field_list().into_iter().map(|(k, v)| format!(" {k}={v:016x}")).collect()
    }

    /// Test hook: `SG_SAVE_AT=TICK,FILE` saves the game to FILE right after game tick TICK, inside the simulation's loop,
    /// so a scripted run can be stopped at an exact tick and the same game played on from the save (save fidelity).
    pub fn save_at_hook(&self) {
        static AT: std::sync::OnceLock<Option<(u32, std::path::PathBuf)>> = std::sync::OnceLock::new();
        let at = AT.get_or_init(|| {
            let v = std::env::var("SG_SAVE_AT").ok()?;
            let (t, f) = v.split_once(',')?;
            Some((t.trim().parse().ok()?, std::path::PathBuf::from(f)))
        });
        if let Some((t, f)) = at.as_ref().filter(|(t, _)| *t == self.game_tick) {
            match self.save_game(f) {
                Ok(()) => println!("saved game {} at tick {t}", f.display()),
                Err(e) => eprintln!("error: {e}"),
            }
        }
    }

    /// The test hook's line after a game tick (or a save or load, `tag`), when SG_FINGERPRINT names a file.
    pub fn fingerprint_note(&self, tag: &str) {
        log::note(|parts| {
            let mut s = format!("{} {}", self.game_tick, self.fingerprint().text(parts > 0));
            if parts > 1 {
                s.push_str(&self.fingerprint_fields());
            }
            if !tag.is_empty() {
                s.push(' ');
                s.push_str(tag);
            }
            s
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod log {
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    /// The file SG_FINGERPRINT names (opened once, written line by line so nothing is lost when the run ends with
    /// `process::exit`), and SG_FINGERPRINT_PARTS (0 the total only, 1 the sections, 2 the fields too).
    static OUT: OnceLock<Option<(Mutex<std::fs::File>, u8)>> = OnceLock::new();

    pub fn note(line: impl FnOnce(u8) -> String) {
        let out = OUT.get_or_init(|| {
            let path = std::env::var_os("SG_FINGERPRINT")?;
            let parts = std::env::var("SG_FINGERPRINT_PARTS").ok().map(|v| v.trim().parse().unwrap_or(1)).unwrap_or(0);
            match std::fs::File::create(&path) {
                Ok(f) => Some((Mutex::new(f), parts)),
                Err(e) => {
                    eprintln!("fingerprint: cannot write {}: {e}", std::path::Path::new(&path).display());
                    None
                }
            }
        });
        if let Some((f, parts)) = out {
            let mut s = line(*parts);
            s.push('\n');
            if let Ok(mut f) = f.lock() {
                let _ = f.write_all(s.as_bytes());
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod log {
    /// No files in a browser: the hook is for desktop test runs.
    pub fn note(_: impl FnOnce(u8) -> String) {}
}
