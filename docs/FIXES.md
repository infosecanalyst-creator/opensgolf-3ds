# Fixes of the original game's bugs

OpenSGolf plays SimGolf 1:1, bugs included, unless a bug is fixed here. Where the bug is the original's own behaviour the
fix sits behind the setting **Fix the original game's bugs** (a line of Preferences itself, with the note "Its own
glitches only; play is otherwise the original's."; kept as settings.json `enhanced.fix_original_bugs`, where it always
was). It is **on by default** and works in Classic too (the Enhanced master switch does not gate it); the presets of
Select Difficulty's Game box and of the extras box leave it as it is. Turn it off to play the original's behaviour. Crashes and the
port's own bugs are fixed whatever the setting.

- Switch in the game: `enhanced::fix_bugs()` (crates/simgolf/src/enhanced.rs); in the simulation `sg_core::fixes::on()`
  (crates/sg-core/src/fixes.rs, set from the same setting; off in tests and tools, so they keep following the exe).
- Test hook: `SG_ORIGINAL_BUGS=1` turns the setting off for one run (not saved), for before / after stills.

The list of bugs to check came from the public changelogs of other SimGolf projects (feature and bug lists only; none of
their code was read). Each entry says whether the port had the bug, why, and what changed.

| # | Bug | In the port? | Kind | Fix |
|---|-----|--------------|------|-----|
| 1 | Black triangles and slits at waterfalls, along streams and lakes above sea level, and on slopes beside them | yes | original | setting |
| 1b | A dark wedge on stepped water, over the waterfall spray (Enhanced reflections) | yes, after 1 | port (Enhanced) | always |
| 1c | Dark see-through blobs and pink rims on the bank rocks at cliff corners with HD art | yes | port (HD) | always |
| 2 | Charged again for a lot put down on top of a standing one of the same kind | yes | original | setting |
| 3 | Crash on female heads (fixed by the official 1.03 patch) | no | crash | tests |
| 4 | Trees and forests missing at the farthest zoom | no | | |
| 5 | Sprite frame lookups past the end of a file | possible | crash | always |
| 6 | Bank rocks, water glints and flower bed colours missing towards the sides of a wide window | yes | port | always |
| 7 | Esc does not close the golfer card | yes | original | setting |
| 8 | Building upgrades look locked one unlock too early | yes | original | setting |
| 9 | A new arrival overwrites a slow golfer still on the course | yes | original | setting |
| 11 | A click on the zoomed-out Golfers panel list picks the wrong golfer | yes | original | setting |
| 10 | Other checks: sprite queue cap, more than 32 golfers drawn, full accomplishment history, Shift+T, Esc in Preferences and the world map, the ball's shadow | no | | |

## 1. Water above sea level (black triangles at waterfalls and on slopes)

**Reproduced.** Water tiles were always given to the terrain at sea level (`App::sync_tile_corners`), following the exe's
load path (golf.exe 0x449540 lifts each corner (level - 3) times and leaves water at sea level; the exe lifts water only
when it is edited). The course's walls are made for water at its real level (flat at its lowest corner, `Course::corner`
and `Course::refresh`, flag 2 of the water row), so a stream running down a hill, or a lake or sea above sea level (Wales
and Oahu have a whole coast at level 1), is drawn as holes: black slits along every bank, black triangles and wells where
the stream steps down, and the waterfalls hang in the air. With Enhanced the holes show the sky instead.

**Fix (setting).** Water gets its level, flat at its lowest corner, as the walls have it: crates/simgolf/src/app.rs:1262
(`sync_tile_corners`). Changing the setting rebuilds the meshes (enhanced.rs, `apply_enhanced`). The waterfalls and their
spray (sg_core::decor::waterfalls, which already read the real levels) now stand on their steps.

Stills (scratchpad, not in the repository): `before_st_*` / `after_st_*` (a stream stepped down a hill, loaded from a save:
rotations 0 to 270, HD, Enhanced, HD with Enhanced, zoom 2), `before_wales*` / `after_wales*`, `before_oahu` / `after_oahu`.

### 1b. The dark wedge over the spray (Enhanced reflections)

Once water has levels, Enhanced's water reflections (fx_reflect.rs) mirrored a building or tree about the upper water's
surface into the one screen-space reflection picture every water pixel samples, so it showed as a dark wedge on the water
below the step, over the waterfall's spray. A tile with water of more than one level within reach now mirrors nothing:
crates/simgolf/src/fx_reflect.rs:58-76 and :109 (`water_mixed`, crates/simgolf/src/fx.rs:795). Stills `before_wedge` /
`after_wedge`.

### 1c. Bank rocks in HD (dark blobs at the cliff corners)

The upscaled `cliffs01.pcx` of an HD pack (the rocks stood along water banks) came out with rims tinted by the magenta key
and see-through holes inside the rocks, drawn as pink edges and dark blobs at the corners of the banks. The HD sheet now
takes its transparency from the classic picture: clear exactly where the classic pixel is the key (with a neighbouring
rock colour under it, so the filtering does not blend in magenta), opaque elsewhere, and an HD pixel the upscaler left
see-through or tinted towards the key takes the classic pixel's colour: `hd::load_pcx_classic_key`,
crates/simgolf/src/hd.rs:357, used for the bank rocks at crates/simgolf/src/main.rs:340. Stills `before_rocks_hd*` /
`after_rocks_hd*`.

## 2. Charged twice for a building that already stands

**Reproduced.** The exe's footprint test (0x40db90, `Land::fits`) lets a lot overlap an object of its own kind (that is how
an upgrade moves its building), so a second ball washer or landmark could be put down on the tile of the first, each time
charged again (`--edit "b:20,20,3;b:20,20,3"`: cash 345,300 then 340,300, one object more). Unique buildings (kinds 6 and
up) were already refused, as the second one is an upgrade.

**Fix (setting).** A lot that is not an upgrade may not cover a tile of a standing object (`Land::covers_object`,
crates/sg-core/src/land.rs:561): `App::edit_building` refuses it with "There is no room for that building there"
(crates/simgolf/src/app.rs:2937) and the pointer's footprint shows red (crates/simgolf/src/cursor_ui.rs:145). Upgrades are
unaffected. Stills `before_dup` / `after_dup` (logs show the cash).

## 3. The female head crash (official patch 1.03)

**Not reproduced.** docs/PATCHES.md: 1.03 "fixes an obscure crash when selecting female golfer heads on the customisation
screen (affected theme packs and importing custom heads)". Every head lookup in the port is bounds-safe: stock heads come
from the 20-cell halo and expression pages of either gender (both 1400 x 840 and 1000 x 744), custom heads through
`Vec::get` (screens_ui.rs `halo_cell`, `head_over_body`, the pair cards; cust_ui.rs `cust_portrait`, the face picker, which
pages within `head_count`), the head defaults through `head_defaults` (40 entries, female + 20). A test now runs every head
byte 0 to 255 of both genders through the defaults and the signature saying (roster.rs, `every_head_byte_is_safe`).

## 4. Trees missing at the farthest zoom

**Not reproduced.** Stills at the farthest zoom (`after_far_*`: rotations 0 to 270, a map corner, an 1920 x 800 window
Classic and Enhanced, HD, Monterey's forests) all draw every tree; the port has no zoom level of detail for sprites.

## 5. Sprite frames past the end of a file (crash)

A sprite whose header promises more views or frames than the file holds (a mod's, a damaged copy's) made
`Sprite::frame_index` point past `frames`, and the texture lookups index with it. It now stays within the file (the last
frame), and a file with no frames is not loaded: crates/sg-core/src/sprites.rs:36 and :128, test
`frame_index_stays_in_the_file`. The accomplishment board reads its list with `get` (screens_ui.rs:1365, :1374), so a
saved game with a short list cannot crash it.

## 6. Missing draws towards the sides of a wide window (port)

The test whether a tile is on the screen (0x42f940: up to 64 pixels past the sides, 42 past the top and bottom) was made
for the 800 x 600 screen, but a wider or taller window shows more of the course, so the bank rocks, the water glints and
the flower beds' colours stopped short of the window's sides. It now takes the window's visible area:
crates/simgolf/src/wild_ui.rs:335. An 800 x 600 window is unchanged.

## 7. Esc and the golfer card

The original's Esc opened the pause menu over an open golfer card. With the setting Esc closes the card first
(crates/simgolf/src/main.rs:1581). Esc already closed Preferences (popup_ui.rs) and the in-game world map (main.rs).

## 8. Upgrades greyed one unlock too early

EXACT (DECODE_PANELS, Building panel): the exe draws a lot as buildable when `id < unlock - level`, so the upgrade of the
newest building looked locked until the next kind unlocked, although its click (0x434140) only tests `unlock <= id`. With
the setting the panel offers the upgrade as soon as the building is unlocked (and beyond 9 holes, as before):
crates/simgolf/src/panels_ui.rs:704.

## 9. A new golfer overwrites a slow one

EXACT (0x421bc0, `Club::create`): a new arrival takes the slot `create counter mod 152` whether or not it is free, so once
the counter comes round a slow pair still on the course is overwritten. With the setting a new pair takes the next two
slots that are both free (the slots go in pairs, s and s ^ 1), the exe's slot when none is: crates/sg-core/src/golfer.rs:1059,
test `new_pairs_skip_golfers_still_playing`.

## 10. Checked, not bugs in the port

- The exe's sprite queue holds 1024 sprites and its golfer list 32; the port's draw lists are growing `Vec`s and its stream
  vertex buffer grows (gfx.rs), so many trees or golfers on the screen draw them all.
- A full accomplishment history: the board draws all 22 photos (still `after_board_full`, every id earned and some out of
  range through `SG_BOARD_AWARDS`); earning goes through `Vec::get`.
- Shift+T hides every sprite in the port (trees, cacti, swamp trees and scenic trees included).
- Esc closes Preferences (a popup) and the world map opened from a game.
- The ball's shadow is the exe's 2 x 2 black cut of BLDG.PCX (palette index 1, not a shading index) one pixel under the
  ball's ground point; the port draws it as a flat 2 x 2 box, so no HD artefact; left as the original draws it.

## 11. The zoomed-out Golfers panel list

EXACT (0x435570 hit test, 0x435760 draw): below the closest zoom the Golfers panel draws its compact list, six rows of
14 pixels to a column and columns 60 apart, but the hit test always takes the large list's cells (four rows of 21,
columns 121 apart), so a click centred the map on another golfer than the one under the pointer, and the compact list's
further columns could not be clicked at all. With the setting the click takes the cell as drawn:
crates/simgolf/src/member_panel.rs:56.

## 12. A free building past the edge of a small property (#125)

EXACT (0x470a60): the properties that come with a building beside the clubhouse (Jurassic Springs' kind 13 and four
others) put it on the far side unless the tile past it is off the land, but the generator marks the land the player does
not own only at its end (the acres at 0x4720e8, after the clubhouse at 0x471618), so that test never sees the owned edge.
On the smallest deals the building could stand partly past the edge, over the cliff. With the setting the building takes
the other side of the clubhouse when its lot would leave the owned square (`generate`, crates/sg-core/src/land.rs); the
generator makes the same draws, so the rest of the map is the exe's. New games only: a saved game keeps its buildings.

## 13. A path under the pine brush (#126)

EXACT (0x41aa9e, table 0x4c2d28): the brush under the pointer draws every picture of `Data/<theme>.pcx` whose entry in the
exe's table is the brush's tile type, and for any tree type also the trees button's picture (cell 7). The table's 15th
entry is 14, the pine's type, but the sheet's 15th picture is a path, so the pine brush (the trees' bottom left button)
showed a path tile with the trees. With the setting a tree brush draws only the trees picture: `brush_cells`,
crates/simgolf/src/cursor_ui.rs.

## 14. Greens join a putting green (#128), rough a driving range (#139)

A putting green's lot is green tiles round a building tile (`Land::place_sized`), and the exe treats them as any green:
a green painted beside one blends into it (Terrain.dll draws no border between tiles of one class), and the hole tool's rule
that a green next to another green starts no hole without a tee (0x41fee2) counts the putting green, so a green painted
against it got no flag. With the setting the putting green's tiles are a group of their own: they keep their border against
a painted green (`tile_relation`, crates/sg-core/src/terrain.rs; the lot's tiles from `Land::putting_greens_to`, not saved),
and the hole tool does not count them as a neighbouring green (`App::green_joins`, crates/simgolf/src/app.rs).
A driving range's lot is rough (deep rough along its far side) round a building tile, so rough painted beside it blends into
it the same way (#139); with the setting its tiles are a group of their own as well (`Land::lots_to`). A marina's water is
left one with the lake it stands in.

## 15. Land past the property's edge (#144)

The land generator draws the spots for its landmarks, difficulty obstacles and their paths anywhere on the map, and marks the
land the club does not own only at the end (0x4720e8). Round an island the cut-off land becomes sea, so an island property
(Oahu, Wales) could start with a landmark and its garden standing out in the water, and every property could have obstacles
past its edge. With the setting an object reaching the cut-off land is removed, its ground goes back to the level round it
and the cut tiles keep no paths or marks (`Land::clear_cut`, crates/sg-core/src/land.rs). The draws are the same, so the
rest of the map is the exe's.
A coastal property's sea is laid down first, with the first tile of each row out of bounds, and the 24 ground patches come
after it; the first patch is always a wetland, and its random walk can reach the sea's edge and turn those tiles back into
wetland or pond, so a strip of marsh stood past the map's edge (the reporter's Florida). With the setting that edge is out
of bounds again at the end.
