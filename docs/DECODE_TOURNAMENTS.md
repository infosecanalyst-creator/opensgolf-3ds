# Tournament play (decoded from the publisher exe)

Source: the publisher's unprotected golf.exe (read only for facts, notes are in my own words), plus progolfers.dta and celebrities.dta from the disc and the tournament result art. Confidence is marked as exact (read directly), medium (structure clear, meaning inferred) or guess. Code: `include/sg/tournament.h`, `src/tournament.cpp`. Addresses are virtual addresses in the exe.

## 1. Flags and state

All in the global game flag word at 0x59e7b8.

| bit | meaning |
|---|---|
| 0x400000 | an SGA tournament offer is pending (the panel's tournament button is shown) |
| 0x200000 | a tournament is running (set when the field is drawn, cleared by the results screen or by cancelling) |
| 0x1000000 | sandbox mode (no offers) |
| 0x4000000 | "championship play": a course saved with "Save Course for Championship" is loaded as a stand-alone tournament |
| 0x2000 | a pro has challenged the player to a match (button "vs. a pro") |
| 0x4000 | request to start a practice round or match (set by the panel buttons) |

Other globals: 0x5a59f8 is the slot of a pending match (-1 = none), 0x567b04 is the first prize in thousands, 0x569498 is a "screen to show next" byte (9 = SGA evaluation screen, 2 = silent evaluation), 0x5685f0 is the open hole count PLUS ONE (it is the number of the next hole to build; every "holes" use below says H for the real count), 0x5787cc is the fame counter, 0x571fd4 is cash in units of $100, 0x5a6d3c is the year index (display year 2001 + it), 0x5a34e0 is the theme (0 Parkland, 1 Desert, 2 Tropical, 3 Links, medium).

## 2. Offer, acceptance, cancel (exact unless noted)

* July offer: in the daily handler at 0x417f8a, when `ticks % 8192 == 4096`, no match pending (0x5a59f8 == -1) and flags 0x1200000 clear, the SGA evaluation (0x44fb30, mode 2, silent) runs. A non-zero prize shows the message "The SGA is interested in holding a tournament at your course." plus a sound, sets 0x400000 and zeroes the prize counter. A zero prize clears 0x400000, so an unanswered offer is withdrawn the next July if the course no longer qualifies. There is no other expiry.
* The tournament panel button appears only when 0x400000 is set (and neither 0x200000 nor 0x4000000). Its tooltip is "Begin Tournament". The other two buttons are "Practice Round" (needs 2 or more holes) and "vs. a pro" (needs flag 0x2000 and a selected challenger).
* Pressing it runs the evaluation again from scratch (prize counter cleared, 0x44fb30 mode 2), so the prize can differ from the July figure. If the prize is now zero it shows the SGA Evaluation screen instead. Otherwise it shows a dialog: "The SGA offers to hold the <tournament name> at your course with a first prize of <n>,000. Open tournament" with two answers, "Great, let the games begin." and "I think I need more practice." Choosing the second only zeroes the prize counter; 0x400000 stays set and the button can be pressed again.
* Choosing "Great..." (function 0x46c970 does the work):
  * snapshots the character table and a second table so they can be restored afterwards;
  * sets 0x200000;
  * stamps accomplishments (see section 5): id 3 always, id 8 if prize >= 500, id 13 if prize >= 1000, with a snapshot position at hole 1's tee;
  * if the skill points were changed since the last check, the "tournament preparation" skill warning code runs first (0x4065c0, not decoded).
* "Cancel match/tournament" (menu item, handler at 0x41c6fe): with a tournament running it asks "Are you sure you want to CANCEL this tournament..." with " Yes, cancel." and " No, keep playing."; yes clears 0x200000 AND 0x400000 (the offer is lost), restores the saved tables, zeroes the prize and sets the pending slot to -1. No payout. With only a match pending it asks "Are you sure you want to CANCEL this match..."; yes clears the match flag (0x200) of each golfer, shows "Match canceled.", fixes the game speed mode and clears the pending slot.
* "Save Course for Championship" (handler 0x41ca98): with a tournament running it shows "Cannot save course during a tournament." with a sound. Otherwise it asks for a name, writes `Themes\Championship\<name>.cse` (a file-exists dialog offers Overwrite or Cancel) and shows "<name> saved for championship play."
* Championship play from the main menu (0x46ddd0, flag 0x4000000): loads a saved course, sets cash to 1000 units, ticks to 0x2c00 (year index 1, month 3) and starts the same field draw at once. In that mode the accomplishment stamping (0x46e7b0) does nothing and the strength score of the field draw is randomised.

## 3. The field (exact for rules, medium for what some fields mean)

* A tournament is ONE round: every golfer plays each open hole once. Rounds = 1. The shotgun start puts two golfers on each open hole: slots 2k and 2k+1 start on hole k+1 (up to 18 pairs). The field is therefore 2H golfers (36 on 18 holes, 18 on 9 holes). Slot 1 is the player's character (own skills, 12 of them), slot 0 is the player's pair partner. The pair partner of slot s is slot s xor 1.
* The golfers are real simulated golfers. Their strokes come from the normal shot simulation, hole by hole, written into a per slot record (record size 0x100, base 0x5794b8): byte +0x21 is the hole being played (0 once finished), strokes for hole h are at +0x23+h. The port does not simulate yet, so `Tournament` takes strokes from the caller; `placeholderStrokes` exists only for demos.
* Pros come from the 100 record table filled from `progolfers.dta` (and then, medium confidence, from any saved character `*.pro` files in the theme folder, appended). Record 0 is never drawn. The record also holds the sum of the ten skills (a short at +0x36).
* The draw, per slot (exact): pick `idx = rand(100)`; skip it if the record is empty, if idx is 0, or if it was used already (the used test stops applying after 800 valid tries, tries are counted cumulatively over the whole slot). Compute `x = (skillSum - 10)^2 / ((difficulty*5 + 20) * 2)`. For slot 0, x is reduced by `difficulty * x / 6`. In championship play x is replaced by a random value below it. Accept only if `x <= tries/(4 - difficulty) + prize/10 + cash/200` (not too strong) and `x >= prize/10 - tries + cash/200` (not too weak); for slot 0 the `tries` in the lower edge is divided by 4. Here prize is the raw first prize in thousands and cash is in units of $100. So bigger prizes pull in stronger pros and the window loosens as tries accumulate, which also guarantees an end.
* The 2H-1 pros are all different unless 800 tries were spent.
* Odd side effect: strength is symmetric around a skill sum of 10, so hackers (sum under 10) count as "strong" like top pros; this is how the numbers read, not a bug in the port.

### progolfers.dta format (exact)

Text file, CRLF. Lines starting with `*` are comments, lines of 9 characters or fewer are skipped. Each record: `name,body,skin,hat,shirt,pants,SSSSSSSSSS` then usually spaces and a loose number.

* body: first character, `% 8`. 0 long sleeves, 1 knickers, 2 short sleeves, 3 short pants, 4 female long sleeves and pants, 5 female short sleeves and shorts, 6 female short sleeves and pants, 7 female tank top and skirt. Bit 2 set means female (the exe sets a different type id for female).
* skin `% 4` (0 caucasian, 1 asian/tanned, 2 latino/very tanned, 3 black). hat, shirt, pants `% 10` (colour tables are in the file header).
* skills: ten characters, `0`-`9` then `A`.. = 10 and up (only `A` to `F` appear, so 0 to 15). Order: Power Hitter, Long Driver, Accurate Driver, Accurate Irons, Accurate Putter, Draw Shot, Fade Shot, High Backspin Shot, Recovery Skills, Luck. The player has two more, Positive Attitude and Mental Toughness (pros get 0 there).
* The trailing loose number (30 to 115) is ignored by the exe; it rises with the skill sum and looks like an author's rating note.
* The table is capped at 100 records. The shipped Standard file gives 96 records (index 0 Joe Pro to 95 Tiger Forest). Roughly the first 12 are hackers with one dominant skill, then pros with two or three dominant skills, then the real field ordered by that note.
* One line (`Brad Fiction`) has a stray blank field. The exe reads positionally and ends with a wrong pants value and skill characters taken from stale buffer text. `parseProGolfers` drops the blank field (my choice, not the exe's) and warns.
* File lookup: `Themes\<current theme>\progolfers.dta` first, then `Themes\Standard\progolfers.dta`; if neither exists the exe prints " - file not found."

### celebrities.dta (exact format, medium use)

`name,type,skin,hair,shirt,pants`; type letters A to K (action star, female pop star, politician, male comedian, supermodel, fitness female, female comedian, leading man, female movie star, rock and roller, athlete); hair 0 grey, 1 blonde, 2 red, 3 brown, 4 black. 21 records in the shipped file, table capped at 100 (stride 0x25 at 0x55d758). They are a separate table from the pros: the tournament field draw never reads them. Strings show their use: celebrities buy vacation homes ("has purchased a vacation home at your golf course! Golfers enjoy seeing celebrities as they play"), and there is a "Celebrity" visitor kind. Challenges and matches use the PRO table ("Famous golfer ... challenges you to a match").

## 4. Scores, standings, ties, payout (exact)

Handled every frame while 0x200000 is set by 0x45a090 (the main loop calls it when no modal flag is up).

* Per slot score = sum over open holes of (strokes - par), counting a hole only when strokes are recorded, the hole's par byte is non-zero, and the golfer is not currently playing that hole. Slots of closed pairs stay out. Any golfer still on the course makes the screen the small live leader board.
* Place: scores are scanned from -100 up to +100 and, inside one score, slots in ascending order; each golfer takes the next place number. Places are therefore unique, never shared. A tie is broken by slot number: slot 0 (the player's partner) beats slot 1 (the player) on equal scores, and the player beats every slot of 2 and up.
* When nobody is left on the course the big results screen is drawn once and the tournament ends. Paid places are 1..H (place number below H+1). The prize starts at the first prize (a counter of 0 is replaced by H*20) and is multiplied by 2/3 with integer division after every paid place (for example 300, 200, 133, 88).
* Payout, only for slot 1 and only if the player's place is paid (all of this is skipped otherwise, including the history record):
  * history ring record `0xE0 | place` at the current 1024-tick block (0x40c6f0, 500 entries; the text strings are "Won match vs." and " in tournament." with " places ");
  * cash += prize * 1000 / 100, that is prize * 10 units of $100; the same amount is added to a short at +0x0e of the current year's finance record (table at 0x584210, 0x14 bytes per year, year index modulo 100; looks like tournament winnings on the Financial Report, medium);
  * fame (0x5787cc) += 4 - place for places 1 to 3, else += 1;
  * if place is 1: accomplishment 11 if H >= 9; and if H >= 18 accomplishment 15, and accomplishment 17 + theme when the first prize is above 100000 (thousand), a value the prize formula (at most 1140 on an 18 hole course) can never reach, so the Grand Slam victory trophy looks unobtainable through this path; recorded as is).
* Then: ends the tournament by clearing 0x200000, restoring the saved tables, removing the temporary objects whose kind is 0x11 or 0x12 (what they are is not decoded), recomputing every hole's par class from its length, zeroing the prize. The player does not have to click anything for the money; the OK button only closes the screen.

## 5. Accomplishment stamps (the "landmark" side effect)

0x46e7b0(id, x, y) records the tick when trophy id was first achieved (once only) and queues a snapshot named `snapshots\accomp<id>`; it is skipped in championship play. The trophy list order from the exe text is: 0 1st Challenge hole, 1 1st Heroic hole, 2 1st skill upgrade, 3 1st Tournament, 4 1st Strategic hole, 5 First match victory, 6 First 9+ hole course, 7 1st Top 100 hole, 8 1st $500,000 Tournament, 9 1st Classic hole, 10 1st Top 18 hole, 11 First tournament victory (9+ holes), 12 1st Grand Slam course (9+ holes), 13 1st $1,000,000 Tournament, 14 First 18 hole course, 15 First tournament victory (18 hole), 16 1st Grand Slam course (18 hole), 17 to 20 Grand Slam Victory (Parkland, Desert, Tropical, Links), 21 1st 100 star rating. The tournament code uses ids 3, 8, 13 (opening), 11, 15, 17+theme (winning). These are accomplishment photos, not spawned landmark objects; no scenery is created by the tournament code that I found (the kind 0x11 and 0x12 objects removed at the end are the only trace of tournament props, medium).

## 6. Screens (layout from the code, 800 x 600)

Art: `Interface/infoscreens/tournament result.pcx` and `tournament result_alpha.pcx`, both 800 x 600, 8 bit. They are cut into six full width bands: header (y 0, h 106), paid row (y 180, h 26), cut line row (y 125, h 22), unpaid row (y 224, h 18), banner A (y 357, h 51), banner B (y 274, h 51).

Final results screen (all done):
* header band at (0,0); title "TOURNAMENT RESULTS" centred at (320, 16) (the code can also pick "TOURNAMENT SCORES" while the player is still out, which cannot happen on this path); column headings at y 50: "Ranking" at x 25, hole numbers 1..H at x = 175 + 27*(h-1), "F" at x 667, "Prize" at x 700;
* rows start at y 77. Row art: places below H+1 use the paid band drawn at y-7, place H+1 the cut line band at y-8, later places the unpaid band at y-4. Row pitch is read from the sprite structs at run time (not read, probably the band heights);
* each row: place number and golfer name at x 25 (the player's row has its own colour 0x80001284, others 0x80000000 for paid and 0x80004210 grey for unpaid), the strokes for each hole in the hole columns (colour by par: normal at par, red 0x6000 over, blue 0x18 under in 565), the score to par at x 669 ("E" for even, "+n" over, "-n" under, same colours), and for paid places the prize text "<n>,000" right aligned at x 780 in the player colour;
* at most 18 rows are drawn and drawing stops below y 548;
* after the rows, banner B at y-4 (or banner A at y-7 when exactly 18 rows were drawn and H+1 == 19), then a click sound (0x38) and an OK button at x 732 under the banner; the loop waits for the button.

Live leader board (while golfers are out): a 144 wide panel at the top left (x 8, height 22*(H+1)+16), title lines "LEADER BOARD of" at (72, 9) and a second line at (72, 21) built from the course name and " Open" with the prize and ",000" at (72, 33); in championship play "LEADER BOARD of the" plus one of three championship names chosen by difficulty ("SGA Jr. Championship" and two more strings, mapping not verified). Rows start at y 45 with an 11 pixel pitch, up to 36 rows: rank, name and score to par ("E", "+n", "-n"), player's row in its own colour.

## 7. Matches and practice rounds (medium)

* Practice Round and match both start through 0x40f190: two free golfer slots are paired (partner links crossed, both start on hole 1), the player's slot gets the match flag 0x200 and becomes the pending slot (0x5a59f8). With no opponent chosen it is a practice round (advisor line "I'm ready for a practice round."; for a tournament "I'm ready to play a tournament."). With a challenger the opponent is built from the pro table record chosen in 0x4c2e14.
* A pro offers a match when, in a periodic check, a pro's skill sum (of 12 bytes, medium) lies within roughly plus or minus (records scanned / 4) of 5 times a player level counter (0x59b730). The flag 0x2000 is set and the challenger index stored. The text: "<Famous golfer> challenges you to a match at your course with a wager of <n> per hole and ...", per hole results ("wins hole #", "leads", "tied"), and at the end "you win" or "you lose", "Pay" or "Collect" and the score. Trophy 5 is the first match victory. Cancelling is described above. The wager size and the payout formula are not decoded.

## 8. What the port implements

* `parseProGolfers`, `loadProGolfers`, `parseCelebrities`, `loadCelebrities`: exact format, see section 3.
* `julyOfferDue`: exact.
* `Tournament::evaluateOffer`, `reopenOffer`, `decline`, `accept`, `cancel`: exact flow, prize and name from `sga.h`.
* `selectField`: exact rule, but the random generator is the caller's and slot to character-table bookkeeping is skipped.
* `rankEntrants`, `prizeLadder`, `Tournament::standings`, `finish`: exact.
* Placeholders: `placeholderStrokes`; clamping a negative formula prize to "no offer" (the exe only treats exactly zero as no offer, totals under 25 give zero or negative values); the screen pitch values; the grand slam victory id, kept as the exe has it.

Open questions: what the tournament preparation warning checks, objects 0x11 and 0x12, the match wager formula, the mapping of difficulty to championship names, whether `*.pro` files really append to the pro table.
