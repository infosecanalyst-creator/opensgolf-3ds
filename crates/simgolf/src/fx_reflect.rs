//! Enhanced graphics' water reflections (docs/ENHANCED.md, "Water reflections"): trees, buildings, golfers and whatever else
//! stands within a few tiles of water are drawn mirrored (flipped about the water's surface, swaying with the wind,
//! fading towards their tops) into a render target before the ground; the water shader samples it at its own pixel,
//! moved by the waves' slope, so the reflections ripple and only show on the water (not on the bank art). The sky is
//! mirrored by the water shader itself (fx_glsl.rs). The same option makes the sun's glitter stronger and warmer at golden
//! hour and draws the water lapping at the shore.
//!
//! The mirror image of an upright object is the same picture flipped on the screen about the reflection of its ground
//! point in the water's plane, so each sprite is drawn as it is, flipped, at that point (no art is edited).
use crate::app::App;
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use miniquad::*;
use sg_core::sprites::SPRITE_UNITS_PER_PIXEL;

/// How far from water (tiles) a sprite may stand and still be mirrored in it.
const REACH: i32 = 4;

impl App {
    /// For every tile, the height of the water within REACH tiles of it (the nearest water tile's), or None.
    pub(crate) fn fx_water_near(&mut self) {
        let t = &self.terrain;
        let (w, h) = (t.w.max(0) as usize, t.h.max(0) as usize);
        let water = sg_core::course::t::WATER as i32;
        let mut near: Vec<Option<(i32, f32)>> = vec![None; w * h];
        let mut queue = std::collections::VecDeque::new();
        for y in 0..h {
            for x in 0..w {
                if t.type_at(x as i32, y as i32) == water {
                    let (cx, cz) = t.tile_centre(x as i32, y as i32);
                    near[y * w + x] = Some((0, t.height_at(cx, cz)));
                    queue.push_back((x, y));
                }
            }
        }
        while let Some((x, y)) = queue.pop_front() {
            let Some((d, lvl)) = near[y * w + x] else { continue };
            if d >= REACH {
                continue;
            }
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let k = ny as usize * w + nx as usize;
                if near[k].is_none() {
                    near[k] = Some((d + 1, lvl));
                    queue.push_back((nx as usize, ny as usize));
                }
            }
        }
        // a tile with water of two levels within reach: a stream stepping down a hill since "Fix original bugs" gives water its
        // level (docs/FIXES.md 1). The reflections go into one screen-space picture that every water pixel samples, so a
        // sprite mirrored about the upper water's surface showed as a dark wedge on the water below, over the waterfall's
        // spray; such tiles mirror nothing
        let mut mixed = vec![false; w * h];
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let Some((_, lvl)) = near[y as usize * w + x as usize] else { continue };
                'scan: for dy in -REACH..=REACH {
                    for dx in -REACH..=REACH {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 || t.type_at(nx, ny) != water {
                            continue;
                        }
                        if let Some((0, l)) = near[ny as usize * w + nx as usize] {
                            if (l - lvl).abs() > 1.0 {
                                mixed[y as usize * w + x as usize] = true;
                                break 'scan;
                            }
                        }
                    }
                }
            }
        }
        self.fx.water_mixed = mixed;
        self.fx.water_near = near.into_iter().map(|n| n.map(|v| v.1)).collect();
        self.fx.water_near_key = (w * 1000 + h) as u64;
    }

    /// Draws the reflections into their target (made or resized as needed) and returns it; the frame's own pass is
    /// resumed afterwards. None when nothing stands near the water on the screen.
    pub(crate) fn fx_reflections(&mut self, g: &mut Gfx, proj: &Mat4, mv: &Mat4) -> Option<TextureId> {
        if !self.show_props || self.props.is_empty() {
            return None;
        }
        let tw = self.terrain.w.max(0) as usize;
        if self.fx.water_near.len() != tw * self.terrain.h.max(0) as usize {
            self.fx_water_near();
        }
        let m = mv.0;
        let mvp = proj.mul(mv).0;
        let (rx, ry, rz) = (m[0], m[4], m[8]);
        let (ux, uy, uz) = (m[1], m[5], m[9]);
        let smooth = crate::enhanced::smooth_motion();
        let water = sg_core::course::t::WATER as i32;
        // what is mirrored: standing sprites near water, not in it (the water's own sprites), on or just off the screen
        let mut items: Vec<(f32, usize, f32, f32, f32, f32)> = Vec::new();
        for (i, p) in self.props.iter().enumerate() {
            if p.hidden || p.flat || p.bed.is_some() || p.weed || p.body.is_none() {
                continue;
            }
            let (x, z, lift) = if smooth { self.shown_at(p) } else { (p.x, p.z, p.lift) };
            let (tx, ty) = self.terrain.tile_of(x, z);
            if !self.terrain.inside(tx, ty) || self.terrain.type_at(tx, ty) == water {
                continue;
            }
            let Some(level) = self.fx.water_near.get(ty as usize * tw + tx as usize).copied().flatten() else { continue };
            if self.fx.water_mixed.get(ty as usize * tw + tx as usize).copied().unwrap_or(false) {
                continue;
            }
            let gy = self.terrain.height_at(x, z) + lift;
            if gy < level - 1.0 {
                continue;
            }
            let my = 2.0 * level - gy;
            let cw = mvp[3] * x + mvp[7] * my + mvp[11] * z + mvp[15];
            let (cx, cy) = ((mvp[0] * x + mvp[4] * my + mvp[8] * z + mvp[12]) / cw, (mvp[1] * x + mvp[5] * my + mvp[9] * z + mvp[13]) / cw);
            if cx.abs() > 1.6 || !(-2.5..=1.3).contains(&cy) {
                continue;
            }
            items.push((m[2] * x + m[6] * gy + m[10] * z, i, x, z, lift, level));
        }
        if items.is_empty() {
            return None;
        }
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        // the target: half the frame's size (low quality: a third, high: full)
        let s = crate::fx::settings();
        let div = [3.0, 2.0, 1.0][s.quality.min(2) as usize];
        let size = ((self.draw_w / div).ceil().max(1.0) as u32, (self.draw_h / div).ceil().max(1.0) as u32);
        if self.fx.reflect.is_some_and(|r| r.2 != size) {
            if let Some((t, p, _)) = self.fx.reflect.take() {
                g.ctx.delete_render_pass(p);
                g.ctx.delete_texture(t);
            }
        }
        let (tex, pass) = match self.fx.reflect {
            Some((t, p, _)) => (t, p),
            None => {
                let t = g.ctx.new_render_texture(TextureParams {
                    width: size.0,
                    height: size.1,
                    format: TextureFormat::RGBA8,
                    ..Default::default()
                });
                let p = g.ctx.new_render_pass(t, None);
                self.fx.reflect = Some((t, p, size));
                (t, p)
            }
        };
        g.flush();
        let back = self.fx.pass_now;
        g.ctx.end_render_pass();
        g.ctx.begin_pass(Some(pass), PassAction::clear_color(0.0, 0.0, 0.0, 0.0));
        // blurred by the ripples anyway: the pixel-art scaler is not worth its cost here
        crate::fx::pause_scaler(true);
        let u = Uniforms::flat(proj, mv);
        let quarter = ((self.view_rot() / 90.0).round() as i32 % 4 + 4) % 4;
        let vscale = (self.draw_w / 800.0).min(self.draw_h / 600.0);
        let upp = 2.0 / (proj.0[0] * self.draw_w).max(1e-6);
        let px = upp * vscale;
        for &(_, i, x, z, lift, level) in &items {
            let p = self.props[i].clone();
            let Some(si) = p.body else { continue };
            let view = self.prop_view(&p, si, quarter);
            let fi = self.sprites[si].s.frame_index(view, p.frame);
            let tex = match p.outfit {
                Some(o) => self.outfit_texture(g, si, view, p.frame, o).unwrap_or_else(|| self.sprite_texture(g, si, view, p.frame)),
                None => self.sprite_texture(g, si, view, p.frame),
            };
            let tex = self.fx_season_texture(g, si, fi, tex).unwrap_or(tex);
            let bend = self.fx_sway(si, fi, x, z).unwrap_or(crate::fx_nature::Bend { lean: 0.0, curve: 1.0, flip: false });
            let tint = self.fx_sprite_tint(x, z, false).unwrap_or([1.0; 4]);
            let sp = &self.sprites[si].s;
            let (w, h, ax, ay) = (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32);
            let k = SPRITE_UNITS_PER_PIXEL * p.scale;
            let (l, r, t, b) = (-ax * k, (w - ax) * k, ay * k, -(h - ay) * k);
            // the ground point mirrored in the water's plane, with the exe's screen offset (flipped up and down)
            let gy = self.terrain.height_at(x, z) + lift;
            let (ox, oy) = (p.shift.0 * px, -p.shift.1 * px);
            let (bx, by, bz) = (x + rx * ox - ux * oy, 2.0 * level - gy + ry * ox - uy * oy, z + rz * ox - uz * oy);
            for q in crate::fx_nature::strips(l, r, t, b, bend) {
                let v = q.map(|(cx, cy, tu, tv)| {
                    // fainter towards the top, a little darker than the thing itself
                    let a = 0.95 * (1.0 - 0.3 * (cy / t.max(1e-3)).clamp(0.0, 1.0));
                    let c = [tint[0] * 0.95, tint[1] * 0.95, tint[2] * 0.97, a];
                    Vert::new(bx + rx * cx - ux * cy, by + ry * cx - uy * cy, bz + rz * cx - uz * cy, tu, tv).col(c)
                });
                g.quad(Mode::Flat, Some(tex), &u, v);
            }
        }
        g.flush();
        crate::fx::pause_scaler(false);
        g.ctx.end_render_pass();
        g.ctx.begin_pass(back, PassAction::Nothing);
        Some(tex)
    }
}
