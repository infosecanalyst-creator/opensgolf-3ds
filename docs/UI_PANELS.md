# Dock panels: Elevation, Add Buildings, Amenities, Employees, Golfers, Player

Machine readable companion: `include/sg/ui_panels.h` (namespace `sg::ui_panels`). Everything below was read from the publisher
exe (`golf_pub.exe`) through the existing Ghidra decompile and plain disassembly. No art is copied; the port loads the sheets from
the disc folder.

Confidence marks: **exact** = read from the code and checked against the disassembly; **derived** = follows from the code but part
of the picture is not decoded; **unknown** = not decoded (listed again at the end).

## 1. Method

* The loader (golf_decomp.c 41050 to 41330) cuts each panel sheet into sprite objects of 0x2c bytes. I used the emulated cut list
  (`spec/cuts.pkl`) to know which object address holds which sheet rectangle, then read the draw and hit routines to see which
  object address is blitted where. A button with N states occupies N consecutive objects, so `lea ecx,[id*0x84 + base]` means
  "first cut of button id" with 0x84 = 3 objects.
* Lesson learned twice: the decompile prints cut calls in a misleading order. The disassembly is the authority (see lot pictures,
  section 5).
* Panel body: the body rectangle is cut from the panel's own sheet and drawn at the same x, y, using the `_A` alpha body first.
  The idle look of every round button, tool button and lot pad is baked into that body. Only hover, selected, armed and disabled
  looks are separate cuts, drawn on top. This is why a button's normal look is not a separate cut.
* Hit tests: the exe's metric `FUN_00467170` (`d = (b<a) ? (b+2a)/2 : (a+2b)/2`, hit when `d < r`) with a per-axis scale; the header
  has `inDisc`. exact.
* Tooltips: shown once the same hit has persisted for more than 10 frames (`DAT_005aa554 > 10`). exact.
* Mode state (`DAT_00567afc`): 0 Build Course, 1 Add Buildings, 2 Golfers, 3 Player with a golfer selected (set when a golfer is
  clicked on the map), 4 Player, 5 closed. Dock buttons 0 to 2 toggle modes 0 to 2 (pressing the active one closes the panel).
  Sub-panel flags: `DAT_0055e924` (mode 0: Terrain or Amenities), `DAT_0055e928` (mode 1: Buildings or Elevation),
  `DAT_00561254` (Employee overlay on modes 2, 3 and 4). exact.

## 2. Build Course panel: the parts missing from the first rebuild (exact)

* Body BaseTerrainPanel (216,482,584,118). Button hits, ids and positions were already rebuilt.
* **Tree button positions are per theme.** The loader copies three positions from a theme table into the slot table each frame
  (table at 0x4c7a2c, 24 bytes per theme): Parkland (704,477) (673,530) (735,511); Desert (704,488) (673,533) (735,496); Tropical
  (704,483) (673,519) (735,496); Links (704,478) (673,512) (735,524). sgview currently uses the Parkland row for every theme.
* Amenities round button: hit (237,525) r20, hover cut BaseTerrainPanel (150,480,34,34) at (220,510). A click sets the Amenities
  flag. Undo round button: hit (261,578) r15, hover cut (40,550,34,40) at (246,562); a click shows the two-line undo help popup and
  arms undo (right click on the map previews, right click again applies; money is refunded).
* The Tees button draws its fourth cut (disabled, `slot*4+3`) while the current hole already has a tee placed or the hole number
  is 19 or more. While the green does not exist yet and the hole number is under 19 an animated 40x40 hint sprite (sprite id
  0x189 + theme) is drawn at (slot 1 x + 24, slot 1 y + 31). derived: I did not map the hole record fields (0x575ab8 and 0x575ac8,
  stride 0x208) to port data, so "tee placed" and "green placed" are my reading.
* Tooltip frame: the box is `FUN_0040cef0(x-80, 402, 160, 112)`, with a strip of 16x19 pieces drawn at y 389 (first piece at
  x-92, then eight at x-76, step 16, the last one is the right end piece). The pieces are cuts of the translucent pop-up sheet
  `TransPopups` (y 60, x 0, 17, 34, size 16x19); only its alpha sheet is cut for this strip, so the box is a shaded translucent
  shape, not coloured art. derived.

## 3. Elevation panel (exact)

Sheet `ElevationPanel`, body (215,482,585,118). Reached only from the Buildings panel; it has no brush size, no smooth or flatten
button: the three tools are the whole panel (the "area" tool smooths by itself).

| id | tooltip | hit centre, r | dst | hover cut | selected cut |
|----|---------|---------------|-----|-----------|--------------|
| 0 | Raise/lower vertex | 352,547 r25 | 312,501 | 0,0,88,99 | 100,0,88,99 |
| 1 | Raise/lower square | 467,547 r25 | 429,501 | 0,100,88,99 | 100,100,88,99 |
| 2 | Raise/lower area | 587,547 r25 | 546,501 | 0,200,88,99 | 100,200,88,99 |
| 3 | Analyze Golf Shot | 715,547 r16 | 688,501 | 400,0,72,99 | 500,0,72,99 |

* Viewing the sheet confirms it: column x 0 is the plain look (used for hover), x 100 the bright green look (selected), x 200 the dark
  look (cut but never drawn); the idle look is baked into the body. Tool 3's pair sits at x 400 and 500.
* Back button "Buildings": hit (270,499) r16, hover cut (0,400,41,44) at (254,483). Undo "Undo": hit (267,561) r12, hover cut
  (0,450,34,40), armed cut (50,450,34,40) while undo mode is on, both at (253,546).
* Draw order (disassembly 0x433e50): alpha body then body at 215,482; hover cut of the back button; armed undo cut while undo mode;
  hover cut of undo or of the hovered tool; selected cut of the current tool; tooltip.
* Click: back leaves the panel (mode 1 buildings, clears the tool); undo shows the help popup, clears the tool, returns to the
  buildings panel and arms undo; a tool hit stores the tool, sets flag 2 of the mode word and clears the building tool.
* Priority in the hit routine: undo, tool 3, 2, 1, 0, back.
* The elevation edit rules (height range 3 to 13, square, area kernel, sounds 0xc7 and 0xc8, keys - and =) are in
  `docs/DECODE_PANELS.md` section 3 and `include/sg/panels.h`; this file does not repeat them.

## 4. Add Buildings panel, levels and upgrades (exact unless noted)

Sheet `BuildingPanel`, body (216,482,584,118). Nine lots in two rows (5 and 4), lot index = building id - 6. Path, bench, flower
bed, ball washer, landmark and home site are on the Amenities panel; the Clubhouse has no button.

* Lot hit: centre x 383 + 80 i (row one), 423 + 80 (i-5) (row two), y 531 / 571, scale ys = 2, r40. Evaluated after the elevation
  button (236,526 r16) and undo (277,567 r12).
* Pads: pad destination (343 + 80 i, 501) row one, (384 + 80 (i-5), 542) row two, the exe steps 0x50 across and then adds 0x29 and
  subtracts 0x117. The pad cuts are drawn only for buildable lots: hover cut while hovered, selected cut while it is the armed
  building (`DAT_004c2854 == id`). Row one cuts (0,0,74,64) hover and (100,0,77,64) selected; row two (0,100,77,58) and (100,100,77,58);
  the cut at x 200 (blue) exists but is never drawn. The exe also issues an "idle" pad blit from an object that is not cut from
  any sheet; treat it as blank.
* **Level upgrade buttons**: there are none. The lot button itself is the upgrade button. Per building a level 0, 1 or 2 is kept
  (`DAT_005a8c38[id]`). A lot is buildable when `id < unlock - level && level < 2 && (level == 0 || nextHole > 10)`, where `unlock`
  is 6 in a normal game (it grows as holes open) and 17 in sandbox. Clicking a not buildable lot arms nothing and shows one of three red
  popups (error sound 0x18), checked in this order: upgraded buildings need more than 9 holes, cannot upgrade further, will become available later
  (strings in the header). Price at placement is `cost * (level + 2) / 2` plus terrain cost under the footprint. The picture of a lot changes
  to the "upgraded" art when its level is not 0.
* **Lot pictures**: the layout sheet of the theme (`Parklands Layout`,
  `Desert Layout`, `Trop Layout`, `Links Layout`, indexed by the exe theme order) is cut into cells 75 wide, 100 high, column = lot
  index, and each column loads four objects. The disassembly (0x43eb43) stores the cuts y 0, 200, 100, 300 into objects +0, +1, +2,
  +3. The draw picks object `4*lot + 2*upgraded + buildable`, so the sheet row is `{0,200,100,300}[2*upgraded + buildable]`
  (grey, colour, grey upgraded, colour upgraded). Picture destination `((lot/5) + (lot%5)*2)*40 + 344` by `453 + (lot/5)*40`, one pixel
  higher while the lot is hovered and buildable.
* Info box: after the hover delay a framed box 160x100 at y 460, x = `clamp((lot%5)*80 + (lot>4 ? 431 : 391), 80, 720) - 80`, with
  centred lines at y 462 (`Upgraded ` + name when upgraded, except the Snack Bar), 472 (`Cost: N`, N grouped with commas and no money sign: "Cost: 10,000" in footage; the frame grows to 160 x 111 from y 455 and is drawn before the panel body, which covers its lower part) and 482 (tip text). N is
  cost * 100 at level 0 and cost * 300 / 2 when upgraded. The frame art is the pop-up frame set (`Pop_UpOk`), found by the previous
  decode.
* Elevation round button: hit (236,526) r16, hover cut (150,480,34,34) at (220,510), tooltip "Elevation". Undo: hit (277,567) r12,
  hover cut (40,550,34,40), armed cut (80,550,34,40), both at (261,553), same popup as above and it stays on the panel.
* Per lot data (name, footprint, cost in $100 units, tip) is in `kLots`; read from the building table at 0x4c2728 (stride 20 bytes:
  name, footprint short at +0x10, cost short at +0x12).

## 5. Amenities panel (exact)

Sheet `AmenitiesPanel`, body (214,482,586,118). Reached from the Terrain panel; back button "Course Terrain" hit (269,500) r16,
hover cut (0,400,34,34) at (254,483).

* Each slot's hover cut is the first of three cuts: hover at x, selected at x+100, disabled at x+200 (right hand group at 300,
  400, 500). Viewing the sheet confirms the mapping: ball washer (0,0), path (0,100), home site sign (0,200); bench (300,0),
  bridge (300,50), obelisk landmark (300,100), flowers (300,150), tree (300,200). The idle looks are baked into the body.
* Slot table, tool ids and hit discs are in `kAmenitySlots`. The slot that is drawn selected comes from the armed tool
  (`amenitySlotForTool`). Slot 8 (762,545) has art but no hit test; the undo slot (7) draws hover only, with no armed look.
* Home Site (slot 2) draws its disabled cut when `DAT_0056d1b0 < 1` (no free site) and cannot be armed.
* **Variant strips**: arming bench, landmark, bridge, flower or tree shows a strip of designs above the panel; the pointer picks a
  design while the strip is visible (hover chooses, the following map click uses the choice). Geometry is in the header
  (`stripX0`, `stripEntryCx`, `stripEntryCy`, per widget parameters `kVariantStrips`); the entry pictures are sprite objects from
  the per-theme sprite libraries (ids 0x208+ benches, 0x168+ landmarks, 0x226+ bridges, 0x1a3+ flowers, 0x12f+ trees) drawn
  through the game's sprite loader, not from the panel sheet. The 40 px (or 100 px for trees) pictures sit on diamond pad cuts
  at AmenitiesPanel (300..600, 350..400, 47x40/41) whose use (normal, highlight, dark) was not mapped. derived.
* Landmarks: only designs whose bit is set in `DAT_00543cfc` are listed (sandbox sets 14 bits); cost `250 + 50*design` units, shown
  as "FREE!" if that design's bit is set in `DAT_00822c70`. Tooltip also shows the design name uppercased and the effect from
  `design & 3`.

## 6. Employee panel (exact)

Sheet `EmployeePanel`, body (215,474,585,126), an overlay on the Golfers and Player panels (opened by the people button on those
panels, which sets the overlay flag and the "hire click pending" flag).

* Hire button (270,558) r16 at (254,542). Only the **hover cut** (0,500,42,42) is a visible hover; pressed (50,500) is drawn for
  the one frame in which the hire dialog is opened; disabled (100,500) is drawn when 20 employees exist. The `y = 550` cuts are
  cut but empty on the sheet, and the exe blits one of them each frame (harmless). The hire thumbs up icon is baked into the body.
* The first employee click handler sets the pending flag, so clicking Hire opens the hire dialog on the next draw; with 20
  employees it plays error sound 0x18 instead. There are no hire or fire "arrows and counts" on this panel: the arrows are the two
  scroll arrows of the roster; there is no count text except the selected employee's counter.
* Roster: 8 portrait slots in 4 columns by 2 rows at the positions in `kEmpPortraitDst` (hit centre dst + (32,16), r16). Scroll
  arrows move one column (2 employees); the right arrow draws only when more than 8 remain, the left only when scrolled. Quirk:
  a click on the right arrow zone adds 2 to the offset even when the arrow is not drawn; the next draw pulls the offset back
  (`offset -= 2` while fewer than 7 remain). Hover of an arrow draws its hover cut only when scrolling is possible.
* Portrait cuts: hover (0,y,63,43), selected (100,y), spare (200,y), y = 0 top row, 50 bottom row. A portrait shows the employee's
  animated staff sprite at (slot x + 30, slot y + 24). The sprite set is `kind + 4*skilled` into eight sets named in the header
  (`employee\GreeterWalk` and friends); walk art for the portrait while idle states 0 to 10, "SQ" and action sprites for other states.
* Selected employee: Move, Fire and Rename (hits and cuts in `kEmpActions`). Idle look is cut 4 (x 700), hover cut 1 (x 400), armed
  (x 500, Move only) while move mode is on for that employee, disabled (x 600) when nobody is selected. Fire charges 25 units
  (booked in the month ledger), Rename opens a text prompt "Rename Employee...", Move arms the next map click.
* Info text (selected employee) centred at x 700: name y 534, `Hired: <date>` 546, `Paid: <dollars>` 558, counter label 572, value 584.
* Tabs: Golfers tab hit (286,492) r16, hover cut (0,400,33,33) at (270,475); Player tab hit (256,510) r16, hover cut (0,450,33,33)
  at (240,494). Slots are tested first.
* Hire dialog: title "HIRE AN EMPLOYEE" at (377,107); four bands, each two lines (regular above, skilled below), tested on pointer y
  only (bands in the header); the dialog returns `kind*3 + 1 + skilled`. Choice lines at x 246 read "Name: $N per week", with a
  highlight bar (x 231, 291 wide, 16 high) behind the pointed line, and a 40x40 animated portrait per kind at x 576, y 192, 262, 332, 402: the kind's walking clip (0x20e + set, the
  skilled set while its line is pointed at) in view k, palette 0x82 + set, one frame per redraw.
  Wages, kinds and counters are in `kStaffKinds`.

## 7. Golfers panel and Player panel (exact for controls, unknown for contents)

Both are reached from dock button 2 (and a click on a golfer on the map selects Player mode 3). They share the round tab buttons.

* **Golfers (MemberPanel)**, body (215,474,585,126). Round tabs: the Golfers tab (hit 285,492 r20, current, no action), the Player
  tab (hit 256,510 r20, hover cut (0,400,33,33) at (240,494), mode 4), the people button (hit 232,538 r20, hover cut (0,450,33,33)
  at (217,522), opens the Employee overlay). Scroll arrows left and right (hit centres (332,592) and (772,592), ys scale 3) with
  hover cuts (0,150,36,13) at (309,587) and (0,250,36,13) at (756,587); each moves the list by 16 entries. The scroll thumb is a
  filled bar (colour 0x739f) at y 590, height 6, from x `351 + clamp(offset*400/total, 0, 399)` to
  `351 + clamp((offset + 16)*400/total, 1, 400) - 1` (48 instead of 16 in the compact layout); while the list is shorter than
  the offset the draw takes 16 off it. Exact (0x435760, disassembly):
  - The list: slots from the newest golfer down, those on a hole or partnered with one on a hole, from the offset on.
  - Large layout (map zoom 4): 4 rows of 21 px per column from y 498, columns 121 px apart from x 310 while the name column
    (x + 15) stays at or left of 695. At an odd slot (the first of a pair) the card cut (0,0,121,44) at (x, y), covering two
    rows: two name bars and the badge with the hole number in Arial Bold 10 at (x + 8, y + 15) (x + 5 from hole 10), "x"
    while going home (hole 19), the partner's hole while waiting; in a story (+0xb0 not -1) four empty hearts (450,230,7,7)
    then one full heart (450,220,7,7) per story step (+0xb2) at (x + 32 + 10 i, y + 17). A golfer on a hole gets the name in
    Manual SSi Bold 15 at (x + 16, y + 5) and the mood face cut (594 - 16 (k - 1), 100, 16, 16), k = clamp(mood + 2, 1, 10),
    at (x + 102, y + 2). Name colour: black, 0x4210 when fatigue > 160, 0x0018 when thirst > 16, 0x6000 when hunger > 16,
    0x7d08 with flag 0x20000000 (later wins).
  - Compact layout (zoom below 4): 6 rows of 14 px, columns 60 px apart while x + 15 <= 755; the face at (x, y) and the name
    in Arial Bold 10 at (x + 15, y + 1), for every listed golfer.
  - Hit: the cell row clamp((y - 498) / 21, 0, 3) + 4 * clamp((x - 310) / 121, 0, 3) right of 309 and below 497 indexes the
    golfers in drawing order; a click centres the map on the golfer (and opens the card when one is open or the golfer's
    +8 field is set). Hover tooltips after 10 frames: "Golfers", the player name (default "Gary Golf"), "Hire Employees".
  - The People dock button sets mode 2 unless it is 2 already (then closes); the Employee overlay flag (0x561254) stays set
    until a tab on the Employee panel clears it, so People reopens the Employee panel while it is set.
* **Player (JoeCoolPanel)**, body (214,474,586,126). Controls in `kPlayerButtons`: the round stats button, three lower left buttons
  (Practice Round, Play, Begin Tournament; drawn disabled while a golfer is selected or when their condition fails), five shot
  shape ovals (only in mode 3: Straight, Fade L to R, Draw R to L, High backspin, Low punch, setting the shot value 0, -1, 1, 3, 4)
  and the two tabs. The ovals are cells of an 80x50 grid on the sheet, one row per shape (5 rows by 4 columns: normal, selected
  green, dark, spare); the selected shape is drawn with the alpha overlay cuts of `JoeCoolPanel_A` (0,0 and 0,100). The exact blend
  of the overlays and the player screen opened by button 0 are unknown.

## 8. Draw and click recipes for sgview

Terrain panel additions
1. Draw: body; tree slots use `kTerrainTreePos[theme]`; Tees slot in cut 4 when the hole has a tee; hover cut of the Amenities button or Undo
   button when hovered.
2. Click: Amenities button sets `amenities = true`; Undo button shows `kUndoHelp1 + kUndoHelp2` and arms undo.

Elevation panel (shown when mode 1 and the elevation flag is set)
1. Draw body at (215,482). If back hovered draw `kElevBackCut` at `kElevBackDst`. If undo mode draw `kElevUndoArmed` at `kElevUndoDst`;
   else if undo hovered draw `kElevUndoHover`. If a tool is hovered draw its `hover` cut at its dst. Draw the current tool's `selected` cut.
2. Hover: `h = elevHit(mx,my)`; after more than 10 frames on the same `h` draw the generic tooltip bar with the tool tip (or "Buildings", "Undo").
3. Click: `h == -2` leave to Buildings, clear elevation flag; `h == -3` show the help popup, clear tool, back to Buildings, arm undo;
   `h in 0..3` set the elevation tool and clear the building tool.

Buildings panel (mode 1, elevation flag clear)
1. Draw alpha body and body at (216,482). If the elevation button is hovered draw its cut; if undo mode draw the armed undo cut, else if undo hovered draw the hover cut.
2. For each lot i in 0..8: if buildable and hovered draw `padHoverCut(i)` at `kLots[i].pad`; if armed (`tool == id`) draw `padSelectedCut(i)` (buildable or not).
3. For each lot draw `lotIconCut(i, level != 0, buildable)` from the theme layout sheet at `lotIconDst(i, hovered && buildable)`.
4. After the hover delay on a lot draw the info box at `lotInfoCentreX(i) - 80, 460` with the three lines.
5. Click: `h = buildingsHit`. -2 sets the elevation flag; -3 undo; lot i arms building `id` unless one of the three refusals applies.

Amenities panel (mode 0, amenities flag set)
1. Draw body at (214,482); hover cut of the hovered slot at its dst (slot 2 disabled cut when no free site); back button hover cut;
   the selected cut of `amenitySlotForTool(tool)`.
2. While a tool with a strip is armed draw the strip background pieces, the entry pictures and let the pointer choose the entry.
3. Click: slot s arms `kAmenitySlots[s].tool`; slot 7 undo; back clears the amenities flag.

Employee overlay
1. Draw body at (215,474). Left arrow enabled cut when offset > 0, right arrow enabled cut when more than 8 remain. Hover cut of whatever is hit
   (hire, arrow, portrait, action). Idle or disabled cuts of Move, Fire, Rename depending on selection. Hire disabled cut at 20 employees.
2. For portraits draw each employee's sprite at (dst.x + 30, dst.y + 24) and the selected portrait's selected cut. Draw the info text for the selection.
3. Click: hire opens the dialog (or error beep at 20); arrows change the offset by 2; portrait selects; move, fire, rename act on the selection;
   tabs change mode.

## 9. Unknowns, in one list

* Meaning of the empty (`y = 550`) Hire cuts and the animated hint sprite on the Green button (what exactly enables it).
* Landmark and bridge strip: which diamond pad cut is used in which state; theme dependence of the 40 px or 100 px picture size rule (the
  exe computes the size from the sprite loader flag, `-1` in the call means "240 or 140").
* TransPopups header strip: only its alpha is cut; how it is composed with the box fill (colour 0x80007fdc for the buildings frame) is not decoded.
* Player panel: the screen opened by the round stats button, the overlay blend of the shot ovals and the exact
  conditions (flags 0x2000, 0x400000, 0x4200000) that enable Play and Tournament.
* Elevation: money cost of terrain edits (none found), the mouse button mapping of raise and lower, and what the analyze tool does on the map.
* Hire fee (none found), wage payment cadence (see DECODE_PANELS.md).
