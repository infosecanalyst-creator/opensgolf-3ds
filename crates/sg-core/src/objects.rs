//! What the exe draws for objects on the map (docs/PUBLISHER_EXE_NOTES.md, "Drawing objects"). The exe registers each building
//! sprite under a number at start-up, loading a different file per theme for the same number; its object drawing routine then
//! picks numbers by object kind and building level (level 2 once the course has more than 10 holes). Landmarks are numbered
//! 0x168 + their type. File names are the exe's, relative to Flics/, without ".flc".

/// Landmark art by landmark type (sprite 0x168 + type).
pub const LANDMARKS: [&str; 19] = [
    "Landmarks/Sundial",
    "Landmarks/Barn",
    "Landmarks/civilwarCannon",
    "Landmarks/Stone Two",
    "Landmarks/wmillA",
    "Landmarks/ParklandRock",
    "Landmarks/civilwarStatue",
    "Landmarks/LighthouseC",
    "Landmarks/Buddha",
    "Landmarks/Windmill",
    "Landmarks/equestrian",
    "Landmarks/Easter",
    "Landmarks/Pagoda",
    "Landmarks/LighthouseB",
    "Landmarks/ChineseHouse",
    "Landmarks/Tarpit",
    "Landmarks/Wtow2",
    "Landmarks/Radio Tower",
    "Landmarks/Red Oil Pump",
];

/// Building sprites 0x1b6..0x1fb by exe theme (0 Parkland, 1 Desert, 2 Tropical, 3 Links); None where a theme loads nothing.
const SPRITES: &[(u16, [Option<&str>; 4])] = &[
    (
        0x1b6,
        [
            Some("bldgs/park/buildDRL1"),
            Some("bldgs/desert/dbuildDRL1"),
            Some("bldgs/tropical/TropDrivingRShopL1"),
            Some("bldgs/links/DR_shop"),
        ],
    ),
    (
        0x1b7,
        [
            Some("bldgs/park/buildDRL2"),
            Some("bldgs/desert/desDRL2house"),
            Some("bldgs/tropical/TropDrivingRShopL2"),
            Some("bldgs/links/DRL2_shop"),
        ],
    ),
    (
        0x1b8,
        [
            Some("bldgs/park/plat_topDRL1"),
            Some("bldgs/desert/dplatDRL1"),
            Some("bldgs/tropical/TropDrivingRL1PlatTop"),
            Some("bldgs/links/DR_platform"),
        ],
    ),
    (
        0x1b9,
        [
            Some("bldgs/park/plat_baseDRL1"),
            Some("bldgs/desert/dplatDRL1base"),
            Some("bldgs/tropical/TropDrivingRL1PlatBase"),
            Some("bldgs/links/DR_platform_dirt"),
        ],
    ),
    (
        0x1ba,
        [
            Some("bldgs/park/100DRL1"),
            Some("bldgs/desert/d100DRL1"),
            Some("bldgs/tropical/TropDrivingR100L1"),
            Some("bldgs/links/DR_100sign"),
        ],
    ),
    (
        0x1bb,
        [
            Some("bldgs/park/200DRL1"),
            Some("bldgs/desert/d200DRL1"),
            Some("bldgs/tropical/TropDrivingR200L1"),
            Some("bldgs/links/DR_200sign"),
        ],
    ),
    (
        0x1bc,
        [
            Some("bldgs/park/300DRL1"),
            Some("bldgs/desert/d300DRL1"),
            Some("bldgs/tropical/TropDrivingR300L1"),
            Some("bldgs/links/DR_300sign"),
        ],
    ),
    (
        0x1bd,
        [
            Some("bldgs/park/flag_DRL1"),
            Some("bldgs/desert/dflagDRL1"),
            Some("bldgs/tropical/TropDrivingRflagL1"),
            Some("bldgs/links/DR_flag"),
        ],
    ),
    (
        0x1be,
        [
            Some("bldgs/park/wall_DRL1"),
            Some("bldgs/desert/dwallDRL1"),
            Some("bldgs/tropical/TropDrivingRfenceL1"),
            Some("bldgs/links/rockwall01"),
        ],
    ),
    (0x1bf, [Some("bldgs/park/buildDRL2_ANIM"), None, Some("bldgs/tropical/TROPDrivingRshopL2_ANIM"), Some("bldgs/links/DRL2_shop_ANIM")]),
    (
        0x1c0,
        [
            Some("bldgs/park/plat_topDRL2"),
            Some("bldgs/desert/desDRL2plattop"),
            Some("bldgs/tropical/TropDrivingRL2PlatTop"),
            Some("bldgs/links/DRL2_platform"),
        ],
    ),
    (
        0x1c1,
        [
            Some("bldgs/park/plat_baseDRL2"),
            Some("bldgs/desert/desDRL2plat"),
            Some("bldgs/tropical/TropDrivingRL2PlatBase"),
            Some("bldgs/links/DRL2_platform_base"),
        ],
    ),
    (
        0x1c2,
        [Some("bldgs/park/CartL1"), Some("bldgs/Desert/DESCartL1"), Some("bldgs/tropical/TropCartL1"), Some("bldgs/links/Cart_garageL1")],
    ),
    (
        0x1c3,
        [Some("bldgs/park/CartL2"), Some("bldgs/Desert/DESCartL2"), Some("bldgs/tropical/TropCartL2"), Some("bldgs/links/Cart_garageL2")],
    ),
    (0x1c4, [Some("bldgs/park/CartL1_ANIM"), None, None, None]),
    (
        0x1c5,
        [
            Some("bldgs/park/CartL1_base"),
            Some("bldgs/Desert/DESCartL1Base"),
            Some("bldgs/tropical/TropCartL1Base"),
            Some("bldgs/links/Cart_garageL1_dirt"),
        ],
    ),
    (
        0x1c6,
        [
            Some("bldgs/park/CartL2_base"),
            Some("bldgs/Desert/DESCartL2Base"),
            Some("bldgs/tropical/TropCartL2_Base"),
            Some("bldgs/links/Cart_garageL2_dirt"),
        ],
    ),
    (0x1c7, [Some("bldgs/park/CartL2_ANIM"), Some("bldgs/Desert/DESCartL2Anim"), None, None]),
    (
        0x1ca,
        [
            Some("Homes/Regular/Parkland/A/ParkHouseA"),
            Some("Homes/Regular/Desert/A/DesHouseA"),
            Some("Homes/Regular/Tropical/B/TropHouseB"),
            Some("Homes/Regular/Links/B/LinksHouseB"),
        ],
    ),
    (0x1c8, [Some("bldgs/park/sign"), Some("bldgs/desert/DESsign"), Some("bldgs/tropical/tropsign"), Some("bldgs/links/sign_links")]),
    (
        0x1c9,
        [
            Some("bldgs/park/Houseconst"),
            Some("bldgs/desert/DESHouseconst"),
            Some("bldgs/tropical/TropHouseconst"),
            Some("bldgs/links/Houseconst_links"),
        ],
    ),
    (0x1ce, [Some("bldgs/park/marL1"), Some("bldgs/Desert/HeliL1"), Some("bldgs/tropical/TropMarL1"), Some("bldgs/links/ChurchL1")]),
    (0x1cf, [Some("bldgs/park/marL2"), Some("bldgs/Desert/HelL2"), Some("bldgs/tropical/TropMarinaL2"), Some("bldgs/links/ChurchL2")]),
    (0x1d0, [Some("bldgs/park/parksnackL2_ANIM"), Some("bldgs/Desert/HelAnimLites"), None, None]),
    (
        0x1d1,
        [
            Some("bldgs/park/marL1_waterANIM"),
            Some("bldgs/Desert/HeliL1_Base"),
            Some("bldgs/tropical/TropMarL1_underwater"),
            Some("bldgs/links/ChurchL1_dirt"),
        ],
    ),
    (
        0x1d2,
        [
            Some("bldgs/park/marL2_waterANIM"),
            Some("bldgs/Desert/HelL2Base"),
            Some("bldgs/tropical/TropMarinaL2_underwater"),
            Some("bldgs/links/ChurchL2_dirt"),
        ],
    ),
    (0x1d3, [Some("bldgs/park/parksnackL1_ANIM"), Some("bldgs/Desert/HelAnimElev"), None, None]),
    (0x1d4, [Some("bldgs/park/tenL1"), Some("bldgs/Desert/spaL1"), Some("bldgs/tropical/TropSwimL1"), Some("bldgs/links/StableL1")]),
    (0x1d5, [Some("bldgs/park/tenL2"), Some("bldgs/Desert/spaL2"), Some("bldgs/tropical/TropSwimL2"), Some("bldgs/links/StableL2")]),
    (0x1d7, [None, Some("bldgs/Desert/spaL1_Base"), Some("bldgs/tropical/TropSwimL1anim"), Some("bldgs/links/StableL1_dirt")]),
    (0x1d8, [None, None, Some("bldgs/tropical/TropSwimL2_anim"), Some("bldgs/links/StableL2_dirt")]),
    (0x1d9, [None, Some("bldgs/Desert/DESCASL2Wheel"), None, Some("bldgs/links/StableL2_anim")]),
    (0x1da, [Some("bldgs/park/airL1"), Some("bldgs/Desert/DESCASL1"), Some("bldgs/tropical/TROPthemeL1"), Some("bldgs/links/CastleL1")]),
    (0x1db, [Some("bldgs/park/airL2"), Some("bldgs/Desert/DESCASL2"), Some("bldgs/tropical/TROPthemeL2"), Some("bldgs/links/CastleL2")]),
    (
        0x1dc,
        [
            Some("bldgs/park/airL1_ANIM"),
            Some("bldgs/Desert/DESCASL1Anim"),
            Some("bldgs/tropical/TROPthemeL1_BoatAnim"),
            Some("bldgs/links/CastleL2_ANIM"),
        ],
    ),
    (
        0x1dd,
        [
            Some("bldgs/park/airL1_base"),
            Some("bldgs/Desert/DESCASL1Base"),
            Some("bldgs/tropical/TROPthemeL2_LoopAnim"),
            Some("bldgs/links/CastleL1_dirt"),
        ],
    ),
    (
        0x1de,
        [
            Some("bldgs/park/airL2_base"),
            Some("bldgs/Desert/DESCASL2Base"),
            Some("bldgs/tropical/TROPthemeL1_CannonAnim"),
            Some("bldgs/links/CastleL2_dirt"),
        ],
    ),
    (0x1df, [Some("bldgs/park/airL2_ANIM"), Some("bldgs/Desert/DESCASL2Status"), Some("bldgs/tropical/TROPthemeL2_SprayAnim"), None]),
    (0x1e0, [Some("bldgs/park/ProsL1"), Some("bldgs/Desert/dproL1"), Some("bldgs/tropical/TropProshopL1"), Some("bldgs/links/Pro_shop")]),
    (
        0x1e1,
        [
            Some("bldgs/park/ProsL2"),
            Some("bldgs/Desert/desproL2"),
            Some("bldgs/tropical/TropProshopL2"),
            Some("bldgs/links/Links_ProshopL2"),
        ],
    ),
    (0x1e2, [Some("bldgs/park/ProsL2_ANIM01"), None, None, Some("bldgs/links/Pro_shop_anim")]),
    (
        0x1e3,
        [
            Some("bldgs/park/ProSL1_base"),
            Some("bldgs/Desert/dproL1Base"),
            Some("bldgs/tropical/TropProshopL1_base"),
            Some("bldgs/links/Pro_shop_dirt"),
        ],
    ),
    (0x1e4, [None, Some("bldgs/Desert/desproL2Anim"), Some("bldgs/tropical/TropProshopL1_anim"), Some("bldgs/links/Links_ProshopL2_dirt")]),
    (0x1e5, [Some("bldgs/park/ProsL2_ANIM02"), None, None, Some("bldgs/links/Links_ProshopL2_anim")]),
    (0x1e6, [Some("bldgs/park/HotelL1"), Some("bldgs/Desert/DESHotelL1"), Some("bldgs/Tropical/TropHotelL1"), Some("bldgs/links/HotelL1")]),
    (0x1e7, [Some("bldgs/park/HotelL2"), Some("bldgs/Desert/DESHoL2"), Some("bldgs/Tropical/TropHotelL2"), Some("bldgs/links/HotelL2")]),
    (0x1e8, [Some("bldgs/park/HotelL1_ANIM"), Some("bldgs/Desert/DESHotelL1Anim"), None, Some("bldgs/links/HotelL1_anim")]),
    (
        0x1e9,
        [
            Some("bldgs/park/HotelL1_base"),
            Some("bldgs/Desert/DESHotelL1base"),
            Some("bldgs/Tropical/TropHotelL1base"),
            Some("bldgs/links/HotelL1_dirt"),
        ],
    ),
    (
        0x1ea,
        [
            Some("bldgs/park/HotelL2_base"),
            Some("bldgs/Desert/DESHoL2Status"),
            Some("bldgs/Tropical/TropHotelL2_base"),
            Some("bldgs/links/HotelL2_dirt"),
        ],
    ),
    (
        0x1eb,
        [
            Some("bldgs/park/HotelL2_ANIM"),
            Some("bldgs/Desert/DESHoL2elev"),
            Some("bldgs/Tropical/TropHotelL2_ANIM"),
            Some("bldgs/links/HotelL2_anim"),
        ],
    ),
    (
        0x1ec,
        [Some("bldgs/park/parksnackL1"), Some("bldgs/Desert/DESSnack"), Some("bldgs/Tropical/TropSnackL1"), Some("bldgs/links/Pub_SQ")],
    ),
    (0x1ed, [Some("bldgs/park/parksnackL1_base"), Some("bldgs/Desert/DESSnackBase"), None, Some("bldgs/links/Pub_SQ_dirt")]),
    (
        0x1ee,
        [Some("bldgs/park/ParkSnackL2"), Some("bldgs/Desert/DESSnackL2"), Some("bldgs/Tropical/TropSnackL2"), Some("bldgs/links/PubL2")],
    ),
    (0x1ef, [Some("bldgs/park/ParkSnackL2_base"), None, Some("bldgs/Tropical/TropSnackL2_ANIM"), Some("bldgs/links/PubL2_dirt")]),
    (0x1f0, [Some("bldgs/park/puttL1"), Some("bldgs/desert/dputtL1"), Some("bldgs/tropical/TropPuttingGL1"), Some("bldgs/links/PG_hutL1")]),
    (
        0x1f1,
        [
            Some("bldgs/park/puttL1"),
            Some("bldgs/desert/dputtL1base"),
            Some("bldgs/tropical/TropPuttingGL1"),
            Some("bldgs/links/PG_hutL1_dirt"),
        ],
    ),
    (
        0x1f2,
        [
            Some("bldgs/park/shortflagL1"),
            Some("bldgs/desert/dshortflagL1"),
            Some("bldgs/tropical/TropPuttingGFlagL1"),
            Some("bldgs/links/PG_pflag"),
        ],
    ),
    (
        0x1f3,
        [Some("bldgs/park/puttL2"), Some("bldgs/desert/desputtL2"), Some("bldgs/tropical/TropPuttingGL2"), Some("bldgs/links/PG_hutL2")],
    ),
    (
        0x1f4,
        [
            Some("bldgs/park/puttL2_ANIM"),
            Some("bldgs/desert/desputtL2"),
            Some("bldgs/tropical/TropPuttingGL2_Anim"),
            Some("bldgs/links/PG_hutL2_dirt"),
        ],
    ),
    (
        0x1f5,
        [
            Some("bldgs/park/puttscenic"),
            Some("bldgs/desert/desputtL2scenic"),
            Some("bldgs/tropical/TropPuttingGL2_Scenic"),
            Some("bldgs/links/PG_scenicL2"),
        ],
    ),
    (0x1f6, [Some("bldgs/park/clubL1"), Some("bldgs/Desert/DESclubL1"), Some("bldgs/tropical/TropclubL1"), Some("bldgs/links/clubL1")]),
    (0x1f7, [Some("bldgs/park/clubL2"), Some("bldgs/Desert/DESclubL2"), Some("bldgs/tropical/TropclubL2"), Some("bldgs/links/clubL2")]),
    (0x1f8, [Some("bldgs/park/clubL2_base"), None, Some("bldgs/tropical/TropclubL2_base"), Some("bldgs/links/clubL2_dirt")]),
    (0x1f9, [Some("bldgs/park/clubL1_base"), Some("bldgs/Desert/DESclubL1base"), None, Some("bldgs/links/clubL1_dirt")]),
    (0x1fa, [Some("bldgs/park/clubL1_ANIM"), None, Some("bldgs/tropical/TropclubL1anim"), Some("bldgs/links/clubL1_ANIM")]),
    (0x1fb, [Some("bldgs/park/clubL2_ANIM"), None, Some("bldgs/tropical/TropclubL2_ANIM"), Some("bldgs/links/clubL2_ANIM")]),
];

pub fn sprite(id: u16, theme: u8) -> Option<&'static str> {
    SPRITES.iter().find(|s| s.0 == id).and_then(|s| s.1[theme.min(3) as usize])
}

/// One drawn layer of an object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub file: &'static str,
    /// Drawn on the ground under everything else (bases, dirt, water).
    pub flat: bool,
    /// Plays through its frames with the game tick.
    pub animated: bool,
}

fn add(out: &mut Vec<Layer>, id: u16, theme: u8, flat: bool, animated: bool) {
    if let Some(file) = sprite(id, theme) {
        out.push(Layer { file, flat, animated });
    }
}

/// The layers the exe draws for a building of `kind` at `level` (0 or 1) in exe theme `theme`, bottom first.
pub fn building_layers(kind: i32, level: u16, theme: u8) -> Vec<Layer> {
    let mut v = Vec::new();
    let l = level.min(1);
    match kind {
        6 => {
            let k = if l == 1 { 3 } else { 0 };
            add(&mut v, 0x1f1 + k, theme, theme == 1 || theme == 3, true);
            add(&mut v, 0x1f0 + k, theme, false, false);
            if l == 1 {
                add(&mut v, 0x1f5, theme, false, true);
            }
        }
        7 => {
            if l == 0 {
                if theme != 2 {
                    add(&mut v, 0x1ed, theme, true, false);
                }
                add(&mut v, 0x1ec, theme, false, true);
                if theme == 0 {
                    add(&mut v, 0x1d3, theme, false, true);
                }
            } else {
                if theme != 2 && theme != 1 {
                    add(&mut v, 0x1ef, theme, true, false);
                }
                add(&mut v, 0x1ee, theme, false, true);
                if theme == 2 {
                    add(&mut v, 0x1ef, theme, false, true);
                }
                if theme == 0 {
                    add(&mut v, 0x1d0, theme, false, true);
                }
            }
        }
        8 => {
            if theme != 2 || l == 0 {
                add(&mut v, 0x1e3 + l, theme, !(theme == 2 && l != 0), true);
            }
            add(&mut v, 0x1e0 + l, theme, false, true);
        }
        9 => {
            if theme != 0 && theme != 1 {
                add(&mut v, 0x1d7 + l, theme, theme != 2, true);
            }
            add(&mut v, 0x1d4 + l, theme, false, true);
        }
        11 => {
            add(&mut v, 0x1c5 + l, theme, true, false);
            add(&mut v, 0x1c2 + l, theme, false, false);
        }
        12 => {
            add(&mut v, 0x1d1 + l, theme, true, true);
            add(&mut v, 0x1ce + l, theme, false, false);
        }
        13 => {
            add(&mut v, 0x1e9 + l, theme, true, false);
            add(&mut v, 0x1e6 + l, theme, false, false);
        }
        14 => {
            add(&mut v, 0x1dd + l, theme, true, false);
            add(&mut v, 0x1da + l, theme, false, false);
        }
        15 => {
            let skip_base = (theme == 2 && l == 0) || (theme == 1 && l == 1);
            if !skip_base {
                add(&mut v, 0x1f9 - l, theme, true, false);
            }
            add(&mut v, 0x1f6 + l, theme, false, false);
            if theme != 1 {
                add(&mut v, 0x1fa + l, theme, false, true);
            }
        }
        _ => {}
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_buildings_use_the_theme_art() {
        // Scotland's free building (kind 14) is the castle in Links; Phoenix's (kind 9) the spa in the Desert.
        assert!(building_layers(14, 0, 3).iter().any(|l| l.file == "bldgs/links/CastleL1"));
        assert!(building_layers(9, 0, 1).iter().any(|l| l.file == "bldgs/Desert/spaL1"));
        assert_eq!(building_layers(15, 0, 0)[0].file, "bldgs/park/clubL1_base");
    }
}
