//! The Eyesore demolition extra (sg_core::extras `demolish`; the port's own): a right click (a
//! long press on a touch screen) on an eyesore the club owns asks whether to pull it down for
//! `sg_core::extras::DEMOLISH_UNITS`; Yes is an action (`Action::Demolish`, so a replay does the same), which takes the
//! landmark away, frees its tile and books the price to Build course. The eyesores are the landmarks of types 16 to 18
//! (the water tower, the radio mast and the oil pump) that a property's difficulty puts on it (sg_core::land's obstacles).
//! With the extra off the original's rule stands: they cannot be removed.

use crate::app::App;
use sg_core::economy::{self, Economy};
use sg_core::land;
use sg_core::lang::{tr, trf};

/// Whether landmark type `t` is an eyesore.
pub fn eyesore(t: i32) -> bool {
    (16..=18).contains(&t)
}

/// An eyesore's name for the confirmation box.
fn eyesore_name(t: i32) -> &'static str {
    tr(match t {
        16 => "STR_DEMOLISH_WATER_TOWER",
        17 => "STR_DEMOLISH_RADIO_MAST",
        _ => "STR_DEMOLISH_OIL_PUMP",
    })
}

/// The price, for the box and the extras' description.
pub fn cost_words() -> String {
    crate::ui::money(sg_core::extras::DEMOLISH_UNITS * Economy::UNIT as i64)
}

impl App {
    /// The eyesore (object record and type) the club owns on tile x, y, with the extra on.
    pub fn eyesore_at(&self, x: i32, y: i32) -> Option<(usize, i32)> {
        if !self.club.extras.demolish {
            return None;
        }
        let l = self.land.as_ref()?;
        if !l.in_play(x, y) {
            return None;
        }
        l.objects
            .iter()
            .position(|o| {
                let s = l.footprint_size(o);
                o.kind == land::K_LANDMARK && eyesore(o.sub) && (o.a..o.a + s).contains(&x) && (o.b..o.b + s).contains(&y)
            })
            .map(|i| (i, l.objects[i].sub))
    }

    /// A right click on an eyesore with the extra on: the confirmation box. True when it was one.
    pub fn ask_demolish(&mut self, x: i32, y: i32) -> bool {
        let Some((_, t)) = self.eyesore_at(x, y) else { return false };
        self.demolish_at = Some((x, y, t));
        self.ui_sound(0x0b);
        self.open_popup(crate::popup_ui::PopupKind::Demolish);
        true
    }

    /// The box's lines: the question, the note, Yes, No.
    pub fn demolish_lines(&self) -> [String; 4] {
        let t = self.demolish_at.map_or(16, |d| d.2);
        [
            trf("STR_DEMOLISH_1", &[("WHAT", &eyesore_name(t)), ("COST", &cost_words())]),
            tr("STR_DEMOLISH_2").to_string(),
            tr("STR_DEMOLISH_YES").to_string(),
            tr("STR_DEMOLISH_NO").to_string(),
        ]
    }

    /// The box's Yes: the action.
    pub fn confirm_demolish(&mut self) {
        if let Some((x, y, _)) = self.demolish_at.take() {
            self.act(sg_core::action::Action::Demolish { x, y });
        }
    }

    /// `Action::Demolish`: the eyesore on x, y pulled down and paid for. False when there is none there any more or the
    /// club cannot pay (the toast says so).
    pub fn demolish(&mut self, x: i32, y: i32) -> bool {
        let Some((i, t)) = self.eyesore_at(x, y) else { return false };
        let cost = sg_core::extras::DEMOLISH_UNITS as f64 * Economy::UNIT;
        let holes = self.hole_numbers.len();
        if !self.econ.affordable(cost, holes) {
            self.show_toast(&trf(
                "STR_TOAST_CHANGE_COSTS",
                &[("COST", &crate::ui::money(cost as i64)), ("CASH", &crate::ui::money(self.econ.cash as i64))],
            ));
            return false;
        }
        let Some(l) = self.land.as_mut() else { return false };
        let o = l.objects[i];
        let size = l.footprint_size(&o);
        l.remove_object(i);
        // the anchor's obstacle mark goes with it, so the tile takes building and painting again
        for r in 0..size {
            for c in 0..size {
                let (a, b) = (o.a + r, o.b + c);
                if l.in_play(a, b) {
                    let k = (a * land::mn() + b) as usize;
                    l.flags[k] &= !land::flag::OBSTACLE;
                }
            }
        }
        l.write_area(&mut self.terrain, o.a, o.b, o.a + size - 1, o.b + size - 1);
        let cash = self.econ.cash;
        self.econ.spend_to(economy::LEDGER_BUILD_COURSE, cost);
        self.float_change(cash, (x, y));
        self.after_object_change();
        self.show_toast(&trf("STR_DEMOLISH_DONE", &[("WHAT", &eyesore_name(t))]));
        self.snd("Interface/Building.wav", 0.7, false);
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_eyesores_are_the_three_obstacle_landmarks() {
        assert!(!super::eyesore(15) && super::eyesore(16) && super::eyesore(17) && super::eyesore(18) && !super::eyesore(19));
        assert_eq!(sg_core::objects::LANDMARKS[16], "Landmarks/Wtow2");
        assert_eq!(sg_core::objects::LANDMARKS[18], "Landmarks/Red Oil Pump");
    }
}
