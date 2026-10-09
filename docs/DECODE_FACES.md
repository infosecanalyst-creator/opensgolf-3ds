# Decode: heads, faces, mood icons, portrait frames and where each is drawn

Source: the existing decompile text only (`spec/golf_decomp.c`, publisher exe). Pixel facts about the art were measured with PIL on the disc files (art only, no executable data read). Own words throughout; no game sentences are copied.

Tags: EXACT = read directly from the code (or measured on the art). DERIVED = follows from code plus a stated inference. UNKNOWN = not established from the decompile text.

Read this first:

1. EXACT: the only routine that draws a golfer head or face portrait is `FUN_0045c200(slot, x, y, mode)` (line 60259). It has four callers: the message popup (11548), the golfer info card (60509 and 61957) and the stats card (62988). The Player Comments report (F2, `FUN_004546b0`), the Membership Roster (F9, `FUN_00454c50`), the Top 10 screen (`FUN_00473470`), the Theme Packs screen, the Player panel (JoeCool) and the Employee panel contain no portrait or mood face draw.
2. EXACT: the small mood face (16x16, ten levels) is drawn in exactly one place, the Golfers dock panel (`FUN_00435760`, lines 29719 and 29882). It comes from `Interface/MemberPanel.pcx`, not from GBUBBLES.pcx.
3. EXACT: the port's current mood face cut in `tools/sgview.cpp` (line 3311, `450 + 16 * face`) has the order REVERSED. The sheet is cut right to left (see 1.5). Correct source x is `594 - 16 * face` with `face = clamp(mood + 2, 1, 10) - 1`.
4. DERIVED: the in-game portrait is a 140x140 head cell sitting on a 140x140 golf ball piece, both drawn at the same origin. Expression of the portrait comes from the golfer's newest reaction (happy, neutral, unhappy).

---

## 1. Head sheet cutting

### 1.1 Expression sheets `Heads/sim_MALE_all_expressionsflat.pcx` and `Heads/sim_FEMALE_all_expressionsflat.pcx` (EXACT)

Loader: `FUN_00442180` (line 39534). Female sheet loaded at 40287 with its cut loop 40288 to 40297; male sheet at 40299 with its loop 40300 to 40309. Each loop runs 19 times (`puVar10` steps by 0x84 bytes = 3 sprite records of 0x2c bytes, from 0x59e7ec to 0x59f1b8 for female and 0x59f23c to 0x59fc08 for male).

For head index `h` (0 to 18), with `col = h / 2` (integer) and `row = h & 1`:

| Item | Value | Tag |
|---|---|---|
| Sheet size | 1000 x 744, 8 bit paletted, transparent colour = palette index 255 (magenta) | EXACT (measured) |
| Block origin | x = col * 100, y = row * 0x174 (372) | EXACT |
| Cell size | 0x5a x 0x78 = 90 x 120 | EXACT |
| Three cells per head, cut in this order | at y = row*372 + 4, + 0x80 (128), + 0xfc (252), all at x = col*100 | EXACT |
| Columns, rows of blocks | 10 columns (100 px pitch) by 2 rows (372 px pitch): 20 block slots | EXACT |
| Heads present | blocks 0 to 18; block 19 (col 9, row 1) is empty on both sheets | EXACT (measured: 0 non transparent pixels) and not cut by the loop |
| Head order | head h sits in block (col = h/2, row = h&1): so even heads run along the top block row, odd heads along the lower block row | EXACT |
| Sprite order inside a head | sprite 0 = top cell = happy, sprite 1 = middle = neutral, sprite 2 = bottom = angry/unhappy | DERIVED (rows read off the art, and matched to the selector in 1.6) |
| Sprite record index | female sprite = (h * 3 + e) at record base 0x59e7ec; male base 0x59f23c = female base + 20 heads * 0x84. So the two genders form one array of 20 heads per gender | EXACT for bases; the index formula for the draw is DERIVED (the draw site's `this` pointer is not shown by the decompile) |
| Content size | a head fills about 62 to 76 px wide and 85 to 99 px high inside the cell, not centred (centre about 50 to 58 px from the left of the 90 px cell) | EXACT (measured) |
| Spill | hair of some heads spills into the 4 px gutters between cells (55 stray pixels on the male sheet, 1269 on the female sheet). The exe ignores them (it cuts exactly the rectangles above) | EXACT (measured) |

Male heads 0 to 18 are therefore "male head 0..18" in the order above; female likewise on its own sheet. Example check: male head 8 (col 4, row 0) is the spiky blond head; the default player file `Themes/Championship/Gary Golf.pro` has head byte 8, male, and its embedded portrait shows exactly that head (EXACT, measured).

### 1.2 Halo pages `Heads/golfballhalopage_male .pcx` and `Heads/golfballhalopage_female.pcx` (EXACT)

Loader lines 40264 (male, path under interface) and 40275 (female, path under heads); loops 40266 to 40273 and 40276 to 40285, 19 iterations each. The two files exist in both `Interface/` and `Heads/` with identical bytes (md5 equal, measured).

For head index `h` (0 to 18): `col = h / 2`, `row = h & 1`, block origin x = col * 0x8c (140), y = row * 0x1a4 (420). Three cells per head, each 0x8c x 0x8c = 140 x 140, at y = origin, origin + 0x8c (140), origin + 0x118 (280). Sheet 1400 x 840 (10 x 2 blocks of 140 x 420). Same head order and same expression order as 1.1 (rows: happy, neutral, angry). Transparent index 255. Block 19 is empty. All EXACT; the head to block mapping matches the expression sheet pixel for pixel in content (same artwork, different cell and padding).

Placement of the content: the head is centred in its 140 cell (content centre about (67 to 72, 66 to 70)). This matches the golf ball piece of 3.3 (ball centre about (70, 64) in its 140 cell). DERIVED: the halo cells are the ball portraits.

### 1.3 Sprite arrays for heads (EXACT addresses, DERIVED roles)

Each head is three sprite records of 0x2c bytes (stride 0x84 per head).

| Array | Base | Heads | Source |
|---|---|---|---|
| Halo, female | 0x562914 | 72 slots (indices 0 to 71) | stock heads 0 to 18, custom from 19 |
| Halo, male | 0x564e34 (= female + 72 * 0x84) | 72 slots | stock 0 to 18, custom from 19 |
| Expression, female | 0x59e7ec | 20 slots (0 to 18 cut) | sheet 1.1 |
| Expression, male | 0x59f23c | 20 slots (0 to 18 cut) | sheet 1.1 |

Evidence for the 72 slot halo arrays: the `.chr` export routine indexes `(head + (isMale ? 0x48 : 0)) * 0x84` from 0x562918 (lines 31776 to 31797, `FUN_00437910`), and 0x48 = 72.

### 1.4 Custom head portraits (the 140 x 420 files)

Format of a portrait file or the portrait embedded in a character file: PCX, 140 x 420, transparent index 255, three faces of 140 x 140 stacked top to bottom at y 0, 140, 280 (cut with the same 0x8c, 0x8c, 0x118 offsets as 1.2). Order top to bottom is happy, neutral, angry (measured on Bonus.chr, Gary Golf.pro, Kelley.chr, Gack.chr). All EXACT / measured.

* Loose files in `Heads/`: `FUN_00442180` lists `heads\*.pcx` (`FUN_0043d2a0`, lines 40311 and 40385). Names starting with F or f feed the female counter, M or m the male counter. The counter starts at 0x13 (19) and each file takes the next index; the loop stops when the counter reaches 0x48 (so the highest custom index is 0x47 = 71). Each file is cut into the halo array of its gender, 3 faces at (0,0), (0,0x8c), (0,0x118) (lines 40313 to 40383 female, 40386 to 40456 male). EXACT. On this disc that is `F_Head Template.pcx` (female 19) and the male files `M_Head Template`, `M_Lee`, `M_Meade`, `M_pixelmonkey`, `M_Sid Head`. `Head Template.pcx` (320 x 420) begins with H and is skipped. UNKNOWN: the order the OS returns names (the code does not sort), so which male file is 19, 20, ... is not fixed by the decompile. The port should key portraits by file, not by index.
* Embedded in character files: file layout (EXACT from `FUN_00437910` writer lines 31751 to 31771 and loader `FUN_00437fa0`): profile record 0x230 bytes, golfer record 0x4e2 bytes, 0x10 colour bytes, an 8 byte marker `*PCXFILE`, then the PCX. After loading, if the profile head byte is greater than 0x13 (so 20 or more) and the gender counter is below 0x48, the head byte is REPLACED by the counter value and the counter is incremented, and the embedded PCX is cut into the next halo slot (lines 32017 to 32025). So a file's own head byte of 20 or more only means "use my embedded portrait"; its numeric value (21, 26, 43 ...) is discarded. EXACT.
* Measured on the disc (profile +0x21 bit 7 = female, +0x22 = head byte): Standard theme `.glf` files carry stock heads and no embedded portrait; Firaxis `.chr` files carry head bytes 21 to 43 and an embedded portrait; see section 3.

### 1.5 Mood face strip: `Interface/MemberPanel.pcx` (EXACT)

Lines 41201 to 41206: after cutting a 121 x 44 card plate at (0,0) the loader cuts ten 16 x 16 pieces at y = 100, with x starting at 0x252 (594) and decreasing by 16 while x is above 0x1b2: x = 594, 578, 562, 546, 530, 514, 498, 482, 466, 450. They fill sprite slots in that order (slot 0 of this 17 piece set is the card plate, slots 1 to 10 are the faces, then two 7 x 7 hearts at (450,220) and (450,230), two 36 x 13 pieces at (0,150) and (0,250), and two 42 x 42 pieces at (0,300) and (50,300); the 17 count matches array 0x59b778, DERIVED).

So face slot k (1 to 10) lives at source x = 594 - 16 * (k - 1), y = 100, 16 x 16. Looking at the sheet left to right (x 450 to 594) the faces run from the broadest green grin down to a red devil, so:

| slot k | source x | face (measured by eye) |
|---|---|---|
| 10 | 450 | green, wide grin |
| 9 | 466 | green, big smile |
| 8 | 482 | green, smile |
| 7 | 498 | green, mild smile |
| 6 | 514 | yellow green |
| 5 | 530 | yellow, flat mouth |
| 4 | 546 | orange |
| 3 | 562 | red orange |
| 2 | 578 | red frown |
| 1 | 594 | red devil |

Selector (EXACT): `k = clamp(mood + 2, 1, 10)` where mood is the signed short at golfer record +0xa4 (0x57955c), which the reaction code keeps within -10 to 10. So every mood of -1 or lower shows slot 1, mood 0 slot 2, mood 3 slot 5, mood 8 or higher slot 10. A golfer arrives with mood 3 to 5 (slots 5 to 7).

`GBUBBLES.pcx` (640 x 480, EXACT): `FUN_00442180` line 40470 loads it and lines 40471 to 40477 cut 40 pieces of 16 x 16 at y = 0x144 (324), x = 16 * i, into array 0x59b050. On the art, pieces 0 to 4 are five round mood faces (devil, red frown, yellow flat, green smile, green grin), then round grey buttons (two arrows, plus, minus, a flag, a speech bubble, a chart line, a dollar sign), a small white bubble, a book, an "i", a "?" and so on. There is NO speech bubble frame art in this file. UNKNOWN: which code draws this array (the draw call's `this` is hidden). The in-world speech bubble is not art: see 2.6.

### 1.6 Which expression row (EXACT selector, DERIVED mapping to rows)

`FUN_004675d0(g)` (line 71026): take the 16 bit word at golfer record +0x88 (0x579540 + g * 0x100), the newest reaction in the history. If bit 0x8000 is set return 2. Otherwise return 1 when bit 0x4000 is clear and 0 when it is set. In other words: newest reaction positive = 0 (happy cell, top), newest reaction negative = 2 (angry cell, bottom), no signed reaction = 1 (neutral cell, middle). The portrait draw calls this together with the gender test (`FUN_0046c940`, male when bit 7 of profile byte +0x21 is clear) right before drawing, so the sprite is chosen by gender, head and this value (DERIVED, since the decompile drops the sprite pointer). A zeroed scratch slot (0x98, 0x99, 0x9c at start) has an all zero history word, so its row is 1 (neutral) (DERIVED).

---

## 2. Per screen usage

Positions are screen pixels in the 800 x 600 layout. "Ball" = the 140 x 140 golf ball piece of 3.3.

### 2.1 Table

| # | Screen | Routine and line | Art | Rectangle and position | Selection rule | Tag |
|---|---|---|---|---|---|---|
| 1 | Golfers dock panel, compact mode (panel size index below 4) | `FUN_00435760` 29607, face at 29719 | `MemberPanel.pcx` faces 16x16 (1.5) | face at (310 + 60 * col, 498 + 14 * row); name at (325 + 60 * col, 499 + 14 * row). Wrap test `local_24 == 6` before increment, so 6 entries per column (see unknowns); stop when the name x passes 755 | slot `clamp(mood + 2, 1, 10)` of the golfer being listed | EXACT |
| 2 | Golfers dock panel, card mode (size index 4 or more) | same, face at 29882 | same | face at (412 + 121 * col, 498 + 21 * row + 2); name at (326 + 121 * col, row y + 5); 4 cards per column (`local_24 == 4`); a 121x44 plate under each odd slot | same mood rule, each golfer of a pair gets its own face | EXACT |
| 3 | Golfer info card (click a golfer), main portrait | `FUN_0045c560` 60456, call 60509 | ball + halo head | ball piece at (172, -2), head drawn by `FUN_0045c200(slot, 172, -2, -1)` | head of the golfer's profile (3), expression 1.6 | DERIVED for sprite kind, EXACT for position |
| 4 | Golfer info card, partner portrait (extended card) | `FUN_0045c560` call 61957 | ball + halo head | ball at (552, 176), head `FUN_0045c200(slot ^ 1, 552, 176, -1)` | only in the extended card (golfer has a story link, field +0xb0 at 0x579568 not -1, and global 0x824144 nonzero). Partner = slot xor 1 | position EXACT; gating DERIVED, value of 0x824144 UNKNOWN |
| 5 | Message popup (advisor and first time comment messages, visitor remarks, golfer remarks) | `FUN_0040d320` 11510, call 11548; messages queued by `FUN_0040cb00(colour, priority, speaker)` (11248) | ball + halo head, text box | with a golfer speaker: x' = x + 0x1c, y' = y + 0x2c; ball and head at (x' - 0x32, y' - 0x28) = (x - 22, y + 4); text box `FUN_0040cef0(x' - 8, y' - 8, 0x1a0, h + 0x10)`, minimum text height 0x60 | speaker = golfer slot stored in global 0x4c2e08. The slot's own head, gender and newest reaction | EXACT geometry, DERIVED sprite |
| 6 | Message popup with a non golfer speaker | same routine 11550 to 11580 | `PopUpIcons` pieces and a site picture | -5 draws only the ball piece at (x' - 0x28, y' - 0x28); -4 draws a piece of array 0x5791f8 chosen by a byte of the current course site record at (x' + 8, y'); -3, -2, -0x14, -0x15, -0x16 draw the pieces at 0x53dea4, 0x53ded0, 0x53de78, 0x53df28, 0x53defc (all 90x80 pieces of PopUpIcons) at (x', y' + 0x10); codes -6 to -14 draw one hidden sprite at (x' + 8, y' - 0x14) | no face involved | EXACT (addresses and offsets), meaning of codes UNKNOWN |
| 7 | Player card at game start | `FUN_004065c0` 3944, call 4024: `FUN_0045f0f0(name, 0, n, -1, 200)` | stats card, head over body | see 2.2 | profile 0 through scratch slot 0x99 (the routine sets identity 0 and the gender word before drawing) | EXACT |
| 8 | Player card when skill points are awarded | `FUN_0046e810` 82368, call 82906: slot 0x9c, x0 = 200 | same | same | scratch slot 0x9c, whose identity is set from profile 0 | DERIVED |
| 9 | Pro match intro ("X vs. Y") | `FUN_004289e0` 19528, calls 21548 and 21551 | stats cards | two cards: x0 = 0 for the golfer that is the pro (slot `s ^ 1`), x0 = 0x15e (350) for `s` | each card shows its own slot | EXACT positions |
| 10 | Stats card used inside the golfer info card | `FUN_0045c560` call 60493 with x0 = -0x32 | stats only (negative x0 means no portrait) | no head | none | EXACT |
| 11 | Pair selection ("select the next pair of golfers") | `FUN_00459850` 57535, draw at 57613 to 57626 | `PairButtons.pcx` card (329x136 pieces at (0,0), (0,136), (0,272)) with ball, head | card slot n: x = (n odd ? 329 : 0), y = 50 + 136 * (n / 2); card plate at (x + 6, y); head drawn at (x, y + 4); name at (x + 0xd6, y + 9), title line at (x + 0x106, y + 0x28). Head is drawn full strength when hovered or selected and dimmed (alpha 0x3f333333, about 0.7) otherwise | candidates are golfer slots whose hole byte (record +0x21) is 0xff. Gender via `FUN_0046c940`; no mood call is made, so the row is UNKNOWN (probably the first row) | position EXACT, sprite kind DERIVED (halo, the ball in the card art matches a 140 cell) |
| 12 | Pick a Pro screen (and the same file list screen) | `FUN_0043a8c0` 34328; hover loads the file with `FUN_00437fa0(file, 0, 1)` at 34887; preview draw at 34523 `(0x45, 0x3d)` | `Title_Pickapro.pcx` ball medallion; portrait or body of the hovered file | drawn at (69, 61); a 140 cell placed there is centred on the medallion rings (about (139, 131)) | the loaded file goes into profile 0 and slot 1. Which sprite object is drawn is hidden | position EXACT, sprite UNKNOWN (DERIVED guess: halo style head, expression neutral) |
| 13 | Customise golfer screen | `FUN_004385d0`; head at (0x150, 0x14) stock or (0x13c, 0x14) custom, body at (0x163, 0x74) | expression head or halo head, body | covered elsewhere; listed here because it confirms 2.2 geometry | | EXACT |
| 14 | Hole Stats, Player Comments F2, Membership Roster F9, Top 10, Theme Packs, End of Year, Tournament results table | `FUN_00453330`, `FUN_004546b0`, `FUN_00454c50`, `FUN_00473470`, the theme pack routine near 87803, `FUN_0045a090` | none | none | these routines contain no call to `FUN_0045c200` and no face clamp; Top 10 draws text and trophy pieces only | EXACT |
| 15 | Employee panel, Player panel (JoeCool), Buildings, Amenities, Terrain panels | `FUN_00436e50` and neighbours | buttons and meters only | no heads | none | EXACT |
| 16 | In-world golfer label and speech bubble | `FUN_00462be0` 67538 | no art | name text above the golfer (white 0x80007fff, or red 0x80007d08 when leaving), plus a short mood bar below it in colour by mood | colour: mood below 0 black, 0 to 2 red, 3 to 4 orange, 5 to 6 yellow green, 7 and up green (0x80007d08, 0x80006300, 0x80001304, 0x800023e8) | EXACT |

### 2.2 `FUN_0045c200` in detail (line 60259)

Arguments (golfer slot, x, y, mode). Profile index = short at record +0xb6 (0x57956e). Head byte = profile +0x22 (0x4d60aa + id * 0x230). Callers use mode -1 (ball portrait) or 1 (head over body); mode 0 would fill an 80 x 96 white rectangle at (x + 4, y) first and is never called in this build (DERIVED dead).

* Mode other than 1: the head sprite is drawn at exactly (x, y) for both stock heads (head byte below 0x14) and custom heads. The code is a pair of branches that differ only in which sprite they use. DERIVED: both use the 140 x 140 halo cell (see 3.3), because the halo content is centred for the ball and the `.chr` exporter indexes the halo array for all heads. UNKNOWN for certain, since the sprite pointer is lost.
* Mode 1 (head over body), EXACT geometry:
  * if profile byte +0x2c (0x4d60b4) has bit 8 set, a sprite is drawn first at (x + 0x10, y + 0x4c) (DERIVED: a back accessory such as a prop behind the body);
  * the palette composer `FUN_00462020(slot)` runs, then the body is drawn at (x + 0x13, y + 0x4c) with the golfer's palette (body sheets are 60 x 120, see 2.4);
  * the head is then drawn: head byte below 0x14 at (x, y - 0x14), otherwise at (x - 0x14, y - 0x14).
  * Customise confirms the same geometry: stock head (0x150, 0x14), custom head (0x13c, 0x14), body (0x163, 0x74), accessory (0x160, 0x74): body origin = head origin + (0x13, 0x60) (EXACT, lines 32296 to 32320).
  * DERIVED: here stock heads use the 90 x 120 expression cells (that is why the stock draw has no 0x14 x shift and the custom 140 cell does), custom heads use the 140 halo cell.
* Edge case, UNKNOWN: head byte exactly 0x13 (19) is the first custom slot yet takes the stock branch in mode 1, where the expression array has no cut sprite for index 19. Either the original shows nothing for the first custom head in mode 1 or the branch uses another array. The port should draw the custom portrait in that case.

### 2.3 Stats card `FUN_0045f0f0` (line 62910), x0 = param_5

Only when x0 is not negative: a backdrop sprite at (x0 + 0xf7, 0x56) (DERIVED `HeadBodyBck.pcx`, 115 x 229), then `FUN_0045c200(slot, x0 + 0xff, 0x66, 1)`, then a small piece at (x0 + 0x146, 0x13e). Slot -1 means the player scratch slot 0x99 whose identity is profile 0. The twelve skill rows are listed from the skill table; the value row for the player comes from the record at 0x5795a8 + slot * 0x100, otherwise from the scratch defaults at 0x5a59fa + 10. Skill details are outside this document.

### 2.4 Body sheets (for the cards that show a body)

`Bodies/FemalePLS`, `FemaleSSS`, `FemalePSS`, `FemaleSkTT` and their `_sm` twins (8 female), `MalePLS`, `MaleKLS`, `MalePSS`, `MaleSSS` and `_sm` twins (8 male), plus `Barrel`: each 60 x 120, one piece cut at (0, 0, 0x3c, 0x78) (EXACT, lines 40493 to 40576 loaders and cuts). Recolour palettes are the `*Swap*.pcx` and `*_DefaultPal.pcx` 60 x 60 files (DERIVED role, composed in `FUN_00462020`). Which of the 8 sheets a profile uses: UNKNOWN (several profile bytes feed `FUN_00462020`).

---

## 3. Identity to head mapping

### 3.1 Chain (EXACT unless noted)

1. A golfer slot (0x100 byte record at 0x5794b8, 152 slots plus scratch slots 0x98, 0x99, 0x9c) has an identity short at +0xb6 (0x57956e).
2. Identity indexes the 0x230 byte profile table at 0x4d6088. Head = byte +0x22. Gender = bit 7 of byte +0x21 (set = female). Title +0x00, name +0x10.
3. The head byte selects the cell: 0 to 18 stock (sheet of the profile's gender), 19 and up custom (halo slot of that gender, 3.2).
4. Expression row = `FUN_004675d0(slot)` (1.6).

### 3.2 How each kind of golfer gets a head

| Golfer | How | Tag |
|---|---|---|
| Arriving member | identity = random 1 to 75 (`rand(0x4b) + 1`, line 13935), rejected when another waiting golfer already has the same identity or the same identity mod 0x13 (19) (line 13962) or the profile is flagged unavailable. Head byte comes from the profile table. | EXACT for the draw, head values UNKNOWN (see below) |
| Theme characters | `FUN_004658b0` (68585): lists `Themes\<theme>\*.chr`, and for the i th file (i from 1) sets profile i from the file and flags the member record (lines 68696 to 68702). So a theme's characters replace members 1 to n. A file with head byte 20 or more brings its own portrait (1.4). | EXACT |
| Player (profile 0) | head and gender chosen in Customise (stock index below 0x13 stored at line 33838; custom portrait kept as an embedded PCX when saved, line 31751 onwards). Pick a Pro loads a `.pro` file into profile 0. Default `Gary Golf.pro`: male, stock head 8. | EXACT |
| Special visitors | `FUN_004659a0` (68712, lines 68734 to 68737) loads four `.glf` files into profiles 0x4c Joe Pro, 0x4d I.M. Picky (title Commissioner), 0x4e Ivana Richman (title Heiress) and 0x4f J.P. Bigdome (title CEO). Standard theme files, measured: Joe Pro male head 8; Commissioner male head 9; Heiress female head 15; CEO male head 11, none with an embedded portrait. Firaxis theme files: Joe Pro "Judicious Pro" head 8; Commissioner head 9; Heiress head 15; CEO "I.B. DerBingle" head 20 (custom, embedded portrait, 140x420). The visitor remarks are message popups with the visitor's slot as speaker. | EXACT (files measured); which theme folder the startup path names is UNKNOWN |
| Tournament pros and pair candidates | generated at lines 80780 to 80960 from the pro table at 0x58dd50 (stride 0x38); the pro's file is loaded through `FUN_00437fa0`; when the file fails, profile fields +0x2b to +0x30 are filled from the table (head byte is not touched there) | EXACT for flow, per pro head UNKNOWN |
| Celebrities | the celebrity visitor is dead code in this build (noted in `include/sg/visitors.h`); celebrity art (`Celebs/*_SQ`, `_Walk`, `_Char`) is loaded as world sprites by `FUN_0043d740`. No portrait draw refers to celebrities | DERIVED |

### 3.3 Ball piece and frame

`Interface/TransPopups.pcx` (800 x 600, index 255 transparent) with its alpha sheet `TransPopups_A.pcx` (white = opaque, measured) holds a 140 x 140 piece at (0, 300) cut by the loader (`FUN_00473bf0(.., 0, 300, 0x8c, 0x8c, 1, 0)`, lines 40860 and 40906, the same piece in both groups). It is a golf ball whose disc spans x 7 to 132, y 302 to 427 on the sheet, centre (69.5, 364.5), i.e. (69.5, 64.5) inside the piece. The portrait frame passed to `FUN_00473f60` in the callers is `&DAT_0058bb7c` = element 29 of the array at 0x58b680 (36 records, EXACT), and I could not order the cut list to prove element 29 is this piece (UNKNOWN), but it is the only 140 x 140 piece in the group and its centre matches the centre of the halo heads (about (67 to 72, 66 to 70)), so DERIVED: ball at (x, y), halo head at (x, y), same origin.

---

## 4. What the port (`tools/sgview.cpp`) should show

1. Roster F9 (`drawRoster`) and Player Comments F2 (`drawComments`): no faces. Keep text only.
2. Golfers dock panel (around line 3306):
   * fix the face source rect: `Rect{594 - 16 * face, 100, 16, 16}` with `face = clamp(mood + 2, 1, 10) - 1`;
   * add the compact layout when the panel size index is below 4 (face at (310 + 60 c, 498 + 14 r) and the name at (325 + 60 c, 499 + 14 r), 6 per column by the code as read);
   * card mode positions already match (face x + 102, y + 2, name x + 16).
3. Golfer info panel: draw the ball piece (TransPopups (0, 300, 140, 140), alpha from TransPopups_A) at (172, -2) with the golfer's halo head cell on top at the same origin; add the second ball and partner head at (552, 176) only for story pairs.
4. Advisor and remark popups (the top centre box at line 3603): when the speaker is a golfer, draw the ball and head at (box left - 0x32 + ... ) i.e. ball and head at (x - 22, y + 4) with the box starting at (x + 20, y + 36) in the original popup coordinates; use the reacting golfer as speaker, and the player's own head for the player's advisor lines.
5. Pair selection and Pick a Pro: draw the head on the card ball as in 2.1 rows 11 and 12.
6. Head cell source for a golfer: stock heads via the halo page (140 cell) for ball portraits, expression sheet only when a head over body figure is drawn (cards, customise). Custom portraits via their 140 x 420 file, rows happy, neutral, angry.
7. Member heads: because the per profile head table is not in the decompile (UNKNOWN), use a placeholder rule and mark it as such in code, for example `head = (memberId - 1) % 19` with the gender the port already tracks (the id mod 19 uniqueness test in the exe suggests the original spreads identities over the 19 heads, DERIVED hint only).

Rect cheat sheet (all EXACT unless noted):

* expression cell: x = (h / 2) * 100, y = (h & 1) * 372 + {4, 128, 252}[e], w = 90, h = 120;
* halo cell: x = (h / 2) * 140, y = (h & 1) * 420 + e * 140, w = h = 140;
* custom file: y = e * 140, w = h = 140, e = 0 happy, 1 neutral, 2 angry.

---

## 5. Unknowns

1. Default head byte and gender of each of the 75 member profiles (and the ones of the 0x58dd50 pro table): exe data tables, not in the decompile text. Not read, by rule.
2. The exact sprite array and index at every draw site that shows a head: the decompile drops the sprite pointer of these `__thiscall` draws (`FUN_00473cb0`, `FUN_00473e60`). All "halo vs expression" statements above are DERIVED from geometry, from the exporter and from the art.
3. The array index of the ball piece (0x58bb7c is element 29 of 36; cut order could not be matched, 2 groups of cuts give conflicting orders).
4. Expression row used by pair selection and by the Pick a Pro preview.
5. Order in which loose `Heads/*.pcx` names and startup `.glf` and `.chr` loads take custom indices (directory order, startup order). Not needed if the port binds portraits by file.
6. Head byte exactly 19 in mode 1 (2.2 edge case).
7. Value of global 0x824144 and what sets story link field +0xb0 (0x579568), which gate the second portrait in the golfer info card.
8. Consumer of the GBUBBLES array (0x59b050): not visible. The in-world bubble has no art.
9. Rows per column of the compact dock list: the loop wraps when `local_24 == 6` before the increment (6 entries), while `docs/DECODE_BUILDINGS.md` section 6 says 7. Card mode (`== 4`) agrees with the docs. Recheck against a screenshot.
10. Body sheet choice per profile and the meaning of profile byte +0x2c bit 8 (accessory sprite behind the body).
11. Theme folder used by the four special `.glf` loads at startup (their path string is only partly visible).
