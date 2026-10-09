//! First-time narration (the tail of the reaction routine 0x467a00): the first time each reaction type fires in a game, the
//! message window explains it once: `'` + the golfer's thought + `' ` + the golfer's name + a sentence for the type.
//!
//! Every rule and string here is from the publisher's golf.exe (the English text is the game's own).

use crate::course::Course;
use crate::golfer::Club;
use crate::thoughts::feature;
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
    /// The narration sentence that follows the golfer's name for reaction type `id` with argument `arg` (the switch at the
    /// end of 0x467a00), or None for the types that have none (those are never narrated).
    pub fn narration_verdict(&self, c: &Course, id: u32, arg: i32, g: usize) -> Option<String> {
        let he = if self.male(g) != 0 { "he" } else { "she" };
        let name = self.vip_name(g);
        // the terrain table's singular name (0x578350 + 0x30 * arg), as the theme renames it
        let ground = || type_name(arg.clamp(0, 22) as u8, c.theme, false);
        // the scenery name of a tile (0x407700)
        let thing = || feature(c, arg / 50, arg % 50, c.theme);
        const HIRE: &str = " Click the employees button on the People menu to hire course employees.";
        let s = match id {
            1 => " has successfully made a tough-looking shot and is enjoying your course.".to_string(),
            2 => format!(
                " is disappointed that an easy looking shot ended up in the {}. If it happens again {he} will become discouraged.",
                ground()
            ),
            3 => format!(" is disappointed that the shot has ended up in a worse location, the {}.", ground()),
            4 => {
                "'s next shot does not involve many dangers or hazards. Too many shots like this may tend to bore your players.".to_string()
            }
            5 | 0x25 | 0x26 => format!("'s next shot looks challenging - {he} will be happy if lands on the green or fairway."),
            6 => " plans to use the slope of the hill to run the ball towards the hole.".to_string(),
            8 => " cannot find a safe place to aim the next shot.  This can be frustrating. Perhaps you should add more fairway to \
                  this hole."
                .to_string(),
            9 => " was almost hit by a shot from another hole. This is very scary.  Try to keep your holes seperated, sometimes \
                  trees can be useful."
                .to_string(),
            10 => format!(
                " is unhappy because no path has been provided to walk through the {}. Press 'p' to select the path tool.",
                ground()
            ),
            11 => format!(" is pleased to have a scenic {} in view for the next shot.", thing()),
            12 => format!("'s ball just bounced off a tree. {name} will be more careful next time."),
            13 => format!("'s ball somehow ended up in the {}. {name} will be more careful next time.", ground()),
            14 => format!(" needs to find a soda vendor soon or {he} will be unhappy."),
            15 => format!(" needs to find a snack bar soon or {he} will be unhappy."),
            16 => " is amazed that the last shot curved off to the left.".to_string(),
            17 => " is amazed that the last shot faded off to the right.".to_string(),
            18 => " is enjoying a tasty snack at your conveniently located snack bar.".to_string(),
            0x14 => format!(" is disappointed to see a {} in the middle of your otherwise lovely golf course.", thing()),
            0x15 => format!(
                " is frustrated that a lot of golfers are waiting at this hole. Placing a Ranger/Marshall near the tee will speed \
                 up play on this hole.{HIRE}"
            ),
            0x16 => {
                let who = self.celebrities.get(arg.max(0) as usize).map(|p| p.name.as_str()).unwrap_or("");
                format!(" is excited to see {who}'s vacation home on your golf course.")
            }
            0x18 => format!(
                " is disappointed to see dandelions growing in the middle of your otherwise lovely golf course. Hire a \
                 groundskeeper to eradicate these weeds.{HIRE}"
            ),
            0x19 => " is enjoying a cool refreshing beverage from your friendly soda vendor.".to_string(),
            0x1a => format!(" needs to find a bench to rest on soon or {he} will start to slow down."),
            0x1b => " is relaxing on your comfy bench and will soon resume play with renewed energy.".to_string(),
            0x1c => format!(" is enjoying the lovely {} you have included in your course.", thing()),
            0x1d => " likes the fact that this hole is very different from the last hole.".to_string(),
            0x1e => " dislikes the fact that this hole is very similar to the last hole. Try to vary the par and direction of \
                     your holes."
                .to_string(),
            0x1f => format!("'s next shot looks extremely difficult. {name} will be thrilled if it finds the green or fairway."),
            0x20 | 0x21 => {
                " has an interesting decision to make on this next shot. This is a characteristic of a well designed hole.".to_string()
            }
            0x23 => " has quit playing in frustration. Security may be required.".to_string(),
            0x24 => " does not enjoy seeing an angry golfer on your course.".to_string(),
            0x27 => " is intrigued by the wildlife on your course.".to_string(),
            0x33 => " appreciates the new driving range you added to your course.".to_string(),
            0x34 => " appreciates the new pro shop you added to your course.".to_string(),
            0x35 => " appreciates the new putting green you added to your course.".to_string(),
            0x36 => " is using this club for the first time today. A good course will use every club in the bag.".to_string(),
            0x37 => " is setting up to hit a draw shot which curves from right to left.".to_string(),
            0x38 => " is setting up to hit a fade shot which curves from left to right.".to_string(),
            0x39 => " is setting up to hit a high shot which will spin back once it hits the green.".to_string(),
            0x3c => " is setting up to hit a low shot which will run under the trees.".to_string(),
            0x41 => " hit a good shot but cannot see exactly where it landed because of trees or elevation. ".to_string(),
            _ => return None,
        };
        Some(s)
    }

    /// The whole narration message for reaction `id`: the thought in quotes, the golfer's name and the verdict.
    pub fn narration_text(&self, c: &Course, id: u32, arg: i32, g: usize) -> Option<String> {
        let verdict = self.narration_verdict(c, id, arg, g)?;
        let thought = self.thought(c, id, arg, g).text;
        Some(format!("'{thought}' {}{verdict}", self.vip_name(g)))
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
                " is disappointed that an easy looking shot ended up in the sand trap. If it happens again {he} will become discouraged."
            )
        );
        assert_eq!(
            cl.narration_verdict(&c, 10, 17, 2).unwrap(),
            " is unhappy because no path has been provided to walk through the water. Press 'p' to select the path tool."
        );
        assert_eq!(
            cl.narration_verdict(&c, 13, 13, 2).unwrap(),
            format!("'s ball somehow ended up in the tree. {name} will be more careful next time.")
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
