//! The simulation fingerprint (the port's own): one 64-bit number for every piece of game state that decides how the game
//! plays on, so two runs of the same game can be compared tick by tick (a bug report reproduced exactly, a determinism
//! test) and a saved game can be shown to load back to the identical game (save fidelity).
//!
//! The hash is FNV-1a 64 over the state's bytes in a fixed order: each value is written as serde serializes it (struct
//! fields in declaration order, sequences with their length, integers little-endian at their own width, `usize` as 64
//! bits, floats by `to_bits`, strings as their length and UTF-8 bytes, an enum by its variant index), so the number is the
//! same on every platform and in every build. Maps are hashed entry by entry and the entries' hashes summed, so a map's
//! iteration order never matters. Never Rust's default hasher, whose keys are random per process.
//!
//! Fields serde skips (state rebuilt every tick or not saved) are hashed by hand where they decide play
//! ([`club_unsaved`]); fields that only feed the drawing are left out by name ([`VISUAL`]).

use serde::ser::{self, Serialize};

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fnv64(u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Fnv64(OFFSET)
    }
}

impl Fnv64 {
    pub fn new() -> Fnv64 {
        Fnv64::default()
    }
    pub fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 ^= x as u64;
            self.0 = self.0.wrapping_mul(PRIME);
        }
    }
    pub fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }
    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn bool(&mut self, v: bool) {
        self.u8(v as u8);
    }
    pub fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    pub fn str(&mut self, s: &str) {
        self.u64(s.len() as u64);
        self.bytes(s.as_bytes());
    }
    pub fn finish(&self) -> u64 {
        self.0
    }
}

/// Fields left out of the fingerprint as (struct, field): what only the drawing shows. Each is set by the simulation but
/// read by nothing that decides play.
pub const VISUAL: &[(&str, &str)] = &[
    // the sprite id and frame drawn for each golfer this tick (Club::drawn: the drawing's copy of the golfer's animation)
    ("Club", "drawn"),
    // the habitat labels and the ball splash on the course (sg_core::wildlife)
    ("Wildlife", "labels"),
    ("Wildlife", "splash"),
];

/// The fingerprint of one value (everything serde writes of it, less the `VISUAL` fields).
pub fn hash<T: Serialize + ?Sized>(v: &T) -> u64 {
    let mut h = Hasher { h: Fnv64::new(), skip: VISUAL, field_skip: None };
    // the hasher never fails: every serde type is taken
    let _ = v.serialize(&mut h);
    h.h.finish()
}

/// Each field of a struct with its own fingerprint, in declaration order (empty for a value that is not a struct): which
/// part of a section two runs differ in.
pub fn fields<T: Serialize + ?Sized>(v: &T) -> Vec<(&'static str, u64)> {
    let mut out = Vec::new();
    let _ = v.serialize(Fields { out: &mut out, name: "" });
    out
}

/// A serializer that takes only a struct, and hashes each of its fields on its own.
struct Fields<'a> {
    out: &'a mut Vec<(&'static str, u64)>,
    name: &'static str,
}

impl<'a> ser::SerializeStruct for Fields<'a> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, v: &T) -> Result<(), Never> {
        if !VISUAL.contains(&(self.name, key)) {
            self.out.push((key, hash(v)));
        }
        Ok(())
    }
    fn end(self) -> Result<(), Never> {
        Ok(())
    }
}

macro_rules! not_a_struct {
    ($($f:ident($($t:ty),*)),* $(,)?) => {
        $(fn $f(self, $(_: $t),*) -> Result<(), Never> { Err(Never) })*
    };
}

impl<'a> ser::Serializer for Fields<'a> {
    type Ok = ();
    type Error = Never;
    type SerializeSeq = ser::Impossible<(), Never>;
    type SerializeTuple = ser::Impossible<(), Never>;
    type SerializeTupleStruct = ser::Impossible<(), Never>;
    type SerializeTupleVariant = ser::Impossible<(), Never>;
    type SerializeMap = ser::Impossible<(), Never>;
    type SerializeStruct = Fields<'a>;
    type SerializeStructVariant = ser::Impossible<(), Never>;
    not_a_struct!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_f32(f32),
        serialize_f64(f64),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8]),
        serialize_none(),
        serialize_unit(),
        serialize_unit_struct(&'static str),
        serialize_unit_variant(&'static str, u32, &'static str),
    );
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<(), Never> {
        v.serialize(self)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _: &'static str, v: &T) -> Result<(), Never> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(self, _: &'static str, _: u32, _: &'static str, _: &T) -> Result<(), Never> {
        Err(Never)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Never> {
        Err(Never)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Never> {
        Err(Never)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeTupleStruct, Never> {
        Err(Never)
    }
    fn serialize_tuple_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeTupleVariant, Never> {
        Err(Never)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Never> {
        Err(Never)
    }
    fn serialize_struct(self, name: &'static str, _: usize) -> Result<Fields<'a>, Never> {
        Ok(Fields { name, ..self })
    }
    fn serialize_struct_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeStructVariant, Never> {
        Err(Never)
    }
}

/// The fingerprint of a whole game, section by section: each named part's own hash, and the total over them in order.
#[derive(Clone, Debug, Default)]
pub struct Fingerprint {
    pub parts: Vec<(&'static str, u64)>,
}

impl Fingerprint {
    pub fn new() -> Fingerprint {
        Fingerprint::default()
    }
    /// A section hashed from a serde value.
    pub fn add<T: Serialize + ?Sized>(&mut self, name: &'static str, v: &T) {
        self.parts.push((name, hash(v)));
    }
    /// A section hashed by the caller.
    pub fn add_hash(&mut self, name: &'static str, h: u64) {
        self.parts.push((name, h));
    }
    pub fn total(&self) -> u64 {
        let mut h = Fnv64::new();
        for (name, v) in &self.parts {
            h.str(name);
            h.u64(*v);
        }
        h.finish()
    }
    /// "TOTAL" in hex; with `parts`, each section's "name=hash" after it.
    pub fn text(&self, parts: bool) -> String {
        let mut s = format!("{:016x}", self.total());
        if parts {
            for (name, v) in &self.parts {
                s.push_str(&format!(" {name}={v:016x}"));
            }
        }
        s
    }
}

/// What of the club serde does not write but play reads: the flags the game sets before each tick, the snapshots still
/// to queue, the weather of the tick (Weather that matters), the planner's weather drag and the calendar's hold.
pub fn club_unsaved(c: &crate::golfer::Club) -> u64 {
    let mut h = Fnv64::new();
    h.bool(c.ticker_busy);
    h.bool(c.advisor);
    h.bool(c.view_close);
    h.bool(c.message_up);
    h.bool(c.show_snapshots);
    h.u64(c.award_snaps.len() as u64);
    for &(id, (x, y)) in &c.award_snaps {
        h.u64(id as u64);
        h.i32(x);
        h.i32(y);
    }
    let p = &c.place;
    h.u32(p.seed);
    h.f32(p.climate.rain);
    h.f32(p.climate.snow);
    h.u8(p.theme);
    for f in [p.force_rain, p.force_heat] {
        h.bool(f.is_some());
        h.f32(f.unwrap_or(0.0));
    }
    h.bool(c.today.is_some());
    if let Some(t) = &c.today {
        h.u32(t.day);
        for f in [t.rain, t.snow, t.wet, t.lush, t.lying, t.heat, t.midday] {
            h.f32(f);
        }
        for b in [t.storm_day, t.storm, t.storm_start, t.storm_end] {
            h.bool(b);
        }
    }
    h.bool(c.announced.is_some());
    h.u32(c.announced.unwrap_or(0));
    h.i32(c.planner.drag);
    h.i32(c.planner.bounce_cut);
    h.bool(c.cal.held);
    h.finish()
}

/// A serde serializer that feeds every value to FNV-1a.
struct Hasher {
    h: Fnv64,
    skip: &'static [(&'static str, &'static str)],
    /// The struct being written, while some of its fields are to be left out.
    field_skip: Option<&'static str>,
}

// value tags, so values of different shapes never hash alike (None against Some(0), an empty string against no string)
const T_NONE: u8 = 0xf0;
const T_SOME: u8 = 0xf1;
const T_UNIT: u8 = 0xf2;
const T_SEQ: u8 = 0xf3;
const T_MAP: u8 = 0xf4;
const T_VARIANT: u8 = 0xf5;
const T_SKIPPED: u8 = 0xf6;
const T_END: u8 = 0xf7;

#[derive(Debug)]
struct Never;

impl std::fmt::Display for Never {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("fingerprint")
    }
}

impl std::error::Error for Never {}

impl ser::Error for Never {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Never
    }
}

/// A sequence or struct under way: the count of elements (a sequence's length may not be known before).
struct Compound<'a> {
    s: &'a mut Hasher,
    n: u64,
    /// The struct's name while some of its fields are left out, and the outer struct's to restore.
    outer: Option<&'static str>,
}

/// A map under way: each entry hashed on its own, the hashes summed (order-free).
struct MapState<'a> {
    s: &'a mut Hasher,
    sum: u64,
    n: u64,
    entry: Option<Fnv64>,
}

impl<'a> ser::Serializer for &'a mut Hasher {
    type Ok = ();
    type Error = Never;
    type SerializeSeq = Compound<'a>;
    type SerializeTuple = Compound<'a>;
    type SerializeTupleStruct = Compound<'a>;
    type SerializeTupleVariant = Compound<'a>;
    type SerializeMap = MapState<'a>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Compound<'a>;

    fn serialize_bool(self, v: bool) -> Result<(), Never> {
        self.h.bool(v);
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<(), Never> {
        self.h.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<(), Never> {
        self.h.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<(), Never> {
        self.h.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<(), Never> {
        self.h.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i128(self, v: i128) -> Result<(), Never> {
        self.h.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<(), Never> {
        self.h.u8(v);
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<(), Never> {
        self.h.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<(), Never> {
        self.h.u32(v);
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<(), Never> {
        self.h.u64(v);
        Ok(())
    }
    fn serialize_u128(self, v: u128) -> Result<(), Never> {
        self.h.bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<(), Never> {
        self.h.u32(v.to_bits());
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<(), Never> {
        self.h.u64(v.to_bits());
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<(), Never> {
        self.h.u32(v as u32);
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<(), Never> {
        self.h.str(v);
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<(), Never> {
        self.h.u64(v.len() as u64);
        self.h.bytes(v);
        Ok(())
    }
    fn serialize_none(self) -> Result<(), Never> {
        self.h.u8(T_NONE);
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<(), Never> {
        self.h.u8(T_SOME);
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Never> {
        self.h.u8(T_UNIT);
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Never> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(self, _: &'static str, index: u32, _: &'static str) -> Result<(), Never> {
        self.h.u8(T_VARIANT);
        self.h.u32(index);
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _: &'static str, v: &T) -> Result<(), Never> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(self, _: &'static str, index: u32, _: &'static str, v: &T) -> Result<(), Never> {
        self.h.u8(T_VARIANT);
        self.h.u32(index);
        v.serialize(self)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Compound<'a>, Never> {
        self.h.u8(T_SEQ);
        Ok(Compound { s: self, n: 0, outer: None })
    }
    fn serialize_tuple(self, _: usize) -> Result<Compound<'a>, Never> {
        self.serialize_seq(None)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Compound<'a>, Never> {
        self.serialize_seq(None)
    }
    fn serialize_tuple_variant(self, _: &'static str, index: u32, _: &'static str, _: usize) -> Result<Compound<'a>, Never> {
        self.h.u8(T_VARIANT);
        self.h.u32(index);
        self.serialize_seq(None)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<MapState<'a>, Never> {
        Ok(MapState { s: self, sum: 0, n: 0, entry: None })
    }
    fn serialize_struct(self, name: &'static str, _: usize) -> Result<Compound<'a>, Never> {
        let outer = self.field_skip;
        self.field_skip = self.skip.iter().any(|&(s, _)| s == name).then_some(name);
        self.h.u8(T_SEQ);
        Ok(Compound { s: self, n: 0, outer })
    }
    fn serialize_struct_variant(self, _: &'static str, index: u32, _: &'static str, _: usize) -> Result<Compound<'a>, Never> {
        self.h.u8(T_VARIANT);
        self.h.u32(index);
        let outer = self.field_skip;
        self.field_skip = None;
        self.h.u8(T_SEQ);
        Ok(Compound { s: self, n: 0, outer })
    }
}

impl Compound<'_> {
    fn element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.n += 1;
        // a struct inside is told its own name (serialize_struct); the outer one's comes back after it
        let mine = self.s.field_skip;
        self.s.field_skip = None;
        let r = v.serialize(&mut *self.s);
        self.s.field_skip = mine;
        r
    }
    fn field<T: Serialize + ?Sized>(&mut self, key: &'static str, v: &T) -> Result<(), Never> {
        if let Some(name) = self.s.field_skip {
            if self.s.skip.iter().any(|&(s, f)| s == name && f == key) {
                return Ok(());
            }
        }
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        self.s.h.u8(T_END);
        self.s.h.u64(self.n);
        self.s.field_skip = self.outer;
        Ok(())
    }
}

impl ser::SerializeSeq for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        Compound::end(self)
    }
}

impl ser::SerializeTuple for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        Compound::end(self)
    }
}

impl ser::SerializeTupleStruct for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        Compound::end(self)
    }
}

impl ser::SerializeTupleVariant for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn end(self) -> Result<(), Never> {
        Compound::end(self)
    }
}

impl ser::SerializeStruct for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, v: &T) -> Result<(), Never> {
        self.field(key, v)
    }
    fn skip_field(&mut self, _: &'static str) -> Result<(), Never> {
        // a field left out of the save while it holds its default (skip_serializing_if)
        self.s.h.u8(T_SKIPPED);
        Ok(())
    }
    fn end(self) -> Result<(), Never> {
        Compound::end(self)
    }
}

impl ser::SerializeStructVariant for Compound<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, _: &'static str, v: &T) -> Result<(), Never> {
        self.element(v)
    }
    fn skip_field(&mut self, _: &'static str) -> Result<(), Never> {
        self.s.h.u8(T_SKIPPED);
        Ok(())
    }
    fn end(self) -> Result<(), Never> {
        Compound::end(self)
    }
}

impl ser::SerializeMap for MapState<'_> {
    type Ok = ();
    type Error = Never;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<(), Never> {
        let mut sub = Hasher { h: Fnv64::new(), skip: self.s.skip, field_skip: None };
        k.serialize(&mut sub)?;
        self.entry = Some(sub.h);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Never> {
        let mut sub = Hasher { h: self.entry.take().unwrap_or_default(), skip: self.s.skip, field_skip: None };
        v.serialize(&mut sub)?;
        self.sum = self.sum.wrapping_add(sub.h.finish());
        self.n += 1;
        Ok(())
    }
    fn end(self) -> Result<(), Never> {
        self.s.h.u8(T_MAP);
        self.s.h.u64(self.n);
        self.s.h.u64(self.sum);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_known_values() {
        // the published FNV-1a 64 test vectors
        assert_eq!(Fnv64::new().finish(), 0xcbf2_9ce4_8422_2325);
        let mut h = Fnv64::new();
        h.bytes(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
        let mut h = Fnv64::new();
        h.bytes(b"foobar");
        assert_eq!(h.finish(), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn fixed_values() {
        // the same value hashes alike in every build and on every platform: these numbers must never change
        assert_eq!(hash(&(1u32, -2i32, "golf", Some(3u8), 0.5f64)), hash(&(1u32, -2i32, "golf", Some(3u8), 0.5f64)));
        assert_eq!(hash(&(1u32, "golf")), 0x7174_8da3_6d20_81ca);
        assert_eq!(hash(&0u32), {
            let mut h = Fnv64::new();
            h.u32(0);
            h.finish()
        });
        assert_eq!(hash(&vec![1u8, 2]), {
            let mut h = Fnv64::new();
            h.bytes(&[T_SEQ, 1, 2, T_END]);
            h.u64(2);
            h.finish()
        });
    }

    #[test]
    fn shapes_differ() {
        assert_ne!(hash(&None::<u8>), hash(&Some(0u8)));
        assert_ne!(hash(&vec![0u8]), hash(&vec![0u8, 0]));
        assert_ne!(hash(&("ab", "c")), hash(&("a", "bc")));
        assert_ne!(hash(&0.0f64), hash(&-0.0f64));
    }

    #[test]
    fn maps_hash_in_any_order() {
        let mut a = std::collections::HashMap::new();
        let mut b = std::collections::HashMap::new();
        for k in 0..50u32 {
            a.insert(k, k * 3);
        }
        for k in (0..50u32).rev() {
            b.insert(k, k * 3);
        }
        assert_eq!(hash(&a), hash(&b));
        b.insert(7, 0);
        assert_ne!(hash(&a), hash(&b));
        let sorted: std::collections::BTreeMap<u32, u32> = a.iter().map(|(&k, &v)| (k, v)).collect();
        assert_eq!(hash(&a), hash(&sorted));
    }

    #[test]
    fn club_same_state_same_value() {
        let a = crate::golfer::Club::default();
        let b = a.clone();
        assert_eq!(hash(&a), hash(&b));
        assert_eq!(club_unsaved(&a), club_unsaved(&b));
    }

    #[test]
    fn club_any_field_changes_the_value() {
        let base = crate::golfer::Club::default();
        let h0 = hash(&base);
        let changes: Vec<(&str, fn(&mut crate::golfer::Club))> = vec![
            ("golfer x", |c| c.g[3].x += 1),
            ("golfer mood", |c| c.g[0].mood -= 1),
            ("golfer card", |c| c.g[5].card[18] = 1),
            ("golfer ball speed", |c| c.g[1].speed = 7),
            ("golfer left", |c| c.g[2].left = true),
            ("hole par", |c| c.holes[1].par = 4),
            ("next hole", |c| c.next_hole += 1),
            ("member rounds", |c| c.members.push(Default::default())),
            ("roster name", |c| c.roster[0].name.push('x')),
            ("tick", |c| c.tick = 1),
            ("game flags", |c| c.game ^= 1),
            ("pro skill", |c| c.pro_skill[9] = 1),
            ("top names", |c| c.top_names[9] = "x".into()),
            ("history", |c| c.history.push([0; 4])),
            ("story slot", |c| c.stories.tutorial += 1),
            ("wildlife", |c| c.wildlife.enabled = !c.wildlife.enabled),
            ("extras", |c| c.extras.hardcore = !c.extras.hardcore),
            ("calendar lag", |c| c.cal.lag = 1),
            ("lamps", |c| c.lamps.push((0, 0))),
        ];
        for (what, f) in changes {
            let mut c = base.clone();
            f(&mut c);
            assert_ne!(hash(&c), h0, "{what} did not change the fingerprint");
        }
    }

    #[test]
    fn club_unsaved_fields_change_the_value() {
        let base = crate::golfer::Club::default();
        let h0 = club_unsaved(&base);
        let changes: Vec<(&str, fn(&mut crate::golfer::Club))> = vec![
            ("ticker busy", |c| c.ticker_busy = !c.ticker_busy),
            ("advisor", |c| c.advisor = !c.advisor),
            ("view close", |c| c.view_close = !c.view_close),
            ("message up", |c| c.message_up = !c.message_up),
            ("award snaps", |c| c.award_snaps.push((1, (2, 3)))),
            ("place", |c| c.place.seed += 1),
            ("today", |c| c.today = Some(Default::default())),
            ("announced", |c| c.announced = Some(0)),
            ("planner drag", |c| c.planner.drag = 1),
            ("calendar held", |c| c.cal.held = !c.cal.held),
        ];
        for (what, f) in changes {
            let mut c = base.clone();
            f(&mut c);
            assert_ne!(club_unsaved(&c), h0, "{what} did not change the fingerprint");
        }
    }

    #[test]
    fn visual_fields_are_left_out() {
        let base = crate::golfer::Club::default();
        let mut c = base.clone();
        c.drawn = vec![(1, 2); 4];
        c.wildlife.labels.push((1, 2, "x".into(), 3));
        c.wildlife.splash = Some((1, 2, 3));
        assert_eq!(hash(&c), hash(&base));
    }

    #[test]
    fn economy_and_course_fields_change_the_value() {
        let e = crate::economy::Economy::default();
        let mut e2 = e.clone();
        e2.cash += 0.01;
        assert_ne!(hash(&e), hash(&e2));
        let mut r = crate::land::ExeRng::from_clock(5);
        let h = hash(&r);
        r.below(100);
        assert_ne!(hash(&r), h);
        let mut t = crate::terrain::Terrain::default();
        t.ty = vec![0; 4];
        let h = hash(&t);
        t.ty[3] = 1;
        assert_ne!(hash(&t), h);
    }

    #[test]
    fn fields_split_a_struct() {
        let base = crate::golfer::Club::default();
        let mut c = base.clone();
        c.next_hole += 1;
        let (a, b) = (fields(&base), fields(&c));
        assert_eq!(a.len(), b.len());
        let differ: Vec<&str> = a.iter().zip(&b).filter(|(x, y)| x.1 != y.1).map(|(x, _)| x.0).collect();
        assert_eq!(differ, ["next_hole"]);
        assert!(!a.iter().any(|(k, _)| *k == "drawn"));
        assert!(fields(&3u32).is_empty());
    }

    #[test]
    fn sections_name_their_parts() {
        let mut f = Fingerprint::new();
        f.add("a", &1u32);
        f.add_hash("b", 2);
        assert_eq!(f.parts.len(), 2);
        let t = f.text(true);
        assert!(t.starts_with(&format!("{:016x} a=", f.total())));
        let mut g = Fingerprint::new();
        g.add_hash("b", 2);
        g.add("a", &1u32);
        assert_ne!(f.total(), g.total());
    }
}
