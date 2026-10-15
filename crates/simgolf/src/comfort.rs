//! Everyday comfort (the port's own, none of it in the original; docs/PLAYING.md "Quick save, pauses and staff areas"):
//!
//! * quick save and quick load (Shift+F5, Shift+F9; controls.rs): the game saved to "&Quick.sgs" in Saved Games without the
//!   Save dialog, and loaded back after a yes / no box. Refused during a tournament, as the championship course's save is
//!   (STR_TOAST_CHAMP_NO_SAVE_IN_TOURNAMENT). The save goes where the Save dialog's do (the browser's store on the web);
//! * pause in the background (an interface extra, extras.rs): the game pauses when its window loses the focus (miniquad's
//!   window_minimized_event: focus out on X11, Windows and macOS, the page hidden or blurred in the browser, the app
//!   paused on Android) and goes on when it comes back, unless the player paused or unpaused it meanwhile;
//! * pause on big news (an interface extra): the game pauses after the club's tick that brings a VIP (CEO, commissioner,
//!   heiress), a famous golfer's challenge, the SGA's tournament offer, a step of the board's debt warning, or cash going
//!   below zero. The news is on the ticker or the year-end report as before; the game waits there.
//!
//! With the extras off and the keys unused nothing changes.
use crate::app::{App, Screen};
use crate::gfx::Gfx;
use crate::ui::Screen as Ui;
use sg_core::lang::tr;

/// The quick save's file name ('&' marks the port's own saves in the Load list, files_ui.rs).
pub const QUICK_FILE: &str = "&Quick.sgs";

/// Why the game paused by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    Background,
    News,
}

/// The comfort features' state (`App::comfort`).
#[derive(Default)]
pub struct State {
    /// The pause these features made (None: the player's own, or not paused).
    pub paused_for: Option<Why>,
    /// A pause for news happened during this frame's ticks: the remaining ticks are dropped.
    pub hold: bool,
    /// The staff areas view (Shift+A, staffview_ui.rs) and its tiles, made again every few frames.
    pub staff_view: bool,
    pub staff_marks: Option<crate::staffview_ui::Marks>,
}

/// What the club looks like for the news the pause watches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct News {
    /// VIPs on the course (golfers of kinds 0x40 commissioner, 0x60 CEO, 0x80 heiress).
    pub vips: u32,
    /// A famous golfer's challenge is waiting (game flag 0x2000).
    pub challenge: bool,
    /// The SGA offers a tournament (game flag 0x400000).
    pub offered: bool,
    /// The board's debt counter (economy.rs year_end).
    pub debt_stage: i32,
    /// Cash below zero (outside a sandbox).
    pub in_debt: bool,
}

/// True when `after` brings news `before` did not have.
pub fn big_news(before: &News, after: &News) -> bool {
    after.vips > before.vips
        || after.challenge && !before.challenge
        || after.offered && !before.offered
        || after.debt_stage > before.debt_stage
        || after.in_debt && !before.in_debt
}

impl App {
    /// The club as the news pause watches it.
    pub fn news(&self) -> News {
        let vips = self.club.g.iter().take(sg_core::golfer::SLOTS).filter(|g| g.hole != 0 && matches!(g.vip(), 0x40 | 0x60 | 0x80)).count();
        News {
            vips: vips as u32,
            challenge: self.club.game & sg_core::pro::CHALLENGE != 0,
            offered: self.club.game & sg_core::tournament::OFFERED != 0,
            debt_stage: self.econ.debt_stage,
            in_debt: self.econ.cash < 0.0 && !self.econ.sandbox,
        }
    }

    /// After a club tick that brought news: the game pauses (when it is running) and the frame's other ticks are dropped.
    pub fn pause_for_news(&mut self) {
        if self.paused {
            return;
        }
        println!("[{:6.1}s] paused for the news", self.sim_time);
        self.paused = true;
        self.comfort.paused_for = Some(Why::News);
        self.comfort.hold = true;
    }

    /// The window went to the background (the Pause in the background extra): the game pauses when it runs.
    pub fn window_hidden(&mut self) {
        if crate::extras::pause_background() && !self.paused && self.land.is_some() {
            self.paused = true;
            self.comfort.paused_for = Some(Why::Background);
        }
    }

    /// The window is back: a pause made by `window_hidden` ends.
    pub fn window_shown(&mut self) {
        if self.comfort.paused_for == Some(Why::Background) && self.paused {
            self.paused = false;
        }
        if self.comfort.paused_for == Some(Why::Background) {
            self.comfort.paused_for = None;
        }
    }

    /// Shift+F5: the game saved to the quick save, with a message on the ticker.
    pub fn quick_save(&mut self) {
        if self.land.is_none() {
            return;
        }
        if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 {
            self.ui_sound(0x18);
            self.show_toast(tr("STR_TOAST_QUICK_NO_SAVE"));
            return;
        }
        let path = crate::files_ui::saved_games_dir().join(QUICK_FILE);
        match self.save_game(&path) {
            Ok(()) => {
                println!("quick saved {}", path.display());
                self.show_toast(&sg_core::lang::trf("STR_TOAST_QUICK_SAVED", &[("KEY", &crate::controls::key_label("quick_load"))]));
            }
            Err(e) => self.show_toast(&sg_core::lang::trf("STR_TOAST_COULD_NOT_SAVE", &[("ERROR", &e)])),
        }
    }

    /// Shift+F9: the yes / no box before the quick save is loaded (or a note when there is none).
    pub fn ask_quick_load(&mut self) {
        if !sg_core::fsutil::exists(crate::files_ui::saved_games_dir().join(QUICK_FILE)) {
            self.ui_sound(0x18);
            self.show_toast(&sg_core::lang::trf("STR_TOAST_QUICK_NONE", &[("KEY", &crate::controls::key_label("quick_save"))]));
            return;
        }
        self.ui_sound(0x0b);
        self.open_popup(crate::popup_ui::PopupKind::QuickLoad);
    }

    /// The box's yes: the quick save loaded.
    pub fn quick_load(&mut self, g: &mut Gfx) {
        let path = crate::files_ui::saved_games_dir().join(QUICK_FILE);
        match self.load_game(g, &path) {
            Ok(()) => {
                println!("loaded game {}", path.display());
                self.screen = Screen::Play;
            }
            Err(e) => self.show_toast(&sg_core::lang::trf("STR_TOAST_COULD_NOT_LOAD_GAME", &[("ERROR", &e)])),
        }
    }

    /// Why the game paused by itself, under the Paused caption (over "Following", which keeps its place).
    pub fn draw_pause_reason(&self, g: &mut Gfx, s: &Ui) {
        if !self.paused {
            return;
        }
        let key = match self.comfort.paused_for {
            Some(Why::Background) => "STR_PAUSED_BACKGROUND",
            Some(Why::News) => "STR_PAUSED_EVENT",
            None => return,
        };
        let y = if self.xs.follow.is_some() { 410.0 } else { 428.0 };
        s.put_shadowed(g, crate::ui::F_MANUAL15, 512.0, y, tr(key), crate::ui::rgb(1.0, 1.0, 1.0), true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn news_is_what_was_not_there_before() {
        let calm = News::default();
        assert!(!big_news(&calm, &calm));
        let vip = News { vips: 1, ..calm };
        assert!(big_news(&calm, &vip));
        // the same VIP still on the course is no news; one leaving is none either
        assert!(!big_news(&vip, &vip));
        assert!(!big_news(&vip, &calm));
        // a second VIP is
        assert!(big_news(&vip, &News { vips: 2, ..calm }));
        assert!(big_news(&calm, &News { challenge: true, ..calm }));
        assert!(big_news(&calm, &News { offered: true, ..calm }));
        assert!(!big_news(&News { offered: true, ..calm }, &calm));
        // cash going below zero, and each step of the board's warning
        let red = News { in_debt: true, ..calm };
        assert!(big_news(&calm, &red));
        assert!(!big_news(&red, &red));
        assert!(big_news(&red, &News { debt_stage: 1, ..red }));
        assert!(!big_news(&News { debt_stage: 1, ..red }, &News { debt_stage: 0, ..calm }));
    }
}
