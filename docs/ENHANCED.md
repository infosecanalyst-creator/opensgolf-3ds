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

- **In the game:** System Functions → Preferences → the *Enhanced graphics* tick switches it on and off; *Enhanced graphics settings...* opens a checkbox list in three pages, picked with
  the tabs along its top (click or tap a tab; Left, Right or Tab on the keyboard): **Display** (the options below),
  **Effects** (the shader effects, cast shadows, the balls' trail and impacts, and the effects' quality) and **Atmosphere**
  (sky, time of day, clouds, haze, rain, mist, tilt-shift, grading and bloom). The master switch *Enhanced graphics (off:
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
  Atmosphere).

## The options

| Option (settings key) | What it does |
|---|---|
| Sharp text at the screen's resolution (`native_text`) | Text is rasterised from the TrueType faces (Klepto, Manual SSi, Arial or its stand-in) at the size it really has on the screen (the size asked for times the window's scale), cached per face and pixel size, and drawn texel for pixel on whole pixels. The pen still advances by the classic metrics in 800×600 units, so line breaks, centring and every layout are unchanged. Classic bakes the faces once at 32 pixels and scales them. |
| Smooth interface art at any size (`smooth_ui`) | Interface pictures, sprites, portraits and HD pack pictures are uploaded with premultiplied alpha and mipmaps (trilinear). Scaled up or down, their edges blend with what is under them without the dark or magenta fringes that straight alpha leaves; drawn smaller (a phone, a small window, the far zoom) they do not shimmer. A texture made this way is drawn with a premultiplied blend state; every other texture keeps the classic one, so the two can be mixed safely. |
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
| Cast shadows (`cast-shadows`) | on | Sprites cast their own outline along the sun onto the ground (see *Cast shadows* below). |
| Time of day (`time-of-day`) | off | Morning, midday, golden hour, sunset and a short blue dusk, one day of light per game month (see *Time of day and weather*). |
| Cloud shadows (`clouds`) | on | Soft cloud shadows drift over the course and dim the sprites under them. |
| Distance haze (`haze`) | on | The far (upper) part of the view pales towards the sky's horizon colour, more when zoomed out. |
| Rain (`rain`) | off | Light rain on some days (the same days for a save), with greyer, dimmer light and faint shadows. |
| Mist (`mist`) | off | Mist pooling in hollows, low ground and over water: thick at dawn and gone by late morning with the time of day, a light veil all day without it. |
| Ball trail (`ball-trail`) | on | A soft trail fading behind a ball in the air. |
| Ball impacts (`ball-impacts`) | on | Where a ball comes down: a puff of sand in a bunker, rings and drops on water, a flick of grass elsewhere. |
| Quality (`quality`) | medium | Low: light map 2 texels a tile, no blur passes (no tilt-shift, no bloom), sharp-bilinear scaling only, the cast shadows' mask at half the resolution. Medium: 4 texels a tile, blurs at a quarter of the resolution, the mask at half. High: 8 texels a tile, blurs at half the resolution, the mask at full. On the Effects page since this version. |

The post-processing (tilt-shift, grading, bloom, vignette) draws the course into an offscreen target, then composites it
into the frame; the interface (dock, panels, text, screens over the course) is drawn after that and is not affected.
With HD or Enhanced's anti-aliasing on, that offscreen course keeps the 4× multisampling. The scaler also handles the
premultiplied textures of Enhanced's *smooth interface art* (it blends them premultiplied).

### Turning it on

The effects are part of Enhanced graphics: with the master switch on, System Functions → Preferences → *Enhanced
graphics...* has a tick for each (stored in `settings.json` under `"enhanced"` as `fx_scaler`, `fx_scaler_sharp`,
`fx_water`, `fx_terrain_light`, `fx_sprite_shadows`, `fx_sky`, `fx_tilt_shift`, `fx_grading`, `fx_bloom`, `fx_quality`
(0 to 2), `fx_cast_shadows`, `fx_ball_trail`, `fx_ball_impacts`, `fx_clouds`, `fx_haze`, `fx_time_of_day`, `fx_rain`,
`fx_mist`), all on but tilt-shift, *sharp bilinear instead of edge-directed*, and the three that change the mood most:
time of day, rain and mist. The ticks map onto `fx::FxSettings` (`fx::sync`): *terrain light*
stands for per-pixel light, occlusion and shadows, *colour grading* for grading and vignette.

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

Checked by rendering the same scenes with the branch before the effects and with them off (four themes at 3840 × 2160,
1920 × 1080, 1280 × 1024 and 800 × 600, a rotated view, a panel open, a played save, the title menu and a report) and
comparing the PNGs: all identical.

### Browser and old GPUs

All shaders are GLSL ES 1.00 (WebGL 1 / GLES 2): no derivatives, no dynamic loops, highp in fragment shaders only where
`GL_FRAGMENT_PRECISION_HIGH` is defined, uniforms shared by both stages declared with the same precision. The
post-processing needs a depth buffer in an offscreen target, which the browser build does not get, so in the browser it
is skipped (tilt-shift, grading, bloom, vignette); the scaler, water, light, shadows and sky still run there, and so do
the cast shadows (their mask is a colour target without depth), the time of day, clouds (a 128 × 128 repeating noise:
a power of two, as WebGL 1 wants for repeating), haze, rain, mist and the ball effects. The cloud drift is kept within one
period of the noise so the shader's coordinates stay small where fragment shaders only have mediump. The light
map is built on the CPU (a few milliseconds; threaded on desktop) when the ground's heights, water or the view's turn
change, not every frame.

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

`crates/simgolf/src/fx_light.rs`. One game month (1024 ticks, about 90 seconds at normal speed) is one day of light,
read from the game's calendar tick (and the tick's fraction, so it moves smoothly); a new game opens in the morning. Key
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
random generator is never used. A wet day dims the sun by half, greys the sky and the colours, fades the shadows, thickens
the clouds and the haze; the drops are thin slanted streaks drawn over the course (about 700 at 1920 × 1080, scaled with
the window), each landing in a small ring.

Clouds are a 128 × 128 repeating fractal noise (the same every run), laid over the course every 26 tiles and drifting at
about 0.6 tiles a second in a direction and cover that change from day to day. In the ground's shader they dim the sun's
part of the light; a sprite under one is dimmed on the CPU from the same noise (and its art shadow faded), so the two
agree. Mist mixes the ground towards a pale sky colour by how low it lies (below the lowest quarter of the course's
heights), how hollow it is (the ambient occlusion) and whether it is water, broken up by the drifting noise. Haze is a
gradient over the upper part of the view in the sky's horizon colour, stronger the more of the course the view takes in.

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

### Limitations

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
size), so it clears the cache before and after (without that the next frame's course came out black).

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
