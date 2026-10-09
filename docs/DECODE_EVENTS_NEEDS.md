# Decode: where and when golfer reactions are raised (needs, ambient, scenic, chat), the needs model, quitting and tantrums

Source: only the existing decompile text `spec/golf_decomp.c` (publisher exe) plus the existing docs DECODE_HOLE_STATS.md (section 3), DECODE_COMMENTS.md, DECODE_BUILDINGS.md (section 5). Facts and structure are in my own words; no game sentences are reproduced.
Tags: EXACT = read directly from the code. DERIVED = follows from the code plus a stated assumption. UNKNOWN = not established.
Line numbers are approximate lines in `spec/golf_decomp.c`. Hex addresses are the exe addresses printed in the decompile headers.

## 0. Map of the code involved

| Function | Address | Lines | Role |
|---|---|---|---|
| FUN_004289e0 | 0x4289e0 | 19528 to ~22400 | the per golfer simulation step; loops over all 152 golfer slots (0 to 0x97); one call is one game tick (DERIVED from the `tick + slot * k` phase tests below) |
| FUN_00467a00 | 0x467a00 | 71398 to ~71900 | the reaction routine `react(golfer, type, location)`: history, delta, mood, stats |
| FUN_00424120 | 0x424120 | 15078 to ~16000 | shot planning; raises shot-time comments and the "landing zone" scenic scan |
| FUN_00402a40 | 0x402a40 | 1223 to 1690 | employee AI (64 slots, 0x4c bytes each) |
| FUN_00430360 | 0x430360 | 24433 to ~25000 | wandering animals; raises type 39 |
| FUN_00427380 | 0x427380 | 17684 to ~19520 | hole completed; raises 19; per hole mood decay at its tail |
| FUN_00421bc0 | 0x421bc0 | 13857 to ~14020 | golfer arrival (zeroes the slot record) |
| FUN_0045de80 | 0x45de80 | 62083 to ~62910 | a pair is sent out to hole 1; raises 61 |
| FUN_004675d0 | 0x4675d0 | 71024 | `polarity(golfer)`: 0, 1 or 2 (section 1.3) |
| FUN_0046c940 | 0x46c940 | 80663 | male test (1 = male) |
| FUN_0040de70 | 0x40de70 | 12027 | nearest bench in a 9 by 9 cell window |
| FUN_0040ddb0 | 0x40ddb0 | 11989 | nearest building record of a given type (must be connected for types above 5) |
| FUN_0040df80 | 0x40df80 | ~12090 | building or object record index that covers a map cell (-1 none) |
| FUN_0040c170 | 0x40c170 | 10789 | ELEVATION of a cell (cached table, clamped 3 or 4 up to 15). Not a distance. |

Global names used below: tick = DAT_00834170 (game tick). Difficulty d = DAT_00822c88 (0 to 3). Theme = DAT_005a34e0. Game flag word = DAT_0059e7b8 (bit 0x200000 = sandbox style mode, DERIVED from fee skipping, bit 0x2000000 = reactions disabled, EXACT).

## 1. Golfer record fields that matter

Record base for slot s is 0x5794b8 + 0x100 * s (EXACT: the record clear loops at lines 13886 and 80746 zero 0x40 ints from DAT_005794b8). Offsets below are record+off, absolute address for slot 0 in brackets.

| Off | Addr | Size | Meaning | Tag |
|---|---|---|---|---|
| +0x00 / +0x04 | 5794b8 / bc | int | position x, y in 1/1024 tile units | EXACT |
| +0x10 | 5794c8 | dword | flag word (bits in 1.2) | EXACT |
| +0x14 | 5794cc | short | remaining length of the current walking segment, nonzero while walking | DERIVED |
| +0x18 | 5794d0 | byte | kind. 0 ordinary visitor. `kind & 0xe0`: 0x20 scripted/tutorial pair (role UNKNOWN), 0x40 pro, 0x60 CEO | EXACT for the tests, DERIVED for names |
| +0x19 | 5794d1 | byte | skill attribute bits: 1 length, 2 accuracy, 4 imagination | EXACT |
| +0x1a | 5794d2 | byte | facing octant 0 to 7 | EXACT |
| +0x1d | 5794d5 | byte | animation state (0xb idle, 0xd tantrum or visit pose, 0x11 sitting) | DERIVED |
| +0x21 | 5794d9 | byte | hole number 1 to 18, 0x13 (19) = finished or leaving, 0 = not on course, -1 = waiting at the clubhouse | EXACT |
| +0x22 | 5794da | byte | strokes taken on the current hole | EXACT |
| +0x23 + hole | 5794db | bytes | scorecard | EXACT |
| +0x36 | 5794ee | byte | running sum of recent reaction deltas (reset when the sign flips) | DERIVED |
| +0x70..+0x79 | 579528 | 10 bytes | history of the last 10 reaction types (index 0 newest) | EXACT |
| +0x7a..+0x83 | 579532 | 10 bytes | stroke count marker per history entry (+1 if type above 3) | DERIVED |
| +0x84 | 57953c | byte | speech bubble timer, set to 7 on every reaction, minus 1 every 8 ticks | EXACT |
| +0x85 | 57953d | byte | type of the last bubble | EXACT |
| +0x88..+0x9b | 579540 | 10 shorts | location history. The TOP BITS are polarity marks (1.3) | EXACT |
| +0x9e | 579556 | short | stop timer. Negative = standing still, climbs by 1 every second tick (`tick & 1`) until 0 (so a value of -32 holds 64 ticks) | EXACT |
| +0xa2 | 57955a | short | partner slot (always slot xor 1) | EXACT |
| +0xa4 | 57955c | short | mood, clamped to -10..10 after every reaction | EXACT |
| +0xa6 | 57955e | short | HUNGER counter | EXACT |
| +0xa8 | 579560 | short | THIRST counter | EXACT |
| +0xaa | 579562 | short | FATIGUE counter | EXACT |
| +0xae | 579566 | short | relationship state with the partner 0 to 3 (set at launch, 1.4) | EXACT |
| +0xb6 | 57956e | short | golfer identity (profile index, 1 to 75) | EXACT |
| +0xd4 / +0xd8 | 57958c / 90 | int | ball x, y (0 = no ball placed) | DERIVED |

### 1.1 Spawn values (EXACT, FUN_00421bc0 lines 13886 to 14000, FUN_0046c970 line 80746)
The whole 0x100 byte record is zeroed on arrival, so hunger, thirst and fatigue start at 0 and the stroke counter at 0. Arrival mood = random 3..5 (d 0: fixed 4). If the Swim Club effect level E (DAT_00543cc4) is nonzero the arrival mood is E + 4 instead. The cart flag 0x10000 is set when the Cart Garage effect (DAT_00543ccc) is nonzero. Counters are never reset by the launch routine FUN_0045de80, only by the service events in section 3.

### 1.2 Golfer flag word bits used here (EXACT unless noted)
0x1000 slow play warning received (persists). 0x2000 heading to the snack bar. 0x4000 hurried (set by a Ranger, or by the first slow play detection on d 0; cleared at the end of each hole). 0x8000 rushing (walks faster; set under some partner and group conditions, DERIVED). 0x10000 riding a cart. 0x20000 left or right throw bias for the tantrum pose. 0x40 heading to the putting green, 0x100 heading to the pro shop, 0x20 heading to the driving range (the last three are the "once heading, use the bigger radius" marks). 0x4, 0x8, 0x10 driving range, pro shop, putting green already visited this round. 0x2000000 heading to a bench. 0x20000000 LEAVING (quit the round). 0x1000000 is tested in the thirst threshold but nothing sets it (UNKNOWN, treated as clear).

### 1.3 Polarity marks and `polarity(g)` (EXACT, lines 71500, 71835 to 71845, 71024 to 71032)
`react` first stores the raw location in location slot 0, then ORs marks into that short AFTER the delta is known: final delta above 0 gives 0x4000; final delta below 0 gives 0xc000 (both bits); a negative base delta that was filtered down to 0 gives 0x8000 only; a zero delta leaves it clear. `polarity(g)` reads slot 0 (the NEWEST event only): 2 if bit 0x8000 set, else 0 if bit 0x4000 set, else 1. So polarity is 0 after a good event, 1 after a neutral one, 2 after a bad one. This corrects the older "taste roll" wording in DECODE_BUILDINGS section 3 (flower beds): it is the polarity of the latest reaction, not a per golfer trait. The mask is removed with `& 0x3fff` before a sentence is built (matches DECODE_COMMENTS section 1).

### 1.4 Relationship state (EXACT, FUN_0045de80 lines 62860 to 62880)
Set for both golfers of a pair at launch: default 0; 2 when the pair share the same gender and at most 1 of the 5 interest bits differ (similar pair); 3 when all 5 interest bits differ; 1 when genders differ, interest-difference count is 4 or more, and both profiles have bit 0x10. Used as the location argument of type 26 (`state == 3`) and by the sentence builder.

## 2. The reaction routine FUN_00467a00 (EXACT, lines 71398 to 71861)

Order of work:
1. Return at once if the golfer's stroke count is above 9, slot is 0x98 or more, or game flag 0x2000000 is set (line 71433 to 71441).
2. Type 19 is rewritten to 23 when: `kind & 0xe0 != 0x20`, the hole has more than 9 counted rounds, and (hole flag 4 and strokes minus par at least 2, or hole flag 8 and strokes minus par below 0). Location stays the mood.
3. Build the sentence into the global buffer, set the bubble timer to 7, store type and location for the bubble. Type 19 returns here (no delta, never stored). Type 35 returns here if the previous newest history entry already is type 35.
4. Shift the 10 entry histories down and insert the new type. Compute the base delta by type (table in 2.1).
5. Types 48 and above with delta 0 return here, so all chat types only make a bubble.
6. Base delta -1 filter (lines 71764 to 71793): only for `kind == 0` and `DAT_00543cf4 != 2`. The reaction keeps -1 only if the same type appears in history slots 1 to N-1 (N = 3 on d 0, 5 on d 1 and 2, 10 on d 3) or the immediately previous reaction carried the 0x8000 mark. Otherwise delta becomes 0. When `DAT_00543cf4 == 1` the delta is forced to 0 (UNKNOWN meaning of that global; the fee doubles when it is 2). For `kind != 0` or `DAT_00543cf4 == 2` the -1 passes unfiltered.
7. Negative delta halving: non pro golfers (`kind & 0xe0 != 0x40`) get `(delta - 1) / 2` truncating toward zero: -1 stays -1, -2 becomes -1, -3 becomes -2. Pros keep the full value.
8. Partner reaction (lines 71798 to 71808): when the final delta is negative, `kind & 0xe0 != 0x20`, `((strokes + slot) & 1) + 2 <= d` (so d 3 always, d 2 on half the strokes, d 0 and 1 never), type not 47, 21 or 24, and the PARTNER has hole above 0, no bubble pending and mood above 0, then the partner raises type 47 on itself (location 0x14) and its bubble timer gains 2. So 47 is spoken by the partner, not by the complainer.
9. Add the delta to mood, clamp to -10..10. Add it to the hole fun total (record+0x158 of the golfer's hole).
10. Litter (lines 71815 to 71828): if random(6) is at most d (probability (d+1)/6 for uniform 0..5) OR the golfer is leaving, and the final delta is negative, and game flag 0x4000000 is clear, and the type is not 47, and no building covers the cell, and the cell has neither bit 0x400 nor 0x800 and is not water: set weed flags 0x4800 on the golfer's cell and growth counter 1. This is the only writer of weeds in the decompile.
11. If the delta is nonzero: increment the per type count of the golfer's hole record (+0xd8) and store the location. Hole index 19 is a legal record index (scratch record), so reactions raised while the golfer is on hole 19 (finished or leaving) land there.

### 2.1 Base delta table for the types in scope (EXACT, switch lines 71512 to 71730)
Final column = after step 7 for an ordinary golfer (before the step 6 filter).

| Type | Base | Final | Notes |
|---|---|---|---|
| 7 bridge | +1 only if location 0 | +1 | |
| 11, 22, 28, 34 | +1 | +1 | |
| 14, 15, 26 | -1 | -1 (or 0 if filtered) | |
| 18 snack | +1 if hunger above 7 (read BEFORE the counter is reset), else 0 | +1 or 0 | |
| 25 drink | +1 if thirst above 7, else 0 | +1 or 0 | |
| 27 bench | +1 if fatigue above 59, else 0 | +1 or 0 | |
| 20, 21, 24, 35, 47 | -2 | -1 | |
| 36 | -3 | -2 | |
| 23 | -1 (d 0) or -2 (d above 0) | -1 | d 0 value goes through the filter |
| 39 | +1 if d below 2 | +1 or 0 | |
| 51, 52, 53 | +1 if d is 0 | +1 or 0 | |
| 48, 49, 58, 61, 62, 63 | 0 | 0 | bubble only |
| 59 | +1 if the golfer previously quit on this hole, else 0 | | |

Bubble note: each of 18, 25, 27 also calls an empty stub FUN_004a0890 when the +1 applies and the profile flag bit is set (no effect).

## 3. The needs model

### 3.1 Per golfer clock (EXACT, FUN_004289e0)
Every phase uses `tick + slot * k` so golfers do not act on the same tick:
* needs step: `(tick + 0x25 * slot) % P == 0`, P = 160 normally, 120 while `+0x14` (walking segment) is nonzero. Skipped when hole is 0x13 (line 19729).
* fatigue step: `(tick + 0x0b * slot) % 24 == 0`, only inside the movement branch and not on hole 0x13 (line 21418).
* scenic glance: `(tick + 0x21 * slot) & mask == 0` (3.5).
* tantrum: `(tick + 0x23 * slot) % 100 == 0` (6.3).
* bubble timer minus 1 every 8 ticks (line 19626); stop timer plus 1 every second tick (line 19797).

### 3.2 Thirst and hunger (EXACT, lines 19729 to 19791)
Once per needs step, in this order:
1. If hole is 2 or more, scorecard byte 0 (record+0x23) is 0, the short at +0x9a (0x579558, a bitmask of shot kinds that already got a club remark) is 0, and `random(DAT_005685f0 / 2) == 0`: raise type 63 (chat, no delta). DAT_005685f0 is hole count + 1, so about 1 in 5 for 9 holes, 1 in 9 for 18 holes. Reason for the scorecard and mask conditions: DERIVED only.
2. Look at the golfer's cell and its 8 neighbours. If any has terrain second class byte (table at 0x578350, record 0x30, byte +0x27) equal to 7, 8 or 14, set `nearHot`. Which tile ids carry those values is UNKNOWN (the +0x26 class byte listed in DECODE_COMMENTS is a different field; id 0x16/0x15 buildings carry value 16 in +0x27, EXACT from use).
3. Branch A (hunger step): if (theme == 3 and random(2) != 0) or `nearHot`. Then only if hole is 3 or more and random(2) == 0: hunger += 1. Then if hunger is above 15 and `hunger % 4 == 0` (so at 16, 20, 24, ...): if `kind & 0xe0 == 0x20` set hunger to 16 and say nothing, else raise 15 (location 0x14).
4. Branch B (thirst step): otherwise thirst += 1 unconditionally. Same rule: when thirst is above 15 and `thirst % 4 == 0`: kind 0x20 golfers are capped at 16 and silent, others raise 14.
So on a given needs step exactly one of hunger or thirst can rise. Thirst is the default. In theme 3 about half the steps are offered to hunger, and near terrain with +0x27 in {7, 8, 14} every step is. On holes 1 and 2 a hunger step does nothing at all.
Rates: thirst +1 per 160 ticks when standing and per 120 ticks when walking, when all steps go to thirst. First comment after 16 rises, i.e. at least 16 * 120 = 1920 ticks of walking. Ticks per second and ticks per hole are UNKNOWN (placeholder 40 ticks per second in PUBLISHER_EXE_NOTES), so rounds per comment cannot be fixed here.
The bubble always appears. The mood effect only counts through the filter in 2 step 6 (previous reaction negative or same type among the last 2, 4 or 9).

### 3.3 Fatigue (EXACT, lines 21418 to 21442, 20833 to 20857)
Every fatigue step while the golfer is moving and not on hole 19: `fatigue += e + hole / 6` where e = 1 if the golfer's cell has path flag 0x20, else the tile table byte +0x25 (0x578375, the same byte that sets walking speed); hole / 6 is integer (0 for holes 1 to 5, 1 for 6 to 11, 2 for 12 to 17, 3 for hole 18). Tiles table value of +0x25 per id: not recorded here (UNKNOWN).
Comment: when the new fatigue is above 159 and `new / 40 != old / 40` raise 26 (so at the first step that reaches 160, then 200, 240, ... each time a multiple of 40 is crossed). `kind & 0xe0 == 0x20` golfers are capped at 160 and silent. Location argument is `(relationship state == 3)`, 0 or 1 (bubble variant only).
Other writers: +1 for each slow play comment (3.7); capped at 40 when the rushing flag 0x8000 is set (line 21376); set to 0 when the golfer quits (lines 20371, 21322) and by the bench (below). Fatigue gauge in the info panel is `fatigue / 4` capped at 80 points, red above 160.

### 3.4 Where the counters are seen by the AI (EXACT, lines 20611 to 20722)
`q` = number of golfers ahead on the same tee who have not struck a ball (line 20418, DERIVED meaning, section 3.7). A golfer that is at the tee (strokes 0) evaluates, before anything else:
* NOT needy when `(hunger < Hlim or hunger < 2) and (thirst < Tlim or thirst < 3)` with `Hlim = (8, or 4 once heading to the snack bar) - 2q` and `Tlim = 12 - 2q` (the 8 variant needs the unset flag 0x1000000). In that case the 0x2000 flag is cleared and the bench test below runs.
* Otherwise it looks for the nearest connected Snack Bar (type 7) with FUN_0040ddb0; it accepts a bar when the distance is under 8 tiles (10 tiles when 0x2000 is already set) and sets 0x2000. No bar in range falls through to the bench test.
* Bench test: needs strokes 0, fatigue nonzero and either `fatigue / 20 >= 6 - q` (fatigue 120 with no queue; 100 with q 1; 80 with q 2) or flag 0x1000. It asks FUN_0040de70 for a bench in the 9 by 9 window around the golfer, accepted within 2 tiles (4 once flag 0x2000000 is set).
* Facilities (putting green, pro shop, driving range) are tested after the bench, only at strokes 0 (3.9).
Info panel colours use the same thresholds: red when hunger or thirst above 16, or fatigue above 160; gauge full at 32, 32 and 320 (EXACT, FUN_0045c560 lines 61085 to 61104).

### 3.5 Service events (EXACT)
* Snack Bar visit (lines 20929 to 20942): raise 18 first (delta test on the OLD hunger), then thirst = 0, hunger = 0, stop timer -48 (96 ticks), club income +5. Both counters are cleared whatever their values.
* Soda Vendor (FUN_00402a40 lines 1287 to 1299): raises 25 first (test on the OLD thirst), then thirst = 0, income +2. Skilled vendors first set thirst to 99, so the +1 is guaranteed.
* Bench (lines 20830 to 20857): raises 27 first (+1 only if fatigue was above 59), clears flag 0x1000, state 0x11, stop timer `-(fatigue * 8 / 20)` (about 0.8 * fatigue ticks), then loops picking a random neighbour cell until it finds a valid one, moves the golfer there and sets fatigue to 0.
* Nothing in the decompile ever lowers hunger, thirst or fatigue except these three paths and the quit reset.

### 3.6 Scenic glance while walking (EXACT, lines 19674 to 19712; DERIVED for the "per hole" counts)
Mask: start 0x1ff; if the newest location slot has neither 0x8000 nor 0x4000, mask 0xff; if it has 0x4000 (good event, or a bad event that counted) mask >>= 2; if the golfer has an active walking segment (record+0x14 above 0) mask >>= 1 again. So after a neutral or no reaction: 255 (127 while walking); after a filtered negative (0x8000 only): 511 (255 while walking); after a good or counted reaction: 127 (63 while walking). The glance fires when `(tick + 0x21 * slot) & mask == 0` and the two newest history entries are not type 11 (type 0x0b check, line 19685). It is exactly periodic, not random: once per (mask + 1) ticks per golfer.
Cell choice: facing octant minus 2 plus random(5), mod 8. That picks one of the 5 neighbours within 90 degrees of the facing, ONE TILE away (offset arrays DAT_004c2878 and DAT_004c2898, unit steps). Only that one cell is examined per glance.
Outcome for the cell (x tile a, y tile b, location argument `a + 50 * b`):
* Tile id 0x16 (22, building or landmark): find the record. Record type 2: raise 11. Record type 4 with sub-id below 16: raise 11. Record type 4 with sub-id 16 or more: raise 20. Other types: nothing.
* Independently, cell flag 0x1000 (flower bed) AND (polarity(g) == 2 or cell flag 0x800 weeds): raise 11 for a clean flower bed, raise 20 for a weedy one. So flower beds are only appreciated by golfers whose newest reaction was bad; weedy ones always offend.
* Home sites (tile id 21), trees, bridges and plain scenery flags are NOT looked at by this glance.
Per hole count: UNKNOWN as a fixed number. It is walking ticks divided by 64 to 256 depending on mood history; a golfer glances once per 63 to 255 ticks of walking.

### 3.7 Scenic scan at shot time (EXACT, FUN_00424120 lines 15631 to 15692)
Runs on every normal shot plan (argument 3 equal to -1, not in the putting sub mode 0xd). Elevation `h = FUN_0040c170(ball cell)`. Sample count `n = (h * 16) / (d + 2)` (integer; h is 3 or 4 up to 15, so 24 to 120 samples at d 0 and 10 to 48 at d 3; higher ground sees more). For each sample: heading = shot heading + (12 - random(25)) * 2^24 (about plus or minus 17 degrees, units of 1.40625 degrees), range in yards = shot length minus random(200) plus 100, divided by 25 for tiles (negative skipped), so cells from about 4 tiles short to 4 tiles beyond the landing point. Sample cell must be inside the map and its elevation must be at most the ball cell elevation + 1.
* Cell flag 0x100 or tile id 19 sets `lovely = cell`.
* Tile with +0x27 byte equal to 16 (buildings): look up the record. Tile 21 (home site): if there is no record, or its type is not 5, or its buyer field (+8) is 0, set `ugly = cell` (unsold lot); otherwise set `celeb = buyer` (the buyer field is celebrity index + 1). Tile 22: record type 4 with sub-id below 16 sets `lovely`, sub-id 16 or more sets `ugly`.
* A candidate behind a tree tile (class 13) in the sampled direction is skipped for the building cases only.
The last candidate in sample order of each kind wins.

## 4. Where each requested event is raised

Location column = the argument given to react. "Every" is not per tick unless stated.

| Type | Raised by | Trigger | Location arg | Mood |
|---|---|---|---|---|
| 14 thirsty | FUN_004289e0 line 19786 | section 3.2 branch B, thirst above 15 and multiple of 4, not kind 0x20 | 0x14 | -1 filtered |
| 15 hungry | line 19767 | branch A, hunger above 15 and multiple of 4, not kind 0x20 | 0x14 | -1 filtered |
| 18 snack eaten | line 20930 | arrival at a Snack Bar target (within 0.5 tile of its anchor), after the walk chosen in 3.4 | 0x14 | +1 if hunger above 7 |
| 25 drink taken | FUN_00402a40 line 1291 | Soda Vendor action step 4 (about 12 ticks after the vendor starts) | 0x14 | +1 if thirst above 7 |
| 26 tired | line 21439 | fatigue crosses a multiple of 40 above 159 (3.3) | 0 or 1 (state is 3) | -1 filtered, bubble colour neutral |
| 27 bench | line 20840 | arrival at the bench target | 0x14 | +1 if fatigue above 59 |
| 24 weeds | FUN_00424120 line 15778 | shot plan, ball cell has weeds 0x800, cell flag 0x4000 clear, `polarity(g) != 0`, no celebrity and no ugly candidate in the scan, and the first branch (bad design test) did not fire. FUN_00409a90 line 8578 also raises it when bit 1 of DAT_0056c7b4 is set (setter not located, UNKNOWN) | 0x14 | -2 final -1 |
| 21 slow play | line 21154 | 3.8 | 0x14 | final -1 |
| 35 tantrum | line 20785 | 6.3 | 0x14 | final -1 |
| 36 another golfer snapped | line 21136 | 3.9 | gender of the golfer who snapped (1 male, 0 female) | final -2 |
| 39 scared an animal | FUN_00430360 line 24563 | 3.10 | animal kind index | +1 if d below 2 |
| 47 partner attitude | react line 71804 | step 8 of section 2, raised on the partner | 0x14 | final -1 |
| 7 scenic bridge | line 21009 | 3.11 | 0 | +1 |
| 11 scenic object | lines 19703, 19709 | walking glance 3.6 | cell | +1 |
| 28 lovely object | FUN_00424120 line 15797 | shot plan, no other reaction raised during this plan, bubble timer 0, `lovely` found and the ball tile has lie penalty 0 or less (byte 0x578372) | cell | +1 |
| 20 ugly object | lines 19703, 19709 (walking glance) and 15782 (shot plan) | walking: 3.6. Shot: `ugly` found and no `celeb` (with `celeb` set the shot raises 22 instead) | cell | final -1 |
| 22 celebrity house | line 15786 | shot plan, `celeb` set (whether or not `ugly` is also set) | buyer minus 1 (celebrity table index) | +1 |
| 51, 52, 53 | lines 20988, 20976, 20970 | arrival at driving range (51), pro shop (52), putting green (53) | 0x14 | +1 only d 0 |
| 34 reply | FUN_00402a40 line 1285 | Club Pro action step 4, 3.12 | `polarity(g)` 0, 1, 2 | +1 |
| 58 reply | same line 1285 | Club Pro, needs too high (3.12) | `polarity(g)` | 0 |
| 48 / 49 chat | FUN_00424120 line 15841 | 3.13 | 0x14 | 0 |
| 61 small talk | FUN_0045de80 lines 62897 to 62901 | pair launch, 3.13 | the golfer's own mood | 0 |
| 62 greeting | FUN_00424120 line 15882 | 3.13 | 0x14 | 0 |
| 63 tournament talk | line 19735 | 3.2 step 1 | 0x14 | 0 |
| 19 hole score | FUN_00427380 line 17973 | when a hole is completed and the bubble timer is 0 | mood | none (never stored) |
| 23 hole too hard or easy | react step 2 | conversion of 19 | mood | final -1 |

### 3.7b Order of the shot plan cascade (EXACT, FUN_00424120 lines 15698 to 15791)
Evaluated once per shot plan, after the scan of 3.7. Let `cond` = (DAT_005a9ce4 == 0) or lie penalty of the planned aim tile is 0 or less, or lie penalty of the ball tile is above 0, or argument 2 is nonzero, or golfer flag 1 is set.
1. If NOT `cond` (DAT_005a9ce4 stayed 1, the aim tile is penalised, the ball tile is clean, full shot mode): raise 8 (bad design). DAT_005a9ce4 is set to 1 at the start of the aim solver FUN_00422fb0 (line 14653) and cleared when a candidate line passes a hazard score test (line 14906), so 8 means the solver found no acceptable line (DERIVED).
2. Else if no `ugly`: if `celeb` raise 22; else if weeds test (24 row above) raise 24; else the ordinary shot comments (29, 4, 30, 6, 32, 33, in DECODE_SOCIAL section 5).
3. Else (`ugly` set): raise 20 if no `celeb`, else 22.
4. After the cascade, only if the newest history stroke marker is unchanged (nothing was raised) and the bubble timer is 0: raise 28 if `lovely`; shot-kind remarks 55 to 57; 59, 48/49, 54, 62 (3.13); and the uphill and downhill remarks 45 and 46 compare the target elevation with the ball elevation.

### 3.8 Slow play (21), EXACT, lines 21093 to 21326 (queue count DERIVED)
Evaluated every tick in the `not stopped` part of the step, for golfers without flags 0x20000000, 0x1000000 or 0x2000 (not leaving, not heading to a snack bar).
* Scan all 152 slots starting from DAT_005a5a24. For every other golfer on the course that started earlier (launch ordinal at +0xbe not later than mine), is on my hole or earlier, and is not leaving: if within 0.5 tile and (in my facing within one octant, or is my partner), set `blocked` and stop myself for 4 plus random(4) stops-units. If it is on my hole and has taken no strokes, add 1 to the queue count `q2`. For a leaving golfer the radius is 2 tiles instead.
* Slow play condition: `blocked` and my strokes is 0 and `q2 > 3` (four or more golfers ahead on this tee, including my partner if it qualifies) and flags 0x6000 are both clear.
* First time: only set flag 0x1000. Every later time (flag already set, 0x4000 clear): raise 21 and fatigue += 1.
* Always: stop timer = -0x20 - random(64) (64 to 190 ticks). On d 0: stop timer = -100 - random(28) and flag 0x4000 is set, which blocks all later slow play events for that golfer for the rest of the hole, so d 0 golfers get the warning flag and never the event.
* If mood is below 0 and `kind & 0xe0 != 0x20` at this moment, BOTH partners leave immediately: partner's ball and position cleared, partner dismissed, own hole becomes 19, flag 0x20000000, fatigue 0, hole quit counter +1, membership and blame bookkeeping as in 6.2.

### 3.9 Another golfer snapped (36), EXACT line 21133 to 21143
In the same scan: if the other golfer has the leaving flag, is within 2 tiles, and my last history type is not 36, raise 36 with the other's gender, stop timer -4 minus random(4), turn to face them.

### 3.10 Animals (39), DERIVED from FUN_00430360 lines 24490 to 24590
Per animal, once per animation cycle (when the frame index wraps), if the animals are enabled (DAT_005a5a00 bit 0x20), the animal is not already scared (animal flag 2 clear) and it has no fixed target: loop over all golfers; a golfer qualifies if it is on the course (hole not -1) and within 2 tiles (0x800) of the animal, OR its ball is in flight and within 2 tiles. A qualifying golfer with bubble timer 0 and last type not 39 raises 39 with the animal kind index; the animal is then marked scared (flag |= 2) and flees. The animal scare flag clears with 1/3 chance per cycle. Animal kinds 0 to 8 (any value above 8 is reset to 1).

### 3.11 Scenic bridge (7), EXACT lines 21006 to 21014
Evaluated when a new walking segment begins (segment counter below 1): the golfer's cell has tile 17 (water), flags 0x20 and 0x100 (a scenic bridge), and the last history type is not 7: raise 7 with location 0. It also records a bridge direction bit in a 5 slot memory at record+0xbc (use UNKNOWN).

### 3.12 Club Pro and other employees (EXACT, FUN_00402a40)
* Target selection (lines 1516 to 1566): golfers with hole above 0, animation state above 6, in a valid cell, at the smallest distance to the employee post, with a distance cap. Per kind:
  * Pro (-2): golfer's newest history type is not 34 or 58.
  * Ranger (-3): golfer without flag 0x4000 and with stroke count at most 1 (older docs call this a group counter); a skilled Ranger also takes a leaving golfer with distance divided by 8.
  * Soda Vendor (-5): golfer thirst above 4 for a basic vendor, above 0 for a skilled one. There is no separate cart girl: the Soda Vendor is the beverage cart.
  * Groundskeeper (-4): chooses weed cells (flag 0x800) in a 33 by 33 window, see below.
* Action is a 7 step countdown, one step per 4 ticks.
* Pro, step 4: if skilled, or thirst and hunger are 16 or lower and fatigue 160 or lower, raise 34 (+1), else raise 58 (no delta). Location is `polarity(g)`. The golfer's stop timer drops by 16. The pro does not change any counter.
* Ranger: sets golfer flag 0x4000. Effect of that flag on the golfer: +1 to the walking step value (line 21335), immune to the slow play event (3.8), clears the mood-adjacent flag logic at line 20512. A skilled Ranger on a leaving golfer instead sets its mood to -11 and its stop timer to 0, which makes the tantrum routine delete it at its next tantrum tick (6.3, DERIVED).
* Soda Vendor: see 3.5 (thirst reset, 25 raised first, +2 income, stop timer -32 on the golfer, facing turned to the vendor).
* Groundskeeper: for a weed cell without flag 0x4000 sets 0x4000 and a work timer from DAT_0053fa28 minus 3, otherwise decrements the cell timer and at below 2 clears flags 0x800 and 0x4000. Cells with 0x4000 never fire event 24. Weeds created by litter start with 0x4800 and timer 1, so a single visit clears them. A skilled one with no weed in range also removes entries of the object table at 0x5736b0 within 5 tiles (UNKNOWN meaning).
* None of the employees changes hunger or fatigue.

### 3.13 Chat events (EXACT triggers, text in DECODE_COMMENTS)
* 48 and 49 (FUN_00424120 lines 15832 to 15846): at a shot plan with nothing raised, bubble timer 0, the golfer has 3 strokes, `(hole + slot)` is even, both golfers are kind 0, same gender, partner has no bubble: slot raises 48, partner raises 49, partner bubble timer +1.
* 62 (lines 15877 to 15883): shot plan with nothing raised, kind 0, bubble timer 0, the ball lies on tile id 1 (green), and `(strokes + slot + hole) & 3 == 0`.
* 59 (line 15825): strokes 0 at the tee, kind 0, golfer identity has a stored score for this hole.
* 61 (FUN_0045de80 line 62897): at launch if `tick & 0x40` is set both golfers raise 61 with their own mood; the partner's bubble timer gets +10 and a field at +0x1ac... on the pair gets +8 (purpose UNKNOWN).
* 63: 3.2 step 1.
* 34 and 58: Club Pro.
* 19: 2 step 2 and section 4 row.

### 3.14 Facility visits behind 51 to 53 (EXACT, lines 20655 to 20715, 20943 to 20995)
Only at strokes 0, only for kind 0, after the bench test, in this order. Each needs the building type's effect level E nonzero (built and connected) and a once per round flag:
* Putting green (type 6): attribute bit 4, flag 0x10 unset. Target nearest type 6 building; accepted within 5 tiles (7 once flag 0x40 set). Arrival: stop -16 minus random(9), state 13, flag 0x10, income 4 (8 at tier 2), raise 53.
* Pro shop (type 8): attribute bit 2, flag 8 unset. Accepted within 5 tiles (7 with flag 0x100). Arrival: same stop, flag 8, income 6 (10), raise 52.
* Driving range (type 10): attribute bit 1, flag 4 unset. Accepted within 6 tiles (9 with flag 0x20). Arrival: same stop, flag 4, income 8 (12), raise 51.
Mood is +1 only on d 0 for these.

## 5. Employee effects on the counters, summary

| Employee | Raises | Hunger | Thirst | Fatigue | Other |
|---|---|---|---|---|---|
| Club Pro | 34 or 58 | none | none | none | +1 mood (34) |
| Ranger | none | none | none | none | sets 0x4000 (hurry, slow play immunity) |
| Groundskeeper | none | none | none | none | removes weeds, so fewer 20 and 24 |
| Soda Vendor | 25 | none | set to 0 (skilled: 99 first for the +1) | none | +2 income |
| Snack Bar (building) | 18 | set to 0 | set to 0 | none | +5 income, 96 tick stop |
| Bench (tile) | 27 | none | none | set to 0 | rest 0.8 * fatigue ticks |

## 6. Mood, quitting and tantrums

### 6.1 Mood drain between events (EXACT, FUN_00427380 lines 19505 to 19520)
At the end of each completed hole: for a non pro golfer `mood -= ((rounds + 6 + hole) * (mood - 1 + d) * (d + 1)) / ((H * 5 + 15) * 8)` with C truncation, where `rounds` is the identity's played-round counter (profile record +0x1a incremented at every pair launch, line 62898 area) and H is the Resort Hotel effect level (DAT_00543cd4). For pros: `mood -= ((hole + 6) * mood * d) / 160`. A negative numerator (mood - 1 + d below 0) raises mood slightly instead. The hole fee uses the mood before this drain.

### 6.2 Who quits a round (EXACT, lines 19861 to 20376, 21166 to 21324)
Quit test (idle branch, evaluated between shots, in practice right after a hole is completed or while waiting at a tee):
* skip if `(sandbox flag 0x200000 or kind & 0xe0 == 0x20)`: such golfers have a negative mood reset to 0 instead.
* quit when mood is below 0 (that is, -1 or lower), the bubble timer is 0 (so at least 56 ticks after the last reaction), and the golfer is not already leaving.
* Effects: hole record quit counter +1 (record+0x15a, taken from the hole they were on); a member (identity tier `& 7` above 1) resigns: tier set to 0 (membership text printed); the identity's per hole blame bit 4 is set; identity byte +0x29 is set to 0xff, which excludes that identity from arrivals (the arrival picker skips it, line 13977). This is done for EVERY quitter, not only members (older docs say only at tier 2 or more). Then stop timer -99, animation 0xd, hole set to 19, flag 0x20000000, ball cleared, fatigue 0, trend byte 0xfd.
* The printed reason depends on the type of the last bubble: 4, 8, 9, 12, 13, 14, 15, 21, 23, 26 and 30 each have their own reason, everything else a generic disgust reason (EXACT map: 4 wants a tougher course; 8, 23, 30 course needs work; 9 punched another golfer; 12 wrapped a club around a tree; 13 threw clubs in the lake; 14 too thirsty; 15 too hungry; 21 insulted another golfer; 26 too tired).
Other quit path: slow play with mood below 0, 3.8, taking the partner with it.
No other threshold exists: there is no fixed thirst, hunger or fatigue value that forces a quit. Needs only matter through the mood they remove (each comment is -1 at most and filtered).

### 6.3 Tantrums (35), EXACT lines 20783 to 20800
Applies to golfers with flag 0x20000000 (quitters). Every 100 ticks per golfer (`(tick + 0x23 * slot) % 100 == 0`): raise 35 (location 0x14), animation 0xd, flag 0x20000 randomly set or cleared (50 percent), stop timer -99, mood -= 1 more; if mood is now below -10 the golfer's hole is set to 0, which removes them at once. Otherwise the quitter walks to the course exit tile (DAT_00578150, DAT_00578154) and is removed there. Quitters on hole 19 file reactions into record 19, so type 35 never reaches the hole 1 to 18 statistics (DERIVED from the record index). Golfers that finish normally (hole 19 without the leaving flag) do not tantrum.

### 6.4 Walking speed (EXACT, lines 21328 to 21412, needed for the glance and fatigue rates)
Step value s = clamp(6 minus tile byte +0x25, 3, 5), forced to 3 when fatigue is 160 or more; plus 1 for the player controlled golfer or its partner, a hurried golfer (0x4000) or sandbox mode; 6 on path cells; plus 1 when `(tick >> 4) & 3 == slot & 3`; replaced by `cartE * 4 + 8` when the rushing flag 0x8000 is set. Movement per tick = `((DAT_0059b04c != 0) + 1) * 64 * clamp(s, 0, distance * 7 / 128) / D` units, where D = 7 on d 0 and 5 otherwise for straight facings, plus 3 for diagonal facings. One tile is 1024 units, so d 0 at s = 4 is 36.6 units per tick (about 28 ticks per tile) and d 1 to 3 at s = 4 is 51 units per tick (about 20 ticks per tile). The meaning of DAT_0059b04c is UNKNOWN (doubles speed).

## 7. Implementation sketch

```
onTick(g):
  if g.hole in (0, -1): return
  // needs
  P = (g.walkSeg != 0) ? 120 : 160
  if g.hole != 19 and (tick + 0x25*g.slot) % P == 0:
      if g.hole >= 2 and g.scorecard0 == 0 and g.clubRemarkMask == 0 and rnd(holeCount+1 >> 1) == 0: react(g, 63, 20)
      hunger_branch = nearHot(g) or (theme == 3 and rnd(2) != 0)
      if hunger_branch:
          if g.hole >= 3 and rnd(2) == 0:
              g.hunger += 1
              if g.hunger > 15 and g.hunger % 4 == 0:
                  if kind20: g.hunger = 16 else react(g, 15, 20)
      else:
          g.thirst += 1
          if g.thirst > 15 and g.thirst % 4 == 0:
              if kind20: g.thirst = 16 else react(g, 14, 20)
  // scenic glance
  mask = (loc0 & 0xC000) == 0 ? 0xFF : 0x1FF
  if loc0 & 0x4000: mask >>= 2
  if g.walkSeg > 0: mask >>= 1
  if ((tick + 0x21*g.slot) & mask) == 0 and last2 history not 11:
      cell = neighbour(g.pos, (g.facing - 2 + rnd(5)) & 7)
      ... section 3.6
  // fatigue (movement only)
  if moving and g.hole != 19 and (tick + 11*g.slot) % 24 == 0:
      old = g.fatigue
      g.fatigue += (cell.path ? 1 : tile.effort25) + g.hole / 6
      if g.fatigue > 159 and g.fatigue/40 != old/40:
          if kind20: g.fatigue = 160 else react(g, 26, g.relState == 3)
```

Reaction: implement section 2 steps in order (history shift, base delta, `-1` filter by N = 3, 5, 5, 10, halve negatives, partner 47, mood clamp -10..10, litter weeds with probability (d + 1) / 6, store count only when delta is not 0 and write polarity marks on location slot 0).

## 8. Unresolved items

1. UNKNOWN: ticks per second and so ticks per hole; all rates above are in ticks.
2. UNKNOWN: which tile ids carry the +0x27 values 7, 8 and 14 that count as `nearHot`, and the +0x25 walking effort per tile id.
3. UNKNOWN: setter of DAT_0056c7b4 bit 1 (second producer of type 24 in FUN_00409a90) and the 0x1000000 golfer flag.
4. UNKNOWN: why event 63 needs scorecard byte 0 equal to 0 and the shot remark mask (+0x9a) equal to 0; the mask is written at line 15861 per shot kind.
5. UNKNOWN: weed growth counter. Litter sets flags 0x4800 and counter 1; only the Groundskeeper handler and a draw routine read the counter and nothing increments it, so weeds created by litter keep flag 0x4000 (which blocks event 24 and is cleared only by the Groundskeeper). If this is faithful, golfer litter is only noticed through the walking glance on flower beds and the groundskeeper. DERIVED, worth a test in play.
6. UNKNOWN: meaning of globals DAT_00543cf4 (1 turns every -1 reaction off, 2 doubles the fee) and DAT_005a9ce4 (set by the aim solver).
7. UNKNOWN: precise role of `kind & 0xe0 == 0x20` golfers (they never complain about needs and never quit), assumed to be the scripted tutorial pair.
8. DERIVED, unverified: the quit test is only evaluated while the ball is cleared (between holes) or the golfer is idle. A golfer with negative mood during a hole therefore finishes the hole first.
9. UNKNOWN: use of the 5 slot bridge memory (record+0xbc), the +8 and +10 bubble and timer bumps in pair launch, and the animal scare scan gating byte DAT_00572cac.
10. UNKNOWN: the hunger and thirst gating bit `(0x579558 short == 0)` semantic beyond "no club remark yet" and the exact value of `DAT_005685f0` (taken as hole count + 1 from FUN_004315e0 and the hole creation code).

## 9. Corrections to earlier docs

* DECODE_BUILDINGS 3 (flower bed glance): "taste roll equals 2" is `polarity(g) == 2`, the golfer's newest reaction was bad.
* DECODE_BUILDINGS 3 (glance rate): the mask is chosen from the newest reaction polarity marks (3.6), not from an unnamed attribute word.
* DECODE_BUILDINGS 5 (Ranger): the eligibility test is stroke count at most 1 and no hurried flag, not a group counter.
* DECODE_COMMENTS 3.3: type 26 location is `relationship state == 3`, types 34 and 58 location is `polarity(g)` (0 good, 1 neutral, 2 bad); type 22 location is the buyer field minus 1.
* DECODE_SOCIAL 1: every quitter, not only members, gets the permanent identity byte at +0x29 set to 0xff.
* DECODE_HOLE_STATS 3 (-1 filter): the "has already complained" exception is "the previous reaction carried the 0x8000 mark", and the look-back is 2, 4 or 9 previous reactions (N minus 1 slots).

## 10. Port status
Implemented: the needs clock (thirst, hunger and fatigue counters, comment thresholds, scripted pair exemption), the walking glance (periodic mask by the newest reaction, one adjacent tile within 90 degrees; landmarks raise 11, flower beds only after a bad reaction), the shot time scenic scan (sample count by elevation and difficulty), service events (Soda Vendor reads the counter before the reset, skilled vendors set 99 first; the Snack Bar placeholder visit raises 18 and clears hunger and thirst), Club Pro replies 34 and 58 by the need thresholds, the hole score call (23 when flagged), and quitting (mood below 0 with a silent speaker, checked at hole end and at a tee).
Tee visits (3.4, 3.5, 3.14) follow the exe's rules: the need test with limits 8 and 12, Snack Bar within 8 tiles (18), bench within 2 tiles (27), then Putting Green, Pro Shop and Driving Range once a round for golfers with the matching skill bit, with the exe's income and the 51 to 53 reactions. The port moves the golfer there instantly and holds it for the visit time, the queue term is 0, and the old 'holes out near a building' income rule is gone. Test hook: `--needs hunger,thirst,fatigue`.
Placeholders: tick rate (13 a second), walking effort per tile, hunger heat tiles (none), the partner (slot xor 1 only while on the same hole), no litter weeds, no slow play (21), animals (39), tantrums (35), snapped golfers (36), bridge (7) or chat events.

## 11. Port status, slow play, quitting and tantrums

* Quit (6.2): a golfer with mood below 0 and a silent speaker starts LEAVING (flag, quit counter, reason toast by the last reaction type, membership resignation when the round ends). Leaving golfers file reactions into record 19: the port skips hole statistics for them.
* Tantrum (6.3): every 100 ticks `(tick + 0x23 * slot) % 100 == 0` raises 35, mood -1, a 99 tick pause and the throwing pose; below -10 the golfer vanishes, otherwise it walks to the clubhouse (PLACEHOLDER for the exit tile) and is removed.
* Snapped (3.9): a golfer within 2 tiles of a leaving one raises 36 (gender is the look parity, PLACEHOLDER) and stops 4 to 7 ticks.
* Slow play (3.8): the scan, the queue count (earlier golfers on my hole with no strokes), the warning flag, the hurry flag on d 0, the 64 to 190 tick stop and the mood-below-0 walk-off of both partners are in. "Blocked" is PLACEHOLDER (earlier golfer within half a tile, in front or the partner) because the port has no walking collision, so four golfers queued on one tee is rare. Test hook: `--mood N`.
