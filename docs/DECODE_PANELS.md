# Decode: Elevation, Buildings, Amenities and Employee panels

Source: the publisher's unprotected golf.exe (Ghidra output plus plain disassembly), read for facts only. The machine-readable
tables are in `include/sg/panels.h` (namespace `sg::panels`); this file explains where they come from and what is still a
placeholder. Method as in `docs/UI_ART_MAP.md` (terrain panel section): the loader (`FUN_00442180`, golf_decomp.c lines
41050-41330) cuts the panel sheets into sprite objects, the draw routine blits those objects at literal coordinates, the hit
routine tests circles, and the click routine acts. Sprite objects are 0x2c bytes apart, so the draw routines pick a sprite by
address arithmetic; I emulated the loader over the disassembly to list every cut with its destination object address and
matched those to the draw code (and checked the result by compositing the sheets).

Marks: **exact** = read from the code; **derived** = follows from the code but I did not see the whole picture;
**placeholder** = guess or not decoded.

## 1. Conventions

* Screen is 800x600. Sheets are the files in the game's Interface folder; "X_A" is the alpha mask of sheet X. Every panel
  background is cut from its sheet at (x, y) and drawn at the same (x, y). Magenta key (the per-theme layout sheets use
  (248,0,248)): test r>=240, b>=240, g<=8.
* Distance metric (`FUN_00467170`): `a=|dx*xs|, b=|dy*ys|, d = (b<a) ? (b+2a)/2 : (a+2b)/2`; hit when `d < r`. `Hit{cx,cy,r,xs,ys}`
  in the header and `hitTest()` implement it. Buildings use `ys=2`, the employee scroll arrows `xs=4`.
* Money is stored in units of $100 (display = units * 100). Costs below are in units unless marked $.
* Every panel's hover tooltip appears after the pointer rests 10 frames on the same slot (`DAT_005aa554 > 10`).
  The generic tooltip (`FUN_00432620`, used by Elevation, Buildings (for Undo) and Employee) is a translucent bar
  (`FUN_004493d0(x-3n, y, x+3n, y, 0x80000000, 10, 5)`, n = text length, x clamped to [3n, 800-3n]) with centred text at
  (x, y-5), colour 0x80007fff, drawn at the pointer position minus 5 pixels in y. exact.
* Button sprites on the panel sheets come in groups; the draw code only uses some of them. The header names the role of each
  cut and says which ones are never drawn.
* The tint call `FUN_004762d0(&DAT_00519fd8, 0,0,0)` runs before hover sprites; what table 0x519fd8 does (probably resets a
  colour transform) is not decoded. placeholder: draw the hover cut as is.

## 2. Navigation graph (dock modes)

`DAT_00567afc` selects the dock mode: 0 Build Course, 1 Add Buildings, 2 Golfers (Member panel), 3 and 4 Player (JoeCool).

```
Build Course (mode 0)
  Terrain panel (DAT_0055e924 = 0)  --- button 0x16 (237,525, r20) ---> Amenities (DAT_0055e924 = 1)
  Amenities                          --- "Course Terrain" (269,500, r16) ---> Terrain panel
Add Buildings (mode 1)
  Buildings panel (DAT_0055e928 = 0) --- round button (236,526, r16) ---> Elevation (DAT_0055e928 = 1)
  Elevation                          --- "Buildings" (270,499, r16) ---> Buildings panel
Golfers (mode 2) and Player (mode 4)
  an "employees" button sets DAT_00561254 = 1 and DAT_0059ca54 = 1: the Employee panel replaces the body
  Employee panel -2 tab (286,492) -> mode 2, -3 tab (256,510) -> mode 4, overlay flag cleared
```

The People dock button (0x432720) sets mode 2 unless the mode is 2 already, which closes the dock (mode 5); it never
touches the overlay flag. So the Employee panel opened from the Player panel (mode 4) stays up when People is pressed (the
mode under it becomes 2), a second press closes the dock, and the next People press opens the Employee panel again until
one of its tabs clears the flag. Mode 3 (set in the golfer update 0x4289e0 when the pro waits at his ball) clears the flag.
The Employee overlay draws over modes 2 and 4 only (0x4362f0 tests mode != 3). The dock's hover light compares the button
with the raw mode, so People lights over the Player panel (0x432ba0). exact.

So the Elevation panel is only reachable through Buildings, and the Employee panel is an overlay of the Golfers and Player
panels (it is "People" in the port's wording). The big round dock buttons choose the mode; they are not part of these panels.
exact.

## 3. Elevation panel

Sheet `ElevationPanel`, background cut (215,482,585,118) at (215,482). exact.

| id | tooltip | hit (cx,cy,r) | draw at | hover cut | selected cut |
|----|---------|---------------|---------|-----------|--------------|
| 0 | Raise/lower vertex | 352,547,25 | 312,501 | 0,0,88,99 | 100,0,88,99 |
| 1 | Raise/lower square | 467,547,25 | 429,501 | 0,100,88,99 | 100,100,88,99 |
| 2 | Raise/lower area | 587,547,25 | 546,501 | 0,200,88,99 | 100,200,88,99 |
| 3 | Analyze Golf Shot | 715,547,16 | 688,501 | 400,0,72,99 | 500,0,72,99 |

The third cut of each tool (x=200, or x=600 for tool 3) is cut by the loader but never drawn. A tool is selected by clicking
it: `DAT_00542f20 = id`, flag 2 of `DAT_0059e7b8` is set, `DAT_004c2848 = -1`. The selected tool's selected cut is always drawn.

Round buttons (exact): **Buildings** hit (270,499,r16), hover cut (0,400,41,44) at (254,483), returns to the Buildings
panel. **Undo** hit (267,561,r12), hover cut (0,450,34,40) at (253,546), an armed cut (50,450,34,40) at the same place while undo
mode (flag 0x8000000) is on, a third cut (100,450) unused. Clicking Undo shows the two-line help "To UNDO a previous action,
right click on the map to display the effect of the undo. Right click again to effect the change. Your money will be
refunded!" as a popup (`FUN_0040d320`), clears the tool flag, returns to the Buildings panel and calls `FUN_0045c0c0(0)`, which I
take to arm undo mode (the armed sprite is drawn while flag 0x8000000 is set). derived. Hit priority: Undo > tool 3 > 2 > 1 > 0 > Buildings.

Edit routines (exact, from the disassembly at 0x41d997 lower and 0x41db46 raise; the keyboard table at 0x42191c maps
ASCII `-` (45) to lower and `=` (61) to raise, which also matches MANUAL_NOTES):

* Heights are bytes at `0x5a4998 + x*51 + y`; legal range 3..13.
* **Vertex** (tool 0): the vertex under the cursor changes by 1. Raise clamps to [3,13]. Lower clamps to [3,13] when the byte at
  `0x571ff6 + 46*DAT_0059bf90` is 2 and to [3,10] otherwise (meaning of that byte not decoded; I would treat it as 13).
* **Square** (tool 1): works on the 2x2 vertex block x..x+1, y-1..y instead of the single vertex: raise adds 1 to the block's
  lowest vertices (if the minimum is below 13); lower subtracts 1 from the block's highest vertices (if the maximum is above 3).
* **Area** (tool 2): the vertex edit as tool 0, then a 5x5 smoothing pass around the vertex using the kernel in the header
  (`kAreaKernel`): raise bumps a cell by 1 when `cell < centre - k`, lower reduces by 1 when `cell > centre + k`
  (`FUN_00406f20` and `FUN_00406f90`).
* Sounds: raise 0xc7 and lower 0xc8 (`FUN_004481b0`), then the touched tiles are redrawn.
* Cost: no money is charged in these routines (placeholder: free). Mouse mapping (which button raises, which lowers)
  is not decoded; the exe reaches these routines from the key dispatch (placeholder: left raise, shift-left or right lower,
  as in docs/EDITING.md).
* **Analyze Golf Shot** (tool 3) only sets the tool; what the map click does with it is not decoded. The manual says it shows
  the paths golfers may take (MANUAL_NOTES). placeholder.

## 4. Buildings panel

Sheet `BuildingPanel`, background (216,482,584,118) at (216,482). exact. Nine lots, ids 6 to 14, in two rows of 5 and 4.
The Pathway, Benches, Flower Bed, Ball Washer, Landmark and Home Site items are NOT here: they are on the Amenities panel.
The Clubhouse is in no panel.

| lot | id | name | footprint | cost (units) | hover text (3rd line) | hit centre (r40, y x2) | pad at | icon at | icon column |
|-----|----|------|-----------|--------------|-----------------------|------------------------|--------|---------|-------------|
| 0 | 6 | Putting Green | 3 | 100 | Helps imaginative golfers | 383,531 | 343,501 | 344,453 | 0 |
| 1 | 7 | Snack Bar | 2 | 150 | Feeds hungry golfers | 463,531 | 423,501 | 424,453 | 1 |
| 2 | 8 | Pro Shop | 2 | 200 | Improves accurate golfers | 543,531 | 503,501 | 504,453 | 2 |
| 3 | 9 | Swim Club | 3 | 300 | Golfers become members | 623,531 | 583,501 | 584,453 | 3 |
| 4 | 10 | Driving Range | 5 | 250 | Helps long hitters | 703,531 | 663,501 | 664,453 | 4 |
| 5 | 11 | Cart Garage | 2 | 400 | Golfers play faster | 423,571 | 384,542 | 384,493 | 5 |
| 6 | 12 | Marina | 2 | 1000 | Increases property values | 503,571 | 464,542 | 464,493 | 6 |
| 7 | 13 | Resort Hotel | 4 | 2500 | Golfers stay happier | 583,571 | 544,542 | 544,493 | 7 |
| 8 | 14 | Airstrip | 6 | 5000 | Higher greens fees | 663,571 | 624,542 | 624,493 | 8 |

Notes: Clubhouse (id 15): footprint 4, cost 200, no panel; its picture is column 9 of the layout sheet. The hover texts
are the exe's own (the tooltip code labels each case). They correct one entry in PUBLISHER_EXE_NOTES: the Swim Club text is
"Golfers become members" (that note said better mood); Marina "Increases property values" and Resort Hotel "Golfers stay happier"
agree in substance. The Parkland picture in the Swim Club slot is a tennis court complex (the slot art follows the theme).

Pads (cuts on `BuildingPanel`): lots 0-4 use `0,0,74,64`, `100,0,77,64`, `200,0,77,64`; lots 5-8 use `0,100,77,58`,
`100,100,77,58`, `200,100,77,58`, giving states hover, selected (bright green) and blue. The panel code draws a pad only for
a buildable lot: hover cut while hovered, selected cut while it is the current tool (`DAT_004c2854 == id`). The blue cut is
never referenced; the pad of a non-buildable lot is just the panel background. The exe's draw also touches an uncut object
(index 3 of each pad group), which I treat as blank.

Lot pictures come from the theme layout sheet (`Parklands Layout`, `Desert Layout`, `Trop Layout`, `Links Layout`, chosen
by the theme index): 75x100 cells, column = lot index, four rows used. Selection rule (exact): the exe loads the four cuts of
each column in the order y=0, y=200, y=100, y=300 into consecutive objects and draws object `4*lot + 2*upgraded + buildable`.
So the picture y is `kIconRowY[2*upgraded + buildable] = {0, 200, 100, 300}`: grey level 0, colour level 0, grey upgraded,
colour upgraded. The picture is drawn 1 pixel higher while hovered and buildable. The remaining rows 4 to 7 of the sheet are
not loaded by the panel code.

Level and unlock (exact formulas; the meaning of two inputs is derived):

* `level = DAT_005a8c38[id]` (0, 1 or 2; the array is indexed by building id, `DAT_005a8c50` is the same array at id 6).
  Upgraded means `level != 0`.
* Buildable (colour picture, pad, clickable): `id < unlock - level && level < 2 && (level == 0 || DAT_005685f0 > 10)`.
  `unlock = DAT_005a6364`: 6 in a normal new game, 17 in sandbox. `DAT_005685f0` is the number of the next hole (the error
  text says upgraded buildings need "beyond 9 holes"). derived.
* Unlock schedule from PUBLISHER_EXE_NOTES (counter incremented when a hole opens; holes 6, 10 and 18 upgrade the course
  instead): Putting Green at hole 2, Snack Bar 3, Pro Shop 4, Swim Club 5, Driving Range 7, Cart Garage 8, Marina 9, Resort
  Hotel 11, Airstrip 12. Consistent with my reading of `FUN_0040e720`.

Click (`FUN_00434140`, exact): lot hit sets `DAT_004c2854 = id`. Then it refuses, with error sound 0x18 and a red message
popup, and clears the tool, when any of these hold, checked in this order:
1. `(level != 0 && FUN_0044faf0(DAT_005685f0 - 1) < 2) || level > 1`: if `level < 2` the text is "Sorry, upgraded buildings are
   not available until you build beyond 9 holes."; otherwise "You cannot upgrade this building any further."
2. `unlock <= id`: "This building is will become available as you build additional holes." (the typo is in the exe).
Otherwise the click arms the lot as the building tool and the map click places it.

Placement price (exact, `0x41f383` onwards): `price = table_cost * (level + 2) / 2 + extra`, where `extra` comes from the
placement scan (`FUN_0040db90`, terrain cost under the footprint; the footprint scanned is `footprint + level`).
So level 0 costs x1, level 1 x1.5 and level 2 x2. The affordability gate `FUN_00406c30(price)` passes when `price <= cash`,
or in sandbox, or while the next hole number is below 4 (the first three holes may overspend), otherwise it shows a "not
enough money" popup. The info box shows `table_cost * 100` dollars for level 0 and `table_cost * 300 / 2` dollars when
`level != 0` (the Snack Bar, id 7, always shows level 0 and never says "Upgraded").

Info box (exact): after 10 frames of hover on a lot, a framed 160x100 box at x = `clamp((lot%5)*80 + (lot > 4 ? 431 : 391),
80, 720) - 80`, y = 460 with centred lines at y 462 (`Upgraded ` + name), 472 (`Cost: N`, grouped, no money sign), 482 (the hover text). The
frame is `FUN_0040d0b0`, which draws 16x16 corner and edge tiles from the per-theme pop-up frame sheet `Pop_UpOk` (`frame` in
the header: row offset Parkland 0, Links 0, Tropical 100, Desert 200; eight pieces; alpha `Pop_UpOk_A`) over a translucent
fill (colour 0x80007fdc). I located this art through the title-screen loader (`FUN_0043cd70`).

Round buttons: to Elevation hit (236,526,r16) with hover cut (150,480,34,34) at (220,510); Undo hit (277,567,r12), hover cut
(40,550,34,40) at (261,553), armed (80,550,34,40) while undo mode is on, third (120,550,34,40) unused. The Elevation button is
checked first, then Undo, then the lots. Undo click: same help popup and `FUN_0045c0c0(0)` call as in the Elevation panel, without leaving the panel.

## 5. Amenities panel

Sheet `AmenitiesPanel`, background (214,482,586,118) at (214,482). exact. Slot ids are the hit-test results.

| slot | tooltip heading | tool id (DAT_004c2854) | hit (cx,cy,r) | draw at | hover | selected | disabled |
|------|-----------------|------------------------|---------------|---------|-------|----------|----------|
| 0 | Ballwasher | 3 Ball Washer | 302,563,20 | 274,536 | 0,0,60,56 | 100,0 | 200,0 |
| 1 | Pathway | 0 Pathway | 333,526,20 | 302,504 | 0,100,64,50 | 100,100 | 200,100 |
| 2 | Building Lot | 5 Home Site | 363,563,20 | 334,538 | 0,200,62,50 | 100,200 | 200,200 |
| 3 | Benches | 1 Benches | 470,575,18 | 449,553 | 300,0,51,47 | 400,0 | 500,0 |
| 4 | Landmarks | 4 Landmark | 591,575,18 | 571,553 | 300,100,51,45 | 400,100 | 500,100 |
| 5 | Flower Bed | 2 Flower Bed | 652,575,18 | 632,553 | 300,150,51,47 | 400,150 | 500,150 |
| 6 | Scenic Trees | 16 Willow Tree | 714,575,18 | 693,553 | 300,200,51,45 | 400,200 | 500,200 |
| 7 | Undo | none | 248,567,12 | 234,553 | 0,450,36,36 | 50,450 | 100,450 |
| 9 | Scenic Bridge | 19 Scenic Bridge | 531,580,12 | 510,553 | 300,50,51,45 | 400,50 | 500,50 |
| (8) | art only | none | none | 762,545 | 0,500,38,36 | 50,500 | 100,500 |

Cut sizes in the row are those of the hover cut; the three cuts of a slot are `x`, `x+100`, `x+200` (slots 3-9: 300, 400, 500).
Priority (later evaluation wins): 9 > 7 > 6 > 5 > 4 > 3 > 2 > 1 > 0 > back. Back button "Course Terrain": hit (269,500,r16), hover
cut (0,400,34,34) at (254,483).

Draw (exact): hovered slot's hover cut at its position; the current tool's selected cut (tool 3 -> slot 0, 0 -> 1, 5 -> 2, 1 -> 3,
4 -> 4, 2 -> 5, 16 or `DAT_004c2848 == 16` -> 6, 19 -> 9); the Home Site slot's third cut when `DAT_0056d1b0 < 1`.

Enable rules: the Home Site is usable only while `DAT_0056d1b0 >= 1`. That counter is, per frame, the number of qualifying
records in the table at 0x5849e0 (stride 0x2c, record valid and flag bits `& 7 > 2`) minus the number of Home Sites already
placed; what a record is (probably a hole or lot site) is not decoded. derived. The other six items have no enable rule in the
panel code (their unlocks are the unlock counter of section 4: Pathway, Benches and Flower Bed are in the initial six; Ball
Washer id 3, Landmark id 4 and Home Site id 5 too).

Click (`FUN_00434ac0`, exact): slot 0 sets tool 3; slot 1 tool 0; slot 2 tool 5 (or none when no home site is left); slot 3
tool 1 and design 0; slot 4 tool 4 with design -1 (pick from the strip); slot 5 tool 2 with a random design `rand(4)`; slot 6 tool
16 with `rand(7)`; slot 9 tool 19 with `rand(8)`; slot 7 shows the undo help text and switches to undo mode. Clicking elsewhere
on the panel while a variant tool is armed re-applies the stored design (`DAT_005a9f50`).

Variant strips (exact structure, art not mapped): benches 5 designs, landmarks 14 (only designs whose bit is set in
`DAT_00543cfc`; sandbox sets 0x3fff), scenic bridge 8, flower beds 15 (5 x 3), scenic trees 7. The strip background pieces,
anchors, entry layout (zigzag, 47 px pitch, ellipse hit, badge cuts) are in the header under `amenities`. The entry pictures are
object sprites of the per-theme building art, with ids base 0x208 benches, 0x168 landmarks, 0x226 bridges, 0x1a3 flower beds,
0x12f trees; which file each id lives in is not decoded.

Costs (building table, units): Pathway 1 per tile, Benches 2, Flower Bed 5, Ball Washer 50, Home Site 10, Scenic Bridge 100,
Willow Tree (scenic trees) 25. Landmarks cost `250 + 50 * design` units, 0 ("FREE!") when that design's bit is already set in
`DAT_00822c70`. Slot tooltip: two bars at (slot x + 20, slot y + 10 and + 20), the name and the cost in dollars as a plain number ("Scenic Bridge" / "10000" in footage), "5000-20,000" for Landmarks, both as wide as the name. The strip (landmark tooltips also show the design name uppercased and the effect
name from `design & 3`: 0 Happy Golfers, 1 No Dandelions, 2 Skill Upgrade, 3 Happy Endings). No gravel/paved distinction exists
in these routines (placeholder: the port's path kinds are its own).

## 6. Employee panel

Sheet `EmployeePanel`, background (215,474,585,126) at (215,474). exact. The slot list is the table at 0x4c7af0.

| slot | role | tooltip | hit (cx,cy,r) | draw at | cuts |
|------|------|---------|---------------|---------|------|
| 0 | Hire | Hire employees | 270,558,16 | 254,542 | hover 0,500,42,42; pressed 50,500; disabled (20 employees) 100,500; idle 50,550 |
| 1 | View | View employees | same as 0 (never reached) | 254,542 | 0,550 / 50,550 / 100,550 |
| 2 | scroll left | none | 321,549,r30, xs4 | 313,519 | hover 0,200,16,62; enabled look 100,200 (drawn only if scrolled) |
| 3 | scroll right | none | 604,549,r30, xs4 | 596,519 | hover 0,300,16,62; enabled look 100,300 (drawn only if more than 8) |
| 4 | Move | Move this employee | 647,513,16 | 631,497 | hover 400,0,52,48; armed 500,0; disabled 600,0; idle 700,0 |
| 5 | Fire | Fire this employee | 699,510,16 | 683,494 | y=50 row, same x pattern |
| 6 | Rename | Rename this employee | 756,513,16 | 740,497 | y=100 row |
| 7 | duplicate of 5 | none | same as 5 (never reached) | 683,494 | y=150 row |
| 8..15 | portraits | none | (sx+32, sy+16), r16 | (336,507),(336,550),(399,507),(399,550),(462,507),(462,550),(525,507),(525,550) | hover 0,y,63,43 (y=0 top row, 50 bottom row); selected 100,y; spare 200,y |

Slot centre rule: slots 0-1 and 4-7 have centre (sx+16, sy+16); portraits (sx+32, sy+16); arrows (sx+8, sy+30) with x scaled 4.
First slot match wins and beats the tabs. Tab buttons: "Golfers" tab hit (286,492,r16), hover cut (0,400,33,33) at (270,475),
returns to mode 2; "Gary Golf" (the player's name; default "Gary Golf") tab hit (256,510,r16), hover cut (0,450,33,33) at
(240,494), returns to mode 4. All exact.

Draw (exact): the three buttons Move, Fire, Rename draw their disabled cut (x=600) while no employee is selected; with a selection
they draw the idle cut (x=700) and the hover cut (x=400) while hovered; the Move button also draws its armed cut (500,0) while move
mode is on for that employee (`DAT_004c2e10`). The selected portrait draws its selected cut. Each portrait shows the employee's
animated sprite at (slot x + 30, slot y + 24); the sprite sets are the per-theme staff sprites (not mapped).

Roster (exact): up to 20 employees (`kMaxEmployees`); the page shows 8 (4 columns of 2), the scroll arrows move 2 (one column)
at a time. The list is the employee records at 0x585850, stride 0x4c, 64 records, active when the byte at +0x12 is set and the
type byte at +0x13 is negative (not -6). A selected employee shows text centred at x=700: name (y=534), "Hired: " plus the hire date (546, the short at +0x22
holds the date), "Paid: " plus the total paid in dollars (558, the short at +0x24 is the sum of wages paid, in units), the
kind's counter label (572) and its value in decimal (584, the short at +0x26). Counter labels are in the header per kind.

Hiring (exact): the Hire click sets `DAT_0059ca54 = 0`; the next draw opens the modal `FUN_00459400` ("HIRE AN EMPLOYEE", title at
(377,107), 8 choices as in the header). With 20 employees the click gives the error sound 0x18 instead. The modal's choice is
`kind*3 + 1 + skilled`; `FUN_0040aa80` then creates the record (`FUN_00402970(-2 - kind)`), refuses a skilled hire with the text in
`kSkilledRefusal` while `FUN_0044faf0(DAT_005685f0 - 1) < 1` (next hole number 6 or less), places the new employee on a random tile within 3 of a fixed course spot (`DAT_00578150` and `DAT_00578154`, probably the
clubhouse; or next to the building at `DAT_00575cc0` when `DAT_00575cb8` is set), and sets the skilled flag (bit 8 at +0x12). A hiring fee: none found (`FUN_00406c30(0)` is only an affordability check; placeholder 0).
The hire dialog shows each choice as "Name: $N per week" with `N = wage * 100`.

Employee kinds, record type, wages (exact, units of $100, table at 0x4c2e2c indexed `2*kind + skilled`):

| kind | record type | skilled name | wage regular | wage skilled | dialog blurb | counters (regular / skilled) |
|------|-------------|--------------|--------------|--------------|--------------|-------------------------------|
| Club Pro | -2 | Celebrity | 3 ($300) | 7 ($700) | Greeters... | Players greeted: / Players cheered: |
| Ranger | -3 | Marshall | 2 ($200) | 3 ($300) | Speed up play... | Players rushed: / Slackers intimidated: |
| Groundskeeper | -4 | Technician | 2 ($200) | 4 ($400) | Weed Killers... | Weeds destroyed: / Weeds eradicated: |
| Soda Vendor | -5 | Refresher | 2 ($200) | 5 ($500) | Thirst quenchers... | Beverages served: / Satisfied customers: |

Wage payment (exact formula, derived meaning of the inputs): the per-period money routine inside the main tick function runs when
`ticks % (1024 / (d + 2)) == 0` with `d = DAT_00822c88` (probably the difficulty, 0 to 3; a month is 1024 ticks). For each active
employee it pays `wage` when `rand(4 - d) <= tier`, with `tier = FUN_0044faf0(DAT_005685f0 - 1)` (0 for next hole number up to 6,
1 up to 10, 2 up to 18). The payment is subtracted from cash and added to the employee's "Paid" total with a floating "-$" text. Because
of the random gate the mean payment rate is below the "per week" the dialog states; the port may pay the full wage per game week
as a simplification (placeholder).

Fire (exact): the Fire click, with an employee selected, charges 25 units ($2,500) (`DAT_00571fd4 -= 25`, also booked in the
month ledger), clears the record's active flag, shows a floating "-$" text there and clears move mode. Rename (exact): opens a text
prompt titled "Rename Employee..." with a length argument of 31.
Move (exact): stores the employee's record index in `DAT_004c2e10`; the next map click moves that employee (the target
handling is in the giant map function, not decoded).

## 7. What is placeholder, in one list

* Hover tint table (`0x519fd8`), mouse mapping of raise and lower, elevation cost, Analyze Golf Shot behaviour.
* The meaning of the byte at `0x571ff6 + 46*DAT_0059bf90` that picks the lower clamp, of `DAT_0056d1b0`'s source records, and of
  `DAT_00822c88` (called difficulty).
* Path kinds (the exe has a single Pathway), variant art ids of the amenity strips, and the staff and building sprite files.
* Employee wage frequency in game time; hiring fee (none found); the tooltip bar's exact fill alpha.
* The blue pad cut and the third cuts that the draw code never references (the header marks them as spare).

## 8. Appendix: Member and Player (JoeCool) panel cuts

Not part of this task, listed because the loader pass produced them (sheet rectangles, in loader order).

MemberPanel (alpha: background only): background (215,474,585,126); round buttons (0,400,33,33), (0,450,33,33); (0,0,121,44);
ten 16x16 pieces at x = 594, 578, ..., 450 along y=100; two 7x7 pieces (450,220), (450,230); (0,150,36,13), (0,250,36,13);
(0,300,42,42), (50,300,42,42).

JoeCoolPanel (alpha: background (214,474,586,126) plus (0,0,80,50), (0,100,80,50)): background (214,474,586,126); (297,300,503,122);
four 42x44 (x = 0, 50, 100, 150 at y=300); 38x32, 38x30, 38x32 columns at y = 500, 535, 565 for x = 0, 50, 100, 150; five rows of 80x50 at
y = 0, 50, 100, 150, 200 for x = 0, 100, 200, 300 (the first column is shifted down by 1 pixel, y = 1, 51, ...); round buttons
(0,400,33,33), (0,450,33,33).
