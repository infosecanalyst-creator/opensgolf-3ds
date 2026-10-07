# Terrain engine notes (from Terrain.dll)

Source of these notes: static analysis of `Terrain.dll` from the retail disc. It is a normal,
unprotected debug build (original source path `C:\Projects\3DTerrainLowPoly\Terrain.cpp`, MSVC
debug CRT), so function bodies are easy to read. Nothing here is copied code, only observed
structure and constants, written down so a new implementation can interoperate with the data.
"Confirmed" means read directly from disassembly. "Inferred" means a reasoned guess.

Helper for reproducing the analysis: `tools/re/x86dis.py`.

## Rendering model (confirmed)

* Legacy fixed-function OpenGL 1.x: `glBegin/glEnd`, vertex arrays (`glVertexPointer` 3 floats,
  `glNormalPointer`), `glLightfv`, `glMaterialfv`, 2D mipmapped textures. Works on macOS through
  SDL2's default (legacy) context. Window and GL context setup in the original is WGL, to be replaced.
* Camera is orthographic, depth range -5000..+5000. `render()` does:
  `glRotated(pitch, 1,0,0)`, `glRotatef(45 + userRotation, 0,1,0)`,
  `glTranslatef((25 - tileX) * 100, 0, (25 - tileY) * 100)` to centre on a tile.
* Pitch is a global set per resolution in `initSystem`: 800x600 = 38.682 degrees,
  1024x768 = 40.542, 1280x1024 = 40.832. These match the pre-rendered sprites' projection, so
  sprites and terrain line up.
* Ortho extents per resolution (resize): 800x600 -> 1767x1325 world units, 1024x768 -> 1810x1303,
  1280x1024 -> 1740x1392; any other size uses width/height directly. `setZoomLevel` scales by 4
  and 0.5 for zoomed levels.
* World scale: one tile is 100 units. Vertices: a 3x3 patch per tile (corners, edge midpoints,
  centre) 50 units apart, X = tileX*100 - W*50 + sub*50, Z likewise, Y starts at 0 with normal (0,1,0).
  The vertex array is global, 12 bytes per vertex, laid out as (3W) x (3H).
* Texture coordinates come from a constant table (values 0, 0.5, 1) indexed per triangle vertex.
* Tiles are drawn as triangle lists stored in the tile: count at tile+0x44, records of 0x38 bytes
  from tile+0x48 (three vertex indices, three pairs of UV-table indices, a GL texture id at
  record+0x24, a byte at record+0x28).
* A global animation phase (at 0x10070a14 in the DLL image) drives animated tiles: for tile type 7
  the texture is picked from the 4 sets starting at index 27 as `(variation & 3) - phase` mod 4.

## Terrain object and Tile (partly confirmed)

| Object | Offset | Meaning |
|--------|--------|---------|
| Terrain | 0x00 | HGLRC (OpenGL context, WGL) |
| Terrain | 0x14 / 0x18 | grid width / height in tiles (`tileAt` bounds check) |
| Terrain | 0x1c | zoom level (render uses `16 << zoom`) |
| Terrain | 0x20 / 0x24 | viewport width / height |
| Terrain | 0x28 | flag from the last argument of `initSystem` (flips the ortho Y range) |
| Terrain | 0x2c | table of 37 type-name slots, 0x18 bytes each; +0x14 of a slot is the count of texture sets loaded |
| Terrain | 0x3a4 | tile array, 0x248 bytes per tile, index = x + y * width |
| Tile | 0x00 | int array read by `getElevation(corner)` (inferred: per corner values; see note) |
| Tile | 0x24 | tile type id (asserted `< NUM_TEXTURED_TILES`) |
| Tile | 0x28 | variation byte |
| Tile | 0x44 | triangle count, records from 0x48 |
| Tile | 0x208 | has-path byte |
| Tile | 0x234 + side | wall flags (`getWall(side)`) |
| Tile | 0x240 | texture set index |

Note: the meaning of the values behind `getElevation` is not settled. A separate routine writes
heights to vertices through a similar looking index table, so these may be vertex indices rather
than heights. Elevation scale in world units is therefore not recovered; the game uses 12 units
per level as a placeholder (`kHeightStep`).

## Tile types (confirmed from the constructor)

IDs 0..30, names used as texture file prefixes. Unnamed ids: 6, 7, 14, 15, 16, 20, 21. Id 19 is a
second "Overgrowth" (probably a copy-paste slip in the original).

| Id | Name | Id | Name | Id | Name |
|----|------|----|------|----|------|
| 0 | Tee | 10 | Overgrowth | 23 | WaterMiddle |
| 1 | PuttingGreen | 11 | Brush | 24 | WaterDeep |
| 2 | Fairway | 12 | Rock | 25 | WaterShallowDesert |
| 3 | FirmFairway | 13 | Woods | 26 | TrickyGreen |
| 4 | Rough | 17 | WaterShallow | 27..30 | SandBunker1..4 |
| 5 | DeepRough | 18 | Marsh | | |
| 8 | GrassySand | 19 | Overgrowth | | |
| 9 | PotSandBunker | 22 | Building | | |

## Texture loading (confirmed)

* Folder: `Data/Textures/<Theme>/` where theme comes from the course type: 0 Parkland (also the
  default), 1 Tropical, 2 Desert, 3 Links (inferred from jump table order).
* For each type the loader tries sets `A`..`E`: file `<TypeName><Set>0001.bmp`; if it exists it
  also loads `0002`..`0009`. Stops at the first missing set. Stored in a global table
  `tex[37 types][25 sets][9 variations]`. Type 0 (Tee) uses a slightly different loop.
* Textures are 64x64 24-bit BMPs, built with `gluBuild2DMipmaps`, linear filtering, repeat wrap.
* Overlays loaded per theme: `Path.tga`, `PathCap.tga`, `PathInside.tga`, `PathCurve.tga`, the same
  four with an `X` suffix, `CliffTest.bmp`, `RetainWallA.bmp`, `strata.bmp`.
* File names differ in case between themes (`overgrowth` vs `Overgrowth`): look them up
  case-insensitively.
* Lighting: `<Theme>Lighting.txt` with `#AMBIENT`, `#DIFFUSE`, `#SPECULAR` (r g b, 0..255) and an
  undecoded `#HIGHLIGHT` hex block. `changeLighting` steps values by 0.1 and clamps at 1.0.

## Edge blending (per-triangle texture variation)

Correction to earlier notes: the letter in a texture name (A..E) is an alternative *look* of the same
terrain, picked at random per tile (`Terrain::setType` does `rand() % number_of_sets_for_the_type`,
stored at tile+0x240). The number 0001..0009 is the *shape*: variation 1 is the plain fill and the
others are border and corner pieces. Variation index = file number - 1.

Every tile is a 3x3 vertex patch, so 4 quadrants of 50x50 units. Each quadrant is split along its
NW-SE diagonal into 2 triangles, giving 8 triangle records (stride 0x38 from tile+0x48). Record order is
quadrants NW, NE, SW, SE, two triangles each: `(j,k),(j+1,k),(j+1,k+1)` then `(j,k),(j+1,k+1),(j,k+1)`
(j = row along tile Y/world Z, k = column along tile X). UV = (k/2, 1 - j/2) with the original's
bottom-up bitmaps, so each quadrant shows one quarter of the 64x64 texture and the border art of the
variations lines up with the tile edge.

Neighbour slots (`Tile::setNeighbor(dir, tile)`): dir 0 = (x, y-1) "N", dir 1 = (x, y+1) "S",
dir 2 = (x-1, y) "W", dir 3 = (x+1, y) "E", stored at tile+0x34/+0x38/+0x3c/+0x40.

`Tile::m1096` (the mesh builder, run for the tile and then its four neighbours by `m103c`) computes the
relation of this tile to N, S, W, E and the four diagonals (`m111d`): 0 same, 1 different class,
2 same class but different type, 0 when off the map. For each of 8 triangles it combines three
relations (a, b, c) into `code = {0,4,40}[a] + {0,1,10}[b] + {0,2,20}[c]`, looks that up in a 71 byte table
at RVA 0x13e7a and maps the result through a 9 entry jump table (`m1262`) to the variation 0..8.
The table and mapping are in `blendVariation()`.

| Triangle index | a | b | c | Record |
|---|---|---|---|---|
| 0 | N | E | NE | 2 |
| 1 | E | N | NE | 3 |
| 2 | E | S | SE | 7 |
| 3 | S | E | SE | 6 |
| 4 | S | W | SW | 5 |
| 5 | W | S | SW | 4 |
| 6 | W | N | NW | 0 |
| 7 | N | W | NW | 1 |

Texture type for a tile type (`Tile::m116d`): type 1 with bit 0x80 of the variation byte becomes
TrickyGreen (26); types 6 and 21 use Rough (4); type 7 uses SandBunker1..4 by `byte & 3`; types 13..16
use Woods (13); type 17 (water) uses WaterMiddle (23) or WaterDeep (24) when the byte is 1 or 2, so
water depth lives in that byte; type 22 uses Rough when the byte is 0x40..0x47. Rough, Fairway and type 7
clamp variations above 4 to 0 (they have only 5 files).

Special tiles: Tee (0) puts the variation byte in as its texture *set* and uses variation 0 on all
8 triangles; PotSandBunker (9) uses variation index 3 (the rounded piece) on all 8 and then lets the
neighbours rebuild. Sand (type 7, `m1203`) is dished: its centre vertex is lowered 13 units, edge
midpoints where the neighbour is also type 7, corners where both edge neighbours and the diagonal are type 7.
Type 6 (`m1230`) raises 20 units. Vertices are not shared between tiles (the vertex array is 3W x 3H),
so both tiles lower their coincident vertices.

The "class" of a type comes from an array golf.exe hands to `Terrain.dll` at run time (an exported
function that copies `count` ints into a global at 0x10106b48). That table lives in the protected
executable, which this project does not read, so `typeClass()` in `terrain.cpp` is my own grouping
(greens, fairways, rough, deep rough and brush, sand, water, rock, woods, building).

## What the game's renderer does and does not reproduce

Reproduces: orthographic camera and angles, world scale, 3x3 vertex patches with 0/0.5/1 UVs, 8 triangles
per tile with per-triangle texture variation exactly as above, random sets, tee/pot bunker/water depth
special cases, sand dishing, type names and texture file selection, per-theme lighting files,
case-insensitive lookup.

Approximates (marked `APPROXIMATION` in code): elevation scale; the light direction and strength; the
type class grouping; the water-in-desert-theme variant (`m1096` swaps triangles next to land to
WaterShallowDesert when a global theme flag is 1, not ported yet).

Not decoded yet: path overlays, cliffs and retaining walls, water animation timing (type 7 phase at
0x10070a14), type 6 geometry, and how `golf.exe` stores a course (that code is in the protected
executable, which this project does not touch).
