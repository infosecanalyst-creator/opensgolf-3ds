//! Rendering on top of miniquad (OpenGL on Windows, Linux and macOS): a column-major matrix type, one shader that covers every
//! draw the game makes (lit terrain, overlays, sprites, 2D interface), static meshes and an immediate-mode batcher.
//!
//! The original renders its terrain with fixed-function OpenGL (Terrain.dll). The shader reproduces what it uses: per-vertex
//! lighting from one directional light fixed in eye space with OpenGL's default material and a white specular of shininess
//! 13 (sg_core::terrain::Lighting::shade), texture modulation, and an alpha test.
use miniquad::*;
use sg_core::assets::Rgba;

/// 4x4 matrix, column-major (m[col * 4 + row]) like OpenGL.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4(pub [f32; 16]);

impl Mat4 {
    pub fn identity() -> Mat4 {
        let mut m = [0.0; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        Mat4(m)
    }
    /// glOrtho.
    pub fn ortho(l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) -> Mat4 {
        let mut m = Mat4::identity().0;
        m[0] = 2.0 / (r - l);
        m[5] = 2.0 / (t - b);
        m[10] = -2.0 / (f - n);
        m[12] = -(r + l) / (r - l);
        m[13] = -(t + b) / (t - b);
        m[14] = -(f + n) / (f - n);
        Mat4(m)
    }
    /// glRotate: angle in degrees about an axis.
    pub fn rotate(deg: f64, x: f32, y: f32, z: f32) -> Mat4 {
        let len = (x * x + y * y + z * z).sqrt();
        let (x, y, z) = (x / len, y / len, z / len);
        let a = deg.to_radians();
        let (s, c) = (a.sin() as f32, a.cos() as f32);
        let k = 1.0 - c;
        let mut m = Mat4::identity().0;
        m[0] = x * x * k + c;
        m[1] = y * x * k + z * s;
        m[2] = x * z * k - y * s;
        m[4] = x * y * k - z * s;
        m[5] = y * y * k + c;
        m[6] = y * z * k + x * s;
        m[8] = x * z * k + y * s;
        m[9] = y * z * k - x * s;
        m[10] = z * z * k + c;
        Mat4(m)
    }
    pub fn translate(x: f32, y: f32, z: f32) -> Mat4 {
        let mut m = Mat4::identity().0;
        m[12] = x;
        m[13] = y;
        m[14] = z;
        Mat4(m)
    }
    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let (a, b) = (&self.0, &o.0);
        let mut r = [0.0; 16];
        for c in 0..4 {
            for row in 0..4 {
                r[c * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[c * 4 + k]).sum();
            }
        }
        Mat4(r)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

impl Vert {
    pub fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Vert {
        Vert { pos: [x, y, z], uv: [u, v], normal: [0.0, 1.0, 0.0], color: [1.0; 4] }
    }
    pub fn col(mut self, c: [f32; 4]) -> Vert {
        self.color = c;
        self
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Uniforms {
    pub mvp: [f32; 16],
    pub mv: [f32; 16],
    pub lit: f32,
    pub alpha_ref: f32,
    pub light_amb: [f32; 3],
    pub light_dif: [f32; 3],
    pub light_spec: [f32; 3],
    /// Toward the light, in eye space.
    pub light_dir: [f32; 3],
}

impl Uniforms {
    pub fn flat(proj: &Mat4, mv: &Mat4) -> Uniforms {
        Uniforms {
            mvp: proj.mul(mv).0,
            mv: mv.0,
            lit: 0.0,
            alpha_ref: -1.0,
            light_amb: [1.0; 3],
            light_dif: [0.0; 3],
            light_spec: [0.0; 3],
            light_dir: [0.0, 1.0, 0.0],
        }
    }
}

/// The fragment shaders' uniforms (alpha_ref, edge) are not declared here: a uniform both stages declare must have the same
/// precision there (GLSL ES 1.00, 4.5.3), and this stage's float is highp where the fragment shader's is mediump. Chrome let
/// the mismatch link; Firefox refused it ("Uniform `alpha_ref` is not linkable between attached shaders"), and the game
/// stopped at a black screen (issue #9). `shader_stages_agree` checks every pair of the game's shaders for it.
const VERTEX: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec3 in_normal;
attribute vec4 in_color;
uniform mat4 mvp;
uniform mat4 mv;
uniform float lit;
uniform vec3 light_amb;
uniform vec3 light_dif;
uniform vec3 light_spec;
uniform vec3 light_dir;
varying lowp vec4 color;
varying highp vec2 uv;
void main() {
    gl_Position = mvp * vec4(in_pos, 1.0);
    uv = in_uv;
    vec4 c = in_color;
    if (lit > 0.5) {
        // OpenGL's lighting of the default material: global ambient 0.2 and the light's ambient times 0.2, the light's
        // diffuse times 0.8, and the specular (shininess 13, viewer at infinity) where the face is lit
        // not normalised, as OpenGL without GL_NORMALIZE: the terrain's normals are unit, the cliff's are not
        vec3 n = (mv * vec4(in_normal, 0.0)).xyz;
        float d = dot(n, light_dir);
        vec3 s = vec3(0.04) + 0.2 * light_amb;
        if (d > 0.0) {
            vec3 h = normalize(light_dir + vec3(0.0, 0.0, 1.0));
            s += 0.8 * light_dif * d + light_spec * pow(max(dot(n, h), 0.0), 13.0);
        }
        c.rgb = c.rgb * min(s, vec3(1.0));
    }
    color = c;
}
"#;

const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying highp vec2 uv;
uniform sampler2D tex;
uniform float alpha_ref;
void main() {
    vec4 c = texture2D(tex, uv) * color;
    if (c.a <= alpha_ref) discard;
    gl_FragColor = c;
}
"#;

/// Enhanced graphics' smooth edges (edges.rs): a colour-keyed picture whose alpha holds the distance to its smoothed outline
/// (-edges::SPREAD..+SPREAD texels over 0..1, so 8 texels across). `edge` is how many screen pixels a texel covers in this
/// draw, so the coverage ramp is one screen pixel wide at any size. Colours come out premultiplied (drawn with the
/// premultiplied blend).
const FRAGMENT_SDF: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying highp vec2 uv;
uniform sampler2D tex;
uniform float alpha_ref;
uniform float edge;
void main() {
    vec4 t = texture2D(tex, uv);
    float a = clamp((t.a - 0.5) * 8.0 * edge + 0.5, 0.0, 1.0);
    vec4 c = vec4(t.rgb * a, a) * color;
    if (c.a <= alpha_ref) discard;
    gl_FragColor = c;
}
"#;

/// The uniforms of FRAGMENT_SDF's draws: the usual ones and the screen pixels per texel.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct SdfUniforms {
    u: Uniforms,
    edge: f32,
}

/// Which fixed state a draw uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Depth tested and written, no blending (terrain, walls).
    Solid,
    /// Depth tested, not written, alpha blended (path overlays).
    Overlay,
    /// No depth test, alpha blended (sprites in painter's order, the ball, the 2D interface).
    Flat,
}

pub struct Mesh {
    vb: BufferId,
    ib: BufferId,
    count: i32,
}

pub struct Gfx {
    pub ctx: Box<dyn RenderingBackend>,
    solid: Pipeline,
    overlay: Pipeline,
    flat: Pipeline,
    /// Enhanced graphics (enhanced.rs): the overlay and flat states blending premultiplied colours, used for the textures in
    /// `premul` (made while Enhanced's smooth interface option was on); every other texture draws as it always has.
    overlay_pm: Pipeline,
    flat_pm: Pipeline,
    premul: std::collections::HashSet<TextureId>,
    /// Enhanced graphics' smooth edges (edges.rs): textures whose alpha is a distance to the outline, drawn with FRAGMENT_SDF
    /// (premultiplied blending) by these pipelines.
    overlay_sdf: Pipeline,
    flat_sdf: Pipeline,
    sdf: std::collections::HashSet<TextureId>,
    pub white: TextureId,
    stream_vb: BufferId,
    stream_ib: BufferId,
    stream_cap: usize,
    // Immediate-mode batch: vertices of consecutive draws with the same texture and state are merged.
    batch: Vec<Vert>,
    batch_tex: Option<TextureId>,
    batch_mode: Mode,
    batch_uniforms: Option<Uniforms>,
    /// The Enhanced graphics effects' pipelines (fx.rs), made on first use; None in Classic.
    pub fx: Option<Box<crate::fx::FxGpu>>,
}

fn seq_indices(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

impl Gfx {
    pub fn new() -> Gfx {
        let mut ctx = window::new_rendering_backend();
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("mvp", UniformType::Mat4),
                            UniformDesc::new("mv", UniformType::Mat4),
                            UniformDesc::new("lit", UniformType::Float1),
                            UniformDesc::new("alpha_ref", UniformType::Float1),
                            UniformDesc::new("light_amb", UniformType::Float3),
                            UniformDesc::new("light_dif", UniformType::Float3),
                            UniformDesc::new("light_spec", UniformType::Float3),
                            UniformDesc::new("light_dir", UniformType::Float3),
                        ],
                    },
                },
            )
            .unwrap_or_else(|e| panic!("the game's base shader does not compile or link here: {e:?}"));
        let sdf_uniforms = vec![
            UniformDesc::new("mvp", UniformType::Mat4),
            UniformDesc::new("mv", UniformType::Mat4),
            UniformDesc::new("lit", UniformType::Float1),
            UniformDesc::new("alpha_ref", UniformType::Float1),
            UniformDesc::new("light_amb", UniformType::Float3),
            UniformDesc::new("light_dif", UniformType::Float3),
            UniformDesc::new("light_spec", UniformType::Float3),
            UniformDesc::new("light_dir", UniformType::Float3),
            UniformDesc::new("edge", UniformType::Float1),
        ];
        // Enhanced's smooth edges: never fatal, a GPU that refuses it draws those textures with the base shader
        let sdf_shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT_SDF },
                ShaderMeta { images: vec!["tex".to_string()], uniforms: UniformBlockLayout { uniforms: sdf_uniforms } },
            )
            .unwrap_or_else(|e| {
                eprintln!("smooth edges shader failed, drawing without it: {e:?}");
                shader
            });
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
        let alpha_blend = Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha)));
        let mut pipe = |shader: ShaderId, depth_test: Comparison, depth_write: bool, color_blend: Option<BlendState>| {
            ctx.new_pipeline(
                &[BufferLayout::default()],
                &attrs,
                shader,
                PipelineParams {
                    depth_test,
                    depth_write,
                    color_blend,
                    alpha_blend: if color_blend.is_some() { alpha_blend } else { None },
                    ..Default::default()
                },
            )
        };
        let solid = pipe(shader, Comparison::Less, true, None);
        let overlay = pipe(shader, Comparison::Less, false, blend);
        let flat = pipe(shader, Comparison::Always, false, blend);
        let pm = Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha)));
        let overlay_pm = pipe(shader, Comparison::Less, false, pm);
        let flat_pm = pipe(shader, Comparison::Always, false, pm);
        let overlay_sdf = pipe(sdf_shader, Comparison::Less, false, pm);
        let flat_sdf = pipe(sdf_shader, Comparison::Always, false, pm);
        let white = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);
        let stream_cap = 4096;
        let stream_vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vert>(stream_cap));
        let stream_ib = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&seq_indices(stream_cap)));
        Gfx {
            ctx,
            solid,
            overlay,
            flat,
            overlay_pm,
            flat_pm,
            premul: Default::default(),
            overlay_sdf,
            flat_sdf,
            sdf: Default::default(),
            white,
            stream_vb,
            stream_ib,
            stream_cap,
            batch: Vec::new(),
            batch_tex: None,
            batch_mode: Mode::Flat,
            batch_uniforms: None,
            fx: None,
        }
    }

    fn pipeline(&self, mode: Mode, tex: TextureId) -> &Pipeline {
        let pm = !self.premul.is_empty() && self.premul.contains(&tex);
        let sdf = self.is_sdf(tex);
        match mode {
            Mode::Solid => &self.solid,
            Mode::Overlay if sdf => &self.overlay_sdf,
            Mode::Flat if sdf => &self.flat_sdf,
            Mode::Overlay if pm => &self.overlay_pm,
            Mode::Overlay => &self.overlay,
            Mode::Flat if pm => &self.flat_pm,
            Mode::Flat => &self.flat,
        }
    }

    /// Whether a texture holds premultiplied colours (Enhanced graphics' smooth interface option, enhanced.rs).
    pub fn is_premul(&self, tex: TextureId) -> bool {
        !self.premul.is_empty() && self.premul.contains(&tex)
    }

    /// Whether a texture's alpha is a distance to its outline (Enhanced graphics' smooth edges, edges.rs).
    pub fn is_sdf(&self, tex: TextureId) -> bool {
        !self.sdf.is_empty() && self.sdf.contains(&tex)
    }

    /// Enhanced graphics' smooth edges: a colour-keyed picture with the distance to its smoothed outline in alpha (edges.rs),
    /// straight colours, mipmapped (trilinear) and clamped.
    fn texture_sdf(&mut self, img: &Rgba) -> TextureId {
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
        let id = self.ctx.new_texture(TextureAccess::Static, TextureSource::Bytes(&img.px), params);
        self.ctx.texture_generate_mipmaps(id);
        self.premul.remove(&id);
        self.sdf.insert(id);
        id
    }

    /// Enhanced graphics: a texture with premultiplied colours, mipmapped (trilinear) and clamped; `aniso` adds anisotropic
    /// filtering where the driver offers it.
    fn texture_enhanced(&mut self, img: &Rgba, aniso: bool) -> TextureId {
        let mut px = img.px.clone();
        for p in px.chunks_exact_mut(4) {
            let a = p[3] as u32;
            if a < 255 {
                for c in &mut p[..3] {
                    *c = ((*c as u32 * a + 127) / 255) as u8;
                }
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
        let id = self.ctx.new_texture(TextureAccess::Static, TextureSource::Bytes(&px), params);
        self.ctx.texture_generate_mipmaps(id);
        if aniso {
            crate::enhanced::anisotropy(self, id);
        }
        self.sdf.remove(&id);
        self.premul.insert(id);
        id
    }

    /// Enhanced graphics' text: a glyph atlas (white, the coverage in alpha) drawn texel for pixel, so neither smoothed nor
    /// mipmapped; premultiplied when the smooth interface option is on.
    pub fn text_atlas(&mut self, img: &Rgba) -> TextureId {
        let pm = crate::enhanced::smooth_ui();
        let mut px = img.px.clone();
        if pm {
            for p in px.chunks_exact_mut(4) {
                p[0] = p[3];
                p[1] = p[3];
                p[2] = p[3];
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
            mipmap_filter: MipmapFilterMode::None,
            allocate_mipmaps: false,
            sample_count: 1,
        };
        let id = self.ctx.new_texture(TextureAccess::Static, TextureSource::Bytes(&px), params);
        if pm {
            self.premul.insert(id);
        }
        crate::fx::plain_texture(id); // text: the effects' scaler leaves it alone
        id
    }

    /// A texture from RGBA pixels. `mipmaps` for terrain textures (trilinear like the original), clamped at the edges.
    pub fn texture(&mut self, img: &Rgba, mipmaps: bool) -> TextureId {
        self.texture_with(img, mipmaps, true)
    }

    /// `texture` the classic way whatever the Enhanced settings (the classic text atlas, which only Classic draws with).
    pub fn texture_classic(&mut self, img: &Rgba) -> TextureId {
        self.texture_with(img, false, false)
    }

    fn texture_with(&mut self, img: &Rgba, mipmaps: bool, enhanced: bool) -> TextureId {
        // transparent pixels still carry the art's key colour (magenta); smoothing would blend it into the edges
        let bled;
        let img = if img.px.chunks_exact(4).any(|p| p[3] == 0) {
            bled = bleed_alpha(img);
            &bled
        } else {
            img
        };
        // a colour-keyed picture (interface art, sprites): smooth outlines at any size (edges.rs)
        if enhanced && !mipmaps && crate::enhanced::smooth_edges() {
            if let Some(soft) = crate::edges::soft_edges(img) {
                return self.texture_sdf(&soft);
            }
        }
        if enhanced && (crate::enhanced::smooth_ui() || (mipmaps && crate::enhanced::terrain_filter())) {
            return self.texture_enhanced(img, mipmaps && crate::enhanced::terrain_filter());
        }
        let params = TextureParams {
            kind: TextureKind::Texture2D,
            width: img.w,
            height: img.h,
            format: TextureFormat::RGBA8,
            wrap: TextureWrap::Clamp,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: if mipmaps { MipmapFilterMode::Linear } else { MipmapFilterMode::None },
            allocate_mipmaps: mipmaps,
            sample_count: 1,
        };
        let id = self.ctx.new_texture(TextureAccess::Static, TextureSource::Bytes(&img.px), params);
        if mipmaps {
            self.ctx.texture_generate_mipmaps(id);
        }
        // a texture made the classic way under the id of one deleted earlier
        self.premul.remove(&id);
        self.sdf.remove(&id);
        id
    }

    /// A texture from an HD pack picture (only used in HD mode, see hd.rs): trilinear filtered with mipmaps, since it is drawn
    /// smaller than its own resolution. The pack's pictures carry their colours under transparent pixels already.
    pub fn texture_hd(&mut self, img: &Rgba) -> TextureId {
        if crate::enhanced::smooth_ui() {
            return self.texture_enhanced(img, crate::enhanced::terrain_filter());
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
        let id = self.ctx.new_texture(TextureAccess::Static, TextureSource::Bytes(&img.px), params);
        self.ctx.texture_generate_mipmaps(id);
        id
    }

    pub fn mesh(&mut self, verts: &[Vert]) -> Mesh {
        let vb = self.ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(verts));
        let ib = self.ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&seq_indices(verts.len())));
        Mesh { vb, ib, count: verts.len() as i32 }
    }
    pub fn delete_mesh(&mut self, m: Mesh) {
        self.ctx.delete_buffer(m.vb);
        self.ctx.delete_buffer(m.ib);
    }

    pub fn draw_mesh(&mut self, mode: Mode, mesh: &Mesh, tex: TextureId, u: &Uniforms) {
        self.flush();
        if mesh.count == 0 {
            return;
        }
        let p = *self.pipeline(mode, tex);
        self.ctx.apply_pipeline(&p);
        self.ctx.apply_bindings(&Bindings { vertex_buffers: vec![mesh.vb], index_buffer: mesh.ib, images: vec![tex] });
        self.ctx.apply_uniforms(UniformsSource::table(u));
        self.ctx.draw(0, mesh.count, 1);
    }

    /// A mesh with another pipeline and its own textures and uniforms (the effects' shaders, fx.rs).
    pub fn draw_mesh_with(&mut self, pipeline: &Pipeline, mesh: &Mesh, images: &[TextureId], u: UniformsSource) {
        self.flush();
        if mesh.count == 0 {
            return;
        }
        self.ctx.apply_pipeline(pipeline);
        self.ctx.apply_bindings(&Bindings { vertex_buffers: vec![mesh.vb], index_buffer: mesh.ib, images: images.to_vec() });
        self.ctx.apply_uniforms(u);
        self.ctx.draw(0, mesh.count, 1);
    }

    /// Queues triangles (three vertices each). Consecutive calls with the same texture, mode and uniforms are drawn together.
    pub fn tris(&mut self, mode: Mode, tex: Option<TextureId>, u: &Uniforms, verts: &[Vert]) {
        let tex = tex.unwrap_or(self.white);
        let same = self.batch_tex == Some(tex)
            && self.batch_mode == mode
            && self.batch_uniforms.map(|b| b.mvp == u.mvp && b.lit == u.lit && b.alpha_ref == u.alpha_ref).unwrap_or(false);
        if !same {
            self.flush();
            self.batch_tex = Some(tex);
            self.batch_mode = mode;
            self.batch_uniforms = Some(*u);
        }
        if self.is_premul(tex) || self.is_sdf(tex) {
            // premultiplied blending takes the vertex colour premultiplied too
            self.batch.extend(verts.iter().map(|v| {
                let mut v = *v;
                let a = v.color[3];
                v.color[0] *= a;
                v.color[1] *= a;
                v.color[2] *= a;
                v
            }));
        } else {
            self.batch.extend_from_slice(verts);
        }
    }

    /// A quad given as four corners in order (two triangles).
    pub fn quad(&mut self, mode: Mode, tex: Option<TextureId>, u: &Uniforms, q: [Vert; 4]) {
        self.tris(mode, tex, u, &[q[0], q[1], q[2], q[0], q[2], q[3]]);
    }

    pub fn flush(&mut self) {
        if self.batch.is_empty() {
            return;
        }
        let (Some(tex), Some(u)) = (self.batch_tex, self.batch_uniforms) else {
            self.batch.clear();
            return;
        };
        if self.batch.len() > self.stream_cap {
            while self.stream_cap < self.batch.len() {
                self.stream_cap *= 2;
            }
            self.ctx.delete_buffer(self.stream_vb);
            self.ctx.delete_buffer(self.stream_ib);
            self.stream_vb =
                self.ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vert>(self.stream_cap));
            self.stream_ib =
                self.ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&seq_indices(self.stream_cap)));
        }
        self.ctx.buffer_update(self.stream_vb, BufferSource::slice(&self.batch));
        // Enhanced graphics only (fx.rs): magnified sprites and interface pictures through the pixel-art scaler
        let sdf = self.is_sdf(tex);
        let colours = if sdf {
            crate::fx::TEX_SDF
        } else if self.is_premul(tex) {
            crate::fx::TEX_PREMUL
        } else {
            crate::fx::TEX_STRAIGHT
        };
        let scaled = self.batch_mode == Mode::Flat
            && tex != self.white
            && crate::fx::scaler_wanted()
            && crate::fx::apply_scaler(&mut self.ctx, &mut self.fx, &self.batch, self.stream_vb, self.stream_ib, tex, &u, colours);
        if !scaled {
            let p = *self.pipeline(self.batch_mode, tex);
            self.ctx.apply_pipeline(&p);
            self.ctx.apply_bindings(&Bindings { vertex_buffers: vec![self.stream_vb], index_buffer: self.stream_ib, images: vec![tex] });
            if sdf {
                let edge = batch_scale(&*self.ctx, &self.batch, tex, &u).unwrap_or(1.0);
                self.ctx.apply_uniforms(UniformsSource::table(&SdfUniforms { u, edge }));
            } else {
                self.ctx.apply_uniforms(UniformsSource::table(&u));
            }
        }
        self.ctx.draw(0, self.batch.len() as i32, 1);
        self.batch.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The precision of each non-sampler uniform a shader stage declares: its own qualifier, else the stage's default for
    /// floats (highp in a vertex shader, the `precision` statement's in a fragment shader; "either" when that depends on
    /// GL_FRAGMENT_PRECISION_HIGH) and for ints (highp in a vertex shader, mediump in a fragment shader).
    fn uniform_precisions(src: &str, vertex: bool) -> Vec<(String, String)> {
        let float_default = if vertex {
            "highp"
        } else if src.contains("GL_FRAGMENT_PRECISION_HIGH") {
            "either"
        } else if src.contains("precision highp float") {
            "highp"
        } else if src.contains("precision lowp float") {
            "lowp"
        } else {
            "mediump"
        };
        let mut out = Vec::new();
        for line in src.lines().map(str::trim).filter(|l| l.starts_with("uniform ")) {
            let words: Vec<&str> = line.trim_end_matches(';').split_whitespace().skip(1).collect();
            let (prec, ty, name) = match words.as_slice() {
                [p @ ("lowp" | "mediump" | "highp"), t, n] => (p.to_string(), *t, *n),
                [t, n] => (String::new(), *t, *n),
                _ => panic!("unexpected uniform line: {line}"),
            };
            if ty.starts_with("sampler") {
                continue;
            }
            let prec = if !prec.is_empty() {
                prec
            } else if ty.starts_with("int") || ty.starts_with("ivec") {
                (if vertex { "highp" } else { "mediump" }).to_string()
            } else {
                float_default.to_string()
            };
            out.push((name.split('[').next().unwrap().to_string(), prec));
        }
        out
    }

    /// Every vertex and fragment shader pair the game links: a uniform both stages declare has the same precision in both
    /// (GLSL ES 1.00 4.5.3; Firefox refuses to link the pair otherwise, issue #9).
    #[test]
    fn shader_stages_agree() {
        use crate::fx_glsl as g;
        let pairs: Vec<(&str, &str, String)> = vec![
            ("base", VERTEX, FRAGMENT.to_string()),
            ("smooth edges", VERTEX, FRAGMENT_SDF.to_string()),
            ("scaler", g::SCALER_VS, g::scaler_fs()),
            ("smooth scaler", g::SCALER_VS, crate::fx_smooth::smooth_fs()),
            ("terrain", g::GROUND_VS, g::terrain_fs()),
            ("water", g::GROUND_VS, g::water_fs()),
            ("shade", g::SHADE_VS, g::shade_fs()),
            ("down", g::POST_VS, g::down_fs()),
            ("blur", g::POST_VS, g::blur_fs()),
            ("composite", g::POST_VS, g::composite_fs()),
            ("photo composite", g::POST_VS, crate::photo::COMPOSITE_FS.to_string()),
        ];
        let mut bad = Vec::new();
        for (name, vs, fs) in &pairs {
            let f = uniform_precisions(fs, false);
            for (u, pv) in uniform_precisions(vs, true) {
                if let Some((_, pf)) = f.iter().find(|(n, _)| *n == u) {
                    if pf != &pv || pv == "either" {
                        bad.push(format!("{name}: uniform {u} is {pv} in the vertex shader, {pf} in the fragment shader"));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
        // the check itself: the pair Firefox refused
        let old_vs = VERTEX.replace("uniform float lit;", "uniform float lit;\nuniform float alpha_ref;");
        let f = uniform_precisions(FRAGMENT, false);
        let v = uniform_precisions(&old_vs, true);
        assert!(v.iter().any(|(n, p)| n == "alpha_ref" && f.iter().any(|(m, q)| m == n && q != p)));
    }

    #[test]
    fn rotate_matches_gl() {
        // A 90 degree turn about Y takes +X to -Z.
        let m = Mat4::rotate(90.0, 0.0, 1.0, 0.0).0;
        let (x, z) = (m[0] * 1.0, m[2] * 1.0);
        assert!(x.abs() < 1e-6 && (z + 1.0).abs() < 1e-6);
    }

    #[test]
    fn ortho_maps_corners() {
        let m = Mat4::ortho(-2.0, 2.0, -1.0, 1.0, -5.0, 5.0).0;
        assert!((m[0] * 2.0 + m[12] - 1.0).abs() < 1e-6);
        assert!((-m[5] + m[13] + 1.0).abs() < 1e-6);
    }
}

/// How many screen pixels a texel of `tex` covers in a batch of quads, measured along its first edge (its first two
/// vertices); None when that edge has no length in the texture.
pub fn batch_scale(ctx: &dyn RenderingBackend, batch: &[Vert], tex: TextureId, u: &Uniforms) -> Option<f32> {
    if batch.len() < 2 {
        return None;
    }
    let (tw, th) = ctx.texture_size(tex);
    // a photo larger than the window (photo.rs) is drawn into a target of its own size
    let (sw, sh) = crate::fx::render_size().unwrap_or_else(window::screen_size);
    let (a, b) = (batch[0], batch[1]);
    let clip = |p: [f32; 3]| {
        let m = &u.mvp;
        let x = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
        let y = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
        let w = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
        (x / w * sw * 0.5, y / w * sh * 0.5)
    };
    let (pa, pb) = (clip(a.pos), clip(b.pos));
    let px = (pb.0 - pa.0).hypot(pb.1 - pa.1);
    let texels = ((b.uv[0] - a.uv[0]) * tw as f32).hypot((b.uv[1] - a.uv[1]) * th as f32);
    let scale = px / texels;
    (texels >= 0.5 && scale.is_finite() && scale > 0.0).then_some(scale)
}

/// Gives every fully transparent pixel the average colour of its opaque neighbours (repeated outwards a few pixels), so linear
/// filtering and mipmaps at a sprite's edge mix in the sprite's own colours instead of the transparent key colour.
pub fn bleed_alpha(img: &Rgba) -> Rgba {
    let (w, h) = (img.w as usize, img.h as usize);
    let mut out = img.clone();
    let mut known: Vec<bool> = img.px.chunks_exact(4).map(|p| p[3] != 0).collect();
    for _ in 0..4 {
        let mut next = known.clone();
        let mut changed = false;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if known[i] {
                    continue;
                }
                let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if known[j] {
                        r += out.px[j * 4] as u32;
                        g += out.px[j * 4 + 1] as u32;
                        b += out.px[j * 4 + 2] as u32;
                        n += 1;
                    }
                }
                if n > 0 {
                    out.px[i * 4] = (r / n) as u8;
                    out.px[i * 4 + 1] = (g / n) as u8;
                    out.px[i * 4 + 2] = (b / n) as u8;
                    next[i] = true;
                    changed = true;
                }
            }
        }
        known = next;
        if !changed {
            break;
        }
    }
    // whatever is still far from any opaque pixel goes black, so even the smallest mipmaps carry no key colour
    for (p, k) in out.px.chunks_exact_mut(4).zip(&known) {
        if !k {
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
        }
    }
    out
}

#[cfg(test)]
mod bleed_tests {
    #[test]
    fn transparent_pixels_take_their_neighbours_colour() {
        let mut img = sg_core::assets::Rgba::new(3, 1);
        img.px = vec![255, 0, 255, 0, 10, 20, 30, 255, 255, 0, 255, 0];
        let out = super::bleed_alpha(&img);
        assert_eq!(&out.px[0..4], &[10, 20, 30, 0]);
        assert_eq!(&out.px[8..12], &[10, 20, 30, 0]);
    }
}
