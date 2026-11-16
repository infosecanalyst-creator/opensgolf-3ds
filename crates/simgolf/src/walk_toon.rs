//! The drawn look for the walk camera's models: flat bands of light and a dark outline, as a cartoon is inked and
//! coloured, so the 3D golfers and buildings sit with the game's hand-drawn pictures rather than looking like plastic.
//!
//! The bands are worked out on the CPU, a whole triangle at once, from its facing to the sun: lit, half lit or in shade.
//! The outline is the model again, a little fatter (each point pushed out along its normal), in a dark shade of its own
//! colour, and drawn pushed back from the eye (`WalkUniforms::pull` below zero) by more than the model is deep: inside the
//! model's outline the model is nearer and covers it, so only the rim round the outside shows.

use crate::gfx::Vert;

/// The bands: lit, half lit, in shade (multiplying the colour).
const LIT: f32 = 1.1;
const HALF: f32 = 0.9;
const SHADE: f32 = 0.74;

/// The band of light for a facing `d` to the sun (-1..1): lit, half lit or in shade, with a narrow soft step between
/// them so a rounded body's bands have clean, smooth edges rather than stair steps along its triangles.
fn band(d: f32) -> f32 {
    let step = |e: f32| ((d - e) / 0.08 + 0.5).clamp(0.0, 1.0);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    SHADE + (HALF - SHADE) * s(step(0.05)) + (LIT - HALF) * s(step(0.45))
}

/// Each point in its band of light by its facing to `sun` (unit, towards it): a smooth model's normals give smooth
/// bands, a flat-sided one's (each face's own normal) a band a face. The normals are cleared so the shader takes the
/// colour as it is.
pub fn cel(v: &mut [Vert], sun: [f32; 3]) {
    for t in v.chunks_exact_mut(3) {
        // a point without a normal takes its triangle's
        let tn = [0, 1, 2].map(|k| t[0].normal[k] + t[1].normal[k] + t[2].normal[k]);
        for p in t.iter_mut() {
            let n = if p.normal == [0.0; 3] { tn } else { p.normal };
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            let k = if l < 1e-4 { HALF } else { band((n[0] * sun[0] + n[1] * sun[1] + n[2] * sun[2]) / l) };
            for c in &mut p.color[..3] {
                *c = (*c * k).min(1.0);
            }
            p.normal = [0.0; 3];
        }
    }
}

/// The finish for a figure seen close (the golfers): a soft two-tone ramp from shade to light rather than hard bands, a
/// little sky light from above, a bright rim where the body turns away from `eye` and a small soft gloss towards the sun,
/// as a polished cartoon is lit. Like `cel` it bakes the light into the colours and clears the normals.
pub fn soft(v: &mut [Vert], sun: [f32; 3], eye: [f32; 3]) {
    let unit = |a: [f32; 3]| {
        let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-6);
        [a[0] / l, a[1] / l, a[2] / l]
    };
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let s = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    for t in v.chunks_exact_mut(3) {
        let tn = [0, 1, 2].map(|k| t[0].normal[k] + t[1].normal[k] + t[2].normal[k]);
        for p in t.iter_mut() {
            let raw = if p.normal == [0.0; 3] { tn } else { p.normal };
            if dot(raw, raw) < 1e-8 {
                for c in &mut p.color[..3] {
                    *c *= HALF;
                }
                continue;
            }
            let n = unit(raw);
            let to_eye = unit([eye[0] - p.pos[0], eye[1] - p.pos[1], eye[2] - p.pos[2]]);
            let d = dot(n, sun);
            let k = SOFT_SHADE + (SOFT_LIT - SOFT_SHADE) * s((d + 0.2) / 0.55) + 0.07 * n[1];
            let rim = (1.0 - dot(n, to_eye).max(0.0)).powi(3) * (0.18 + 0.22 * d.max(0.0));
            let gloss = dot(n, unit([sun[0] + to_eye[0], sun[1] + to_eye[1], sun[2] + to_eye[2]])).max(0.0).powi(24) * 0.14;
            for c in &mut p.color[..3] {
                let lit = *c * k;
                *c = (lit + (1.0 - lit).max(0.0) * rim + gloss).min(1.0);
            }
            p.normal = [0.0; 3];
        }
    }
}

/// The two ends of `soft`'s ramp.
const SOFT_LIT: f32 = 1.08;
const SOFT_SHADE: f32 = 0.7;

/// The outline shell of `v`: each point `width` out along its normal, in a dark shade of its own colour, unlit.
pub fn outline(v: &[Vert], width: f32, out: &mut Vec<Vert>) {
    out.reserve(v.len());
    for p in v {
        let mut q = *p;
        let n = p.normal;
        q.pos = [p.pos[0] + n[0] * width, p.pos[1] + n[1] * width, p.pos[2] + n[2] * width];
        // a dark shade of the colour itself, as a painter's line is, not black ink
        q.color = [p.color[0] * 0.42 + 0.02, p.color[1] * 0.4 + 0.02, p.color[2] * 0.44 + 0.03, 1.0];
        q.normal = [0.0; 3];
        q.uv = [0.0; 2];
        out.push(q);
    }
}

/// How thick an outline is at `dist` world units from the eye: about a pixel on the screen, never thinner
/// than `least`.
pub fn width(dist: f32, least: f32) -> f32 {
    (dist * 0.0016).max(least)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri(n: [f32; 3]) -> Vec<Vert> {
        (0..3)
            .map(|k| {
                let mut v = Vert::new(k as f32, 0.0, 0.0, 0.0, 0.0).col([0.5, 0.5, 0.5, 1.0]);
                v.normal = n;
                v
            })
            .collect()
    }

    #[test]
    fn a_face_to_the_sun_is_lit_and_one_away_in_shade() {
        let sun = [0.0, 1.0, 0.0];
        let (mut up, mut down) = (tri([0.0, 1.0, 0.0]), tri([0.0, -1.0, 0.0]));
        cel(&mut up, sun);
        cel(&mut down, sun);
        assert!(up[0].color[0] > 0.5 && down[0].color[0] < 0.5);
        assert_eq!(up[0].normal, [0.0; 3]);
    }

    #[test]
    fn the_outline_stands_out_along_the_normals_and_is_dark() {
        let mut o = Vec::new();
        outline(&tri([0.0, 0.0, 1.0]), 0.1, &mut o);
        assert!((o[0].pos[2] - 0.1).abs() < 1e-6);
        assert!(o[0].color[0] < 0.3);
    }
}
