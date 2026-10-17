//! OpenSGolf extras: live scorecards (extras.rs). While the golfer card is open (a click on a golfer, the People screen's
//! Card, the profile page's Card), the golfer's round so far is drawn under it on the Player panel's own round scorecard
//! (player_panel.rs, the cover cut of JoeCoolPanel the original shows during the pro's round): hole numbers and pars along
//! the top, then the golfer and the partner, each hole's score coloured against par as the original colours the pro's,
//! the hole in progress in grey; the wide last column holds the course's par and each golfer's strokes so far, and the
//! score against par ("E", "+3", "-1") stands at the end of each name's bar. Without the Player panel's sheet (the free
//! base set) the scorecard is drawn plain in the same places.
use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::screens_ui::{top, BODY, SMALL};
use crate::ui::{rgb, Screen as Ui};
use sg_core::lang::{tr, trf};

/// Where the scorecard goes: the cover cut (drawn at (297, 493) on the Player panel) moved to sit under the golfer card,
/// centred on it (the card spans x 236 to 638) and clear of its story page and Profile button (down to y 340).
const D: (f32, f32) = (-112.0, -145.0);
const COVER: (f32, f32, f32, f32) = (297.0, 300.0, 503.0, 122.0);
const COVER_AT: (f32, f32) = (297.0, 493.0);

impl App {
    /// The live scorecard's rectangle while the golfer card is open (none with the extra off).
    pub fn live_card_rect(&self) -> Option<Rect> {
        if !crate::extras::live_scorecards() {
            return None;
        }
        let gi = self.card?;
        (self.club.g[gi].hole > 0).then_some(Rect::new(COVER_AT.0 + D.0, COVER_AT.1 + D.1, COVER.2, COVER.3))
    }

    /// The scorecard of golfer `gi`'s round under the golfer card.
    pub fn draw_live_scorecard(&self, g: &mut Gfx, s: &Ui, gi: usize) {
        let Some(r) = self.live_card_rect() else { return };
        let im = &self.panel_art.player;
        if im.tex.is_some() {
            crate::panels_ui::blit(g, s, im, COVER, r.x, r.y);
        } else {
            draw_plain_cover(g, s, r);
        }
        let white = c15(0x7fff);
        let black = c15(0);
        // the Player panel's row labels and its two lines on the round (here the hole the golfer is on)
        s.text(g, 410.0 + D.0, top(519.0 + D.1, SMALL), tr("STR_SCORECARD_HOLE"), SMALL, white);
        s.text(g, 414.0 + D.0, top(534.0 + D.1, SMALL), tr("STR_SCORECARD_PAR"), SMALL, white);
        let h = self.club.g[gi].hole;
        let where_ =
            if (1..19).contains(&h) { trf("STR_SCORECARD_ON_HOLE", &[("HOLE", &h)]) } else { tr("STR_SCORECARD_FINISHED").to_string() };
        s.text(g, 334.0 + D.0, top(522.0 + D.1, BODY), tr("STR_SCORECARD_LIVE"), BODY, black);
        s.text(g, 334.0 + D.0, top(533.0 + D.1, BODY), &where_, BODY, black);
        self.draw_card_rows(g, s, gi, D, false, true);
    }
}

/// The scorecard without the sheet: the panel's blue, the name bars and the cells in cream, in the cover's places.
fn draw_plain_cover(g: &mut Gfx, s: &Ui, r: Rect) {
    let (x, y) = (r.x - COVER_AT.0, r.y - COVER_AT.1);
    s.fill(g, r.x, r.y, r.w, r.h, rgb(0.40, 0.40, 0.62));
    let cream = rgb(0.99, 0.97, 0.93);
    for by in [549.0, 569.0] {
        s.fill(g, 308.0 + x, by + y, 124.0, 14.0, cream);
    }
    let rows = [(513.0, 15.0), (529.0, 14.0), (544.0, 22.0), (567.0, 22.0)];
    for (ry, rh) in rows {
        for i in 1..19 {
            s.fill(g, 436.0 + 18.0 * (i - 1) as f32 + x, ry + y, 16.0, rh, cream);
        }
        s.fill(g, 762.0 + x, ry + y, 35.0, rh, cream);
    }
}

#[cfg(test)]
mod tests {
    use crate::player_panel::{round_so_far, score_colour, vs_par};

    #[test]
    fn the_round_so_far_counts_finished_holes_against_par() {
        let mut holes = vec![sg_core::golfer::Hole::default(); 20];
        for (h, par) in [(1, 4), (2, 3), (3, 5), (4, 4)] {
            holes[h].par = par;
        }
        let mut card = [0i8; 19];
        card[1] = 5;
        card[2] = 2;
        card[3] = 5;
        card[4] = 7; // the hole in progress: not counted
        assert_eq!(round_so_far(&card, 4, &holes), (12, 0));
        assert_eq!(round_so_far(&card, 3, &holes), (7, 0));
        card[3] = 7;
        assert_eq!(round_so_far(&card, 4, &holes), (14, 2));
        // walking off after the last hole (19): every hole counts
        assert_eq!(round_so_far(&card, 19, &holes), (21, 5));
        // a hole with no par (closed) is left out
        holes[2].par = 0;
        assert_eq!(round_so_far(&card, 19, &holes), (19, 6));
        assert_eq!(vs_par(0), "E");
        assert_eq!(vs_par(3), "+3");
        assert_eq!(vs_par(-2), "-2");
        // the original's colours: under par red, par dark grey, over par blue then purple
        assert_eq!(
            [score_colour(2, 4), score_colour(3, 4), score_colour(4, 4), score_colour(5, 4), score_colour(6, 4)],
            [0x7d08, 0x6000, 0x2108, 0x0018, 0x4010]
        );
    }
}
