# Information screens, part two: Histograph, Accomplishments, World map, Buy Land pricing, overview text

Companion header: `include/sg/ui_screens2.h` (namespace `sg::ui_screens2`, reuses the types and colours of `sg::ui_screens`).
Same method and conventions as `docs/UI_SCREENS.md` (read that first, section 0). Everything below was read from the publisher exe
decompile, its disassembly and its string table; no art is copied, the port loads sheets from the disc folder. Confidence marks:
**exact** = literal arguments or constants read from the code; **derived** = follows from code with a small inferred part;
**unknown** = not decoded. Routine addresses are given so the next reader can re-check.

Shared facts used below (exact): the game year is 8192 ticks, split into eight "months" of 1024 ticks named March to October;
year 1 is 2001; money text is `units * 100` with the section sign as currency symbol. Colour words are 5-5-5 under a `0x8000` flag.
Text call shapes: `0x404b70` centred, `0x4049d0` left aligned, `0x476700` the large title face.

## 1. Histograph (F3), routine `0x455ed0`

Key F3 selects it through the screen selector table at `0x4219b0` (exact). The screen is a modal loop like the other information
screens: it ends on any mouse click or the global key flag, and draws the OK hover piece while the pointer is inside the button box.

**Art (exact).** `infoscreens/histograph.pcx` (+ `histograph_alpha.pcx`), one 800x600 piece blitted at (0,0). Fonts: large face for the title,
body face for everything else.

**Fixed text (exact).**
* Title "HISTOGRAPH", large face, left edge at (375,20). The word is about 50 px wide so it centres near x 400 (derived).
* Legend under the plot, centred at y 542, black: "Skill" x 177, "Cash" x 332, "Fun" x 488, "Event" x 644.
* Axis labels: ten rows at y = 514 - 50 k for k = 0..9 (514, 464, ... 64). Left labels centred at x 77 in purple (`0x80004010`, the skill colour),
  right labels centred at x 727 in black.

**Plot (exact).** Clip rectangle (106,72,598,450). Baseline y 521 (`0x209`); a point of height h is drawn at y 521 - h. First x is 107.
There is one sample per game month. The x step per month is `clamp(600 / (months + 1), 1, 4)` with `months = tick >> 10`. For month m = 1..months the
screen draws a line from the previous point to the point of array element m (element 0 is skipped, the first segment starts at height 0) for each of four series. Arrays hold
500 shorts, written every month at slot `(tick >> 10) % 500` by the monthly update (`0x4179a0` and `0x417cc1`), so the plot can hold 500 months.

| Series | Array | Source (exact) | Colour | Height rule |
|---|---|---|---|---|
| Skill | `0x5409ac` | skill rating `0x541cd8` | `0x80004010` purple | v = value / skillScale, then `h(v)` |
| Cash | `0x51b388` | cash in units `0x571fd4` | black if the sample is >= 0, red `0x80007d08` if negative | v = value / cashScale (signed), then `h(v)` using the absolute value for v < 501 |
| Fun | `0x53e63c` | fun rating `0x59ae78` | `0x800003e0` green | `h(value)`, no divisor |
| Fourth | `0x520640` | count of staff records in the table at `0x5849e2` (stride 44) whose byte at +0x27 is not 0xff and whose type byte `& 7` exceeds 1 | `0x80000210` teal | 4 px per unit, no scale (the meaning "hired staff" is derived) |

`h(v) = v < 501 ? |v| / 2 : v / 10 + 200` (exact), so the plot is linear to 250 px, then compressed ten to one.
The cash line draws the absolute value for small values, so a negative balance is a mirrored red line (exact from the code, intent derived).

Scales (exact), chosen from the current value at the moment the screen opens:
* cashScale: 1; 2 when cash > 2500; 4 when > 5000; 10 when > 10000; 20 when > 25000; 40 when > 50000; 80 when > 100000.
* skillScale: 1, or 2 when skill > 2500.

Axis label text (exact): left label k is a plain signed number: `100 * t * k` for k <= 4 and `500 * t * (k - 4)` above (t = skillScale).
Right label k is the section sign, a number and "k": `10 * s * k` for k <= 4 and `50 * s * (k - 4)` above (s = cashScale). These match the height rule: tick k sits 50 px per step.
The number routine for the left labels is the plain decimal helper at `0x42dd50`; whether it inserts thousands separators is not read (unknown, derived plain).

**Event markers (exact layout, wording exact).** Array `0x568600` holds one short per month, `code | arg` with the code a multiple of 0x20 and arg in the low five bits;
the logger `0x40c6f0(code, arg)` overwrites the slot of the current month, so at most one event per month is kept. For each month with a non-zero entry the screen draws:
a 2x2 dot at (x + 10, skill point y) colour `0x80000210`; a stem at x + 11 up to the label height (`0x800003ff`); a second dot there; the label text left aligned
at (x + 14, 518 - labelHeight) in `0x80000210`. labelHeight = `(previous % 450) + 10`, so the labels stagger upward by 10 px per event and wrap after 450.

| Code | Text (arg in the low five bits) |
|---|---|
| 0x20 | "Hole N opened." |
| 0x40 | name from the building list at `0x4c26b0` (stride 20, "Pathway" first) + " built." |
| 0x60 | "Won match vs. " + opponent name from the table at `0x58dd50` (stride 56) |
| 0x80 | "Hole N rated top 100." |
| 0xa0 | "Hole N rated top 18!" |
| 0xc0 | owner name from the table at `0x55d738` (stride 37) + " buys a home." |
| 0xe0 | "Gary Golf places N in tournament." |
| 0x100 | pro name from the list at `0x4c2c18` (first entry "J.P.Bigdome") + " joins the board." |
| 0x120 | "Happy Ending." |
| 0x140 | "Additional land purchased." (logged by Buy Land, arg = tract) |
| 0x160 | "Ivana donates a " + landmark name built by `0x4074a0` |

The 0xe0 line uses the fixed player name "Gary Golf" for every tournament event (exact, the name is a literal in the routine).

**OK button (exact).** Piece drawn at (704,551); the pointer test is x 704..747, y 551..594. It is the OkStates hover piece (derived).

Unknown: whether the screen shows only the months already elapsed or also pads the right side (the loop runs `months` times, derived as only elapsed months); the texture of the empty plot is baked in the art.

## 2. Professional accomplishments (F10), routines `0x46f180`, `0x46e810`, `0x46e7b0`

F10 does not draw its own screen: the key handler at `0x41def5` calls `0x46f180`, which plays sound 0x7e, sets the class word to 99 and the delay counter to 20, runs the board
routine `0x46e810`, then resets both. The same routine is what pops up when a milestone is reached: `0x46e7b0(class, worldX, worldY)` records the milestone, and the board
opens 20 frames later with the new polaroid (the delay lets the live view be photographed). The popup plays sound 0x38 first (class 99 does not).

**Data (exact).** 22 milestone records, stride 48 bytes at `0x4c1578`: name (40 bytes), the tick at +0x28 (0 = not earned), the site index at +0x2c. A milestone is recorded only once (the
tick is written only if it is still 0, and not at all when a global flag bit 0x4000000 is set). Names, in class order:
0 1st Challenge hole, 1 1st Heroic hole, 2 1st skill upgrade, 3 1st Tournament, 4 1st Strategic hole, 5 First match victory, 6 First 9+ hole course, 7 1st Top 100 hole,
8 1st $500,000 Tournament, 9 1st Classic hole, 10 1st Top 18 hole, 11 First tournament victory (9+ holes), 12 1st Grand Slam course (9+ holes), 13 1st $1,000,000 Tournament,
14 First 18 hole course, 15 First tournament victory (18 hole), 16 1st Grand Slam course (18 hole), 17 to 20 Grand Slam Victory (Parkland, Desert, Tropical, Links), 21 1st 100 star rating.

Trigger call sites (derived, class passed literally, condition from context): classes 0, 1, 4, 9 on first hole of that kind (`0x...` calls at the hole rating code), 2 on a skill upgrade, 5 on a match win,
6 and 14 at the first 9 / 18 hole course, 7 and 10 when a hole reaches Top 100 / Top 18, 11 and 15 tournament wins, 12 and 16 grand slam courses, `17 + course type` grand slam wins, 21 at 100 stars.
Classes 3, 8 and 13 have no direct call in the code read; unknown.

**Art (exact).** Backdrop `Interface/bulletinboard&mantlewood.pcx` (the cork sheet exists beside it, loaded by other code), copied to (0,0) as a full 800x600 piece. Pieces cut from
`TrophyParts_A.pcx` (the plaque strips and tower) and `tacs&tees_A.pcx` (polaroid frame, labels). The non _A sheets have the same cuts and are loaded for another palette mode (derived).
Cut table (exact):
* Plaques, 248x29 on TrophyParts_A: classes 0..10 at x 9, y = 221 + 35 k; classes 11..21 at x 547, y = 221 + 35 k. Three alternative 248x30 strips at x 282, y = 186 + 35 k (indices 29, 30, 31) replace
  classes 0, 1, 4 when the edition byte `0x822c88` is below 2 (derived meaning of the byte).
* Tower pieces: base (283,357,248,146) drawn at (288,351); top (276,292,257,47) at (284, 304 - 14 n); two cap strips (9,116,248,30) at (288, 339 - 14 n) and (288, 335 - 14 n); a lower cap (9,186,248,19);
  foot (317,490,183,52) at (313,484); signs (510,13,132,162) at (519,395) and (649,13,130,163) at (177,395).
* On tacs&tees_A: polaroid frame (63,251,229,209); note card (58,49,207,178) at (600,350); corners (51,482,184,80) at (51,482) and (732,511,53,52) at (732,511); a second corner (668,511,53,52) cut only.
* Label strips for the to do list, one cut per class (x, y, w, h) on tacs&tees_A: see `labelCut` in the header (x 400 for classes 0..13 and x 610 for 14..21, widths 110 to 180, heights 18 to 40), plus three
  alternates for the low edition at (610,381,132,20), (610,415,126,20), (610,450,109,21).

**Layout (exact positions, roles derived).** Milestones earned, n in total, are sorted by tick ascending. For each (k = 0 oldest):
* polaroid x = 20 + (k odd ? 515 : 0) + (35 k mod 50), y = 443 - 14 k;
* frame piece at (x - 7, y - 173); the 200x160 snapshot copied to (x, y - 166); the pin piece (TacksandArrow_A, 0x5a4558) at (x + 100, y - 179);
* caption left aligned at (x, y), blue `0x80002108`: "<course name>  <day> <Month> <Year>": two spaces after the course name, one space after the day. Day = `((tick & 1023) * 30 >> 10) + 1`, Month = names March..October by `(tick >> 10) & 7`,
  Year = 2001 + tick / 8192. The course name is the site name stored with the record (exact calls `0x40daa0(0)` and `0x40d7b0`).
The tower: plaques are drawn newest first at x 288, starting y = 349 - 14 n, each older one 14 px lower, except that the step after the newest plaque is 16 px because the lower cap (at plaque y + 14) is drawn there; the base (and foot) are drawn last. The earned count n therefore moves the whole
tower up 14 px per milestone.
To do list (right side): the first three classes with tick 0, in class order, as label strips. Label j is drawn at (616 + off / 2, 400 + off) with off starting at 0 and increasing by (label height - 4) after each; the note card is at (600,350).

**Snapshot (exact).** At record time the world spot is converted to screen pixels; the crop origin is `(clamp(sx - 100, 0, 600), clamp(sy - 100, 0, 440))`, size 200x160, saved to `snapshots/accomp<class>.bmp`
(file name literal `snapshots\accomp`, `.bmp`); if the spot is off screen the camera is moved to its tile instead. The board reloads the files when it opens.

**After closing (exact flow, derived meaning).** When the pending-skill mask `0x4c2c9c` is non zero and the class is not 99, a dialog with the text "Add three skill points to your player...\n" opens on the player card ("Gary Golf" literal). OK close hit area is not a separate button here: any click closes (exact modal loop).

Other pieces (exact, 0x46e810 with the loader at 0x445ee5..0x4461c0): each polaroid's pin is TacksandArrow_A object 8, (100,0,20,24), at (x + 100, y - 179); the caption
is Arial Bold 10 (0x519fd8). The to-do strips are the handwritten lines of tacs&tees_A (table 0x4c2d38: x, y for 25 entries, then w, h for 25; entries 22..24 are the
easy editions' dogleg right, dogleg left and par five lines used for classes 0, 1 and 4). After the strips: TrophyParts_A (510,13,132,162) at (519,395) and
(649,13,130,163) at (177,395) (the two golf clubs), tacs&tees_A (51,482,184,80) at (51,482) (tee and ball) and (732,511,53,52) at (732,511) (the tick), then the tower.

Unknown: what writes the site index at +0x2c; the pin sheet piece used for the polaroid pin (position exact, identity derived); sound 0x38 meaning; the exact edition meaning of `0x822c88`.

## 3. World map / property chooser (F6, shift+w), routine `0x46f550`

F6 reaches this screen through the key table (`0x4219b0` / `0x4219f4`, F6 calls `0x407d30(0)`; the shift+w route was not traced separately). Before the game starts (tick 0) it is "Where will you build your golf course?"; later it is the course switch screen with Save / Load buttons.
The routine returns the chosen list slot, or -1 for cancel.

**Art (exact).** `Interface/WorldBase.pcx` is the backdrop (the world picture with the 16 pin positions); `WorldButton.pcx` holds the button strips: (250,408,76,114), (724,408,76,114), (550,458,76,64), (400,458,76,64),
(550,408,76,50), (400,408,76,50), (732,532,68,68). A 68x68 piece (732,532) is also cut from WorldBase.pcx. The pin and arrow pieces come from `TacksandArrow_A.pcx`, cells 20x24 at x = 50 * variant, y = 50 * state.
* Headline: "Where will you build" centred (126,19) and "your golf course?" centred (126,33). Cash text centred at (740,18); the text "Unlimited " + section sign replaces it in the unlimited-money mode.
* Legend row: pin pieces at (24,550), (124,550), (244,550) with labels "Available" (44,560), "Insufficient funds" (144,560), "Already purchased" (264,560), left aligned, black.
* Hit tests (exact): Cancel centre (768,557) radius under 25; Reset World or Save Game centre (760,476) radius under 25; Load Game centre (774,425) radius under 20. Labels "Reset World" (before play, regenerates the list), "Save Game" (after), "Load Game", "Cancel".

**Property records (exact).** 16 records, stride 130 bytes at `0x4c1e90`. Fields: region name (+0), course name after an order byte (+0x18), pin x and y on the world picture (+0x3a, +0x3c), course type (+0x3e: 0 parkland, 1 desert, 2 tropical, 3 links),
lie class (+0x3f: 0 inland, 1 coastal, 2 island), terrain (+0x40: 0 flat, 1 rolling, 2 hilly), bonus text (+0x41). The table is in the header (`kSites`). In this exe the region names are Monterey, San Diego, Rocky Mtns., Las Vegas, Phoenix, Hawaii,
Oahu, Nova Scotia, Northeast, Carolina, Ireland, Scotland, Wales, Spain, Florida, Jamaica, and the course names are Ocean's Edge, Dolphin Coast, Jurassic Springs, Ace in the Hole, Coyote Flats, Flamingo Shores, Island Palms, Windy Point, Ravenwood Farms,
Christmas Pines, County Kincaide, Harold's Keep, Thistle Runes, Sangria Bay, Ocean Grove, Scorpion Cove. The in game course name used by the date captions is the course name.

**List generation (exact flow).** `0x46f2b0` builds a list of 16 slots (stride 46 bytes at `0x571ff4`): slots 12..15 take one random site per distinct course type (a repeated type is rejected); slots 0..11 take random sites not yet listed,
and slot 0 must be a parkland site. Randoms use the game generator. Each slot stores site id, acres, terrain, lie, type, pin x and y, and two purchase markers (0xff = not purchased).

**Price and size by slot (exact).** Price in money units, cheapest slot first: 500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000 (so 50,000 to 1,000,000 dollars after the factor 100). Acres = `(slot + 4) * 10`,
plus 20 (slot over 3) or 10 (slot 3 or less) for an inland site, minus the same for an island site; 250 in unlimited-money mode. The sizes in `include/sg/properties.h` (Monterey 110 acres, Rocky Mtns. 180) agree with this rule at slots 7 and 12.

**Cards (exact positions).** 16 card origins, ring order, `cardPos` in the header: (251,13), (302,64), (346,115), (384,166), (416,220), (437,277), (454,337), (465,398), (469,458), (463,520), (466,13), (509,64), (546,115), (580,166), (606,220), (623,277).
Text offsets from the origin: name centred (+96,+4); bonus centred (+104,+16); then either the selected and affordable lines "Buy N acres of <flat|rolling|hilly>" (+112,+30), "<inland|coastal|island> <parkland|desert|tropical|links> property" (+106,+40),
"for only <section sign><price>." (+104,+50); or the compact line "N acres: <section sign><price>" (+112,+30) (blue `0x80002108`); a selected purchased card adds "Already Purchased." at (+112,+45).
A pin piece is drawn at the site's pin on the map and a matching piece beside the card at (+160,+10); the state column is 1 affordable, 0 insufficient funds, 2 purchased, 3 hover blink (derived from the loop).
The variant column of the pin is chosen from the pin position (left or right of x 200, above or below y 300) with small position offsets.
Hover: the nearest of the card centre (origin + 25) or the pin within 30 px (derived). A purchased card appends " - <year>" (purchase year + 2001) to its compact line, "????" if unknown (derived).

**Purchase flow (exact).** Click on an unpurchased card: if its price is more than cash and the unlimited flag is off, a dark red message box (position 200,200) shows "You need more money before you can purchase this property."; otherwise a confirm box
(`0x40cef0(204,276,192,48)`) then the text "We're off to <name>!" (white, centred near 300,285) and "... one moment please ..." (300,300), cash -= price, a bit `1 << slot` is set in the owned mask `0x543cfc`, and a sound
by course type plays: parkland 0x33, desert 0x78 + rnd(3), tropical 0x6e + rnd(3), links 0x73 + rnd(3). The routine returns the slot.
Save (button 1): a box (120,80,528,80) with "SAVE GAME: edit name then press Enter." white at (384,88) and an edit field "Save File Name..." at (128,102), 40 characters, default name = course name + date; on success "Game Saved." (green `0x80001284`).

Unknown: the exact hover rule (the loop compares card index with the site table), which code writes the purchase-year bytes, the exact pin variant offsets, and how the owned mask maps back to loading a saved course.

## 4. Buy Land: tract price and ownership, routine `0x4587a0`

The screen is entered from a key handler (`0x41e331`) after a permission check; it draws the course overview map as backdrop through `0x456be0` with the land mode flag set, then the cards. It returns 1 on a purchase, 0 otherwise; the caller then increments the **purchase counter** `0x53a450`.

**Tract data (exact).** The working tile map is 50 x 50, stored x-major (index = x * 50 + y). Unowned land is marked with tile code 0x14 and the real terrain is kept in a shadow array (`0x542414`).
Tract t (0..8) is a 16 x 16 block with map origin x = 16 * (t mod 3) + 1, y = 16 * (t div 3) + 1, so the nine tracts cover tiles 1..48. Tract numbers run top to bottom in the cards (card = column t div 3, row t mod 3),
while on the map they run left to right (exact from the two index expressions).

**Price formula (exact).** For tract t scan its 256 tiles; for every tile with code 0x14 count it and, with probability 1/3 (`rnd(3) == 0`), add `2^n` to a running sum f, where n is the low count in `0x53a450` (number of land purchases this game).
Start f at 5. Price in money units = `floor(f * 20 / 100) * 10` (displayed times 100). The random draws are made once when the screen first draws; the nine prices are then cached for that visit (a new visit rolls new draws).
Expected price for n = 0 with a full tract is therefore about 5 + 256/3 = 90, so 10 * floor(90 * 0.2) = 180 units (18,000 dollars), roughly doubling for each earlier purchase. A tract with no 0x14 tiles is "Already purchased." and cannot be bought.
Acres shown = for-sale tile count / 10 (integer).

**Card text (exact where listed).** Line "Buy tract #N" (blue `0x80002108`, left aligned at (cardTextX + 4, cardTextY + 2), card text origin (78|334|600, 62 + 68 * row)); then "<acres> acres of <a>, <b>, and <c>", naming the three most numerous terrain types in
the tract (names from the terrain table at `0x578350`, stride 48, with an alternate name at +0x10 for one terrain type; the table is built at run time, content not read: unknown), wrapped to 165 px; the price right aligned at
(cardX + 165, below the wrapped text) as "Price: " + section sign + money. The selected card shows the diamond highlight; map overlay digits 1..9 (large face) mark each tract at its centre tile.
Pick areas: card column left edges 15, 272, 538 (rows 54, 122, 190, pitch 68); the OK piece area is (662,533) 64x64, any other click is a no-op (sound 0x18); a right click or the OK closes with no purchase.

**Purchase effects (exact).** cash -= price; the current-year ledger short at `0x58421e + 20 * (year mod 100)` -= price (the Financial Report "Home Sites" or build line, derived mapping); each 0x14 tile of the tract is restored from the shadow terrain;
event 0x140 | tract is logged for the histograph; the caller adds 1 to the purchase counter. If cash is short the check `0x406c30` allows the purchase when price <= 0, price <= cash, the unlimited flag is set, or the course tier `0x5685f0` is below 4;
otherwise the dark red box "This change costs <section sign>X. You have only <section sign>Y." shows at (200,200) and the player stays on the screen.

**Other uses of the counter (derived).** The counter also scales a price shown in a message (a value times (counter + 3) times 50, derived use), a county commissioner threshold `(counter + 2) * (edition + 2) * 50`, a message variant selected by `counter & 0x7f`, and bit 0x80 is a flag cleared at some events. It resets to 0 on a new game.

## 5. Course overview texts (F5): Aura, Value, Routing, routine `0x456be0`

Mode word `0x4d20dc`: -1 routing, 1 aura, 2 home site value. Common (exact): "ROUTING MAP" centred (400,15); the course name centred at (400,48), black, drawn 10 px lower (y 58) when an internal flag is set (derived: a hole is selected).

**Aura mode (exact).** Heading "AURA" centred (400,85); lower label "COURSE AURA" centred (400,188); the explanatory paragraph (wrapped helper `0x478530`, left 250, top 110, width 300, shadow font): "Course AURA indicates where on your course
players have made mostly happy comments and where they have made unhappy comments."; legend "Unhappy" left aligned at (136,206) and "Happy" at (624,206).

**Value mode (exact).** Lower label "HOME SITE VALUE" centred (400,188); "Low" left aligned (142,206) and "High" (631,206). Two lists in the upper box, left aligned in black, rows y 86, 104, 121, 138, 156:
left column x 51: "Things which INCREASE home value:", "Close to water and trees.", "Close to a fun golf hole.", "Close to a top 100 or top 18 hole.", "Building a Marina" (the building name appended is the literal "Marina");
right column x 494: "Things which DECREASE home value:", "Close to an unfun hole.", "Close to another building.", "Too close to green, fairway, or OB.", "Far away from the golf course.".
The heading for this mode is blank (the subtitle line at y 85 is only drawn in routing and aura mode).

**Routing mode rows (exact).** Subtitle "COURSE ROUTING" at (400,85); help lines (400,108) and (400,126). Column headings at y 85: "Hole #" 71, "PAR" 132, "YDS" 182, "Time" 239 (left list) and 564, 630, 680, 737 (right list).
Rows: hole i = 1..18; left list holes 1..9, right list holes 10..18; row r = (i - 1) mod 9; text y = 104 + 17 r; column shift 0 or 498. Number centred at (71 + shift, y), PAR at (132 + shift, y), YDS (182 + shift, y), Time (239 + shift, y).
The right list number sits at 569, 5 px right of its heading (564). Empty slots draw grey (`0x80006318`), filled black. PAR = par byte, YDS = yards, Time = `(total ticks / rounds) / 40` followed by "m", blank until a round exists.
The selected row draws a green outer rectangle (shift + 32, y - 3, 238, 17) `0x800023e8` and a white inner (shift + 33, y - 2, 236, 15) `0x80007fff`, then the text on top. The hole record stride is 520 bytes (par at -0x20, yards at -0x1c, round count at +0, tick total at +0x1cc); the tee marker
for the selected hole is drawn on the map at the hole's coordinates with the iso transform (`0x456b70`).

**Map tiles (exact).** Each tile that is not out of bounds (type 20) is a 12 x 6 diamond at the iso point of `0x456b70`. Routing and employees: by class (the switch on the class byte). Aura: red = unhappy byte >> 3, green = happy byte >> 3, blue = (byte)(happy + unhappy) >> 4; water (type 17) 0x0218. The two byte maps come from `0x4616f0`, rebuilt every frame of the aura tab: nothing unless the landing counters (0x53ea24, as bytes) sum to 10 or more; the sum, capped at 1750, divides each tile's happy count (0x5a6378) times 20000 and unhappy count (0x56c7e4) times -40000; `0x4615f0` spreads two thirds of the value over the 15 x 15 tiles round it (skipping off-map and out-of-bounds tiles, `0x40bf60`), dividing by `0x467170(3 dx, 3 dy) + 2`, positive values onto the happy map and the rest subtracted from the unhappy map, each byte clamped 0..255. Home site value: q = clamp((lot value / 4 - site work) * 3 / 2, 0, 255), green q >> 3, with the lot value `0x42ef40` and the site work `0x40db90(a, b, 2, 5)`; dark red 0x2000 where that returns -1. The out-of-bounds colours in the code (0x2108, black) are unreachable. On a course where no rounds have been played both maps are black: no landings, and the lot value counts only holes with tee shots. A tree sprite lookup (0x19d, 0x197, 0x194, 0xf9 for class 13 tiles) is computed and never drawn.

**Hole marks (exact).** For each hole with a par: sprite `0x18d + clamp(par - 3, 0, 2)` (tee markers), its last frame, view 0, queued (`0x4628d0`) at the back tee's map point, box 40 x 40, zoom 4, palette 0x5e; a line (`Terrain::drawLine`, colour 0x7fff, width 2, last argument 10 not decoded) from the tee through the 250 yard marker (else the 200 one; markers at 0x59ae80 + 24 h, map units >> 10) to the pin; the hole number centred 4 px above that marker, or above the tee-to-pin midpoint when the hole has neither, in 0x7ff0 with the shadow style (`0x404bc0`) and the 14 point face, except on the employees tab; then the theme's flag `0x189 + theme`, frame 0, view 3, zoom 2, palette 0x60 + theme at the pin. Queued sprites are drawn after the frame's other drawing, in depth (y) order, at scale zoom x global zoom / 16 (the global zoom is 4 in play: full size and half size, derived).

**Employee figures (exact).** The list figure and the Employee panel portraits (0x436e50, box 30 x 40) are the employee's current clip: walk (0x20e) below state 11, stand (0x216) at 11, action (0x21e) at 12, plus kind + 4 x skilled; frame = frame counter mod clip length, view (camera - facing - 2) & 7, palette 0x82 + set, zoom 4; depth y - 32 on the map list. Both reset an action clip that has wrapped to standing (the port's tick already does).

**Employee panel text (exact, 0x436e50).** Centred at x 700, black: name (534, the face left from the frame, derived body face 0x51b360), "Hired: <Month> <Year>" (546) and "Paid: <dollars>" (558, grouped with commas, no currency sign) and the counter label (572) in 0x519fd8, the count (584) in 0x51b360. The routing list's line is "Hired: <Month> <Year>, paid: (section sign)<dollars>".

**Tab buttons, hover and clicks (exact, 0x457fb5..0x458188 and the loop after it).** Hover spots in test order: employees (62..141, 315..394) = 1, routing (183..262, 254..333) = 0,
aura (536..615, 254..333) = 2, value (659..738, 315..394) = 3, OK (662..741, 532..596) = -2, and in the employees mode with more than eight employees the list arrows
(775..792, 64..100) = 9 and (775..792, 205..241) = 10. Cuts of the bottom sheet (table 0x4ba698): the spot under the pointer draws its pale cut (0..3: (261,0) at (182,254),
(0,0) at (60,313), (396,0) at (483,254), (130,0) at (612,313)); the open mode draws its yellow cut (cuts 4..7, y 114/118, aura at (482,254)); OK hover cut 8 at (662,532);
arrow hover cuts 9, 10 at (775,64), (775,205). A click below y 280 switches to mode spot - 1 on a button, leaves on OK, else does nothing; then the arrows scroll the
employee list by one; then a click below y 256 leaves (so y 257..280 closes the map); otherwise (in every mode) the row is `clamp((y - 104) / 17 + 1, 1, 19)`, plus 9
when x > 399, clamped to 1..18: a left click selects it, a right click moves the selected hole there: the hole is held aside, the holes between shift one place
toward the old slot, the hole goes into the new slot and the tee and green tiles are renumbered (so it is a move, not the swap the help line names); the selection
keeps its number. Keys close. The map opens on hole 1 and the list top; the mode word persists.

**Employees mode (exact).** The course name plus " Employees" at (400,48) in the 16 point face 0x821f28 (Manual SSi 16; 0x821ee8 is Manual SSi 14, 0x821020 Klepto 24).
Listed: staff records in use with a negative job other than -6 (the player's pro); eight at a time, the first four in the left column. Entry v (1..8):
column shift 0 for v > 4, -374 otherwise; row offset 45 ((v - 1) & 3). The employee's figure at (746 + shift, 103 + row); "<n>. <name>" left at (415 + shift, 71 + row)
(n counts all listed employees, the name is 0x467600: first name and title, e.g. "Chuck Club Pro"); "Hired: <Month> <Year>, paid: (section sign)<money>" at (430 + shift, 86 + row)
from the month hired (+0x12, tick >> 10, set at hire 0x40acc0) and the wages paid (+0x14 in units, added at each wage event 0x417960); the counter line at
(430 + shift, 101 + row): Players greeted / Players cheered (Club Pro, Celebrity), Players rushed / Slackers intimidated, Weeds destroyed / Weeds eradicated,
Beverages served / Satisfied customers, then the count (+0x16). More than eight: the track (cut 11) at (775,64) and a white thumb x 781, w 6, y 106 + top * 95 / n,
h min(95 - that, 760 / n). On the map every staff record (the pro too) gets an ellipse round its post: 25 points, x = sin * r, y = cos * (r / 2), lines 2 wide, r 24 (48
groundskeeper), r += r / 2 when experienced; colours Soda Vendor 0x03ff, Groundskeeper 0x7ff0, Ranger 0x0018, Club Pro 0x6318, pro 0x7fff; then, except for the pro,
"<n>. " centred 10 px above the post and the name centred on it in Arial Bold 10 (0x519fd8), both in that colour.

## 6. Unknowns in one list

* Histograph: formatting of the left axis numbers (separators); whether the fourth series is exactly "staff"; hover pieces of OK.
* Accomplishments: writer of the site index (+0x2c); the three classes without a direct call (3, 8, 13); the pin piece identity; the edition byte meaning.
* World map: hover rule (card index versus site index), purchase year writers, pin variant offsets, loading saved courses from the owned mask.
* Buy Land: terrain name table content (built at run time); exact pixel position of the price line under wrapped text.
* Overview: the exact y of the "Marina" line relative to the rest (read: 156 for both columns, 0x9c) is exact; the employee mode texts were not re-traced here.

## 7. What the lead can wire (summary)

* Histograph: arrays of 500 shorts per series sampled once per month, the four series rules and colours, the scale functions, the event word table (`kEvents`), the plot box and clip.
* Accomplishments: the milestone table and polaroid layout, snapshot crop rule (200x160), 20 frame delay and the F10 entry (class 99).
* World map: cards, texts, price and acres rules, list generation, purchase effects.
* Buy Land: price formula (draws once per visit; counter n doubles the unit price per purchase), tract geometry and card text offsets, purchase effects.

## 3b. Port status: world map buttons, list generation and purchase flow

* Buttons: Reset World (before the game) or Save Game (course switch screen), and Load Game, are drawn from the WorldButton.pcx cuts (blue idle, yellow under the pointer) with the exe's circular hit areas. Cancel uses the exe's circle too.
* List generation follows the exe: each site holds a price slot; top four slots take one site of each course type, slot 0 is parkland, the rest are random. Price and acres come from the slot (acres use the site's lie class, 250 shown in unlimited mode). The default list is the one in the screenshots the chooser was first built from. Reset World rolls a new list (test hook `--worldreset SEED`).
* Purchase: an unpurchased card asks for a confirm (the wording and the Yes/No box are the port's own, the exe's box content is not decoded), then shows "We're off to X!" and "... one moment please ..." for two frames before the switch or the character screen. Too little money shows the dark red box at (200,200).
* Save Game saves to the single course file and toasts "Game Saved." The exe asks for a file name and lists files for Load; the port has one save slot.
* Still open: save file name prompt and file list, purchase year in the compact card line, pin pieces on the map and the hover rule.
