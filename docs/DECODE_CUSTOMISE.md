# Decode: Customise Character screen and Pick A Pro screen

Source: only the Ghidra decompile text `spec/golf_decomp.c` (publisher golf.exe). No other executable, no raw data tables
and no disassembly were used. Art measurements come from the PCX files on the disc (PIL). Facts are written in my own
words; quoted strings are short UI labels only.

Marks: **EXACT** = read literally from the decompile. **DERIVED** = follows from the code plus art or arithmetic.
**UNKNOWN** = the decompile does not show it.

Line numbers refer to `golf_decomp.c`.

## 0. Orientation (what the decompile actually contains)

| Item | Where | Mark |
|------|-------|------|
| Customise screen routine `FUN_004385d0(charRecord, golferSlot)`, lines 32196 to 34183 | one big modal loop | EXACT |
| Only call sites: lines 30078 and 30082, both `FUN_004385d0(0, 0x98)` inside the dock click handler (case 0 of `FUN_00436060`), i.e. the dock button for the player's own golfer | | EXACT |
| Just before the call the handler writes 0x20 into the golfer slot's type byte (`DAT_00582cd0`, which is `DAT_005794d0 + 0x98*0x100`) and sets the three clothing toggles to 7 if the record's flag dword (offset 0x2C) is still zero, else to its low 3 bits | line 30074 to 30081 | EXACT |
| Because the type byte is 0x20, the routine's mode flag `local_2e0 = ((type & 0xE0) == 0x20)` is 1 on every real call. Mode 1 shows the skill panel, mode 0 shows the membership and bio panel. Mode 0 code exists but no caller reaches it | line 32202 | EXACT |
| The screen is the player's own character editor, not a separate "new game" wizard. Char record index is always 0, golfer slot always 0x98 | | EXACT |
| Face picker overlay `FUN_00438390` (32097), its hit test `FUN_004382f0` (32069), main hit test `FUN_00438260` (32032) | | EXACT |
| Save `FUN_00437910` (31301), Load `FUN_00437fa0` (31810) | | EXACT |
| Pick A Pro `FUN_0043a8c0` (34330 to 35137), sheet cuts at 34426 to 34439 | | EXACT |
| Sprite sheet loader `FUN_00442180` (39532 onward): CGbuttons cuts at 40969 to 41022, HeadSelect cuts 41022 to 41044, HeadBodyBck cut at 41044 to 41046 | | EXACT |

Important limitation. Every hotspot centre, every hover sprite position and every label pointer lives in a static data
table (`DAT_004c7b38`, `DAT_004c7b90`, `DAT_004c7b9c`, `DAT_004c7be0`, `DAT_004c2858`, `DAT_004c2864`, `DAT_004c2cc0`,
`DAT_004c2d10`). The decompile only shows the code that indexes them, not their contents. The sprite object that each
draw call uses is passed in a register that the decompiler dropped (calls look like `FUN_00473cb0(PTR_DAT_004c1570, x, y,
...)`, where `PTR_DAT_004c1570` is the destination surface). So numeric centres below are DERIVED from the art, and the
sprite-to-call mapping is DERIVED from loader order and counts.

Conventions: screen 800x600. Mouse state `DAT_00822d68`: 1 = left click, 2 = right click, 0 = none. Colours are 15 bit
RGB with a 0x8000 prefix (0x7FFF white, 0x0848 dark navy, 0x7B20 orange, 0x4210 grey, 0x0000 black). Distance metric
`FUN_00467170(dx,dy)`: `a=|dx|, b=|dy|, d = (b<a) ? (b+2a)/2 : (a+2b)/2` (EXACT, 70713).

## 1. Layout and hit tests

### 1.1 Background art (DERIVED, measured on the PCX)

`CustGolfBckgrnd.pcx` (female, 800x600) and `CustGlfBckMale.pcx` (male) differ only in 2266 pixels inside
x 42..359, y 37..179. Choice is made by bit 7 of the record's flag byte B (offset 0x21): bit set draws the female sheet
(EXACT, 32258 to 32263 and 33892 to 33900). The gender button swaps the sheet instantly with `FUN_00475840(sheet,0,0,0x100,2)`.

Regions seen in the art:

| Region | Rect (x, y, w, h) | Mark |
|--------|-------------------|------|
| Name box with 4 text rows, separators at y 44, 65, 85 | 40, 18, 226, 90 | DERIVED |
| Left column of 5 cream buttons, 112x28 each, pitch 28 | origin (39, 116 + 28k), k 0..4 | DERIVED (template match of the CGButtons cuts) |
| Right column of 3 cream buttons, 112x35, pitch 35 | origin (157, 136 + 35k), k 0..2 | DERIVED |
| Three round buttons left of the preview window | cut origin (310, 38), (310, 88), (310, 138), visible circle centres about (328, 56), (328, 106), (328, 156) | DERIVED |
| Preview window (sky and grass) | about 335, 18, 112, 226 (HeadBodyBck is 115x229) | DERIVED |
| Five round tabs right of the window | cut origin (436, 12 + 50k), visible centres about (454, 30 + 50k), k 0..4 | DERIVED |
| Large cream panel (membership, bio, or skills) | about 455, 18, 265, 225 | DERIVED |
| Load icon, Save icon in the top corners | visible centres about (49, 22) and (259, 22), low confidence (+-4 px) | DERIVED |
| Undo ring (top right) and OK ring | visible centres about (762, 37) and (762, 218), low confidence (+-6 px) | DERIVED |
| Bottom table: 20 rows, dark left column x 40..400, cream right column x 400..762, rows from y 269 to 587 (line pitch about 15.9) | | DERIVED |

### 1.2 Main hit test `FUN_00438260` (32032 to 32067)

Scans a table of (x, y) short pairs starting at `DAT_004c7b38` until an entry with x == 0 (EXACT). Entries 0..7 (the first
eight, address below `0x4c7b58`) use `dx/3` (wide ellipse), entries 8 and up use plain `dx` (EXACT). The winner is the
entry with the smallest distance, and it must be below 0x28 (40) (EXACT). Returns -1 for no hit.
Entry centres: UNKNOWN (table data). Art based guesses are in 1.3.

Special case: if the result is 0xF (hair colour) and the record is male (flag B bit 7 clear), the result becomes -1
(EXACT, 32290). Hair colour cannot be changed for males.

Result indices (21 total), role from the click switch (33422 onward) and tooltip switch (33241 onward):

| idx | Role | Click effect | Tooltip label | Mark |
|-----|------|--------------|---------------|------|
| 0 | right column toggle 0 | flip bit 1 of working copy `DAT_00582dd1` | none | EXACT |
| 1 | right column toggle 1 | flip bit 2 of `DAT_00582dd1` | none | EXACT |
| 2 | right column toggle 2 | flip bit 4 of `DAT_00582dd1` | none | EXACT |
| 3..7 | left column trait 0..4 | flip bit k-3 (values 1,2,4,8,0x10) of record byte 0x20 | none | EXACT |
| 8 | Load icon | open the file picker, see 4 | Load character | EXACT |
| 9 | Save icon | save to file, see 4 | Save character | EXACT |
| 10 | body type | cycle high nibble of byte 0x23 through 0..3 (left click +1, otherwise -1, mod 4), set edited flag | Body type | EXACT |
| 11 | adult or child | flip bit 3 of the flag dword at 0x2C, set edited flag | Adult/child | EXACT |
| 12 | face | open the face picker (1.5), store the chosen head index in byte 0x22 | Select face | EXACT |
| 13 | shirt colour | byte 0x24 +1 or -1 mod 10, set edited flag | Shirt Color | EXACT |
| 14 | pants colour | byte 0x25 +1 or -1 mod 10, set edited flag | Pants Color | EXACT |
| 15 | hair colour | byte 0x28 +1 or -1 mod 5, set edited flag (disabled for males) | Hair Color | EXACT |
| 16 | skin tone | byte 0x27 +1 or -1 mod 4, set edited flag | Skin Tone | EXACT |
| 17 | gender | flip flag B bit 7, set edited flag, swap background sheet | Gender | EXACT |
| 18 | undo | copy the 0x230 byte snapshot taken at entry back over the record; stays in the screen | Cancel | EXACT |
| 19 | OK | commit and leave, see 6 | short label not in the symbol table (likely "OK") | EXACT behaviour, UNKNOWN text |
| 20 | Update Bio link | multi line bio editor (mode 0 only) | Update Bio (mode 0 only) | EXACT |

"Left click +1, otherwise -1": the code tests `DAT_00822d68 == 1` and uses the opposite step for anything else (in
practice right click) (EXACT).

Which round tab is which is not in the code. From the art and the tooltip list: face icon (head with hair) = 12, body
icon = 10, gender symbol = 17, people icon = 11, shirt icon = 13, pants icon = 14. The remaining two icons (a torso
figure and a head silhouette) are 15 and 16. Best guess: head silhouette = hair colour, torso = skin tone. (DERIVED, low
confidence.)

### 1.3 Estimated hit centres (DERIVED from art, use as defaults only)

| idx | Estimated centre |
|-----|------------------|
| 0, 1, 2 | (213, 153), (213, 188), (213, 223) (button rect centre, labels are centred at x = 212 in code) |
| 3..7 | (95, 130 + 28k), k = idx - 3 (labels are centred at x = 93 in code) |
| 8, 9 | (49, 22), (259, 22) |
| 12, 10, 17 | (328, 56), (328, 106), (328, 156) |
| 11, 13, 14 and the two unidentified tabs | x = 454, y = 30, 80, 130, 180, 230 in art order torso, people, head, shirt, pants |
| 18, 19 | (762, 37), (762, 218) |
| 20 | text link drawn at (580, 98), centre UNKNOWN |

Because the first 8 use `dx/3` with threshold 40, their horizontal reach is about +-120 px, which overlaps the
neighbouring column; the nearest centre wins (EXACT logic, DERIVED geometry).

### 1.4 Text field hit areas (checked when no icon is hit, or idx == 20) (EXACT, 32297 to 32320 and 33984 to 34155)

| Area | Test | Effect |
|------|------|--------|
| Name box rows | `|x - 154| < 80` and `|y - 64| < 40`, row = (y - 24) / 20 (rows 0..3) | row 0 edit name, row 1 edit title, row 2 cycle marital status, row 3 cycle age group |
| Bottom table right column | `|x - 578| < 178` (x 401..755) and `y > 269`, row = (y - 270) / 16, no upper bound check | edit dialogue slot `row` |
| Skill value boxes (mode 1 only) | `|x - 520| < 25` and `61 < y < 222`, row = (y - 64) / 16 | left click +1, other click -1, see 5.3 |

The hover state of the four name box rows is the same test (row = (y - 24) / 20) and only colours the text.

Row 2 and 3 cycle rules (EXACT):

* Marital status uses bits 0x08, 0x10, 0x20, 0x40 of flag B (Single, Married, Divorced, Widowed). Left click goes
  Single, Married, Divorced, Widowed, Single. Other click goes the reverse way. If none is set it starts at Single (left)
  or Widowed (other).
* Age group uses bits 0x01, 0x02, 0x04 (Young, Middle Aged, Mature). Left click goes Young, Middle Aged, Mature, Young.
  Other click reverses.

### 1.5 Face picker overlay (`FUN_00438390`, 32097 to 32170) (EXACT unless stated)

* Draws the HeadSelect panel cut (0, 258, 800x342) every frame at (0, 258).
* Ten ball slots whose top left positions are a table at `DAT_004c7be0` (10 x,y int pairs). Table contents UNKNOWN, but
  the art has room for two rows of five. Each slot draws a ball sprite at (x, y) and, if page + slot is below the head
  count, a second sprite at (x - 8, y) (presumably the face thumbnail, UNKNOWN which object).
* Slot hit test `FUN_004382f0`: `dist(mx - X - 60, my - Y - 60) < 60` for each of the ten slots (EXACT).
* Next page: centre (778, 363) radius 60 returns 10. Previous page: centre (18, 495) radius 60 returns 11. Else -1.
* Hover on next draws a highlight sprite at (707, 284). Hover on previous draws one at (10, 425). When the page is 0 a
  dim arrow sprite is drawn at (10, 425) (EXACT positions, DERIVED that these are the HeadSelect arrow cuts).
* Paging: left click on next adds 10 if the page start is below 70 (0x46), left click on previous subtracts 10 if above 0;
  then clamped to `[0, count - 10]` and not below 0.
* Return value: left click on ball i gives `page + i`. Right click, or any click with my < 258, gives -1 (cancel).
  Other results give 0 (no change).
* Head count per gender is `DAT_0059b76c[g]` (g = 1 male, 0 female, via `FUN_0046c940`). Starts at 19 (0x13) and grows by
  one for every custom head file found, capped by the test `< 0x48`.

Cuts on `HeadSelect.pcx` (EXACT, 41022 to 41043):

| Object | Rect (x, y, w, h) | Role (DERIVED from art) |
|--------|-------------------|-------------------------|
| 0 | 0, 258, 800, 342 | panel |
| 1 | 432, 105, 87, 152 | previous arrow, bright |
| 2 | 519, 105, 87, 152 | previous arrow, dim |
| 3 | 614, 0, 93, 152 | next arrow, bright |
| 4 | 707, 0, 93, 152 | next arrow, dim |
| 5 | 0, 0, 133, 137 | ball, normal |
| 6 | 150, 0, 133, 137 | ball, highlighted (yellow) |

The third ball at x 300 is not cut. Whether a hovered ball uses object 6: UNKNOWN (decompile does not show hover on slots).

## 2. Art cuts

### 2.1 `CGButtons.pcx` cuts (EXACT, 40969 to 41021). Objects are allocated contiguously, 0x2C bytes apart, in this order

| Objects | Rect (w x h) and origins | Count | Mark |
|---------|--------------------------|-------|------|
| O0..O5 | 112x35 at (0,300) (150,300) (0,350) (150,350) (0,400) (150,400) | 6 | EXACT |
| O6..O15 | 112x28 at (0,50) (150,50) (0,100) (150,100) ... (0,250) (150,250) | 10 | EXACT |
| O16..O19 | 35x37 at (0,0) (50,0) (100,0) (150,0) | 4 | EXACT |
| O20..O35 | 50x50 at x in {300, 350}, y in {0,50,...,350}, row major | 16 | EXACT |
| O36..O39 | 60x61 at (500,0) (600,0) (500,100) (600,100) | 4 | EXACT |
| O40 | 57x38 at (300,500) | 1 | EXACT |

Address check that ties the draw code to this order: the loader loops stop at 0x59E1E0 (end of O6..O15 starts there),
0x59E290, 0x59E550, 0x59E600; with 0x2C per object and the first object at 0x59DF20 these add up exactly. The draw code
starts its clothing toggle loops at 0x59DF4C and 0x59E054, which are the second object of the first pair in each group,
so the "on" sprite is the second of each pair (EXACT arithmetic, DERIVED reading).

Meaning of each pair, from the art (DERIVED):

* O0..O5 and O6..O15: pairs of (idle, on). Idle is cream (234,202,122), on is yellow (254,190,2). The right column
  (3 buttons, 35 tall) uses O0..O5. The left column (5 buttons, 28 tall) uses O6..O15.
* O16 and O18 hold the Load and Save icons in highlight colours. O17 and O19 are fully transparent (0 opaque pixels).
* O20..O35: pairs (idle at x=300, yellow at x=350). Rows 0..4 are the five right tabs (torso, people, head, shirt,
  pants), rows 5..7 are the three left round buttons (face, body, gender symbol).
* O36 is the undo ring highlight, O38 the OK ring highlight. O37 and O39 are fully transparent.
* O40 is a blue oval disc. Its use on this screen: UNKNOWN.

Idle versus hover:

* Idle state is already printed into the background sheet, the code never redraws the idle cuts (EXACT that no restore
  draw exists in the loop).
* Hover: the code draws one sprite at the position `DAT_004c7b90[idx]` (x, y shorts) for idx 0..19, except when the hit
  is -1 or 20 (EXACT, 32299 to 32302). Which object: UNKNOWN. Best guess is the yellow member of the matching pair.
* Clothing toggle "on" overlay: for each set bit of `DAT_00582dd1` (3 toggles) the sprite at `DAT_004c7b90[bit]` is drawn,
  and for each set bit of record byte 0x20 (5 traits) the sprite at `DAT_004c7b9c[bit]` is drawn (EXACT, 32692 and
  32730). Those two tables are the first 3 and the next 5 entries of the same position table (DERIVED from addresses).

### 2.2 Other sheets used by this screen

| Sheet | Cuts | Mark |
|-------|------|------|
| `HeadBodyBck.pcx` (115x229) | one cut (0,0,115,229). Drawn at (336, 20) for built-in heads and at (316, 20) for custom heads (the two branches at 32310 to 32331 differ only in this x). It is the sky and grass window. | EXACT cut, DERIVED role, shift reason UNKNOWN |
| Heads expression sheets, see 3.2 | 90x120 cells | EXACT |
| Heads halo pages, see 3.2 | 140x140 cells | EXACT |
| Bodies `*.pcx` | loaded as 60x120 objects: female order PLS, SSS, PSS, SkTT then the four `_sm`; male order PLS, KLS, PSS, SSS then the four `_sm` (40496 to 40580). `FemaleSkSS` is on disc but never loaded in the decompile. Whether this screen draws them: UNKNOWN. | EXACT load, UNKNOWN use here |

## 3. Data and index math

### 3.1 Character record, 0x230 bytes, array base `s_Golf_Pro_004d6088` (EXACT offsets, stride 0x230)

| Off | Size | Field | Mark |
|-----|------|-------|------|
| 0x00 | 16 | title ("Profession" edit) | EXACT |
| 0x10 | 16 | name ("New name" edit) | EXACT |
| 0x20 | 1 | trait bits 0..4 (five left column buttons) | EXACT |
| 0x21 | 1 | flag B: bits 0..2 age group (1 Young, 2 Middle Aged, 4 Mature), bits 3..6 marital (0x08 Single, 0x10 Married, 0x20 Divorced, 0x40 Widowed), bit 7 set = female | EXACT |
| 0x22 | 1 | head index | EXACT |
| 0x23 | 1 | high nibble bits 4..5 = body type 0..3, low nibble = fourth palette selector (0..9 in files), called "hat" in progolfers.dta | EXACT split, DERIVED name |
| 0x24 | 1 | shirt colour 0..9 | EXACT |
| 0x25 | 1 | pants colour 0..9 | EXACT |
| 0x26 | 1 | alternate skin palette index, written by the palette routine (4 when clothing toggle 2 is on, else equal to skin tone) | EXACT |
| 0x27 | 1 | skin tone 0..3 | EXACT |
| 0x28 | 1 | hair colour 0..4 | EXACT |
| 0x2C | 4 | flag dword: bits 0..2 = the three right column toggles, bit 3 = child, bit 7 = "edited" (palette values above are used only when the dword is non-zero) | EXACT |
| 0x30 | 512 | bio text, edited with the bio editor (limit 500 chars) | EXACT |

File check (disc .chr and .pro files): byte 0x2C is 0x80 or 0x8x in edited characters, 0x00 in untouched ones. Byte 0x21
bit 7 is set for female characters. Byte 0x22 values of 20 and above (for example 0x15) belong to files that carry an
embedded portrait. This agrees with the code (DERIVED). This corrects `FORMATS.md`: bytes 0x20..0x2F are not opaque,
and 0x30..0x22F is the bio text, not padding.

Dialogue slots: `DAT_00543d10`, 25 slots of 50 bytes per record (`record * 0x4E2 + slot * 0x32`) (EXACT).

Golfer slot skill bytes (slot `s`): `DAT_005795a8 + s*0x100`, 16 bytes, 10 used. The player's own live copy is
`DAT_005a5a04` (EXACT). Order is the 10 entry label table `PTR_s_Power_Hitter_004c2c3c` (EXACT count: addresses 0x4C2C3C
to 0x4C2C64 step 4). Names in file order from `progolfers.dta` notes: power hitter, long driver, accurate driver,
accurate irons, accurate putter, draw shot, fade shot, high backspin shot, recovery skills, luck (DERIVED, first name
EXACT from the symbol).

### 3.2 Head sheets (EXACT cut loops at 40264 to 40310)

Both genders have 19 built-in heads, `n = 0..18` (loop bounds give (0x565800-0x564E34)/0x84 = 19 etc.).

Expression sheet `Heads/sim_FEMALE_all_expressionsflat.pcx` and `sim_MALE_...` (1000x744):

```
col   = n >> 1
band  = n & 1
x     = col * 100
y0    = band * 372            (0x174)
row0 (happy)   : rect (x, y0 +   4, 90, 120)
row1 (neutral) : rect (x, y0 + 128, 90, 120)
row2 (angry)   : rect (x, y0 + 252, 90, 120)
```

Even n sit in the top band (10 heads), odd n in the bottom band (9 heads). The expression order happy, neutral, angry is
DERIVED from the art (row 0 smiling, row 1 flat, row 2 snarling) and from the portrait layout in `FORMATS.md`; the
decompile never names the rows.

Halo page `Heads/golfballhalopage_female.pcx` and `golfballhalopage_male .pcx` (1400x840): same indexing with 140 px
pitch:

```
x  = (n >> 1) * 140
y0 = (n & 1) * 420        (0x1A4)
cells at y0, y0 + 140, y0 + 280, each 140x140   (same three expressions, same order)
```

Custom heads (EXACT, 40310 to 40457): the engine lists `heads\*.pcx`, and every file whose name starts with F or f joins
the female list, M or m the male list, each 140x420 cut into three 140x140 cells at y 0, 140, 280. Counters start at 19 and stop at
0x48. On this disc that gives female 19 + 1 (`F_Head Template`) and male 19 + 5 (`M_Head Template`, `M_Lee`, `M_Meade`,
`M_pixelmonkey`, `M_Sid Head`). `Head Template.pcx` (320x420) and the sheets themselves do not match the prefix test.

Head index boundaries: built-in range is 0..18 (count constant 0x13) but the code treats `< 0x14` as built-in in save and in
the draw routine, and `< 0x13` when copying default skin and hair on pick. The first custom index is therefore either 19
or 20, an off by one that the decompile does not settle: UNKNOWN. (Disc files such as Der Bingle use 21.)

When a face is picked, if the index is below 0x13 the record's skin (byte 0x27) and hair (byte 0x28) are overwritten from
a per head default table with entry stride 0x44 (`DAT_004D55E8`, +0 for skin, +3 for hair); female uses entry
`index + 0x14` (EXACT, 33836 to 33846). Table contents UNKNOWN.

### 3.3 Preview drawing (what is visible)

Per frame (EXACT, 32310 to 32384):

1. If the child bit is set: draw an object at (352, 116).
2. Rebuild the preview palette with `FUN_00462020(0x99)` (virtual slot 0x99, palette object index 0x199).
3. Draw an object at (355, 116) (size matches a 90x120 head cell, so this is the head, DERIVED). Which head row (happy,
   neutral, angry): UNKNOWN.
4. Draw the window background object at (336, 20) or (316, 20) as in 2.2.
5. Draw an object at (299, 220). Role UNKNOWN.
6. Queue two small walking figures (`FUN_004628d0`): sprite anchor (315, 239) and (335, 239), box 30x40, zoom 4,
   depth 239, palette index 0x199. The sprites are `FUN_0043d6f0(set, 5, dir)` with dir 0 and dir 4: frame 5 of the
   normal walk animation, facing opposite directions (EXACT arithmetic: `base[a] + frames[a] * dir + frame`). Pixel top
   left of those figures depends on the zoom variable `DAT_004c2844`: x - (Z*30*4)/16, y - (Z*40*4)/16 for positive
   zoom 4 (formula EXACT, value of Z UNKNOWN).

Body animation set `a` in step 6 (EXACT, 32337 to 32361). Gender g from `FUN_0046c940`: 1 male, 0 female.

```
female (g = 0):
    a = 7
    if flag B has Young (bit 0)  : a = 6
    if trait bit 3 (0x08) of 0x20 : a = 8
    if flag B has Mature (bit 2) : a = 5
    if flag dword at 0x2C != 0    : a = bodyType + 5          (bodyType = byte 0x23 >> 4 & 3)
male (g = 1):
    a = 1
    if clothing toggle 2 (bit 4 of DAT_00582dd1) is on:
         a = clothing toggle 1 (bit 2) on ? 2 : 3
    if flag dword at 0x2C != 0    : a = bodyType
```

(the first branch, run when the dword is zero, picks defaults from age and trait bits; the dword override picks purely
from the body type button.) Mapping of `a` to body file comes from the animation loader (EXACT, 39924 to 40101):

| a | File family |
|---|-------------|
| 0 | MalePLS |
| 1 | MaleKLS |
| 2 | MalePSS |
| 3 | MaleSSS |
| 4 | not loaded in the body block |
| 5 | FemalePLS |
| 6 | FemaleSSS |
| 7 | FemalePSS |
| 8 | FemaleSkTT |

Animation id for a given body set is `animOffset + a` (offset 0 is the normal walk), spaced 10 apart per animation
(EXACT, from the `FUN_0043d740(offset, name, 8 frames per direction, ...)` calls where the base index is `offset +
DAT_00820b6c`).

Palette swaps, `FUN_00462020` (67008 to 67410) (EXACT facts only):

* Inputs: shirt (byte 0x24), pants (0x25), skin (0x27), alt skin (0x26), hair (0x28), plus the low nibble of 0x23.
  Female values are offset by 0x8B before indexing (the female palette objects start there), male values index directly.
* Values come from the record when the edited bit is set. When not edited they are derived from the golfer slot number:
  pants = slot % 10, shirt = (slot * 3) % 10 (male), shirt = (slot * 2) % 3 + 1 (female). Pro and celebrity slots read
  from their roster rows.
* Swap sources are palette objects loaded from `Bodies/MaleSwap01..10.pcx` and `FemaleSwap01..10.pcx` (40 files, 60x60,
  only the palette matters). Male objects start at `DAT_0081CA10` and female swaps at `DAT_0081FA30`, stride 0x58 (EXACT
  addresses in 39720 to 39804).
* Measured on disc (DERIVED): across the ten swap palettes, entries 0..39 and 60..79 take 10 different values, entries
  40..59 take 5 (matches the five hair colours), entries 80..119 take 6 or 7, and 144..175 take 3. The body sprites use
  indices 8..119 and 250. This points to groups of about 20 palette entries for shirt, pants, hair, skin and the
  fourth selector, but the exact start index of each group (the decompile shows 0x14, 0x28, 0x3C, 0x50 and others as
  garbled stack arguments) is UNKNOWN.

Unknown here: exact entry ranges per body part, which Swap file serves which of the 10 colour values (value 0 may be the
default palette, since the male object table loads `MaleSwap01` twice).

## 4. Name, title, text fields, tables and the file icons

### 4.1 Single line edit box `FUN_0045b2c0(prompt, x, y, maxLen)` (59267 onward) (EXACT)

* Box of width `maxLen * 12 + 32`, height 48 at (x, y), three nested rectangles (white offset (-1,-1), black offset (+1,+1),
  fill 0x35B3), prompt at (x + 4, y + 6), white input field at (x + 16, y + 23, maxLen*12, 22), text at (x + 20, y + 28),
  caret line drawn in 0x6000.
* Keys: Enter accepts, Esc empties the buffer, Backspace, Delete (0x2E), Home (0x24), End (0x23), Left (0x25), Right
  (0x27). Printable characters are accepted while the length is below `maxLen - 1`.
* Result is left in the shared buffer `DAT_0051A068`.

### 4.2 Fields

| Field | Call | Rules | Mark |
|-------|------|-------|------|
| Name | prompt "New name: ", box at (200, 32), max 16, buffer cleared first | stored only if the buffer is non-empty, written to record offset 0x10 | EXACT |
| Title | prompt "Profession: ", box at (200, 32), max 16, buffer cleared first | stored only if non-empty, record offset 0x00 | EXACT |
| Dialogue slot | prompt "New text: ", box at (300, (row + 15) * 16), max 48, buffer prefilled with the current slot text | no emptiness check: the buffer is always copied back, so Esc blanks the slot | EXACT |

Name box text positions (EXACT, centred text, centre x = 154): row 0 name (y 24, built by `FUN_004676e0(0x99,0)` plus a
child suffix when bit 3 is set), row 1 title (y 48), row 2 marital status (y 69), row 3 age group (y 90). Hover colours:
row 0 orange 0x7B20, row 1 white 0x7FFF, row 2 and 3 white, otherwise dark navy 0x0848.

### 4.3 Left and right columns (EXACT)

* Five traits: centred label at x = 93, y = 122 + 28 * i (label pointer table `DAT_004C2864`, 5 entries). Text colour
  dark navy when the bit is set, grey 0x4210 when clear.
* Three toggles: label first letter forced upper case (`-0x20`), centred at x = 212, y = 142 + 35 * i (pointer table
  `DAT_004C2858`, 3 entries, first label begins with "length", the others UNKNOWN). Same colours. When a bit is on the
  yellow sprite is drawn.
* Label texts of both tables: UNKNOWN except what is stated.

### 4.4 Bottom table (EXACT code, DERIVED row count)

For row r = 0, 1, ... while the byte table `DAT_004C2D10[r]` is not -1 (EXACT):

* row y = 271 + 16 * r (0x10F + 0x10 r).
* Left column text at x = 52 (0x34), left aligned: label `PTR_s_Signature_saying_004C2CC0[r]` followed by a suffix
  string. Row 0 is "Signature saying" (EXACT symbol). Other labels: UNKNOWN.
* Right column text at x = 412 (0x19C), left aligned: the dialogue slot `r` of this record. Colour dark navy 0x0848. If
  the slot is empty the code asks `FUN_00469b00(code, ...)` for a stock replacement line and draws it grey (0x4210).
* `DAT_004C2D10[r]` is an event code. `FUN_00469b00(code, ..., golfer)` loops over the golfer's 25 slots and uses the
  first slot whose code matches and whose text is non-empty, otherwise it falls back to a built-in switch (EXACT,
  74552 onward). Pick A Pro calls it with code 0x3E for the signature line, so row 0 holds event 0x3E (DERIVED).
* The art has 20 rows and the shipped files use 20 slots (`FORMATS.md`: slots 20..24 unused), so the table has 20 entries
  (DERIVED). The code loop has no fixed count, it ends on the -1 sentinel.
* Slot meanings by index (shot reactions, crowding, thirst, hunger, tiredness) come from `FORMATS.md`, not from the
  decompile (DERIVED, outside source).

Hover tooltips: after the pointer stays on the same hit index for more than 10 frames (`DAT_005AA554`), the generic tooltip
`FUN_00432620(mx, my)` shows the label from 1.2 (EXACT).

### 4.5 Pro mode panel (always shown for the player) (EXACT, 33012 to 33125)

* Heading "Golf Skill Levels" centred at (585, 30).
* 10 rows, i = 0..9, `y = 61 + 16 * i`.
  * Value box: black rect (498, y - 3, 42, 14), inner rect (499, y - 2, 40, 12) in 0x7D08 when the value is non-zero, in
    0x21E8 when zero.
  * Value text inside, left aligned at x = 501: prefix string + (value * 10) + suffix string, black. Drawn only when the
    value is non-zero. Prefix and suffix characters: UNKNOWN (single character data strings).
  * Skill name at x = 546, left aligned, black if value non-zero, grey 0x4210 if zero.
* Values come from `DAT_005A5A04[i]` when the char record is 0, otherwise from `DAT_005795A8[i + slot*0x100]`.

### 4.6 Skill editing rules and budget (EXACT)

* Edit is allowed only when mode is 1 and the char record is not 0 (`param_1 != 0`). The player (record 0) can never edit
  skills here, the panel is read only.
* For other records: left click adds 1 up to a maximum of 10, any other click subtracts 1 down to 0; hitting a limit plays
  sound effect 0x18.
* There is no points budget, no spending check and no running total anywhere in this routine. A budget, if it exists,
  is elsewhere. Values 0..10 and the "times 10" display are the only rules visible. The 16 row left and right columns,
  budget and skill ranges guessed in the task brief do not exist on this screen: the skill panel is the 10 row list above
  and the bottom table is the 20 row dialogue table.

### 4.7 Mode 0 panel (unreachable from current callers) (EXACT, 32767 to 33011)

Membership rank text centred at (580, 30) (six possible labels incl. Visitor, Member, Silver Member, Gold Member,
Platinum Member, Resigned), lines "Low round", "Handicap", "Rounds played" centred at x = 580, y = 50, 64, 78, the Update Bio link at (580, 98), and the bio text wrapped from (484, 114).

## 5. Flows

### 5.1 Entry and exit

1. Caller sets the golfer slot type to 0x20 and the three toggles (see section 0), then calls the screen.
2. On entry the routine copies the 0x230 byte record to a snapshot (`s_Swapper_004E1618`), plays sound 0x2D, sets up the
   preview slot 0x99 (record index in `DAT_00582E6E`, flags in `DAT_00582E70`, toggles in `DAT_00582DD1`), and loads the
   background by gender.
3. Loop: clear, hit test, draw hover, draw all text, process hover tooltip, flip buffers. A click outside the loop's
   draw runs the click switch, then waits for button release twice.
4. The loop leaves only through OK (idx 19). No Esc handling or right click exit is visible.

### 5.2 Undo (idx 18)

Copies the snapshot back over the 0x230 byte record. The loop continues. Dialogue slots, skills and the working toggle
copy are not part of the snapshot, so edits to the 25 slots persist after undo (EXACT, DERIVED consequence).

### 5.3 OK (idx 19) and what it writes (EXACT, 33911 to 33916 and 34164 to 34181)

* Writes the working toggles into `DAT_005794D1[slot*0x100]` (the golfer slot byte) and into the low 3 bits of the record's
  flag dword at 0x2C.
* If the golfer slot type is 0x20 and the record index is non-zero and the slot has a roster link (`DAT_00579573`
  not 0xFF): copies the look into the 0x38 byte roster row: [+2] = byte 0x23 low nibble, [+1] = skin (0x27), [+3] = shirt
  (0x24), [+4] = pants (0x25). For the player (record 0) this copy does not run.
* Restores sound state, plays sound 0x2D, frees the screen objects and returns. The record fields were already modified
  live during the session, so OK mostly commits the toggles and exits. The head index, colours and body type were
  written the instant they were clicked.

### 5.4 Face pick

Click idx 12, run the overlay, on a result other than -1 write byte 0x22. If the result is below 0x13, also reset skin
and hair to the head's defaults (3.2). Set the edited bit only for the colour, body, gender, child and picker
paths that the code shows (picker does not set the edited bit in the lines shown, UNKNOWN whether another path does).

### 5.5 Load (idx 8) and Save (idx 9)

File layout (EXACT, 31301 to 31806, matches `FORMATS.md`):

```
0x000  0x230 bytes   character record
0x230  0x4E2 bytes   25 dialogue slots of 50 bytes
0x712  0x10  bytes   four dwords: the skill bytes of the golfer slot (10 used, 6 spare), they are NOT padding
0x722  8     bytes   tag "*PCXFILE" (written by save, read and ignored by load)
0x72A  rest          8 bit PCX, 140x420, three 140x140 portraits stacked
```

Save (`FUN_00437910(record, slot or -1, flag)`):

* The path is "Themes\<current theme>\<character name>.pro" for the player customise screen (mode 1), ".chr" for mode 0.
  When a golfer slot is given (always for the player) the skill dwords come from that slot, and for record 0 from
  `DAT_005A5A04`. The name must pass the file name validity check `FUN_00405ac0`, otherwise a message about an invalid
  file name is shown. A second check opens the path for writing; failure shows an invalid path message.
* If the file already exists a yes or no box asks about overwriting; "no" aborts the save (returns 0).
* The portrait surface is 140x420, 8 bit, filled with 0xFF (transparent index). For a head index below 0x14 the three
  cells come from the halo page objects of that head (female array `DAT_00562918`, male array offset by 0x48 entries),
  stride 0x84 (3 objects), else from the custom head objects (`DAT_00563368` plus index - 0x14, same gender offset). Cells
  are blitted at y 0, 140, 280. Happy, neutral, angry order DERIVED.
* After a successful save the screen shows a notice "Character saved as Themes\<dir>\<name>.pro" with the folder, name and
  extension joined from the same pieces (the message text is longer than a UI label, so only its shape is given here).
* The Save icon calls `FUN_00437910(record, slot, 0)`. Slot -1 is used when mode 0 (EXACT).

Load (icon 8) then `FUN_00437fa0(fileName, record, slot or -1)`:

* The picker builds "Themes\<theme>\*.pro" (mode 1) or "*.chr" (mode 0), lists matching names (up to 100 byte entries in
  `DAT_0080B130`), shows a "Pick one:" list through `FUN_0046DE70` (up to 20 lines at the position given by (100, 20)) and
  loads the selection. If the folder has no match nothing opens.
* The loader reads 0x230 + 0x4E2 + 0x10 + 8 + the PCX in that order. For a record number at or above 0x4D the base path is
  "Themes\Standard\", otherwise the current theme folder.
* Skills are stored into the golfer slot (and into `DAT_005A5A04` for record 0) only when a slot was given and either the
  record is non-zero or bit 0x4000000 of the game mode word `DAT_0059E7B8` is set. For the player the skills are therefore
  applied only in that mode (UNKNOWN which game state sets it).
* If the loaded record has a custom head index (above 0x13) and there is room (count below 0x48) the file's portrait PCX is
  registered as a new custom head: the record's head index is set to the new count, the count is incremented, and the
  PCX is cut into three 140x140 cells at y 0, 140, 280 (EXACT).

## 6. Pick A Pro screen (`FUN_0043a8c0`, 34330 to 35137)

It is a file picker, not a built in roster. It lists `.pro` files of the Championship theme folder
("Themes\Championship\*.pro"); the routine temporarily overwrites the current theme name with "Championship" and
restores it on every exit (EXACT, 34364 to 34409 and 35066 to 35133).

### 6.1 Layout

Art `Title_Pickapro.pcx` (800x600) shows: round portrait frame on the left (portrait drawn at (69, 61), 140x140), a
10 row skill list under it (oval badge plus bar per row), a signature quote panel along the bottom left, a title bar
across the top right, one big cream list panel (about x 306..769, y 110..411), a scroll rail at x 778..790, a dark info bar
(about x 377..770, y 437..490, not written to in this routine, UNKNOWN use), and three round buttons at the bottom right
(check, folder with x, back arrow) (DERIVED from art).

Text positions (EXACT):

| Item | Position | Notes |
|------|----------|-------|
| Title "Pick A Pro" | centred at (504, 42), colour black | |
| List rows | left aligned at x = 320, y = 116 + 16 * i, i = 0..15 (16 visible rows) | selected row: bar (310, y - 1, 456, 15) colour 0x1284 and white text, others black text |
| Portrait | object drawn at (69, 61) | which expression cell: UNKNOWN |
| "<name>'s skills" line | left aligned at (56, 255) | name is the loaded character's name |
| "Signature saying" label | left aligned at (56, 544) | |
| Signature text | `FUN_00404AD0` at (36, 562), white | quote built from `FUN_00469b00(0x3E,...)` wrapped in quote marks |
| Skill rows | name at x = 94, y = 279 + 24 * i (grey-blue 0x0210), value at x = 59 (value * 10 with the same prefix and suffix as 4.5) | i = 0..9, values from `DAT_005A5A04[i]` |

The brief mentioned "10 name rows" and a "large text panel": the 10 rows are the skill list on the left, the large
panel is the 16 row file list on the right.

### 6.2 Hit areas (EXACT, `FUN_00467170 < r`)

| idx | Centre | Radius | Role | Sheet cut (idle patch from Title_PickAPro and hover from Title_LoadGame_MO) |
|-----|--------|--------|------|---------------------------------------------------------------------------------|
| 0 | (633, 555) | 26 | OK check | (600, 520, 70x70) |
| 1 | (769, 556) | 24 | back arrow | (740, 530, 60x60) |
| 2 | (784, 120) | 20 | scroll up | (778, 100, 22x40) |
| 3 | (784, 400) | 20 | scroll down | (778, 380, 22x40) |
| 4 | (704, 555) | 23 | delete (folder icon) | (670, 520, 70x70) |

Hover text: after 30 stable frames the label is drawn at (cutX + 30, cutY + 16), colour white. Labels: idx 0 "..." (an
empty string when nothing is selected, a short word when something is), idx 1 "Cancel", idx 4 "Delete" (EXACT). The MO
sheet has two extra cuts at (600, 435) and (670, 435), 70x70, which the code draws over the check and delete positions
whenever nothing is selected: they are the greyed disabled versions (DERIVED; the draw call's object is hidden).

### 6.3 Behaviour (EXACT unless noted)

* List scan each frame: count n = number of matching files. Scroll start is clamped to `[0, n - 16]`.
* Click in the list: `x > 320` and `y > 115`: row = (y - 116) / 16 + scroll. If row < n: select it and immediately
  load the file into char record 0 with `FUN_00437fa0(file, 0, 1)`, which fills the portrait and skills for preview.
  If row >= n: deselect (-1).
* Scroll buttons move by 4 rows with clamping. Mouse wheel or keys: UNKNOWN.
* OK: if a valid row is selected, draws "Loading..." at (388, 472), loads the file again for real and returns 1. If nothing is
  selected it returns 0.
* Back or the Esc key (0x1B) returns 0 after releasing the picker objects.
* Delete: a yes or no box (400 x 100) names the file and asks for confirmation; "yes" removes "Themes\Championship\<name>"
  with `FUN_004A64B8`.
* Hover tracking, 30 frame dwell for the tooltip labels, and a redraw of idle patches over the old hover target happen
  each frame.

### 6.4 How a pro is chosen and which portrait shows

A pro is whichever `.pro` file the player clicks. Loading puts the file's record, 25 dialogue slots, skill bytes and
embedded 140x420 portrait into the live player record 0 (so the picked pro becomes the player character). The portrait shown is
the halo page cell of the loaded head index if built in, or the custom cell registered from the file's PCX. Which of the
three expression cells is shown: UNKNOWN (the draw object is not visible in the decompile; the first cell is the likely
choice).

On this disc the Championship folder has one `.pro` ("Gary Golf") and Firaxis has "Golfmonkey". The listing only scans
Championship, so only the first is offered by this screen (DERIVED from the listing code and the file system).

## 7. Unknowns, collected

1. Exact hit centres and hover sprite positions (data tables `DAT_004c7b38`, `4c7b90`, `4c7b9c`, `4c7be0`).
2. Which sprite object each draw call uses (hidden register argument): hover sprites, the sprites at (352,116), (355,116),
   (299,220), the second sprite per face slot, the Pick A Pro portrait cell.
3. Label texts for the 5 traits, the 3 clothing toggles, rows 1..19 of the dialogue table, the tooltip text of idx 19 and
   the prefix and suffix characters around skill values.
4. Starting value of the body type counter (`uStack_2d0`, the decompiler shows it uninitialised before the first cycle,
   most likely the high nibble of byte 0x23).
5. Which preview expression is drawn on the customise screen (happy, neutral or angry).
6. Palette entry ranges per body part and Swap file to colour value mapping.
7. First custom head index (19 or 20) and use of the 60x120 `Bodies/*.pcx` stills on this screen.
8. Whether any keyboard exit (Esc) exists on the customise screen and which game state sets the 0x4000000 skill-apply bit.
9. Any skill point budget (not on this screen).
10. The unidentified round tabs (torso versus head silhouette to skin versus hair) and the use of the blue oval disc O40.

## 8. Resolved with the complete decompile and the exe's data tables

* Hit centres (0x4c7b38): toggles (210, 150 / 185 / 220), traits (93, 127 / 156 / 182 / 211 / 239), Load (48, 22),
  Save (258, 22), 10 body type (454, 30), 11 adult/child (454, 80), 12 face (454, 130), 13 shirt (454, 180), 14 pants
  (454, 230), 15 hair (328, 56), 16 skin (328, 107), 17 gender (328, 157), 18 Undo (762, 33), 19 Exit (760, 220),
  20 Update Bio (580, 98). Hover and lit positions (0x4c7b90): toggles (157, 135 + 35 k), traits (39, 115 + 28 k),
  Load (33, 7), Save (244, 7), 10..14 (436, 11 + 50 k), 15..17 (310, 37 + 50 k), Undo (743, 16), Exit (736, 192).
  The tooltip of 19 is "Exit".
* Face picker slots (0x4c7be0): (29, 305), (97, 425), (165, 305), ... alternating rows, 68 apart.
* Per head defaults (0x4d55e8, 0x44 an entry, men 0..19 then women 20..39): skin +0, hair +3, signature saying +4.
* Dialogue rows: labels 0x4c2cc0 (Signature saying, Made good shot, ... Found a bench) and event codes 0x4c2d10
  (3e 01 04 05 1f 02 03 09 0c 0d 1c 14 27 15 0e 19 0f 12 1a 1b, then -1). The label is drawn with "..." in the
  colour of the event's stock line; the right column shows the record's slot or, empty, the stock line in grey.
  0x469b00 uses a golfer's own non-empty slot for the event before its stock line (golfer slots below 0x98).
* The object at (299, 220) is CGButtons O40, the blue disc under the two walking figures (frame 5 of the walk,
  views 0 and 4, at (315, 239) and (335, 239)). The body still is the 0x5439e0 array by body type (+4 child) of the
  gender; the walkers' body set is the age/trait rule for women and the edited toggles for men.
* Mode 0 panel: level (Visitor 0x6318, Member 0x4210, Silver Member 0x2108, Gold Member 0x7ff0, Platinum Member
  white, Resigned 0x6000; a teal bar (500, 28, 160, 17) above Silver), "Low round: " / "Handicap: " / "Rounds
  played: " at (580, 50 / 64 / 78), "Bio" at (580, 98), the biography from (484, 114).
* Load lists `Themes\<pack>\*.pro` (mode 1) or `*.chr` (mode 0) in a "Pick one..." list (0x46de70 at (100, 20));
  Save writes the same folder; an existing file asks "<path> / already exists! / Overwrite the old version. /
  Cancel" (popup at (200, 30)); a saved one says "Character saved as / Themes\<pack>\<name>.pro" at (300, 100).
