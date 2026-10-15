//! The Course sharing interface extra (extras.rs; docs/PLAYING.md "Sharing courses and challenges"): System Functions,
//! *Share course* puts a course code on the clipboard (the course saved for championship play, as text) and saves the
//! same course as a file. A friend brings it in with Ctrl+V (Cmd+V on a Mac), the *Paste a course code* button on Play a
//! Championship's course list, or by dropping the file on the window; the course joins their championship courses, and
//! the sharer's course record is the challenge to beat. Not in the original.

use crate::app::*;
use sg_core::lang::{tr, trf};

/// What a course code starts with (the number is the code's version).
pub const PREFIX: &str = "OPENSGOLF-COURSE-1:";

/// Seconds a Ctrl+V or the paste button waits for the clipboard (the browser hands over a paste just after the key).
const PASTE_WAIT: f64 = 1.5;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                s.push(B64[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// Decodes base64, skipping spaces and line breaks (a code pasted from a chat may be wrapped). None if it is not base64.
pub fn unbase64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0);
    for ch in s.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=') {
        let v = B64.iter().position(|&c| c == ch)? as u32;
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// The save inside a course code (anything around the code, such as a message to the friend, is ignored).
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let at = text.find(PREFIX)?;
    let body: String = text[at + PREFIX.len()..].chars().take_while(|c| c.is_ascii_alphanumeric() || "+/=\r\n \t".contains(*c)).collect();
    unbase64(&body).filter(|b| is_save(b))
}

impl App {
    /// System Functions, *Share course*: the course code on the clipboard and the course saved for championship play.
    pub fn share_course(&mut self) {
        if self.land.is_none() {
            return;
        }
        // as Save course for Championship: not during a tournament
        if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 {
            self.ui_sound(24);
            return self.show_toast(tr("STR_TOAST_CHAMP_NO_SAVE_IN_TOURNAMENT"));
        }
        let own = self.save_bytes(9);
        let shared = self.save_bytes(1).and_then(|b| read_save_bytes(&b, "share")).and_then(|mut s| {
            s.shared = true;
            // the message history and the golfers' words stay with the sharer
            s.extras = Default::default();
            encode_save(&s, 9)
        });
        match shared {
            Ok(b) => {
                let code = format!("{PREFIX}{}", base64(&b));
                miniquad::window::clipboard_set(&code);
                // the sharer's own copy for championship play, without the challenge mark
                if let Ok(own) = own {
                    let path = self.championship_dir().join(format!("{}.{}", self.course_name, crate::champ_ui::COURSE_EXT));
                    sg_core::fsutil::write_file(&path, &own);
                }
                self.show_toast(&trf("STR_TOAST_SHARE_COPIED", &[("COURSE", &self.course_name)]));
            }
            Err(e) => self.show_toast(&trf("STR_TOAST_COULD_NOT_SAVE", &[("ERROR", &e)])),
        }
    }

    /// Ctrl+V or the paste button: a course code is taken from the clipboard over the next moment.
    pub fn paste_course(&mut self) {
        self.paste_wait = crate::app::now() + PASTE_WAIT;
    }

    /// Each frame while a paste is waited for: a course code on the clipboard comes in.
    pub fn share_tick(&mut self, t: f64) {
        if self.paste_wait == 0.0 {
            return;
        }
        if let Some(b) = miniquad::window::clipboard_get().as_deref().and_then(decode) {
            self.paste_wait = 0.0;
            self.import_course(&b);
        } else if t > self.paste_wait {
            self.paste_wait = 0.0;
            self.show_toast(tr("STR_TOAST_SHARE_NO_CODE"));
        }
    }

    /// A shared course (a code's or a dropped file's bytes): saved with the championship courses. A dropped saved game
    /// (`.sgs`) goes to Saved Games instead.
    pub fn import_course(&mut self, b: &[u8]) {
        self.import_save(b, None)
    }

    /// Files dropped on the window: shared courses and saved games, or a text file holding a course code.
    pub fn import_dropped(&mut self, name: &str, b: &[u8]) {
        if is_save(b) {
            self.import_save(b, Some(name));
        } else if let Some(c) = std::str::from_utf8(b).ok().and_then(decode) {
            self.import_save(&c, None);
        } else {
            self.show_toast(&trf("STR_ERROR_NOT_A_SAVED_GAME", &[("FILE", &name)]));
        }
    }

    fn import_save(&mut self, b: &[u8], dropped: Option<&str>) {
        let s = match read_save_bytes(b, dropped.unwrap_or("course code")) {
            Ok(s) => s,
            Err(e) => return self.show_toast(&e),
        };
        // a safe file name: no path characters, and not an autosave's leading '&'
        let clean = |n: &str| {
            let s: String = n.chars().map(|c| if "\\/:*?\"<>|".contains(c) || c.is_control() { '_' } else { c }).collect();
            let s = s.trim_start_matches(['&', '.']).trim().to_string();
            if s.is_empty() {
                "Shared".to_string()
            } else {
                s
            }
        };
        // never over a file that is there already: "Name (2)", "Name (3)"...
        let unique = |dir: std::path::PathBuf, stem: String, ext: &str| {
            let mut p = dir.join(format!("{stem}.{ext}"));
            let mut n = 2;
            while sg_core::fsutil::read_file(&p).is_some() && n < 100 {
                p = dir.join(format!("{stem} ({n}).{ext}"));
                n += 1;
            }
            p
        };
        let game = dropped.is_some_and(|n| n.to_ascii_lowercase().ends_with(".sgs"));
        let path = if game {
            let stem = dropped.and_then(|n| std::path::Path::new(n).file_stem()).map(|s| s.to_string_lossy().to_string());
            unique(crate::files_ui::saved_games_dir(), clean(&stem.unwrap_or_else(|| s.course_name.clone())), "sgs")
        } else {
            unique(self.championship_dir(), clean(&s.course_name), crate::champ_ui::COURSE_EXT)
        };
        if !sg_core::fsutil::write_file(&path, b) {
            return self.show_toast(&trf("STR_ERROR_COULD_NOT_WRITE", &[("FILE", &path.display())]));
        }
        let key = if game { "STR_TOAST_SHARE_GAME_ADDED" } else { "STR_TOAST_SHARE_COURSE_ADDED" };
        self.show_toast(&trf(key, &[("COURSE", &s.course_name)]));
        // an open list shows it at once
        if let Some((kind, from_game, course)) = self.title.files.as_ref().map(|f| (f.kind, f.from_game, f.course.clone())) {
            if self.screen == Screen::Files {
                self.title.files = None;
                self.open_files(kind, from_game);
                if let Some(f) = self.title.files.as_mut() {
                    f.course = course;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        for n in 0..10 {
            let d: Vec<u8> = (0..n).map(|i| (i * 37 + 5) as u8).collect();
            assert_eq!(unbase64(&base64(&d)).unwrap(), d);
        }
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(unbase64("TW\nE=").unwrap(), b"Ma");
    }
}
