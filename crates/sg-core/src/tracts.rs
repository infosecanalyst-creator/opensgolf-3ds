//! Buying land (the exe's land screen 0x4587a0, reached after the County Commissioner approves an expansion): the map is cut
//! into nine 16 x 16 tracts; a tract with out-of-bounds tiles is for sale, priced by a random roll over those tiles that doubles
//! with every purchase, and buying it gives back the tiles the land generator had marked out of bounds.
//!
//! On a larger map (crate::mapsize) there are still nine tracts on the screen's 3 x 3 diamond, each (n - 2) / 3 tiles square
//! (16 on the original's 50: 24 at 74, 32 at 98, 42 at 128). The price roll runs over every out-of-bounds tile as before, so a
//! tract costs the same per acre on every size, and a bigger tract costs more in proportion to its land.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Buying land"), restated in our own words.

use crate::land::{mn, ExeRng, Land, T_OUT};
use crate::lang::{tr, trf};

/// One tract as the screen shows it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tract {
    /// Out-of-bounds tiles in the tract (each a tenth of an acre).
    pub oob: i32,
    /// Price in units of $100. A purchased tract keeps 50 (the exe's 5 * 10), and clicking it still buys.
    pub price: i32,
    /// The three most common types the tract had before the border was drawn.
    pub top: [u8; 3],
}

/// A tract's edge in tiles: 16 on the original's map, (n - 2) / 3 on any.
pub fn side() -> i32 {
    (mn() - 2) / 3
}

/// First tile of tract i (0..8).
pub fn origin(i: usize) -> (i32, i32) {
    ((i % 3) as i32 * side() + 1, (i / 3) as i32 * side() + 1)
}

/// Rolls the nine tracts (the exe does this on every redraw of the screen, keeping the prices of the first one).
pub fn roll(land: &Land, purchases: i32, rng: &mut ExeRng) -> [Tract; 9] {
    let mut out = [Tract::default(); 9];
    for (i, tr) in out.iter_mut().enumerate() {
        let (a0, b0) = origin(i);
        let mut f: i32 = 5;
        let mut n = 0;
        let mut hist = [0i32; 23];
        for db in 0..side() {
            for da in 0..side() {
                let t = ((a0 + da) * mn() + b0 + db) as usize;
                if land.ty[t] == T_OUT {
                    n += 1;
                    hist[(land.original[t] as usize).min(22)] += 1;
                    if rng.below(3) == 0 {
                        f = f.wrapping_add(1i32.wrapping_shl((purchases & 31) as u32));
                    }
                }
            }
        }
        let price = if n > 0 { f * 20 / 100 } else { f };
        let mut top = [0u8; 3];
        let mut last = 0u8;
        for slot in top.iter_mut() {
            let mut best: Option<usize> = None;
            for (t, &c) in hist.iter().enumerate() {
                if c > 0 && best.is_none_or(|b| c > hist[b]) {
                    best = Some(t);
                }
            }
            if let Some(b) = best {
                last = b as u8;
                hist[b] = 0;
            }
            *slot = last;
        }
        *tr = Tract { oob: n, price: price * 10, top };
    }
    out
}

/// Buys tract i: its out-of-bounds tiles take back their types from before the border (heights, flags and objects stay).
pub fn buy(land: &mut Land, i: usize) {
    let (a0, b0) = origin(i);
    for db in 0..side() {
        for da in 0..side() {
            let t = ((a0 + da) * mn() + b0 + db) as usize;
            if land.ty[t] == T_OUT {
                land.ty[t] = land.original[t];
                land.layer[t] = crate::data::land_type(land.ty[t]).layer;
            }
        }
    }
}

/// A terrain type's name as the theme calls it; trees are named in the plural on the land screen.
pub fn type_name(t: u8, theme: u8, plural: bool) -> String {
    let named = match (theme, t) {
        (1, 4) => Some((tr("STR_THEME_TERRAIN_DESERT"), tr("STR_THEME_TERRAIN_DESERT_PLURAL"))),
        (1, 5) => Some((tr("STR_THEME_TERRAIN_ROUGH"), tr("STR_THEME_TERRAIN_ROUGH_PLURAL"))),
        (1, 10) => Some((tr("STR_THEME_TERRAIN_BRUSH"), tr("STR_THEME_TERRAIN_BRUSH_PLURAL"))),
        (1, 11) => Some((tr("STR_THEME_TERRAIN_RAVINE"), tr("STR_THEME_TERRAIN_RAVINE_PLURAL"))),
        (1, 13) => Some((tr("STR_THEME_TERRAIN_JOSHUA_TREE"), tr("STR_THEME_TERRAIN_JOSHUA_TREE_PLURAL"))),
        (1, 14) => Some((tr("STR_THEME_TERRAIN_CACTUS"), tr("STR_THEME_TERRAIN_CACTUS_PLURAL"))),
        (1, 15) | (2, 15) => Some((tr("STR_THEME_TERRAIN_PALM_TREE"), tr("STR_THEME_TERRAIN_PALM_TREE_PLURAL"))),
        (1, 18) => Some((tr("STR_THEME_TERRAIN_CANYON"), tr("STR_THEME_TERRAIN_CANYON_PLURAL"))),
        (0, 10) | (2, 10) => Some((tr("STR_THEME_TERRAIN_STREAM"), tr("STR_THEME_TERRAIN_STREAM_PLURAL"))),
        (0, 11) | (2, 11) => Some((tr("STR_THEME_TERRAIN_BRUSH"), tr("STR_THEME_TERRAIN_BRUSH_PLURAL"))),
        (0, 18) | (2, 18) => Some((tr("STR_THEME_TERRAIN_WETLANDS"), tr("STR_THEME_TERRAIN_WETLANDS_PLURAL"))),
        (2, 13) => Some((tr("STR_THEME_TERRAIN_TROPICAL_BUSH"), tr("STR_THEME_TERRAIN_TROPICAL_BUSH_PLURAL"))),
        (2, 14) => Some((tr("STR_THEME_TERRAIN_TROPICAL_TREE"), tr("STR_THEME_TERRAIN_TROPICAL_TREE_PLURAL"))),
        (3, 10) => Some((tr("STR_THEME_TERRAIN_BURN"), tr("STR_THEME_TERRAIN_BURN_PLURAL"))),
        (3, 11) => Some((tr("STR_THEME_TERRAIN_GORSE"), tr("STR_THEME_TERRAIN_GORSE_PLURAL"))),
        (3, 13) => Some((tr("STR_THEME_TERRAIN_MAPLE_TREE"), tr("STR_THEME_TERRAIN_MAPLE_TREE_PLURAL"))),
        (3, 14) => Some((tr("STR_THEME_TERRAIN_PINE"), tr("STR_THEME_TERRAIN_PINE_PLURAL"))),
        (3, 15) => Some((tr("STR_THEME_TERRAIN_SCOTS_PINE"), tr("STR_THEME_TERRAIN_SCOTS_PINE_PLURAL"))),
        _ => None,
    };
    if let Some((one, many)) = named {
        return if plural { many } else { one }.to_string();
    }
    let base = crate::lang::text(crate::data::land_type(t).name_key).to_string();
    if plural && crate::data::land_type(t).class == 13 {
        trf("STR_THEME_TERRAIN_TREES", &[("TREE", &base)])
    } else {
        base
    }
}

/// The tract's line on the screen.
pub fn describe(tract: &Tract, i: usize, theme: u8) -> String {
    if tract.oob == 0 {
        return tr("STR_TRACT_ALREADY_PURCHASED").to_string();
    }
    let name = |t: u8| type_name(t, theme, crate::data::land_type(t).class == 13);
    trf(
        "STR_TRACT_BUY",
        &[
            ("NUMBER", &(i + 1)),
            ("ACRES", &(tract.oob / 10)),
            ("TERRAIN1", &name(tract.top[0])),
            ("TERRAIN2", &name(tract.top[1])),
            ("TERRAIN3", &name(tract.top[2])),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_and_buying() {
        let mut land = Land::from_terrain(&crate::terrain::Terrain::default(), 0);
        let (a0, b0) = origin(4);
        for da in 0..side() {
            for db in 0..side() {
                let t = ((a0 + da) * mn() + b0 + db) as usize;
                land.original[t] = 13;
                land.ty[t] = T_OUT;
            }
        }
        let mut rng = ExeRng::from_clock(5);
        let tr = roll(&land, 0, &mut rng);
        assert_eq!(tr[0].oob, 0);
        assert_eq!(tr[0].price, 50);
        assert_eq!(tr[4].oob, 256);
        // 256 tiles, about a third add 1: f near 90, price near 18 * 10
        assert!((120..=240).contains(&tr[4].price), "{}", tr[4].price);
        assert_eq!(tr[4].top[0], 13);
        assert!(describe(&tr[4], 4, 0).starts_with("Buy tract #5, 25 acres of trees"));
        buy(&mut land, 4);
        assert_eq!(land.ty[(a0 * mn() + b0) as usize], 13);
        assert_eq!(roll(&land, 1, &mut rng)[4].oob, 0);
    }
}
