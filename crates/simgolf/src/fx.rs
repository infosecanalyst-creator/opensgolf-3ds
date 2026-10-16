//! Enhanced graphics effects (docs/ENHANCED.md, "Effects"): a pixel-art scaler for the sprites and interface pictures,
//! animated water, per-pixel terrain light with ambient occlusion and soft sun shadows, softened sprite shadows, a sky behind
//! the course, and post-processing of the course (tilt-shift, colour grade, bloom, vignette). None of it edits the art.
//!
//! Classic is untouched: every hook in the renderer is behind [`active`], which is false unless the player turned the effects
//! on, and with it false none of this module's code runs and the classic pixels are the same byte for byte.
//!
//! The options are an [`FxSettings`]; [`options`] lists them (key, label, choices, get, set) for a preferences screen. For now
//! they come from `--fx` / `--fx-set KEY=VALUE,...` or the environment variable `OPENSGOLF_FX` ([`configure`]).
use crate::app::App;
use crate::fx_glsl as glsl;
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use miniquad::*;
use sg_core::assets::Rgba;
use sg_core::terrain::{Terrain, HEIGHT_STEP, TILE_SIZE};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

// ---- settings -----------------------------------------------------------------------------------------------------------------

/// The pixel-art scaler's modes.
pub const SCALER_OFF: u8 = 0;
pub const SCALER_SHARP: u8 = 1;
pub const SCALER_EDGE: u8 = 2;
/// The strongest: edge-directed interpolation along the art's own edges, inside the art too (fx_smooth.rs).
pub const SCALER_SMOOTH: u8 = 3;

/// The effects' options. `enabled` is the master switch (the Enhanced graphics setting); every other option defaults to what
/// the effects use when it is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FxSettings {
    pub enabled: bool,
    /// SCALER_OFF, SCALER_SHARP (sharp bilinear), SCALER_EDGE (edge directed) or SCALER_SMOOTH (fx_smooth.rs).
    pub scaler: u8,
    pub water: bool,
    /// Per-pixel terrain light.
    pub lighting: bool,
    pub occlusion: bool,
    pub shadows: bool,
    pub soft_sprite_shadows: bool,
    pub sky: bool,
    pub tilt_shift: bool,
    pub grading: bool,
    pub bloom: bool,
    pub vignette: bool,
    /// 0 low, 1 medium, 2 high.
    pub quality: u8,
    /// Shadows cast along the sun by sprites (fx_shadow.rs).
    pub cast_shadows: bool,
    /// The day's light follows the game's clock (fx_light.rs).
    pub time_of_day: bool,
    pub clouds: bool,
    pub rain: bool,
    pub mist: bool,
    pub haze: bool,
    /// Balls in flight trail a fading streak; where they land, sand, water or grass is thrown up (fx_ball.rs).
    pub ball_trail: bool,
    pub ball_impacts: bool,
    /// Trees, buildings, golfers and the sky mirrored in the water, the sun's glitter, the shore lapping (fx_reflect.rs).
    pub reflections: bool,
    /// A wind for each day: trees, palms, bushes and flags sway, the rain and the clouds follow it (fx_nature.rs).
    pub wind: bool,
    /// Lit windows, lamps and fireflies at dusk, with the time of day (fx_night.rs).
    pub evening_lights: bool,
    /// Blossom in spring, autumn colours in September and October, frost and dew in the mornings (fx_nature.rs).
    pub seasons: bool,
    /// Wet, glossy ground and puddles during and after rain, now and then a rainbow (fx_wet.rs).
    pub after_rain: bool,
    /// Lightning in heavy rain on parkland and links courses (fx_wet.rs).
    pub lightning: bool,
    /// Tiny tufts of grass standing up from the rough, swaying in the wind (fx_life.rs).
    pub grass: bool,
    /// Contrast-adaptive sharpening of the finished picture (the composite in fx_glsl.rs).
    pub sharpen: bool,
    /// Snow cover, falling snow and icy ponds early and late in the year (fx_winter.rs).
    pub winter: bool,
    /// Spectators, a grandstand and fireworks at tournaments (fx_crowd.rs).
    pub crowds: bool,
    /// Birds overhead, butterflies over flower beds, autumn leaves falling (fx_life.rs).
    pub life: bool,
    /// How long a day of light lasts (fx_light.rs DAY_MONTHS): 0 short (one game month), 1 normal (three), 2 long (six).
    pub day_length: u8,
    // the water's own options (fx_water.rs; the shader's in fx_glsl.rs)
    /// The theme's water colour lighter in the shallows and deeper in the middle, the sky's reflection and the haze kept
    /// from paling it.
    pub water_colour: bool,
    /// Soft painterly crests of light following the wind and the weather.
    pub waves: bool,
    /// Lines of water lapping at the shore and the bank darker where it is wet.
    pub shoreline: bool,
    /// Sunlight rippling over the bottom of shallow water.
    pub caustics: bool,
    /// Rings opening on the water while it rains.
    pub rain_rings: bool,
    /// Reeds, lily pads, ducks, swans and jumping fish.
    pub water_life: bool,
    /// Ponds icing over in the late-winter cold snap (with the winter option).
    pub ice: bool,
    /// Spray and foam round the fountains and at the foot of the waterfalls.
    pub spray: bool,
}

impl Default for FxSettings {
    fn default() -> Self {
        FxSettings {
            enabled: false,
            scaler: SCALER_SMOOTH,
            water: true,
            lighting: true,
            occlusion: true,
            shadows: true,
            soft_sprite_shadows: true,
            sky: true,
            // a strong look: offered, not on by default
            tilt_shift: false,
            grading: true,
            bloom: true,
            vignette: true,
            quality: 1,
            cast_shadows: true,
            // these three change the mood the most: offered, not on by default
            time_of_day: false,
            clouds: true,
            rain: false,
            mist: false,
            haze: true,
            ball_trail: true,
            ball_impacts: true,
            reflections: true,
            wind: true,
            evening_lights: true,
            seasons: true,
            after_rain: true,
            // a strong effect: offered, not on by default
            lightning: false,
            grass: true,
            sharpen: true,
            winter: true,
            crowds: true,
            life: true,
            day_length: 1,
            water_colour: true,
            waves: true,
            shoreline: true,
            caustics: true,
            rain_rings: true,
            water_life: true,
            ice: true,
            spray: true,
        }
    }
}

thread_local! {
    /// The scaler's blend between texels, in texels (SG_FX_SOFT=sharp,edge overrides them for tuning).
    static SOFT_SHARP: Cell<f32> = Cell::new(soft_env(0, 0.0));
    static SOFT_EDGE: Cell<f32> = Cell::new(soft_env(1, 0.5));
    static SETTINGS: Cell<FxSettings> = Cell::new(FxSettings::default());
    /// Textures the scaler leaves alone (the font atlases: text is not pixel art).
    static PLAIN: RefCell<HashSet<TextureId>> = RefCell::new(HashSet::new());
}

fn soft_env(i: usize, default: f32) -> f32 {
    std::env::var("SG_FX_SOFT").ok().and_then(|v| v.split(',').nth(i).and_then(|x| x.parse().ok())).unwrap_or(default)
}

pub fn settings() -> FxSettings {
    let mut s = SETTINGS.with(|s| s.get());
    // an effects shader the GPU would not compile (`shader`): the effects stay off
    if BROKEN.with(|b| b.get()) {
        s.enabled = false;
    }
    s
}

thread_local! {
    /// Set when the GPU refused an effects shader.
    static BROKEN: Cell<bool> = const { Cell::new(false) };
}

pub fn set_settings(s: FxSettings) {
    SETTINGS.with(|c| c.set(s));
}

/// Whether the effects are on. False in Classic: then no effect code runs.
pub fn active() -> bool {
    settings().enabled
}

/// Marks a texture the scaler must not touch (text).
pub fn plain_texture(t: TextureId) {
    PLAIN.with(|p| p.borrow_mut().insert(t));
}

/// One option for a preferences screen: `label` and `choices` are text keys (lang/en.txt); `get` gives the index of the
/// current choice, `set` picks one.
pub struct FxOption {
    pub key: &'static str,
    pub label: &'static str,
    pub choices: &'static [&'static str],
    pub get: fn(&FxSettings) -> usize,
    pub set: fn(&mut FxSettings, usize),
}

const ON_OFF: &[&str] = &["STR_FX_OFF", "STR_FX_ON"];

macro_rules! flag {
    ($key:literal, $label:literal, $field:ident) => {
        FxOption { key: $key, label: $label, choices: ON_OFF, get: |s| s.$field as usize, set: |s, v| s.$field = v != 0 }
    };
}

static OPTIONS: [FxOption; 41] = [
    FxOption { key: "effects", label: "STR_FX_EFFECTS", choices: ON_OFF, get: |s| s.enabled as usize, set: |s, v| s.enabled = v != 0 },
    FxOption {
        key: "scaler",
        label: "STR_FX_SCALER",
        choices: &["STR_FX_OFF", "STR_FX_SCALER_SHARP", "STR_FX_SCALER_EDGE", "STR_FX_SCALER_SMOOTH"],
        get: |s| s.scaler.min(SCALER_SMOOTH) as usize,
        set: |s, v| s.scaler = (v as u8).min(SCALER_SMOOTH),
    },
    flag!("water", "STR_FX_WATER", water),
    flag!("lighting", "STR_FX_LIGHTING", lighting),
    flag!("occlusion", "STR_FX_OCCLUSION", occlusion),
    flag!("shadows", "STR_FX_SHADOWS", shadows),
    flag!("sprite-shadows", "STR_FX_SPRITE_SHADOWS", soft_sprite_shadows),
    flag!("sky", "STR_FX_SKY", sky),
    flag!("tilt-shift", "STR_FX_TILT_SHIFT", tilt_shift),
    flag!("grading", "STR_FX_GRADING", grading),
    flag!("bloom", "STR_FX_BLOOM", bloom),
    flag!("vignette", "STR_FX_VIGNETTE", vignette),
    FxOption {
        key: "quality",
        label: "STR_FX_QUALITY",
        choices: &["STR_FX_QUALITY_LOW", "STR_FX_QUALITY_MEDIUM", "STR_FX_QUALITY_HIGH"],
        get: |s| s.quality.min(2) as usize,
        set: |s, v| s.quality = v.min(2) as u8,
    },
    flag!("cast-shadows", "STR_FX_CAST_SHADOWS", cast_shadows),
    flag!("time-of-day", "STR_FX_TIME_OF_DAY", time_of_day),
    flag!("clouds", "STR_FX_CLOUDS", clouds),
    flag!("rain", "STR_FX_RAIN", rain),
    flag!("mist", "STR_FX_MIST", mist),
    flag!("haze", "STR_FX_HAZE", haze),
    flag!("ball-trail", "STR_FX_BALL_TRAIL", ball_trail),
    flag!("ball-impacts", "STR_FX_BALL_IMPACTS", ball_impacts),
    flag!("reflections", "STR_FX_REFLECTIONS", reflections),
    flag!("wind", "STR_FX_WIND", wind),
    flag!("evening-lights", "STR_FX_EVENING_LIGHTS", evening_lights),
    flag!("seasons", "STR_FX_SEASONS", seasons),
    flag!("after-rain", "STR_FX_AFTER_RAIN", after_rain),
    flag!("lightning", "STR_FX_LIGHTNING", lightning),
    flag!("grass", "STR_FX_GRASS", grass),
    flag!("sharpen", "STR_FX_SHARPEN", sharpen),
    flag!("winter", "STR_FX_WINTER", winter),
    flag!("crowds", "STR_FX_CROWDS", crowds),
    flag!("life", "STR_FX_LIFE", life),
    FxOption {
        key: "day-length",
        label: "STR_FX_DAY_LENGTH",
        choices: &["STR_FX_DAY_SHORT", "STR_FX_DAY_NORMAL", "STR_FX_DAY_LONG"],
        get: |s| s.day_length.min(2) as usize,
        set: |s, v| s.day_length = v.min(2) as u8,
    },
    flag!("water-colour", "STR_FX_WATER_COLOUR", water_colour),
    flag!("waves", "STR_FX_WAVES", waves),
    flag!("shoreline", "STR_FX_SHORELINE", shoreline),
    flag!("caustics", "STR_FX_CAUSTICS", caustics),
    flag!("rain-rings", "STR_FX_RAIN_RINGS", rain_rings),
    flag!("water-life", "STR_FX_WATER_LIFE", water_life),
    flag!("ice", "STR_FX_ICE", ice),
    flag!("spray", "STR_FX_SPRAY", spray),
];

/// The options in the order a preferences screen lists them; the first is the master switch.
pub fn options() -> &'static [FxOption] {
    &OPTIONS
}

/// Prints the options and their current choices (`--fx-list`).
pub fn print_options() {
    let s = settings();
    for o in options() {
        let names: Vec<String> = o.choices.iter().enumerate().map(|(i, c)| format!("{i} {}", sg_core::lang::tr(c))).collect();
        println!("{:<15} {:<40} {} [{}]", o.key, sg_core::lang::tr(o.label), (o.get)(&s), names.join(", "));
    }
}

/// Applies `key=value,...` to settings: a value is a choice's index, on/off/true/false/yes/no, or a choice's name
/// (sharp, edge, smooth, low, medium, high). A bare `on`/`1` (or `off`/`0`) turns the effects on (off).
pub fn apply_spec(s: &mut FxSettings, spec: &str) -> Result<(), String> {
    for item in spec.split(',').map(str::trim).filter(|i| !i.is_empty()) {
        let (key, val) = match item.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => ("effects", item),
        };
        let opt = options().iter().find(|o| o.key.eq_ignore_ascii_case(key)).ok_or_else(|| format!("unknown effect option {key}"))?;
        let v = match val.to_ascii_lowercase().as_str() {
            "on" | "true" | "yes" => 1,
            "off" | "false" | "no" | "none" => 0,
            "sharp" | "sharp-bilinear" | "low" => 1 - (opt.key == "quality") as usize,
            "edge" | "edge-directed" | "high" => 2,
            "medium" | "normal" => 1,
            "short" => 0,
            "long" => 2,
            "smooth" => 3,
            n => n.parse::<usize>().map_err(|_| format!("bad value {val} for {key}"))?,
        };
        if v >= opt.choices.len() {
            return Err(format!("bad value {val} for {key}"));
        }
        (opt.set)(s, v);
    }
    Ok(())
}

/// The temporary switch until the effects join the Enhanced graphics settings: `--fx` (on) and `--fx-set LIST` on the command
/// line, else `OPENSGOLF_FX` (a list as for [`apply_spec`], e.g. `1` or `scaler=sharp,tilt-shift=on`).
pub fn configure(flag: bool, spec: Option<&str>) {
    let mut s = settings();
    let env = std::env::var("OPENSGOLF_FX").ok();
    if let Some(e) = env.as_deref().filter(|_| !flag && spec.is_none()) {
        s.enabled = true;
        if let Err(err) = apply_spec(&mut s, e) {
            eprintln!("OPENSGOLF_FX: {err}");
        }
    }
    if flag || spec.is_some() {
        s.enabled = true;
    }
    FORCED.with(|f| f.set(s.enabled && (flag || spec.is_some() || env.is_some())));
    if let Some(sp) = spec {
        if let Err(err) = apply_spec(&mut s, sp) {
            eprintln!("--fx-set: {err}");
        }
    }
    set_settings(s);
}

thread_local! {
    /// The effects were turned on for this run by --fx / --fx-set / OPENSGOLF_FX, whatever the Enhanced master switch.
    static FORCED: Cell<bool> = const { Cell::new(false) };
}

/// The effects as the Enhanced graphics ticks set them (on with the master switch, or when forced on for the run).
pub fn from_enhanced(e: &crate::enhanced::Settings) -> FxSettings {
    let mut s = settings();
    s.enabled = e.enabled || FORCED.with(|f| f.get());
    s.scaler = if !e.fx_scaler { SCALER_OFF } else { [SCALER_SHARP, SCALER_EDGE, SCALER_SMOOTH][e.scaler_mode() as usize] };
    s.water = e.fx_water;
    s.lighting = e.fx_terrain_light;
    s.occlusion = e.fx_terrain_light;
    s.shadows = e.fx_terrain_light;
    s.soft_sprite_shadows = e.fx_sprite_shadows;
    s.sky = e.fx_sky;
    s.tilt_shift = e.fx_tilt_shift;
    s.grading = e.fx_grading;
    s.vignette = e.fx_grading;
    s.bloom = e.fx_bloom;
    s.quality = e.fx_quality.min(2);
    s.cast_shadows = e.fx_cast_shadows;
    s.time_of_day = e.fx_time_of_day;
    s.clouds = e.fx_clouds;
    s.rain = e.fx_rain;
    s.mist = e.fx_mist;
    s.haze = e.fx_haze;
    s.ball_trail = e.fx_ball_trail;
    s.ball_impacts = e.fx_ball_impacts;
    s.reflections = e.fx_reflections;
    s.wind = e.fx_wind;
    s.evening_lights = e.fx_evening_lights;
    s.seasons = e.fx_seasons;
    s.after_rain = e.fx_after_rain;
    s.lightning = e.fx_lightning;
    s.grass = e.fx_grass;
    s.sharpen = e.fx_sharpen;
    s.winter = e.fx_winter;
    s.crowds = e.fx_crowds;
    s.life = e.fx_life;
    s.day_length = e.fx_day_length.min(2);
    s.water_colour = e.fx_water_colour;
    s.waves = e.fx_waves;
    s.shoreline = e.fx_shoreline;
    s.caustics = e.fx_caustics;
    s.rain_rings = e.fx_rain_rings;
    s.water_life = e.fx_water_life;
    s.ice = e.fx_ice;
    s.spray = e.fx_spray;
    s
}

/// Puts the Enhanced settings' effects in use (at start up, before `configure` applies the run's own switches).
pub fn sync(e: &crate::enhanced::Settings) {
    set_settings(from_enhanced(e));
}

/// Whether the post-processing passes run: they need a depth buffer in an offscreen target, which the browser build does not
/// get (the effects that draw directly still run there).
fn post_supported() -> bool {
    !cfg!(target_arch = "wasm32")
}

fn wants_post(s: &FxSettings) -> bool {
    // photo mode always composites (its looks, focus blur and vignette; photo.rs), in the browser too
    s.enabled && ((post_supported() && (s.tilt_shift || s.grading || s.bloom || s.vignette || s.sharpen)) || crate::photo::post().is_some())
}

thread_local! {
    /// The size of the target being drawn when it is not the window (photo mode's larger photos, photo.rs).
    static RENDER_SIZE: Cell<Option<(f32, f32)>> = const { Cell::new(None) };
}

/// Drawing into a target of this size instead of the window (None: the window again), for the scaler's magnification.
pub fn set_render_size(size: Option<(f32, f32)>) {
    RENDER_SIZE.with(|r| r.set(size));
}

/// The size of the target being drawn when it is not the window (None: the window).
pub fn render_size() -> Option<(f32, f32)> {
    RENDER_SIZE.with(|r| r.get())
}

// ---- theme looks --------------------------------------------------------------------------------------------------------------

/// Per theme (the exe's order: parkland, desert, tropical, links): the sky's top, its haze at the horizon (also what the
/// water reflects low down), the haze below the map, the grade (contrast, vibrance), a tint and the bloom's threshold.
pub(crate) struct Look {
    pub(crate) sky_top: [f32; 3],
    pub(crate) sky_low: [f32; 3],
    pub(crate) below: [f32; 3],
    contrast: f32,
    vibrance: f32,
    tint: [f32; 3],
    bloom_from: f32,
}

const LOOKS: [Look; 4] = [
    Look {
        sky_top: [0.33, 0.55, 0.84],
        sky_low: [0.80, 0.89, 0.96],
        below: [0.47, 0.58, 0.66],
        contrast: 1.06,
        vibrance: 0.16,
        tint: [1.0, 1.0, 1.0],
        bloom_from: 0.78,
    },
    Look {
        sky_top: [0.38, 0.60, 0.86],
        sky_low: [0.97, 0.88, 0.72],
        below: [0.72, 0.56, 0.42],
        contrast: 1.05,
        vibrance: 0.10,
        tint: [1.03, 1.0, 0.96],
        bloom_from: 0.80,
    },
    Look {
        sky_top: [0.16, 0.58, 0.90],
        sky_low: [0.82, 0.96, 0.98],
        below: [0.34, 0.66, 0.72],
        contrast: 1.05,
        vibrance: 0.20,
        tint: [1.0, 1.01, 1.01],
        bloom_from: 0.80,
    },
    Look {
        sky_top: [0.47, 0.57, 0.70],
        sky_low: [0.83, 0.86, 0.88],
        below: [0.46, 0.52, 0.56],
        contrast: 1.08,
        vibrance: 0.07,
        tint: [0.98, 1.0, 1.03],
        bloom_from: 0.82,
    },
];

// ---- GPU state (in Gfx) -------------------------------------------------------------------------------------------------------

#[repr(C)]
struct ScalerUniforms {
    mvp: [f32; 16],
    sp: [f32; 4],
    alpha_ref: f32,
    soft: f32,
    pm: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GroundUniforms {
    mvp: [f32; 16],
    mv: [f32; 16],
    light_amb: [f32; 3],
    light_dif: [f32; 3],
    light_spec: [f32; 3],
    light_dir: [f32; 3],
    lm_rect: [f32; 4],
    params: [f32; 4],
    water: [f32; 4],
    eye_w: [f32; 3],
    light_w: [f32; 3],
    sky_top: [f32; 3],
    sky_low: [f32; 3],
    cloud: [f32; 4],
    cover: [f32; 4],
    mist: [f32; 4],
    mist_col: [f32; 3],
    fx1: [f32; 4],
    fx2: [f32; 4],
    fx3: [f32; 4],
}

#[repr(C)]
struct PassUniforms {
    p: [f32; 4],
}

#[repr(C)]
struct CompositeUniforms {
    tilt: [f32; 4],
    glow: [f32; 4],
    grade: [f32; 4],
    tint: [f32; 4],
}

/// The effects' pipelines and the full-screen quad, made the first time an effect draws.
#[derive(Clone, Copy)]
pub struct FxGpu {
    scaler: Pipeline,
    /// The scaler blending premultiplied colours (Enhanced's smooth interface textures).
    scaler_pm: Pipeline,
    /// The Smooth scaler (fx_smooth.rs), straight and premultiplied.
    smooth: Pipeline,
    smooth_pm: Pipeline,
    ground_solid: Pipeline,
    ground_overlay: Pipeline,
    water: Pipeline,
    down: Pipeline,
    blur: Pipeline,
    composite: Pipeline,
    /// The cast shadows' mask multiplied into the frame (fx_shadow.rs).
    pub(crate) shade: Pipeline,
    pub(crate) quad_vb: BufferId,
    pub(crate) quad_ib: BufferId,
    /// A 1 x 1 light map (full sun, no occlusion, far from water) for when none is built.
    no_lightmap: TextureId,
    /// The clouds' tiling noise (fx_light.rs).
    pub(crate) clouds: TextureId,
    /// A soft round dot (the balls' impacts and the rain's splashes, fx_ball.rs).
    pub(crate) dot: TextureId,
    /// A 1 x 1 detail map (no puddles, grass) and an empty reflection, for when none is made.
    no_detail: TextureId,
    no_reflection: TextureId,
}

/// An effects shader. Should the GPU refuse one (a phone's GLES compiler is stricter than a desktop's), the effects are turned
/// off for the rest of the run (`settings` then says so) and a plain stand-in takes its place, so the frame being drawn
/// still has a pipeline and the game goes on in Classic rather than closing.
pub(crate) fn shader(ctx: &mut dyn RenderingBackend, vs: &str, fs: &str, images: &[&str], uniforms: &[(&str, UniformType)]) -> ShaderId {
    let meta = || ShaderMeta {
        images: images.iter().map(|s| s.to_string()).collect(),
        uniforms: UniformBlockLayout { uniforms: uniforms.iter().map(|&(n, t)| UniformDesc::new(n, t)).collect() },
    };
    match ctx.new_shader(ShaderSource::Glsl { vertex: vs, fragment: fs }, meta()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Enhanced graphics: an effects shader does not compile on this GPU, the effects are off ({e:?})");
            BROKEN.with(|b| b.set(true));
            let fs = stand_in_fs(images.first().copied().unwrap_or("tex"));
            ctx.new_shader(ShaderSource::Glsl { vertex: glsl::POST_VS, fragment: &fs }, meta())
                .unwrap_or_else(|e| panic!("effects stand-in shader: {e:?}"))
        }
    }
}

/// The stand-in for an effects shader the GPU refused: the first picture, as it is.
fn stand_in_fs(image: &str) -> String {
    format!(
        "#version 100\nprecision mediump float;\nvarying highp vec2 uv;\nuniform sampler2D {image};\n\
         void main() {{ gl_FragColor = texture2D({image}, uv); }}\n"
    )
}

impl FxGpu {
    fn new(ctx: &mut dyn RenderingBackend) -> FxGpu {
        use UniformType::*;
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
        let pipe = |ctx: &mut dyn RenderingBackend, sh: ShaderId, depth_test: Comparison, depth_write: bool, b: bool| {
            ctx.new_pipeline(
                &[BufferLayout::default()],
                &attrs,
                sh,
                PipelineParams {
                    depth_test,
                    depth_write,
                    color_blend: if b { blend } else { None },
                    alpha_blend: if b { alpha_blend } else { None },
                    ..Default::default()
                },
            )
        };
        let scaler_sh = shader(
            ctx,
            glsl::SCALER_VS,
            &glsl::scaler_fs(),
            &["tex"],
            &[("mvp", Mat4), ("sp", Float4), ("alpha_ref", Float1), ("soft", Float1), ("pm", Float1)],
        );
        let ground_uniforms = [
            ("mvp", Mat4),
            ("mv", Mat4),
            ("light_amb", Float3),
            ("light_dif", Float3),
            ("light_spec", Float3),
            ("light_dir", Float3),
            ("lm_rect", Float4),
            ("params", Float4),
            ("water", Float4),
            ("eye_w", Float3),
            ("light_w", Float3),
            ("sky_top", Float3),
            ("sky_low", Float3),
            ("cloud", Float4),
            ("cover", Float4),
            ("mist", Float4),
            ("mist_col", Float3),
            ("fx1", Float4),
            ("fx2", Float4),
            ("fx3", Float4),
        ];
        let ground_images = ["tex", "lmap", "clouds", "fxmap", "reflt"];
        let terrain_sh = shader(ctx, glsl::GROUND_VS, &glsl::terrain_fs(), &ground_images, &ground_uniforms);
        let water_sh = shader(ctx, glsl::GROUND_VS, &glsl::water_fs(), &ground_images, &ground_uniforms);
        let shade_sh = shader(ctx, glsl::SHADE_VS, &glsl::shade_fs(), &["mask"], &[("tint", Float4)]);
        let down_sh = shader(ctx, glsl::POST_VS, &glsl::down_fs(), &["src"], &[("p", Float4)]);
        let blur_sh = shader(ctx, glsl::POST_VS, &glsl::blur_fs(), &["src"], &[("p", Float4)]);
        let comp_sh = shader(
            ctx,
            glsl::POST_VS,
            &glsl::composite_fs(),
            &["scene", "blurred", "bloom"],
            &[("tilt", Float4), ("glow", Float4), ("grade", Float4), ("tint", Float4)],
        );
        let smooth_sh = shader(
            ctx,
            glsl::SCALER_VS,
            &crate::fx_smooth::smooth_fs(),
            &["tex"],
            &[("mvp", Mat4), ("sp", Float4), ("alpha_ref", Float1), ("soft", Float1), ("pm", Float1)],
        );
        let pm_pipe = |ctx: &mut dyn RenderingBackend, sh| {
            ctx.new_pipeline(
                &[BufferLayout::default()],
                &attrs,
                sh,
                PipelineParams {
                    depth_test: Comparison::Always,
                    depth_write: false,
                    color_blend: alpha_blend,
                    alpha_blend,
                    ..Default::default()
                },
            )
        };
        let scaler = pipe(ctx, scaler_sh, Comparison::Always, false, true);
        let scaler_pm = pm_pipe(ctx, scaler_sh);
        let smooth = pipe(ctx, smooth_sh, Comparison::Always, false, true);
        let smooth_pm = pm_pipe(ctx, smooth_sh);
        let ground_solid = pipe(ctx, terrain_sh, Comparison::Less, true, false);
        let ground_overlay = pipe(ctx, terrain_sh, Comparison::Less, false, true);
        let water = pipe(ctx, water_sh, Comparison::Less, true, false);
        let down = pipe(ctx, down_sh, Comparison::Always, false, false);
        let blur = pipe(ctx, blur_sh, Comparison::Always, false, false);
        let composite = pipe(ctx, comp_sh, Comparison::Always, false, false);
        // multiply: the frame times the shade, its alpha kept
        let shade = ctx.new_pipeline(
            &[BufferLayout::default()],
            &attrs,
            shade_sh,
            PipelineParams {
                depth_test: Comparison::Greater,
                depth_write: false,
                color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::Value(BlendValue::SourceColor))),
                alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One)),
                ..Default::default()
            },
        );
        let v = |x: f32, y: f32| Vert::new(x, y, 0.0, (x + 1.0) * 0.5, (y + 1.0) * 0.5);
        let quad = [v(-1.0, -1.0), v(1.0, -1.0), v(1.0, 1.0), v(-1.0, -1.0), v(1.0, 1.0), v(-1.0, 1.0)];
        let quad_vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&quad));
        let quad_ib = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&[0u32, 1, 2, 3, 4, 5]));
        let no_lightmap = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 0]);
        let clouds = crate::fx_light::cloud_texture(ctx);
        let dot = crate::fx_ball::dot_texture(ctx);
        let no_detail = ctx.new_texture_from_rgba8(1, 1, &[0, 255, 0, 0]);
        let no_reflection = ctx.new_texture_from_rgba8(1, 1, &[0, 0, 0, 0]);
        FxGpu {
            scaler,
            scaler_pm,
            smooth,
            smooth_pm,
            ground_solid,
            ground_overlay,
            water,
            down,
            blur,
            composite,
            shade,
            quad_vb,
            quad_ib,
            no_lightmap,
            clouds,
            dot,
            no_detail,
            no_reflection,
        }
    }
}

pub(crate) fn gpu(g: &mut Gfx) -> &FxGpu {
    if g.fx.is_none() {
        g.fx = Some(Box::new(FxGpu::new(&mut *g.ctx)));
    }
    g.fx.as_deref().expect("effects pipelines")
}

// ---- the pixel-art scaler -----------------------------------------------------------------------------------------------------

/// How a texture holds its colours, for the scaler: straight, premultiplied (Enhanced's smooth interface textures), or
/// straight with the distance to a smoothed outline in alpha (Enhanced's smooth edges for colour-keyed art, edges.rs).
pub const TEX_STRAIGHT: u8 = 0;
pub const TEX_PREMUL: u8 = 1;
pub const TEX_SDF: u8 = 2;

/// Whether the scaler may take over a batch (called by Gfx::flush for the flat 2D / sprite draws).
pub fn scaler_wanted() -> bool {
    let s = settings();
    s.enabled && s.scaler != SCALER_OFF && !SCALER_PAUSED.with(|p| p.get())
}

thread_local! {
    /// The scaler is not used for a while (the reflections' offscreen pass, fx_reflect.rs).
    static SCALER_PAUSED: Cell<bool> = const { Cell::new(false) };
}

/// Holds the scaler off (true) or lets it run again (false).
pub(crate) fn pause_scaler(on: bool) {
    SCALER_PAUSED.with(|p| p.set(on));
}

/// Draw state for a batch of flat quads drawn with the scaler: sets the pipeline, bindings and uniforms and returns true, or
/// returns false (the caller draws the classic way) when the picture is not magnified, is text, or the batch is not quads.
/// The magnification is measured on the batch's first quad edge (its first two vertices). `colours`: how the texture holds
/// its colours (TEX_STRAIGHT, TEX_PREMUL or TEX_SDF; with the last two the batch's vertex colours are premultiplied and the
/// output is too).
#[allow(clippy::too_many_arguments)]
pub fn apply_scaler(
    ctx: &mut Box<dyn RenderingBackend>,
    fx: &mut Option<Box<FxGpu>>,
    batch: &[Vert],
    vb: BufferId,
    ib: BufferId,
    tex: TextureId,
    u: &Uniforms,
    colours: u8,
) -> bool {
    if batch.len() < 6 || PLAIN.with(|p| p.borrow().contains(&tex)) {
        return false;
    }
    let (tw, th) = ctx.texture_size(tex);
    let Some(scale) = crate::gfx::batch_scale(&**ctx, batch, tex, u) else { return false };
    if scale < 1.05 {
        return false;
    }
    // edge-directed rules need room to show; at low magnifications and on the lowest quality the sharp filter does it (the
    // Smooth filter interpolates, so it already helps from a little magnification on)
    let s = settings();
    let mode = match s.scaler {
        SCALER_EDGE if scale >= 1.5 && s.quality > 0 => SCALER_EDGE,
        SCALER_SMOOTH if scale >= 1.2 && s.quality > 0 => SCALER_SMOOTH,
        _ => SCALER_SHARP,
    };
    if fx.is_none() {
        *fx = Some(Box::new(FxGpu::new(&mut **ctx)));
    }
    let Some(f) = fx.as_deref() else { return false };
    let (straight, pm) = if mode == SCALER_SMOOTH { (&f.smooth, &f.smooth_pm) } else { (&f.scaler, &f.scaler_pm) };
    ctx.apply_pipeline(if colours != TEX_STRAIGHT { pm } else { straight });
    ctx.apply_bindings(&Bindings { vertex_buffers: vec![vb], index_buffer: ib, images: vec![tex] });
    ctx.apply_uniforms(UniformsSource::table(&ScalerUniforms {
        mvp: u.mvp,
        sp: [tw as f32, th as f32, scale, mode as f32],
        alpha_ref: u.alpha_ref,
        soft: if mode != SCALER_SHARP { SOFT_EDGE.with(|s| s.get()) } else { SOFT_SHARP.with(|s| s.get()) },
        pm: colours as f32,
    }));
    true
}

// ---- world state (in App) -----------------------------------------------------------------------------------------------------

struct PostTargets {
    size: (u32, u32),
    small: (u32, u32),
    scene: TextureId,
    depth: TextureId,
    pass: RenderPass,
    msaa: Option<[TextureId; 2]>,
    /// Quarter (or half) size: blur ping-pong and bloom ping-pong.
    tex: [TextureId; 4],
    passes: [RenderPass; 4],
}

/// The effects' per-course state: the water's meshes, the light map, the post-processing targets, softened sprite shadows.
#[derive(Default)]
pub struct FxWorld {
    water: Vec<crate::app::Batch>,
    lightmap: Option<TextureId>,
    lightmap_key: u64,
    lm_rect: [f32; 4],
    post: Option<PostTargets>,
    /// (sprite, frame) -> (the classic shadow texture it was made from, the softened one).
    shadows: HashMap<(usize, usize), (TextureId, TextureId)>,
    pub(crate) time: f32,
    last_clock: f64,
    /// The course is being drawn offscreen this frame (fx_world_begin .. fx_world_end).
    post_on: bool,
    /// The pass the course is being drawn into this frame (the frame's own, or the post-processing's scene).
    pub(crate) pass_now: Option<RenderPass>,
    /// This frame's sun, sky and weather (fx_light.rs).
    pub(crate) atmos: crate::fx_light::Atmos,
    /// The low ground's height and depth for the mist (world units), from the last light map.
    pub(crate) mist_level: (f32, f32),
    /// The sun the light map's shadows were marched towards (quantised), with the time of day on.
    lm_sun: (i32, i32),
    /// Cast shadows (fx_shadow.rs): the mask target and the silhouettes.
    pub(crate) cast: crate::fx_shadow::CastState,
    /// Ball trails and impacts (fx_ball.rs).
    pub(crate) balls: crate::fx_ball::BallState,
    /// Sprite kinds and the seasons' recoloured trees (fx_nature.rs).
    pub(crate) nature: crate::fx_nature::NatureState,
    /// The detail map (puddle potential, grass; fx_wet.rs) and the key of the ground it was made for.
    detail: Option<TextureId>,
    detail_key: u64,
    /// The reflections' target (fx_reflect.rs), its pass and size, and whether it was drawn this frame.
    pub(crate) reflect: Option<(TextureId, RenderPass, (u32, u32))>,
    /// Near-water tiles (fx_reflect.rs): per tile, the water's height when water is within a few tiles.
    pub(crate) water_near: Vec<Option<f32>>,
    pub(crate) water_near_key: u64,
    /// Tiles with water of more than one level within reach (a stream stepping down a slope): nothing is mirrored there
    /// (fx_reflect.rs), as a reflection made for one level would show on the water below it.
    pub(crate) water_mixed: Vec<bool>,
    /// How far the clouds have drifted (fx_light.rs; world units, within one period of their noise).
    pub(crate) cloud_acc: [f32; 2],
    /// The evening lights: windows' glow per sprite frame (fx_night.rs).
    pub(crate) night: crate::fx_night::NightState,
    /// The bodies of water and each tile's distance from the land (fx_water.rs).
    pub(crate) ponds: crate::fx_water::Ponds,
}

impl FxWorld {
    /// Whether water batches are kept (to drop when the effects are switched off).
    pub fn has_water(&self) -> bool {
        !self.water.is_empty()
    }
}

/// Terrain texture types that are water (Terrain.dll's WaterShallow, WaterMiddle, WaterDeep, WaterShallowDesert).
pub fn is_water_tex(tex_type: i32) -> bool {
    matches!(tex_type, 17 | 23 | 24 | 25)
}

/// Whether rebuild_batches should set the water apart for the water shader.
pub fn water_on() -> bool {
    let s = settings();
    s.enabled && s.water
}

/// The rotation of the view (eye from world), as render_world builds it.
pub(crate) fn view_rotation(app: &App) -> [f32; 16] {
    Mat4::rotate(crate::app::pitch_for(app.draw_w, app.draw_h), 1.0, 0.0, 0.0).mul(&Mat4::rotate((45.0 + app.rot) as f64, 0.0, 1.0, 0.0)).0
}

/// An eye-space direction in world space (the view's rotation transposed).
pub(crate) fn to_world(m: &[f32; 16], e: [f32; 3]) -> [f32; 3] {
    let mut w = [0.0; 3];
    for (c, o) in w.iter_mut().enumerate() {
        *o = (0..3).map(|r| m[c * 4 + r] * e[r]).sum();
    }
    w
}

/// A world-space direction in eye space (the view's rotation).
pub(crate) fn to_eye(m: &[f32; 16], w: [f32; 3]) -> [f32; 3] {
    let mut e = [0.0; 3];
    for (r, o) in e.iter_mut().enumerate() {
        *o = (0..3).map(|c| m[c * 4 + r] * w[c]).sum();
    }
    e
}

impl App {
    /// After the terrain's batches are built (rebuild_batches, while the effects are on or have water batches to drop): keeps
    /// the water batches it set apart and makes the light map when the ground changed.
    pub fn fx_terrain_built(&mut self, g: &mut Gfx, water: Vec<crate::app::Batch>) {
        for b in std::mem::replace(&mut self.fx.water, water) {
            g.delete_mesh(b.mesh);
        }
        let s = settings();
        if !s.enabled {
            return;
        }
        self.fx.mist_level = mist_level(&self.terrain);
        self.fx_lightmap(g, false);
        self.fx_detail_map(g);
        if s.reflections {
            self.fx_water_near();
        }
        if s.water_life || s.spray {
            self.fx_ponds();
        }
    }

    /// Makes the detail map (puddle potential and grass, fx_wet.rs) when the ground changed and the seasons or the wet
    /// ground want it.
    fn fx_detail_map(&mut self, g: &mut Gfx) {
        let s = settings();
        if !(s.after_rain || s.seasons || s.lighting) {
            return;
        }
        let n = per_tile([4usize, 8, 8][s.quality.min(2) as usize], &self.terrain);
        let key = lightmap_key(&self.terrain, n, 0, FxSettings::default()) ^ 0x5d;
        if self.fx.detail.is_some() && self.fx.detail_key == key {
            return;
        }
        let img = crate::fx_wet::build_detail(&self.terrain, n);
        if let Some(old) = self.fx.detail.take() {
            g.ctx.delete_texture(old);
        }
        let tex = g.ctx.new_texture_from_data_and_format(
            &img.px,
            TextureParams { width: img.w, height: img.h, format: TextureFormat::RGBA8, ..Default::default() },
        );
        self.fx.detail = Some(tex);
        self.fx.detail_key = key;
    }

    /// Makes the light map when the ground, the view's turn or (with the time of day) the sun changed. `sun_moved`: only
    /// the sun may have moved (the ground's key is kept).
    pub(crate) fn fx_lightmap(&mut self, g: &mut Gfx, sun_moved: bool) {
        let s = settings();
        let need = s.shadows || s.occlusion || s.water || s.mist;
        let n = per_tile([2usize, 4, 8][s.quality.min(2) as usize], &self.terrain);
        let mut light_w = to_world(&view_rotation(self), sg_core::terrain::light_direction());
        let mut sun_q = (0, 0);
        if s.time_of_day {
            light_w = self.fx.atmos.sun_w;
            sun_q = crate::fx_light::sun_step(light_w);
        }
        if sun_moved && sun_q == self.fx.lm_sun {
            return;
        }
        self.fx.lm_sun = sun_q;
        let key =
            lightmap_key(&self.terrain, n, (self.rot.round() as i32).rem_euclid(360), s) ^ (sun_q.0 as u64) << 40 ^ (sun_q.1 as u64) << 52;
        if !need {
            return;
        }
        if self.fx.lightmap.is_some() && self.fx.lightmap_key == key {
            return;
        }
        let t0 = crate::app::now();
        let img = build_lightmap(&self.terrain, n, light_w, s.shadows, s.occlusion);
        if std::env::var_os("SG_FX_LOG").is_some() {
            let t = &self.terrain;
            let wet: Vec<(i32, i32)> =
                (0..t.w * t.h).map(|k| (k % t.w, k / t.w)).filter(|&(x, y)| t.type_at(x, y) == sg_core::course::t::WATER as i32).collect();
            let n = wet.len().max(1) as i32;
            let mid = (wet.iter().map(|w| w.0).sum::<i32>() / n, wet.iter().map(|w| w.1).sum::<i32>() / n);
            println!(
                "fx: light map {}x{} for {}x{} tiles in {:.0} ms; {} water tiles around {:?}",
                img.w,
                img.h,
                t.w,
                t.h,
                (crate::app::now() - t0) * 1000.0,
                wet.len(),
                mid
            );
        }
        if let Some(f) = std::env::var_os("SG_FX_DUMP") {
            sg_core::png::write_png(std::path::Path::new(&f), &img);
        }
        if let Some(old) = self.fx.lightmap.take() {
            g.ctx.delete_texture(old);
        }
        let tex = g.ctx.new_texture_from_data_and_format(
            &img.px,
            TextureParams { width: img.w, height: img.h, format: TextureFormat::RGBA8, ..Default::default() },
        );
        self.fx.lightmap = Some(tex);
        self.fx.lightmap_key = key;
        let (w, h) = (self.terrain.w as f32 * TILE_SIZE, self.terrain.h as f32 * TILE_SIZE);
        self.fx.lm_rect = [-w * 0.5, -h * 0.5, 1.0 / w, 1.0 / h];
    }

    /// Takes new settings (from the Enhanced graphics preferences): what the terrain's meshes depend on is rebuilt.
    pub fn fx_set(&mut self, s: FxSettings) {
        let old = settings();
        set_settings(s);
        if old != s {
            self.dirty = true;
        }
    }

    pub(crate) fn look(&self) -> &'static Look {
        &LOOKS[self.exe_theme().min(3) as usize]
    }

    /// Draws the ground in place of render_world's classic loops: the sky behind, then terrain, walls and paths with the
    /// effects' light, and the water.
    pub fn fx_draw_ground(&mut self, g: &mut Gfx, proj: &Mat4, mv: &Mat4, lit: &Uniforms, overlay: &Uniforms) {
        let s = settings();
        let dt = (self.clock - self.fx.last_clock).clamp(0.0, 0.1) as f32;
        self.fx.last_clock = self.clock;
        self.fx.time += dt;
        // the sun, sky and weather of this moment (fx_light.rs): the classic midday unless the time of day or rain is on
        let atm = self.fx_atmos();
        self.fx.atmos = atm;
        let period = 1.0 / crate::fx_light::CLOUD_REPEAT;
        self.fx.cloud_acc = [
            (self.fx.cloud_acc[0] + atm.cloud_vel[0] * dt).rem_euclid(period),
            (self.fx.cloud_acc[1] + atm.cloud_vel[1] * dt).rem_euclid(period),
        ];
        if s.time_of_day && s.shadows {
            self.fx_lightmap(g, true);
        }
        let (mut lit, mut overlay) = (*lit, *overlay);
        if atm.tinted {
            for l in [&mut lit, &mut overlay] {
                for k in 0..3 {
                    l.light_amb[k] *= atm.amb[k];
                    l.light_dif[k] *= atm.sun[k];
                    l.light_spec[k] *= atm.sun[k];
                }
                l.light_dir = atm.sun_e;
            }
        }
        let (lit, overlay) = (&lit, &overlay);
        if s.sky {
            self.fx_sky(g, &atm);
        }
        let season_on = atm.season.fresh + atm.season.autumn + atm.season.frost + atm.season.dew + atm.snow > 0.0;
        let terrain_fx = s.lighting || s.occlusion || s.shadows || atm.cloud > 0.0 || atm.mist > 0.0 || atm.wet > 0.0 || season_on;
        let rot = view_rotation(self);
        let lm = self.fx.lightmap.unwrap_or_else(|| gpu(g).no_lightmap);
        let clouds = gpu(g).clouds;
        let has_lm = self.fx.lightmap.is_some();
        if (atm.wet > 0.0 || season_on || s.lighting) && self.fx.detail.is_none() {
            self.fx_detail_map(g);
        }
        let detail = self.fx.detail.unwrap_or_else(|| gpu(g).no_detail);
        // what stands round the water, drawn mirrored for the water shader (fx_reflect.rs)
        let reflt = if s.reflections && !self.fx.water.is_empty() { self.fx_reflections(g, proj, mv) } else { None };
        let upp = 2.0 / (proj.0[0] * self.draw_w).max(1e-6);
        let u = GroundUniforms {
            mvp: proj.mul(mv).0,
            mv: mv.0,
            light_amb: lit.light_amb,
            light_dif: lit.light_dif,
            light_spec: lit.light_spec,
            light_dir: lit.light_dir,
            lm_rect: if has_lm { self.fx.lm_rect } else { [0.0, 0.0, 0.0, 0.0] },
            params: [
                s.lighting as i32 as f32,
                if s.shadows && has_lm { 0.62 * atm.shadow } else { 0.0 },
                if s.occlusion && has_lm { 1.0 } else { 0.0 },
                -1.0,
            ],
            water: [
                self.fx.time,
                if s.reflections { 0.22 + 0.2 * atm.golden } else { 0.22 },
                if s.reflections { 1.0 + 1.8 * atm.golden } else { 1.0 },
                0.0,
            ],
            eye_w: [rot[2], rot[6], rot[10]],
            light_w: if atm.tinted { atm.sun_w } else { to_world(&rot, sg_core::terrain::light_direction()) },
            sky_top: atm.sky_top,
            sky_low: atm.sky_low,
            cloud: [atm.cloud, crate::fx_light::CLOUD_REPEAT, atm.cloud_off[0], atm.cloud_off[1]],
            cover: [atm.cover, atm.cover + 0.22, 1.0 / self.draw_w.max(1.0), 1.0 / self.draw_h.max(1.0)],
            mist: [atm.mist, self.fx.mist_level.0, 1.0 / self.fx.mist_level.1.max(1.0), self.fx.time.rem_euclid(10000.0)],
            mist_col: atm.mist_col,
            fx1: [atm.wet, 1.02 - 0.5 * atm.puddles, atm.rain, self.fx.time.rem_euclid(1000.0)],
            // the frost's slot carries the snow above 1 (fx_winter.rs): 1 + 2 x snow, the frost full under snow
            fx2: [
                atm.season.fresh * 0.8,
                atm.season.autumn,
                if atm.snow > 0.0 { 1.0 + 2.0 * atm.snow.min(1.0) } else { atm.season.frost.min(1.0) },
                atm.season.dew,
            ],
            fx3: [
                if reflt.is_some() { 1.0 } else { 0.0 },
                // the smooth ground normals (SG_FX_FACETED=1: the tiles' flat faces, as before, for comparisons)
                if s.lighting && self.fx.detail.is_some() && std::env::var_os("SG_FX_FACETED").is_none() { 1.0 } else { 0.0 },
                if s.reflections { 1.0 } else { 0.0 },
                1.0 / (upp * 1.6).max(0.05),
            ],
        };
        let reflt = reflt.unwrap_or_else(|| gpu(g).no_reflection);
        let imgs = |t: TextureId| [t, lm, clouds, detail, reflt];
        // the water's own options (fx_water.rs): in slots the water shader does not otherwise use (fx_glsl.rs water_fs)
        let mut u = u;
        let mut uw = u;
        if s.shoreline {
            u.fx3[2] = 2.0;
            uw.fx3[2] = 2.0;
        }
        let wu = self.fx_water_uniforms(&atm, 2.0 / (self.view_zoom() * (self.draw_w / 800.0).min(self.draw_h / 600.0)).max(1e-6));
        uw.fx1 = wu.fx1;
        uw.params[3] = wu.heading;
        uw.fx3[3] = wu.size;
        uw.water[3] = wu.haze;
        g.flush();
        let p = *gpu(g);
        for b in self.batches.iter().chain(&self.wall_batches) {
            if terrain_fx {
                g.draw_mesh_with(&p.ground_solid, &b.mesh, &imgs(b.tex), UniformsSource::table(&u));
            } else {
                g.draw_mesh(Mode::Solid, &b.mesh, b.tex, lit);
            }
        }
        for b in &self.fx.water {
            g.draw_mesh_with(&p.water, &b.mesh, &imgs(b.tex), UniformsSource::table(&uw));
        }
        let mut uo = u;
        uo.params[3] = overlay.alpha_ref;
        for b in self.path_batches.iter().chain(&self.mud_batches) {
            if terrain_fx {
                g.draw_mesh_with(&p.ground_overlay, &b.mesh, &imgs(b.tex), UniformsSource::table(&uo));
            } else {
                g.draw_mesh(Mode::Overlay, &b.mesh, b.tex, overlay);
            }
        }
    }

    /// The sky behind the course: its colour at the top of the screen fading to the horizon's haze, then to the haze below
    /// the map at the bottom.
    fn fx_sky(&self, g: &mut Gfx, look: &crate::fx_light::Atmos) {
        let u = Uniforms::flat(&Mat4::identity(), &Mat4::identity());
        let c = |rgb: [f32; 3]| [rgb[0], rgb[1], rgb[2], 1.0];
        let (top, mid, low) = (c(look.sky_top), c(look.sky_low), c(look.below));
        let v = |x: f32, y: f32, col: [f32; 4]| Vert::new(x, y, 0.0, 0.0, 0.0).col(col);
        g.quad(Mode::Flat, None, &u, [v(-1.0, 1.0, top), v(1.0, 1.0, top), v(1.0, 0.1, mid), v(-1.0, 0.1, mid)]);
        g.quad(Mode::Flat, None, &u, [v(-1.0, 0.1, mid), v(1.0, 0.1, mid), v(1.0, -1.0, low), v(-1.0, -1.0, low)]);
    }

    /// A softened copy of a sprite's shadow frame (the art's own shadow: none is invented), or None to draw the classic one.
    pub fn fx_shadow(&mut self, g: &mut Gfx, si: usize, view: i32, frame: i32, classic: TextureId) -> Option<TextureId> {
        let s = settings();
        if !s.enabled || !s.soft_sprite_shadows || crate::hd::active() {
            return None;
        }
        let i = self.sprites[si].s.frame_index(view, frame);
        if let Some(&(src, t)) = self.fx.shadows.get(&(si, i)) {
            if src == classic {
                return Some(t);
            }
            g.ctx.delete_texture(t);
        }
        let img = soften_shadow(&self.sprites[si].s.frames[i]);
        let t = g.texture(&img, false);
        self.fx.shadows.insert((si, i), (classic, t));
        Some(t)
    }

    /// Before the course is drawn: with post-processing on, moves the drawing to the offscreen scene target and returns true.
    /// `target` is the frame's own pass (where the course is drawn without post-processing).
    pub fn fx_world_begin(&mut self, g: &mut Gfx, target: Option<RenderPass>) -> bool {
        self.fx.pass_now = target;
        let s = settings();
        if !wants_post(&s) {
            return false;
        }
        g.flush();
        g.ctx.end_render_pass();
        let size = (self.draw_w as u32, self.draw_h as u32);
        let div = if s.quality >= 2 { 2 } else { 4 };
        if self.fx.post.as_ref().is_some_and(|p| p.size != size || p.small != (size.0.div_ceil(div), size.1.div_ceil(div))) {
            if let Some(p) = self.fx.post.take() {
                free_post(g, p);
            }
        }
        if self.fx.post.is_none() {
            self.fx.post = Some(make_post(g, size, div));
        }
        let Some(p) = &self.fx.post else { return false };
        let b = if s.time_of_day || s.rain { crate::fx_light::below_now(self) } else { self.look().below };
        g.ctx.begin_pass(Some(p.pass), PassAction::clear_color(b[0], b[1], b[2], 1.0));
        self.fx.pass_now = Some(p.pass);
        self.fx.post_on = true;
        true
    }

    /// After the course: blurs and composites it into `target` (the frame's own pass), where the interface then draws.
    pub fn fx_world_end(&mut self, g: &mut Gfx, target: Option<RenderPass>) {
        if !std::mem::take(&mut self.fx.post_on) {
            return;
        }
        g.flush();
        g.ctx.end_render_pass();
        let s = settings();
        let look = self.look();
        let Some(p) = &self.fx.post else { return };
        let f = *gpu(g);
        let full = (p.size.0 as f32, p.size.1 as f32);
        let small = (p.small.0 as f32, p.small.1 as f32);
        let k = full.0 / small.0 / 4.0;
        let run = |g: &mut Gfx, pipe: &Pipeline, pass: RenderPass, src: TextureId, u: [f32; 4]| {
            g.ctx.begin_pass(Some(pass), PassAction::Nothing);
            g.ctx.apply_pipeline(pipe);
            g.ctx.apply_bindings(&Bindings { vertex_buffers: vec![f.quad_vb], index_buffer: f.quad_ib, images: vec![src] });
            g.ctx.apply_uniforms(UniformsSource::table(&PassUniforms { p: u }));
            g.ctx.draw(0, 6, 1);
            g.ctx.end_render_pass();
        };
        // a photo larger than the window (photo.rs) blurs as far across the picture as the screen shows
        let spread = crate::photo::post().map(|ph| ph.spread).unwrap_or(1.0);
        let blur = |g: &mut Gfx, a: usize, b: usize| {
            run(g, &f.blur, p.passes[b], p.tex[a], [spread / small.0, 0.0, 0.0, 0.0]);
            run(g, &f.blur, p.passes[a], p.tex[b], [0.0, spread / small.1, 0.0, 0.0]);
        };
        // photo mode (photo.rs): its own focus blur, as many blur passes as its strength, and its own composite
        let photo = crate::photo::post();
        let tilt = (s.tilt_shift && s.quality > 0 && photo.is_none()) || photo.is_some_and(|ph| ph.blur > 0);
        let bloom = s.bloom && s.quality > 0;
        if tilt {
            run(g, &f.down, p.passes[0], p.scene, [k / full.0 * 2.0, k / full.1 * 2.0, 0.0, 0.0]);
            for _ in 0..photo.map(|ph| ph.blur_passes()).unwrap_or(2) {
                blur(g, 0, 1);
            }
        }
        if bloom {
            run(g, &f.down, p.passes[2], p.scene, [k / full.0 * 2.0, k / full.1 * 2.0, 1.0, look.bloom_from]);
            blur(g, 2, 3);
        }
        g.ctx.begin_pass(target, PassAction::Nothing);
        if let Some(ph) = photo {
            let theme = crate::photo::ThemeGrade { contrast: look.contrast, vibrance: look.vibrance, tint: look.tint };
            crate::photo::composite(g, &f, [p.scene, p.tex[0], p.tex[2]], ph, theme, if bloom { 0.45 } else { 0.0 }, full.0 / full.1);
            return;
        }
        g.ctx.apply_pipeline(&f.composite);
        g.ctx.apply_bindings(&Bindings {
            vertex_buffers: vec![f.quad_vb],
            index_buffer: f.quad_ib,
            images: vec![p.scene, p.tex[0], p.tex[2]],
        });
        let gr = |on: bool, v: f32| if on { v } else { 0.0 };
        g.ctx.apply_uniforms(UniformsSource::table(&CompositeUniforms {
            tilt: [gr(tilt, 1.0), 0.5, 0.16, 0.0],
            glow: [gr(bloom, 0.45), gr(s.vignette, 0.28), full.0 / full.1, gr(s.sharpen, 0.6)],
            // z, w: the scene's texel for the sharpening
            grade: [gr(s.grading, look.contrast), look.vibrance, 1.0 / full.0, 1.0 / full.1],
            tint: [look.tint[0], look.tint[1], look.tint[2], 1.0],
        }));
        g.ctx.draw(0, 6, 1);
    }
}

fn make_post(g: &mut Gfx, size: (u32, u32), div: u32) -> PostTargets {
    let (w, h) = size;
    let small = (w.div_ceil(div).max(1), h.div_ceil(div).max(1));
    let params = |w: u32, h: u32, format| TextureParams { width: w, height: h, format, ..Default::default() };
    let scene = g.ctx.new_render_texture(params(w, h, TextureFormat::RGBA8));
    let depth = g.ctx.new_render_texture(params(w, h, TextureFormat::Depth));
    // with HD on, the course keeps its anti-aliasing: a multisampled pass resolving into the scene
    let msaa = if crate::hd::active() || crate::enhanced::msaa() { crate::hd::msaa_pass(g, scene, w, h) } else { None };
    let pass = msaa.map(|m| m.0).unwrap_or_else(|| g.ctx.new_render_pass(scene, Some(depth)));
    let tex = [0; 4].map(|_| g.ctx.new_render_texture(params(small.0, small.1, TextureFormat::RGBA8)));
    let passes = tex.map(|t| g.ctx.new_render_pass(t, None));
    // start black, so a pass that is off this frame composites nothing stale
    for p in passes {
        g.ctx.begin_pass(Some(p), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        g.ctx.end_render_pass();
    }
    PostTargets { size, small, scene, depth, pass, msaa: msaa.map(|m| m.1), tex, passes }
}

fn free_post(g: &mut Gfx, p: PostTargets) {
    g.ctx.delete_render_pass(p.pass);
    for pass in p.passes {
        g.ctx.delete_render_pass(pass);
    }
    for t in p.tex.into_iter().chain([p.scene, p.depth]).chain(p.msaa.into_iter().flatten()) {
        g.ctx.delete_texture(t);
    }
    // miniquad keeps the deleted textures in its cache of bound textures: a new one under the same GL name would not be
    // bound again (photo.rs); clearing the cache (what the end of a frame does) makes the next draws bind theirs
    g.ctx.commit_frame();
}

// ---- the light map ------------------------------------------------------------------------------------------------------------

fn lightmap_key(t: &Terrain, n: usize, rot: i32, s: FxSettings) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (t.w, t.h, n, rot, s.shadows, s.occlusion).hash(&mut h);
    for y in 0..=t.h {
        for x in 0..=t.w {
            t.corner_at(x, y).hash(&mut h);
        }
    }
    for y in 0..t.h {
        for x in 0..t.w {
            (t.type_at(x, y) == sg_core::course::t::WATER as i32).hash(&mut h);
        }
    }
    h.finish()
}

/// The ground's heights on the corner grid, sampled in tile units like Terrain::height_at.
struct Heights {
    w: usize,
    h: usize,
    c: Vec<f32>,
    max: f32,
}

impl Heights {
    fn new(t: &Terrain) -> Heights {
        let (w, h) = (t.w.max(1) as usize, t.h.max(1) as usize);
        let mut c = Vec::with_capacity((w + 1) * (h + 1));
        for y in 0..=h {
            for x in 0..=w {
                c.push(t.corner_at(x as i32, y as i32) as f32 * HEIGHT_STEP);
            }
        }
        let max = c.iter().copied().fold(f32::MIN, f32::max);
        Heights { w, h, c, max }
    }
    fn at(&self, fx: f32, fz: f32) -> f32 {
        let fx = fx.clamp(0.0, self.w as f32 - 0.001);
        let fz = fz.clamp(0.0, self.h as f32 - 0.001);
        let (x, y) = (fx as usize, fz as usize);
        let (u, v) = (fx - x as f32, fz - y as f32);
        let r = self.w + 1;
        let a = self.c[y * r + x] * (1.0 - u) + self.c[y * r + x + 1] * u;
        let b = self.c[(y + 1) * r + x] * (1.0 - u) + self.c[(y + 1) * r + x + 1] * u;
        a * (1.0 - v) + b * v
    }
}

/// The light map, `n` texels a tile: r = sun visibility (soft shadows of the relief, marched towards the decoded light), g =
/// ambient occlusion (baked per vertex of the tile meshes' 3 x 3 grid from the height field's horizon, valleys darker, spread
/// to the texels bilinearly), b = signed distance to the shore (0.5 at the shore, +-3 tiles to 1 / 0: fx_water.rs SHORE_TILES),
/// a = water (254 inside a small pond: `small_ponds`).
/// Texels per tile for the light and detail maps: the quality's count, fewer on a larger course size so a map keeps at most
/// about 400 texels a side (the original's 50 tiles at 8), which bounds the maps' build time and memory (2 at the least).
fn per_tile(n: usize, t: &Terrain) -> usize {
    n.min((400 / t.w.max(t.h).max(1) as usize).max(2))
}

fn build_lightmap(t: &Terrain, n: usize, light_w: [f32; 3], shadows: bool, occlusion: bool) -> Rgba {
    let hs = Heights::new(t);
    let (tw, th) = (hs.w, hs.h);
    let (w, h) = (tw * n, th * n);
    let mut img = Rgba::new(w as u32, h as u32);
    // ambient occlusion per mesh vertex (half a tile apart)
    let (aw, ah) = (2 * tw + 1, 2 * th + 1);
    let mut ao = vec![1.0f32; aw * ah];
    if occlusion {
        const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
        const DIRS: [(f32, f32); 8] = [(1.0, 0.0), (D, D), (0.0, 1.0), (-D, D), (-1.0, 0.0), (-D, -D), (0.0, -1.0), (D, -D)];
        const DIST: [f32; 5] = [0.5, 1.0, 1.5, 2.5, 4.0];
        for gz in 0..ah {
            for gx in 0..aw {
                let (px, pz) = (gx as f32 * 0.5, gz as f32 * 0.5);
                let h0 = hs.at(px, pz);
                // the horizon each way (how much sky the point sees) and the curvature (a valley's floor sits below the
                // ground around it), both from the height field
                let (mut occ, mut around) = (0.0, 0.0);
                for (dx, dz) in DIRS {
                    let mut slope = 0.0f32;
                    for d in DIST {
                        slope = slope.max((hs.at(px + dx * d, pz + dz * d) - h0) / (d * TILE_SIZE));
                    }
                    occ += slope / (1.0 + slope * slope).sqrt();
                    around += hs.at(px + dx * 1.5, pz + dz * 1.5);
                }
                let valley = ((around / 8.0 - h0) / (1.5 * TILE_SIZE)).max(0.0);
                ao[gz * aw + gx] = (1.0 - 1.4 * occ / 8.0 - 1.6 * valley).clamp(0.35, 1.0);
            }
        }
    }
    // sun visibility per texel
    let mut sun = vec![1.0f32; w * h];
    let horiz = (light_w[0] * light_w[0] + light_w[2] * light_w[2]).sqrt();
    if shadows && horiz > 1e-3 && light_w[1] > 0.0 {
        let (lx, lz) = (light_w[0] / horiz, light_w[2] / horiz);
        let rise = light_w[1] / horiz * TILE_SIZE; // world units up per tile towards the light
        let row = |j: usize, out: &mut [f32]| {
            for (i, o) in out.iter_mut().enumerate() {
                let (px, pz) = ((i as f32 + 0.5) / n as f32, (j as f32 + 0.5) / n as f32);
                let h0 = hs.at(px, pz) + 0.5;
                let mut vis = 1.0f32;
                let mut s = 0.06f32;
                while s < 12.0 {
                    let ray = h0 + s * rise;
                    if ray > hs.max + 1.0 {
                        break;
                    }
                    let d = ray - hs.at(px + lx * s, pz + lz * s);
                    // a penumbra widening with the distance to what casts it
                    vis = vis.min(3.0 * d / (s * TILE_SIZE));
                    if vis <= 0.0 {
                        break;
                    }
                    s += (0.08 * s).max(0.5 / n as f32);
                }
                let v = vis.clamp(0.0, 1.0);
                *o = v * v * (3.0 - 2.0 * v);
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).clamp(1, 16);
            let rows_per = h.div_ceil(threads).max(1);
            std::thread::scope(|sc| {
                for (k, chunk) in sun.chunks_mut(rows_per * w).enumerate() {
                    let row = &row;
                    sc.spawn(move || {
                        for (r, out) in chunk.chunks_mut(w).enumerate() {
                            row(k * rows_per + r, out);
                        }
                    });
                }
            });
        }
        #[cfg(target_arch = "wasm32")]
        for (j, out) in sun.chunks_mut(w).enumerate() {
            row(j, out);
        }
    }
    // water, and the signed distance to the shore in texels (a chamfer transform each way)
    // the land outside the property (type 20, not drawn) is neither: no foam along the property's edge
    let class: Vec<u8> = (0..w * h)
        .map(|k| match t.type_at((k % w / n) as i32, (k / w / n) as i32) {
            x if x == sg_core::course::t::WATER as i32 => 1,
            20 => 2,
            _ => 0,
        })
        .collect();
    let water: Vec<bool> = class.iter().map(|&c| c == 1).collect();
    let to_land = chamfer(&class, w, h, 0);
    let to_water = chamfer(&class, w, h, 1);
    let pond = small_ponds(&water, &to_land, w, h, n);
    for k in 0..w * h {
        let (i, j) = (k % w, k / w);
        // occlusion: bilinear between the vertex grid's values
        let (fx, fz) = ((i as f32 + 0.5) / n as f32 * 2.0, (j as f32 + 0.5) / n as f32 * 2.0);
        let (x0, z0) = ((fx as usize).min(aw - 2), (fz as usize).min(ah - 2));
        let (u, v) = ((fx - x0 as f32).clamp(0.0, 1.0), (fz - z0 as f32).clamp(0.0, 1.0));
        let a = ao[z0 * aw + x0] * (1.0 - u) + ao[z0 * aw + x0 + 1] * u;
        let b = ao[(z0 + 1) * aw + x0] * (1.0 - u) + ao[(z0 + 1) * aw + x0 + 1] * u;
        let occ = a * (1.0 - v) + b * v;
        let sd = if water[k] { to_land[k] - 0.5 } else { -(to_water[k] - 0.5) } / n as f32 / crate::fx_water::SHORE_TILES;
        let p = &mut img.px[k * 4..k * 4 + 4];
        p[0] = (sun[k] * 255.0).round() as u8;
        p[1] = (occ * 255.0).round() as u8;
        p[2] = ((0.5 + sd * 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
        p[3] = if pond[k] {
            254
        } else if water[k] {
            255
        } else {
            0
        };
    }
    img
}

/// The water texels inside a small pond, one no more than about three tiles across (its middle at most 1.6 tiles from the
/// shore), which freezes over in a cold snap where a larger water only ices at its rim (fx_glsl.rs `water_fs`; the port's
/// own). Only texels two or more in from the shore are marked, so the land's bilinear look-ups never reach them.
fn small_ponds(water: &[bool], to_land: &[f32], w: usize, h: usize, n: usize) -> Vec<bool> {
    let mut out = vec![false; w * h];
    let mut seen = vec![false; w * h];
    let mut stack = Vec::new();
    let mut body = Vec::new();
    for start in 0..w * h {
        if !water[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        body.clear();
        let mut deepest = 0.0f32;
        while let Some(k) = stack.pop() {
            body.push(k);
            deepest = deepest.max(to_land[k]);
            let (i, j) = (k % w, k / w);
            let mut go = |m: usize| {
                if water[m] && !seen[m] {
                    seen[m] = true;
                    stack.push(m);
                }
            };
            if i > 0 {
                go(k - 1);
            }
            if i + 1 < w {
                go(k + 1);
            }
            if j > 0 {
                go(k - w);
            }
            if j + 1 < h {
                go(k + w);
            }
        }
        if deepest <= 1.6 * n as f32 + 0.5 {
            for &k in &body {
                out[k] = to_land[k] >= 2.0;
            }
        }
    }
    out
}

/// Distance in texels (a chamfer transform, 1 and sqrt 2 steps) from each texel to the nearest texel whose class is `target`;
/// far away is 1e6.
fn chamfer(class: &[u8], w: usize, h: usize, target: u8) -> Vec<f32> {
    let mut d: Vec<f32> = class.iter().map(|&c| if c == target { 0.0 } else { 1e6 }).collect();
    let (a, b) = (1.0f32, std::f32::consts::SQRT_2);
    for j in 0..h {
        for i in 0..w {
            let k = j * w + i;
            let mut v = d[k];
            if i > 0 {
                v = v.min(d[k - 1] + a);
            }
            if j > 0 {
                v = v.min(d[k - w] + a);
                if i > 0 {
                    v = v.min(d[k - w - 1] + b);
                }
                if i + 1 < w {
                    v = v.min(d[k - w + 1] + b);
                }
            }
            d[k] = v;
        }
    }
    for j in (0..h).rev() {
        for i in (0..w).rev() {
            let k = j * w + i;
            let mut v = d[k];
            if i + 1 < w {
                v = v.min(d[k + 1] + a);
            }
            if j + 1 < h {
                v = v.min(d[k + w] + a);
                if i + 1 < w {
                    v = v.min(d[k + w + 1] + b);
                }
                if i > 0 {
                    v = v.min(d[k + w - 1] + b);
                }
            }
            d[k] = v;
        }
    }
    d
}

/// A shadow frame with its density blurred (a small Gaussian), so its edge fades softly into the ground.
fn soften_shadow(img: &Rgba) -> Rgba {
    let (w, h) = (img.w as usize, img.h as usize);
    const K: [f32; 5] = [0.0625, 0.25, 0.375, 0.25, 0.0625];
    let a: Vec<f32> = img.px.chunks_exact(4).map(|p| p[3] as f32).collect();
    let mut tmp = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for (k, wt) in K.iter().enumerate() {
                let xx = x as i32 + k as i32 - 2;
                if xx >= 0 && (xx as usize) < w {
                    s += a[y * w + xx as usize] * wt;
                }
            }
            tmp[y * w + x] = s;
        }
    }
    let mut out = img.clone();
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for (k, wt) in K.iter().enumerate() {
                let yy = y as i32 + k as i32 - 2;
                if yy >= 0 && (yy as usize) < h {
                    s += tmp[yy as usize * w + x] * wt;
                }
            }
            let p = &mut out.px[(y * w + x) * 4..(y * w + x) * 4 + 4];
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
            p[3] = s.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_parse() {
        let mut s = FxSettings::default();
        apply_spec(&mut s, "on,scaler=sharp,tilt-shift=on,quality=high,bloom=0").unwrap();
        assert!(s.enabled && s.tilt_shift && !s.bloom);
        assert_eq!((s.scaler, s.quality), (SCALER_SHARP, 2));
        apply_spec(&mut s, "quality=low,scaler=edge").unwrap();
        assert_eq!((s.scaler, s.quality), (SCALER_EDGE, 0));
        apply_spec(&mut s, "scaler=smooth").unwrap();
        assert_eq!(s.scaler, SCALER_SMOOTH);
        assert!(apply_spec(&mut s, "nonsense=1").is_err());
        assert!(apply_spec(&mut s, "scaler=7").is_err());
    }

    #[test]
    fn options_round_trip() {
        let mut s = FxSettings::default();
        for o in options() {
            for v in 0..o.choices.len() {
                (o.set)(&mut s, v);
                assert_eq!((o.get)(&s), v, "{}", o.key);
            }
        }
    }

    #[test]
    fn chamfer_counts_texels() {
        let class = [1, 0, 2, 0];
        let d = chamfer(&class, 4, 1, 1);
        assert_eq!(d, vec![0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn small_ponds_freeze_over_lakes_do_not() {
        // 2 texels a tile: a 3 x 3 tile pond (6 x 6 texels) and a 12 x 12 tile lake (24 x 24) on a 40 x 30 texel map
        let (w, h, n) = (40, 30, 2);
        let mut class = vec![0u8; w * h];
        for j in 2..8 {
            for i in 2..8 {
                class[j * w + i] = 1;
            }
        }
        for j in 3..27 {
            for i in 12..36 {
                class[j * w + i] = 1;
            }
        }
        let water: Vec<bool> = class.iter().map(|&c| c == 1).collect();
        let to_land = chamfer(&class, w, h, 0);
        let pond = small_ponds(&water, &to_land, w, h, n);
        // the pond's inner texels are marked, its shore texels not (the land's look-ups never meet the mark)
        assert!(pond[4 * w + 4] && pond[5 * w + 5]);
        assert!(!pond[2 * w + 2] && !pond[4 * w + 2]);
        // nothing of the lake
        assert!((0..w * h).filter(|&k| k % w >= 12).all(|k| !pond[k]));
    }

    #[test]
    fn shaders_have_their_corners() {
        let s = crate::fx_glsl::scaler_fs();
        assert_eq!(s.matches("c = corner(").count(), 4);
        assert_eq!(s.matches(" = T(cell").count(), 21);
    }
}

/// The low ground's height for the mist: a little above the lowest quarter of the corners, and the depth over which the mist
/// thins out (world units).
fn mist_level(t: &Terrain) -> (f32, f32) {
    let mut c: Vec<i32> = (0..=t.h).flat_map(|y| (0..=t.w).map(move |x| (x, y))).map(|(x, y)| t.corner_at(x, y)).collect();
    if c.is_empty() {
        return (0.0, 2.0 * HEIGHT_STEP);
    }
    c.sort_unstable();
    let q = c[c.len() / 4] as f32 * HEIGHT_STEP;
    (q + 1.0 * HEIGHT_STEP, 2.5 * HEIGHT_STEP)
}
