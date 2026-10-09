# Course editing

Press Tab in the game to toggle edit mode (the active tool is shown under the club name).

| Input | Action |
|---|---|
| Left click / drag | apply the tool |
| Shift + left | lower terrain (Raise tool) |
| Right drag | pan |
| Z / X | zoom in / out (the original's keys; + and - and the wheel also zoom outside edit mode) |
| F, G, R, S, W, P | edit mode, the original's terrain hotkeys: fairway, green (press again for tee), rough, sandtrap, water, pathway |
| = and - | edit mode: raise / lower terrain (the original's keys; shift flips) |
| Shift+S / Shift+L | save / load (as in the original, alongside F5 and F9) |
| Shift+P / Shift+T | pause / toggle trees and scenery |
| [ and ] | previous / next tile type |
| , and . | brush radius (1, 3, 5 ... tiles across) |
| F5 / F9 | save / load `course.sgc` in the working directory |

Paint types: fairway, firm fairway, green, tee, rough, deep rough, woods, sand bunker (type 7, dished),
pot bunker, water shallow/middle/deep (type 17 plus depth byte), rock, brush, building lot.
Edge blending and trees are rebuilt after every edit. Painting tee, green, water or building flattens that tile's corners to their average level.

Height edits move one corner (plus the brush area) by one level, clamped to 0..24. Neighbouring corners are
relaxed so they never differ by more than one level. The limit of one is a PLACEHOLDER: the original slope rule is not decoded.

## Paths and extra tile types

Paths are an overlay on top of the terrain, one flag per tile (0 none, 1 gravel, 2 paved). This is OUR model: how the original
stores and connects paths lives in golf.exe, which is not read. Terrain.dll only gives the artwork: `Path.tga`, `PathCurve.tga`,
`PathCap.tga`, `PathInside.tga` and the paved `*X.tga` versions (64x64 with alpha). The game cuts a centre piece and an arm
toward each neighbouring path tile out of the cross shaped `Path.tga`, and uses the round `PathCurve.tga` for path ends and bends.
`PathCap` and `PathInside` are not used yet.

The game also ships texture sets for Cliff, Ravine, FlowerBed, ZenSand and GrassBunker, but their tile ids are not in Terrain.dll (the manual, p. 16, says Ravine is the Desert theme's name for the Stream hazard, and says nothing about cliffs).
The editor offers them under made-up ids 31 to 35 (marked "editor id" in the type list). Nothing here claims those are the original
ids, and no automatic cliffs on steep slopes are generated.

## Retaining walls

The original has no wall tool: it rebuilds the walls whenever the ground changes (0x449540). A tile gets a wall on each edge
whose neighbour stands higher there: the tile's two corners on that edge, flattened as its type flattens them (tees at their
highest corner, water at its lowest, buildings at their level), are compared with the neighbour's (0x42f530, the course's wall
bits). The style handed to Terrain.dll is the terrain table's byte +0x28 of the tile's type (1 water, 2 any other ground; the
neighbour's when it is 0). The port derives the walls the same way (`build_walls`); the look is a PLACEHOLDER (a strip of
`RetainingWallA.bmp` as tall as the step), since the port's ground is one continuous surface rather than Terrain.dll's
per-tile corners. The exe also stands rock cuts of `cliffs01.pcx` on water banks where a wall faces the camera, picked by the
bank's height in steps (`sg_core::decor::bank_rocks`, drawn by the port before the sprites).

## The tool under the pointer

Decoded from the main frame (0x40f5c0), see `crates/simgolf/src/cursor_ui.rs`: a terrain brush shows its tile picture from
`Data/<theme>.pcx` at half strength on the tile (no outline, no brush size; sand traps turn with Tab), a woods brush a
see-through tree, the green's tricky variant the words "Tricky Green"; Undo names what it would undo ("Reset to rough",
"Remove Path", "Demolish Snack Bar", green when a right click would undo it); the elevation tools draw a black tile grid, the
heights of the nearby vertices and a purple mark or square; the building tool outlines the footprint (white, red where it
will not fit), draws the building see-through (red where refused), names amenities and shows a clearing charge as "-N"; a bench
shows a seat on each side a golfer could sit facing, a willow and a scenic bridge their design; a landmark rings the area it
cheers; a Home Site shows its sums (lot value share, clearing, site preparation, profit) in a box at the right. While a hole
is being built the green brush shows the see-through pin flag until the hole has its green, and a white line from the tee to
the pointer with "N yards" on its middle (the tee brush the same from a green that waits for its tee).

Garden items take their design from a strip the Amenities panel opens over itself (0x432200): five benches, the club's
landmarks, eight bridges, fifteen flower beds (three shapes in five colours) or seven scenic trees, each on a button (blue for
the current design, light under the pointer); a click in the panel picks the design last under the pointer. Picking the
amenity starts at the first bench, a random flower bed of the first shape, a random tree or bridge, and no landmark (a click on
the course with no landmark chosen puts the tool down). Over a landmark the strip names it with its price and its effect.

SG_BUILD=kind arms the building tool with an exe object kind for scripted stills (SG_DESIGN=n picks design n), SG_PAINT=type
the terrain brush of a tile type, and `--edit "K:owned,free"` sets the club's landmark masks.
SG_CURSOR_TILE="x,y" (tiles, fractions allowed) holds the pointer on a tile for scripted stills.

## Water

Water tiles drift their texture by about a pixel, a shimmer that is my APPROXIMATION (the original's water animation is not decoded).
At the closest zoom the frame plots single glinting pixels on water (0x411574, `draw_water_glints`): two tiles in three each
frame, a 5 frame grey-white-grey twinkle once in 64 frames per tile, on the line through the tile's centre.
In the Desert theme shallow water uses the `WaterShallowDesert` textures, which the original does by swapping type 17 for 25.
Where two water tiles meet across a wall the frame draws waterfalls (0x410ea4, `sg_core::decor::waterfalls`): a fall pours
into the lower tile over the side facing the camera, short or tall by the height between them, with a spray at its foot, and
a short fall seen from behind goes down the far sides; all in the theme's water palette, in fixed views, frames running from
7 * x plus the tick.

## Club money (mostly placeholder)

`sg/economy.h`. The original manual (see docs/MANUAL_NOTES.md) gives rules but not one amount. What it confirms and what is implemented:
a golfer pays a green fee on completing a hole, the primary income (implemented: a fee per completed hole, F6 and F7 change it by 5);
Sandbox mode has unlimited funds (implemented: `--sandbox`, nothing is charged and the game cannot end); the starting cash is a fixed
allotment that depends on the property and difficulty (NOT implemented: one fixed number is used).
Other sources: a strategy guide says laying a path costs 100 a square (implemented), and Wikipedia says the game ends if the budget
stays in the red long enough (implemented as a grace period).

Invented: the fee amount (40), the 100,000 start (unverified, I could not find it in the manual or any source I could read), the per
tile daily upkeep (a stand-in for the wages and running costs the manual mentions but does not price), the 120 second game day and
the 30 day grace period. Not modelled: memberships, building lots, refreshments, employees, prize money, SGA ranked holes raising
fees, golfer happiness, difficulty levels. Cash, day and fee show in the window title. The real figures live in golf.exe, which is not read.

## Course file (`.sgc`, our own format, text)

```
SGCOURSE 1
<w> <h>
<w x h tiles: type,variationByte,set>   one row per line
<(w+1) x (h+1) corner levels>           one row per line
```

Optional `PATHS` (digits 0, 1, 2) and `WALLS` (hex digit of the four edge bits, N=1 E=2 S=4 W=8) sections follow, one row per tile row.
The golfer route and clubhouse position belong to the demo and are not saved; they are kept from the running course on load.

## Command line

`--course FILE` loads a course, `--save FILE` writes it after edits, and
`--edit "p:x,y,type[,vbyte,radius];r:cx,cy,delta[,radius];w:x,y,kind[,radius]"` applies scripted paint and raise edits, which is how the headless tests run.

## Clubhouse connection, hole report and staff

- A path only counts if it joins the Clubhouse lot through 4-adjacent path tiles (manual: buildings need an unbroken pathway to the Clubhouse). Detached paths are drawn as brown mud tracks, as the manual describes.
- F1 prints the hole report (length, par, SGA class from length/accuracy/imagination). It also prints after every edit. The thresholds are placeholders; the manual gives the class names only.
- Shift+C, Shift+R, Shift+G, Shift+V hire a Club Pro, Ranger, Groundskeeper or Soda Vendor. Ctrl with the same key fires one. The title bar shows fun, attitude (red/yellow/green) and staff count.
- Effects, wages and fun deltas are placeholders. A Ranger speeds play by 20%, a Club Pro and Soda Vendor raise fun. Wages are 30, 25, 20 and 15 a day.
