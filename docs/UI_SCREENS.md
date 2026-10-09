# Information screens: layouts and data fields

Companion header: `include/sg/ui_screens.h` (namespace `sg::ui_screens`). Everything here was read from the publisher exe
(`golf_pub.exe`) through the existing decompile, the disassembly and a small call tracer (a script that lists the sprite blit and text
calls of one routine with their literal arguments). No art is copied; the port loads the sheets from the disc folder.

Confidence marks: **exact** = literal arguments read from the disassembly; **derived** = follows from code but a part is inferred;
**unknown** = not decoded.

## 0. Conventions shared by all screens (exact)

* Every screen routine first calls a fade/dim fill on the main surface (strength float 0.25, `FUN_00474440(...,0x3e800000,...)`),
  then blits its opaque panel pieces on top, so the course is still faintly visible around panels that do not cover the 800x600 area.
  The pieces are cut from the sheet and blitted at the same x,y they have on the sheet (cut x,y == destination x,y) unless noted. Each
  colour sheet `X.pcx` has an alpha sheet (`X_alpha.pcx`, SGA uses `SGAreport_alpha.pcx`) cut identically.
* Text call shapes: left, centre and right aligned at (x, y) where y is the text baseline-ish row used by the exe; the port should treat
  y as the vertical centre of a 12 to 13 px line (the course report art rows of 17 px confirm this). Colour is set by the call
  `colour(c, 0xffffffff, 2, 2)`: the 4 arguments are text colour, a second colour (white, unused look), and a 2 px shadow offset.
  Colour words are 15 bit RGB 5-5-5 values with the high half `0x8000` as a flag (checked: `0x7d08` is red in 5-5-5, which fits
  "- not acceptable -"): `0x80000000` black text (default), `0x80007d08` red (about RGB 255,66,66), `0x80006000` dark red (198,0,0, negative
  money), `0x80007fff` white, `0x800023e8` bright green, `0x80001284` dark green. Take 5 bit channels times 8.
* Fonts: three font objects: `0x821ee8` body (nearly every cell), `0x821020` large title face, `0x821f28` a third face used for the SGA grade.
  The exe string table holds "Manual SSi" and "Klepto ITC TT"; which one is which is not decoded.
* Money text: `FUN_0042dc00(units * 100)` builds a decimal text with a leading `-` for negatives and `,` thousands separators in a
  shared buffer; there is no currency sign (the Financial Report labels the Total row "Total (section sign)"). A ledger value
  of 0 is replaced by a placeholder text from `0x4e9a84` (a short filler; it is in the data section, content not read).
* Close button: the round "OK" tick is cut from `OkStates.pcx` (see section 1). Each screen routine ends in a modal loop: poll,
  draw the hover cut when the pointer is inside the button rectangle, and **close on any mouse click anywhere** (`FUN_0045ae70`
  returns 1 when a mouse button flag is set) or when the global quit/key flag `DAT_00822d68` is set. The tick is only a hover cue.
* OkStates cuts (loader at 0x44bee0, 44x42 cells with a 1 px margin): index 0 (1,1) normal blue, 1 (46,1) hover yellow, 2 (91,1) grey,
  3 (1,1) duplicate of 0 (the loop's y test sends index 3 to row 1), 4 (46,44) pressed yellow. Buttons below use index 0 and 1.

## 1. Financial Report (exact; data source mapping partly derived)

* Opens: Information menu "Financial Report" or its shortcut. Routine `FUN_0044f6b0`.
* Art: `infoscreens/FINANCEreport.pcx` (+ `FINANCEreport_alpha.pcx`), one piece (8,7,784,289) blitted at (8,7). The art has a pale title
  pill, a left label column of nine rows, eight year columns (header pill plus nine value rows each) and a lower total row.
* Title "FINANCIAL REPORT": large face, left aligned at (218,21).
* Row labels (centred at x 98): y = 87 + 17 * i for i 0..7, the ninth row (Total) at y 230 (the extra 7 px gap before the total row). Texts from
  the pointer table at 0x4d2054: Greens Fees, Home Sites, Food/Drink, Build course, Facilities, Salaries, Maint./Interest, Other,
  Total (section sign).
* Year columns: column c (0 based) has header text (the year, 2001 + year index, drawn through the generic number function
  `FUN_004ad425(index + 0x7d1,...)`) centred at x = 222 + 75 c, y 60. Values are **right aligned** at x = 256 + 75 c, y = 86 + 17 r for r 0..7 and the
  total at y 229 (step 24 before the last row). Value colour: black, or dark red `0x80006000` if the cell is negative.
* Which years: `n = clamp(yearIndex, 0, 99)` (`DAT_005a6d3c`, whole years since start); first column year `max(0, n - 8)`. The loop draws years
  first..n inclusive, so once n >= 8 it addresses 9 columns; the ninth (x 822) falls outside the art and the screen, so only eight are
  visible. Treat as a quirk: either port faithfully (the newest year disappears after year 8) or draw the last eight years. derived.
* Data: array at 0x584210, 100 records (index = year mod 100) of 10 shorts (0x14 bytes), values in cash units (shown x 100).
  Short k is row k for k 0..7; short 8 (the Total) is **recomputed on draw** as the sum of shorts 0..7 and written back; short 9 is unused.
  Writers found (exact): k0 += every greens fee paid by a golfer (the amount added to cash on hole completion); k1 -= the cost of a building removal
  job of type 5 (derived: probably the home site / lot job); k2 += snack-bar and similar purchases (+2, +5 and tier amounts);
  k3 += terrain tile refund or cost entries (`DAT_0059c090` per tile: positive on removal refund, negative on build); k5 salaries;
  k7 gets tournament prizes (`+prize*10`), shows wagers and other income. **unknown:** the writers of k4 (Facilities) and k6 (Maint./Interest) were
  not found as plain stores (they are probably reached through an alias or a computed index); the port should map its own upkeep, facility
  and interest figures to them.
* Close: button hit rectangle x 726..769, y 249..292 (draw OkStates cut 1 on hover, 0 otherwise, at (726,249)); any click closes.
* Recipe: blit the art; draw title; for each of the 9 label rows draw centred text; for each visible year draw header and the nine
  values using the sign rule; draw OK cut at (726,249); on click return to the game.

## 2. Membership Roster (exact for layout, derived for field meaning)

* Opens: Information menu "Membership Roster". Routine `FUN_00454c50`. Modal; same close rule as section 0, except clicks on the scroll bar column.
* Art: `infoscreens/memberRoster.pcx` + `_alpha` (full 800x600 piece at 0,0), `memberRoster_buttons.pcx` (300x127), `memberRoster_scrollbar.pcx` (18x447).
  The body art already holds: the title pill, header pills, 22 name rows (20 px pitch), the fixed-width columns, an 18 column by 22 row grid of
  checker cells (21 px pitch) and the legend bar with six baked icons (navy ball, silver ball, gold ball, red ball, camera, heart).
* Title "Membership Roster": large face, left at (212,19).
* Header labels at y 60: "Member" left at x 33, "Low" centred 170, "Hcp" centred 209, "Rnds" centred 249, "Status" centred 320, then the hole numbers 1 to 18 centred in
  19 px cells starting x 381 with 21 px pitch (the exe uses its width-limited text call at x = 381 + 21 (h-1), y 60, width 19).
* Rows: 22 visible rows, y = 89 + 20 r (r 0..21; 89 is the text row, icons use y - 4). Member list = every record in the golfer table whose
  existence short (+0x2a) is nonzero, **sorted by name ascending** (`strcmp` min search over names at 0x4d6098, stride 0x230, entry 0 is the player "Gary Golf" by default),
  skipping `scroll` rows first.
  * Name: left at x 28.
  * Tier ball: cut drawn at (122, y - 4) unless the status is "Visitor".
  * Low (byte +0 of the 0x2c byte record at 0x5849e0): centred at x 170. Hcp (signed byte +1): left at x 209. Rnds (short +0x2a, shows "-" filler if 0): left at x 249.
  * Status text centred at x 320 from the status code `flags & 7` (record byte +2): 1 Visitor, 2 Member, 3 Silver Member, 4 Gold Member, 0 nothing;
    and **Resigned** when the record's byte +0x29 is 0xff. (Platinum has no text here; the exe has only four.)
  * Hole grid, hole h (1..18): per hole flag byte at +2+h of the record. bit0 draws the camera icon (Photo Opp), bit1 the heart (Happy Ending),
    bit2 the red ball (Resigned at that hole), each at (381 + 21 (h-1), y - 4). Icon variants alternate by column parity to match the checker shade.
* Legend labels at y 536, centred: Member 82, Silver Member 208, Gold Member 326, Resigned 450, Photo Opp 573, Happy Ending 693.
* Buttons sheet cuts (all from `memberRoster_buttons.pcx`; loader at 0x44c48c, 19x18 unless noted): tier balls at (46,1) navy (Member), (66,1) silver, (86,1) gold, (106,1) red;
  camera (126,1), heart (146,1); the second variants of the same six at y 20 (same x), used on the other checker shade. Scroll arrow hover pieces (166,1,18,37) up and
  (185,1,18,37) down. Three extra small balls (204,1) (224,1) (244,1) are cut but not seen drawn. The big OK tick is the sheet's first 44x44 piece
  (the exe draws it from a separate `OkStates` style object at (732,548); hit x 732..775, y 548..591).
* Scroll bar (shown fully only when more than 22 members): track piece blitted at (767,80). Arrow hit zones x 767..784: up y 80..116, down y 490..526. Hover draws the 18x37
  piece at (767,80) or (767,490). A click on an arrow scrolls by `clamp(remaining, 1, 3)` rows, never past `n - 22`. The thumb is a flat bar
  x 773, w 6, y = 122 + scroll * 364 / n, h = min(364 - (thumb y offset), 8008 / n), colour `0x80007fff`.
* Extra code after the legend builds text strings (a "none" fallback and counts) into a buffer without drawing them. Treated as dead. unknown purpose.
* Recipe: blit art; labels; for r in rows draw name, ball, Low, Hcp, Rnds, status and the 18 icons; legend text; hover cut for OK and arrows; scroll thumb. Data from the port's
  `sg::Roster` (`include/sg/membership.h`): name, tier (Visitor/Member/Silver/Gold/resigned), rounds, best round and handicap are not tracked there yet and must be added; per hole
  flags need a small per member array that records photo ops, happy endings and resignation hole.

## 3. Hole Stats dialog (layout exact; field rules are in docs/DECODE_HOLE_STATS.md)

* Opens: click a hole in the Course Report (row hit) or from the course; routine `FUN_00453330(holeIndex)`. Closes on any click (OK hover cut only).
* Art: `infoscreens/HoleSTAT.pcx` + `_alpha`; three pieces: top (0,0,800,220) at (0,0); a comment row strip (0,282,800,16) drawn once per comment row at (0,y);
  bottom (0,347,800,55) drawn at the y after the last row. The dialog is only about 480 px wide inside the 800 wide strips (the art is centred, the strips are
  transparent outside x 160..640). Row height = 16 (strip height).
* Title (large face, centred at (385,48)): "HOLE STATS for " + hole number + name + " (" + number + ")". The text before the name is the exe string; the name is the hole record name.
* Left column: label left at x 190, value centred at x 356. Rows y 83 "Fun Factor" (value is `NNN%` plus the word label (poor/fair/good/very good/outstanding) in the same run),
  y 104 "Length", y 125 "Accuracy", y 146 "Imagination" (signed decimals plus word labels, same string style).
* Right column: label left at x 441. y 83 "Yards": value centred 536. y 104: **one rotating statistic** (Avg. Drive, Longest Drive, Fairways hit, Greens in Reg, Average Putts) value centred at
  x 591 (the label text carries the colon). y 125 "Par": value centred 536. y 146 "Stroke average": value centred 591.
* Histogram: heading "Average shots on this hole" left at (190,176). Six columns centred at x = 445 + 34 k (k 0..5); stroke label (number, last has the "and above" form) at y 168, count
  (sum over the eight skill groups of the histogram bin, drawn only when nonzero) at y 188. First stroke value = max(par - 2, 1). The pills in the art carry the shapes, no bars are drawn.
* "Comments" heading left at (190,202). Then up to five comment rows, strip piece at (0, y) for each, starting at y 220 and stepping 16: the text is centred at x 400:
  `NN% ` (count * 100 / rounds) followed by the comment sentence built from the event type and its stored location argument (`FUN_00469b00`). Colour: green family for a positive
  fun delta, red family for a negative one (exact selection reads a global at 0x58b198; words in section 0). If the hole is not open (par byte 0) the first row shows "Under Construction!"
  in blinking red (colour flips with `timeGetTime() & 0x200`).
* OK button: OkStates cut at (574, yAfterRows + 8), hit x 574..617, height 44; pointer inside draws the hover cut.
* Recipe: dim, blit the three art pieces with repeated strips, draw the 4+4 labelled values, histogram numbers and the first five non-zero event types ranked by count.
  The rules (word label thresholds, rotating stat choice) are in DECODE_HOLE_STATS.md section "Hole Stats dialog".

## 4. SGA Evaluation report (exact layout; scoring in docs/PUBLISHER_EXE_NOTES.md and include/sg/sga.h)

* Same routine as the Course Report: `FUN_0044fb30(mode)`; mode 0 is the Course Report, mode 1 the SGA report (waits for a click), mode 2 the SGA report drawn for the tournament offer
  without waiting (the caller shows the offer popup on top).
* Art: `infoscreens/SGA.pcx` piece (35,26,730,419) + alpha `SGAreport_alpha.pcx`, blitted at (35,26). The art holds the title pill, three header pills (Selection Criteria, Grade,
  Ideal/Minimum), ten labelled criterion pills with their value pills, a ten row table (two columns split near x 476), a big text area with the SGA ball logo at lower left, and a baked OK tick.
* Title "REPORT of the SIM GOLF ASSOCIATION": large face, left at (190,42).
* Header texts at y 77: "Selection Criteria: " + course class name (class table at 0x4c2a18: Municipal, ...) left at x 75; "Grade: " + class grade letter-word left at x 306 (the grade comes from
  `FUN_0044faf0(holes - 1)`: 0 under 6 holes, 1 under 10, 2 under 18, else 3 or none); "Ideal  (Minimum)" left at x 497.
* Ten criteria rows at y = 104 + 17 i (i 0..9): label centred at x 119 in this order: Length of Course, Number of Holes, Time to Play, Fun Factor, Holes with Variety, Scenic Holes,
  Length Holes, Accuracy Holes, Imagination Holes, Facilities on Site. Each row: actual value centred at x 238 (Length shows `NNN yds`), a row of **score pips** at x = 303 + 14 j for j < score (score 0..10,
  pip sprite object 0x59b33c, not a sheet cut; the port should draw a small tick or dot), and when the score is 0 the red text "- not acceptable -" centred at x 386 instead. The ideal/minimum text is left at x 490,
  e.g. "4 hours or less  (max: 5 hrs)" for Time to Play and "100%+" for Fun Factor.
* Bottom block: "Committee recommendation" left at (174,288). If the total is 0 or less: "0/100" left at (370,80) and "Improvement Required." (third face) centred at (400,310). Otherwise: total right aligned at (396,77), tournament
  name centred at (400,310) in the large face (names by total, see PUBLISHER_EXE_NOTES), and "NNN,000 first prize." centred at (400,338) in the third face.
* OK button: mode 1 at (701,398) (hit 44x44), cut from OkStates, hover cut when inside; mode 2 draws it and returns.
* Side effect: passing the evaluation sets game flags `0x402000` (tournament pending); the port already handles this in sga.h.
* Recipe: blit art; header texts; for each criterion compute (actual, score) from `sg::evaluateCourse`; pips; ideal text; recommendation block. sgview's `drawSga` should be replaced by this layout.

## 5. Keyboard shortcuts screen (exact)

* Opens: F8 (Information menu "Handy keyboard commands"). Routine `FUN_0044e770`. Art `infoscreens/shortcuts.pcx` + `_alpha`, piece (0,0,800,409) at (0,0) (the lower 191 px stay the game).
  Title "KEYBOARD SHORTCUTS": large face centred at (287,28).
* Left list: key text centred at x 68, action text left at x 108, y = 81 + 17 i. Right list: key centred x 444, action left x 483, same y values. The F8 row is drawn in a highlight colour.
  The last left rows are spaced differently (Tab at y 319, Esc at y 343).
* OK button at (710,362), hover cut; any click closes.
* Complete list (keys exactly as shown; actions paraphrased):

| Key | Action |
|-----|--------|
| F1 | Course status report |
| F2 | Player comments report |
| F3 | Histograph |
| F4 | Financial report |
| F5 | Course overview map (routing, value, aura, employees) |
| F6 | World map |
| F7 | SGA evaluation |
| F8 | This list of shortcuts |
| F9 | Membership roster |
| F10 | Professional accomplishments |
| ? | Show the last message again |
| shift+b | Sell a building lot |
| shift+r | Routing, aura and lot value screens |
| shift+w | World map, switch course |
| Tab | Rotate the building or tree before placing it |
| Esc | Quit the game |
| g | Green and tee tool |
| f | Fairway tool |
| r | Rough tool |
| s | Sand trap tool |
| t | Trees tool |
| w | Water tool |
| p | Path tool |
| b | Benches tool |
| z | Zoom in |
| x | Zoom out |
| shift+p | Pause or resume |
| shift+t | Hide or show trees |
| shift+n | Toggle name labels |
| e | Toggle elevation mode |
| / | Instant shot analysis |

## 6. Tournament screens (layout derived from literal calls; data rules in docs/DECODE_TOURNAMENTS.md)

Two routines share `FUN_0045a090` and the results routine that follows it.

* **Leader board overlay** (during play, top left): a translucent frame box `FUN_0040cef0(0, 8, 144, rows * 22 + 16)` and pale yellow text (`0x80007ff0`) left aligned at x 72:
  line 1 y 9 "LEADER BOARD of the" (SGA tier names: Qualifying School, Jr. Championship, Tour Championship, Open Championship) or "LEADER BOARD of" + year + " Open" style, line 2 y 21
  the year/name, line 3 y 33 the third part. Then one 20 px row per ranked golfer (`DAT_005685f0 * 20 - 20` is the scroll limit). exact for lines, derived for the rows.
* **Results screen**: art `infoscreens/tournament result.pcx` + `_alpha`, flag 1 pieces: header (0,0,800,106); row pieces (0,125,800,22), (0,180,800,26) (a taller row with name pill and prize pill),
  (0,224,800,18) (a thin row, no prize), end pieces (0,274,800,51) and (0,357,800,51); all blitted at (0, y) with the pieces chosen per rank row. Title "TOURNAMENT RESULTS" centred at (320,16)
  in the title face. Tournament name centred at (175,50). Column heads at y 50: hole numbers (18 cells, 27 px pitch, first cell left edge 163, so centres 176 + 27 k), a "F" (final) head centred at 667 and "Prize" centred at 700.
  Rows start at y 77. Name left in the pill (x 14..158), per hole strokes in the 27 px cells, total score centred at 669, prize in the right pill (x 690..790, only top ranks). Text colour
  `0x80000018` (near black) in the table, `0x80001284` dark green for the player's own row. OK tick at (732, yBelowLastRow) with the OkStates hover cut.
* "TOURNAMENT SCORES" (the title string beside it) is the in-progress variant of the same screen. Pair selection: art `PairBase` + `PairButtons`, heading "SELECT THE NEXT PAIR OF GOLFERS" at (338,14);
  Prize and Ranking strings exist but the pair selection layout is **unknown** (only the heading and the OK at 553,422 style calls were traced).
* Also drawn when a tournament ends: a lowscore panel (`lowscore.pcx`, cuts 195,45,411x79 / 195,224,411x17 / 195,321,411x61) used for the "Best Scores" screen. Fields not traced: **unknown**.

## 7. End of year screen (layout exact; wording derived)

* Opens: automatically when `ticks % 8192 == 0` and no other screen is up, routine `FUN_0044cff0`. Art `endoyear.pcx` + alpha: top piece (187,40,429,176) at (187,40); highlight strip piece (187,291,429,15) repeated per
  highlight line at (187, y); bottom piece (187,374,429,55) at (187, y after the last line). The panel is 429 px wide, centred.
* Title "END of  YEAR: " + (year counter + 2000): large face centred at (406,55).
* Column heads at y 93, centred: "This Year" at 465, "Last Year" at 566.
* Four summary rows at y 115, 135, 155, 175. The label is a sentence centred at x 301 built from a pattern plus the verb "increased", "not changed" or "decreased" (chosen by comparing this and last year):
  "Cash reserves have ...", "Your fun rating has ...", "Your skill rating has ...", "Your membership has ...". Values right aligned: this year at x 500 (dark green `0x80001284`), last year at x 601 (black).
  Sources: cash per year short array at 0x51b388 (money formatted, x 100), fun rating 0x53e63c, skill rating (next array), membership 0x520640, all indexed by year.
* "Highlights" heading centred (404,200). Then one centred line per notable event of the year (x 404, a strip piece behind each, step about 15; unknown first y, probably 217) in black (a red `0x80007d08` for bad news).
  Event kinds with text patterns: "<name> buys a home", "Hole N opened", "<pathway> built", "Won match vs <name>", "Hole N rated top 100", "Hole N rated top 18", "<name> places N in tournament",
  "<name> joins the board", "Happy Ending", "Additional land purchased", "Ivana donates a ...". Month names from the month table at 0x4c2908 prefix lines.
* After the highlights, when cash is negative and the board flag is clear the board message appears (strings in the exe: two years to recover, board concerned, one more year, very worried,
  unable to make a profit, contract terminated). The port already has these rules.
* OK tick at (550, y of bottom piece), hover cut, any click closes.

## 8. Player Comments report (layout exact; PlayComt)

* Opens: F2 / Information menu "Player Comments". Routine `FUN_004546b0`. Art `PlayComt.pcx` + `_alpha`: top (148,45,505,102), row strip (148,224,505,17), bottom (148,321,505,61), the panel is 505 px wide, centred.
* Title "PLAYER COMMENTS REPORT" large face centred at (389,61). Heads at y 96: "Comments" left at 182, "Hole" centred at 504, "Frequency" centred at 598.
* Rows: up to 20, strip piece at (148, 124 + 15 r), text y = 126 + 15 r. The comment text left at x 182, the hole name or number centred at x 504, the frequency (percent) centred at x 598. Rows are the 20 most
  frequent comment event types across all holes (sum over holes of the per hole event counts at 0x575d90, 0x82 ints stride), highest first; the hole shown is the hole with the most of that type; the text comes from
  `FUN_00469b00(eventType, location, ...)`. Row text colour: red for bad comments (`0x80007d08`), dark green `0x80001284` for good ones (a bright green word is replaced by the dark one), otherwise black.
  Stops when the count is 0 or nothing was recorded.
* Bottom strip at the y after the last row; OK tick at (591, that y), cut from OkStates; any click closes.

## 9. Course overview screens (F5 / shift+r: Routing, Value, Aura, Employees)

* Routine `FUN_00456be0`; it is also the backdrop of the Buy Land screen. Art: header strips `route screens_course/_value/_aura/_employ.pcx` (800x253 each, blitted at (0,0), one per mode) and
  `route screens_bottom.pcx` whose lower piece (0,253,800,347) is blitted at (0,253). The black diamond is the window for the course map: the map tile (tx,ty) is drawn through the exe's iso transform
  `FUN_00456b70` after `x = tx * 8 + 0x184, y = ty * 8 + 0x68` (Buy Land uses 0x180 and 100); the map shows the whole property scaled 1 tile = 8 px.
* The four round mode buttons are cuts of the **upper part of the bottom sheet** (table at 0x4ba698..0x4ba728). Blue (idle) and yellow (selected) pieces, destination = sheet position of the lower piece:
  groundskeeper (employees) idle (0,0,129,113) selected (0,114,129,113) at (60,313); dollar house (home site value) idle (130,0,130,113) selected (130,114,130,113) at (612,313);
  flag with path (routing) idle (261,0,134,117) selected (261,118,134,117) at (182,254); clover (aura) idle (396,0,139,117) selected (396,118,139,117) at (483,254).
  Draw the idle cut over the baked art only for the unselected buttons; the selected mode draws its yellow cut. Hover state not separate.
* Other cuts: big OK tick (536,0,65,65) drawn at (71,533) (cut 8); scroll arrows up (713,1,18,37) and down (732,1,18,37); scroll track (751,0,18,178). Arrows at x 775: up (775,64), down (775,205) (hover cuts), track at (775,64).
* Titles (title face, centred x 400): ROUTING MAP (400,15); mode sub titles: "COURSE ROUTING" or "AURA" (400,85); "HOME SITE VALUE" or "COURSE AURA" (400,188) (the lower label).
  Instruction lines centred (400,108) "Left click to select hole." and (400,126) "Right click to swap holes." (routing mode).
* Routing list heads at y 85: left list "Hole #" at 71, "PAR" 132, "YDS" 182, "Time" 239; right list "Hole #" 564, "PAR" 630, "YDS" 680, "Time" 737. Rows: 17 px stripes below, nine holes per list (derived).
* Aura and Value modes: the explanatory texts exist ("Course AURA indicates where ...", "Things which INCREASE home value: Close to water and trees / a fun golf hole / a top 100 or top 18 hole",
  "Things which DECREASE home value: Close to another building / an unfun hole / too close to green, fairway or OB / far from the golf course") shown in the centre box; positions not traced (unknown).
  Employees mode lists the staff counters (Greeters, Players greeted, Players cheered, Players rushed, Slackers intimidated, Weeds destroyed, Weeds eradicated, Beverages served, Satisfied customers and "paid:" wage text).
* Recipe: map renderer already in the port (draw the course top view into the diamond), then art + buttons; click on a button changes mode; a click on a hole row selects it; right click swaps two holes in the play order
  (this is the exe's way of renumbering holes).

## 10. Buy Land screen (layout exact for art, text fields derived)

* Routine `FUN_004587a0`. Art `infoscreens/buy_land.pcx` (full 800x600 piece at (0,0), no alpha) drawn over the overview map backdrop; `buy_land_buttons.pcx` (700x400) for the pieces.
* Cuts of the buttons sheet (loader at 0x44c8e4): 18 numbered ball pieces 59x61 at x = 1 + 60 (n % 9), y = 1 (yellow, row one) and y = 64 (silver, row two), n 0..8 for tracts 1..9
  (loader index i: i 0..8 yellow, 9..17 silver); yellow diamond tile highlight (1,127,192,95); OK tick (194,127,64,64); compass (259,127,72,61).
* Tract bars, three columns by three rows (art): bar left edges x 38, 295, 560; rows y 53, 121, 189 (pitch 68). Tract n (0..8) is column n / 3, row n % 3. The ball sits at the idle table position
  (15|272|538, 54|122|190); the hover ball (highlighted piece) at the selected table (x 112..496, y 296..488) in the iso diamond: the exe shows the highlighted number on the map tile instead.
* Text: tract text left aligned at (x0 + 4, y0 + 2) with x0 = 78 + 256 * column (`0x4e + 0x100*col`, third column 600) and y0 = 62 + 68 * row; lines such as "Buy tract #N", "Price: <money>", "<n> acres" or "Already purchased." in dark blue (`0x80002108`).
  Price = scan of the 16x16 tile block of that tract (`0x14` tiles) times a base from `DAT_0053a450` shift; not fully decoded.
* Title "TRACTS FOR SALE" centred (400,16). "Cash Reserve" label near (548,263), cash amount centred at (720,264) inside the two small pills of the art at (485..650,262) and (665..775,262).
* Map: tract overlay numbers drawn on the iso map at the tract centre with the third font. Hit test: click a bar or a tract on the map; buying deducts price from cash, marks tract owned and plays the usual purchase sounds (not traced).
* OK tick at (662,533); compass button at (71,533) (hover cut). The text " I don't think I'll buy any land." is the decline line shown when no tract is chosen.

## 11. Hire dialog

Already decoded in `docs/UI_PANELS.md` section 6 (title "HIRE AN EMPLOYEE" at (377,107), choice lines at x 246, highlight bar, animated 40x40 portraits at x 576 and y 192, 262, 332, 402). Extra facts from the loader and the trace:
art `infoscreens/hire.pcx` + `_alpha` as a full 800x600 piece at (0,0); OK tick (OkStates) at (553,422) with hover cut; the four band headings are "Thirst quenchers...", "Weed Killers...", "Speed up play...", "Greeters..." and each line ends with " per week".

## 12. Other pieces seen (not needed first)

* Histograph (`histograph.pcx`, full 800x600 piece, F3): title "HISTOGRAPH", texts "Satisfied customers" etc. use the Employees counters. Layout not traced.
* `general_selectionBOX`: a generic choice box, unknown use (Information menu choice dialog).
* Pair selection (`PairBase`, `PairButtons` cuts (0,0,329,136), (0,136,329,136), (0,272,329,136) and a 75x75 piece at (693,502)): match play opponent choice.

## 13. Unknowns, in one list

* Writers of Financial Report rows Facilities and Maint./Interest; what exactly fills the ninth year column.
* Which of the two typefaces ("Manual SSi" and "Klepto ITC TT") is the title face and which the body face.
* Roster: rounds, low score, handicap fields' writers (record bytes +0, +1, short +0x2a); per hole flag bits' writers.
* Hole Stats: exact colour selection rule of the comment lines; first comment row y (taken as 220).
* SGA: the pip sprite (object 0x59b33c) art; exact y of total text relative to art.
* Tournament results row selection logic, leader board row layout; pair selection layout; best scores (`lowscore`) screen.
* End of year: first highlight y and step, summary arrays other than cash.
* Buy Land: price formula, text lines per bar, map overlay detail. Course overview: Aura and Value text positions, routing row y positions.
* Comments screen top 20 rows vs 15 px stripe overlap (strips are 17 px tall on a 15 px pitch, so they overlap by 2 px).

## 14. What the lead must wire (summary)

1. A small screen framework: dim (0.25), blit sheet pieces at their own position, text helper with left/centre/right alignment and a 2 px shadow, a 5-5-5 colour word converter (`rgb555R/G/B`), an OK button helper (cut 0 idle, 1 hover), modal loop that closes on any click or Esc.
2. Data feeds the port does not have yet: yearly ledger with eight rows (Financial Report), per member low score, handicap, rounds, per hole flags and resignation hole (Roster),
   per hole event counts and location arguments (already in `holestats.h`) for Hole Stats and Player Comments, yearly series for cash, fun, skill, membership and a highlights log (End of Year).
3. Keyboard: F1 report, F2 comments, F3 histograph, F4 finance, F5 overview, F6 world map, F7 SGA, F8 shortcuts, F9 roster, F10 accomplishments (see the table in section 5).
4. Rows of the course report should link to the Hole Stats dialog (hole row click).
5. Fix the colour interpretation: colour words are 5-5-5, so `0x7d08` is red and `0x6000` dark red.
