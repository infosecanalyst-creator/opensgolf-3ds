//! Bigger text (the port's own accessibility helper, docs/PLAYING.md "Text size"): Preferences' *Text size* line steps
//! normal, 125% and 150%, kept as `text_size` (100, 125 or 150) in settings.json. Normal leaves every screen exactly as it
//! was (nothing here scales by 1).
//!
//! It is not one switch on the font: each screen that takes it asks for its fonts through [`big`] (or [`fit`]) and lays
//! its lines out with the bigger fonts' own line heights, so a box grows with its text and its hit areas move with it.
//! Screens drawn on the original's fixed art (the golfer card, the reports' panels) cannot grow: their text takes
//! [`fit`], as big as still fits the room the art leaves, or a cap.
//!
//! The text itself is the same TrueType text at a bigger size: with Enhanced graphics' sharp text it is rasterised at the
//! window's own pixel size (ui.rs text_native), in Classic it comes from the same baked glyphs, scaled.
use crate::ui::Fnt;
use std::cell::Cell;

/// The settings.json key.
const KEY: &str = "text_size";
/// The sizes Preferences steps through, in percent.
pub const SIZES: [u32; 3] = [100, 125, 150];

thread_local! {
    static PCT: Cell<u32> = const { Cell::new(100) };
}

/// The text size in percent (100, 125 or 150).
pub fn pct() -> u32 {
    PCT.with(|c| c.get())
}

/// The text size as a factor (1, 1.25 or 1.5).
pub fn scale() -> f32 {
    pct() as f32 / 100.0
}

/// Sets the size for this run (any value is taken to the nearest of `SIZES`).
pub fn set(p: u32) {
    let p = SIZES.iter().copied().min_by_key(|&s| s.abs_diff(p)).unwrap_or(100);
    PCT.with(|c| c.set(p));
}

/// The words for a size on Preferences' line.
pub fn name_key(p: u32) -> &'static str {
    match p {
        125 => "STR_TEXT_SIZE_125",
        150 => "STR_TEXT_SIZE_150",
        _ => "STR_TEXT_SIZE_NORMAL",
    }
}

/// The size after `p` in Preferences' order (150 back to normal).
pub fn next(p: u32) -> u32 {
    let i = SIZES.iter().position(|&s| s == p).unwrap_or(0);
    SIZES[(i + 1) % SIZES.len()]
}

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// At start up: the saved size (normal when there is none). SG_TEXT_SIZE=125 or 150 overrides it for one run (a test
/// hook, not saved).
pub fn init() {
    let saved = settings_json()[KEY].as_u64().unwrap_or(100) as u32;
    let p = std::env::var("SG_TEXT_SIZE").ok().and_then(|v| v.trim().trim_end_matches('%').parse().ok()).unwrap_or(saved);
    set(p);
}

/// Preferences' line: the next size, in use at once and saved (normal is saved by taking the key away).
pub fn cycle() {
    save(next(pct()));
}

/// A size in use at once and saved (Preferences' line and its search box).
pub fn save(p: u32) {
    set(p);
    let mut v = settings_json();
    if let Some(o) = v.as_object_mut() {
        if pct() == 100 {
            o.remove(KEY);
        } else {
            o.insert(KEY.into(), serde_json::Value::from(pct()));
        }
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE), t.as_bytes());
    }
}

/// The factor a screen uses, at most `cap` (never below 1).
pub fn k(cap: f32) -> f32 {
    scale().min(cap).max(1.0)
}

/// A size in pixels at the text size.
pub fn px(size: f32) -> f32 {
    sized(size, scale())
}

/// `size` times `k`, to a whole pixel (unchanged at 1).
pub fn sized(size: f32, k: f32) -> f32 {
    if k == 1.0 {
        size
    } else {
        (size * k).round()
    }
}

/// A font at the text size.
pub fn big(f: Fnt) -> Fnt {
    f.scaled(scale())
}

/// A font as big as the text size asks (at most `cap` times its own) whose rendering of `text` is no wider than `w`, but
/// never smaller than `f` itself: for text on the original's fixed art.
pub fn fit(f: Fnt, cap: f32, text: &str, w: f32) -> Fnt {
    let top = k(cap);
    if top == 1.0 {
        return f;
    }
    let base = f.width(text);
    if base <= 0.0 {
        return f.scaled(top);
    }
    // the width grows with the size (rounded per glyph), so step down a pixel at a time from the biggest
    let mut px = (f.px * top).round();
    while px > f.px {
        let g = Fnt { face: f.face, px };
        if g.width(text) <= w {
            return g;
        }
        px -= 1.0;
    }
    f
}

/// `fit` with a size in pixels (the `Screen::text` calls).
pub fn fit_px(size: f32, cap: f32, text: &str, w: f32) -> f32 {
    let top = k(cap);
    if top == 1.0 {
        return size;
    }
    let mut px = (size * top).round();
    while px > size {
        if crate::ui::text_width(text, px) <= w {
            return px;
        }
        px -= 1.0;
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_step_round_and_normal_changes_nothing() {
        assert_eq!(next(100), 125);
        assert_eq!(next(125), 150);
        assert_eq!(next(150), 100);
        assert_eq!(next(77), 125);
        set(137);
        assert_eq!(pct(), 125);
        set(100);
        assert_eq!(scale(), 1.0);
        assert_eq!(k(2.0), 1.0);
        assert_eq!(px(15.0), 15.0);
        let f = crate::ui::F_MANUAL15;
        assert_eq!(big(f), f);
        assert_eq!(fit(f, 2.0, "anything at all", 1.0), f);
        set(150);
        assert_eq!(px(15.0), 23.0);
        assert_eq!(k(1.25), 1.25);
        assert_eq!(k(0.5), 1.0);
        assert_eq!(big(f).px, 23.0);
        assert_eq!(f.scaled(k(1.2)).px, 18.0);
        // a fit never goes below the font's own size, nor above the size asked for
        assert_eq!(fit(f, 2.0, "x", 0.0).px, f.px);
        assert!(fit(f, 2.0, "x", 1e6).px <= 23.0);
        set(100);
    }
}
