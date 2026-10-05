# Home sites: sale, price, growth, value and messages (from the publisher's exe)

Facts only, in our own words, from the publisher-supplied unprotected exe (decompile plus disassembly for the large UI function that the decompiler dropped).
Tags: EXACT (code or table read directly), DERIVED (shape read, meaning inferred), UNKNOWN (read but purpose not settled).
Money is stored in units of $100; the screens multiply by 100 for display. A month is 1024 ticks, a game year is 8 months (8192 ticks).
Building records (16 bytes at 0x58bcb8, 256 slots) are described in DECODE_BUILDINGS.md. Fields used here for type 5 (home site):
+0 type, +2 x, +4 y, +7 flags, +8 buyer field (word at 0x58bcc0 + 16*i), +0xc value field (0x58bcc4 + 16*i), +0xe variant byte.

Key routines (addresses): lot value 0x42ef40 with per-tile term 0x42ee80, placement click 0x41f34e to 0x41fc7c, demolish 0x40e400, record create 0x40e000,
monthly home pass inside the main tick 0x417973 to 0x417c3e, house lot renderer 0x4652a9 to 0x46550b, sold-house yard drawing 0x4012d0, resident walker spawn 0x4011e0,
resident walker removal 0x4011b0, resident walker AI 0x4017d0, free-site counter 0x42dea0, Home Site Value overview 0x456be0, year-end highlights 0x44cff0,
golfer glance code (decompile lines 15600 to 15800), dead vacation-home branch 0x42710e to 0x427345 (inside 0x4266b0).

## Headline findings (read these first)

1. EXACT: the buyers are not golfers. Home sites are bought by random CELEBRITIES from the celebrities.dta table (up to 100 records, 21 on the shipped disc), chosen by the monthly home pass.
2. EXACT: the golfer-tier code that says "has decided to buy a vacation home" (inside the special-visitor routine 0x4266b0) is unreachable. It sits behind a test that masks a byte with 0xe0 and then compares the result to 0x100, which can never be true. No code anywhere writes -1 into a home site's +8 field either.
   So the string at line 17458 and the "find a type-5 record whose +8 is -1" search never run in this build. DECODE_SOCIAL.md already lists the international celebrity visitor as unreachable; this confirms it and shows what replaced it.
3. EXACT: the club is paid once, at placement (a quarter of the lot value, booked in Home Sites). A celebrity purchase pays nothing; it only changes +8, shows a message, logs a highlight, spawns a resident, adds a score point and lets golfers glance at it for +1 mood.

## 1. Who buys, when, and how a site is chosen

### 1.1 Gating of the Home Site tool (EXACT unless marked)
- Counter at 0x56d1b0 ("free sites", recomputed every frame by 0x42dea0): count of members in the 76-record roster table at 0x5849e0 (stride 0x2c) that are not resigned
  (record byte +0x29 is not 0xff) and whose status (record byte +2, low 3 bits) is 3 or more (Silver, Gold or Platinum; Member is 2, Visitor is 1), minus the number of home site
  records in the building table (sold or unsold, every type-5 record counts).
- The tool is unusable while the counter is below 1 (greyed third cut). If tool 5 is armed and the counter is 0 or less the tool is cancelled. So one home site per Silver-or-better member.
- The same counter is printed as "Waiting list: N" (this defines the roster's waiting list that DECODE_SOCIAL left open) and is drawn as a row of up to 9 small icons or "x N" in the rating panel (0x4193b0). EXACT for the counter, DERIVED for which panel shows each form.
- The member promotion message at status 3 contains the home-site hint (section 4.4).
- DERIVED link to membership growth: in the end-of-round membership application rule (0x428698 to 0x4286bb) a golfer whose roster flag bits (&3) are zero, on difficulty above 0 and year above 0, does NOT get the usual +1 toward upgrading when
  (cash is at least 200 units or the waiting list counter is above 0). The meaning of the other inputs to that score is not decoded here.

### 1.2 Placement conditions (EXACT)
- Placement uses the common building path (price = base 10 + site work; see DECODE_BUILDINGS.md sections 2 and 3). Extra conditions for type 5: lot value (0x42ef40, section 3.1) must be at least 50 units, otherwise the text in section 4.1 is shown, nothing is placed and nothing is charged.
- New record: +0 = 5, +2 x, +4 y, +7 = 0, +8 = 0, +0xc = 0, +0xe = (a rolling global byte 0x5a34f0) and 3. Footprint 2 by 2. The stamp writes tile id 21 and the building tile flag.
- The record is never given the connected bit: the flood fill (0x42f340) only touches types 6 to 16, and the per-frame level arrays skip type 5. A home site needs no path to the clubhouse. EXACT.
- No construction animation, dust effect or sound is started for a home site (that block only runs for types 4, 7, 8, 9, 11, 12, 13, 14 and 15). EXACT.

### 1.3 State flags of a site (EXACT)
- For sale (unsold): +8 == 0. Includes every freshly placed site.
- Sold: +8 != 0. Live code stores (celebrity table index + 1), 1 to 100. The dead golfer branch would have stored a golfer attribute value masked to 0x1f; nothing stores -1.
- There is no separate "under construction" state or timer. What the player sees as sign, construction and finished house is a pure function of +0xc (section 3.1) while +8 is 0.
- +7 stays 0 (no connected bit). +0xe is unused by the home renderer (DERIVED).

### 1.4 The monthly home pass (EXACT)
Every tick the main step tests tick % P == 0 where P = 1024 / (d + 2) (integer division) and d is the global at 0x822c88 (called difficulty, 0 easiest, in the other docs).
When true, let i0 = (tick / P) mod (d + 2). The pass visits records i0, i0 + (d + 2), i0 + 2(d + 2) and so on while the index is below 256. Over d + 2 consecutive periods every record is visited once, so each record is visited about once per month (1024 ticks; exactly 1024 for d = 0, 1 and 2... P*(d+2) is 1024, 1023, 1024, 1020 for d = 0 to 3).
DERIVED quirk: in the fast-forward state (flag 0x8 of 0x59e7b8) the tick is forced even, so a period P that is odd can miss its odd multiples.

For each visited record of type 5, in this order:
1. Value update (all type-5 records, sold or not): V_new = lot + V - trunc(V / 12), where lot is the lot value of the site (section 3.1) and V is the +0xc field. Steady state is V = 12 * lot (from below it reaches exactly 12 * lot).
   Special case: if bit 7 of the byte at 0x56c7b4 is set, V is forced to 2500 and the bit cleared. Nothing in the exe sets that bit (UNKNOWN, probably a debug or cheat hook).
2. Candidate: draw random r in 0..99 until the table entry 0x55d738 + 37 r is in use (its type byte at +0x20 is not 0xff). Uniform among loaded celebrities. A celebrity can buy many sites; there is no ownership check. EXACT.
3. If +8 is already nonzero, stop (already sold).
4. Threshold test: sale needs V strictly greater than (300 N + 400) * (d + 2), where N is the global counter at 0x4c284c (number of sales so far in this game, never decremented, saved in save games, reset to 0 on a new game).
5. The sale message (section 4.2) is posted through the ticker routine 0x40cb00 with priority 0. That routine refuses (returns 0) when a message is already showing or pending, or when the mode word 0x567afc equals 3. A refused post means no sale this pass; the site is retried one month later (V was already updated).
6. On a successful post: +8 = r + 1; highlight record (0xc0 or r) logged for the current month; sound 0x33; N += 1; a resident walker is spawned (0x4011e0, section 3.3).

Sale threshold table (units of V; compare with 12 * lot): d = 0: 800, 1400, 2000, 2600, 3200, 3800, 4400 for N = 0 to 6; d = 1: 1200, 2100, 3000, 3900, 4800, 5700, 6600; d = 2: 1600, 2800, 4000, 5200, 6400, 7600, 8800; d = 3: 2000, 3500, 5000, 6500, 8000, 9500, 11000.
Smallest lot value that can ever sell (12 * lot above the threshold): d = 0: 67, 117, 167, 217, 267; d = 1: 101, 176, 251; d = 2: 134, 234, 334; d = 3: 167, 292, 417 (for N = 0, 1, 2...).
Months from placement to sale on d = 0, N = 0 (DERIVED simulation of the exact recurrence): lot 67 about 53 months, lot 80 about 21, lot 100 about 13, lot 150 about 7, lot 200 about 5, lot 300 about 3, lot 500 about 2. Lot 66 or less never sells at N = 0, d = 0. On d = 3, N = 0 lot 167 needs about 63 months, lot 200 about 21.
So the effective rule is: a site sells only if 12 * lot value exceeds the threshold, sooner the more valuable the lot, and every sale raises the bar for all later ones by 300 * (d + 2) of V (25 * (d + 2) of lot value).

### 1.5 Tiers and probabilities
- There is no tier test and no per-golfer roll in the live path. The only randomness is which celebrity (uniform) and the order in which records are visited (fixed). EXACT.
- Dead branch (UNKNOWN intent, for completeness): for a golfer whose attribute byte 0x5794d0 masked with 0xe0 would equal 0x100 the code tests a target-hole record byte equal to 0 and a golfer progress short above 1; yes sets the site buyer field, no prints the "decided not to buy" text of section 4.5.

## 2. Price and booking

### 2.1 At placement (EXACT)
Let price = base cost 10 + site work (DECODE_BUILDINGS.md section 2; type 5 has level 0, so base * (0 + 2) / 2 = 10), lot = lot value of the clicked tile.
- Affordability is checked against the full price.
- The Facilities ledger cell (0x584218 + 0x14 * (year % 100), row k4) is debited by the full price, and the running total at 0x578338 is increased by the full price (UNKNOWN what reads that total).
- club share = trunc(lot / 4) (round toward zero). Net cost = price - club share. The Home Sites ledger cell (0x584212 + 0x14 * (year % 100), row k1) is CREDITED by the club share.
- Cash (0x571fd4) is reduced by the net cost, which is negative (cash goes up) whenever the club share is larger than the price. A floating amount equal to minus the net cost is shown at the tile.
- Ledger row mapping used here: k0 Greens Fees 0x584210, k1 Home Sites 0x584212, k2 Food/Drink 0x584214, k3 Build course 0x584216 (terrain), k4 Facilities 0x584218, k5 Salaries 0x58421a, k6 Maint./Interest 0x58421c, k7 Other 0x58421e, total 0x584220. This also settles the writer of k4: every building placement debits it.
- The placement preview shows: "Lot value:" = trunc(lot / 4) * 100, "Site clear:" = site work * 100, "Site prep.:" = 10 * 100, and "Profit:" = (trunc(lot / 4) - 10 - site work) * 100, with a "$" prefix; the Profit text changes colour when it is 0 or less. EXACT for the quantities, DERIVED for the colour meaning.

### 2.2 At sale (EXACT)
- Nothing is paid to the club, nothing is booked. The celebrity purchase is event and score only.
- Per period or per month: no income from sold or unsold homes. V is only used for drawing and for the sale test.

### 2.3 At removal (EXACT)
- Cost = trunc(lot / 2) + trunc(+8 / 50), where lot is recomputed at removal time. Since +8 is at most 100, the second term is 0 for +8 below 50 and 1 for 50 to 99 (celebrity table indexes 49 to 98), 2 for 100. Looks like a bug (the value field +0xc was probably intended); read as stated. UNKNOWN intent.
- Cash is reduced by the cost, the Home Sites cell is debited by the cost, a floating "-cost" is shown, and if +8 was nonzero the resident walkers standing on that site are freed. No refund of the earlier share. N is not decreased.

## 3. Growth, size classes and value feedback

### 3.1 Lot value (0x42ef40, EXACT)
lot = trunc(M * R / 40), where
- R is the sum of the per-tile terms of the 12 tiles in the ring around the 2 by 2 footprint (x from X-1 to X+2 on rows Y-1 and Y+2, plus columns X-1 and X+2 on rows Y and Y+1). First coordinate is the 50-wide index used by the tile array.
- Per-tile term (0x42ee80): 0 if the tile is off the map or has tile id 20 (out of bounds, unowned). Then by tile id: 4 rough +12; 21 home site tile -16; any tile whose hazard byte (terrain table +0x22) is 0 or less (tee 0, green -1, fairway 0, firm fairway 0) -8; 17 water +32;
  18 wetlands +16 only on theme 1 (desert); tree class (ids 13, 14, 15, 16, table +0x26 = 13): trunc(cost byte * 5 / 2) = 25 for tree, pine, palm and 62 for elm; every other tile: trunc(cost byte * hazard byte / 2), i.e. deep rough 2, mound 4, sand trap 9, waste bunker 4, pot bunker 20, ravine 25, brush 6, rocks 6, wetlands 3, marsh 9, building tile 22 0.
  The cost byte is terrain table +0x23 (units of $100 build cost) and the hazard byte +0x22, with the runtime table at 0x578350 (23 entries of 0x30 bytes copied from 0x4c1a40).
  Only the home-site tile counts as "another building"; the ordinary building tile (22) scores 0.
- M is the best "hole score" over the 18 hole slots, floor 0: for each hole slot that exists (record byte +0x10 nonzero, 0x575cb8 stride 0x208) and has a nonzero play counter (0x575cd8): score = trunc(1000 * funSum / (playsHalf + 4 + plays)) + (3 - d) * 100, plus 100 if the hole's ranking flag bit 0 is set (top 100) and 100 more if bit 1 is set (top 18),
  where funSum is the short at 0x575e10 (the hole's mood sum, same quantity as DECODE_HOLE_STATS.md with a factor 10 instead of 100) and the denominator matches the Hole fun formula there. The hole score is divided (integer) by (distance + 8) where distance is the smallest of the distances from the site to the hole's tee point, its green point and an optional third point (table at 0x59aea8, stride 24, entries of -1 are skipped, values shifted down 10 bits).
  The distance helper 0x40acd0 is an integer distance (DERIVED: Euclidean, truncated, with very large components scaled down by 8 first).
- Marina term: if the Marina effect level E (global array at 0x543ca0 + 4 * type, so 0x543cd0 for type 12) is nonzero, M = M + trunc(E * M / 3). So +33 percent at level 1, +67 percent at level 2. The effect level only counts a connected Marina (needs a path to the clubhouse).
  The exe text "increases the value of all homes and lots at your course" (line 13494) and the hover text "Increases property values" are the Marina's descriptions; no landmark has a home value effect (the four landmark effects are mood, skills, dandelions and golfer stories).
- Nothing else enters: no pathway, no number of nearby homes beyond the -16 per home-site ring tile, no golfer count.

### 3.2 Value field and size classes (EXACT)
- +0xc = V, smoothed value, section 1.4 step 1. Starts at 0 and takes about one month to receive its first value (V1 = lot).
- Size class for unsold lots (0x40e5b0, applied to V; thresholds are 200 per step): V below 200 (also any negative) class 0; 200 to 599 class 1; 600 to 1199 class 2; 1200 to 1999 class 3; 2000 and above class 4. In lot value terms at steady state (V = 12 lot): 17, 50, 100 and 167.
- Drawing while +8 == 0: class 0 sign sprite (id 0x1c8, files "sign" per theme), class 1 house-under-construction sprite (id 0x1c9, files "Houseconst"), classes 2 and 3 the finished regular house (id 0x1ca, files "HouseA", "HouseB" by theme; classes 2 and 3 look identical), class 4 the same house plus four small 19 by 13 decoration sprites at the four diagonal offsets. Draw priority 0xf, 0x13 and 0x60.
  The view orientation uses (global 0x5685f4 / 2 + x tile) and 3. When bit 5 of 0x59e7b8 is set the raw V number is also printed on the lot (debug overlay). EXACT.
- So "construction" is a purely visual stage between V 200 and 599; there is no timer. Because the lot must be worth at least 50 (steady V 600 or more) to be placed, a legally placed lot always ends at class 2 or higher, and typically passes class 1 within 1 to 5 months (lot 50: 5 months to reach V 200 only if it started... see simulation: V 200 after 5 months at lot 50, V 600 after 52 months).
  Simulated months to reach V 200, 600, 1200, 2000 (d irrelevant): lot 50: 5, 52, never, never; lot 100: 3, 8, 60, never; lot 150: 2, 5, 13, never; lot 200: 1, 4, 8, 21; lot 300: 1, 3, 5, 10.
- Sold sites (+8 != 0) are not drawn by this routine. The tile pass (0x413f1a) draws a 6 by 6 estate yard instead (0x4012d0): ground tiles (fence, patio, concrete, dirt) at the edge, a celebrity house sprite (six variants: Sum_home, political_home, ArtDeco_Home, DesertCelebHouse, Summer1stCelebHouse, Summer2ndCelebHouse, the first three with a dirt underlay) and animated props (grill, jacuzzi, punching bag, video game, classic car, BMW convertible; the estate item names are at 0x4c11e0). The estate layout slot is the record index parity (slot 0 or 1). DERIVED for the item list, EXACT for the call structure.
  V keeps updating after the sale but has no further visible effect (UNKNOWN whether any screen reads +0xc of a sold house).

### 3.3 Resident walker (EXACT structure, DERIVED purpose)
- On sale a walker is created in a 100-entry table at 0x56d1b8 (stride 0x3c, free when the type word at 0x56d1d8 is -1): sprite kind = the celebrity's type byte (0 to 10), layout slot = random 0 to 5, home tile = the site, start cell random inside the 4 by 4 yard, animation 0x140 (idle). If the table is full nothing happens.
- AI (0x4017d0): wanders inside the yard, idles 64 to 127 ticks between moves; with probability 1 in 24 (1 in 64 when bit 0x200000 of 0x59e7b8 is set) when it steps outside it shouts (positional sound 0x82 + kind) and plays animation 0x14d. Purely cosmetic.
- Removal on demolish clears every walker whose home tile equals the site (0x4011b0).

### 3.4 Golfer reaction (EXACT unless marked)
- Glance code (decompile lines 15640 to 15690): when a golfer looks at tile id 21 and the matching record is type 5 with +8 == 0, or the record cannot be found, the glance is negative (event 0x14, mood -2, text "is disappointed to see a home site in the middle of your otherwise lovely golf course", plus the thought bubble about an ugly building).
- When the record is type 5 with +8 != 0 (sold), the glance stores +8 and the golfer gets event 0x16 with argument +8 - 1: mood +1, a short stop, text and bubble in section 4.3. Event 0x16 takes priority over the negative glance if both kinds were seen in one look. The golfer's probes per look are (distance units * 16) / (d + 2); the last sold house found is the one reported.
  The extra eligibility conditions of the glance (golfer flags) are not decoded (DERIVED).
- So an unsold lot costs -2 per glance and a sold house gives +1, which is a reason to place sites only where they will sell.

### 3.5 Counters tying homes to the rest of the game
- Free site counter (0x56d1b0) versus roster: section 1.1.
- 0x5a9cb0 = number of type-5 records with +8 != 0 (sold homes), recomputed each frame (0x418c1f). It is stored as a byte in the per-record report row (0x571ffd + 46 * index) and is a term of the "Top 10 Golf Courses" comparison score: total = ranked-hole flags (0x5a9cac) + cash / 1000 (0x5a9cb4, units, so one point per $100,000) + 0x5a9cb8 + sold homes (one point each) + 0x561258, drawn as icons. EXACT sum, DERIVED name of the screen quantity.

## 4. Text shown to the player

Quoted fragments are short; templates use <> placeholders. Colour 0x80004010 is the ticker colour used by all the messages below that go through 0x40cb00.

### 4.1 Unattractive lot (placement refused, EXACT)
"This is not a very attractive location for a building lot. " followed by a hint sentence: "Home buyers like water, woods, and grass near a golf hole with a good fun factor." Shown when the lot value is below 50 on a click. No cost.

### 4.2 Celebrity buys a home (ticker, EXACT)
"International <kind> <Name> has purchased a vacation home at your golf course! Golfers enjoy seeing celebrities as they play."
<kind> is one of 11 words picked by the celebrity's type letter A to K from celebrities.dta: action star, pop sensation, noted statesman, funny man, supermodel, fitness guru, funny lady, leading man, beauty, rocker, superstar.
Posted only if the ticker is free, otherwise the sale is skipped for that month.

### 4.3 Golfer events about homes (EXACT)
- Event 0x16 (sold house in view), tracker text: "<golfer> is excited to see <Name>'s vacation home on your golf course." and the golfer's bubble "Hey that's <Name>'s house!".
- Event 0x14 for an unsold home site tile: "<golfer> is disappointed to see a home site in the middle of your otherwise lovely golf course." (the object name is "home site").

### 4.4 Member promotion hint (EXACT)
Tier below 3 uses "<Name> applies for membership at your club! " style text. At tier 3 (Silver) the message is of the form "<Name> has decided to upgrade to a prestigious membership at your club! <Silver > members will pay big bucks for an attractive home site on your course." Tiers 4 and 5 use "coveted" and "an exclusive" and have no home hint.
(The tier name inserted before "members" is the status word "Silver " from the table at 0x4c2a88, so there is a doubled space.)

### 4.5 Tool and help texts (EXACT)
Tool name "Building Lot"; hover description "Sell lot for cash"; keyboard help line "Sell a building lot." (key shift+b); overview help line "Routing, Aura, and lot values" (key shift+r). The roster/HUD text "Waiting list: <N>" uses the free-site counter.

### 4.6 Unreachable golfer texts (dead code, for the record)
Yes: "International celebrity <Name> has decided to buy a vacation home at your course. 'Groovy course!', <he or she> says." No: "... has decided not to buy a vacation home at your course. 'More fun in Malibu' <he or she> says." Both can never print.

### 4.7 Events and highlights logged
- Highlight ring: 500 entries of 16 bits, index (tick / 1024) mod 500, so ONE entry per month, later events in the same month overwrite it. A sale logs the value 0xc0 OR celebrity index. The year-end report decodes the top bits (value and 0xffe0) and the low 5 bits as the index, then prints "<celebrity name> buys a home." with the month name. Only celebrity indexes 0 to 31 decode correctly (the shipped file has 21).
- Sound 0x33 on sale (EXACT). Failure sound 0x18 on a rejected placement click (EXACT).
- Score point per sold home (section 3.5). No cash event is logged at sale.

## 5. Home Site Value overview screen (0x456be0, mode 2 of the overview opened with shift+r)

- Title "HOME SITE VALUE" centred at x 400, y 0xbc; scale labels "High" at x 0x277 and "Low" at x 0x8e, both y 0xce. (The two end labels have no gradient bar read here; the colour is per tile.)
- Per land tile (every tile that is not id 20): lot = lot value of the tile used as the top-left corner; site = site work from the placement scan 0x40db90 with side 2 and type 5.
  q = trunc((trunc(lot / 4) - site) * 3 / 2), clamped to 0 to 255, g = q / 8 (0 to 31). Tile colour = ARGB flag bit 31 plus green channel g (value 0x80000000 + g * 32), i.e. pure green, brighter means more profit per site (quarter lot value minus site work, base price 10 not subtracted).
  If the scan returns -1 (cannot build there) the tile is dark red 0x80002000. Unowned tiles (outside the course) are drawn in the flat grey 0x80002108. EXACT.
  Full brightness needs trunc(lot / 4) - site of at least 171.
- Text blocks: "Things which INCREASE home value:" with "Close to water and trees.", "Close to a fun golf hole.", "Close to a top 100 or top 18 hole.", "Building a Marina". "Things which DECREASE home value:" with "Close to an unfun hole.", "Close to another building.", "Too close to green, fairway, or OB.", "Far away from the golf course."
  DERIVED reading against the formula: the decrease items map to the hole term (unfun holes only fail to add, M is never negative), the -16 home-site ring tiles, the -8 playing-surface tiles and the 1 / (distance + 8) falloff.
- Other overview modes (employees 0, routing -1, aura 1) share the same routine and are not covered here.

## 6. Still unknown or open
- Which input switches the hole ranking flags 0x575eb8 bit 0 and bit 1 (top 100, top 18) beyond what DECODE_HOLE_STATS.md states.
- What sets bit 7 of 0x56c7b4 (the forced V = 2500 hook), what reads 0x578338, and what mode 3 of 0x567afc is.
- Whether any screen shows +0xc of a sold house, the exact eligibility flags of a golfer glance, and the full meaning of the roster byte indexed by hole count in the membership application rule.
- The exact distance helper 0x40acd0 beyond the Euclidean reading, and the estate layout tables at 0x4c11e0 and 0x4e6d20.

## 7. Port status (tools/sgview.cpp, include/sg/homes.h)

Built from the facts above: lot value, the Home Site tool gated by Silver members, placement with the club share and the unattractive lot text, the monthly value pass and the celebrity sale with the ticker message, demolition with a confirm (a second right click within five seconds),
house art by value stage and celebrity houses with a resident, golfer glances (-2 for a for-sale lot, +1 for a sold house), the Home Site Value map colours, the placement preview, and save and load (HOMES section).
PLACEHOLDERS: the sale sound (a Buy sound stands in for id 0x33), the choice of house A or B and of the celebrity house variant, the resident standing still instead of wandering, the estate yard props, the hole ranking flags in the hole score (always off),
the optional third hole point in the distance term, the glance rule (one look per hole within five tiles of the green), the editor-only tile ids in the ring term, and "Waiting list" shown only in the tool tip. Score points for sold homes are not in a Top 10 screen yet.
Test hooks: `--silver N`, `--home x,y`, `--fake plays,mood`, `--months N`.
