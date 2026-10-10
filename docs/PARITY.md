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
| Popup menus | Information (12 items) and System Functions (8 items) on the InfoButtons 9-slice frame, plus Preferences and rename prompt. EXACT: the frame, fill, text colours and offsets, radio balls, checkboxes and OK tick (0x46d6e0, DECODE_MENUS 1), the list box 0x46de70, the retirement question's rules. PLACEHOLDER: Preferences labels beyond three. Load Game saves a While Browsing file but Cancel does not yet restore it |
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
| Player panel (JoeCoolPanel, dock modes 3 and 4) | EXACT | crates/simgolf/src/player_panel.rs (dock panel 5). Hit zones, draw table (0x4c7ac0), cut states (hover, selected, disabled, idle at x 0/50/100/150, ovals 0/100/200/300), enable conditions, skill list (494,540), the round scorecard over the cover cut (0x461110) and tooltips read from the exe. Port choices: the shot ovals' hits follow the aim (exe: any mode 3), each oval uses its own alpha cell (the exe's two alpha cells are near identical), Customise opens on the pro's slot (exe: slot 0x98); the panel has no cancel button, N twice still cancels |
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
| Name tags over golfers and staff | EXACT font and place, PLACEHOLDER toggle | 0x462be0: Arial Bold 10 at zoom 4 (Manual SSi 15 above), centred with the text's top at the golfer's point, no shadow; the Shift+N toggle is mine |
| Text sizes and placement | EXACT | The exe's font size is GDI's em height in pixels (jgl.dll builds a LOGFONT with lfHeight = -size); a text call's y is the top, the baseline sitting tmAscent - tmInternalLeading below it (TA_BASELINE TextOut), widths in whole pixels per glyph. ui::Fnt carries face and size for each font object (F_ARIAL10, F_MANUAL15, F_MANUAL20, F_KLEPTO18/24, F_MANUAL24, the info set F_INFO_TITLE/F_INFO20/16/14) and Screen::put* place text by its top. Converted: HUD, ticker, thoughts and advisor bubbles, names, floating money, tooltip bars, the Amenities info box, golfer card, pair screen, top 10, the info screens' titles and bodies (report, hole stats, best scores, comments, histograph, finance, shortcuts, SGA, results, leaderboard, year end, land, hire), the title menu, difficulty, load and theme screens and the credits |
| Ticker strip and dialog frame fills | EXACT | 0x40ca10 darkens 16 x 16 tiles through s_TransPopups masks (the frame's 3 x 3 rounded cuts by tile place, the strip's square at (317, 0)) with colour table 0x824148, which halves each channel; the rail pieces are TransPopups (300/317/334, 34), cut by 0x4466b1 |
| Tooltip bar | EXACT | 0x432620: Arial Bold 10, three pixels per character either side of the point (kept on screen), a black line 10 thick at half opacity (Terrain::drawLine's last argument is the alpha in tenths), white text centred with its top 5 above the point; the golfer card's button tips use it at the pointer |
| Terrain button tooltip | EXACT, DERIVED text file match | 0x433190: the translucent frame (x - 80, 402, 160 x 112) with its TransPopups tab strip, the name, " $" and the cost in Manual SSi 15, the lie's mood face, the description from Interface/<theme>.txt (the line after "*Name", matched without case: DERIVED) wrapped at 128 in Arial Bold 10, the bounce and roll meters with the GBUBBLES ball; the exe's "Lie:" words are built and never drawn. The amenity strip's box is still a PLACEHOLDER |
| Speech bubbles (thoughts, advisor) | EXACT, DERIVED tail | Text top at zoom * 10 above the person, Arial Bold 10 at zoom 4 (Manual SSi 15 above), a 10 pixel half black line the text's width plus 8, the tail cut (512, 140) of course1.pcx at (x + 4, y + 10) through the halving table; the tail's shape (its darker pixels) is DERIVED |
| Flower beds under rotation | EXACT | 0x41266b rotates the neighbour mask right one place per quarter turn (0x5685f4 / 2) before the piece table, so pieces follow the screen; the single and four-sided pieces keep view (a - 2b) & 3 whatever the camera |
| Landmarks on the course | EXACT | 0x463180 case 4: sprite 0x168 + type in palette 100 + type, view (facing + rot / 2) & 3, type 3 turning by the full eighth count |
| Weedy flower bed palette | PLACEHOLDER | The exe loads palette 0xbb only from flics\bldgs\flowers\...IckyPal, which the disc does not have, so its load fails; the same named files under Flics/Flowers are used |
| Sprite shadows | EXACT order | 0x43d740 composites NameShadow.flc under Name.flc into the same frames at load, so shadows are drawn with their sprite in depth order (a nearer sprite's shadow falls over a farther one), not in a pass of their own |

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

Soak check (update 78): an AddressSanitizer plus UBSan build ran headless through long hooked sessions (3000 simulated seconds on a new property with a match, practice round, tournament reveal, tutorial page, skills dialog) with no reports. Leak detection was off and no real mouse input was exercised.

## Update 79 additions

| Item | Status | Notes |
|---|---|---|
| Starting land | EXACT | Replaces the tract guess: the owned area is a centred square of tiles [k, 49-k] with k from the acres byte (FUN_00470a60: smallest k >= 1 with 4*(25-k)^2 <= acres*10); coastal leaves the y-low side open, islands are fully owned, sandbox owns tiles 1..48. Ownership is per tile; buying a tract takes the whole tract. Tract price and acres shown use the remaining unowned tile count. Which screen side is "y-low" for coastal is a weak reading. Saves write LAND 2 (k, lie); old saves keep their tract mask. Notes in DECODE_LAND.md |
| Voice emotion banks | EXACT ids, DERIVED shadowing | Type 1 plays Happy (Success when upset), types 2, 3, 8 play Sad (when upset the exe's Failure slots are shadowed by HARD/EASY/TRICKY/BLIND voices or the boing, played here). Male PLS, KLS, SSS use the exe's "PLS/MKLS/SSS ... mix" files; male PLS Sad is silent (the exe asks for a file that is not on the disc). "Upset" (exe flag 0x20000) is still a stand-in: the last reaction was bad |
| Types 12 and 13 | DERIVED | mWATER0 for men; women get the tree leaves effect (the exe's female slot is shadowed); 500 ms delay |
| Voice positioning | EXACT formulas, GUESSED units | Golfers off an 800 x 500 screen area are silent; pan = sx*127/800 - 64; volume byte = (|sx-400|>>4)+50 (the zoom subtraction is not applied, the port's zoom is another scale); pitch jitter 300 - rand(600) treated as cents; gain = volume/127*1.5. New Mixer::playAt (pan, pitch, delay) checked with a synthetic clip |
| Golfer card skills panel | EXACT rule | Drawn automatically for pros and the player's golfer, never for ordinary golfers; the S key toggle is gone (DECODE_CARDS3.md) |
| Golfer card Customize | weak reading | Live only on the player's own golfer, opens the character editor; View Story stays pale (its screen is not in the decompile) |

Not found in the decompile: the info card click handlers, the story screen, the Load panel class words. Tested headless only (start renders on two properties, a tract purchase, card render, synthetic mixer test, sanitizer soak); not run on the Mac.

## Update 80 fixes

| Item | Status | Notes |
|---|---|---|
| Dock hover overlay | FIXED | The hover sprites on 3mainLowerLeft.pcx carry their own hard edged shadow, which was drawn over the shadow already baked into the dock, and the sprite was placed by its centre, a few pixels off. Hover now draws only the pixels that differ from the normal sprite (new ui::loadPcxHoverDiff) at the exact spot the normal sprite sits on the baked dock (found by matching the sheet cut against the dock art, kDockPos). Buttons 4 (zoom out) and 9 (tools) have no exact baked match, so they keep the whole sprite at the old position |

Checked headless on all dock buttons; the panel tab and player panel hovers were not changed and not re-verified with real mouse input.

Tool (update 80): `sgsprites <game dir> <output dir>` exports every Flics sprite to a PNG sheet (row per view, column per frame, shadows as _shadow). Palette variants are not applied.
