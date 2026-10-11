//! The game's data tables as content packs can change them (docs/MODDING.md, section 3).
//!
//! The engine's built-in tables (the exe's terrain table in course::TYPES and land::TYPES, the object list
//! land::BUILDINGS, the property records land::RECORDS, the wages economy::WAGE_UNITS, the offer prices
//! land::SLOT_PRICE_UNITS and the economy's constants) stay where they are and are the `original` base set's data. This
//! module turns them into [`Tables`] (one list of entries with an `id` per file), merges the `data/*.toml` files of the
//! enabled packs over them entry by entry, and builds the [`Data`] the game reads through [`get`]. Until a merged set is
//! installed, [`get`] returns the built-in tables unchanged, so a game without packs reads exactly the values it always did.
//!
//! Money in the files is in the game's money as the player sees it (a fairway costs 300); the engine counts in units of
//! 100, so every amount must be a multiple of 100.
use crate::course::TypeRow;
use crate::land::{PropertyRecord, TypeInfo};
use crate::properties::Property;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::OnceLock;

/// Tile ids the terrain table has entries for: the exe's 23 types, then the paint-only ids of the port's terrain
/// (water depths, tricky green, sand bunker) that only have a price.
pub const TERRAIN_SLOTS: usize = 28;
/// The exe's tile types with a full row (bounce, roll, lie...): 0..22.
pub const TILE_ROWS: usize = 23;
/// The data files a pack may have, in the order they are read.
pub const FILES: [&str; 6] = ["terrain", "buildings", "amenities", "staff", "properties", "economy"];
/// Object kinds listed in amenities.toml (the rest of the 20 kinds are in buildings.toml).
pub const AMENITY_KINDS: [usize; 9] = [0, 1, 2, 3, 4, 16, 17, 18, 19];

const TERRAIN_IDS: [&str; TERRAIN_SLOTS] = [
    "tee",
    "green",
    "fairway",
    "firm-fairway",
    "rough",
    "deep-rough",
    "mounds",
    "sand-trap",
    "waste-bunker",
    "pot-bunker",
    "ravine",
    "brush",
    "rocks",
    "tree",
    "pine-tree",
    "palm-tree",
    "elm-tree",
    "water",
    "wetlands",
    "marsh",
    "out-of-bounds",
    "home-site",
    "building",
    "water-middle",
    "water-deep",
    "water-shallow-desert",
    "tricky-green",
    "sand-bunker",
];
const EXTRA_TERRAIN_NAMES: [&str; TERRAIN_SLOTS - TILE_ROWS] = ["water", "deep water", "desert water", "tricky green", "sand trap"];

const OBJECT_IDS: [&str; 20] = [
    "pathway",
    "benches",
    "flower-bed",
    "ball-washer",
    "landmark",
    "home-site",
    "putting-green",
    "snack-bar",
    "pro-shop",
    "swim-club",
    "driving-range",
    "cart-garage",
    "marina",
    "resort-hotel",
    "airstrip",
    "clubhouse",
    "willow-tree",
    "tv-tower",
    "tv-booth",
    "scenic-bridge",
];
const STAFF_IDS: [&str; 4] = ["club-pro", "ranger", "groundskeeper", "soda-vendor"];
/// The employee kinds' names (text keys; the files hold the English text).
const STAFF_NAMES: [&str; 4] = ["STR_STAFF_CLUB_PRO", "STR_STAFF_RANGER", "STR_STAFF_GROUNDSKEEPER", "STR_STAFF_SODA_VENDOR"];

/// Money as the engine counts it (units of 100) from money as the files give it.
fn units(money: i64, what: &str) -> Result<i32, String> {
    if money % 100 != 0 {
        return Err(format!("{what} = {money}: money must be a multiple of 100"));
    }
    if !(0..=2_000_000_000).contains(&money) {
        return Err(format!("{what} = {money}: out of range"));
    }
    Ok((money / 100) as i32)
}

fn in_range(v: i64, lo: i64, hi: i64, what: &str) -> Result<(), String> {
    if (lo..=hi).contains(&v) {
        Ok(())
    } else {
        Err(format!("{what} = {v}: must be {lo}..{hi}"))
    }
}

// ---- the entries of each file ------------------------------------------------------------------------------------------------

/// One entry of a data file: an `id`, the engine slot it fills, and a check of its values.
pub trait Entry: Serialize + DeserializeOwned + Clone {
    fn id(&self) -> &str;
    /// The engine slot (tile id, object kind, staff kind, property index) this entry describes.
    fn slot(&self) -> i64;
    fn check(&self) -> Result<(), String>;
}

/// data/terrain.toml: a tile type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDef {
    pub id: String,
    /// The tile id (0..27); fixed for the built-in entries.
    pub tile: u8,
    /// The type's name in the exe's terrain table (tract descriptions use it).
    pub name: String,
    /// What painting one tile of it costs (the Build Course panel shows it).
    pub cost: i64,
    /// What clearing one tile of it costs before something else is painted or built on it.
    pub clear_cost: i64,
    /// What one such tile next to a home site adds to the lot's view (the exe's price byte, +0x23).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<i32>,
    /// How high a ball bounces on it (0..15; the tooltip's bounce meter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounce: Option<i32>,
    /// How far a ball rolls on it (0..15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roll: Option<i32>,
    /// The lie: how hard a shot from it is (-1 green .. 8; the tooltip's face).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hazard: Option<i8>,
    /// What walking over it costs a golfer or an employee (also the tile's draw layer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk: Option<i32>,
    /// The rough class: tiles of one class blend at their borders (0 tee, 1 green, 2 fairway, 4 rough, 7 sand, 8 waste,
    /// 13 trees, 17 water, 18 out of bounds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<u8>,
    /// The scenery group: which art set (trees, rocks, grasses) is drawn on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u8>,
    /// Shape flags: 1 level table, 2 flat at the lowest corner, 4 flat at the highest, 8 height 0, 0x100.. scenery bits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
}

impl Entry for TerrainDef {
    fn id(&self) -> &str {
        &self.id
    }
    fn slot(&self) -> i64 {
        self.tile as i64
    }
    fn check(&self) -> Result<(), String> {
        units(self.cost, "cost")?;
        let clear = units(self.clear_cost, "clear_cost")?;
        if (self.tile as usize) < TILE_ROWS {
            in_range(clear as i64, 0, 255, "clear_cost / 100")?;
            let need = [
                ("view", self.view.is_some()),
                ("bounce", self.bounce.is_some()),
                ("roll", self.roll.is_some()),
                ("hazard", self.hazard.is_some()),
                ("walk", self.walk.is_some()),
                ("class", self.class.is_some()),
                ("group", self.group.is_some()),
                ("flags", self.flags.is_some()),
            ];
            if let Some((f, _)) = need.iter().find(|(_, ok)| !ok) {
                return Err(format!("tile {} needs `{f}`", self.tile));
            }
            in_range(self.bounce.unwrap_or(0) as i64, 0, 15, "bounce")?;
            in_range(self.roll.unwrap_or(0) as i64, 0, 15, "roll")?;
            in_range(self.hazard.unwrap_or(0) as i64, -1, 8, "hazard")?;
            in_range(self.walk.unwrap_or(0) as i64, 0, 255, "walk")?;
            in_range(self.view.unwrap_or(0) as i64, 0, 255, "view")?;
        }
        Ok(())
    }
}

/// data/buildings.toml and data/amenities.toml: an object the player can place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectDef {
    pub id: String,
    /// The exe's object kind (0..19); fixed for the built-in entries.
    pub kind: u8,
    pub name: String,
    /// The edge of its square footprint, in tiles.
    pub footprint: i32,
    /// Its price (an upgrade of a building costs price x (level + 2) / 2).
    pub price: i64,
}

impl Entry for ObjectDef {
    fn id(&self) -> &str {
        &self.id
    }
    fn slot(&self) -> i64 {
        self.kind as i64
    }
    fn check(&self) -> Result<(), String> {
        units(self.price, "price")?;
        in_range(self.footprint as i64, 1, 8, "footprint")
    }
}

/// data/staff.toml: a kind of employee.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaffDef {
    pub id: String,
    /// The employee kind (0 Club Pro, 1 Ranger, 2 Groundskeeper, 3 Soda Vendor); fixed for the built-in entries.
    pub kind: u8,
    pub name: String,
    /// The wage charged each pay interval (the hire panel shows it "per week").
    pub wage: i64,
    /// The wage of an experienced employee.
    pub wage_experienced: i64,
}

impl Entry for StaffDef {
    fn id(&self) -> &str {
        &self.id
    }
    fn slot(&self) -> i64 {
        self.kind as i64
    }
    fn check(&self) -> Result<(), String> {
        units(self.wage, "wage")?;
        units(self.wage_experienced, "wage_experienced").map(|_| ())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Parkland,
    Desert,
    Tropical,
    Links,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Coast {
    Inland,
    Coast,
    Island,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Relief {
    Gentle,
    Rolling,
    Hilly,
}

/// data/properties.toml: one of the sixteen properties of the world map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyDef {
    pub id: String,
    /// The property's place in the exe's list (0..15; 0..9 on the map's left arc, 10..15 on the right); fixed for the
    /// built-in entries.
    pub index: u8,
    /// The place, as the world map shows it.
    pub name: String,
    /// The course's own name.
    pub course: String,
    /// The property's bonus, as the world map shows it.
    pub bonus: String,
    /// The marker on the world map art.
    pub map_x: i32,
    pub map_y: i32,
    pub theme: Theme,
    /// inland: the owned land is a square; coast: the sea runs along one side; island: the land is ringed by water.
    pub coast: Coast,
    pub relief: Relief,
}

impl Entry for PropertyDef {
    fn id(&self) -> &str {
        &self.id
    }
    fn slot(&self) -> i64 {
        self.index as i64
    }
    fn check(&self) -> Result<(), String> {
        if self.name.is_empty() || self.course.is_empty() {
            return Err("name and course must not be empty".into());
        }
        Ok(())
    }
}

/// data/economy.toml: the money rules, one `[economy]` table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EconomyDef {
    /// What a normal game starts with, before the property's price is taken off.
    pub start_funds: i64,
    /// What a sandbox game starts with (the property is free).
    pub sandbox_funds: i64,
    /// The price of each of the sixteen offer slots of the world map, cheapest first.
    pub offer_prices: Vec<i64>,
    /// What firing an employee costs.
    pub firing_cost: i64,
    /// Maintenance of each active hole, charged with the wages every 1024 / (difficulty + 2) ticks.
    pub hole_maintenance: i64,
    /// Interest on debt, in percent of the (negative) cash, charged at each month start.
    pub debt_interest_percent: i64,
    /// Holes needed for the course ranks Daily Fee, Country Club and Championship (below the first: Municipal).
    pub rank_holes: Vec<i64>,
}

impl EconomyDef {
    pub fn check(&self) -> Result<(), String> {
        units(self.start_funds, "start_funds")?;
        units(self.sandbox_funds, "sandbox_funds")?;
        units(self.firing_cost, "firing_cost")?;
        units(self.hole_maintenance, "hole_maintenance")?;
        in_range(self.debt_interest_percent, 0, 100, "debt_interest_percent")?;
        if self.offer_prices.len() != 16 {
            return Err(format!("offer_prices has {} prices, needs 16", self.offer_prices.len()));
        }
        for &p in &self.offer_prices {
            units(p, "offer_prices")?;
        }
        if self.rank_holes.len() != 3 || !self.rank_holes.windows(2).all(|w| w[0] < w[1]) || self.rank_holes[0] < 1 {
            return Err("rank_holes needs three hole counts going up, e.g. [6, 10, 18]".into());
        }
        Ok(())
    }
}

// ---- the tables -------------------------------------------------------------------------------------------------------------

/// Every data file's entries, as merged so far.
#[derive(Clone, Debug, PartialEq)]
pub struct Tables {
    pub terrain: Vec<TerrainDef>,
    pub buildings: Vec<ObjectDef>,
    pub amenities: Vec<ObjectDef>,
    pub staff: Vec<StaffDef>,
    pub properties: Vec<PropertyDef>,
    pub economy: EconomyDef,
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

fn theme_of(t: u8) -> Theme {
    [Theme::Parkland, Theme::Desert, Theme::Tropical, Theme::Links][(t as usize).min(3)]
}

fn object_def(kind: usize) -> ObjectDef {
    let (name, footprint, price) = crate::land::BUILDINGS[kind];
    ObjectDef { id: OBJECT_IDS[kind].into(), kind: kind as u8, name: en(name).into(), footprint, price: price as i64 * 100 }
}

impl Tables {
    /// The engine's built-in tables.
    pub fn builtin() -> Tables {
        use crate::economy::Economy;
        let terrain = (0..TERRAIN_SLOTS)
            .map(|t| {
                let row = (t < TILE_ROWS).then(|| crate::course::TYPES[t]);
                TerrainDef {
                    id: TERRAIN_IDS[t].into(),
                    tile: t as u8,
                    name: if t < TILE_ROWS { en(crate::land::TYPES[t].name_key).into() } else { EXTRA_TERRAIN_NAMES[t - TILE_ROWS].into() },
                    cost: Economy::builtin_terrain_cost_units(t as i32) as i64 * 100,
                    clear_cost: Economy::builtin_terrain_clear_units(t as i32) as i64 * 100,
                    view: row.map(|_| crate::homes::BUILTIN_VIEW[t]),
                    bounce: row.map(|r| r.bounce),
                    roll: row.map(|r| r.roll),
                    hazard: row.map(|r| r.hazard),
                    walk: row.map(|r| r.walk),
                    class: row.map(|r| r.class),
                    group: row.map(|r| r.group),
                    flags: row.map(|r| r.flags),
                }
            })
            .collect();
        let buildings = (0..20).filter(|k| !AMENITY_KINDS.contains(k)).map(object_def).collect();
        let amenities = AMENITY_KINDS.iter().map(|&k| object_def(k)).collect();
        let staff = (0..4)
            .map(|k| StaffDef {
                id: STAFF_IDS[k].into(),
                kind: k as u8,
                name: en(STAFF_NAMES[k]).into(),
                wage: crate::economy::WAGE_UNITS[k][0] as i64 * 100,
                wage_experienced: crate::economy::WAGE_UNITS[k][1] as i64 * 100,
            })
            .collect();
        let properties = crate::land::RECORDS
            .iter()
            .enumerate()
            .map(|(i, r)| PropertyDef {
                id: slug(en(r.name_key)),
                index: i as u8,
                name: en(r.name_key).into(),
                course: en(r.course_key).into(),
                bonus: en(r.bonus_key).into(),
                map_x: r.map_x,
                map_y: r.map_y,
                theme: theme_of(r.theme),
                coast: [Coast::Inland, Coast::Coast, Coast::Island][(r.coast as usize).min(2)],
                relief: [Relief::Gentle, Relief::Rolling, Relief::Hilly][(r.relief as usize).min(2)],
            })
            .collect();
        let economy = EconomyDef {
            start_funds: crate::properties::START_FUNDS as i64,
            // the exe starts a sandbox game with 10000 units
            sandbox_funds: crate::properties::START_FUNDS as i64 * 10,
            offer_prices: crate::land::SLOT_PRICE_UNITS.iter().map(|&u| u as i64 * 100).collect(),
            firing_cost: Economy::FIRING_UNITS as i64 * 100,
            hole_maintenance: Economy::UNIT as i64,
            debt_interest_percent: 2,
            rank_holes: vec![6, 10, 18],
        };
        Tables { terrain, buildings, amenities, staff, properties, economy }
    }

    /// Merges one data file of a pack (`file` is its name without `.toml`, e.g. "terrain"). Problems (a bad file, a bad
    /// entry) are returned as messages and the entries they concern are left as they were; the rest of the file applies.
    pub fn merge(&mut self, file: &str, text: &str) -> Vec<String> {
        let mut problems = Vec::new();
        let doc: toml::Table = match text.parse() {
            Ok(d) => d,
            Err(e) => return vec![format!("data/{file}.toml: {}", toml_error(text, &e))],
        };
        let builtin = builtin_tables();
        let key = table_key(file);
        for k in doc.keys().filter(|k| k.as_str() != key) {
            problems.push(format!("data/{file}.toml: unknown table [{k}] (expected [[{key}]])"));
        }
        let Some(v) = doc.get(key) else { return problems };
        let mut p = |e: String| problems.push(format!("data/{file}.toml: {e}"));
        match file {
            "terrain" => merge_list(&mut self.terrain, &builtin.terrain, v, &mut p),
            "buildings" => merge_list(&mut self.buildings, &builtin.buildings, v, &mut p),
            "amenities" => merge_list(&mut self.amenities, &builtin.amenities, v, &mut p),
            "staff" => merge_list(&mut self.staff, &builtin.staff, v, &mut p),
            "properties" => merge_list(&mut self.properties, &builtin.properties, v, &mut p),
            "economy" => match v {
                toml::Value::Table(t) => {
                    let mut base = toml::Table::try_from(&self.economy).unwrap_or_default();
                    for (k, v) in t {
                        base.insert(k.clone(), v.clone());
                    }
                    match toml::Value::Table(base).try_into::<EconomyDef>() {
                        Ok(e) => match e.check() {
                            Ok(()) => self.economy = e,
                            Err(e) => p(format!("[economy]: {e}")),
                        },
                        Err(e) => p(format!("[economy]: {}", toml_error("", &e))),
                    }
                }
                _ => p("economy must be one [economy] table".into()),
            },
            _ => p("not a data file the engine reads".into()),
        }
        problems
    }

    /// The tables as the files a pack would hold, by file name (without `.toml`).
    pub fn to_toml(&self) -> Vec<(&'static str, String)> {
        fn list<T: Serialize>(key: &str, v: &[T]) -> String {
            let mut t = toml::Table::new();
            t.insert(key.into(), toml::Value::Array(v.iter().filter_map(|e| toml::Value::try_from(e).ok()).collect()));
            toml::to_string(&t).unwrap_or_default()
        }
        let mut eco = toml::Table::new();
        eco.insert("economy".into(), toml::Value::try_from(&self.economy).unwrap_or(toml::Value::Table(Default::default())));
        vec![
            ("terrain", list("terrain", &self.terrain)),
            ("buildings", list("building", &self.buildings)),
            ("amenities", list("amenity", &self.amenities)),
            ("staff", list("staff", &self.staff)),
            ("properties", list("property", &self.properties)),
            ("economy", toml::to_string(&eco).unwrap_or_default()),
        ]
    }
}

/// A TOML error on one line: "line N: what is wrong" (the line when the error points into `text`).
pub fn toml_error(text: &str, e: &toml::de::Error) -> String {
    let msg = e.message().trim().replace('\n', " ");
    match e.span() {
        Some(s) if !text.is_empty() => format!("line {}: {msg}", text[..s.start.min(text.len())].matches('\n').count() + 1),
        _ => msg,
    }
}

/// The key of a file's entries: `[[terrain]]`, `[[building]]`, `[[amenity]]`, `[[staff]]`, `[[property]]`, `[economy]`.
pub fn table_key(file: &str) -> &'static str {
    match file {
        "terrain" => "terrain",
        "buildings" => "building",
        "amenities" => "amenity",
        "staff" => "staff",
        "properties" => "property",
        _ => "economy",
    }
}

fn merge_list<T: Entry>(list: &mut Vec<T>, builtin: &[T], v: &toml::Value, p: &mut dyn FnMut(String)) {
    let Some(arr) = v.as_array() else {
        p("entries must be an array of tables ([[...]])".into());
        return;
    };
    for e in arr {
        let Some(t) = e.as_table() else {
            p("an entry is not a table".into());
            continue;
        };
        let Some(id) = t.get("id").and_then(|v| v.as_str()) else {
            p("an entry has no `id`".into());
            continue;
        };
        let at = list.iter().position(|x| x.id() == id);
        match t.get("remove") {
            Some(toml::Value::Boolean(true)) => {
                match at {
                    Some(i) => {
                        list.remove(i);
                    }
                    None => p(format!("{id}: nothing to remove")),
                }
                continue;
            }
            Some(toml::Value::Boolean(false)) | None => {}
            Some(_) => {
                p(format!("{id}: `remove` must be true or false"));
                continue;
            }
        }
        let mut base = at.and_then(|i| toml::Table::try_from(&list[i]).ok()).unwrap_or_default();
        for (k, v) in t.iter().filter(|(k, _)| k.as_str() != "remove") {
            base.insert(k.clone(), v.clone());
        }
        let entry = match toml::Value::Table(base).try_into::<T>() {
            Ok(x) => x,
            Err(e) => {
                p(format!("{id}: {}", toml_error("", &e)));
                continue;
            }
        };
        if let Err(e) = entry.check() {
            p(format!("{id}: {e}"));
            continue;
        }
        if let Some(b) = builtin.iter().find(|b| b.id() == id) {
            if b.slot() != entry.slot() {
                p(format!("{id}: its slot ({}) is fixed and cannot change", b.slot()));
                continue;
            }
        }
        match at {
            Some(i) => list[i] = entry,
            None => list.push(entry),
        }
    }
}

// ---- what the engine reads --------------------------------------------------------------------------------------------------

/// The engine's view of the merged tables, laid out by slot like the built-in tables it replaces.
#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    /// course::TYPES: the runtime terrain rows.
    pub rows: [TypeRow; TILE_ROWS],
    /// land::TYPES: the same table as the land generator and the editor read it.
    pub land_types: [TypeInfo; TILE_ROWS],
    /// Painting and clearing costs in units, by tile id 0..27.
    pub paint_units: [i32; TERRAIN_SLOTS],
    pub clear_units: [i32; TERRAIN_SLOTS],
    /// The home-site view weight of each type (homes.rs).
    pub views: [i32; TILE_ROWS],
    /// land::BUILDINGS: (name, footprint, price in units) by object kind.
    pub buildings: [(&'static str, i32, i32); 20],
    /// economy::WAGE_UNITS: (basic, experienced) wage in units by employee kind.
    pub wages: [[i32; 2]; 4],
    pub staff_names: [&'static str; 4],
    /// land::RECORDS and properties::PROPERTIES.
    pub records: [PropertyRecord; 16],
    pub properties: [Property; 16],
    pub economy: EconomyData,
}

/// The economy's constants in engine units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EconomyData {
    /// Starting money (in money, not units).
    pub start_funds: i32,
    pub sandbox_funds: i32,
    /// land::SLOT_PRICE_UNITS.
    pub offer_units: [i32; 16],
    pub firing_units: i32,
    pub maintenance_units: i32,
    pub interest_percent: i32,
    pub rank_holes: [usize; 3],
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// The built-in English text of a key (the data files hold names as text).
fn en(key: &'static str) -> &'static str {
    crate::lang::en(key)
}

/// A name for the engine's tables: the built-in key while the name is the built-in English text (so it follows the
/// player's language), else the pack's own text (`crate::lang::text` shows either).
fn name_of(text: &str, key: &'static str) -> &'static str {
    if text == en(key) {
        key
    } else {
        leak(text)
    }
}

impl Data {
    /// Builds the engine's tables from merged ones. Built-in entries a pack removed keep their built-in values (the engine
    /// has a slot for each); entries with ids the engine does not know are kept in the tables but not used. Both are
    /// reported in the returned warnings.
    pub fn from_tables(t: &Tables) -> (Data, Vec<String>) {
        let b = builtin_tables();
        let mut warn = Vec::new();
        fn pick<'a, T: Entry>(list: &'a [T], builtin: &'a [T], id: &str, file: &str, warn: &mut Vec<String>) -> &'a T {
            match list.iter().find(|e| e.id() == id) {
                Some(e) => e,
                None => {
                    warn.push(format!("data/{file}.toml: {id} was removed, but the engine needs it: the built-in values are used"));
                    builtin.iter().find(|e| e.id() == id).expect("built-in entry")
                }
            }
        }
        fn unknown<T: Entry>(list: &[T], builtin: &[T], file: &str, warn: &mut Vec<String>) {
            for e in list.iter().filter(|e| !builtin.iter().any(|b| b.id() == e.id())) {
                warn.push(format!("data/{file}.toml: {} is a new entry; the engine has no use for it yet", e.id()));
            }
        }
        unknown(&t.terrain, &b.terrain, "terrain", &mut warn);
        unknown(&t.buildings, &b.buildings, "buildings", &mut warn);
        unknown(&t.amenities, &b.amenities, "amenities", &mut warn);
        unknown(&t.staff, &b.staff, "staff", &mut warn);
        unknown(&t.properties, &b.properties, "properties", &mut warn);

        let ter: Vec<&TerrainDef> = TERRAIN_IDS.iter().map(|id| pick(&t.terrain, &b.terrain, id, "terrain", &mut warn)).collect();
        let rows: [TypeRow; TILE_ROWS] = std::array::from_fn(|i| {
            let d = ter[i];
            let r = crate::course::TYPES[i];
            TypeRow {
                bounce: d.bounce.unwrap_or(r.bounce),
                roll: d.roll.unwrap_or(r.roll),
                hazard: d.hazard.unwrap_or(r.hazard),
                walk: d.walk.unwrap_or(r.walk),
                class: d.class.unwrap_or(r.class),
                group: d.group.unwrap_or(r.group),
                flags: d.flags.unwrap_or(r.flags),
            }
        });
        let clear_units: [i32; TERRAIN_SLOTS] = std::array::from_fn(|i| (ter[i].clear_cost / 100) as i32);
        let land_types: [TypeInfo; TILE_ROWS] = std::array::from_fn(|i| TypeInfo {
            name_key: name_of(&ter[i].name, crate::land::TYPES[i].name_key),
            // the land table reads the same bytes of the exe's terrain table: +0x22 (the lie), +0x24 (the clearing cost),
            // +0x25 (walking cost / layer) and +0x26 (the class)
            desert_group: rows[i].hazard,
            clear_cost: clear_units[i].clamp(0, 255) as u8,
            layer: rows[i].walk.clamp(0, 255) as u8,
            class: rows[i].class,
        });
        let objects: Vec<&ObjectDef> = (0..20)
            .map(|k| {
                if AMENITY_KINDS.contains(&k) {
                    pick(&t.amenities, &b.amenities, OBJECT_IDS[k], "amenities", &mut warn)
                } else {
                    pick(&t.buildings, &b.buildings, OBJECT_IDS[k], "buildings", &mut warn)
                }
            })
            .collect();
        let staff: Vec<&StaffDef> = STAFF_IDS.iter().map(|id| pick(&t.staff, &b.staff, id, "staff", &mut warn)).collect();
        let props: Vec<&PropertyDef> =
            b.properties.iter().map(|bp| pick(&t.properties, &b.properties, &bp.id, "properties", &mut warn)).collect();
        let records: [PropertyRecord; 16] = std::array::from_fn(|i| {
            let p = props[i];
            let r = &crate::land::RECORDS[i];
            PropertyRecord {
                name_key: name_of(&p.name, r.name_key),
                course_key: name_of(&p.course, r.course_key),
                bonus_key: name_of(&p.bonus, r.bonus_key),
                map_x: p.map_x,
                map_y: p.map_y,
                theme: p.theme as u8,
                coast: p.coast as u8,
                relief: p.relief as u8,
            }
        });
        let e = &t.economy;
        let u = |m: i64| (m / 100) as i32;
        let data = Data {
            rows,
            land_types,
            paint_units: std::array::from_fn(|i| u(ter[i].cost)),
            clear_units,
            views: std::array::from_fn(|i| ter[i].view.unwrap_or(0)),
            buildings: std::array::from_fn(|k| {
                (name_of(&objects[k].name, crate::land::BUILDINGS[k].0), objects[k].footprint, u(objects[k].price))
            }),
            wages: std::array::from_fn(|k| [u(staff[k].wage), u(staff[k].wage_experienced)]),
            staff_names: std::array::from_fn(|k| name_of(&staff[k].name, STAFF_NAMES[k])),
            records,
            properties: std::array::from_fn(|i| Property {
                name_key: records[i].name_key,
                course_key: records[i].course_key,
                bonus_key: records[i].bonus_key,
                theme: crate::land::port_theme(records[i].theme),
                column: if i < 10 { 0 } else { 1 },
            }),
            economy: EconomyData {
                start_funds: e.start_funds as i32,
                sandbox_funds: e.sandbox_funds as i32,
                offer_units: std::array::from_fn(|i| u(e.offer_prices[i])),
                firing_units: u(e.firing_cost),
                maintenance_units: u(e.hole_maintenance),
                interest_percent: e.debt_interest_percent as i32,
                rank_holes: std::array::from_fn(|i| e.rank_holes[i] as usize),
            },
        };
        (data, warn)
    }
}

fn builtin_tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(Tables::builtin)
}

static BUILTIN: OnceLock<Data> = OnceLock::new();
static CURRENT: AtomicPtr<Data> = AtomicPtr::new(std::ptr::null_mut());

/// The tables the game reads: the merged ones once [`install`]ed, the built-in ones before.
pub fn get() -> &'static Data {
    let p = CURRENT.load(Ordering::Acquire);
    if p.is_null() {
        BUILTIN.get_or_init(|| Data::from_tables(builtin_tables()).0)
    } else {
        // SAFETY: CURRENT only ever holds a pointer from Box::into_raw in `install`, and that box is never freed (a table
        // handed out by `get` may be held for the rest of the run).
        unsafe { &*p }
    }
}

/// Makes `d` the tables the game reads, from now on. The tables it replaces stay allocated (they may still be borrowed).
pub fn install(d: Data) {
    CURRENT.store(Box::into_raw(Box::new(d)), Ordering::Release);
}

/// Back to the built-in tables.
pub fn reset() {
    CURRENT.store(std::ptr::null_mut(), Ordering::Release);
}

// ---- accessors (the reads the engine routes through here) ------------------------------------------------------------------

/// The runtime terrain row of a type (types past 22 read row 22, as the exe's table ends there).
pub fn row(ty: impl Into<i64>) -> &'static TypeRow {
    &get().rows[(ty.into().clamp(0, TILE_ROWS as i64 - 1)) as usize]
}

/// The land table's row of a type.
pub fn land_type(ty: impl Into<i64>) -> &'static TypeInfo {
    &get().land_types[(ty.into().clamp(0, TILE_ROWS as i64 - 1)) as usize]
}

/// Units it costs to paint a tile of a type; 0 for ids the table does not have.
pub fn paint_units(ty: i32) -> i32 {
    get().paint_units.get(ty as usize).copied().unwrap_or(0)
}

/// Units it costs to clear a tile of a type before painting or building over it; 0 for ids the table does not have.
pub fn clear_units(ty: i32) -> i32 {
    get().clear_units.get(ty as usize).copied().unwrap_or(0)
}

/// The object table: (name, footprint, price in units) by kind.
pub fn buildings() -> &'static [(&'static str, i32, i32); 20] {
    &get().buildings
}

/// The property records.
pub fn records() -> &'static [PropertyRecord; 16] {
    &get().records
}

/// The properties as the chooser lays them out.
pub fn properties() -> &'static [Property; 16] {
    &get().properties
}

/// Wage units by employee kind and experience.
pub fn wages() -> &'static [[i32; 2]; 4] {
    &get().wages
}

pub fn economy() -> &'static EconomyData {
    &get().economy
}

/// The id of an object kind (buildings.toml and amenities.toml), e.g. "snack-bar"; empty for an unknown kind.
pub fn object_id(kind: usize) -> &'static str {
    OBJECT_IDS.get(kind).copied().unwrap_or("")
}

/// The id of the property in slot `i` (properties.toml), e.g. "las-vegas". The ids of the slots are fixed.
pub fn property_id(i: usize) -> String {
    crate::land::RECORDS.get(i).map(|r| slug(en(r.name_key))).unwrap_or_default()
}

/// The slot of a property id.
pub fn property_index(id: &str) -> Option<usize> {
    (0..crate::land::RECORDS.len()).find(|&i| property_id(i) == id)
}

/// The theme of the property in slot `i` as the loaded tables have it.
pub fn property_theme(i: usize) -> Theme {
    theme_of(records().get(i).map(|r| r.theme).unwrap_or(0))
}

// ---- the template for modders ------------------------------------------------------------------------------------------------

/// The comment at the top of each file `sgtool dump-data` writes: what the file is and what each field means.
pub fn file_header(file: &str) -> &'static str {
    match file {
        "terrain" => {
            "# data/terrain.toml: tile types (OpenSGolf built-in values, written by `sgtool dump-data`).
# Merge rules: an entry with the id of an earlier one replaces the fields it gives; `remove = true` deletes it.
# Money is the game's money as shown (a multiple of 100).
#   id          the entry's id (fixed for built-in entries)
#   tile        tile id 0..27 (fixed); 0..22 are the exe's types, 23..27 paint-only ids with a price only
#   name        the type's name in the exe's table (tract descriptions)
#   cost        price of painting one tile
#   clear_cost  price of clearing one tile of it before painting or building over it
#   view        what one such tile next to a home site adds to the lot's view
#   bounce      0..15, how high a ball bounces on it
#   roll        0..15, how far a ball rolls on it
#   hazard      the lie, -1 (green) .. 8: how hard a shot from it is
#   walk        walking cost for golfers and staff (also the draw layer)
#   class       blend class (0 tee, 1 green, 2 fairway, 4 rough, 7 sand, 8 waste, 13 trees, 17 water, 18 out of bounds)
#   group       scenery group: which trees, rocks or grasses are drawn on it
#   flags       shape flags: 1 level table, 2 flat at the lowest corner, 4 flat at the highest, 8 height 0
"
        }
        "buildings" | "amenities" => {
            "# data/buildings.toml and data/amenities.toml: objects the player places (OpenSGolf built-in values).
# Merge rules: an entry with the id of an earlier one replaces the fields it gives; `remove = true` deletes it.
#   id          the entry's id (fixed for built-in entries)
#   kind        the exe's object kind 0..19 (fixed)
#   name        the name the build panel shows
#   footprint   edge of the square footprint, in tiles (1..8)
#   price       price in the game's money, a multiple of 100 (an upgrade costs price x (level + 2) / 2)
"
        }
        "staff" => {
            "# data/staff.toml: kinds of employee (OpenSGolf built-in values).
#   id                the entry's id (fixed for built-in entries)
#   kind              0 Club Pro, 1 Ranger, 2 Groundskeeper, 3 Soda Vendor (fixed)
#   name              the kind's name
#   wage              wage per pay interval (the hire panel shows it per week), a multiple of 100
#   wage_experienced  the same for an experienced employee
"
        }
        "properties" => {
            "# data/properties.toml: the sixteen properties of the world map (OpenSGolf built-in values).
#   id          the entry's id (fixed for built-in entries)
#   index       place in the list, 0..15 (fixed): 0..9 on the map's left arc, 10..15 on the right
#   name        the place, as the map shows it
#   course      the course's own name
#   bonus       the property's bonus, as the map shows it
#   map_x/map_y the marker on the world map art
#   theme       parkland | desert | tropical | links
#   coast       inland (a square of land) | coast (the sea along one side) | island (water all round)
#   relief      gentle | rolling | hilly
"
        }
        _ => {
            "# data/economy.toml: the money rules (OpenSGolf built-in values). Money is the game's money, multiples of 100.
#   start_funds            what a normal game starts with, before the property's price
#   sandbox_funds          what a sandbox game starts with
#   offer_prices           the sixteen offer slots of the world map, cheapest first
#   firing_cost            what firing an employee costs
#   hole_maintenance       maintenance per active hole, charged with the wages every 1024 / (difficulty + 2) ticks
#   debt_interest_percent  interest on debt at each month start
#   rank_holes             holes for Daily Fee, Country Club, Championship (fewer than the first: Municipal)
"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merged_defaults_equal_the_builtin_tables() {
        let (d, warn) = Data::from_tables(&Tables::builtin());
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(d.rows, crate::course::TYPES);
        assert_eq!(d.land_types, crate::land::TYPES);
        assert_eq!(d.buildings, crate::land::BUILDINGS);
        assert_eq!(d.records, crate::land::RECORDS);
        assert_eq!(d.properties, crate::properties::PROPERTIES);
        assert_eq!(d.wages, crate::economy::WAGE_UNITS);
        assert_eq!(d.staff_names, STAFF_NAMES);
        assert_eq!(d.economy.offer_units, crate::land::SLOT_PRICE_UNITS);
        assert_eq!(d.economy.start_funds, crate::properties::START_FUNDS);
        assert_eq!(d.economy.sandbox_funds, crate::properties::START_FUNDS * 10);
        assert_eq!(d.economy.firing_units as f64, crate::economy::Economy::FIRING_UNITS);
        assert_eq!(d.economy.maintenance_units, 1);
        for t in -1..40 {
            assert_eq!(d.paint_units.get(t as usize).copied().unwrap_or(0), crate::economy::Economy::builtin_terrain_cost_units(t));
            assert_eq!(d.clear_units.get(t as usize).copied().unwrap_or(0), crate::economy::Economy::builtin_terrain_clear_units(t));
        }
        assert_eq!(d.views, crate::homes::BUILTIN_VIEW);
        for h in 0..30 {
            let r = d.economy.rank_holes.iter().filter(|&&n| h >= n).count() as i32;
            assert_eq!(
                r,
                match h {
                    0..=5 => 0,
                    6..=9 => 1,
                    10..=17 => 2,
                    _ => 3,
                }
            );
        }
        // what `get` serves before anything is installed
        assert_eq!(get().rows, crate::course::TYPES);
    }

    #[test]
    fn dumped_tables_read_back_unchanged() {
        let b = Tables::builtin();
        let mut t = Tables::builtin();
        for (file, text) in b.to_toml() {
            assert!(text.contains(&format!("[{}", table_key(file))), "{file}");
            assert_eq!(t.merge(file, &text), Vec::<String>::new(), "{file}");
        }
        assert_eq!(t, b);
    }

    #[test]
    fn merge_replaces_fields_removes_and_reports() {
        let mut t = Tables::builtin();
        let p = t.merge(
            "terrain",
            r#"
[[terrain]]
id = "fairway"
cost = 100

[[terrain]]
id = "rough"
bounce = 99

[[terrain]]
id = "glass"
tile = 30
name = "glass"
cost = 0
clear_cost = 0

[[terrain]]
id = "marsh"
remove = true

[[terrain]]
id = "tee"
tile = 5
"#,
        );
        assert_eq!(p.len(), 2, "{p:?}"); // rough's bounce is out of range, tee's tile cannot change
        assert!(p[0].contains("rough") && p[0].contains("bounce"), "{p:?}");
        assert!(p[1].contains("tee") && p[1].contains("fixed"), "{p:?}");
        let fairway = t.terrain.iter().find(|e| e.id == "fairway").unwrap();
        assert_eq!((fairway.cost, fairway.bounce), (100, Some(4)));
        assert!(t.terrain.iter().all(|e| e.id != "marsh"));
        let (d, warn) = Data::from_tables(&t);
        assert_eq!(d.paint_units[2], 1);
        assert_eq!(d.rows[4], crate::course::TYPES[4]);
        // marsh falls back to its built-in values; glass is kept but unused
        assert_eq!(d.clear_units[19], 100);
        assert_eq!(warn.len(), 2, "{warn:?}");

        assert!(t.merge("terrain", "this is = = not toml")[0].contains("terrain.toml"));
        assert!(t.merge("staff", "[[staff]]\nid = \"ranger\"\nwage = 250\n")[0].contains("multiple of 100"));
        assert!(t.merge("economy", "[economy]\noffer_prices = [1]\n")[0].contains("16"));
        assert!(t.merge("economy", "[economy]\nstart_funds = 50000\n").is_empty());
        assert_eq!(t.economy.start_funds, 50000);
        assert!(t.merge("properties", "[[property]]\nid = \"oahu\"\ntheme = \"moon\"\n")[0].contains("oahu"));
        assert!(t.merge("properties", "[[property]]\nid = \"oahu\"\ntheme = \"desert\"\ncoast = \"inland\"\n").is_empty());
        let (d, _) = Data::from_tables(&t);
        assert_eq!((d.records[6].theme, d.records[6].coast), (1, 0));
        assert_eq!(d.properties[6].theme, crate::land::port_theme(1));
    }
}
