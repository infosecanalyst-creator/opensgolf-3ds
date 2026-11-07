//! Rendering on the 3DS: the same public surface as `simgolf/src/gfx.rs` (Mat4, Vert, Uniforms, Mode, Mesh, TextureId, Gfx
//! with texture / mesh / tris / quad / flush / draw_mesh), backed by citro3d instead of miniquad, plus the two screens and the
//! stereoscopic top screen. `render.rs`, `app.rs` and the `*_ui.rs` modules of the game crate call only this surface, so they
//! draw through here unchanged.
//!
//! Two vertex shaders:
//!   * `shader/main.v.pica` for everything the game draws call by call (`Vert`: position, uv, normal, colour; lit or not).
//!   * `shader/terrain.v.pica` for the course: `TerrainVert` is 24 bytes (position, uv, and the lighting worked out once
//!     when the mesh is built), half of `Vert`, and the shader does a few multiply-adds instead of the full light model.
//!
//! Hardware rules learned on a New 3DS XL (emulators accept all of these; the console hangs or crashes):
//!   * Every shader output register must be written with full-mask writes; `mov out.xy, ...` alone hangs the GPU.
//!   * citro3d keeps pointers to the bound program and bound textures: both are boxed and never move.
//!   * Vertex data written by the CPU must be flushed from the data cache before the GPU reads it.
//!   * Vertex buffers are addressed from a base: the base is set to the buffer itself (the heap can sit above what
//!     citro3d's default base reaches).
//!
//! What is deliberately not here: the Enhanced graphics pipelines (premultiplied, SDF edges, the fx shaders). The 3DS build is
//! Classic rendering only: `is_premul` and `is_sdf` always say no, `text_atlas` and `texture_hd` fall back to plain textures.
use crate::screens::{Screen, BOTTOM_H, BOTTOM_W, TOP_H, TOP_W};
use citro3d::render::{ClearFlags, ScreenTarget, Target};
use citro3d_sys as c3d;
use ctru::linear::LinearAllocator;
use ctru::services::gfx::{Screen as _, TopScreen3D};
use sg_core::assets::Rgba;

// ---- the surface the game crate uses ---------------------------------------------------------------------------------------

/// 4x4 matrix, column-major (m[col * 4 + row]) like OpenGL; identical to the desktop gfx.rs so `render.rs` compiles as is.
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
    /// The point (x, y, z, 1) through this matrix: (x, y, z, w) out.
    pub fn apply(&self, p: [f32; 3]) -> [f32; 4] {
        let m = &self.0;
        let mut o = [0.0; 4];
        for (r, v) in o.iter_mut().enumerate() {
            *v = m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
        }
        o
    }

    /// What the PICA200 needs on top of an OpenGL-style clip matrix: clip z in -1..0 instead of -1..1, and the 90 degree turn
    /// into the 3DS's portrait framebuffers (what citro3d's `Mtx_*Tilt` functions fold into their projections).
    pub fn pica_fixup() -> Mat4 {
        // z' = 0.5 z - 0.5
        let mut z = Mat4::identity().0;
        z[10] = 0.5;
        z[14] = -0.5;
        // (x, y) -> (y, -x)
        let mut t = [0.0; 16];
        t[4] = 1.0; // col 1 (y) into row 0
        t[1] = -1.0; // col 0 (x) into row 1
        t[10] = 1.0;
        t[15] = 1.0;
        Mat4(t).mul(&Mat4(z))
    }
}

/// A vertex of everything drawn call by call (the interface, sprites, the ball): 48 bytes, fed to the GPU as four float3s.
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

/// A course vertex: position, uv and its lighting, 24 bytes (two float3s, the layout the stock citro3d examples use).
/// `light` packs the two per-vertex lighting terms as integers: `diffuse * 255 * 256 + specular * 255` (both 0..1, 8 bits
/// each; the GPU's 24-bit floats hold integers to 2^17 exactly). See `TerrainVert::pack_light` and `shader/terrain.v.pica`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainVert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub light: f32,
}

impl TerrainVert {
    pub fn pack_light(diffuse: f32, specular: f32) -> f32 {
        let d = (diffuse.clamp(0.0, 1.0) * 255.0).round();
        let s = (specular.clamp(0.0, 1.0) * 255.0).round();
        d * 256.0 + s
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

/// What the terrain shader needs per draw: the matrix and the three light colours its per-vertex terms scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainUniforms {
    pub mvp: [f32; 16],
    /// 0.04 + 0.2 * ambient (with the brightness calibration folded in), rgb.
    pub ka: [f32; 3],
    /// 0.8 * diffuse (brightness folded in), rgb.
    pub kd: [f32; 3],
    /// The specular colour, rgb.
    pub ks: [f32; 3],
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

/// A texture handle: miniquad's own type (the 3DS stand-in for miniquad, vendor/miniquad-3ds), holding this renderer's slot
/// number, so the game's code, which keeps miniquad::TextureId everywhere, holds these as they are.
pub use miniquad::TextureId;

/// Static geometry in linear memory (the GPU reads it in place), uploaded once per rebuild.
pub struct Mesh {
    verts: Vec<Vert, LinearAllocator>,
}

/// Static course geometry (see `TerrainVert`).
pub struct TerrainMesh {
    verts: Vec<TerrainVert, LinearAllocator>,
}

impl TerrainMesh {
    pub fn len(&self) -> usize {
        self.verts.len()
    }
}

/// Which eye the top screen is being drawn for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eye {
    Left,
    Right,
}

// ---- textures ------------------------------------------------------------------------------------------------------------

struct Tex {
    raw: c3d::C3D_Tex,
    /// The picture's size over the power-of-two texture it sits in: the shader multiplies UVs by this.
    uv_scale: [f32; 2],
    /// What `update_texture` needs to write a new picture into the same texture.
    pw: u32,
    ph: u32,
    fmt: TexFormat,
    on_vram: bool,
}

/// Rounds up to the PICA's 8..1024 power-of-two texture sizes.
fn pot(n: u32) -> u32 {
    n.max(8).next_power_of_two().min(1024)
}

/// Halves a picture (box filter) until it fits 1024 x 1024. Sprite sheets wider than that lose detail; the proper fix is to cut
/// such sheets into several textures in `sg_core::sprites` (an open item in README.md).
fn fit_1024(img: &Rgba) -> std::borrow::Cow<'_, Rgba> {
    if img.w <= 1024 && img.h <= 1024 {
        return std::borrow::Cow::Borrowed(img);
    }
    let (w, h) = (img.w / 2, img.h / 2);
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            for c in 0..4 {
                let s = |dx: u32, dy: u32| img.px[(((y * 2 + dy) * img.w + x * 2 + dx) * 4 + c) as usize] as u32;
                px[((y * w + x) * 4 + c) as usize] = ((s(0, 0) + s(1, 0) + s(0, 1) + s(1, 1) + 2) / 4) as u8;
            }
        }
    }
    let half = Rgba { w, h, px };
    std::borrow::Cow::Owned(fit_1024(&half).into_owned())
}

/// The picture in the top-left corner of a `pw` x `ph` canvas (the rest transparent black).
fn pad(img: &Rgba, pw: u32, ph: u32) -> Rgba {
    if img.w == pw && img.h == ph {
        return img.clone();
    }
    let mut px = vec![0u8; (pw * ph * 4) as usize];
    for y in 0..img.h.min(ph) {
        let n = (img.w.min(pw) * 4) as usize;
        let s = (y * img.w * 4) as usize;
        let d = (y * pw * 4) as usize;
        px[d..d + n].copy_from_slice(&img.px[s..s + n]);
    }
    Rgba { w: pw, h: ph, px }
}

/// The next mipmap level: half the size, each texel the average of a 2 x 2 block. Blocks are aligned, so pictures packed
/// into an atlas on a power-of-two grid stay apart until the level where a cell is one texel.
fn half(img: &Rgba) -> Rgba {
    let (w, h) = ((img.w / 2).max(1), (img.h / 2).max(1));
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            for c in 0..4 {
                let s = |dx: u32, dy: u32| {
                    let (sx, sy) = ((x * 2 + dx).min(img.w - 1), (y * 2 + dy).min(img.h - 1));
                    img.px[((sy * img.w + sx) * 4 + c) as usize] as u32
                };
                px[((y * w + x) * 4 + c) as usize] = ((s(0, 0) + s(1, 0) + s(0, 1) + s(1, 1) + 2) / 4) as u8;
            }
        }
    }
    Rgba { w, h, px }
}

/// The texture formats used: 16-bit where the picture allows it (half the memory and the GPU's texture bandwidth).
#[derive(Clone, Copy, PartialEq)]
enum TexFormat {
    /// Fully opaque pictures (the course textures).
    Rgb565,
    /// Cut-out pictures: alpha only 0 or 255.
    Rgba5551,
    /// Soft alpha edges: kept at full precision.
    Rgba8,
}

impl TexFormat {
    fn of(img: &Rgba) -> TexFormat {
        let mut opaque = true;
        for a in img.px.iter().skip(3).step_by(4) {
            match *a {
                255 => {}
                0 => opaque = false,
                _ => return TexFormat::Rgba8,
            }
        }
        if opaque {
            TexFormat::Rgb565
        } else {
            TexFormat::Rgba5551
        }
    }
    fn gpu(self) -> ctru_sys::GPU_TEXCOLOR {
        match self {
            TexFormat::Rgb565 => ctru_sys::GPU_RGB565,
            TexFormat::Rgba5551 => ctru_sys::GPU_RGBA5551,
            TexFormat::Rgba8 => ctru_sys::GPU_RGBA8,
        }
    }
    fn bytes(self) -> u32 {
        if self == TexFormat::Rgba8 {
            4
        } else {
            2
        }
    }
}

/// Lays pixels out the way the PICA reads a texture: 8 x 8 tiles in row order, texels inside a tile in Morton (Z) order, rows
/// bottom-up; RGBA8 as bytes A B G R, the 16-bit formats as little-endian words. Padded to `pw` x `ph`. In linear memory, so
/// the GPU's copy engine can move it into VRAM.
fn swizzle(img: &Rgba, pw: u32, ph: u32, fmt: TexFormat) -> Vec<u8, LinearAllocator> {
    let bpp = fmt.bytes() as usize;
    let mut out = Vec::with_capacity_in((pw * ph) as usize * bpp, LinearAllocator);
    out.resize((pw * ph) as usize * bpp, 0);
    const fn morton(x: u32, y: u32) -> u32 {
        let (mut x, mut y, mut r, mut i) = (x, y, 0u32, 0);
        while i < 3 {
            r |= (x & 1) << (2 * i) | (y & 1) << (2 * i + 1);
            x >>= 1;
            y >>= 1;
            i += 1;
        }
        r
    }
    for y in 0..img.h.min(ph) {
        let ty = ph - 1 - y; // bottom-up
        for x in 0..img.w.min(pw) {
            let s = ((y * img.w + x) * 4) as usize;
            let (r, g, b, a) = (img.px[s], img.px[s + 1], img.px[s + 2], img.px[s + 3]);
            let tile = (ty / 8) * (pw / 8) + x / 8;
            let d = (tile * 64 + morton(x & 7, ty & 7)) as usize * bpp;
            match fmt {
                TexFormat::Rgba8 => out[d..d + 4].copy_from_slice(&[a, b, g, r]),
                TexFormat::Rgb565 => {
                    let v = (r as u16 >> 3) << 11 | (g as u16 >> 2) << 5 | b as u16 >> 3;
                    out[d..d + 2].copy_from_slice(&v.to_le_bytes());
                }
                TexFormat::Rgba5551 => {
                    let v = (r as u16 >> 3) << 11 | (g as u16 >> 3) << 6 | (b as u16 >> 3) << 1 | (a >= 128) as u16;
                    out[d..d + 2].copy_from_slice(&v.to_le_bytes());
                }
            }
        }
    }
    out
}

/// Writes a picture into a swizzled texture of `page` x `page` texels (the layout `swizzle` makes) with its top-left texel
/// at (`ox`, `oy`), counted from the top. Safety: `dst` holds the whole texture.
unsafe fn blit_swizzled(dst: *mut u8, page: u32, fmt: TexFormat, img: &Rgba, ox: u32, oy: u32) {
    let bpp = fmt.bytes() as usize;
    const fn morton(x: u32, y: u32) -> u32 {
        let (mut x, mut y, mut r, mut i) = (x, y, 0u32, 0);
        while i < 3 {
            r |= (x & 1) << (2 * i) | (y & 1) << (2 * i + 1);
            x >>= 1;
            y >>= 1;
            i += 1;
        }
        r
    }
    for y in 0..img.h {
        let py = oy + y;
        if py >= page {
            break;
        }
        let ty = page - 1 - py; // bottom-up
        for x in 0..img.w {
            let px = ox + x;
            if px >= page {
                break;
            }
            let s = ((y * img.w + x) * 4) as usize;
            let (r, g, b, a) = (img.px[s], img.px[s + 1], img.px[s + 2], img.px[s + 3]);
            let tile = (ty / 8) * (page / 8) + px / 8;
            let d = dst.add((tile * 64 + morton(px & 7, ty & 7)) as usize * bpp);
            match fmt {
                TexFormat::Rgba8 => std::ptr::copy_nonoverlapping([a, b, g, r].as_ptr(), d, 4),
                TexFormat::Rgb565 => {
                    let v = (r as u16 >> 3) << 11 | (g as u16 >> 2) << 5 | b as u16 >> 3;
                    std::ptr::copy_nonoverlapping(v.to_le_bytes().as_ptr(), d, 2);
                }
                TexFormat::Rgba5551 => {
                    let v = (r as u16 >> 3) << 11 | (g as u16 >> 3) << 6 | (b as u16 >> 3) << 1 | (a >= 128) as u16;
                    std::ptr::copy_nonoverlapping(v.to_le_bytes().as_ptr(), d, 2);
                }
            }
        }
    }
}

// ---- the renderer ----------------------------------------------------------------------------------------------------------

/// Set when stdout goes to the PC over 3dslink (`run-console.ps1`): `trace` then prints there instead of writing the card.
pub static NET_LOG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A startup or first-frame marker: to the PC over 3dslink when launched that way, else appended to
/// `sdmc:/3ds/opensgolf/stage.txt` (so a hang on the console leaves a trail). Only startup and frame 1 call it.
/// newlib's `mallinfo`: what the heap has handed out and what it holds free.
#[repr(C)]
struct Mallinfo {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

extern "C" {
    fn mallinfo() -> Mallinfo;
    static __ctru_heap_size: u32;
}

/// The memory left, for the log: the heap (the program's allocations) and linear memory (textures, vertex buffers).
pub fn mem_line() -> String {
    let mi = unsafe { mallinfo() };
    let heap = unsafe { __ctru_heap_size } as usize;
    let free = heap.saturating_sub(mi.arena) + mi.fordblks;
    let linear = unsafe { ctru_sys::linearSpaceFree() } as usize;
    format!("heap {:.1} MB used, {:.1} MB free of {:.0}; linear {:.1} MB free", mi.uordblks as f32 / 1048576.0, free as f32 / 1048576.0, heap as f32 / 1048576.0, linear as f32 / 1048576.0)
}

pub fn trace(stage: &str) {
    println!("[stage] {stage}");
    if NET_LOG.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("sdmc:/3ds/opensgolf/stage.txt") {
        let _ = writeln!(f, "{stage}");
    }
}

/// A shader program parsed from a picasso binary embedded at build time.
struct Program {
    /// Boxed: citro3d keeps the bound program's pointer and dereferences it at the next bind, so it must never move.
    prog: Box<ctru_sys::shaderProgram_s>,
    dvlb: *mut ctru_sys::DVLB_s,
    attrs: c3d::C3D_AttrInfo,
}

impl Program {
    /// `layout`: the float component count of each vertex attribute, in shader input order.
    fn load(bin: &'static [u8], layout: &[i32]) -> Program {
        let words: Vec<u32> = bin.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        // DVLB_ParseFile keeps pointers into the words: they live for the rest of the program
        let words = Box::leak(words.into_boxed_slice());
        unsafe {
            let dvlb = ctru_sys::DVLB_ParseFile(words.as_mut_ptr(), bin.len() as u32);
            assert!(!dvlb.is_null(), "a vertex shader did not parse");
            let mut prog: Box<ctru_sys::shaderProgram_s> = Box::new(std::mem::zeroed());
            ctru_sys::shaderProgramInit(&mut *prog);
            ctru_sys::shaderProgramSetVsh(&mut *prog, (*dvlb).DVLE);
            let mut attrs: c3d::C3D_AttrInfo = std::mem::zeroed();
            c3d::AttrInfo_Init(&mut attrs);
            for (i, &n) in layout.iter().enumerate() {
                c3d::AttrInfo_AddLoader(&mut attrs, i as i32, ctru_sys::GPU_FLOAT, n);
            }
            Program { prog, dvlb, attrs }
        }
    }
    fn slot(&self, name: &str) -> i32 {
        let c = std::ffi::CString::new(name).unwrap();
        let i = unsafe { ctru_sys::shaderInstanceGetUniformLocation(self.prog.vertexShader, c.as_ptr()) } as i32;
        assert!(i >= 0, "uniform {name} not found in a vertex shader");
        i
    }
}

/// `shader/main.v.pica`'s uniform registers.
struct MainSlots {
    mvp: i32,
    mv: i32,
    lamb: i32,
    ldif: i32,
    lspec: i32,
    ldir: i32,
    uvs: i32,
}

/// `shader/terrain.v.pica`'s uniform registers.
struct TerrainSlots {
    mvp: i32,
    uvs: i32,
    ka: i32,
    kd: i32,
    ks: i32,
}

#[derive(Clone, Copy, PartialEq)]
enum Bound {
    None,
    Main,
    Terrain,
}

pub struct Gfx {
    /// The game's direct GPU calls (its desktop visual effects, which stay off on the 3DS) go to a GPU that draws nothing.
    pub ctx: Box<dyn miniquad::RenderingBackend>,
    /// The game's Enhanced graphics pipelines: never made on the 3DS.
    pub fx: Option<Box<crate::fx::FxGpu>>,
    _inst: citro3d::Instance,
    main: Program,
    main_slots: MainSlots,
    terrain: Program,
    terrain_slots: TerrainSlots,
    bound: Bound,
    frame_no: u32,
    top_left: ScreenTarget<'static>,
    top_right: ScreenTarget<'static>,
    bottom: ScreenTarget<'static>,
    /// Boxed: citro3d keeps a pointer to the bound texture's `C3D_Tex` in its context and reads it at the next draw, so a
    /// texture must never move once bound.
    textures: Vec<Option<Box<Tex>>>,
    /// How many textures went into VRAM (the GPU's own, faster memory) and how many stayed in main memory.
    pub tex_vram: u32,
    pub tex_linear: u32,
    pub white: TextureId,
    /// Immediate-mode batch: vertices of consecutive draws with the same texture, state and uniforms are merged.
    batch: Vec<Vert>,
    batch_tex: Option<TextureId>,
    batch_mode: Mode,
    batch_uniforms: Option<Uniforms>,
    /// Two frames' worth of streamed vertices in linear memory: the GPU may still read last frame's while this one fills.
    stream: [Vec<Vert, LinearAllocator>; 2],
    stream_frame: usize,
    /// The screen draws currently land on, and the eye for the top screen.
    screen: Screen,
    eye: Eye,
    /// The 3D slider, 0..1, read once per frame.
    slider: f32,
    /// ZL: the top screen's picture is on the touchscreen and the other way round (3D off).
    swapped: bool,
    /// Draws are the world (sheared per eye) rather than the interface (at screen depth).
    world: bool,
    /// The uniforms and UV scale last sent to the GPU, so a draw with the same ones sends nothing (the course is hundreds of
    /// draws with identical matrices and lights). Cleared whenever the program changes.
    sent: Option<(Mat4, [f32; 16], f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3])>,
    sent_terrain: Option<(Mat4, [f32; 3], [f32; 3], [f32; 3])>,
    sent_uvs: Option<[f32; 2]>,
    /// The terrain height (in clip-z units) that gets the full eye separation (`set_stereo_span`).
    stereo_span: f32,
    /// The fixed state last applied, so a batch with the same one skips the GPU register writes.
    mode_applied: Option<(Mode, bool)>,
    /// Meshes deleted this frame, freed at the next frame's start, once the GPU has finished reading them.
    retired: Vec<Mesh>,
    retired_terrain: Vec<TerrainMesh>,
    /// Draw calls issued this frame (for the frame-time readout).
    pub draws: u32,
    /// The vertex buffer the GPU is pointed at, so draws from the same one skip the set-up.
    buf_bound: *const u8,
    /// Between `begin_frame` and `end_frame`: textures made now (the game makes its sprites' textures as it first draws
    /// them) go into main memory. A VRAM texture is filled by the GPU's copy engine through the same queue the frame's
    /// commands are in, and filling one mid-frame stopped the program (svcBreak under C3D_TexLoadImage, Azahar).
    in_frame: bool,
    /// Sprite pictures packed into shared pages (`Atlas`): per texture slot, the page and where the picture sits on it.
    subs: Vec<Option<Sub>>,
    atlas: Vec<Page>,
}

/// A picture on an atlas page: the page's texture and the picture's rectangle there in UV units (x0, y0, x1, y1).
#[derive(Clone, Copy)]
struct Sub {
    page: TextureId,
    pi: usize,
    rect: [f32; 4],
}

/// One atlas page: a PAGE x PAGE texture in main memory (written in place by the CPU), filled shelf by shelf.
struct Page {
    tex: TextureId,
    fmt: TexFormat,
    x: u32,
    y: u32,
    shelf_h: u32,
    /// Pictures on it still in use. When the last is deleted (the game clears its cache of golfers' outfit frames) the page
    /// is cleared at the next frame's start and filled again.
    live: u32,
    retired: bool,
}

/// The atlas: the game makes a texture for every sprite frame it draws, and a course view of trees, golfers and their
/// shadows alternates between dozens of them, so each sprite was its own draw call (1,900 a frame at a close zoom, most of
/// the frame time). On shared pages, neighbouring sprites batch into one draw.
const PAGE: u32 = 512;
/// The largest picture put on a page; bigger ones keep their own texture.
const SUB_MAX: u32 = 192;

const STREAM_CAP: usize = 24_000; // vertices per frame; a full course view with hundreds of sprites is a few thousand
const FRAME_TRACE: u32 = 1; // frames whose steps go to `trace`

impl Gfx {
    /// Takes over both screens. `gfx` is the libctru graphics service, leaked to `'static` so the render targets (which borrow
    /// the screens) can live in this struct for the rest of the program.
    pub fn new(gfx: &'static ctru::services::gfx::Gfx) -> Gfx {
        let _ = std::fs::remove_file("sdmc:/3ds/opensgolf/stage.txt");
        trace("gfx: C3D_Init");
        // 2 MB of GPU commands per frame (citro3d's default of 256 KB overflows with two eyes of a full course, and an
        // overflow is a svcBreak inside GPUCMD_Add)
        let inst = citro3d::Instance::with_cmdbuf_size(0x200000).expect("citro3d");
        let depth = Some(citro3d::render::DepthFormat::Depth24Stencil8);
        // the top screen as two eyes (gfxSet3D(true)); the slider decides whether the right one is drawn
        let top3d: &'static TopScreen3D<'static> = Box::leak(Box::new(TopScreen3D::from(&gfx.top_screen)));
        let (mut left, mut right) = top3d.split_mut();
        let fb = left.raw_framebuffer();
        let (lw, lh) = (fb.width as usize, fb.height as usize);
        let top_left = inst.render_target(lw, lh, left, depth).expect("top-left target");
        let fb = right.raw_framebuffer();
        let (rw, rh) = (fb.width as usize, fb.height as usize);
        let top_right = inst.render_target(rw, rh, right, depth).expect("top-right target");
        let mut bottom_screen = gfx.bottom_screen.borrow_mut();
        let fb = bottom_screen.raw_framebuffer();
        let (bw, bh) = (fb.width as usize, fb.height as usize);
        let bottom = inst.render_target(bw, bh, bottom_screen, depth).expect("bottom target");
        trace("gfx: targets ok");

        // `Vert` (12 floats) is fed as FOUR FLOAT3s, (x y z) (u v nx) (ny nz r) (g b a), which the shader reassembles; the
        // course's `TerrainVert` as two, (x y z) (u v light): layouts proven on the console.
        let main = Program::load(include_bytes!(concat!(env!("OUT_DIR"), "/main.shbin")), &[3, 3, 3, 3]);
        let main_slots = MainSlots {
            mvp: main.slot("mvp"),
            mv: main.slot("mv"),
            lamb: main.slot("lamb"),
            ldif: main.slot("ldif"),
            lspec: main.slot("lspec"),
            ldir: main.slot("ldir"),
            uvs: main.slot("uvs"),
        };
        let terrain = Program::load(include_bytes!(concat!(env!("OUT_DIR"), "/terrain.shbin")), &[3, 3]);
        let terrain_slots = TerrainSlots {
            mvp: terrain.slot("mvp"),
            uvs: terrain.slot("uvs"),
            ka: terrain.slot("ka"),
            kd: terrain.slot("kd"),
            ks: terrain.slot("ks"),
        };
        unsafe { c3d::C3D_CullFace(ctru_sys::GPU_CULL_NONE) };
        trace("gfx: shaders ok");

        let mut g = Gfx {
            _inst: inst,
            main,
            main_slots,
            terrain,
            terrain_slots,
            bound: Bound::None,
            frame_no: 0,
            top_left,
            top_right,
            bottom,
            textures: Vec::new(),
            tex_vram: 0,
            tex_linear: 0,
            white: TextureId::from_index(0),
            ctx: miniquad::window::new_rendering_backend(),
            fx: None,
            batch: Vec::new(),
            batch_tex: None,
            batch_mode: Mode::Flat,
            batch_uniforms: None,
            stream: [Vec::with_capacity_in(STREAM_CAP, LinearAllocator), Vec::with_capacity_in(STREAM_CAP, LinearAllocator)],
            stream_frame: 0,
            screen: Screen::Top,
            eye: Eye::Left,
            slider: 0.0,
            swapped: false,
            world: false,
            sent: None,
            sent_terrain: None,
            sent_uvs: None,
            stereo_span: 0.25,
            mode_applied: None,
            retired: Vec::new(),
            retired_terrain: Vec::new(),
            draws: 0,
            buf_bound: std::ptr::null(),
            in_frame: false,
            subs: Vec::new(),
            atlas: Vec::new(),
        };
        g.white = g.texture(&Rgba { w: 8, h: 8, px: vec![255; 8 * 8 * 4] }, false);
        trace("gfx: ready");
        g
    }

    // -- textures -----------------------------------------------------------------------------------------------------------

    /// Classic only: nothing is premultiplied or distance-field on the 3DS.
    pub fn is_premul(&self, _tex: TextureId) -> bool {
        false
    }
    pub fn is_sdf(&self, _tex: TextureId) -> bool {
        false
    }

    /// A texture from RGBA pixels. `mipmaps` for terrain textures (trilinear like the original), clamped at the edges. Stored
    /// 16-bit where the picture allows (`TexFormat::of`), and in VRAM while it has room (main memory after that).
    pub fn texture(&mut self, img: &Rgba, mipmaps: bool) -> TextureId {
        if !mipmaps && img.w <= SUB_MAX && img.h <= SUB_MAX && img.w > 0 && img.h > 0 {
            let fmt = TexFormat::of(img);
            // cut-outs and soft-edged pictures (sprites and their shadows); opaque ones (the white texture) stay alone
            // all on full-colour pages: a sprite's shadow (soft) and body (cut-out) are drawn one after the other, and pages
            // of two formats would split every such pair into two draws again
            if fmt != TexFormat::Rgb565 {
                if let Some(id) = self.sub_texture(img, TexFormat::Rgba8) {
                    return id;
                }
            }
        }
        self.texture_levels(img, if mipmaps { u8::MAX } else { 0 })
    }

    /// A free texture slot (a slot is free when it holds neither a texture nor an atlas picture).
    fn free_slot(&mut self) -> usize {
        let n = self.textures.len().max(self.subs.len());
        self.textures.resize_with(n, || None);
        self.subs.resize_with(n, || None);
        match (0..n).find(|&i| self.textures[i].is_none() && self.subs[i].is_none()) {
            Some(i) => i,
            None => {
                self.textures.push(None);
                self.subs.push(None);
                n
            }
        }
    }

    /// Puts a picture on an atlas page of its format (a new page when none has room); its texture id.
    fn sub_texture(&mut self, img: &Rgba, fmt: TexFormat) -> Option<TextureId> {
        // a cell of whole 8 x 8 tiles with a clear texel all round, so filtering never reaches a neighbour's texels
        let (cw, ch) = ((img.w + 2 + 7) & !7, (img.h + 2 + 7) & !7);
        let mut pi = None;
        for (i, p) in self.atlas.iter_mut().enumerate().rev() {
            if p.fmt != fmt || p.retired {
                continue;
            }
            if p.x + cw > PAGE {
                // the next shelf
                p.y += p.shelf_h;
                p.x = 0;
                p.shelf_h = 0;
            }
            if p.y + ch <= PAGE {
                pi = Some(i);
                break;
            }
        }
        let pi = match pi {
            Some(i) => i,
            None => {
                let tex = self.page_texture(fmt)?;
                self.atlas.push(Page { tex, fmt, x: 0, y: 0, shelf_h: 0, live: 0, retired: false });
                self.atlas.len() - 1
            }
        };
        let (ox, oy, page) = {
            let p = &mut self.atlas[pi];
            let at = (p.x, p.y);
            p.x += cw;
            p.shelf_h = p.shelf_h.max(ch);
            p.live += 1;
            (at.0 + 1, at.1 + 1, p.tex)
        };
        let Some(Some(t)) = self.textures.get_mut(page.index()) else { return None };
        unsafe {
            let data = t.raw.__bindgen_anon_1.data as *mut u8;
            blit_swizzled(data, PAGE, fmt, img, ox, oy);
            c3d::C3D_TexFlush(&mut t.raw);
        }
        let f = PAGE as f32;
        let sub = Sub { page, pi, rect: [ox as f32 / f, oy as f32 / f, (ox + img.w) as f32 / f, (oy + img.h) as f32 / f] };
        let i = self.free_slot();
        self.subs[i] = Some(sub);
        let id = TextureId::from_index(i);
        miniquad::register_texture(id, img.w, img.h);
        Some(id)
    }

    /// An empty (transparent) atlas page in main memory, where the CPU can write pictures into it at any time.
    fn page_texture(&mut self, fmt: TexFormat) -> Option<TextureId> {
        let mut raw: c3d::C3D_Tex = unsafe { std::mem::zeroed() };
        unsafe {
            let mut params: c3d::C3D_TexInitParams = std::mem::zeroed();
            params.width = PAGE as u16;
            params.height = PAGE as u16;
            params.set_maxLevel(0);
            params.set_type(ctru_sys::GPU_TEX_2D);
            params.set_format(fmt.gpu());
            params.set_onVram(false);
            if !c3d::C3D_TexInitWithParams(&mut raw, std::ptr::null_mut(), params) {
                return None;
            }
            let data = raw.__bindgen_anon_1.data as *mut u8;
            std::ptr::write_bytes(data, 0, (PAGE * PAGE * fmt.bytes()) as usize);
            c3d::C3D_TexFlush(&mut raw);
            c3d::C3D_TexSetFilter(&mut raw, ctru_sys::GPU_LINEAR, ctru_sys::GPU_LINEAR);
            c3d::C3D_TexSetWrap(&mut raw, ctru_sys::GPU_CLAMP_TO_EDGE, ctru_sys::GPU_CLAMP_TO_EDGE);
        }
        self.tex_linear += 1;
        let t = Box::new(Tex { raw, uv_scale: [1.0, 1.0], pw: PAGE, ph: PAGE, fmt, on_vram: false });
        let i = self.free_slot();
        self.textures[i] = Some(t);
        Some(TextureId::from_index(i))
    }

    /// `texture` with at most `max_level` mipmap levels below the picture (an atlas stops while its cells are still
    /// several texels wide, so neighbouring cells do not blend into each other).
    pub fn texture_levels(&mut self, img: &Rgba, max_level: u8) -> TextureId {
        let img = fit_1024(img);
        let (pw, ph) = (pot(img.w), pot(img.h));
        let fmt = TexFormat::of(&img);
        let levels = if max_level > 0 { (unsafe { c3d::C3D_TexCalcMaxLevel(pw, ph) } as u8).min(max_level) } else { 0 };
        let mut raw: c3d::C3D_Tex = unsafe { std::mem::zeroed() };
        let mut in_vram = false;
        unsafe {
            let mut params: c3d::C3D_TexInitParams = std::mem::zeroed();
            params.width = pw as u16;
            params.height = ph as u16;
            params.set_maxLevel(levels);
            params.set_type(ctru_sys::GPU_TEX_2D);
            params.set_format(fmt.gpu());
            params.set_onVram(!self.in_frame);
            let on_vram = !self.in_frame && c3d::C3D_TexInitWithParams(&mut raw, std::ptr::null_mut(), params);
            if !on_vram {
                params.set_onVram(false);
                raw = std::mem::zeroed();
                assert!(c3d::C3D_TexInitWithParams(&mut raw, std::ptr::null_mut(), params), "texture {pw}x{ph}: out of memory");
            }
            // Every level is made here on the CPU (a 2 x 2 box filter) and loaded as is. citro3d's own mipmap generation
            // goes through the GPU's display-transfer engine and gave black levels for RGB565 textures (Azahar).
            let mut level = pad(&img, pw, ph);
            for l in 0..=levels {
                if l > 0 {
                    level = half(&level);
                }
                let data = swizzle(&level, level.w, level.h, fmt);
                // VRAM: citro3d copies each level in with the GPU's copy engine, which reads RAM, not the CPU's cache
                let _ = ctru_sys::GSPGPU_FlushDataCache(data.as_ptr() as *const _, data.len() as u32);
                c3d::C3D_TexLoadImage(&mut raw, data.as_ptr() as *const _, ctru_sys::GPU_TEXFACE_2D, l as i32);
            }
            in_vram = on_vram;
            if on_vram {
                self.tex_vram += 1;
            } else {
                c3d::C3D_TexFlush(&mut raw);
                self.tex_linear += 1;
            }
            if levels > 0 {
                c3d::C3D_TexSetFilterMipmap(&mut raw, ctru_sys::GPU_LINEAR);
            }
            c3d::C3D_TexSetFilter(&mut raw, ctru_sys::GPU_LINEAR, ctru_sys::GPU_LINEAR);
            c3d::C3D_TexSetWrap(&mut raw, ctru_sys::GPU_CLAMP_TO_EDGE, ctru_sys::GPU_CLAMP_TO_EDGE);
        }
        let t = Box::new(Tex { raw, uv_scale: [img.w as f32 / pw as f32, img.h as f32 / ph as f32], pw, ph, fmt, on_vram: in_vram });
        let i = self.free_slot();
        self.textures[i] = Some(t);
        let id = TextureId::from_index(i);
        // the game asks its GPU handle for texture sizes (`ctx.texture_size`): the picture's, as its desktop renderer has it
        miniquad::register_texture(id, img.w, img.h);
        id
    }
    /// A picture drawn texel for texel (nearest filtering, no mipmaps), which `update_texture` can redraw: the bottom
    /// screen's course plan, one texel per tile.
    pub fn texture_crisp(&mut self, img: &Rgba) -> TextureId {
        let id = self.texture_levels(img, 0);
        if let Some(Some(t)) = self.textures.get_mut(id.index()) {
            unsafe { c3d::C3D_TexSetFilter(&mut t.raw, ctru_sys::GPU_NEAREST, ctru_sys::GPU_NEAREST) };
        }
        id
    }

    /// Writes a new picture into a texture made by `texture_crisp`: the same size, and opaque if the first one was. Call it
    /// after `begin_frame`, when the GPU has finished the last frame and is not reading the texture.
    pub fn update_texture(&mut self, id: TextureId, img: &Rgba) {
        let Some(Some(t)) = self.textures.get_mut(id.index()) else { return };
        if pot(img.w) != t.pw || pot(img.h) != t.ph || TexFormat::of(img) != t.fmt {
            return;
        }
        let data = swizzle(&pad(img, t.pw, t.ph), t.pw, t.ph, t.fmt);
        unsafe {
            let _ = ctru_sys::GSPGPU_FlushDataCache(data.as_ptr() as *const _, data.len() as u32);
            c3d::C3D_TexLoadImage(&mut t.raw, data.as_ptr() as *const _, ctru_sys::GPU_TEXFACE_2D, 0);
            if !t.on_vram {
                c3d::C3D_TexFlush(&mut t.raw);
            }
        }
    }

    pub fn texture_classic(&mut self, img: &Rgba) -> TextureId {
        self.texture(img, false)
    }
    pub fn text_atlas(&mut self, img: &Rgba) -> TextureId {
        self.texture(img, false)
    }
    pub fn texture_hd(&mut self, img: &Rgba) -> TextureId {
        self.texture(img, true)
    }
    pub fn delete_texture(&mut self, id: TextureId) {
        // a picture on an atlas page: its slot is freed (its cell stays used until the program ends)
        if let Some(s) = self.subs.get_mut(id.index()) {
            if let Some(sub) = s.take() {
                if let Some(p) = self.atlas.get_mut(sub.pi) {
                    p.live = p.live.saturating_sub(1);
                    p.retired |= p.live == 0;
                }
                return;
            }
        }
        if let Some(Some(mut t)) = self.textures.get_mut(id.index()).map(|s| s.take()) {
            unsafe {
                // never leave the context pointing at freed memory
                c3d::C3D_TexBind(0, std::ptr::null_mut());
                c3d::C3D_TexDelete(&mut t.raw);
            }
        }
    }

    // -- meshes and batches ---------------------------------------------------------------------------------------------------

    pub fn mesh(&mut self, verts: &[Vert]) -> Mesh {
        let mut v = Vec::with_capacity_in(verts.len(), LinearAllocator);
        v.extend_from_slice(verts);
        flush_cache(&v);
        Mesh { verts: v }
    }
    pub fn delete_mesh(&mut self, m: Mesh) {
        // its linear memory is freed at the next frame's start (`begin_frame`), when the GPU is done with the last frame
        self.retired.push(m);
    }

    pub fn terrain_mesh(&mut self, verts: &[TerrainVert]) -> TerrainMesh {
        let mut v = Vec::with_capacity_in(verts.len(), LinearAllocator);
        v.extend_from_slice(verts);
        flush_cache(&v);
        TerrainMesh { verts: v }
    }
    pub fn delete_terrain_mesh(&mut self, m: TerrainMesh) {
        self.retired_terrain.push(m);
    }

    pub fn draw_mesh(&mut self, mode: Mode, mesh: &Mesh, tex: TextureId, u: &Uniforms) {
        if self.sub_of(tex).is_some() {
            // a picture on an atlas page: its UVs moved onto the page, through the batch
            self.tris(mode, Some(tex), u, &mesh.verts);
            self.flush();
            return;
        }
        self.flush();
        if mesh.verts.is_empty() {
            return;
        }
        self.apply(mode, tex, u);
        self.draw_linear(&mesh.verts, false); // flushed once when the mesh was made
    }

    /// Vertices `first..first + count` of a course mesh, through the terrain shader (depth tested and written, opaque).
    /// A chunk keeps all its textures' triangles in one mesh and draws each texture as a range of it, so the vertex buffer
    /// is set up once per chunk rather than once per texture (that set-up is most of a draw's GPU commands).
    pub fn draw_terrain(&mut self, mesh: &TerrainMesh, first: usize, count: usize, tex: TextureId, u: &TerrainUniforms) {
        self.flush();
        if count == 0 || first + count > mesh.verts.len() {
            return;
        }
        self.use_program(Bound::Terrain);
        self.apply_mode(Mode::Solid, false, -1.0);
        let uv_scale = self.bind_texture(tex);
        let mvp = self.eye_mvp(&Mat4(u.mvp));
        let key = (mvp, u.ka, u.kd, u.ks);
        if self.sent_terrain != Some(key) {
            unsafe {
                let vs = ctru_sys::GPU_VERTEX_SHADER;
                c3d::C3D_FVUnifMtx4x4(vs, self.terrain_slots.mvp, &to_c3d(&mvp));
                let v = |id: i32, a: [f32; 3], w: f32| c3d::C3D_FVUnifSet(vs, id, a[0], a[1], a[2], w);
                v(self.terrain_slots.ka, u.ka, 1.0);
                v(self.terrain_slots.kd, u.kd, 0.0);
                v(self.terrain_slots.ks, u.ks, 0.0);
            }
            self.sent_terrain = Some(key);
        }
        if self.sent_uvs != Some(uv_scale) {
            unsafe { c3d::C3D_FVUnifSet(ctru_sys::GPU_VERTEX_SHADER, self.terrain_slots.uvs, uv_scale[0], uv_scale[1], 0.0, 0.0) };
            self.sent_uvs = Some(uv_scale);
        }
        let base = mesh.verts.as_ptr() as *const u8;
        if self.buf_bound != base {
            if !self.set_buffer(base, std::mem::size_of::<TerrainVert>(), 2, 0x10) {
                return;
            }
        }
        unsafe { c3d::C3D_DrawArrays(ctru_sys::GPU_TRIANGLES, first as i32, count as i32) };
        self.draws += 1;
    }

    /// Queues triangles (three vertices each). Consecutive calls with the same texture, mode and uniforms are drawn together.
    pub fn tris(&mut self, mode: Mode, tex: Option<TextureId>, u: &Uniforms, verts: &[Vert]) {
        let tex = tex.unwrap_or(self.white);
        if let Some(sub) = self.sub_of(tex) {
            self.batch_to(mode, sub.page, u);
            let [x0, y0, x1, y1] = sub.rect;
            self.batch.extend(verts.iter().map(|v| {
                let mut v = *v;
                v.uv = [x0 + v.uv[0].clamp(0.0, 1.0) * (x1 - x0), y0 + v.uv[1].clamp(0.0, 1.0) * (y1 - y0)];
                v
            }));
            return;
        }
        self.batch_to(mode, tex, u);
        self.batch.extend_from_slice(verts);
    }

    fn sub_of(&self, tex: TextureId) -> Option<Sub> {
        self.subs.get(tex.index()).copied().flatten()
    }

    /// Starts a new batch unless the open one has this texture, mode and uniforms.
    fn batch_to(&mut self, mode: Mode, tex: TextureId, u: &Uniforms) {
        let same = self.batch_tex == Some(tex)
            && self.batch_mode == mode
            && self.batch_uniforms.map(|b| b.mvp == u.mvp && b.lit == u.lit && b.alpha_ref == u.alpha_ref).unwrap_or(false);
        if !same {
            self.flush();
            self.batch_tex = Some(tex);
            self.batch_mode = mode;
            self.batch_uniforms = Some(*u);
        }
    }

    /// A quad given as four corners in order (two triangles).
    pub fn quad(&mut self, mode: Mode, tex: Option<TextureId>, u: &Uniforms, q: [Vert; 4]) {
        self.tris(mode, tex, u, &[q[0], q[1], q[2], q[0], q[2], q[3]]);
    }

    /// The game's Enhanced graphics draws (fx.rs, a GLSL pipeline): nothing on the 3DS, where they stay off.
    pub fn draw_mesh_with(&mut self, _pipeline: &miniquad::Pipeline, _mesh: &Mesh, _images: &[TextureId], _u: miniquad::UniformsSource) {}

    pub fn flush(&mut self) {
        if self.batch.is_empty() {
            return;
        }
        let (Some(tex), Some(u)) = (self.batch_tex, self.batch_uniforms) else {
            self.batch.clear();
            return;
        };
        let f = self.stream_frame;
        if self.stream[f].len() + self.batch.len() > STREAM_CAP {
            // more than a frame's budget: the rest of this frame is dropped rather than overwriting what the GPU reads
            self.batch.clear();
            return;
        }
        let start = self.stream[f].len();
        self.stream[f].extend_from_slice(&self.batch);
        self.batch.clear();
        let mode = self.batch_mode;
        self.apply(mode, tex, &u);
        let s = self.stream[f][start..].as_ptr();
        let n = self.stream[f].len() - start;
        let verts = unsafe { std::slice::from_raw_parts(s, n) };
        self.draw_linear(verts, true);
    }

    /// Draws `Vert` triangles from linear memory with the main program.
    fn draw_linear(&mut self, verts: &[Vert], flush: bool) {
        if flush {
            flush_cache(verts);
        }
        self.draw_buffer(verts.as_ptr() as *const u8, std::mem::size_of::<Vert>(), 4, 0x3210, verts.len());
    }

    /// Points the GPU at a vertex buffer in linear memory: `attrs` attributes with input permutation `perm`.
    fn set_buffer(&mut self, ptr: *const u8, stride: usize, attrs: i32, perm: u64) -> bool {
        unsafe {
            let mut buf: c3d::C3D_BufInfo = std::mem::zeroed();
            c3d::BufInfo_Init(&mut buf);
            // The PICA addresses a vertex buffer as base + 28-bit offset. citro3d's default base only reaches the first
            // 128 MB of RAM, and the heap can sit above that: basing the buffer at itself makes the offset zero.
            let pa = ctru_sys::osConvertVirtToPhys(ptr as *const _);
            buf.base_paddr = pa & !0xF;
            if c3d::BufInfo_Add(&mut buf, ptr as *const _, stride as isize, attrs, perm) < 0 {
                self.buf_bound = std::ptr::null();
                return false;
            }
            c3d::C3D_SetBufInfo(&mut buf);
        }
        self.buf_bound = ptr;
        true
    }

    /// One draw call over a whole vertex buffer.
    fn draw_buffer(&mut self, ptr: *const u8, stride: usize, attrs: i32, perm: u64, count: usize) {
        if count == 0 {
            return; // a zero-vertex draw is undefined on the hardware
        }
        if !self.set_buffer(ptr, stride, attrs, perm) {
            return;
        }
        unsafe { c3d::C3D_DrawArrays(ctru_sys::GPU_TRIANGLES, 0, count as i32) };
        self.draws += 1;
    }

    /// Binds a shader program and its vertex layout, if not already bound. The uniform caches are per program.
    fn use_program(&mut self, p: Bound) {
        if self.bound == p {
            return;
        }
        unsafe {
            let prog = match p {
                Bound::Terrain => &mut self.terrain,
                _ => &mut self.main,
            };
            c3d::C3D_BindProgram(&mut *prog.prog);
            c3d::C3D_SetAttrInfo(&mut prog.attrs);
        }
        self.bound = p;
        self.buf_bound = std::ptr::null();
        self.sent = None;
        self.sent_terrain = None;
        self.sent_uvs = None;
    }

    /// The depth, blend and alpha-test state of a mode, when it changes.
    fn apply_mode(&mut self, mode: Mode, alpha_test: bool, alpha_ref: f32) {
        if self.mode_applied == Some((mode, alpha_test)) {
            return;
        }
        unsafe {
            use ctru_sys::*;
            let (ce, ae) = (GPU_BLEND_ADD, GPU_BLEND_ADD);
            // the PICA's depth runs the other way from OpenGL's (citro3d's own default is GREATER against a buffer cleared
            // to 0), so OpenGL's LESS is GREATER here
            match mode {
                Mode::Solid => {
                    c3d::C3D_DepthTest(true, GPU_GREATER, GPU_WRITE_ALL);
                    c3d::C3D_AlphaBlend(ce, ae, GPU_ONE, GPU_ZERO, GPU_ONE, GPU_ZERO);
                }
                Mode::Overlay => {
                    c3d::C3D_DepthTest(true, GPU_GREATER, GPU_WRITE_COLOR);
                    c3d::C3D_AlphaBlend(ce, ae, GPU_SRC_ALPHA, GPU_ONE_MINUS_SRC_ALPHA, GPU_ONE, GPU_ONE_MINUS_SRC_ALPHA);
                }
                Mode::Flat => {
                    c3d::C3D_DepthTest(false, GPU_ALWAYS, GPU_WRITE_COLOR);
                    c3d::C3D_AlphaBlend(ce, ae, GPU_SRC_ALPHA, GPU_ONE_MINUS_SRC_ALPHA, GPU_ONE, GPU_ONE_MINUS_SRC_ALPHA);
                }
            }
            // the fragment shader's `if (c.a <= alpha_ref) discard`
            if alpha_test {
                c3d::C3D_AlphaTest(true, GPU_GREATER, (alpha_ref * 255.0) as i32);
            } else {
                c3d::C3D_AlphaTest(false, GPU_ALWAYS, 0);
            }
        }
        self.mode_applied = Some((mode, alpha_test));
    }

    /// Binds a texture to unit 0; its UV scale.
    fn bind_texture(&mut self, tex: TextureId) -> [f32; 2] {
        match self.textures.get_mut(tex.index()) {
            Some(Some(t)) => {
                unsafe { c3d::C3D_TexBind(0, &mut t.raw) };
                t.uv_scale
            }
            _ => [1.0, 1.0],
        }
    }

    /// A world or interface matrix as the GPU takes it: the eye's stereo shear on world draws (never on the 2D interface),
    /// then the PICA fix-up.
    fn eye_mvp(&self, mvp: &Mat4) -> Mat4 {
        let m = if self.screen == Screen::Top && self.slider > 0.0 && self.world { self.shear_clip(mvp) } else { *mvp };
        if self.swapped {
            // the top picture (400 x 240) on the 320-wide touchscreen: full width, 0.8 of the height; the touchscreen picture
            // (320 x 240) centred on the 400-wide top screen: 0.8 of the width. Bars take the clear colour.
            let (sx, sy) = if self.screen == Screen::Top { (1.0, 0.8) } else { (0.8, 1.0) };
            let mut sc = Mat4::identity().0;
            sc[0] = sx;
            sc[5] = sy;
            return Mat4::pica_fixup().mul(&Mat4(sc).mul(&m));
        }
        Mat4::pica_fixup().mul(&m)
    }

    /// Fixed state and uniforms for a main-program draw: the mode's depth and blend, the alpha test, the texture, the
    /// matrices and lights.
    fn apply(&mut self, mode: Mode, tex: TextureId, u: &Uniforms) {
        self.use_program(Bound::Main);
        self.apply_mode(mode, u.alpha_ref >= 0.0, u.alpha_ref);
        let uv_scale = self.bind_texture(tex);
        let final_mvp = self.eye_mvp(&Mat4(u.mvp));
        let key = (final_mvp, u.mv, u.lit, u.light_amb, u.light_dif, u.light_spec, u.light_dir);
        if self.sent.as_ref() != Some(&key) {
            unsafe {
                let vs = ctru_sys::GPU_VERTEX_SHADER;
                c3d::C3D_FVUnifMtx4x4(vs, self.main_slots.mvp, &to_c3d(&final_mvp));
                c3d::C3D_FVUnifMtx4x4(vs, self.main_slots.mv, &to_c3d(&Mat4(u.mv)));
                let v = |id: i32, a: [f32; 3], w: f32| c3d::C3D_FVUnifSet(vs, id, a[0], a[1], a[2], w);
                v(self.main_slots.lamb, u.light_amb, 0.0);
                v(self.main_slots.ldif, u.light_dif, 0.0);
                v(self.main_slots.lspec, u.light_spec, 0.0);
                v(self.main_slots.ldir, u.light_dir, u.lit);
            }
            self.sent = Some(key);
        }
        if self.sent_uvs != Some(uv_scale) {
            unsafe { c3d::C3D_FVUnifSet(ctru_sys::GPU_VERTEX_SHADER, self.main_slots.uvs, uv_scale[0], uv_scale[1], 0.0, 0.0) };
            self.sent_uvs = Some(uv_scale);
        }
    }

    /// Whether the draws that follow are the world (course, sprites, ball), which get the stereo shear, or the 2D interface,
    /// which sits at the screen's depth.
    pub fn stereo_world(&mut self, on: bool) {
        self.flush();
        self.world = on;
    }

    /// Sets the stereo scale: `h_full` world units of height is the height the depth budget below is measured against.
    pub fn set_stereo_span(&mut self, h_full: f32, _pitch_deg: f32, _depth_range: f32) {
        self.stereo_span = h_full.max(1.0);
    }

    /// The largest horizontal stereo offset a world point gets (clip units): the margin the course's off-screen test leaves
    /// so nothing one eye still sees is dropped.
    pub fn stereo_margin(&self) -> f32 {
        (STEREO_BASE + STEREO_POP * 2.0) * self.slider
    }

    /// -1 for the left eye, +1 for the right, 0 when there is no stereo (bottom screen, slider down).
    fn eye_sign(&self) -> f32 {
        if self.screen != Screen::Top || self.slider <= 0.0 {
            return 0.0;
        }
        match self.eye {
            Eye::Left => -1.0,
            Eye::Right => 1.0,
        }
    }

    /// The stereo depth of a world draw, done in clip space: each eye's picture moves sideways by
    /// `eye * slider * (STEREO_BASE - STEREO_POP * height / h_full)`.
    /// The ground plane (height 0) sits a little behind the screen (the base offset); height brings things forward, so
    /// hills and buildings stand up off the course toward the glass, and only what is high above it (a ball in flight)
    /// comes out in front. (Tested on a New 3DS XL: the course *at* the glass with hills in front strained; the whole scene
    /// receding made buildings go backwards while covering the land behind them, which strains too.)
    ///
    /// The height term comes from the matrix itself: for ground points clip z is an affine function of clip y, so
    /// `z - a*y` is zero on the ground and grows with height; dividing by its value for one unit of world height gives the
    /// height of any vertex.
    fn shear_clip(&self, mvp: &Mat4) -> Mat4 {
        let e = self.eye_sign();
        if e == 0.0 {
            return *mvp;
        }
        let a = ground_ratio(mvp);
        let mut m = mvp.0;
        let per_height = m[6] - a * m[5]; // `z - a*y` for one unit of world height (column 1 is world y)
        if per_height.abs() < 1e-9 {
            return *mvp;
        }
        let pop = -e * STEREO_POP * self.slider / self.stereo_span;
        for c in 0..3 {
            // skipping column 3 (translation) keeps the ground's depth the same wherever the camera is
            let height = (m[c * 4 + 2] - a * m[c * 4 + 1]) / per_height;
            m[c * 4] += pop * height;
        }
        m[12] += e * STEREO_BASE * self.slider;
        Mat4(m)
    }

    /// The horizontal stereo offset (clip units, the eye being drawn) a point at world height `y` gets: for things drawn in
    /// screen space but meant to sit at a height (the ball, whose height is capped by the caller).
    pub fn parallax_clip(&self, _mvp: &Mat4, y: f32) -> f32 {
        let e = self.eye_sign();
        e * self.slider * (STEREO_BASE - STEREO_POP * y / self.stereo_span)
    }

    // -- frames and screens -----------------------------------------------------------------------------------------------------

    /// ZL: swaps the two screens' pictures (and turns 3D off while they are swapped).
    pub fn set_swapped(&mut self, on: bool) {
        if on != self.swapped {
            self.swapped = on;
            unsafe { ctru_sys::gfxSet3D(!on) };
        }
    }

    /// The 3D slider (0..1) as the game should read it; 0 means draw the top screen once.
    pub fn slider(&self) -> f32 {
        self.slider
    }

    /// Starts a frame: reads the slider, flips the stream buffer, waits for the previous frame's GPU work.
    pub fn begin_frame(&mut self) {
        self.frame_no += 1;
        let tr = self.frame_no <= FRAME_TRACE;
        if tr {
            trace(&format!("frame {}: begin", self.frame_no));
        }
        self.slider = if self.swapped { 0.0 } else { unsafe { ctru_sys::osGet3DSliderState() } };
        self.stream_frame ^= 1;
        self.stream[self.stream_frame].clear();
        self.mode_applied = None;
        self.bound = Bound::None;
        self.buf_bound = std::ptr::null();
        self.draws = 0;
        self.in_frame = true;
        unsafe {
            // SYNCDRAW: waits for the previous frame's GPU work, after which last frame's meshes can go
            c3d::C3D_FrameBegin(c3d::C3D_FRAME_SYNCDRAW as u8);
            self.retired.clear();
            self.retired_terrain.clear();
            // atlas pages whose pictures were all deleted: cleared (the GPU is done with them) and filled again
            for p in self.atlas.iter_mut().filter(|p| p.retired) {
                if let Some(Some(t)) = self.textures.get_mut(p.tex.index()) {
                    let data = t.raw.__bindgen_anon_1.data as *mut u8;
                    std::ptr::write_bytes(data, 0, (PAGE * PAGE * p.fmt.bytes()) as usize);
                    c3d::C3D_TexFlush(&mut t.raw);
                }
                (p.x, p.y, p.shelf_h, p.retired) = (0, 0, 0, false);
            }
        }
        // textures the game deleted through its GPU handle since the last frame (the GPU is done with them now)
        for t in miniquad::take_deleted() {
            self.delete_texture(t);
        }
        unsafe {
            // texture * vertex colour, both rgb and alpha (the fragment half of the GLSL), for every draw
            let env = c3d::C3D_GetTexEnv(0);
            c3d::C3D_TexEnvInit(env);
            c3d::C3D_TexEnvSrc(env, c3d::C3D_Both, ctru_sys::GPU_TEXTURE0, ctru_sys::GPU_PRIMARY_COLOR, 0);
            c3d::C3D_TexEnvFunc(env, c3d::C3D_Both, ctru_sys::GPU_MODULATE);
        }
        self.use_program(Bound::Main);
        if tr {
            trace(&format!("frame {}: started", self.frame_no));
        }
    }

    /// Directs the following draws at a screen (and eye), clearing it to the colour `(r, g, b)`.
    pub fn begin_screen(&mut self, screen: Screen, eye: Eye, rgb: [u8; 3]) {
        // the clear colour as the console and Azahar both show it: 0xAARRGGBB
        let rgba = 0xff00_0000 | (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32;
        self.flush();
        self.screen = screen;
        self.eye = eye;
        let target = match (screen, eye, self.swapped) {
            (Screen::Top, Eye::Left, false) => &mut self.top_left,
            (Screen::Top, Eye::Right, false) => &mut self.top_right,
            (Screen::Bottom, _, false) => &mut self.bottom,
            (Screen::Top, _, true) => &mut self.bottom,
            (Screen::Bottom, _, true) => &mut self.top_left,
        };
        target.clear(ClearFlags::ALL, rgba, 0);
        unsafe { c3d::C3D_FrameDrawOn(target.as_raw()) };
        self.mode_applied = None;
        // a new target: re-send the matrix (the eye's shear differs)
        self.sent = None;
        self.sent_terrain = None;
    }

    pub fn end_frame(&mut self) {
        self.flush();
        unsafe { c3d::C3D_FrameEnd(0) };
        self.in_frame = false;
        if self.frame_no <= FRAME_TRACE {
            trace(&format!("frame {}: ended ({} draws)", self.frame_no, self.draws));
        }
    }

    /// Width and height of the screen draws currently land on.
    pub fn screen_size(&self) -> (f32, f32) {
        match self.screen {
            Screen::Top => (TOP_W as f32, TOP_H as f32),
            Screen::Bottom => (BOTTOM_W as f32, BOTTOM_H as f32),
        }
    }
}

/// For ground points clip z is an affine function of clip y: the ratio, from the matrix's world x and z columns.
fn ground_ratio(mvp: &Mat4) -> f32 {
    let m = &mvp.0;
    let (y0, z0, y2, z2) = (m[1], m[2], m[9], m[10]);
    if y0.abs() > y2.abs() {
        z0 / y0
    } else if y2.abs() > 1e-6 {
        z2 / y2
    } else {
        0.0
    }
}

/// The stereo depth budget, in clip units per eye at full slider (the top screen is 2.0 clip units = 400 pixels wide):
/// the ground's distance behind the screen, and how far `h_full` of height brings a point forward. Together: flat ground
/// 1.2 px per eye behind the glass, a `h_full` hill 0.8 px in front of it. Tuned by eye on a New 3DS XL.
const STEREO_BASE: f32 = 0.006;
const STEREO_POP: f32 = 0.01;

impl Drop for Gfx {
    fn drop(&mut self) {
        unsafe {
            for p in [&mut self.main, &mut self.terrain] {
                ctru_sys::shaderProgramFree(&mut *p.prog);
                ctru_sys::DVLB_Free(p.dvlb);
            }
        }
    }
}

/// The CPU wrote these vertices through its data cache; the GPU reads RAM (a no-op in an emulator, needed on the console).
fn flush_cache<T>(v: &[T]) {
    unsafe {
        let _ = ctru_sys::GSPGPU_FlushDataCache(v.as_ptr() as *const _, std::mem::size_of_val(v) as u32);
    }
}

/// Our column-major Mat4 as citro3d's C3D_Mtx: four rows, each an FVec stored w, z, y, x in memory.
fn to_c3d(m: &Mat4) -> c3d::C3D_Mtx {
    let a = &m.0;
    let mut out: c3d::C3D_Mtx = unsafe { std::mem::zeroed() };
    // C3D_Mtx is a union { C3D_FVec r[4]; float m[16] } with m laid out row by row as [w, z, y, x]
    let f: &mut [f32; 16] = unsafe { &mut out.m };
    for r in 0..4 {
        let (x, y, z, w) = (a[r], a[4 + r], a[8 + r], a[12 + r]);
        f[r * 4] = w;
        f[r * 4 + 1] = z;
        f[r * 4 + 2] = y;
        f[r * 4 + 3] = x;
    }
    out
}


/// How many screen pixels a texel of `tex` covers in a batch (the game's gfx.rs, for its smooth interface edges, which are
/// off on the 3DS): None on the 3DS, where textures are not measured.
pub fn batch_scale(_ctx: &dyn miniquad::RenderingBackend, _batch: &[Vert], _tex: TextureId, _u: &Uniforms) -> Option<f32> {
    None
}
