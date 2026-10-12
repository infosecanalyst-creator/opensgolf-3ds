# Enhanced graphics (optional)

*Enhanced graphics* are optional extras for today's large screens. They change how the game is drawn, never its art files,
layouts or rules. **Classic stays the default and is unchanged**: with Enhanced off the game runs the same drawing code and
produces the same pixels as before, byte for byte. Enhanced can be combined with HD graphics ([HD.md](HD.md)).

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
With HD on, that offscreen course keeps HD's 4× anti-aliasing.

### Turning it on

Until the effects join the Enhanced graphics preferences, a temporary switch:

- `--fx` turns them on with the defaults above; `--fx-set LIST` turns them on and sets options, a comma separated list of
  `key=value` (value: `on` / `off`, a choice's number, or `sharp`, `edge`, `low`, `medium`, `high`), for example
  `--fx-set scaler=sharp,tilt-shift=on,quality=high`;
- otherwise the environment variable `OPENSGOLF_FX` (`1`, or a list as above);
- `--fx-list` prints the options, their labels and choices.

For the preferences screen, `fx::options()` lists every option as `FxOption { key, label, choices, get, set }` (label and
choices are text keys in `lang/en.txt`, `STR_FX_*`), the first being the effects' own on/off. `App::fx_set(settings)`
applies a changed `FxSettings` (it rebuilds what the terrain meshes depend on).

### How Classic stays the same

Every hook is behind `fx::active()`, false unless the effects were turned on:

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
