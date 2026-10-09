# Interface art on the disc and what the exe does with it

Purpose: the port's menus are currently my own text lists. The original draws its panels from art on the disc (Interface/*.pcx, each with a matching _A alpha sheet) and places every piece at fixed pixel positions given in golf.exe. This file records what each sheet holds (seen by viewing it) and what the exe's loader does with it. Nothing here copies the art: the port loads it from the disc folder at run time.

Confidence marks: seen = I looked at the image; loader = read from the exe's sprite loader; guess = my reading, not yet confirmed in code.

## Sheets (all 800 x 600 unless noted, 256 colour with a colour key)
- 3mainLowerLeft: the lower left dock (already used by the port, positions measured).
- BaseTerrainPanel, ElevationPanel, BuildingPanel, AmenitiesPanel, EmployeePanel, MemberPanel, JoeCoolPanel (seen for the first three): the Build Course, Elevation, Building lot, Amenities, Hire and Member panels. Each shows a raised bar along the bottom with a checkered tray where the item buttons sit, round tab buttons at the left in three states, and (BuildingPanel) isometric slabs for the lot buttons. The per-theme button art is ParkLand/Links/Desert/TropicalTerrainButtons and Choose*Buttons (100 pixel strips, loader: cut as 200 pixel steps, 0xb6 by 0x49 and 0x49 tall pieces).
- TacksandArrow (loader): cut into 20 by 24 pixel cells on a 50 pixel pitch, 4 columns by 5 rows, giving 20 tack sprites (seen: five colours, grey, yellow, red, white and cyan, four frames each); two large pink arrows sit below them. A second sheet, _A, is cut the same way as the alpha.
- Flics/Bldgs/holemark.pcx (loader): numbered hole signposts 1 to 18, each with a green arrow, in three sizes: 17 by 14 pixels (9 per row) at the top, 18 by 17 at y 0x24, 19 by 21 at y 0x49 (seen). The three sizes are most likely zoom levels (guess).
- tacs&tees: the "Clubhouse Notes" sheet (seen): a yellow note, a photo frame and the list of 28 professional accomplishments in handwriting, plus check marks and six coloured push pins. The list is in docs/PUBLISHER_EXE_NOTES.md; it includes three goals not tracked yet: first dogleg right hole, first dogleg left hole and first par five hole.
- infoscreens/route screens_course, _value, _aura, _employ (800 by 253 strips) and route screens_bottom (seen): the course overview screen. The bottom sheet has a large diamond shaped window for the course map framed by four round mode buttons in blue and yellow (selected) states: a groundskeeper, a flag with a path, a clover and a dollar house. The four strips are the headers for the four modes (course, value, aura, employ). The tacks and hole signs are probably the map markers (guess).
- infoscreens: coursereport, HoleSTAT, FINANCEreport, histograph, lowscore, memberRoster (+ buttons, scrollbar), PlayComt (golfer comments), SGA, tournament result, buy_land (+ buttons), hire, ENDoYEAR, shortcuts, general_selectionBOX, OkStates, black. The port draws only the course report from this set so far.
- GolferStats, s_GolferStats, courseinfo, s_courseinfo, TransPopups, s_TransPopups, PopUpIcons, Pop_UpOk, InfoButtons, CGButtons (customise character), CustGolfBckgrnd, CustGlfBckMale, HeadBodyBck, HeadSelect, golfballhalopage_*, PairBase and PairButtons (choosing a pair of golfers for a match), Top10_*, TROPHYparts, TrophyMantle, bulletinboard&mantle*, WorldBase and WorldButton, Title* and Loading_Screens.

## What is not known yet (so not claimed)
- The exact pixel layout of each panel and which sprite goes in which slot: it is in the exe's per-panel draw routines (blit calls with literal coordinates, for example the buildings panel code near golf_decomp.c line 28630 steps buttons 0x50 pixels apart from (0x157, 0x1f5)).
- Which routine draws the tacks, hole signs and arrows, and what picks the colour and frame. The tack blit code near 0x470000 draws a tack of five colours at a screen point inside a 200 by 300 pixel region with three edge variants, so the tacks are probably map pins (guess).
- Whether a shot path is drawn on the course while a hole is being built. The preview in the port (dotted line and landing markers) is my own invention and should be replaced once the exe's behaviour is decoded.

## Plan
Rebuild each panel from its sheet with exe coordinates, one at a time: Build Course (terrain), Elevation, Building lots, Amenities, Employees, then the information screens, decoding each draw routine first.

## Build Course (terrain) panel, decoded and rebuilt

Facts read from the publisher exe (tables at 0x4c79a0 and 0x4c79b4, loader near golf_decomp line 37700):
* The panel body is the 584x118 piece of BaseTerrainPanel at its own sheet position (216,482). The round tab buttons on its left are part of that piece.
* 16 button slots. Ids in slot order: 0,1,7,4,9,10,17,2,3,5,8,11,12,13,14,15 (tile ids: tee, green, sand trap, rough, pot bunker, stream, water, fairway, firm fairway, deep rough, waste bunker, brush, rocks, three tree buttons).
* Slot positions: row one y 508 at x 270, 332, 394, 456, 518, 580, 642; row two y 546 at x 301, 363, 425, 487, 549, 611; trees at (704,477), (673,530), (735,511). Positions are top left corners (the sprite heights make the bottoms land on y 600).
* Sprite cell k of the theme sheet belongs to slot k (confirmed by looking at the art), four states in the order normal, hover, selected (gold outline), disabled.
* Tree buttons have three states only, at x = 520 + state*80, rows y 0 / 150 / 300; sizes depend on the theme.
* Tooltip: a 160x112 box at y 402, name from the per-theme tile table, then the cost per tile (cost byte times 100). The frame art for the box is not located; sgview draws a plain box.
* Per-theme names differ (Desert: "desert" for rough, "rough" for deep rough, cactus, joshua tree; Tropical: tropical bush and tree; Links: gorse). Links tree names are inferred.
Placeholders: the disabled state is never used (what disables a button is not decoded); the three tree buttons all paint our Woods tile (species choice not decoded); gravel/paved path, raise, lower and open-hole are temporary text chips until the Elevation panel is rebuilt.
