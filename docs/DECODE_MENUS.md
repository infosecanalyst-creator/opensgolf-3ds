# DECODE_MENUS: in-game menus, popups, hotkeys, call sites

Clean-room notes. Tags: EXACT (read directly), DERIVED (inferred from structure), UNKNOWN.
Function names are decompiler labels (FUN_xxxxxxxx).

## 1. Generic popup FUN_0046d6e0 (EXACT)

Signature: (centreX, topY, initialMask, mode, disabledMask). One text blob, split on newlines.
- Lines starting with a space are selectable options; text drawn at box x + 0x24, 24 px row pitch.
- Other lines are centred headings.
- Width = longest line + 0x31 px. Height = (lines*3+3)*8.
- Mode 1: menu/radio. Up/Down arrows (0x26/0x28) move, Enter (0xd) or Space (0x20) accept, Esc (0x1b) returns -1, mouse click returns the index. An OK ball at bottom-right also accepts.
- Mode 0: checkbox list, returns a bit mask.
- Disabled options are greyed; choosing one plays error sound 0x18.
- Return is the option index counted from the first option line.

Users: Information menu, System menu, Preferences, demolish confirm (FUN_0040a4e0, yes/no, 400x100), overwrite confirm (FUN_00437910, 200,0x1e), delete confirm in Load/Pick (400,100), tournament prep list FUN_0046d200 (400,100, 20 checkboxes, mask 0xfffff, skipped when flag 0x4000000), skill waiver confirm in the stats card (400,200).

## 2. Toolbar (bottom left) (EXACT unless noted)

Handlers: click FUN_00432720, hit test FUN_004326a0, draw plus tooltip FUN_00432ba0, tooltip bubble FUN_00432620. None have decompiled callers; the undecompiled main loop FUN_0040f5c0 calls them (UNKNOWN internals).

Eleven items, base sprite at (0, 435). Hit: nearest item by weighted distance (dist*(i+6)/8 < 30). Item positions come from a data table (UNKNOWN values). Tooltip appears after 11 stable frames.

| # | Tooltip caption | Action |
|---|---|---|
| 0 | Build Course | toggle side tab 0 (terrain) |
| 1 | Add Buildings | toggle tab 1 |
| 2 | People / Employees (separator DERIVED) | toggle tab 2 |
| 3 | Information | open Information popup (not when screen mode 2) |
| 4 | Pause or Unpause (varies with pause flag 4) | returns key P (shift+p) |
| 5 | System Functions | open System popup |
| 6 | Repeat last message | returns ? |
| 7 | Zoom Map | returns z |
| 8 | Unzoom Map | returns x |
| 9 | Rotate Map | returns synthetic code for Home (-0x24) |
| 10 | Rotate Map | returns synthetic code for PageUp (-0x21) |

Clicking the active tab closes the panel (value 5). Tab panel art: mode 0 at (7,436) with marker (34,509); mode 1 at (84,463) with (95,525); modes 2-4 at (146,504) with (142,555). Zoom-limit indicator sprites and a blinking pause sprite are drawn from data-table positions (UNKNOWN positions).

Synthetic key codes returned to the main loop: uppercase letter means shift+letter; negative numbers are minus the Windows VK code (F1 -112, F2 -113, F3 -114, F4 -115, F7 -118, F9 -120, F10 -121). The main-loop consumer is UNKNOWN (prior notes cite key tables near 0x4219b0 and handler 0x41def5).

## 3. Information menu (FUN_0046d6e0(200,0xfa,1,1,0)) (EXACT mapping, caption order from SCREENSHOT_NOTES, header text UNKNOWN)

| Idx | Caption | Result |
|---|---|---|
| 0 | Repeat Last Message | ? |
| 1 | Course Report | F1 |
| 2 | Player Comments | F2 |
| 3 | Routing Map | R |
| 4 | Histogram | F3 |
| 5 | SGA Evaluation | F7 |
| 6 | Financial Report | F4 |
| 7 | Membership Roster | F9 |
| 8 | Professional Accomplishments | F10 |
| 9 | World Map | W |
| 10 | Best Scores | calls FUN_00455a30 directly, returns -1 |
| 11 | Top 10 Designers | calls FUN_00473470 directly, returns -1 |

Cancel (Esc) returns nothing. Course info items are only reachable when screen mode is not 2.

## 4. System Functions menu (FUN_0046d6e0(0xfa,0x154,1,1,0)) (8 options; labels partly UNKNOWN)

Order DERIVED from case layout; the index-to-target byte table is not in the text.
- Save the current game: returns S.
- Load a previous game: returns L.
- Cancel match or tournament: returns C (handler 0x41c6fe).
- Save <player 0 name> for Championship: FUN_00437910(0,1,1) writes Themes\Championship\<name>.pro, then ticker message "<name> saved for championship play." Menu reshown on failure.
- Rename Your Course: text prompt "Rename Course..." (FUN_0045b2c0), max 0x20 chars, validated by FUN_00405ac0, error sound 0x18 on failure.
- Preferences: FUN_00432560.
- Save Course for Championship: returns n (handler 0x41ca98).
- Quit: sets DAT_0059b734 = 1, returns Esc. Reader of that flag not in decompiled text (UNKNOWN).

Save dialog FUN_00405b10: heading "SAVE GAME, edit name then press Enter", confirmation "Game Saved". Autosave files begin with '&'.

## 5. Preferences (FUN_00432560) (EXACT bits, labels mostly UNKNOWN)

Checkbox popup (400,200), mask in DAT_005a5a00, title "Preferences...", six entries. Only first label readable: "Display golfer names on screen".
- 1 golfer names (name labels).
- 2 tied to member/pair logic (UNKNOWN meaning).
- 4 advisor and first-time messages.
- 8 a message with sound 0x38 (UNKNOWN meaning).
- 0x10 no reader found.
- 0x20 ambient animals.

## 6. Play panel (FUN_00436060 click, FUN_004362f0 draw) (EXACT)

Buttons: 0 customise golfer (FUN_004385d0), 1 Practice Round (needs 2+ holes, no match pending), 2 Play / vs. a pro, 3 Begin Tournament (returns J; needs flag 0x400000, not 0x4200000), 4-8 shot types (Straight, Fade L to R, Draw R to L, High backspin, Low punch), 9 Golfers tab, 10 Hire employee. Tooltips: Practice Round, Play, "vs. a pro", Begin Tournament, Golfers, Hire employee.

## 7. Hotkeys (keyboard help screen FUN_0044e770, EXACT as listed; dispatcher UNKNOWN)

F1 Course status report; F2 Player comments; F3 Histograph; F4 Financial report; F5 Course overview map; F6 World map; F7 SGA evaluation; F8 this help screen; F9 Membership roster; F10 Professional accomplishments; ? Repeat last message; shift+B sell building lot; shift+R routing/aura/lot values; shift+W world map / change courses; Tab rotate before placing; Esc quit; g green tees; f fairway; r rough; s sand; t trees; w water; p path; b benches; z zoom; x unzoom; shift+P pause; shift+T trees off/on; shift+N toggle names; e elevation mode; / instant shot analysis.

Speed control UI: none found beyond pause (UNKNOWN). Camera: zoom z/x and rotate codes; scroll keys UNKNOWN.

## 8. Title screen (FUN_0043cd70 loader) (partly UNKNOWN)

Seven hover regions (x,y,w,h): (42,21,324x121), (416,31,363x141), (510,359,271x111), (287,474,288x115), (13,382,287x115), (168,198,472x176), (721,528,58x57). By icon art (DERIVED): top-left Continue Saved Game (FUN_0043b610), top-right Start New Game (difficulty FUN_0043a400), right Select A Theme (FUN_004725b0), bottom-centre Play a Championship (FUN_0046ddd0), left Sandbox, centre logo (possibly credits FUN_0044b9c0, UNKNOWN), bottom-right Exit. Dispatch order and quit path live in FUN_0040f5c0 (decompile failed; UNKNOWN). FUN_0040f5c0 sole caller is startup FUN_0045baf0, after the title asset loader.

## 9. Call sites (EXACT)

- FUN_00455a30 (Best Scores): only caller FUN_00432720, Information item 10. Draws lowscore.pcx.
- FUN_00473470 (Top 10 Designers): callers FUN_00432720 (item 11) and FUN_0044cff0 (end-of-year screen).
- FUN_00459850 (pair selection): no decompiled caller (UNKNOWN trigger). Lists waiting golfers (state 0xff, newest first, 0x98 slots), up to 2 selectable, then FUN_0045de30 swaps into pair slots and FUN_0045de80 initialises the pair (state 1, partner links, story pick by trait compatibility), sets 0x20000 on both. Cards show traits, age, marital status. Purpose DERIVED: match-making for story events.
- FUN_0040f190: starts practice/match using two arrival slots (FUN_00421bc0).
- FUN_0044b9c0 (credits): no decompiled caller (UNKNOWN); layout per DECODE_TITLE2.md.
- courseinfo family (loader FUN_00442180): courseinfo.pcx, courseinfo_A.pcx (alpha), s_courseinfo.pcx (shape mask). Four HUD cuts (x,y,w,h): (48,6,183x58) course name/date pill; (647,13,138x36) money counter; (675,55,110x37) fun rating; (697,99,88x36) skill rating. Drawn by the main frame at 0x418ee9 (plate at (48,5), pills at (649,13), (677,55), (699,99)), each after its s_ cut darkens the course; full layout in crates/simgolf/src/hud_ui.rs (EXACT).
- lowscore.pcx: drawn only by FUN_00455a30.
- Club emblems: parklink.pcx and tropdesert.pcx, 8 cuts of 80x80 each (+ _A alpha), stored as 16 sprites with 0x2c stride at 0x5791f8.
