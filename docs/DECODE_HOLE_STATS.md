# Per hole statistics (publisher exe, decoded)

Source: the publisher-supplied golf.exe only. Facts are written in our own words; code is in include/sg/holestats.h and src/holestats.cpp (test: /tmp/claude-0/hs/test_holestats.cpp, build with `g++ -std=c++17 -Iinclude -include string src/holestats.cpp test_holestats.cpp`).
Confidence: EXACT = integer rules read straight from the code; GUESS = my naming or reading.

## 1. The record

A table of 20 records (0..19, record 19 is scratch space used while a hole is edited), 0x208 bytes each, starting at 0x575ab0. Holes are 1 based: hole h is at 0x575ab0 + h * 0x208. A record whose par byte is 0 is an unopened hole. Offsets below are inside the record.

| Offset | Size | Meaning |
|---|---|---|
| 0x000 | byte | par (0 = hole not open) |
| 0x002 | byte | compass octant of the last leg (angle from the waypoint at +0x10 to the green, octant = ((angle >> 28) + 1) >> 1 & 7) |
| 0x004 | short | yards (0 means "measure it" when the hole is opened) |
| 0x008 / 0x00c | int, int | tee tile x, y |
| 0x010 / 0x014 | int, int | last waypoint before the green (tee, or dogleg point), x, y |
| 0x018 / 0x01c | int, int | green tile x, y |
| 0x020 | int | rounds counted: golfers who have hit a tee shot here |
| 0x024 | int | shot plans: times a golfer planned a normal shot toward the default target (any shot, putts included; not counted while a golfer is in the "state 0xd" sub mode) |
| 0x028 | 88 shorts | stroke histogram: 8 groups (golfer skill mask 0..7) of 11 bins (strokes 0..10). Bins 1..9 are read, bin 0 and 10 are never read. Index = mask * 11 + strokes. |
| 0x0d8 | 64 shorts | reaction counts per event type (section 3). Only non zero reactions are counted. |
| 0x158 | short | fun total: signed sum of reaction deltas |
| 0x15a | short | quits: golfers who gave up or left the course while on this hole |
| 0x15c | short | sum of tee drive distances in yards (over counted rounds) |
| 0x15e | short | fairways hit |
| 0x160 | short | greens in regulation |
| 0x162 | short | putts |
| 0x164 | short | not used (never read or written) |
| 0x166 | short | longest drive |
| 0x16c | 64 shorts | last location argument per event type (the comment screen uses it & 0x3fff to say what the comment is about) |
| 0x1ec | int | total time: sum over finished holes of (elapsed ticks / 2) |
| 0x1f0 | int | hole expense (see section 6: effectively always 0) |
| 0x1f4 | int | revenue: fees collected, money units of 100 |
| 0x1f8 | int | second expense term (never written) |
| 0x1fc | int | variety counter (section 5) |
| 0x200 | int | flags (below) |
| 0x204 | int | unused |

Flag bits (offset 0x200): 0x01 named a Top 100 hole; 0x02 named a Top 18 hole; 0x04 too hard; 0x08 too easy; 0x10 transient (cleared every analysis); 0x20 and 0x40 layout bits, 0x20 is "dogleg to the left" (confirmed by the name text), 0x40 is its right twin (GUESS); 0x80 naming popup pending; 0x100 / 0x200 / 0x400 strong length / accuracy / imagination demand; 0x1000 uphill; 0x2000 downhill. The setters of 0x20, 0x40, 0x1000 and 0x2000 were not located.

Golfer groups: a golfer carries a skill mask in the low three bits of byte 1 of its record (1 length, 2 accuracy, 4 imagination). Normal games only create golfers with masks 3, 5, 6 and 7 (each lacks at most one skill); the choice is the mask used least so far for that golfer class. When game flag 0x40 (word at 0x59e7b8) is set all eight masks are created. The histogram group is the mask. Only ordinary visiting golfers (golfer kind byte 0) enter the histogram; staff, pros and celebrities do not.

When a hole is (re)opened (function 0x40e720) it zeroes revenue, plans, rounds, time, quits, fun, histogram bins 0..9, and the 64 event counts. It leaves drive sum, longest drive, fairways, greens, putts and the location array alone.

## 2. Who writes what

Golfer finishes a hole (0x427380, after the holing putt has been added to the stroke count):
- if ordinary golfer: hist[mask][clamp(strokes, 0, 9)] += 1.
- revenue += fee (unless game flag 0x200000 is set). Fee, money units of 100: golfer mood (-10..10), doubled in the mode where global 0x543cf4 is 2 (meaning unknown), +2 if the hole is Top 100, +2 if Top 18, + global bonus 0x543cd8 (source unknown), + membership term for a member golfer with tier byte & 7 above 3 (tier 4: +2, tiers 5 to 7: +5). EXACT.
- time += (now - golfer's tick of arrival at this hole) / 2 when positive, then arrival = now.
- the score reaction (type 0x13) is raised when the golfer has no pending speech bubble (byte 0x57953c is 0).

Ball lands (0x428xxx big golfer step):
- if the golfer's stroke count was 0 (the tee shot): rounds += 1; drive yards = distance * 25 / 1024 (distance in 1/1024 tile units, so one tile = 25 yards); driveSum += drive; longest = max; fairways += 1 when the landing tile's lie penalty is 0 or less (tile table byte at 0x578372 stride 0x30).
- then strokes += 1; greens += 1 when the new stroke count equals par - 2 and the ball is on a green tile (terrain id 1).
- putts += 1 for every stroke taken in putting mode, including the holing putt.
Shot planning (0x424120, argument 3 equal to -1, golfer not in sub state 0xd): shotPlans += 1.
Quit: when a golfer abandons the course the hole's 0x15a counter rises.

Reactions (0x467a00(golfer, type, location)): computes a delta (section 3), adds it to the golfer's mood (clamped -10..10) and to the hole's fun total (0x158), and when the delta is not 0 bumps the per type count and stores the location. Type 0x13 (hole score) returns early and is never stored, except that on a hole flagged too hard (bit 4) where the golfer finished 2 or more over par, or too easy (bit 8) where the golfer finished under par, with more than 9 counted rounds and a golfer whose kind is not 0x20, it becomes type 0x17 (the "this hole is too hard / too easy" complaint) and goes through the normal path.

## 3. Reaction deltas (EXACT, from the switch)

Base delta by type (decimal; hex in brackets), difficulty d = game difficulty 0..3:
+1: 1, 6, 11, 22, 28, 29, 32, 33, 34, 44, 46. -1: 2, 12, 13, 14, 15, 26. -2: 3, 4, 8, 10, 43, 20, 21, 24, 30, 35, 47. -3: 9, 36. 0: 5, 31, 45 (uphill, so never counted) and every unlisted type. Conditional: 7 gives +1 only when its location argument is 0; 18, 25, 27 give +1 when a golfer need counter exceeds a threshold (hunger above 7, thirst above 7, fatigue above 59); 59 gives +1 when an object flag is set. 23 (too hard/easy) is -1, or -2 when d != 0. 39 and 54 give +1 when d < 2. 51 to 53 (driving range comments) give +1 when d is 0. 65 gives -2 when d > 1.
Then: a base of -1 on an ordinary golfer only counts if the same type is in the golfer's last few reactions (3 slots on d 0, 5 on d 1 or 2, 10 on d 3) or the golfer has already complained; otherwise it becomes 0. Then any negative delta becomes (delta - 1) / 2 in C division (so -1 stays -1, -2 becomes -1, -3 becomes -2) unless the golfer is a pro kind (kind & 0xe0 == 0x40).
Names of the types used in the comment screen (event, delta): 1 good shot, 2 and 3 bad shot remarks, 4 miss on an easy shot, 5 hazard ahead, 6 uses a slope, 7 bridge, 8 bad design, 9 ball nearly hit someone, 10 walking through an object, 11 "check out this nice <object>" (scenic), 12 and 13 trouble and water, 14 thirsty, 15 hungry, 16 hook, 17 slice, 18 snack, 20 ugly view, 21 slow play, 22 "hey, that's <name>'s house" style feature, 23 hole too hard or easy, 24 weeds, 25 drink, 26 tired, 27 bench, 28 "look at that lovely <object>", 29 variety of the course, 30 same as last hole, 31 "never seen such <object>", 43 steep slope, 45 and 46 uphill and downhill shots, 47 partner attitude, 51 to 53 driving range.

## 4. How the report turns the record into numbers (function 0x44fb30)

Constants: d = game difficulty (word at 0x822c88), threshold T = 25 if d == 0 else 50. All divisions are C integer divisions.

Per hole h with par != 0 (open):
- group g (mask) totals: rounds N[g] = 8 + sum of bins 1..9, strokes S[g] = 8 * par + sum of bin * count. The prior of 8 par rounds means no group is ever empty. avg100(g) = S[g] * 100 / N[g].
- Avg (strokes column): sum of all bin * count over all groups and bins 1..9, times 100, divided by the count of finished rounds (0 if none). No prior here.
- Par and yards columns: the record values; subtotals after hole 9 and the total after hole 18 sum yards, par, avg and minutes. Total row of the +Len/+Acc/+Img/Fun columns is the sum divided by the number of open holes.
- Minutes: (time / rounds) / 40, where time is offset 0x1ec and rounds is offset 0x20; 0 when there are no rounds. Total shows hours and minutes (h and m).
- Fun percent: 0 when rounds is 0, otherwise fun * 100 / (shotPlans / 2 + 4 + rounds). Can be negative or above 100. The Hole Stats dialog uses the same formula but only shows it when shotPlans is not 0.
- Demand differentials, in hundredths of a stroke:
  - length: avg100(6) - avg100(7), accuracy: avg100(5) - avg100(7), imagination: avg100(3) - avg100(7). Group 7 has every skill, 6 lacks length (bit 1), 5 lacks accuracy, 3 lacks imagination.
  - with game flag 0x40 each also gets a second term: avg100(0) - avg100(skill alone) (groups 1, 2, 4).
- "Demands a skill": differential >= T. The counts for the SGA evaluation (Length Holes, Accuracy Holes, Imagination Holes) are the numbers of open holes that satisfy this.
- Hole type (the Type column): mask = bit 1 if length >= T, bit 2 if accuracy >= T, bit 4 if imagination >= T; then find the lowest of the three differentials (candidate initial 100; length only becomes the candidate when it is below 100; accuracy and imagination replace it when strictly lower); if that lowest value is below 100 its bit is removed from the mask. So a Classic hole needs all three differentials of at least 100. Names by mask: 0 Breather, 1 Freeway (length), 2 Precise (accuracy), 3 Challenge (length and accuracy), 4 Creative (imagination), 5 Heroic (length and imagination), 6 Strategic (accuracy and imagination), 7 Classic.
- Scenic hole: events[22] / 2 + events[28] + events[11] > 7 (all three are the "nice object" comments). Counted for the SGA "Scenic Holes" figure. A marker icon is drawn in the report when the hole is scenic or has the Top 100 flag.
- Variety: a hole counts toward "Holes with Variety" when its index is above 0 and its variety counter (section 5) is below 2. Hole 1 always counts.
- Avg fee: revenue * 100 / finished rounds. Revenue column: revenue * 100. Profit: (revenue - expense - expense2) * 100, red when negative.
- Cell highlights (sprite ids 233, 283, 330, 375, 422, 468, 716 in the exe): strokes cell when flags & 0xc; minutes cell when minutes >= 5 * par or minutes <= 3 * par; fun cell when fun < 10 or fun >= 50; each of the three demand cells when negative or >= 50; profit cell when negative; the 468 sprite on the type column area when variety is not 2 (GUESS what it means).
- Columns on screen (x positions): name 15, yards 151, par 172, avg 227, minutes 278, fun 324, +Len 369, +Acc 415, +Img 462, type 471, avg fee 635, revenue 709, profit 786.

## 5. Variety counter and the background analysis (function 0x42dea0)

Run periodically for holes 1 to 18 in order. For each open hole it recomputes the numbers above (without the flag 0x40 term) and updates flags. EXACT:
- Clear flags 0x04, 0x08, 0x10 and 0x700. With N finished rounds (bins 1..9, all groups) and S their strokes: if N > 9: too hard (0x04) when S > par * N + ((6 - d) * N) / 3; too easy (0x08) when S < par * N - ((3 - d) * N) / 6. Strong bits 0x100, 0x200, 0x400 when length, accuracy, imagination differential >= 50.
- Mask for the popup and the variety test: same recipe as the report but the "drop the lowest" rule only fires when the lowest differential is below 50 on d 0, below 100 on d > 0. A popup announces a hole's type the first time it appears.
- Score = length + accuracy + imagination differentials; on d < 2 floored at fun percent * 6 / (2 if d == 0 else 3).
- Top 100 test: revenue > 200 and score > 200 and flags & 0xd == 0 (not yet Top 100, not hard, not easy), then the player is asked to accept the name ("... has been rated as one of the best 100 holes in the country by Golf Enquirer magazine"). Top 18 test: revenue > 400 and score > 300 and flags & 0xe == 0 ("... Top 18 holes in the country by Great Golf Holes magazine"). Each award sets the flag, plays a sound and spawns a celebration object at the green; the flags raise the per hole fee by 2 each.
- Variety counter (offset 0x1fc) is 0 for hole 1. For hole index >= 2 it counts up to five similarities with the previous record: (a) same type mask as the previous open hole, with more than 7 rounds, and not Classic; (b) ((previous flags xor flags) & 0x60) == 0, so both or neither bend the same way; (c) event counts 45 and 46 both zero (no uphill or downhill remarks); (d) same par as the previous record; (e) the heading tee to green differs from the previous hole's heading by less than 40 units of 1.40625 degrees (about 56 degrees; 32 bit angle difference shifted right by 24, absolute value). If the sum is not 0 and d < 2, subtract 1. Heading is a compass bearing with y pointing down.
- It feeds comments 29 and 30. On d 0: for hole index above 1, when the golfer's stroke count equals (golfer slot & 1) + 1 and variety <= a random 0..2, reaction 29 (praise). On d > 0: stroke count 1, hole above 1 and variety > random(0..2) + 3 gives reaction 30 ("this hole is like the last", -2).

## 6. Hole Stats dialog (function 0x453330)

Rows: Fun Factor (percent and a word: below 0 poor, 0 to 19 fair, 20 to 39 good, 40 to 59 very good, 60 and up outstanding); Length, Accuracy, Imagination differentials; one rotating stat row picked at random each time the dialog opens: Avg. Drive (driveSum / rounds), Longest Drive, Fairways hit (fairways * 100 / rounds), Greens in Reg (greens * 100 / rounds still counted), Average Putts (putts * 100 / rounds, formatted with two decimals; the last two use rounds minus golfers currently on the hole with strokes taken, minimum 1); Yards; Par; Stroke average; the histogram "Average shots on this hole" shows 6 columns starting at max(par - 2, 1), the last column being "that many and above" (bins up to 9); Comments: the five event types with the highest count (types 0..49 only, first highest wins ties), each shown as count * 100 / rounds percent plus the comment text for its stored location.
Caution: the dialog's own Length/Accuracy/Imagination use only the histogram bins inside the six visible columns, so they can differ from the report's.

## 7. What is placeholder or open

- Hole expense (0x1f0) is reduced by every money popup that names a hole, but every caller found passes "no hole", so it stays 0; 0x1f8 is never written. Profit equals revenue in this build as far as I can tell. That sits oddly with the red profit cell in the screenshots, so a writer I did not find may exist.
- Meaning of game flags 0x40, 0x200000 of the word at 0x59e7b8 and of globals 0x543cf4, 0x543cd8; the setters of hole flags 0x20, 0x40, 0x1000, 0x2000; the exact atan2 approximation (we use the real atan2: heading differences of under a degree do not matter for the 56 degree test).
- The golfer need counter thresholds for reactions 18, 25, 27 are taken as given (the sim must supply a boolean).
- The par rule at hole opening: adjusted yards y = yards (+25 if bit 0x20 or 0x40 set and y > 250); above 300 subtract 25 times a theme index (global 0x5a8c60, 0 to 3); par is 3, 2 below 51, 4 above 249, 5 above 474, 6 above 625. Measured yards are L + (L - 250) / 4 above 250. Helpers: holeParFromYards, measuredToYards.
