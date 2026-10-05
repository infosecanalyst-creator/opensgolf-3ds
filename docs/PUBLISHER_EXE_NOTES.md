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
- After each hole mood falls by (hole field + 6 + holes played) * (mood - 1 + difficulty) * (difficulty + 1) / ((course factor * 5 + 15) * 8), integer division. The hole field and course factor are not decoded; the viewer uses 0.
- Mood smoothing uses old * 7 / 8 + new (not used by the viewer yet).
- Not yet decoded: the events that raise mood, the club fun and skill ratings. The viewer's +1 for par or better is a placeholder.

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
- The viewer implements these; mood changes on a hole are placeholders (+1 par or better, -1 triple bogey or worse).

## Terrain table (static table at 0x4c1a40 in the publisher exe, 0x30 bytes per entry)

Confidence: the table layout is read from the exe; "cost" is the byte the build menu multiplies by 100 and shows after "Cost:" (medium).

Cost per tile in units of 100 by tile id: tee 5, green 10, fairway 3, firm fairway 3, rough 1, deep rough 2, mound 4, sand trap 6, waste bunker 4, pot bunker 8, ravine 10, brush 4, rocks 4, tree 10, pine 10, palm 10, elm 25, water 50, wetlands 2, marsh 6, building 0. A second byte per entry (1 to 8 for most; water 8, out of bounds 8, tee 0) looks like a hazard severity and is not used yet.

Course ranks named in the exe text: Municipal, Golf Club, Country Club, Championship (upgrade messages mention beyond 9 holes for upgraded buildings).

## Clubs and shot planning (partial)

Confidence: low to medium; only the outline is read.
- The exe names 14 clubs by index 0..13: Putter, Sand Wedge, Lob Wedge, 9 Iron down to 2 Iron, 4 Wood, 3 Wood, Driver (names table around 0x4c51b0).
- The golfer's club index is chosen in the shot planner (around line 15290 of golf_decomp.c) from the distance to the target after scoring candidate landing spots; the planner weighs the terrain value under each spot (the second byte per entry of the terrain table) times distance, with a 5 added when two risky spots are found. The club is then derived as (max range - needed range) * 60 / (3 * max range), clamped to 1..11, with 5 forced and 13 (putter) used on a green.
- Not decoded: per-club distances and spread, skill scaling. The viewer's shot model stays a placeholder.

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

- The club choice does not set distance: the golfer asks for a range (distance to the target in range units, capped at the maximum range) and the exe solves the launch speed by bisection so the carry is 4/5 of that range; the club is only a display. The solver uses a coarse two-tick step version of the flight. include/sg/flight.h reproduces this; its output carries about 96 percent of the 4/5 target.
- Vertical launch speed = (est / 8) + 512 where est = u * 33 - u * u / 48 + 64 and u = range * 20 / 25.
- Aim error: a random hook or slice amount is drawn per shot (a table-free random value: 50-wide roll, halved under 20, minus 10 under 40, doubled less 50 above) and scaled by skill bytes; it bends the heading every tick. Full scaling by skill is not decoded.
- Scale: 25 range units equal one tile. How many yards a range unit is, and the tick length in seconds, are unknown; the viewer uses placeholders (kTicksPerSecond = 40).

## Calendar (date stamp routine in the exe, near golf_decomp.c line 8081)

Confidence: medium. The stamp shows month = (ticks and 0x1fff) / 1024 + 3, so months 3 to 10, and day = (ticks and 0x3ff) * 30 / 1024 + 1. One month is 1024 ticks, one day about 34 ticks, and the exe's time wraps every eight months (March to October). The viewer now uses an eight-month year; the start year is a placeholder.

## Property prices and amenity income (golf_decomp.c, near 0x46f1d0 and the golfer visit code)

Confidence: prices high (table read directly), amenity income medium.
- The property price table in the exe has 16 entries, in units of 100: 500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000. That is the same set of prices as properties.h, so the chooser prices match the exe. (Which price belongs to which place is not read from the exe; properties.h keeps the screenshot order.)
- Amenity visits pay the course in units of 100: building type 7 pays 5; type 6 pays 4 or 8, type 8 pays 6 or 10, type 10 pays 8 or 12, where the lower value applies when the building's upgrade level is below 2.
- Removing a building 25 units is charged by one routine (-0x19); a removal pays back part of the build cost (not decoded).

## Where money moves (search result, negative findings included)

Confidence: low to medium. Direct writes to the cash variable (0x571fd4) in the decompile are: the starting value 1000 units, golfer fees and amenity payments (additions), property purchase (price table), the 25 unit removal charge, a building removal cost in a second routine, golfer wagers, and a few small additions. No direct monthly wage or upkeep deduction was found, and the construction cost deduction was not found either. Cash may also be changed through another alias or a ledger of per-month entries (100-entry table with income and expense columns around 0x584212 to 0x58421e), so the absence is not proof that wages do not exist. Staff wages and upkeep in the viewer therefore stay placeholders.
- The cash reserve display and the finance graph choose their scale from cash thresholds 2500, 5000, 10000, 25000, 50000 and 100000 units (graph steps 2, 4, 10, 20, 40, 80).

## Opening holes, building unlocks and course upgrades (hole-opened handler FUN_0040e720, golf_decomp.c near line 12572)

Confidence: the open rule and the unlock schedule high (read from the code and the unlock counter's start values); the effect wording of each unlock medium; the course stage names and what each stage changes low.
- A hole is built as a tee, then a green, and then it must be opened (the H key). Opening needs both a tee and a green to exist for the hole; otherwise the exe plays its error sound and does nothing. Golfers only play open holes, and the exe's text says a hole must be opened before the next one can be built. A tee already built for a hole must be deleted (right click) before it can be moved. At most 18 holes. The game text also says a building is not operational until a path joins it to the clubhouse.
- Opening a hole: the hole's length in yards is computed from the tee and green positions (a few adjustments: longer when a dogleg flag is set, shorter when the Driving Range upgrade level is high). The hole class from that length: up to 50 yards is class 2, up to 249 class 3, up to 474 class 4, up to 625 class 5, longer class 6 (this is the par). The announcement names the hole, uphill or downhill, and a dogleg left or right, then says it is now open for play.
- Unlock counter (0x5a6364). A normal new game sets it to 6; Sandbox mode sets it to 17 (everything available, money 100000 units, a flag set for all unlocked). A building type with index b can be built when the counter is greater than b. The building type names in the exe's table order (index = unlock level): 0 Pathway, 1 Benches, 2 Flower Bed, 3 Ball Washer, 4 Landmark, 5 Home Site, 6 Putting Green, 7 Snack Bar, 8 Pro Shop, 9 Swim Club, 10 Driving Range, 11 Cart Garage, 12 Marina, 13 Resort Hotel, 14 Airstrip, 15 Clubhouse.
- Each time a hole opens, with n the number of that hole: holes 6, 10 and 18 upgrade the course instead (below); otherwise, if n is greater than (counter minus 5) and the counter is at most 14, the building type at index `counter` unlocks and the counter goes up by one. From a normal start this unlocks Putting Green at hole 2, Snack Bar at 3, Pro Shop at 4, Swim Club at 5, Driving Range at 7, Cart Garage at 8, Marina at 9, Resort Hotel at 11 and Airstrip at 12. Hole 1 unlocks nothing. The build panel marks a locked building with a message that it becomes available later.
- What each unlock does, per the exe's text (effects not yet implemented here): Putting Green helps golfers with imagination improve their putting; Snack Bar feeds hungry golfers and pays extra; Pro Shop lets accurate golfers upgrade equipment; Swim Club starts golfers in a better mood (matched to the slot by the text order; the Tennis Court in this project's list is placed there by guess); Driving Range lets long hitters improve their length; Cart Garage speeds play; Marina raises home and lot values; Resort Hotel keeps golfers from tiring late in a round; Airstrip allows higher green fees. The Ball Washer improves the accuracy of golfers on its hole and cheers them (case 3, index 3, available from the start).
- Course stages: the strings "Municipal", "Golf Club", "Country Club" and "Championship" appear with text saying the course is upgraded, upgraded buildings become buildable, and more experienced employees can be hired for a slightly higher salary. The handler calls its upgrade routine when hole 6, hole 10 and hole 18 open (stage 1, 2 and 3), so those are taken as the thresholds. What the upgrade changes is not decoded.
- The advisor's first-hole script, in the order the exe shows it: build a tee, then a green, then press H to open the hole, then golfers arrive, then build more holes and make them different, add paths, buildings need a path to the clubhouse, flowers and benches cheer golfers, hire employees (the project's advisor text is its own wording).

## Single-player structure: visitors, goals, tournaments, membership, board (exe strings and the visitor handler FUN_004266b0)

Confidence: the list of systems high (the strings and screens exist); the investor payout medium; every threshold and trigger not decoded (marked below).
- Special visitors. Four kinds are announced as "is playing your course today": a corporate CEO (may invest in a seat on the board), a county commissioner (may approve a request to buy more land), a wealthy heiress (may donate a scenic landmark) and an international celebrity (may buy a vacation home on the course). Each has a yes and a no message; the no messages say the visitor will be back if the skill rating or the fun rating is over a figure. Landmarks are placed from the improvements menu and bring one of four effects nearby (golfer stories go happily, skills improve rapidly, no dandelions, happy thoughts). Thresholds and arrival timing: not decoded.
- Investor payout (handler at 0x4266b0): when the visitor leaves happy and the course has a rating tier above 2, the club is paid 5000, or 10000 when the tier is above 4, and the investor count is limited to 8 (the counter must stay below 8). The tier variable's meaning is not decoded.
- Professional accomplishments (a trophy list that also awards skill points for a customised player character): 1st Tournament, First match victory, First 9+ hole course, First 18 hole course, 1st Top 100 hole, 1st Top 18 hole, 1st Classic hole, 1st Challenge hole, 1st Heroic hole, 1st Strategic hole, 1st skill upgrade, 1st $500,000 Tournament, 1st $1,000,000 Tournament, first tournament victory (9+ holes and 18 holes), 1st Grand Slam course (9+ and 18 holes), Grand Slam victory per theme (Parkland, Desert, Tropical, Links), 1st 100 star rating.
- Tournaments. The SGA offers to hold a tournament with a first prize (shown in thousands); the tiers in the text are Jr. Qualifying School, SGA Qualifying School, Jr. Tour Event and Championship, SGA Amateur Championship, Senior SGA Tour Event and Championship, SGA Tour Event, SGA Players Championship, SGA Championship, Mini Slam and Grand Slam. A course that passes the stricter SGA evaluation (after an upgrade) doubles prize money. You start the tournament from a panel button; a leader board and results screen follow; the course cannot be saved during one. Famous golfers can also challenge you to a match with a wager per hole (match play, pay or collect). Offer rules and prize sizes: not decoded.
- Membership. Golfers can become members, graded Member, Silver, Gold and Platinum; there is a Membership Roster screen and a membership count that rises and falls; members pay extra per hole, like customised carts and pay a lot for home sites. A waiting list counter exists. Rules: not decoded.
- Board and finance: already in the port (two years to return to positive cash, concern, worry, termination).
- Golfer stories: a golfer's story has chapters (View Story, Next Chapter) and may end happily (joins the board, places in a tournament, buys a home) or not; the Happy Endings improvement makes stories proceed happily.
- Why golfers leave: too tired, too thirsty, too hungry, waiting too long, thinks the course needs improvement, clubs into the lake. Golfers have thirst, hunger, energy and attitude meters.
- Building improvements: a list of upgrades (higher greens fees, happier golfers, golfers become members, higher property values, faster play, sell lot for cash, helps accurate, hungry, long and imaginative golfers) at 5000 to 20,000; upgraded buildings are not available until the course has more than 9 holes.
- Information screens: Financial Report, Membership Roster, Professional Accomplishments, World Map, Best Scores, Top 10 Designers, SGA Evaluation, keyboard shortcuts.

## The player's character (strings in the exe, character files on the disc)

Confidence: that the system exists and what it offers, high; the details of each part, not decoded.
- At the start of a game the player is offered "customize my character" or to stick with the default, named Gary Golf in the exe. The character can be given a name, a gender and a personality. The text says the player is represented by this character. The advisor lines are written in the first person ("Let's build our first golf hole", "If you hire employees I won't have to do everything myself", "I'm ready for a practice round", "I'm ready to play a tournament"), and a PARTNER token stands for the other golfer, so the advisor speaks as the character.
- Before playing the character's golf skills can be improved with skill points; more skill points are won for each accomplishment on the trophy list; a tournament preparation list can recommend adding three skill points. A warning appears when skill points are left unspent. The character plays the tournaments and matches.
- The character editor lists gender, skin tone, hair colour, pants colour, shirt colour, face, adult or child, body type, with save and load of character files and an "update bio" button. A record shows rounds played, handicap and low round.
- On the disc: Themes/<pack>/*.chr (a character file with sprites and the character's comments in it, about 40 KB each), Themes/Standard/*.glf (golfer files such as the famous golfers) and progolfers.dta and celebrities.dta. The formats of .chr and .glf are not decoded.

## Game clock and the board (decoded)
* The tick counter at 0x834170 drives the calendar: year index = ticks >> 13 (8192 ticks a year), month index = (ticks >> 10) & 7. The year has EIGHT months: March, April, May, June, July, August, September, October. Displayed year = 2001 + year index. (Start year 2001 matches; one tick is a small game step, the real-time length of a tick is not decoded.)
* The year-end screen (function at 0x44cff0) is called when ticks % 8192 == 0 and the screen flag is clear. The same place decays several per-golfer arrays by one eighth.
* Board rule at year end (not in sandbox, flag 0x1000000 of 0x59e7b8): cash (0x571fd4, units of 100) below zero moves a strike counter (0x5a6374): 0 to 1 "The board is concerned about our negative cash situation. You have two years to return to positive cash."; 1 to 2 "The board is very worried about our lingering debt. You have one more year to get out of debt."; the third negative year end "You have been unable to make a profit on this course. Regrettably, the board has terminated your contract." Cash at or above zero resets the counter to 0.

## SGA evaluation and tournaments (decoded, function 0x44fb30; implemented in sg/sga.h, not yet wired to the sim)
* Course grade from the number of open holes H: up to 5 Municipal, 6 to 9 Golf Club, 10 to 17 Country Club, exactly 18 Championship (more than 18 gives no grade).
* Ten criteria, each scored 0 to 10 and drawn as pips; a 0 prints "not acceptable" and makes the total -999; otherwise the total is the sum (max 100). Targets: five, nine, H-1 or 18 holes by grade.
  1. Length of Course: ideal yards = ((target*100)/18) * (grade*5 + 57). Eighteen holes: 10 - (ideal - yards)/100; otherwise 10 + ((yards - ideal) * (grade+4) * 5) / ideal. Clamped.
  2. Number of Holes: 10 - |target - H| * (4 - grade).
  3. Time to Play: 10 - (minutes - 235)/6 (C division); a result of 0 under 301 minutes becomes 1. The minutes are the sum over holes of (hole time total / rounds counted) / 40.
  4. Fun Factor: (fun% - 109)/10 + 10, where fun% averages a per-hole figure that can exceed 100.
  5 to 9. Variety, Scenic, Length, Accuracy, Imagination holes: count - target + 10. Scenic counts a hole when half of one stat plus two more exceeds 7 (the stat meanings are not decoded). Length/accuracy/imagination counts come from a comparison of golfers with and without the skill; the length differential threshold is 25 on difficulty 0 and 50 otherwise.
  10. Facilities on Site: (kinds of facility built - target/2) * 2 + 8, kinds counted from bits 6 to 19 of a built-types mask (0x5a6370).
* Totals of 90 or more spawn extra objects on holes 9 and 18 (object kinds not identified).
* Tournament by total: under 45 Jr. Qualifying School, then every 5 points: SGA Qualifying School, Jr. Tour Event, Jr. Tour Championship, SGA Amateur Championship, Senior SGA Tour Event, Senior SGA Championship, SGA Tour Event, SGA Players Championship, SGA Championship, and from 90 the Mini Slam (Grand Slam for an 18 hole Championship grade course). A total of 0 or less prints "Improvement Required" and no tournament.
* First prize in thousands = (((total - 50)/5) + 5) * (grade + 1) * (H + 1) * 2. Each lower place gets two thirds of the place above. A prize of 0 is replaced by H*20 - 20. When the winner is the player slot (slot 1), the club's cash rises by the prize and a "fame" counter (0x5787cc) rises by 4 - place (at least 1).
* The offer: when ticks % 8192 == 4096 (the start of July), no tournament is pending (0x5a59f8 is -1) and the flags 0x1200000 are clear, the evaluation runs silently and, if it yields a prize, the SGA offers: "The SGA is interested in holding a tournament at your course. 'We'd like to schedule the <name> at your course with a first prize of <n>,000. Open Golf Tournament here as soon as possible.'" Playing the tournament, the field, scoring and leaderboard are not decoded.
