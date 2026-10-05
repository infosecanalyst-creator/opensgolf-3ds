# Golfer cards, pair screen, golfer generation (second pass)

Sources: decompiled text only (spec/golf_decomp.c). No binary was opened. Tags: EXACT (read directly), DERIVED (inferred, method given), UNKNOWN.
Caveats that apply everywhere:
- The dump has no data bytes. A string shows only where code references it by symbol, and long symbols are cut near 30 characters, so long sentences are paraphrased here and tagged with the cut.
- Strings reached through pointer tables are not visible, so those tables are UNKNOWN.
- Colours are 0x8000 plus RGB555 words (as in DECODE_COMMENTS.md). Text helpers: FUN_004049d0 left aligned, FUN_00404b70 centred, FUN_00404bc0 centred (second style). Which typeface each font object holds is UNKNOWN. Objects: 0x519928 (heading), 0x51b360 (body), 0x519fd8, 0x821020, 0x821f08, 0x821ee8.
- The callers of FUN_00459850 (pair screen), FUN_0045c560 (info card) and FUN_0040f190 (round start) sit in a region missing from the dump (about 0x4103xx to 0x421b5f). Their call context is UNKNOWN.

## 1. Skills dialog, FUN_0045f0f0
Signature: (title text, bit mask of skills already learnt, points to spend, golfer slot or -1, x offset). Returns the new mask (EXACT).
Modes (EXACT): points > 0 means editable (the player's own skill-spending dialog). Points = -1 means read only display of a golfer (called from the info card with x offset -50 and the slot). Offset < 0 uses the small panel.

Layout (EXACT numbers, x offset X = param):
- Panel frame via FUN_0040cef0: top 0x32 (50). If X < 0: x = X+0x4e, width 0xd0 (208). Else x = X+0x2e, width 0x140 (320). Height 0x13c (316).
- Title: heading font, centred at x = X+0xd2, y = 0x3a (58), colour 0x80007fff (white).
- If X >= 0: a sprite from the sheet at 0x4c1570 drawn at (X+0xf7, 0x56), a face via FUN_0045c200 at (X+0xff, 0x66) (a placeholder face id 0x99 when no golfer), and the OK button sprite 0x56a894 at (X+0x146, 0x13e).
- Ten rows, body font, row y starts 0x5a (90) and steps 0x18 (24). Skill names left aligned at x = X+0x8a, text y = row+7. Skill name colour 0x80004210 (grey) when the golfer has 0 points, otherwise 0x80004210 plus 0xffffbdf0 which is 0x80000000 after wrap (black) (DERIVED from the arithmetic: grey if none, black if any).
- Row background strip sprite 0x58baf8 at x = X+0x52. When editable, a minus/plus pair sprite 0x58bba8 at x = X+0x32. Hover: the cursor in x between X+0x32 and X+0x52 selects the top half (y within 12 pixels of the row top) as "add" (sprite 0x58bbd4) or the lower half as "remove" (sprite 0x58bc00) (EXACT).
- Value text: when a skill has points N, the text is a sign then N times 10 then a percent mark, left aligned at x = X+0x57, y = row+7. The sign string for 1 to 9 is DAT_004c5340 and for 10 or more is DAT_004e9a84. The proven use of DAT_004c5340 is "+" (money popup FUN_0040c910) so the value reads "+10%" up to "+90%" (DERIVED; matches the screenshot note "+50%"). DAT_004e9a84 is the empty string (DERIVED from its use as an empty filler in many calls), so 10 points would read "100%" with no sign. DAT_004c60c4 is the percent literal (DERIVED).
- The value source: slot = -1 or slot has flag byte at 0x57956e equal to 0 reads the working copy of the player's skill bytes (0x5a59fa+10+i); otherwise the golfer's own skill bytes at 0x5795a8 + i (EXACT). Only 10 skills are drawn; the player data holds 12, the last two (Positive Attitude, Mental Toughness) are never drawn here (EXACT loop bound).
- Skill names, in row order (EXACT, from the pointer table at 0x4c2c3c): Power Hitter, Long Driver, Accurate Driver, Accurate Irons, Accurate Putter, Draw Shot (R to L), Fade Shot (L to R), High Backspin Shot, Recovery Skills, Luck. (Order matches SCREENSHOT_NOTES. The full spellings of entries 6 and 7 come from the screenshots, not from the dump.)
- Points line when editable: number of points left, then the words "skill points" (EXACT, symbol s_skill_points_, the exact text is "skill points " with a trailing space per symbol cut; DERIVED), centred (style 2) at x = X+0xd2, y = 0x50 (80). Colour is 0x80007d08 (yellow) when points left > points used so far counter, else 0x80007d08 + 0x2f7 (changes hue; DERIVED from the masked add). 
- Limits (EXACT): max 9 points... precisely, a skill can be raised while its count is below 10 (so up to 10 points, shown 100%). Add needs free points and updates the learnt mask bit.
- Add (click top half): if points remain and skill < 10: skill +1, points left -1, spent counter +1, mask bit set. Refund (click bottom half): only if the skill is above 0 and above its starting value (the snapshot taken on entry), so previously committed points cannot be refunded (EXACT). Refund: skill -1, points left +1, spent -1; mask bit cleared when it reaches 0. Invalid clicks play sound id 0x18 (EXACT).
- Confirm: click within 20 pixels of (X+0x15e, 0x14e), the OK button. If points are still unspent a confirm box (400 x 200, modal, FUN_0046d6e0) appears. Text (paraphrase): "You haven't used all your skill points ..." with a button reading, cut at 30 chars, "Yea, I don't need no stinkin' sk..." (probably "skill points"). Yes returns the mask, no returns to editing (EXACT flow, the long sentence tail UNKNOWN).
- Cancel: there is no cancel button; refunds go back only to the entry snapshot (EXACT). In read only mode any click leaves.

Art for this dialog: TransPopups.pcx and its alpha (_A) loaded in FUN_00442180 (lines ~40517 to 40628); GolferStats.pcx is NOT this dialog, it is the info card sheet (section 2). The shadow s_ sheets are 50% darken masks: the card is blitted with the darken remap table DAT_00822c74 via FUN_004740f0 first, then the colour sprite on top (EXACT for the info card, DERIVED for the dialog).

## 2. Info card, FUN_0045c560 (layout S, single golfer)
Order of drawing (EXACT):
1. If the slot flag at 0x5794d0 is set, the name builder is run and the skills dialog (read only, X = -50) is shown first.
2. Backdrop: darken at (0xec, 0x1a) when the slot has no partner or partners are disabled (DAT_00824144 = 0), else plain draw at (0xec,0x1a) and the partner variant sprite 0x58b548 at y 0xa5; single variant sprite 0x58b51c at y 0x1a. Name plate sprite 0x58bb7c at (0xac, -2). Face via FUN_0045c200 at (0xac, -2).
3. Heading font text lines, all centred on x = 0x1a4 (420):
   - y 0x28 (40), colour 0x80000210: name line. Built as: prefix from table DAT_004c4974, then the full name from FUN_004676e0, then suffix from DAT_004c59e0 (words UNKNOWN). A third piece is the profile title string "Golf Pro"(EXACT literal at the profile table start, +0 of each record).
   - Body font. y 0x3e (62), colour 0x80000210 (style 1): marital phrase then age phrase. Marital words (EXACT, one of the four per flag bits 8, 0x10, 0x20, 0x40 of the 16-bit flag word at 0x579570): Single, Married, Divorced, Widowed. Then the separator (DAT_s___age, text starts with a comma and the word age, DERIVED from name) then the age from FUN_00453260. The pair screen uses a different phrase (section 3).
   - y 0x4b (75), colour 0x80000210, centred: trait line. Loops over 5 bits of the trait byte (profile +0x20, table at 0x4c2864, 5 entries); words joined with ", " (DAT_004c52b8 is the comma separator, DERIVED) and "and" (EXACT literal s_and) before the last. The 5 words are UNKNOWN (screenshot notes list five trait buttons in DECODE_CUSTOMISE.md which probably match, DERIVED).
   - y 0x5b (91), colour 0x80004210, second style: mood quote from FUN_00469a20 with the golfer's mood (word at 0x57955c) (section 2.3), wrapped in quote marks from DAT_004c4e54 (DERIVED).
   - y 0x69 (105), colour 0x80000210: the "likes" line (see 2.2).
4. Meters (font 0x519fd8). Label "Mood"-like heading from DAT_004d2120 (UNKNOWN literal) centred x 0x24a, y 0x2c. Five bars at x 0x222, width 0x50 (80), height 4, y = 0x36, 0x46, 0x56, 0x66, 0x76; labels centred x 0x24a at y 0x3c, 0x4c, 0x5c, 0x6c, labelled (EXACT): the first bar's label is the DAT_004d2120 string, then Attitude, Energy, Hunger, Thirst. Each bar: a full-width back strip then a coloured strip; strip colour is yellow-ish 0x80007d08 for values above 40 of 80 (the same strip recoloured by the masked add 0xffffa6e0, DERIVED) else red-ish 0x80006000 style. Values (EXACT formulas):
   - bar 1 from the mood word: (8 - mood) * 10 clamped 0..80.
   - Attitude: (4 - attitude) * 10 clamped, attitude from FUN_0045c420.
   - Energy: (word at 0x579562) / 4.
   - Hunger: (word at 0x57955e) * 5 / 2.
   - Thirst: (word at 0x579560) * 5 / 2.
5. Scorecard strip: nine columns, x starts 0x10a (266) step 0x12 (18) while below 0x24e. Hole numbers at y 0x84 black; strokes at y 0x94 (heading font), colour by strokes versus par (the hole par is read at stride 0x208 from 0x575cb8): black 0x80000000 when no par info; under par by exactly 1 or more: 0x80007d08, 0x80007ff0 (par minus 2 or better), over par: 0x8000211f (+1), 0x80004010 (+2 or worse); the current-hole entry uses grey 0x80004210. The total is drawn at x 0x256 (598), y 0x94. A status text built from "hole" and "shot" words is overwritten and never shown (EXACT: dead strings s_hole_, s_shot_). The 14 trait word table at 0x4c2960 is likewise built into a buffer that is then discarded (EXACT), so its words never appear on screen (UNKNOWN literals).

### 2.1 Age, FUN_00453260
Age in years derived from the golfer's record (birth stamp against the game date; the formula is as in docs/DECODE_GOLFERCARD.md). Exact constants UNKNOWN to this pass.

### 2.2 "likes" line (y 0x69)
Walks the 3 skill-preference bits of the byte at 0x5794d1 using the word table at 0x4c2858. First word is "length" (EXACT). The next two are not visible (probably "accuracy" and "imagination", from the existing course-type string "length, accuracy, and imagination", DERIVED). Words are separated by DAT_004c52b8. If byte 0x579572 = 1 a fixed phrase from DAT_004c5340 follows; if 2, phrase from DAT_004d3a08 follows (both UNKNOWN as literals; DAT_004c5340 is "+" elsewhere so this use is probably another string, UNKNOWN).

### 2.3 Mood quote, FUN_00469a20 (EXACT selection, texts are cut at 30 chars)
By mood level 0 to 7 (low is unhappy):
0 "I hate this stupid course." (truncated symbol, end may continue)
1 "I don't like this course much."
2 "I'm not having much fun today."
3 "This course is almost adequate."
4 "Well, I guess this course is OK" (cut)
5 "This course is quite nice."
6 "This is a really good course." when the second argument is below 2, otherwise "This is a really surprising cour..." (cut)
7 "This course is excellent." when argument 2 below 1, otherwise "This course is fascinating."
Default (mood out of 0 to 0x7f): "I hate this course, I'm leaving" (cut). Mood above range when argument 2 < 1: "This is best course I've ever pl..." (cut: played), else "... yet this is a most interesting c..." (cut). The rest of the sentences are UNKNOWN. FUN_00469a20 also has a second mode (variants via a switch) with shot remarks such as "How'd you like that shot", "Oh yeah, nothin' but DATA" (partner), "Whoa, I am so GOOD", "Oh yeah, I rock" (EXACT symbols, cut). These feed comment bubbles, see DECODE_COMMENTS.md.

### 2.4 Partner layout (P)
When the slot has a partner id (word at 0x579568 not -1) and partners are on, the backdrop uses the larger variant and a second column block is drawn for the partner by the same routines with offset; lines at lines ~61030 to 61620 of the dump repeat the name, marital, traits, quote and meters for the partner (DERIVED by structure, coordinates for the partner column not re-measured: UNKNOWN). FUN_0045de30 swaps two records, FUN_0045de80 sets two golfers up as a pair and picks a shared story using the compatibility mask of the theme's story file (EXACT).

## 3. Pair selection screen, FUN_00459850
Title text (EXACT, symbol): "SELECT THE NEXT PAIR OF GOLFERS" centred at (0x152, 14) using font 0x821020. Background from the sheet at 0x519cd4 (PairBase.pcx) drawn at (0,0).
Candidates: walks the ring starting from a head index; only slots with state byte 0x5794d9 = 0xff (waiting) are listed, up to 16, laid out two per row (DERIVED from index arithmetic):
- card cell pitch: x 0x14e (334), y 0x88 (136); column 0 at x 6, column 1 at x 0x14f+6 (second column offset 0x149 + 6 relative); first row y 0x32 (50).
- face sprite via sheet at 0x4c1570 at (x, y+4) (card base) then name text centred at (x+0xd6, y+9) (font 0x821f08, colour white 0x80000000 on dark card).
- font 0x821ee8 for the rest: title/pro text at (x+0x106, y+0x28), age phrase at (x+0x106, y+0x48), marital phrase at (x+0x106, y+0x68), all centred.
- Marital words: Single, Married, Divorced, Widowed (EXACT, same flag bits as the info card). Age phrase: the age number followed by "years old" (EXACT, symbol s_years_old, may carry a leading space).
- Trait lines: the trait byte's set bits (up to 5) are listed left aligned at x+0x92, first at y + (5 - count)*9 + 0x24, step 0x12 (EXACT), words from the 5 entry table at 0x4c2864 (literals UNKNOWN).
Selection: clicking a card (hit test x - 6 divided by 0x14e plus 2 times (y - 0x32) divided by 0x88) toggles a selection bit. Two chosen cards confirm: both golfers get flag 0x20000 set at 0x5794c8 and are paired (FUN_0045de80). Invalid clicks play sound 0x18 (EXACT). If the player picks nothing the screen does not auto choose in this routine (EXACT), any automatic choice is UNKNOWN.
A second screen in the same routine shows tournament results: headings "TOURNAMENT SCORES" (while running) or "TOURNAMENT RESULTS", then columns "Ranking" at (0x19, 0x32), and "Prize" at (700, 0x32), rows of the first 6 or more players (EXACT).

### 3.1 Arrival and waiting model
- Slot creation: FUN_00421bc0 picks the next ring slot (counter % 152), zeroes the 0x100 byte record, sets world position from the course entry point, and creates a golfer in waiting state 0xff (EXACT). It tries up to 999 profile picks and gives up with the message "Your membership is declining..." (cut, EXACT start) and returns -1.
- The golfer waits in state 0xff in the main update loop (lines ~19386 to 19470). The wait length and the trigger that sends a waiting golfer to the pair screen are UNKNOWN because the callers are in the missing region.

## 4. Profile generation (FUN_00421bc0)
Facts (EXACT unless marked):
- Profile index: random 1..75 (call to the random helper FUN_0045c1e0 with limit 0x4b, plus 1) from the table at 0x4d6088 (record 0x230: title, name, trait byte, flags, head). FUN_0046c940 decides if the slot is a "member" case (result 0 gives skill-pref 6).
- Skill preference byte (0x5794d1): 1 by default; bits 0x11 in trait byte give 3; bits 0xc add 4; if still 1 it is 4 or 5 by slot parity; a profile field at record +0x14 (0x4d60b4) overrides with its low 3 bits.
- Rejection rules: the same profile index (or the same index mod 19) must not already be in use by another golfer; a profile whose head slot is unusable (byte -1) is rejected; unless the caller is a special case, the profile's availability flag must be set. Up to 999 tries.
- Flags word at 0x579570: random 15-bit value from FUN_0045c1e0(0x7fff) first (marital and age bits), then replaced on success by the profile's second flag byte plus a random value 0..127 shifted left 8.
- Mood word starts at 3 plus random 0..2 (so 3 to 5); forced to 4 if option DAT_00822c88 is zero; overridden to 4 + DAT_00543cc4 when set.
- Profile class 0x579572 = profile index mod 3. State = 0xff (waiting). Counter 0x5794d5 = 0xb.
- Names come from the profile record at +0x10 (the data files progolfers.dta and celebrities.dta, see their format comments) and the name builder FUN_004676e0 (prefix/suffix tables DAT_004c4974 and DAT_004c59e0, contents UNKNOWN). Skill bytes for visitors: not set here, UNKNOWN where. The .glf/.chr files were deliberately not parsed.
- Visitor type and tier probabilities (membership counts, the 8-entry table at 0x59dea0 chosen by slot & 3) are used to pick the lowest used skill preference group first: it counts uses per group and prefers the least used (EXACT), then a random draw picks among ties.

## 5. Course info art
courseinfo.pcx and s_courseinfo.pcx are loaded in FUN_00442180 (lines ~40657 to 40671). No drawing routine that uses them was found (UNKNOWN; probably in the missing region).

## 6. Art cuts
- GolferStats.pcx / s_GolferStats.pcx: the info card sheet. Cut by FUN_00473bf0 into 0x2c byte sprite records at 0x58b51c (single card), 0x58b548 (partner card), 0x58bb7c (name plate), 0x58baf8, 0x58bba8, 0x58bbd4, 0x58bc00 (skill row strip, +/- pair, add hover, remove hover). Exact pixel rectangles were not re-measured in this pass (UNKNOWN; see tools/sgview.cpp drawGolferCard for the earlier measured values).
- s_GolferStats.pcx is a shadow mask: drawn first with the 50% darken remap table, then the colour sprite.
- PairBase / PairButtons: lines ~43808 to 43810; not measured (UNKNOWN).

## 7. Consolidated UNKNOWN
5-trait words; 14-word table (dead); 2nd/3rd likes words; DAT_004d3a08, DAT_004d2120; name prefix and suffix tables; DAT_004c5340 non-"+" use; font typefaces; callers and triggers of pair screen and info card; automatic pairing and waiting time; visitor arrival timing and visitor skill population; partner column coordinates; sprite rectangles; any courseinfo draw site; tails of long quotes.
