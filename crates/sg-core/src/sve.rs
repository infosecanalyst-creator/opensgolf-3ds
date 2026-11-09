//! Saved games of the original SimGolf (`Saved Games\<name>.sve`), read into an OpenSGolf game. The layout is the
//! publisher's golf.exe's save writer 0x40b4a0 and its block list 0x40afa0 (the loader 0x40b9b0 runs the same list the
//! other way): a 100-byte header text, then 86 raw memory blocks in a fixed order, then up to three more that game flags
//! (read earlier in the same file) switch on. There is no version field and no checksum; the file's length tells the
//! variants apart. docs/DECODE_SAVE.md has every block, its meaning, how sure the reading is and where it goes here.
//!
//! Pure data in, data out (no files): the app (share.rs, files_ui.rs) hands the bytes over and makes the result a saved
//! game of its own. What the port has no place for is listed in `Game::dropped`, shown to the player after loading.

use crate::course::Course;
use crate::economy::{Economy, LEDGER_COLUMNS, STAFF_KINDS};
use crate::golfer::Club;
use crate::land::{self, Land, Noise, Object, Slot};
use crate::roster::{self, Person};
use crate::staff::{self, Employee};

/// The header text's bytes ("-<course name and tier>, <day> <month> <year>", 0x40b4a0).
pub const HEADER: usize = 100;

/// The blocks every save has, in file order: (exe address, bytes). 0x40afa0, EXACT.
pub const BLOCKS: [(u32, usize); 86] = [
    (0x822c78, 0x4),
    (0x5685f0, 0x4),
    (0x58d36c, 0x4),
    (0x59aafc, 0x4),
    (0x5a34e0, 0x1),
    (0x59ae78, 0x4),
    (0x541cd8, 0x4),
    (0x5a882c, 0x4),
    (0x56949c, 0x4),
    (0x5a636c, 0x4),
    (0x5a9cac, 0x4),
    (0x5a9cb0, 0x4),
    (0x5787cc, 0x4),
    (0x561258, 0x4),
    (0x571fd4, 0x4),
    (0x59bf90, 0x4),
    (0x575ab0, 0x28a0),
    (0x5722e8, 0x9c4),
    (0x56988c, 0x9c4),
    (0x53caf0, 0x1388),
    (0x53bbac, 0x9c4),
    (0x5a4998, 0xa29),
    (0x838c1c, 0x169),
    (0x58bcb8, 0x1000),
    (0x4c284c, 0x4),
    (0x561250, 0x4),
    (0x5794b8, 0x9c00),
    (0x585850, 0x1300),
    (0x834170, 0x4),
    (0x5a6364, 0x4),
    (0x53a450, 0x4),
    (0x822c88, 0x4),
    (0x5409ac, 0x3e8),
    (0x51b388, 0x3e8),
    (0x53e63c, 0x3e8),
    (0x4c2850, 0x4),
    (0x59b730, 0x4),
    (0x59ae7c, 0x4),
    (0x59dea0, 0x80),
    (0x59c08c, 0x4),
    (0x568600, 0x3e8),
    (0x822c70, 0x4),
    (0x5685f8, 0x4),
    (0x5a5a24, 0x4),
    (0x584210, 0x7d0),
    (0x543cfc, 0x4),
    (0x4c2e14, 0x4),
    (0x572cac, 0x4),
    (0x59aaf8, 0x4),
    (0x56ae70, 0x1700),
    (0x5422f8, 0xf0),
    (0x59ae80, 0x1c8),
    (0x59b048, 0x4),
    (0x5736b0, 0x2400),
    (0x59fc60, 0x3880),
    (0x5849e0, 0xe70),
    (0x5a59f8, 0x4),
    (0x5a5a04, 0x20),
    (0x56d1b8, 0x1770),
    (0x4c2c94, 0x12),
    (0x5689e8, 0x320),
    (0x5419d0, 0x40),
    (0x572cb0, 0xa00),
    (0x4d6088, 0xa640),
    (0x543d10, 0x19a28),
    (0x567b04, 0x4),
    (0x5a6378, 0x9c4),
    (0x56c7e4, 0x9c4),
    (0x53ea24, 0x9c4),
    (0x542414, 0x9c4),
    (0x59e7b8, 0x4),
    (0x4c1578, 0x420),
    (0x571fd8, 0x30e),
    (0x520640, 0x3e8),
    (0x567328, 0x64),
    (0x571d38, 0x4),
    (0x56fcb0, 0x1002),
    (0x59d81c, 0x100),
    (0x5a46b8, 0x100),
    (0x56a51c, 0x4),
    (0x5a6374, 0x4),
    (0x56a524, 0x28),
    (0x5a47e0, 0x4),
    (0x5830b8, 0x9c4),
    (0x59c090, 0x9c4),
    (0x5a5a00, 0x4),
];

/// Game flag word (0x59e7b8) bits that add blocks at the end: the last four profile records (0x4e06c8, written when bit
/// 0x40000000 is set, which every loaded or running game has), and a tournament's backup of the profiles and their
/// dialogue (0x58f338 and 0x520a28, while bit 0x200000, a tournament, is set). EXACT.
pub const FLAG_LOADED: u32 = 0x4000_0000;
pub const FLAG_TOURNAMENT: u32 = 0x20_0000;
/// Sandbox (0x1000000: no board, 250 acres) and championship play (0x4000000).
pub const FLAG_SANDBOX: u32 = 0x100_0000;
pub const FLAG_CHAMPIONSHIP: u32 = 0x400_0000;
pub const LOADED_BLOCKS: [(u32, usize); 1] = [(0x4e06c8, 0x8c0)];
pub const TOURNAMENT_BLOCKS: [(u32, usize); 2] = [(0x58f338, 0xb7c0), (0x520a28, 0x19a28)];

/// Bytes of a save with no optional block (294733).
pub const BASE_LEN: usize = {
    let mut n = HEADER;
    let mut i = 0;
    while i < BLOCKS.len() {
        n += BLOCKS[i].1;
        i += 1;
    }
    n
};

/// The original's map edge, tiles and corner pitch.
const N: usize = 50;
const TILES: usize = N * N;
const HP: usize = N + 1;

/// Records in the tables read here.
const HOLE_REC: usize = 0x208;
const OBJECTS: usize = 256;
const EMPLOYEES: usize = 64;
const EMPLOYEE_REC: usize = 0x4c;
const MEMBER_REC: usize = 0x2c;
const PROPERTY_REC: usize = 0x2e;
const AWARDS: usize = 22;
const AWARD_REC: usize = 48;
const SAYINGS_REC: usize = roster::SAYINGS * roster::SAYING;

/// Why a file is not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Shorter than any save (bytes).
    TooShort(usize),
    /// Not one of the lengths its game flags allow (bytes, expected).
    Length(usize, usize),
    /// The header is not text.
    Header,
    /// A value no save of the original holds (what).
    Value(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::TooShort(n) => write!(f, "{n} bytes, shorter than any saved game ({BASE_LEN})"),
            Error::Length(n, e) => write!(f, "{n} bytes where its flags call for {e}"),
            Error::Header => write!(f, "no header text"),
            Error::Value(w) => write!(f, "{w} out of range"),
        }
    }
}

/// A save file checked against the layout: its blocks by exe address.
pub struct Sve<'a> {
    data: &'a [u8],
    /// (address, file offset, bytes) of every block present.
    blocks: Vec<(u32, usize, usize)>,
    pub header: String,
    pub flags: u32,
}

/// Every block of a save with game flags `flags`, in file order: (address, offset, bytes), and the file's length.
pub fn layout(flags: u32) -> (Vec<(u32, usize, usize)>, usize) {
    let mut out = Vec::with_capacity(BLOCKS.len() + 3);
    let mut at = HEADER;
    let opt_loaded: &[(u32, usize)] = if flags & FLAG_LOADED != 0 { &LOADED_BLOCKS } else { &[] };
    let opt_tourney: &[(u32, usize)] = if flags & FLAG_TOURNAMENT != 0 { &TOURNAMENT_BLOCKS } else { &[] };
    for &(a, n) in BLOCKS.iter().chain(opt_loaded).chain(opt_tourney) {
        out.push((a, at, n));
        at += n;
    }
    (out, at)
}

fn le16(b: &[u8], o: usize) -> i16 {
    b.get(o..o + 2).map(|s| i16::from_le_bytes([s[0], s[1]])).unwrap_or(0)
}

fn le32(b: &[u8], o: usize) -> i32 {
    b.get(o..o + 4).map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]])).unwrap_or(0)
}

fn byte(b: &[u8], o: usize) -> u8 {
    b.get(o).copied().unwrap_or(0)
}

/// A NUL-ended string (Latin-1, as the original writes its text).
fn cstr(b: &[u8]) -> String {
    b.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect()
}

impl<'a> Sve<'a> {
    /// Checks a file: its length against the layout its flags call for, its header, and the values the port relies on.
    pub fn parse(data: &'a [u8]) -> Result<Sve<'a>, Error> {
        if data.len() < BASE_LEN {
            return Err(Error::TooShort(data.len()));
        }
        let (base, _) = layout(0);
        let at = base.iter().find(|b| b.0 == 0x59e7b8).map(|b| b.1).unwrap_or(0);
        let flags = le32(data, at) as u32;
        let (blocks, len) = layout(flags);
        if data.len() != len {
            return Err(Error::Length(data.len(), len));
        }
        let head = &data[..HEADER];
        let end = head.iter().position(|&c| c == 0).ok_or(Error::Header)?;
        if end == 0 || head[..end].iter().any(|&c| c < 0x20 || c == 0x7f) {
            return Err(Error::Header);
        }
        let s = Sve { data, blocks, header: cstr(head), flags };
        s.check()?;
        Ok(s)
    }

    /// A block's bytes (empty for an address the file does not have).
    pub fn block(&self, addr: u32) -> &'a [u8] {
        self.blocks.iter().find(|b| b.0 == addr).map(|&(_, o, n)| &self.data[o..o + n]).unwrap_or(&[])
    }

    fn int(&self, addr: u32) -> i32 {
        le32(self.block(addr), 0)
    }

    /// Cash in money units of $100 (0x571fd4).
    pub fn cash_units(&self) -> i32 {
        self.int(0x571fd4)
    }

    /// The game tick (0x834170): 1024 a month, 8192 a year (March to October).
    pub fn tick(&self) -> i32 {
        self.int(0x834170)
    }

    /// 0 easiest .. 3 (0x822c88).
    pub fn difficulty(&self) -> i32 {
        self.int(0x822c88)
    }

    /// The exe's theme: 0 Parkland, 1 Desert, 2 Tropical, 3 Links (0x5a34e0).
    pub fn theme(&self) -> u8 {
        byte(self.block(0x5a34e0), 0)
    }

    /// The offer slot of the course being played (0x59bf90) and its record in the property list (0x571fd8, 46 bytes each).
    pub fn slot(&self) -> usize {
        self.int(0x59bf90) as usize
    }

    fn slot_record(&self) -> &'a [u8] {
        let b = self.block(0x571fd8);
        let o = self.slot().min(15) * PROPERTY_REC;
        b.get(o..o + PROPERTY_REC).unwrap_or(&[])
    }

    /// The property (the exe's site id at record +0x1c; the order of `land::RECORDS`).
    pub fn property(&self) -> usize {
        byte(self.slot_record(), 0x1c) as usize
    }

    /// The custom name in the save's string pool (0x56fcb0, offsets 0x59d81c, both by name index; 0x45b9f0), if any:
    /// index 0 is the course, 1..18 the holes, 0x14.. the best rounds' golfers.
    pub fn name(&self, index: usize) -> Option<String> {
        let at = le16(self.block(0x59d81c), index * 2);
        if at < 0 || index >= 0x80 {
            return None;
        }
        let pool = self.block(0x56fcb0);
        pool.get(at as usize..).map(cstr).filter(|s| !s.is_empty())
    }

    fn check(&self) -> Result<(), Error> {
        if !(0..=3).contains(&self.difficulty()) {
            return Err(Error::Value("difficulty"));
        }
        if self.theme() > 3 {
            return Err(Error::Value("theme"));
        }
        if self.slot() > 15 || self.property() > 15 {
            return Err(Error::Value("property"));
        }
        if self.tick() < 0 {
            return Err(Error::Value("date"));
        }
        if !(1..=19).contains(&self.int(0x5685f0)) {
            return Err(Error::Value("holes"));
        }
        if self.block(0x5722e8).iter().chain(self.block(0x542414)).any(|&t| t as usize >= land::TYPES.len()) {
            return Err(Error::Value("terrain"));
        }
        if self.block(0x5a4998).iter().any(|&h| h > 20) {
            return Err(Error::Value("elevation"));
        }
        let objs = self.block(0x58bcb8);
        if (0..OBJECTS).any(|i| !(-1..20).contains(&(le16(objs, i * 16) as i32))) {
            return Err(Error::Value("buildings"));
        }
        Ok(())
    }

    /// The game in the port's terms. Runs at the original's map size whatever size is in play.
    pub fn to_game(&self) -> Game {
        let _size = crate::mapsize::scoped(crate::mapsize::ORIGINAL);
        let mut carried = vec![
            "STR_SVE_PART_LAND",
            "STR_SVE_PART_HOLES",
            "STR_SVE_PART_BUILDINGS",
            "STR_SVE_PART_MONEY",
            "STR_SVE_PART_DATE",
            "STR_SVE_PART_DIFFICULTY",
            "STR_SVE_PART_PROPERTY",
        ];
        let mut dropped = vec!["STR_SVE_LOST_GOLFERS", "STR_SVE_LOST_HOLE_STATS", "STR_SVE_LOST_WILDLIFE", "STR_SVE_LOST_STORIES"];
        let tick = self.tick() as u32;
        let difficulty = self.difficulty();
        let sandbox = self.flags & FLAG_SANDBOX != 0;
        let rec = self.slot_record();
        let property = self.property();
        let acres = byte(rec, 0x1d) as i32;
        let slot = Slot { property, acres: if sandbox || acres == 0 { 250 } else { acres } };
        let land = self.land(slot, difficulty);
        let roster = self.roster();
        let mut club = Club::new(roster);
        self.club(&mut club, tick, difficulty, sandbox, &land);
        if club.members.iter().any(|m| m.level & 7 != 0) {
            carried.push("STR_SVE_PART_MEMBERS");
        }
        carried.push("STR_SVE_PART_PEOPLE");
        let employees = self.employees();
        if employees.len() > 1 {
            carried.push("STR_SVE_PART_STAFF");
        }
        carried.push("STR_SVE_PART_RECORDS");
        if self.flags & FLAG_TOURNAMENT != 0 {
            dropped.push("STR_SVE_LOST_TOURNAMENT");
        }
        if (1..=18).any(|h| self.name(h).is_some()) {
            dropped.push("STR_SVE_LOST_HOLE_NAMES");
        }
        dropped.push("STR_SVE_LOST_OPTIONS");
        let cash = self.cash_units() as f64 * 100.0;
        let mut econ = Economy {
            start_cash: cash,
            cash,
            sandbox,
            tick,
            debt_stage: self.int(0x5a6374).clamp(0, 3),
            ledger: self.ledger(tick),
            ..Default::default()
        };
        for e in &employees {
            if (-5..=-2).contains(&e.job) {
                econ.staff[(-2 - e.job as i32) as usize % STAFF_KINDS] += 1;
            }
        }
        let mut course = Course::default();
        let counts = |addr: u32| -> Vec<u8> {
            let b = self.block(addr);
            (0..TILES).map(|i| byte(b, i)).collect()
        };
        course.landing = counts(0x53ea24).into_iter().map(|v| v as i8).collect();
        course.happy = counts(0x5a6378);
        course.unhappy = counts(0x56c7e4);
        course.sync(&land, difficulty == 0 || sandbox);
        let rel = land.slot.record();
        let course_name = self.name(0).unwrap_or_else(|| rel.course().to_string());
        club.course_name = course_name.clone();
        Game {
            header: self.header.clone(),
            course_name,
            theme: self.theme(),
            theme_pack: cstr(self.block(0x567328)).trim().to_string(),
            difficulty,
            tick,
            sandbox,
            course_id: self.int(0x822c78),
            noise: Noise::from_exe_grid(self.block(0x838c1c)),
            weeds: land.flags.iter().map(|f| f & (crate::course::f::WEED | crate::course::f::GROWING)).collect(),
            land,
            club,
            econ,
            course,
            employees,
            carried,
            dropped,
        }
    }

    fn land(&self, slot: Slot, difficulty: i32) -> Land {
        let mut land = Land::blank(self.slot(), slot, difficulty);
        let bytes = |addr: u32, n: usize| -> Vec<u8> {
            let b = self.block(addr);
            (0..n).map(|i| byte(b, i)).collect()
        };
        land.ty = bytes(0x5722e8, TILES);
        land.var = bytes(0x56988c, TILES);
        let fl = self.block(0x53caf0);
        land.flags = (0..TILES).map(|i| le16(fl, i * 2) as u16).collect();
        land.layer = bytes(0x53bbac, TILES);
        land.height = bytes(0x5a4998, HP * HP);
        land.original = bytes(0x542414, TILES);
        land.undo = bytes(0x5830b8, TILES);
        land.refund = bytes(0x59c090, TILES).into_iter().map(|v| v as i8).collect();
        // the object table: 16-byte records, kind -1 free (0x40e000 fills the first free one)
        let ob = self.block(0x58bcb8);
        let mut objects: Vec<Object> = (0..OBJECTS)
            .map(|i| {
                let r = &ob[i * 16..i * 16 + 16];
                Object {
                    kind: le16(r, 0) as i32,
                    a: le16(r, 2) as i32,
                    b: le16(r, 4) as i32,
                    dir: r[6] & 3,
                    flags: r[7],
                    sub: le32(r, 8),
                    val: le32(r, 12),
                }
            })
            .collect();
        while objects.last().is_some_and(|o| o.kind == -1) {
            objects.pop();
        }
        if let Some(c) = objects.iter().find(|o| o.kind == land::K_CLUBHOUSE) {
            land.clubhouse = (c.a, c.b);
            land.staff_start = (c.a + 1, c.b + 1);
            land.view = (c.a + 6, c.b - 4);
        }
        land.objects = objects;
        land
    }

    /// The people: the profile records (0x4d6088, 0x230 bytes each; the last four only in a save with flag 0x40000000)
    /// and their dialogue slots (0x543d10, 25 of 50 bytes each). Record 0 is the player's pro.
    fn roster(&self) -> Vec<Person> {
        let mut people = roster::fallback();
        let mut recs = self.block(0x4d6088).to_vec();
        recs.extend_from_slice(self.block(0x4e06c8));
        let says = self.block(0x543d10);
        for (i, p) in people.iter_mut().enumerate() {
            let Some(r) = recs.get(i * roster::RECORD..(i + 1) * roster::RECORD) else { break };
            if r[0x10] == 0 {
                continue;
            }
            let mut q = Person::parse(r);
            if let Some(d) = says.get(i * SAYINGS_REC..(i + 1) * SAYINGS_REC) {
                q.sayings = (0..roster::SAYINGS).map(|k| cstr(&d[k * roster::SAYING..(k + 1) * roster::SAYING])).collect();
                if q.sayings.iter().all(|s| s.is_empty()) {
                    q.sayings.clear();
                }
            }
            *p = q;
        }
        people
    }

    fn club(&self, c: &mut Club, tick: u32, difficulty: i32, sandbox: bool, land: &Land) {
        c.tick = tick;
        c.difficulty = difficulty;
        c.sandbox = sandbox;
        c.island = land.slot.record().coast == 2;
        c.course_theme = self.theme();
        c.year = (tick >> 13) as i32;
        c.cash = self.cash_units();
        c.next_hole = self.int(0x5685f0);
        c.unlocked = self.int(0x5a6364).clamp(0, 20);
        c.purchases = self.int(0x53a450);
        c.landmarks_owned = self.int(0x543cfc) as u32;
        c.free_landmarks = self.int(0x822c70) as u32;
        c.happy_endings = self.int(0x561258);
        c.trophies = self.int(0x5787cc);
        c.last_beat = self.int(0x59b048) as u32;
        c.types_announced = self.int(0x5685f8) as u32;
        c.create_counter = self.int(0x59ae7c);
        c.last_created = self.int(0x5a5a24);
        c.tee_counter = self.int(0x56a51c);
        c.wager_level = self.int(0x59b730);
        c.challenge_pro = self.int(0x4c2e14);
        c.ceo_count = self.int(0x572cac) & !0x80;
        c.donations = self.int(0x59aaf8);
        c.celeb_homes = self.int(0x4c284c);
        c.sga_score = self.int(0x561250);
        c.purse = self.int(0x567b04);
        c.narrated = [self.int(0x59c08c) as u32, self.int(0x571d38) as u32];
        let kinds = self.block(0x59dea0);
        for (k, v) in c.kind_counts.iter_mut().enumerate() {
            *v = le32(kinds, k * 4);
        }
        // the pro's skill bytes (the first 16 of 0x5a5a04; the rest are the opponent's) and the mask of those he has
        let skills = self.block(0x5a5a04);
        for (k, v) in c.pro_skill.iter_mut().enumerate() {
            *v = byte(skills, k);
        }
        c.pro_mask = le16(self.block(0x4c2c94), 8) as u16;
        // holes 1..18 (0x575ab0 + h * 0x208) and their yardage markers (0x59ae80 + h * 24)
        let hb = self.block(0x575ab0);
        let mk = self.block(0x59ae80);
        for h in 1..=18 {
            let r = &hb[h * HOLE_REC..(h + 1) * HOLE_REC];
            let rec = &mut c.holes[h];
            rec.par = r[0] as i32;
            rec.tee_facing = r[1] as i8 as i32;
            rec.fwd_facing = r[2] as i8 as i32;
            rec.length = le16(r, 4) as i32;
            rec.back = (le32(r, 8), le32(r, 0xc));
            rec.fwd = (le32(r, 0x10), le32(r, 0x14));
            rec.pin = (le32(r, 0x18), le32(r, 0x1c));
            rec.tee_shots = le32(r, 0x20);
            rec.plans = le32(r, 0x24);
            rec.time = le32(r, 0x1ec);
            rec.build_cost = le32(r, 0x1f0);
            rec.fees = le32(r, 0x1f4);
            rec.maint = le32(r, 0x1f8);
            rec.monotony = le32(r, 0x1fc);
            rec.flags = le32(r, 0x200) as u32;
            for k in 0..3 {
                rec.markers[k] = (le32(mk, h * 24 + k * 8), le32(mk, h * 24 + k * 8 + 4));
            }
        }
        // membership (0x5849e0, 0x2c bytes per person)
        let mb = self.block(0x5849e0);
        for (i, m) in c.members.iter_mut().enumerate() {
            let Some(r) = mb.get(i * MEMBER_REC..(i + 1) * MEMBER_REC) else { break };
            m.best = r[0];
            m.avg = r[1] as i8;
            m.level = r[2];
            m.holes.copy_from_slice(&r[3..0x16]);
            m.card.copy_from_slice(&r[0x16..0x29]);
            m.gone = r[0x29];
            m.rounds = le16(r, 0x2a) as i32;
        }
        // best rounds (0x56a524) and their golfers' names (string pool 0x14 + k)
        let tops = self.block(0x56a524);
        for k in 0..10 {
            c.top_rounds[k] = le32(tops, k * 4);
            c.top_names[k] = self.name(0x14 + k).unwrap_or_default();
        }
        // accomplishments (0x4c1578, 48 bytes each: the course's name, the tick at +0x28, 0 when not earned)
        let aw = self.block(0x4c1578);
        for id in 0..AWARDS {
            let r = &aw[id * AWARD_REC..(id + 1) * AWARD_REC];
            let t = le32(r, 0x28);
            if t != 0 {
                c.awards |= 1 << id;
                if let Some(e) = c.earned.get_mut(id) {
                    *e = Some(crate::records::Earned { tick: t as u32, course: cstr(&r[..0x28]) });
                }
            }
        }
        // the monthly event log and history (500 shorts each, by month)
        let shorts = |addr: u32| -> Vec<i32> {
            let b = self.block(addr);
            (0..500).map(|i| le16(b, i * 2) as i32).collect()
        };
        c.event_log = shorts(0x568600).into_iter().map(|v| v as u16).collect();
        let (cash, fun, skill, members) = (shorts(0x51b388), shorts(0x53e63c), shorts(0x5409ac), shorts(0x520640));
        c.history = (0..500).map(|i| [cash[i], fun[i], skill[i], members[i]]).collect();
        // a match or tournament in progress is not carried over: no pending match, the game flags left to the port
        c.gary = -1;
        c.game = 0;
        c.tourney_opts = 0;
    }

    /// The yearly ledger (0x584210: 100 records of 10 shorts by year mod 100, money units), years 0 to now, in dollars.
    fn ledger(&self, tick: u32) -> Vec<[f64; LEDGER_COLUMNS]> {
        let b = self.block(0x584210);
        let years = (tick >> 13) as usize + 1;
        (0..years.min(100)).map(|y| std::array::from_fn(|k| le16(b, (y % 100) * 0x14 + k * 2) as f64 * 100.0)).collect()
    }

    /// The employees (0x585850, 64 records of 0x4c bytes): position +0 / +4, post +0x10 / +0x11 (-1 none), state +0x12 (1
    /// active, 8 experienced, 0x10 leaving), job +0x13, facing +0x16, the month hired +0x22 and wages paid +0x24 (units),
    /// golfers served +0x26. What they were busy with is not carried: they stand and look for work.
    fn employees(&self) -> Vec<Employee> {
        let b = self.block(0x585850);
        let mut out = Vec::new();
        for i in 0..EMPLOYEES {
            let r = &b[i * EMPLOYEE_REC..(i + 1) * EMPLOYEE_REC];
            if r[0x12] & 1 == 0 || r[0x12] & 0x10 != 0 {
                continue;
            }
            let job = r[0x13] as i8;
            if !(-6..=-2).contains(&job) {
                continue;
            }
            let (pa, pb) = (r[0x10] as i8, r[0x11] as i8);
            out.push(Employee {
                active: true,
                job,
                upgraded: r[0x12] & 8 != 0,
                going_home: false,
                x: le32(r, 0).clamp(0, N as i32 * staff::UNIT - 1),
                y: le32(r, 4).clamp(0, N as i32 * staff::UNIT - 1),
                post: if pa >= 0 && pb >= 0 { Some((pa as i32, pb as i32)) } else { None },
                dir: (r[0x16] & 7) as i8,
                anim: staff::ANIM_STAND,
                target: -1,
                served: le16(r, 0x26),
                hired: le16(r, 0x22) as i32,
                paid: le16(r, 0x24) as i32,
                ..Default::default()
            });
        }
        // the player's pro first, as a new game makes him
        out.sort_by_key(|e| e.job != staff::job::OWNER);
        out
    }
}

/// An original saved game in the port's terms (`Sve::to_game`).
pub struct Game {
    /// The save's header text and the course's name (the custom one, else the property's course).
    pub header: String,
    pub course_name: String,
    /// The exe's theme (0 Parkland, 1 Desert, 2 Tropical, 3 Links) and theme pack folder ("Standard"...).
    pub theme: u8,
    pub theme_pack: String,
    pub difficulty: i32,
    pub tick: u32,
    pub sandbox: bool,
    /// The Top 10 course id (0x822c78).
    pub course_id: i32,
    /// The height noise grid (None only for a file too short, which `parse` refuses).
    pub noise: Option<Noise>,
    /// Weed bits per tile (0x800 a weed, 0x4000 being worked on), index a * 50 + b, for the staff's tile state.
    pub weeds: Vec<u16>,
    pub land: Land,
    pub club: Club,
    pub econ: Economy,
    pub course: Course,
    pub employees: Vec<Employee>,
    /// What came over and what the port has no place for: text keys (lang/en.txt STR_SVE_*).
    pub carried: Vec<&'static str>,
    pub dropped: Vec<&'static str>,
}

/// Reads an original saved game.
pub fn read(data: &[u8]) -> Result<Game, Error> {
    Sve::parse(data).map(|s| s.to_game())
}

/// Whether bytes have a save's length (a quick test before `read`).
pub fn plausible(data: &[u8]) -> bool {
    data.len() >= BASE_LEN && Sve::parse(data).is_ok()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Writes a save of the original's layout with the given blocks filled (a test's stand-in for the exe's writer 0x40b4a0).
    pub fn write(header: &str, flags: u32, fill: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let (blocks, len) = layout(flags);
        let mut out = vec![0u8; len];
        let h = header.as_bytes();
        out[..h.len().min(99)].copy_from_slice(&h[..h.len().min(99)]);
        for &(a, o, n) in &blocks {
            if a == 0x59e7b8 {
                out[o..o + 4].copy_from_slice(&flags.to_le_bytes());
            }
            if let Some((_, v)) = fill.iter().find(|f| f.0 == a) {
                out[o..o + v.len().min(n)].copy_from_slice(&v[..v.len().min(n)]);
            }
        }
        out
    }

    fn int(v: i32) -> Vec<u8> {
        v.to_le_bytes().to_vec()
    }

    /// A small game: offer slot 3 holding site 7 (Nova Scotia, Links), one hole open, a clubhouse, a path, a weed, $12,300,
    /// April 2003, difficulty 2, a Silver member and an experienced groundskeeper beside the pro.
    pub fn sample(flags: u32) -> Vec<u8> {
        let mut ty = vec![land::T_ROUGH; TILES];
        ty[10 * 50 + 10] = land::T_TEE;
        ty[10 * 50 + 20] = land::T_GREEN;
        ty[0] = land::T_OUT;
        let mut height = vec![3u8; HP * HP];
        height[5 * HP + 5] = 7;
        let mut flags_map = vec![0u8; TILES * 2];
        flags_map[(12 * 50 + 12) * 2] = 0x20;
        flags_map[(30 * 50 + 30) * 2 + 1] = 0x08;
        let mut objs = vec![0xffu8; OBJECTS * 16];
        let mut club = vec![0u8; 16];
        club[0..2].copy_from_slice(&15i16.to_le_bytes());
        club[2..4].copy_from_slice(&20i16.to_le_bytes());
        club[4..6].copy_from_slice(&22i16.to_le_bytes());
        club[6] = 1;
        club[7] = 0x40;
        objs[..16].copy_from_slice(&club);
        let mut holes = vec![0u8; HOLE_REC * 20];
        let h1 = HOLE_REC;
        holes[h1] = 4;
        holes[h1 + 1] = 2;
        holes[h1 + 4..h1 + 6].copy_from_slice(&250i16.to_le_bytes());
        holes[h1 + 8..h1 + 12].copy_from_slice(&10i32.to_le_bytes());
        holes[h1 + 12..h1 + 16].copy_from_slice(&10i32.to_le_bytes());
        holes[h1 + 0x18..h1 + 0x1c].copy_from_slice(&10i32.to_le_bytes());
        holes[h1 + 0x1c..h1 + 0x20].copy_from_slice(&20i32.to_le_bytes());
        holes[h1 + 0x1f4..h1 + 0x1f8].copy_from_slice(&77i32.to_le_bytes());
        let mut props = vec![0u8; 0x30e];
        props[3 * PROPERTY_REC + 0x1c] = 7;
        props[3 * PROPERTY_REC + 0x1d] = 80;
        let mut emp = vec![0u8; EMPLOYEES * EMPLOYEE_REC];
        for (i, job) in [(0usize, -6i8), (1, -4)] {
            let r = &mut emp[i * EMPLOYEE_REC..(i + 1) * EMPLOYEE_REC];
            r[0..4].copy_from_slice(&(21 * 1024 + 0x600i32).to_le_bytes());
            r[4..8].copy_from_slice(&(23 * 1024 + 0x600i32).to_le_bytes());
            r[0x10] = if i == 0 { 21 } else { 24 };
            r[0x11] = if i == 0 { 23 } else { 19 };
            r[0x12] = 1 | if i == 1 { 8 } else { 0 };
            r[0x13] = job as u8;
            r[0x22] = 9;
        }
        let mut members = vec![0u8; 84 * MEMBER_REC];
        members[5 * MEMBER_REC + 2] = 3;
        members[5 * MEMBER_REC + 0x2a] = 12;
        let mut profiles = vec![0u8; 76 * roster::RECORD];
        profiles[..8].copy_from_slice(b"Golf Pro");
        profiles[0x10..0x19].copy_from_slice(b"Sandy Lie");
        profiles[0x22] = 3;
        let mut pool = vec![0u8; 0x1002];
        pool[..10].copy_from_slice(b"Puffin Bay");
        let mut index = vec![0xffu8; 0x100];
        index[0..2].copy_from_slice(&0i16.to_le_bytes());
        let mut ledger = vec![0u8; 0x7d0];
        ledger[2 * 0x14 + 2..2 * 0x14 + 4].copy_from_slice(&40i16.to_le_bytes());
        let mut awards = vec![0u8; 0x420];
        awards[3 * AWARD_REC..3 * AWARD_REC + 4].copy_from_slice(b"Bay\0");
        awards[3 * AWARD_REC + 0x28..3 * AWARD_REC + 0x2c].copy_from_slice(&5000i32.to_le_bytes());
        let tick = 2 * 8192 + 1024 + 140;
        write(
            "-Puffin Bay Country Club, 5 April 2003",
            flags,
            &[
                (0x822c78, int(1234)),
                (0x5685f0, int(2)),
                (0x5a34e0, vec![3]),
                (0x571fd4, int(123)),
                (0x59bf90, int(3)),
                (0x575ab0, holes),
                (0x5722e8, ty.clone()),
                (0x542414, ty),
                (0x53caf0, flags_map),
                (0x5a4998, height),
                (0x58bcb8, objs),
                (0x585850, emp),
                (0x834170, int(tick)),
                (0x5a6364, int(9)),
                (0x822c88, int(2)),
                (0x584210, ledger),
                (0x5849e0, members),
                (0x4d6088, profiles),
                (0x4c1578, awards),
                (0x571fd8, props),
                (0x567328, b"Standard".to_vec()),
                (0x56fcb0, pool),
                (0x59d81c, index),
                (0x5a6374, int(1)),
                (0x5830b8, vec![0xff; TILES]),
            ],
        )
    }

    #[test]
    fn the_layout_adds_up() {
        assert_eq!(BASE_LEN, 294_733);
        assert_eq!(layout(0).1, BASE_LEN);
        assert_eq!(layout(FLAG_LOADED).1, BASE_LEN + 0x8c0);
        assert_eq!(layout(FLAG_LOADED | FLAG_TOURNAMENT).1, BASE_LEN + 0x8c0 + 0xb7c0 + 0x19a28);
        // offsets the decode notes quote
        let (b, _) = layout(FLAG_LOADED);
        let at = |a: u32| b.iter().find(|x| x.0 == a).unwrap().1;
        assert_eq!(at(0x571fd4), 153);
        assert_eq!(at(0x575ab0), 161);
        assert_eq!(at(0x5722e8), 10561);
        assert_eq!(at(0x58bcb8), 26023);
        assert_eq!(at(0x59e7b8), 282_121);
        assert_eq!(at(0x4e06c8), 294_733);
    }

    #[test]
    fn a_constructed_save_reads_back() {
        let data = sample(FLAG_LOADED);
        let s = Sve::parse(&data).unwrap();
        assert_eq!(s.header, "-Puffin Bay Country Club, 5 April 2003");
        assert_eq!((s.cash_units(), s.difficulty(), s.theme(), s.slot(), s.property()), (123, 2, 3, 3, 7));
        let g = s.to_game();
        assert_eq!(g.course_name, "Puffin Bay");
        assert_eq!(g.theme_pack, "Standard");
        assert_eq!((g.econ.cash, g.econ.debt_stage, g.tick >> 13), (12300.0, 1, 2));
        assert_eq!(g.econ.ledger.len(), 3);
        assert_eq!(g.econ.ledger[2][1], 4000.0);
        assert_eq!((g.land.slot.property, g.land.slot.acres, g.land.slot_index), (7, 80, 3));
        assert_eq!(g.land.tile(10, 10), land::T_TEE);
        assert_eq!(g.land.height[5 * HP + 5], 7);
        assert_eq!(g.land.flags[12 * 50 + 12], 0x20);
        assert_eq!(g.weeds[30 * 50 + 30], 0x800);
        assert_eq!(g.land.objects.len(), 1);
        assert_eq!((g.land.objects[0].kind, g.land.objects[0].a, g.land.objects[0].b, g.land.objects[0].dir), (15, 20, 22, 1));
        assert_eq!(g.land.clubhouse, (20, 22));
        let h = &g.club.holes[1];
        assert_eq!((h.par, h.tee_facing, h.length, h.back, h.pin, h.fees), (4, 2, 250, (10, 10), (10, 20), 77));
        assert_eq!(g.club.holes[2].par, 0);
        assert_eq!((g.club.next_hole, g.club.unlocked, g.club.difficulty), (2, 9, 2));
        assert_eq!(g.club.roster[0].name, "Sandy Lie");
        assert_eq!(g.club.roster[1].name, "Jim", "an empty record keeps the exe's person");
        assert_eq!((g.club.members[5].level, g.club.members[5].rounds), (3, 12));
        assert_eq!(g.club.earned[3].as_ref().map(|e| (e.tick, e.course.as_str())), Some((5000, "Bay")));
        assert!(g.club.awards & 8 != 0);
        assert_eq!(g.employees.len(), 2);
        assert_eq!(g.employees[0].job, staff::job::OWNER);
        assert!(g.employees[1].upgraded && g.employees[1].post == Some((24, 19)));
        assert_eq!(g.econ.staff, [0, 0, 1, 0]);
        assert!(g.carried.contains(&"STR_SVE_PART_STAFF") && g.carried.contains(&"STR_SVE_PART_MEMBERS"));
        assert!(!g.dropped.contains(&"STR_SVE_LOST_TOURNAMENT"));
        assert_eq!(g.course.ty.len(), TILES);
        assert!(g.noise.is_some());
    }

    #[test]
    fn the_flag_variants_are_read() {
        assert!(Sve::parse(&sample(0)).is_ok());
        let g = read(&sample(FLAG_LOADED | FLAG_TOURNAMENT | FLAG_SANDBOX)).unwrap();
        assert!(g.dropped.contains(&"STR_SVE_LOST_TOURNAMENT"));
        assert!(g.sandbox && g.econ.sandbox && g.club.sandbox);
        assert_eq!(g.land.slot.acres, 250);
    }

    #[test]
    fn wrong_files_are_refused() {
        let good = sample(FLAG_LOADED);
        assert_eq!(Sve::parse(&good[..1000]).err(), Some(Error::TooShort(1000)));
        assert!(matches!(Sve::parse(&good[..good.len() - 1]).err(), Some(Error::Length(..))));
        let mut longer = good.clone();
        longer.push(0);
        assert!(matches!(Sve::parse(&longer).err(), Some(Error::Length(..))));
        let mut bad = good.clone();
        bad[0] = 0;
        assert_eq!(Sve::parse(&bad).err(), Some(Error::Header));
        let (b, _) = layout(FLAG_LOADED);
        let at = |a: u32| b.iter().find(|x| x.0 == a).unwrap().1;
        let mut bad = good.clone();
        bad[at(0x822c88)] = 9;
        assert_eq!(Sve::parse(&bad).err(), Some(Error::Value("difficulty")));
        let mut bad = good.clone();
        bad[at(0x5722e8) + 7] = 200;
        assert_eq!(Sve::parse(&bad).err(), Some(Error::Value("terrain")));
        // a top10.sve and an OpenSGolf save are no saved games of the original
        assert!(Sve::parse(&[0u8; 1560]).is_err());
        assert!(!plausible(b"SIMGOLF-NATIVE-SAVE\n"));
    }

    /// Truncated, garbage and damaged input never panics: it is refused or read.
    #[test]
    fn garbage_never_panics() {
        let mut rng = crate::land::ExeRng { state: 7 };
        let good = sample(FLAG_LOADED);
        for n in [0, 1, 99, 100, 101, BASE_LEN - 1, BASE_LEN, BASE_LEN + 1, good.len() - 4] {
            let _ = read(&good[..n.min(good.len())]);
        }
        for len in [BASE_LEN, layout(FLAG_LOADED).1] {
            for _ in 0..8 {
                let junk: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
                assert!(read(&junk).is_err(), "random bytes are refused");
            }
        }
        // a good save with bytes changed here and there: refused or read, never a panic
        for k in 0..200 {
            let mut d = good.clone();
            for _ in 0..(1 + k % 40) {
                let i = (rng.below(0x7fff) as usize * 37 + rng.below(0x7fff) as usize) % d.len();
                d[i] = rng.below(256) as u8;
            }
            if let Ok(g) = read(&d) {
                assert_eq!(g.land.ty.len(), TILES);
            }
        }
        // the flags word pointing at blocks the file does not have
        let (b, _) = layout(0);
        let at = b.iter().find(|x| x.0 == 0x59e7b8).unwrap().1;
        let mut d = sample(0);
        d[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read(&d).is_err());
    }
}
