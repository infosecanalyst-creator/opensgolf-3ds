//! What golfers think and say (text routine 0x469b00): one line per mood event, chosen by the pair's relationship, the
//! person, gender, the hole and the event's argument, with the golfer's and partner's names and an event noun put in.
//!
//! Rules are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Golfer thoughts"). The exe's strings are data that
//! is not available beyond the first words its labels show; the lines here start with those words where they are known and
//! are otherwise written for this port.

use crate::course::{f, idx, inside, Course};
use crate::golfer::Club;
use crate::lang::{tr, trf};

/// The tone of a line: neutral, pleased, upset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Good,
    Bad,
}

/// A line of thought: its text, its tone, and whether the partner says it (about or to the golfer).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub tone: Tone,
    pub partner: bool,
}

/// Terrain names: the single name and the group name (trees use the group name in most remarks).
pub fn terrain(t: u8) -> (&'static str, &'static str) {
    match t {
        0 => (tr("STR_THOUGHT_TERRAIN_TEE"), tr("STR_THOUGHT_TERRAIN_TEE_GROUP")),
        1 => (tr("STR_THOUGHT_TERRAIN_GREEN"), tr("STR_THOUGHT_TERRAIN_GREEN_GROUP")),
        2 | 3 => (tr("STR_THOUGHT_TERRAIN_FAIRWAY"), tr("STR_THOUGHT_TERRAIN_FAIRWAY_GROUP")),
        4 => (tr("STR_THOUGHT_TERRAIN_ROUGH"), tr("STR_THOUGHT_TERRAIN_ROUGH_GROUP")),
        5 => (tr("STR_THOUGHT_TERRAIN_DEEP_ROUGH"), tr("STR_THOUGHT_TERRAIN_DEEP_ROUGH_GROUP")),
        6 => (tr("STR_THOUGHT_TERRAIN_MOUNDS"), tr("STR_THOUGHT_TERRAIN_MOUNDS_GROUP")),
        7 => (tr("STR_THOUGHT_TERRAIN_SAND_TRAP"), tr("STR_THOUGHT_TERRAIN_SAND_TRAP_GROUP")),
        8 => (tr("STR_THOUGHT_TERRAIN_WASTE_BUNKER"), tr("STR_THOUGHT_TERRAIN_WASTE_BUNKER_GROUP")),
        9 => (tr("STR_THOUGHT_TERRAIN_POT_BUNKER"), tr("STR_THOUGHT_TERRAIN_POT_BUNKER_GROUP")),
        10 => (tr("STR_THOUGHT_TERRAIN_RAVINE"), tr("STR_THOUGHT_TERRAIN_RAVINE_GROUP")),
        11 => (tr("STR_THOUGHT_TERRAIN_BRUSH"), tr("STR_THOUGHT_TERRAIN_BRUSH_GROUP")),
        12 => (tr("STR_THOUGHT_TERRAIN_ROCKS"), tr("STR_THOUGHT_TERRAIN_ROCKS_GROUP")),
        13 => (tr("STR_THOUGHT_TERRAIN_TREE"), tr("STR_THOUGHT_TERRAIN_TREE_GROUP")),
        14 => (tr("STR_THOUGHT_TERRAIN_PINE"), tr("STR_THOUGHT_TERRAIN_PINE_GROUP")),
        15 => (tr("STR_THOUGHT_TERRAIN_PALM"), tr("STR_THOUGHT_TERRAIN_PALM_GROUP")),
        16 => (tr("STR_THOUGHT_TERRAIN_ELM"), tr("STR_THOUGHT_TERRAIN_ELM_GROUP")),
        17 => (tr("STR_THOUGHT_TERRAIN_WATER"), tr("STR_THOUGHT_TERRAIN_WATER_GROUP")),
        18 => (tr("STR_THOUGHT_TERRAIN_WETLANDS"), tr("STR_THOUGHT_TERRAIN_WETLANDS_GROUP")),
        19 => (tr("STR_THOUGHT_TERRAIN_MARSH"), tr("STR_THOUGHT_TERRAIN_MARSH_GROUP")),
        20 => (tr("STR_THOUGHT_TERRAIN_OUT_OF_BOUNDS"), tr("STR_THOUGHT_TERRAIN_OUT_OF_BOUNDS_GROUP")),
        _ => (tr("STR_THOUGHT_TERRAIN_BUILDING"), tr("STR_THOUGHT_TERRAIN_BUILDING_GROUP")),
    }
}

/// Club names by club number (0x40a9a0).
pub const CLUB_KEYS: [&str; 14] = [
    "STR_CLUB_DRIVER",
    "STR_CLUB_3_WOOD",
    "STR_CLUB_4_WOOD",
    "STR_CLUB_2_IRON",
    "STR_CLUB_3_IRON",
    "STR_CLUB_4_IRON",
    "STR_CLUB_5_IRON",
    "STR_CLUB_6_IRON",
    "STR_CLUB_7_IRON",
    "STR_CLUB_8_IRON",
    "STR_CLUB_9_IRON",
    "STR_CLUB_LOB_WEDGE",
    "STR_CLUB_SAND_WEDGE",
    "STR_CLUB_PUTTER",
];

/// The name of club k (0..13).
pub fn club_name(k: usize) -> &'static str {
    tr(CLUB_KEYS[k])
}

const PETS: [&str; 4] = ["STR_THOUGHT_PET_HONEY", "STR_THOUGHT_PET_SUGAR", "STR_THOUGHT_PET_DEAR", "STR_THOUGHT_PET_SWEETIE"];

fn is_tree(t: u8) -> bool {
    crate::data::row(t).class == 13
}

/// The name of the scenery on a tile (0x407700, simplified to what this port tracks): landmarks, flower beds, scenic trees,
/// fountains, bridges and the like.
pub fn feature(c: &Course, a: i32, b: i32, theme: u8) -> String {
    if !inside(a, b) {
        return tr("STR_THOUGHT_FEATURE_VIEW").into();
    }
    let i = idx(a, b);
    let t = c.ty[i];
    let fl = c.flags[i];
    if t == 21 {
        return tr("STR_THOUGHT_FEATURE_HOME_SITE").into();
    }
    if t == 22 {
        let o = c.object_at(a, b);
        if let Some(obj) = (o >= 0).then(|| c.objects.get(o as usize)).flatten() {
            if obj.kind == 4 {
                return crate::vips::landmark_name(obj.sub).to_lowercase();
            }
            if obj.kind == 2 {
                return tr("STR_THOUGHT_FEATURE_FLOWERBED").into();
            }
        }
        return tr("STR_THOUGHT_FEATURE_LANDMARK").into();
    }
    if fl & f::FLOWERS != 0 {
        let k = (a * 7 + b) % 5;
        return match theme {
            1 => [
                tr("STR_THOUGHT_FEATURE_RED_POPPY_BED"),
                tr("STR_THOUGHT_FEATURE_ORANGE_FLOWERBED"),
                tr("STR_THOUGHT_FEATURE_PURPLE_IRIS_PATCH"),
                tr("STR_THOUGHT_FEATURE_BLUE_FOXGLOVE"),
                tr("STR_THOUGHT_FEATURE_PINK_SAGE_BUSH"),
            ][k as usize],
            2 => [
                tr("STR_THOUGHT_FEATURE_AQUA_HIBISCUS"),
                tr("STR_THOUGHT_FEATURE_ORANGE_BOUGAINVILLEA"),
                tr("STR_THOUGHT_FEATURE_PURPLE_ORCHID"),
                tr("STR_THOUGHT_FEATURE_BLUE_HYDRANGEA"),
                tr("STR_THOUGHT_FEATURE_RED_ROSE_BUSH"),
            ][k as usize],
            _ => [
                tr("STR_THOUGHT_FEATURE_GREEN_FLOWERBED"),
                tr("STR_THOUGHT_FEATURE_YELLOW_DAISY_BED"),
                tr("STR_THOUGHT_FEATURE_PURPLE_IRIS_PATCH"),
                tr("STR_THOUGHT_FEATURE_WHITE_CARNATION"),
                tr("STR_THOUGHT_FEATURE_RED_ROSE_BUSH"),
            ][k as usize],
        }
        .into();
    }
    match crate::data::row(t).class {
        4 if t == 4 => {
            if theme == 1 { tr("STR_THOUGHT_FEATURE_MYSTERIOUS_BONES") } else { tr("STR_THOUGHT_FEATURE_ORNAMENTAL_GRASS") }.into()
        }
        4 if t == 10 => if theme == 1 { tr("STR_THOUGHT_FEATURE_ROCK_FORMATION") } else { tr("STR_THOUGHT_FEATURE_SCENIC_TREE") }.into(),
        4 if t == 12 => if theme == 1 { tr("STR_THOUGHT_FEATURE_NATURAL_BRIDGE") } else { tr("STR_THOUGHT_FEATURE_ROCK_FORMATION") }.into(),
        4 => tr("STR_THOUGHT_FEATURE_ROSE_BUSH").into(),
        7 => tr("STR_THOUGHT_FEATURE_RHODODENDRON").into(),
        12 => tr("STR_THOUGHT_FEATURE_ROCK_FORMATION").into(),
        13 => if theme == 1 { tr("STR_THOUGHT_FEATURE_SCENIC_CACTUS") } else { tr("STR_THOUGHT_FEATURE_SCENIC_TREE") }.into(),
        17 if fl & f::PATH != 0 => tr("STR_THOUGHT_FEATURE_SCENIC_BRIDGE").into(),
        17 => if theme == 1 { tr("STR_THOUGHT_FEATURE_ROCK_FORMATION") } else { tr("STR_THOUGHT_FEATURE_FOUNTAIN") }.into(),
        18 => tr("STR_THOUGHT_FEATURE_FOUNTAIN").into(),
        _ => tr("STR_THOUGHT_FEATURE_WILDFLOWER").into(),
    }
}

impl Club {
    fn rel(&self, g: usize) -> i32 {
        self.g[g].field_ae
    }

    fn pet(g: usize) -> &'static str {
        tr(PETS[g & 3])
    }

    /// The text of mood event `id` with argument `arg` for golfer g (0x469b00).
    pub fn thought(&self, c: &Course, id: u32, arg: i32, g: usize) -> Line {
        self.thought_as(c, id, arg, g, None, true)
    }

    /// The stock text of an event as golfer g would say it if g were person `record` (the Customise screen asks this for
    /// its preview slot 0x99, which is past the golfer slots and so never takes a person's own dialogue slot).
    pub fn stock_thought(&self, c: &Course, id: u32, arg: i32, g: usize, record: usize) -> Line {
        self.thought_as(c, id, arg, g, Some(record as i32), false)
    }

    fn thought_as(&self, c: &Course, id: u32, arg: i32, g: usize, record: Option<i32>, own: bool) -> Line {
        let rel = self.rel(g);
        let r = record.unwrap_or(self.g[g].roster.max(0));
        let male = match record {
            Some(k) => self.roster.get(k as usize).map(|p| p.male_bit()).unwrap_or(1) != 0,
            None => self.male(g) != 0,
        };
        let p = g ^ 1;
        let me = self.name(g);
        let partner = self.name(p);
        let theme = c.theme;
        let mut tone = Tone::Plain;
        let mut by_partner = false;
        let ty = arg.clamp(0, 22) as u8;
        let (t1, t2) = terrain(ty);
        let place = if is_tree(ty) {
            trf("STR_THOUGHT_PLACE_UNDER", &[("TERRAIN", &t1)])
        } else {
            trf("STR_THOUGHT_PLACE_IN", &[("TERRAIN", &t1)])
        };
        let noun = if is_tree(ty) { t2 } else { t1 };
        let hole = self.g[g].hole.clamp(0, 18) as usize;
        // a person's own dialogue slot for this event comes first (0x469b00 for golfer slots below 0x98)
        if own && g < crate::golfer::SLOTS {
            let own = crate::roster::SAYING_CODES.iter().position(|&c| c == id).and_then(|row| {
                self.roster.get(r as usize).and_then(|p| p.saying(row)).map(|s| s.replace("PARTNER", &partner).replace("MYNAME", &me))
            });
            if let Some(text) = own {
                return Line { text: text.replace("DATA", t1), tone, partner: false };
            }
        }
        let text: String = match id {
            0x01 => {
                tone = Tone::Good;
                if rel & 1 == 0 {
                    match (r & 1, r & 2) {
                        (0, 0) => trf("STR_THOUGHT_01_HOWD_YOU_LIKE", &[("PARTNER", &partner)]),
                        (0, _) => trf("STR_THOUGHT_01_NOTHIN_BUT", &[("TERRAIN", &t1)]),
                        (_, 0) => tr("STR_THOUGHT_01_SO_GOOD").into(),
                        _ => tr("STR_THOUGHT_01_I_ROCK").into(),
                    }
                } else {
                    by_partner = true;
                    let pm = self.male(p) != 0;
                    let pr = self.g[p].roster & 1;
                    match (pr, pm) {
                        (0, true) => trf("STR_THOUGHT_01_PARTNER_COULDNT", &[("GOLFER", &me)]),
                        (0, false) => trf("STR_THOUGHT_01_PARTNER_SOOOO_GOOD", &[("GOLFER", &me)]),
                        (_, true) => trf("STR_THOUGHT_01_PARTNER_GREAT_SHOT", &[("GOLFER", &me)]),
                        _ => trf("STR_THOUGHT_01_PARTNER_TRULY_ROCK", &[("GOLFER", &me)]),
                    }
                }
            }
            0x02 => {
                tone = Tone::Bad;
                match rel {
                    2 => {
                        by_partner = true;
                        trf("STR_THOUGHT_02_HA_YOURE", &[("GOLFER", &me), ("PLACE", &place)])
                    }
                    4 => {
                        by_partner = true;
                        trf("STR_THOUGHT_02_OH_DARN_YOURE", &[("PLACE", &place), ("PET", &Self::pet(g))])
                    }
                    _ => trf("STR_THOUGHT_02_DARN_IM", &[("PLACE", &place)]),
                }
            }
            0x03 => {
                tone = Tone::Bad;
                match rel {
                    2 => {
                        by_partner = true;
                        trf("STR_THOUGHT_03_EVEN_BETTER", &[("PLACE", &place), ("GOLFER", &me)])
                    }
                    3 => {
                        by_partner = true;
                        trf("STR_THOUGHT_03_NOW_YOURE", &[("GOLFER", &me), ("PLACE", &place)])
                    }
                    4 => {
                        by_partner = true;
                        trf("STR_THOUGHT_03_PREFER_BEING", &[("PLACE", &place)])
                    }
                    _ => trf(
                        "STR_THOUGHT_03_NOW_IM",
                        &[
                            (
                                "EXCLAMATION",
                                &[
                                    tr("STR_THOUGHT_03_EXCL_GREAT"),
                                    tr("STR_THOUGHT_03_EXCL_OH_NO"),
                                    tr("STR_THOUGHT_03_EXCL_SWELL"),
                                    tr("STR_THOUGHT_03_EXCL_LOVELY"),
                                ][(r & 3) as usize],
                            ),
                            ("PLACE", &place),
                        ],
                    ),
                }
            }
            0x04 => {
                tone = Tone::Bad;
                if rel == 4 {
                    by_partner = true;
                    trf("STR_THOUGHT_04_ONLY_YOU", &[("GOLFER", &me)])
                } else {
                    tr("STR_THOUGHT_04_PRETTY_EASY").into()
                }
            }
            0x05 | 0x25 | 0x26 => match rel {
                1 => {
                    by_partner = true;
                    trf("STR_THOUGHT_05_CAREFUL_OF", &[("TERRAIN", &noun), ("PET", &Self::pet(g))])
                }
                2 if g & 1 == 0 => {
                    by_partner = true;
                    trf("STR_THOUGHT_05_SAY_HELLO", &[("TERRAIN", &noun), ("GOLFER", &me)])
                }
                2 => {
                    by_partner = true;
                    trf("STR_THOUGHT_05_GOING_INTO", &[("TERRAIN", &noun), ("GOLFER", &me)])
                }
                3 | 5 => {
                    by_partner = true;
                    trf("STR_THOUGHT_05_WATCH_OUT", &[("TERRAIN", &noun), ("GOLFER", &me)])
                }
                _ => match id {
                    0x05 => trf("STR_THOUGHT_05_STAY_AWAY", &[("TERRAIN", &noun)]),
                    0x25 => trf("STR_THOUGHT_25_HIT_IT_PAST", &[("TERRAIN", &noun)]),
                    _ => trf("STR_THOUGHT_26_WATCH_OUT_FOR", &[("TERRAIN", &noun)]),
                },
            },
            0x06 => {
                tone = Tone::Good;
                tr("STR_THOUGHT_06_USE_THE_SLOPE").into()
            }
            0x07 => {
                tone = Tone::Good;
                if self.g[g].flags & 0x10000 != 0 {
                    tr("STR_THOUGHT_07_RIDING_OVER_BRIDGE").into()
                } else {
                    tr("STR_THOUGHT_07_WALKING_OVER_BRIDGE").into()
                }
            }
            0x08 => {
                tone = Tone::Bad;
                tr("STR_THOUGHT_08_WHO_DESIGNED").into()
            }
            0x09 => {
                tone = Tone::Bad;
                tr("STR_THOUGHT_09_ALMOST_HIT_ME").into()
            }
            0x0a => {
                tone = Tone::Bad;
                let w = if t1.ends_with('s') { tr("STR_THOUGHT_0A_THESE") } else { tr("STR_THOUGHT_0A_THIS") };
                trf("STR_THOUGHT_0A_MUST_I_WALK", &[("THIS", &w), ("TERRAIN", &t1)])
            }
            0x0b => {
                tone = Tone::Good;
                let what = feature(c, arg / crate::course::n(), arg % crate::course::n(), theme);
                let adj = if arg & 0x100 != 0 { tr("STR_THOUGHT_0B_NICE") } else { tr("STR_THOUGHT_0B_BEAUTIFUL") };
                if rel == 1 {
                    trf("STR_THOUGHT_0B_LOOK_AT_THIS", &[("ADJ", &adj), ("WHAT", &what), ("PET", &Self::pet(p))])
                } else {
                    trf("STR_THOUGHT_0B_CHECK_OUT", &[("ADJ", &adj), ("WHAT", &what)])
                }
            }
            0x0c => {
                tone = Tone::Bad;
                trf("STR_THOUGHT_0C_DARN_THAT", &[("TERRAIN", &t1)])
            }
            0x0d => {
                tone = Tone::Bad;
                if rel == 2 || rel == 4 {
                    by_partner = true;
                    trf("STR_THOUGHT_0D_HEAR_A_SPLASH", &[("GOLFER", &me)])
                } else {
                    [tr("STR_THOUGHT_0D_DROWNED_IT"), tr("STR_THOUGHT_0D_SWIM"), tr("STR_THOUGHT_0D_IN_THE_DRINK")][(r % 3) as usize].into()
                }
            }
            0x0e => {
                tone = Tone::Bad;
                tr("STR_THOUGHT_0E_THIRSTY").into()
            }
            0x0f => {
                tone = Tone::Bad;
                tr("STR_THOUGHT_0F_HUNGRY").into()
            }
            0x10 => tr("STR_THOUGHT_10_HOOKED").into(),
            0x11 => tr("STR_THOUGHT_11_SLICED").into(),
            0x12 => {
                tone = Tone::Good;
                tr("STR_THOUGHT_12_GOOD_SNACK").into()
            }
            0x13 | 0x17 => {
                // the hole just finished: the golfer has moved on to the next one by the time the thought shows, so take it
                // from the thought's stamp (hole * 11 + strokes, written when the event fired)
                let gg = &self.g[g];
                let hole = if gg.thought as u32 == id { (gg.thought_stamp / 11) as usize } else { hole }.min(18);
                let par = self.holes[hole].par;
                let d = self.g[g].card[hole] as i32 - par;
                let word = match d {
                    -4 => tr("STR_THOUGHT_SCORE_TRIPLE_EAGLE"),
                    -3 => tr("STR_THOUGHT_SCORE_DOUBLE_EAGLE"),
                    -2 => tr("STR_THOUGHT_SCORE_EAGLE"),
                    -1 => tr("STR_THOUGHT_SCORE_BIRDIE"),
                    0 => tr("STR_THOUGHT_SCORE_PAR"),
                    1 => tr("STR_THOUGHT_SCORE_BOGEY"),
                    2 => tr("STR_THOUGHT_SCORE_DOUBLE_BOGEY"),
                    3 => tr("STR_THOUGHT_SCORE_TRIPLE_BOGEY"),
                    _ => tr("STR_THOUGHT_SCORE_ARGGG"),
                };
                let fl = self.holes[hole].flags;
                let remark = if fl & 4 != 0 && d >= 2 {
                    tone = Tone::Bad;
                    tr("STR_THOUGHT_13_TOO_HARD").to_string()
                } else if fl & 8 != 0 && d < 0 {
                    tone = Tone::Bad;
                    tr("STR_THOUGHT_13_TOO_EASY").to_string()
                } else if id == 0x17 {
                    tone = Tone::Bad;
                    tr("STR_THOUGHT_17_TOO_HARD_EASY").to_string()
                } else {
                    let s = (g + hole) & 3;
                    let class = self.g[g].class;
                    let mask = match s {
                        0 if class & 1 != 0 => 0x100,
                        1 if class & 2 != 0 => 0x200,
                        2 if class & 4 != 0 => 0x400,
                        _ => 0,
                    };
                    if mask & fl != 0 && d <= 0 {
                        match mask {
                            0x100 => tr("STR_THOUGHT_13_LENGTH_HELPS"),
                            0x200 => tr("STR_THOUGHT_13_REWARDS_ACCURACY"),
                            _ => tr("STR_THOUGHT_13_IMAGINATIVE"),
                        }
                        .to_string()
                    } else if (rel == 2 || rel == 4) && self.g[p].card[hole] != 0 {
                        if self.g[p].card[hole] > self.g[g].card[hole] {
                            tr("STR_THOUGHT_13_TAKE_THAT").into()
                        } else {
                            tr("STR_THOUGHT_13_YOU_WERE_LUCKY").into()
                        }
                    } else {
                        course_opinion(arg, d).into()
                    }
                };
                trf("STR_THOUGHT_13_HOLE_DONE", &[("SCORE", &word), ("REMARK", &remark)])
            }
            0x14 => {
                tone = Tone::Bad;
                trf("STR_THOUGHT_14_UGLY", &[("WHAT", &feature(c, arg / crate::course::n(), arg % crate::course::n(), theme))])
            }
            0x15 => {
                tone = Tone::Bad;
                tr("STR_THOUGHT_15_SLOWPOKES").into()
            }
            0x16 => {
                tone = Tone::Good;
                let who = self
                    .celebrities
                    .get(arg.max(0) as usize)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| tr("STR_THOUGHT_16_A_MOVIE_STAR").into());
                trf("STR_THOUGHT_16_THATS_HOUSE", &[("CELEBRITY", &who)])
            }
            0x18 => {
                tone = Tone::Bad;
                tr("STR_THOUGHT_18_SOMETHING_GROWING").into()
            }
            0x19 => {
                tone = Tone::Good;
                tr("STR_THOUGHT_19_FOAMY_BEVERAGE").into()
            }
            0x1a => match rel {
                1 => {
                    by_partner = true;
                    trf("STR_THOUGHT_1A_YOU_LOOK_TIRED_PET", &[("PET", &Self::pet(g))])
                }
                3 | 5 => {
                    by_partner = true;
                    trf("STR_THOUGHT_1A_YOU_LOOK_TIRED", &[("GOLFER", &me)])
                }
                _ => tr("STR_THOUGHT_1A_GETTING_TIRED").into(),
            },
            0x1b => {
                tone = Tone::Good;
                tr("STR_THOUGHT_1B_COMFORTABLE_BENCH").into()
            }
            0x1c => {
                tone = Tone::Good;
                let what = feature(c, arg / crate::course::n(), arg % crate::course::n(), theme);
                match rel {
                    1 => trf("STR_THOUGHT_1C_LOVELY_PET", &[("WHAT", &what), ("PET", &Self::pet(p))]),
                    3 | 5 => trf("STR_THOUGHT_1C_LOVELY_PARTNER", &[("WHAT", &what), ("PARTNER", &partner)]),
                    _ => match (male, r & 1) {
                        (false, 0) => trf("STR_THOUGHT_1C_MY_LOVELY", &[("WHAT", &what)]),
                        (false, _) => trf("STR_THOUGHT_1C_WHAT_A_NICE", &[("WHAT", &what)]),
                        (true, 0) => trf("STR_THOUGHT_1C_LIKING", &[("WHAT", &what)]),
                        _ => trf("STR_THOUGHT_1C_COOL", &[("WHAT", &what)]),
                    },
                }
            }
            0x1d => {
                tone = Tone::Good;
                tr("STR_THOUGHT_1D_VARIETY").into()
            }
            0x1e => {
                tone = Tone::Bad;
                let fl = self.holes[hole].flags;
                let pf = self.holes[hole.saturating_sub(1)].flags;
                if fl & 0x20 != 0 && pf & 0x20 != 0 {
                    tr("STR_THOUGHT_1E_DOGLEG_LEFT").into()
                } else if fl & 0x40 != 0 && pf & 0x40 != 0 {
                    tr("STR_THOUGHT_1E_DOGLEG_RIGHT").into()
                } else if hole > 1 && self.holes[hole].par == self.holes[hole - 1].par {
                    trf("STR_THOUGHT_1E_ANOTHER_PAR", &[("PAR", &self.holes[hole].par)])
                } else {
                    tr("STR_THOUGHT_1E_LIKE_THE_LAST").into()
                }
            }
            0x1f => match r & 3 {
                0 => trf("STR_THOUGHT_1F_NEVER_SEEN", &[("TERRAIN", &noun)]),
                1 => trf("STR_THOUGHT_1F_TON_OF", &[("TERRAIN", &noun)]),
                2 => trf("STR_THOUGHT_1F_LOOK_AT_ALL", &[("TERRAIN", &noun)]),
                _ => trf("STR_THOUGHT_1F_ANY_MORE", &[("TERRAIN", &noun)]),
            },
            0x20 => {
                tone = Tone::Good;
                tr("STR_THOUGHT_20_LONG_OR_SAFE").into()
            }
            0x21 => {
                tone = Tone::Good;
                tr("STR_THOUGHT_21_LEFT_OR_RIGHT").into()
            }
            0x22 => {
                tone = Tone::Good;
                match arg {
                    0 => tr("STR_THOUGHT_22_THANKS_FOR_ASKING"),
                    1 => tr("STR_THOUGHT_22_THANKS_FOR_NOTICING"),
                    2 => tr("STR_THOUGHT_22_ENCOURAGEMENT"),
                    _ => "",
                }
                .into()
            }
            0x3a => match arg {
                0 => tr("STR_THOUGHT_3A_TRYING_TO_PLAY"),
                1 => tr("STR_THOUGHT_3A_FINE_THANKS"),
                _ => tr("STR_THOUGHT_3A_LOUSY"),
            }
            .into(),
            0x23 => {
                tone = Tone::Bad;
                match ((self.tick as usize + g * 45) / 80) & 3 {
                    0 => tr("STR_THOUGHT_23_NEVER_AGAIN"),
                    1 => tr("STR_THOUGHT_23_HATE_MY_CLUBS"),
                    2 => tr("STR_THOUGHT_23_SCREAM"),
                    _ => tr("STR_THOUGHT_23_PIECE_OF_ME"),
                }
                .into()
            }
            0x24 => {
                tone = Tone::Bad;
                if arg != 0 { tr("STR_THOUGHT_24_GUY_CRAZY") } else { tr("STR_THOUGHT_24_SHE_FLIPPED") }.into()
            }
            0x27 => {
                tone = Tone::Good;
                let animal = crate::wildlife::KINDS
                    .get(arg.max(0) as usize)
                    .map(|k| k.name().to_lowercase())
                    .unwrap_or_else(|| tr("STR_THOUGHT_27_CRITTER").into());
                if male {
                    trf("STR_THOUGHT_27_SCARED", &[("ANIMAL", &animal)])
                } else {
                    trf("STR_THOUGHT_27_FRIGHTENED", &[("ANIMAL", &animal)])
                }
            }
            0x28 => [
                tr("STR_THOUGHT_28_OH_MAMA"),
                tr("STR_THOUGHT_28_OHH_BABY"),
                tr("STR_THOUGHT_28_YES"),
                tr("STR_THOUGHT_28_GADZOOKS"),
                tr("STR_THOUGHT_28_OH_YEAH"),
                tr("STR_THOUGHT_28_SMOKER"),
                tr("STR_THOUGHT_28_SCREAMER"),
                tr("STR_THOUGHT_28_NAILED_IT"),
            ][(r & 7) as usize]
                .into(),
            0x29 => if g & 1 == 0 { tr("STR_THOUGHT_29_CLEAN_AND_SHINY") } else { tr("STR_THOUGHT_29_HIT_IT_STRAIGHT") }.into(),
            0x2a => {
                by_partner = true;
                trf("STR_THOUGHT_2A_LUCKY_BOUNCE", &[("GOLFER", &me)])
            }
            0x2b => {
                tone = Tone::Bad;
                tr("STR_THOUGHT_2B_STEEP_SLOPE").into()
            }
            0x2c => {
                tone = Tone::Good;
                tr("STR_THOUGHT_2C_NICE_LANDMARK").into()
            }
            0x2d => tr("STR_THOUGHT_2D_UPHILL").into(),
            0x2e => {
                tone = Tone::Good;
                tr("STR_THOUGHT_2E_DOWNHILL").into()
            }
            0x2f => {
                tone = Tone::Bad;
                match r & 3 {
                    0 => trf("STR_THOUGHT_2F_ATTITUDE", &[("PARTNER", &partner)]),
                    1 => trf("STR_THOUGHT_2F_STOP_WHINING", &[("PARTNER", &partner)]),
                    2 => trf("STR_THOUGHT_2F_TALK_TO_THE_HAND", &[("PARTNER", &partner)]),
                    _ => trf("STR_THOUGHT_2F_CONCENTRATING", &[("PARTNER", &partner)]),
                }
            }
            0x30 => trf("STR_THOUGHT_30_WHAT_DO_YOU_DO", &[("PARTNER", &partner)]),
            0x31 => {
                let job = self.roster.get(r as usize).map(|p| p.job.clone()).unwrap_or_default();
                if job.is_empty() {
                    tr("STR_THOUGHT_31_THIS_AND_THAT").into()
                } else {
                    trf("STR_THOUGHT_31_IM_A", &[("JOB", &job.to_lowercase())])
                }
            }
            0x33..=0x35 => {
                let (k, second) = match id {
                    0x33 => (10, tr("STR_THOUGHT_33_HELPED_DISTANCE")),
                    0x34 => (8, tr("STR_THOUGHT_34_IMPROVED_ACCURACY")),
                    _ => (6, tr("STR_THOUGHT_35_IMPROVED_PUTTING")),
                };
                let lvl = c.level[k];
                let name = match id {
                    0x33 => tr("STR_THOUGHT_33_DRIVING_RANGE"),
                    0x34 => tr("STR_THOUGHT_34_PRO_SHOP"),
                    _ => tr("STR_THOUGHT_35_PUTTING_GREEN"),
                };
                let first = match lvl {
                    1 => trf("STR_THOUGHT_33_NEW_HAS", &[("BUILDING", &name)]),
                    2 => trf("STR_THOUGHT_33_UPGRADED_HAS", &[("BUILDING", &name)]),
                    3 => trf("STR_THOUGHT_33_DELUXE_HAS", &[("BUILDING", &name)]),
                    _ => String::new(),
                };
                if first.is_empty() {
                    String::new()
                } else if (self.tick as usize + g * 5) & 8 == 0 {
                    trf("STR_THOUGHT_33_SECOND", &[("SECOND", &second)])
                } else {
                    trf("STR_THOUGHT_33_FIRST", &[("FIRST", &first)])
                }
            }
            0x36 => {
                if self.difficulty <= 1 {
                    tone = Tone::Good;
                }
                trf("STR_THOUGHT_36_USE_MY_CLUB", &[("CLUB", &club_name(arg.clamp(0, 13) as usize))])
            }
            0x37 => tr("STR_THOUGHT_37_DRAW").into(),
            0x38 => tr("STR_THOUGHT_38_FADE").into(),
            0x39 => tr("STR_THOUGHT_39_HIGH_AND_SOFT").into(),
            0x3c => {
                if is_tree(ty) {
                    trf("STR_THOUGHT_3C_LOW_UNDER", &[("TERRAIN", &t1)])
                } else {
                    tr("STR_THOUGHT_3C_LOW").into()
                }
            }
            0x3b => {
                let m = self.member(g);
                if m.card.get(hole).copied().unwrap_or(0) != 0 && self.holes[hole].flags & 1 == 0 {
                    trf("STR_THOUGHT_3B_LAST_TIME", &[("SCORE", &m.card[hole])])
                } else if self.holes[hole].flags & 2 != 0 {
                    tr("STR_THOUGHT_3B_TOP_18").into()
                } else if self.holes[hole].flags & 1 != 0 {
                    trf("STR_THOUGHT_3B_TOP_100", &[("PARTNER", &partner), ("HOLE", &hole)])
                } else {
                    tone = Tone::Good;
                    tr("STR_THOUGHT_3B_ALWAYS_LIKED").into()
                }
            }
            0x3d => {
                let ask = g & 1 == 1;
                match (arg, ask) {
                    (3, true) => trf("STR_THOUGHT_3D_HOW_ARE_YOU", &[("PARTNER", &partner)]),
                    (3, false) => tr("STR_THOUGHT_3D_FINE").into(),
                    (4, true) => tr("STR_THOUGHT_3D_NEW_CLUBS").into(),
                    (4, false) => tr("STR_THOUGHT_3D_BROTHER_IN_LAW").into(),
                    (5, true) => tr("STR_THOUGHT_3D_GREAT_DAY").into(),
                    (5, false) => tr("STR_THOUGHT_3D_SURE_DOES").into(),
                    (6, true) => tr("STR_THOUGHT_3D_BIRDS_SINGING").into(),
                    (6, false) => tr("STR_THOUGHT_3D_ANY_BETTER").into(),
                    (7, true) => tr("STR_THOUGHT_3D_PREPARE_AMAZED").into(),
                    (7, false) => tr("STR_THOUGHT_3D_MAGIC_TIME").into(),
                    (_, true) => tr("STR_THOUGHT_3D_TEE_OFF_FIRST").into(),
                    _ => tr("STR_THOUGHT_3D_DONT_CARE").into(),
                }
            }
            0x3e => {
                let p = self.roster.get(r as usize).cloned().unwrap_or_default();
                signature_saying(p.head_index(r as usize), !male).replace("PARTNER", &partner)
            }
            0x3f => {
                let pro = self.g[g].vip() == 0x20;
                match (pro, g & 3) {
                    (true, 0) => tr("STR_THOUGHT_3F_A_GAME").into(),
                    (true, 1) => tr("STR_THOUGHT_3F_TROPHY_NOW").into(),
                    (true, 2) => tr("STR_THOUGHT_3F_CADDIE").into(),
                    (true, _) => tr("STR_THOUGHT_3F_LOSERS_GO_HOME").into(),
                    (false, 0) => tr("STR_THOUGHT_3F_GOOD_TO_BE_BACK").into(),
                    (false, 1) => trf("STR_THOUGHT_3F_GREAT_TOURNAMENT", &[("PARTNER", &partner)]),
                    (false, 2) => tr("STR_THOUGHT_3F_TICKETS").into(),
                    _ => tr("STR_THOUGHT_3F_COULD_HAVE_WON").into(),
                }
            }
            0x41 => {
                if self.difficulty > 1 {
                    tone = Tone::Bad;
                }
                tr("STR_THOUGHT_41_DIDNT_SEE").into()
            }
            _ => String::new(),
        };
        Line { text, tone, partner: by_partner }
    }
}

/// The course opinion at the end of a hole (0x469a20), by mood and the hole's score against par.
pub fn course_opinion(mood: i32, d: i32) -> &'static str {
    match mood {
        0 => tr("STR_THOUGHT_OPINION_0"),
        1 => tr("STR_THOUGHT_OPINION_1"),
        2 => tr("STR_THOUGHT_OPINION_2"),
        3 => tr("STR_THOUGHT_OPINION_3"),
        4 => tr("STR_THOUGHT_OPINION_4"),
        5 => tr("STR_THOUGHT_OPINION_5"),
        6 if d < 2 => tr("STR_THOUGHT_OPINION_6"),
        6 => tr("STR_THOUGHT_OPINION_6_SURPRISING"),
        7 if d < 1 => tr("STR_THOUGHT_OPINION_7"),
        7 => tr("STR_THOUGHT_OPINION_7_FASCINATING"),
        8..=0x7f if d < 1 => tr("STR_THOUGHT_OPINION_8"),
        8..=0x7f => tr("STR_THOUGHT_OPINION_8_INTERESTING"),
        _ => tr("STR_THOUGHT_OPINION_LEAVING"),
    }
}

/// The stock signature saying (event 0x3e, table 0x4d55ec) of a stock head of a gender; custom heads have none.
pub fn signature_saying(head: u8, female: bool) -> &'static str {
    crate::roster::head_defaults(head, female).map(|d| d.2).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines() {
        let mut cl = Club::default();
        let c = Course::default();
        cl.g[2].roster = 4;
        cl.g[3].roster = 5;
        let l = cl.thought(&c, 0x0c, 13, 2);
        assert_eq!(l.text, "Darn that stupid tree!");
        assert_eq!(l.tone, Tone::Bad);
        let l = cl.thought(&c, 0x02, 7, 2);
        assert_eq!(l.text, "Darn, I'm in the sand trap!");
        cl.g[2].field_ae = 2;
        let l = cl.thought(&c, 0x02, 13, 2);
        assert!(l.partner && l.text.contains("under the tree"));
        assert_eq!(cl.thought(&c, 0x36, 0, 2).text, "I'll use my Driver for this shot.");
    }
}
