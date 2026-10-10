//! Terrain model. Facts here come from analysis of the original Terrain.dll (see docs/TERRAIN.md). Where the original behaviour is
//! not yet reproduced it is marked APPROXIMATION.
use crate::rng::Rng;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Ids 0..30 are the original's (NUM_TEXTURED_TILES); 31..35 are OUR editor-only ids, see below.
pub const TYPE_COUNT: usize = 36;
/// World units per tile edge (original: 100).
pub const TILE_SIZE: f32 = 100.0;
/// World units per elevation level: Terrain.dll sets a corner to (level x 15.0); the exe never changes that scale
/// (setSplineHeight is not imported).
pub const HEIGHT_STEP: f32 = 15.0;
/// Original ortho near/far: -5000 .. +5000.
pub const DEPTH_RANGE: f32 = 5000.0;

// Tile type ids, in the order the original Terrain constructor registers their names.
// Ids 6, 7, 14, 15, 16, 20, 21 are unnamed in the original (reserved/unused).
pub const TT_TEE: u8 = 0;
pub const TT_PUTTING_GREEN: u8 = 1;
pub const TT_FAIRWAY: u8 = 2;
pub const TT_FIRM_FAIRWAY: u8 = 3;
pub const TT_ROUGH: u8 = 4;
pub const TT_DEEP_ROUGH: u8 = 5;
/// Blended sand bunker (unnamed id 7 in the original, dished like the original).
pub const TT_SAND: u8 = 7;
pub const TT_GRASSY_SAND: u8 = 8;
pub const TT_POT_SAND_BUNKER: u8 = 9;
pub const TT_OVERGROWTH: u8 = 10;
pub const TT_BRUSH: u8 = 11;
pub const TT_ROCK: u8 = 12;
pub const TT_WOODS: u8 = 13;
pub const TT_WATER_SHALLOW: u8 = 17;
pub const TT_MARSH: u8 = 18;
pub const TT_OVERGROWTH2: u8 = 19;
pub const TT_BUILDING: u8 = 22;
pub const TT_WATER_MIDDLE: u8 = 23;
pub const TT_WATER_DEEP: u8 = 24;
pub const TT_WATER_SHALLOW_DESERT: u8 = 25;
pub const TT_TRICKY_GREEN: u8 = 26;
pub const TT_SAND_BUNKER1: u8 = 27;
// NOT original ids. These texture sets ship with the game (Cliff*, Ravine*, FlowerBed*, ZenSand*, GrassBunker*) but the ids and
// rules that use them live in golf.exe, so the editor gives them ids of its own.
pub const TT_CLIFF: u8 = 31;
pub const TT_RAVINE: u8 = 32;
pub const TT_FLOWER_BED: u8 = 33;
pub const TT_ZEN_SAND: u8 = 34;
pub const TT_GRASS_BUNKER: u8 = 35;

const NAMES: [Option<&str>; TYPE_COUNT] = [
    Some("Tee"),
    Some("PuttingGreen"),
    Some("Fairway"),
    Some("FirmFairway"),
    Some("Rough"),
    Some("DeepRough"),
    None,
    None,
    Some("GrassySand"),
    Some("PotSandBunker"),
    Some("Overgrowth"),
    Some("Brush"),
    Some("Rock"),
    Some("Woods"),
    None,
    None,
    None,
    Some("WaterShallow"),
    Some("Marsh"),
    Some("Overgrowth"),
    None,
    None,
    Some("Building"),
    Some("WaterMiddle"),
    Some("WaterDeep"),
    Some("WaterShallowDesert"),
    Some("TrickyGreen"),
    Some("SandBunker1"),
    Some("SandBunker2"),
    Some("SandBunker3"),
    Some("SandBunker4"),
    Some("Cliff"),
    Some("Ravine"),
    Some("FlowerBed"),
    Some("ZenSand"),
    Some("GrassBunker"),
];

/// None for unnamed ids.
pub fn tile_type_name(ty: i32) -> Option<&'static str> {
    if (0..TYPE_COUNT as i32).contains(&ty) {
        NAMES[ty as usize]
    } else {
        None
    }
}

pub fn is_water(ty: i32) -> bool {
    ty == TT_WATER_SHALLOW as i32 || ty == TT_WATER_MIDDLE as i32 || ty == TT_WATER_DEEP as i32 || ty == TT_WATER_SHALLOW_DESERT as i32
}

pub fn is_sand(ty: i32) -> bool {
    ty == TT_SAND as i32 || ty == TT_POT_SAND_BUNKER as i32 || ty == TT_GRASSY_SAND as i32 || ty >= TT_SAND_BUNKER1 as i32
}

/// Texture files for one theme folder (Parkland, Links, Desert, Tropical):
/// `<Name><Set A..E><Variation 0001..0009>.bmp` (files[type][set][variation 0..8], None if missing), 64x64, looked up
/// case-insensitively.
#[derive(Clone, Debug, Default)]
pub struct TextureCatalog {
    pub dir: PathBuf,
    pub files: Vec<Vec<Vec<Option<PathBuf>>>>,
}

impl TextureCatalog {
    pub fn load(theme_dir: &Path) -> Result<TextureCatalog, String> {
        let mut index: HashMap<String, PathBuf> = HashMap::new();
        for p in crate::fsutil::list_dir(theme_dir) {
            if !crate::fsutil::is_dir(&p) {
                if let Some(n) = p.file_name() {
                    index.insert(n.to_string_lossy().to_lowercase(), p.clone());
                }
            }
        }
        if index.is_empty() {
            return Err(format!("cannot read texture folder {}", theme_dir.display()));
        }
        let mut cat = TextureCatalog { dir: theme_dir.to_path_buf(), files: vec![Vec::new(); TYPE_COUNT] };
        let mut found = 0;
        for t in 0..TYPE_COUNT {
            let Some(name) = NAMES[t] else { continue };
            for letter in 'A'..='E' {
                let mut vars = vec![None; 9];
                let mut have = 0;
                for v in 1..=9 {
                    let key = format!("{name}{letter}{v:04}.bmp").to_lowercase();
                    if let Some(p) = index.get(&key) {
                        vars[v - 1] = Some(p.clone());
                        have += 1;
                    }
                }
                if have > 0 {
                    found += have;
                    cat.files[t].push(vars);
                }
            }
        }
        if found == 0 {
            return Err(format!("no terrain textures found in {}", theme_dir.display()));
        }
        Ok(cat)
    }

    /// Best texture for a tile: wraps set/variation into what exists, falls back to similar types.
    pub fn pick(&self, ty: i32, set: i32, variation: i32) -> Option<&PathBuf> {
        // Missing types fall back to something visually close.
        const FALLBACKS: [(u8, u8); 14] = [
            (TT_OVERGROWTH2, TT_OVERGROWTH),
            (TT_WATER_SHALLOW_DESERT, TT_WATER_SHALLOW),
            (TT_TEE, TT_FAIRWAY),
            (TT_TRICKY_GREEN, TT_PUTTING_GREEN),
            (TT_FIRM_FAIRWAY, TT_FAIRWAY),
            (TT_GRASSY_SAND, TT_ROUGH),
            (TT_MARSH, TT_ROUGH),
            (TT_BUILDING, TT_ROCK),
            (TT_OVERGROWTH, TT_BRUSH),
            (TT_BRUSH, TT_ROUGH),
            (TT_WOODS, TT_BRUSH),
            (TT_POT_SAND_BUNKER, TT_GRASSY_SAND),
            (TT_SAND_BUNKER1, TT_POT_SAND_BUNKER),
            (TT_ROUGH, TT_FAIRWAY),
        ];
        let mut ty = ty;
        for _ in 0..6 {
            if ty >= 0 && (ty as usize) < self.files.len() && !self.files[ty as usize].is_empty() {
                let sets = &self.files[ty as usize];
                let vars = &sets[set as usize % sets.len()];
                if let Some(p) = &vars[variation as usize % vars.len()] {
                    return Some(p);
                }
                if let Some(p) = vars.iter().flatten().next() {
                    return Some(p); // variation missing: use the first that exists
                }
            }
            match FALLBACKS.iter().rfind(|f| f.0 as i32 == ty) {
                Some(f) => ty = f.1 as i32,
                None => break,
            }
        }
        None
    }
}

/// `<Theme>Lighting.txt` (Terrain.dll 0x10006dd0, the file picked by theme at 0x10003980): the line after "#AMBIENT", then
/// the one after "#DIFFUSE", then the one after "#SPECULAR", each three numbers 0..255 divided by 255, set as light 0's
/// ambient, diffuse and specular colours. The "#HIGHLIGHT" block after them is never read, by the renderer or the exe.
/// Light 0 is directional (`light_direction`), the material keeps OpenGL's defaults (ambient 0.2, diffuse 0.8) except a
/// white specular with shininess 13, colour material is off and textures modulate the lit colour (the renderer's
/// glTexEnvi call passes the texture target where GL_TEXTURE_ENV belongs, so it is refused and modulation stays): see
/// `Lighting::shade`.
#[derive(Clone, Copy, Debug)]
pub struct Lighting {
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting { ambient: [0.78; 3], diffuse: [0.94; 3], specular: [1.0; 3] }
    }
}

impl Lighting {
    /// OpenGL's fixed-function colour at a vertex with eye-space unit normal `n`, before the texture: the global ambient
    /// (0.2) and the light's ambient times the material ambient (0.2), the light's diffuse times the material diffuse (0.8)
    /// times n.l, and where n.l > 0 the light's specular times (n.h)^13 with h halfway between l and the viewer at infinity
    /// (0, 0, 1); clamped to 1.
    pub fn shade(&self, n: [f32; 3]) -> [f32; 3] {
        let l = light_direction();
        let d = n[0] * l[0] + n[1] * l[1] + n[2] * l[2];
        let hv = [l[0], l[1], l[2] + 1.0];
        let hl = (hv[0] * hv[0] + hv[1] * hv[1] + hv[2] * hv[2]).sqrt();
        let nh = ((n[0] * hv[0] + n[1] * hv[1] + n[2] * hv[2]) / hl).max(0.0);
        let mut out = [0.0; 3];
        for k in 0..3 {
            let mut c = 0.2 * 0.2 + 0.2 * self.ambient[k];
            if d > 0.0 {
                c += 0.8 * self.diffuse[k] * d + self.specular[k] * nh.powi(13);
            }
            out[k] = c.min(1.0);
        }
        out
    }
}

/// Light 0's direction in eye space (Terrain.dll 0x10007155..0x10007201, and the same at 0x100035fd): (-0.5, 0.1, -1)
/// turned 40 degrees about x, then 45 degrees about y, then normalised; it is given while the modelview matrix is the
/// identity (the renderer pushes and pops its view around each frame), so it stays fixed to the screen whatever the view.
pub fn light_direction() -> [f32; 3] {
    let (s, c) = 40f32.to_radians().sin_cos();
    let (x, y, z) = (-0.5f32, 0.1f32, -1.0f32);
    let (y, z) = (c * y - s * z, s * y + c * z);
    let (s, c) = 45f32.to_radians().sin_cos();
    let (x, z) = (c * x + s * z, -s * x + c * z);
    let l = (x * x + y * y + z * z).sqrt();
    [x / l, y / l, z / l]
}

pub fn parse_lighting(text: &str) -> Lighting {
    let mut out = Lighting::default();
    let grab = |tag: &str, dst: &mut [f32; 3]| {
        if let Some(p) = text.find(tag) {
            let v: Vec<i32> = text[p + tag.len()..].split_whitespace().take(3).map_while(|t| t.parse().ok()).collect();
            if v.len() == 3 {
                *dst = [v[0] as f32 / 255.0, v[1] as f32 / 255.0, v[2] as f32 / 255.0];
            }
        }
    };
    grab("#AMBIENT", &mut out.ambient);
    grab("#DIFFUSE", &mut out.diffuse);
    grab("#SPECULAR", &mut out.specular);
    out
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Terrain {
    pub w: i32,
    pub h: i32,
    /// Per tile, w*h.
    pub ty: Vec<u8>,
    pub variation: Vec<u8>,
    pub set: Vec<u8>,
    /// (w+1)*(h+1) corner elevation levels.
    pub corner: Vec<i8>,
    /// Per tile: 0 none, 1 gravel path, 2 paved path (our own overlay model).
    pub path_kind: Vec<u8>,
    /// Retaining walls: one flag per tile edge (bit 0 north = -z, 1 east, 2 south, 3 west). Terrain.dll's Tile keeps a flag byte
    /// per direction (Tile::getWall/setWall); what the walls look like and cost is decided elsewhere, so the height and look here
    /// are placeholders. Edits keep the two tiles that share an edge in step.
    pub wall_mask: Vec<u8>,
    /// Desert theme: shallow water uses the WaterShallowDesert textures.
    pub desert: bool,
    /// Demo walking route, x,z pairs in world units (tee to green).
    pub path: Vec<f32>,
    /// Tile of the demo clubhouse footprint centre, -1 if none.
    pub clubhouse_x: i32,
    pub clubhouse_y: i32,
    /// Clubhouse footprint edge in tiles when it is even: the sprite then stands on the corner between the middle tiles, half a
    /// tile on from (clubhouse_x, clubhouse_y). 0 for the demo course's odd footprint.
    pub clubhouse_size: i32,
    /// The view's quarter turn (0..3), which turns the sand trap pictures (see `texture_type_for`).
    #[serde(skip)]
    pub sand_phase: i32,
}

pub const MAX_LEVEL: i32 = 24;
/// The exe's corner heights: sea level 3 (our 0), the Elevation tools' range 3..=13, and the cap on a lowered corner of a
/// property that is not hilly (the byte at 0x571ff6 + 46 * site, the relief, is not 2).
pub const EXE_SEA: i32 = 3;
pub const EXE_LOW: i32 = 3;
pub const EXE_HIGH: i32 = 13;
pub const EXE_LOW_CAP: i32 = 10;
/// The Area tool's kernel (0x4c2f68, 5 x 5 shorts): row = x offset + 2, column = y offset + 2. Not symmetric.
pub const AREA_KERNEL: [[i32; 5]; 5] = [[6, 4, 3, 4, 6], [4, 2, 1, 2, 4], [3, 1, 0, 1, 3], [4, 2, 1, 2, 3], [6, 4, 3, 4, 6]];
const DX4: [i32; 4] = [0, 1, 0, -1];
const DY4: [i32; 4] = [-1, 0, 1, 0];

impl Terrain {
    pub fn corner_at(&self, x: i32, y: i32) -> i32 {
        self.corner[(y * (self.w + 1) + x) as usize] as i32
    }
    pub fn tile_index(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }
    pub fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }
    pub fn type_at(&self, x: i32, y: i32) -> i32 {
        if self.inside(x, y) {
            self.ty[self.tile_index(x, y)] as i32
        } else {
            -1
        }
    }
    pub fn path_at(&self, x: i32, y: i32) -> i32 {
        if !self.inside(x, y) || self.path_kind.is_empty() {
            0
        } else {
            self.path_kind[self.tile_index(x, y)] as i32
        }
    }
    pub fn wall_at(&self, x: i32, y: i32, dir: i32) -> bool {
        self.inside(x, y) && !self.wall_mask.is_empty() && (self.wall_mask[self.tile_index(x, y)] >> dir) & 1 != 0
    }

    /// World position of a tile centre.
    pub fn tile_centre(&self, tx: i32, ty: i32) -> (f32, f32) {
        (
            tx as f32 * TILE_SIZE - self.w as f32 * TILE_SIZE * 0.5 + TILE_SIZE * 0.5,
            ty as f32 * TILE_SIZE - self.h as f32 * TILE_SIZE * 0.5 + TILE_SIZE * 0.5,
        )
    }
    /// Tile under a world position (may be outside the map).
    pub fn tile_of(&self, wx: f32, wz: f32) -> (i32, i32) {
        (
            ((wx + self.w as f32 * TILE_SIZE * 0.5) / TILE_SIZE).floor() as i32,
            ((wz + self.h as f32 * TILE_SIZE * 0.5) / TILE_SIZE).floor() as i32,
        )
    }
    /// Nearest tile corner to a world position.
    pub fn corner_of(&self, wx: f32, wz: f32) -> (i32, i32) {
        (
            ((wx + self.w as f32 * TILE_SIZE * 0.5) / TILE_SIZE).round() as i32,
            ((wz + self.h as f32 * TILE_SIZE * 0.5) / TILE_SIZE).round() as i32,
        )
    }

    /// Ground height at a world position (bilinear between corner elevations; ignores sand dishing).
    pub fn height_at(&self, wx: f32, wz: f32) -> f32 {
        let mut fx = (wx + self.w as f32 * TILE_SIZE * 0.5) / TILE_SIZE;
        let mut fz = (wz + self.h as f32 * TILE_SIZE * 0.5) / TILE_SIZE;
        fx = fx.clamp(0.0, self.w as f32 - 0.001);
        fz = fz.clamp(0.0, self.h as f32 - 0.001);
        let (x, y) = (fx as i32, fz as i32);
        let (u, v) = (fx - x as f32, fz - y as f32);
        let a = self.corner_at(x, y) as f32 * (1.0 - u) + self.corner_at(x + 1, y) as f32 * u;
        let b = self.corner_at(x, y + 1) as f32 * (1.0 - u) + self.corner_at(x + 1, y + 1) as f32 * u;
        (a * (1.0 - v) + b * v) * HEIGHT_STEP
    }

    /// Tile type under a world position, -1 off the map.
    pub fn type_at_world(&self, wx: f32, wz: f32) -> i32 {
        let (x, y) = self.tile_of(wx, wz);
        self.type_at(x, y)
    }

    /// Editing. Water depth is the tile's variation byte (0 shallow, 1 middle, 2 deep) as in the original.
    pub fn paint(&mut self, x: i32, y: i32, ty: i32, vbyte: i32) {
        if !self.inside(x, y) {
            return;
        }
        let i = self.tile_index(x, y);
        self.ty[i] = ty as u8;
        self.variation[i] = vbyte as u8;
        // stable random look
        let h = (x as u32).wrapping_mul(73856093) ^ (y as u32).wrapping_mul(19349663) ^ (ty as u32).wrapping_mul(83492791);
        self.set[i] = ((h >> 7) % 5) as u8;
    }

    pub fn set_wall(&mut self, x: i32, y: i32, dir: i32, on: bool) {
        if !self.inside(x, y) {
            return;
        }
        if self.wall_mask.len() != self.ty.len() {
            self.wall_mask = vec![0; self.ty.len()];
        }
        let mut set = |tx: i32, ty: i32, d: i32| {
            if !self.inside(tx, ty) {
                return;
            }
            let i = self.tile_index(tx, ty);
            let m = &mut self.wall_mask[i];
            *m = if on { *m | (1 << d) } else { *m & !(1 << d) };
        };
        set(x, y, dir);
        set(x + DX4[dir as usize], y + DY4[dir as usize], (dir + 2) & 3);
    }

    /// Raises (+) or lowers (-) one corner by `delta` levels, within 0..=10 (the exe's heights 3..13). Nothing spreads to
    /// the neighbours: the exe's elevation edits change only the corners they name (see `edit_elevation`), and a step
    /// between two corners is drawn as a wall. Scripted runs and the keyboard's round brush use this.
    pub fn raise_corner(&mut self, cx: i32, cy: i32, delta: i32) {
        if cx < 0 || cy < 0 || cx > self.w || cy > self.h {
            return;
        }
        let i = (cy * (self.w + 1) + cx) as usize;
        self.corner[i] = (self.corner[i] as i32 + delta).clamp(EXE_LOW - EXE_SEA, EXE_HIGH - EXE_SEA) as i8;
    }

    /// The Elevation panel's edits (raise 0x41db46, lower 0x41d997) on corner (cx, cy), the exe's vertex (cx, cy - 1); the
    /// exe's heights are ours plus 3. `tool` 0 changes the corner by one, raising up to 13 and lowering down to 3, but on a
    /// property that is not hilly a lowered corner is also capped at 10. `tool` 1 works on the 2 x 2 block (cx..cx+1,
    /// cy-1..cy) instead: raising adds one to its lowest corners while that is below 13, lowering takes one off its highest
    /// while that is above 3. `tool` 2 is tool 0 followed by a pass over the 5 x 5 corners round it (0x406f20, 0x406f90):
    /// with c the centre's new height and k from `AREA_KERNEL`, raising adds one to every corner below c - k, lowering takes
    /// one off every corner above c + k (no limits there). Nothing else moves.
    pub fn edit_elevation(&mut self, tool: i32, cx: i32, cy: i32, raise: bool, hilly: bool) {
        let w = self.w;
        let h = self.h;
        let ok = |x: i32, y: i32| x >= 0 && y >= 0 && x <= w && y <= h;
        let at = |t: &Terrain, x: i32, y: i32| t.corner[(y * (w + 1) + x) as usize] as i32 + EXE_SEA;
        let set = |t: &mut Terrain, x: i32, y: i32, v: i32| t.corner[(y * (w + 1) + x) as usize] = (v - EXE_SEA) as i8;
        if tool == 1 {
            let block: Vec<(i32, i32)> =
                [(cx, cy - 1), (cx + 1, cy - 1), (cx, cy), (cx + 1, cy)].into_iter().filter(|&(x, y)| ok(x, y)).collect();
            if raise {
                let m = block.iter().map(|&(x, y)| at(self, x, y)).fold(99, i32::min);
                for &(x, y) in &block {
                    if at(self, x, y) == m && m < EXE_HIGH {
                        set(self, x, y, m + 1);
                    }
                }
            } else {
                let m = block.iter().map(|&(x, y)| at(self, x, y)).fold(0, i32::max);
                for &(x, y) in &block {
                    if at(self, x, y) == m && m > EXE_LOW {
                        set(self, x, y, m - 1);
                    }
                }
            }
            return;
        }
        if !ok(cx, cy) {
            return;
        }
        let v = at(self, cx, cy);
        let top = if raise || hilly { EXE_HIGH } else { EXE_LOW_CAP };
        let c = (v + if raise { 1 } else { -1 }).clamp(EXE_LOW, top);
        set(self, cx, cy, c);
        if tool == 2 {
            for (i, row) in AREA_KERNEL.iter().enumerate() {
                for (j, &k) in row.iter().enumerate() {
                    let (x, y) = (cx + i as i32 - 2, cy + j as i32 - 2);
                    if !ok(x, y) {
                        continue;
                    }
                    let h = at(self, x, y);
                    if raise && h < c - k {
                        set(self, x, y, h + 1);
                    } else if !raise && h > c + k {
                        set(self, x, y, h - 1);
                    }
                }
            }
        }
    }

    /// Sets the tile's four corners to their average level (scripted painting of water, greens, tees, buildings; the
    /// port's own tooling).
    pub fn flatten_tile(&mut self, x: i32, y: i32) {
        if !self.inside(x, y) {
            return;
        }
        let mut sum = 0;
        for j in 0..2 {
            for i in 0..2 {
                sum += self.corner_at(x + i, y + j);
            }
        }
        let level = ((sum + 2) / 4) as i8;
        for j in 0..2 {
            for i in 0..2 {
                self.corner[((y + j) * (self.w + 1) + x + i) as usize] = level;
            }
        }
    }

    /// Course file: a small text format, see docs/GAMELOGIC.md.
    pub fn to_course_text(&self) -> String {
        use std::fmt::Write;
        let mut f = String::new();
        let _ = write!(f, "SGCOURSE 1\n{} {}\n", self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                let i = self.tile_index(x, y);
                let _ = write!(f, "{},{},{} ", self.ty[i], self.variation[i], self.set[i]);
            }
            f.push('\n');
        }
        for y in 0..=self.h {
            for x in 0..=self.w {
                let _ = write!(f, "{} ", self.corner_at(x, y));
            }
            f.push('\n');
        }
        if self.path_kind.iter().any(|&k| k != 0) {
            f.push_str("PATHS\n");
            for y in 0..self.h {
                for x in 0..self.w {
                    let _ = write!(f, "{}", self.path_at(x, y));
                }
                f.push('\n');
            }
        }
        if self.wall_mask.iter().any(|&m| m != 0) {
            f.push_str("WALLS\n");
            for y in 0..self.h {
                for x in 0..self.w {
                    let _ = write!(f, "{:X}", self.wall_mask[self.tile_index(x, y)] & 15);
                }
                f.push('\n');
            }
        }
        f
    }

    pub fn save(&self, file: &Path) -> Result<(), String> {
        if crate::fsutil::write_file(file, self.to_course_text().as_bytes()) {
            Ok(())
        } else {
            Err(format!("cannot write {}", file.display()))
        }
    }

    /// Parses a course file. The golfers' route, the clubhouse and the desert flag are scenery of the demo, not part of the course
    /// file, so they are taken over from `prev`.
    pub fn from_course_text(text: &str, prev: &Terrain) -> Option<Terrain> {
        let mut tok = text.split_whitespace();
        if tok.next()? != "SGCOURSE" || tok.next()?.parse::<i32>().ok()? != 1 {
            return None;
        }
        let w: i32 = tok.next()?.parse().ok()?;
        let h: i32 = tok.next()?.parse().ok()?;
        if !(1..=512).contains(&w) || !(1..=512).contains(&h) {
            return None;
        }
        let n = (w * h) as usize;
        let mut t = Terrain { w, h, ty: vec![0; n], variation: vec![0; n], set: vec![0; n], ..Default::default() };
        t.corner = vec![0; ((w + 1) * (h + 1)) as usize];
        for i in 0..n {
            let mut f = tok.next()?.split(',').map(crate::formats::atoi);
            t.ty[i] = f.next()? as u8;
            t.variation[i] = f.next()? as u8;
            t.set[i] = f.next()? as u8;
        }
        for c in t.corner.iter_mut() {
            *c = tok.next()?.parse::<i32>().ok()? as i8;
        }
        t.path_kind = vec![0; n];
        t.wall_mask = vec![0; n];
        // Optional sections: PATHS, WALLS (one row of digits per tile row).
        while let Some(word) = tok.next() {
            let paths = word == "PATHS";
            if !paths && word != "WALLS" {
                break;
            }
            for y in 0..h {
                let row = tok.next()?.as_bytes();
                if (row.len() as i32) < w {
                    return None;
                }
                for x in 0..w {
                    let v = (row[x as usize] as char).to_digit(16).unwrap_or(0) as u8;
                    let i = (y * w + x) as usize;
                    if paths {
                        t.path_kind[i] = v.min(2);
                    } else {
                        t.wall_mask[i] = v;
                    }
                }
            }
        }
        t.path = prev.path.clone();
        t.clubhouse_x = prev.clubhouse_x;
        t.clubhouse_y = prev.clubhouse_y;
        t.clubhouse_size = prev.clubhouse_size;
        t.desert = prev.desert;
        Some(t)
    }

    pub fn load(file: &Path, prev: &Terrain) -> Result<Terrain, String> {
        let text = crate::fsutil::read_text(file).ok_or_else(|| format!("cannot read {}", file.display()))?;
        Terrain::from_course_text(&text, prev).ok_or_else(|| format!("bad course file {}", file.display()))
    }

    /// A made-up demonstration course (tee, winding fairway, green, bunkers, pond, woods).
    pub fn demo_course(w: i32, h: i32, seed: u32) -> Terrain {
        let n = (w * h) as usize;
        let mut t = Terrain {
            w,
            h,
            ty: vec![TT_ROUGH; n],
            variation: vec![0; n],
            set: vec![0; n],
            corner: vec![0; ((w + 1) * (h + 1)) as usize],
            path_kind: vec![0; n],
            clubhouse_x: -1,
            clubhouse_y: -1,
            clubhouse_size: 0,
            sand_phase: 0,
            ..Default::default()
        };
        let mut rng = Rng::new(if seed != 0 { seed } else { 1 });
        let (wf, hf) = (w as f32, h as f32);
        let disc = |t: &mut Terrain, cx: f32, cy: f32, rx: f32, ry: f32, ty: u8, only_rough: bool| {
            for y in 0..h {
                for x in 0..w {
                    let dx = (x as f32 + 0.5 - cx) / rx;
                    let dy = (y as f32 + 0.5 - cy) / ry;
                    let i = (y * w + x) as usize;
                    if dx * dx + dy * dy <= 1.0 && (!only_rough || t.ty[i] == TT_ROUGH || t.ty[i] == TT_DEEP_ROUGH) {
                        t.ty[i] = ty;
                    }
                }
            }
        };
        // Outer deep rough, then scattered woods and brush.
        for y in 0..h {
            for x in 0..w {
                let edge = x.min(y).min(w - 1 - x).min(h - 1 - y);
                if edge < 2 {
                    t.ty[(y * w + x) as usize] = TT_DEEP_ROUGH;
                }
            }
        }
        for _ in 0..(w * h / 14) {
            let cx = rng.range(w) as f32;
            let cy = rng.range(h) as f32;
            let r = 1.0 + rng.unit() * 2.2;
            let ty = if rng.range(3) == 0 {
                TT_BRUSH
            } else if rng.range(3) == 0 {
                TT_ROCK
            } else {
                TT_WOODS
            };
            disc(&mut t, cx, cy, r, r, ty, true);
        }
        // Fairway along a curve from tee (bottom-left) to green (top-right).
        let (sx, sy, ex, ey) = (5.0f32, (h - 6) as f32, (w - 7) as f32, 6.0f32);
        let curve = |s: f32| (sx + (ex - sx) * s + (s * 3.14159f32 * 1.4).sin() * hf * 0.14, sy + (ey - sy) * s);
        for step in 0..=200 {
            let (cx, cy) = curve(step as f32 / 200.0);
            disc(&mut t, cx, cy, 3.4, 3.4, TT_ROUGH, false);
        }
        for step in 0..=200 {
            let (cx, cy) = curve(step as f32 / 200.0);
            disc(&mut t, cx, cy, 2.3, 2.3, TT_FAIRWAY, false);
            if step % 5 == 0 {
                t.path.push(cx * TILE_SIZE - wf * TILE_SIZE * 0.5);
                t.path.push(cy * TILE_SIZE - hf * TILE_SIZE * 0.5);
            }
            if step % 4 == 0 {
                disc(&mut t, cx, cy, 1.0, 1.0, TT_FIRM_FAIRWAY, false);
            }
        }
        let gcx = ex + (3.14159f32 * 1.4).sin() * hf * 0.14;
        let gcy = ey;
        disc(&mut t, gcx, gcy, 4.6, 4.6, TT_FIRM_FAIRWAY, false);
        disc(&mut t, gcx, gcy, 3.2, 3.2, TT_PUTTING_GREEN, false);
        disc(&mut t, sx, sy, 2.2, 1.6, TT_TEE, false);
        // Clubhouse lot beside the tee: a 5x5 block of Building tiles.
        t.clubhouse_x = sx as i32 + 6;
        t.clubhouse_y = sy as i32 - 3;
        for y in t.clubhouse_y - 2..=t.clubhouse_y + 2 {
            for x in t.clubhouse_x - 2..=t.clubhouse_x + 2 {
                if t.inside(x, y) {
                    let i = t.tile_index(x, y);
                    t.ty[i] = TT_BUILDING;
                }
            }
        }
        // Bunkers beside the green and mid fairway, a pond to one side.
        disc(&mut t, gcx - 4.5, gcy + 2.5, 1.9, 1.3, TT_POT_SAND_BUNKER, false);
        disc(&mut t, gcx + 3.2, gcy + 3.6, 1.6, 1.6, TT_POT_SAND_BUNKER, false);
        disc(&mut t, wf * 0.5 + 3.0, hf * 0.5 - 1.0, 1.7, 1.2, TT_POT_SAND_BUNKER, false);
        disc(&mut t, wf * 0.5 - 2.0, hf * 0.5 + 6.0, 2.3, 1.5, TT_SAND, true); // blended sand bunker (type 7), dished
        let (pcx, pcy) = (wf * 0.30, hf * 0.38);
        disc(&mut t, pcx, pcy, 5.5, 4.2, TT_WATER_SHALLOW, false);
        disc(&mut t, pcx, pcy, 4.2, 3.0, TT_WATER_MIDDLE, false);
        disc(&mut t, pcx, pcy, 2.4, 1.6, TT_WATER_DEEP, false);
        for i in 0..n {
            // Sets A..E are alternative looks of the same terrain, chosen at random per tile (as the original does).
            t.set[i] = rng.range(5) as u8;
            t.variation[i] = 0;
            if t.ty[i] == TT_TEE {
                t.variation[i] = rng.range(5) as u8; // tees use this byte as their set
            }
            // Water is one type; its depth is the tile's variation byte (1 = middle, 2 = deep).
            if t.ty[i] == TT_WATER_MIDDLE {
                t.ty[i] = TT_WATER_SHALLOW;
                t.variation[i] = 1;
            } else if t.ty[i] == TT_WATER_DEEP {
                t.ty[i] = TT_WATER_SHALLOW;
                t.variation[i] = 2;
            }
        }
        // Gentle hills, flattened under water and greens.
        let p1 = rng.unit() * 6.28;
        let p2 = rng.unit() * 6.28;
        for y in 0..=h {
            for x in 0..=w {
                let v =
                    3.2f32 * (x as f32 * 0.17 + p1).sin() + 2.6f32 * (y as f32 * 0.13 + p2).cos() + 2.0f32 * ((x + y) as f32 * 0.09).sin();
                t.corner[(y * (w + 1) + x) as usize] = ((v + 5.0).round() as i32).clamp(0, 12) as i8;
            }
        }
        for y in 0..h {
            for x in 0..w {
                let ty = t.ty[(y * w + x) as usize];
                let flat = ty == TT_WATER_SHALLOW
                    || ty == TT_WATER_MIDDLE
                    || ty == TT_WATER_DEEP
                    || ty == TT_PUTTING_GREEN
                    || ty == TT_TEE
                    || ty == TT_BUILDING;
                if !flat {
                    continue;
                }
                let level = if ty == TT_PUTTING_GREEN || ty == TT_TEE {
                    6
                } else if ty == TT_BUILDING {
                    5
                } else {
                    1
                };
                for dy in 0..=1 {
                    for dx in 0..=1 {
                        t.corner[((y + dy) * (w + 1) + (x + dx)) as usize] = level;
                    }
                }
            }
        }
        t
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub u: f32,
    pub v: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
}

/// One of the 8 triangles of a tile (2 per 50x50 quadrant of the 3x3 patch). Each triangle carries its own texture: the original
/// picks texture variation 1..9 per triangle from how the tile's neighbours differ from it (see docs/TERRAIN.md, "Edge blending").
#[derive(Clone, Copy, Debug)]
pub struct TileTri {
    /// Texture type id (may differ from the tile type: water depth, bunker style...).
    pub tex_type: i32,
    /// Random per-tile look, wraps into the sets that exist (A..E).
    pub set: i32,
    /// 0..8 -> file suffix 0001..0009.
    pub variation: i32,
    pub v: [Vertex; 3],
}

/// Terrain "class" used to decide where borders are drawn: tiles of the same class blend seamlessly. For the game's tile types
/// 0..22 this is the table golf.exe hands to Terrain.dll at start-up (`land::border_class`, with its Desert rule); the editor-only
/// ids above 22 are grouped with the type they look like.
pub fn type_class(ty: i32, desert: bool) -> i32 {
    match ty {
        0..=22 => crate::land::border_class(ty as u8, desert) as i32,
        23..=25 => crate::land::border_class(17, desert) as i32,
        26 => 1,
        27..=30 | 34 | 35 => 7,
        _ => 100 + ty,
    }
}

/// Relation of a tile to a neighbour: 0 same, 1 different class, 2 same class but different type. Out-of-map neighbours count as 0.
/// Water depth (tile.variation 0/1/2) takes part, as in the original.
pub fn tile_relation(t: &Terrain, x: i32, y: i32, nx: i32, ny: i32) -> i32 {
    if !t.inside(nx, ny) {
        return 0;
    }
    let (me, nb) = (t.tile_index(x, y), t.tile_index(nx, ny));
    let (mt, nt) = (t.ty[me], t.ty[nb]);
    if mt == TT_WATER_SHALLOW {
        let level = t.variation[me];
        let nl = if nt == TT_WATER_SHALLOW { t.variation[nb] } else { 0 };
        if level == 0 && nt != TT_WATER_SHALLOW {
            return 1;
        }
        if nl == level {
            return 0;
        }
        return if level == 0 || (level == 1 && nl == 2) { 2 } else { 1 };
    }
    if type_class(mt as i32, t.desert) != type_class(nt as i32, t.desert) {
        return 1;
    }
    if mt != nt {
        2
    } else {
        0
    }
}

/// Variation 0..8 chosen from the relations (a, b, c) of a triangle: its two adjacent edge neighbours and the diagonal one.
/// Direct port of the original lookup (Tile::m1028 + m1262).
pub fn blend_variation(a: i32, b: i32, c: i32) -> i32 {
    const RAW: [u8; 71] = [
        0, 0, 0, 1, 2, 3, 4, 3, 9, 9, 0, 9, 0, 9, 2, 9, 4, 9, 9, 9, 0, 0, 9, 9, 2, 3, 9, 9, 9, 9, 5, 9, 9, 9, 2, 9, 9, 9, 9, 9, 6, 6, 6, 1,
        9, 9, 9, 9, 9, 9, 7, 9, 7, 9, 9, 9, 9, 9, 9, 9, 8, 6, 9, 9, 9, 9, 9, 9, 9, 9, 7,
    ];
    const MAP: [u8; 10] = [0, 2, 4, 3, 1, 6, 8, 7, 5, 0];
    let r = |v: i32, one: i32, two: i32| {
        if v == 1 {
            one
        } else if v == 2 {
            two
        } else {
            0
        }
    };
    let code = r(a, 4, 40) + r(b, 1, 10) + r(c, 2, 20);
    MAP[RAW[code as usize] as usize] as i32
}

/// Texture type the original picks for a tile type (Tile::m116d).
fn texture_type_for(tile_type: i32, vbyte: i32, param: i32, desert: bool, phase: i32) -> i32 {
    match tile_type {
        1 => {
            if vbyte & 0x80 != 0 {
                TT_TRICKY_GREEN as i32
            } else {
                param
            }
        }
        6 | 21 => TT_ROUGH as i32,
        // Terrain.dll picks the sand picture as (variation & 3) - phase, the phase being the view's quarter turn: the exe
        // hands it the view as 0, 90, 180 or -90 degrees (0x4498a0) and the DLL's setter (0x1000adc0) maps those to 0..3
        // (TERRAIN.md)
        7 => TT_SAND_BUNKER1 as i32 + ((vbyte & 3) - phase).rem_euclid(4),
        13..=16 => 13,
        17 => match vbyte {
            1 => TT_WATER_MIDDLE as i32,
            2 => TT_WATER_DEEP as i32,
            _ if desert => TT_WATER_SHALLOW_DESERT as i32,
            _ => param,
        },
        22 => {
            if (0x40..0x48).contains(&vbyte) {
                TT_ROUGH as i32
            } else {
                param
            }
        }
        _ => param,
    }
}

/// Appends the 8 triangles of tile (tx,ty). Tile = 3x3 vertex patch (corners, edge midpoints, centre), 50 units apart; quadrants
/// split along their NW-SE diagonal; UVs 0, 0.5, 1. Sand (type 7) tiles are dished 13 units like the original. World X/Z are
/// centred on the map: x = tx*100 - w*50.
pub fn build_tile_triangles(t: &Terrain, tx: i32, ty: i32, out: &mut Vec<TileTri>) {
    let me = t.tile_index(tx, ty);
    let (ttype, vbyte, set) = (t.ty[me] as i32, t.variation[me] as i32, t.set[me] as i32);

    // Heights of the 3x3 patch: bilinear between the tile's corner elevations, sand tiles dished.
    let mut c = [[0f32; 2]; 2];
    for j in 0..2 {
        for i in 0..2 {
            c[j][i] = t.corner_at(tx + i as i32, ty + j as i32) as f32 * HEIGHT_STEP;
        }
    }
    let mut hgt = [[0f32; 3]; 3];
    for j in 0..3 {
        for i in 0..3 {
            let (fx, fy) = (i as f32 * 0.5, j as f32 * 0.5);
            hgt[j][i] = (c[0][0] * (1.0 - fx) + c[0][1] * fx) * (1.0 - fy) + (c[1][0] * (1.0 - fx) + c[1][1] * fx) * fy;
        }
    }
    if ttype == 7 {
        let dish = 13.0;
        hgt[1][1] -= dish;
        let sand = |dx: i32, dy: i32| t.type_at(tx + dx, ty + dy) == 7;
        if sand(0, -1) {
            hgt[0][1] -= dish;
        }
        if sand(0, 1) {
            hgt[2][1] -= dish;
        }
        if sand(-1, 0) {
            hgt[1][0] -= dish;
        }
        if sand(1, 0) {
            hgt[1][2] -= dish;
        }
        if sand(0, -1) && sand(-1, 0) && sand(-1, -1) {
            hgt[0][0] -= dish;
        }
        if sand(0, -1) && sand(1, 0) && sand(1, -1) {
            hgt[0][2] -= dish;
        }
        if sand(0, 1) && sand(-1, 0) && sand(-1, 1) {
            hgt[2][0] -= dish;
        }
        if sand(0, 1) && sand(1, 0) && sand(1, 1) {
            hgt[2][2] -= dish;
        }
    }
    let ox = tx as f32 * TILE_SIZE - t.w as f32 * TILE_SIZE * 0.5;
    let oz = ty as f32 * TILE_SIZE - t.h as f32 * TILE_SIZE * 0.5;
    let vtx = |j: usize, k: usize| Vertex {
        x: ox + k as f32 * TILE_SIZE * 0.5,
        y: hgt[j][k],
        z: oz + j as f32 * TILE_SIZE * 0.5,
        u: k as f32 * 0.5,
        v: j as f32 * 0.5,
        nx: 0.0,
        ny: 1.0,
        nz: 0.0,
    };

    // Variation per triangle record (rec = 2*quadrant + {0,1}; quadrants NW, NE, SW, SE).
    let mut var = [0i32; 8];
    if ttype == TT_TEE as i32 || ttype == TT_POT_SAND_BUNKER as i32 {
        // One flat texture; pot bunkers are always the rounded piece.
        var = [if ttype == TT_TEE as i32 { 0 } else { 3 }; 8];
    } else {
        let rel = |dx: i32, dy: i32| tile_relation(t, tx, ty, tx + dx, ty + dy);
        let (n, s, w, e) = (rel(0, -1), rel(0, 1), rel(-1, 0), rel(1, 0));
        let (nw, ne, sw, se) = (rel(-1, -1), rel(1, -1), rel(-1, 1), rel(1, 1));
        // Original triangle index -> (a, b, c), and -> record.
        let abc = [[n, e, ne], [e, n, ne], [e, s, se], [s, e, se], [s, w, sw], [w, s, sw], [w, n, nw], [n, w, nw]];
        const REC_OF: [usize; 8] = [2, 3, 7, 6, 5, 4, 0, 1];
        let tex_type = texture_type_for(ttype, vbyte, ttype, t.desert, t.sand_phase);
        for i in 0..8 {
            let mut v = blend_variation(abc[i][0], abc[i][1], abc[i][2]);
            if (tex_type == TT_ROUGH as i32 || ttype == 7 || tex_type == TT_FAIRWAY as i32) && v > 4 {
                v = 0;
            }
            var[REC_OF[i]] = v;
        }
    }
    let tex_type = if ttype == TT_TEE as i32 || ttype == TT_POT_SAND_BUNKER as i32 {
        ttype
    } else {
        texture_type_for(ttype, vbyte, ttype, t.desert, t.sand_phase)
    };
    let use_set = if ttype == TT_TEE as i32 { vbyte } else { set };
    for q in 0..4 {
        let (j, k) = (q / 2, q % 2);
        // The tile is a fan of eight triangles round its centre, each holding half of one tile edge (the edge its blend
        // variation belongs to), so every quadrant is split along the diagonal from the tile corner to the centre: NW and SE
        // along one diagonal, NE and SW along the other.
        let tris = if j == k {
            [[vtx(j, k), vtx(j + 1, k), vtx(j + 1, k + 1)], [vtx(j, k), vtx(j + 1, k + 1), vtx(j, k + 1)]]
        } else if k == 1 {
            // NE: corner (0,2); first the north half edge, then the east one
            [[vtx(0, 1), vtx(1, 1), vtx(0, 2)], [vtx(1, 1), vtx(1, 2), vtx(0, 2)]]
        } else {
            // SW: corner (2,0); first the west half edge, then the south one
            [[vtx(1, 0), vtx(2, 0), vtx(1, 1)], [vtx(2, 0), vtx(2, 1), vtx(1, 1)]]
        };
        for (s, tri) in tris.iter().enumerate() {
            let (ux, uy, uz) = (tri[1].x - tri[0].x, tri[1].y - tri[0].y, tri[1].z - tri[0].z);
            let (vx, vy, vz) = (tri[2].x - tri[0].x, tri[2].y - tri[0].y, tri[2].z - tri[0].z);
            let (mut nx, mut ny, mut nz) = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx);
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if len > 0.0 {
                nx /= len;
                ny /= len;
                nz /= len;
            }
            if ny < 0.0 {
                nx = -nx;
                ny = -ny;
                nz = -nz;
            }
            let mut tt = TileTri { tex_type, set: use_set, variation: var[q * 2 + s], v: *tri };
            for v in tt.v.iter_mut() {
                v.nx = nx;
                v.ny = ny;
                v.nz = nz;
            }
            out.push(tt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_lookup_matches_original_table() {
        assert_eq!(blend_variation(0, 0, 0), 0);
        // a different class: code 4 -> raw 2 -> variation 4; b alone: code 1 -> raw 0 -> 0; all three: code 7 -> raw 3 -> 3.
        assert_eq!(blend_variation(1, 0, 0), 4);
        assert_eq!(blend_variation(0, 1, 0), 0);
        assert_eq!(blend_variation(1, 1, 1), 3);
        assert_eq!(blend_variation(2, 2, 2), 7);
    }

    #[test]
    fn course_text_roundtrip() {
        let mut t = Terrain::demo_course(12, 10, 3);
        t.path_kind[5] = 2;
        t.set_wall(2, 2, 1, true);
        let back = Terrain::from_course_text(&t.to_course_text(), &t).unwrap();
        assert_eq!(back.ty, t.ty);
        assert_eq!(back.corner, t.corner);
        assert_eq!(back.path_kind, t.path_kind);
        assert_eq!(back.wall_mask, t.wall_mask);
        assert!(back.wall_at(3, 2, 3));
    }

    #[test]
    fn exe_light() {
        let l = light_direction();
        assert!((l[0] + 0.7570).abs() < 1e-3 && (l[1] - 0.6409).abs() < 1e-3 && (l[2] + 0.1271).abs() < 1e-3, "{l:?}");
        let park = parse_lighting("#AMBIENT\r\n200 200 180\r\n#DIFFUSE\r\n240 240 240\r\n#SPECULAR\r\n255 255 245\r\n#HIGHLIGHT\r\nx0x");
        // facing away from the light: the ambient terms only
        let back = park.shade([-l[0], -l[1], -l[2]]);
        assert!((back[0] - (0.04 + 0.2 * 200.0 / 255.0)).abs() < 1e-5);
        // facing it: ambient and full diffuse; the light is almost square to the viewer, so the highlight adds little
        let full = 0.04 + 0.2 * 200.0 / 255.0 + 0.8 * 240.0 / 255.0;
        let lit = park.shade(l);
        assert!(lit[0] > full && lit[0] < full + 0.01, "{lit:?}");
        // flat ground at the 800 x 600 view's pitch (38.68 degrees): about 0.56
        let (s, c) = 38.682186f32.to_radians().sin_cos();
        let ground = park.shade([0.0, c, s]);
        assert!((ground[0] - 0.56).abs() < 0.02, "{ground:?}");
    }

    #[test]
    fn elevation_edits_as_the_exe() {
        let mut t = Terrain::demo_course(8, 8, 1);
        t.corner.iter_mut().for_each(|c| *c = 2); // the exe's height 5 everywhere
                                                  // a vertex: one step, nothing spreads
        t.edit_elevation(0, 4, 4, true, false);
        assert_eq!((t.corner_at(4, 4), t.corner_at(3, 4), t.corner_at(5, 4)), (3, 2, 2));
        // raising stops at 13 (ours 10); lowering on land that is not hilly is capped at 10 (ours 7)
        t.corner[(4 * 9 + 4) as usize] = 10;
        t.edit_elevation(0, 4, 4, true, false);
        assert_eq!(t.corner_at(4, 4), 10);
        t.edit_elevation(0, 4, 4, false, false);
        assert_eq!(t.corner_at(4, 4), 7);
        t.corner[(4 * 9 + 4) as usize] = 10;
        t.edit_elevation(0, 4, 4, false, true);
        assert_eq!(t.corner_at(4, 4), 9);
        // the 2 x 2 block: its lowest corners go up together
        t.corner.iter_mut().for_each(|c| *c = 2);
        t.corner[(3 * 9 + 5) as usize] = 4;
        t.edit_elevation(1, 4, 4, true, false);
        assert_eq!([t.corner_at(4, 3), t.corner_at(5, 3), t.corner_at(4, 4), t.corner_at(5, 4)], [3, 4, 3, 3]);
        t.edit_elevation(1, 4, 4, false, false);
        assert_eq!([t.corner_at(4, 3), t.corner_at(5, 3), t.corner_at(4, 4), t.corner_at(5, 4)], [3, 3, 3, 3]);
        // the area: corners further below the new centre than the kernel allows come up one
        t.corner.iter_mut().for_each(|c| *c = 0);
        for _ in 0..4 {
            t.edit_elevation(2, 4, 4, true, false);
        }
        assert_eq!(t.corner_at(4, 4), 4);
        assert_eq!(t.corner_at(5, 4), 3, "kernel 1 next to the centre");
        assert_eq!(t.corner_at(6, 6), 0, "kernel 6 in the corner");
        assert_eq!(t.corner_at(6, 4), 1, "kernel 3 two corners out");
        // the exe's kernel is not symmetric: 3 at (+1, +2) but 4 at (+1, -2)
        assert_eq!((t.corner_at(5, 6), t.corner_at(5, 2)), (1, 0));
    }

    #[test]
    fn tile_has_eight_triangles() {
        let t = Terrain::demo_course(10, 10, 9);
        let mut v = Vec::new();
        build_tile_triangles(&t, 4, 4, &mut v);
        assert_eq!(v.len(), 8);
        assert!(v.iter().all(|tri| tri.v.iter().all(|p| p.ny >= 0.0)));
    }
}
