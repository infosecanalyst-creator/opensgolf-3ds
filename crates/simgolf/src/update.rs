//! "Check for updates" on the title menu (the port's own; GitHub issue #28).
//!
//! Only a click on the button goes online: one request for OpenSGolf's latest release on GitHub
//! ([`LATEST_API`], the release's tag and nothing about the player is sent beyond what any HTTPS request carries), run in
//! the background by the content downloads' client (net.rs). The answer says the game is up to date, or which version is
//! out with a button that opens the release page ([`RELEASES_PAGE`]) in the system's browser; nothing is downloaded or
//! replaced by the game itself. Desktop and Android builds with the `net` feature have the button; the browser version is
//! always the newest, and a build without `net` cannot ask, so neither shows it.

use crate::app::App;
use sg_core::lang::{tr, trf};

/// GitHub's API answer for the newest release (not a draft or a prerelease); its `tag_name` is like "v0.3.1".
pub const LATEST_API: &str = "https://api.github.com/repos/crspytopgn/opensgolf/releases/latest";
/// The newest release's page, with its downloads.
pub const RELEASES_PAGE: &str = "https://github.com/crspytopgn/opensgolf/releases/latest";
/// This build's version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The API's answer is a few KB (the release notes and the list of files); anything far larger is not it.
const MAX_REPLY: u64 = 1 << 20;

/// The test hook's variable (App::check_for_updates).
pub const TEST_URL_ENV: &str = "SG_UPDATE_URL";

/// The title button's hover value (after gamefiles::FILES_HOVER).
pub const UPDATE_HOVER: i32 = 8;

/// True where the button is offered: a build that can make HTTPS requests itself, not the browser version.
pub const AVAILABLE: bool = cfg!(all(not(target_arch = "wasm32"), feature = "net"));

/// A version as compared: major, minor, patch, then true for a release (a prerelease such as 0.4.0-rc1 comes before
/// 0.4.0).
pub type Version = (u64, u64, u64, bool);

/// Reads "0.3.1", "v0.3.1", "0.4" or "0.4.0-rc1" (build metadata after '+' is ignored); None when it is not a version.
pub fn parse_version(s: &str) -> Option<Version> {
    let s = s.trim();
    let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
    let s = s.split('+').next().unwrap_or("");
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) if !p.is_empty() => (c, true),
        Some(_) => return None,
        None => (s, false),
    };
    let mut parts = [0u64; 3];
    let mut n = 0;
    for p in core.split('.') {
        if n == 3 || p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        parts[n] = p.parse().ok()?;
        n += 1;
    }
    (n >= 2).then_some((parts[0], parts[1], parts[2], !pre))
}

/// The `tag_name` of the API's answer.
pub fn latest_tag(body: &[u8]) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_slice(body).map_err(|e| format!("not a release ({e})"))?;
    v.get("tag_name").and_then(|t| t.as_str()).map(str::to_string).ok_or_else(|| "no release tag in the answer".to_string())
}

/// What the check found.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// This version is the newest (or newer than the newest release: a build from the sources).
    Latest,
    /// A newer version is out (its number, without the "v").
    Newer(String),
    /// The check failed (why).
    Failed(String),
}

/// The API's answer (or why there is none) against the version `current`.
pub fn outcome(reply: Result<Vec<u8>, String>, current: &str) -> Outcome {
    let tag = match reply.and_then(|b| latest_tag(&b)) {
        Ok(t) => t,
        Err(e) => return Outcome::Failed(e),
    };
    let Some(latest) = parse_version(&tag) else { return Outcome::Failed(format!("unknown release tag \"{tag}\"")) };
    match parse_version(current) {
        Some(cur) if cur >= latest => Outcome::Latest,
        _ => Outcome::Newer(tag.trim().trim_start_matches(['v', 'V']).to_string()),
    }
}

/// The title button: top right, where the original's art and the free set's leave the background bare (the files button
/// and its hint are at the top left).
pub fn update_button_rect(label: &str) -> crate::render::Rect {
    let w = crate::ui::F_MANUAL15.width(label).max(120.0) + 24.0;
    crate::render::Rect::new(792.0 - w, 6.0, w, 24.0)
}

/// Opens `url` in the system's browser; false when that could not be started.
pub fn open_url(url: &str) -> bool {
    #[cfg(target_os = "android")]
    {
        crate::android::open_url(url)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = url;
        false
    }
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    {
        let mut cmd = if cfg!(target_os = "windows") {
            // the shell's own handler for an address (no console window, unlike `cmd /c start`)
            let mut c = std::process::Command::new("rundll32");
            c.args(["url.dll,FileProtocolHandler", url]);
            c
        } else if cfg!(target_os = "macos") {
            let mut c = std::process::Command::new("open");
            c.arg(url);
            c
        } else {
            let mut c = std::process::Command::new("xdg-open");
            c.arg(url);
            c
        };
        cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        match cmd.spawn() {
            // reaped on a thread of its own so no zombie is left behind
            Ok(mut child) => {
                let _ = std::thread::Builder::new().name("opensgolf-open".into()).spawn(move || child.wait());
                true
            }
            Err(e) => {
                eprintln!("open {url}: {e}");
                false
            }
        }
    }
}

impl App {
    /// The button's label: what it does, or that the check is under way.
    fn update_label(&self) -> &'static str {
        if self.update_check.is_some() {
            tr("STR_UPDATE_CHECKING_BUTTON")
        } else {
            tr("STR_UPDATE_CHECK")
        }
    }

    /// The title button's rectangle (None where there is no button).
    pub fn update_button(&self) -> Option<crate::render::Rect> {
        AVAILABLE.then(|| update_button_rect(self.update_label()))
    }

    /// Draws the title button, in the files button's style.
    pub fn draw_update_button(&self, g: &mut crate::gfx::Gfx, s: &crate::ui::Screen) {
        use crate::ui::{rgb, rgba};
        let Some(r) = self.update_button() else { return };
        let lit = self.hover == UPDATE_HOVER;
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, rgba(1.0, 1.0, 1.0, 0.85));
        s.fill(g, r.x, r.y, r.w, r.h, if lit { rgba(0.13, 0.32, 0.55, 0.95) } else { rgba(0.08, 0.16, 0.32, 0.88) });
        let label = self.update_label();
        s.put_centered(g, crate::ui::F_MANUAL15, r.x + r.w / 2.0, r.y + 4.0, label, rgb(1.0, 1.0, if lit { 1.0 } else { 0.8 }));
    }

    /// A click on the button: the request starts (once at a time). Test hook: `SG_UPDATE_URL=FILE` asks FILE (a saved
    /// answer, e.g. `file:///tmp/latest.json`) instead of GitHub, and starts the check at once when the game starts.
    pub fn check_for_updates(&mut self) {
        if !AVAILABLE || self.update_check.is_some() {
            return;
        }
        let url = std::env::var(TEST_URL_ENV).unwrap_or_else(|_| LATEST_API.to_string());
        println!("checking for updates: {url}");
        self.update_check = Some(crate::net::get(&url, MAX_REPLY));
    }

    /// Each frame: the answer, once it is in, in the game's box.
    pub fn poll_update_check(&mut self) {
        let Some(reply) = self.update_check.as_mut().and_then(|r| r.poll()) else { return };
        self.update_check = None;
        match outcome(reply, VERSION) {
            Outcome::Latest => self.show_notice(&trf("STR_UPDATE_LATEST", &[("VERSION", &VERSION)])),
            Outcome::Newer(v) => {
                println!("update: OpenSGolf {v} is out (this is {VERSION})");
                let mut lines = crate::gamefiles::wrap(&trf("STR_UPDATE_NEWER", &[("VERSION", &v), ("CURRENT", &VERSION)]), 560.0);
                lines.push(format!(" {}", tr("STR_UPDATE_OPEN_PAGE")));
                lines.push(format!(" {}", tr("STR_GAME_FILES_NOT_NOW")));
                self.notice = Some(crate::gamefiles::Notice {
                    lines,
                    confirm: Some(crate::gamefiles::Action::OpenReleases),
                    alt: None,
                    sel: Some(0),
                });
            }
            Outcome::Failed(e) => {
                eprintln!("update check: {e}");
                // the address is not news to the player (net.rs puts it first)
                let e = e.split_once(": ").filter(|(url, _)| url.contains("://")).map_or(e.as_str(), |(_, why)| why);
                self.show_notice(&trf("STR_UPDATE_FAILED", &[("ERROR", &e)]));
            }
        }
    }

    /// "Open the download page": the release page in the browser, or where it is when no browser could be opened.
    pub fn open_releases_page(&mut self) {
        if !open_url(RELEASES_PAGE) {
            self.show_notice(&trf("STR_UPDATE_NO_BROWSER", &[("URL", &RELEASES_PAGE)]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(tag: &str) -> Result<Vec<u8>, String> {
        Ok(format!(r#"{{"url":"x","tag_name":"{tag}","name":"OpenSGolf","prerelease":false,"assets":[]}}"#).into_bytes())
    }

    #[test]
    fn versions_are_read() {
        assert_eq!(parse_version("0.3.1"), Some((0, 3, 1, true)));
        assert_eq!(parse_version("v0.3.1"), Some((0, 3, 1, true)));
        assert_eq!(parse_version(" V1.2 "), Some((1, 2, 0, true)));
        assert_eq!(parse_version("v0.4.0-rc1"), Some((0, 4, 0, false)));
        assert_eq!(parse_version("0.4.0+build7"), Some((0, 4, 0, true)));
        for bad in ["", "v", "latest", "0", "0.3.x", "1.2.3.4", "0..1", "0.3.1-", "-0.3.1", "v 0.3.1", "0.3.1a"] {
            assert_eq!(parse_version(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn versions_compare_as_numbers() {
        let v = |s| parse_version(s).unwrap();
        assert!(v("0.3.10") > v("0.3.9"));
        assert!(v("0.10.0") > v("0.9.99"));
        assert!(v("1.0.0") > v("0.99.99"));
        assert!(v("v0.3.1") == v("0.3.1"));
        assert!(v("0.4.0") > v("0.4.0-rc1"));
        assert!(v("0.4.0-rc1") > v("0.3.9"));
    }

    #[test]
    fn the_answer_is_judged() {
        assert_eq!(outcome(reply("v0.3.1"), "0.3.1"), Outcome::Latest);
        assert_eq!(outcome(reply("v0.3.0"), "0.3.1"), Outcome::Latest, "a build newer than the release");
        assert_eq!(outcome(reply("v0.3.10"), "0.3.9"), Outcome::Newer("0.3.10".into()));
        assert_eq!(outcome(reply("0.4.0"), "0.3.1"), Outcome::Newer("0.4.0".into()));
        assert_eq!(outcome(reply("v0.4.0-beta"), "0.3.1"), Outcome::Newer("0.4.0-beta".into()));
        assert_eq!(outcome(reply("v0.3.1-rc2"), "0.3.1"), Outcome::Latest);
        assert_eq!(outcome(reply("v0.3.2"), "0.3.2-rc1"), Outcome::Newer("0.3.2".into()));
        // failures: no answer, not JSON, no tag, a tag that is not a version
        assert!(matches!(outcome(Err("offline".into()), "0.3.1"), Outcome::Failed(e) if e == "offline"));
        assert!(matches!(outcome(Ok(b"<html>rate limited</html>".to_vec()), "0.3.1"), Outcome::Failed(_)));
        assert!(matches!(outcome(Ok(b"{\"tag_name\":".to_vec()), "0.3.1"), Outcome::Failed(_)));
        assert!(matches!(outcome(Ok(br#"{"message":"Not Found"}"#.to_vec()), "0.3.1"), Outcome::Failed(_)));
        assert!(matches!(outcome(Ok(br#"{"tag_name":7}"#.to_vec()), "0.3.1"), Outcome::Failed(_)));
        assert!(matches!(outcome(reply("nightly"), "0.3.1"), Outcome::Failed(_)));
        // this build's own version is one
        assert!(parse_version(VERSION).is_some());
    }

    #[test]
    fn the_button_stays_clear_of_the_files_button() {
        let u = update_button_rect("Check for updates");
        let f = crate::gamefiles::files_button_rect("Use my SimGolf files...");
        assert!(u.x > f.x + f.w + 100.0 && u.x + u.w <= 800.0);
    }
}
