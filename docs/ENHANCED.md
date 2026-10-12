# Enhanced graphics (optional, not in the original)

SimGolf was made for an 800×600 screen. On a 4K or ultrawide monitor the port scales that screen up, which keeps the
original's look but leaves soft text, soft interface art and a HUD bunched in the middle of a wide window. **Enhanced
graphics** is an opt-in way of drawing the same game for today's screens. It changes no art: everything is drawn from the
same files, only with different filtering, at the screen's own resolution, and placed for the window's shape.

Enhanced is **not** part of the original game and is not a parity feature (docs/PARITY.md): it is the port's own option, off by
default.

## The Classic guarantee

**Classic is the default and is never changed by Enhanced.** Every hook into the drawing code asks one of the switches in
`crates/simgolf/src/enhanced.rs` first (`native_text()`, `smooth_ui()`, `widescreen()`, ...), and all of them are false while the
master switch is off. With Enhanced off the game runs the code it ran before Enhanced existed and draws the same pixels:
this was checked by rendering the same eight scenes (800×600, 1280×800, 1600×900, 1920×1080, 3440×1440 and 3840×2160; the
course with and without panels, the title menu, the Course Report, the roster, a loaded game) with the build before
Enhanced and the build with it, and comparing the PNGs byte for byte. Switching Enhanced off in the game (or starting with
`--classic-render`) gives the classic frame again; the test that turns it off at run time and compares against a fresh
Classic run matched byte for byte too.

Enhanced never touches the simulation. Saves, the random generator and the game's rules are the same with it on or off.
The one place where a rule reads the camera is the zoom: the great-shot SimFoto, the thoughts and the like depend on the
original's three zoom levels. With Enhanced's continuous zoom those rules read the **nearest** original level
(`App::exe_zoom`), so a shot is still judged "at the closest zoom" exactly when the view is nearest that level.

Enhanced and HD (docs/HD.md) can be combined: HD replaces the art with the player's upscaled pack, Enhanced changes how it
is filtered, placed and animated.

## Turning it on

- **In the game:** System Functions → Preferences → *Enhanced graphics...* opens a checkbox list: the master switch
  *Enhanced graphics (off: Classic)* and one line per option below. Tick what you want and click the OK tick. The choice is
  saved in `settings.json` in the save folder, under `"enhanced"`:

  ```json
  "enhanced": { "enabled": true, "native_text": true, "smooth_ui": true, "widescreen": true,
                "terrain_filter": true, "msaa": true, "smooth_camera": true, "smooth_motion": true }
  ```

  Options that the textures carry (smooth interface art, sharper course textures) and the master switch make the art again
  when they change; the others take effect on the next frame.
- **Command line:** `--enhanced` turns the master switch on for one run, `--classic-render` turns it off (neither is saved;
  the options keep their saved values). `--classic` is the HD switch (docs/HD.md), not this one.
- **Test hook:** `--edit M:5` opens the Enhanced graphics screen, for stills.

## The options

| Option (settings key) | What it does |
|---|---|
| Sharp text at the screen's resolution (`native_text`) | Text is rasterised from the TrueType faces (Klepto, Manual SSi, Arial or its stand-in) at the size it really has on the screen (the size asked for times the window's scale), cached per face and pixel size, and drawn texel for pixel on whole pixels. The pen still advances by the classic metrics in 800×600 units, so line breaks, centring and every layout are unchanged. Classic bakes the faces once at 32 pixels and scales them. |
| Smooth interface art at any size (`smooth_ui`) | Interface pictures, sprites, portraits and HD pack pictures are uploaded with premultiplied alpha and mipmaps (trilinear). Scaled up or down, their edges blend with what is under them without the dark or magenta fringes that straight alpha leaves; drawn smaller (a phone, a small window, the far zoom) they do not shimmer. A texture made this way is drawn with a premultiplied blend state; every other texture keeps the classic one, so the two can be mixed safely. |
| Widescreen: interface at the window's edges (`widescreen`) | On a window that is not 4:3 the course already fills the window; with this option the HUD goes to the window's edges too: the course badge in the top left corner, the rating pills in the top right, the dock, its panels and the face strip in the bottom left, the leader board top left, messages, the golfer card and *Paused* at the top centre, the aim box bottom right, the touch buttons on the right edge. Reports, dialogs and pop-ups stay centred. Full-screen pages (title screens, the trophy room, Tracts for Sale, the Routing Map, the pair chooser, Top 10, Customise) are pillarboxed: the bars get a dark green gradient (over the course, translucent so the course shows dimly) and a soft shadow along the page's edges. The pointer follows: each frame remembers the rectangles it drew and the shift each had, and a pointer over one of them is mapped back into that piece's own 800×600 layout, so every click and hover test of the game works unchanged. Tested at 16:9 (1920×1080, 3840×2160) and 21:9 (3440×1440). |
| Sharper course textures (`terrain_filter`) | The terrain's mipmapped textures get anisotropic filtering (up to 8×) where the driver has `GL_EXT_texture_filter_anisotropic` (or OpenGL 4.6), so the course stays sharp towards the top of the screen at the steep viewing angle. Skipped in browsers and on phones (WebGL 1 / GLES 2 do not promise it) and wherever the driver does not know the setting. |
| Anti-aliasing (`msaa`) | The frame is drawn into a 4× multisampled target and resolved (the HD mode's target): smooth terrain edges, cliffs and lines. Desktop only, where the renderer can resolve multisampled targets. |
| Smooth camera: zoom, turns and panning (`smooth_camera`) | The mouse wheel and a two-finger pinch zoom continuously between the original's farthest and closest levels (a wheel notch is a quarter of a doubling); the zoom buttons and z / x still step to the next original level. Every zoom change is eased. A quarter turn (PageUp, Home, the dock's turn buttons, a twist) is animated instead of snapping; the sprites switch to the next view half way through. Arrow-key scrolls glide, and a dragged course keeps moving for a moment when let go. The rules keep the original's levels: see *The Classic guarantee*. |
| Smooth motion between game ticks (`smooth_motion`) | The simulation runs in 87 ms ticks (about 11.5 a second) as the original does; the screen is drawn at the display's refresh rate (60 to 144 Hz, miniquad's swap interval 1). With this option golfers, staff, celebrity residents, animals, carts, the balloon, blimp, sailboat and birds, and the balls (their flight arcs too, without the original's whole-pixel height steps) are drawn between where they were at the last two ticks instead of jumping once a tick. Moves of more than two tiles in a tick (a golfer sent home, a new ball) are not smoothed. This is drawing only: the simulation, saves and tests are unchanged. It shows things one tick (87 ms) behind the simulation. |

## Effects

Shader effects on the course and on the art, in `crates/simgolf/src/fx.rs` (settings, light map, drawing) and
`crates/simgolf/src/fx_glsl.rs` (the shaders). None of them edits or replaces a picture: they read the classic art, the
decoded terrain light (`sg_core::terrain::Lighting`, `light_direction`) and the height field, and draw them differently.

### What each effect does

| Option (key) | Default with Enhanced | What it does |
|---|---|---|
| Pixel-art scaling (`scaler`) | edge-directed | Sprites and interface pictures that are drawn larger than their pixels (an 800 × 600 game on a 4K screen is magnified 3.6 times) go through a scaler instead of plain bilinear blur. **Sharp bilinear** keeps every art pixel crisp and blends only a one-screen-pixel seam between them. **Edge-directed** is an xBR-style filter (levels 1 and 2: 45° and the shallow / steep 30° edges, evaluated for the four corners of each texel from its 5 × 5 neighbourhood) on a softened sharp-bilinear base: outlines and diagonals become smooth lines instead of stair steps. Colours are compared and mixed premultiplied, so the colour-keyed transparent pixels count as one colour whatever RGB they hide and sprite outlines get smooth, fringe-free edges. Text is left alone (the font atlases are marked plain), so are pictures that are not magnified (HD pack art, far zooms). |
| Animated water (`water`) | on | Water tiles get their own shader: travelling waves bent by noise perturb the normal; the surface reflects the theme's sky (pale low down, the deeper colour only on slopes turned up), and facets the waves turn towards the decoded light's half vector glint. Shallow water foams in drifting bands along the shore (from a signed distance to the land, at the light map's resolution). The theme's own water textures stay the base, desert water swap included; where an edge piece shows its bank (rocks, grass, sand), the effects fade out by comparing each pixel with the texture's own middle colour, so the bank art is untouched. The decoded ripple, rock, dolphin, fountain and waterfall sprites still draw on top. |
| Per-pixel terrain light (`lighting`) | on | The decoded light model (Terrain.dll's light 0, the theme's `Lighting.txt` colours, OpenGL's default material, specular 13, the port's brightness calibration) computed per pixel from the interpolated normals instead of per vertex. Same colours, smoother highlights. |
| Ambient occlusion (`occlusion`) | on | Baked per vertex of the tile meshes' 3 × 3 grid from the height field: how much of the horizon the point sees in eight directions, plus how far it sits below the ground around it (valley floors darker). Spread to the light map bilinearly and applied to the ambient and direct light. |
| Soft terrain shadows (`shadows`) | on | A height-field shadow pass: from every light-map texel a ray marches towards the decoded light (in world space: the eye-space light turned back by the view, 25° above the horizon) over the heights, keeping the smallest clearance over distance, which gives a penumbra that widens with the distance to the caster. Shadowed ground keeps 38% of its direct light. The game's hills are gentle, so the shadows are mostly a soft darkening behind ridges and below raised ground. |
| Soft sprite shadows (`sprite-shadows`) | on | The art's own shadow frames (`*Shadow.flc`, drawn under buildings, trees and people) are blurred by a small Gaussian so their edges fade into the ground. No shadow is invented where the art has none. Off in HD (the pack's frames are used as they are). |
| Sky (`sky`) | on | Instead of the classic black beyond the property and below its cliff, a gradient from the theme's sky through a haze at mid screen to the haze below the map. Classic keeps its black. |
| Tilt-shift (`tilt-shift`) | off | Depth of field by screen height: the course is sharp in a band through the middle of the screen and blurs towards the top and bottom, a "miniature" look. |
| Colour grading (`grading`) | on | Gentle contrast and vibrance (saturates the dull colours more than the vivid ones) and a slight tint, per theme: neutral parkland, warm desert, clear tropical, cool links. |
| Bloom (`bloom`) | on | The brightest parts (sparkle, white sand, highlights) glow a little: a bright pass, blurred at a quarter (high quality: half) of the resolution, added back. |
| Vignette (`vignette`) | on | Slightly darker screen corners. |
| Quality (`quality`) | medium | Low: light map 2 texels a tile, no blur passes (no tilt-shift, no bloom), sharp-bilinear scaling only. Medium: 4 texels a tile, blurs at a quarter of the resolution. High: 8 texels a tile, blurs at half the resolution. |

The post-processing (tilt-shift, grading, bloom, vignette) draws the course into an offscreen target, then composites it
into the frame; the interface (dock, panels, text, screens over the course) is drawn after that and is not affected.
With HD or Enhanced's anti-aliasing on, that offscreen course keeps the 4× multisampling. The scaler also handles the
premultiplied textures of Enhanced's *smooth interface art* (it blends them premultiplied).

### Turning it on

The effects are part of Enhanced graphics: with the master switch on, System Functions → Preferences → *Enhanced
graphics...* has a tick for each (stored in `settings.json` under `"enhanced"` as `fx_scaler`, `fx_scaler_sharp`,
`fx_water`, `fx_terrain_light`, `fx_sprite_shadows`, `fx_sky`, `fx_tilt_shift`, `fx_grading`, `fx_bloom`), all on but
tilt-shift and *sharp bilinear instead of edge-directed*. The ticks map onto `fx::FxSettings` (`fx::sync`): *terrain light*
stands for per-pixel light, occlusion and shadows, *colour grading* for grading and vignette; the quality is medium.

Finer control and tests, for one run:

- `--fx` turns the effects on (alone, without the rest of Enhanced) with the defaults above; `--fx-set LIST` also sets
  options, a comma separated list of `key=value` (value: `on` / `off`, a choice's number, or `sharp`, `edge`, `low`,
  `medium`, `high`), for example `--fx-set scaler=sharp,tilt-shift=on,quality=high`; the keys are those of the table above;
- otherwise the environment variable `OPENSGOLF_FX` (`1`, or a list as above);
- `--fx-list` prints the options, their labels and choices (`fx::options()`: key, label, choices, get, set).

### How Classic stays the same

Every hook is behind `fx::active()`, false unless the effects were turned on:

- `enhanced.rs`: the Preferences ticks and their settings fields; `apply_enhanced` hands them to `fx` (`App::fx_set`).
- `gfx.rs`: `Gfx::flush` asks `fx::apply_scaler` to draw a flat batch only when the scaler is wanted; otherwise the classic
  pipeline. `Gfx` holds the effects' pipelines (`fx`, None until used) and gained `draw_mesh_with` for them.
- `render.rs`: `render_world` calls `fx_draw_ground` instead of its three classic loops (terrain, walls, paths) when active;
  `draw_props` swaps a shadow frame for its softened copy when active.
- `app.rs`: `rebuild_batches` sets the water triangles apart and builds the light map only when active.
- `main.rs`: `draw_frame` moves the course offscreen and composites it only when post-processing is on.
- `ui.rs`: the font atlases are marked as text for the scaler (no effect on drawing).

Checked by rendering the same scenes with the branch before the effects and with them off (four themes at 3840 × 2160,
1920 × 1080, 1280 × 1024 and 800 × 600, a rotated view, a panel open, a played save, the title menu and a report) and
comparing the PNGs: all identical.

### Browser and old GPUs

All shaders are GLSL ES 1.00 (WebGL 1 / GLES 2): no derivatives, no dynamic loops, highp in fragment shaders only where
`GL_FRAGMENT_PRECISION_HIGH` is defined, uniforms shared by both stages declared with the same precision. The
post-processing needs a depth buffer in an offscreen target, which the browser build does not get, so in the browser it
is skipped (tilt-shift, grading, bloom, vignette); the scaler, water, light, shadows and sky still run there. The light
map is built on the CPU (a few milliseconds; threaded on desktop) when the ground's heights, water or the view's turn
change, not every frame.

### Performance

The cost is per pixel and small: the terrain shader adds one light-map fetch and the light per pixel; water about a dozen
noise evaluations; the edge-directed scaler 22 texture fetches per sprite or interface pixel (the costliest effect); the
composite three fetches per pixel plus blur passes at a quarter of the resolution (six small passes with bloom and
tilt-shift). On a mid-range GPU that is well under a millisecond for each at 3840 × 2160. `SG_FX_BENCH=N` (with `--png`)
draws N frames offscreen and prints the mean frame time, to measure on real hardware. The test machine has no GPU (Mesa's software renderer),
where only relative costs mean anything:

| 3840 × 2160, software renderer (llvmpipe, 4 shared cores, best of 3 runs of 6 frames) | ms a frame |
|---|---|
| Classic | 117 |
| effects on, every option off | 123 |
| scaler, sharp bilinear | 278 |
| scaler, edge-directed | 851 |
| water | 308 |
| light + occlusion + shadows | 252 |
| sky | 154 |
| grading + vignette | 161 |
| bloom | 190 |
| tilt-shift | 214 |
| defaults, quality low / medium / high | 233 / 468 / 711 |

On the CPU renderer every texture fetch is expensive, which is why the edge-directed scaler dominates there; a GPU does
these fetches in parallel from cache. The runs varied by up to 2× with other work on the machine.

If a GPU struggles: quality low (no blur passes, sharp scaler), then the scaler sharp or off, then tilt-shift and bloom off.

### Limitations

- The light map follows the heights and water, not the sand bunkers' dished centres (the mesh's own normals still light them).
- The shadows only come from the terrain; buildings and trees keep the shadows drawn into their art.
- With very gentle hills the decoded light (25° up) casts little shadow; the occlusion is what shows the relief.
- Tilt-shift blurs by screen height only (the camera is orthographic: there is no real depth to focus on).

## For developers: adding an option

The other Enhanced options (shaders and effects) add themselves the same way:

1. a field in `enhanced::Settings` (with `#[serde(default)]` old settings files still read) and its default in
   `Settings::DEFAULT`;
2. an `OptionEntry` in `enhanced::OPTIONS` (settings key, a text key in `lang/en.txt` whose text starts with a space, getter,
   setter, and `Reload::Art` when the option changes how textures are made);
3. a switch like `pub fn smooth_ui() -> bool { on(|s| s.smooth_ui) }`, asked before the new code runs. It is false whenever
   the master switch is off, which is what keeps Classic byte-identical.

The Preferences screen lists `OPTIONS` in order (bit 0 the master switch, bit 1 + i option i, at most 31 options). Interface
code that should sit at a window edge calls `Screen::anchor(Anchor::...)` before drawing; nothing happens in Classic or
with the widescreen option off.
