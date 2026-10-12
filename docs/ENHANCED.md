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
