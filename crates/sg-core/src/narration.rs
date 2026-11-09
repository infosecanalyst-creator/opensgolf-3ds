//! First-time narration (the tail of the reaction routine 0x467a00): the first time each reaction type fires in a game, the
//! message window explains it once: `'` + the golfer's thought + `' ` + the golfer's name + a sentence for the type
//! (STR_NARR_* in lang/en.txt).
//!
//! Every rule and string here is from the publisher's golf.exe (the English text is the game's own).

use crate::course::Course;
use crate::golfer::Club;
use crate::lang::{tr, trf};
use crate::thoughts::feature_at;
use crate::tracts::type_name;

/// Ticks that must pass after a narration before the next one (0x467a00 compares the tick 0x834170 with 0x55e5ac).
pub const GAP: u32 = 500;
/// The message window priority the exe passes (0x40cb00): refused while a message is up, and shown 8 frames late.
pub const PRIORITY: i32 = -8;

/// The bit of reaction type `id` and which of the two masks holds it (0x59c08c for types below 32, 0x571d38 for the
/// rest; the exe masks the shift to 5 bits, so type 0x41 shares type 0x21's bit).
pub fn bit(id: u32) -> (usize, u32) {
    if id < 32 {
        (0, 1 << (id & 31))
    } else {
        (1, 1 << ((id - 32) & 31))
    }
}

impl Club {
    /// The narration sentence about the golfer for reaction type `id` with argument `arg` (the switch at the end of
    /// 0x467a00, which writes the golfer's name and then the sentence), or None for the types that have none (those are
    /// never narrated).
    pub fn narration_verdict(&self, c: &Course, id: u32, arg: i32, g: usize) -> Option<String> {
        let he = if self.male(g) != 0 { tr("STR_NARR_HE") } else { tr("STR_NARR_SHE") };
        let name = self.vip_name(g);
        let key = match id {
            1 => "STR_NARR_01_TOUGH_SHOT",
            2 => "STR_NARR_02_EASY_SHOT_IN",
            3 => "STR_NARR_03_WORSE_LOCATION",
            4 => "STR_NARR_04_NO_DANGERS",
            5 | 0x25 | 0x26 => "STR_NARR_05_CHALLENGING",
            6 => "STR_NARR_06_SLOPE",
            8 => "STR_NARR_08_NO_SAFE_PLACE",
            9 => "STR_NARR_09_ALMOST_HIT",
            10 => "STR_NARR_0A_NO_PATH",
            11 => "STR_NARR_0B_SCENIC",
            12 => "STR_NARR_0C_BOUNCED_OFF_TREE",
            13 => "STR_NARR_0D_ENDED_UP_IN",
            14 => "STR_NARR_0E_SODA",
            15 => "STR_NARR_0F_SNACK",
            16 => "STR_NARR_10_CURVED_LEFT",
            17 => "STR_NARR_11_FADED_RIGHT",
            18 => "STR_NARR_12_TASTY_SNACK",
            0x14 => "STR_NARR_14_EYESORE",
            0x15 => "STR_NARR_15_WAITING",
            0x16 => "STR_NARR_16_VACATION_HOME",
            0x18 => "STR_NARR_18_DANDELIONS",
            0x19 => "STR_NARR_19_BEVERAGE",
            0x1a => "STR_NARR_1A_BENCH_NEEDED",
            0x1b => "STR_NARR_1B_BENCH",
            0x1c => "STR_NARR_1C_LOVELY",
            0x1d => "STR_NARR_1D_DIFFERENT",
            0x1e => "STR_NARR_1E_SIMILAR",
            0x1f => "STR_NARR_1F_DIFFICULT",
            0x20 | 0x21 => "STR_NARR_20_DECISION",
            0x23 => "STR_NARR_23_QUIT",
            0x24 => "STR_NARR_24_ANGRY_GOLFER",
            0x27 => "STR_NARR_27_WILDLIFE",
            0x33 => "STR_NARR_33_DRIVING_RANGE",
            0x34 => "STR_NARR_34_PRO_SHOP",
            0x35 => "STR_NARR_35_PUTTING_GREEN",
            0x36 => "STR_NARR_36_NEW_CLUB",
            0x37 => "STR_NARR_37_DRAW",
            0x38 => "STR_NARR_38_FADE",
            0x39 => "STR_NARR_39_HIGH",
            0x3c => "STR_NARR_3C_LOW",
            0x41 => "STR_NARR_41_CANNOT_SEE",
            _ => return None,
        };
        let s = match id {
            // the terrain table's singular name (0x578350 + 0x30 * arg), as the theme renames it
            2 | 3 | 10 | 13 => trf(key, &[("GOLFER", &name), ("TERRAIN", &type_name(arg.clamp(0, 22) as u8, c.theme, false)), ("HE", &he)]),
            // the scenery name of a tile (0x407700)
            11 | 0x14 | 0x1c => trf(key, &[("GOLFER", &name), ("WHAT", &feature_at(c, arg, c.theme))]),
            0x16 => {
                let who = self.celebrities.get(arg.max(0) as usize).map(|p| p.name.as_str()).unwrap_or("");
                trf(key, &[("GOLFER", &name), ("CELEBRITY", &who)])
            }
            _ => trf(key, &[("GOLFER", &name), ("HE", &he)]),
        };
        Some(s)
    }

    /// The whole narration message for reaction `id`: the thought in quotes, then the narration sentence.
    pub fn narration_text(&self, c: &Course, id: u32, arg: i32, g: usize) -> Option<String> {
        let verdict = self.narration_verdict(c, id, arg, g)?;
        let thought = self.thought(c, id, arg, g).text;
        Some(trf("STR_NARR_MESSAGE", &[("THOUGHT", &thought), ("NARRATION", &verdict)]))
    }

    /// Whether reaction `id` of golfer g may be narrated now (0x467a00): not type 0x32, more than `GAP` ticks since the last
    /// narration, the golfer not in a conversation (+0xae), the type not narrated yet this game, and the golfer drawn
    /// strictly inside x 100..700, y 100..400 of the screen.
    pub fn may_narrate(&self, g: usize, id: u32) -> bool {
        let (m, b) = bit(id);
        let gg = &self.g[g];
        id != 0x32
            && self.tick.wrapping_sub(self.last_narration) as i32 > GAP as i32
            && gg.field_ae == 0
            && self.narrated[m] & b == 0
            && 100 < gg.sx
            && gg.sx < 700
            && 100 < gg.sy
            && gg.sy < 400
    }

    /// Posts the first-time narration of reaction `id` when it may be (see `may_narrate`) and the advisor option is on; the
    /// type is marked narrated and the gap restarted only when the message window took the message.
    pub(crate) fn narrate(&mut self, c: &Course, g: usize, id: u32, arg: i32) {
        if !self.may_narrate(g, id) {
            return;
        }
        let Some(text) = self.narration_text(c, id, arg, g) else { return };
        if !self.advisor || !self.message_by(text, g as i32, PRIORITY) {
            return;
        }
        let (m, b) = bit(id);
        self.narrated[m] |= b;
        self.last_narration = self.tick;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golfer::Event;

    fn club() -> (Course, Club) {
        let mut cl = Club::default();
        cl.g[2].roster = 4;
        cl.g[3].roster = 5;
        cl.g[2].sx = 400;
        cl.g[2].sy = 300;
        cl.tick = 0x2c00;
        (Course::default(), cl)
    }

    #[test]
    fn verdicts() {
        let (c, cl) = club();
        let he = if cl.male(2) != 0 { "he" } else { "she" };
        let name = cl.vip_name(2);
        assert_eq!(
            cl.narration_verdict(&c, 2, 7, 2).unwrap(),
            format!(
                "{name} is disappointed that an easy looking shot ended up in the sand trap. If it happens again {he} will become discouraged."
            )
        );
        assert_eq!(
            cl.narration_verdict(&c, 10, 17, 2).unwrap(),
            format!("{name} is unhappy because no path has been provided to walk through the water. Press 'p' to select the path tool.")
        );
        assert_eq!(
            cl.narration_verdict(&c, 13, 13, 2).unwrap(),
            format!("{name}'s ball somehow ended up in the tree. {name} will be more careful next time.")
        );
        assert_eq!(cl.narration_verdict(&c, 0x26, 0, 2), cl.narration_verdict(&c, 5, 0, 2));
        assert!(cl.narration_verdict(&c, 7, 0, 2).is_none());
        assert!(cl.narration_verdict(&c, 0x13, 0, 2).is_none());
        let mut d = c.clone();
        d.theme = 3;
        assert!(cl.narration_verdict(&d, 3, 10, 2).unwrap().ends_with("the burn."));
        let t = cl.narration_text(&c, 1, 0, 2).unwrap();
        assert!(t.starts_with('\''));
        assert!(t.ends_with(&format!("' {name} has successfully made a tough-looking shot and is enjoying your course.")));
    }

    #[test]
    fn once_per_game() {
        let (c, mut cl) = club();
        assert_eq!(bit(3), (0, 8));
        assert_eq!(bit(0x41), bit(0x21));
        cl.narrate(&c, 2, 1, 0);
        assert!(matches!(cl.out.last(), Some(Event::Message { speaker: 2, priority: PRIORITY, .. })));
        assert_eq!(cl.narrated, [2, 0]);
        assert_eq!(cl.last_narration, 0x2c00);
        // too soon for another, and the same type never again
        cl.ticker_busy = false;
        cl.tick += GAP;
        assert!(!cl.may_narrate(2, 6));
        cl.tick += 1;
        assert!(cl.may_narrate(2, 6));
        assert!(!cl.may_narrate(2, 1));
        // off screen, in a conversation, type 0x32
        cl.g[2].sx = 700;
        assert!(!cl.may_narrate(2, 6));
        cl.g[2].sx = 400;
        cl.g[2].field_ae = 1;
        assert!(!cl.may_narrate(2, 6));
        cl.g[2].field_ae = 0;
        assert!(!cl.may_narrate(2, 0x32));
        // a busy ticker or the option off: nothing marked
        cl.ticker_busy = true;
        cl.narrate(&c, 2, 6, 0);
        assert_eq!(cl.narrated, [2, 0]);
        cl.ticker_busy = false;
        cl.advisor = false;
        cl.narrate(&c, 2, 0x33, 0);
        assert_eq!(cl.narrated, [2, 0]);
        cl.advisor = true;
        cl.narrate(&c, 2, 0x33, 0);
        assert_eq!(cl.narrated, [2, 1 << 0x13]);
    }
}
