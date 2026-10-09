# Facts read from the publisher-supplied golf.exe

Provenance: this build of golf.exe was sent to the project owner by the publisher. Its PE header is plain (.text, .rdata, .data, .rsrc, no SafeDisc sections or strings). The protected disc exe, the earlier no-CD exe and the patch exes are still not used.

Method: only facts are written down here (rules, names, thresholds), in our own words, so the game code stays an independent implementation. No code from the exe is copied. Game text is paraphrased, not reproduced.

## Confirmed by the exe's text

- Course types: Municipal, Daily Fee, Country Club, plus Resort. A Daily Fee course is 6 or more holes. Skilled employees need it.
- Basic employees: Club Pro, Ranger, Groundskeeper, Soda Vendor. The Ranger works near a tee and speeds play on that hole, so placement matters (our Ranger effect is global for now).
- The Groundskeeper removes weeds. The Soda Vendor feeds hungry golfers and earns a little extra revenue.
- Happy golfers pay higher fees, unhappy golfers pay less. Fees are paid at the end of each hole. Fee levels can be raised (the SGA rating raises them).
- Hole classes and their meaning: Freeway = length; Precise = accuracy; Heroic = length plus imagination; Strategic = accuracy plus imagination; Classic = all three; Breather = easy hole.
- Two ratings per course: skill rating and fun rating. Golfers have an attitude. Top 18 and Top 100 hole listings exist, plus a 100 star rating achievement.
- Debt ladder: a warning that gives two years to return to positive cash, then the board is concerned, then very worried, then your contract is terminated. (Our 30 day grace is a placeholder to replace.)
- Membership: member tiers Member, Silver, Gold, Platinum. Members buy home lots, golf carts, and the roster can decline.
- Tournaments: prizes can be doubled for passing the stricter SGA evaluation; named prize levels of $500,000 and $1,000,000 appear as achievements.
- Lots can be sold for cash. Four difficulties are chosen at start.

## Not yet found (numbers)

Starting cash, fee amounts, wages, upkeep, day length and the rating formulas are numeric constants in code, not strings. Finding them needs a closer look at the code and data tables.

## Numbers found in the code (first pass; confidence noted)

- Money is stored in units of $100. A state dump in the exe prints cash as stored value x 100. Confidence: high.
- New game starting cash is a stored value of 1000, so $100,000. Confidence: high (a plain constant plus a flag term).
- Two other start paths store 10000 ($1,000,000) and 100000 ($10,000,000) when a special game flag is set. These look like the sandbox / unlocked modes. Which is which, and what the extra constant 0x3fff unlocks, is not yet confirmed. Confidence: low.
- A global tick counter drives the economy. Time is counted in blocks of 1024 ticks, with a 500 entry history ring for cash and ratings. The charge interval shrinks with game speed (1024 / (speed + 2) ticks). Confidence: medium.
- Each active staff-like record is charged 1 stored unit ($100) per interval, and a second loop charges small per-item costs (stored values 2 to 7) looked up from a table. What those records are (employees, maintenance, repairs) is not yet confirmed. Confidence: low.
- The "two years to return to positive cash" ladder is a three step message sequence (concern, worry, termination) in a separate screen function. The thresholds that trigger it are not yet traced.

## Not found yet

Green fee amounts and the fee formula, wage per employee type, building prices, difficulty differences, rating formulas, SGA thresholds.

## Green fee and golfer mood (from the Ghidra decompile of the publisher exe)

Confidence: medium. The structure is read from the code; the meaning of some inputs is inferred.

- Money is stored in units of 100 (display multiplies by 100).
- Fee per hole, in units: the golfer's mood value, doubled when one global setting equals 2, plus 2 for each of two hole flags (probably Top 100 and Top 18), plus an airstrip bonus, plus, when the hole type value is above 3, 2 and 3 more unless the value is exactly 4.
- Initial mood: 3 plus a random 0 to 2, or 4 on difficulty 0.
- After each hole mood falls by (hole field + 6 + holes played) * (mood - 1 + difficulty) * (difficulty + 1) / ((course factor * 5 + 15) * 8), integer division. The hole field and course factor are not decoded; the game uses 0.
- Mood smoothing uses old * 7 / 8 + new (not used by the game yet).
- Not yet decoded: the events that raise mood, the club fun and skill ratings. The game's +1 for par or better is a placeholder.

## Golfer mood events (decompile of the golfer event routine, around line 71400 of golf_decomp.c)

Confidence: the deltas are read directly from the code; what in-game situation each event id means is NOT decoded yet (only a few can be guessed from call sites).

- One routine takes (golfer, event id, argument). Each event changes the golfer's mood by a small delta. The mood is then clamped to -10..10 and the change is also tallied per employee-or-building slot and per map tile (good and bad counters).
- Positive deltas of +1: events 1, 6, 7 (only when the argument is 0), 0xb, 0x16, 0x1c, 0x1d, 0x20, 0x21, 0x22, 0x2c, 0x2e. Event 0x3b gives +1 when a flag on the golfer's slot is set.
- Negative: events 2, 4, 5-type and 0x15, 0x18, 0x1e, 0x14, 0x2b, 0xa, 8, 3 give -2 (event 3 and 8 and 0x14 included); event 9 and 0x24 give -3; events 0xc, 0xd, 0xe, 0xf, 0x1a give -1; event 0x17 gives -1, or -2 on any difficulty above 0; 0x2f gives -2; 0x41 gives -2 above difficulty 1.
- Some events depend on difficulty: 0x27 and 0x36 give +1 only on the two easiest difficulties; 0x33-0x35 give +1 only on difficulty 0.
- Threshold events: 0x12 gives +1 when a golfer counter exceeds 7; 0x19 when another counter exceeds 7; 0x1b when a counter exceeds 59.
- From call sites: events 1, 2, 3, 0xd come from the golfer's ball-position terrain checks (ground type under the ball, for example water or rough), 0xc from a shot that leaves the playing area. Treat these as guesses.
- A golfer whose mood drops below -10 leaves the course.

## Hole and club ratings (course statistics pass 0x42dea0, Course Report 0x44fb30)

Confidence: high (arithmetic read from the machine code).
- The pass runs every frame and recomputes everything. Per open hole: every skill class starts with 8 rounds at par; the
  average strokes (hundredths) of the classes lacking Length, Accuracy or Imagination minus the full-skill class give the
  three differentials; club skill is their sum over holes, club fun the sum of hole fun = mood sum * 100 /
  (planned shots / 2 + 4 + tee shots).
- A skill makes the hole's type when its differential is at least 50 (25 on the easiest difficulty), unless it is the
  weakest and below 100 (50 on the easiest; the report always uses 100). Types: Breather, Freeway, Precise, Challenge,
  Creative, Heroic, Strategic, Classic. The first hole of each type is announced once.
- With 10 or more finished rounds a hole is too hard above (6 - difficulty) / 3 strokes over par on average and too easy
  below (3 - difficulty) / 6 under. A variety counter compares each hole with the one before (type, dogleg, slope remarks,
  par, heading within about 56 degrees).
- Top 100: fees collected above $20,000 and score above 200; Top 18: above $40,000 and 300; score = the three
  differentials, raised to fun * 3 on the easiest difficulty and fun * 2 on the next; too hard or too easy holes cannot win.
  Awards are absolute, not a ranking.
- The report: yards as stored, average strokes of finished rounds, minutes = time / plays / 40, fun, the differentials,
  type, average fee, revenue and profit = fees - build cost - maintenance (one unit per open hole each charge interval).
  The total row adds the averages, divides fun and the differentials by the open holes, and (an exe bug, kept) shows the
  last hole's minutes past the hour.

## Terrain table (static table at 0x4c1a40 in the publisher exe, 0x30 bytes per entry)

Confidence: the table layout is read from the exe; "cost" is the byte the build menu multiplies by 100 and shows after "Cost:" (medium).

Cost per tile in units of 100 by tile id: tee 5, green 10, fairway 3, firm fairway 3, rough 1, deep rough 2, mound 4, sand trap 6, waste bunker 4, pot bunker 8, ravine 10, brush 4, rocks 4, tree 10, pine 10, palm 10, elm 25, water 50, wetlands 2, marsh 6, building 0. A second byte per entry (1 to 8 for most; water 8, out of bounds 8, tee 0) looks like a hazard severity and is not used yet.

Course ranks named in the exe text: Municipal, Golf Club, Country Club, Championship (upgrade messages mention beyond 9 holes for upgraded buildings).

## Clubs and shot planning (partial)

Confidence: low to medium; only the outline is read.
- The exe names 14 clubs by index 0..13: Putter, Sand Wedge, Lob Wedge, 9 Iron down to 2 Iron, 4 Wood, 3 Wood, Driver (names table around 0x4c51b0).
- The golfer's club index is chosen in the shot planner (around line 15290 of golf_decomp.c) from the distance to the target after scoring candidate landing spots; the planner weighs the terrain value under each spot (the second byte per entry of the terrain table) times distance, with a 5 added when two risky spots are found. The club is then derived as (max range - needed range) * 60 / (3 * max range), clamped to 1..11, with 5 forced and 13 (putter) used on a green.
- Superseded by "Golfers and shots" below: the club is only a label; distance, spread and skill scaling are decoded there.

## Maximum range and ball flight (golf_decomp.c, functions near 0x422530 and 0x422bxx)

Confidence: medium for the structure and constants (read directly), low for what the golfer fields mean (names are my guesses).

Maximum range for a shot (before clamping to 330):
- Start: 150. Add a base term: on difficulty 0, 25 (40 for golfers with a certain flag, probably the pro flag); on other difficulties, a per-golfer byte times 50 / 3.
- Pro-type golfers get another 50. A Length-type skill digit d (flag bit 0) adds d * 4 - 20; an Accuracy-type digit (flag bit 1, tee shots only) adds (digit - 5) * 6.
- A positive mood-like byte adds up to 3/24 of the total. Pros add 15 times a global setting.
- Lies: take the terrain table's second byte (hazard severity, capped 0..3) and subtract severity * total / 8; any lie other than the tee then cuts a further fifth.
- Result is capped at 330 (0x14a).

Ball flight (per tick, fixed point, one tile = 1024 units): position moves along the heading by speed / 16; height changes by vertical / 32; vertical falls by 64 each tick. Launch values from the target range r (0 to max range): u = r * 20 / 25, speed = u * 33 - u * u / 48 + 64, vertical = speed / 8 + 512. Angle error is drawn at random per shot (extra from the golfer's accuracy), and landing on ground applies a terrain roll friction taken from the terrain table (byte at +0x21; speed shrinks by speed >> friction / 2 for friction under 5, by speed / 64 minus 32 otherwise; water and rough halve it).

## Launch speed and aim error (follow-up)

- The club choice does not set distance: the golfer asks for a range (distance to the target in range units, capped at the maximum range) and the exe solves the launch speed by bisection so the carry is 4/5 of that range; the club is only a display. The solver uses a coarse two-tick step version of the flight. crates/sg-core/src/flight.rs reproduces this; its output carries about 96 percent of the 4/5 target.
- Vertical launch speed = (est / 8) + 512 where est = u * 33 - u * u / 48 + 64 and u = range * 20 / 25.
- Aim error: a random hook or slice amount is drawn per shot (a table-free random value: 50-wide roll, halved under 20, minus 10 under 40, doubled less 50 above) and scaled by skill bytes; it bends the heading every tick. Full scaling by skill is not decoded.
- Scale: 25 range units equal one tile. How many yards a range unit is, and the tick length in seconds, are unknown; the game uses placeholders (TICKS_PER_SECOND = 40).

## Calendar (date stamp routine in the exe, near golf_decomp.c line 8081)

Confidence: medium. The stamp shows month = (ticks and 0x1fff) / 1024 + 3, so months 3 to 10, and day = (ticks and 0x3ff) * 30 / 1024 + 1. One month is 1024 ticks, one day about 34 ticks, and the exe's time wraps every eight months (March to October). The game now uses an eight-month year; the start year is a placeholder.

## Property prices and amenity income (golf_decomp.c, near 0x46f1d0 and the golfer visit code)

Confidence: prices high (table read directly), amenity income medium.
- The property price table in the exe has 16 entries, in units of 100: 500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000. The price belongs to an offer slot, not to a place: see "Properties and new-game land" below.
- Amenity visits pay the course in units of 100: building type 7 pays 5; type 6 pays 4 or 8, type 8 pays 6 or 10, type 10 pays 8 or 12, where the lower value applies when the building's upgrade level is below 2.
- The 25-unit charge (-0x19) found earlier is for firing an employee (booked under Salaries), not for removing a building; what a building removal pays back is not decoded.

## Where money moves (search result, negative findings included)

Confidence: low to medium. Direct writes to the cash variable (0x571fd4) in the decompile are: the starting value 1000 units, golfer fees and amenity payments (additions), property purchase (price table), the 25 unit removal charge, a building removal cost in a second routine, golfer wagers, and a few small additions. No direct monthly wage or upkeep deduction was found, and the construction cost deduction was not found either. Cash may also be changed through another alias or a ledger of per-month entries (100-entry table with income and expense columns around 0x584212 to 0x58421e), so the absence is not proof that wages do not exist. Staff wages and upkeep in the game therefore stay placeholders.
- The cash reserve display and the finance graph choose their scale from cash thresholds 2500, 5000, 10000, 25000, 50000 and 100000 units (graph steps 2, 4, 10, 20, 40, 80).

## Construction charges and the yearly ledger (golf_decomp.c, read for facts only)

Confidence: high for the structure, medium for which in-game action fills each ledger column.

- The map is at most 50 x 50 tiles: every per-tile array is 2500 entries with a row stride of 50.
- Terrain work is queued, not paid at once. A queuing routine stores, per tile, the target type and a signed cost byte (units of
  100). When the work on that tile is carried out, the cost byte is added to cash (it is negative for a charge), shown as a floating
  money number at the tile, and added to ledger column 3; the pending entry is then cleared. So money leaves as the crew works.
  The callers of the queuing routine are reached through a table the decompile does not resolve; where the cost byte comes from
  (presumably the terrain table's cost byte) is still to be confirmed.
- The Financial Report keeps one ledger record per year: 10 signed 16-bit values in units of 100, indexed by years since 2001
  (the report prints the year as 2001 + index). Values 0..7 are categories, value 8 is written by the report as their sum.
  The report's labels, read from the exe's data: Greens Fees, Home Sites, Food/Drink, Build course, Facilities, Salaries,
  Maint./Interest, Other, then the total. Writers found in the code:
  - Greens Fees: the per-hole fee routine (it also adds the fee to a running total in the golfer's record);
  - Home Sites: a lot routine (object type 5) subtracts a cost of (stored price / 50 + a tile value / 2) when a lot is cleared;
  - Food/Drink: amenity visits (+2 to +12) and the Soda Vendor: when a vendor reaches a golfer it serves them, the course earns
    2 units ($200), the golfer's thirst field is reset and the golfer gets mood event 0x19 (argument 20);
  - Build course: terrain work as above;
  - Salaries: the only writer found is one action in the employee dialog (the dialog that also offers Hire employee and Rename
    Employee) that charges 25 units ($2,500) for the selected employee. No periodic wage deduction exists in the decompile, so the
    staff appear to cost a one-off amount rather than a running wage; which dialog action this is still needs the UI;
  - Maint./Interest: no direct writer found yet (it may be updated through a pointer);
  - Other: prize style additions, a deduction in a monthly routine, wagers.

## Golfer mood events, completed (event routine and its message table)

Confidence: high. The routine has two switches: the first sets the mood change, the second picks the golfer's thought to show,
indexed by event - 1. Reading both together gives each event's meaning (described in our own words) and its exact amount.
This replaces the event list further up, which had a few amounts wrong (for example a missed easy putt is -1, not -2).
crates/sg-core/src/mood.rs holds the table.

| Event | Meaning | Mood |
|---|---|---|
| 1 | holed a difficult putt | +1 |
| 2 | missed a putt that looked easy | -1 |
| 3 | unhappy with how the shot turned out | -2 |
| 4 | the next shot will not involve what the golfer hoped for | -2 |
| 6 | plans to use the slope of the hill | +1 |
| 7 | (positive, only when the argument is 0) | +1 |
| 8 | cannot find a safe place to aim | -2 |
| 9 | nearly hit by another group's shot | -3 |
| 0xa | no path built where the golfer wants to walk (the hint points at the path tool) | -2 |
| 0xb | scenic view in sight of the next shot | +1 |
| 0xc | ball bounced off a tree | -1 |
| 0xd | ball ended up in a hazard | -1 |
| 0xe | thirsty, needs a soda vendor | -1 |
| 0xf | hungry, needs a snack bar | -1 |
| 0x10, 0x11 | amazed the last shot drew / faded | 0 |
| 0x12 | eating a snack at a snack bar | +1 once a golfer counter is above 7 |
| 0x14 | sees an eyesore in the middle of the hole | -2 |
| 0x15 | frustrated by slow play (the hint suggests a Ranger near the tee) | -2 |
| 0x16 | excited to see a celebrity's vacation home on the course | +1 |
| 0x17 | (negative) | -1, -2 above the easiest difficulty |
| 0x18 | sees weeds (the hint suggests a Groundskeeper) | -2 |
| 0x19 | refreshing drink from a Soda Vendor | +1 once a golfer counter is above 7 |
| 0x1a | tired, needs a bench | -1 |
| 0x1b | resting on a bench | +1 once a golfer counter is above 59 |
| 0x1c | enjoys a feature the player built | +1 |
| 0x1d / 0x1e | likes / dislikes something about this hole | +1 / -2 |
| 0x20, 0x21 | faces an interesting (strategic) decision | +1 |
| 0x23 | quits in frustration | -2 |
| 0x24 | sees an angry golfer | -3 |
| 0x27 | intrigued by the wildlife | +1 on the two easiest difficulties |
| 0x33, 0x34, 0x35 | appreciates a new driving range, pro shop, putting green | +1 on the easiest difficulty |
| 0x36 | uses a club for the first time | +1 on the two easiest difficulties |
| 0x37 to 0x3c | sets up a draw, fade, high or low shot (thought only) | 0 |
| 0x41 | good shot but cannot see where it went | -2 above difficulty 1 |

Events 0x22, 0x2c, 0x2e give +1; 0x2b and 0x2f give -2 (meanings not attached to a thought). The mood is clamped to -10..10 and a
golfer pushed below -10 leaves. The game raises 1, 2, 3 and 0xd from shot outcomes today (the putt distances that count as easy or
tough are placeholders); the rest need the systems that trigger them (needs, benches, wildlife, landmarks, other groups).

## Difficulty (every reader of the difficulty variable, 0 easiest to 3)

Confidence: medium to high; the rules are read directly, some inputs are not decoded.
- Starting mood of an ordinary golfer: 3 + random 0..2, or 4 on Easy (a game setting can override it as value + 4). Pro and
  celebrity golfers start at 3 + random 0..3, or 5 on Easy.
- Mood events: the difficulty-dependent amounts in the table above.
- After each hole: the mood drop (formula above) grows with difficulty.
- SGA skill demand: a skill counts as demanded at 25 hundredths of a stroke on Easy, 50 otherwise; a second rating threshold is
  50 on Easy and 100 otherwise.
- Which golfers come: a candidate from the 100-entry golfer roster is accepted when (its rating - 10)^2 / ((difficulty * 5 + 20) * 2)
  is at most tries / (4 - difficulty) + club reputation / 10 + cash in units / 200. Harder games make strong golfers pickier.
- Waiting: a waiting golfer gives up when a random 0..5 is at least difficulty + 1, so patience falls as difficulty rises.
- The game offers the four difficulties on the property chooser (our own control) and with `--difficulty 0-3`.

## Sounds (golf_publisher.exe machine code plus the decompile)

Confidence: high for the table and the plays listed; the ball and swing sounds' trigger code is not found yet.
- The exe keeps a table of 300 sound slots, 0x6c bytes each, starting at 0x80d840. One start-up routine loads 219 files into
  slots (the file list in the exe matches the folders under Sounds/). Two helpers play slot N: one with volume and three more
  settings, one with a 1000 ms argument (probably a fade). Slots 16 to 24 are loaded twice: first effects, then golfer emotion
  sounds, so their meaning depends on when they are played.
- Plays with a fixed slot, by what the calling routine does:
  - green fee collected: no sound;
  - "Other" income from a golfer (prize style money): `effects/cash.wav`;
  - the two wager outcomes that also book "Other": `effects/taunt009.wav` and `effects/unrest 2.wav`;
  - the SGA hole rating announcements: `Applause.wav` and `ApplauseGood.wav`;
  - the player's own golfer receiving a bad mood event (while a flag is set): `ApplauseBad.wav`;
  - golfer behaviour routine: `effects/belch.wav`, `effects/tada.wav`;
  - course editing routine: `interface/bass up 2.wav` and `bass down 2.wav` (raise, lower), `interface/building.wav`,
    `effects/tree sprout 3.wav`, `effects/palm.wav`, `effects/maple.wav` (tree planting), `effects/camera.wav`,
    `effects/fly.wav`, `effects/club 2 drop no wtr.wav`;
  - `GolfAmbience122.wav` from the ambience routine; `Golf Harp 6/7.wav` from two screen transitions.
- The ball and swing sounds (`Golf sfx/*`, slots 188 and up) are never passed as constants; they are chosen by computed slot
  numbers in code not yet traced. The game keeps its file-name based choice for those.
- Change made: the game no longer plays applause and the cash register on every holed ball.

## Golfer needs (golfer behaviour routine)

Confidence: medium; the counters and thresholds are read directly, the terrain test and the tick length are not decoded.
- Each golfer has a hunger counter and a thirst counter. Every 160 ticks (120 in one golfer state) one of them grows by one:
  hunger when the golfer stands near terrain of three particular kinds or, in the Tropical theme, half the time (and then only for
  some golfer types, on a coin flip); thirst otherwise.
- Once a counter is above 15, the golfer raises the hungry event (0xf) or the thirsty event (0xe) on every fourth update
  (special golfers of one kind are held at 16 instead).
- A snack bar visit (building type 7) raises the snack event (0x12, +1 when hunger was above 7) and resets both counters.
- A Soda Vendor who reaches a golfer serves them: the club earns 2 units under Food/Drink, thirst resets, and the drink event
  (0x19) follows. A vendor will not bother a golfer whose hunger and thirst are both 16 or less and whose tiredness is under 161.
- A third counter, tiredness, drives the bench events (0x1a, 0x1b); see "Golfers and shots" below.
- The game follows these rules with two placeholders: needs update every 4 seconds (160 ticks at the assumed 40 ticks a second),
  and a vendor or snack bar is "reached" by chance or distance instead of by walking there.

## The game tick

Confidence: high.
- A fresh, complete Ghidra 12.1 decompile of golf_publisher.exe now covers the stretch the earlier export lost (0x40f190 to
  0x421b60). Ghidra's own analysis left 13% of the code outside functions; functions were added only where routines really start
  (after padding or a return), which brought coverage to 93%, and all 2,688 functions plus the 18 remaining code runs decompile.
  The 59 KB main frame routine at 0x40f5c0 needed a larger decompiler output buffer. The decompile stays outside the repository.
- One tick is one frame of that main routine. Near its end the game updates the golfers, then the staff, then a third routine,
  and adds one to the tick counter at 0x834170. When bit 0x8 of the game state flags is set, the counter is also rounded up to an
  even number, so updates gated on odd ticks are skipped (a fast mode). Right after, `(tick & 3) <= difficulty` gates a further
  update.
- Frame pacing: the routine notes the time when a frame starts, and at the end waits until (175 + 75 when game-state flag 0x20000 is
  set) / 2 ms have passed, using the 10 ms wait routine. So a tick lasts at least 87 ms at normal speed (about 11.5 ticks a second)
  and 125 ms with that flag. The wait is skipped while another flag (0x59b04c) is set.
- Consequences used by the game: a month (1024 ticks) lasts 89 seconds, a golfer's needs update every 160 ticks (14 seconds), and
  ball flights last their tick count times 87 ms.

## Properties and new-game land (offer routine 0x46f2b0, land generator 0x470a60 and their helpers)

Confidence: high. Every rule below was read from the complete Ghidra decompile and checked against the machine code where the
decompile was unclear. Implemented in `sg-core/src/land.rs`.

### Random numbers and noise
- One generator serves the whole game: a 32-bit LCG, state = state x 1103515245 + 12345. A draw in 0..n-1 takes bits 16..30 of
  the state as a fraction of 32768, multiplies by n and truncates. The state is seeded with the Windows millisecond clock x 37.
- A height noise grid of 16 x 16 random values 0..15 (wrapping) is filled once at start-up (18 x 18 draws). It is read with
  bilinear interpolation (cells of 256 units, 32 sub-steps). The "field" used by the land is two octaves mixed 6:4, times 7,
  divided by 128 and clamped to 0..512.

### Property records (16 records of 0x82 bytes at 0x4c1e90)
Name, bonus text, a marker position on the world map art, and three small numbers: theme (exe order 0 Parkland, 1 Desert,
2 Tropical, 3 Links), coast (0 inland, 1 sea along one side, 2 island) and relief (0 gentle, 1 rolling, 2 hilly).

| # | Property | Theme | Coast | Relief | # | Property | Theme | Coast | Relief |
|---|---|---|---|---|---|---|---|---|---|
| 0 | Monterey | Parkland | 1 | 1 | 8 | Northeast | Parkland | 0 | 1 |
| 1 | San Diego | Parkland | 1 | 0 | 9 | Carolina | Parkland | 0 | 0 |
| 2 | Rocky Mtns. | Parkland | 0 | 2 | 10 | Ireland | Links | 0 | 2 |
| 3 | Las Vegas | Desert | 0 | 0 | 11 | Scotland | Links | 1 | 0 |
| 4 | Phoenix | Desert | 0 | 0 | 12 | Wales | Links | 2 | 1 |
| 5 | Hawaii | Tropical | 1 | 1 | 13 | Spain | Desert | 1 | 1 |
| 6 | Oahu | Tropical | 2 | 0 | 14 | Florida | Tropical | 1 | 0 |
| 7 | Nova Scotia | Links | 1 | 1 | 15 | Jamaica | Tropical | 0 | 1 |

### The offer
At the start of a new game the properties are dealt into 16 offer slots. Slots 12..15 are filled first, at random, with four
different themes. Slots 0..11 then take the remaining properties at random, except that slot 0 must be a Parkland property. A
slot's price is the price table entry of the slot. Its acreage is (slot + 4) x 10, plus 10 (slots 0..3) or 20 (slots 4..15)
for an inland property, minus the same for an island; in sandbox mode every slot has 250 acres and the starting money is
10000 units instead of 1000. The chooser draws each property on its own fixed card (properties 0..9 down the left, 10..15 down
the right) with the acreage and price of the slot it was dealt into, greyed when the price is above the cash.

### The land generator (map 50 x 50 tiles, corner heights 51 x 51, sea level 3)
1. Height scale: 48, 32 or 16 for relief 0, 1, 2; then +1/2 for slots 0..3, +1/4 for slots 4..7, -1/4 for slots 12..15;
   +16 in sandbox mode. Bigger is flatter.
2. Ground: rough. Not in sandbox: slots 8..11 deep rough, slots 12..15 brush; Desert uses rocks for both.
3. Tree streaks: random walks from a random tile, each step moving -1..1 in both directions, each walk at most limit/128 tiles
   where the limit starts at 5000 and grows by 2500 per walk; stop when the walks total more than 1250 tiles or the limit
   reaches 45000. Types by theme: Parkland trees (pines on the far half, all pines at Carolina), Desert pine/rocks alternating,
   Tropical pine then rocks or trees, Links deep rough/brush alternating. 1 tile in 64 becomes a "spot" (see 9).
4. Creek (not on islands): from the far edge, at 16 + rand(16), stepping to whichever of three neighbours has the lowest
   noise, never straight back; water (brush in the Desert). A random side tile at sea level becomes a waste bunker or rocks
   bank (ravine in the Desert). It stops at existing water or the near edge.
5. Coast (coast 1): along the b = 0 side, each row's first w tiles become water, w starting at 5 and moving by -2..2 each row,
   kept in 2..12; the outermost tile of each row is out of bounds.
6. Corner heights: from the noise (finer scale for relief 2), lowered towards the near edge in the Desert, divided by the
   height scale, plus 1, clamped to 3 (4 above Easy)..15. Corners of water, wetlands and marsh tiles are at sea level.
7. 24 patches on a 4 x 6 grid, each a random walk of at least 5 tiles that ends with chance 1/16 per step: low ground gets
   water (Parkland, Tropical), palm (Desert) or rocks (Links); high ground gets rocks (Parkland, Desert) or palm (Tropical,
   Links). The first patch, and on Difficult/Impossible each with chance 1 in 8/(difficulty-1), is wetlands instead.
8. Elm trees become ordinary trees.
9. (4 - difficulty) x 9 wildlife spots on tiles that are not buildings, rough, wetlands, brush/ravine, or water near the coast.
10. Where a tile edge next to water or wetlands has both corners above sea level, the corner is lowered (at 4) or moved up or
    down by one at random.
11. Clubhouse (4 x 4): a random spot with both coordinates in 15..31 on plain ground where the footprint fits. Placing a
    building flattens its corners to the anchor's height and makes its tiles building tiles.
12. The property's feature: Monterey 16 brush spots (the cypresses), Ireland 16 rough spots (the leprechauns), San Diego
    (8 - difficulty) dolphin spots at sea, a free building beside the clubhouse with a 3-tile path (Rocky Mtns. Resort Hotel,
    Phoenix Swim Club plus a landmark, Carolina Putting Green, Scotland the Airstrip slot (drawn as the castle in Links),
    Florida Pro Shop), Nova Scotia two lighthouses on the shore, and (8 - difficulty) landmarks for Hawaii (waterfall pools),
    Oahu (garden pieces), Northeast (battlefield statues and cannons), Wales (standing stones), Spain (vineyards) and Jamaica
    (statues), each with its own ring of tiles around it.
13. Obstacles: one per difficulty step, on plain ground, ringed with brush in the Desert.
14. The first employee starts beside the clubhouse.
15. Owned land: m = the first k with acres x 10 >= 4 (25 - k)^2. Inland: a border m tiles wide all round becomes out of
    bounds (type 20). Coast: the same except on the sea side. Island: each row's border is |row - 25| clamped to 8..50,
    plus m - 12 + rand(3), and becomes water. The map before this step is kept; buying land later restores tiles from it.
16. Water depth: water tiles next to land are shallow, next to shallow water middling, the rest deep.

### Buildings (records of 0x14 bytes ending at 0x4c27f0: name, footprint edge, price in units)
Pathway 1/1, Benches 1/2, Flower Bed 1/5, Ball Washer 1/50, Landmark 1/15, Home Site 2/10, Putting Green 3/100, Snack Bar 2/150,
Pro Shop 2/200, Swim Club 3/300, Driving Range 5/250, Cart Garage 2/400, Marina 2/1000, Resort Hotel 4/2500, Airstrip 6/5000,
Clubhouse 4/200, Willow Tree 1/25, TV Tower 1/10, TV Booth 1/10, Scenic Bridge 1/100. The sprite for a building depends on
the theme (for example the Links set draws the Airstrip slot as a castle); that table is not decoded yet.

### Terrain classes and the out-of-bounds land
At start-up the exe hands Terrain.dll one class per tile type (byte +0x26 of the terrain table): tee 0, green 1, fairway and
firm fairway 2, rough, deep rough, mound, ravine, brush, rocks, wetlands and marsh 4, sand trap and pot bunker 7, waste bunker 8,
the four tree types 13, water 17, out of bounds and buildings 18. In the Desert theme every type with a positive byte +0x22,
except deep rough and sand trap, is sent as class 4. Terrain.dll does not draw out-of-bounds tiles at all.

### Re-check of the facts taken from the first decompile
The routines the port relies on were compared between the earlier decompile and the complete one: 14 are identical, and two
(the course statistics routine and the club planning routine) differ only in how Ghidra typed an array; the formulas are the same.

## Staff (employee routines 0x402970 and 0x402a40, path search 0x42e7e0, weeds in the main loop)

Confidence: high for the rules below (read from the decompile, with the machine code where Ghidra lost arguments). Implemented
in `sg-core/src/staff.rs`.

- Employee records: 64 of 0x4c bytes at 0x585850. Position in map units (1024 per tile), post tile, an active byte, a job code,
  facing 0..7, steps left on the current heading, a wait counter, an animation state (7..10 walking, 11 standing, 12 working),
  an action code with a countdown, the golfer last served and a count of golfers served.
- Jobs: the hire menu offers three choices per kind and maps choice c to job -2 - c/3 (Club Pro -2, Ranger -3,
  Groundskeeper -4, Soda Vendor -5); the third choice of each kind is the experienced version (Celebrity, Marshall, Lawn
  Technician, Refresher), which walks 2 faster, works faster and has its own clips and sounds. A new game creates job -6, the
  player's own pro ("Gary Golf" by default), posted on the clubhouse; it does all four jobs, walks 2 faster, and is drawn with
  golfer clips. Hired staff appear inside the clubhouse with a post 3 tiles off it on each axis (either side at random).
- Each tick an idle employee looks for work. The Groundskeeper (and Gary) looks for weeds within 16 tiles of the post, nearest
  by a measure that counts distance from both the post and the employee. The others look at every golfer standing on neither
  a tee nor a building and within 4 tiles of the post: the Ranger takes golfers not yet hurried and not busy, the Club Pro golfers
  he has not just talked to, the Soda Vendor golfers whose thirst is above 4 (above 0 for the Refresher). Gary takes whichever
  applies, and walks to the tile the player last clicked (except every other 512 ticks).
- Walking: one path decision per tile from a weighted flood out of the target tile (walking cost per ground type, diagonals +1,
  water and out of bounds +16 near the target, paths cost 0 or 1 when the target is a few tiles away, staff pay +1 on greens,
  fairways, tees and sand and half elsewhere, a tile next to another walker +2). Speed per tick is 6 minus the ground's walking
  cost, kept in 2..5, or 6 on a path, +2 experienced, +2 for Gary, 3 when leaving; a straight step moves speed x 32/3 units, a
  diagonal one speed x 8 on each axis. An employee gives way (waits 4..7 ticks) to another just ahead.
- On arrival: the Soda Vendor stops the golfer for 64 ticks and, four counts later (a count is 4 ticks), sells a drink: mood
  event 0x19, +2 units under Food/Drink, thirst back to 0 (the Refresher's drink always counts as quenching). The Club Pro stops
  the golfer for 32 more ticks and then raises event 0x22 (+1), or 0x3a (no change) when an inexperienced pro finds the golfer
  hungry, thirsty or tired. The Ranger marks the golfer as hurried: the golfer's walking speed is one step faster. The
  Groundskeeper pulls a weed (a weed already being worked on goes on the first visit). Each action plays a sound: slots 92..95
  for the four jobs, +4 experienced, +8 for Gary.
- Firing removes the employee at once and costs 25 units under Salaries.
- Weeds: on every tick with (tick & 3) <= difficulty one random tile may sprout a weed, if it has at most three weeds and at least
  one tee, green or fairway on its four sides and one of them is a weed, or (creek and garden tiles) with a chance that grows with
  the neighbouring weeds. Only fairway and rough- or tree-class tiles take weeds. The weed sprite is the theme's
  (dandelion for Parkland and Links, oil slick for Desert, dry grass for Tropical); its sound is slot 34 (`effects/fly.wav`).
- Weed growth (done per tile while drawing, so once per tick): a tile marked as being worked on advances its counter by one
  (with chance 1/8 while a weed stands there) and drops the mark once the counter is past 24. New weeds start marked, so they
  grow first; a grown weed is one a golfer complains about (event 0x18, -2, raised while planning a shot from that tile) and
  one the Groundskeeper must work on (his first visit marks it and sets its counter to the weed animation's frame count minus 3,
  later visits count it down and pull it below 2).
- Weed seeds: besides the generator's 0x1000 tiles, weeds start where unhappy golfers stand (see "Money over time").
- Sound slots are not load order: the loader passes each file's slot in a register. The full table was read from the machine code.

## Drawing objects (sprite registration around 0x43e000 and the object drawing routine 0x463180)

Confidence: high. Implemented in `sg-core/src/objects.rs`.
- At start-up the exe registers every sprite under a number. Building sprites keep their numbers across themes and load a
  different file per theme (so the Airstrip's numbers load the castle in Links, a casino in the Desert and a theme park in the
  Tropics; the Swim Club's load tennis courts in Parkland, a spa in the Desert and stables in Links).
- The drawing routine picks numbers by object kind and level, the level being 2 once the course has more than 10 holes:
  clubhouse base 0x1f9 (0x1f8), body 0x1f6 (0x1f7), animation 0x1fa (0x1fb); hotel base 0x1e9, body 0x1e6; airstrip base
  0x1dd, body 0x1da; pro shop base 0x1e3, body 0x1e0; swim club 0x1d7/0x1d4; cart garage 0x1c5/0x1c2; marina 0x1d1/0x1ce;
  putting green 0x1f1/0x1f0; snack bar 0x1ed/0x1ec (level 2: 0x1ef/0x1ee). Some themes skip a base layer.
- Landmarks draw as sprite 0x168 + their type (Sundial, Barn, civil war cannon, standing stones, ..., radio tower, oil pump),
  animated by the game tick, facing (their direction + the camera's quarter turn) of four views.

## Money over time, fees and quitting (main frame 0x40f5c0, fee routine 0x427380, mood routine 0x467a00)

Confidence: high. Implemented in `sg-core/src/economy.rs` and the game.
- Every 1024/(difficulty+2) ticks each active hole costs 1 unit (Maint./Interest), and each employee's wage is charged when
  rand(4 - difficulty) <= course rank. Wages per charge (units), basic/experienced: Club Pro 3/7, Ranger 2/3,
  Groundskeeper 2/4, Soda Vendor 2/5 (the hire screen shows them times 100 "per week"). Hiring is free; firing costs 25.
- No upkeep exists for terrain, paths, walls or buildings; no taxes, loans or interest on savings.
- At each month start (1024 ticks) debt pays cash/50 (2%, toward zero) under Maint./Interest.
- At each year end (8 months) negative cash moves the board one step (concerned, very worried, contract terminated); the third
  negative year end in a row ends the game, any positive one resets it. Sandbox games pay all costs but skip the board and
  the affordability check.
- An action is affordable when it costs under 1 unit, or no more than the cash, or in sandbox, or while the course has fewer
  than 3 holes.
- Course rank by holes: Municipal under 6, Daily Fee 6..9 (experienced staff), Country Club 10..17 (building upgrades, level 2
  art), Championship 18.
- Green fee per hole = the golfer's mood + 2 (Top 100 hole) + 2 (Top 18 hole) + the Airstrip's level + 2 (Gold member) or
  5 (Platinum member). The player cannot set a fee. (Earlier notes read the type bonus wrongly; corrected.)
- Golfers quit when their mood is below 0 between shots (not below -10).
- Weeds start from golfers: after a negative mood event, with chance (difficulty+1)/6, the golfer's tile gets a new growing
  weed (not on water, nor where a weed or footprint is). This answers the open question in "Staff".

## Placing buildings (main frame near 0x41f3a3, removal 0x40e400)

Confidence: high for the rules listed; the per-kind special cases (flower bed, landmark choice, home site value) partly.
- The cursor's footprint test (0x40db90) decides where a building may go and returns a clearing charge (trees, rocks, water
  under it; halved for a path). The cost is price x (level + 2) / 2 + clearing, booked to Facilities when placed. There is no
  "building lot" and no path needed to build; without a path joining it to the clubhouse a building is not operational
  (no income, monthly warning on Moderate and up).
- Each kind from the Putting Green up exists once: building it again removes the old one and places a new one at level + 1
  (an upgrade, with its cost); the exe only allows that from 10 holes ("Sorry, upgraded buildings are not available until you
  build beyond 9 holes.").
- Landmark cost (5 x id + 25) x 2 units, free when donated. Home sites need a site value of at least 50 (0x42ef40).
- Removal frees the record and turns the footprint into rough (deep rough on slots 12..15; water under a Marina in Parkland
  and Tropical), clearing its path marks; kinds 0..5 give their price back (landmark (level + 5) x 10).

## Golfers and shots (golfer update 0x4289e0, mood events 0x467a00, hole end 0x427380, planner 0x424120)

Confidence: high; read routine by routine, with the machine code where the decompile lost arguments. Implemented in
`sg-core/src/golfer.rs`, `planner.rs`, `course.rs` and `geom.rs`; docs/GAMELOGIC.md describes the behaviour.
- Golfer records: 152 of 0x100 bytes. Partners are slots g and g^1. Mood is the 16-bit field at +0xa4 (-10..10); the byte at +0x36
  is a separate momentum used only by pro golfers' shots, and it is what the shot planner reads as "mood" (so ordinary golfers
  always plan as if their momentum were below 2).
- Mood events: the event's amount is the table above; then an amount of -1 only counts for an ordinary golfer when the same event
  is among the last 3 (Easy), 5 or 10 (Hard) thoughts or the previous change was negative; other negative amounts become
  (amount - 1) / 2 except for the County commissioner; on Hard settings a negative event can pass event 0x2f to a cheerful
  partner. Snack (0x12), drink (0x19) and bench (0x1b) test hunger > 7, thirst > 7 and tiredness > 59.
- Walking: the speed is 6 minus the ground's walking cost (3..5, 3 when tired), 6 on a path, +1 for hurried golfers and on one
  tick in four; carts run at 8 + 4 per cart garage. Easy golfers walk slower (divisor 7 instead of 5).
- Arrival, detours, give-way, slow play, quitting, the swing and the ball are restated in docs/GAMELOGIC.md; the constants are in
  the code with the exe routine named.
- Quirks kept: the course record announcement compares with the table it has just updated, so it never happens; the ready test
  compares a y offset with a distance; ball tree sounds share slot 8 with a golfer's voice because the loader fills it twice.

## Garden items and undo (building tool 0x41f34e, undo 0x40a4e0)

Confidence: high.
- Paths, benches, flower beds, willows and scenic bridges are tile flags, not objects: a bench sets path + bench (0x220), a flower
  bed 0x5000 and turns the tile to rough, a willow turns it into an elm tile, a bridge sets path + scenic (0x120) on water.
- Each records an undo code and the amount paid; undoing gives exactly that back (booked to Build course). Buildings are never
  refunded: undo on one demolishes it.

## Building holes (painting 0x41fee2, layout pass 0x41353d, open hole 0x40e720)

Confidence: high (layout pass read from the machine code, where the decompiler lost call arguments).
- One hole is built at a time, number `next_hole` (0x5685f0). Painting a tee records it as that hole's back tee (and forward tee
  unless the game has two tees); a tee is one tile, so painting a second one for the same hole is refused with a message.
  Painting a green records the cup when the hole has none yet and the tile is not next to another green (or the tee is already
  down); the cup tile carries the hole number and 0x80. Painting over a hole's tee or cup takes the hole out of play (par 0)
  and lowers `next_hole` to it. With eighteen holes no more tees or new greens can be painted.
- While the hole being built has a tee and a cup, the main frame plans it with a trial golfer record (0x9a): first a golfer of
  the Length and Accuracy classes plans the tee shot only, then one of all three classes plays up to five shots from the tee,
  stopping when a landing is in a hazard (hazard byte above 1) or at the cup. The landings are kept and replayed until a
  terrain refresh clears game flag 0x40000. From them: the tee facing (heading to the first landing), the length (sum of the
  shot distances in range units, only when the shots reach the cup), flags 0x1000 / 0x2000 when the cup is more than one height
  step above / below the tee, flags 0x20 / 0x40 when the line to the cup bends more than 1/36 of a turn one way or the other
  (holes shorter than 250 count as straight), and three yardage markers 4, 6 and 8 tiles from the cup on holes longer than 150,
  200 and 250. A marker must stand on fairway clear of its tile's non-fairway edges; the exe tries 25 headings around the line
  toward the first landing and keeps the last that fits.
- Opening (key H, or a click on the cup green of a hole with no par) needs the tee and the cup. Without a planned length it
  uses the straight distance, with a quarter of anything over 250 added. Par: 2 under 51, 3, 4 over 249, 5 over 474, 6 over
  625, after adding 25 to doglegs over 250 and taking 25 per driving-range level off lengths over 300. It clears the hole's
  statistics and the best rounds, sets the heading from the forward tee to the cup, and `next_hole` becomes the first hole
  without a par. Opening hole 1 moves the first employee's post a third of the way from the tee toward the clubhouse.

## Tile decorations: trees and weeds (main frame tile loop 0x413da3)

Confidence: high for the choice of sprite, palette and placement; screen offsets are converted to tiles at the default view.
- Tree (13), pine (14) and palm (15) tiles draw their own trees; elms and scenic tiles are drawn elsewhere. For each of the
  four axes through the tile the exe checks whether the two neighbours are both of the tile's scenery group or both not;
  0 such axes give the small sprite, 1 or 2 the medium, 3 or 4 the large. A tile with a variant byte (painted trees: the
  brush variant, 12..14) takes size `variant % 3` instead.
- Broadleaf: sprites 0x19b..0x19d (maple, Joshua tree, tropical brush), palette 0x25 + ((noise(a*256, b*256) / 32) & 3).
  A fully grown broadleaf with no broadleaf among its eight neighbours is drawn as the scenic elm in Parkland and Links.
- Pine: sprites 0x192..0x194, or the second species 0x198..0x19a where (noise / 13) & 4 (the variant's (v / 3) & 1 for
  painted ones); Desert and Tropical take the species' own palette, Parkland and Links one of four by the noise.
- Palm: sprites 0x195..0x197, palette 0x32 + ((noise(a*128, b*128) / 32) & 3).
- Small pines and palms are drawn as two trees on the tile; other trees once, nudged sideways by ((a + 3b) % 5 - 2) zoom
  units. The view is (a - 2b) & 3.
- A painted tile restarts its growth counter (flag 0x4000); a growing tree shows the counter as its frame, plays its sound
  (0x93 broadleaf, 0x91 pine, 0x92 palm) when it starts and stops growing at its last frame.
- Weeds: crabgrass (0x184) on types that are no hazard, the theme's weed (0x190: dandelion, oil slick in Desert, dry grass in
  Tropical) elsewhere; frame = last frame - (b & 2) - (a & 1), never below (a + 2b) & 3, the growth counter while growing.

## Tile decorations: flags, tee markers, benches, ornamental trees (tile loops 0x410604 and 0x412354)

Confidence: high for sprites, palettes and the rules; the flower bed shape table and the bridge pieces are not ported (the
shape table is data this project could not read).
- Cup tile: the theme's flag pops up (0x185 + theme) while the tile grows, then waves (0x189 + theme) in view
  clamp(mood sum * 100 / (tee shots + planned shots / 2 + 4) / 10, 0, 3); a hole not yet open shows view 1. Palette 0x63.
- Tee of an open hole: two markers, red (par 3), white (par 4) or blue (par 5 and up), either side of the tee across its
  facing: 4 and 2 zoom units out on a diagonal heading, 5 and 3 on a straight one. The exe also rewrites the tee tile's
  variant from the facing and the dogleg flags, which picks the tee's ground texture.
- Bench tile: one seat per heading a golfer can sit facing (0x407400), sprite 0x208 + variant % 7 with its own palette,
  4 and 2.5 zoom units toward that side; a tile with no such heading loses its bench flag.
- Scenic elm tile (the willow tool): a bed of scenic flowers and an ornamental tree, plum, dogwood, elm, Japanese maple,
  cypress, scenic tree or peach by variant & 7, or the willow (coloured by the noise) for 0.

## Buying land and home sites (land screen 0x4587a0, lot value 0x42ef40, main frame 0x417a08)

Confidence: high.
- Nine tracts of 16 x 16 tiles from (1, 1); a tract is for sale while it holds out-of-bounds tiles. Price: f = 5, plus
  2^purchases for each out-of-bounds tile with chance 1/3; price = f * 20 / 100 thousand dollars. The rolls repeat on every
  redraw of the screen (consuming the generator) but the first prices stay. Buying restores the tiles the land generator had
  marked out of bounds. A bought tract keeps a price of $5,000, and clicking it charges that again and still counts as a
  purchase (an exe bug kept as is).
- A home site needs Silver or better members to outnumber the sites, and a lot value of 50: the twelve tiles around the 2 x 2
  lot (rough 12, water 32, trees 25 or more, other lots -16, tees, greens and fairways -8, ...) times the best hole fun over
  distance (fun = mood sum * 1000 / (planned / 2 + 4 + tee shots) + (3 - difficulty) * 100, +100 each for Top 100 and Top 18
  marks; distance in tiles to tee, cup or the far yardage marker, plus 8), divided by 40; a Marina in use adds a third per level.
- Placing sells the lot at once for a quarter of its value (Home Sites); demolishing buys it back for its smoothed value / 50
  plus half its value. At each charge interval every (difficulty + 2)th object, starting at (tick / interval) % (difficulty + 2),
  re-values its site: v = value + v - v / 12. The site shows a sign below 200, a house being built below 600, then a house.
- Path tiles on water are bridges (0x41192c). With no path neighbour on water: a scenic bridge (one of eight looks, the tile's
  low flag bits) for the bridge tool, otherwise a deck end cap on either side. Connected to other water path tiles: the deck
  piece by the four-neighbour path mask (straight, corner, T, cross, with a view per mask), a reflection under straight
  pieces, caps toward dry path neighbours, and a ramp where a straight piece meets land on one side (which ramp view goes
  with which side is inferred from the order of the exe's cases).

## Golfer stories (story table 0x4659a0, pairing 0x45de80, beat 0x466370, choice 0x4669f0)

Confidence: high.
- Every *.txt in the theme's folder (the Standard folder when it has none) is a story: a title line, then four scenes, each an
  opening line and indented replies, best first. The exe stores the file names 100 bytes apart but reads them 50 apart, so
  story id 2i is file i and odd ids are empty; the tutorial id is the OpeningDay file's list index used as an id (kept).
- The first eight letters of the file name are conditions on the pair (genders, marital status, trait bits, ages) and the
  landmark the story gives. A pair gets one of up to three random stories that fit and are not being played on the course;
  the opening pair before tick 0x800 gets the tutorial. After N Happy Endings a pair keeps its story with chance 7/N.
- Pairing also sets both moods to 2 + compatibility (5 less the differing trait bits).
- A beat needs the pair on one hole, both standing, within 0x3000 / (scene + 6) map units, 150 ticks since the last beat on
  the course and the owner's smoothed mood above 4 * scene + 4 (the exe also needs the owner on screen). The partner's reply is
  (mood - 1) / 2, capped below the best after scene 1, nudged by the newest thought and a "stories go well" landmark. The best
  reply passes the scene (owner mood + 1), one less fails quietly, lower fails with owner mood - 1; a failed scene goes back
  and the owner's smoothed mood loses a third.
- Passing scene 4 is a Happy Ending: the hole is marked in both members' records (worth membership points and a good thought
  later) and the story's landmark type becomes available and free once, if a random walk from the owner reaches rough.

## Special visitors (pairing 0x45de80, verdicts 0x4266b0, celebrity homes 0x417a08)

Confidence: high.
- The second golfer of pair number v = (slot / 2) % 6 becomes a visitor when: v = 1, a CEO (at most 8 a game, CEO k needs hole
  2k + 2); v = 3, the County commissioner (not on islands, rating above (purchases + 2) * (difficulty + 2) * 50); v = 5, the
  Wealthy heiress (rating above (donations + 1)^2 * 25). The rating is fun on the two easier difficulties, skill (hundredths)
  on the harder ones. No visitors in sandbox games. Visitors get four random special skills plus fixed ones, values 1..8.
- A visitor is pleased after playing to the last hole with mood 3 or more: the CEO invests $5,000 ($10,000 at mood 5 or
  more), the commissioner approves an expansion (the land offer follows), the heiress donates a landmark type (free to place
  once). Otherwise only a message.
- Each re-valued home site draws a celebrity; an unowned site worth more than (300 * homes bought + 400) * (difficulty + 2)
  is bought as a vacation home. No money changes hands.


## The player's pro (round start 0x40f190, aiming 0x41b20f, skills 0x409620 and 0x407e00, match money 0x427380)

Confidence: high unless marked.
- Panel button 1 asks for a practice round and drops a pending challenge's flag; button 2 asks for the match while a challenge
  is pending. Both need two holes, no tournament and the pro off the course. The round starts next frame: golfers waiting
  or unstarted on hole 1 go home, an even slot and the next one are made partners; in a match the even one becomes the
  famous golfer (his table skills, stake nibble L*16 + 7), and the odd one the pro (stake nibble 4, his own skills) at his
  employee's place. The employee is hidden while he plays and stands where he finished.
- Each full shot off the green 25 or more yards from the pin waits for the player. The aiming frame snaps to the hovered
  tile's nearest corner when the pointer is closer to it, limits the distance to the shot type's range (high 13/18, low
  1/2), refuses fade, draw and high from a hazard (sound 0x18) and picks a preview club; a click is refused for the first
  ten frames. The real shot goes to the centre of the tile whose corner was chosen. The aim line uses fractions 5, 9, 12,
  15, 16 and 20 of the distance over 20 (16 for the high shot), bending by a sixth of a turn times the shot shape (the
  second point's bend and the heights are medium confidence).
- When a shot of the pro ends on a playable lie and beats (D + 1) times a bar that grows with his skills, difficulty and the
  shot's difficulty, one skill grows (picked at random among those the shot showed: putting on the green, recovery from a
  hazard, luck after a tree or a hole-out, driving off the tee, the shot shape, power when the shot beat his range, irons
  for middle clubs), up to 6, 8, 10 or 99 by course rank; a new skill starts at 3 - difficulty. Ending in a hazard may
  cost a skill above 3 one point and his momentum.
- A famous golfer challenges when a newcomer with all three skills arrives after the first year, hole L + 3 exists and no
  pro is out; the challenger's skill total is close to 20 + 5L. Each hole of the match is worth 20L units, and the match
  again; a loss or a tie lowers L, a win adds an accomplishment (three skill points).
