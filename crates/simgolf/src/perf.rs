//! The port's own performance log, off unless asked for: `--perf` or the environment variable OPENSGOLF_PERF=SECONDS prints
//! one line every SECONDS (default 5) of real time to the console with the frame and simulation times, the draw calls, the
//! GPU objects alive, the process's memory and the size of every collection that could grow over a long game. With it on,
//! the renderer is wrapped in a counting layer (`Counting`); off, nothing here runs and the renderer is miniquad's own.
//!
//! OPENSGOLF_PERF_SPEED=N (test runs only, with the log on) runs the game clock N times faster than the fastest speed, to
//! reach a long game quickly on a virtual display.
use miniquad::*;
use std::cell::{Cell, RefCell};
use std::time::Instant;

thread_local! {
    static ON: Cell<Option<f64>> = const { Cell::new(None) };
    static ASKED: Cell<bool> = const { Cell::new(false) };
    static C: RefCell<Counters> = RefCell::new(Counters::default());
}

#[derive(Default, Clone)]
struct Counters {
    // alive now
    textures: i64,
    texture_bytes: i64,
    buffers: i64,
    passes: i64,
    // this interval
    draws: u64,
    buffer_updates: u64,
    buffer_bytes: u64,
    stream_bytes: i64,
    upload_bytes: u64,
    textures_made: u64,
    frames: u64,
    frame_ms: f64,
    frame_max: f64,
    sim_ms: f64,
    sim_max: f64,
    ticks: u64,
    frame_start: Option<Instant>,
    frame_sim: f64,
    started: Option<Instant>,
    last: Option<Instant>,
}

/// Turns the log on (every `secs` seconds of real time).
pub fn enable(secs: f64) {
    ON.with(|o| o.set(Some(secs.max(0.5))));
}

/// Whether the log is on (OPENSGOLF_PERF or --perf).
pub fn on() -> bool {
    ON.with(|o| {
        if o.get().is_none() && !ASKED.with(|a| a.replace(true)) {
            if let Some(v) = std::env::var_os("OPENSGOLF_PERF") {
                o.set(Some(v.to_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(5.0).max(0.5)));
            }
        }
        o.get().is_some()
    })
}

/// The extra game clock factor of a test run (OPENSGOLF_PERF_SPEED; 1 normally).
pub fn speed() -> f64 {
    if !on() {
        return 1.0;
    }
    std::env::var("OPENSGOLF_PERF_SPEED").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.0).clamp(1.0, 1000.0)
}

fn with(f: impl FnOnce(&mut Counters)) {
    C.with(|c| f(&mut c.borrow_mut()));
}

/// The start of a frame (before its update).
pub fn frame_begin() {
    with(|c| c.frame_start = Some(Instant::now()));
}

/// Simulation run during the frame: its time and how many game ticks.
pub fn sim(ms: f64, ticks: u64) {
    with(|c| {
        c.frame_sim += ms;
        c.ticks += ticks;
    });
}

/// A streamed vertex upload of `bytes` (gfx.rs flush).
pub fn vb_upload(bytes: usize) {
    with(|c| c.buffer_bytes += bytes as u64);
}

/// The end of a frame (once shown, before the frame cap's wait).
pub fn frame_end() {
    with(|c| {
        let Some(t) = c.frame_start.take() else { return };
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        c.frames += 1;
        c.frame_ms += ms;
        c.frame_max = c.frame_max.max(ms);
        c.sim_ms += c.frame_sim;
        c.sim_max = c.sim_max.max(c.frame_sim);
        c.frame_sim = 0.0;
    });
}

/// The process's resident memory in MB (Linux; 0 elsewhere).
fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1).and_then(|p| p.parse::<f64>().ok()))
        .map(|pages| pages * 4096.0 / 1048576.0)
        .unwrap_or(0.0)
}

/// Prints the interval's line when it is due; `sizes` names the collections to show (called only then).
pub fn report(game_tick: u32, sizes: impl FnOnce() -> String) {
    let Some(every) = ON.with(|o| o.get()) else { return };
    let now = Instant::now();
    let line = C.with(|c| {
        let mut c = c.borrow_mut();
        let start = *c.started.get_or_insert(now);
        let last = *c.last.get_or_insert(now);
        let dt = (now - last).as_secs_f64();
        if dt < every {
            return None;
        }
        let n = c.frames.max(1) as f64;
        let line = format!(
            "perf t={:.0}s tick={game_tick} fps={:.1} frame={:.2}ms max={:.1}ms sim={:.2}ms max={:.1}ms ticks/frame={:.1} \
             draws/frame={:.0} uploads/frame={:.0} vb_upload={:.0}KB/frame stream_vb_max={:.0}KB tex_upload={:.0}KB/frame tex={} ({:.1}MB) tex_made={} bufs={} passes={} rss={:.1}MB ",
            (now - start).as_secs_f64(),
            c.frames as f64 / dt,
            c.frame_ms / n,
            c.frame_max,
            c.sim_ms / n,
            c.sim_max,
            c.ticks as f64 / n,
            c.draws as f64 / n,
            c.buffer_updates as f64 / n,
            c.buffer_bytes as f64 / n / 1024.0,
            c.stream_bytes as f64 / 1024.0,
            c.upload_bytes as f64 / n / 1024.0,
            c.textures,
            c.texture_bytes as f64 / 1048576.0,
            c.textures_made,
            c.buffers,
            c.passes,
            rss_mb(),
        );
        let keep = (c.textures, c.texture_bytes, c.buffers, c.passes, c.started, c.stream_bytes);
        *c = Counters::default();
        (c.textures, c.texture_bytes, c.buffers, c.passes, c.started, c.stream_bytes) = keep;
        c.last = Some(now);
        Some(line)
    });
    if let Some(l) = line {
        println!("{l}{}", sizes());
    }
}

fn tex_bytes(p: &TextureParams) -> i64 {
    let base = p.format.size(p.width, p.height) as i64 * p.sample_count.max(1) as i64;
    if p.allocate_mipmaps {
        base * 4 / 3
    } else {
        base
    }
}

/// miniquad's renderer with every call passed through and the GPU objects and draws counted (only with the log on).
pub struct Counting(pub Box<dyn RenderingBackend>);

impl RenderingBackend for Counting {
    fn info(&self) -> ContextInfo {
        self.0.info()
    }
    fn new_shader(&mut self, shader: ShaderSource, meta: ShaderMeta) -> Result<ShaderId, ShaderError> {
        self.0.new_shader(shader, meta)
    }
    fn new_texture(&mut self, access: TextureAccess, data: TextureSource, params: TextureParams) -> TextureId {
        with(|c| {
            c.textures += 1;
            c.textures_made += 1;
            c.texture_bytes += tex_bytes(&params);
        });
        self.0.new_texture(access, data, params)
    }
    fn texture_params(&self, texture: TextureId) -> TextureParams {
        self.0.texture_params(texture)
    }
    unsafe fn texture_raw_id(&self, texture: TextureId) -> RawId {
        self.0.texture_raw_id(texture)
    }
    fn texture_set_min_filter(&mut self, texture: TextureId, filter: FilterMode, mipmap_filter: MipmapFilterMode) {
        self.0.texture_set_min_filter(texture, filter, mipmap_filter)
    }
    fn texture_set_mag_filter(&mut self, texture: TextureId, filter: FilterMode) {
        self.0.texture_set_mag_filter(texture, filter)
    }
    fn texture_set_wrap(&mut self, texture: TextureId, wrap_x: TextureWrap, wrap_y: TextureWrap) {
        self.0.texture_set_wrap(texture, wrap_x, wrap_y)
    }
    fn texture_generate_mipmaps(&mut self, texture: TextureId) {
        self.0.texture_generate_mipmaps(texture)
    }
    fn texture_resize(&mut self, texture: TextureId, width: u32, height: u32, bytes: Option<&[u8]>) {
        let old = tex_bytes(&self.0.texture_params(texture));
        self.0.texture_resize(texture, width, height, bytes);
        let new = tex_bytes(&self.0.texture_params(texture));
        with(|c| c.texture_bytes += new - old);
    }
    fn texture_read_pixels(&mut self, texture: TextureId, bytes: &mut [u8]) {
        self.0.texture_read_pixels(texture, bytes)
    }
    fn texture_update_part(&mut self, texture: TextureId, x: i32, y: i32, w: i32, h: i32, bytes: &[u8]) {
        with(|c| c.upload_bytes += bytes.len() as u64);
        self.0.texture_update_part(texture, x, y, w, h, bytes)
    }
    fn new_render_pass_mrt(&mut self, color: &[TextureId], resolve: Option<&[TextureId]>, depth: Option<TextureId>) -> RenderPass {
        with(|c| c.passes += 1);
        self.0.new_render_pass_mrt(color, resolve, depth)
    }
    fn render_pass_color_attachments(&self, render_pass: RenderPass) -> &[TextureId] {
        self.0.render_pass_color_attachments(render_pass)
    }
    fn delete_render_pass(&mut self, render_pass: RenderPass) {
        with(|c| c.passes -= 1);
        self.0.delete_render_pass(render_pass)
    }
    fn new_pipeline(&mut self, layout: &[BufferLayout], attrs: &[VertexAttribute], shader: ShaderId, p: PipelineParams) -> Pipeline {
        self.0.new_pipeline(layout, attrs, shader, p)
    }
    fn apply_pipeline(&mut self, pipeline: &Pipeline) {
        self.0.apply_pipeline(pipeline)
    }
    fn delete_pipeline(&mut self, pipeline: Pipeline) {
        self.0.delete_pipeline(pipeline)
    }
    fn new_buffer(&mut self, type_: BufferType, usage: BufferUsage, data: BufferSource) -> BufferId {
        with(|c| c.buffers += 1);
        let id = self.0.new_buffer(type_, usage, data);
        if usage == BufferUsage::Stream {
            let bytes = self.0.buffer_size(id) as i64;
            with(|c| c.stream_bytes = c.stream_bytes.max(bytes));
        }
        id
    }
    fn buffer_update(&mut self, buffer: BufferId, data: BufferSource) {
        with(|c| c.buffer_updates += 1);
        self.0.buffer_update(buffer, data)
    }
    fn buffer_size(&mut self, buffer: BufferId) -> usize {
        self.0.buffer_size(buffer)
    }
    fn delete_buffer(&mut self, buffer: BufferId) {
        with(|c| c.buffers -= 1);
        self.0.delete_buffer(buffer)
    }
    fn delete_texture(&mut self, texture: TextureId) {
        let b = tex_bytes(&self.0.texture_params(texture));
        with(|c| {
            c.textures -= 1;
            c.texture_bytes -= b;
        });
        self.0.delete_texture(texture)
    }
    fn delete_shader(&mut self, program: ShaderId) {
        self.0.delete_shader(program)
    }
    fn apply_viewport(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.0.apply_viewport(x, y, w, h)
    }
    fn apply_scissor_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.0.apply_scissor_rect(x, y, w, h)
    }
    fn apply_bindings_from_slice(&mut self, vertex_buffers: &[BufferId], index_buffer: BufferId, textures: &[TextureId]) {
        self.0.apply_bindings_from_slice(vertex_buffers, index_buffer, textures)
    }
    fn apply_uniforms_from_bytes(&mut self, uniform_ptr: *const u8, size: usize) {
        self.0.apply_uniforms_from_bytes(uniform_ptr, size)
    }
    fn clear(&mut self, color: Option<(f32, f32, f32, f32)>, depth: Option<f32>, stencil: Option<i32>) {
        self.0.clear(color, depth, stencil)
    }
    fn begin_default_pass(&mut self, action: PassAction) {
        self.0.begin_default_pass(action)
    }
    fn begin_pass(&mut self, pass: Option<RenderPass>, action: PassAction) {
        self.0.begin_pass(pass, action)
    }
    fn end_render_pass(&mut self) {
        self.0.end_render_pass()
    }
    fn commit_frame(&mut self) {
        self.0.commit_frame()
    }
    fn draw(&self, base_element: i32, num_elements: i32, num_instances: i32) {
        with(|c| c.draws += 1);
        self.0.draw(base_element, num_elements, num_instances)
    }
}
