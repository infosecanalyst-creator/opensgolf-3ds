# Decode: where and when the SHOT related golfer reactions are raised, and what they do to mood and hole fun

Source: only `spec/golf_decomp.c` (Ghidra text of the publisher golf.exe) and the existing docs (`DECODE_HOLE_STATS.md` section 3, `DECODE_COMMENTS.md`). No binary was opened. Everything is in my own words; game strings are identified by at most a few words.

Tags: EXACT = read directly from decompiled code. DERIVED = follows from code plus a stated assumption. UNKNOWN = not established (usually because a table or helper is not visible in the text).

Line numbers are lines of `spec/golf_decomp.c` (approximate to a few lines). "d" is the game difficulty word at 0x822c88 (0 to 3). "rand(n)" is `FUN_0045c1e0(n)`, a 15 bit LCG scaled to an integer 0 to n-1 (DERIVED from the other docs' use of `rand(100)`).

## 0. Routines and data used

| Address | Decompile lines | Role |
|---|---|---|
| FUN_00467a00 | 71398 to 74454 | the reaction routine `react(golfer, type, location)`. Lines 71398 to 71860 are the mood, count and history logic; 71862 to 74451 build the one-time advisor message |
| FUN_00424120 | 15078 to ~16200 | shot planner `plan(golfer, mode, tx, ty, shape)`. Reaction tree at 15694 to 15884 |
| FUN_00422fb0 | 14579 to 15075 | aim point search used by the planner (sets two globals that gate types 8, 6, 32, 33) |
| FUN_004289e0 | 19528 to ~23000 | per tick golfer step (walking, swing, ball flight, landing) |
| FUN_00427380 | 17684 to 19520 | golfer finishes a hole (raises type 0x13, applies end of hole mood decay) |
| FUN_004675d0 | 71024 | `lastMood(g)`: 2 if the newest history slot has the 0x8000 flag, else 1 when bit 0x4000 is clear, else 0 (0 happy, 1 neutral, 2 unhappy). Used as the location of type 34 and 58 |
| FUN_0040c170 | 10789 | height of a tile corner vertex (table 0x5a4998, stride 51, values 3 to 15; 3 outside the map). Used for uphill and downhill |
| FUN_0040c450 | 10899 | directional slope of a tile along a heading octant (corner height differences) |
| FUN_0040c4b0 | 10917 | distance between a position (1/1024 tile units) and a tile, in yards (25 yards per tile) |
| FUN_004070b0 | 5242 | obstacle height band test for a terrain id (section 3.1) |

### 0.1 Golfer record (0x100 bytes at 0x5794b8 + g * 0x100), fields used here

| Offset | Address | Meaning (my names) | Tag |
|---|---|---|---|
| +0x00/+0x04 | 5794b8/bc | body position x, y (1/1024 tile) | EXACT |
| +0x10 | 5794c8 | flag dword (bits used here: 0x1 penalty replay, 0x2 ball hit an obstacle this shot, 0x20/0x40 slice/hook marker set at plan time, 0x80 high shot planned, 0x200 player controlled golfer, 0x400000 pure strike planned, 0x20000 complained about lie, 0x40000 ball in flight, 0x10000 riding a cart, 0x40000000 walked through penalty ground last segment, 0x20000000 rampaging golfer) | EXACT for use, DERIVED for names |
| +0x14 | 5794cc | short, walking segment counter | EXACT |
| +0x18 | 5794d0 | kind: 0 ordinary visitor, (kind & 0xe0) == 0x40 pro, (kind & 0xe0) == 0x20 VIP (DERIVED names) | EXACT |
| +0x19 | 5794d1 | skill mask: 1 length, 2 accuracy, 4 imagination (matches DECODE_HOLE_STATS) | EXACT |
| +0x1a | 5794d2 | facing octant 0 to 7 | EXACT |
| +0x1b | 5794d3 | hazard score `hz` (section 2.2) | EXACT |
| +0x1c | 5794d4 | club index (0 driver ... 12 sand wedge, 13 putter) | DERIVED (PUBLISHER_EXE_NOTES) |
| +0x20 | 5794d8 | step sub state: 0 ball in flight / waiting, 1 walking, 2 and up swing counter | DERIVED |
| +0x21 | 5794d9 | current hole (1 based, 0 none, 0x13 pseudo hole, 0xff arriving) | EXACT |
| +0x22 | 5794da | strokes taken on this hole | EXACT |
| +0x23 + hole | 5794db.. | scorecard bytes | EXACT |
| +0x36 | 5794ee | signed momentum counter | EXACT |
| +0x70 | 579528 | history: last 10 reaction types, slot 0 newest | EXACT |
| +0x7a | 579532 | history stamps: `hole * 11 + strokes (+1 when type > 3)` | EXACT |
| +0x84 | 57953c | speech timer, 0 means silent (set to 7 by every reaction, decremented every 8th tick) | EXACT |
| +0x85 | 57953d | type of the last reaction (bit 0x80 set in partner chat mode) | EXACT |
| +0x88 to +0x9a | 579540.. | history: short per slot, low bits the location, 0x4000 positive applied, 0x8000 base was negative, 0xc000 negative applied | EXACT |
| +0x9e | 579556 | pose hold timer (negative, counts up one per 2 ticks; golfer stands still while negative) | DERIVED |
| +0xa0 | 579558 | short, bit mask of club indexes used so far | EXACT |
| +0xa2 | 57955a | partner golfer index | EXACT |
| +0xa4 | 57955c | mood, clamped -10 to 10 | EXACT |
| +0xac | 579564 | plan class: 0 normal, 1 draw, -1 fade, 3 high, 4 low running | EXACT |
| +0xae | 579566 | conversation state 0 to 4 (this is the "state" the sentence builder reads; NOT the speech timer) | DERIVED |
| +0xc4/+0xc8 | 57957c/80 | ball position before the current shot | DERIVED |
| +0xcc/+0xd0 | 579584/88 | chosen aim tile | EXACT |
| +0xd4/+0xd8 | 57958c/90 | ball position | EXACT |
| +0xdc, +0xe0, +0xe4, +0xe8, +0xec | 579594.. | ball height, heading (32 bit angle), speed, vertical speed, curve | DERIVED |

Terrain table (runtime 0x578350, stride 0x30, see DECODE_COMMENTS 3.1): `pen(t)` is the signed byte at +0x22 (0x578372), `cls(t)` the byte at +0x26, `sub(t)` the byte at +0x27. The values of `pen` per terrain id are UNKNOWN to me (static table, not in the decompile text). The code only uses `pen <= 0` as "clean lie" (fairway, green, tee class) and `pen > 0` as "trouble". DECODE_HOLE_STATS already relies on `pen <= 0` for fairways hit.

## 1. The reaction routine FUN_00467a00(g, type, loc)

### 1.1 Order of operations (EXACT, lines in brackets)

1. Return at once when: golfer strokes > 9 (71433); g >= 0x98 (71436); game flag word 0x59e7b8 has bit 0x2000000 (71439).
2. Type 0x13 to 0x17 conversion (71442 to 71452): as in DECODE_HOLE_STATS section 2 (confirmed, including the "more than 9 counted rounds" and "kind & 0xe0 != 0x20" conditions, hole flag 4 with 2 or more over par, hole flag 8 with under par).
3. Build the sentence (71453), then set the speech timer to 7, the last type and its location (71456 to 71471). In partner chat mode (global 0x5409a8 set by the chat routines) the timer and type go to the partner instead, with 0x80 added to the type.
4. Type 0x23 returns if it equals the newest history type (71472 to 71478). Type 0x13 returns here (71479).
5. History shift (71482 to 71503): slots 9 down to 1 take the previous contents, slot 0 gets the type, the stamp and the location. Note: EVERY type except 0x13 and a repeated 0x23 enters the history, including delta 0 types (5, 31, 37, 38, 16, 17, 40, 41, 42, 45, 55 to 57, 60). They push older entries out of the repeat windows below.
6. Base delta `b` from the switch (71512 to 71730), table in section 4. Types above 0x2f with `b == 0` return at 71740 (no mood, no count).
7. Momentum (71744 to 71763), only when kind != 0: if sign(momentum) == -sign(b) the momentum becomes 0, else momentum += b. For types 2, 3, 12, 13 with negative momentum and b < 0 and momentum < b, momentum becomes 1. When the player controlled golfer (global 0x5a59f8) gets b < 0 and has flag 0x40000, an alert sound is queued. Momentum feeds the planner (range and error) and decays by 1 per finished hole (19500).
8. Repeat gate for single complaints (71764 to 71794), then scaling (71795), partner attitude (71798), apply (71809), cell marker (71815), counts (71830), flags and per cell counters (71839 to 71861), advisor message (71862 on).

### 1.2 The mood rule

Let `b` be the base delta (-3 to +1), `kind` the golfer kind.

Gate (EXACT): runs only when `b == -1`, `kind == 0` (ordinary visitor) and global 0x543cf4 != 2.
* Start with `a = 0`.
* Window `W`: 3 when d == 0, 5 when d is 1 or 2, 10 when d == 3. For slots 1 to W-1 of the history (slot 0 is the current reaction): if any slot holds the same type, `a = -1`.
* If slot 1 (the immediately previous reaction) carries the 0x8000 flag (its base delta was negative, even if it was gated to 0), `a = -1`.
* If global 0x543cf4 == 1, `a = 0` and skip the scaling (all single complaints are muted in that mode; meaning of the global UNKNOWN).
* So for an ordinary golfer, a -1 base reaction only costs mood if it repeats the same type inside the window, or follows directly after a negative base reaction.
Kinds other than 0 skip the gate, and global 0x543cf4 == 2 disables the gate (a stays -1).

Scaling (EXACT, 71795): any negative `a` becomes `(a - 1) / 2` with C truncation: -1 to -1, -2 to -1, -3 to -2, unless `(kind & 0xe0) == 0x40` (pro), which keeps the full value. Positive deltas are never scaled.

Partner attitude cascade (EXACT, 71798 to 71808): when the scaled delta is negative, `(kind & 0xe0) != 0x20`, `(((strokes + g) & 1) + 2) <= d`, and the type is not 0x2f, 0x15 or 0x18: if the partner is on a hole (hole > 0), has a silent speech timer and mood > 0, the partner reacts with type 0x2f (location 0x14) and its timer gets +2. So on d 2 and 3 a complaint can make a happy partner complain about the complainer.

Apply (EXACT, 71809 to 71814):
* `mood = clamp(mood + a, -10, 10)`.
* `hole.fun (+0x158) += a` with NO clamp (it is added even when the mood sits at its limit, and even when `a == 0`).

Bookkeeping when `a != 0` (EXACT, 71830 to 71838): `hole.count[type] (+0xd8 + 2*type) += 1`, `hole.loc[type] (+0x16c + 2*type) = loc (low 16 bits)`, and a positive `a` sets 0x4000 in history slot 0. Then, independent of `a != 0` for the 0x8000 and 0xc000 flags: base negative sets 0x8000, applied negative sets 0xc000 (71839 to 71845). Per cell counters (71846 to 71861): positive `a` adds 1 to the byte at 0x5a6378[cell], negative `a` adds 1 to the byte at 0x56c7e4[cell] (cell = x tile * 50 + y tile of the golfer's body, no saturation; use UNKNOWN, probably a map overlay).

The hole used is the golfer's CURRENT hole (byte +0x21). This confirms DECODE_HOLE_STATS section 3 ("adds to mood clamped -10..10 and to the hole fun total, bumps count and stores location when delta is not 0"), with these additions: early returns (strokes > 9 etc.), `fun` is unclamped and added even for `a == 0`, the exact gate windows (3, 5, 10 slots, plus the previous-negative rule), the 0x543cf4 modes, the partner cascade, momentum, the cell counters.

Ground marker (EXACT, 71815 to 71829): when `a < 0`, game flag 0x4000000 is clear, type != 0x2f, and either `rand(6) <= d` or the golfer has flag 0x20000000 (rampaging), and FUN_00407000(x, y, 2) is false (no nearby landmark of the matching class), then if the cell flags (0x53caf0 short) & 0xc00 == 0 and the terrain is not water: `cell |= 0x4800`, byte 0x578804[cell] = 1, redraw flag set. Probability per negative reaction: (d+1)/6 (always for rampaging golfers). Visual meaning UNKNOWN (litter or damage); the 0x800 bit also makes the scenic glance code call a decoration cell "ugly" (type 20).

Advisor message (EXACT, 71862 to 74451, optional UI): the first time each type fires in a saved game (bit set in 0x59c08c for types below 32, 0x571d38 for 32 and up) a ticker message is queued when all hold: option bit 4 of 0x5a5a00, more than 500 ticks since the last message (0x55e5ac), conversation state (+0xae) == 0, golfer screen position strictly inside x 100 to 700 and y 100 to 400, and the ticker is free (FUN_0040cb00). Its colour depends on the sign of the applied delta. Message meanings are in sections 2 and 3 where useful. Not part of the mood.

Needs speech state? The reaction routine itself never tests the speech timer or the conversation state. It always overwrites the timer with 7. Only individual callers test the timer (section 2.4, 3.4).

## 2. Producers in the shot planner FUN_00424120

### 2.1 When it runs (EXACT)

Called with default target (`tx == -1`) once per stroke when the golfer is at the ball and starts the shot (21566, mode 0 for computer golfers, mode 1 when the player directs the golfer, golfer flag 0x200). Also called at 14350 with an explicit target (preview), where the whole reaction tree is skipped (it sits inside `if (tx == -1)` at 15622).

* shot plans counter (hole +0x24) increments only when the club index is NOT 13 (putter) (15631 to 15633). Correction to DECODE_HOLE_STATS section 1: putter plans are not counted (club 13 is forced on a green within 49 yards, 15385).
* `b0c` = shot distance in yards, clamped 0 to max range (15374). One tile = 25 yards.

### 2.2 Hazard score hz (byte +0x1b) (DERIVED, 15259 to 15361, 15699)

hz is recomputed at every plan, then overwritten by the reaction tree, then read again at landing. This is the key number for types 1, 2, 4, 5, 31, 37, 38.

Plan time value:
* `n = b0c / 25`. `acc = 4 - (g & 3)`.
* Walk `k = 0 .. n-1` along the shot line at distance (0.5 + k) tiles; stop at the map edge. Tile weight `w = pen(terrain)`. `acc += k_plus_1... ` precisely: `weight[terrain] += (k+1) * w * 2` (selection table), a second ray rotated by either +15 or -15 degrees (coin toss) adds `k * w * 2 + pen(second tile) * (k+1)` to `acc`.
* Counters: `afc` += 1 for each sample after the first on a tree class tile (class 13); += 2 when the tile is more than 1 height unit above the ball tile (first half of the line) or above the aim tile (second half). A sample tile with flag 0x100 records `ad4 = terrain id` (a "scenic object on the line").
* The 8 neighbours of the aim tile: in map, `acc += pen * (n+1) / 2` (weight table too); outside the map `acc += 4 + 4n`.
* `hz = acc / (n + 3)`; `+4` when `afc > 1`; 0 when the ball is on a green; capped at 10 when `b0c < 41` and hz > 10.
* Then (15699, any club) `hz += | H(aim tile) - H(ball tile) |` (FUN_0040c170 heights).
* The reaction tree then overwrites hz: after a hazard remark (types 5, 37, 38) hz = 10; after type 31, hz = 20; when the hazard test fails hz = 6; after type 4, hz = 0. Other branches (8, 20, 22, 24, 29, 30, 6, 32, 33) leave hz as computed.
The terrain weights depend on `pen` values (UNKNOWN table), so absolute hz numbers cannot be tabulated yet.

### 2.3 The aim search globals (EXACT in structure, FUN_00422fb0)

Run only when `b0c` exceeds the threshold (75 yards, or 25 yards for golfers with skill bit 4 or flag 1) and the ball is not on a green, mode 0 (15159 to 15175). Otherwise the planner sets `DAT_005a7140 = 0` and aims at the green (15219).

* `DAT_005a9ce4` ("no safe route"): set to 1 at the start (14653); set to 0 when, for a simulated chain of N steps (N = 2, 4, 8), the count of landing tiles with `pen <= 0` reaches `((d + 4) * N) / 8` (14904). With N = 2 that threshold is 1 for every d; N = 4 gives 2, 2, 3, 3 for d 0 to 3; N = 8 gives 4, 5, 6, 7. If the search is not run the old value stays (stale; see 7.3).
* `DAT_005a7140` bits (set only when the search reaches N = 4, otherwise it keeps a stale value): 1 = choose distance (the spread of the surviving candidates' distances is 76 yards or more), 2 = choose side (spread under 76 yards but the heading spread of candidates more than 30 degrees, 0x15555555), 4 = the chosen route uses a slope/roll (candidate flag). Exact distance reference of the spreads is hidden by Ghidra (UNKNOWN).
* Lane result: -1, 0 or +1 (the draw/fade lane, returned as `shape`). Lanes +-1 are only allowed for golfers with skill bit 4 on a clean lie (kind != 0 needs ability bits 0x20 and 0x40), only for candidate distances of 76 yards or more, and only when they have the lowest cost.
* Chosen aim tile (579584/88) is the cheapest candidate, or the green if none.

### 2.4 The reaction tree, in the order the code tests it (15694 to 15884)

First the hazard score gets its height term (15694 to 15699). Then three tiers. `tile` below is the golfer's own tile (ball tile); `cur = terrain(ball tile)`; `tgt = terrain(aim tile)`.

Tier 1: at most one of these per plan; none needs a silent speaker.

| Order | Type | Trigger (EXACT unless noted) | Location | Notes |
|---|---|---|---|---|
| 1 | 22 celebrity house or 20 ugly object | scenic glance scan found one (line 15636 to 15691), 22 wins over 20 | celebrity index or cell | outside this doc's scope, listed for precedence |
| 2 | 8 bad design | `DAT_005a9ce4 != 0` AND `pen(tgt) > 0` AND `pen(cur) <= 0` AND mode == 0 AND flag 1 clear (15700 to 15790) | 0x14 | every other tier 1 test lives in the "else" of this condition. Toast: no safe place to aim |
| 3 | 24 weeds | cell flag 0x800 set on the ball tile, 0x4000 clear, and `lastMood(g) != 0` (15709) | 0x14 | |
| 4 | 29 variety | hole >= 2 AND strokes == (g & 1) + 1 AND `V <= rand(3)` where V = hole +0x1fc (15716 to 15722) | 0x14 | independent of d (correction to DECODE_HOLE_STATS section 5, which said d 0 only). P = 100% for V = 0, 2/3 for V = 1, 1/3 for V = 2, 0 for V >= 3. Ends the plan's tier 1 |
| 5 | 30 repetition | d > 0 AND hole >= 2 AND strokes == 1 AND `V > rand(3) + 3` (15751 to 15766) | 0x14 | P = 1/3 for V = 4, 2/3 for V = 5. Only reached if 29 did not fire |
| 6 | 4 easy shot | d > 0 AND NOT (`rand(4) + 2d < hz`) AND `b0c >= 101` AND `pen(cur) <= 0` AND `ad4 == 0` (15751 to 15763) | 0x14 | then hz = 0. Meaning from the toast: the next shot looks easy and holds no challenge; this is a plan time complaint, not a miss. P (d = 1): hz 0 to 2 always, 3 = 75%, 4 = 50%, 5 = 25%, 6 or more never |
| 7 | 5 / 37 / 38 hazard ahead, or 31 lots of terrain | `rand(5) + 6 + d < hz` AND `b0c > 40` AND `ad4 == 0` (15724 to 15746). Reached directly for d == 0, and from the d > 0 chain when the type 4 test fails | terrain id with the largest weight (lowest id wins ties) | if hz >= 24 then type 31 and hz = 20. Else relative direction `r = (dirOfWorstTerrain - shotOctant) & 7`: r in {0, 1, 7} gives 38, r in {2, 6} gives 5, r in {3, 4, 5} gives 37 (rays store octant xor 4, so ray hazards give r = 4, type 37). Then hz = 10. If the test fails hz = 6. P = clamp((hz - 6 - d) / 5, 0, 1). All four types have delta 0 |
| 8 | 6 uses slope | `(DAT_005a7140 & 3) == 0` (so only bit 4 set) AND `pen(cur) <= 0` (15768 to 15772) | 0x14 | only in the branch where `DAT_005a7140 != 0` and plan class != 4 |
| 9 | 32 choose distance / 33 choose side | `DAT_005a7140` has bit 1 (32) or bit 2 (33), after the line `if b0c < 100 or mode != 0 then DAT_005a7140 &= ~3` (15712, 15774) | 0x14 | so only shots of 100 yards or more, computer controlled. Type = 0x20 when bit 1 is set, else 0x21 |

If `DAT_005a7140 == 0` or plan class == 4 the chain goes 4, 5, 6, 7 (rows 4 to 7); otherwise rows 8, 9. Row 4 and row 5 sit at the head of the first sub branch.

Tier 2 (only when no reaction was stored this plan, tested as "history stamp unchanged", and the speech timer is 0; each of these sets the timer to 7 so later ones in the same plan skip the timer test only for themselves, see the note below):

| Order | Type | Trigger | Location |
|---|---|---|---|
| 1 | 28 lovely object | `ad4 != 0` AND `pen(cur) <= 0` (15796); then tiers 2 and 3 are skipped | cell |
| 2 | 55 draw | shape == +1 (15800) | 0x14 |
| 3 | 56 fade | shape == -1 (15805) | 0x14 |
| 4 | 57 high soft shot | flag 0x80 AND `b0c > 100` (15809) | 0x14 |
| 5 | 60 low running shot, or 9 | plan class == 4 (15812): `strokes != 0` gives 60, `strokes == 0` gives type 9 (see 7.2) | terrain id of the ball tile |
| 6 | 59 and 48 / 49 | not shot related | |

Types 55, 56, 57 and 60 have delta 0 (EXACT, no switch case), so they only set the speech timer, enter the history and may raise an advisor message. Toast texts confirm the names: 55 draw, 56 fade, 57 high, 60 low.

After tier 2 the timer is re-read: if it is non zero the stamp variable is poisoned (15850) so tier 3 cannot fire.

Tier 3 (needs no reaction at all this plan, speaker silent):

| Type | Trigger | Location |
|---|---|---|
| 54 club first use | hole >= 2, club < 13, bit for the club in +0xa0 clear (15856); the bit is always set afterwards (15861). Delta +1 only when d < 2 | club index |
| 46 downhill | `H(aim tile) < H(ball tile)` (15864 to 15868) | 0x14 |
| 45 uphill | `H(aim tile) > H(ball tile)` (15870 to 15874) | 0x14 |
| 62 greeting | on a green, `(strokes + g + hole) & 3 == 0`, kind 0 | not shot related |

Plan class and flags set around the tree (15518 to 15620, 15818 to 15822):
* Low running shot (class 4): AI golfers need skill bit 4 or flag 2 (previous shot hit an obstacle, flag cleared at 15885), not on a green, and BOTH tiles 0x80 and 0x400 units ahead along the heading must be class 13 (trees). Player golfers need the low button (shape global 0x58f330 == 4).
* High shot (class 3, flag 0x80): skill bit 4, shape 0, ball not on green, `pen(cur) == 0`, club index > 3, `b0c > 25`, and the aim tile is a green (or the player chose high, 0x58f330 == 3).
* Slice / hook markers: when `b0c > 75`, curve > `30 << 16` (`30 << 15` for skill bit 2) sets flag 0x20; curve < minus that sets 0x40 (15815 to 15823), but only inside the silent branch of tier 2. They are consumed at landing (section 3.3).
* Pure strike flag 0x400000 (15524 to 15531): game flag 0x800000 clear, `b0c > 75`, and `rand(power) > (|curve| >> 9) + 0x200` where `power` is the launch speed value at +0xe4 and `curve` the random aim error scaled by skill (random error: `r = rand(101) - 50`, magnitude |r|/2 below 20, |r| - 10 below 40, else 2|r| - 50, sign kept, times 0x50000 before skill scaling). Computer golfers also halve the curve; the player golfer's curve becomes 0. Pros and VIPs additionally get the flag from a bad lie recovery roll `rand(10 * pen(lie)) <= byte 0x5795b0` (15976).

Mood effect: only the stored types change mood: 8 (-2), 29 (+1), 30 (-2), 4 (-2), 6 (+1), 32 and 33 (+1), 46 (+1), 54 (+1 when d < 2), 24 (-2), 28 (+1), 20 (-2), 22 (+1). Types 5, 31, 37, 38, 45, 55, 56, 57, 60 are mood neutral.

## 3. Producers in the golfer step FUN_004289e0

### 3.1 Obstacle trouble (type 12), 21780 to 21815 (EXACT)

During flight, while ball height >= 2 and the ball sits over tile `t`, FUN_004070b0(t, height) tests a band: only terrain ids 13 to 16 (trees) and 21, 22 (building, band 0 to 200, disabled when the cell's low 5 bits are 1 to 4) can be hit. Tree bands (lower, upper) in ball height units, redrawn every tick:
* theme 0 (DAT_005a34e0): id 13 (50, 400+rand(100)), id 14 (20, 100+rand(100)), id 15 (50, 100+rand(100)), id 16 (75, 400+rand(200)).
* theme 1: as theme 0 except id 15 is (100, 300+rand(100)).
* theme 2: id 13 (20, 100+rand(100)), id 14 (50, 400+rand(100)), id 15 (50, 100+rand(100)), id 16 (75, 400+rand(200)).
* theme 3: ids 13 to 16 all (20, 400+rand(100)).
Hit when `lower < height < upper`. If hit and golfer flag 2 is clear: `dist` = distance from the ball to the tile centre (1/1024 units; plus `dist * byte 0x5795b1 / 4` for a kind != 0 golfer with ability bit 0x200); the collision is real when `dist < rand(0x180)`. Effect: heading += `(rand(128) + 64) * 2^24`, speed -= rand(speed), a thud sound, and reaction type 12 with location = terrain id, then flag 2 is set (first collision per shot only). Probability per airborne tick over a tall enough tile: `(384 - dist) / 384` for dist < 384.
Delta -1 (gated, section 1.2). Toast: the ball bounced off a tree.

### 3.2 Near miss (type 9), 21936 to 21954 (EXACT)

On a ground impact (height < 1, vertical speed < 0, 21818) the rebound vertical speed is `clamp(-64 - v * k / 12, 0, 9999)` (k is the terrain bounce byte +0x20, min 2 when the 0x578370 test at 21822 applies; rebound below 0x80 becomes 0). When the rebound exceeds 200: for every other golfer `k` with hole != 0, hole != the shooter's hole, hole != 0x13, a ball in play (position x of the ball != 0) and distance between the SHOOTER'S BALL and golfer k's body below `(d + 2) * 0x400 / 2` (1024, 1536, 2048, 2560 units for d 0 to 3): `react(k, 9, 0x14)`. Delta -3 (applied -2 for ordinary golfers, no gate because it is not -1). Holes 1 apart are the typical victims. Strokes > 9 victims are skipped by the early return.

### 3.3 Landing (types 1, 2, 3, 13, 16, 17), 21956 to 22163 (EXACT)

Trigger: ball speed < 0x40, height 0, vertical speed 0 (the ball has stopped, 21956). Order inside:
1. Terrain `T` = terrain id of the resting tile; `strokes += 1` happens BEFORE any reaction (22004), so the 10th stroke's reactions are dropped by the early return.
2. Clean lie (`pen(T) <= 0` and tile inside the map, 22021): if `hz > 8` and the resting tile is closer to the hole than the previous ball tile (tile distances, FUN_0040acd0, hole green tile at hole +0x18): pose id 0xc, pose hold -99, facing turned, `react(g, 1, T)` (22023 to 22037). So type 1 needs a "tough looking" plan (hz > 8: only after a hazard remark or with hz left high) and real progress.
3. Trouble (`pen(T) > 0` or outside the map, 22040 on):
   a. Easy looking: `hz < d + 3` (hz is 0 only after type 4) AND golfer flag 2 clear AND resting cell flag 0x100 clear: `react(g, 2, T)` with pose 0xd (22041 to 22048).
   b. Started in trouble: let `P` = terrain of the previous ball tile; if `pen(P) > 0` AND `pen(P) <= pen(T)` AND cell flag 0x100 clear: `react(g, 2 if T == P else 3, T)` and flag 0x20000 set (22049 to 22062). Type 3 therefore means "already in trouble and now in different, at least as bad, trouble".
   c. Water (T == 17): sound, `react(g, 13, T)` then the drop search (22063 to 22115). The same path runs when a local flag `bVar4` is true for a ball resting at the centre of a ravine tile (id 10; DERIVED from 21712 to 21734).
   d. Outside the map or terrain 20 and not water: `react(g, 2, T)`, the ball goes back to the previous spot (22117 to 22124).
   After c or d: pose 0xd, flag 1 (replay), and `strokes += 1` when strokes < 8 (penalty stroke, 22125 to 22133).
4. Hook and slice (22155 to 22162): speech timer == 0 AND `pen(T) > 0`: flag 0x20 gives `react(g, 17, 0x14)`, flag 0x40 gives `react(g, 16, 0x14)`. Delta 0 (neutral). Because 1, 2, 3, 13 set the timer to 7, hook and slice only appear when nothing else was said.
Other landing reactions in the block (not shot list): 51 (22152) when the first shot on holes below 6 out-carries the golfer's max range.

Deltas: 1 is +1; 2 is -1 (gated); 3 is -2 (applied -1); 13 is -1 (gated). Types 2 and 13 therefore only cost mood on repeats within the window (3, 5 or 10 reactions, section 1.2) or right after another complaint, and 3 costs -1 each time.

Second producer of type 1 (UNKNOWN details): the snapshot routine FUN_00407e00 (called through FUN_00409620 at landing for golfers followed by a photo slot) raises `react(g, 1, 1)` (8365) when the shot photo mode is 0 and the newest history type is not 1. Trigger is a distance test against hz ((`dist+1` times a base of about 0x14 to 0x19 +/- 5) < shot yards) and needs the golfer on screen.

### 3.4 Strike, bounce and walking

* Type 40 good shot exclamation (22292): at swing frame 5 (the one tick where animation id == 5, counter 8) when plan flag 0x400000 is set: sound 0xbd, `react(g, 40, 0)`, speech timer forced to 3. Delta 0.
* Type 42 lucky bounce (21915 to 21934): on a ground impact, kind != 0, rebound speed > 0x100, `rand(100) < byte 0x5795b1`, ball within 75 yards of the hole and `pen(T) <= 0`: heading pulled 4/5 toward the hole and `react(g, 42, 0x14)`. Pros and VIPs only. Delta 0.
* Type 10 walks through an obstacle (21051 to 21071): evaluated at the end of a walking segment (golfer at least 0x400 units from its destination and segment counter +0x14 <= 0). The flag 0x40000000 is cleared and nothing happens when any of: d == 0, strokes > 1, hole 0x13, `pen(terrain under the BALL) > 0`, (strokes != 0 and flag 0x800), cell flag 0x20 on the golfer's tile, golfer tile terrain 22, or `pen(golfer tile terrain) < 2`. Otherwise, when the facing octant is even (axis aligned move) and the golfer tile is not a sand trap (id 7): if flag 0x40000000 is already set and the last reaction type is not 10, `react(g, 10, terrain id of the golfer's tile)` and the flag is cleared; else the flag is set. So it takes two consecutive qualifying segment ends. Delta -2 (applied -1).
* Type 43 steep slope (21072 to 21083): same event as type 10: hole != 0x13, golfer not riding (flag 0x10000 clear), last reaction type != 43, facing even, and FUN_0040c450(x, y, facing) > 2: `react(g, 43, slope value)`. d is irrelevant. Delta -2 (applied -1). The text ignores the location.

## 4. Summary table: shot related types

Base delta from the reaction switch (71512 to 71730, EXACT); "applied (ord)" is for an ordinary golfer after section 1.2.

| Type | Meaning | Base | Applied (ord) | Raised in | Trigger summary | Loc | Per |
|---|---|---|---|---|---|---|---|
| 1 | good shot | +1 | +1 | landing 22036 | clean lie, hz > 8, ended closer to the hole (also snapshot 8365 with loc 1) | landing terrain id | stroke |
| 2 | bad lie same terrain, easy shot gone wrong, out of bounds | -1 | -1 on repeat | landing 22047, 22060, 22121 | section 3.3 a, b, d | terrain id | stroke |
| 3 | bad lie different terrain | -2 | -1 | landing 22060 | section 3.3 b | terrain id | stroke |
| 4 | easy shot, no challenge | -2 | -1 | plan 15761 | d > 0, long, clean, few hazards | 0x14 | plan |
| 5 | hazard ahead (side) | 0 | 0 | plan 15744 | r in {2, 6} | worst terrain id | plan |
| 6 | uses slope | +1 | +1 | plan 15770 | slope route, clean lie | 0x14 | plan |
| 8 | bad design | -2 | -1 | plan 15790 | no safe route, aim on trouble | 0x14 | plan |
| 9 | ball nearly hit someone | -3 | -2 | flight 21949; plan 15813 | section 3.2; tee plan class 4 (7.2) | 0x14 or terrain | bounce |
| 10 | walk through obstacle | -2 | -1 | walk 21066 | section 3.4 | terrain id | segment |
| 12 | obstacle trouble | -1 | -1 on repeat | flight 21812 | tree or building hit | terrain id | shot |
| 13 | ball in water | -1 | -1 on repeat | landing 22067 | resting in water (or ravine centre) | terrain id | stroke |
| 16 | hook | 0 | 0 | landing 22160 | flag 0x40, trouble, silent | 0x14 | stroke |
| 17 | slice | 0 | 0 | landing 22157 | flag 0x20, trouble, silent | 0x14 | stroke |
| 29 | variety | +1 | +1 | plan 15720 | section 2.4 row 4 | 0x14 | plan |
| 30 | repetition | -2 | -1 | plan 15765 | section 2.4 row 5 | 0x14 | plan |
| 31 | lots of terrain | 0 | 0 | plan 15744 | hz >= 24 | worst terrain id | plan |
| 32 | choosing distance | +1 | +1 | plan 15774 | spread >= 76 yards, shot >= 100 yards | 0x14 | plan |
| 33 | choosing side | +1 | +1 | plan 15774 | heading spread > 30 degrees | 0x14 | plan |
| 37 | hazard ahead (behind/along) | 0 | 0 | plan 15744 | r in {3, 4, 5} | worst terrain id | plan |
| 38 | hazard ahead (straight) | 0 | 0 | plan 15744 | r in {0, 1, 7} | worst terrain id | plan |
| 40 | good shot exclamation | 0 | 0 | strike 22292 | pure strike flag | 0 | stroke |
| 41 | not a shot event | 0 | 0 | 20948 | golfer uses a kind 3 building (UNKNOWN which) | 0x14 | visit |
| 42 | lucky bounce | 0 | 0 | flight 21933 | pros and VIPs only | 0x14 | bounce |
| 43 | steep slope | -2 | -1 | walk 21082 | climbing slope > 2 | slope value | segment |
| 44 | consolation | +1 | +1 | none found | no caller anywhere in the decompile (all 70 or so call sites checked) | n/a | never |
| 45 | uphill shot | 0 | 0 | plan 15874 | aim corner higher than ball corner | 0x14 | plan |
| 46 | downhill shot | +1 | +1 | plan 15868 | aim corner lower than ball corner | 0x14 | plan |
| 55 | draw | 0 | 0 | plan 15801 | lane +1 | 0x14 | plan |
| 56 | fade | 0 | 0 | plan 15806 | lane -1 | 0x14 | plan |
| 57 | high soft shot | 0 | 0 | plan 15810 | flag 0x80, > 100 yards | 0x14 | plan |
| 60 | low running shot | 0 | 0 | plan 15813 | plan class 4, strokes > 0 | ball terrain id | plan |

Needs speech free or conversation state: types 4, 5, 6, 8, 29, 30, 31, 32, 33, 37, 38, 10, 12, 13, 1, 2, 3, 9, 43, 42 do NOT test the speech timer or the conversation state (the new reaction just overwrites the timer). Types 28, 55, 56, 57, 60, 9 (tee case), 45, 46, 54 and 16, 17 need the speech timer == 0; 45, 46, 54 additionally need that no reaction fired in this plan. Type 47 needs the partner's timer == 0. Nothing here needs the conversation state (+0xae); only the sentence builder and the advisor message read it.

## 5. Hole record updates confirmed

The only writer for the per type count (+0xd8), per type location (+0x16c) and fun total (+0x158) is `react`: count and location only when the applied delta is non zero, fun always (section 1.2). Type 0x13 never reaches them; the conversion to 0x17 does. The hole counted is the golfer's current hole. This matches DECODE_HOLE_STATS section 3 with the additions listed there.

## 6. End of hole mood decay (FUN_00427380, 19489 to 19520, EXACT, secondary)

When a golfer finishes a hole: smoothed mood (+0xb4 short) = `old * 7 / 8 + mood`; momentum decays by 1 toward 0. Then mood -= `((P + 6 + hole) * (mood - 1 + d) * (d + 1)) / ((K * 5 + 15) * 8)` (P: short at 0x584a0a + profile * 0x2c, K: global 0x543cd4; both UNKNOWN meaning), and for pros mood -= `((hole + 6) * mood * d) / 160`. Initial mood at spawn is 4 for d == 0, else 3 + rand(3), plus global 0x543cc4 when set (13994). A golfer with mood below -10 after a tantrum leaves (20797).

## 7. Unresolved items and suspicious spots

1. `pen`, `cls`, `sub` values per terrain id (table +0x22, +0x26, +0x27 and +0x2c flags) are not in the decompile text, so hz and the "trouble" tests cannot be evaluated numerically. UNKNOWN: values for ids 0 to 22, in particular which ids have `pen <= 0` and which have `pen >= 2` (type 10).
2. Type 9 is raised from the planner at 15813 for a tee shot (strokes == 0) with plan class 4 (trees directly ahead), with the golfer's terrain id as location. Decompiler shows `(strokes != 0 ? 0x3c : 9)`; it may be an original quirk or intended. EXACT as decompiled, behaviour risk.
3. `DAT_005a7140` and `DAT_005a9ce4` keep stale values when the aim search does not run or ends at N = 2; types 6, 8, 32, 33 can then fire from an old search. EXACT in the code, probably an original bug.
4. Cell flag meanings: 0x100 (scenic or decoration marker that suppresses lie complaints), 0x800 and 0x4000 (damage marker), 0x1000, 0x80, 0x20, 0x400. Only partly known.
5. Which of `DAT_00543cf4` values 1 and 2 correspond to which game mode; meaning of game flags 0x2000000, 0x4000000, 0x800000, golfer flag 0x20000000.
6. The distance references of the aim search spreads (stack variables hidden by Ghidra) and the exact meaning of candidate bit "slope" (bit 4 of `DAT_005a7140`).
7. Which building kind raises type 41, what flag 0x20 and 0x40 of the ability word at +0x16 are, how the conversation state +0xae is set (62860 to 62890).
8. Type 44 has no caller; probably dead code.
9. Snapshot raised type 1 (8365): trigger only outlined.
10. Plan time random error scaling by skill (the 0x5795a8 to 0x5795af bytes) not decoded, so the pure strike probability (type 40) is only known as an inequality.

## 8. Implementation sketch

```
react(g, type, loc):
  if strokes[g] > 9 or g >= 0x98 or flagNoReact: return
  type = convert13to23(g, type)
  say(g, type, loc); timer[g] = 7; lastType[g] = type; lastLoc[g] = loc
  if type == 0x23 and hist[g][0] == 0x23: return
  if type == 0x13: return
  hist.push(g, type, stamp = hole*11 + strokes + (type > 3), loc)
  b = baseDelta(type, d)
  if b == 0 and type > 0x2f: return
  a = b
  if kind != 0: momentum update
  if a == -1 and kind == 0 and mode != 2:
      a = 0
      for s in 1 .. W-1: if hist[s].type == type: a = -1
      if hist[1].flags & 0x8000: a = -1
      if mode == 1: a = 0
  if a < 0 and (kind & 0xe0) != 0x40: a = (a - 1) / 2   // C division
  partner cascade; mood = clamp(mood + a, -10, 10); hole.fun += a
  ground marker roll; if a != 0: hole.count[type]++, hole.loc[type] = loc
  history flags and per cell counters
```

The planner tree, landing block, flight and walking hooks follow sections 2 and 3 literally.

## 9. Port status
Implemented in `include/sg/reactions.h`, `src/reactions.cpp` (engine) and `tools/sgview.cpp` (producers): the reaction routine (history, base delta, the lone -1 gate with windows 3, 5, 10, the negative halving, partner cascade flag, mood clamp, polarity marks), the plan time tree (22 or 20 from the scenic scan, 29, 30, 4, hazard remarks 31 and 37 as delta 0 speech, then 28, 46 and 45), landing reactions 1, 2, 3, 13 and near miss 9. The hazard score follows the doc's shape with a PLACEHOLDER lie penalty table, which makes type 4 ("too easy") fire on every long shot over clean ground on difficulty above 0, as the doc's probabilities say it should on a hazard free hole.
Not implemented yet: the aim search outputs (types 6, 8, 32, 33), tree and building collisions in flight (12), walking checks 10 and 43, hook and slice (16, 17), lucky bounce 42, club first use 54, shot kind remarks 55 to 57 and 60, pure strike 40. Tests: `tests/reactions_test.cpp`.

## 10. Port status, second pass (flight, club remark, hook and slice)

* Type 12 (3.1) is live: `ShotSim` tests each flight tick (placeholder 40 a second) for a ball at least 2 units up over a Woods or Building tile, redraws the band per theme, applies the `dist < rand(384)` test against the tile centre, and on the first hit per shot turns the remaining flight by 90 to 268 degrees and scales it by a random fraction. Hook: `ShotSim::obsCount/obsType`, `theme`.
* Type 54 (tier 3) is live with a PLACEHOLDER club index from the plan distance (0 driver to 12 wedge) and the per golfer mask. It runs before the 45/46 remarks.
* Types 16 and 17 (3.3 step 4) are live with a PLACEHOLDER curve: the sideways error of the port's shot, threshold 6 degrees, shots over 75 yards, bad lie, silent speaker.
* Still not produced: 6, 8, 32, 33 (aim search), 55 to 57 and 60 (no draw, fade, high or low shots in the port), 40 (pure strike needs the exe's curve scale), 42 (no bounce model), 10 and 43 (slope and penalty tables unknown), 9 on the tee.
