//! The port's Cheats box (docs/PLAYING.md, "Cheats"; GitHub #42): a hidden key on the course screen (Ctrl+Shift+C,
//! remappable in controls.rs) opens a list in the original's popup style (popup_ui's two-choice and list boxes, the
//! generic popup 0x46d6e0) with money to add or take away, a VIP visit forced now, all the land bought, and the Extras
//! box's three Sandbox switches. Nothing changes until the player picks a line; the box stays open for more until Done or
//! Esc. The rules are sg_core::cheats; using any cheat marks the game (`Club::cheated`), which the Load list notes.
//! Not in the original.

use crate::app::App;
use crate::popup_ui::PopupKind;
use sg_core::cheats::{self, MONEY_LARGE, MONEY_SMALL};
use sg_core::lang::{tr, trf};

/// The box's options in order (counted from the first option line, as the popup returns them).
pub const ADD_SMALL: usize = 0;
pub const ADD_LARGE: usize = 1;
pub const TAKE_SMALL: usize = 2;
pub const VIP: usize = 3;
pub const LAND: usize = 4;
/// The Sandbox switches of the Extras box (sg_core::extras::OPTION_KEYS 7, 8 and 9), shortcuts to the same ticks.
pub const SWITCHES: [usize; 3] = [5, 6, 7];
pub const DONE: usize = 8;

/// The Extras options the switch lines stand for.
const SWITCH_OPTIONS: [usize; 3] = [7, 8, 9];

impl App {
    /// The hidden key: the Cheats box over the course (only with a game on).
    pub fn open_cheats(&mut self) {
        if self.land.is_none() {
            return;
        }
        self.ui_sound(0x0b);
        self.open_popup(PopupKind::Cheats);
    }

    /// The box's lines (a title, a note, the options) and its greyed options: a VIP while none can come, the land when
    /// there is none left to buy.
    pub fn cheat_lines(&self) -> (Vec<String>, u32) {
        let money = |v: f64| crate::ui::money(v as i64);
        let ex = self.club.extras;
        let switch = |i: usize| {
            let on = ex.option(SWITCH_OPTIONS[i]) != 0;
            trf(
                "STR_CHEATS_SWITCH",
                &[
                    ("SWITCH", &tr(sg_core::extras::OPTION_KEYS[SWITCH_OPTIONS[i]]).trim()),
                    ("STATE", &tr(if on { "STR_CHEATS_ON" } else { "STR_CHEATS_OFF" })),
                ],
            )
        };
        let mut v = vec![
            tr("STR_CHEATS_TITLE").to_string(),
            tr("STR_CHEATS_NOTE").to_string(),
            trf("STR_CHEATS_ADD", &[("MONEY", &money(MONEY_SMALL))]),
            trf("STR_CHEATS_ADD", &[("MONEY", &money(MONEY_LARGE))]),
            trf("STR_CHEATS_TAKE", &[("MONEY", &money(MONEY_SMALL))]),
            tr("STR_CHEATS_VIP").to_string(),
            tr("STR_CHEATS_LAND").to_string(),
        ];
        v.extend((0..SWITCHES.len()).map(switch));
        v.push(tr("STR_CHEATS_DONE").to_string());
        let mut disabled = 0;
        if !cheats::vip_possible(&self.club) {
            disabled |= 1 << VIP;
        }
        if self.land.as_ref().is_none_or(|l| cheats::tracts_for_sale(l).is_empty()) {
            disabled |= 1 << LAND;
        }
        (v, disabled)
    }

    /// Option `k` of the box picked (not Done): the cheat is used, with a word on the ticker line.
    pub fn use_cheat(&mut self, k: usize) {
        match k {
            ADD_SMALL | ADD_LARGE | TAKE_SMALL => {
                let amount = match k {
                    ADD_SMALL => MONEY_SMALL,
                    ADD_LARGE => MONEY_LARGE,
                    _ => -MONEY_SMALL,
                };
                cheats::add_money(&mut self.club, &mut self.econ, amount);
                let sign = if amount > 0.0 { "+" } else { "" };
                let text = trf(
                    "STR_TOAST_CHEAT_MONEY",
                    &[("MONEY", &format!("{sign}{}", crate::ui::money(amount as i64))), ("CASH", &crate::ui::money(self.econ.cash as i64))],
                );
                self.show_toast(&text);
            }
            VIP => {
                if cheats::force_vip(&mut self.club) {
                    self.show_toast(tr("STR_TOAST_CHEAT_VIP"));
                }
            }
            LAND => self.cheat_buy_land(),
            _ => {
                if let Some(i) = SWITCHES.iter().position(|&s| s == k) {
                    // the same tick as the Extras box's (kept as the next games' default too)
                    let mut e = self.club.extras;
                    e.step_option(SWITCH_OPTIONS[i]);
                    self.set_extras(e);
                }
            }
        }
        println!("cheat {k} used");
    }

    /// Buy all the land: every tract still for sale becomes the club's, for nothing, and the drawn terrain follows (as the
    /// land screen's purchase, `land_click`).
    fn cheat_buy_land(&mut self) {
        let Some(land) = self.land.as_mut() else { return };
        let bought = cheats::buy_all_land(&mut self.club, land);
        if bought.is_empty() {
            return;
        }
        let side = sg_core::tracts::side();
        for i in bought {
            let (a0, b0) = sg_core::tracts::origin(i);
            for a in a0..a0 + side {
                for b in b0..b0 + side {
                    let t = (a * sg_core::land::mn() + b) as usize;
                    let o = self.terrain.tile_index(a, b);
                    if self.terrain.ty[o] == sg_core::land::T_OUT {
                        self.terrain.ty[o] = land.ty[t];
                        self.terrain.variation[o] = if land.ty[t] == sg_core::land::T_WATER { land.var[t] } else { 0 };
                    }
                }
            }
        }
        self.dirty = true;
        self.refresh_trees();
        self.show_toast(tr("STR_TOAST_CHEAT_LAND"));
    }
}
