//! Golfer colours (docs/DECODE_BODIES.md). Golfer sprites carry placeholder colours; the exe recomposes a palette for every
//! golfer just before drawing it, copying ranges of entries out of the swap palettes in Bodies/ (MaleSwap01..10,
//! FemaleSwap01..10): the shirt, the trousers, the skin and the hat each pick a whole swap file by a value 0..9, and the
//! hair (women) or the hands (men) one more. The code here is our own; the ranges and value rules are from the decode notes.

use crate::golfer::Club;

/// What a golfer wears: colour values 0..9 (each names swap file value + 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Outfit {
    pub female: bool,
    pub shirt: u8,
    pub pants: u8,
    pub skin: u8,
    pub hat: u8,
    /// Women's hair (entries 80..89).
    pub hair: u8,
    /// The hands and arms (men 80..89, women 90..96); 4 is the pale glove colour.
    pub alt_skin: u8,
}

/// The swap palettes of both genders, file 01 first.
#[derive(Clone, Debug, Default)]
pub struct Swaps {
    pub male: Vec<[u8; 768]>,
    pub female: Vec<[u8; 768]>,
}

impl Swaps {
    /// Reads Bodies/MaleSwapNN.pcx and Bodies/FemaleSwapNN.pcx (the palettes at the end of each file).
    pub fn load(bodies_dir: &std::path::Path) -> Swaps {
        let read = |prefix: &str| -> Vec<[u8; 768]> {
            (1..=10)
                .map_while(|n| {
                    crate::fsutil::read_file(bodies_dir.join(format!("{prefix}{n:02}.pcx")))
                        .and_then(|d| crate::assets::read_pcx_palette(&d))
                })
                .collect()
        };
        Swaps { male: read("MaleSwap"), female: read("FemaleSwap") }
    }

    /// The golfer palette for an outfit (the composer 0x462020): None when the swap files are missing.
    pub fn compose(&self, o: &Outfit) -> Option<[u8; 768]> {
        let set = if o.female { &self.female } else { &self.male };
        let base = set.first()?;
        let file = |v: u8| set.get(v.min(9) as usize).unwrap_or(base);
        let mut pal = *base;
        let mut copy = |from: &[u8; 768], first: usize, n: usize| {
            pal[first * 3..(first + n) * 3].copy_from_slice(&from[first * 3..(first + n) * 3]);
        };
        copy(file(o.shirt), 0, 20);
        copy(file(o.pants), 20, 20);
        copy(file(o.skin), 40, 20);
        copy(file(o.hat), 60, 20);
        if o.female {
            copy(file(o.hair), 80, 10);
            copy(file(o.alt_skin), 90, 7);
        } else {
            copy(file(o.alt_skin), 80, 10);
        }
        Some(pal)
    }
}

impl Club {
    /// The outfit of golfer slot g: a famous pro wears the colours of his row in progolfers.dta (hands forced pale); anyone
    /// else the colours kept in the person record, or, for a record that has none yet, the exe's defaults from the identity.
    pub fn outfit(&self, g: usize) -> Outfit {
        let gg = &self.g[g];
        let p = self.roster.get(gg.roster.max(0) as usize).cloned().unwrap_or_default();
        let female = p.female();
        if gg.kind == 0x20 && gg.famous > 0 {
            if let Some(pro) = self.pros.get(gg.famous as usize) {
                return Outfit {
                    female,
                    shirt: pro.shirt,
                    pants: pro.pants,
                    skin: pro.skin.min(3),
                    hat: pro.hat,
                    hair: pro.hat,
                    alt_skin: 4,
                };
            }
        }
        let mut o = p.outfit(gg.roster.max(0) as u32);
        if gg.kind & 0xe0 == 0x20 {
            o.alt_skin = 4; // pros and the player's pro: pale hands
        }
        o
    }
}

impl crate::roster::Person {
    /// The colours of a person record with identity `id` (its roster index): the record's own bytes once it has been fixed
    /// (+0x2c non-zero), otherwise the defaults the composer writes the first time. The per-head default skin and hair table
    /// of the exe is not decoded, so those two defaults come from the identity here.
    pub fn outfit(&self, id: u32) -> Outfit {
        let female = self.female();
        if self.fixed != 0 {
            return Outfit {
                female,
                shirt: self.shirt % 10,
                pants: self.pants % 10,
                skin: self.skin % 4,
                hat: self.b23 & 0xf,
                hair: self.hair % 6,
                alt_skin: self.alt_skin % 10,
            };
        }
        let skin = ((id * 7 + 3) % 4) as u8;
        Outfit {
            female,
            shirt: if female { ((id * 2) % 3 + 1) as u8 } else { ((id * 3) % 10) as u8 },
            pants: (id % 10) as u8,
            skin,
            hat: if female { 9 } else { 0 },
            hair: ((id * 5) % 6) as u8,
            alt_skin: skin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_come_from_the_chosen_files() {
        let mk = |v: u8| [v; 768];
        let s = Swaps { male: (0..10).map(mk).collect(), female: (0..10).map(|v| mk(v + 100)).collect() };
        let o = Outfit { female: false, shirt: 3, pants: 4, skin: 1, hat: 7, hair: 0, alt_skin: 2 };
        let p = s.compose(&o).unwrap();
        assert_eq!(p[0], 3);
        assert_eq!(p[20 * 3], 4);
        assert_eq!(p[40 * 3], 1);
        assert_eq!(p[60 * 3], 7);
        assert_eq!(p[80 * 3], 2);
        assert_eq!(p[100 * 3], 0); // constant entries from file 01
        let f = s.compose(&Outfit { female: true, hair: 5, alt_skin: 3, ..o }).unwrap();
        assert_eq!(f[80 * 3], 105);
        assert_eq!(f[90 * 3], 103);
    }
}
