//! The Enhanced graphics effects' shaders (fx.rs). All are GLSL ES 1.00 (OpenGL 2, GLES 2 and WebGL 1): no derivatives, no
//! texture LOD in vertex shaders, no dynamic loops; highp in fragment shaders only where the GPU has it.

/// Fragment shader preamble: highp when the GPU has it in fragment shaders (texel arithmetic on large textures needs it).
const FRAG_HEAD: &str = "#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
";

// ---- the pixel-art scaler (sprites and interface pictures) ------------------------------------------------------------------

pub const SCALER_VS: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec4 in_color;
uniform mat4 mvp;
varying lowp vec4 color;
varying highp vec2 uv;
void main() {
    gl_Position = mvp * vec4(in_pos, 1.0);
    uv = in_uv;
    color = in_color;
}
"#;

/// The scaler's fragment shader. `sp` = (texture width, height, output pixels per texel, mode: 1 sharp bilinear, 2 edge
/// directed); `soft` the width in texels of the blend between texels (at least one output pixel). The edge-directed mode is an xBR-style filter (levels 1 and 2: 45 degree and the shallow / steep 30 degree
/// edges) evaluated for all four corners of the texel from its 5 x 5 neighbourhood, with the edges anti-aliased over one output
/// pixel, on top of a sharp-bilinear base. Colours are compared and mixed premultiplied, so a colour-keyed (transparent) texel
/// counts as one colour whatever its hidden RGB, and the art's transparent edges get smooth outlines without fringes.
pub fn scaler_fs() -> String {
    let mut s = String::from(FRAG_HEAD);
    s.push_str(
        r#"varying lowp vec4 color;
varying highp vec2 uv;
uniform sampler2D tex;
uniform vec4 sp;
uniform float alpha_ref;
uniform float soft;
uniform float pm;

// a texel, premultiplied (textures made by Enhanced's smooth interface option are premultiplied already: pm 1; pm 2: a
// colour-keyed picture with the distance to its smoothed outline in alpha (edges.rs), taken here as the original key)
vec4 P(vec4 c) {
    if (pm > 1.5) {
        float k = step(0.5, c.a);
        return vec4(c.rgb * k, k);
    }
    return pm > 0.5 ? c : vec4(c.rgb * c.a, c.a);
}
vec4 T(vec2 cell) { return P(texture2D(tex, (cell + 0.5) / sp.xy)); }
float df(vec4 a, vec4 b) {
    vec4 d = abs(a - b);
    return dot(d, vec4(0.299, 0.587, 0.114, 0.9));
}
bool eq(vec4 a, vec4 b) { return df(a, b) < 0.07; }
float cov(float x, float w) { return clamp(x / w + 0.5, 0.0, 1.0); }

// One corner of the texel, named as xBR names the bottom right one: F to the right, H below, I diagonal, B above, D left,
// C up right, G down left, F4 two right, I4 right of I, H5 two down, I5 below I. u, v: the output pixel inside the texel
// towards this corner (-0.5..0.5); w: one output pixel in texels.
vec4 corner(vec4 col, vec4 E, vec4 F, vec4 H, vec4 I, vec4 B, vec4 D, vec4 C, vec4 G, vec4 F4, vec4 I4, vec4 H5, vec4 I5,
            float u, float v, float w) {
    if (eq(E, F) || eq(E, H)) return col;
    float e = df(E, C) + df(E, G) + df(I, F4) + df(I, H5) + 4.0 * df(H, F);
    float i = df(H, D) + df(H, I5) + df(F, I4) + df(F, B) + 4.0 * df(E, I);
    bool keep = (!eq(F, B) && !eq(H, D)) || (eq(E, I) && !eq(F, I4) && !eq(H, I5)) || eq(E, G) || eq(E, C);
    if (!(e < i) || !keep) return col;
    vec4 px = df(E, F) <= df(E, H) ? F : H;
    float a = cov(u + v - 0.5, w * 1.4142);
    float kf = df(F, G);
    float kh = df(H, C);
    if (2.0 * kf <= kh && !eq(E, G) && !eq(D, G)) a = max(a, cov(u + 2.0 * v - 0.5, w * 2.236));
    if (kf >= 2.0 * kh && !eq(E, C) && !eq(B, C)) a = max(a, cov(2.0 * u + v - 0.5, w * 2.236));
    return mix(col, px, a);
}

void main() {
    vec2 p = uv * sp.xy;
    vec2 cell = floor(p);
    vec2 f = p - cell - 0.5;
    // sharp bilinear: texel centres flat, a linear ramp at the texel edges one output pixel wide, or `soft` texels when that
    // is wider
    float rw = max(1.0 / sp.z, soft);
    float region = 0.5 - 0.5 * rw;
    vec2 g = (f - clamp(f, -region, region)) / rw + 0.5;
    vec4 c = P(texture2D(tex, (cell + g) / sp.xy));
    if (sp.w > 1.5) {
        float w = rw;
"#,
    );
    // the 5 x 5 neighbourhood without its corners, n<x><y> for offsets x, y in -2..2 written 0..4
    for y in 0..5i32 {
        for x in 0..5i32 {
            if (x == 0 || x == 4) && (y == 0 || y == 4) {
                continue;
            }
            s.push_str(&format!("        vec4 n{x}{y} = T(cell + vec2({}.0, {}.0));\n", x - 2, y - 2));
        }
    }
    for (dx, dy) in [(1i32, 1i32), (-1, 1), (1, -1), (-1, -1)] {
        let n = |a: i32, b: i32| format!("n{}{}", 2 + a * dx, 2 + b * dy);
        let args = [(0, 0), (1, 0), (0, 1), (1, 1), (0, -1), (-1, 0), (1, -1), (-1, 1), (2, 0), (2, 1), (0, 2), (1, 2)]
            .iter()
            .map(|&(a, b)| n(a, b))
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!("        c = corner(c, {args}, f.x * {dx}.0, f.y * {dy}.0, w);\n"));
    }
    s.push_str(
        r#"    }
    // straight colours out for the classic blending; premultiplied ones (and the vertex colour premultiplied) for pm
    vec4 o = pm > 0.5 ? c : (c.a > 0.0 ? vec4(c.rgb / c.a, c.a) : vec4(0.0));
    if (pm > 1.5) {
        // the outline from the distance field, one output pixel wide (edges.rs: 8 texels over alpha 0..1); the colour the
        // scaler found, else (just outside the keyed texels) the colour under them
        vec4 t = texture2D(tex, uv);
        float a = clamp((t.a - 0.5) * 8.0 * sp.z + 0.5, 0.0, 1.0);
        vec3 rgb = c.a > 0.004 ? c.rgb / c.a : t.rgb;
        o = vec4(rgb * a, a);
    }
    o *= color;
    if (o.a <= alpha_ref) discard;
    gl_FragColor = o;
}
"#,
    );
    s
}

// ---- the ground: terrain, walls, path overlays and water --------------------------------------------------------------------

/// Shared by the terrain and the water (the uniforms both stages declare are mediump in both, as GLSL ES requires equal
/// precision): the classic per-vertex light (gfx.rs) split into its ambient and direct parts, the
/// eye-space normal for per-pixel light, the world position and the light map's coordinates.
pub const GROUND_VS: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec3 in_normal;
attribute vec4 in_color;
uniform mat4 mvp;
uniform mat4 mv;
uniform mediump vec3 light_amb;
uniform mediump vec3 light_dif;
uniform mediump vec3 light_spec;
uniform mediump vec3 light_dir;
uniform vec4 lm_rect;
varying lowp vec4 color;
varying highp vec2 uv;
varying highp vec2 lm_uv;
varying highp vec3 w_pos;
varying mediump vec3 n_eye;
varying mediump vec3 v_dir;
varying mediump float up_n;
void main() {
    gl_Position = mvp * vec4(in_pos, 1.0);
    uv = in_uv;
    color = in_color;
    w_pos = in_pos;
    up_n = in_normal.y;
    lm_uv = (in_pos.xz - lm_rect.xy) * lm_rect.zw;
    vec3 n = (mv * vec4(in_normal, 0.0)).xyz;
    n_eye = n;
    float d = dot(n, light_dir);
    v_dir = vec3(0.0);
    if (d > 0.0) {
        vec3 h = normalize(light_dir + vec3(0.0, 0.0, 1.0));
        v_dir = 0.8 * light_dif * d + light_spec * pow(max(dot(n, h), 0.0), 13.0);
    }
}
"#;

/// Uniforms after the vertex shader's: `lmap` the light map (r sun, g ambient occlusion, b signed distance to the shore, a
/// water); params = (per-pixel light 0/1, shadow strength, occlusion strength, alpha test); water = (time, reflection, sparkle,
/// foam); eye_w and light_w are towards the viewer and the light in world space; sky colours for the reflections.
/// The atmosphere (fx_light.rs): `clouds` a tiling noise; clouds = (shadow strength, 0: none; texture repeats per world unit;
/// drift x, z in world units); cover = (the noise's cloud edge from, to, 0, 0); mist = (density, 0: none; the low ground's
/// height; 1 / its depth; time) and mist_col its colour. cover.zw is one pixel of the target (for gl_FragCoord).
/// The newer effects: `fxmap` the detail map (fx_wet.rs), `reflt` the reflections (fx_reflect.rs), fx1..fx3 their settings
/// (see terrain_fs and water_fs); all are 0 when those effects are off, and then nothing they do runs.
const GROUND_FS_HEAD: &str = r#"varying lowp vec4 color;
varying highp vec2 uv;
varying highp vec2 lm_uv;
varying highp vec3 w_pos;
varying mediump vec3 n_eye;
varying mediump vec3 v_dir;
varying mediump float up_n;
uniform sampler2D tex;
uniform sampler2D lmap;
uniform mediump vec3 light_amb;
uniform mediump vec3 light_dif;
uniform mediump vec3 light_spec;
uniform mediump vec3 light_dir;
uniform vec4 params;
uniform vec4 water;
uniform vec3 eye_w;
uniform vec3 light_w;
uniform vec3 sky_top;
uniform vec3 sky_low;
uniform sampler2D clouds;
uniform vec4 cloud;
uniform vec4 cover;
uniform vec4 mist;
uniform vec3 mist_col;
uniform sampler2D fxmap;
uniform sampler2D reflt;
uniform vec4 fx1;
uniform vec4 fx2;
uniform vec4 fx3;

float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = p - i;
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}

// how much of the sun the drifting clouds let through
float cloud_sun() {
    if (cloud.x <= 0.0) return 1.0;
    float n = texture2D(clouds, (w_pos.xz + cloud.zw) * cloud.y).r;
    return 1.0 - cloud.x * smoothstep(cover.x, cover.y, n);
}

// mist lying in low ground, in hollows (the occlusion) and over water, drifting
vec3 misty(vec3 c, vec4 lm) {
    if (mist.x <= 0.0) return c;
    float low = clamp((mist.y - w_pos.y) * mist.z + 0.5, 0.0, 1.0);
    float n = texture2D(clouds, w_pos.xz * 0.00045 + vec2(mist.w * 0.0021, mist.w * 0.0013)).r;
    float d = mist.x * clamp(0.08 + 0.3 * low + 1.2 * (1.0 - lm.g) + 0.25 * lm.a, 0.0, 1.0) * (0.35 + 0.9 * n);
    return mix(c, mist_col, clamp(d, 0.0, 0.55));
}

// the decoded light, per pixel or as the vertex computed it, with the sun's visibility and the occlusion applied
vec3 light(vec4 lm) {
    float sun = mix(1.0, lm.r, params.y) * cloud_sun();
    float ao = mix(1.0, lm.g, params.z);
    vec3 dir = v_dir;
    if (params.x > 0.5 && fx3.y > 0.5 && up_n > 0.5) {
        // the ground's smooth normal from the detail map (world space), instead of the tile's flat face
        vec2 q = texture2D(fxmap, lm_uv).ba * 2.0 - 1.0;
        vec3 nw = vec3(q.x, sqrt(max(1.0 - dot(q, q), 0.0)), q.y);
        float d = dot(nw, light_w);
        dir = vec3(0.0);
        if (d > 0.0) {
            vec3 h = normalize(light_w + eye_w);
            dir = 0.8 * light_dif * d + light_spec * pow(max(dot(nw, h), 0.0), 13.0);
        }
    } else if (params.x > 0.5) {
        // the classic normals are not normalised (the cliff's are longer than 1): keep their length, restore what the
        // interpolation lost
        float len = length(n_eye);
        vec3 n = n_eye / max(len, 0.0001) * max(len, 1.0);
        float d = dot(n, light_dir);
        dir = vec3(0.0);
        if (d > 0.0) {
            vec3 h = normalize(light_dir + vec3(0.0, 0.0, 1.0));
            dir = 0.8 * light_dif * d + light_spec * pow(max(dot(n, h), 0.0), 13.0);
        }
    }
    return min((vec3(0.04) + 0.2 * light_amb + dir * sun) * ao, vec3(1.0));
}
"#;

/// The terrain. The seasons, frost and dew and the wet ground (fx_nature.rs, fx_wet.rs) run only when their uniforms are
/// above 0: fx1 = (wetness, the puddles' level: potential above it is a puddle, rain, time); fx2 = (spring freshness, autumn,
/// frost, dew); fx3.w the sparkle's cells per world unit; `fxmap` r the puddle potential, g grass.
pub fn terrain_fs() -> String {
    format!(
        "{FRAG_HEAD}{GROUND_FS_HEAD}{}",
        r#"// the grass through the year: fresher in spring, a little golden in autumn (green pixels of grass ground only)
vec3 seasonal(vec3 c, float grass) {
    float gm = grass * clamp((c.g - max(c.r, c.b)) * 8.0, 0.0, 1.0) * smoothstep(0.6, 0.85, up_n);
    float l = dot(c, vec3(0.299, 0.587, 0.114));
    vec3 s = mix(c, c * vec3(1.04, 1.09, 0.90), fx2.x);
    s = mix(s, vec3(l * 1.22, l * 1.10, l * 0.60), fx2.y * 0.32);
    return mix(c, s, gm);
}
// early mornings: frost in patches (more in hollows) and dew or rime glinting in the sun
vec3 frosty(vec3 c, float grass, vec4 lm) {
    // level ground only (not the cliffs and walls)
    float gm = grass * (1.0 - lm.a) * smoothstep(0.6, 0.85, up_n);
    float patchy = noise(w_pos.xz * 0.012) * 0.6 + noise(w_pos.xz * 0.05) * 0.4 + 0.3 * (1.0 - lm.g);
    float f = fx2.z * gm * smoothstep(0.12, 0.8, patchy);
    float l = dot(c, vec3(0.299, 0.587, 0.114));
    c = mix(c, vec3(0.84, 0.88, 0.92) * (0.55 + 0.6 * l), f * 0.42);
    float h = hash(floor(vec2(w_pos.x + w_pos.y * 1.37, w_pos.z - w_pos.y * 0.71) * fx3.w));
    float tw = 0.5 + 0.5 * sin(fx1.w * 2.6 + h * 40.0);
    float sp = step(0.991, h) * tw * min(fx2.w + fx2.z, 1.0) * gm * cloud_sun();
    return c + vec3(0.92, 0.96, 1.0) * sp * 0.6;
}
// wet ground: darker and a little glossy, with puddles mirroring the sky (rain rings in them while it rains)
vec3 wetted(vec3 c, float pot, vec4 lm) {
    float land = 1.0 - lm.a;
    float wet = fx1.x * land;
    land *= smoothstep(0.6, 0.85, up_n);
    // the map's potential, its edge broken up by a finer noise so the puddles' outlines are round and irregular
    pot += (noise(w_pos.xz * 0.09) - 0.5) * 0.10 + (noise(w_pos.xz * 0.23) - 0.5) * 0.04;
    float pud = smoothstep(fx1.y, fx1.y + 0.035, pot) * land;
    vec3 n = vec3((noise(w_pos.xz * 0.05) - 0.5) * 0.08, 1.0, (noise(w_pos.xz * 0.05 + 9.0) - 0.5) * 0.08);
    if (fx1.z > 0.0 && pud > 0.0) {
        vec2 q = w_pos.xz * 0.06;
        vec2 cell = floor(q);
        vec2 f = q - cell;
        float hc = hash(cell);
        vec2 ctr = vec2(hash(cell + 7.1), hash(cell + 3.3)) * 0.6 + 0.2;
        float ph = fract(fx1.w * 0.9 + hc);
        vec2 dv = f - ctr;
        float d = length(dv);
        float ring = (1.0 - smoothstep(0.0, 0.05, abs(d - ph * 0.35))) * (1.0 - ph);
        n.xz += dv / max(d, 0.001) * ring * 0.4 * fx1.z;
    }
    n = normalize(n);
    vec3 r = reflect(-eye_w, n);
    vec3 sky = mix(sky_low, sky_top, clamp((r.y - 0.55) * 2.0, 0.0, 1.0));
    float fres = clamp(1.0 - dot(n, eye_w), 0.0, 1.0);
    c *= 1.0 - 0.24 * wet;
    c = mix(c, sky, wet * (0.05 + 0.12 * fres));
    float rim = smoothstep(fx1.y - 0.08, fx1.y, pot) * (1.0 - pud) * land;
    c *= 1.0 - 0.18 * rim * min(fx1.x * 2.0, 1.0);
    vec3 pc = mix(c * 0.42, sky * 0.78, 0.42 + 0.3 * fres);
    vec3 hv = normalize(light_w + eye_w);
    pc += light_spec * pow(max(dot(n, hv), 0.0), 60.0) * 0.8 * cloud_sun();
    return mix(c, pc, pud);
}
void main() {
    vec4 lm = texture2D(lmap, lm_uv);
    vec4 c = texture2D(tex, uv) * color;
    if (c.a <= params.w) discard;
    vec4 dm = vec4(0.0);
    if (fx1.x + fx2.x + fx2.y + fx2.z + fx2.w > 0.0) {
        dm = texture2D(fxmap, lm_uv);
        if (fx2.x + fx2.y > 0.0) c.rgb = seasonal(c.rgb, dm.g);
    }
    c.rgb *= light(lm);
    if (fx2.z + fx2.w > 0.0) c.rgb = frosty(c.rgb, dm.g, lm);
    if (fx1.x > 0.0) c.rgb = wetted(c.rgb, dm.r, lm);
    c.rgb = misty(c.rgb, lm);
    gl_FragColor = c;
}
"#
    )
}

pub fn water_fs() -> String {
    format!(
        "{FRAG_HEAD}{GROUND_FS_HEAD}{}",
        r#"// the slope of the surface (world units; a tile is 100): a few travelling waves, their crests bent by noise so they do not
// line up, and finer noise ripples
vec2 waves(vec2 p, float t) {
    vec2 q = p + (vec2(noise(p * 0.011 + t * 0.05), noise(p * 0.011 + 31.0 - t * 0.04)) - 0.5) * 90.0;
    vec2 g = vec2(0.0);
    vec2 d1 = vec2(0.80, 0.60); g += d1 * cos(dot(d1, q) * 0.047 + t * 1.1) * 0.055;
    vec2 d2 = vec2(-0.45, 0.89); g += d2 * cos(dot(d2, q) * 0.071 + t * 1.6) * 0.045;
    vec2 d3 = vec2(0.97, -0.24); g += d3 * cos(dot(d3, q) * 0.113 - t * 2.1) * 0.035;
    vec2 d4 = vec2(-0.70, -0.71); g += d4 * cos(dot(d4, q) * 0.191 + t * 2.7) * 0.025;
    vec2 r = p * 0.09;
    g += (vec2(noise(r + vec2(t * 0.35, 0.0)), noise(r + vec2(13.0, -t * 0.3))) - 0.5) * 0.10;
    g += (vec2(noise(r * 2.7 - t * 0.5), noise(r * 2.7 + 5.0 + t * 0.45)) - 0.5) * 0.06;
    return g;
}
void main() {
    vec4 lm = texture2D(lmap, lm_uv);
    vec4 tc = texture2D(tex, uv);
    vec4 c = tc * color;
    c.rgb *= light(lm);
    // where the picture is the water itself and not its bank (rocks, grass, sand drawn into the edge pieces): close to the
    // texture's own middle colour, read from a small mip level
    vec3 ref = texture2D(tex, vec2(0.5, 0.5), 5.0).rgb;
    float wet = 1.0 - smoothstep(0.10, 0.26, length(tc.rgb - ref));
    float sd = (lm.b - 0.5) * 2.0; // tiles into the water from the shore
    float t = water.x;
    vec2 g = waves(w_pos.xz, t);
    vec3 n = normalize(vec3(-g.x, 1.0, -g.y));
    vec3 r = reflect(-eye_w, n);
    // calm water reflects the pale sky low down; only slopes turned up catch the deeper colour above
    vec3 sky = mix(sky_low, sky_top, clamp((r.y - 0.62) * 2.5, 0.0, 1.0));
    float fres = clamp(1.0 - dot(n, eye_w), 0.0, 1.0);
    float refl = water.y * (0.55 + 0.9 * fres * fres);
    float sun = mix(1.0, lm.r, params.y) * cloud_sun();
    c.rgb = mix(c.rgb, sky, refl * wet);
    // what stands round the water mirrored in it (fx_reflect.rs: drawn flipped into `reflt`), rippled by the waves
    if (fx3.x > 0.0) {
        vec4 rf = texture2D(reflt, gl_FragCoord.xy * cover.zw + g * vec2(0.012, 0.008));
        float k = fx3.x * wet;
        c.rgb = c.rgb * (1.0 - rf.a * k) + rf.rgb * k;
    }
    // sparkle: the decoded light's highlight. The sun is never behind the water in this view, so a mirror highlight would not
    // show; instead the facets that the waves turn towards the half vector (more than calm water is) glint, broken up by noise
    vec3 h = normalize(light_w + eye_w);
    float lift = dot(n, h) - h.y;
    float sheen = smoothstep(0.0, 0.10, lift);
    float glint = smoothstep(0.70, 0.95, noise(w_pos.xz * 0.45 + vec2(t * 1.3, -t * 0.9)))
                * smoothstep(0.60, 0.90, noise(w_pos.xz * 0.21 - vec2(t * 0.4, t * 0.6)));
    c.rgb += (light_spec * 0.5 + 0.5) * (0.10 * sheen * min(water.z, 1.0) + 0.9 * glint * sheen * water.z) * sun * wet;
    // foam along the shore: bands drifting in, broken by noise
    float band = 1.0 - smoothstep(0.03, 0.28, sd);
    float wash = 0.5 + 0.5 * sin(sd * 38.0 - t * 1.6 + noise(w_pos.xz * 0.05) * 6.0);
    float foam = band * smoothstep(0.45, 0.95, wash * (0.6 + 0.6 * noise(w_pos.xz * 0.12 + t * 0.2)));
    float wetfoam = 1.0 - smoothstep(0.14, 0.40, length(tc.rgb - ref));
    c.rgb = mix(c.rgb, vec3(0.94, 0.97, 1.0) * light(lm), foam * water.w * wetfoam * step(0.0, sd));
    // the water lapping at the shore: a thin line running up and back, the wet sheen behind it
    if (fx3.z > 0.0) {
        float lw = 0.06 + 0.045 * sin(t * 1.25 + noise(w_pos.xz * 0.02) * 6.28);
        float on = step(0.0, sd) * wetfoam;
        float lap = (1.0 - smoothstep(0.0, 0.04, abs(sd - lw))) * on;
        c.rgb = mix(c.rgb, vec3(0.95, 0.98, 1.0) * light(lm), lap * fx3.z * 0.6);
        c.rgb += (1.0 - smoothstep(0.0, lw, sd)) * on * 0.07 * fx3.z;
    }
    c.rgb = misty(c.rgb, lm);
    gl_FragColor = c;
}
"#
    )
}

// ---- post-processing ------------------------------------------------------------------------------------------------------

/// A full-screen pass: in_pos is in clip space, in_uv the source's coordinates.
pub const POST_VS: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
varying highp vec2 uv;
void main() {
    gl_Position = vec4(in_pos, 1.0);
    uv = in_uv;
}
"#;

/// Downsampling: a 4 x 4 box from four bilinear taps; `p.z` > 0.5 keeps only what is brighter than the threshold p.w (the
/// bloom's bright pass). p.xy is one source texel.
pub fn down_fs() -> String {
    format!(
        "{FRAG_HEAD}{}",
        r#"varying highp vec2 uv;
uniform sampler2D src;
uniform vec4 p;
void main() {
    vec2 o = p.xy;
    vec3 c = (texture2D(src, uv + vec2(-o.x, -o.y)).rgb + texture2D(src, uv + vec2(o.x, -o.y)).rgb
            + texture2D(src, uv + vec2(-o.x, o.y)).rgb + texture2D(src, uv + vec2(o.x, o.y)).rgb) * 0.25;
    if (p.z > 0.5) {
        float l = max(c.r, max(c.g, c.b));
        c *= smoothstep(p.w, p.w + 0.25, l);
    }
    gl_FragColor = vec4(c, 1.0);
}
"#
    )
}

/// A separable 9-tap Gaussian in five bilinear fetches along p.xy (one texel in the blur's direction).
pub fn blur_fs() -> String {
    format!(
        "{FRAG_HEAD}{}",
        r#"varying highp vec2 uv;
uniform sampler2D src;
uniform vec4 p;
void main() {
    vec2 d = p.xy;
    vec3 c = texture2D(src, uv).rgb * 0.2270270;
    c += (texture2D(src, uv + d * 1.3846154).rgb + texture2D(src, uv - d * 1.3846154).rgb) * 0.3162162;
    c += (texture2D(src, uv + d * 3.2307692).rgb + texture2D(src, uv - d * 3.2307692).rgb) * 0.0702703;
    gl_FragColor = vec4(c, 1.0);
}
"#
    )
}

/// The composite: the course, the tilt-shift blur by screen height (tilt = strength, focus centre, focus half height, 0),
/// the bloom (glow.x), the colour grade (grade = contrast, vibrance, 0, 0; tint multiplies) and the vignette (glow.y; glow.z
/// the aspect ratio).
pub fn composite_fs() -> String {
    format!(
        "{FRAG_HEAD}{}",
        r#"varying highp vec2 uv;
uniform sampler2D scene;
uniform sampler2D blurred;
uniform sampler2D bloom;
uniform vec4 tilt;
uniform vec4 glow;
uniform vec4 grade;
uniform vec4 tint;
void main() {
    vec3 c = texture2D(scene, uv).rgb;
    if (tilt.x > 0.0) {
        // uv.y runs up the screen in a render target
        float k = smoothstep(tilt.z, tilt.z + 0.28, abs(uv.y - tilt.y)) * tilt.x;
        c = mix(c, texture2D(blurred, uv).rgb, k);
    }
    if (glow.x > 0.0) c += texture2D(bloom, uv).rgb * glow.x;
    if (grade.x > 0.0) {
        c = (c - 0.5) * grade.x + 0.5;
        float l = dot(c, vec3(0.299, 0.587, 0.114));
        float sat = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
        c = mix(vec3(l), c, 1.0 + grade.y * (1.0 - sat));
        c *= tint.rgb;
    }
    if (glow.y > 0.0) {
        vec2 q = (uv - 0.5) * vec2(glow.z, 1.0);
        c *= 1.0 - glow.y * smoothstep(0.35, 1.05, length(q));
    }
    gl_FragColor = vec4(clamp(c, 0.0, 1.0), 1.0);
}
"#
    )
}

/// The cast shadows' composite's full-screen quad, at the far end of the depth range: drawn with the depth test "greater",
/// it shades only where the ground was drawn (not the sky or the black beyond the map).
pub const SHADE_VS: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
varying highp vec2 uv;
void main() {
    gl_Position = vec4(in_pos.xy, 0.9999, 1.0);
    uv = in_uv;
}
"#;

/// The cast shadows' composite (fx_shadow.rs): the frame multiplied by the shadow mask (`mask` holds the light left: white
/// where nothing casts, darker under the silhouettes), the shade tinted by `tint` (the colour of full shadow).
pub fn shade_fs() -> String {
    format!(
        "{FRAG_HEAD}{}",
        r#"varying highp vec2 uv;
uniform sampler2D mask;
uniform vec4 tint;
void main() {
    float m = texture2D(mask, uv).r;
    gl_FragColor = vec4(mix(tint.rgb, vec3(1.0), m), 1.0);
}
"#
    )
}
