# DECODE_PICKAPRO: Pick A Pro, Load Previous Game left panel, championship flow

Tags: EXACT, DERIVED, UNKNOWN. Colours are 5-5-5 words (RGB = channel*8, bit 0x8000 flag).
Text primitives: FUN_004049d0 left, FUN_00404b70 centred (shadow colour -1: no shadow); FUN_00404ad0 left, FUN_00404bc0 centred (shadow in palette colour 1 one pixel below; 0x4767a0 draws the shadow only when the colour is not -1). Font object choice is hidden in the decompile, so face and size are UNKNOWN unless stated.

## 1. Pick A Pro (FUN_0043a8c0), screen art Title_Pickapro.pcx (EXACT)

- Temporarily sets the theme name buffer to "Championship"; restored on every exit.
- Lists Themes\Championship\*.pro. Title "Pick A Pro" centred at (504,42), black.
- List rows: x=0x140, y=0x74+16*i, 16 visible; selected row bar (0x136, y-1, 0x1c8 wide, 0xf high) colour 0x1284 (32,160,32) with white text, others black. Scroll step 4 rows, clamp [0, count-16].
- Clicking a row preview-loads that pro into player record 0 via FUN_00437fa0(file,0,1). Clicking empty space deselects.
- No selection: disabled OK sprites cut at (600,0x208) and (0x29e,0x208).

### Left panel (drawn only when a pro is selected)

- Portrait: head sprite blitted at (69,61), 140x140, the loaded pro's face (EXACT position). Which face expression/sprite: UNKNOWN (object hidden).
- Header: "<name>'s skills" left at (56,255), black, over the art pill.
- Ten skill rows, y = 279 + 24*i (0x117+0x18*i). Name left at x=94, colour 0x80000210 (teal 0,128,128), inside the cream box. Value text is "+" then skill*10 then "%", centred at x=59, colour 0x0210 (teal), inside the oval badge. Skill values from DAT_005a5a04[i].
- Row names in order (first from label table, rest from SCREENSHOT_NOTES, DERIVED): Power Hitter, Long Driver, Accurate Driver, Accurate Irons, Accurate Putter, Draw Shot (R to L), Fade Shot (L to R), High Backspin Shot, Recovery Skills, Luck.
- No bars are drawn by code; ovals and boxes are baked into the art (measured: ovals x about 38-82, boxes about 90-227, rows y about 276-505).
- Bottom bar: label "Signature saying:" left at (56,544), black over the pill. Signature text = open quote + sentence for dialogue event 0x3e (FUN_00469b00, profile 0 first slot, stock fallback) + close quote, drawn at (36,562), white 0x7fff, via FUN_00404ad0, in the dark bar (art x about 30-555, y 550-580).

### Buttons and tooltips

Hit circles: OK (633,555) r<26; back (769,556) r<24; up (784,120) r<20; down (784,400) r<20; delete (704,555) r<23. Tooltips after 30 frames, centred at (cutX+0x1e, cutY+0x10), white on a small dark rect (cutX+6, cutY+0xe, 0x30x0x10).
- OK: word stored in a shared literal shorter than 5 chars (UNKNOWN; candidates Load, Pick, Okay). Same literal on Load and Theme Packs.
- Back: "Cancel" (EXACT). Delete: "Delete" (EXACT).
- OK with selection: "Loading..." white at (388,472), FUN_00437fa0(file,0,1), returns 1. No selection returns 0. Back/Esc returns 0.
- Delete confirm: FUN_0046d6e0(400,100,1,1,0); heading begins "Are you sure you want to delete" plus the file name; options begin "Yes, delete this file." and "No, never ..." style (rest UNKNOWN). Deletion by FUN_004a64b8.

## 2. .pro/.chr file format (EXACT; save FUN_00437910, load FUN_00437fa0)

0x230 profile record; 0x4e2 dialogue table (25 x 50); 16 skill bytes; 8-byte tag "*PCXFILE"; 8-bit PCX 140x420 holding three 140x140 faces at y 0, 140, 280. Load: if head byte > 0x13 and the custom counter for the gender is < 0x48, head byte becomes the counter and the PCX is cut into halo slots. Records >= 0x4d load from Themes\Standard. Save path Themes\<theme>\<name>.pro or .chr; messages for overwrite, invalid file name, invalid file path.

## 3. Load Previous Game left panel (FUN_0043b610, art Title_LoadGame.pcx) (EXACT)

Param 2 = 1 means championship mode (.cse list, "Select Championship Course"); param 1 = 1 from title.
- Oval thumbnail at (84,32): club emblem sprite (80x80) index from the property record of the previewed save (sprite base 0x5791f8, 0x2c stride; FUN_00473f60). Black oval window in art about x 66-184, y 25-113.
- Headers Holes, Par, Yards at x 0x42, 0x7d, 0xb8, y 0x7c, centred, colour 0x4210 grey; values y 0x92 black.
- Cash label (0x4b,0xa9) colour 0x2108; money centred at (0xa1,0xa9).
- Labels Fun Rating, Length Skill, Accuracy Skill, Imagination, left x=0x30, y 0xc0, 0xd2, 0xe4, 0xf6, colour 0x2108. Values centred x=0xb8: fun 0x1284 green, others 0x0210 teal.
- Per-hole table: header y=0x10e; rows y=0x11e+0x11*(h-1), x 0x42/0x7d/0xb8.
- Bottom (white, x=0x184): "(Theme: name)" y=0x1ba; "Designed by <player 0 name>" y=0x1ca; "Course Record: n by name" y=0x1da when a record exists.
- List: rows x=0x140, y=0x74+16i, selected bar colour 0x7b20 gold; scroll thumb x=0x30d, w=6. "(more...)" row. Saves in "Saved Games\*.sve", skipping "While Browsing". Mid-game entry saves "While Browsing.sve" and Cancel restores it. OK shows "Loading..." at (0x226,0x1c8) gold. Hit circles and tooltips as in section 1.

## 4. Championship flow (EXACT unless noted)

FUN_0046ddd0: save difficulty; FUN_0043b610(1,1) (cancel returns 0); set cash 1000 (=$100,000), tick 0x2c00, flags = 0x4000000 (overwrites all), restore difficulty; FUN_0043a8c0 (cancel clears flags, returns 0); FUN_0046c970; cash 1000 again; return 1. Return to title: UNKNOWN (caller in main loop).

FUN_0046c970 builds the field: snapshots profile and dialogue tables, resets 36 golfer slots (2 per open hole pair, max 18) with shotgun start; slot 1 is the player (skills from DAT_005a5a04, profile 0); the rest are pros from table 0x58dd50 (stride 0x38) chosen by a randomised strength window from skill sum and difficulty; loads each pro .glf via FUN_00437fa0 else fills from table. Sets flag 0x200000 (tournament running), counter DAT_0056a51c = 100, DAT_005a47e0 = -1.
- Prize: DAT_00567b04 (thousands) is not set by the championship flow and not restored by the .cse loader; the results routine (FUN_0045a090) substitutes 20*holes thousand when zero (EXACT: DAT_005685f0 * 0x14 - 0x14, i.e. 20*(holes-1) as written; check holes variable meaning, DERIVED). So championship prize is that fallback unless a save supplied one.
- Tournament name: chosen elsewhere from evaluation total; in championship mode the SGA offer and prep list are skipped. Screen after start: UNKNOWN.
- Results screen titles: "TOURNAMENT RESULTS" / "TOURNAMENT SCORES"; championship leaderboard headings "LEADER BOARD of the" with SGA Qualifying School, SGA Jr. Championship, SGA Tour Championship, SGA Open Championship by difficulty 0-3.
