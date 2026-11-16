//! The Move the clubhouse extra (sg_core::extras `move_clubhouse`; the port's own). The original
//! puts the clubhouse where the property's generator dropped it, for good. With the extra on, a right click (a long press
//! on a touch screen) on the clubhouse asks whether to move it; Yes arms the building tool with the clubhouse, whose
//! see-through picture follows the pointer as any building's does, and a click puts it down there: the old footprint
//! goes back to rough, the clubhouse stands on the new spot (keeping its object record, the first), and the price is
//! what a new clubhouse costs to build (the buildings table's), plus clearing, booked to Facilities. The click is the
//! building tool's own action (`Action::Build` with the clubhouse's kind), so a replay does the same.
//!
//! Golfers already out keep playing; the next pair tees off from the new door. Paths are not moved: the clubhouse's
//! joining flood (sg_core::land::joined_tiles) starts from wherever it stands, so the buildings off its old paths show as
//! unjoined until a path reaches the new spot (the Path help's markers say which).

use crate::app::App;
use sg_core::economy::{self, Economy};
use sg_core::land;
use sg_core::lang::{tr, trf};

impl App {
    /// The clubhouse's object record.
    fn clubhouse_index(&self) -> Option<usize> {
        self.land.as_ref()?.objects.iter().position(|o| o.kind == land::K_CLUBHOUSE)
    }

    /// Whether tile x, y is under the clubhouse, with the extra on.
    pub fn on_clubhouse(&self, x: i32, y: i32) -> bool {
        if !self.club.extras.move_clubhouse {
            return false;
        }
        let (Some(l), Some(i)) = (self.land.as_ref(), self.clubhouse_index()) else { return false };
        let o = l.objects[i];
        let s = l.footprint_size(&o);
        (o.a..o.a + s).contains(&x) && (o.b..o.b + s).contains(&y)
    }

    /// What a move costs before clearing: a new clubhouse's price.
    pub fn move_clubhouse_units() -> i64 {
        sg_core::data::buildings()[land::K_CLUBHOUSE as usize].2 as i64
    }

    /// A right click on the clubhouse with the extra on: the question. True when it was the clubhouse.
    pub fn ask_move_clubhouse(&mut self, x: i32, y: i32) -> bool {
        if !self.on_clubhouse(x, y) {
            return false;
        }
        self.ui_sound(0x0b);
        self.open_popup(crate::popup_ui::PopupKind::MoveClubhouse);
        true
    }

    /// The box's lines: the question, the note, Yes, No.
    pub fn move_clubhouse_lines(&self) -> [String; 4] {
        let cost = crate::ui::money(Self::move_clubhouse_units() * Economy::UNIT as i64);
        [
            trf("STR_MOVE_CLUBHOUSE_1", &[("COST", &cost)]),
            tr("STR_MOVE_CLUBHOUSE_2").to_string(),
            tr("STR_MOVE_CLUBHOUSE_YES").to_string(),
            tr("STR_MOVE_CLUBHOUSE_NO").to_string(),
        ]
    }

    /// The box's Yes: the building tool with the clubhouse in hand.
    pub fn start_move_clubhouse(&mut self) {
        self.clubhouse_tool_was = Some(self.build_idx);
        self.tool = 4;
        self.build_idx = land::K_CLUBHOUSE as usize;
        self.design = None;
        self.edit = true;
        self.show_toast(tr("STR_MOVE_CLUBHOUSE_PICK"));
    }

    /// The building tool's click with the clubhouse in hand (edit_building): moves it to the footprint at tx, ty.
    pub fn move_clubhouse(&mut self, tx: i32, ty: i32) -> bool {
        let theme = self.exe_theme();
        let holes = self.hole_numbers.len();
        let refuse = |app: &mut App, key: &'static str| {
            app.ui_sound(24);
            app.show_toast(tr(key));
            false
        };
        if !self.club.extras.move_clubhouse {
            return false;
        }
        if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 {
            return refuse(self, "STR_MOVE_CLUBHOUSE_TOURNAMENT");
        }
        let Some(i) = self.clubhouse_index() else { return false };
        let Some(l) = self.land.as_mut() else { return false };
        l.sync_from_terrain(&self.terrain);
        let old = l.objects[i];
        if (old.a, old.b) == (tx, ty) {
            return false;
        }
        let size = sg_core::data::buildings()[land::K_CLUBHOUSE as usize].1;
        // its own footprint may be overlapped (fits lets an object stand over one of its kind); nothing else's, and the
        // whole footprint on land the club owns
        let owned = (0..size).all(|r| (0..size).all(|c| l.in_play(tx + r, ty + c)));
        let Some(clear) = l.fits(tx, ty, size, land::K_CLUBHOUSE, theme).filter(|_| owned) else {
            return refuse(self, "STR_TOAST_NO_ROOM");
        };
        let cost = (Self::move_clubhouse_units() + clear as i64) as f64 * Economy::UNIT;
        if !self.econ.affordable(cost, holes) {
            self.show_toast(&trf(
                "STR_TOAST_CHANGE_COSTS",
                &[("COST", &crate::ui::money(cost as i64)), ("CASH", &crate::ui::money(self.econ.cash as i64))],
            ));
            return false;
        }
        let Some(l) = self.land.as_mut() else { return false };
        let os = l.footprint_size(&old);
        l.remove_object(i);
        l.write_area(&mut self.terrain, old.a, old.b, old.a + os - 1, old.b + os - 1);
        // the record freed above is the first free one, so the clubhouse keeps its place in the table (the first)
        let n = l.place(&mut self.exe_rng, tx, ty, land::K_CLUBHOUSE, 0x60, theme);
        debug_assert_eq!(n, i);
        l.objects[n].flags = old.flags | 0x40;
        l.objects[n].dir = (self.turn & 3) as u8;
        l.view = (tx + 6, ty - 4);
        l.clubhouse = (tx, ty);
        let ci = (tx * land::mn() + ty) as usize;
        l.flags[ci] &= !land::flag::PATH;
        l.write_area(&mut self.terrain, tx, ty, tx + size - 1, ty + size - 1);
        self.terrain.clubhouse_x = tx + 2;
        self.terrain.clubhouse_y = ty + 2;
        let cash = self.econ.cash;
        self.econ.spend_to(economy::LEDGER_FACILITIES, cost);
        self.float_change(cash, (tx + 1, ty + 1));
        self.after_object_change();
        self.snd("Interface/Building.wav", 0.7, false);
        self.show_toast(tr("STR_MOVE_CLUBHOUSE_DONE"));
        // the tool goes back to what it held
        if let Some(k) = self.clubhouse_tool_was.take() {
            self.build_idx = k;
        }
        self.edit = false;
        true
    }
}
