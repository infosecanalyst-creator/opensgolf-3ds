//! The walk camera's ground (walk.rs): the course's own tiles with their terrain art, cut into chunks it can leave out when
//! they are off the screen or lost in the haze, each triangle marked with what kind of ground it is for the detail layer
//! (walk_gl.rs); the water apart, for its own shader; and the land beyond the property's edge, a skirt of the theme's
//! ground running out to low hills (or down into the sea where the course meets it), so the world does not stop at a
//! line. It is built from the terrain when the walk begins and again whenever the terrain changes; it reads the game's
//! state and changes none of it.
use crate::app::App;
use crate::gfx::{Gfx, Mesh, Vert};
use crate::walk_gl::{cat, cat_colour};
use miniquad::TextureId;
use sg_core::assets::Rgba;
use sg_core::terrain::{build_tile_triangles, Terrain, TILE_SIZE};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Tiles along a chunk's edge.
pub const CHUNK: i32 = 12;

/// A part of the ground: its bounds and its triangles by texture (with the texture's size in texels).
pub struct Chunk {
    pub lo: [f32; 3],
    pub hi: [f32; 3],
    pub parts: Vec<Part>,
}

/// One texture's triangles in a chunk: the art, its de-featured copy for near the eye (`clean_image`), its size in texels.
pub struct Part {
    pub tex: TextureId,
    pub clean: TextureId,
    pub mesh: Mesh,
    pub size: f32,
}

/// The land beyond the map's edge: the edge's heights and wetness all round it (a point every half tile), from which the
/// skirt is built and its height anywhere worked out again (the trees out there stand on it).
pub struct Beyond {
    hx: f32,
    hz: f32,
    edge_h: Vec<f32>,
    wet: Vec<f32>,
    mean_h: f32,
    sea_level: f32,
    theme: usize,
}

impl Beyond {
    fn new(t: &Terrain, theme: usize, sea_level: f32) -> Beyond {
        let (hx, hz) = (t.w as f32 * TILE_SIZE * 0.5, t.h as f32 * TILE_SIZE * 0.5);
        let n = (4 * (t.w + t.h)) as usize;
        let edge: Vec<(f32, f32)> = (0..n).map(|i| on_rect(hx, hz, i as f32 / n as f32)).collect();
        let inside = |x: f32, z: f32| (x.clamp(-hx + 0.01, hx - 0.01), z.clamp(-hz + 0.01, hz - 0.01));
        let tile_of = |x: f32, z: f32| (((x + hx) / TILE_SIZE).floor() as i32, ((z + hz) / TILE_SIZE).floor() as i32);
        // whether the edge meets water there, smoothed along it
        let raw_wet: Vec<f32> = edge
            .iter()
            .map(|&(x, z)| {
                let (x, z) = inside(x, z);
                let (tx, ty) = tile_of(x, z);
                is_water(t, tx, ty) as i32 as f32
            })
            .collect();
        let wet: Vec<f32> = (0..n).map(|i| (0..9).map(|k| raw_wet[(i + n + k - 4) % n]).sum::<f32>() / 9.0).collect();
        let edge_h: Vec<f32> = edge.iter().map(|&(x, z)| t.height_at(inside(x, z).0, inside(x, z).1)).collect();
        let mean_h = edge_h.iter().sum::<f32>() / n.max(1) as f32;
        Beyond { hx, hz, edge_h, wet, mean_h, sea_level, theme }
    }

    fn n(&self) -> usize {
        self.edge_h.len()
    }

    /// The point `d` out from the edge's point `i`: the land sinks into the sea where the edge is water; elsewhere it eases
    /// to the edge's mean height, then rises into hills far off.
    fn point(&self, i: usize, d: f32) -> [f32; 3] {
        let n = self.n();
        let i = i % n;
        let (x, z) = on_rect(self.hx + d, self.hz + d, i as f32 / n as f32);
        let fade = |a: f32, b: f32| ((d - a) / (b - a)).clamp(0.0, 1.0);
        let s = |v: f32| v * v * (3.0 - 2.0 * v);
        let land = self.edge_h[i]
            + (self.mean_h - self.edge_h[i]) * s(fade(0.0, 1500.0))
            + hill_height(self.theme, x, z) * s(fade(1200.0, 6000.0));
        let sea = self.sea_level - 25.0 * s(fade(0.0, 60.0));
        let k = self.wet[i];
        let y = if d == 0.0 { self.edge_h[i] } else { land * (1.0 - k) + sea * k };
        [x, y, z]
    }

    /// The skirt's height at a point beyond the edge, as its mesh has it (between its rings and points), and how wet the
    /// edge is there; None inside the map or past the last ring.
    pub fn at(&self, x: f32, z: f32) -> Option<(f32, f32)> {
        let d = (x.abs() - self.hx).max(z.abs() - self.hz);
        if d <= 0.0 {
            return None;
        }
        let r = RINGS.iter().rposition(|&rd| rd <= d)?;
        if r + 1 >= RINGS.len() {
            return None;
        }
        let v = (d - RINGS[r]) / (RINGS[r + 1] - RINGS[r]);
        let n = self.n();
        let f = rect_param(self.hx + d, self.hz + d, x, z) * n as f32;
        let i = f.floor() as usize % n;
        let u = f - f.floor();
        // the quad's two triangles, split as the mesh splits them (q0 q1 q2, q0 q2 q3)
        let (q0, q1, q2, q3) = (
            self.point(i, RINGS[r])[1],
            self.point(i + 1, RINGS[r])[1],
            self.point(i + 1, RINGS[r + 1])[1],
            self.point(i, RINGS[r + 1])[1],
        );
        let y = if u >= v { q0 + u * (q1 - q0) + v * (q2 - q1) } else { q0 + v * (q3 - q0) + u * (q2 - q3) };
        Some((y, self.wet[i].max(self.wet[(i + 1) % n])))
    }
}

/// Where a point on the edge of a `hx` x `hz` rectangle is round it (0..1), as `on_rect` counts.
pub fn rect_param(hx: f32, hz: f32, x: f32, z: f32) -> f32 {
    let per = 4.0 * (hx + hz);
    // the side the point is nearest
    let d = if (z + hz).abs() <= (x - hx).abs().min((z - hz).abs()).min((x + hx).abs()) {
        x + hx
    } else if (x - hx).abs() <= (z - hz).abs().min((x + hx).abs()) {
        2.0 * hx + (z + hz)
    } else if (z - hz).abs() <= (x + hx).abs() {
        2.0 * hx + 2.0 * hz + (hx - x)
    } else {
        4.0 * hx + 2.0 * hz + (hz - z)
    };
    (d / per).rem_euclid(1.0)
}

pub struct Ground {
    /// What the ground was built from (`ground_key`).
    pub key: u64,
    pub chunks: Vec<Chunk>,
    pub water: Vec<Chunk>,
    /// The land beyond the edge and its texture's size.
    pub skirt: Option<Part>,
    /// The sea beyond the edge where the course meets it.
    pub sea: Option<Part>,
    /// The textures made for the walk (the skirt's, the de-featured copies), freed with it.
    own: Vec<TextureId>,
    /// The land beyond the edge.
    pub beyond: Beyond,
    /// The open water's colour (the deep water's picture's average).
    pub water_colour: [f32; 3],
}

impl Ground {
    pub fn free(self, g: &mut Gfx) {
        for c in self.chunks.into_iter().chain(self.water) {
            for p in c.parts {
                g.delete_mesh(p.mesh);
            }
        }
        for p in self.skirt.into_iter().chain(self.sea) {
            g.delete_mesh(p.mesh);
        }
        for t in self.own {
            g.ctx.delete_texture(t);
        }
    }
}

/// A hash of everything the ground is built from: the terrain's tiles, corners and paths, the theme and the art in use.
pub fn ground_key(app: &App) -> u64 {
    let t = &app.terrain;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (t.w, t.h, app.theme, t.water_one, t.desert, t.sand_phase).hash(&mut h);
    t.ty.hash(&mut h);
    t.variation.hash(&mut h);
    t.set.hash(&mut h);
    t.corner.hash(&mut h);
    t.tile_corner.hash(&mut h);
    t.path_kind.hash(&mut h);
    app.batches.len().hash(&mut h);
    h.finish()
}

/// The detail category of a terrain texture type in a theme (app::THEMES: 0 Parkland, 1 Links, 2 Desert, 3 Tropical).
pub fn category(tex_type: i32, theme: usize) -> u8 {
    use sg_core::terrain as tt;
    match tex_type as u8 {
        tt::TT_TEE => cat::TEE,
        tt::TT_PUTTING_GREEN | tt::TT_TRICKY_GREEN => cat::GREEN,
        tt::TT_FAIRWAY | tt::TT_FIRM_FAIRWAY => cat::FAIRWAY,
        tt::TT_ROUGH | tt::TT_DEEP_ROUGH | tt::TT_MARSH | tt::TT_GRASS_BUNKER if theme == 2 => cat::DESERT,
        tt::TT_ROUGH | tt::TT_DEEP_ROUGH | tt::TT_MARSH | tt::TT_GRASS_BUNKER => cat::ROUGH,
        tt::TT_SAND | tt::TT_GRASSY_SAND | tt::TT_POT_SAND_BUNKER | tt::TT_ZEN_SAND => cat::SAND,
        27..=30 => cat::SAND,
        tt::TT_OVERGROWTH | tt::TT_OVERGROWTH2 | tt::TT_BRUSH | tt::TT_WOODS | tt::TT_FLOWER_BED => cat::WOODS,
        tt::TT_ROCK | tt::TT_CLIFF | tt::TT_RAVINE => cat::ROCK,
        tt::TT_WATER_SHALLOW | tt::TT_WATER_MIDDLE | tt::TT_WATER_DEEP | tt::TT_WATER_SHALLOW_DESERT => cat::WATER,
        _ if theme == 2 => cat::DESERT,
        _ => cat::GRASS,
    }
}

/// Whether a tile is water (the course's type 17).
pub fn is_water(t: &Terrain, x: i32, y: i32) -> bool {
    t.inside(x, y) && t.type_at(x, y) == 17
}

/// A terrain picture from the player's own files (or the free set's).
fn load_art(path: &std::path::Path) -> Option<Rgba> {
    let data = sg_core::fsutil::read_file(path)?;
    if sg_core::fsutil::ext_lower(path) == ".tga" {
        sg_core::assets::decode_tga(&data).ok()
    } else {
        sg_core::assets::decode_bmp(&data).ok()
    }
}

/// A picture as a mipmapped texture of the walk's own, repeating or clamped, anisotropically filtered.
fn art_texture(g: &mut Gfx, img: &Rgba, repeat: bool) -> TextureId {
    let params = miniquad::TextureParams {
        kind: miniquad::TextureKind::Texture2D,
        width: img.w,
        height: img.h,
        format: miniquad::TextureFormat::RGBA8,
        wrap: if repeat { miniquad::TextureWrap::Repeat } else { miniquad::TextureWrap::Clamp },
        min_filter: miniquad::FilterMode::Linear,
        mag_filter: miniquad::FilterMode::Linear,
        mipmap_filter: miniquad::MipmapFilterMode::Linear,
        allocate_mipmaps: true,
        sample_count: 1,
    };
    let id = g.ctx.new_texture(miniquad::TextureAccess::Static, miniquad::TextureSource::Bytes(&img.px), params);
    g.ctx.texture_generate_mipmaps(id);
    crate::walk_gl::set_anisotropy(g, id, 8.0);
    id
}

/// A terrain picture with its small painted features taken out (`CLEAN` texels across, at 64 texels a tile): the median
/// of each texel's neighbourhood, channel by channel, which drops a painted bush, stone or wave line but keeps the edges
/// between kinds of ground. Larger pictures are first boxed down to 64 texels a side. `wrap` reads round the edges (a
/// repeating picture), else clamps.
pub fn clean_image(img: &Rgba, wrap: bool) -> Rgba {
    let mut src = img.clone();
    while src.w > 64 && src.h > 64 && src.w.is_multiple_of(2) && src.h.is_multiple_of(2) {
        let (w, h) = (src.w as usize / 2, src.h as usize / 2);
        let mut d = Rgba::new(w as u32, h as u32);
        for y in 0..h {
            for x in 0..w {
                for c in 0..4 {
                    let at = |dx: usize, dy: usize| src.px[((2 * y + dy) * src.w as usize + 2 * x + dx) * 4 + c] as u32;
                    d.px[(y * w + x) * 4 + c] = ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
                }
            }
        }
        src = d;
    }
    let (w, h) = (src.w as i32, src.h as i32);
    let r = CLEAN * w.max(1) / 64;
    let r = r.max(1);
    let mut out = src.clone();
    let mut buf: Vec<u8> = Vec::with_capacity(((2 * r + 1) * (2 * r + 1)) as usize);
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                buf.clear();
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx * dx + dy * dy > r * r + r {
                            continue;
                        }
                        let (sx, sy) = if wrap {
                            ((x + dx).rem_euclid(w), (y + dy).rem_euclid(h))
                        } else {
                            ((x + dx).clamp(0, w - 1), (y + dy).clamp(0, h - 1))
                        };
                        buf.push(src.px[((sy * w + sx) * 4 + c) as usize]);
                    }
                }
                let mid = buf.len() / 2;
                let (_, m, _) = buf.select_nth_unstable(mid);
                out.px[((y * w + x) * 4 + c) as usize] = *m;
            }
        }
    }
    out
}

/// The radius of `clean_image`'s median, texels at 64 a tile.
const CLEAN: i32 = 4;

/// Smooth noise over the plane (not repeating), 0..1.
pub fn noise2(x: f32, y: f32, seed: u32) -> f32 {
    let h = |a: i32, b: i32| {
        let mut h = (a as u32).wrapping_mul(0x8da6_b343) ^ (b as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
        h ^= h >> 13;
        h = h.wrapping_mul(0x5bd1_e995);
        h ^= h >> 15;
        (h & 0xffff) as f32 / 65535.0
    };
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (tx, ty) = (x - ix as f32, y - iy as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(tx), s(ty));
    let top = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * sx;
    let bot = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * sx;
    top + (bot - top) * sy
}

/// The hills' height at a world point, before it is faded in with distance: rolling in Parkland, low dunes on Links,
/// steep ridges in the Desert, high peaks in the Tropics.
pub fn hill_height(theme: usize, x: f32, z: f32) -> f32 {
    let (scale, height, sharp) = match theme {
        1 => (2600.0, 420.0, 1.0),
        2 => (3800.0, 1100.0, 2.2),
        3 => (3200.0, 1300.0, 1.8),
        _ => (3000.0, 950.0, 1.0),
    };
    let mut v = 0.0;
    let mut amp = 1.0;
    let mut f = 1.0 / scale;
    let mut total = 0.0;
    for o in 0..4 {
        v += noise2(x * f, z * f, 31 + o) * amp;
        total += amp;
        amp *= 0.45;
        f *= 2.1;
    }
    let v = (v / total).clamp(0.0, 1.0);
    // a ridged look for sharper themes: peaks from the noise's middle
    let r = if sharp > 1.0 { (1.0 - (v - 0.5).abs() * 2.0).powf(sharp) } else { v };
    r * height
}

/// The rings of the skirt, as distances out from the map's edge (world units): close together near the edge, where the
/// ground meets the map's own, and ever wider out to the horizon's hills.
pub const RINGS: [f32; 19] = [
    0.0, 30.0, 70.0, 130.0, 210.0, 320.0, 460.0, 650.0, 900.0, 1250.0, 1700.0, 2300.0, 3100.0, 4200.0, 5600.0, 7500.0, 10000.0, 13500.0,
    18000.0,
];

/// A point on the edge of a `hx` x `hz` rectangle (half sizes) round the map's centre, at `s` (0..1) of the way round it
/// from its north-west corner clockwise.
pub fn on_rect(hx: f32, hz: f32, s: f32) -> (f32, f32) {
    let per = 4.0 * (hx + hz);
    let mut d = s.rem_euclid(1.0) * per;
    let sides = [
        (2.0 * hx, (-hx, -hz), (1.0, 0.0)),
        (2.0 * hz, (hx, -hz), (0.0, 1.0)),
        (2.0 * hx, (hx, hz), (-1.0, 0.0)),
        (2.0 * hz, (-hx, hz), (0.0, -1.0)),
    ];
    for (len, (x0, z0), (dx, dz)) in sides {
        if d <= len {
            return (x0 + dx * d, z0 + dz * d);
        }
        d -= len;
    }
    (-hx, -hz)
}

impl App {
    /// Builds the walk camera's ground from the terrain (see the module's notes).
    pub(crate) fn walk_ground(&mut self, g: &mut Gfx) -> Ground {
        let key = ground_key(self);
        let theme = self.theme;
        let (w, h) = (self.terrain.w, self.terrain.h);
        let mut own: Vec<TextureId> = Vec::new();
        // a catalog picture as a repeating texture and its de-featured copy (the skirt's ground, the sea)
        let mut skirt_avg = None;
        let mut water_avg = None;
        let mut repeat = |g: &mut Gfx, app: &App, ty: i32| -> Option<(TextureId, TextureId, f32)> {
            let img = load_art(app.catalog.pick(ty, 0, 0)?)?;
            let (t, c) = (art_texture(g, &img, true), art_texture(g, &clean_image(&img, true), true));
            own.extend([t, c]);
            if ty == sg_core::terrain::TT_ROUGH as i32 {
                skirt_avg = crate::walk_gl::average_colour(&img);
            } else {
                water_avg = crate::walk_gl::average_colour(&img);
            }
            Some((t, c, img.w as f32))
        };
        let skirt_tex = repeat(g, self, sg_core::terrain::TT_ROUGH as i32);
        let sea_tex =
            repeat(g, self, sg_core::terrain::TT_WATER_DEEP as i32).or_else(|| repeat(g, self, sg_core::terrain::TT_WATER_SHALLOW as i32));
        let skirt_cat = if theme == 2 { cat::DESERT } else { cat::ROUGH };
        // each terrain texture's size and de-featured copy, by its texture, and its average colour
        let mut known: HashMap<TextureId, (TextureId, f32)> = HashMap::new();
        let mut averages: HashMap<TextureId, [f32; 3]> = HashMap::new();
        // each category's colour, summed over its triangles' pictures (the near ground's evening, walk_gl.rs `art`)
        let mut cat_sum = [[0.0f32; 4]; 16];
        let mut pending = Vec::new();
        let mut chunks = Vec::new();
        let mut water = Vec::new();
        let mut tris = Vec::new();
        let mut sea_level = f32::MAX;
        let white = g.white;
        for cy in (0..h).step_by(CHUNK as usize) {
            for cx in (0..w).step_by(CHUNK as usize) {
                let mut land: HashMap<TextureId, Vec<Vert>> = HashMap::new();
                let wet: HashMap<TextureId, Vec<Vert>> = HashMap::new();
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for y in cy..(cy + CHUNK).min(h) {
                    for x in cx..(cx + CHUNK).min(w) {
                        tris.clear();
                        build_tile_triangles(&self.terrain, x, y, &mut tris);
                        let out = self.terrain.type_at(x, y) == 20;
                        for t in &tris {
                            // water and the tiles beside it are drawn with the water the art paints into them (the
                            // ground's shader tells its texels apart by colour)
                            let shore = !out && (-1..=1).any(|dy| (-1..=1).any(|dx| is_water(&self.terrain, x + dx, y + dy)));
                            let (tex, c) = if out {
                                // land outside the property: the skirt's ground, continuing under the edge
                                let tex = skirt_tex.map(|s| s.0).unwrap_or(white);
                                known.entry(tex).or_insert_with(|| skirt_tex.map(|s| (s.1, s.2)).unwrap_or((white, 1.0)));
                                (tex, skirt_cat)
                            } else {
                                let path = self.catalog.pick(t.tex_type, t.set, t.variation).cloned();
                                let tex = path.as_ref().and_then(|p| self.texture_for(g, p)).unwrap_or(white);
                                if let std::collections::hash_map::Entry::Vacant(slot) = known.entry(tex) {
                                    let size = g.ctx.texture_size(tex).0 as f32;
                                    let clean = match path.as_deref().and_then(load_art) {
                                        Some(img) => {
                                            let c = art_texture(g, &clean_image(&img, false), false);
                                            own.push(c);
                                            if let Some(a) = crate::walk_gl::average_colour(&img) {
                                                averages.insert(tex, a);
                                            }
                                            c
                                        }
                                        None => tex,
                                    };
                                    slot.insert((clean, size));
                                }
                                // a water tile's dry texels (its shore) are the land round it, the water itself is found
                                // texel by texel (walk_gl.rs `shore_wet`)
                                let c = if self.terrain.type_at(x, y) == 17 { skirt_cat } else { category(t.tex_type, theme) };
                                (tex, c)
                            };
                            if let Some(a) = averages.get(&tex) {
                                let e = &mut cat_sum[c as usize & 15];
                                (e[0], e[1], e[2], e[3]) = (e[0] + a[0], e[1] + a[1], e[2] + a[2], e[3] + 1.0);
                            }
                            for v in &t.v {
                                for (k, p) in [v.x, v.y, v.z].into_iter().enumerate() {
                                    lo[k] = lo[k].min(p);
                                    hi[k] = hi[k].max(p);
                                }
                                let mut vv = Vert::new(v.x, v.y, v.z, v.u, v.v);
                                if out {
                                    vv.uv = [v.x / TILE_SIZE, v.z / TILE_SIZE];
                                }
                                vv.normal = [v.nx, v.ny, v.nz];
                                vv.color = cat_colour(c);
                                if shore {
                                    vv.color[3] += 0.5;
                                }
                                if self.terrain.type_at(x, y) == 17 {
                                    sea_level = sea_level.min(v.y);
                                }
                                land.entry(tex).or_default().push(vv);
                            }
                        }
                    }
                }
                pending.push((lo, hi, land, wet));
            }
        }
        // the land's vertices take their category's colour
        let cat_avg: Vec<[f32; 3]> =
            cat_sum.iter().map(|e| if e[3] > 0.0 { [e[0] / e[3], e[1] / e[3], e[2] / e[3]] } else { [0.5; 3] }).collect();
        // and the water's, the open water's colour (the deep water's picture), its depth in alpha
        let wa = water_avg.unwrap_or(cat_avg[cat::WATER as usize]);
        for (lo, hi, mut land, wet) in pending {
            for v in land.values_mut().flatten() {
                let c = crate::walk_gl::cat_of(v.color[3]).0 as usize & 15;
                v.color = [cat_avg[c][0], cat_avg[c][1], cat_avg[c][2], v.color[3]];
            }
            {
                let lo2 = lo;
                let mut parts = |m: HashMap<TextureId, Vec<Vert>>, out: &mut Vec<Chunk>| {
                    let parts: Vec<Part> = m
                        .into_iter()
                        .map(|(t, v)| {
                            let (clean, size) = known.get(&t).copied().unwrap_or((t, 64.0));
                            Part { tex: t, clean, mesh: g.mesh(&v), size }
                        })
                        .collect();
                    if !parts.is_empty() {
                        out.push(Chunk { lo: lo2, hi, parts });
                    }
                };
                parts(land, &mut chunks);
                parts(wet, &mut water);
            }
        }
        if sea_level == f32::MAX {
            sea_level = 0.0;
        }
        let beyond = Beyond::new(&self.terrain, theme, sea_level);
        let skirt_colour = if cat_sum[skirt_cat as usize][3] > 0.0 { cat_avg[skirt_cat as usize] } else { skirt_avg.unwrap_or([0.5; 3]) };
        let (skirt, sea) = walk_skirt(g, &beyond, skirt_tex, sea_tex, sea_level, skirt_cat, skirt_colour, wa);
        Ground { key, chunks, water, skirt, sea, own, beyond, water_colour: wa }
    }
}

/// The land beyond the map's edge (and the sea, where the course's edge is water).
#[allow(clippy::too_many_arguments)]
fn walk_skirt(
    g: &mut Gfx,
    beyond: &Beyond,
    skirt_tex: Option<(TextureId, TextureId, f32)>,
    sea_tex: Option<(TextureId, TextureId, f32)>,
    sea_level: f32,
    skirt_cat: u8,
    skirt_colour: [f32; 3],
    water_colour: [f32; 3],
) -> (Option<Part>, Option<Part>) {
    let n = beyond.n();
    let wet = &beyond.wet;
    let point = |i: usize, d: f32| beyond.point(i, d);
    let rings = RINGS.len();
    let pts: Vec<[f32; 3]> = (0..rings).flat_map(|r| (0..n).map(move |i| (r, i))).map(|(r, i)| point(i, RINGS[r])).collect();
    let at = |r: usize, i: usize| pts[r * n + i % n];
    // normals from the neighbours along and across the rings
    let normal = |r: usize, i: usize| -> [f32; 3] {
        let a = at(r, i + 1);
        let b = at(r, i + n - 1);
        let c = at((r + 1).min(rings - 1), i);
        let d = at(r.saturating_sub(1), i);
        let u = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let v = [c[0] - d[0], c[1] - d[1], c[2] - d[2]];
        let mut nn = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        if nn[1] < 0.0 {
            nn = nn.map(|q| -q);
        }
        let l = (nn[0] * nn[0] + nn[1] * nn[1] + nn[2] * nn[2]).sqrt().max(1e-6);
        nn.map(|q| q / l)
    };
    let vert = |r: usize, i: usize, c: u8| {
        let p = at(r, i);
        let mut v = Vert::new(p[0], p[1], p[2], p[0] / TILE_SIZE, p[2] / TILE_SIZE);
        v.normal = normal(r, i);
        v.color = [skirt_colour[0], skirt_colour[1], skirt_colour[2], cat_colour(c)[3]];
        v
    };
    let mut land = Vec::new();
    let mut sea = Vec::new();
    for r in 0..rings - 1 {
        for i in 0..n {
            let quad = [vert(r, i, skirt_cat), vert(r, i + 1, skirt_cat), vert(r + 1, i + 1, skirt_cat), vert(r + 1, i, skirt_cat)];
            land.extend_from_slice(&[quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]);
            if wet[i] > 0.0 || wet[(i + 1) % n] > 0.0 {
                // the sea's surface over this strip, a little past the land's edge
                let s = |rr: usize, ii: usize| {
                    let p = at(rr, ii);
                    let mut v = Vert::new(p[0], sea_level, p[2], p[0] / TILE_SIZE, p[2] / TILE_SIZE);
                    v.color = [water_colour[0], water_colour[1], water_colour[2], 1.0];
                    v
                };
                let q = [s(r, i), s(r, i + 1), s(r + 1, i + 1), s(r + 1, i)];
                sea.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
            }
        }
    }
    let (white, one) = (g.white, (g.white, g.white, 1.0));
    let (st, sc, ss) = skirt_tex.unwrap_or(one);
    let skirt = Some(Part { tex: st, clean: sc, mesh: g.mesh(&land), size: ss });
    let (wt, wc, ws) = sea_tex.unwrap_or(one);
    let sea = (!sea.is_empty()).then(|| Part { tex: wt, clean: wc, mesh: g.mesh(&sea), size: ws });
    let _ = white;
    (skirt, sea)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rectangle_is_walked_round_from_its_north_west_corner() {
        let (hx, hz) = (300.0, 200.0);
        assert_eq!(on_rect(hx, hz, 0.0), (-300.0, -200.0));
        // the north edge is 600 of the 2000 round it
        let (x, z) = on_rect(hx, hz, 0.3);
        assert!((x - 300.0).abs() < 1e-3 && (z + 200.0).abs() < 1e-3);
        let (x, z) = on_rect(hx, hz, 0.5);
        assert!((x - 300.0).abs() < 1e-3 && (z - 200.0).abs() < 1e-3);
        // every point is on the edge
        for i in 0..100 {
            let (x, z) = on_rect(hx, hz, i as f32 / 100.0);
            assert!(((x.abs() - hx).abs() < 1e-3 && z.abs() <= hz + 1e-3) || ((z.abs() - hz).abs() < 1e-3 && x.abs() <= hx + 1e-3));
        }
    }

    #[test]
    fn cleaning_takes_out_small_features_and_keeps_edges() {
        // sand on the left half, grass on the right, a small dark bush in the sand
        let mut img = Rgba::new(64, 64);
        for y in 0..64 {
            for x in 0..64 {
                let c = if x < 32 { [200u8, 150, 90, 255] } else { [60, 140, 50, 255] };
                img.px[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4].copy_from_slice(&c);
            }
        }
        for y in 20..24 {
            for x in 10..14 {
                img.px[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4].copy_from_slice(&[40, 60, 30, 255]);
            }
        }
        let out = clean_image(&img, false);
        let at = |x: usize, y: usize| [out.px[(y * 64 + x) * 4], out.px[(y * 64 + x) * 4 + 1], out.px[(y * 64 + x) * 4 + 2]];
        assert_eq!(at(12, 22), [200, 150, 90], "the bush is gone");
        // the edge between sand and grass stays sharp
        assert_eq!(at(30, 40), [200, 150, 90]);
        assert_eq!(at(33, 40), [60, 140, 50]);
        // a big picture is boxed down to 64 texels first
        let big = Rgba::new(256, 256);
        assert_eq!(clean_image(&big, true).w, 64);
    }

    #[test]
    fn a_point_s_place_round_the_rectangle_comes_back() {
        for i in 0..200 {
            let s = i as f32 / 200.0;
            let (x, z) = on_rect(300.0, 200.0, s);
            let back = rect_param(300.0, 200.0, x, z);
            let d = (back - s).abs().min(1.0 - (back - s).abs());
            assert!(d < 1e-4, "{s} -> {back}");
        }
    }

    #[test]
    fn categories_by_theme() {
        use sg_core::terrain as tt;
        assert_eq!(category(tt::TT_FAIRWAY as i32, 0), cat::FAIRWAY);
        assert_eq!(category(tt::TT_ROUGH as i32, 0), cat::ROUGH);
        // the desert's rough is its sandy ground
        assert_eq!(category(tt::TT_ROUGH as i32, 2), cat::DESERT);
        assert_eq!(category(28, 1), cat::SAND);
        assert_eq!(category(tt::TT_WATER_DEEP as i32, 3), cat::WATER);
        assert_eq!(category(tt::TT_TRICKY_GREEN as i32, 3), cat::GREEN);
    }

    #[test]
    fn hills_stay_in_their_theme_s_range() {
        for theme in 0..4 {
            let mut hi: f32 = 0.0;
            for i in 0..400 {
                let v = hill_height(theme, i as f32 * 137.0, i as f32 * -91.0);
                assert!(v >= 0.0);
                hi = hi.max(v);
            }
            assert!(hi > 50.0 && hi < 1400.0, "{theme}: {hi}");
        }
    }
}
