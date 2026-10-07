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

## Hole and club ratings (decompile of the course statistics routine)

Confidence: structure read from the code; some counters are not decoded.

- For each hole the exe keeps average strokes per golfer skill class, starting from a prior of 8 samples at par. A skill counts as demanded when golfers lacking it average 50 hundredths of a stroke or more above full-skill golfers (25 on the easiest difficulty); 50 or more also sets a "strong" flag. The three classes are Length, Accuracy and Imagination.
- Club skill rating = sum over holes of the three differences, kept in hundredths and printed with two decimals. Length, Accuracy and Imagination sub-ratings are the same sums per skill.
- Hole fun = 100 * (sum of mood changes at the hole) / (half of an undecoded counter + 4 + plays). Club fun rating = sum of hole fun, printed as a whole number.
- The game implements these; mood changes on a hole are placeholders (+1 par or better, -1 triple bogey or worse).

## Terrain table (static table at 0x4c1a40 in the publisher exe, 0x30 bytes per entry)

Confidence: the table layout is read from the exe; "cost" is the byte the build menu multiplies by 100 and shows after "Cost:" (medium).

Cost per tile in units of 100 by tile id: tee 5, green 10, fairway 3, firm fairway 3, rough 1, deep rough 2, mound 4, sand trap 6, waste bunker 4, pot bunker 8, ravine 10, brush 4, rocks 4, tree 10, pine 10, palm 10, elm 25, water 50, wetlands 2, marsh 6, building 0. A second byte per entry (1 to 8 for most; water 8, out of bounds 8, tee 0) looks like a hazard severity and is not used yet.

Course ranks named in the exe text: Municipal, Golf Club, Country Club, Championship (upgrade messages mention beyond 9 holes for upgraded buildings).

## Clubs and shot planning (partial)

Confidence: low to medium; only the outline is read.
- The exe names 14 clubs by index 0..13: Putter, Sand Wedge, Lob Wedge, 9 Iron down to 2 Iron, 4 Wood, 3 Wood, Driver (names table around 0x4c51b0).
- The golfer's club index is chosen in the shot planner (around line 15290 of golf_decomp.c) from the distance to the target after scoring candidate landing spots; the planner weighs the terrain value under each spot (the second byte per entry of the terrain table) times distance, with a 5 added when two risky spots are found. The club is then derived as (max range - needed range) * 60 / (3 * max range), clamped to 1..11, with 5 forced and 13 (putter) used on a green.
- Not decoded: per-club distances and spread, skill scaling. The game's shot model stays a placeholder.

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
- The property price table in the exe has 16 entries, in units of 100: 500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000. That is the same set of prices as properties.h, so the chooser prices match the exe. (Which price belongs to which place is not read from the exe; properties.h keeps the screenshot order.)
- Amenity visits pay the course in units of 100: building type 7 pays 5; type 6 pays 4 or 8, type 8 pays 6 or 10, type 10 pays 8 or 12, where the lower value applies when the building's upgrade level is below 2.
- Removing a building 25 units is charged by one routine (-0x19); a removal pays back part of the build cost (not decoded).

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
- A third counter, tiredness, drives the bench events (0x1a, 0x1b); benches are not built in the game yet.
- The game follows these rules with two placeholders: needs update every 4 seconds (160 ticks at the assumed 40 ticks a second),
  and a vendor or snack bar is "reached" by chance or distance instead of by walking there.

## The game tick (machine code; the decompile has a gap here)

Confidence: high for the tick, open for its length in seconds.
- The decompile has no functions between 0x40f190 and 0x421b60: Ghidra did not turn about 74 KB of code into C, and the main
  frame routine is in that stretch. The facts below come from the machine code.
- One tick is one simulation step. At the end of the frame routine the game updates the golfers, then the staff, then a third
  routine, then adds one to the tick counter at 0x834170. When bit 0x8 of the game state flags is set, the counter is also rounded
  up to an even number, so updates gated on odd ticks are skipped (a fast mode). Right after, `(tick & 3) <= difficulty` gates a
  further update.
- The pacing routine waits in units of 10 ms (its argument times 1000 / 100). Which wait the main loop uses, and so how many ticks
  run per second, is not settled yet: it sits in the undecompiled stretch. Until it is, the port keeps 40 ticks a second.
