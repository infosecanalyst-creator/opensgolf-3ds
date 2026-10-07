//! `sgtool fixture DIR`: writes a stand-in game folder with plain placeholder art under the file names the port looks for, so the
//! game can be started and screenshotted without the real data (tests, CI). Nothing in it comes from the original game: textures
//! are flat colours with a little noise, sprites are simple shapes, the story and golfer list are made up.
use std::path::Path;

fn put(dir: &Path, rel: &str, bytes: &[u8]) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&p, bytes).unwrap_or_else(|e| panic!("cannot write {}: {e}", p.display()));
}

fn bmp24(w: usize, h: usize, px: impl Fn(usize, usize) -> [u8; 3]) -> Vec<u8> {
    let stride = (w * 3 + 3) & !3;
    let size = 54 + stride * h;
    let mut d = vec![0u8; 54];
    d[0] = b'B';
    d[1] = b'M';
    d[2..6].copy_from_slice(&(size as u32).to_le_bytes());
    d[10] = 54;
    d[14] = 40;
    d[18..22].copy_from_slice(&(w as u32).to_le_bytes());
    d[22..26].copy_from_slice(&(h as u32).to_le_bytes());
    d[26] = 1;
    d[28] = 24;
    for y in (0..h).rev() {
        let mut row = vec![0u8; stride];
        for x in 0..w {
            let c = px(x, y);
            row[x * 3] = c[2];
            row[x * 3 + 1] = c[1];
            row[x * 3 + 2] = c[0];
        }
        d.extend(row);
    }
    d
}

fn tga32(w: usize, h: usize, px: impl Fn(usize, usize) -> [u8; 4]) -> Vec<u8> {
    let mut d = vec![0u8; 18];
    d[2] = 2;
    d[12..14].copy_from_slice(&(w as u16).to_le_bytes());
    d[14..16].copy_from_slice(&(h as u16).to_le_bytes());
    d[16] = 32;
    d[17] = 0x28; // top-down, 8 alpha bits
    for y in 0..h {
        for x in 0..w {
            let c = px(x, y);
            d.extend_from_slice(&[c[2], c[1], c[0], c[3]]);
        }
    }
    d
}

/// 8-bit RLE PCX with a palette.
fn pcx8(w: usize, h: usize, pal: &[[u8; 3]; 256], px: impl Fn(usize, usize) -> u8) -> Vec<u8> {
    let mut d = vec![0u8; 128];
    d[0] = 10;
    d[1] = 5;
    d[2] = 1;
    d[3] = 8;
    d[8..10].copy_from_slice(&((w - 1) as u16).to_le_bytes());
    d[10..12].copy_from_slice(&((h - 1) as u16).to_le_bytes());
    d[65] = 1;
    let bpl = w + (w & 1);
    d[66..68].copy_from_slice(&(bpl as u16).to_le_bytes());
    for y in 0..h {
        let row: Vec<u8> = (0..bpl).map(|x| if x < w { px(x, y) } else { 0 }).collect();
        let mut x = 0;
        while x < row.len() {
            let v = row[x];
            let mut n = 1;
            while x + n < row.len() && row[x + n] == v && n < 63 {
                n += 1;
            }
            if n > 1 || v >= 0xC0 {
                d.push(0xC0 | n as u8);
            }
            d.push(v);
            x += n;
        }
    }
    d.push(0x0C);
    for c in pal {
        d.extend_from_slice(c);
    }
    d
}

/// FLC with the Firaxis header extension: `views` runs of `fpv` frames (plus the ring frame each run stores), on a 480x480 canvas
/// whose centre is the ground point. Palette index 255 is the transparent key.
fn flc(
    w: usize,
    h: usize,
    views: usize,
    fpv: usize,
    crop: (usize, usize),
    pal: &[[u8; 3]; 256],
    px: impl Fn(usize, usize, usize, usize) -> u8,
) -> Vec<u8> {
    let mut d = vec![0u8; 128];
    d[4..6].copy_from_slice(&0xAF12u16.to_le_bytes());
    d[6..8].copy_from_slice(&((views * fpv) as u16).to_le_bytes());
    d[8..10].copy_from_slice(&(w as u16).to_le_bytes());
    d[10..12].copy_from_slice(&(h as u16).to_le_bytes());
    d[12..14].copy_from_slice(&8u16.to_le_bytes());
    d[16..20].copy_from_slice(&83u32.to_le_bytes());
    d[96..98].copy_from_slice(&(views as u16).to_le_bytes());
    d[98..100].copy_from_slice(&(fpv as u16).to_le_bytes());
    d[100..102].copy_from_slice(&(crop.0 as u16).to_le_bytes());
    d[102..104].copy_from_slice(&(crop.1 as u16).to_le_bytes());
    d[104..106].copy_from_slice(&480u16.to_le_bytes());
    d[106..108].copy_from_slice(&480u16.to_le_bytes());
    d[112..114].copy_from_slice(&(if views >= 8 { 0xFF } else { 0x0F } as u16).to_le_bytes());
    let mut first = true;
    for v in 0..views {
        for f in 0..=fpv {
            let fr = f.min(fpv - 1); // the ring frame repeats the last one
            let mut chunks: Vec<(u16, Vec<u8>)> = Vec::new();
            if first {
                let mut p = vec![1, 0, 0, 0];
                for c in pal {
                    p.extend_from_slice(c);
                }
                chunks.push((4, p));
                first = false;
            }
            let mut brun = Vec::new();
            for y in 0..h {
                brun.push(0);
                let row: Vec<u8> = (0..w).map(|x| px(v, fr, x, y)).collect();
                for part in row.chunks(127) {
                    brun.push((-(part.len() as i32)) as i8 as u8);
                    brun.extend_from_slice(part);
                }
            }
            chunks.push((15, brun));
            let mut body = Vec::new();
            for (t, c) in &chunks {
                body.extend_from_slice(&((c.len() + 6) as u32).to_le_bytes());
                body.extend_from_slice(&t.to_le_bytes());
                body.extend_from_slice(c);
            }
            d.extend_from_slice(&((body.len() + 16) as u32).to_le_bytes());
            d.extend_from_slice(&0xF1FAu16.to_le_bytes());
            d.extend_from_slice(&(chunks.len() as u16).to_le_bytes());
            d.extend_from_slice(&[0; 8]);
            d.extend(body);
        }
    }
    let len = d.len() as u32;
    d[0..4].copy_from_slice(&len.to_le_bytes());
    d
}

fn wav_tone(freq: f32, secs: f32) -> Vec<u8> {
    let rate = 22050u32;
    let n = (rate as f32 * secs) as usize;
    let samples: Vec<u8> = (0..n)
        .flat_map(|i| {
            let t = i as f32 / rate as f32;
            let env = (1.0 - t / secs).max(0.0);
            (((t * freq * std::f32::consts::TAU).sin() * 6000.0 * env) as i16).to_le_bytes()
        })
        .collect();
    let mut d = Vec::new();
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + samples.len() as u32).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    d.extend_from_slice(&16u32.to_le_bytes());
    d.extend_from_slice(&[1, 0, 1, 0]);
    d.extend_from_slice(&rate.to_le_bytes());
    d.extend_from_slice(&(rate * 2).to_le_bytes());
    d.extend_from_slice(&[2, 0, 16, 0]);
    d.extend_from_slice(b"data");
    d.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    d.extend(samples);
    d
}

fn hash(x: usize, y: usize, s: usize) -> u32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ (s as u32).wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

fn shade(c: [u8; 3], k: i32) -> [u8; 3] {
    c.map(|v| (v as i32 + k).clamp(0, 255) as u8)
}

/// Placeholder colour per terrain texture name.
fn terrain_colour(name: &str) -> [u8; 3] {
    match name {
        "Tee" => [120, 190, 90],
        "PuttingGreen" | "TrickyGreen" => [90, 200, 90],
        "Fairway" => [100, 175, 70],
        "FirmFairway" => [130, 170, 70],
        "Rough" => [70, 130, 50],
        "DeepRough" | "Overgrowth" => [50, 100, 40],
        "GrassySand" => [170, 170, 100],
        "Brush" => [80, 100, 50],
        "Rock" | "Cliff" => [130, 125, 120],
        "Woods" => [40, 80, 35],
        "Marsh" => [70, 110, 90],
        "Building" => [150, 140, 120],
        "Ravine" => [100, 80, 60],
        "FlowerBed" => [190, 90, 140],
        n if n.starts_with("Water") => [50, 100, 170],
        n if n.contains("Sand") || n.contains("Bunker") => [225, 210, 150],
        _ => [128, 128, 128],
    }
}

pub fn write_fixture(dir: &Path, font: Option<&Path>) {
    const THEMES: [&str; 4] = ["Parkland", "Links", "Desert", "Tropical"];
    const NAMES: [&str; 29] = [
        "Tee",
        "PuttingGreen",
        "Fairway",
        "FirmFairway",
        "Rough",
        "DeepRough",
        "GrassySand",
        "PotSandBunker",
        "Overgrowth",
        "Brush",
        "Rock",
        "Woods",
        "WaterShallow",
        "Marsh",
        "Building",
        "WaterMiddle",
        "WaterDeep",
        "WaterShallowDesert",
        "TrickyGreen",
        "SandBunker1",
        "SandBunker2",
        "SandBunker3",
        "SandBunker4",
        "Cliff",
        "Ravine",
        "FlowerBed",
        "ZenSand",
        "GrassBunker",
        "Overgrowth",
    ];
    for (ti, theme) in THEMES.iter().enumerate() {
        for name in NAMES {
            let base = shade(terrain_colour(name), ti as i32 * 6 - 9);
            for v in 1..=9 {
                let img = bmp24(64, 64, |x, y| shade(base, (hash(x, y, v) % 21) as i32 - 10));
                put(dir, &format!("Data/Textures/{theme}/{name}A{v:04}.bmp"), &img);
            }
        }
        let path = |cross: bool| {
            tga32(64, 64, move |x, y| {
                let arm = |a: usize| (21..43).contains(&a);
                let on = if cross { arm(x) || arm(y) } else { (x as i32 - 32).pow(2) + (y as i32 - 32).pow(2) < 30 * 30 };
                if on {
                    [200, 190, 160, 255]
                } else {
                    [0, 0, 0, 0]
                }
            })
        };
        put(dir, &format!("Data/Textures/{theme}/Path.tga"), &path(true));
        put(dir, &format!("Data/Textures/{theme}/PathX.tga"), &path(true));
        put(dir, &format!("Data/Textures/{theme}/PathCurve.tga"), &path(false));
        put(dir, &format!("Data/Textures/{theme}/PathCurveX.tga"), &path(false));
        put(
            dir,
            &format!("Data/Textures/{theme}/RetainingWallA.bmp"),
            &bmp24(64, 64, |_, y| if y % 16 < 2 { [90, 80, 70] } else { [150, 140, 125] }),
        );
        put(dir, &format!("{theme}Lighting.txt"), b"#AMBIENT\n200 200 200\n#DIFFUSE\n240 240 240\n#SPECULAR\n255 255 255\n");
    }

    // Sprites: palette 0..31 a few solid colours, 255 the magenta key.
    let mut pal = [[0u8; 3]; 256];
    let colours =
        [[30, 110, 40], [90, 60, 30], [200, 60, 50], [230, 230, 230], [60, 60, 160], [220, 190, 140], [120, 120, 120], [250, 220, 60]];
    for (i, c) in colours.iter().enumerate() {
        pal[i + 1] = *c;
    }
    pal[255] = [255, 0, 255];
    let tree = flc(64, 96, 1, 3, (208, 168), &pal, |_, f, x, y| {
        let (dx, dy) = (x as i32 - 32, y as i32 - 36);
        let r = 18 + f as i32 * 4;
        if dx * dx + dy * dy < r * r {
            1
        } else if (28..36).contains(&x) && y >= 50 {
            2
        } else {
            255
        }
    });
    let building = flc(120, 100, 4, 1, (180, 160), &pal, |v, _, x, y| {
        if y >= 30 && (10..110).contains(&x) {
            if y < 40 {
                3
            } else {
                [6, 7, 5, 4][v]
            }
        } else {
            255
        }
    });
    let base =
        flc(160, 80, 4, 1, (160, 200), &pal, |_, _, x, y| if (x as i32 - 80).pow(2) / 4 + (y as i32 - 40).pow(2) < 900 { 7 } else { 255 });
    let golfer = |colour: u8| {
        flc(24, 48, 8, 6, (228, 196), &pal, move |v, f, x, y| {
            let head = (x as i32 - 12).pow(2) + (y as i32 - 8).pow(2) < 25;
            let body = (8..16).contains(&x) && (13..32).contains(&y);
            let legs = (y >= 32) && ((x as i32 - 10 - ((f + v) % 3) as i32).abs() < 2 || (x as i32 - 14 + ((f + v) % 3) as i32).abs() < 2);
            if head {
                6
            } else if body {
                colour
            } else if legs {
                5
            } else {
                255
            }
        })
    };
    let tree_files: [[&str; 6]; 4] = [
        [
            "Trees/TreeMapleLarge",
            "Trees/TreeMapleMedium",
            "Trees/TreePineLarge",
            "Trees/TreePineMedium",
            "Trees/TreePineFirLg",
            "Trees/TreeMapleSmall",
        ],
        [
            "Trees/Links/LinksPine_Tall",
            "Trees/Links/LinksPine_Med",
            "Trees/Links/LinksTree3_Tall",
            "Trees/Links/LinksTree3_Med",
            "Trees/Links/LinksTree4_Tall",
            "Trees/Links/LinksTree4_Med",
        ],
        [
            "Trees/Desert/JoshuaTree_Lg",
            "Trees/Desert/JoshuaTree_Md",
            "Trees/Desert/CactusA_Lg",
            "Trees/Desert/CactusB_Md",
            "Trees/Desert/TreeCactusLg",
            "Trees/Desert/CactusC_Lg",
        ],
        [
            "Trees/Tropic/TreePalm/TreePalmLg",
            "Trees/Tropic/TreePalm/TreePalmMed",
            "Trees/Tropic/Tree_Cerc/Cerc_Large",
            "Trees/Tropic/Tree_Drac/Drac_Large",
            "Trees/Tropic/Tree_Tall_Palm/TallPalm_Large",
            "Trees/Tropic/Tree_Cerc/Cerc_Med",
        ],
    ];
    for t in tree_files.iter().flatten() {
        put(dir, &format!("Flics/{t}.flc"), &tree);
    }
    for (b, g) in [
        ("Bldgs/Park/clubL2", "Bldgs/Park/clubL2_base"),
        ("Bldgs/links/clubL2", "Bldgs/links/clubL2_dirt"),
        ("Bldgs/Desert/DESclubL2", "Bldgs/Desert/DesClubL1base"),
        ("Bldgs/Tropical/TROPclubL2", "Bldgs/Tropical/TROPclubL2_base"),
    ] {
        put(dir, &format!("Flics/{b}.flc"), &building);
        put(dir, &format!("Flics/{g}.flc"), &base);
    }
    for b in [
        "Bldgs/Park/ParkSnackL1",
        "Bldgs/Tropical/TROPsnackL1",
        "Bldgs/Park/ProsL1",
        "Bldgs/Desert/dproL1",
        "Bldgs/Tropical/TROPproshopL1",
        "Bldgs/Park/cartL1",
        "Bldgs/links/Cart_garageL1",
        "Bldgs/Desert/DEScartL1",
        "Bldgs/Tropical/TROPcartL1",
        "Bldgs/Park/HotelL1",
        "Bldgs/links/HotelL1",
        "Bldgs/Desert/DesHotelL1",
        "Bldgs/Tropical/TROPhotelL1",
        "Bldgs/Park/tenL1",
        "Bldgs/Desert/tenL1",
        "Bldgs/Park/MarL1",
        "Bldgs/Tropical/TROPmarL1",
    ] {
        put(dir, &format!("Flics/{b}.flc"), &building);
    }
    for (l, look) in ["Male/MaleKLS", "Male/MalePLS", "Female/FemalePLS", "Female/FemaleSSS"].iter().enumerate() {
        let g = golfer(4 + (l % 4) as u8);
        for anim in ["_NormalWalk", "_NormalAddress", "_PerfectSwing", "_NormalSwing", "_PuttAddress", "_Putt", "_Happy"] {
            put(dir, &format!("Flics/{look}{anim}.flc"), &g);
        }
    }

    // Interface art: a navy and sand palette; 0 background, 1 panel, 2 highlight, 3 light, 253 the magenta key.
    let mut ipal = [[0u8; 3]; 256];
    ipal[0] = [40, 70, 120];
    ipal[1] = [200, 190, 150];
    ipal[2] = [250, 230, 140];
    ipal[3] = [148, 150, 198];
    ipal[4] = [120, 160, 90];
    ipal[5] = [255, 0, 0];
    ipal[6] = [248, 0, 248];
    ipal[253] = [255, 0, 255];
    let menu_btns =
        [(40, 25, 325, 120), (415, 40, 360, 135), (12, 395, 285, 100), (508, 380, 270, 95), (285, 478, 285, 105), (718, 528, 64, 64)];
    let in_btn = |x: usize, y: usize| menu_btns.iter().any(|&(bx, by, bw, bh)| x >= bx && y >= by && x < bx + bw && y < by + bh);
    put(dir, "Interface/TitleBASE.pcx", &pcx8(800, 600, &ipal, |x, y| if (x / 40 + y / 40) % 2 == 0 { 0 } else { 3 }));
    put(dir, "Interface/TitleUnSel.pcx", &pcx8(800, 600, &ipal, |x, y| if in_btn(x, y) { 1 } else { 253 }));
    put(dir, "Interface/TitleMO.pcx", &pcx8(800, 600, &ipal, |x, y| if in_btn(x, y) { 2 } else { 253 }));
    put(dir, "Interface/WorldBase.pcx", &pcx8(800, 600, &ipal, |x, y| if (x + y) % 97 < 3 { 0 } else { 4 }));
    put(
        dir,
        "Interface/infoscreens/coursereport.pcx",
        &pcx8(800, 512, &ipal, |_, y| {
            if y < 104 {
                1
            } else if y < 200 {
                3
            } else if y < 260 {
                5
            } else {
                1
            }
        }),
    );
    put(
        dir,
        "Interface/3mainLowerLeft.pcx",
        &pcx8(800, 600, &ipal, |x, y| {
            let dock = [(43, 473, 33), (117, 497, 30), (177, 536, 26)];
            if y >= 430 && x < 215 {
                if dock.iter().any(|&(cx, cy, r)| (x as i32 - cx).pow(2) + (y as i32 - cy).pow(2) < r * r) {
                    2
                } else {
                    1
                }
            } else if x < 400 && y < 400 {
                1
            } else {
                6
            }
        }),
    );
    for f in ["ChooseParklandButtons.pcx", "ChooseLinksButtons.pcx", "ChooseDesertButtons.pcx", "ChooseTropicalButtons.pcx"] {
        put(dir, &format!("Interface/{f}"), &pcx8(252, 52, &ipal, |x, _| if x < 200 { 3 } else { 2 }));
    }

    // Text data (made up).
    put(
        dir,
        "Themes/Standard/progolfers.dta",
        b"* fixture golfers: name,body,skin,hat,shirt,pants,skills\r\nTest Golfer A, 1, 1, 0, 2, 3, 8888888888 50\r\nTest Golfer B, 0, 2, 1, 1, 1, 4567456745\r\n",
    );
    put(
        dir,
        "Themes/Standard/fixture_story.txt",
        b"Fixture Story\r\nNice day for a round.\r\n It is, PARTNER.\r\n\r\nHow is your swing?\r\n Getting better.\r\n",
    );
    for (name, f) in [
        ("GolfAmbience122.wav", 220.0),
        ("Applause.wav", 330.0),
        ("ApplauseGood.wav", 440.0),
        ("Effects/cash.wav", 880.0),
        ("Golf_Sfx/Drive With Ball.wav", 660.0),
        ("Golf_Sfx/Putt.wav", 520.0),
        ("Interface/Button1.wav", 1000.0),
    ] {
        put(dir, &format!("Sounds/{name}"), &wav_tone(f, 0.3));
    }
    if let Some(f) = font {
        match std::fs::read(f) {
            Ok(b) => put(dir, "KLEPTO__.TTF", &b),
            Err(e) => eprintln!("font {}: {e}", f.display()),
        }
    }
}
