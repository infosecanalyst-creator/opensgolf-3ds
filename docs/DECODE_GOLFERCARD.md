# Decode: golfer info card, stats card, Player panel, course info

Source: the existing decompile text only (`spec/golf_decomp.c`, publisher exe). Art facts were measured with PIL on the disc PCX files. Own words throughout; no game sentences are copied.

Tags: EXACT = read directly from the code (or measured on the art). DERIVED = follows from code plus a stated inference. UNKNOWN = not established from the decompile text.

Colour note: colour arguments are 32-bit values whose low 15 bits look like 0RRRRRGGGGGBBBBB (0x7fff is white). DERIVED. The 0x80000000 bit is a flag whose meaning is UNKNOWN (set on almost every text call).

## 0. Read this first

1. EXACT: two big routines. `FUN_0045c560(slot, flag)` (lines 60456 to 62047) is the golfer info card. `FUN_0045f0f0(title, x, points, slot, xoff)` (62910 to 63343) is the stats card (skills dialog). The info card calls the stats card itself in its first lines, and the stats card draws the portrait through `FUN_0045c200` (see DECODE_FACES.md).
2. EXACT: the card is a modal-style redraw routine. It is called every frame, reads the mouse with `FUN_0047ab50`, sets the hover id `DAT_00824130` (-1 = none), and draws. The click action is NOT in this routine (UNKNOWN: handler lives with the caller, probably `FUN_0045c030` / `FUN_0045ae70`).
3. UNKNOWN: who opens the info card from a world golfer click. No world-golfer hit test and no caller of `FUN_0045c560` was found by name. Do not invent one; the port's own click on a golfer sprite plus camera follow is a port decision.
4. UNKNOWN: any draw site of `courseinfo.pcx` / `s_courseinfo.pcx` (loader only, see section 6).

## 1. Golfer record fields used (golfer base 0x5794b8, stride 0x100; 16-bit view stride 0x80)

All EXACT as addresses. Meaning is EXACT where stated by use, otherwise DERIVED.

| address (slot s) | use on the card |
|---|---|
| `0x57956e[s*0x80]` (short) | profile index into the profile table (base 0x4d6088, stride 0x230); 0 means no real golfer (pro/no slot) |
| `0x579568[s*0x80]` | partner golfer slot, -1 if none (a playing partner changes the layout, section 2.2) |
| `0x57955a[s*0x80]` | partner slot used by the Next Chapter button test |
| `0x57956a[s*0x80]` (byte) | state value; value 4 on both golfer and partner hides the Next Chapter button |
| `0x57955c` (short) | mood level, bar uses `(8 - mood) * 10` |
| `0x579562` (short) | energy value |
| `0x57955e` (short) | hunger value |
| `0x579560` (short) | thirst value |
| `0x579540..` (5 shorts) | newest five reactions; top 2 bits 01 = good (+1), 11 = bad (-1) (`FUN_0045c420`) |
| `0x579558` (short bitmask) | 14 bits of favourites or traits, each picks a word from the table at `PTR_DAT_004c2960` (see 3.4) |
| `0x5794d9` (byte) | number of holes played (0..18), -1 means the golfer has no round record (the routine returns) |
| `0x5794da` (byte) | strokes so far on the hole in progress |
| `0x5794db[s*0x100 + h]` | strokes on finished hole h (1-based), 0 means not played |
| `0x5794d0` (byte) | nonzero makes the info card draw the stats card first (DERIVED: skill dialog pending) |
| `0x5794c8[s*0x40]` | bit 0x100000 decides which of two golfers is "self" in the partner scorecard |
| `0x5795a8[s*0x100 + i]` | 10 skill values of the golfer (i = 0..9) |
| `0x575cb8 + 0x208*(h-1)` (byte) | par of hole h |

Mapping to port data: see section 7.

## 2. Golfer info card (`FUN_0045c560`)

### 2.1 Common frame (EXACT unless marked)

- Header panel sprite `0x58b51c` at (0xec, 0x1a) when the golfer has no partner or `DAT_00824144 == 0`. A shaded copy of the background is drawn first with the colour remap table `DAT_00822c74` (`FUN_004740f0`). With a partner (and `DAT_00824144 != 0`) the header uses `0x58b548` at (0xec, 0xa5) and a plain draw at (0xec, 0x1a) first (`FUN_00473e60`).
- Portrait backing `0x58bb7c` drawn at (0xac, -2). Portrait via `FUN_0045c200(slot, 0xac, -2, -1)`. Mode -1 is the plain draw (mode 1 is the stats card variant, mode 0 clears a 0x50x0x60 rectangle first).
- Fonts: large `0x519928`, body `0x51b360`, small `0x519fd8` (selected with `FUN_004762d0`).
- DERIVED: the screen is 800x600, the card is full width, with portrait on the left, text centred on x = 0x1a4 (420), and the five meters on the right at x 0x222..0x272.

### 2.2 Two layouts

The routine builds `local_4 = 1` when the golfer has a partner and `DAT_00824144 != 0`. The two layouts differ as follows (EXACT for coordinates, DERIVED for names).

Layout S (single golfer):
- Scorecard of one golfer, button icons at y 0xed (see 2.6), hit centres at y 0x106.
- Buttons: Customize, Move/Eject Golfer, Take Snapshot, View Story (View Story only when a partner slot exists, test on `0x579568`), plus a fifth with no tooltip (UNKNOWN meaning, hit id 4).

Layout P (with partner, two scorecards):
- Second portrait (partner) backing `0x58bb7c` at (0x228, 0xb0), `FUN_0045c200(partner, 0x228, 0xb0, -1)`.
- Partner scorecard lines (small font) at x 0xf8 + 10 per hole, rows starting y 0xb3, step 9 or 10, in the two golfers' colours; a divider line `FUN_00478b80(0x148,0xaf,0x210,0xaf)` and heading at (0x1ac, 0xa7) in colour 0x800003ff.
- Buttons at y 0x10e/0x110: Customize (0x10d,0x10e), Next Chapter (0x14e,0x110, hidden when both golfers are state 4), Take Snapshot (399,0x110), Golfer Comments (0x1d0,0x110), id 4 at (0x20e,0x110) with no tooltip. Hit centres at x 0x126, 0x167, 0x1a8, 0x1e9, 0x221, y 0x11f.
- Story/comment lines (up to 5, from the table at `0x579528 + n`, ids at `0x579532 + n`) are drawn left aligned at x 0xf8, y from 0xaa step 0xd (13), body font, long lines (width over 0x181) are wrapped after 0x12 characters; id '2' selects the small font. Text content of these lines is UNKNOWN (strings are in exe data).

### 2.3 Text items in layout S (all centred on x 0x1a4 unless stated)

| y | font | colour | content | tag |
|---|---|---|---|---|
| 0x28 | large, centred variant C ("bc0") | 0x80000210 | name line: three strings joined: prefix (DAT 0x4c4974), the profile's own name string (first field of the 0x230 record), suffix (DAT 0x4c59e0) | EXACT structure, UNKNOWN literal text |
| 0x3e | body, centred A ("b70") | 0x80000210 | one of four marital states (Single, Married, Divorced, Widowed) joined with an age phrase from `FUN_00453260` | EXACT states, DERIVED field choice |
| 0x4b | body, A | 0x80000210 | a list built with "and" between items (DERIVED: likes or occupation) | UNKNOWN |
| 0x5b | body, C | 0x80004210 | list drawn from the skill/length/accuracy words in `PTR_s_length_004c2858` | DERIVED |
| 0x69 | body, A | 0x80000210 | a closing line (DERIVED: mood quote `FUN_00469a20`) | UNKNOWN |

Name builder is `FUN_004676e0(slot, 0)` (line 71161); it fills `DAT_0051a068`, the shared text buffer.

### 2.4 The favourites/traits line (EXACT mechanism)

For bit k = 0..13 of the short at 0x579558: if set, append the k-th string of `PTR_DAT_004c2960`, otherwise append an empty/placeholder string (`DAT_004c4944`). The table has 14 slots (0x4c2960 to 0x4c2998). The strings themselves are UNKNOWN.

### 2.5 Meters (EXACT)

Five horizontal meters, 80 pixels wide, 4 high, left x 0x222, right edge 0x272, at y 0x36, 0x46, 0x56, 0x66, 0x76. Labels are drawn with the small font centred at x 0x24a, one row above their bar (y 0x2c, 0x3c, 0x4c, 0x5c, 0x6c). Label strings: first label is `PTR_DAT_004d2120` (DERIVED: mood), then Attitude, Energy, Hunger, Thirst (these four are named strings).

For each meter, w is a "dark" width (the part that is not filled):

1. Mood: `w = clamp((8 - mood) * 10, 0, 0x50)`.
2. Attitude: `w = clamp((4 - n) * 10, 0, 0x50)` where n = (count of last 5 reactions that are good) minus (count that are bad), range -5..5.
3. Energy: `w = clamp(energy / 4, 0, 0x50)` (the exe divides with a rounding step toward zero).
4. Hunger: `w = clamp(hunger * 5 / 2, 0, 0x50)`, integer division.
5. Thirst: `w = clamp(thirst * 5 / 2, 0, 0x50)`.

Drawing: a bright bar of the full 0x50 width at x 0x222, then a darker bar of width w drawn at x = 0x272 - w. Bright colour is 0x7d08 (DERIVED: red) when w > 0x28, otherwise 0x23e8 (DERIVED: green). Dark colour is 0x6000 when w > 0x28, otherwise 0x1284. The helper `FUN_00467130(v, lo, hi)` clamps (EXACT). Note: a full bar means the thing is satisfied; the hunger and thirst values fall as the golfer fills up (DERIVED from the formula direction, the meters do not use a flip).

Port source: `Golfer::rx` (needs counters, see DECODE_EVENTS_NEEDS.md) for hunger, thirst, energy; mood level from `Golfer::mood`; reactions from the history in `Reactor`. Exact scaling of the port's own counters versus the exe's is UNKNOWN; apply the formulas above after mapping.

### 2.6 Scorecard strip (layout S) (EXACT)

- Eighteen columns. Column i (1..18) has x = 0x10a + 0x12 * (i - 1), so the first is 0x10a and the last 0x23a (loop ends when x reaches 0x24e).
- Row 1 hole number at y 0x84, body font, colour 0x80000000 (black). The number text is made with the C `itoa` helper.
- Row 2 score at y 0x94, large font. Only if strokes on the hole are nonzero. Colour rules, with p = par of that hole (read at `0x575cb8 + 0x208 * (i-1)`), s = strokes:
  - default 0x80000000 (black)
  - s < p: 0x80007d08
  - s < p - 1: 0x80007ff0 (overrides, eagle or better)
  - s > p: 0x8000211f
  - s > p + 1: 0x80004010 (overrides, double bogey or worse)
- If i equals the holes-played byte, the hole in progress is shown in grey (0x80004210) from the stroke counter at 0x5794da, and its strokes are added to the total.
- Total at (0x256, 0x94), large font, black. The total is the sum of all shown strokes.
- Before drawing, if the holes-played byte is below 0x13 (19) the current mood byte is copied into slot `0x579515 + holes` (EXACT, purpose DERIVED: records mood per hole).

Port source: `Golfer::strokesRound` and the per-hole strokes list (a port-side vector is needed; the port stores only the round total), par from `HoleInfo` / holestats.

### 2.7 Buttons, hit areas, tooltips (EXACT)

Hit test: `FUN_00467170(dx, dy) < 0x19` (distance metric, treated as a radius of 25). The hover id replaces `DAT_00824130`.

Layout S: ids 0..4 with centres (0x13e,0x106), (0x17f,0x106), (0x1c0,0x106), (0x201,0x106), (0x253,0x106); icons at (0x125,0xed), (0x166,0xed), (0x1a7,0xed), (0x1e8,0xed), (0x23a,0xed). Sprite blocks at 0x567390, 0x567498, 0x56751c, 0x5675a0, 0x567624; each block has two sprites 0x2c bytes apart: normal and hover (hover sprite index is `hover id == this id`).

Layout P: ids 0, 5, 2, 3, 4 with centres x 0x126, 0x167, 0x1a8, 0x1e9, 0x221, all y 0x11f. Sprite blocks 0x567390 (id 0), 0x5676a8 (id 5), 0x5677b0 (id 2), 0x56772c (id 3), 0x567834 (id 4).

Tooltip: the hover id must stay the same for at least 7 frames (counter `DAT_00823768` resets to 0 on any change). Then `FUN_00432620(mouseX, mouseY)` draws the box. Texts by id: 0 Customize, 1 Move/Eject Golfer, 2 Take Snapshot, 3 View Story (layout S) or Golfer Comments (layout P), 5 Next Chapter. Id 4 has no tooltip text.

Sprite block to PopUpIcons/InfoButtons cut mapping: UNKNOWN (hidden destination of the cut calls); see section 5 for the measured sheet contents.

### 2.8 Members, visitors, pros

- EXACT: nothing in the card branches on a "member vs visitor" flag. The only golfer-type switch is the profile record (the golf pro entry sits at the start of the profile table, the string "Golf Pro" is the first record) and slot state byte 0x57956a.
- DERIVED: the profile table is what makes pros differ. A pro has profile index of the pro record. The stats card calls `FUN_0045c200`, which reads a byte at profile offset 0x22 (`0x4d60aa`) and compares with 0x14 (a head-size/group split). See DECODE_FACES.md.
- UNKNOWN: whether the port's `Visitor`/`Roster` flags should change text; the exe's card does not.

## 3. Stats card (`FUN_0045f0f0`)

Called with (title text buffer, x, points, slot, xoff). `points` = skill points to spend (-1 means "view only/no OK button return -1", 0 means view only, positive means editable). `xoff` (param_5) is 0 for the full card or negative (-0x32 from the info card) for the narrow one (EXACT).

### 3.1 Frame (EXACT)

- If xoff < 0: `FUN_0040cef0(xoff + 0x4e, 0x32, 0xd0, 0x13c, 1)` (narrow, 208 x 316 at x = 0x1c..).
- Otherwise: `FUN_0040cef0(xoff + 0x2e, 0x32, 0x140, 0x13c, 1)` (wide, 320 x 316).
- Frame helper builds a 3x3 frame from 16x16 pieces (see DECODE_PANELS.md).

### 3.2 Header and portrait (wide card only)

- Title: large font, centre C at (xoff + 0xd2, 0x3a), colour 0x80007fff (white). Text is the first parameter, UNKNOWN literal (the name builder output).
- Wide card only: portrait ring `FUN_00473cb0(PTR_DAT_004c1570, xoff + 0xf7, 0x56, 1,1,1,0)`, then `FUN_0045c200(slot, xoff + 0xff, 0x66, 1)`, then sprite `0x56a894` at (xoff + 0x146, 0x13e) (identity of that sprite is UNKNOWN; likely the OK button, because the OK hit area is at the same place, see 3.4).
- When slot is -1 (the player), the routine fills the player's own head record (`DAT_00582e70`, `DAT_00582e6e`, from `DAT_004d60a9`) and uses slot 0x99.

### 3.3 Skill rows (EXACT)

Ten rows, row r = 0..9, y_r = 0x5a + 0x18 * r. Names come from the pointer table starting at `PTR_s_Power_Hitter_004c2c3c` (10 entries, the first is "Power Hitter"; other nine names are in the table but not read in this decode, UNKNOWN literals).

Per row:
1. Skill value v = player's own value from `0x5a59fa + 10 + r` when slot is -1 or the slot has no profile, else the golfer's `0x5795a8[slot*0x100 + r]`.
2. Bar icon `0x58baf8` at (xoff + 0x52, y_r).
3. If points > 0: control sprite `0x58bba8` at (xoff + 0x32, y_r) (plus/minus pad). While hovering the top half (y_r < my <= y_r + 0xc and mx within xoff+0x32..xoff+0x52) the hover sprite `0x58bbd4` is drawn and the hover index = r (add one); hovering the lower half (y_r + 0xc < my <= y_r + 0x18) draws `0x58bc00` at y_r + 0xc and the hover index = r + 10 (subtract one).
4. If v is nonzero: value digits (clamped to 9 for the text case) at (xoff + 0x57, y_r + 7), left aligned, colour black.
5. Row name at (xoff + 0x8a, y_r + 7), left aligned, colour black when v > 0, grey (0x4210) when v = 0.

Skill points line: at (xoff + 0xd2, 0x50), centre C, colour 0x7d08 (red) while points are left, else 0x2f7 + 0x7d08 (a green-ish tone). Text UNKNOWN (contains the number of points left).

### 3.4 Interaction (EXACT)

- OK hit area: |mx - xoff - 0x15e| < 0x14 and |my - 0x14e| < 0x14.
- If points remain and OK is pressed, a confirm dialog (a sub-screen, 800x600 with text "You haven't used all your skill points..." and a stinking-skill-points joke answer) is shown. If accepted, the routine returns `DAT_004c2c9c` (the changed-skills bitmask); otherwise it loops.
- Plus click: allowed if points left (local_314 < points), skill < 10: skill += 1, points left -= 1, bit r of `DAT_004c2c9c` set.
- Minus click: allowed if skill > 0 and skill > the value it started the dialog with (copy `local_2e4`): skill -= 1, points returned (EXACT test, the exact arithmetic of the tail was only read to this point; mark the rest DERIVED).
- Info card embed: with xoff = -0x32 the card is the narrow variant used while the info card is up and the golfer's flag at 0x5794d0 is nonzero.

## 4. Player panel (JoeCool), dock modes 3 and 4

Read from `FUN_004362f0` (draw, line 30129), `FUN_00435f00` (hit test, 29984), `FUN_00436060` (click, 30048).

- EXACT: panel base sprite `0x58b46c` at (0xd6, 0x1da).
- EXACT hit zones (screen px, panel at fixed position): round buttons radius 0x14 at (0x111, 0x22f); radius 0xf at (0x139, 0x207), (0x139, 0x224), (0x139, 0x244); radius 0x14 at y 0x1f9 and x 0x19e, 0x1ed, 0x23c, 0x28b, 0x2da; radius 0xf at (0x11e, 0x1ea); a "not inside" test radius 0xf at (0xe7, 0x21c).
- EXACT: draw positions for the five lower-row icons come from a table of shorts at `0x4c7ac0` and following (pairs x,y: `0x4c7ac4`, `0x4c7ac8`, `0x4c7acc`, then `0x4c7ad0..0x4c7ae2`); the table values are UNKNOWN (data bytes).
- EXACT: skill list uses the same 10-name table as the stats card (`PTR_s_Power_Hitter_004c2c3c`), small font `0x519fd8`, text colour 0x80007fff (white) on the panel, left aligned.
- EXACT: shows the strings "Practice Round" and "Play" on the action button depending on state.
- UNKNOWN: pixel cuts of JoeCoolPanel.pcx per piece; the Player panel's sprite array base is `0x586f50` (stride 0x2c, includes the selected-skill marker `0x586f7c`).

## 5. Art sheets (measured)

- `GolferStats.pcx` plus `GolferStats_A.pcx`: plate A at (64, 33) size 402 x 244 is fully opaque; its transparent window runs x 9..392, y 142..217 relative to the plate (EXACT, measured on the alpha). Plate B at (64, 300) size 402 x 138 is opaque (EXACT). The colour sheet uses palette index 255 (magenta) as transparent (EXACT). Tan region and cream region extents on plate A were sampled by colour (tan x 62..302, y 34..100; cream x 62..392, y 10..97), DERIVED, treat as approximate.
- `s_GolferStats.pcx` is the shadow companion sheet (loaded at the same point); DERIVED use as a drop shadow behind the card, exact piece list UNKNOWN.
- Scorecard vertical rules: measured on the colour sheet earlier but the numbers were not recorded here. UNKNOWN until re-measured.
- `PopUpIcons`, `InfoButtons`, `TransPopups`: loader sections `FUN_00442180` lines 40478, 40750 to 40800, 40801 to 40925. The cut-order to array index mapping is not provable because the cut helper's destination is hidden (EXACT limitation). Contents seen by eye: PopUpIcons holds small round icons (DERIVED: the info card button set and mood icons), InfoButtons holds the 5 round card buttons in pairs of normal and hover (DERIVED), TransPopups holds translucent popup frames (DERIVED).
- The sprite array bases in 2.7 are fixed constants; the port should cut the sheets in loader order and index by pair (normal at 2k, hover at 2k + 1), checking visually.

## 6. Course info screen

- EXACT: `courseinfo.pcx` and `s_courseinfo.pcx` are loaded in `FUN_00442180` at lines 40954 to 40968.
- UNKNOWN: any routine that draws them. No draw site was found by name; do not invent one. The port may draw the sheet as the hole-info backdrop (the existing hole stats overlay) as a design choice.

## 7. Implementation notes: item to port data source

| card item | port source |
|---|---|
| name | `sg::Roster` member name (membership.h) or the visitor's `look` name; CharRec name (charrec.h) |
| title / marital / age lines | CharRec age and status fields; UNKNOWN literal wording, use the manual's phrasing from MANUAL_NOTES.md |
| favourites bits (14) | CharRec preference mask, else a placeholder; mark as PLACEHOLDER |
| mood bar | `Golfer::mood` (0..10 scale) mapped to the exe's 0..8 range: bar fill = 80 - w where w = (8 - m) * 10 |
| attitude bar | last five reactions in `Reactor` (good = +1, bad = -1) |
| energy / hunger / thirst | `Reactor` needs counters; scale by the formulas in 2.5 |
| scorecard per hole | needs a new `std::array<int,18> strokes` on `Golfer`; par from `HoleInfo`; colours per 2.6 |
| total | sum of the strokes array, or `strokesRound` |
| current hole grey score | `Golfer::hole` and the live stroke count |
| skills (10 rows) | CharRec skills (10) for members; visitors use a generated set; the port's `skillMask` is a PLACEHOLDER and must not be shown |
| partner layout | no port data; skip layout P until group play exists |
| tooltips | hover timer: 7 frames, then draw box next to the mouse |
| Customize / Move Eject / Snapshot / Story | existing port actions where present (snapshot, eject), otherwise hide the button |
| stats card plus/minus | CharRec skill points; edit rules per 3.4 (cap 10, cannot go below the starting value, bitmask of changed skills) |

Open items to resolve with further decompile work: strings behind `DAT_` addresses, the caller of `FUN_0045c560`, the world golfer click, the course info draw site, the meaning of `DAT_00824144`, the TransPopups array order, and sprite `0x56a894`.
