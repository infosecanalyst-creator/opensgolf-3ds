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

const VERTEX: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec3 in_normal;
attribute vec4 in_color;
uniform mat4 mvp;
uniform mat4 mv;
uniform float lit;
uniform float alpha_ref;
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
    pub white: TextureId,
    stream_vb: BufferId,
    stream_ib: BufferId,
    stream_cap: usize,
    // Immediate-mode batch: vertices of consecutive draws with the same texture and state are merged.
    batch: Vec<Vert>,
    batch_tex: Option<TextureId>,
    batch_mode: Mode,
    batch_uniforms: Option<Uniforms>,
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
            .expect("shader");
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
        let mut pipe = |depth_test: Comparison, depth_write: bool, color_blend: Option<BlendState>| {
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
        let solid = pipe(Comparison::Less, true, None);
        let overlay = pipe(Comparison::Less, false, blend);
        let flat = pipe(Comparison::Always, false, blend);
        let white = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);
        let stream_cap = 4096;
        let stream_vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vert>(stream_cap));
        let stream_ib = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&seq_indices(stream_cap)));
        Gfx {
            ctx,
            solid,
            overlay,
            flat,
            white,
            stream_vb,
            stream_ib,
            stream_cap,
            batch: Vec::new(),
            batch_tex: None,
            batch_mode: Mode::Flat,
            batch_uniforms: None,
        }
    }

    fn pipeline(&self, mode: Mode) -> &Pipeline {
        match mode {
            Mode::Solid => &self.solid,
            Mode::Overlay => &self.overlay,
            Mode::Flat => &self.flat,
        }
    }

    /// A texture from RGBA pixels. `mipmaps` for terrain textures (trilinear like the original), clamped at the edges.
    pub fn texture(&mut self, img: &Rgba, mipmaps: bool) -> TextureId {
        // transparent pixels still carry the art's key colour (magenta); smoothing would blend it into the edges
        let bled;
        let img = if img.px.chunks_exact(4).any(|p| p[3] == 0) {
            bled = bleed_alpha(img);
            &bled
        } else {
            img
        };
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
        id
    }

    /// A texture from an HD pack picture (only used in HD mode, see hd.rs): trilinear filtered with mipmaps, since it is drawn
    /// smaller than its own resolution. The pack's pictures carry their colours under transparent pixels already.
    pub fn texture_hd(&mut self, img: &Rgba) -> TextureId {
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
        let p = *self.pipeline(mode);
        self.ctx.apply_pipeline(&p);
        self.ctx.apply_bindings(&Bindings { vertex_buffers: vec![mesh.vb], index_buffer: mesh.ib, images: vec![tex] });
        self.ctx.apply_uniforms(UniformsSource::table(u));
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
        self.batch.extend_from_slice(verts);
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
        let p = *self.pipeline(self.batch_mode);
        self.ctx.apply_pipeline(&p);
        self.ctx.apply_bindings(&Bindings { vertex_buffers: vec![self.stream_vb], index_buffer: self.stream_ib, images: vec![tex] });
        self.ctx.apply_uniforms(UniformsSource::table(&u));
        self.ctx.draw(0, self.batch.len() as i32, 1);
        self.batch.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
