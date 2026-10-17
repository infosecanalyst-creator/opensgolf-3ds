//! The More visitors extra (sg_core::extras; the port's own, not in the original): new kinds of visiting golfer.
//!
//! * Touring pros: strong players who drop in now and then once the course is good enough, play quickly (the original's
//!   hurried pace: a step faster, no dawdling, no giving way), pay a higher green fee and rate the course in a few words at
//!   the end of their round.
//! * Celebrities: invented personalities (a film star, a pop star, a TV chef, a retired footballer...) who come rarely,
//!   draw a few extra golfers to the clubhouse while the word gets round, pay a little more and say what they thought.
//!
//! A visitor takes the place of the second golfer of a new pair, as the original's VIPs do (`Club::vip_check`), when no VIP
//! came with it. Who visits, how often, what they need of the course and what they bring are data: data/visitors.toml, a
//! file content packs can add to (docs/MODDING.md, section 3), with built-in entries whose names and lines are our own
//! (lang/en.txt, STR_VISITOR_*). Every visitor wears the game's own bodies, heads and outfit colours, recombined.
//!
//! The visitor's person record goes into one of the roster's spare records (the exe's three "Dummy" records after the
//! CEO), so the People screen, the golfer card, the voices and the Best Scores show it like anyone else; the golfer slot
//! keeps the kind (`KIND_TOURING_PRO`, `KIND_CELEBRITY`: kinds the exe never uses, so its VIP rules pass them by) and, in
//! the famous golfer byte the exe gives pros only, the visitor's entry in the data. With the extra off nothing here runs
//! and nothing is drawn from the game's generator.

use crate::golfer::{flag, Club, Member, SLOTS};
use crate::land::ExeRng;
use crate::lang::{text, trf};
use crate::roster::Person;
use serde::{Deserialize, Serialize};

/// Golfer kinds of the visitors (the exe's kinds are 0x20 pro, 0x40, 0x60, 0x80 its VIPs).
pub const KIND_TOURING_PRO: u8 = 0xa0;
pub const KIND_CELEBRITY: u8 = 0xc0;
/// The roster records the visitors' persons take (the exe's spare records 80 and 81, "Dummy").
pub const ROSTER_TOURING_PRO: i32 = 80;
pub const ROSTER_CELEBRITY: i32 = 81;
/// The sound a visitor's arrival plays (the original's VIP arrival).
const ARRIVAL_SOUND: i32 = 0x26;

/// What kind of visitor an entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisitorKind {
    TouringPro,
    Celebrity,
}

impl VisitorKind {
    fn index(self) -> usize {
        self as usize
    }
    pub fn golfer_kind(self) -> u8 {
        [KIND_TOURING_PRO, KIND_CELEBRITY][self.index()]
    }
    fn roster(self) -> i32 {
        [ROSTER_TOURING_PRO, ROSTER_CELEBRITY][self.index()]
    }
}

/// data/visitors.toml, `[[visitor]]`: one visitor. Text fields hold a text key (STR_..., the built-in entries, so they
/// follow the player's language) or a pack's own text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisitorDef {
    pub id: String,
    pub kind: VisitorKind,
    pub name: String,
    /// The profession the golfer card shows, and a celebrity's title in the ticker ("TV Chef Arlo Pennington").
    pub job: String,
    #[serde(default)]
    pub female: bool,
    /// Age (the golfer card shows the nearest the game's age bands give: 20, 30, 45, 50, 65, 75 or 95, plus a year or
    /// two) and marriage, as the card and the stories read them.
    #[serde(default = "d_age")]
    pub age: u8,
    #[serde(default)]
    pub married: bool,
    /// The body (0..3 of the gender's four) and the stock head (0..18 of the gender's).
    #[serde(default)]
    pub body: u8,
    #[serde(default)]
    pub head: u8,
    /// Colours: the swap palettes the outfit takes (bodies.rs): skin 0..3, hair 0..5 (women), shirt, trousers, hat 0..9.
    #[serde(default)]
    pub skin: u8,
    #[serde(default)]
    pub hair: u8,
    #[serde(default)]
    pub shirt: u8,
    #[serde(default)]
    pub pants: u8,
    #[serde(default)]
    pub hat: u8,
    /// Skill class bits (1 Length, 2 Accuracy, 4 Imagination), as a golfer's.
    #[serde(default = "d_class")]
    pub class: u8,
    /// Special skills in the order of the skill cards (Power Hitter .. Luck), 0..10 each (tens of percent).
    #[serde(default)]
    pub skills: Vec<u8>,
    /// How often this one comes, against the others of the kind.
    #[serde(default = "d_weight")]
    pub weight: i64,
    /// What the visitor says of the course at the end of the round, the unhappiest first (any number, spread over the
    /// moods); none: the kind's own lines.
    #[serde(default)]
    pub remarks: Vec<String>,
}

fn d_class() -> u8 {
    7
}
fn d_weight() -> i64 {
    1
}
fn d_age() -> u8 {
    30
}

/// The age band bits of a person's flag byte (+0x21) nearest to `age` (stories.rs `Club::age`: 20, 30 and 45 years by bit).
fn age_bits(age: u8) -> u8 {
    (0..8u8).min_by_key(|&b| (20 * (b & 1) as i32 + 15 * (b & 2) as i32 + 45 * (b >> 2) as i32 - age as i32).abs()).unwrap_or(1)
}

impl crate::data::Entry for VisitorDef {
    fn id(&self) -> &str {
        &self.id
    }
    /// Visitors have no engine slot: packs add as many as they like.
    fn slot(&self) -> i64 {
        -1
    }
    fn check(&self) -> Result<(), String> {
        let r = |v: u8, hi: u8, what: &str| if v > hi { Err(format!("{what} = {v}: must be 0..{hi}")) } else { Ok(()) };
        if self.name.trim().is_empty() || self.job.trim().is_empty() {
            return Err("name and job must not be empty".into());
        }
        r(self.body, 3, "body")?;
        r(self.head, 18, "head")?;
        r(self.skin, 3, "skin")?;
        r(self.hair, 5, "hair")?;
        r(self.shirt, 9, "shirt")?;
        r(self.pants, 9, "pants")?;
        r(self.hat, 9, "hat")?;
        if !(1..=7).contains(&self.class) {
            return Err(format!("class = {}: must be 1..7", self.class));
        }
        if self.skills.len() > 10 || self.skills.iter().any(|&s| s > 10) {
            return Err("skills: at most ten values, each 0..10".into());
        }
        if !(0..=1000).contains(&self.weight) {
            return Err(format!("weight = {}: must be 0..1000", self.weight));
        }
        if !(18..=99).contains(&self.age) {
            return Err(format!("age = {}: must be 18..99", self.age));
        }
        if self.remarks.len() > 9 {
            return Err("remarks: at most nine".into());
        }
        Ok(())
    }
}

impl VisitorDef {
    /// The visitor as a roster person: the body, head and colours fixed (record +0x2c set), as a customised golfer's.
    pub fn person(&'static self) -> Person {
        let pro = self.kind == VisitorKind::TouringPro;
        Person {
            job: text(&self.job).to_string(),
            name: text(&self.name).to_string(),
            traits: 0,
            // the flag byte: bit 7 a woman, 0x10 married or 0x08 single, the age bands
            b21: if self.female { 0x80 } else { 0 } | if self.married { 0x10 } else { 0x08 } | age_bits(self.age),
            b23: (self.body & 3) << 4 | (self.hat & 0xf),
            fixed: (self.class & 7) as i32,
            shirt: self.shirt,
            pants: self.pants,
            // a pro's gloved hands, as the famous pros' (bodies.rs)
            alt_skin: if pro { 4 } else { self.skin },
            skin: self.skin,
            hair: self.hair,
            head: Some(self.head),
            ..Default::default()
        }
    }

    /// What the visitor says of the course at mood `mood`: the entry's own lines or the kind's, spread over moods 0..8.
    pub fn remark(&'static self, mood: i32) -> &'static str {
        const PRO: [&str; 5] = [
            "STR_VISITOR_PRO_REMARK_0",
            "STR_VISITOR_PRO_REMARK_1",
            "STR_VISITOR_PRO_REMARK_2",
            "STR_VISITOR_PRO_REMARK_3",
            "STR_VISITOR_PRO_REMARK_4",
        ];
        const CELEBRITY: [&str; 3] = ["STR_VISITOR_CELEBRITY_REMARK_0", "STR_VISITOR_CELEBRITY_REMARK_1", "STR_VISITOR_CELEBRITY_REMARK_2"];
        let pick = |n: usize| mood.clamp(0, 8) as usize * n / 9;
        if !self.remarks.is_empty() {
            return text(&self.remarks[pick(self.remarks.len())]);
        }
        let own: &[&'static str] = if self.kind == VisitorKind::TouringPro { &PRO } else { &CELEBRITY };
        text(own[pick(own.len())])
    }
}

/// data/visitors.toml, `[visits]`: how often the visitors come, what they wait for and what they bring.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisitsDef {
    /// The chance (percent) that a new pair brings a touring pro, once the course is ready for one.
    #[serde(default = "d_pro_chance")]
    pub pro_chance: i64,
    /// Holes open, and the club rating (the one the VIPs go by: the fun rating on Easy and Normal, the skill rating in
    /// hundredths on Hard and Impossible), before touring pros come.
    #[serde(default = "d_pro_holes")]
    pub pro_holes: i64,
    #[serde(default = "d_pro_rating")]
    pub pro_rating: i64,
    /// Game ticks at least between two touring pros' visits (a month is 1024).
    #[serde(default = "d_pro_gap")]
    pub pro_gap: i64,
    /// A touring pro's green fee, in percent of the fee the same golfer would pay.
    #[serde(default = "d_pro_fee_percent")]
    pub pro_fee_percent: i64,
    /// Touring pros play quickly.
    #[serde(default = "d_true")]
    pub pro_quick: bool,
    /// The same for celebrities.
    #[serde(default = "d_celebrity_chance")]
    pub celebrity_chance: i64,
    #[serde(default = "d_celebrity_holes")]
    pub celebrity_holes: i64,
    #[serde(default = "d_celebrity_rating")]
    pub celebrity_rating: i64,
    #[serde(default = "d_celebrity_gap")]
    pub celebrity_gap: i64,
    #[serde(default = "d_celebrity_fee_percent")]
    pub celebrity_fee_percent: i64,
    /// Golfers a celebrity's visit draws to the clubhouse beyond the usual arrivals.
    #[serde(default = "d_celebrity_crowd")]
    pub celebrity_crowd: i64,
}

fn d_pro_chance() -> i64 {
    10
}
fn d_pro_holes() -> i64 {
    9
}
fn d_pro_rating() -> i64 {
    400
}
fn d_pro_gap() -> i64 {
    2048
}
fn d_pro_fee_percent() -> i64 {
    200
}
fn d_true() -> bool {
    true
}
fn d_celebrity_chance() -> i64 {
    5
}
fn d_celebrity_holes() -> i64 {
    6
}
fn d_celebrity_rating() -> i64 {
    250
}
fn d_celebrity_gap() -> i64 {
    4096
}
fn d_celebrity_fee_percent() -> i64 {
    150
}
fn d_celebrity_crowd() -> i64 {
    6
}

impl Default for VisitsDef {
    fn default() -> Self {
        VisitsDef {
            pro_chance: d_pro_chance(),
            pro_holes: d_pro_holes(),
            pro_rating: d_pro_rating(),
            pro_gap: d_pro_gap(),
            pro_fee_percent: d_pro_fee_percent(),
            pro_quick: true,
            celebrity_chance: d_celebrity_chance(),
            celebrity_holes: d_celebrity_holes(),
            celebrity_rating: d_celebrity_rating(),
            celebrity_gap: d_celebrity_gap(),
            celebrity_fee_percent: d_celebrity_fee_percent(),
            celebrity_crowd: d_celebrity_crowd(),
        }
    }
}

impl VisitsDef {
    pub fn check(&self) -> Result<(), String> {
        let r = |v: i64, lo: i64, hi: i64, what: &str| {
            if (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{what} = {v}: must be {lo}..{hi}"))
            }
        };
        r(self.pro_chance, 0, 100, "pro_chance")?;
        r(self.pro_holes, 1, 18, "pro_holes")?;
        r(self.pro_rating, 0, 1_000_000, "pro_rating")?;
        r(self.pro_gap, 0, 1_000_000, "pro_gap")?;
        r(self.pro_fee_percent, 0, 1000, "pro_fee_percent")?;
        r(self.celebrity_chance, 0, 100, "celebrity_chance")?;
        r(self.celebrity_holes, 1, 18, "celebrity_holes")?;
        r(self.celebrity_rating, 0, 1_000_000, "celebrity_rating")?;
        r(self.celebrity_gap, 0, 1_000_000, "celebrity_gap")?;
        r(self.celebrity_fee_percent, 0, 1000, "celebrity_fee_percent")?;
        r(self.celebrity_crowd, 0, 8, "celebrity_crowd")
    }

    /// Chance, holes, rating and gap of a kind.
    fn rules(&self, k: VisitorKind) -> (i64, i64, i64, i64) {
        match k {
            VisitorKind::TouringPro => (self.pro_chance, self.pro_holes, self.pro_rating, self.pro_gap),
            VisitorKind::Celebrity => (self.celebrity_chance, self.celebrity_holes, self.celebrity_rating, self.celebrity_gap),
        }
    }
}

/// A built-in visitor (the text keys are written out so lang/en.txt can be checked against them).
struct Builtin {
    id: &'static str,
    kind: VisitorKind,
    name: &'static str,
    job: &'static str,
    remarks: [&'static str; 3],
    female: bool,
    age: u8,
    married: bool,
    /// body, head, skin, hair, shirt, pants, hat, class
    looks: [u8; 8],
    skills: [u8; 10],
}

/// The built-in visitors: six touring pros and eight celebrities, all invented.
const BUILTIN: [Builtin; 14] = [
    Builtin {
        id: "rena-vasquell",
        kind: VisitorKind::TouringPro,
        name: "STR_VISITOR_NAME_RENA_VASQUELL",
        job: "STR_VISITOR_JOB_TOURING_PRO",
        remarks: ["", "", ""],
        female: true,
        age: 29,
        married: false,
        looks: [1, 4, 1, 2, 3, 0, 9, 7],
        skills: [3, 2, 4, 4, 3, 2, 2, 3, 2, 1],
    },
    Builtin {
        id: "hugo-brandvold",
        kind: VisitorKind::TouringPro,
        name: "STR_VISITOR_NAME_HUGO_BRANDVOLD",
        job: "STR_VISITOR_JOB_TOURING_PRO",
        remarks: ["", "", ""],
        female: false,
        age: 34,
        married: true,
        looks: [2, 7, 0, 0, 6, 1, 2, 7],
        skills: [5, 4, 2, 2, 2, 1, 3, 2, 3, 1],
    },
    Builtin {
        id: "sami-varkonyi",
        kind: VisitorKind::TouringPro,
        name: "STR_VISITOR_NAME_SAMI_VARKONYI",
        job: "STR_VISITOR_JOB_TOURING_PRO",
        remarks: ["", "", ""],
        female: false,
        age: 26,
        married: false,
        looks: [0, 12, 2, 0, 4, 8, 0, 7],
        skills: [2, 2, 4, 3, 5, 3, 1, 2, 3, 2],
    },
    Builtin {
        id: "lottie-pembrook",
        kind: VisitorKind::TouringPro,
        name: "STR_VISITOR_NAME_LOTTIE_PEMBROOK",
        job: "STR_VISITOR_JOB_TOURING_PRO",
        remarks: ["", "", ""],
        female: true,
        age: 31,
        married: true,
        looks: [3, 9, 0, 4, 7, 2, 9, 7],
        skills: [2, 3, 3, 5, 3, 1, 3, 4, 2, 1],
    },
    Builtin {
        id: "matteo-kringel",
        kind: VisitorKind::TouringPro,
        name: "STR_VISITOR_NAME_MATTEO_KRINGEL",
        job: "STR_VISITOR_JOB_TOURING_PRO",
        remarks: ["", "", ""],
        female: false,
        age: 41,
        married: true,
        looks: [1, 3, 1, 0, 1, 5, 4, 7],
        skills: [3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
    },
    Builtin {
        id: "ines-marlowe",
        kind: VisitorKind::TouringPro,
        name: "STR_VISITOR_NAME_INES_MARLOWE",
        job: "STR_VISITOR_JOB_TOURING_PRO",
        remarks: ["", "", ""],
        female: true,
        age: 38,
        married: false,
        looks: [0, 15, 3, 1, 5, 3, 9, 7],
        skills: [2, 2, 3, 4, 4, 2, 2, 2, 5, 2],
    },
    Builtin {
        id: "brock-stellan",
        kind: VisitorKind::Celebrity,
        name: "STR_VISITOR_NAME_BROCK_STELLAN",
        job: "STR_VISITOR_JOB_FILM_STAR",
        remarks: ["STR_VISITOR_REMARK_BROCK_STELLAN_0", "STR_VISITOR_REMARK_BROCK_STELLAN_1", "STR_VISITOR_REMARK_BROCK_STELLAN_2"],
        female: false,
        age: 45,
        married: false,
        looks: [3, 17, 1, 0, 0, 0, 0, 3],
        skills: [0; 10],
    },
    Builtin {
        id: "marisol-vane",
        kind: VisitorKind::Celebrity,
        name: "STR_VISITOR_NAME_MARISOL_VANE",
        job: "STR_VISITOR_JOB_POP_STAR",
        remarks: ["STR_VISITOR_REMARK_MARISOL_VANE_0", "STR_VISITOR_REMARK_MARISOL_VANE_1", "STR_VISITOR_REMARK_MARISOL_VANE_2"],
        female: true,
        age: 24,
        married: false,
        looks: [2, 1, 2, 3, 8, 6, 9, 6],
        skills: [0; 10],
    },
    Builtin {
        id: "arlo-pennington",
        kind: VisitorKind::Celebrity,
        name: "STR_VISITOR_NAME_ARLO_PENNINGTON",
        job: "STR_VISITOR_JOB_TV_CHEF",
        remarks: ["STR_VISITOR_REMARK_ARLO_PENNINGTON_0", "STR_VISITOR_REMARK_ARLO_PENNINGTON_1", "STR_VISITOR_REMARK_ARLO_PENNINGTON_2"],
        female: false,
        age: 50,
        married: true,
        looks: [1, 10, 0, 0, 9, 7, 0, 6],
        skills: [0; 10],
    },
    Builtin {
        id: "gus-mallory",
        kind: VisitorKind::Celebrity,
        name: "STR_VISITOR_NAME_GUS_MALLORY",
        job: "STR_VISITOR_JOB_FOOTBALLER",
        remarks: ["STR_VISITOR_REMARK_GUS_MALLORY_0", "STR_VISITOR_REMARK_GUS_MALLORY_1", "STR_VISITOR_REMARK_GUS_MALLORY_2"],
        female: false,
        age: 52,
        married: true,
        looks: [3, 6, 1, 0, 2, 4, 0, 5],
        skills: [0; 10],
    },
    Builtin {
        id: "della-starling",
        kind: VisitorKind::Celebrity,
        name: "STR_VISITOR_NAME_DELLA_STARLING",
        job: "STR_VISITOR_JOB_SOAP_STAR",
        remarks: ["STR_VISITOR_REMARK_DELLA_STARLING_0", "STR_VISITOR_REMARK_DELLA_STARLING_1", "STR_VISITOR_REMARK_DELLA_STARLING_2"],
        female: true,
        age: 47,
        married: false,
        looks: [1, 8, 0, 3, 6, 9, 9, 6],
        skills: [0; 10],
    },
    Builtin {
        id: "ziggy-marchetti",
        kind: VisitorKind::Celebrity,
        name: "STR_VISITOR_NAME_ZIGGY_MARCHETTI",
        job: "STR_VISITOR_JOB_RACING_DRIVER",
        remarks: ["STR_VISITOR_REMARK_ZIGGY_MARCHETTI_0", "STR_VISITOR_REMARK_ZIGGY_MARCHETTI_1", "STR_VISITOR_REMARK_ZIGGY_MARCHETTI_2"],
        female: false,
        age: 30,
        married: false,
        looks: [0, 13, 1, 0, 3, 9, 3, 1],
        skills: [0; 10],
    },
    Builtin {
        id: "prudence-oakes",
        kind: VisitorKind::Celebrity,
        name: "STR_VISITOR_NAME_PRUDENCE_OAKES",
        job: "STR_VISITOR_JOB_NOVELIST",
        remarks: ["STR_VISITOR_REMARK_PRUDENCE_OAKES_0", "STR_VISITOR_REMARK_PRUDENCE_OAKES_1", "STR_VISITOR_REMARK_PRUDENCE_OAKES_2"],
        female: true,
        age: 65,
        married: true,
        looks: [3, 11, 1, 4, 4, 2, 9, 6],
        skills: [0; 10],
    },
    Builtin {
        id: "felix-nightly",
        kind: VisitorKind::Celebrity,
        name: "STR_VISITOR_NAME_FELIX_NIGHTLY",
        job: "STR_VISITOR_JOB_ASTRONOMER",
        remarks: ["STR_VISITOR_REMARK_FELIX_NIGHTLY_0", "STR_VISITOR_REMARK_FELIX_NIGHTLY_1", "STR_VISITOR_REMARK_FELIX_NIGHTLY_2"],
        female: false,
        age: 58,
        married: true,
        looks: [2, 14, 0, 0, 5, 6, 0, 4],
        skills: [0; 10],
    },
];

/// The built-in visitors as data/visitors.toml entries.
pub fn builtin() -> Vec<VisitorDef> {
    BUILTIN
        .iter()
        .map(|b| {
            let [body, head, skin, hair, shirt, pants, hat, class] = b.looks;
            VisitorDef {
                id: b.id.into(),
                kind: b.kind,
                name: b.name.into(),
                job: b.job.into(),
                female: b.female,
                age: b.age,
                married: b.married,
                body,
                head,
                skin,
                hair,
                shirt,
                pants,
                hat,
                class,
                skills: if b.kind == VisitorKind::TouringPro { b.skills.to_vec() } else { Vec::new() },
                weight: 1,
                remarks: b.remarks.iter().filter(|r| !r.is_empty()).map(|r| r.to_string()).collect(),
            }
        })
        .collect()
}

/// What the club keeps of the visits (saved with it, only once a visitor has come): the golfers a celebrity's visit
/// still draws, and the game tick of each kind's last visit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Visits {
    pub crowd: i32,
    pub last: [u32; 2],
}

impl Visits {
    pub fn is_zero(&self) -> bool {
        *self == Visits::default()
    }
}

/// The visitor entry of a golfer of kind `kind` whose famous byte is `famous`, if the data (still) has it.
fn entry(kind: u8, famous: i32) -> Option<&'static VisitorDef> {
    crate::data::get().visitors.get(famous.max(0) as usize).filter(|v| v.kind.golfer_kind() == kind & 0xe0)
}

impl Club {
    /// Whether golfer `g` is one of the More visitors extra's visitors.
    pub fn is_visitor(&self, g: usize) -> bool {
        matches!(self.g.get(g).map(|x| x.kind & 0xe0), Some(KIND_TOURING_PRO | KIND_CELEBRITY))
    }

    /// The club rating the visitors go by (as the VIPs: fun on Easy and Normal, skill in hundredths above).
    fn visitor_rating(&self) -> i32 {
        if self.difficulty < 2 {
            self.ratings.fun
        } else {
            self.ratings.skill
        }
    }

    /// The pairing's visitor check (the extra on, at the end of `pair_setup`): when neither golfer is a VIP, the pair's second
    /// golfer may become a celebrity, else a touring pro, if the course is ready for one, none of the kind is about, the
    /// last one came long enough ago and the chance comes up.
    pub(crate) fn visitor_check(&mut self, rng: &mut ExeRng, p1: usize, p2: usize) {
        if self.g[p1].kind != 0 || self.g[p2].kind != 0 || self.game & crate::golfer::game::TOURNAMENT != 0 {
            return;
        }
        let d = crate::data::get();
        let holes = (self.next_hole - 1) as i64;
        let rating = self.visitor_rating() as i64;
        for kind in [VisitorKind::Celebrity, VisitorKind::TouringPro] {
            let (chance, need_holes, need_rating, gap) = d.visits.rules(kind);
            let last = self.visits.last[kind.index()];
            let about = self.g.iter().take(SLOTS).any(|o| o.hole != 0 && o.roster == kind.roster());
            let total: i64 = d.visitors.iter().filter(|v| v.kind == kind).map(|v| v.weight).sum();
            let rested = last == 0 || self.tick as i64 >= last as i64 + gap;
            if about || total <= 0 || holes < need_holes || rating < need_rating || !rested || (kind.roster() as usize) >= self.roster.len()
            {
                continue;
            }
            if (rng.below(100) as i64) >= chance {
                continue;
            }
            let mut r = rng.below(total.min(i32::MAX as i64) as i32) as i64;
            let Some(i) = d.visitors.iter().position(|v| {
                if v.kind != kind {
                    return false;
                }
                r -= v.weight;
                r < 0
            }) else {
                continue;
            };
            self.visitor_arrives(p1, p2, i);
            return;
        }
    }

    /// Visitor entry `i` takes the place of golfer `p2` (pair partner `p1`).
    pub fn visitor_arrives(&mut self, p1: usize, p2: usize, i: usize) {
        let d = crate::data::get();
        let Some(v) = d.visitors.get(i) else { return };
        let slot = v.kind.roster() as usize;
        if slot >= self.roster.len() || slot >= self.members.len() {
            return;
        }
        let person = v.person();
        let b21 = person.b21;
        self.roster[slot] = person;
        self.members[slot] = Member { level: 1, ..Default::default() };
        let gg = &mut self.g[p2];
        gg.roster = slot as i32;
        gg.kind = v.kind.golfer_kind();
        gg.famous = i as i32;
        gg.class = v.class & 7;
        gg.looks = (gg.looks & 0xff00) | b21 as u16;
        gg.skill_mask = 0;
        gg.skills = [0; 12];
        for (k, &s) in v.skills.iter().enumerate().take(10) {
            if s != 0 {
                gg.skills[k] = s;
                gg.skill_mask |= 1 << k;
            }
        }
        if v.kind == VisitorKind::TouringPro {
            // the pros' tees, and their draw, fade and backspin shots always to hand (as a famous pro's, pro.rs)
            gg.level = 2;
            gg.skill_mask |= 0xe0;
            if d.visits.pro_quick {
                gg.flags |= flag::HURRIED;
            }
        }
        for g in [p1, p2] {
            self.g[g].story = -1;
            self.g[g].field_ae = 0;
        }
        let name = text(&v.name);
        let job = text(&v.job);
        let said = match v.kind {
            VisitorKind::TouringPro => trf("STR_VISITOR_PRO_ARRIVES", &[("NAME", &name)]),
            VisitorKind::Celebrity => {
                self.visits.crowd += d.visits.celebrity_crowd as i32;
                trf("STR_VISITOR_CELEBRITY_ARRIVES", &[("JOB", &job), ("NAME", &name)])
            }
        };
        self.visits.last[v.kind.index()] = self.tick.max(1);
        self.message_by(said, p2 as i32, 1);
        self.sound(ARRIVAL_SOUND, None);
    }

    /// A visitor's green fee for a hole: the fee golfer `g` would pay, scaled by the kind's percentage.
    pub(crate) fn visitor_fee(&self, g: usize, fee: i32) -> i32 {
        let v = &crate::data::get().visits;
        match self.g[g].kind & 0xe0 {
            KIND_TOURING_PRO => (fee as i64 * v.pro_fee_percent / 100) as i32,
            KIND_CELEBRITY => (fee as i64 * v.celebrity_fee_percent / 100) as i32,
            _ => fee,
        }
    }

    /// A touring pro keeps his quick pace on the next hole (the hole's end clears the hurry flag).
    pub(crate) fn visitor_hurry(&mut self, g: usize) {
        if self.g[g].kind & 0xe0 == KIND_TOURING_PRO && crate::data::get().visits.pro_quick {
            self.g[g].flags |= flag::HURRIED;
        }
    }

    /// The end of a visitor's round: a few words on the course in the ticker.
    pub(crate) fn visitor_verdict(&mut self, g: usize) {
        let gg = &self.g[g];
        let kind = gg.kind & 0xe0;
        if !matches!(kind, KIND_TOURING_PRO | KIND_CELEBRITY) {
            return;
        }
        let (name, job) = {
            let p = self.roster.get(gg.roster.max(0) as usize).cloned().unwrap_or_default();
            (p.name, p.job)
        };
        let fallback = || -> &'static str {
            if kind == KIND_TOURING_PRO {
                text("STR_VISITOR_PRO_REMARK_2")
            } else {
                text("STR_VISITOR_CELEBRITY_REMARK_1")
            }
        };
        let remark = entry(kind, gg.famous).map_or_else(fallback, |v| v.remark(gg.mood));
        let said = if kind == KIND_TOURING_PRO {
            trf("STR_VISITOR_PRO_RATES", &[("NAME", &name), ("REMARK", &remark)])
        } else {
            trf("STR_VISITOR_CELEBRITY_SAYS", &[("JOB", &job), ("NAME", &name), ("REMARK", &remark)])
        };
        self.message_by(said, g as i32, 1);
    }

    /// A celebrity's crowd: whether the arrivals make one more golfer now beyond their own rule (one golfer a tick while
    /// the queue has room), counting it down when `made`.
    pub(crate) fn visitor_crowd(&mut self, made: bool) -> bool {
        if !self.extras.more_visitors || self.visits.crowd <= 0 {
            return false;
        }
        if made {
            self.visits.crowd -= 1;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golfer::tests::{fnv, nine_hole_run};
    use crate::golfer::Event;

    #[test]
    fn the_built_in_visitors_check_and_have_their_text() {
        let b = builtin();
        assert!(b.iter().any(|v| v.kind == VisitorKind::TouringPro) && b.iter().any(|v| v.kind == VisitorKind::Celebrity));
        for v in &b {
            crate::data::Entry::check(v).unwrap_or_else(|e| panic!("{}: {e}", v.id));
            for k in std::iter::once(&v.name).chain([&v.job]).chain(&v.remarks) {
                assert!(crate::lang::has(k), "{} has no text for {k}", v.id);
            }
        }
        let d = crate::data::get();
        assert_eq!(d.visitors, b);
        let v = d.visitors.iter().find(|v| v.id == "arlo-pennington").unwrap();
        let p = v.person();
        assert_eq!((p.name.as_str(), p.job.as_str(), p.female()), ("Arlo Pennington", "TV Chef", false));
        assert_eq!(p.kind(0), 6, "the person's class is fixed");
        assert_eq!(v.remark(0), crate::lang::tr("STR_VISITOR_REMARK_ARLO_PENNINGTON_0"));
        assert_eq!(v.remark(9), crate::lang::tr("STR_VISITOR_REMARK_ARLO_PENNINGTON_2"));
        let pro = d.visitors.iter().find(|v| v.kind == VisitorKind::TouringPro).unwrap();
        assert_eq!(pro.remark(-3), crate::lang::tr("STR_VISITOR_PRO_REMARK_0"));
        assert_eq!(pro.remark(8), crate::lang::tr("STR_VISITOR_PRO_REMARK_4"));
        assert!(VisitsDef::default().check().is_ok());
    }

    #[test]
    fn visitors_come_pay_more_and_comment_with_the_extra_on() {
        let on = crate::extras::Extras { more_visitors: true, ..Default::default() };
        let (club, events) = nine_hole_run(on, 20_000);
        let said: Vec<&str> =
            events.iter().filter_map(|e| if let Event::Message { text, .. } = e { Some(text.as_str()) } else { None }).collect();
        let pros = said.iter().filter(|t| t.starts_with("Touring pro ") && t.contains("dropped in")).count();
        let celebs = said.iter().filter(|t| t.contains("Word gets around")).count();
        let verdicts = said.iter().filter(|t| t.contains("rates your course: '") || t.contains(" on your course: '")).count();
        // (the bare test course drives many golfers off before their round ends, visitors too)
        assert!(pros >= 1 && celebs >= 1 && verdicts >= 1, "{pros} {celebs} {verdicts}: {said:#?}");
        // never sooner than the gap allows
        assert!(pros <= 20_000 / 2048 + 1 && celebs <= 20_000 / 4096 + 1);
        // the run differs from the one without the extra, which never sees a visitor
        let (off, off_events) = nine_hole_run(Default::default(), 20_000);
        assert_ne!(fnv(&serde_json::to_string(&events).unwrap()), fnv(&serde_json::to_string(&off_events).unwrap()));
        assert!(
            off.visits.is_zero() && !off_events.iter().any(|e| matches!(e, Event::Message { text, .. } if text.contains("Touring pro")))
        );
        // the save keeps the visits and the visitors' records, and reads back the same
        let json = serde_json::to_string(&club).unwrap();
        assert!(json.contains("\"visits\""));
        let back: Club = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), json);
        // a save from before the extra (no visits) reads with nothing pending
        let old: Club = serde_json::from_str(&serde_json::to_string(&off).unwrap()).unwrap();
        assert!(old.visits.is_zero());
    }

    #[test]
    fn a_touring_pro_pays_double_hurries_and_rates_the_course() {
        let (_, mut club) = crate::golfer::tests::test_course();
        club.extras.more_visitors = true;
        let pro = crate::data::get().visitors.iter().position(|v| v.kind == VisitorKind::TouringPro).unwrap();
        club.g[2].roster = 5;
        club.g[3].roster = 6;
        club.visitor_arrives(2, 3, pro);
        let g = &club.g[3];
        assert_eq!((g.kind, g.roster, g.famous as usize, g.level), (KIND_TOURING_PRO, ROSTER_TOURING_PRO, pro, 2));
        assert!(g.flags & flag::HURRIED != 0 && g.skill_mask & 0xe0 == 0xe0 && g.skills[0] != 0);
        assert!(club.is_visitor(3) && !club.is_visitor(2));
        assert_eq!(club.name(3), "Rena Vasquell");
        assert_eq!(club.roster[80].job, "Touring Pro");
        assert_eq!(club.visitor_fee(3, 5), 10);
        assert_eq!(club.visitor_fee(2, 5), 5);
        assert_eq!(club.look(3), 6, "her body (1) among the women's");
        assert_eq!((club.age(3), club.marital(3)), (30, crate::lang::tr("STR_MARITAL_SINGLE")), "29: the nearest band");
        club.g[3].flags &= !flag::HURRIED;
        club.visitor_hurry(3);
        assert!(club.g[3].flags & flag::HURRIED != 0);
        club.g[3].mood = 8;
        club.out.clear();
        club.visitor_verdict(3);
        let t = club.out.iter().find_map(|e| if let Event::Message { text, .. } = e { Some(text.clone()) } else { None }).unwrap();
        assert_eq!(t, format!("Touring pro Rena Vasquell rates your course: '{}'", crate::lang::tr("STR_VISITOR_PRO_REMARK_4")));
    }

    #[test]
    fn packs_add_visitors_and_change_the_rules() {
        let mut t = crate::data::Tables::builtin();
        let w = t.merge(
            "visitors",
            r#"
[visits]
celebrity_chance = 10

[[visitor]]
id = "dana-fairway"
kind = "celebrity"
name = "Dana Fairway"
job = "Weather Presenter"
female = true
age = 40
head = 5
remarks = ["Grey skies all round.", "Bright spells.", "Sunshine all day!"]

[[visitor]]
id = "gus-mallory"
weight = 0

[[visitor]]
id = "bad"
kind = "touring-pro"
name = "Bad"
job = "Pro"
head = 40
"#,
        );
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].contains("bad") && w[0].contains("head"), "{w:?}");
        assert_eq!(t.visits.celebrity_chance, 10);
        assert_eq!(t.visits.pro_chance, 10, "the rest stay");
        let dana = t.visitors.iter().find(|v| v.id == "dana-fairway").unwrap();
        assert_eq!((dana.class, dana.weight, dana.remarks.len()), (7, 1, 3));
        assert_eq!(t.visitors.iter().find(|v| v.id == "gus-mallory").unwrap().weight, 0);
        assert!(t.merge("visitors", "[visits]\npro_fee_percent = 5000\n")[0].contains("pro_fee_percent"));
        assert!(t.merge("visitors", "[guests]\n")[0].contains("unknown table"));
        let (d, warn) = crate::data::Data::from_tables(&t);
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(d.visitors.len(), 15);
    }

    #[test]
    fn a_celebrity_draws_a_crowd() {
        let (_, mut club) = crate::golfer::tests::test_course();
        let i = crate::data::get().visitors.iter().position(|v| v.id == "gus-mallory").unwrap();
        club.visitor_arrives(2, 3, i);
        assert_eq!(club.visits.crowd, 6);
        assert_eq!(club.name(3), "Gus Mallory");
        assert_eq!(club.age(3), 51, "52: the 20 and 30 bands, and a year from his record");
        assert!(!club.visitor_crowd(false), "only with the extra on");
        club.extras.more_visitors = true;
        for _ in 0..6 {
            assert!(club.visitor_crowd(true));
        }
        assert!(!club.visitor_crowd(true));
    }
}
