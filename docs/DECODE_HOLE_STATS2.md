# Hole Stats dialog, rating words, roster handicap and Top 100 / Top 18 (publisher exe, second pass)

Source: the publisher-supplied golf.exe only (existing Ghidra decompile plus a direct read of the string table). All facts are in our own words; short quoted fragments only identify strings.
Tags: EXACT = read straight from code or data. DERIVED = follows from code but one link is inferred. UNKNOWN = not found.
Companion to docs/DECODE_HOLE_STATS.md (record layout, reaction deltas, report maths) and docs/UI_SCREENS.md section 3 (pixel layout). Routine addresses: Hole Stats dialog 0x453330, word helper for differentials 0x4532a0, signed hundredths formatter 0x42dd50, comment sentence builder 0x469b00, hole name builder 0x407280, background analysis 0x42dea0, round end block inside 0x427380.

## 0. Corrections to earlier docs

1. The rotating stat row is driven by a clock, not chosen at random when the dialog opens (section 2). EXACT.
2. The Length / Accuracy / Imagination words use a different scale from the Fun Factor words (section 1.3). EXACT.
3. Only the first rotating label ("Avg. Drive: ") carries a colon in the exe string table; the other four labels have none. EXACT.
4. The title shows the "(N)" tail only when the hole has the Top 100 flag or a pending naming flag; otherwise it reads "Hole N" (section 1.1). EXACT.
5. The Top 18 flag alone never gives a hole a proper name (section 6.3). EXACT.

## 1. The dialog rows

All numbers are pixel positions of the draw calls; "left" and "centre" are the two text helpers (0x476650 left aligned, 0x476700 centred on x). Text colour is black (0x80000000 in the exe's 5-5-5 plus flag word) except comment rows.

### 1.1 Title (EXACT)
Centred at (385, 48): the fixed words "HOLE STATS for " then the hole name from 0x407280, then, only when `flags & 0x81` is non zero (Top 100 or naming pending), the text " (" + hole number + ")".
Hole name rule (0x407280): if `flags & 0x81` is 0 the name is the word "Hole " plus the number. Otherwise: the player's own stored name for that hole if one exists (lookup 0x45b9f0, by hole), else a default name from a table of 18 chosen by the hole's par: par 3 uses a table of women's first names (Alexandra, Belinda, Carmen, ... Zelda), par 4 uses a table of shrub and flower names (Olive, Dogwood, Peach, ... Holly), any other par (2, 5, 6) uses a table of dark or hellish names (Pride, Inferno, Fortress ... Purgatory). Tables are indexed by hole number 1..18.

### 1.2 Row list (EXACT positions)
Left column, label left at x 190, value centred at x 356:
- y 83 "Fun Factor". Value drawn only when shot plans (record +0x24) is not 0.
- y 104 "Length", y 125 "Accuracy", y 146 "Imagination": always drawn.
Right column, label left at x 441:
- y 83 "Yards": value = record short +4, plain integer, centred at x 536. Always drawn.
- y 104: the rotating stat (section 2), value centred at x 591. Drawn only when the hole has at least one counted round.
- y 125 label "Par " (string has a trailing space): value = par byte, plain integer, centred at x 536. Always drawn (a closed hole shows 0).
- y 146 "Stroke average": value centred at x 591, drawn only when at least one round falls in the six visible histogram columns (section 1.5).
Other texts: "Average shots on this hole" left at (190,176); "Comments" left at (190,202).

### 1.3 Fun Factor value (EXACT)
fun percent = fun total (short +0x158) * 100 / (shotPlans / 2 + 4 + rounds), C integer division, can be negative or above 100.
Text = the integer, then the two characters "%-", then a bracketed word with no leading space:
- below 0: poor
- 0 to 19: fair
- 20 to 39: good
- 40 to 59: very good
- 60 and above: outstanding
(Oddity: the string between number and word really is a percent sign followed by a hyphen; how the font draws it is UNKNOWN, it is probably a plain hyphen so a row reads like "45%-(good)".)

### 1.4 Length, Accuracy, Imagination (EXACT)
What they show: a signed difference in strokes, shown with two decimals, followed by a word. Not a bar, not a pure word scale.
Computation (histogram bins limited to the six visible columns, section 1.5; groups are the 8 golfer skill masks):
- For each group g: count N[g] = 8 + (counts in the visible bins), strokes S[g] = 8 * par + sum(bin * count over visible bins). avg100[g] = S[g] * 100 / N[g] (integer division). The 8 phantom par rounds mean no group is empty.
- Length = avg100[6] - avg100[7]; Accuracy = avg100[5] - avg100[7]; Imagination = avg100[3] - avg100[7]. (Group 7 has all skills; 6 lacks length, 5 lacks accuracy, 3 lacks imagination.)
Number format (0x42dd50, value v in hundredths): the sign character is "+" for v >= 0 and "-" for v < 0, then |v| / 100, a dot, then |v| % 100 with a leading zero added when below 10. Examples: +0.35, -1.20, +0.07.
Word (0x4532a0, appended after the number with a leading space), by v in hundredths:
- v < 0: poor
- 0 to 24: fair
- 25 to 49: good
- 50 to 99: very good
- 100 and above: outstanding
Resulting row shape: "+0.57 (very good)". Rows are drawn whether or not the hole is open (an unopened hole gives "+0.00 (fair)").
Note these thresholds (25, 50, 100) line up with the demand cut off T = 25 on difficulty 0 or 50 otherwise, and the "classic needs all three at 100" rule from the report; the words are a presentation of the same numbers.

### 1.5 Histogram (EXACT)
Six columns, centre x = 445 + 34 k for k 0..5. First stroke value f = max(par - 2, 1). Column k covers stroke value f + k; the last column (k = 5) covers f + 5 up to 9 and its label gets a plus sign appended ("7+"). Label drawn at y 168; the count (sum of the 8 groups' bins in that column, no phantom rounds) at y 188, only when non zero. Bins 0 and 10 are never read.
Stroke average row (x 591, y 146) = (sum of stroke * count over the visible columns only) * 100 / (count over visible columns), shown with two decimals and no sign for positive values (the formatter gets an empty prefix, so only negatives would carry "-", which cannot occur). So holes in one, or very high scores outside the six columns, do not enter this row. EXACT.

## 2. The rotating stat row (EXACT)

Index = (timeGetTime() >> 10) % 5, i.e. it advances every 1.024 seconds by itself while the dialog is open; the dialog loop redraws everything whenever the index changes and exits on a click. (If the hole has no rounds the modulus is 3 but nothing is drawn in this row.) No click cycling.
Order and formats (value follows the label, value centred at x 591):
| idx | label (exact string) | value |
|---|---|---|
| 0 | "Avg. Drive: " | driveSum (+0x15c) / rounds (+0x20), integer, then " yds" |
| 1 | "Longest Drive" | longest drive (+0x166) then " yds" |
| 2 | "Fairways hit" | fairways (+0x15e) * 100 / rounds, then "%" |
| 3 | "Greens in Reg" | greens (+0x160) * 100 / R, then "%" |
| 4 | "Average Putts" | putts (+0x162) * 100 / R, two decimals, no sign |
R = rounds minus the number of golfers currently on this hole who have taken at least one stroke (scan of the 144 golfer slots), but never below 1. Indices 0 to 2 divide by the plain rounds counter. The row and its label are skipped entirely when rounds is 0.
Label positions: the label left at x 441, y 104 (the same y for all five).

## 3. Hole type names (EXACT)

There are no separate flag bits that choose the name; the name comes from a 3 bit mask of demanded skills, table indexed by mask (strings in the exe, one table of 8 pointers at 0x4c2a28):
mask 0 Breather, 1 Freeway, 2 Precise, 3 Challenge, 4 Creative, 5 Heroic, 6 Strategic, 7 Classic.
Bits: 1 length, 2 accuracy, 4 imagination. Each hole has exactly one name, so "several apply" is just the combined mask. Mask build (identical in the report 0x44fb30 and the background analysis 0x42dea0): a bit is set when its differential is at least T (T = 25 on difficulty 0, 50 otherwise). Then the lowest of the three differentials is found (start value 100; length is the candidate only if below 100; accuracy and imagination replace the candidate only when strictly lower); if that lowest value is below the drop limit its bit is cleared. Drop limit: report = 100 always; background analysis = 50 on difficulty 0, 100 otherwise. The hole flag bits 0x100, 0x200, 0x400 ("strong" demand) are set when a differential is at least 50 and are not used for names.
Where names appear: the Type column of the Course Report (the table above), and a one-time popup. The popup (0x42dea0 calls 0x460df0 then a yes/no box) fires for a non zero mask the first time that mask value is seen in the whole game (bit per mask kept in a global bitmask at 0x5685f8; it is set only if the player accepts). Each popup text is a short blurb starting with the type name in capitals and quotes: Heroic (rewards length and imagination), Creative (imagination), Challenge (length and accuracy), Precise (accuracy), Freeway (rewards length, hit it far), Strategic (accuracy and imagination, length less important), Classic (maximum skill: length, accuracy, imagination); Breather (mask 0) never pops up. The hole type is NOT drawn on the Hole Stats dialog itself. Trophy latches for these names: see docs/EXE_HOLESTATS.md.

## 4. Comment lines area (EXACT unless marked)

- Rows: at most 5 comment rows, then the OK button below the last row. Each row is a 16 px strip blitted at (0, y) with the text centred at x 400. First row y = the dialog's running y after the top art (taken as 220 in the port). The y of each next row adds the strip height (16).
- Qualifying events: copy the 64 event counts (+0xd8); five times in a row pick the event type with the highest count among types 0..49 only (types 50..63, including the driving range comments 51..53, never appear); a strictly greater test means the lowest type number wins ties; a type is used only if its count is above 0; once used its copy is zeroed. Nothing is drawn if rounds is 0.
- Percentage: count * 100 / rounds (integer division; may exceed 100 since one golfer can raise the same comment several times).
- Row text: percent number, then "%" and three spaces, then an apostrophe, the sentence built by 0x469b00 from (type, last stored location & 0x3fff, hole), then a closing apostrophe. (Order of the pieces is DERIVED from the append order; the strings are EXACT.) The builder is called in its "hole stats" mode (the fourth argument 0x98), which makes it skip the golfer specific name lookups.
- If the hole is closed (par 0), the first row is "Under Construction!" in red 0x80007d08, drawn only while `(timeGetTime() & 0x200) != 0`. Since the dialog only redraws once per 1.024 s the blink is effectively a phase sample (DERIVED).
- Colour rule (EXACT mechanism, DERIVED per type table): the sentence builder sets a global (0x58b198) to one of three colour words: grey 0x80006318 by default, bright green 0x800023e8, red 0x80007d08, depending on the event type it formats. The dialog maps it: green 0x800023e8 is drawn as the darker green 0x80001284; red stays red 0x80007d08; anything else (grey) is drawn as plain black 0x80000000. So colour belongs to the event type, not to the sign of the stored fun delta.
  Type to colour from the builder's switch (DERIVED by scanning every case, types 0..49): green for 1, 6, 7, 11, 18, 22, 25, 27, 28, 29, 32, 33, 34, 39, 44, 46; red for 2, 3, 4, 8, 9, 10, 12, 13, 14, 15, 19, 20, 21, 23, 24, 30, 35, 36, 43, 47; no colour assignment found (so black) for 0, 5, 16, 17, 26, 31, 37, 38, 40, 41, 42, 45, 48, 49. The green set matches the positive base deltas in DECODE_HOLE_STATS.md section 3 and the red set the negative ones; 18, 25, 27 and 39 and 7 are the conditional positives. Some neutral types may set a colour in a branch the scan classed as none: UNKNOWN for those.

## 5. Roster handicap, low score, rounds (EXACT)

Per member record (0x2c bytes at 0x5849e0, indexed by the golfer's roster id): byte +0 Low, signed byte +1 Hcp, short +0x2a Rounds.
Rounds: incremented by 1 for each of the two golfers of a pairing when the pairing is created (tee off, routine 0x45de80), only when the golfer's kind byte is 0 (ordinary visitor or member). It is a count of rounds started, not finished. EXACT.
Round end (block in 0x427380, runs after the finishing putt of a hole when game flag 0x200000 of 0x59e7b8 is clear and the hole just finished is hole 18 or the next hole is closed (par 0); the roster id comes from the golfer record):
- Walk holes 1..18 (all 18 always, closed holes included). For a hole beyond the last hole the golfer played: adjusted total += 5 and over-par sum += 1. For a played hole with strokes s and par p: raw total += s; adjusted total += s; over-par sum += s - p.
- Low (byte +0): if the adjusted total is below the stored Low or Low is 0, Low = adjusted total. So Low is an 18 hole equivalent where every unplayed hole counts 5 (a 9 hole course adds 45). Stored as a byte.
- Hcp (byte +1): new = over-par sum (unplayed holes count +1 over par each). If the old Hcp byte is 0 it is replaced by new; otherwise Hcp = (old + new) / 2 with C truncating division on a signed byte. So it is a running average over par of recent rounds that halves the history at every round, not a best-N differential.
Display (roster columns, and the golfer info panel lines for low round, handicap, rounds played): Low shows "-" when 0 else the number; Hcp shows "-" when 0 or negative else the number with no sign; Rnds shows "-" when 0 else the number. Status text from `flags & 7`: Visitor, Member, Silver Member, Gold Member, Platinum Member (info panel), Resigned text when the resigned marker is set.
UNKNOWN: whether pros, staff and celebrities own a roster record (the code does not test the kind byte in this block); whether the adjusted-total byte can overflow for very long bad rounds (it is 18 holes at most 9 strokes each is under 255 normally, but no clamp was seen).

## 6. Top 100 / Top 18 flags (hole flag word, bits 0 and 1)

### 6.1 Where
Only one writer for each bit: the background analysis 0x42dea0 (holes 1..18 in order). It has a single caller (call site 0x41967e, inside the main loop function 0x40f5c0, in the code that draws the course rating panel; the same pass refreshes the waiting list counter at 0x56d1b0 and the fun and demand sums, so it effectively runs every time that panel is drawn, which the homes doc already treats as every frame). EXACT call site, DERIVED frequency.

### 6.2 Absolute thresholds, no ranking (EXACT)
Nothing compares holes with each other: there is no sorted list, no limit of 100 or 18 awards and no per course quota. A hole is awarded when its own numbers pass:
- score = length + accuracy + imagination differentials (hundredths of a stroke, the report's group formula without the flag 0x40 extras). On difficulty below 2 the score is raised to at least (fun percent * 6 / 2) on difficulty 0 or (fun percent * 6 / 3) on difficulty 1, where fun percent is the same per hole figure with divisor shotPlans / 2 + 4 + rounds (0 when no rounds).
- Top 100: revenue (+0x1f4) above 200 and score above 200 and `flags & 0xd` is 0 (not already Top 100, not too hard, not too easy). Then the naming flag 0x80 is set and a yes/no box ("... rated as one of the best 100 holes in the country by" the Golf Enquirer, per hole) asks the player. Yes: set bit 0x01, log history event 0x80 | hole, play sound 0x2e, spawn celebration object type 7 at the green. No: clear 0x80; the test is repeated on later passes.
- Top 18: revenue above 400 and score above 300 and `flags & 0xe` is 0 (not already Top 18, not too hard, not too easy; Top 100 may or may not be set). Box naming the Great Golf Holes magazine. Yes: set bit 0x02, log event 0xa0 | hole, sound 0x2f, celebration object type 10. No: nothing set, retried later.
- Neither bit is cleared by the analysis. Hole reopening (0x40e720) resets revenue to 0 but its flag writes do not clear bits 0 or 1. A full record copy from the edit scratch record was not checked: UNKNOWN. The "too hard" / "too easy" bits (0x4, 0x8) are recomputed each pass, so an awarded hole is not revoked when it later becomes too hard.

### 6.3 Consumers (EXACT)
Fee per finished hole: +2 for bit 0, +2 for bit 1 (also the home lot value path, see DECODE_HOMES.md). Name: bit 0 or the pending bit 0x80 switches the display name from "Hole N" to a proper name (section 1.1); bit 1 alone does not. The course report draws a marker icon when the hole is scenic or has bit 0 (decode doc section 4) and also the report row logic reads bit 0 and bit 1 together (a small 0, 1 or 2 count used for the star style at report row 47156 in the decompile: 0 none, 1 Top 100 only, 2 both or Top 18 with Top 100). DERIVED for that last point.

## 7. Still unknown
- Exact setters of hole flags 0x20, 0x40, 0x1000, 0x2000 (unchanged).
- Whether the "%-" in the fun row renders as a hyphen.
- Per type colour of the 14 types with no colour assignment (section 4) and the true first comment row y (220 assumed).
- Pro, staff, celebrity roster records and the flag clear on a full hole record copy.
- Why the analysis bumps the fee scratch word at 0x4c2850 by 1 on each award (it is overwritten at the next finished hole, so it has no visible effect).

## 8. Port status
Implemented: Fun Factor word row (shown only with shot plans, "%-" printed literally), Length/Accuracy/Imagination as signed hundredths plus word over the visible columns with 8 phantom par rounds, rotating stat row on a 1.024 s clock (with the on-hole correction for the divisor), "Par " and "Stroke average" rules, histogram label rows, title with the "(N)" tail and own-made default hole names (the exe's 54 names are not copied; own lists of 18 per class), the five comment rows, roster Low/Hcp/Rnds rules (rounds count when a pairing is created; Low and Hcp from `roundEnd`, unplayed holes of 18 count 5 strokes and 1 over par, so a short course inflates both exactly as the exe does), and the Top 100 / Top 18 analysis with a Yes/No popup (flags feed hole fees and home lot value).
Placeholders: re-ask delay after a declined award (60 s), award sound and popup art, the "(N)" naming uses no player typed names, hole type popup and the closed-hole blink phase use wall clock.
Tests: `tests/stats_test.cpp`.
