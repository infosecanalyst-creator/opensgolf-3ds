# Sprites (Flics/*.flc)

Everything in `Flics/` is a pre-rendered sprite stored as an 8-bit FLC (FLI/FLC chunk types 4, 7, 12, 15
as described in `FORMATS.md`). A sprite is usually two files: `Name.flc` and `NameShadow.flc`. Colour
variants are separate 8-bit PCX palettes (`PalGreenPalm.pcx`, `clubL2 palette.pcx`...) that replace the
FLC palette. Buildings may also have `_base` / `_dirt` ground overlays and `_ANIM` animation sprites.

## Header extension

All 1,893 sprite FLCs carry a Firaxis extension in the "reserved" part of the 128 byte FLC header
(offsets are from the start of the file, all little endian u16):

| Offset | Meaning | Examples |
|---|---|---|
| 96 | number of views stored | 1, 2, 4 (trees, buildings), 8 (people, animals) |
| 98 | frames per view | 16 (tree growth), 1 (static building), 20 (swing) |
| 100, 102 | crop origin (x, y) in the render canvas | tree 217,173; hotel 126,113 |
| 104, 106 | render canvas size | always 480 x 480 |
| 108 | duration of one view in ms | `framesPerView * speed` |
| 112 | bit mask of stored views | 0x0F, 0xFF, 0x01, 0x03 |

The header `frames` field is `views * framesPerView`. The file stores each view as
`framesPerView + 1` chunks: the extra last chunk of every view is the FLC "ring frame" that loops the
view back to its first frame, so the loader must decode all chunks and drop one per view, otherwise
frames of later views are shifted by one per view (this was why trees first looked as if they had odd
cycle lengths). `decodeFlc` does this; frames are view-major.

## Anchor and scale

Sprites were rendered on a 480x480 canvas whose centre (240, 240) is the object's ground point. The
frame is the crop of that canvas starting at the crop origin, so the ground point is pixel
`(240 - cropX, 240 - cropY)` inside the frame. For `TreeMapleLarge` that is (23, 67) in a 67x74 frame,
which is the foot of the trunk.

Scale: the 800x600 camera shows 1767 world units across 800 pixels (see `TERRAIN.md`), so one sprite
pixel is 1767/800 = 2.209 world units. A 4x4 tile building footprint (565 units wide on screen) is
about 256 px wide, matching the 229 to 243 px building sprites. `kSpriteUnitsPerPixel`.

## Transparency and shadows

Palette index 255 is the key colour (magenta or cyan depending on the file); it is not index 0.
Shadow sprites use the same frame layout and anchor as their body and a 5 colour palette ramp
(white, light green, mid green, green plus the key); the game draws it as translucent black whose
density follows the ramp. The shadow does **not** rotate between views, only the body does: the
light is fixed relative to the camera, so shadows always fall the same way on screen. (The game's
terrain light is fixed in eye space for the same reason.)

## Views and camera rotation

The game renders sprites for 4 camera yaws, 90 degrees apart (view 0 is the default camera). I fitted
the screen position of a building's tower across the 4 hotel views: the object turns counter clockwise
on screen as the view index grows, which is what `glRotatef(45 + 90*k, 0, 1, 0)` does to the world,
so `view = round(rot / 90) mod 4` with the game's rotation. 8 view sprites (people, animals) add
45 degree steps for the facing direction. View 0 faces screen down-left at the default camera, which is
world -X (heading `atan2(z, x)` = 180 degrees); each next view turns 45 degrees counter clockwise on
screen (down-left, down, down-right, right, up-right, up, up-left, left), so heading `phi` at camera
quarter `q` uses `view = (round((180 - phi) / 45) + 2 * q) mod 8`. Checked in the game with golfers walking
the fairway at 0 and 90 degrees of rotation: they face their direction of travel.

## Walk cycle

`Male|Female/<Body>_NormalWalk.flc` has 8 views x 16 frames at 83 ms (1.33 s per cycle). The demo moves
golfers at 85 to 110 world units per second along the fairway and plays the cycle at the file's own frame time.
How the game ties stride length to speed is in the protected executable and is not reproduced.

## Tree frames

Trees have 16 frames per view that are a growth cycle (seed, bare sapling, leaf out, full crown), not
a sway animation. The demo draws the last frame. How the game picks the growth frame is in the
protected executable.

## What is not done

* Animated sprites (`_ANIM`, people, water) and their timing.
* Choosing a colour variant: the demo only applies the palettes it needs for the tropical palms.
* Sorting. The game draws sprites far to near by eye depth (painter's order), which is how a 2D
  overlay behaves; it is not an exact replica of the original order.
