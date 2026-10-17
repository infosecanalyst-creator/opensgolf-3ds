//! The Path styles and lamps extra (the port's own, purely cosmetic; sg_core::extras `path_style`, docs/PLAYING.md): every
//! path of the course drawn in another material, and lamp posts the player places from the landmark strip.
//!
//! * The materials (brick, gravel, dark stone, wooden boardwalk) are the theme's own path textures (Path.tga and the rest,
//!   which the free base set paints too) recoloured and patterned here: the art's brightness is kept as shading (its grain
//!   and its dark rims), the pattern is laid in the texture's pixels, along each arm of the cross the texture is (bricks and
//!   planks run with the path), with periods that divide the texture so it joins from tile to tile. The colours are rounded
//!   to the original's 15-bit palette steps.
//! * A lamp post is drawn by code, in the sprites' depth order, in sprite pixels (so it grows and shrinks with the zoom as
//!   the art does): a dark post lit from the left like the game's art, and a lantern. With Enhanced graphics' time of day the
//!   lantern lights from dusk with a halo, and a warm pool of light lies on the ground round the post, drawn with the
//!   evening's other pools in their one pass (fx_night.rs `fx_light_pools`). Classic graphics have no evening light, but
//!   the lantern still lights by the same clock (the light's day runs on the game tick), with no halo or pool.
//!
//! Nothing here draws or changes anything while the extra is off.
use crate::app::App;
use crate::gfx::{Gfx, Vert};
use miniquad::TextureId;
use sg_core::assets::Rgba;
use sg_core::sprites::SPRITE_UNITS_PER_PIXEL;
use sg_core::terrain::TILE_SIZE;

fn hash(a: u32, b: u32, salt: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9e37_79b1) ^ b.wrapping_mul(0x85eb_ca77) ^ salt.wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ h >> 15
}

/// A colour channel rounded to the original's 15-bit palette steps (5 bits, the low bits repeating the high ones).
fn c5(v: f32) -> u8 {
    let v = v.clamp(0.0, 255.0) as u8 & 0xf8;
    v | v >> 5
}

/// The material's own colour of texture pixel (x, y) (before the art's shading) and how much of the art's shading it
/// keeps. `along_u`: the pixel is on the arm that runs along the texture's u (or in the middle, or on a curve piece).
fn material_at(material: u8, x: i32, y: i32, along_u: bool) -> ([f32; 3], f32) {
    // s along the arm, t across it
    let (s, t) = if along_u { (x, y) } else { (y, x) };
    match material {
        // brick: running bond, 8 x 4 pixel bricks in pale mortar
        1 => {
            let row = t.div_euclid(4);
            let s = s + if row & 1 != 0 { 4 } else { 0 };
            if t.rem_euclid(4) == 0 || s.rem_euclid(8) == 0 {
                return ([178.0, 166.0, 148.0], 0.3);
            }
            const BRICKS: [[f32; 3]; 4] = [[158.0, 66.0, 48.0], [142.0, 58.0, 44.0], [170.0, 84.0, 58.0], [132.0, 62.0, 52.0]];
            let c = BRICKS[(hash(s.div_euclid(8).rem_euclid(8) as u32, row.rem_euclid(16) as u32, 1) % 4) as usize];
            let top = if t.rem_euclid(4) == 1 { 1.08 } else { 1.0 };
            (c.map(|v| v * top), 0.35)
        }
        // gravel: pea gravel of greys and tans, pebbles of one and two pixels
        2 => {
            const STONES: [[f32; 3]; 5] =
                [[198.0, 194.0, 184.0], [160.0, 156.0, 148.0], [120.0, 116.0, 112.0], [178.0, 160.0, 128.0], [146.0, 138.0, 124.0]];
            let (x, y) = (x.rem_euclid(64) as u32, y.rem_euclid(64) as u32);
            let big = hash(x >> 1, y >> 1, 2);
            let k = if big & 0x300 == 0 { big } else { hash(x, y, 3) };
            (STONES[(k % 5) as usize], 0.55)
        }
        // dark stone: irregular slate flagstones about 8 pixels across, dark joints
        3 => {
            let (cx, cy) = (x.div_euclid(8), y.div_euclid(8));
            let (mut d1, mut d2, mut id) = (f32::MAX, f32::MAX, 0u32);
            for j in -1..=1 {
                for i in -1..=1 {
                    // cells wrap with the texture (8 cells of 8 pixels)
                    let (ci, cj) = ((cx + i).rem_euclid(8) as u32, (cy + j).rem_euclid(8) as u32);
                    let h = hash(ci, cj, 4);
                    let px = ((cx + i) * 8) as f32 + 1.5 + (h & 0xff) as f32 / 255.0 * 5.0;
                    let py = ((cy + j) * 8) as f32 + 1.5 + (h >> 8 & 0xff) as f32 / 255.0 * 5.0;
                    let d = ((x as f32 + 0.5 - px).powi(2) + (y as f32 + 0.5 - py).powi(2)).sqrt();
                    if d < d1 {
                        (d2, d1, id) = (d1, d, h);
                    } else if d < d2 {
                        d2 = d;
                    }
                }
            }
            if d2 - d1 < 1.1 {
                return ([44.0, 46.0, 52.0], 0.2);
            }
            let v = (id >> 16 & 0xff) as f32 / 255.0 * 24.0 - 12.0;
            ([88.0 + v, 92.0 + v, 102.0 + v * 0.8], 0.4)
        }
        // boardwalk: planks across the path, four pixels wide with dark gaps, a little grain along each
        _ => {
            let plank = s.div_euclid(4);
            if s.rem_euclid(4) == 0 {
                return ([56.0, 40.0, 28.0], 0.15);
            }
            const WOOD: [[f32; 3]; 3] = [[150.0, 108.0, 66.0], [136.0, 96.0, 58.0], [162.0, 120.0, 76.0]];
            let h = hash(plank.rem_euclid(16) as u32, 0, 5);
            let grain = 1.0 + 0.08 * (t as f32 * 1.3 + (h & 0xff) as f32 * 0.1).sin();
            (WOOD[(h % 3) as usize].map(|v| v * grain), 0.3)
        }
    }
}

/// A path texture of the theme (`img`, a cross of arms 0.33..0.67 of the tile wide, or a round curve piece when `disc`)
/// drawn in `material` (`Extras::path_material`, 1..4; 0 gives it back as it is).
pub fn style_image(img: &Rgba, material: u8, disc: bool) -> Rgba {
    let mut out = img.clone();
    if material == 0 {
        return out;
    }
    let (w, h) = (img.w as usize, img.h as usize);
    let lum = |p: &[u8]| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
    let (mut sum, mut n) = (0.0f32, 0.0f32);
    for p in img.px.chunks_exact(4).filter(|p| p[3] > 0) {
        sum += lum(p);
        n += 1.0;
    }
    let mean = (sum / n.max(1.0)).max(1.0);
    for y in 0..h {
        for x in 0..w {
            let k = (y * w + x) * 4;
            if img.px[k + 3] == 0 {
                continue;
            }
            let (u, v) = ((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            let mid = |c: f32| (0.33..0.67).contains(&c);
            let along_u = disc || !(mid(u) && !mid(v));
            let (c, keep) = material_at(material, x as i32, y as i32, along_u);
            let d = (lum(&img.px[k..k + 3]) / mean).clamp(0.3, 1.6);
            let shade = 1.0 + (d - 1.0) * keep;
            for i in 0..3 {
                out.px[k + i] = c5(c[i] * shade);
            }
        }
    }
    out
}

impl App {
    /// The theme's path texture `file` in the course's path style (the original's with the extra off or set to the
    /// original paths, as `theme_texture` gives it).
    pub(crate) fn path_texture(&mut self, g: &mut Gfx, file: &str) -> Option<TextureId> {
        let m = self.club.extras.path_material();
        let p = self.game_path(&format!("Data/Textures/{}/{}", crate::app::THEMES[self.theme], file));
        if m == 0 {
            return self.texture_for(g, &p);
        }
        let key = std::path::PathBuf::from(format!("{}#path-style-{m}", p.display()));
        if let Some(t) = self.textures.get(&key) {
            return *t;
        }
        let id = sg_core::fsutil::read_file(&p)
            .and_then(|d| sg_core::assets::decode_tga(&d).ok())
            .map(|img| g.texture(&style_image(&img, m, file.contains("Curve")), true));
        self.textures.insert(key, id);
        id
    }

    /// Tab with the Pathway tool: the course's next path style, named in a toast. False with the extra off.
    pub fn next_path_style(&mut self) -> bool {
        if !self.club.extras.next_path_style() {
            return false;
        }
        self.econ.extras = self.club.extras;
        self.dirty = true;
        self.ui_sound(0x0b);
        let k = sg_core::extras::PATH_STYLE_KEYS[self.club.extras.path_style as usize];
        self.show_toast(&sg_core::lang::trf("STR_TOAST_PATH_STYLE", &[("STYLE", &sg_core::lang::tr(k))]));
        true
    }

    /// Whether a lamp post may stand on tile (a, b): the club's open ground (no water, building, home site, tee or green).
    pub fn lamp_tile_ok(&self, a: i32, b: i32) -> bool {
        use sg_core::land;
        sg_core::course::inside(a, b) && !self.course.oob(a, b) && {
            let ty = self.course.ty[sg_core::course::idx(a, b)];
            ![land::T_WATER, land::T_BUILDING, land::T_HOME_SITE, land::T_TEE, land::T_GREEN].contains(&ty)
        }
    }

    /// The landmark tool with the lamp post chosen: puts one on tile (a, b), where the pointer is on it.
    pub fn place_lamp(&mut self, a: i32, b: i32) {
        if !self.lamp_tile_ok(a, b) {
            self.ui_sound(24);
            return self.show_toast(sg_core::lang::tr("STR_TOAST_LAMP_NOT_HERE"));
        }
        let cost = sg_core::extras::LAMP_UNITS as f64 * sg_core::economy::Economy::UNIT;
        if !self.econ.affordable(cost, self.hole_numbers.len()) {
            return self.show_toast(&sg_core::lang::trf(
                "STR_TOAST_CHANGE_COSTS",
                &[("COST", &crate::ui::money(cost as i64)), ("CASH", &crate::ui::money(self.econ.cash as i64))],
            ));
        }
        // the point under the pointer, kept a little inside the tile
        let k = 1024.0 / TILE_SIZE;
        let (hw, hh) = (self.terrain.w as f32 * TILE_SIZE * 0.5, self.terrain.h as f32 * TILE_SIZE * 0.5);
        let (x, y) = if self.has_hit { (((self.hit_x + hw) * k) as i32, ((self.hit_z + hh) * k) as i32) } else { (0, 0) };
        // (a click from a test hook, the pointer elsewhere: the tile's middle)
        let (x, y) = if (x >> 10, y >> 10) == (a, b) { (x, y) } else { (a * 1024 + 512, b * 1024 + 512) };
        let (x, y) = (x.clamp(a * 1024 + 160, a * 1024 + 864), y.clamp(b * 1024 + 160, b * 1024 + 864));
        if !self.club.add_lamp(x, y) {
            self.ui_sound(24);
            return self.show_toast(sg_core::lang::tr("STR_TOAST_LAMPS_FULL"));
        }
        self.econ.spend_to(sg_core::economy::LEDGER_FACILITIES, cost);
        self.ui_sound(262);
    }

    /// Undo on a tile with a lamp post: it comes off with what it cost back. Returns whether there was one.
    pub fn remove_lamp(&mut self, a: i32, b: i32) -> bool {
        if !self.club.extras.lamps() || !self.club.remove_lamp(a, b) {
            return false;
        }
        let amount = sg_core::extras::LAMP_UNITS as f64 * sg_core::economy::Economy::UNIT;
        self.econ.earn(amount);
        self.econ.book(sg_core::economy::LEDGER_FACILITIES, amount);
        self.ui_sound(262);
        true
    }

    /// The lamp posts standing (world x, z), none while the extra is off.
    pub(crate) fn lamp_spots(&self) -> Vec<(f32, f32)> {
        if !self.club.extras.lamps() {
            return Vec::new();
        }
        self.club.lamps.iter().map(|&(x, y)| self.units_to_world(x, y)).collect()
    }

    /// The lamp posts as (depth key, texture, triangles) to be drawn in the sprites' order: the post and lantern on the
    /// white texture, and with Enhanced graphics from dusk (its time of day) the lantern's halo on the soft `dot`. Classic
    /// graphics light the lantern by the same clock (the light's day of Enhanced graphics, which runs on the game tick),
    /// with no halo.
    pub(crate) fn lamp_quads(&self, white: TextureId, dot: Option<TextureId>) -> Vec<(f32, Option<TextureId>, Vec<Vert>)> {
        let mut out = Vec::new();
        if !self.club.extras.lamps() || self.club.lamps.is_empty() {
            return out;
        }
        let night = if crate::fx::active() { self.fx.atmos.night } else { crate::fx_night::night_at(self.fx_day().1) };
        let m = self.mv;
        let (rx, ry, rz) = (m[0], m[4], m[8]);
        let (ux, uy, uz) = (m[1], m[5], m[9]);
        let s = SPRITE_UNITS_PER_PIXEL;
        for (x, z) in self.lamp_spots() {
            let y = self.terrain.height_at(x, z);
            let key = m[2] * x + m[6] * y + m[10] * z;
            // Enhanced graphics: the time of day, rain and clouds tint it as they tint the sprites
            let tint = if crate::fx::active() { self.fx_sprite_tint(x, z, false) } else { None }.unwrap_or([1.0; 4]);
            let at = |cx: f32, cy: f32, tu: f32, tv: f32, c: [f32; 4]| {
                let (cx, cy) = (cx * s, cy * s);
                Vert::new(x + rx * cx + ux * cy, y + ry * cx + uy * cy, z + rz * cx + uz * cy, tu, tv).col(c)
            };
            let mut v = Vec::new();
            let mut rect = |x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 3], lit: bool| {
                let c = if lit { [c[0], c[1], c[2], 1.0] } else { [c[0] * tint[0], c[1] * tint[1], c[2] * tint[2], 1.0] };
                let (a, b, cc, d) = (at(x0, y1, 0.5, 0.5, c), at(x1, y1, 0.5, 0.5, c), at(x1, y0, 0.5, 0.5, c), at(x0, y0, 0.5, 0.5, c));
                v.extend_from_slice(&[a, b, cc, a, cc, d]);
            };
            // in sprite pixels, up from the ground point: a foot, the post lit from the left, a collar, the lantern
            // between its frame's sides and a cap with a finial
            let dark = [0.13, 0.13, 0.16];
            rect(-2.0, 0.0, 2.0, 1.5, [0.17, 0.17, 0.20], false);
            rect(-1.0, 1.5, 0.0, 20.0, [0.38, 0.38, 0.42], false);
            rect(0.0, 1.5, 1.0, 20.0, [0.15, 0.15, 0.18], false);
            rect(-1.5, 20.0, 1.5, 21.0, dark, false);
            let glass = [0.78 + 0.22 * night, 0.76 + 0.17 * night, 0.62 + 0.04 * night];
            rect(-2.5, 21.0, 2.5, 26.5, glass, night > 0.0);
            rect(-3.0, 21.0, -2.5, 26.5, dark, false);
            rect(2.5, 21.0, 3.0, 26.5, dark, false);
            rect(-0.3, 21.0, 0.3, 26.5, dark, false);
            rect(-3.5, 26.5, 3.5, 27.5, dark, false);
            rect(-2.5, 27.5, 2.5, 28.5, [0.30, 0.30, 0.34], false);
            rect(-0.6, 28.5, 0.6, 30.0, dark, false);
            out.push((key, Some(white), v));
            if let Some(dot) = dot.filter(|_| night > 0.0) {
                let mut h = Vec::new();
                let mut blob = |r: f32, c: [f32; 4]| {
                    let (a, b, cc, d) = (
                        at(-r, 23.75 + r, 0.0, 0.0, c),
                        at(r, 23.75 + r, 1.0, 0.0, c),
                        at(r, 23.75 - r, 1.0, 1.0, c),
                        at(-r, 23.75 - r, 0.0, 1.0, c),
                    );
                    h.extend_from_slice(&[a, b, cc, a, cc, d]);
                };
                blob(13.0, [1.0, 0.76, 0.42, 0.42 * night]);
                blob(4.5, [1.0, 0.95, 0.80, 0.9 * night]);
                out.push((key, Some(dot), h));
            }
        }
        out
    }

    /// The lamp post's picture on the landmark strip, centred on (cx, cy) in 800 x 600 pixels: a post and lantern in the
    /// course's colours, filling the strip button's height.
    pub(crate) fn draw_lamp_icon(&self, g: &mut Gfx, s: &crate::ui::Screen, cx: f32, cy: f32) {
        let k = 1.0;
        let base = cy + 16.0;
        let mut r = |x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 3]| {
            s.fill(g, cx + x0 * k, base - y1 * k, (x1 - x0) * k, (y1 - y0) * k, crate::ui::rgb(c[0], c[1], c[2]));
        };
        let dark = [0.13, 0.13, 0.16];
        r(-3.0, 0.0, 3.0, 2.0, [0.17, 0.17, 0.20]);
        r(-1.5, 2.0, 0.0, 22.0, [0.38, 0.38, 0.42]);
        r(0.0, 2.0, 1.5, 22.0, [0.15, 0.15, 0.18]);
        r(-2.0, 22.0, 2.0, 23.0, dark);
        r(-4.0, 23.0, 4.0, 30.0, dark);
        r(-3.0, 23.0, 3.0, 30.0, [1.0, 0.86, 0.5]);
        r(-0.5, 23.0, 0.5, 30.0, dark);
        r(-5.0, 30.0, 5.0, 31.5, dark);
        r(-3.5, 31.5, 3.5, 33.0, [0.30, 0.30, 0.34]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain cross like the theme's Path.tga: 64 x 64, the arms a third of it wide, a beige with some grain.
    fn cross() -> Rgba {
        let mut img = Rgba::new(64, 64);
        for y in 0..64 {
            for x in 0..64 {
                let on = (21..43).contains(&x) || (21..43).contains(&y);
                if on {
                    let g = (hash(x, y, 9) % 40) as u8;
                    img.px[(y * 64 + x) as usize * 4..][..4].copy_from_slice(&[190 + g / 2, 175 + g / 2, 150 + g / 2, 255]);
                }
            }
        }
        img
    }

    #[test]
    fn the_original_style_is_the_art_itself() {
        let img = cross();
        assert_eq!(style_image(&img, 0, false).px, img.px);
    }

    #[test]
    fn materials_keep_the_shape_and_differ() {
        let img = cross();
        let mut means = Vec::new();
        for m in 1..=4 {
            let out = style_image(&img, m, false);
            assert_eq!((out.w, out.h), (64, 64));
            // the art's outline is kept: the same pixels see-through
            assert!(out.px.chunks_exact(4).zip(img.px.chunks_exact(4)).all(|(a, b)| a[3] == b[3]), "material {m}");
            // 15-bit colours
            assert!(out.px.chunks_exact(4).filter(|p| p[3] > 0).all(|p| (0..3).all(|i| p[i] >> 5 == p[i] & 7)));
            let (mut sum, mut n) = ([0f32; 3], 0.0);
            for p in out.px.chunks_exact(4).filter(|p| p[3] > 0) {
                for i in 0..3 {
                    sum[i] += p[i] as f32;
                }
                n += 1.0;
            }
            means.push(sum.map(|v| v / n));
        }
        // brick is red, dark stone darker than gravel, boardwalk brown
        assert!(means[0][0] > means[0][2] + 40.0);
        assert!(means[2][0] < means[1][0] - 40.0);
        assert!(means[3][0] > means[3][2] + 30.0);
    }

    #[test]
    fn patterns_run_with_the_arm_and_join_tile_to_tile() {
        // the boardwalk's gaps cross the arm: along u on the west-east arm, along v on the north-south arm
        assert_eq!(material_at(4, 8, 30, true).0, [56.0, 40.0, 28.0]);
        assert_ne!(material_at(4, 9, 30, true).0, [56.0, 40.0, 28.0]);
        assert_eq!(material_at(4, 30, 8, false).0, [56.0, 40.0, 28.0]);
        // every pattern repeats over the texture's 64 pixels, so an arm runs on into the next tile's
        for m in 1..=4 {
            for t in 21..43 {
                assert_eq!(material_at(m, 0, t, true), material_at(m, 64, t, true), "material {m}");
                assert_eq!(material_at(m, t, 0, false), material_at(m, t, 64, false), "material {m}");
            }
        }
    }
}
