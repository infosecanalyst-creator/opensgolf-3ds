//! The play-time count (the port's own, none of it in the original; docs/PLAYING.md "Play-time count"): so the project can
//! see how many hours are played on each system and version, the game sends one anonymous count to the project's own
//! play-time counter ([`COUNT_URL`], a Cloudflare Worker: stats/README.md) for every 15 minutes of play, and one when it
//! starts:
//!
//! * `start/<platform>/<version>` once at start, e.g. `start/windows/0.6.0`;
//! * `play-15min/<platform>` for each 15 minutes of play time, e.g. `play-15min/linux-arm64`.
//!
//! The play time is the support reminder's clock (support.rs): real seconds on the course, not paused, not in the
//! background, not on the menus. Each count also carries the version, the count's number in this run and a run id: a random
//! number made at each start and never saved, so one run's counts can be told apart (how long people play at a time)
//! but never linked to a player or to another run. Nothing else is sent: no names, saved games or course data, and no
//! cookies. Builds before 0.8.2 counted with GoatCounter instead; the Worker imports those counts every week.
//! `<platform>` is windows, macos, linux, android or web, with `-arm64` on 64-bit ARM (but Android).
//!
//! It is on by default; the first time a course is played the ticker says so once, and Preferences > More > "Count my play
//! time (anonymous)" turns it off for good (kept in settings.json under [`SETTINGS_KEY`]). Builds for a store or a package
//! manager (`OPENSGOLF_NO_SELF_UPDATE=1`: the `opensgolf_store` cfg, build.rs) have none of it: no tick, no notice, nothing
//! sent, and no address in the program.
//!
//! Each count is one GET of the counter's address ([`count_url`]: `e` the event, `v` the version, `s` the run id or `kept`,
//! `n` the count's number in the run, `rnd` against caching): on the desktop and Android by ureq as the update check uses
//! it (net.rs), on a thread of its own with a short timeout; in the browser by the page (web/opensgolf.js sg_playcount,
//! which makes the run id too: sg_random; the page's own "Count my play time" tick off turns it off as the game's tick
//! does). On the desktop and Android a
//! 15-minute count that does not get through (no internet) is kept as a number in settings.json ([`Record::pending`], at
//! most [`MAX_PENDING`], a week's play) and sent with the next count that does, [`BATCH`] at a time a second apart:
//! nothing but the same counts, later; taking the tick off deletes them. The start count is not kept. Nothing is ever
//! shown about a failure.
//!
//! Never from tests or scripted runs: under `cargo test`, with --png, --time or --screen, or with any SG_* test hook
//! nothing is sent and the notice is not shown. Test hook: `SG_PLAYCOUNT_LOG=1` prints each count instead of sending it
//! (and shows the notice, without remembering it), with `SG_PLAYTIME=SECONDS` to start the clock near a mark.
// the address and the User-Agent go unused where nothing is sent from Rust (a store's build, the browser, no `net`)
#![cfg_attr(any(opensgolf_store, target_arch = "wasm32", not(feature = "net")), allow(dead_code))]
use serde::{Deserialize, Serialize};

/// Whether this build has the count at all (not a store's build).
pub const AVAILABLE: bool = cfg!(not(opensgolf_store));
/// The project's play-time counter (stats/: a Cloudflare Worker).
#[cfg(not(opensgolf_store))]
pub const COUNT_URL: &str = "https://opensgolf-stats.openmcm2-laps.workers.dev/count";
/// The run id of counts kept from an earlier run (they belong to none).
pub const KEPT_RUN: &str = "kept";
/// settings.json's key.
pub const SETTINGS_KEY: &str = "playcount";
/// The play time between two counts, in seconds.
pub const EVERY: f64 = 15.0 * 60.0;
/// The test hook's variable.
pub const LOG_ENV: &str = "SG_PLAYCOUNT_LOG";
/// At most this many unsent 15-minute counts are kept (a week of play).
pub const MAX_PENDING: u32 = 7 * 24 * 4;
/// Kept counts sent along with one that gets through, a second apart.
pub const BATCH: u32 = 16;
/// How long a count may take before it is dropped.
#[cfg(all(not(opensgolf_store), not(target_arch = "wasm32"), feature = "net"))]
const TIMEOUT_S: u64 = 10;

/// What is kept in settings.json.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    /// Preferences > More's tick was taken off: nothing is sent.
    pub off: bool,
    /// The one-time notice was shown.
    pub noticed: bool,
    /// 15-minute counts that did not get through (no internet), to be sent later: only their number is kept.
    pub pending: u32,
}

/// Counts that a sending thread gave back (they did not get through), for the frame to keep in [`Record::pending`].
static RETURNED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// `pending` after `back` more counts were given back, never above [`MAX_PENDING`].
pub fn keep(pending: u32, back: u32) -> u32 {
    pending.saturating_add(back).min(MAX_PENDING)
}

/// How many kept counts go out with the next one: at most [`BATCH`].
pub fn batch(pending: u32) -> u32 {
    pending.min(BATCH)
}

/// What this run does with the count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Nothing: a store's build, or the player turned it off.
    Off,
    /// A test or scripted run: nothing sent, no notice.
    Quiet,
    /// SG_PLAYCOUNT_LOG: each count printed instead of sent.
    Log,
    /// Counts are sent.
    Send,
}

/// The run's mode: `available` (the build has it), `off` (the player's tick), `log` (the test hook), `scripted` (a test or
/// scripted run).
pub fn mode(available: bool, off: bool, log: bool, scripted: bool) -> Mode {
    if !available || off {
        Mode::Off
    } else if log {
        Mode::Log
    } else if scripted {
        Mode::Quiet
    } else {
        Mode::Send
    }
}

/// The platform's name in the counts: `os` and `arch` as `std::env::consts` has them, `web` in the browser.
pub fn platform(os: &str, arch: &str, web: bool) -> String {
    if web {
        return "web".into();
    }
    let os = match os {
        "windows" | "macos" | "linux" | "android" => os,
        _ => "other",
    };
    if arch == "aarch64" && os != "android" {
        format!("{os}-arm64")
    } else {
        os.to_string()
    }
}

/// This build's platform name.
pub fn platform_here() -> String {
    platform(std::env::consts::OS, std::env::consts::ARCH, cfg!(target_arch = "wasm32"))
}

/// The count sent at start.
pub fn start_event(platform: &str, version: &str) -> String {
    format!("start/{platform}/{version}")
}

/// The count sent for each 15 minutes of play.
pub fn play_event(platform: &str) -> String {
    format!("play-15min/{platform}")
}

/// The first mark after the play time `played`: the next whole 15 minutes (time played before this run is not counted
/// again).
pub fn first_mark(played: f64) -> f64 {
    ((played.max(0.0) / EVERY).floor() + 1.0) * EVERY
}

/// Whether the play time `played` has reached the mark `next`; if so the mark moves to the next one after `played` (one
/// count however far the clock jumped).
pub fn reached(played: f64, next: &mut f64) -> bool {
    if played < *next {
        return false;
    }
    *next = first_mark(played);
    true
}

/// Whether the one-time notice is due on the course.
pub fn notice_due(mode: Mode, r: &Record) -> bool {
    matches!(mode, Mode::Send | Mode::Log) && !r.noticed
}

/// `s` as a query value (letters, digits and `-._~` kept, the rest as %XX).
pub fn query_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The test hook's variable for a stand-in counter on this computer (tools/playcount_test.sh); not an SG_* hook, so the
/// game counts as it would for a player.
pub const TEST_URL_ENV: &str = "OPENSGOLF_COUNT_TEST_URL";

/// A stand-in counter's address from [`TEST_URL_ENV`]: only one on this computer (`http://127.0.0.1` or
/// `http://localhost`), so counts can never be sent anywhere else.
pub fn local_test_url(v: Option<&str>) -> Option<String> {
    let v = v?.trim();
    let local = ["http://127.0.0.1:", "http://127.0.0.1/", "http://localhost:", "http://localhost/"];
    local.iter().any(|p| v.starts_with(p)).then(|| v.to_string())
}

/// The address that counts `event` of `version`, count `n` of the run `run` ([`KEPT_RUN`] for kept counts); `rnd`
/// against caching.
#[cfg(not(opensgolf_store))]
pub fn count_url(event: &str, version: &str, run: &str, n: u32, rnd: u64) -> String {
    let e = query_value(event);
    let (v, s) = (query_value(version), query_value(run));
    #[cfg(not(target_arch = "wasm32"))]
    let base = local_test_url(std::env::var(TEST_URL_ENV).ok().as_deref()).unwrap_or_else(|| COUNT_URL.to_string());
    #[cfg(target_arch = "wasm32")]
    let base = COUNT_URL;
    format!("{base}?e={e}&v={v}&s={s}&n={n}&rnd={rnd:x}")
}

/// The User-Agent of a count, e.g. "OpenSGolf/0.6.0 (linux)".
pub fn user_agent(version: &str, platform: &str) -> String {
    format!("OpenSGolf/{version} ({platform})")
}

/// 64 random bits as 16 hex digits: the run id, made at each start and never saved.
pub fn run_id_hex(bits: u64) -> String {
    format!("{bits:016x}")
}

/// A fresh run id: the standard library's random hash keys (and the clock) on the desktop and Android, the browser's
/// crypto.getRandomValues in the browser (web/opensgolf.js sg_random).
pub fn new_run_id() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    let bits = {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        h.write_u32(std::process::id());
        h.finish()
    };
    #[cfg(all(target_arch = "wasm32", not(opensgolf_store)))]
    let bits = {
        #[link(wasm_import_module = "env")]
        extern "C" {
            /// 32 random bits from the browser (web/opensgolf.js).
            fn sg_random() -> u32;
        }
        // SAFETY: no arguments
        unsafe { (u64::from(sg_random()) << 32) | u64::from(sg_random()) }
    };
    #[cfg(all(target_arch = "wasm32", opensgolf_store))]
    let bits = 0u64;
    run_id_hex(bits)
}

/// The play-time count's state (`App::playcount`).
#[derive(Debug)]
pub struct State {
    pub record: Record,
    /// The mode with the tick on (the tick can be taken off and put back while the game runs).
    base: Mode,
    /// The next play time that sends a count.
    next: f64,
    /// The browser page's own "Count my play time" tick is off (web/index.html #playcount): off as with the game's tick.
    page_off: bool,
    platform: String,
    /// This run's id ([`new_run_id`]; empty until a count is sent).
    run: String,
    /// 15-minute counts sent (or tried) in this run: the next one's number less one.
    marks: u32,
}

impl Default for State {
    fn default() -> State {
        State {
            record: Record::default(),
            base: Mode::Off,
            next: EVERY,
            page_off: false,
            platform: String::new(),
            run: String::new(),
            marks: 0,
        }
    }
}

impl State {
    /// The mode in use.
    pub fn mode(&self) -> Mode {
        mode(AVAILABLE, self.record.off || self.page_off, self.base == Mode::Log, self.base == Mode::Quiet)
    }

    /// Preferences > More's tick.
    pub fn on(&self) -> bool {
        !self.record.off
    }
}

/// settings.json in the save folder.
fn settings_path() -> std::path::PathBuf {
    let mut p = crate::app::save_dir();
    p.push(crate::hd::SETTINGS_FILE);
    p
}

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(settings_path())
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// Saves the record into settings.json, keeping its other keys.
fn save(r: &Record) {
    let mut v = settings_json();
    if let Ok(e) = serde_json::to_value(r) {
        v[SETTINGS_KEY] = e;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(settings_path(), t.as_bytes());
    }
}

/// Whether a test hook is set (any SG_* variable but the log's own). The browser has no environment (reading it panics
/// there), so never there.
fn test_hooks() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    return std::env::vars_os().any(|(k, _)| k.to_str().is_some_and(|k| k.starts_with("SG_") && k != LOG_ENV));
    #[cfg(target_arch = "wasm32")]
    false
}

/// The log test hook (`SG_PLAYCOUNT_LOG`), never in the browser (see `test_hooks`).
fn log_hook() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    return std::env::var_os(LOG_ENV).is_some_and(|v| !v.is_empty() && v != "0");
    #[cfg(target_arch = "wasm32")]
    false
}

/// At start up, after the support reminder's clock (`played`): the saved record, the run's mode (`scripted`: --png,
/// --time or --screen), and the start count.
pub fn init(played: f64, scripted: bool) -> State {
    let mut record: Record = serde_json::from_value(settings_json()[SETTINGS_KEY].clone()).unwrap_or_default();
    // an edited or damaged settings.json never asks for more than a week of kept counts; none when the tick is off
    record.pending = if record.off { 0 } else { record.pending.min(MAX_PENDING) };
    let log = log_hook();
    let scripted = scripted || cfg!(test) || test_hooks();
    let base = mode(AVAILABLE, false, log, scripted);
    let mut s =
        State { record, base, next: first_mark(played), page_off: page_off(), platform: platform_here(), run: String::new(), marks: 0 };
    if log {
        println!("play count: {:?} (platform {}, next count at {:.0} s of play)", s.mode(), s.platform, s.next);
    }
    let start = start_event(&s.platform, crate::update::VERSION);
    s.send(&start, false);
    s
}

impl State {
    /// Sends `event` (Send), prints it (Log), or nothing; with up to [`BATCH`] kept counts after it. `play`: a 15-minute
    /// count, kept for later if it does not get through.
    fn send(&mut self, event: &str, play: bool) {
        let mode = self.mode();
        if !matches!(mode, Mode::Send | Mode::Log) {
            return;
        }
        if self.run.is_empty() {
            self.run = new_run_id();
        }
        let n = if play {
            self.marks += 1;
            self.marks
        } else {
            0
        };
        // the browser keeps nothing: its page is online to have loaded at all, and it tells the game nothing back
        let later = if cfg!(target_arch = "wasm32") { 0 } else { batch(self.record.pending) };
        if later > 0 {
            self.record.pending -= later;
            if mode == Mode::Send {
                save(&self.record);
            }
        }
        match mode {
            Mode::Log => println!(
                "play count: would send {event} (run {}, count {n}){}",
                self.run,
                if later > 0 { format!(" and {later} kept") } else { String::new() }
            ),
            _ => send_now(event, play, later, &self.platform, &self.run, n),
        }
    }
}

/// The browser page's own tick is off (web/opensgolf.js sg_playcount_off); never elsewhere.
fn page_off() -> bool {
    #[cfg(all(not(opensgolf_store), target_arch = "wasm32"))]
    {
        #[link(wasm_import_module = "env")]
        extern "C" {
            /// 1 when the page's "Count my play time" tick is off (web/opensgolf.js).
            fn sg_playcount_off() -> i32;
        }
        // SAFETY: no arguments; the page reads its own storage
        unsafe { sg_playcount_off() != 0 }
    }
    #[cfg(not(all(not(opensgolf_store), target_arch = "wasm32")))]
    false
}

/// The count on its way, then `later` kept 15-minute counts: a thread's GETs on the desktop and Android (what does not get
/// through is given back to [`RETURNED`]: the event if `play`, and the kept counts not yet sent), the page's fetch in the
/// browser. `run` and `n`: the run id and the count's number in it.
#[allow(unused_variables)]
fn send_now(event: &str, play: bool, later: u32, platform: &str, run: &str, n: u32) {
    #[cfg(all(not(opensgolf_store), not(target_arch = "wasm32"), feature = "net"))]
    {
        let event = event.to_string();
        let run = run.to_string();
        let kept = play_event(platform);
        let agent_name = user_agent(crate::update::VERSION, platform);
        // named: a panic in it ends only the count (android.rs); the answer is read only for whether it got through
        let spawned = std::thread::Builder::new().name("opensgolf-count".into()).spawn(move || {
            use std::sync::atomic::Ordering;
            let agent: ureq::Agent =
                ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(TIMEOUT_S))).build().into();
            let get = |e: &str, run: &str, n: u32| {
                let rnd = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
                let url = count_url(e, crate::update::VERSION, run, n, rnd);
                agent.get(&url).header("User-Agent", &agent_name).call().is_ok()
            };
            if !get(&event, &run, n) {
                RETURNED.fetch_add(u32::from(play) + later, Ordering::Relaxed);
                return;
            }
            for i in 0..later {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if !get(&kept, KEPT_RUN, 0) {
                    RETURNED.fetch_add(later - i, Ordering::Relaxed);
                    return;
                }
            }
        });
        if spawned.is_err() {
            RETURNED.fetch_add(u32::from(play) + later, std::sync::atomic::Ordering::Relaxed);
        }
    }
    #[cfg(all(not(opensgolf_store), target_arch = "wasm32"))]
    {
        #[link(wasm_import_module = "env")]
        extern "C" {
            /// Sends a count: the page fetches the address (web/opensgolf.js).
            fn sg_playcount(ptr: *const u8, len: usize);
        }
        let url = count_url(event, crate::update::VERSION, run, n, u64::from(n));
        // SAFETY: the page reads `len` bytes at `ptr`, which live for the call
        unsafe { sg_playcount(url.as_ptr(), url.len()) };
    }
}

impl crate::app::App {
    /// Once a frame, after the play time is counted: a count at each 15 minutes of play, and the one-time notice the first
    /// time the course is played.
    pub fn playcount_frame(&mut self) {
        let mode = self.playcount.mode();
        if matches!(mode, Mode::Off | Mode::Quiet) {
            return;
        }
        let played = self.support.record.played;
        let p = &mut self.playcount;
        // counts that did not get through are kept for later (the tick off throws them away)
        let back = RETURNED.swap(0, std::sync::atomic::Ordering::Relaxed);
        if back > 0 && mode == Mode::Send {
            p.record.pending = keep(p.record.pending, back);
            save(&p.record);
        }
        if reached(played, &mut p.next) {
            let e = play_event(&p.platform);
            p.send(&e, true);
            // the mark is kept with the clock at once, so a restart soon after never sends it again
            if mode == Mode::Send {
                self.support_save_now();
            }
        }
        let quiet = self.popup.is_none() && self.notice.is_none() && self.ui_ok;
        if self.screen == crate::app::Screen::Play && quiet && notice_due(mode, &self.playcount.record) {
            self.playcount.record.noticed = true;
            if mode == Mode::Send {
                save(&self.playcount.record);
            } else {
                println!("play count: notice shown (not remembered: {LOG_ENV})");
            }
            self.show_toast(sg_core::lang::tr("STR_PLAYCOUNT_NOTICE"));
        }
    }

    /// Preferences > More's tick: off stops every count (and the start count of later runs); saved.
    pub fn set_playcount(&mut self, on: bool) {
        if !AVAILABLE || self.playcount.on() == on {
            return;
        }
        self.playcount.record.off = !on;
        // the notice would only say what the player has just seen in Preferences
        self.playcount.record.noticed = true;
        // off: counts kept for later are thrown away, never sent
        if !on {
            self.playcount.record.pending = 0;
            RETURNED.store(0, std::sync::atomic::Ordering::Relaxed);
        }
        save(&self.playcount.record);
        if self.playcount.base == Mode::Log {
            println!("play count: {}", if on { "on" } else { "off" });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_every_15_minutes_of_play() {
        let mut next = first_mark(0.0);
        assert_eq!(next, EVERY);
        let mut sent = 0;
        let mut t = 0.0;
        // an hour of play in 1/60 s frames: four counts, each at its mark
        while t < 3600.0 + 0.5 {
            if reached(t, &mut next) {
                sent += 1;
                assert!((t - sent as f64 * EVERY).abs() < 0.1, "at {t}");
            }
            t += 1.0 / 60.0;
        }
        assert_eq!(sent, 4);
        // time played before this run is not counted again; a jump counts once
        let mut next = first_mark(20_000.0);
        assert_eq!(next, 20_700.0);
        assert!(!reached(20_699.0, &mut next));
        assert!(reached(30_000.0, &mut next));
        assert_eq!(next, 30_600.0);
        assert!(!reached(30_001.0, &mut next));
    }

    #[test]
    fn nothing_when_off_scripted_or_in_a_store_build() {
        assert_eq!(mode(true, false, false, false), Mode::Send);
        assert_eq!(mode(true, true, false, false), Mode::Off, "the tick off");
        assert_eq!(mode(true, true, true, false), Mode::Off, "the tick off, the log too");
        assert_eq!(mode(false, false, false, false), Mode::Off, "a store build");
        assert_eq!(mode(false, false, true, true), Mode::Off);
        assert_eq!(mode(true, false, false, true), Mode::Quiet, "a test or scripted run");
        assert_eq!(mode(true, false, true, true), Mode::Log, "the test hook");
        // this build's AVAILABLE follows the store build's switch
        assert_eq!(AVAILABLE, !crate::update::STORE_BUILD);
        // under cargo test a run never sends
        let s = State { base: mode(AVAILABLE, false, false, cfg!(test)), ..Default::default() };
        assert_ne!(s.mode(), Mode::Send);
        // the tick: off is Off, back on is the run's mode again
        let mut s = State { base: Mode::Send, ..Default::default() };
        assert_eq!(s.mode(), if AVAILABLE { Mode::Send } else { Mode::Off });
        s.record.off = true;
        assert_eq!(s.mode(), Mode::Off);
        assert!(!s.on());
        // the browser page's own tick off is off too, whatever the game's tick
        let s = State { base: Mode::Send, page_off: true, ..Default::default() };
        assert_eq!(s.mode(), Mode::Off);
        assert!(!notice_due(s.mode(), &s.record));
    }

    #[test]
    fn the_notice_once_and_only_where_counts_go() {
        let mut r = Record::default();
        assert!(notice_due(Mode::Send, &r));
        assert!(notice_due(Mode::Log, &r));
        assert!(!notice_due(Mode::Quiet, &r), "not in stills or tests");
        assert!(!notice_due(Mode::Off, &r));
        r.noticed = true;
        assert!(!notice_due(Mode::Send, &r), "once");
        // it reads back from settings.json, and old settings read as on and not yet shown
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(serde_json::from_value::<Record>(v).unwrap(), r);
        let none: Record = serde_json::from_value(serde_json::json!({})[SETTINGS_KEY].clone()).unwrap_or_default();
        assert_eq!(none, Record { off: false, noticed: false, pending: 0 });
    }

    #[test]
    fn counts_that_do_not_get_through_are_kept_a_week_at_most() {
        assert_eq!(keep(0, 1), 1);
        assert_eq!(keep(5, 17), 22);
        assert_eq!(keep(MAX_PENDING - 1, 5), MAX_PENDING, "a week of play at most");
        assert_eq!(keep(u32::MAX, 1), MAX_PENDING);
        assert_eq!(batch(0), 0);
        assert_eq!(batch(3), 3);
        assert_eq!(batch(200), BATCH);
        // old settings without the number read as none kept
        let r: Record = serde_json::from_value(serde_json::json!({"off": false, "noticed": true})).unwrap();
        assert_eq!(r.pending, 0);
    }

    #[test]
    fn a_stand_in_counter_only_on_this_computer() {
        assert_eq!(local_test_url(Some("http://127.0.0.1:8770/count")).as_deref(), Some("http://127.0.0.1:8770/count"));
        assert_eq!(local_test_url(Some("http://localhost:9/c")).as_deref(), Some("http://localhost:9/c"));
        assert_eq!(local_test_url(Some("https://example.com/count")), None);
        assert_eq!(local_test_url(Some("http://127.0.0.1.example.com/count")), None);
        assert_eq!(local_test_url(Some("http://localhost.example.com/")), None);
        assert_eq!(local_test_url(None), None);
    }

    #[test]
    fn what_is_sent() {
        assert_eq!(platform("windows", "x86_64", false), "windows");
        assert_eq!(platform("windows", "aarch64", false), "windows-arm64");
        assert_eq!(platform("macos", "aarch64", false), "macos-arm64");
        assert_eq!(platform("linux", "x86_64", false), "linux");
        assert_eq!(platform("linux", "aarch64", false), "linux-arm64");
        assert_eq!(platform("android", "aarch64", false), "android");
        assert_eq!(platform("", "wasm32", true), "web");
        assert_eq!(platform("freebsd", "x86_64", false), "other");
        assert_eq!(start_event("linux", "0.6.0"), "start/linux/0.6.0");
        assert_eq!(play_event("macos-arm64"), "play-15min/macos-arm64");
        assert_eq!(query_value("start/linux/0.6.0-rc1+x"), "start%2Flinux%2F0.6.0-rc1%2Bx");
        assert_eq!(user_agent("0.6.0", "linux"), "OpenSGolf/0.6.0 (linux)");
        #[cfg(not(opensgolf_store))]
        assert_eq!(
            count_url("play-15min/linux", "0.8.2", "00ff00ff00ff00ff", 3, 255),
            format!("{COUNT_URL}?e=play-15min%2Flinux&v=0.8.2&s=00ff00ff00ff00ff&n=3&rnd=ff")
        );
        assert!(count_url("play-15min/web", "0.8.2", KEPT_RUN, 0, 1).contains("&s=kept&n=0&"));
        // a run id is 16 hex digits (the counter takes 8 to 32), different each run
        assert_eq!(run_id_hex(0xab), "00000000000000ab");
        let (a, b) = (new_run_id(), new_run_id());
        assert_eq!(a.len(), 16);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_ne!(a, b);
    }
}
