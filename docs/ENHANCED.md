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

- **In the game:** System Functions → Preferences → the *Enhanced graphics* tick switches it on and off; *Enhanced graphics settings...* opens a checkbox list in four pages, picked with
  the tabs along its top (click or tap a tab; Left, Right or Tab on the keyboard): **Display** (the options below),
  **Effects** (the shader effects, cast shadows, the balls' trail and impacts, and the effects' quality), **Atmosphere**
  (sky, time of day, the day's length, evening lights, clouds, haze, water reflections, tilt-shift, grading and bloom) and
  **Weather** (rain, wet ground after it, lightning, mist, wind and the seasons). The box is wide enough for the four tabs. The master switch *Enhanced graphics (off:
  Classic)* heads every page. The box keeps one size whatever the page, so it fits an 800 × 600 window. Tick what you want
  (the quality line steps through low, medium and high on each click) and click the OK tick; what was ticked on every page
  is taken together, Esc forgets it all. The choice is saved in `settings.json` in the save folder, under `"enhanced"`:

  ```json
  "enhanced": { "enabled": true, "native_text": true, "smooth_ui": true, "widescreen": true,
                "terrain_filter": true, "msaa": true, "smooth_camera": true, "smooth_motion": true }
  ```

  Options that the textures carry (smooth interface art, sharper course textures) and the master switch make the art again
  when they change; the others take effect on the next frame.
- **Command line:** `--enhanced` turns the master switch on for one run, `--classic-render` turns it off (neither is saved;
  the options keep their saved values). `--classic` is the HD switch (docs/HD.md), not this one.
- **Test hook:** `--edit M:5` opens the Enhanced graphics screen, for stills (`M:5,1` on its Effects page, `M:5,2` on
  Atmosphere, `M:5,3` on Weather).

## The options

| Option (settings key) | What it does |
|---|---|
| Sharp text at the screen's resolution (`native_text`) | Text is rasterised from the TrueType faces (Klepto, Manual SSi, Arial or its stand-in) at the size it really has on the screen (the size asked for times the window's scale), cached per face and pixel size, and drawn texel for pixel on whole pixels. The pen still advances by the classic metrics in 800×600 units, so line breaks, centring and every layout are unchanged. Classic bakes the faces once at 32 pixels and scales them. |
| Smooth interface art at any size (`smooth_ui`) | Interface pictures, sprites, portraits and HD pack pictures are uploaded with premultiplied alpha and mipmaps (trilinear). Scaled up or down, their edges blend with what is under them without the dark or magenta fringes that straight alpha leaves; drawn smaller (a phone, a small window, the far zoom) they do not shimmer. A texture made this way is drawn with a premultiplied blend state; every other texture keeps the classic one, so the two can be mixed safely. Colour-keyed pictures (all-or-nothing alpha: the dock, the title art, the theme icons, cursors, sprites) also lose their stair-stepped outlines on a large window (edges.rs): their alpha is replaced by the distance to a smoothed outline (an exact distance transform smoothed with a Gaussian, by at most half a texel, only where the shape is at least a few texels thick, never across a texel centre), and FRAGMENT_SDF in gfx.rs (or the pixel-art scaler, `pm` 2) turns it into coverage one screen pixel wide at whatever size the picture is drawn. Thin lines, single texels and one-texel gaps keep their width; the colours are untouched (only the outline is smoothed: jagged edges drawn inside the art's colours are the pixel-art scaler's job, its *Smooth* filter above all). |
| Widescreen: interface at the window's edges (`widescreen`) | On a window that is not 4:3 the course already fills the window; with this option the HUD goes to the window's edges too: the course badge in the top left corner, the rating pills in the top right, the dock, its panels and the face strip in the bottom left, the leader board top left, messages, the golfer card and *Paused* at the top centre, the aim box bottom right, the touch buttons on the right edge. Reports, dialogs and pop-ups stay centred. Full-screen pages (title screens, the trophy room, Tracts for Sale, the Routing Map, the pair chooser, Top 10, Customise) are pillarboxed: the bars get a dark green gradient (over the course, translucent so the course shows dimly) and a soft shadow along the page's edges. The pointer follows: each frame remembers the rectangles it drew and the shift each had, and a pointer over one of them is mapped back into that piece's own 800×600 layout, so every click and hover test of the game works unchanged. Tested at 16:9 (1920×1080, 3840×2160) and 21:9 (3440×1440). |
| Sharper course textures (`terrain_filter`) | The terrain's mipmapped textures get anisotropic filtering (up to 8×) where the driver has `GL_EXT_texture_filter_anisotropic` (or OpenGL 4.6), so the course stays sharp towards the top of the screen at the steep viewing angle. Skipped in browsers and on phones (WebGL 1 / GLES 2 do not promise it) and wherever the driver does not know the setting. |
| Anti-aliasing (`msaa`) | The frame is drawn into a 4× multisampled target and resolved (the HD mode's target): smooth terrain edges, cliffs and lines. Desktop only, where the renderer can resolve multisampled targets. |
| Smooth camera: zoom, turns and panning (`smooth_camera`) | The mouse wheel and a two-finger pinch zoom continuously between the original's farthest and closest levels (a wheel notch is a quarter of a doubling); the zoom buttons and z / x still step to the next original level. Every zoom change is eased. A quarter turn (PageUp, Home, the dock's turn buttons, a twist) is animated instead of snapping; the sprites switch to the next view half way through. Arrow-key scrolls glide, and a dragged course keeps moving for a moment when let go. The rules keep the original's levels: see *The Classic guarantee*. |
| Photo mode (`photo_mode`) | Lets photo mode open (the C key or the camera button over the course): see *Photo mode* below. On by default. |
| Smooth motion between game ticks (`smooth_motion`) | The simulation runs in 87 ms ticks (about 11.5 a second) as the original does; the screen is drawn at the display's refresh rate (60 to 144 Hz, miniquad's swap interval 1). With this option golfers, staff, celebrity residents, animals, carts, the balloon, blimp, sailboat and birds, and the balls (their flight arcs too, without the original's whole-pixel height steps) are drawn between where they were at the last two ticks instead of jumping once a tick. Moves of more than two tiles in a tick (a golfer sent home, a new ball) are not smoothed. This is drawing only: the simulation, saves and tests are unchanged. It shows things one tick (87 ms) behind the simulation. |

## Effects

Shader effects on the course and on the art, in `crates/simgolf/src/fx.rs` (settings, light map, drawing) and
`crates/simgolf/src/fx_glsl.rs` (the shaders). None of them edits or replaces a picture: they read the classic art, the
decoded terrain light (`sg_core::terrain::Lighting`, `light_direction`) and the height field, and draw them differently.

### What each effect does

| Option (key) | Default with Enhanced | What it does |
|---|---|---|
| Pixel-art scaling (`scaler`) | smooth | Sprites and interface pictures that are drawn larger than their pixels (an 800 × 600 game on a 4K screen is magnified 3.6 times) go through a scaler instead of plain bilinear blur, with one of three filters. **Sharp bilinear** keeps every art pixel crisp and blends only a one-screen-pixel seam between them. **Edge-directed** is an xBR-style filter (levels 1 and 2: 45° and the shallow / steep 30° edges, evaluated for the four corners of each texel from its 5 × 5 neighbourhood) on a softened sharp-bilinear base: outlines and diagonals become smooth lines instead of stair steps where the art has runs of equal colours. **Smooth (strongest)**, the default, follows the art's own edges (see *The Smooth scaler* below): the stair steps inside the art too (the dock's rings, plus and minus, turn arrows, the diagonal lines of the badges and frames, flag poles, railings, roof edges) become smooth lines at any magnification, while flat areas, soft shading and textured art keep the softened sharp-bilinear look. Colours are compared and mixed premultiplied, so the colour-keyed transparent pixels count as one colour whatever RGB they hide and sprite outlines get smooth, fringe-free edges. Text is left alone (the font atlases are marked plain), so are pictures that are not magnified (HD pack art, far zooms). |
| Animated water (`water`) | on | Water tiles get their own shader: travelling waves bent by noise perturb the normal; the surface reflects the theme's sky (pale low down, the deeper colour only on slopes turned up), and facets the waves turn towards the decoded light's half vector glint. Shallow water foams in drifting bands along the shore (from a signed distance to the land, at the light map's resolution). The theme's own water textures stay the base, desert water swap included; where an edge piece shows its bank (rocks, grass, sand), the effects fade out by comparing each pixel with the texture's own middle colour, so the bank art is untouched. The decoded ripple, rock, dolphin, fountain and waterfall sprites still draw on top. |
| Per-pixel terrain light (`lighting`) | on | The decoded light model (Terrain.dll's light 0, the theme's `Lighting.txt` colours, OpenGL's default material, specular 13, the port's brightness calibration) computed per pixel instead of per vertex. Same colours, smoother highlights. The ground is lit with its smooth normal: the corner grid's normals (central differences of the heights) spread bilinearly, baked into the detail map (see *After the rain*), so a gently tilted tile no longer shows as a square of its own shade with hard edges, as the meshes' flat faces do in the classic light (walls and cliffs keep their own normals). |
| Ambient occlusion (`occlusion`) | on | Baked per vertex of the tile meshes' 3 × 3 grid from the height field: how much of the horizon the point sees in eight directions, plus how far it sits below the ground around it (valley floors darker). Spread to the light map bilinearly and applied to the ambient and direct light. |
| Soft terrain shadows (`shadows`) | on | A height-field shadow pass: from every light-map texel a ray marches towards the decoded light (in world space: the eye-space light turned back by the view, 25° above the horizon) over the heights, keeping the smallest clearance over distance, which gives a penumbra that widens with the distance to the caster. Shadowed ground keeps 38% of its direct light. The game's hills are gentle, so the shadows are mostly a soft darkening behind ridges and below raised ground. |
| Soft sprite shadows (`sprite-shadows`) | on | The art's own shadow frames (`*Shadow.flc`, drawn under buildings, trees and people) are blurred by a small Gaussian so their edges fade into the ground. No shadow is invented where the art has none. Off in HD (the pack's frames are used as they are). |
| Sky (`sky`) | on | Instead of the classic black beyond the property and below its cliff, a gradient from the theme's sky through a haze at mid screen to the haze below the map. Classic keeps its black. |
| Tilt-shift (`tilt-shift`) | off | Depth of field by screen height: the course is sharp in a band through the middle of the screen and blurs towards the top and bottom, a "miniature" look. |
| Colour grading (`grading`) | on | Gentle contrast and vibrance (saturates the dull colours more than the vivid ones) and a slight tint, per theme: neutral parkland, warm desert, clear tropical, cool links. |
| Bloom (`bloom`) | on | The brightest parts (sparkle, white sand, highlights) glow a little: a bright pass, blurred at a quarter (high quality: half) of the resolution, added back. |
| Vignette (`vignette`) | on | Slightly darker screen corners. |
| Cast shadows (`cast-shadows`) | on | Sprites cast their own outline along the sun onto the ground (see *Cast shadows* below). |
| Time of day (`time-of-day`) | off | Morning, midday, golden hour, sunset and a short blue dusk, one day of light per game month (see *Time of day and weather*). |
| Cloud shadows (`clouds`) | on | Soft cloud shadows drift over the course and dim the sprites under them. |
| Distance haze (`haze`) | on | The far (upper) part of the view pales towards the sky's horizon colour, more when zoomed out. |
| Rain (`rain`) | off | Light rain on some days (the same days for a save), with greyer, dimmer light and faint shadows. |
| Mist (`mist`) | off | Mist pooling in hollows, low ground and over water: thick at dawn and gone by late morning with the time of day, a light veil all day without it. |
| Ball trail (`ball-trail`) | on | A soft trail fading behind a ball in the air. |
| Ball impacts (`ball-impacts`) | on | Where a ball comes down: a puff of sand in a bunker, rings and drops on water, a flick of grass elsewhere. |
| Day length (`day-length`) | normal | How long a day of light lasts: short (one game month, the first versions' pace), normal (three) or long (six); see *Time of day and weather*. |
| Water reflections (`reflections`) | on | Trees, buildings and golfers mirrored in the water, rippling; stronger sun glitter at golden hour; the water lapping at the shore (see *Water reflections*). |
| Wind (`wind`) | on | A wind for each day: trees, palms, bushes and grasses sway, flags fly downwind, the rain and the clouds follow it (see *Wind*). |
| Evening lights (`evening-lights`) | on | With the time of day: lit windows, lamps with pools of light and fireflies at dusk (see *Evening lights*). |
| Seasons (`seasons`) | on | Blossom and fresh greens in spring, autumn colours in September and October, frost and dew on spring and autumn mornings (see *Seasons*). |
| After the rain (`after-rain`) | on | With rain: wet, glossy ground and puddles during and after a rainy day, now and then a rainbow (see *After the rain*). |
| Lightning (`lightning`) | off | Flashes and distant flicker in heavy rain on parkland and links courses. |
| Quality (`quality`) | medium | Low: light map 2 texels a tile, no blur passes (no tilt-shift, no bloom), sharp-bilinear scaling only, the cast shadows' mask at half the resolution. Medium: 4 texels a tile, blurs at a quarter of the resolution, the mask at half. High: 8 texels a tile, blurs at half the resolution, the mask at full. On the Effects page since this version. |

The post-processing (tilt-shift, grading, bloom, vignette) draws the course into an offscreen target, then composites it
into the frame; the interface (dock, panels, text, screens over the course) is drawn after that and is not affected.
With HD or Enhanced's anti-aliasing on, that offscreen course keeps the 4× multisampling. The scaler also handles the
premultiplied textures of Enhanced's *smooth interface art* (it blends them premultiplied).

### Turning it on

The effects are part of Enhanced graphics: with the master switch on, System Functions → Preferences → *Enhanced
graphics...* has a tick for each (stored in `settings.json` under `"enhanced"` as `fx_scaler`, `fx_scaler_mode` (the
filter: 0 sharp bilinear, 1 edge-directed, 2 smooth; a line that steps through them; an older file's `fx_scaler_sharp`
still picks sharp bilinear until the filter is chosen again),
`fx_water`, `fx_terrain_light`, `fx_sprite_shadows`, `fx_sky`, `fx_tilt_shift`, `fx_grading`, `fx_bloom`, `fx_quality`
(0 to 2), `fx_cast_shadows`, `fx_ball_trail`, `fx_ball_impacts`, `fx_clouds`, `fx_haze`, `fx_time_of_day`, `fx_rain`,
`fx_mist`, `fx_reflections`, `fx_wind`, `fx_evening_lights`, `fx_seasons`, `fx_after_rain`, `fx_lightning`,
`fx_day_length` (0 to 2, default 1)), all on but tilt-shift, lightning, and the three that change the mood most: time of
day, rain and mist; the scaler's filter is *Smooth (strongest)*. The ticks map onto `fx::FxSettings` (`fx::sync`): *terrain light*
stands for per-pixel light, occlusion and shadows, *colour grading* for grading and vignette.

Finer control and tests, for one run:

- `--fx` turns the effects on (alone, without the rest of Enhanced) with the defaults above; `--fx-set LIST` also sets
  options, a comma separated list of `key=value` (value: `on` / `off`, a choice's number, or `sharp`, `edge`, `smooth`, `low`,
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
- `render.rs`: `draw_props` draws the cast shadows after the ground overlays, tints a sprite (`fx_sprite_tint`, None unless
  the time of day, rain or clouds are on) and leaves out the art's shadow frames while the cast shadows replace them;
  `render_world` draws the haze and rain over the course; `draw_balls` draws the trails before the balls and the impacts
  after them. The view a prop's sprite is drawn in moved into `App::prop_view` (the same code, shared with the shadows).
- `app.rs`: `step_game` lets `fx_ball_tick` read each ball after a simulation tick (it writes nothing back).
- `main.rs`: `fx_world_begin` is told the frame's pass (the cast shadows draw their mask offscreen and come back to it).

The time of day, weather and ball effects were checked the same way: Classic scenes and Enhanced scenes with the new
options off, rendered with the build before them and with them, compared byte for byte (15 scenes: four themes, 800 × 600
to 3840 × 2160, a rotated view, a panel, a played save at two moments, the menu, a report, five Enhanced scenes): all
identical.

The reflections, wind, evening lights, seasons, after-the-rain, lightning and day length options, and the ground's smooth
normals, were checked the same way: 17 scenes (the ten Classic scenes above, five Enhanced scenes, one at dusk with the
time of day and mist, one in rain) rendered with the build before them and with them, the new options off (the day length
short) and `SG_FX_FACETED=1`: all identical. Their hooks: `render.rs` `draw_props` asks `fx_sprite_extras` (the season's
texture, the wind's bend, the windows' glow) for each body, draws the light pools after the cast shadows and the lamps in
the sprites' order; `fx_light.rs` `fx_overlay` draws the rainbow, fireflies and lightning; `fx.rs` `fx_draw_ground` draws
the reflections' target first and hands the water and terrain shaders the detail map, the reflections and three uniform
vectors that are 0 (their code skipped) while the options are off.

Checked by rendering the same scenes with the branch before the effects and with them off (four themes at 3840 × 2160,
1920 × 1080, 1280 × 1024 and 800 × 600, a rotated view, a panel open, a played save, the title menu and a report) and
comparing the PNGs: all identical.

### Browser and old GPUs

All shaders are GLSL ES 1.00 (WebGL 1 / GLES 2): no derivatives, no dynamic loops, highp in fragment shaders only where
`GL_FRAGMENT_PRECISION_HIGH` is defined, uniforms shared by both stages declared with the same precision. The
post-processing needs a depth buffer in an offscreen target, which the browser build does not get, so in the browser it
is skipped (tilt-shift, grading, bloom, vignette); the scaler, water, light, shadows and sky still run there, and so do
the cast shadows (their mask is a colour target without depth), the time of day, clouds (a 128 × 128 repeating noise:
a power of two, as WebGL 1 wants for repeating), haze, rain, mist and the ball effects, and the newer reflections (a colour
target without depth, sampled with `gl_FragCoord`), wind, evening lights, seasons, wet ground and lightning (the terrain
shader stays within WebGL 1's sixteen fragment uniform vectors and five textures). The cloud drift is kept within one
period of the noise so the shader's coordinates stay small where fragment shaders only have mediump. The light
map is built on the CPU (a few milliseconds; threaded on desktop) when the ground's heights, water or the view's turn
change, not every frame.

### The Smooth scaler

`crates/simgolf/src/fx_smooth.rs`, the scaler's third filter and the default. The game's art is pre-rendered and already
anti-aliased: lines and rings are drawn with in-between colours and the sprites are softly shaded, so the edge-directed
filter's rules, which look for runs of equal colours, rarely fire inside the art, and the stair steps of its rings, arrows
and diagonals stay. The Smooth filter measures the direction of the art's edges instead and interpolates along them
(steering kernel regression):

1. the 4 × 4 texels around the pixel are read once (16 fetches, premultiplied, the colour key taken as transparent);
2. the brightness gradients of the nine 2 × 2 blocks among them, weighted by a Gaussian around the pixel, give a
   structure tensor: its main axis is the edge's normal, `(l1 - l2) / (l1 + l2)` how clearly there is one edge (corners,
   dots and the noise of foliage or roof tiles have none) and `l1` how strong it is;
3. the texels are weighted by a Gaussian 1.2 texels long along the edge and 0.35 across it, so a stair-stepped circle, a
   shallow curve or a one-texel line becomes a smooth line, with no preferred angle (xBR knows 45° and 30°);
4. the same kernel half a texel to either side of the pixel gives the edge's two colours, and the pixel's place between
   them is steepened to a transition about one screen pixel wide: crisp without overshoot or halos. Not where the three
   are out of order (the middle of a thin line), lie on a straight ramp (soft shading), or differ little (the steps of
   the art's quantised gradients, which would turn into contour bands);
5. the result is mixed with the softened sharp-bilinear base (the edge-directed filter's) by the edge measure of 2, so
   flat areas, shading, textures and single-texel details stay as the art drew them instead of turning plastic.

Transparent edges: colour-keyed pictures keep the outline of the smooth interface art (edges.rs, the distance field,
one screen pixel wide) and take their colour from this filter, unpremultiplied, so no key colour or dark fringe shows;
premultiplied pictures (portraits, shadows) and straight ones get their alpha from the filter itself. It works at any
magnification from 1.2 (below that, and on quality low, the sharp filter draws), since it interpolates rather than
deciding per texel corner. GLSL ES 1.00 as the others: no texelFetch, no loops, the exponentials clamped so mediump
cannot underflow. The constants (kernel widths, thresholds) are at the top of the file.

Checked on the dock, the rating icons, the course badge, the Course Report, Tracts for Sale, the course sprites
(clubhouses, trees, cacti, the flag, golfers) at 1920 × 1080 and 3840 × 2160 against the edge-directed filter, crops
magnified 2 to 5 times. Known limits: on the dark rims of the rating icons, whose art has several close shades, the
filter leaves faint streaks along the rim at 5× and more; foliage and roof textures stay as crisp as sharp bilinear
(the filter does not invent smoothness where the art has no edge). Classic (1920 × 1080, and a report at 3840 × 2160) and
Enhanced with the edge-directed (3840 × 2160) or sharp (1920 × 1080) filter were rendered with the build before this
filter and with it and compared byte for byte: identical. The browser build (headless Chromium, SwiftShader WebGL) runs
it without errors.

### Cast shadows

Every standing sprite (trees, buildings, flags, people, animals; not ground overlays such as building bases and flower
beds, not the water's own sprites) can cast its outline onto the ground. The outline is the sprite's frame read as a
shape: padded, and blurred more the higher a row is above the sprite's ground point (a penumbra that widens away from the
base), cached per frame. It is drawn as two quads lying on the ground: the part of the picture above the ground point is
sheared along the shadow's direction (a point *c* up the picture stands *c* / (the eye's up axis' height) above the ground,
and its shadow lies that height times the shadow's length away), the part below it (a building's front step, a tree's
roots) stays where it is on the ground. A flier's shadow is carried away by its height. The outlines go into a mask (a
render target at half the screen's size, white, darkened under each outline, so overlapping shadows do not darken the
ground twice), which a full-screen pass multiplies into the frame with a cool shade colour, after the ground and its
overlays and before any sprite, and only where ground was drawn (its quad sits at the far end of the depth range with
the test "greater", so the sky and the black beyond the map are left alone).

**Why the art's shadows stay.** The art has pre-rendered shadow frames (`*Shadow.flc`) for nearly every tree, building and
person, rendered with the sprites in 3D: they are the right shape for the object's depth, which a 2D outline is not. So
with the sun where the art was lit (the time of day off) the art's shadows are kept, softened by *Soft sprite shadows*,
and only sprites without one cast an outline (pins and flags, tee markers, some decorations and props, the birds). The
cast shadows follow the art's shadows: measured on the tree and building frames, those fall to the right on the screen
and a little up it, about two thirds of the height of what casts them, and the outlines are laid the same way, so the two
kinds look alike side by side. With the time of day on the sun moves and the art's shadows would point the wrong way:
then the art's shadow frames are not drawn and every sprite casts its outline, turning and stretching with the sun (up to
2.4 times the height near sunset) and fading at dusk.

### Time of day and weather

`crates/simgolf/src/fx_light.rs`. A day of light lasts three game months (3 × 1024 ticks, about four and a half minutes
at normal speed) with the *Day length* at normal; short makes it one month (the first versions' pace, about 90 seconds)
and long six. It is read from the game's calendar tick (and the tick's fraction, so it moves smoothly); a new game opens in the morning. Key
moments: dawn, morning, a long midday that is exactly the classic decoded light, a golden afternoon, sunset and a short
blue dusk, blended with a smooth step. Each moment turns the sun round the vertical from the decoded light's direction
and sets its height, multiplies the decoded light's diffuse, specular and ambient colours, tints the pre-lit sprites, and
moves the sky's colours. The sun swings round the front of the view (from the left in the morning to the right at
sunset), so shadows lengthen sideways and away from the viewer. A low sun's light is raised so flat ground keeps at least
three quarters of its midday brightness; dusk keeps a cool light from above with no shadows. The terrain's light map is
marched towards the moving sun again when it has turned 3° or risen 2° (a few milliseconds, threaded). The water reflects
the moment's sky, the post-processing's backdrop takes the moment's haze.

Rain falls on about one day in four on parkland (35% on the links; never on desert or tropical courses, which stay sunny). Which days is a hash of
the course's name and size and the day: render-side only, the same days whenever the save is loaded, and the game's own
random generator is never used. With the normal or long day length the rain, the cloud cover and the wind blend with the
neighbouring day's over the first and last eighth of each day (half way at the boundary), so a spell of rain comes and
goes gently, and the clouds' drift is carried on from frame to frame (`FxWorld::cloud_acc`) so a change of wind turns
them instead of moving them; the short day length keeps the first versions' changes at the day's boundary. A wet day
dims the sun by half, greys the sky and the colours, fades the shadows, thickens
the clouds and the haze; the drops are thin slanted streaks drawn over the course (about 700 at 1920 × 1080, scaled with
the window), each landing in a small ring.

Clouds are a 128 × 128 repeating fractal noise (the same every run), laid over the course every 26 tiles and drifting at
about 0.6 tiles a second in a direction and cover that change from day to day. In the ground's shader they dim the sun's
part of the light; a sprite under one is dimmed on the CPU from the same noise (and its art shadow faded), so the two
agree. Mist mixes the ground towards a pale sky colour by how low it lies (below the lowest quarter of the course's
heights), how hollow it is (the ambient occlusion) and whether it is water, broken up by the drifting noise. Haze is a
gradient over the upper part of the view in the sky's horizon colour, stronger the more of the course the view takes in.

### Water reflections

`crates/simgolf/src/fx_reflect.rs`. Before the ground is drawn, every standing sprite within four tiles of water (not the
water's own sprites) is drawn mirrored into a render target (half the frame's size; a third at low quality, full at high):
an upright object's mirror image is its own picture flipped on the screen about the reflection of its ground point in
the water's plane (the nearest water's height), so each sprite is drawn flipped there, bending with the wind as the
sprite does, a little darker and fading towards its top. The water shader samples that target at its own pixel moved by
the waves' slope, so the reflections ripple and only show on the water itself (not on the bank art, which the water
shader already tells apart). Objects in front of the water mirror onto the land in front of them, as in life, so the
reflections show where the trees and buildings stand behind the water from the camera. The same option makes the glints
up to 2.8 times stronger as the sun gets low and warm (golden hour) and draws a thin line of water lapping in and out at
the shore, with the wet sheen behind it. The pixel-art scaler is held off for the reflections' pass (`fx::pause_scaler`).

### Wind

`crates/simgolf/src/fx_nature.rs`. Each day has a wind: its direction and strength (calm to a strong breeze, windier when
it rains) from the course's hash and the day. Gusts run across the course along it (value noise in time and in distance
along the wind). Sprites are told apart by their files (`kind_of`: deciduous and flowering trees, conifers, palms,
cacti, bushes, grasses and reeds, flags, buildings). Trees, palms, bushes and grasses lean downwind and sway (each with
its own frequency and phase, palms and grasses more, conifers less, cacti and buildings not at all): the sprite is drawn
as five strips whose corners move along the screen's right by the bend at their height (height^curve), so the ground
point stays and the top moves most (`strips`). Flags fly downwind: the art's flag points right of its pole in every
view, so it is mirrored about the pole when the wind blows to the screen's left. The rain's streaks slant with the wind's
part across the screen and the clouds drift along it, faster in a stronger wind. Only the drawn corners move (a few
pixels): the drawing order, the hit tests and the simulation are untouched.

### Evening lights

`crates/simgolf/src/fx_night.rs`, with the time of day only. From sunset (phase 0.84 to 0.935) to just after dawn:

- **Windows.** A building frame's windows are found in its art: dark, neutral or bluish pixels (glass) with light, low
  saturated pixels (frames, walls) within three pixels on both sides or round a corner, gathered into small compact
  patches (at most 80 pixels and 16 across, at least 40% of their box filled, so roof edges and dark trim are not
  taken), plus turquoise stained glass. About four windows in five are lit (a hash of the window), each a slightly
  different warm white, with a soft orange halo; the glow is cached per frame and drawn over the building (the bloom
  picks it up on the desktop).
- **Lamps.** A slim post with a glowing head and a halo beside the two outer corners (on the screen) of each building and
  every few tiles along the paths (a hash keeps them at least two tiles apart), drawn in the sprites' order; a pool of
  warm light on the ground under each, and a wider glow round each building.
- **Fireflies.** On parkland and tropical courses, at dusk and more in summer (with the seasons), a few on some tiles of
  rough, brush, woods, marsh and within reach of water: green-gold lights drifting slowly a little above the ground,
  glowing up and fading.

### Seasons

`crates/simgolf/src/fx_nature.rs` and the terrain shader. The game's eight months, March to October, read from the
calendar tick (`SG_FX_MONTH` for stills). Spring (March, April, fading through May): deciduous trees freshen (their
greens a little yellower and brighter) and blossom (pink or white flecks in small clusters, shaded by the leaf they sit
on; flowering trees such as dogwood, peach and plum are full of it); conifers freshen half as much; the grass is a
fresher green. Summer: the art as it is. Autumn (from mid September, full in October): deciduous leaves turn a few at a
time (each pixel by its own hash) to red, orange or gold, darker leaves redder, the hue range differing between kinds of
tree; the grass turns a little golden. Palms and cacti do not change. The trees are recoloured copies of their frames
(foliage picked by hue, 48° to 178°), made when the season moves on by an eighth and cached; the grass is tinted in the
terrain shader (green pixels of grassy ground only). Per theme: no autumn or blossom in the tropics, little in the
desert, a little less on the links. With the time of day, early mornings in March, April and October have frost lying
in patches (more in hollows) and, in spring and autumn, dew or rime glinting in the sun (tiny twinkling sparkles, on
level ground only).

### After the rain

`crates/simgolf/src/fx_wet.rs` and the terrain shader, with rain on. During a rainy day the ground gets wet (darker by up
to a quarter, a little of the sky's colour in it) and puddles fill; through the next day it dries and the puddles shrink
away. Where puddles gather is the **detail map** (made with the light map, 8 texels a tile, 4 at low quality): the
potential of each spot, high on level fairway and rough (and tees and greens less), more in hollows, broken into blobs by
a noise so only its peaks fill; the shader's finer noise rounds their outlines. A puddle is dark water mirroring the sky
with the sun's glint; while it rains, rings spread in it. On about half the days after rain, early in the day and with
the sun out, a rainbow stands in the sky: a faint arc over the upper screen, opposite the sun, with a fainter second bow
outside it, fading towards the ground at both ends; it is at infinity, so it stays put as the view pans. **Lightning**
(its own tick, off by default): in heavy rain on parkland and links courses a near strike now and then (a bright flash
and two after-flickers, the sky end brighter) and fainter, more frequent flicker in the distance, timed by a hash of the
effects' clock.

### Ball trail and impacts

`crates/simgolf/src/fx_ball.rs`. After each simulation tick the balls are read (nothing is written back): a ball in the
air adds its place to its trail (the last 8 ticks are kept), and a ball whose height was above 0 at the last tick and is 0
now has come down. The trail is a ribbon through those places up to the ball as drawn, as wide as the ball and fading
with age. An impact is drawn by what the tile is: sand traps, waste and pot bunkers throw a pale dust cloud and darker
grains; water spreads two rings and throws drops (next to the original's own splash sprite); anything else flicks a few
blades of grass. A bounce makes a smaller one. What varies between impacts comes from a hash of the impact.

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

The Smooth filter (16 texture fetches and some 60 exponentials a magnified pixel, most of them only where there is an
edge) against the others, on the played desert save with the dock open, the scaler alone (every other effect off) and
with the default effects, the best of three alternating runs of 6 frames:

| software renderer, ms a frame | 1920 × 1080 | 3840 × 2160 |
|---|---|---|
| Classic | 31 | 59 |
| effects on, every option off | 37 | 96 |
| scaler, sharp bilinear | 38 | 117 |
| scaler, edge-directed | 134 | 419 |
| scaler, smooth | 131 | 366 |
| default effects with the edge-directed filter | 217 | 701 |
| default effects with the smooth filter (the default) | 206 | 637 |

So Smooth costs no more than the edge-directed filter it replaces as the default (the xBR rules fetch 22 texels a pixel).

The cast shadows, time of day, weather and ball effects were measured the same way, each against the effects without
them (the scaler off, so its cost does not drown theirs), in alternating runs because the shared machine's load moved the
times by up to 2× between runs: a played desert course at 1920 × 1080, 20 frames a run, the best of four runs each.

| 1920 × 1080, software renderer | without | with | cost |
|---|---|---|---|
| cloud shadows (one texture fetch a ground pixel) | 122 | 142 | +19 ms |
| distance haze (a blended gradient over the upper screen) | 143 | 154 | +11 ms |
| mist (one texture fetch a ground pixel) | 67 | 92 | +25 ms |
| rain (about 700 streaks) | 98 | 110 | +12 ms |
| time of day (uniforms; the light map again every 3° of sun) | 110 | 104 | within the noise |
| cast shadows, sprites without art shadows | 167 | 159 | within the noise |
| cast shadows from every sprite (with the time of day) | 185 | 187 | +2 ms |
| ball trail and impacts | 103 | 111 | +9 ms (only while balls fly) |
| all of the new options at once | 180 | 222 | +42 ms (+23%) |

As with the other effects the software renderer pays for every texture fetch; on a GPU the two extra fetches a ground
pixel (clouds, mist), the half-size shadow mask (one pass of outline quads and one full-screen multiply) and a few
thousand rain and particle vertices are far below a millisecond. If a GPU struggles, mist and clouds go first.

The reflections, wind, evening lights, seasons and wet ground were measured the same way (a parkland course turned so
trees and a hotel stand behind the water, at dusk in October with wet ground and a breeze, the scaler off), but the
shared machine was loaded far more heavily this time (load 8 to 9 on 4 cores) and single runs varied by up to 3×, so only
the totals mean much: at 1920 × 1080 the medians of four alternating runs were 625 ms a frame without the five options
and 783 ms with all of them (about +25%); each option alone was within the noise. At 3840 × 2160 two runs each gave 868 /
2136 ms without and 1873 / 2644 ms with. What they cost on the GPU: the reflections draw the sprites near water a second
time into a half-size target (a third at low quality) and the water shader one more fetch; the seasons and the wet
ground one detail-map fetch and a few noise evaluations a ground pixel; the wind bends sprites into five strips (more
vertices, no more pixels); the evening lights a cached glow quad per building and a few hundred small quads for lamps
and fireflies. The recoloured trees are made on the CPU when the season moves on by an eighth (a few milliseconds, once).
If a GPU struggles: quality low, then reflections off.

### Limitations

- The reflections mirror the sprites, not the terrain (a hill behind a lake is not mirrored), and a reflection is the
  sprite flipped, not the far side of the object.
- The windows are found by colour in the art, so a few dark patches that are not windows glow and a few windows (light
  grey panes) stay dark; HD packs are not searched (the windows glow only in the classic art), nor recoloured for the
  seasons.
- The wind bends the bodies, not the art's shadow frames or the cast shadows; a flag's shadow does not turn with it.

- The light map follows the heights and water, not the sand bunkers' dished centres (the mesh's own normals still light them).
- The cast shadows are a sprite's outline laid flat from its ground point: a building's depth is not modelled (the part of
  it behind the ground point casts as if it stood on it), and on a slope the shadow lies in the plane of its ground point.
- Mist and haze veil the ground; the sprites standing in mist are not veiled (they are tinted for light, rain and clouds
  only).
- Under the time of day the water and sprites keep their midday art: the light tints them, it does not relight them.
- With very gentle hills the decoded light (25° up) casts little shadow; the occlusion is what shows the relief.
- Tilt-shift blurs by screen height only (the camera is orthographic: there is no real depth to focus on).

## Photo mode

`crates/simgolf/src/photo.rs`. A way to take pictures of the course, for wallpapers and sharing. It opens only with Enhanced
graphics on (its effects included) and the Display page's *Photo mode* tick, on the course with nothing over it:

- **C** (a key the original's key handler leaves unused; Shift+C stays *Save course for Championship*), or
- the **camera button** under the rating pills at the top right (Enhanced only; it shows *Photo mode (C)* when the pointer is
  on it). A tap works the same on a phone or tablet.

The interface goes away (HUD, dock, panels, ticker, names and thoughts, tooltips, the tool's preview and the aim line) and
the course is shown alone. The game **pauses** on opening, so the golfers hold their swing; Space (or the *Resume* button)
lets it run on while the photo is framed. The mouse pointer hides after 2.5 seconds without input.

| Input | What it does |
|---|---|
| drag, arrows | move the camera (with the smooth camera's glide, whatever the *Smooth camera* tick) |
| wheel, pinch, Z / X, + / - | zoom smoothly from half the original's farthest level to four times its closest |
| PageUp / Home, a two-finger twist | turn the view a quarter |
| click or tap | put the focus there (the focus blur is sharp round it) |
| F | the focus blur on (strength 3) or off; [ and ] weaken and strengthen it (1 to 5) |
| L (Shift+L back) | the look: Natural (the theme's own grade), Warm, Vivid, B&W, Vintage |
| V | the vignette on or off |
| R | the photo's size: the window's, twice the window's, 4K (3840 × 2160) |
| Enter | take the photo |
| Space, P | pause or resume the game |
| H | hide or show the hint and the buttons |
| Esc, C, the Android back button | leave photo mode |

A hint at the top lists the keys, and a row of buttons along the bottom does the same for the mouse and touch (Leave,
Pause / Resume, Look, Blur, Vignette, Size, Take photo); both fade out after a few seconds and come back with any input. None
of them is in a photo.

**Leaving** gives everything back as it was: the camera's place, zoom and turn (an animated turn too), the pause, the
followed golfer. Photo mode closes by itself when the game needs the screen (a report, a question, a great shot's photo).

**The picture.** Photo mode always draws through the effects' post-processing (fx.rs `fx_world_begin` / `fx_world_end`),
whatever the post-processing ticks, with its own composite: the focus blur (the course blurred at a quarter of its size, one
to three more passes for a stronger blur, mixed in by the distance to the focus point, the sharp area flattened into a wide
band as a tilted camera over a miniature sees it), the user's bloom, the look (contrast, vibrance, saturation, lifted
blacks, a tint and, for Vintage, a sepia tone) and the vignette. The zoom goes past the original's levels, so the sprites are
magnified more: the pixel-art scaler (when on) keeps them sharp.

**Taking a photo** draws the course alone once more into an offscreen target of the photo's size (anti-aliased with
Enhanced's multisampling up to 4K), with the scaler told the target's size and the blur spread by the target's size over
the window's, so a larger photo looks like the screen, only sharper. A size the graphics driver cannot draw is scaled down
to its largest texture (at most 8192 pixels a side). The shutter sounds (the SimFoto's), the screen flashes, and a line says
where the photo went. Photos are PNG files named `OpenSGolf <course> <date> <time>.png` (UTC):

- desktop: `Photos` in the save folder (next to `settings.json` and `Saved Games`);
- browser: downloaded (the page hands every PNG the game writes to the browser as a download, as with Shift+F6);
- Android: `Android/data/org.opensgolf.game/files/Photos` (the app's external files folder, next to `game/`, reachable from
  a computer over USB; the phone's gallery is not told about it).

In the browser the post-processing needs a depth buffer in an offscreen target, which WebGL 2 refuses with miniquad's
unsized depth format; `web/opensgolf.js` gives such a texture the sized format of its type (16 bits), so photo mode's
looks, blur and photos work in the browser too. The rest of the browser build keeps skipping the post-processing as before.

**Classic.** Nothing of photo mode exists with Enhanced off: the C key, the button and every hook (`App::photo`, the zoom's
range, the touch buttons, render.rs's tool preview, fx.rs's composite) ask whether photo mode is open, which it cannot be.
Checked: Classic scenes (800 × 600 to 3840 × 2160, a panel, a report, the menu, Preferences, a new game) render byte for
byte as before; Enhanced scenes differ only by the camera button.

**Test hook:** `--edit "P:LOOK,BLUR,VIGNETTE,ZOOM,HINT,SHOOT"` opens photo mode for stills: LOOK 0 to 4, BLUR 0 (off) to
5, VIGNETTE 0 or 1, ZOOM in percent of the closest level (0 keeps the zoom), HINT 1 shows the hint and buttons, SHOOT 1 to
3 takes a photo at the window's size, twice it or 4K after the first frame (written to `Photos` in the save folder).
`SG_CLICK=x,y` puts the focus there first. For example
`--size 1920x1080 --enhanced --edit "P:1,3,1,0,0,3"` with `SG_CLICK=1500,560` saves a warm 4K photo focused there.

**Note for developers.** miniquad keeps a cache of the texture bound to each unit and does not drop a deleted texture from
it, so a texture made later under the same GL name may not be bound again. The end of a frame (`commit_frame`) clears the
cache; the photo's offscreen drawing makes and deletes targets in the middle of a frame (the post-processing at the photo's
size), so it clears the cache before and after, and so does fx.rs `free_post` when the post-processing's targets are
made again for a new size (without that the next frame's course came out black or banded).

## For developers: adding an option

The other Enhanced options (shaders and effects) add themselves the same way:

1. a field in `enhanced::Settings` (with `#[serde(default)]` old settings files still read) and its default in
   `Settings::DEFAULT`;
2. an `OptionEntry` in `enhanced::OPTIONS` (settings key, a text key in `lang/en.txt` whose text starts with a space, the
   page it is on (`DISPLAY`, `EFFECTS`, `ATMOSPHERE`), its choices' text keys for a setting with more than two values (the
   label then has a `{VALUE}`; empty for a tick), getter, setter, and `Reload::Art` when the option changes how textures
   are made; the `tick!` macro writes the common case);
3. a switch like `pub fn smooth_ui() -> bool { on(|s| s.smooth_ui) }`, asked before the new code runs. It is false whenever
   the master switch is off, which is what keeps Classic byte-identical.

The Preferences screen lists a page's options in `OPTIONS` order under the master switch (bit 0 the master switch, bit 1 +
i the page's option i, at most 31 a page); the box is sized for the longest page and the widest line of all. Interface
code that should sit at a window edge calls `Screen::anchor(Anchor::...)` before drawing; nothing happens in Classic or
with the widescreen option off.
