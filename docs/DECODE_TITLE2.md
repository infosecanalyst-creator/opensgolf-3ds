# Title screen remainder (decoded from the decompile)

Marks: EXACT = read directly from code, DERIVED = inferred from code plus art, UNKNOWN = not recoverable from the decompile.
Text calls: 0x4049d0 left aligned, 0x404b70 centred, 0x404ad0 left with bottom clamp, 0x404bc0 centred. Colours are 15-bit 5-5-5 (channel * 8).
Hit tests use the octagonal distance max(|dx|,|dy|)+min/2 (FUN_00467170). Hover tooltip appears after 30 stable frames.
Dispatch from the title menu lives in FUN_0040f5c0, whose decompile failed: call order is UNKNOWN. Callee behaviour below is EXACT.

## 1. Load Previous Game (FUN_0043b610) EXACT unless noted
Art: Title_LoadGame.pcx base. Title_LoadGame_MO.pcx cuts: (600,520,70x70) OK, (740,530,60x60) back, (778,100,22x40) up, (778,380,22x40) down, (670,520,70x70) delete, grey disabled OK (600,435,70x70) and delete (670,435,70x70), drawn at (600,520) and (670,520) while nothing is selected.
Hit circles (centre, r): OK (633,555) 26; back (769,556) 24; up (784,120) 20; down (784,400) 20; delete (704,555) 23.
- Title "Load Previous Game" centred (504,42) black; championship mode shows "Select Championship Course".
- List source: "Saved Games\*.sve" (championship: "Themes\Championship\*.cse"). Names containing "Shadow" are skipped.
- 16 rows, left x=320, y=116+16i, black. Selected row bar (310,y-1,456x15) colour 0x7b20. If more rows exist, "(more...)" at y=372.
- Scroll thumb: x=781, w=6, y=147+scroll*228/count, h=clamp(16*228/count), white.
- Names starting with '&' are autosaves, shown as "autosave" plus the rest in brackets (punctuation DERIVED).
- Click: x>320, y>115 gives row=(y-116)/16+scroll. Valid row selects (normal load also preview loads it). Row >= count deselects. Scroll buttons move 4 rows, clamp [0,count-16]. Esc cancels.
- Left info panel (only with a selection): headers grey 0x4210 centred y=124 at x=66/125/184: Holes, Par, Yards; values y=146. Cash label (75,169), money centred (161,169). Labels left x=48 y=192/210/228/246: Fun Rating, Length Skill, Accuracy Skill, Imagination; values centred x=184 (fun green 0x1284, skills blue 0x0210, skills shown as v/100 with two decimals). Header row y=270 (Hole, Par, Yards); per hole rows y=286+17(h-1), x=66/125/184.
- Oval site thumbnail at (84,32): which sprite UNKNOWN.
- Dark bar, x=388 white: " (Theme: <pack>)" y=442; "Designed by <name>" y=458; "Course Record: n by name" y=474 when a record exists.
- OK: draws "Loading..." at (550,456) colour 0x7b20 and loads. Mid game, the entry saves "While Browsing.sve" and Cancel restores it.
- Delete: yes/no box (FUN_0046d6e0) then removes the file. Tooltip words: UNKNOWN (use Cancel, Delete, Load).
## 2. Pick A Pro (FUN_0043a8c0)
Same layout and hit circles, art Title_Pickapro.pcx, list "Themes\Championship\*.pro", selected bar 0x1284 with white text. Loads the pro into player record 0 (the player's own character, not an opponent). See DECODE_CUSTOMISE.md section 6.
## 3. Championship flow (FUN_0046ddd0) EXACT
1. Remember difficulty. 2. Load screen in championship mode (.cse), 0 = cancel. 3. Cash=$100,000 units 1000, tick=0x2c00, flag 0x4000000, restore difficulty. 4. Pick A Pro; on cancel clear flags and return 0. 5. FUN_0046c970 draws the tournament field and starts it, cash reset to 1000, return 1. Button order in title: UNKNOWN.
## 4. Theme Packs (FUN_004725b0)
- Themes = folders in Themes\ excluding names with '.' and "Championship". On disc: Firaxis, More_Stories, Standard, The_Sims. EXACT.
- Probe per theme: *.txt (Stories), *.chr (Characters), celebrities.dta, progolfers.dta, *.cse (Courses; the code tests the wrong bit, implement as "has a .cse"), landmark flc.
- Art: Title_ThemePacks.pcx; MO sheet cuts: (1,1,87x31) yellow empty, (89,1,87x31) yellow ball, (177,1,87x31) beige ball, (1,33,221x31) name pill, (265,1,71x80) OK, (337,1,48x49) back, (386,1,19x35)/(406,1,19x35) arrows. Base recuts at (581,520),(726,533),(769,81),(769,471).
- Title "Select a Theme Pack" left (78,34). Headers centred x=346/436/526/616/706, y alternating 62/37/62/37/62: Stories, Characters, Celebrities, Pro Golfers, Courses.
- Rows y=92+32i; name left (73,y+11) grey 0x4210; current theme gets pill at (72,y), black text. Cells at (x_k-43,y): current row yellow (ball if file exists), other rows beige ball only if it exists.
- Click row=(my-91)/32 sets the theme. OK (615,555) r26 keeps it. Back (749,556) r24 or a key resets to "Standard". Click elsewhere keeps. Scroll arrows (778,99),(778,487) are inert.
## 5. Difficulty (FUN_0043a400)
- Base TitleSelDiffUnSel.pcx; hover cuts in TitleSelDiffMO.pcx: (0,0,338x146)->(193,32); (400,0,338x146)->(150,161); (0,300,332x114)->(161,320); (400,300,332x130)->(200,436); back (732,532,68x68).
- Hit: ellipses with dx/2, octagonal distance <80, centres (348,107), (319,218), (320,367), (363,495); back circle (767,557) r<25.
- "Select Difficulty" centred (602,42). Labels centred x=388/360/366/402, y=113/227/346/464: Easy (first string UNKNOWN, likely), Moderate, Difficult, Impossible. Hovered black at y, otherwise grey 0x4210 at y+2.
- Returns 0..3, or -1 on back, empty click or key. Stored in the difficulty global. Effects elsewhere; Top 10 multiplier is d+1.
## 6. Credits (FUN_0044b9c0)
- creditsbckgrd.pcx blitted at (0,0) every frame; bink64.pcx (64x64) logo.
- credits.txt: lines after "#CREDITS" until the next '#' line, max 512, blanks kept. A "$bink" style marker line draws the logo centred (x=368) (DERIVED).
- Text left x=50, large face, white with shadow (2,2). Pitch from font metrics: UNKNOWN (use font height).
- First line y=600-elapsed_ms/30. Ends when y <= -(pitch*count+150), or any click/key. Frame wait 10 ms.
- credit.wav is registered but its trigger is UNKNOWN (play it with the screen, DERIVED).
