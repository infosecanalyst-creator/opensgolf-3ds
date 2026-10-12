//! Enhanced graphics' cast shadows (docs/ENHANCED.md, "Cast shadows"): each sprite's own outline, softened more the higher it
//! is above the ground (a penumbra widening away from the base), laid on the ground along the sun: the part of the picture
//! above the sprite's ground point is sheared away from the sun, the part below it (a building's front, a tree's roots) stays
//! where it is on the ground. The art is not edited: the outline is read from the sprite's frame.
//!
//! The shadows go into a mask first (a render target at half the screen's size: white, darkened under each outline, so where
//! two shadows overlap the ground is not darkened twice), which is then multiplied into the frame over the ground and under
//! every sprite.
//!
//! The art already has shadow frames for most buildings, trees and people. While the sun stands where the art was lit
//! (the time of day off), those are kept and only sprites without one cast a shadow; with the time of day on the sun moves,
//! so every sprite casts along it and the art's shadow frames are not drawn (they would point the old way).
use crate::app::{App, Prop};
use crate::gfx::{Gfx, Mode, Uniforms, Vert};
use miniquad::*;
use sg_core::assets::Rgba;
use sg_core::sprites::SPRITE_UNITS_PER_PIXEL;
use std::collections::HashMap;

/// Transparent pixels round each outline, room for the blur.
const PAD: usize = 10;

/// The mask target and the outlines made so far.
#[derive(Default)]
pub struct CastState {
    mask: Option<(TextureId, RenderPass, (u32, u32))>,
    /// (sprite, frame index) -> the softened outline.
    outlines: HashMap<(usize, usize), TextureId>,
}

/// Whether the art's own shadow frames are hidden (the cast shadows replace them while the sun moves).
pub fn hide_art_shadows() -> bool {
    let s = crate::fx::settings();
    s.enabled && s.cast_shadows && s.time_of_day
}

/// A frame's outline (black, its density in alpha), padded and blurred by a radius growing with the height above the
/// sprite's ground point `ay` (rows below it stay sharp-ish).
fn outline(img: &Rgba, ay: i32) -> Rgba {
    let (w, h) = (img.w as usize, img.h as usize);
    let (pw, ph) = (w + 2 * PAD, h + 2 * PAD);
    let mut a = vec![0f32; pw * ph];
    for y in 0..h {
        for x in 0..w {
            a[(y + PAD) * pw + x + PAD] = img.px[(y * w + x) * 4 + 3] as f32 / 255.0;
        }
    }
    let radius = |y: usize| -> usize {
        let up = (ay + PAD as i32 - y as i32).max(0) as f32;
        (0.8 + up * 0.035).min(5.0) as usize
    };
    let mut tmp = vec![0f32; pw * ph];
    for _ in 0..2 {
        // along the rows, then down the columns, each a box of the row's radius (twice: close to a Gaussian)
        for y in 0..ph {
            let r = radius(y);
            let row = &a[y * pw..(y + 1) * pw];
            let mut acc = 0.0;
            let n = (2 * r + 1) as f32;
            for x in 0..=r.min(pw - 1) {
                acc += row[x];
            }
            for x in 0..pw {
                tmp[y * pw + x] = acc / n;
                if x + r + 1 < pw {
                    acc += row[x + r + 1];
                }
                if x >= r {
                    acc -= row[x - r];
                }
            }
        }
        for x in 0..pw {
            for y in 0..ph {
                let r = radius(y) as i32;
                let (y0, y1) = ((y as i32 - r).max(0) as usize, (y as i32 + r).min(ph as i32 - 1) as usize);
                let s: f32 = (y0..=y1).map(|yy| tmp[yy * pw + x]).sum();
                a[y * pw + x] = s / (2 * r + 1) as f32;
            }
        }
    }
    let mut out = Rgba::new(pw as u32, ph as u32);
    for (k, v) in a.iter().enumerate() {
        out.px[k * 4 + 3] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    out
}

impl App {
    /// The softened outline of a sprite's frame.
    fn fx_outline(&mut self, g: &mut Gfx, si: usize, frame_index: usize) -> TextureId {
        if let Some(&t) = self.fx.cast.outlines.get(&(si, frame_index)) {
            return t;
        }
        if self.fx.cast.outlines.len() > 4000 {
            for (_, t) in self.fx.cast.outlines.drain() {
                g.ctx.delete_texture(t);
            }
        }
        let sp = &self.sprites[si].s;
        let img = outline(&sp.frames[frame_index], sp.anchor_y);
        let t = g.texture(&img, false);
        crate::fx::plain_texture(t); // a soft shadow: the pixel-art scaler leaves it alone
        self.fx.cast.outlines.insert((si, frame_index), t);
        t
    }

    /// Draws the cast shadows of the props (`items` in drawing order, `shown` where each is drawn with smooth motion) into
    /// the mask and multiplies it into the frame: called after the ground and the ground overlays, before the sprites.
    pub(crate) fn fx_cast_shadows(&mut self, g: &mut Gfx, u: &Uniforms, items: &[(f32, usize)], shown: &[(f32, f32, f32)]) {
        let s = crate::fx::settings();
        let atm = self.fx.atmos;
        if !s.cast_shadows || atm.shadow < 0.02 || atm.sun_w[1] <= 0.0 {
            return;
        }
        let all = s.time_of_day;
        let mv = self.mv;
        let (rx, rz) = (mv[0], mv[8]); // the screen's right along the ground
        let (ux, uz) = (mv[1], mv[9]);
        // the ground's direction up the screen, scaled so a step of 1 moves 1 up the screen
        let fl = ux.hypot(uz).max(1e-4);
        let (fx, fz) = (ux / fl, uz / fl);
        let fe = (mv[1] * fx + mv[9] * fz).max(1e-3);
        let (gx, gz) = (fx / fe, fz / fe);
        // a point `c` up a sprite stands c / mv[5] above the ground; its shadow falls shadow_len times that height along
        // shadow_dir (fx_light.rs: the art's own shadows at midday, turning and stretching with the sun)
        let reach = atm.shadow_len / mv[5].max(0.1);
        let (sx, sz) = (atm.shadow_dir[0] * reach, atm.shadow_dir[1] * reach);
        let lift_k = atm.shadow_len;
        let quarter = ((self.view_rot() / 90.0).round() as i32 % 4 + 4) % 4;
        let px = self.upp * (self.draw_w / 800.0).min(self.draw_h / 600.0);
        // what to cast: on screen (with a margin for the shadow's length), standing (not ground overlays), not in water
        let mut quads: Vec<(TextureId, [Vert; 8])> = Vec::new();
        for &(_, i) in items {
            let p: Prop = self.props[i].clone();
            let Some(si) = p.body else { continue };
            if p.flat || p.hidden || p.bed.is_some() || p.weed || (!all && p.shadow.is_some()) {
                continue;
            }
            let (x, z, lift) = shown.get(i).copied().unwrap_or((p.x, p.z, p.lift));
            let (tx, ty) = self.terrain.tile_of(x, z);
            if self.terrain.type_at(tx, ty) == sg_core::course::t::WATER as i32 {
                continue;
            }
            let gy = self.terrain.height_at(x, z);
            let m = &u.mvp;
            let cw = m[3] * x + m[7] * gy + m[11] * z + m[15];
            let (cx, cy) = ((m[0] * x + m[4] * gy + m[8] * z + m[12]) / cw, (m[1] * x + m[5] * gy + m[9] * z + m[13]) / cw);
            if cx.abs() > 2.2 || cy.abs() > 2.2 {
                continue;
            }
            let view = self.prop_view(&p, si, quarter);
            let fi = self.sprites[si].s.frame_index(view, p.frame);
            let tex = self.fx_outline(g, si, fi);
            let sp = &self.sprites[si].s;
            let (w, h, ax, ay) = (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32);
            let k = SPRITE_UNITS_PER_PIXEL * p.scale;
            let pad = PAD as f32;
            // the ground point, with the exe's screen offset (right along the ground, up as ground further back) and a
            // flier's height carried away from the sun
            let (ox, oy) = (p.shift.0 * px, -p.shift.1 * px);
            let bx = x + rx * ox + gx * oy + atm.shadow_dir[0] * lift_k * lift;
            let bz = z + rz * ox + gz * oy + atm.shadow_dir[1] * lift_k * lift;
            let y = gy + 0.5;
            let alpha = if atm.cloud > 0.0 { self.fx_cloud_sun(x, z) } else { 1.0 };
            let col = [1.0, 1.0, 1.0, alpha];
            let (tw, th) = (w + 2.0 * pad, h + 2.0 * pad);
            let at = |px_x: f32, px_y: f32| {
                let c = (px_x - ax) * k;
                let up = (ay - px_y) * k;
                let (dx, dz) = if up > 0.0 { (sx * up, sz * up) } else { (gx * up, gz * up) };
                Vert::new(bx + rx * c + dx, y, bz + rz * c + dz, (px_x + pad) / tw, (px_y + pad) / th).col(col)
            };
            let (l0, r0, t0, b0) = (-pad, w + pad, -pad, h + pad);
            let mid = ay.clamp(t0, b0);
            quads.push((tex, [at(l0, t0), at(r0, t0), at(r0, mid), at(l0, mid), at(l0, mid), at(r0, mid), at(r0, b0), at(l0, b0)]));
        }
        if quads.is_empty() {
            return;
        }
        // into the mask
        g.flush();
        let back = self.fx.pass_now;
        g.ctx.end_render_pass();
        let div = if s.quality >= 2 { 1.0 } else { 2.0 };
        let size = ((self.draw_w / div).ceil().max(1.0) as u32, (self.draw_h / div).ceil().max(1.0) as u32);
        if self.fx.cast.mask.is_some_and(|m| m.2 != size) {
            if let Some((t, p, _)) = self.fx.cast.mask.take() {
                g.ctx.delete_render_pass(p);
                g.ctx.delete_texture(t);
            }
        }
        let (mask, pass) = match self.fx.cast.mask {
            Some((t, p, _)) => (t, p),
            None => {
                let t = g.ctx.new_render_texture(TextureParams {
                    width: size.0,
                    height: size.1,
                    format: TextureFormat::RGBA8,
                    ..Default::default()
                });
                let p = g.ctx.new_render_pass(t, None);
                self.fx.cast.mask = Some((t, p, size));
                (t, p)
            }
        };
        g.ctx.begin_pass(Some(pass), PassAction::clear_color(1.0, 1.0, 1.0, 1.0));
        for (tex, v) in &quads {
            g.quad(Mode::Flat, Some(*tex), u, [v[0], v[1], v[2], v[3]]);
            g.quad(Mode::Flat, Some(*tex), u, [v[4], v[5], v[6], v[7]]);
        }
        g.flush();
        g.ctx.end_render_pass();
        // multiplied into the course: a cool shade, as strong as the sun is
        g.ctx.begin_pass(back, PassAction::Nothing);
        let f = *crate::fx::gpu(g);
        let k = 0.5 * atm.shadow;
        let tint = [1.0 - k * 0.98, 1.0 - k * 0.92, 1.0 - k * 0.78, 1.0];
        g.ctx.apply_pipeline(&f.shade);
        g.ctx.apply_bindings(&Bindings { vertex_buffers: vec![f.quad_vb], index_buffer: f.quad_ib, images: vec![mask] });
        g.ctx.apply_uniforms(UniformsSource::table(&tint));
        g.ctx.draw(0, 6, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlines_are_padded_and_soft() {
        let mut img = Rgba::new(4, 20);
        for p in img.px.chunks_exact_mut(4) {
            p[3] = 255;
        }
        let o = outline(&img, 19);
        assert_eq!((o.w, o.h), (4 + 2 * PAD as u32, 20 + 2 * PAD as u32));
        // inside the base it stays dense, beside the top it spreads further than beside the base
        let a = |x: usize, y: usize| o.px[(y * o.w as usize + x) * 4 + 3];
        assert!(a(PAD + 2, PAD + 18) > 200);
        assert!(a(PAD - 1, PAD + 1) > a(PAD - 1, PAD + 18));
    }
}
