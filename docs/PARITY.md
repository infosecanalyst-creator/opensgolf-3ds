# Parity plan: how "1:1" is checked and what blocks a release

The goal is an OpenTTD style release: a faithful reimplementation that loads the owner's own disc data. Fidelity is only real if it is checked against the original in several independent ways, not only against what has been decoded so far.

## Four checks

1. **Asset coverage (automatic).** `docs/PARITY_AUDIT.md` lists every disc file the exe references that the port never names. Re-run the audit script after each batch of work; the list must shrink. Pattern loaded sets (terrain tiles, FLC animations) need a manual look, so the numbers are an upper bound on gaps, not an exact count.
2. **Screen and action inventory (from the exe).** Every dialog, key handler, button and menu action in the decompile gets a row below with status: done, placeholder, missing. A screen counts as done only when it uses the original art and the original rules.
3. **Placeholder burn down.** Every approximation in code is tagged `PLACEHOLDER`. The count (91 on 2026-10-04) goes down only; a release needs 0, or each remaining one listed here with the reason the exe value cannot be recovered.
4. **Side by side with the real game.** Reference screenshots and play footage of the original, compared screen by screen, and the owner playing and reporting. This is the only check that catches errors in the decoding itself.

## Known gaps found by the first audit (2026-10-04)

The pre-game character page the port used to show does not exist in the original (new game goes straight from the property pick into play; the Customise screen opens from the Golfers dock tab). It was removed.

| Area | Gap | Notes |
|---|---|---|
| Character | DONE (first pass): Customise screen on the original art, face picker, .pro load and save, advisor portrait. Still open: preview bodies (palette swaps), exact trait and toggle label texts, hit centres from the exe tables, stock default sayings, Save portrait | CustGolfBckgrnd, CGButtons, HeadSelect, HeadBodyBck; spec in DECODE_CUSTOMISE.md |
| Faces | Advisor popup, Customise and the golfer info card done. Open: stats card, pair selection, Pick a Pro, remark popups for golfers, member heads (exe table unknown, port uses a PLACEHOLDER rule) | spec in DECODE_FACES.md |
| Pick a Pro | Championship flow, course picker and Pick A Pro screen built (headless only). Left panel done (portrait, ten skill rows, signature saying; stock saying is a PLACEHOLDER) | Title_Pickapro.pcx, .pro files in Themes/Championship |
| Pair selection | Not built (spec DECODE_TOP10_PAIR.md section 3; the exe's call site is unknown and the port has no waiting queue yet) | PairBase, PairButtons |
| Golfer card | DONE (first pass, single layout): plate, ball and face, five meters, 18 hole scorecard, five round buttons with hover art and tooltip. Open: partner layout (needs group play), shaded backdrop and s_GolferStats shadow, stats card (skills dialog), exact text lines (exe literals unknown, port lines are PLACEHOLDER), Customize and View Story buttons (drawn pale, no action) | GolferStats; spec DECODE_GOLFERCARD.md |
| Course info | courseinfo art unused | courseinfo, s_courseinfo |
| Top 10 | DONE: record file, score and insert rules (tested), trophy screen on the real art, submitted at end of year and shown if the club made the table. Open: Top10_Charms and TrophyMantle (the exe never references them), key F11 is a PLACEHOLDER for the Information menu | Top10_Trophies, Top10_Blank |
| Low score screen | lowscore.pcx unused | |
| Title | DONE (first pass): difficulty select, theme packs and Load Previous Game on the real art. Open: Load side panel (course info, thumbnail), delete yes/no box, scroll thumb drag, theme pack contents are not used by the game yet, multi-slot save names | Title_LoadGame, Title_ThemePacks, TitleSelDiff |
| Voices | DONE (first pass): clips play per reaction type and gender (docs/DECODE_VOICES.md). Open: pan, pitch jitter, start delays, emotion banks for types 1, 2, 3, 8, type 12 and 13 clip choice, staff lines, id to file map is rebuilt from meaning | SimsFX |
| Bodies | 32 of 42 body sprite files unused | golfer and employee clothing variants and palette swaps |
| Credits | DONE (first pass): scrolling text over the background, logo marker. Opened by clicking the title logo (PLACEHOLDER, the exe's trigger is unknown) | credits.txt, creditsbckgrd |
| Mood face bug | FIXED 2026-10-04 (reversed order) | compact dock layout still open |

## Order of work

1. Character customise screen and faces (visible, owner reported).
2. Voices for reactions (reaction engine already knows the type).
3. Pick a Pro, pair selection, golfer card, course info, Top 10, low score.
4. Title screen remainder, credits.
5. Placeholder burn down, starting with the terrain penalty table and aim search.

## Rules for every change (so a gap is never silent)

* Before building any screen or feature, check what the exe draws and plays for it: list the art files, sounds and animations it references (grep the decompile for the screen's routine) and use them. A screen built from flat boxes is a placeholder and must be tagged `PLACEHOLDER` with the art it should use.
* After each batch run `python3 tools/parity_audit.py GAME_DIR spec/golf_decomp.c` and compare the gap count with the last commit; it must not rise, and the finished screen's files must leave the list.
* Update the gap table above in the same commit.

## Update 67 additions (headless only, never run on the Mac)
| Area | State |
|---|---|
| HUD | Top bar uses the original courseinfo art and shade sheet |
| Best Scores | Module, test, recording, persistence and screen done; trigger keys F12 (Best), F11 (Top 10), Shift+K are PLACEHOLDERS until menus fully replace them |
| Popup menus | Information (12 items) and System Functions (8 items) on the InfoButtons 9-slice frame, plus Preferences and rename prompt. PLACEHOLDER: radio ball sprite, text colours, Preferences labels beyond three. Load Game saves a While Browsing file but Cancel does not yet restore it |
| Open | Load screen emblem and tooltips, pair selection trigger, stats card, golfer bodies and palettes, play-core constants from DECODE_PLAYCORE.md, world facts from DECODE_WORLD2.md |
Audit count: 982 gaps (down from 987).

## Update 68 additions (headless only, never run on the Mac)
| Area | State |
|---|---|
| Pick A Pro | Left panel on the real art |
| Load screen | Club emblem (parklink/tropdesert cuts) replaces the thumbnail; Cancel/Delete tooltips; Loading... text; delete yes/no box in the popup frame; Cancel from System Load restores the While Browsing save. PLACEHOLDER: OK tooltip word, second delete option wording |
| Pair selection | Screen built from PairBase/PairButtons; trigger key backquote is a PLACEHOLDER (exe call site unknown); trait labels and ages are stand-ins; pairing only announced |
| Not done | Golfer stats card (skills dialog), golfer bodies and palettes, play-core constant swaps, world data facts |

## Update 69 additions (headless only, never run on the Mac)
| Area | State |
|---|---|
| Surface table | include/sg/lie.h holds the exe's 23 surface records (bounce, friction, penalty, costs, walk effort, class) read from golf.exe's data section; the port's tile ids map onto them (editor-only tiles are PLACEHOLDER picks) |
| Ball flight | include/sg/ballphys.h runs the exe's per-tick flight, bounce (rebound = -64 + vertical * k / 12, rolls under 0x80), ground friction, water stop and tree hits. Shots now pick the launch range by previewing the roll, as the exe does. PLACEHOLDER: slope term, edge friction, curve of draw and fade, tick rate in seconds |
| Ball washer | Golfers whose tee has a washer within 3 tiles lose a third of their direction error, cleared when the ball stops off the fairway (EXACT rule) |
| Heiress | Exact landmark draw, separate available mask (strip lists only available designs) and free mask; price fixed to 50 + 10 * kind units; donation message names design, value, placement and effect. PLACEHOLDER: starting available set (0x000f; sandbox all) |
| Save dialog | System menu Save Game opens the exe's box (54,80,628x80, 48 character field, default name course + day + month + year, trim and character check, overwrite confirm) |
| Skills panel | Read only skills panel from the golfer card (S key is a PLACEHOLDER trigger); panel art is the popup frame, row strips unmeasured |
| Golfer colours | Palette composer (Swap01..10) recolours in-world golfers by shirt, pants, skin, hat, alternate skin; members use their progolfers.dta row. PLACEHOLDER: female hair, default skin for non members. Customise preview bodies not done |
| Tests | ballphys, shot and visitors tests added (10 in all) |
| Not done | Customise preview bodies and the stats card art, tutorial triggers, SGA offer popup art, employee Move and Rename, multi slot saves |

## Update 70 additions (headless only, never run on the Mac)
| Area | State |
|---|---|
| Club emblem | Property to emblem table (parklink and tropdesert cuts, DERIVED from the art and your real HUD screenshots: Las Vegas roulette, Ireland shield, Hawaii flamingo agree) replaces the old prop/8 guess; Load panel and in-game HUD both use it |
| HUD emblem | Drawn at the left of the course pill via new ui::drawImageScaled; 76 px size and position are PLACEHOLDER (measured by eye from screenshots) |
| Terrain generator | Terrain::generate from the site record: theme scatter mix, coastal shore on one side, island ring on four, ponds inland, relief amplitude from terrain kind and price slot (96 / base, PLACEHOLDER reading of the exe's 0x30/0x20/0x10 base height), clubhouse at a random cell of the 17 x 17 centre window with a cleared lot. The exe's stroke tables are not decoded, so the land is generated, not the original's |
| Starting land | The tract holding the clubhouse first, then nearest tracts (PLACEHOLDER count and order) |
| Ugly landmarks | As many as the difficulty, kind 16 parkland, 18 desert, 17 tropical and links, on open rough ground; their pictures (Radio Tower, Red Oil Pump, TarPit, Railroad Tracks) are PLACEHOLDER picks; golfers that glance at one get the ugly view reaction |
| Skills dialog | Editable dialog for your own character at X = 200 (frame, name, "N skill points", ten rows with add/refund toggles, value badges, cream name boxes, portrait, OK, unspent confirm); rules per DECODE_CARDS2 (cap 10, refund only to the entry value). Opens at a normal new game with 10 minus spent points; badge, toggle and name box shapes are PLACEHOLDER because the row sprites are not located; skills do not change shot accuracy yet |
| Test hooks | --prop N, --skills |
| Tests | terrain_gen added (11 in all) |
| Not done | 18-dot rating row, Paused label, Customise preview bodies, stats card art, tutorial triggers, SGA offer popup art, employee Move and Rename, Information menu styling |

## Update 71 additions (headless only, never run on the Mac)
| Area | State |
|---|---|
| Skills dialog | Rebuilt to match the real screenshot: black panel with a lavender edge, yellow "Add N skill points." line, oval toggles, cream "+N0%" ovals and name boxes, standing portrait (Bodies/<set>.pcx recoloured with the swap palette, head on top at the Customise offsets) on a white oval, explanation box under the dialog. Shapes and colours measured by eye (PLACEHOLDER) |
| Skill points | Closing the dialog sets the player's tournament skills (point x 1.5, cap 15); the exe's real use of each skill in the player's shots is still the shot model's |
| HUD rating row | Golf ball dots, stars (course grade + 1) and hearts from StarsHeartsETC.pcx, spaced over 146 px; flags per hole up to six, otherwise a flag and "x N". The dot count (1.5 per hole) is PLACEHOLDER; the hearts counter is 0 until happy endings are tracked |
| HUD text | Course name and date moved to the real rows; money green (red when negative), fun yellow, skill cyan, as in the screenshots; "Paused" label at the top centre (position DERIVED) |
| Notices | `say` messages use the dark green translucent panel with lavender edge; SGA offers show the trophy plate (geometry PLACEHOLDER) |
| Menus | Information and System menus restyled (slate panel, heading row, teal items, amber ball bullets, no OK button) and the heading row now sits inside the exe's (n*3+3)*8 height |
| Employees | Per-employee names (Rename Employee prompt, 31 characters), hire date, Move (armed, next map click places the employee), Fire removes that employee; info box text is dark (PLACEHOLDER colour); Paid is wage x months employed (PLACEHOLDER) |
| Shot Analysis | Analyze Golf Shot tool and the / key: 5 sample first shots for ALL skills, no Imagination, no Accuracy, no Length drawn on the hole, with the panel and a yards figure (DERIVED: closer to the green than the all-skills average; the exe's own routine is not decoded) |
| Bug fix | Body palette loader used a 96 byte path buffer, so a long install path silently disabled recoloured golfers; now a std::string |
| Test hooks | --empsel N, --say TEXT, --analyze N |
| Not done | Customise preview bodies, stats card art, tutorial triggers, employee info colours, per-employee counters and wages, happy ending hearts, Practice Round button |

## Update 72 additions

| Area | State | Notes |
|------|-------|-------|
| Course emblems (HUD, 16 sites) | DERIVED | Sheet/cut table checked against three real screenshots |
| World map status dots, globe pins, legend | DERIVED | Pin positions from the site table match the real map |
| Font capital I glyph | EXACT quirk | Mapped to the l glyph |
| Customise preview stack (body, head, two walkers) | DERIVED | Offsets measured by eye, PLACEHOLDER |
| Story selection from Themes/Standard and More_Stories | DERIVED | First pair forced to OpeningDay; later pairs by seed and count |
| Story happy ending | EXACT table, DERIVED trigger | Full pass of the story lines ends it: hearts +1, highlight 0x120, landmark donated by first letter (DECODE_WORLD2 1.5). The chapter timing is a PLACEHOLDER |
| clang -Wall pass | checked | 3 harmless warnings, no portability errors in sgview.cpp |

Not yet run on the Mac. Headless checks only.

## Update 73 fixes

| Area | State | Notes |
|------|-------|-------|
| Colour-key fringe | fixed | Keyed art loses blended key-colour edge pixels, and transparent pixels take the nearest opaque colour so filtering cannot pull pink in |
| Skills dialog OK button | fixed | Drawn through a circle mask (the art is a round button on a square lavender backdrop) |
| Starting land | PLACEHOLDER | Tracts owned at start = clamp((acres + 10) / 15, 4, 9); the exe's real starting ownership is still not decoded |

## Update 74 additions

| Area | State | Notes |
|------|-------|-------|
| Player panel (JoeCoolPanel, dock modes 3 and 4) | DERIVED | Opens from the player tab of the Golfers panel; hit zones, tooltips and button table are EXACT (ui_panels.h). Skill list, scorecard dst (297,366) and the hover and disabled cut columns (50, 100) are PLACEHOLDER |
| Practice Round | DERIVED | Needs two open holes and no player out; the player's character walks the course with its own skills and body, pays no fee, is not recorded in the books and never quits. Play (vs a pro) and Begin Tournament only show a notice |
| Shot shape ovals | DERIVED | Selection is stored and drawn; it does not change shots yet |
| Hole open message | PLACEHOLDER wording | Odd and even hole texts and the dogleg side follow DECODE_WORLD2 5.4 |
| Building advice | DERIVED | Advisor suggests the newest unlocked building that is not built yet |
| Unowned land | DERIVED | Drawn black as in the real game |
| Course Report | fixed | The keyed gap under the Total row is lavender; legend labels moved down |
| Title screen | fixed | Logo blob no longer cut; Start New Game on two lines; Theme line only for non-standard packs |

Not run on the Mac. Practice Round clicks were not exercised, only the hooked headless render.

## Update 75 additions

| Area | State | Notes |
|------|-------|-------|
| Shot shapes | PLACEHOLDER numbers | The player's chosen oval now bends (fade, draw), tightens (backspin) or shortens (punch) the player's own shots; reliability follows the matching skill |
| Match against a pro | DERIVED | A pro within a quarter of the table size of the player's skill sum may challenge each month (one in three); Play starts player and pro on the course, holes are compared when both finish, the winner collects or pays the lead times the wager. Wager, odds and window are PLACEHOLDER; "First match victory" is awarded |
| Cancel Match | DERIVED | System menu item works while a match is on |
| Begin Tournament button | EXACT enable rule | Lit while an SGA offer is pending; opens the SGA response screen |
| Per-employee counters | DERIVED | Each employee keeps own greeted, hurried or sold count in the info box; a loaded game restarts the individual counts |
| Tutorial | structure EXACT, wording my own, start key PLACEHOLDER | 11 fun and 9 skill pages, any key advances, Escape stops, game pauses; Shift+F8 starts it |

Tested headless only (a hooked match run to a win, the tutorial page render); not run on the Mac, no button clicks exercised.

## Update 76 additions

| Item | Status | Notes |
|---|---|---|
| Refusal, chip, TaDa, Twinkle, bagpipe, end-of-year, tournament place sounds | DERIVED | Wired to the shipped wav files; trigger points are my reading |
| Emotion voice clips | weak reading | Bank-to-reaction mapping guessed from file names |
| Bottom golfer mood strip | PLACEHOLDER position | Round faces from MemberPanel.pcx with number cells; click opens the golfer card |
| Name tags over golfers and staff | PLACEHOLDER | Seen in real screenshots; font, offset and Shift+N toggle are mine |

Tested headless only; not run on the Mac.

## Update 77 additions

| Item | Status | Notes |
|---|---|---|
| Tournament reveal | PLACEHOLDER | After accepting, the results screen fills in hole by hole (0.7 s per hole) as a running leaderboard, then shows the final results; any key or click skips. The rounds are still computed up front, so this is a presentation of them, not live play on the course |
| Multi-slot saves | already present | Named save files and the Load screen list were already in place; the old gap entry was stale |

Tested headless only (a 7-hole reveal render); not run on the Mac.

Note (update 77): the pair-selection screen builder FUN_0044bde0 has no caller and no address reference anywhere in spec/golf_decomp.c, so its real trigger cannot be recovered from the decompile. The backquote key stays a PLACEHOLDER until another source shows it.

## Update 78 additions

| Item | Status | Notes |
|---|---|---|
| Golfer remarks in the world | DERIVED | Each accepted reaction floats its comment sentence (from the existing comment generator, coloured by polarity) above the golfer for 5 seconds; the position, size, duration and fade are my choices from the real screenshots. Shares the Shift+N toggle with name tags |
