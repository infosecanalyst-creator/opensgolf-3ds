//! Photo mode (Enhanced graphics, docs/ENHANCED.md "Photo mode"): the port's own, not in the original. The interface goes
//! away and the course is shown alone, by default paused so the golfers hold their swing; a free camera (drag or the arrows
//! to move, the wheel, a pinch or z / x to zoom well past the original's three levels, PageUp / Home to turn), a focus blur
//! round the point clicked, a few looks (colour grades) and a vignette, all drawn through the effects' post-processing
//! (fx.rs). A photo is drawn offscreen at the window's size, twice it or 3840 x 2160 and saved as a PNG: on the desktop in
//! the save folder's `Photos`, in the browser as a download, on Android in the app's external files folder's `Photos`.
//!
//! Nothing here runs in Classic: photo mode opens only with Enhanced graphics and its effects on (and the Display page's
//! *Photo mode* tick), and every hook elsewhere asks whether `App::photo` is open. Leaving puts the camera, the pause and
//! the followed golfer back exactly as they were. The simulation is not touched: it is paused or runs as usual.
use crate::app::{now, App, Screen, ZOOM_UNIT};
use crate::fx::FxGpu;
use crate::gfx::Gfx;
use crate::ui::{rgba, Screen as Ui};
use crate::Stage;
use miniquad::*;
use sg_core::lang::{tr, trf};
use std::cell::Cell;
use std::path::PathBuf;

/// The zoom photo mode allows: from half the original's farthest level to four times its closest.
pub const ZOOM_RANGE: (f32, f32) = (ZOOM_UNIT * 0.5, ZOOM_UNIT * 16.0);

/// The looks (colour grades), by text key; L steps through them.
pub const LOOKS: [&str; 5] =
    ["STR_PHOTO_LOOK_NATURAL", "STR_PHOTO_LOOK_WARM", "STR_PHOTO_LOOK_VIVID", "STR_PHOTO_LOOK_BW", "STR_PHOTO_LOOK_VINTAGE"];
/// The photo's sizes, by text key: the window's, twice the window's, 3840 x 2160 (4K UHD).
pub const SIZES: [&str; 3] = ["STR_PHOTO_SIZE_WINDOW", "STR_PHOTO_SIZE_2X", "STR_PHOTO_SIZE_4K"];
/// The focus blur's strongest step.
pub const MAX_BLUR: u8 = 5;
/// Seconds the hint and the buttons stay after photo mode opens (after any other input: `SHOW`), and their fade.
const HINT: f64 = 7.0;
const SHOW: f64 = 4.0;
const FADE: f64 = 0.8;
/// Seconds without input before the pointer is hidden.
const POINTER_IDLE: f64 = 2.5;
/// Drawable pixels a press may move and still be a click (it sets the focus) rather than a drag.
const CLICK_SLOP: f32 = 8.0;

/// What photo mode changes and gives back when it closes.
#[derive(Clone, Copy, Debug)]
struct Saved {
    cam: (f32, f32),
    zoom: f32,
    rot: f32,
    view_zoom: f32,
    rot_left: f32,
    pan_left: (f32, f32),
    pan_vel: (f32, f32),
    follow: bool,
    paused: bool,
}

/// The choices photo mode keeps from one opening to the next (for the run).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Choices {
    look: usize,
    blur: u8,
    vignette: bool,
    size: usize,
}

thread_local! {
    static CHOICES: Cell<Choices> = const { Cell::new(Choices { look: 0, blur: 0, vignette: true, size: 0 }) };
    /// This frame's post-processing for photo mode (None when it is closed); read by fx.rs.
    static POST: Cell<Option<PhotoPost>> = const { Cell::new(None) };
    /// The composite's pipeline, made the first time it draws.
    static PIPE: Cell<Option<Pipeline>> = const { Cell::new(None) };
}

/// Photo mode's state while it is open (`App::photo`).
pub struct Photo {
    saved: Saved,
    pub look: usize,
    /// The focus blur: 0 off, 1 to MAX_BLUR.
    pub blur: u8,
    pub vignette: bool,
    pub size: usize,
    /// The point in focus, as a fraction of the window (x right, y down).
    pub focus: (f32, f32),
    /// The hint and buttons show until then (they fade out over FADE seconds).
    shown_until: f64,
    /// H: the hint and buttons kept hidden.
    hidden: bool,
    /// A photo is being drawn offscreen: no hint or buttons in it.
    capturing: bool,
    /// The photo to take after this frame (its size).
    pub pending_shot: Option<usize>,
    /// What the last photo did (shown above the buttons) and until when.
    message: Option<(String, f64)>,
    /// When the last photo was taken (the screen flashes).
    flash: f64,
    /// Where the mouse button went down, and whether it has moved since (a drag) .
    press: Option<(f32, f32)>,
    moved: bool,
    last_input: f64,
    pointer_hidden: bool,
}

/// Photo mode's part in the effects' composite (fx.rs): the look, the focus blur and the vignette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotoPost {
    pub look: usize,
    pub blur: u8,
    /// The point in focus in the render target's coordinates (0 to 1, y up).
    pub focus: (f32, f32),
    pub vignette: bool,
    /// How much larger than the window the target is (a photo at twice its size or 4K): the blur reaches as far across
    /// the picture as on the screen.
    pub spread: f32,
}

impl PhotoPost {
    /// Blur passes over the quarter size copy: more for a stronger blur.
    pub fn blur_passes(&self) -> usize {
        1 + (self.blur as usize).div_ceil(2)
    }
}

/// This frame's photo mode post-processing, or None.
pub fn post() -> Option<PhotoPost> {
    POST.with(|p| p.get())
}

/// The theme's grade from fx.rs (contrast, vibrance, tint), the Natural look's base.
pub struct ThemeGrade {
    pub contrast: f32,
    pub vibrance: f32,
    pub tint: [f32; 3],
}

#[repr(C)]
struct CompositeUniforms {
    focus: [f32; 4],
    glow: [f32; 4],
    grade: [f32; 4],
    tint: [f32; 4],
    tone: [f32; 4],
}

/// The composite: the course, the focus blur (focus = x, y, on, the sharp radius), the bloom (glow.x), the vignette
/// (glow.y; glow.z the aspect ratio), the grade (contrast, vibrance, saturation, lifted blacks), the tint and a sepia tone.
pub(crate) const COMPOSITE_FS: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying highp vec2 uv;
uniform sampler2D scene;
uniform sampler2D blurred;
uniform sampler2D bloom;
uniform vec4 focus;
uniform vec4 glow;
uniform vec4 grade;
uniform vec4 tint;
uniform vec4 tone;
void main() {
    vec3 c = texture2D(scene, uv).rgb;
    if (focus.z > 0.0) {
        // sharp round the focus point, softening with the distance; flattened so the sharp area is a wide band, as a
        // camera tilted over a miniature sees it
        vec2 d = (uv - focus.xy) * vec2(glow.z * 0.42, 1.0);
        float k = smoothstep(focus.w, focus.w + 0.36, length(d)) * focus.z;
        c = mix(c, texture2D(blurred, uv).rgb, k);
    }
    if (glow.x > 0.0) c += texture2D(bloom, uv).rgb * glow.x;
    c = (c - 0.5) * grade.x + 0.5;
    float l = dot(c, vec3(0.299, 0.587, 0.114));
    float sat = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
    c = mix(vec3(l), c, (1.0 + grade.y * (1.0 - sat)) * grade.z);
    c *= tint.rgb;
    if (tone.x > 0.0) c = mix(c, vec3(l * 1.08, l * 0.94, l * 0.74), tone.x);
    c = grade.w + c * (1.0 - grade.w * 1.4);
    if (glow.y > 0.0) {
        vec2 q = (uv - 0.5) * vec2(glow.z, 1.0);
        c *= 1.0 - glow.y * smoothstep(0.3, 1.05, length(q));
    }
    gl_FragColor = vec4(clamp(c, 0.0, 1.0), 1.0);
}
"#;

/// A look's grade: contrast, vibrance, saturation, lifted blacks, tint, sepia and the vignette's strength.
fn look_grade(look: usize, t: &ThemeGrade) -> ([f32; 4], [f32; 3], f32, f32) {
    match look {
        // Warm: a golden afternoon
        1 => ([t.contrast * 1.03, t.vibrance + 0.08, 1.04, 0.0], [1.08, 1.0, 0.86], 0.0, 0.34),
        // Vivid: punchy colour
        2 => ([1.13, 0.5, 1.14, 0.0], [1.0, 1.0, 1.0], 0.0, 0.3),
        // Black and white
        3 => ([1.2, 0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 0.0, 0.42),
        // Vintage: faded, warm, a little sepia, darker corners
        4 => ([0.92, 0.0, 0.66, 0.06], [1.06, 1.0, 0.86], 0.32, 0.5),
        // Natural: the theme's own grade (fx.rs)
        _ => ([t.contrast, t.vibrance, 1.0, 0.0], t.tint, 0.0, 0.32),
    }
}

/// Draws the composite into the frame's pass (already begun; fx.rs `fx_world_end`): `images` are the course, its blurred
/// copy and its bloom.
pub(crate) fn composite(g: &mut Gfx, f: &FxGpu, images: [TextureId; 3], p: PhotoPost, theme: ThemeGrade, bloom: f32, aspect: f32) {
    let pipe = PIPE.with(|c| c.get()).unwrap_or_else(|| {
        // a GPU that refuses it turns the effects off (fx::shader) rather than closing the game
        let uniforms = ["focus", "glow", "grade", "tint", "tone"].map(|n| (n, UniformType::Float4));
        let sh = crate::fx::shader(&mut *g.ctx, crate::fx_glsl::POST_VS, COMPOSITE_FS, &["scene", "blurred", "bloom"], &uniforms);
        let attrs = [
            VertexAttribute::new("in_pos", VertexFormat::Float3),
            VertexAttribute::new("in_uv", VertexFormat::Float2),
            VertexAttribute::new("in_normal", VertexFormat::Float3),
            VertexAttribute::new("in_color", VertexFormat::Float4),
        ];
        let params = PipelineParams { depth_test: Comparison::Always, depth_write: false, ..Default::default() };
        let pipe = g.ctx.new_pipeline(&[BufferLayout::default()], &attrs, sh, params);
        PIPE.with(|c| c.set(Some(pipe)));
        pipe
    });
    let (grade, tint, sepia, vig) = look_grade(p.look, &theme);
    // a stronger blur: a smaller sharp area, blurring more of the way
    let band = 0.3 - 0.03 * p.blur as f32;
    let amount = 0.55 + 0.09 * p.blur as f32;
    g.ctx.apply_pipeline(&pipe);
    g.ctx.apply_bindings(&Bindings { vertex_buffers: vec![f.quad_vb], index_buffer: f.quad_ib, images: images.to_vec() });
    g.ctx.apply_uniforms(UniformsSource::table(&CompositeUniforms {
        focus: [p.focus.0, p.focus.1, if p.blur > 0 { amount } else { 0.0 }, band],
        glow: [bloom, if p.vignette { vig } else { 0.0 }, aspect, 0.0],
        grade,
        tint: [tint[0], tint[1], tint[2], 1.0],
        tone: [sepia, 0.0, 0.0, 0.0],
    }));
    g.ctx.draw(0, 6, 1);
}

/// Where photos are saved: `Photos` in the save folder (on Android in the app's external files folder, which a computer
/// sees over USB; in the browser the file is downloaded).
pub fn photos_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    if let Some(p) = crate::android::game_dir().parent() {
        return p.join("Photos");
    }
    crate::app::save_dir().join("Photos")
}

/// The date and time (UTC) of a Unix time, as year, month, day, hour, minute, second.
pub(crate) fn civil(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // days to a date in the proleptic Gregorian calendar (H. Hinnant's civil_from_days)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + (m <= 2) as i64;
    (y, m, d, (rem / 3600) as u32, (rem / 60 % 60) as u32, (rem % 60) as u32)
}

/// A photo's file name: the course's name and the date and time it was taken.
fn photo_name(course: &str, secs: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    let course = course.trim();
    let name = if course.is_empty() {
        format!("OpenSGolf {y:04}-{mo:02}-{d:02} {h:02}-{mi:02}-{s:02}.png")
    } else {
        format!("OpenSGolf {course} {y:04}-{mo:02}-{d:02} {h:02}-{mi:02}-{s:02}.png")
    };
    name.chars().map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '_' } else { c }).collect()
}

/// The largest render target the graphics driver takes (each side), at most 8192.
fn max_target(g: &mut Gfx) -> u32 {
    if g.ctx.info().backend != Backend::OpenGl {
        return 8192;
    }
    let mut v: i32 = 0;
    // SAFETY: a plain query of one integer into a local
    unsafe { miniquad::gl::glGetIntegerv(0x0D33, &mut v) }; // GL_MAX_TEXTURE_SIZE
    if v <= 0 {
        4096
    } else {
        (v as u32).min(8192)
    }
}

/// Photo mode can open now: Enhanced graphics with its effects and the Photo mode tick, on the course with nothing over it.
pub fn can_open(app: &App) -> bool {
    crate::enhanced::photo_mode()
        && crate::fx::active()
        && app.photo.is_none()
        && app.screen == Screen::Play
        && app.popup.is_none()
        && app.great_shot.is_none()
        && app.simfoto.is_none()
        && app.rename.is_none()
        && app.title.save_name.is_none()
        && app.analysis.is_none()
        && app.notice.is_none()
}

/// The buttons along the bottom (for the mouse and touch): what each does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Leave,
    Pause,
    Look,
    Blur,
    Vignette,
    Size,
    Shoot,
}

const ACTIONS: [Action; 7] = [Action::Leave, Action::Pause, Action::Look, Action::Blur, Action::Vignette, Action::Size, Action::Shoot];
const BUTTON_W: f32 = 98.0;
const BUTTON_H: f32 = 30.0;
const BUTTON_GAP: f32 = 6.0;

/// The buttons' rectangles in 800 x 600 units (x, y, w, h) for a window whose extent is `ext`.
fn button_rects(ext: (f32, f32, f32, f32)) -> [(f32, f32, f32, f32); 7] {
    let n = ACTIONS.len() as f32;
    let total = n * BUTTON_W + (n - 1.0) * BUTTON_GAP;
    let x0 = 400.0 - total * 0.5;
    let y = ext.3 - BUTTON_H - 14.0;
    std::array::from_fn(|i| (x0 + i as f32 * (BUTTON_W + BUTTON_GAP), y, BUTTON_W, BUTTON_H))
}

/// The camera button over the course (Enhanced only), in the HUD's 800 x 600 layout, held to the top right corner.
const CAMERA_BUTTON: (f32, f32, f32, f32) = (764.0, 140.0, 30.0, 24.0);

impl App {
    /// Opens photo mode: the camera, the pause and the followed golfer are kept to give back; the game pauses.
    pub fn photo_open(&mut self) -> bool {
        if !can_open(self) {
            return false;
        }
        let t = now();
        let c = CHOICES.with(|c| c.get());
        let saved = Saved {
            cam: (self.cam_x, self.cam_z),
            zoom: self.zoom,
            rot: self.rot,
            view_zoom: self.enh.view_zoom,
            rot_left: self.enh.rot_left,
            pan_left: self.enh.pan_left,
            pan_vel: self.enh.pan_vel,
            follow: self.follow,
            paused: self.paused,
        };
        if self.enh.view_zoom <= 0.0 {
            self.enh.view_zoom = self.zoom;
        }
        self.follow = false;
        self.paused = true;
        self.photo = Some(Box::new(Photo {
            saved,
            look: c.look,
            blur: c.blur,
            vignette: c.vignette,
            size: c.size,
            focus: (0.5, 0.5),
            shown_until: t + HINT,
            hidden: false,
            capturing: false,
            pending_shot: None,
            message: None,
            flash: -10.0,
            press: None,
            moved: false,
            last_input: t,
            pointer_hidden: false,
        }));
        true
    }

    /// Closes photo mode and gives back the camera, the pause and the followed golfer as they were.
    pub fn photo_close(&mut self) {
        let Some(p) = self.photo.take() else { return };
        CHOICES.with(|c| c.set(Choices { look: p.look, blur: p.blur, vignette: p.vignette, size: p.size }));
        POST.with(|c| c.set(None));
        if p.pointer_hidden {
            window::show_mouse(true);
        }
        let s = p.saved;
        let quarters = ((s.rot - self.rot) / 90.0).round() as i32;
        if quarters.rem_euclid(4) != 0 {
            self.rotate_view(quarters);
        }
        self.rot = s.rot;
        (self.cam_x, self.cam_z) = s.cam;
        self.zoom = s.zoom;
        self.enh.view_zoom = s.view_zoom;
        self.enh.rot_left = s.rot_left;
        self.enh.pan_left = s.pan_left;
        self.enh.pan_vel = s.pan_vel;
        self.follow = s.follow;
        self.paused = s.paused;
    }

    /// Scripted runs: the camera was placed (--center) after the test hook opened photo mode; leaving goes back there.
    pub fn photo_rebase_camera(&mut self) {
        let cam = (self.cam_x, self.cam_z);
        if let Some(p) = self.photo.as_mut() {
            p.saved.cam = cam;
        }
    }

    /// The camera button: shown over the course with Enhanced graphics while photo mode can open.
    fn photo_button_shown(&self) -> bool {
        self.ui_ok && !self.no_hud && can_open(self)
    }

    /// Whether the pointer (800 x 600 units, in the HUD's own layout) is on the camera button.
    pub fn photo_button_hit(&self, vx: f32, vy: f32) -> bool {
        let (x, y, w, h) = CAMERA_BUTTON;
        self.photo_button_shown() && vx >= x && vx < x + w && vy >= y && vy < y + h
    }

    /// The camera button at the top right, under the rating pills: a small camera on a dark rounded plate, and its
    /// tooltip while the pointer is on it.
    pub fn draw_photo_button(&mut self, g: &mut Gfx) {
        if !self.photo_button_shown() {
            return;
        }
        let s = Ui::new(self.draw_w, self.draw_h);
        s.anchor(crate::enhanced::Anchor::TopRight);
        let (x, y, w, h) = CAMERA_BUTTON;
        let (px, py) = self.info.pointer;
        let hover = px >= x && px < x + w && py >= y && py < y + h;
        let plate = if hover { rgba(0.16, 0.3, 0.5, 0.85) } else { rgba(0.05, 0.1, 0.2, 0.66) };
        s.fill(g, x + 2.0, y, w - 4.0, h, plate);
        s.fill(g, x, y + 2.0, 2.0, h - 4.0, plate);
        s.fill(g, x + w - 2.0, y + 2.0, 2.0, h - 4.0, plate);
        s.fill(g, x + 2.0, y, w - 4.0, 1.0, rgba(0.7, 0.85, 1.0, 0.7));
        let ink = rgba(0.95, 0.97, 1.0, 1.0);
        let (cx, cy) = (x + w * 0.5, y + h * 0.5 + 1.0);
        // body and viewfinder
        s.fill(g, cx - 9.0, cy - 6.0, 18.0, 12.0, ink);
        s.fill(g, cx - 4.0, cy - 9.0, 8.0, 3.0, ink);
        s.fill(g, cx + 5.0, cy - 4.5, 2.5, 1.5, rgba(0.05, 0.1, 0.2, 1.0));
        // the lens
        disc(g, &s, cx, cy + 0.5, 4.6, rgba(0.05, 0.1, 0.2, 1.0));
        disc(g, &s, cx, cy + 0.5, 2.6, rgba(0.55, 0.75, 0.95, 1.0));
        if hover {
            let tip = tr("STR_PHOTO_BUTTON_TIP");
            let tw = crate::ui::text_width(tip, 12.0);
            s.fill(g, x - tw - 14.0, y + 3.0, tw + 10.0, 18.0, rgba(0.0, 0.0, 0.0, 0.7));
            s.text(g, x - tw - 9.0, y + 16.0, tip, 12.0, rgba(1.0, 1.0, 1.0, 1.0));
        }
        g.flush();
    }

    /// The point in focus for this frame's composite (photo mode open), or nothing.
    fn photo_post_now(&self) -> Option<PhotoPost> {
        let p = self.photo.as_ref()?;
        let spread = (self.draw_h / window::screen_size().1.max(1.0)).max(1.0);
        Some(PhotoPost { look: p.look, blur: p.blur, focus: (p.focus.0, 1.0 - p.focus.1), vignette: p.vignette, spread })
    }
}

/// A filled circle of radius `r` (800 x 600 units), in thin rows.
fn disc(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, r: f32, c: [f32; 4]) {
    let n = (r * 2.0).ceil() as i32 * 2;
    for i in 0..n {
        let y0 = -r + 2.0 * r * i as f32 / n as f32;
        let y1 = -r + 2.0 * r * (i + 1) as f32 / n as f32;
        let ym = (y0 + y1) * 0.5;
        let half = (r * r - ym * ym).max(0.0).sqrt();
        s.fill(g, cx - half, cy + y0, half * 2.0, y1 - y0, c);
    }
}

impl Stage {
    /// C or the camera button: opens photo mode.
    pub(crate) fn photo_enter(&mut self) {
        if self.app.photo_open() {
            self.dragging = false;
            self.editing = false;
            self.app.drag_state(false, now());
            println!("photo mode: open");
        }
    }

    /// Leaves photo mode (Esc, C, the Leave button, or the game needing the screen).
    pub(crate) fn photo_leave(&mut self) {
        if self.app.photo.is_some() {
            self.app.photo_close();
            self.dragging = false;
            println!("photo mode: closed");
        }
    }

    fn photo_poke(&mut self) {
        let t = now();
        if let Some(p) = self.app.photo.as_mut() {
            p.last_input = t;
            p.shown_until = p.shown_until.max(t + SHOW);
        }
    }

    /// A key in photo mode.
    pub(crate) fn photo_key(&mut self, k: KeyCode) {
        self.photo_poke();
        let shift = self.shift;
        let step = 4.0 * std::f32::consts::SQRT_2 * sg_core::terrain::TILE_SIZE;
        match k {
            KeyCode::Escape | KeyCode::C => self.photo_leave(),
            KeyCode::Enter | KeyCode::KpEnter => self.photo_action(Action::Shoot),
            KeyCode::Space | KeyCode::P => self.photo_action(Action::Pause),
            KeyCode::L if shift => {
                if let Some(p) = self.app.photo.as_mut() {
                    p.look = (p.look + LOOKS.len() - 1) % LOOKS.len();
                }
            }
            KeyCode::L => self.photo_action(Action::Look),
            KeyCode::V => self.photo_action(Action::Vignette),
            KeyCode::R => self.photo_action(Action::Size),
            KeyCode::F => {
                if let Some(p) = self.app.photo.as_mut() {
                    p.blur = if p.blur == 0 { 3 } else { 0 };
                }
            }
            KeyCode::LeftBracket | KeyCode::RightBracket => {
                if let Some(p) = self.app.photo.as_mut() {
                    p.blur = if k == KeyCode::RightBracket { (p.blur + 1).min(MAX_BLUR) } else { p.blur.saturating_sub(1) };
                }
            }
            KeyCode::H => {
                if let Some(p) = self.app.photo.as_mut() {
                    p.hidden = !p.hidden;
                }
            }
            KeyCode::Up => self.app.key_pan(0.0, step),
            KeyCode::Down => self.app.key_pan(0.0, -step),
            KeyCode::Left => self.app.key_pan(-step, 0.0),
            KeyCode::Right => self.app.key_pan(step, 0.0),
            KeyCode::Z | KeyCode::Equal | KeyCode::KpAdd => self.app.zoom_by(0.5),
            KeyCode::X | KeyCode::Minus | KeyCode::KpSubtract => self.app.zoom_by(-0.5),
            KeyCode::PageUp => self.app.rotate_view(1),
            KeyCode::Home => self.app.rotate_view(-1),
            _ => {}
        }
    }

    fn photo_action(&mut self, a: Action) {
        match a {
            Action::Leave => return self.photo_leave(),
            Action::Pause => {
                self.app.paused = !self.app.paused;
                return;
            }
            _ => {}
        }
        let Some(p) = self.app.photo.as_mut() else { return };
        match a {
            Action::Leave | Action::Pause => {}
            Action::Look => p.look = (p.look + 1) % LOOKS.len(),
            // the button steps through off and the strengths
            Action::Blur => p.blur = (p.blur + 1) % (MAX_BLUR + 1),
            Action::Vignette => p.vignette = !p.vignette,
            Action::Size => p.size = (p.size + 1) % SIZES.len(),
            Action::Shoot => p.pending_shot = Some(p.size),
        }
    }

    /// The pointer moved in photo mode: a drag moves the camera.
    pub(crate) fn photo_motion(&mut self, x: f32, y: f32, dx: f32, dy: f32) {
        self.photo_poke();
        let slop = CLICK_SLOP * self.app.dpi.max(1.0);
        let Some(p) = self.app.photo.as_mut() else { return };
        if let Some((px, py)) = p.press {
            if (x - px).hypot(y - py) > slop {
                p.moved = true;
            }
        }
        if self.dragging && p.moved {
            let k = self.app.upp;
            let sin_p = (crate::app::pitch_for(self.app.draw_w, self.app.draw_h).to_radians()).sin() as f32;
            self.app.drag_pan(-dx * k, dy * k / sin_p, now());
        }
    }

    /// The wheel in photo mode: a smooth zoom, a notch a quarter of a doubling.
    pub(crate) fn photo_wheel(&mut self, y: f32) {
        self.photo_poke();
        if y != 0.0 {
            self.app.zoom_by((y * 0.25).clamp(-0.5, 0.5));
        }
    }

    /// A mouse button (or a tap) in photo mode: a press starts a drag; a release that did not move is a click on a button
    /// or, elsewhere, puts the focus there.
    pub(crate) fn photo_button(&mut self, down: bool, button: MouseButton, x: f32, y: f32) {
        self.photo_poke();
        if button != MouseButton::Left {
            return;
        }
        let t = now();
        if down {
            if let Some(p) = self.app.photo.as_mut() {
                p.press = Some((x, y));
                p.moved = false;
            }
            self.dragging = true;
            self.app.drag_state(true, t);
            return;
        }
        if self.dragging {
            self.app.drag_state(false, t);
        }
        self.dragging = false;
        let Some(p) = self.app.photo.as_mut() else { return };
        let click = p.press.take().is_some() && !p.moved;
        if !click {
            return;
        }
        let shown = self.photo_overlay_alpha(t) > 0.05;
        let s = Ui::new(self.app.draw_w, self.app.draw_h);
        let (vx, vy) = s.view.to_virtual_raw(x, y);
        if shown {
            for (i, r) in button_rects(s.extent()).iter().enumerate() {
                if vx >= r.0 && vx < r.0 + r.2 && vy >= r.1 && vy < r.1 + r.3 {
                    self.photo_action(ACTIONS[i]);
                    return;
                }
            }
        }
        if let Some(p) = self.app.photo.as_mut() {
            p.focus = ((x / self.app.draw_w).clamp(0.0, 1.0), (y / self.app.draw_h).clamp(0.0, 1.0));
        }
    }

    /// How visible the hint and the buttons are now (0 to 1).
    fn photo_overlay_alpha(&self, t: f64) -> f32 {
        match self.app.photo.as_ref() {
            Some(p) if !p.hidden && !p.capturing => ((p.shown_until - t) / FADE).clamp(0.0, 1.0) as f32,
            _ => 0.0,
        }
    }

    /// Once a frame: photo mode closes when the game needs the screen (a report, a question, a great shot's photo); the
    /// pointer is hidden while it is still.
    pub(crate) fn photo_update(&mut self) {
        let Some(p) = self.app.photo.as_ref() else { return };
        let app = &self.app;
        if app.screen != Screen::Play || app.popup.is_some() || app.great_shot.is_some() || app.simfoto.is_some() || app.notice.is_some() {
            self.photo_leave();
            return;
        }
        let hide = now() - p.last_input > POINTER_IDLE && !self.dragging;
        if hide != p.pointer_hidden {
            window::show_mouse(!hide);
            if let Some(p) = self.app.photo.as_mut() {
                p.pointer_hidden = hide;
            }
        }
    }

    /// Draws photo mode's frame (inside `draw_frame`, its pass begun): the course alone through the effects, then the hint,
    /// the buttons, the focus mark, the last photo's message and the shutter's flash (none of them in a photo).
    pub(crate) fn draw_photo(&mut self, target: Option<RenderPass>) {
        let post = self.app.photo_post_now();
        POST.with(|c| c.set(post));
        let on = crate::fx::active() && self.app.fx_world_begin(&mut self.g, target);
        self.app.render_world(&mut self.g);
        if on {
            self.app.fx_world_end(&mut self.g, target);
        }
        let s = Ui::new(self.app.draw_w, self.app.draw_h);
        self.app.view = s.view;
        let t = now();
        let a = self.photo_overlay_alpha(t);
        let Some(p) = self.app.photo.as_ref() else { return };
        if p.capturing {
            return;
        }
        let g = &mut self.g;
        let (l, top, r, b) = s.extent();
        let white = |k: f32| rgba(1.0, 1.0, 1.0, k);
        let shade = |k: f32| rgba(0.0, 0.0, 0.0, k);
        let text = |g: &mut Gfx, x: f32, y: f32, str: &str, size: f32, k: f32| {
            s.text(g, x + 1.0, y + 1.0, str, size, shade(0.8 * k));
            s.text(g, x, y, str, size, white(k));
        };
        if a > 0.0 {
            // the hint: what the keys do, at the top
            let lines = [tr("STR_PHOTO_TITLE"), tr("STR_PHOTO_HINT_1"), tr("STR_PHOTO_HINT_2")];
            let sizes = [15.0, 12.0, 12.0];
            let wide = lines.iter().zip(sizes).map(|(t, z)| crate::ui::text_width(t, z)).fold(0.0, f32::max);
            let y0 = top + 8.0;
            s.fill(g, 400.0 - wide * 0.5 - 14.0, y0, wide + 28.0, 62.0, shade(0.5 * a));
            let mut y = y0 + 19.0;
            for (line, size) in lines.iter().zip(sizes) {
                text(g, 400.0 - (crate::ui::text_width(line, size) * 0.5).floor(), y, line, size, a);
                y += 18.0;
            }
            // the buttons
            let (mx, my) = s.view.to_virtual_raw(self.mouse.0, self.mouse.1);
            for (i, rc) in button_rects((l, top, r, b)).iter().enumerate() {
                let (x, y, w, h) = *rc;
                let hover = mx >= x && mx < x + w && my >= y && my < y + h;
                let k = if ACTIONS[i] == Action::Shoot { 0.8 } else { 0.62 };
                let bg = if hover { rgba(0.2, 0.36, 0.56, k * a) } else { rgba(0.04, 0.08, 0.12, k * a) };
                s.fill(g, x, y, w, h, bg);
                s.fill(g, x, y, w, 1.0, rgba(0.75, 0.88, 1.0, 0.7 * a));
                let label = self::label(ACTIONS[i], p, self.app.paused);
                let size = if crate::ui::text_width(&label, 12.0) > w - 6.0 { 10.0 } else { 12.0 };
                text(g, x + w * 0.5 - (crate::ui::text_width(&label, size) * 0.5).floor(), y + h * 0.5 + 4.0, &label, size, a);
            }
            // the point in focus
            if p.blur > 0 {
                let (fx, fy) = s.view.to_virtual_raw(p.focus.0 * self.app.draw_w, p.focus.1 * self.app.draw_h);
                for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    let (cx, cy) = (fx + dx * 9.0, fy + dy * 7.0);
                    s.line(g, cx, cy, cx - dx * 5.0, cy, 1.5, white(0.85 * a));
                    s.line(g, cx, cy, cx, cy - dy * 4.0, 1.5, white(0.85 * a));
                }
            }
        }
        // what the last photo did, above the buttons
        if let Some((msg, until)) = &p.message {
            let k = ((until - t) / FADE).clamp(0.0, 1.0) as f32;
            if k > 0.0 {
                let msg = fit(msg, 13.0, r - l - 40.0);
                let msg = &msg;
                let w = crate::ui::text_width(msg, 13.0);
                let y = b - BUTTON_H - 46.0;
                s.fill(g, 400.0 - w * 0.5 - 10.0, y, w + 20.0, 22.0, shade(0.6 * k));
                text(g, 400.0 - (w * 0.5).floor(), y + 16.0, msg, 13.0, k);
            }
        }
        // the shutter's flash
        let f = 1.0 - ((t - p.flash) / 0.35) as f32;
        if f > 0.0 {
            s.fill(g, l, top, r - l, b - top, white(0.75 * f.min(1.0)));
        }
        g.flush();
    }

    /// The photo's size in pixels for a choice of SIZES, within what the driver can draw.
    fn photo_size(&mut self, size: usize) -> (u32, u32) {
        let (ww, wh) = (self.app.draw_w.round().max(1.0) as u32, self.app.draw_h.round().max(1.0) as u32);
        let (w, h) = match size {
            0 => (ww, wh),
            1 => (ww * 2, wh * 2),
            _ => (3840, 2160),
        };
        let max = max_target(&mut self.g);
        if w.max(h) <= max {
            return (w, h);
        }
        let k = max as f32 / w.max(h) as f32;
        (((w as f32 * k) as u32).max(1), ((h as f32 * k) as u32).max(1))
    }

    /// Draws the course alone at w x h pixels offscreen and reads it back.
    fn photo_render(&mut self, w: u32, h: u32) -> sg_core::assets::Rgba {
        let (ow, oh) = (self.app.draw_w, self.app.draw_h);
        self.app.draw_w = w as f32;
        self.app.draw_h = h as f32;
        crate::fx::set_render_size(Some((w as f32, h as f32)));
        if let Some(p) = self.app.photo.as_mut() {
            p.capturing = true;
        }
        let params = |format| TextureParams { width: w, height: h, format, ..Default::default() };
        let color = self.g.ctx.new_render_texture(params(TextureFormat::RGBA8));
        // anti-aliased when the effects' multisampling is on, up to 4K (larger photos are sharp enough and would take a lot
        // of memory)
        let msaa = if w as u64 * h as u64 <= 3840 * 2160 { crate::hd::msaa_pass(&mut self.g, color, w, h) } else { None };
        // in the browser the page gives WebGL 2 the sized depth format miniquad's depth textures need (web/opensgolf.js)
        let depth = msaa.is_none().then(|| self.g.ctx.new_render_texture(params(TextureFormat::Depth)));
        let pass = msaa.map(|m| m.0).unwrap_or_else(|| self.g.ctx.new_render_pass(color, depth));
        // miniquad remembers which texture each unit holds and does not forget a deleted one: a texture made later under
        // the same GL name would not be bound again and samples nothing. The photo's drawing makes and deletes targets
        // (the effects' post-processing at its size), so the remembered bindings are cleared before and after it, as the
        // end of a frame clears them.
        self.g.ctx.commit_frame();
        self.draw_frame(Some(pass));
        let mut raw = vec![0u8; (w * h * 4) as usize];
        self.g.ctx.texture_read_pixels(color, &mut raw);
        self.g.ctx.delete_render_pass(pass);
        // (the pass's deletion, made by the commit below, deletes its textures)
        let _ = depth;
        self.g.ctx.commit_frame();
        self.app.draw_w = ow;
        self.app.draw_h = oh;
        crate::fx::set_render_size(None);
        if let Some(p) = self.app.photo.as_mut() {
            p.capturing = false;
        }
        let mut img = sg_core::assets::Rgba::new(w, h);
        let stride = (w * 4) as usize;
        for y in 0..h as usize {
            // GL's rows start at the bottom
            img.px[y * stride..(y + 1) * stride].copy_from_slice(&raw[(h as usize - 1 - y) * stride..(h as usize - y) * stride]);
        }
        img.px.as_chunks_mut::<4>().0.iter_mut().for_each(|p| p[3] = 255);
        img
    }

    /// Takes the photo asked for (Enter, the button): drawn offscreen at its size and saved as a PNG; the shutter sounds
    /// and the screen flashes. Called after a frame is shown.
    pub(crate) fn photo_take(&mut self) {
        let Some(size) = self.app.photo.as_mut().and_then(|p| p.pending_shot.take()) else { return };
        let (w, h) = self.photo_size(size);
        let t0 = now();
        let img = self.photo_render(w, h);
        let secs = miniquad::date::now() as i64;
        let dir = photos_dir();
        let mut path = dir.join(photo_name(&self.app.course_name, secs));
        // two photos in one second: number the next ones
        let mut n = 2;
        while cfg!(not(target_arch = "wasm32")) && path.exists() && n < 100 {
            path = dir.join(photo_name(&self.app.course_name, secs).replace(".png", &format!(" ({n}).png")));
            n += 1;
        }
        let ok = sg_core::png::write_png(&path, &img);
        let file = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        let msg = if !ok {
            tr("STR_PHOTO_FAILED").to_string()
        } else if cfg!(target_arch = "wasm32") {
            trf("STR_PHOTO_DOWNLOADED", &[("FILE", &file), ("WIDTH", &w), ("HEIGHT", &h)])
        } else {
            trf("STR_PHOTO_SAVED", &[("FILE", &path.display()), ("WIDTH", &w), ("HEIGHT", &h)])
        };
        println!(
            "photo: {} {}x{} in {:.0} ms: {}",
            if ok { "saved" } else { "could not save" },
            w,
            h,
            (now() - t0) * 1000.0,
            path.display()
        );
        self.app.ui_sound(0x95);
        let t = now();
        if let Some(p) = self.app.photo.as_mut() {
            p.flash = t;
            p.message = Some((msg, t + 5.0));
        }
    }
}

/// `msg` shortened to `width` (800 x 600 units) by leaving out the start of what follows its first ": " (a long folder).
fn fit(msg: &str, size: f32, width: f32) -> String {
    if crate::ui::text_width(msg, size) <= width {
        return msg.to_string();
    }
    let Some(i) = msg.find(": ") else { return msg.to_string() };
    let (head, tail) = (&msg[..i + 2], &msg[i + 2..]);
    let mut cut = tail.char_indices().map(|(k, _)| k).collect::<Vec<_>>().into_iter();
    loop {
        let Some(k) = cut.next() else { return msg.to_string() };
        let s = format!("{head}...{}", &tail[k..]);
        if crate::ui::text_width(&s, size) <= width {
            return s;
        }
    }
}

/// A button's text.
fn label(a: Action, p: &Photo, paused: bool) -> String {
    let on_off = |b: bool| tr(if b { "STR_PHOTO_ON" } else { "STR_PHOTO_OFF" }).to_string();
    match a {
        Action::Leave => tr("STR_PHOTO_BTN_LEAVE").to_string(),
        Action::Pause => tr(if paused { "STR_PHOTO_BTN_RESUME" } else { "STR_PHOTO_BTN_PAUSE" }).to_string(),
        Action::Look => trf("STR_PHOTO_BTN_LOOK", &[("VALUE", &tr(LOOKS[p.look % LOOKS.len()]))]),
        Action::Blur => {
            let v = if p.blur == 0 { on_off(false) } else { p.blur.to_string() };
            trf("STR_PHOTO_BTN_BLUR", &[("VALUE", &v)])
        }
        Action::Vignette => trf("STR_PHOTO_BTN_VIGNETTE", &[("VALUE", &on_off(p.vignette))]),
        Action::Size => trf("STR_PHOTO_BTN_SIZE", &[("VALUE", &tr(SIZES[p.size % SIZES.len()]))]),
        Action::Shoot => tr("STR_PHOTO_BTN_SHOOT").to_string(),
    }
}

/// Test hook `--edit "P:LOOK,BLUR,VIGNETTE,ZOOM,HINT,SHOOT"`: opens photo mode with a look (0 to 4), a focus blur (0 off,
/// 1 to 5), the vignette (0, 1), the zoom in percent of the original's closest level (0 keeps it), the hint and buttons
/// shown (1) or not (0, the default), and SHOOT 1 to 3 takes a photo at SIZES[SHOOT - 1] after the first frame.
pub fn test_hook(app: &mut App, v: &[i32]) {
    let at = |k: usize| v.get(k).copied().unwrap_or(0);
    if !app.photo_open() {
        eprintln!("photo mode: cannot open here (Enhanced graphics and its effects must be on, on the course)");
        return;
    }
    let Some(p) = app.photo.as_mut() else { return };
    p.look = (at(0).max(0) as usize).min(LOOKS.len() - 1);
    p.blur = at(1).clamp(0, MAX_BLUR as i32) as u8;
    p.vignette = v.len() < 3 || at(2) != 0;
    p.hidden = at(4) == 0;
    if (1..=3).contains(&at(5)) {
        p.pending_shot = Some(at(5) as usize - 1);
        p.size = at(5) as usize - 1;
    }
    if at(3) > 0 {
        let z = (ZOOM_UNIT * 4.0 * at(3) as f32 / 100.0).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        app.zoom = z;
        app.enh.view_zoom = z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        // 2026-10-12 17:05:33 UTC
        assert_eq!(civil(1_791_824_733), (2026, 10, 12, 17, 5, 33));
        assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0, 0));
    }

    #[test]
    fn names_are_safe() {
        let n = photo_name("A/B: \"Club\"", 0);
        assert_eq!(n, "OpenSGolf A_B_ _Club_ 1970-01-01 00-00-00.png");
        assert_eq!(photo_name("  ", 0), "OpenSGolf 1970-01-01 00-00-00.png");
    }

    #[test]
    fn buttons_fit_the_screen() {
        let r = button_rects((0.0, 0.0, 800.0, 600.0));
        assert!(r[0].0 >= 0.0 && r[6].0 + r[6].2 <= 800.0);
    }
}
