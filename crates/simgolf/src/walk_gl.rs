//! The walk camera's own drawing (walk.rs): its shaders, the textures it makes for itself and the stream of vertices it
//! draws each frame. Nothing here is made until the walk camera is first shown, so nothing changes without its switch.
//!
//! One program draws the course: the ground with the player's terrain art and a detail layer made here (`detail_image`),
//! the water, the objects' shapes with their own sprites projected onto them, the sprites stood up as cards, flat colours
//! and the shadows, all in world coordinates and hazed with distance. A second one draws the sky. Both are GLSL ES 1.00,
//! so they build wherever the game's own shaders do.
use crate::gfx::{bleed_alpha, Gfx, Mesh, Vert};
use miniquad::*;
use sg_core::assets::Rgba;

/// What a draw is, for the fragment shader (WalkUniforms::sun.w).
pub mod mode {
    /// Terrain art with the detail layer; the category in the vertex colour's red (`cat`).
    pub const GROUND: f32 = 0.0;
    /// A sprite's texels, cut out where transparent; lit by the normal when it has one.
    pub const CUTOUT: f32 = 1.0;
    /// The water: the art's colour under moving ripples, the sky's reflection and the sun's glints.
    pub const WATER: f32 = 3.0;
    /// The vertex colour, lit by the normal.
    pub const FLAT: f32 = 4.0;
    /// A picture laid on the ground (a building's base, an object's shadow): its texels times the vertex colour, blended.
    pub const DECAL: f32 = 5.0;
    /// A soft round shadow: the vertex colour's alpha fading out from the texture coordinates' centre.
    pub const BLOB: f32 = 6.0;
    /// Terrain art laid over the ground or on its walls (paths, cliffs): its texels times the vertex colour, lit, with the
    /// detail of category `WalkUniforms::info[1]`.
    pub const ART: f32 = 7.0;
}

/// The ground's categories, for its detail layer (vertex colour alpha = category / 32, plus a half on and beside water;
/// the colour itself is the category's average colour, which the near ground is evened towards).
pub mod cat {
    pub const GRASS: u8 = 0;
    pub const FAIRWAY: u8 = 1;
    pub const GREEN: u8 = 2;
    pub const ROUGH: u8 = 3;
    pub const SAND: u8 = 4;
    pub const DESERT: u8 = 5;
    pub const TEE: u8 = 6;
    pub const WOODS: u8 = 7;
    pub const ROCK: u8 = 8;
    pub const PATH: u8 = 9;
    pub const WATER: u8 = 10;
}

/// The vertex colour that carries a ground category (its colour filled in by the ground's builder).
pub fn cat_colour(c: u8) -> [f32; 4] {
    [0.5, 0.5, 0.5, c as f32 / 32.0]
}

/// The category back from a vertex colour's alpha, and whether it is on or beside water.
pub fn cat_of(a: f32) -> (u8, bool) {
    let shore = a >= 0.5;
    (((a - if shore { 0.5 } else { 0.0 }) * 32.0).round() as u8, shore)
}

const FRAG_HEAD: &str = "#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
";

pub const WALK_VS: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec3 in_normal;
attribute vec4 in_color;
uniform mat4 mvp;
uniform vec4 pull;
varying highp vec3 wpos;
varying highp vec2 uv;
varying mediump vec3 nrm;
varying mediump vec4 color;
void main() {
    // a decal is drawn a little towards the eye, so it stays over the ground it lies on at any distance
    vec3 p = in_pos + (pull.xyz - in_pos) * pull.w;
    gl_Position = mvp * vec4(p, 1.0);
    wpos = in_pos;
    uv = in_uv;
    nrm = in_normal;
    color = in_color;
}
"#;

/// The course's fragment shader. One world unit is a 25 yard tile's hundredth, so a metre is about 4.37 of them (`M`).
pub fn walk_fs() -> String {
    let mut s = String::from(FRAG_HEAD);
    s.push_str(
        r#"varying highp vec3 wpos;
varying highp vec2 uv;
varying mediump vec3 nrm;
varying mediump vec4 color;
uniform sampler2D tex;
uniform sampler2D detail;
uniform sampler2D noise;
uniform sampler2D clean;
uniform vec4 eye;
uniform vec4 sun;
uniform vec4 haze;
uniform vec4 zenith;
uniform vec4 amb;
uniform vec4 dif;
uniform vec4 info;
uniform vec4 water;
const float M = 4.3745;

vec3 fogged(vec3 c, float dist, vec3 v) {
    float s = pow(max(dot(v, sun.xyz), 0.0), 6.0);
    vec3 fc = mix(haze.rgb, vec3(1.0, 0.93, 0.80), s * amb.w);
    float f = 1.0 - exp(-pow(dist * haze.w, 1.5));
    return mix(c, fc, clamp(f, 0.0, 1.0));
}

// the terrain lighting of the overhead view (gfx.rs), in world space: OpenGL's default material under one light
vec3 lit(vec3 c, vec3 n) {
    float d = dot(normalize(n), sun.xyz);
    vec3 s = vec3(0.04) + 0.2 * amb.rgb + 0.8 * dif.rgb * max(d, 0.0);
    return c * min(s, vec3(1.0));
}

// the art's colour, its painted features faded out near the eye: there each texel covers many pixels and a painted bush or
// stone smears into a blotch, so the near ground takes the art's de-featured copy (walk_ground.rs clean_image: its
// colours and the edges between kinds of ground, without the small painted things) and the detail layer draws the grain
vec4 art(vec2 t, float dist, float keep, float even, vec3 avg) {
    vec4 c = texture2D(tex, t);
    float near = 1.0 - smoothstep(12.0 * M, 90.0 * M, dist);
    // (far off the art alone: the near ground's work is left out)
    if (near <= 0.0) return c;
    // and its large darker and lighter zones evened out towards the average colour of its kind of ground (the same for
    // every tile of the kind, so no tile's edge shows)
    vec4 soft = mix(texture2D(clean, t), vec4(avg, 1.0), even);
    return mix(c, mix(soft, c, keep), near);
}

// the detail layer: a multiplier near 1 for a category
float grain(float k, float dist) {
    vec2 w = wpos.xz;
    float near = 1.0 - smoothstep(25.0 * M, 160.0 * M, dist);
    vec4 big = texture2D(noise, w / (220.0 * M));
    float g = (big.r - 0.5) * 0.12;
    // (far off only the broad variation: the fine detail has faded out)
    if (near <= 0.0 && (k < 0.5 || k > 2.5)) return 1.0 + g;
    vec4 f = texture2D(detail, w / (2.3 * M));
    vec4 m = texture2D(detail, w / (11.0 * M));
    if (k < 0.5) {
        // grass
        g += ((f.r - 0.5) * 0.22 + (m.g - 0.5) * 0.14) * near;
    } else if (k < 1.5) {
        // fairway: mown stripes across the tile grid's diagonal, and fine blades
        float st = smoothstep(-0.25, 0.25, sin((w.x + w.y) * 0.7071 / (3.2 * M) * 3.14159));
        g += (st - 0.5) * 0.09 + ((f.r - 0.5) * 0.16 + (m.g - 0.5) * 0.06) * near;
    } else if (k < 2.5) {
        // green: close cut, the finest grain and faint narrow stripes
        float st = smoothstep(-0.3, 0.3, sin((w.x - w.y) * 0.7071 / (1.6 * M) * 3.14159));
        g += (st - 0.5) * 0.05 + (texture2D(detail, w / (0.9 * M)).r - 0.5) * 0.10 * near;
    } else if (k < 3.5) {
        // rough: longer grass in clumps
        g += ((f.r - 0.5) * 0.30 + (m.g - 0.5) * 0.30) * near;
    } else if (k < 4.5) {
        // sand: grains and soft rake marks
        g += ((f.b - 0.5) * 0.16 + (m.b - 0.5) * 0.10) * near;
    } else if (k < 5.5) {
        // desert ground: grains, pebbles and windblown ripples
        g += ((f.b - 0.5) * 0.20 + (f.a - 0.5) * 0.30 + (m.g - 0.5) * 0.16) * near;
    } else if (k < 6.5) {
        // tee: short grass
        g += ((texture2D(detail, w / (1.1 * M)).r - 0.5) * 0.14) * near;
    } else if (k < 7.5) {
        // woods floor: leaf litter
        g += ((f.g - 0.5) * 0.30 + (f.a - 0.5) * 0.25 + (m.g - 0.5) * 0.25) * near;
    } else if (k < 8.5) {
        // rock
        g += ((m.g - 0.5) * 0.30 + (f.b - 0.5) * 0.20) * near;
    } else {
        // path, water's edge
        g += ((f.a - 0.5) * 0.24 + (f.b - 0.5) * 0.14) * near;
    }
    return 1.0 + g;
}

// a sprite's texel with its colour divided by its alpha again (sprite textures are premultiplied)
vec4 unpremul(vec4 c) {
    return vec4(c.rgb / max(c.a, 0.004), c.a);
}

// the water's ripples: a slope from two layers of noise drifting apart
vec2 ripple(vec2 p, float t) {
    vec2 a = p * 0.11 + vec2(t * 0.013, t * 0.009);
    vec2 b = p * 0.23 - vec2(t * 0.011, -t * 0.016);
    float e = 1.0 / 256.0;
    float a0 = texture2D(noise, a).b;
    float b0 = texture2D(noise, b).a;
    return vec2(texture2D(noise, a + vec2(e, 0.0)).b - a0 + texture2D(noise, b + vec2(e, 0.0)).a - b0,
                texture2D(noise, a + vec2(0.0, e)).b - a0 + texture2D(noise, b + vec2(0.0, e)).a - b0);
}

// water: the art's colour `c` under moving ripples, the sky's reflection and the sun's glints
vec3 water_at(vec3 c, vec3 v) {
    vec2 sl = ripple(wpos.xz / M, eye.w) * 7.0;
    vec3 n = normalize(vec3(sl.x, 1.0, sl.y));
    vec3 r = reflect(v, n);
    float fres = 0.03 + 0.97 * pow(1.0 - max(dot(n, -v), 0.0), 5.0);
    vec3 sky = mix(haze.rgb, zenith.rgb, clamp(r.y * 2.0, 0.0, 1.0));
    float glint = pow(max(dot(r, sun.xyz), 0.0), 180.0) * 2.5;
    vec3 body = lit(c, vec3(0.0, 1.0, 0.0)) * 0.9;
    return mix(body, sky, clamp(fres * 0.9 + 0.08, 0.0, 1.0)) + glint * vec3(1.0, 0.95, 0.85);
}

// how much a texel of the art is water: near the open water's colour, or the lighter shallows painted at a shore, and
// bluer than it is red (grass and sand are not)
float wetness(vec3 c) {
    return (1.0 - smoothstep(0.2, 0.45, length(c - water.rgb))) * smoothstep(-0.02, 0.07, c.b - c.r);
}

// the water's edge: the wetness of the art a little blurred, its threshold wobbled by noise along the ground, so the art's
// shores (drawn along the tiles' edges) wander and fade softly instead of running straight
float shore_wet(vec2 t) {
    float w = wetness(texture2D(tex, t, 1.5).rgb);
    float n = texture2D(noise, wpos.xz / (9.0 * M)).g + texture2D(noise, wpos.xz / (3.0 * M)).a * 0.5 - 0.75;
    return smoothstep(0.25, 0.75, w + n * 0.55);
}

void main() {
    vec3 d = wpos - eye.xyz;
    float dist = length(d);
    vec3 v = d / max(dist, 1e-3);
    float m = sun.w;
    vec4 o;
    if (m < 0.5) {
        // the category in the vertex's alpha (/ 32), and half added on and beside water
        float shore = step(0.5, color.a);
        float k = floor((color.a - 0.5 * shore) * 32.0 + 0.5);
        // fairways and greens keep more of their art (their mowing); a tee's raised bevel and everything else is evened
        bool cut = k > 0.5 && k < 2.5;
        // (a tee's picture has a dark rim round its raised box, which round the eye standing on it would be a dark ring)
        bool tee = k > 5.5 && k < 6.5;
        vec4 c = art(uv, dist, cut ? 0.45 : 0.1, tee ? 0.85 : cut ? 0.15 : 0.5, color.rgb);
        vec3 g = lit(c.rgb * grain(k, dist), nrm);
        if (shore > 0.5) {
            // the water drawn into these tiles' art, wherever it is
            vec4 a = art(uv, dist, 0.0, 0.0, vec3(0.0));
            float w = shore_wet(uv);
            if (w > 0.0) g = mix(g, water_at(a.rgb, v), w);
        }
        o = vec4(g, 1.0);
    } else if (m < 1.5) {
        vec4 c = unpremul(texture2D(tex, uv));
        if (c.a < zenith.w) discard;
        float l = 1.0;
        if (dot(nrm, nrm) > 0.25) {
            vec3 n = normalize(nrm);
            l = 0.78 + 0.30 * max(dot(n, sun.xyz), 0.0) + 0.06 * n.y;
        }
        o = vec4(c.rgb * color.rgb * l * dif.w, 1.0);
    } else if (m < 3.5) {
        // the sea beyond the edge
        vec4 c = art(uv, dist, 0.0, 0.0, vec3(0.0));
        o = vec4(water_at(c.rgb, v), 1.0);
    } else if (m < 4.5) {
        float l = 1.0;
        if (dot(nrm, nrm) > 0.25) {
            vec3 n = normalize(nrm);
            l = 0.62 + 0.45 * max(dot(n, sun.xyz), 0.0);
        }
        o = vec4(color.rgb * l, color.a);
    } else if (m < 5.5) {
        vec4 c = unpremul(texture2D(tex, uv)) * color;
        if (c.a < 0.01) discard;
        o = c;
    } else if (m < 6.5) {
        float r = length(uv * 2.0 - 1.0);
        float a = color.a * (1.0 - smoothstep(0.25, 1.0, r));
        if (a < 0.005) discard;
        o = vec4(color.rgb, a);
    } else {
        vec4 c = texture2D(tex, uv) * color;
        if (c.a < zenith.w) discard;
        o = vec4(lit(c.rgb * grain(info.y, dist), nrm), c.a);
    }
    // what lies on the ground or is blended fades into the haze by its alpha: the haze is drawn under it already
    vec3 f = fogged(o.rgb, dist, v);
    gl_FragColor = vec4(f, o.a);
}
"#,
    );
    s
}

pub const SKY_VS: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec3 in_normal;
attribute vec4 in_color;
uniform vec4 cam_f;
uniform vec4 cam_r;
uniform vec4 cam_u;
varying highp vec3 dir;
void main() {
    gl_Position = vec4(in_pos.xy, 0.99999, 1.0);
    dir = cam_f.xyz + in_pos.x * cam_r.xyz + in_pos.y * cam_u.xyz;
}
"#;

/// The sky: the theme's colours from the horizon's haze up to the zenith, the sun with its glow, and clouds drifting on a
/// plane far overhead (the noise texture), thinning towards the horizon where they would alias.
pub fn sky_fs() -> String {
    let mut s = String::from(FRAG_HEAD);
    s.push_str(
        r#"varying highp vec3 dir;
uniform sampler2D noise;
uniform vec4 eye;
uniform vec4 sun;
uniform vec4 haze;
uniform vec4 zenith;
uniform vec4 cloud;
void main() {
    vec3 d = normalize(dir);
    float h = d.y;
    vec3 c = mix(haze.rgb, zenith.rgb, pow(clamp(h, 0.0, 1.0), 0.5));
    float sd = max(dot(d, sun.xyz), 0.0);
    c += vec3(1.0, 0.88, 0.70) * (pow(sd, 10.0) * 0.10 + pow(sd, 400.0) * 0.30 + pow(sd, 3000.0) * 0.6);
    if (h > 0.0) {
        vec2 p = d.xz / (h + 0.08) * 0.22 + vec2(eye.w * 0.0035, eye.w * 0.0012);
        float n = texture2D(noise, p).r * 0.62 + texture2D(noise, p * 2.9 + vec2(0.37, 0.11)).g * 0.38;
        float cov = zenith.w;
        float cl = smoothstep(1.0 - cov, 1.0 - cov + 0.22, n);
        float under = texture2D(noise, p + sun.xz * 0.015).r * 0.62 + texture2D(noise, (p + sun.xz * 0.015) * 2.9 + vec2(0.37, 0.11)).g * 0.38;
        vec3 cc = mix(cloud.rgb, cloud.rgb * vec3(0.74, 0.77, 0.84), clamp((under - n) * 4.0 + 0.45, 0.0, 1.0));
        cc += vec3(1.0, 0.9, 0.75) * pow(sd, 4.0) * 0.25;
        c = mix(c, cc, cl * smoothstep(0.0, 0.22, h) * 0.95);
    }
    // the sun's disc over the clouds' thin edges
    c = mix(c, vec3(1.0, 0.98, 0.92), smoothstep(1.0 - cloud.w * 1.6, 1.0 - cloud.w, sd));
    gl_FragColor = vec4(c, 1.0);
}
"#,
    );
    s
}

/// The course program's uniforms, in the order its shader meta lists them.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WalkUniforms {
    pub mvp: [f32; 16],
    /// The eye (xyz) and how far a vertex is drawn towards it (w; decals).
    pub pull: [f32; 4],
    /// The eye (xyz) and the clock in seconds (w).
    pub eye: [f32; 4],
    /// Towards the sun (xyz) and the draw's mode (w).
    pub sun: [f32; 4],
    /// The haze's colour (rgb) and its density per world unit (w).
    pub haze: [f32; 4],
    /// The sky overhead (rgb), the cut out's alpha threshold (w).
    pub zenith: [f32; 4],
    /// The light's ambient colour (rgb) and how much the sun tints the haze (w).
    pub amb: [f32; 4],
    /// The light's diffuse colour (rgb) and the sprites' brightness (w).
    pub dif: [f32; 4],
    /// The texture's size in texels (x), an ART draw's category (y), the screen's focal length in pixels (z).
    pub info: [f32; 4],
    /// The open water's colour (rgb; the ground's shader finds the water drawn into the art by it).
    pub water: [f32; 4],
}

impl WalkUniforms {
    pub fn with_mode(mut self, m: f32) -> Self {
        self.sun[3] = m;
        self
    }
}

/// The sky program's uniforms.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SkyUniforms {
    /// The view's direction, and its right and up scaled to the screen's edges.
    pub cam_f: [f32; 4],
    pub cam_r: [f32; 4],
    pub cam_u: [f32; 4],
    pub eye: [f32; 4],
    pub sun: [f32; 4],
    pub haze: [f32; 4],
    /// The zenith's colour, and how much of the sky the clouds cover (w, 0..1).
    pub zenith: [f32; 4],
    /// The clouds' colour, and the sun's disc's size (w, 1 - cos of its radius).
    pub cloud: [f32; 4],
}

/// Which fixed state a walk draw uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pipe {
    /// Depth tested and written, opaque (ground, walls, cut out sprites and shapes).
    Solid,
    /// Depth tested, blended (decals, shadows, trails).
    Blend,
}

/// One draw of the frame's stream (`WalkGpu::stream`).
struct Draw {
    pipe: Pipe,
    tex: TextureId,
    u: WalkUniforms,
    start: i32,
    count: i32,
}

/// The walk camera's GPU state, made on first use.
pub struct WalkGpu {
    solid: Pipeline,
    blend: Pipeline,
    sky: Pipeline,
    pub detail: TextureId,
    pub noise: TextureId,
    sky_mesh: Mesh,
    vb: BufferId,
    ib: BufferId,
    cap: usize,
    verts: Vec<Vert>,
    draws: Vec<Draw>,
    /// The blended draws, after the sky (`submit_late`).
    late: Vec<Draw>,
}

fn uniforms_walk() -> Vec<UniformDesc> {
    vec![
        UniformDesc::new("mvp", UniformType::Mat4),
        UniformDesc::new("pull", UniformType::Float4),
        UniformDesc::new("eye", UniformType::Float4),
        UniformDesc::new("sun", UniformType::Float4),
        UniformDesc::new("haze", UniformType::Float4),
        UniformDesc::new("zenith", UniformType::Float4),
        UniformDesc::new("amb", UniformType::Float4),
        UniformDesc::new("dif", UniformType::Float4),
        UniformDesc::new("info", UniformType::Float4),
        UniformDesc::new("water", UniformType::Float4),
    ]
}

fn uniforms_sky() -> Vec<UniformDesc> {
    ["cam_f", "cam_r", "cam_u", "eye", "sun", "haze", "zenith", "cloud"].iter().map(|n| UniformDesc::new(n, UniformType::Float4)).collect()
}

impl WalkGpu {
    /// The programs, pipelines and textures; None when the GPU will not build the shaders (the walk camera then keeps to
    /// the overhead view).
    pub fn new(g: &mut Gfx) -> Option<WalkGpu> {
        let ctx = &mut g.ctx;
        let fs = walk_fs();
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: WALK_VS, fragment: &fs },
                ShaderMeta {
                    images: vec!["tex".to_string(), "detail".to_string(), "noise".to_string(), "clean".to_string()],
                    uniforms: UniformBlockLayout { uniforms: uniforms_walk() },
                },
            )
            .map_err(|e| eprintln!("walk camera: the course shader does not build here: {e:?}"))
            .ok()?;
        let sfs = sky_fs();
        let sky_shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: SKY_VS, fragment: &sfs },
                ShaderMeta { images: vec!["noise".to_string()], uniforms: UniformBlockLayout { uniforms: uniforms_sky() } },
            )
            .map_err(|e| eprintln!("walk camera: the sky shader does not build here: {e:?}"))
            .ok()?;
        let attrs = [
            VertexAttribute::new("in_pos", VertexFormat::Float3),
            VertexAttribute::new("in_uv", VertexFormat::Float2),
            VertexAttribute::new("in_normal", VertexFormat::Float3),
            VertexAttribute::new("in_color", VertexFormat::Float4),
        ];
        let blend = Some(BlendState::new(
            Equation::Add,
            BlendFactor::Value(BlendValue::SourceAlpha),
            BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
        ));
        let keep_alpha = Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One));
        let mut pipe = |shader: ShaderId, depth_test: Comparison, depth_write: bool, color_blend: Option<BlendState>| {
            ctx.new_pipeline(
                &[BufferLayout::default()],
                &attrs,
                shader,
                PipelineParams {
                    depth_test,
                    depth_write,
                    color_blend,
                    alpha_blend: if color_blend.is_some() { keep_alpha } else { None },
                    ..Default::default()
                },
            )
        };
        // (miniquad tests depth only in a pipeline that writes it: the blended draws write it too, harmlessly, as they lie
        // on the ground or are thin, and so are hidden behind what stands in front of them)
        let solid = pipe(shader, Comparison::LessOrEqual, true, None);
        let blend = pipe(shader, Comparison::LessOrEqual, true, blend);
        // the sky is drawn after the solid ground and things, only where nothing stands (its depth is the far end's)
        let sky = pipe(sky_shader, Comparison::LessOrEqual, true, None);
        let detail = repeat_texture(g, &detail_image(256, 7));
        let noise = repeat_texture(g, &noise_image(256, 11));
        let quad = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        let sky_mesh = g.mesh(&quad.map(|(x, y)| Vert::new(x, y, 0.0, 0.0, 0.0)));
        let cap = 65536;
        let vb = g.ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vert>(cap));
        let ib =
            g.ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&(0..cap as u32).collect::<Vec<_>>()));
        Some(WalkGpu { solid, blend, sky, detail, noise, sky_mesh, vb, ib, cap, verts: Vec::new(), draws: Vec::new(), late: Vec::new() })
    }

    fn pipeline(&self, p: Pipe) -> Pipeline {
        match p {
            Pipe::Solid => self.solid,
            Pipe::Blend => self.blend,
        }
    }

    pub fn draw_sky(&self, g: &mut Gfx, u: &SkyUniforms) {
        g.draw_mesh_with(&self.sky, &self.sky_mesh, &[self.noise], UniformsSource::table(u));
    }

    /// A static mesh, at once; `clean` is its art's de-featured copy (the ground's, walk_ground.rs), else the art again.
    pub fn draw_mesh(&self, g: &mut Gfx, p: Pipe, mesh: &Mesh, tex: TextureId, clean: TextureId, u: &WalkUniforms) {
        g.draw_mesh_with(&self.pipeline(p), mesh, &[tex, self.detail, self.noise, clean], UniformsSource::table(u));
    }

    /// Triangles for the frame's stream, drawn in order by `submit` (one upload for the frame); blended ones wait for
    /// `submit_late`, after the sky.
    pub fn tris(&mut self, p: Pipe, tex: TextureId, u: &WalkUniforms, v: &[Vert]) {
        if v.is_empty() {
            return;
        }
        let list = if p == Pipe::Blend { &mut self.late } else { &mut self.draws };
        let start = self.verts.len() as i32;
        let same = list.last().is_some_and(|d| {
            d.pipe == p
                && d.tex == tex
                && d.start + d.count == start
                && d.u.mvp == u.mvp
                && d.u.sun == u.sun
                && d.u.zenith == u.zenith
                && d.u.pull == u.pull
                && d.u.info == u.info
        });
        if same {
            if let Some(d) = list.last_mut() {
                d.count += v.len() as i32;
            }
        } else {
            list.push(Draw { pipe: p, tex, u: *u, start, count: v.len() as i32 });
        }
        self.verts.extend_from_slice(v);
    }

    /// Draws the stream's solid draws (and uploads all of its vertices).
    pub fn submit(&mut self, g: &mut Gfx) {
        g.flush();
        if self.verts.is_empty() {
            self.draws.clear();
            self.late.clear();
            return;
        }
        if self.verts.len() > self.cap {
            while self.cap < self.verts.len() {
                self.cap *= 2;
            }
            g.ctx.delete_buffer(self.vb);
            g.ctx.delete_buffer(self.ib);
            self.vb = g.ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vert>(self.cap));
            let idx: Vec<u32> = (0..self.cap as u32).collect();
            self.ib = g.ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&idx));
        }
        g.ctx.buffer_update(self.vb, BufferSource::slice(&self.verts));
        let draws = std::mem::take(&mut self.draws);
        self.run(g, &draws);
    }

    /// Draws the stream's blended draws (after `submit` and the sky).
    pub fn submit_late(&mut self, g: &mut Gfx) {
        let late = std::mem::take(&mut self.late);
        self.run(g, &late);
        self.verts.clear();
    }

    fn run(&self, g: &mut Gfx, draws: &[Draw]) {
        for d in draws {
            g.ctx.apply_pipeline(&self.pipeline(d.pipe));
            g.ctx.apply_bindings(&Bindings {
                vertex_buffers: vec![self.vb],
                index_buffer: self.ib,
                images: vec![d.tex, self.detail, self.noise, d.tex],
            });
            g.ctx.apply_uniforms(UniformsSource::table(&d.u));
            g.ctx.draw(d.start, d.count, 1);
        }
    }
}

/// A texture that repeats, mipmapped (trilinear).
fn repeat_texture(g: &mut Gfx, img: &Rgba) -> TextureId {
    let params = TextureParams {
        kind: TextureKind::Texture2D,
        width: img.w,
        height: img.h,
        format: TextureFormat::RGBA8,
        wrap: TextureWrap::Repeat,
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Linear,
        allocate_mipmaps: true,
        sample_count: 1,
    };
    let id = g.ctx.new_texture(TextureAccess::Static, TextureSource::Bytes(&img.px), params);
    g.ctx.texture_generate_mipmaps(id);
    set_anisotropy(g, id, 8.0);
    id
}

/// A sprite's frame as a texture of the walk camera's own: clamped, mipmapped (seen from far off it shrinks a lot), its
/// colours premultiplied by their alpha so the filtering and the mipmaps mix in nothing of the transparent texels' colour
/// (the shader divides it out again: `unpremul`).
pub fn sprite_texture(g: &mut Gfx, img: &Rgba) -> TextureId {
    let mut img = bleed_alpha(img);
    for p in img.px.chunks_exact_mut(4) {
        let a = p[3] as u32;
        for c in &mut p[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
    let params = TextureParams {
        kind: TextureKind::Texture2D,
        width: img.w,
        height: img.h,
        format: TextureFormat::RGBA8,
        wrap: TextureWrap::Clamp,
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Linear,
        allocate_mipmaps: true,
        sample_count: 1,
    };
    let id = g.ctx.new_texture(TextureAccess::Static, TextureSource::Bytes(&img.px), params);
    g.ctx.texture_generate_mipmaps(id);
    id
}

/// The most anisotropic filtering the GPU offers (1 when none or not asked: the browser's and phones' builds), asked once.
fn max_anisotropy(_g: &mut Gfx) -> f32 {
    thread_local! {
        static MAX: std::cell::Cell<f32> = const { std::cell::Cell::new(-1.0) };
    }
    if let Some(m) = MAX.with(|m| Some(m.get()).filter(|v| *v >= 0.0)) {
        return m;
    }
    let m = ask_anisotropy();
    MAX.with(|c| c.set(m));
    m
}

fn ask_anisotropy() -> f32 {
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    {
        const MAX_MAX_ANISOTROPY: u32 = 0x84FF;
        unsafe {
            use miniquad::gl;
            while gl::glGetError() != 0 {}
            let mut v = 0;
            gl::glGetIntegerv(MAX_MAX_ANISOTROPY, &mut v);
            if gl::glGetError() == 0 && v > 1 {
                return v as f32;
            }
        }
    }
    1.0
}

/// Sets a texture's anisotropic filtering (`level` 1 is none), where the desktop's GL has it; nothing elsewhere.
pub fn set_anisotropy(g: &mut Gfx, tex: TextureId, level: f32) {
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    {
        use miniquad::gl;
        const TEXTURE_2D: u32 = 0x0DE1;
        const TEXTURE_BINDING_2D: u32 = 0x8069;
        const MAX_ANISOTROPY: u32 = 0x84FE;
        let max = max_anisotropy(g);
        if max <= 1.0 {
            return;
        }
        #[allow(irrefutable_let_patterns)]
        let miniquad::RawId::OpenGl(id) = (unsafe { g.ctx.texture_raw_id(tex) }) else {
            return;
        };
        unsafe {
            // miniquad keeps track of the bound texture: put back what was bound
            let mut was = 0;
            gl::glGetIntegerv(TEXTURE_BINDING_2D, &mut was);
            gl::glBindTexture(TEXTURE_2D, id);
            gl::glTexParameterf(TEXTURE_2D, MAX_ANISOTROPY, level.clamp(1.0, max));
            gl::glBindTexture(TEXTURE_2D, was as u32);
            while gl::glGetError() != 0 {}
        }
    }
    #[cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
    {
        let _ = (g, tex, level);
    }
}

// ---- the textures made here ------------------------------------------------------------------------------------------------

/// A hash of a lattice point to 0..1.
fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0
}

/// Smooth value noise with `period` lattice cells across the picture's `size`, so it repeats seamlessly.
fn value_noise(x: f32, y: f32, period: i32, size: usize, seed: u32) -> f32 {
    let (fx, fy) = (x * period as f32 / size as f32, y * period as f32 / size as f32);
    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(tx), s(ty));
    let h = |a: i32, b: i32| hash(a.rem_euclid(period), b.rem_euclid(period), seed);
    let top = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * sx;
    let bot = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * sx;
    top + (bot - top) * sy
}

/// Octaves of `value_noise` from `period` up, each half as strong, scaled back to 0..1 round 0.5.
fn fbm(x: f32, y: f32, period: i32, octaves: u32, size: usize, seed: u32) -> f32 {
    let (mut sum, mut amp, mut total, mut p) = (0.0, 1.0, 0.0, period);
    for o in 0..octaves {
        sum += value_noise(x, y, p, size, seed.wrapping_add(o * 977)) * amp;
        total += amp;
        amp *= 0.5;
        p *= 2;
    }
    sum / total
}

/// Stretches values round their mean to fill 0..1 (the octaves' sum bunches up round the middle).
fn normalise(v: &mut [f32]) {
    let n = v.len() as f32;
    let mean = v.iter().sum::<f32>() / n;
    let sd = (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / n).sqrt().max(1e-6);
    for x in v.iter_mut() {
        *x = (0.5 + (*x - mean) / sd * 0.18).clamp(0.0, 1.0);
    }
}

/// The ground's detail layer, `size` square, repeating: red fine grass blades (fine noise and speckle), green clumps (several
/// octaves), blue sand grains with soft ripples, alpha pebbles (scattered round stones, light on top and dark below).
pub fn detail_image(size: usize, seed: u32) -> Rgba {
    let n = size * size;
    let (mut r, mut gg, mut b, mut a) = (vec![0.0f32; n], vec![0.0f32; n], vec![0.0f32; n], vec![0.5f32; n]);
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (x as f32, y as f32);
            let i = y * size + x;
            // blades: fine noise stretched one way, and a finer speckle
            r[i] = fbm(fx, fy, 64, 2, size, seed) * 0.5 + hash(x as i32, y as i32, seed ^ 0x55) * 0.5;
            gg[i] = fbm(fx, fy, 4, 5, size, seed ^ 0x77);
            let ripple = ((fx * std::f32::consts::TAU * 9.0 / size as f32) + fbm(fx, fy, 8, 2, size, seed ^ 0x99) * 6.0).sin();
            b[i] = hash(x as i32, y as i32, seed ^ 0x33) * 0.55 + fbm(fx, fy, 32, 2, size, seed ^ 0x11) * 0.25 + ripple * 0.10;
        }
    }
    // pebbles: one or none per cell of a 16 x 16 grid, so they stay apart
    let cells = 16;
    let cs = size as f32 / cells as f32;
    for cy in 0..cells {
        for cx in 0..cells {
            if hash(cx, cy, seed ^ 0xabc) < 0.35 {
                continue;
            }
            let px = (cx as f32 + 0.2 + hash(cx, cy, seed ^ 1) * 0.6) * cs;
            let py = (cy as f32 + 0.2 + hash(cx, cy, seed ^ 2) * 0.6) * cs;
            let rad = 1.2 + hash(cx, cy, seed ^ 3) * cs * 0.22;
            let shade = 0.25 + hash(cx, cy, seed ^ 4) * 0.5;
            let r0 = rad.ceil() as i32 + 1;
            for dy in -r0..=r0 {
                for dx in -r0..=r0 {
                    let (x, y) = ((px as i32 + dx).rem_euclid(size as i32) as usize, (py as i32 + dy).rem_euclid(size as i32) as usize);
                    let d = ((dx as f32 + 0.5 - px.fract()).powi(2) + (dy as f32 + 0.5 - py.fract()).powi(2)).sqrt() / rad;
                    if d < 1.0 {
                        // lit from above: the top lighter, the lower edge darker
                        let lit = shade + (0.5 - dy as f32 / (2.0 * rad)) * 0.35;
                        let k = 1.0 - d * d;
                        a[y * size + x] = a[y * size + x] * (1.0 - k) + lit.clamp(0.0, 1.0) * k;
                    }
                }
            }
        }
    }
    normalise(&mut r);
    normalise(&mut gg);
    normalise(&mut b);
    let mut img = Rgba::new(size as u32, size as u32);
    for i in 0..n {
        img.px[i * 4] = (r[i] * 255.0) as u8;
        img.px[i * 4 + 1] = (gg[i] * 255.0) as u8;
        img.px[i * 4 + 2] = (b[i] * 255.0) as u8;
        img.px[i * 4 + 3] = (a[i].clamp(0.0, 1.0) * 255.0) as u8;
    }
    img
}

/// Smooth noise for the clouds and the water, `size` square, repeating: red and green clouds' billows at two scales, blue
/// and alpha the water's ripples.
pub fn noise_image(size: usize, seed: u32) -> Rgba {
    let n = size * size;
    let mut ch = [vec![0.0f32; n], vec![0.0f32; n], vec![0.0f32; n], vec![0.0f32; n]];
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (x as f32, y as f32);
            let i = y * size + x;
            ch[0][i] = fbm(fx, fy, 4, 6, size, seed);
            ch[1][i] = fbm(fx, fy, 8, 5, size, seed ^ 0x1234);
            ch[2][i] = fbm(fx, fy, 16, 3, size, seed ^ 0x2345);
            ch[3][i] = fbm(fx, fy, 32, 2, size, seed ^ 0x3456);
        }
    }
    for c in ch.iter_mut() {
        normalise(c);
    }
    let mut img = Rgba::new(size as u32, size as u32);
    for i in 0..n {
        for (k, c) in ch.iter().enumerate() {
            img.px[i * 4 + k] = (c[i] * 255.0) as u8;
        }
    }
    img
}

/// The average colour of a picture's opaque texels (0..1), None when it has none.
pub fn average_colour(img: &Rgba) -> Option<[f32; 3]> {
    let (mut s, mut n) = ([0.0f32; 3], 0.0f32);
    for p in img.px.chunks_exact(4).filter(|p| p[3] > 127) {
        for k in 0..3 {
            s[k] += p[k] as f32;
        }
        n += 1.0;
    }
    (n > 0.0).then(|| s.map(|v| v / n / 255.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The made textures repeat without a seam: the left column matches the right as a neighbour would, as do the rows.
    #[test]
    fn made_textures_repeat_seamlessly() {
        let size = 64;
        for img in [detail_image(size, 7), noise_image(size, 11)] {
            for ch in [1usize, 2] {
                let at = |x: usize, y: usize| img.px[(y * size + x) * 4 + ch] as i32;
                let mut seam = 0;
                let mut inner = 0;
                for k in 0..size {
                    seam += (at(size - 1, k) - at(0, k)).abs() + (at(k, size - 1) - at(k, 0)).abs();
                    inner += (at(size / 2, k) - at(size / 2 - 1, k)).abs() + (at(k, size / 2) - at(k, size / 2 - 1)).abs();
                }
                // the step across the seam is like any other step between neighbours
                assert!((seam as f32) < inner as f32 * 2.5 + 64.0, "channel {ch}: seam {seam} inner {inner}");
            }
        }
    }

    #[test]
    fn made_textures_use_their_range() {
        let img = noise_image(64, 3);
        let reds: Vec<u8> = img.px.chunks_exact(4).map(|p| p[0]).collect();
        let (lo, hi) = (*reds.iter().min().unwrap(), *reds.iter().max().unwrap());
        assert!(lo < 70 && hi > 185, "{lo}..{hi}");
        let mean = reds.iter().map(|&v| v as f32).sum::<f32>() / reds.len() as f32;
        assert!((mean - 127.0).abs() < 12.0);
    }

    #[test]
    fn categories_survive_the_vertex_colour() {
        for c in 0..=cat::WATER {
            for shore in [0.0, 0.5] {
                let r = cat_colour(c)[3] + shore;
                // through an 8 bit varying at worst
                let q = (r * 255.0).round() / 255.0;
                assert_eq!(cat_of(q), (c, shore > 0.0));
            }
        }
    }
}
