//! The Multiple tees extra in the game (the port's own; sg_core::extras `Tee`, docs/PLAYING.md): the Tees tool's forward and
//! middle tees, how they are placed and drawn.
//!
//! * Tab with the Tees tool in hand steps it from the hole's own tee (the original's tool: the hole being built gets its
//!   tee) to a forward tee and a middle tee, and back. The pointer then names the tee and, over a tile where it may go, the
//!   hole it would belong to, with a line to that hole's cup and the yards. A click paints the tile a tee as the original's
//!   tool does (the same price, booked to that hole's building cost); painting over it with any brush takes that tee away
//!   and leaves the hole open. A tee is moved as the original's is: painted over, then placed again.
//! * The tee tiles are the game's own tee pictures, as every tee: the arrow of an open hole, turned toward the hole's cup,
//!   and the game's own tee markers either side of it, red for the forward tee and white for the middle one (the original
//!   sets those colours by par on a hole's own tee, which keeps its colour).
//!
//! * With the Second course extra (sg_core::second) the tees go to the holes of the course the Build Course panel's A/B
//!   switch shows, and Course B's golfers choose their tee in the same way.
//!
//! Nothing here changes or draws anything while the extra is off, or on a hole without such a tee.
use crate::app::App;
use sg_core::extras::{Tee, TeeRefusal};
use sg_core::lang::{tr, trf};

impl App {
    /// The tee the Tees tool places now: a forward or middle tee when the extra is on and Tab has stepped the tool there, else
    /// the hole's own tee (the original's tool).
    pub fn tee_tool(&self) -> Tee {
        // the tee a Paint action holds, while it is applied (replay.rs)
        if let Some(t) = self.tee_forced {
            return t;
        }
        let tee_brush = self.edit && self.tool == 0 && crate::app::PAINT.get(self.paint_idx).is_some_and(|p| p.ty == 0);
        if self.club.extras.tees && tee_brush {
            Tee::of_step(self.tee_step)
        } else {
            Tee::Back
        }
    }

    /// Tab with the Tees tool in hand and the extra on: the tool steps to the next tee (hole's own, forward, middle). The
    /// original's Tab goes on as before as well (it turns nothing a tee shows).
    pub fn tee_tab(&mut self) {
        let tee_brush = self.edit && self.tool == 0 && crate::app::PAINT.get(self.paint_idx).is_some_and(|p| p.ty == 0);
        if !self.club.extras.tees || !tee_brush {
            return;
        }
        self.tee_step = (self.tee_step + 1) % 3;
        let what = match Tee::of_step(self.tee_step) {
            Tee::Back => tr("STR_TEE_HOLE_TOOL").to_string(),
            t => tr(t.key()).to_string(),
        };
        self.ui_sound(0x0b);
        self.show_toast(&trf("STR_TOAST_TEE_STEP", &[("TEE", &what)]));
    }

    /// What a refused forward or middle tee says.
    pub fn tee_refusal_text(&self, t: Tee, e: TeeRefusal) -> Option<String> {
        let tee = tr(t.key());
        Some(match e {
            TeeRefusal::Off => return None,
            TeeRefusal::Tournament => tr("STR_TOAST_TEE_TOURNAMENT").to_string(),
            TeeRefusal::NoHole => trf("STR_TOAST_TEE_NO_HOLE", &[("TEE", &tee)]),
            TeeRefusal::Taken => trf("STR_TOAST_TEE_TAKEN", &[("TEE", &tee)]),
            TeeRefusal::Has(h) => trf("STR_TOAST_TEE_HAS", &[("HOLE", &self.hole_text(self.hole_b(), h as i32)), ("TEE", &tee)]),
            TeeRefusal::Order(h) => trf("STR_TOAST_TEE_ORDER", &[("HOLE", &self.hole_text(self.hole_b(), h as i32))]),
        })
    }

    /// The yards from tile (a, b) to the cup of hole `h` of Course B (`course_b`, the Second course extra) or Course A, as
    /// the tee tool's line counts them (25 a tile, truncated).
    pub fn yards_to_cup(&self, course_b: bool, h: usize, (a, b): (i32, i32)) -> i32 {
        let Some(pin) = self.club.course_holes(course_b).get(h).map(|r| r.pin) else { return 0 };
        let (dx, dy) = ((a - pin.0) * 25, (b - pin.1) * 25);
        ((dx * dx + dy * dy) as f64).sqrt() as i32
    }

    /// The tee markers of the open holes' forward (red) and middle (white) tees, with the extra on, on both courses (the
    /// Second course extra's Course B too): the game's own markers, chosen as the original chooses them by par (red the
    /// par 3's, white the par 4's).
    pub(crate) fn push_extra_tee_markers(&mut self) {
        if !self.club.extras.tees {
            return;
        }
        let mut draws = Vec::new();
        for course_b in [false, true] {
            for rec in self.club.course_holes(course_b).iter().take(19).skip(1) {
                if rec.par == 0 {
                    continue;
                }
                for (t, p) in rec.extra_tees() {
                    if !sg_core::course::inside(p.0, p.1) {
                        continue;
                    }
                    let facing = rec.extra_tee_facing(p).unwrap_or(rec.tee_facing);
                    draws.push((p, if t == Tee::Forward { 3 } else { 4 }, facing));
                }
            }
        }
        for ((a, b), par, facing) in draws {
            for d in sg_core::decor::tee_markers(par, facing, None) {
                self.push_decor(a, b, d, false, true);
            }
        }
    }
}
