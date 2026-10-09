# DECODE_WORLD2: landmarks, amenities, employees, land, stories, saves

Clean-room fact record. Source: the text decompile (golf_decomp.c), disc text files, and earlier docs. Function names are
cited (FUN_xxxxxxxx addresses are stable). No code is reproduced. Tags: EXACT (read directly), DERIVED (with method),
UNKNOWN (not in the available text). Money is stored in units of $100 (EXACT). Time: 1024 ticks a month, 8192 ticks a year,
8 months a year (March to October), displayed year = (ticks >> 13) + 2001 (EXACT).

## 1. Landmarks

### 1.1 Kinds, names, sprites (EXACT, name function FUN_004074a0, sprite loader around FUN_0043d740)

Object record: type field 4 means landmark; the int parameter at +8 is the kind (EXACT). Sprite id = 0x168 + kind (EXACT).

| Kind | Name (UI text) | Sprite id | Flic base name | Class |
|------|----------------|-----------|----------------|-------|
| 0 | garden sundial | 0x168 | Sundial | scenic |
| 1 | traditional barn | 0x169 | Barn | scenic |
| 2 | Civil War cannon | 0x16a | civilwarCannon | scenic |
| 3 | ancient stonehenge rock | 0x16b | Stone_Two | scenic |
| 4 | operating water mill | 0x16c | wmillA | scenic |
| 5 | unusual rock face | 0x16d | ParklandRock | scenic |
| 6 | Civil War statue | 0x16e | civilwarStatue | scenic |
| 7 | scenic New England lighthouse | 0x16f | LighthouseC | scenic |
| 8 | peaceful Buddha | 0x170 | Buddha | scenic |
| 9 | Dutch windmill | 0x171 | Windmill | scenic |
| 10 | historic equestrian statue | 0x172 | equestrian | scenic |
| 11 | haunting Easter Island head | 0x173 | Easter | scenic |
| 12 | exquisite pagoda | 0x174 | Pagoda | scenic |
| 13 | historic Hatteras lighthouse | 0x175 | LighthouseB | scenic |
| 14 | ornate oriental house | 0x176 | ChineseHouse | scenic |
| 15 | dusty dinosaur tarpit | 0x177 | Tarpit | scenic |
| 16 | unsightly water tower | 0x178 | Wtow2 | ugly |
| 17 | unsightly radio antenna | 0x179 | Radio_Tower | ugly |
| 18 | unsightly oil pump | 0x17a | Red_Oil_Pump | ugly |

* Flics live in Flics/Landmarks/*.flc, palettes loaded as flics/Landmarks/<name>Pal (EXACT). A sprite 0x100 named
  Landmarks_Gold is loaded; its use is UNKNOWN.
* Kinds 0 to 15 are scenic (glance reaction 11, nice); 16 to 18 are ugly (reaction 20) (EXACT, glance routine in
  DECODE_EVENTS_NEEDS).
* Draw: FUN_00463180 case 4. Animation frame = (tile x + ticks) mod frameCount[kind]. Variant comes from the record
  rotation byte (+0xe) plus half of the camera rotation value DAT_005685f4; kind 3 uses the full value (EXACT).
* Ugly kinds are never in the player strip; the generator places them (see 4.4).

### 1.2 Effect rule (EXACT, FUN_00407000)

Each landmark carries one effect class: class = kind AND 3.

| Class | Label shown in tooltip | Bit | Consumer |
|-------|------------------------|-----|----------|
| 0 | Happy Golfers | 1 | none found in text (UNKNOWN; probably only the glance reaction 11) |
| 1 | No Dandelions | 2 | FUN_00467a00: a golfer bad reaction normally drops a dandelion (cell flag 0x4800); suppressed if a class 1 landmark is in range |
| 2 | Skill Upgrade | 4 | FUN_00409620: for the player's slot the random threshold is halved, then FUN_00407e00 (skill snapshot) is triggered (effect meaning DERIVED from use) |
| 3 | Happy Endings | 8 | pair state test near line 19457 sets chapter value 99; story progress near FUN_00466b70 is forced to maximum |

* Range test: for each type-4 record with kind below 16, distance from the golfer in range units (25 per tile, converted by
  FUN_0040c4b0 from a Euclidean distance FUN_0040acd0) must be below ((kind*5 + 40) * 5) / 3 (EXACT). Radius in tiles =
  that value / 25: kind 0 about 2.7 tiles, kind 15 about 7.7 tiles (DERIVED arithmetic).
* The function returns true when all bits of the requested mask were found. It also stores its result in a global that is
  never read (EXACT). A mask 6 call near line 8446 ignores its result.
* Kinds 16 to 18 never contribute (they fail the kind below 16 test) (EXACT).

### 1.3 Price, value, refund

* Tooltip price = (kind*5 + 25) * 200 dollars = 50 + 10*kind units (EXACT, tooltip code FUN_00434ac0, call near 28776).
  This agrees with EXE_COSTS and corrects UI_PANELS ("250 + 50*design units" is wrong).
* If the donated mask bit is set the tooltip shows the word FREE instead (EXACT).
* Tooltip also shows the design name in capitals and the effect label of 1.2.
* Removal refund: (kind + 5) * 10 units per EXE_COSTS (EXACT per that doc; not re-read here).
* Donated landmark stated value in the donation message: (kind*5 + 5) * 200 dollars (EXACT, FUN_0042dc00 call).
* Footprint and building table entry for type 4: see EXE_COSTS (building table 0x4c26b0).

### 1.4 Masks (EXACT)

* DAT_00543cfc: available landmark mask, 16 bits, bit = kind. The strip lists only designs whose bit is set (14 strip slots,
  base sprite 0x168; strip built by FUN_00432200, panel handler FUN_00434cf0, hit routine FUN_00434980).
* DAT_00822c70: free (donated) mask. Same bit meaning.
* A property purchase on the world map also ORs 1 << listSlot into DAT_00543cfc (FUN_0046f550 and its thunk). This means
  the same variable carries two meanings (landmark availability and purchased property slots). Whether that is a real
  overlap or a different variable sharing a label: UNKNOWN.
* Both masks are saved (FUN_0040afa0).

### 1.5 How a landmark becomes available or free

1. Heiress visitor donation (FUN_004266b0, roughly lines 16590 to 17198) (EXACT):
   * Refused if the visitor finished flag is set or the golfer mood is below 3. Refusal text paraphrase: she decided not to
     donate and will return when fun or skill rating exceeds (visits+1)^2 * 25.
   * Draw: i starts at the golfer mood. Repeat: kind = clamp(rand(i), 0, 15); stop if that bit is NOT set in the
     available mask; otherwise i increases by one while i < 25. If every try hits an existing bit, the last draw is
     donated anyway.
   * On donation: highlight FUN_0040c6f0 with code 0x160 and the kind, sound 0x2f via FUN_004481b0, the kind bit set in
     both masks, a message with the dollar value (1.3), and an explanation of where to place it (Landmarks under
     Improvements) plus one sentence by effect class (happy thoughts; no dandelions; the named player's skills improve
     rapidly; golfer stories proceed happily).
   * Visit counter DAT_0059aaf8 increments. Come-back threshold (count+1)^2 * 25 compared with fun rating, or skill rating
     when difficulty DAT_00822c88 is 2 or more (EXACT; the 2 or more reading is DERIVED from "fun if below 2").
   * Heiress spawn test near line 62309: (h+1)^2 * 25 < rating and the available mask below 0xffff (EXACT).
2. Story happy ending: FUN_004722c0(golfer, storyId), called from FUN_00466370 (line 69430). Switch on the first
   character of the story file name (table DAT_0053a454, 0x32 bytes per name) (EXACT):

| First letter | Kind granted |
|--------------|--------------|
| C | 0 sundial |
| P | 1 barn |
| A | 2 cannon |
| M | 3 stonehenge |
| L | 4 water mill |
| H | 5 rock face |
| G | 7 New England lighthouse |
| F, R | 8 Buddha |
| X | 9 windmill |
| S | 11 Easter Island |
| anything else | random 0 to 9 |

   It sets the bit in both masks and shows a message that the design is now in the landmark list.
3. Starting set: the initial value of DAT_00543cfc at a new game is UNKNOWN (set in a function not in the text).
4. Disc check: the first letter column agrees with the filenames (for example "CMMxxMxxMaleBonding" starts with C).
   (DERIVED by listing Themes/Standard and Themes/More_Stories.)

### 1.6 How a donated landmark is placed

* Donated means only that the free bit is set, so the price becomes zero in the tooltip (EXACT). Placement uses the same
  strip and click flow as bought landmarks (tool id 4). The click-to-place action path is not traceable: the queue writer
  is not in the text, only FUN_00409cf0 consuming codes with flag 0x100 via FUN_0040e000(x, y, type, flags) (UNKNOWN for
  the exact validity rules of a tile; DERIVED that the footprint comes from the building table).
* Whether placing a free landmark clears its free bit after use: UNKNOWN (no write to the free mask outside the donation
  and unlock paths was found).

### 1.7 Mask bit meanings, summary

DAT_00543cfc and DAT_00822c70: bit k = landmark kind k (k 0..15). Effect bit from FUN_00407000: 1 << (kind AND 3).

## 2. Amenity effects

State (EXACT): the Amenities panel offers only ball washer, path, home site, bench, bridge, landmark, flowers, tree, undo.
There is NO water cooler, trash can, restroom or shelter anywhere in this build.

### 2.1 Ball washer (building type 3, cost 50 units, footprint 1, sprite 0x183) (EXACT)

* Seek: a golfer at a tee looks for the nearest washer via FUN_0040ddb0(3, x, y) when its ball-washed flag 0x4000000 is
  clear and the washer is within distance 0xc00 (3 tiles) (lines 20431 to 20447).
* On arrival (lines 20739 to 20752): stop timer = -(12 + rand(8)) ticks, state 0x13, flag 0x4000000 set, sound 0x29, golfer
  record byte +0x3c increases by 3, washer cell animation counter reset.
* Accuracy: the next shot's signed direction error (DAT_005795a4) loses one third (error minus error/3). Golfers whose
  kind AND 0xe0 equals 0x20 skip the reduction (line 15243).
* The flag is cleared when the ball stops (line 21766), unless the resting tile id is 2.
* Panel text says a washer near the tee improves accuracy (EXACT). Upgrade art: Flics/Bldgs "upgrade bwasher.flc".
* Effect on needs or mood: none found beyond the +3 on the record byte (meaning of that byte: UNKNOWN).

### 2.2 Bench (building type 1, cost 2 units, refund base cost, five designs, sprites 0x208 to 0x20c) (EXACT)

* Detailed rules are in DECODE_EVENTS_NEEDS 3.4 and 3.5. Summary (EXACT per that doc): golfer needs fatigue above zero and
  strokes zero; bench accepted within 2 tiles (4 tiles with flag 0x2000000) in a 9 by 9 search window (FUN_0040de70);
  resting stop time about 0.8 * fatigue ticks; fatigue then set to zero; reaction 27 gives +1 mood only if fatigue was
  above 59.

### 2.3 Flower bed (tool id 2, five designs, base sprite 0x1a3) and tree (tool 0x10, seven designs, base 0x12f), bridge (tool 0x13, eight designs, base 0x226)

* Initial random variant: flower rand(4), tree rand(7), bridge rand(8) (EXACT).
* Flower bed sets cell flag 0x1000 (weedy flag is 0x800). The glance raises reaction 11 (clean) or 20 (weedy), only after a
  bad reaction (EXACT per DECODE_EVENTS_NEEDS).
* Radius for flowers, tree, bridge effects beyond the glance: UNKNOWN.

### 2.4 Facility visit ranges (EXACT per DECODE_EVENTS_NEEDS 3.14)

| Building | Range in tiles | Range with flag 0x2000 |
|----------|----------------|------------------------|
| Snack bar | 8 | 10 |
| Putting green | 5 | 7 |
| Pro shop | 5 | 7 |
| Driving range | 6 | 9 |

Income per visit (EXACT): snack bar 5 units; soda vendor sale 2 units; putting green 4 or 8; pro shop 6 or 10; driving range
8 or 12 (the lower value applies when the facility level DAT_005a8c50, DAT_005a8c58, DAT_005a8c60 is below 2).

### 2.5 Strip tooltips

Tooltip layout (FUN_00434ac0) shows price (or FREE), design name, effect label for landmarks. Unlock timing of buildings
by hole count is in 5.4.

## 3. Employees

Facts from EXE_COSTS and UI_PANELS are restated, with new reads.

* Maximum 20 employees (EXACT per UI_PANELS). Types: Club Pro, Ranger, Groundskeeper, Soda Vendor.
* Hire dialog returns kind*3 + 1 + skilled (EXACT per UI_PANELS).
* Wages per wage event, basic/skilled (EXACT per EXE_COSTS): Club Pro 3/7, Ranger 2/3, Groundskeeper 2/4, Soda Vendor 2/5
  (units). Wage period = 1024 / (difficulty + 2) ticks (EXACT per EXE_COSTS). Hole upkeep 1 unit per hole per period.
* Firing: a flat fee of 25 units is charged (EXACT, line 30432). Any other firing rule (minimum tenure, refusal): UNKNOWN.
* Soda vendor sale income 2 units (EXACT, line 1271).
* Employee panel actions: Move, Fire, Rename. Rename opens a prompt titled "Rename Employee..."; Move arms the next map
  click as the destination (EXACT per UI_PANELS). The mechanical placement rule (tile validity, whether Move costs
  anything): UNKNOWN. The pick-up of an employee from the map: UNKNOWN.
* Work radius per type, name generation list and algorithm: UNKNOWN (code not located in the decompiled text).

## 4. Starting land per property

### 4.1 Property list (FUN_0046f2b0, EXACT)

* 16 slots, stride 46 bytes at 0x571ff4. Slots 12 to 15 each take a random site of a distinct theme. Slots 0 to 11 take
  random sites not yet listed; slot 0 must be parkland (field value 0).
* Slot fields: site id, acres byte at +1, lie at +2, theme at +3, terrain at +4, map pin x and y shorts, purchase markers
  at +0xa and +0xb (0xff = unpurchased). Sandbox flag 0x1000000 stores acres 250 (0xfa).
* Price lookup FUN_0046f1d0(slot) returns the table value in units.

### 4.2 Site table (from earlier project docs and include/sg/ui_screens2.h; NOT re-verifiable in the decompile)

Course type 0 parkland, 1 desert, 2 tropical, 3 links. Lie 0 inland, 1 coastal, 2 island. Terrain 0 flat, 1 rolling, 2 hilly.
Format: region, course, type, lie, terrain, bonus.

| Region | Course | Type | Lie | Terrain | Bonus |
|--------|--------|------|-----|---------|-------|
| Monterey | Ocean's Edge | 0 | 1 | 1 | Scenic Cypress |
| San Diego | Dolphin Coast | 0 | 1 | 0 | Dolphins |
| Rocky Mtns. | Jurassic Springs | 0 | 0 | 2 | Free Hotel |
| Las Vegas | Ace in the Hole | 1 | 0 | 0 | Fun Fun Fun |
| Phoenix | Coyote Flats | 1 | 0 | 0 | Free Spa |
| Hawaii | Flamingo Shores | 2 | 1 | 1 | Scenic Waterfall |
| Oahu | Island Palms | 2 | 2 | 0 | Japanese Garden |
| Nova Scotia | Windy Point | 3 | 1 | 1 | Scenic Lighthouse |
| Northeast | Ravenwood Farms | 0 | 0 | 1 | Civil War Battlefield |
| Carolina | Christmas Pines | 0 | 0 | 0 | Free Putting Green |
| Ireland | County Kincaide | 3 | 0 | 2 | Leprechauns |
| Scotland | Harold's Keep | 3 | 1 | 0 | Free Castle |
| Wales | Thistle Runes | 3 | 2 | 1 | Stonehenge |
| Spain | Sangria Bay | 1 | 1 | 1 | Scenic Vineyards |
| Florida | Ocean Grove | 2 | 1 | 0 | Free Pro Shop |
| Jamaica | Scorpion Cove | 2 | 0 | 1 | Scenic Statues |

Tag: UNKNOWN as to the decompile (source is project docs, tagged by them as read from data). The unlock effect of the bonus
column in game code: UNKNOWN.

### 4.3 Price and acreage

* Slot price table in units: 500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000
  (EXACT per ui_screens2.h, consistent with FUN_0046f1d0 use).
* Acres byte = (slot + 4) * 10, plus 20 (slot above 3) or 10 (slot 3 or below) for inland lie, minus the same for island
  lie, coastal unchanged; 250 in sandbox (EXACT per docs and FUN_0046f2b0 sandbox branch).
* Purchase (FUN_0046f550): affordable if price is at most cash, or unlock counter DAT_005a6364 equals 0x11 (sandbox);
  message "We're off to <name>!"; cash falls by price; bit slot set in DAT_00543cfc (see 1.4); sound by theme: parkland
  0x33, desert 0x78 + rand(3), tropical 0x6e + rand(3), links 0x73 + rand(3) (EXACT).
* Tract (Buy Land) price: see UI_SCREENS2 (16 by 16 tract, f = 5 + sum of 2^n with probability 1/3 per for-sale tile,
  price = floor(f*20/100)*10 units, n is the land purchase counter DAT_0053a450) (EXACT per that doc).

### 4.4 Terrain generator (FUN_00470a60, lines 85521 to 86430, partially read)

Inputs from the slot (EXACT): site id, price slot, theme (DAT_00571ff8), lie (DAT_00571ff7), terrain (DAT_00571ff6).

* Base height DAT_004c2fa0: 0x30 flat, 0x20 rolling, 0x10 hilly; then +50 percent when slot/4 is 0, +25 percent when 1,
  minus 25 percent when 3, plus 0x10 in sandbox (EXACT).
* Base fill tile: rough (4) for slots below 8; slots 8 to 11 use 0xc unless theme is 1 (then 3); slots 12 and up use
  0xc minus (theme not equal to 1) (EXACT).
* Ridge and water strokes: budgets from 5000 to 45000 in steps of 0x9c4; tile choice alternates by theme; one cell in 64
  gets flag 0x100 (EXACT, tile lists per theme UNKNOWN in detail).
* Shoreline strips for lie other than island, tiles 0xb or 0x11. Scatter clusters of marsh (0x12), water (0x11) and
  others by theme. 16 random flagged cells.
* Unowned border: tile 0x14 stamped around a core derived from the acres byte by a loop that runs while acres*10 is below
  n*n*4 (lines 86365 to 86371). Owned region is a centred square; FUN_00470a10(x, y) writes the unowned tile. A shadow
  copy goes to DAT_00542414 (EXACT).
* Tile ids: 4 rough, 0xb brush, 0xd woods, 0x11 shallow water, 0x12 marsh, 0x14 for sale, 0x15 home site, 0x16 building or
  landmark (EXACT).
* Landmark seeding: ugly landmark kinds 16 (theme 0), 18 (theme 1), 17 (themes 2 and 3), count equal to difficulty
  DAT_00822c88 (lines 86331 to 86344). Scenic kinds 1, 2, 3, 4, 6, 7, 9, 10, 11, 12, 13, 14 are placed by the generator
  switch by site (EXACT that they appear; which site gets which: UNKNOWN).
* Map arrays 50 by 50, index x*50 + y: tile id DAT_005722e8, flags DAT_0053caf0 (shorts), elevation DAT_005a4998 (stride
  0x33), shadow terrain DAT_00542414 (EXACT).

### 4.5 New game start

* Clubhouse (type 0xf, flags 0x60) is created by FUN_0040e000 at a random cell in a 17 by 17 window near the centre,
  retrying until the tile id equals the base fill tile and the footprint fits (lines 85989 to 86001). Position stored at
  DAT_00578150 and DAT_00578154 (EXACT). Camera starts at about (x + 21, y + 11) (DERIVED from the loads, approximate).
* Hole 1 first waypoint start values in DAT_00585860 and DAT_00585861 (EXACT location, meaning DERIVED).
* Initial holes: unlock counter DAT_005a6364 is 6 at a normal start and 17 in sandbox (EXACT per PUBLISHER_EXE_NOTES).
  Whether any hole is pre-built: UNKNOWN.
* Cash: literal 1000 units assigned only in the championship loader FUN_0046ddd0 (which also sets ticks 0x2c00 and flag
  0x4000000). The normal new game cash assignment is in a function that failed to decompile. Earlier docs say 1000 units
  (DERIVED from championship loader and EXE_COSTS). Cash by difficulty: UNKNOWN. Difficulty selector FUN_0043a400 returns
  0 to 3; the store is not visible. Sandbox flag is 0x1000000 in DAT_0059e7b8.
* Purchases may go negative only when course grade (from DAT_005685f0) is below 4 (FUN_00406c30); otherwise the red box
  "This change costs X. You have only Y." appears (EXACT).

## 5. Tutorial, story and advisor triggers

### 5.1 Story file naming code (EXACT, FUN_0045de80, lines 61659 to 62000)

Files: Themes/Standard (about 10) and Themes/More_Stories (about 27), CRLF text, eight-letter code then the title.
Positions counted from 0, letters upper-cased:

| Pos | Meaning |
|-----|---------|
| 0 | setting letter, used for the landmark unlock (1.5); not tested at pairing |
| 1 | gender need for golfer 1 via FUN_0046c940 (nonzero = male): F female, M male, O opposite to golfer 2, S same as golfer 2 |
| 2 | marital need for golfer 1 via flag B bits: D 0x20, M 0x10, N 0x10, S 0x08, W 0x40; bit must be set; other letter = no constraint |
| 3 | trait bit test on golfer 1 profile byte: A 4, B 8, G 0x10, L 4, M 1, N 0x10, O 2, P 8, S 2, T 1; a set bit rejects in the golfer 1 branch (mixed rule). Trait names: UNKNOWN (profile byte offset 0x20 bits 0 to 4 are the five left-column trait buttons per DECODE_CUSTOMISE) |
| 4 | age relation via FUN_00453260: O golfer 1 at least 10 older than golfer 2; S similar (difference at most a third of golfer 1 age); Y at least 10 younger |
| 5 | gender need for golfer 2 (as 1) |
| 6 | marital need for golfer 2 (as 2) |
| 7 | trait test for golfer 2: A, N, O, P, T required; B, G, L, M, S excluded |

Flag B bits (EXACT per DECODE_CUSTOMISE): age 1 Young, 2 Middle Aged, 4 Mature; marital 0x08 Single, 0x10 Married, 0x20
Divorced, 0x40 Widowed; 0x80 female. Story text: a title line, optional author line, blank-separated blocks of one prompt
line and one to three indented replies; PARTNER stands for the other golfer (EXACT from disc files).

### 5.2 Pick and trigger rule (EXACT, FUN_0045de80)

* Up to 3 attempts. Each attempt picks a random story index in 0..99 with a non-empty name (up to 99 retries) that is not
  the opening story DAT_00838a9c, then applies the code tests above. A story already used by another pair (scan of
  DAT_00579568) is rejected. Success stores the story id in both golfers and sets flag 0x100000 on golfer 1.
* If the pair includes slot below 2 and ticks are below 0x800 (the first pair of a new game), the opening story is forced
  (GxxxxxxxOpeningDay is the disc file with that name; the mapping from DAT_00838a9c to it is DERIVED).
* A pair compatibility count (5 minus differing low flag bits of the two profiles) is computed; its use is UNKNOWN.
* The pair routine also prints a line starting "Your Fun Rating is up to ..."; the rest is unread.

### 5.3 Story progression (FUN_00466370, lines 69285 to 69450)

* Chapters 0 to 4 per golfer (DAT_0057956a). Dialog selection FUN_004668f0; a successful line adds 3 to record field +0x3c.
* Happy ending at chapter 4 for both golfers: theme sound (0x33, 0x78, 0x6e, 0x73 for parkland, desert, tropical, links),
  hearts counter DAT_00561258 +1, highlight FUN_0040c6f0(0x120, storyId), both golfers flagged 2 in the profile table, and
  FUN_004722c0 (landmark unlock, 1.5).

### 5.4 Advisor messages (FUN_0040e720 hole open handler; EXACT where text is read)

Message text is paraphrased; ids below are the unlock counter DAT_005a6364 values at which each building advice appears.

| Counter | Message topic |
|---------|---------------|
| 3 | a ball washer near the tee improves accuracy |
| 5 | a home site is a quick way to raise cash |
| 6 | putting green |
| 7 | snack bar (tutorial text also says golfers do not live by golf alone) |
| 8 | pro shop |
| 9 | swim club (players start in a better mood) |
| 10 | driving range |
| 0xb | cart garage (faster play) |
| 0xc | marina (home values) |
| 0xd | resort hotel (golfers tire less) |
| 0xe | airstrip (higher greens fees) |

* Building unlock by hole opened (PUBLISHER_EXE_NOTES): hole 1 none; hole 2 putting green; 3 snack bar; 4 pro shop; 5 swim
  club; 7 driving range; 8 cart garage; 9 marina; 11 resort hotel; 12 airstrip. Holes 6, 10, 18 upgrade the course via
  FUN_0040e5f0(0, 1, 2). Odd holes print "New players are flocking to your course"; even holes print "As your course grows
  you can add ...". Also prints hole name and dogleg direction (EXACT). The counter-to-hole mapping for the advisor table
  versus unlocks differs by offset (the table keys on DAT_005a6364, unlocks on hole number): DERIVED, unverified.
* Tutorial: FUN_004604f0 is the tutorial page player. Cases 0xb and 0x1d snapshot the game via FUN_0040b4a0 and print "Now
  back to your course". Case 0x15 is the tutorial start welcome. Case 8 snack bar, 9 benches, 10 a closing wrap-up.
* Complete list of advisor trigger ids, conditions and text keys: UNKNOWN (not read; no table found in the text).

### 5.5 SGA tournament offer

* Evaluation FUN_0044fb30; offer made when ticks mod 8192 equals 4096 (EXACT per PUBLISHER_EXE_NOTES). Year-end board rule
  FUN_0044cff0. Art Interface/infoscreens/SGA.pcx exists on disc.
* Popup layout (positions, button rects, text): UNKNOWN (not read).

## 6. Save and autosave

### 6.1 Save dialog (FUN_00405b10, EXACT)

* Box FUN_0040cc00(0x36, 0x50, 0x274, 0x50, 0): x 54, y 80, width 628, height 80. Prompt "SAVE GAME: edit name then press
  Enter." at (0x180, 0x58). Edit field at (0x40, 0x66), max 0x30 (48) characters.
* Default name by FUN_0040daa0(-1): course name (with argument 0 or 1 adds a tier word Municipal, Golf Club, Country Club or
  Championship), a separator, the day ((ticks mod 1024) * 30 >> 10) + 1, and FUN_0040d7b0(ticks) month name plus year.
* Validation FUN_00405ac0: rejects names containing any forbidden character (set includes < and >; the full set lives at
  0x4c3ee0, UNKNOWN in full), trims trailing spaces, requires length above zero. Existing file test FUN_004378a0; if it
  exists an overwrite confirm via FUN_0046d6e0(400, 0x8c, 1, 1, 0). Results: green toast "Game Saved." or "Invalid File Name".

### 6.2 File format

* FUN_0040b4a0(name) writes saved_games\<name>.sve, open mode 0x8301. First a 100-byte header text, then state from
  FUN_0040afa0(0) (lines 9806 to 9903): cash DAT_00571fd4, ticks DAT_00834170, unlock counter DAT_005a6364, land purchase
  counter DAT_0053a450, difficulty DAT_00822c88, masks DAT_00543cfc and DAT_00822c70, debt counter DAT_005a6374, object
  table DAT_0058bcb8 (0x1000 bytes), map arrays (0x9c4 bytes each), golfer table DAT_005794b8 (0x9c00 bytes), property list
  DAT_00571fd8 (0x30e bytes), history ring DAT_00584210 (2000 bytes) (EXACT).
* Load: FUN_0040b840(mode) (mode 0 full with "Loading Course" text; 1 to 3 header only); FUN_0040b9b0(name, restore,
  headerOnly, championship) loads saved_games\<name> or Themes\Championship\<name>. It keeps the theme byte, preserves bit 4
  of DAT_0059e7b8 and sets 0x40000000; when restore is set and the name is not "While Browsing.sve" it shows "Loading Game".
  FUN_0040bbf0 is the partial loader (omits the debt counter among others) (EXACT).

### 6.3 Load Previous Game browser (FUN_0043b610)

* Called mid-game it first saves the current game under the name "While Browsing" (FUN_0040b4a0). File listing FUN_0043d2a0
  skips names containing "Shadow" or "shadow", keeps about 100 names of 100 bytes, excludes "While Browsing".
* Entries beginning with '&' display as the literal "autosave: " followed by the header text read from the file via
  FUN_0040b9b0(name, 0, 1, 0). Other entries show the filename. Cancel reloads "While Browsing.sve". Delete uses a confirm
  ("Yes, delete this file" / "No, never mind") and FUN_004a64b8 (EXACT). Layout: DECODE_TITLE2 section 1.

### 6.4 Who writes '&' names (UNKNOWN)

* Callers of FUN_0040b4a0: save dialog, While Browsing (line 34911), tutorial snapshot (line 65016, name literal at 0x4d44fc,
  unreadable), world map Save Game (lines 83804 and 85217). No autosave writer was found: autosave trigger, slot naming
  and rotation: UNKNOWN. The tutorial literal probably starts with '&' (DERIVED from the browser display rule, unverified).

### 6.5 Year end flow and debt ladder (FUN_0044cff0, lines 43943 to 45649)

* The "END of YEAR" screen shows This Year and Last Year columns and highlight events (0x100 joins the board, 0x120 Happy
  Ending, 0x140 additional land purchased, 0x160 heiress donates a landmark) (EXACT).
* At the end, if cash DAT_00571fd4 is below zero and the sandbox flag 0x1000000 is clear, counter DAT_005a6374 steps:
  0 to 1 (board concerned, two years to recover), 1 to 2 (very worried, one more year), 2 to 3 (board terminates the
  contract). If cash is zero or more, the counter resets to 0 (EXACT). Year end occurs every 8192 ticks (EXACT).
* Game over action after counter 3 (end screen, forced load): UNKNOWN (caller failed to decompile). The port's economy
  already uses a three-stage ladder; the 30 day grace in docs/EDITING.md is a placeholder and should be replaced by this
  year-end ladder (DERIVED recommendation).
* Interest on negative cash: cash/50 monthly (EXACT per EXE_COSTS).

## 7. Corrections to earlier docs

* UI_PANELS landmark cost: use 50 + 10*kind units, not 250 + 50*design.
* Heiress draw is exact (1.5), not approximate.
