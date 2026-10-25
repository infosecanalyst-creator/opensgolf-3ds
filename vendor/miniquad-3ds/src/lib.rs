//! OpenSGolf on the 3DS: a stand-in for miniquad, so the game's own code compiles for the 3DS unchanged.
//!
//! The data types and the `RenderingBackend` trait are miniquad 0.4.11's own (graphics.rs, event.rs, conf.rs, the GL
//! function table in native/gl.rs, which is plain Rust: its functions are pointers filled in at run time and are never
//! filled in here). The 3DS draws through its own renderer (crates/simgolf-3ds/src/gfx.rs); the game's few direct GPU
//! calls (its desktop visual effects, switched off on the 3DS) go to `NullBackend`, which does nothing. The window
//! functions are the 3DS's answers (no mouse to show, no clipboard, no files dropped on a window).
#![allow(clippy::all, dead_code, unused)]

pub mod conf;
mod event;
pub mod graphics;
pub mod native {
    pub mod gl;
    pub(crate) mod query_stab;
}
mod default_icon;

use std::collections::HashMap;
use std::ops::{Index, IndexMut};

pub use event::*;
pub use graphics::*;
pub use native::gl;

#[derive(Clone)]
pub(crate) struct ResourceManager<T> {
    id: usize,
    resources: HashMap<usize, T>,
}

impl<T> Default for ResourceManager<T> {
    fn default() -> Self {
        Self { id: 0, resources: HashMap::new() }
    }
}

impl<T> ResourceManager<T> {
    pub fn add(&mut self, resource: T) -> usize {
        self.resources.insert(self.id, resource);
        self.id += 1;
        self.id - 1
    }
    pub fn remove(&mut self, id: usize) -> T {
        self.resources.remove(&id).unwrap()
    }
}

impl<T> Index<usize> for ResourceManager<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.resources[&index]
    }
}

impl<T> IndexMut<usize> for ResourceManager<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.resources.get_mut(&index).unwrap()
    }
}

pub mod date {
    /// Seconds since the Unix epoch, as miniquad's.
    pub fn now() -> f64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
    }
}

/// Textures the game deleted through its GPU handle (`g.ctx.delete_texture`, its usual way): the 3DS renderer frees them at
/// its next frame (`take_deleted`), as only it holds them.
static DELETED: std::sync::Mutex<Vec<TextureId>> = std::sync::Mutex::new(Vec::new());
/// The sizes of the 3DS renderer's textures (the pictures', before padding to a power of two), for the game's
/// `ctx.texture_size` questions.
static SIZES: std::sync::Mutex<Option<HashMap<usize, (u32, u32)>>> = std::sync::Mutex::new(None);

/// The 3DS renderer: a texture it made, and the picture's size.
pub fn register_texture(id: TextureId, w: u32, h: u32) {
    if let Ok(mut m) = SIZES.lock() {
        m.get_or_insert_with(HashMap::new).insert(id.index(), (w, h));
    }
}

/// The 3DS renderer: the textures the game has deleted since it last asked.
pub fn take_deleted() -> Vec<TextureId> {
    let out = DELETED.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default();
    if let Ok(mut m) = SIZES.lock() {
        if let Some(m) = m.as_mut() {
            for t in &out {
                m.remove(&t.index());
            }
        }
    }
    out
}

/// A GPU that draws nothing: the target of the game's direct GPU calls on the 3DS (desktop effects that stay off).
#[derive(Default)]
pub struct NullBackend {
    textures: HashMap<usize, TextureParams>,
    next: usize,
    passes: HashMap<usize, Vec<TextureId>>,
}

impl RenderingBackend for NullBackend {
    fn info(&self) -> ContextInfo {
        ContextInfo { backend: Backend::OpenGl, gl_version_string: String::new(), glsl_support: GlslSupport::default(), features: Features::default() }
    }
    fn new_shader(&mut self, _shader: ShaderSource, _meta: ShaderMeta) -> Result<ShaderId, ShaderError> {
        Err(ShaderError::LinkError("no shaders on the 3DS".into()))
    }
    fn new_texture(&mut self, _access: TextureAccess, _data: TextureSource, params: TextureParams) -> TextureId {
        // far above the 3DS renderer's own texture slots
        self.next += 1;
        let id = 1_000_000 + self.next;
        self.textures.insert(id, params);
        TextureId::from_index(id)
    }
    fn texture_params(&self, texture: TextureId) -> TextureParams {
        if let Some(p) = self.textures.get(&texture.index()) {
            return *p;
        }
        // one of the 3DS renderer's
        let size = SIZES.lock().ok().and_then(|m| m.as_ref().and_then(|m| m.get(&texture.index()).copied()));
        let (width, height) = size.unwrap_or((0, 0));
        TextureParams { width, height, ..Default::default() }
    }
    unsafe fn texture_raw_id(&self, texture: TextureId) -> RawId {
        RawId::OpenGl(texture.index() as u32)
    }
    fn texture_set_min_filter(&mut self, _t: TextureId, _f: FilterMode, _m: MipmapFilterMode) {}
    fn texture_set_mag_filter(&mut self, _t: TextureId, _f: FilterMode) {}
    fn texture_set_wrap(&mut self, _t: TextureId, _x: TextureWrap, _y: TextureWrap) {}
    fn texture_generate_mipmaps(&mut self, _t: TextureId) {}
    fn texture_resize(&mut self, t: TextureId, width: u32, height: u32, _bytes: Option<&[u8]>) {
        if let Some(p) = self.textures.get_mut(&t.index()) {
            p.width = width;
            p.height = height;
        }
    }
    fn texture_read_pixels(&mut self, _t: TextureId, _bytes: &mut [u8]) {}
    fn texture_update_part(&mut self, _t: TextureId, _x: i32, _y: i32, _w: i32, _h: i32, _bytes: &[u8]) {}
    fn new_render_pass_mrt(&mut self, color_img: &[TextureId], _resolve: Option<&[TextureId]>, _depth: Option<TextureId>) -> RenderPass {
        self.next += 1;
        self.passes.insert(self.next, color_img.to_vec());
        RenderPass(self.next)
    }
    fn render_pass_color_attachments(&self, render_pass: RenderPass) -> &[TextureId] {
        self.passes.get(&render_pass.0).map(|v| v.as_slice()).unwrap_or(&[])
    }
    fn delete_render_pass(&mut self, render_pass: RenderPass) {
        self.passes.remove(&render_pass.0);
    }
    fn new_pipeline(&mut self, _l: &[BufferLayout], _a: &[VertexAttribute], _s: ShaderId, _p: PipelineParams) -> Pipeline {
        Pipeline(0)
    }
    fn apply_pipeline(&mut self, _pipeline: &Pipeline) {}
    fn delete_pipeline(&mut self, _pipeline: Pipeline) {}
    fn new_buffer(&mut self, _t: BufferType, _u: BufferUsage, _d: BufferSource) -> BufferId {
        BufferId(0)
    }
    fn buffer_update(&mut self, _b: BufferId, _d: BufferSource) {}
    fn buffer_size(&mut self, _b: BufferId) -> usize {
        0
    }
    fn delete_buffer(&mut self, _b: BufferId) {}
    fn delete_texture(&mut self, t: TextureId) {
        if self.textures.remove(&t.index()).is_none() {
            // one of the 3DS renderer's: freed at its next frame
            if let Ok(mut v) = DELETED.lock() {
                v.push(t);
            }
        }
    }
    fn delete_shader(&mut self, _p: ShaderId) {}
    fn apply_viewport(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {}
    fn apply_scissor_rect(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {}
    fn apply_bindings_from_slice(&mut self, _v: &[BufferId], _i: BufferId, _t: &[TextureId]) {}
    fn apply_uniforms_from_bytes(&mut self, _p: *const u8, _s: usize) {}
    fn clear(&mut self, _c: Option<(f32, f32, f32, f32)>, _d: Option<f32>, _s: Option<i32>) {}
    fn begin_default_pass(&mut self, _a: PassAction) {}
    fn begin_pass(&mut self, _p: Option<RenderPass>, _a: PassAction) {}
    fn end_render_pass(&mut self) {}
    fn commit_frame(&mut self) {}
    fn draw(&self, _b: i32, _n: i32, _i: i32) {}
}

pub mod window {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static QUIT: AtomicBool = AtomicBool::new(false);

    /// A quit asked for by the game (its Quit menu item): the 3DS frontend reads it each frame.
    pub fn quit_requested() -> bool {
        QUIT.load(Ordering::Relaxed)
    }
    pub fn new_rendering_backend() -> Box<dyn RenderingBackend> {
        Box::new(NullBackend::default())
    }
    /// The game lays its interface out in the original's 800 x 600; the 3DS frontend maps that onto its screens.
    pub fn screen_size() -> (f32, f32) {
        (800.0, 600.0)
    }
    pub fn dpi_scale() -> f32 {
        1.0
    }
    pub fn high_dpi() -> bool {
        false
    }
    pub fn blocking_event_loop() -> bool {
        false
    }
    pub fn order_quit() {
        QUIT.store(true, Ordering::Relaxed);
    }
    pub fn quit() {
        order_quit();
    }
    pub fn request_quit() {
        order_quit();
    }
    pub fn cancel_quit() {
        QUIT.store(false, Ordering::Relaxed);
    }
    pub fn set_cursor_grab(_grab: bool) {}
    pub fn schedule_update() {}
    pub fn show_mouse(_shown: bool) {}
    pub fn set_mouse_cursor(_cursor_icon: CursorIcon) {}
    pub fn set_window_size(_w: u32, _h: u32) {}
    pub fn set_window_position(_x: u32, _y: u32) {}
    pub fn get_window_position() -> (u32, u32) {
        (0, 0)
    }
    pub fn set_fullscreen(_fullscreen: bool) {}
    pub fn clipboard_get() -> Option<String> {
        None
    }
    pub fn clipboard_set(_data: &str) {}
    pub fn dropped_file_count() -> usize {
        0
    }
    pub fn dropped_file_bytes(_index: usize) -> Option<Vec<u8>> {
        None
    }
    pub fn dropped_file_path(_index: usize) -> Option<std::path::PathBuf> {
        None
    }
    pub fn show_keyboard(_show: bool) {}
    pub fn set_ime_position(_x: i32, _y: i32) {}
    pub fn set_ime_enabled(_enabled: bool) {}
}

/// The desktop entry point; the 3DS frontend runs its own loop and never calls it.
pub fn start<F>(_conf: conf::Conf, _f: F)
where
    F: 'static + FnOnce() -> Box<dyn EventHandler>,
{
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum CursorIcon {
    Default,
    Help,
    Pointer,
    Wait,
    Crosshair,
    Text,
    Move,
    NotAllowed,
    EWResize,
    NSResize,
    NESWResize,
    NWSEResize,
}
