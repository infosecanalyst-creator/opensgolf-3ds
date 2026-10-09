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
| T | switch tool: Paint, Raise/Lower, Path (shift removes; [ and ] switch gravel / paved), Wall (nearest tile edge; shift removes) |
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

Terrain.dll's `Tile` keeps one wall flag per direction (`Tile::getWall` / `setWall`, byte at +0x234 and a value per direction at +0x210),
so walls are per tile edge, and the editor does the same: the Wall tool toggles the edge nearest the cursor. How the original draws
and prices a wall is not in the DLL (it only stores the flags), so the look is a PLACEHOLDER: a 30 unit strip of `RetainingWallA.bmp`
standing on the edge with a thin cap. Edges are shared, so the neighbouring tile's flag is kept in step.

## Water

Water tiles drift their texture by about a pixel, a shimmer that is my APPROXIMATION (the original's water animation is not decoded).
In the Desert theme shallow water uses the `WaterShallowDesert` textures, which the original does by swapping type 17 for 25.

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
`--edit "p:x,y,type[,vbyte,radius];r:cx,cy,delta[,radius];w:x,y,kind[,radius];k:x,y,dir"` (k = wall on edge dir 0..3 = N, E, S, W) applies scripted paint and raise edits, which is how the headless tests run.

## Clubhouse connection, hole report and staff

- A path only counts if it joins the Clubhouse lot through 4-adjacent path tiles (manual: buildings need an unbroken pathway to the Clubhouse). Detached paths are drawn as brown mud tracks, as the manual describes.
- F1 prints the hole report (length, par, SGA class from length/accuracy/imagination). It also prints after every edit. The thresholds are placeholders; the manual gives the class names only.
- Shift+C, Shift+R, Shift+G, Shift+V hire a Club Pro, Ranger, Groundskeeper or Soda Vendor. Ctrl with the same key fires one. The title bar shows fun, attitude (red/yellow/green) and staff count.
- Effects, wages and fun deltas are placeholders. A Ranger speeds play by 20%, a Club Pro and Soda Vendor raise fun. Wages are 30, 25, 20 and 15 a day.
