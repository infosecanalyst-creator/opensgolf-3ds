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

// a texel, premultiplied (textures made by Enhanced's smooth interface option are premultiplied already: pm 1)
vec4 P(vec4 c) { return pm > 0.5 ? c : vec4(c.rgb * c.a, c.a); }
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
void main() {
    gl_Position = mvp * vec4(in_pos, 1.0);
    uv = in_uv;
    color = in_color;
    w_pos = in_pos;
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
const GROUND_FS_HEAD: &str = r#"varying lowp vec4 color;
varying highp vec2 uv;
varying highp vec2 lm_uv;
varying highp vec3 w_pos;
varying mediump vec3 n_eye;
varying mediump vec3 v_dir;
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

// the decoded light, per pixel or as the vertex computed it, with the sun's visibility and the occlusion applied
vec3 light(vec4 lm) {
    float sun = mix(1.0, lm.r, params.y);
    float ao = mix(1.0, lm.g, params.z);
    vec3 dir = v_dir;
    if (params.x > 0.5) {
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

pub fn terrain_fs() -> String {
    format!(
        "{FRAG_HEAD}{GROUND_FS_HEAD}{}",
        r#"void main() {
    vec4 lm = texture2D(lmap, lm_uv);
    vec4 c = texture2D(tex, uv) * color;
    if (c.a <= params.w) discard;
    c.rgb *= light(lm);
    gl_FragColor = c;
}
"#
    )
}

pub fn water_fs() -> String {
    format!(
        "{FRAG_HEAD}{GROUND_FS_HEAD}{}",
        r#"float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = p - i;
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}
// the slope of the surface (world units; a tile is 100): a few travelling waves, their crests bent by noise so they do not
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
    float sun = mix(1.0, lm.r, params.y);
    c.rgb = mix(c.rgb, sky, refl * wet);
    // sparkle: the decoded light's highlight. The sun is never behind the water in this view, so a mirror highlight would not
    // show; instead the facets that the waves turn towards the half vector (more than calm water is) glint, broken up by noise
    vec3 h = normalize(light_w + eye_w);
    float lift = dot(n, h) - h.y;
    float sheen = smoothstep(0.0, 0.10, lift);
    float glint = smoothstep(0.70, 0.95, noise(w_pos.xz * 0.45 + vec2(t * 1.3, -t * 0.9)))
                * smoothstep(0.60, 0.90, noise(w_pos.xz * 0.21 - vec2(t * 0.4, t * 0.6)));
    c.rgb += (light_spec * 0.5 + 0.5) * (0.10 * sheen + 0.9 * glint * sheen) * water.z * sun * wet;
    // foam along the shore: bands drifting in, broken by noise
    float band = 1.0 - smoothstep(0.03, 0.28, sd);
    float wash = 0.5 + 0.5 * sin(sd * 38.0 - t * 1.6 + noise(w_pos.xz * 0.05) * 6.0);
    float foam = band * smoothstep(0.45, 0.95, wash * (0.6 + 0.6 * noise(w_pos.xz * 0.12 + t * 0.2)));
    float wetfoam = 1.0 - smoothstep(0.14, 0.40, length(tc.rgb - ref));
    c.rgb = mix(c.rgb, vec3(0.94, 0.97, 1.0) * light(lm), foam * water.w * wetfoam * step(0.0, sd));
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
