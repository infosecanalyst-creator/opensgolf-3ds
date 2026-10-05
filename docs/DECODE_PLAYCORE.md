# DECODE_PLAYCORE: golf-play core rules decoded from the Ghidra text

Source: spec/golf_decomp.c (text only). Cite by function name; older docs' line numbers are stale.
Tags: EXACT (read directly), DERIVED (computed or inferred, method given), UNKNOWN (not in the text; address given).
Facts are in my own words. No code is reproduced.

## 0. Conventions (EXACT unless noted)

- Golfer record base 0x5794b8, stride 0x100 bytes (int index stride 0x40). Two golfers per group, partner slot = slot xor 1.
- World unit: 1024 units per tile. FUN_0040c4b0 converts units to yards as dist*25>>10, so 25 yards per tile. Map grid is 50 x 50 (0x32) tiles.
- Corner height levels 3..15; 16 ball-height units per level.
- Angles: 32-bit, 2^32 = 360 degrees. Heading 0 = north (y decreasing), clockwise positive. Octant = ((a>>28)+1>>1)&7. Octant step tables at 0x4c2878 (dx) and 0x4c2898 (dy).
- Helpers: FUN_00467130 clamp(v,lo,hi); FUN_00467270 sine-like; FUN_004672b0 cosine-like; FUN_004672d0 atan2(dx,-dy); FUN_0045c1e0(n) 16-bit random below n.
- Skill bytes (golfer +0xf0.. i.e. 0x5795a8..): Power a8, Long Driver a9, Accurate Driver aa, Accurate Irons ab, Accurate Putter ac, Draw ad, Fade ae, High Backspin af, Recovery b0, Luck b1. Order of the ten skills is EXACT.
- Ability word (short at +0x16): bit 1 power, 2 long driver, 0x10 putter, 0x20 fade lane, 0x40 draw lane, 0x200 luck. Skill mask (+0x19): bit 1 length, bit 2 accuracy, bit 4 imagination.

## 1. Terrain and lie table

Table: runtime 0x578350, stride 0x30 bytes, 23 records (ids 0..22). Static source 0x4c1a40 (0x114 dwords copied at load; only names patched per theme). The exe ids 0..22 are DERIVED to equal the Terrain.dll tile ids (match by name order in DECODE_COMMENTS).

Field offsets inside a record (EXACT):
- +0x20 bounce factor k; +0x21 roll friction (for the green, the putt solver's friction exponent, byte 0x5783a1); +0x22 penalty (signed); +0x23 build cost; +0x24 site cost; +0x25 walk effort; +0x26 class (13 trees, 17 water, 18 OOB or building, 7 and 8 sand); +0x27 subclass; +0x2c flags (0x08 flat/height 0 e.g. water, 0x02 and 0x04 ramp).
- UNKNOWN: the numeric contents of +0x20, +0x21, +0x25, +0x27 for every id. They live in the static block at 0x4c1a40, which is not present in the text. Needs a table dump of 0x4c1a40 (23 x 0x30 bytes).

Penalty byte +0x22 known values:
- EXACT (older docs): tee 0, green -1, fairway 0, firm fairway 0.
- DERIVED (from the lot-value formula in DECODE_HOMES): deep rough 2, mound 2, sand trap 3, waste bunker 2, pot bunker 5, ravine 5, brush 3, rocks 3, wetlands 3, marsh 3.
- EXACT: water (17) and OOB (20) are 8 by default. The planner (FUN_00424120) writes 32 (0x20) into both while golfer flag 1 (penalty replay) is set and restores 8 on exit (bytes 0x5786a2 and 0x578732).
- UNKNOWN: rough (guess 1), trees 13..16, buildings 21/22.

Lie effects in the shot (all EXACT, from the launch solver FUN_004223c0/FUN_004223f0/FUN_00422430 and max range FUN_00422530):
- Max range in yards, capped at 330. Base: distance-to-hole d under 1 yard gives 25 (plus 15 with skill bit 1); otherwise byte 0x579572 times 50 divided by 3. Then add 150, plus 50 with skill bit 1.
- For non-putter clubs only: ability bit 1 adds 4*power - 20; ability bit 2 on the tee adds (longdriver - 5)*6.
- Momentum above 0 adds clamp(momentum,0,3)*range/24. Skill bit 1 adds DAT_00543cc8*15. Lie penalty above 0 subtracts clamp(pen,0,3)*range/8. Off the tee, subtract range/5.
- The final speed of the strike is switched by terrain id of the lie (a per-id multiplier branch in the launch solver). The per-id factors were read only partially: UNKNOWN as a full table.
- Playability: a ball in class 17 or 18 (water, OOB) is never played from; it is dropped (below). Trees 13..16 are obstacles in flight (section 3), not lies.

Drop rules (EXACT, FUN_004289e0 landing branch):
- Water: choose the farthest point on the flown line that is neither water nor ravine and not farther from the hole than the ball's origin; the candidate score is range minus 4*pen. Set golfer flag 1 and add one penalty stroke if strokes < 8.
- OOB or off map: reaction 2, replay from the previous spot, flag 1, +1 penalty stroke if strokes < 8.
- Bunker-specific rules (sand ids 7/8 class): only through the speed switch and friction below. No separate bunker-exit rule found: UNKNOWN.

## 2. Aim search and planner

### 2.1 Planner FUN_00424120 (EXACT structure; details in DECODE_EVENTS_SHOTS tiers)
- Sets pen of water and OOB to 32 when flag 1, tries candidate aim sets, restores to 8. Compares candidate scores by hazard score hz (FUN_00421fa0 line evaluator) plus remaining distance. Club selection follows the max range (FUN_00422530): the shortest club whose reach covers the chosen aim distance, putter when on the green.
- Correction to older doc: the low-punch shot requires only EITHER the tile 0x80 ahead OR the tile 0x400 ahead to be a tree id; it does not need both.
- Ten skills: Accurate Driver/Irons/Putter shrink the spread, Draw/Fade enable the curve lanes (ability 0x40/0x20), High Backspin enables flag 0x80 (section 3), Recovery matters off-fairway, Luck (0x200) feeds a reroll. The exact per-skill constants inside the spread and curve code were not fully extracted: UNKNOWN for the numeric slope per skill point.
- Positive Attitude and Mental Toughness: not located anywhere in the planner, range, launch or flight code read. UNKNOWN. The only momentum-like input found is the momentum term in max range.

### 2.2 Aim search FUN_00422fb0 (EXACT structure)
- Walks candidate aim points using the preview flight FUN_004226a0 and the line evaluator FUN_00421fa0, scoring each landing. Some call arguments (to FUN_004226a0, FUN_00421fa0, FUN_0040c4b0) were dropped by Ghidra, so candidate spacing and angular steps are UNKNOWN.

### 2.3 Shot shapes
- Straight, fade, draw: curve value added to heading per tick (section 3). Fade and draw pick the curve sign; ability bits 0x20 and 0x40 gate them. EXACT gating, UNKNOWN magnitudes.
- High backspin: ball flag 0x80; on bounce 1 the speed halves and flag 0x100 is set; on the next bounce with 0x100 the heading reverses (EXACT).
- Low punch: flat trajectory used when a tree stands 0x80 or 0x400 ahead (EXACT trigger, above).

### 2.4 Putting
- Green rolling uses the green tile's friction byte directly with no slope term; heading changes by curve/2 each tick; on every 8th tick there is a 1-in-8 chance of a sign flip of the break (EXACT).
- Holing radius, putt speed scale, break magnitude per slope: UNKNOWN (constants not isolated).

## 3. Flight model (FUN_004226a0 preview, FUN_004289e0 real)

Per tick (EXACT):
- x += sin(heading)*speed/16; y -= cos(heading)*speed/16; height += vertical/32; vertical decreases by 64 per tick while airborne or moving (gravity).
- Airborne drag: speed -= speed>>5. Heading += curve/2 (preview halves the curve first then adds the full value).
- On ground: friction f = clamp(tile byte +0x21 minus slope along heading, 0, 99); minimum 2 on tile edges where the neighbour terrain differs; capped at 4 after 128 ticks. If f < 5: speed -= (speed>>f)/2. If f >= 5: speed = speed - (speed>>6) + 32.
- Cross slope: heading -= slope * 2^25.
- Water and ravine centre: speed halves each tick.
- Bounce: rebound = clamp(-64 + |vertical|*k/12, 0, 9999); a rebound under 0x80 becomes 0 (ball starts rolling). k is table byte +0x20 (value UNKNOWN per surface).
- Stop: speed < 0x40 with height 0 and vertical 0.
- Trees and buildings: height bands per theme (FUN_004070b0, EXACT as in older doc). Only ids 13..16 and 21/22 can be hit. A hit occurs if the distance to the obstacle is less than rand(0x180). Effect: heading += (0x40 + rand(0x80)) * 2^24; speed -= rand(speed); ball flag 2; reaction type 12.
- Launch per club: loft and speed from FUN_004223c0/4223f0/4224 30 as in include/sg/flight.h (DERIVED match, same numbers). Per-club table values: see launchFor in flight.h; not re-extracted here.
- Units: speed unit per tick is 1/16 world unit per speed point; 1024 units per tile; 25 yards per tile.
- Splash: water landing sets reaction 2 style penalty path and the drop rule in section 1; the splash animation id is UNKNOWN.

Swing timing (EXACT): swing counter n; animation id = clamp(n-3,0,6); counter caps at 0x78. Impact at n = 8 (animation id 5): sets flag 0x40000 and plays the strike sound. A pure strike raises reaction 40 and sets speech timer 3.

## 4. Timing

- Global tick counter 0x834170, starts at 0x2c00. 1024 ticks per month, 8192 per year (8 months). 40 ticks per game minute (EXACT).
- Real-time tick rate and game speed settings: UNKNOWN. The caller of FUN_004289e0 is an indirect dispatch (no direct callers in the call graph) and no speed strings exist. Variables that look speed-related: difficulty d (0..3) and DAT_0059b04c (non-zero doubles walk movement). include/sg/reactions.h kSimTickHz = 13 is a placeholder, not decoded.
- Walking step s = clamp(6 - walk effort byte(+0x25 of the tile), 3, 5 normally or 3 when fatigue < 160). Movement per tick = ((DAT_0059b04c != 0)+1) * 64 * s / divisor; divisor 7 (straight) or 10 (diagonal) when d == 0, 5 or 8 when d > 0 (EXACT).
- Fatigue grows every 24 ticks by (1 if on a path cell else walk effort byte) + hole/6 (EXACT).
- Per-golfer clock phase uses tick + slot*k (EXACT shape; k values in DECODE_EVENTS_NEEDS).
- Tee group spacing, slow-play thresholds, ticks per hole: UNKNOWN here. FUN_00427380 (hole finish) and the tee ordering at the tail of FUN_004289e0 were not fully decoded for numeric thresholds.
- Tile size: 25 yards (EXACT).

## 5. Walking and collision (FUN_0042e7e0, FUN_0042e7a0, FUN_0042ee80)

FUN_0042e7e0 is a flood-fill path search on the 50 x 50 grid, returning the first octant to step:
- Cost array at 0x58d370, work queue of up to 0x400 cells, 8 neighbours using the octant tables. Base step cost per cell comes from a signed byte array at 0x53bbac plus 1 for diagonal directions (odd octant index) (EXACT).
- The partner golfer's cell adds 2. Flag 0x20 in the cell flag array 0x53caf0 marks path (cart path) cells: a straight move between two path cells uses a path-direction check (FUN_0042e7a0) and is cheap or blocked accordingly.
- Search budget: 160, or 250 when mode flag 0x100 is set. Costs saturate below 256.
- Mode 0x100 (or golfer state 0x13): cells with terrain 1, 2, 0 or class 7 cost +1; all other terrain costs are halved (a "seek rough/sand" mode, DERIVED purpose).
- Water (17) and OOB (20) cells without path flag and with low accumulated cost get +16: strongly avoided, not strictly forbidden. Destinations in water are rejected elsewhere.
- Cells rejected when FUN_0040bf60 reports blocked (buildings, objects) (EXACT call; blocker taxonomy UNKNOWN beyond that).
- Final direction: among the 8 neighbours of the goal with a non-zero cost and not blocked, choose the lowest cost (random start rotation for tie breaks, preferring the path-aligned direction). If none, fall back to a direct heading.
- FUN_0042ee80 gives per-tile terrain preference scores: water-class path 12, id 0x14 gives 0, id 0x15 gives -16, non-positive pen tiles -8, id 17 gives 32, id 18 gives 16 in one theme, class 13 gives pen*5/2, others pen * walk byte / 2 (EXACT numbers, purpose DERIVED as the avoid-hazard weighting).
- Golfers hold 4 to 8 ticks when near others (EXACT range).

## 6. Reaction producers (EXACT where stated; see DECODE_EVENTS_SHOTS for tiers)

- 12: tree or building hit in flight (section 3), no argument beyond golfer slot.
- 2: OOB or off-map landing, replay.
- 40: pure strike at impact (swing counter 8), with speech timer 3.
- 10 and 43: raised in the landing and result branches of FUN_004289e0 (conditions as in DECODE_EVENTS_SHOTS).
- 42, 9: landing reactions in DECODE_EVENTS_SHOTS.
- UNKNOWN trigger and arguments: types 6, 8, 32, 33, 55, 56, 57, 60, 16, 17. Not found raised by the golf-play functions read (FUN_00421bc0, FUN_00421fa0, FUN_004223c0..FUN_00424120, FUN_004289e0). Likely raised from need/mood code (see DECODE_EVENTS_NEEDS) or from the unread tail of FUN_004289e0.

## 7. UNKNOWN list with addresses

- Terrain numeric table: 0x4c1a40 (static), 0x578350 (runtime), stride 0x30.
- Real tick rate and speed settings: main loop that dispatches FUN_004289e0 (not in text).
- Positive Attitude and Mental Toughness effects.
- Aim search steps (dropped Ghidra arguments).
- Per-skill spread and curve constants, putt break magnitude and holing radius.
- Tee spacing and slow-play thresholds: FUN_00427380 and the tee ordering code.
