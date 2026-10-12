//! Enhanced graphics' strongest pixel-art scaler, *Smooth* (docs/ENHANCED.md, "Pixel-art scaling"): an edge-directed
//! reconstruction for magnified sprites and interface pictures that smooths the stair steps inside the art as well as its
//! outline, at any (non-integer) magnification.
//!
//! The game's art is pre-rendered and already anti-aliased (soft shading, lines and rings drawn with in-between colours),
//! so rules that look for runs of equal colours (xBR, the *edge-directed* mode) rarely fire on it. This filter instead
//! measures the direction of the art's edges and interpolates along them:
//!
//! 1. the 4 x 4 texels around the output pixel are read once (16 fetches, premultiplied, so a colour-keyed transparent
//!    texel is one colour whatever RGB it hides);
//! 2. their brightness gradients (one for each of the nine 2 x 2 blocks, weighted by a Gaussian around the pixel, so the
//!    direction changes smoothly from pixel to pixel) give a structure tensor: its main axis is the edge's normal, the gap
//!    between its two eigenvalues how clearly there is one edge (a corner, a dot or noise has none) and the larger one how
//!    strong it is;
//! 3. the texels are weighted by an anisotropic Gaussian, long along the edge and narrow across it (steering kernel
//!    regression): a stair-stepped circle, a shallow curve or a one-texel line becomes a smooth line, at any angle;
//! 4. across the edge the result is sharpened without overshoot: the same kernel half a texel to either side gives the two
//!    colours of the edge, and the pixel's place between them is steepened to a transition about one screen pixel wide.
//!    Where the three are not in order (the middle of a thin line), lie on a straight ramp (soft shading) or differ little
//!    (the steps of quantised shading) nothing changes;
//! 5. the result is mixed with the softened sharp-bilinear base (the edge-directed mode's) by the edge measure of 2, so
//!    flat areas, soft shading, texture noise and single-texel details stay as the art drew them instead of turning
//!    plastic.
//!
//! GLSL ES 1.00 (WebGL 1): no texelFetch, no loops (the taps are written out here), exponents clamped so mediump does not
//! underflow. Uniforms and colour handling (straight, premultiplied, or a colour-keyed picture with the distance to its
//! smoothed outline in alpha, edges.rs) are those of the other scaler modes (fx_glsl::scaler_fs).

/// Fragment shader preamble (as fx_glsl's): highp where the GPU has it, texel arithmetic on large textures needs it.
const FRAG_HEAD: &str = "#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
";

/// Kernel widths in texels: across a clear edge, along it, and where there is no edge (before the mix with the base).
pub const SIGMA_ACROSS: f32 = 0.35;
pub const SIGMA_ALONG: f32 = 1.2;
pub const SIGMA_ROUND: f32 = 0.5;
/// The two edge colours are taken this far (texels) to either side of the pixel.
pub const SIDE: f32 = 0.5;
/// The sharpened edge's width in screen pixels.
pub const EDGE_PIXELS: f32 = 1.0;
/// The edge strength (gradient of brightness + alpha / 2, per texel) where smoothing starts and where it is full.
pub const STRENGTH_LO: f32 = 0.02;
pub const STRENGTH_HI: f32 = 0.12;
/// The difference between the edge's two colours (premultiplied RGBA, Euclidean) below which it is not sharpened (soft
/// shading, the steps of the art's quantised gradients: sharpened they turn into contour bands) and above which it is
/// fully.
pub const SHARPEN_LO: f32 = 0.08;
pub const SHARPEN_HI: f32 = 0.2;
/// The width (texels) of the Gaussian that weights the nine block gradients of the structure tensor.
pub const TENSOR_SIGMA: f32 = 0.8;
/// The coherence ((l1 - l2) / (l1 + l2) of the structure tensor) below which there is no one edge (corners, dots, the noise
/// of textured art): smoothing starts above it and is full at 1.
pub const COHERENCE_LO: f32 = 0.3;

fn f(v: f32) -> String {
    format!("{v:.4}")
}

/// The Smooth scaler's fragment shader. Uniforms as fx_glsl::scaler_fs: `sp` = (texture width, height, screen pixels per
/// texel, mode), `pm` how the texture holds its colours, `soft` the width in texels of the base's blend between texels.
pub fn smooth_fs() -> String {
    let mut s = String::from(FRAG_HEAD);
    s.push_str(
        r#"varying lowp vec4 color;
varying highp vec2 uv;
uniform sampler2D tex;
uniform vec4 sp;
uniform float alpha_ref;
uniform float soft;
uniform float pm;

// a texel, premultiplied (pm 1: premultiplied already; pm 2: colour-keyed with the distance to its smoothed outline in
// alpha, edges.rs, taken here as the original key)
vec4 P(vec4 c) {
    if (pm > 1.5) {
        float k = step(0.5, c.a);
        return vec4(c.rgb * k, k);
    }
    return pm > 0.5 ? c : vec4(c.rgb * c.a, c.a);
}
float lum(vec4 c) { return dot(c.rgb, vec3(0.299, 0.587, 0.114)) + 0.5 * c.a; }
float ex(float e) { return exp(-min(e, 9.0)); }

void main() {
    vec2 p = uv * sp.xy;
    // the sharp-bilinear base, softened over `soft` texels (as the edge-directed mode's)
    vec2 cell = floor(p);
    vec2 fc = p - cell - 0.5;
    float rw = max(1.0 / sp.z, soft);
    float region = 0.5 - 0.5 * rw;
    vec2 g = (fc - clamp(fc, -region, region)) / rw + 0.5;
    vec4 c = P(texture2D(tex, (cell + g) / sp.xy));
    // the 4 x 4 texels around p: t<i><j> at offsets i - 1, j - 1 from the texel left of and above p
    vec2 q = p - 0.5;
    vec2 o = floor(q);
    vec2 fr = q - o;
"#,
    );
    for j in 0..4 {
        for i in 0..4 {
            s.push_str(&format!(
                "    vec4 t{i}{j} = P(texture2D(tex, (o + vec2({:.1}, {:.1})) / sp.xy)); float l{i}{j} = lum(t{i}{j});\n",
                i as f32 - 0.5,
                j as f32 - 0.5
            ));
        }
    }
    // the structure tensor (a, b, d) from the gradients of the nine 2 x 2 blocks of the neighbourhood, weighted by a
    // Gaussian of their centres' distance to p (nine samples: a steadier direction than fewer, and noise has none)
    s.push_str("    float ta = 0.0; float tb = 0.0; float td = 0.0; float tw = 0.0; float gx; float gy; float bw; vec2 bc;\n");
    for j in 0..3 {
        for i in 0..3 {
            s.push_str(&format!(
                "    gx = 0.5 * (l{i1}{j} + l{i1}{j1} - l{i}{j} - l{i}{j1}); gy = 0.5 * (l{i}{j1} + l{i1}{j1} - l{i}{j} - l{i1}{j});\n",
                i1 = i + 1,
                j1 = j + 1
            ));
            s.push_str(&format!(
                "    bc = vec2({:.1}, {:.1}) - fr; bw = exp(-dot(bc, bc) * {}); tw += bw; ta += bw * gx * gx; tb += bw * gx * gy; td += bw * gy * gy;\n",
                i as f32 - 0.5,
                j as f32 - 0.5,
                f(0.5 / (TENSOR_SIGMA * TENSOR_SIGMA))
            ));
        }
    }
    s.push_str("    ta /= tw; tb /= tw; td /= tw;\n");
    s.push_str(&format!(
        r#"    float tr = ta + td;
    float det = sqrt((ta - td) * (ta - td) + 4.0 * tb * tb);
    float coh = tr > 1e-6 ? det / tr : 0.0;
    float kc = clamp((coh - {clo}) / {cspan}, 0.0, 1.0);
    float k = kc * kc * clamp((sqrt(0.5 * (tr + det)) - {lo}) / {span}, 0.0, 1.0);
    if (k > 0.004) {{
        float ang = 0.5 * atan(2.0 * tb, ta - td);
        vec2 n = vec2(cos(ang), sin(ang));
        float st = mix({round}, {along}, k);
        float sn = mix({round}, {across}, k);
        float it = 0.5 / (st * st);
        float inn = 0.5 / (sn * sn);
        vec4 s0 = vec4(0.0); vec4 sm = vec4(0.0); vec4 sq = vec4(0.0);
        float w0 = 0.0; float wm = 0.0; float wq = 0.0;
        vec2 d; float dn; float dt; float w;
"#,
        lo = f(STRENGTH_LO),
        clo = f(COHERENCE_LO),
        cspan = f(1.0 - COHERENCE_LO),
        span = f(STRENGTH_HI - STRENGTH_LO),
        round = f(SIGMA_ROUND),
        along = f(SIGMA_ALONG),
        across = f(SIGMA_ACROSS),
    ));
    let h = f(SIDE);
    for j in 0..4 {
        for i in 0..4 {
            s.push_str(&format!(
                "        d = vec2({}.0, {}.0) - fr; dn = dot(d, n); dt = d.y * n.x - d.x * n.y; dt = dt * dt * it;\n",
                i - 1,
                j - 1
            ));
            s.push_str(&format!(
                "        w = ex(dt + dn * dn * inn); s0 += w * t{i}{j}; w0 += w;\n        w = ex(dt + (dn + {h}) * (dn + {h}) * inn); sm += w * t{i}{j}; wm += w;\n        w = ex(dt + (dn - {h}) * (dn - {h}) * inn); sq += w * t{i}{j}; wq += w;\n"
            ));
        }
    }
    s.push_str(&format!(
        r#"        s0 /= w0; sm /= wm; sq /= wq;
        // sharpen across the edge between its two colours, without overshoot; not in the middle of a thin line (the
        // pixel is not between them) nor on a straight ramp (the pixel half way: unchanged)
        vec4 e = sq - sm;
        float ee = dot(e, e);
        if (ee > 1e-6) {{
            float t = dot(s0 - sm, e) / ee;
            float gain = max(1.0, {two_h} * sp.z / {px});
            float t2 = clamp((t - 0.5) * gain + 0.5, 0.0, 1.0);
            vec4 cv = s0 - 0.5 * (sq + sm);
            float m = k * ee / (ee + 8.0 * dot(cv, cv)) * clamp((sqrt(ee) - {elo}) / {espan}, 0.0, 1.0);
            s0 = mix(s0, sm + e * t2, m);
        }}
        c = mix(c, s0, clamp(2.0 * k, 0.0, 1.0));
    }}
    // straight colours out for the classic blending; premultiplied ones (and the vertex colour premultiplied) for pm
    vec4 o2 = pm > 0.5 ? c : (c.a > 0.0 ? vec4(c.rgb / c.a, c.a) : vec4(0.0));
    if (pm > 1.5) {{
        // the outline from the distance field, one output pixel wide (edges.rs: 8 texels over alpha 0..1); the colour
        // found here, else (just outside the keyed texels) the colour under them
        vec4 tt = texture2D(tex, uv);
        float a = clamp((tt.a - 0.5) * 8.0 * sp.z + 0.5, 0.0, 1.0);
        vec3 rgb = c.a > 0.004 ? c.rgb / c.a : tt.rgb;
        o2 = vec4(rgb * a, a);
    }}
    o2 *= color;
    if (o2.a <= alpha_ref) discard;
    gl_FragColor = o2;
}}
"#,
        two_h = f(2.0 * SIDE),
        elo = f(SHARPEN_LO),
        espan = f(SHARPEN_HI - SHARPEN_LO),
        px = f(EDGE_PIXELS),
    ));
    s
}

#[cfg(test)]
mod tests {
    #[test]
    fn shader_reads_sixteen_texels_once() {
        let s = super::smooth_fs();
        assert_eq!(s.matches("texture2D(tex, (o + vec2(").count(), 16);
        assert_eq!(s.matches("s0 += w * t").count(), 16);
        assert!(!s.contains("texelFetch") && !s.contains("for ("));
    }

    #[test]
    fn settings_choose_the_filter() {
        use crate::enhanced::Settings;
        use crate::fx::{from_enhanced, SCALER_EDGE, SCALER_SHARP, SCALER_SMOOTH};
        assert_eq!(from_enhanced(&Settings::DEFAULT).scaler, SCALER_SMOOTH);
        // a settings file from before the choice: its sharp-bilinear tick still counts, its default is the new default
        let old: Settings = serde_json::from_str(r#"{"enabled": true, "fx_scaler_sharp": true}"#).unwrap();
        assert_eq!(from_enhanced(&old).scaler, SCALER_SHARP);
        let old: Settings = serde_json::from_str(r#"{"enabled": true, "fx_scaler_sharp": false}"#).unwrap();
        assert_eq!(from_enhanced(&old).scaler, SCALER_SMOOTH);
        let mut s = old;
        let o = crate::enhanced::OPTIONS.iter().find(|o| o.key == "fx_scaler_mode").unwrap();
        (o.set)(&mut s, 1);
        assert_eq!(from_enhanced(&s).scaler, SCALER_EDGE);
    }
}
