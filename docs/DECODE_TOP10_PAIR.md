# Top 10, Best Scores and Pair selection

## 1. Top 10 Designers (FUN_00473470, insert FUN_004732d0)
- Reached from the Information menu "Top 10 Designers" and after the End of Year screen (shown only if the new entry made the list). EXACT.
- Art: Top10_Blank.pcx at (0,0); Top10_Trophies.pcx cells copied to the same position for occupied ranks. Cells: 5x2 grid, x=160*col; row 0 y=40 h=304, row 1 y=344 h=256. EXACT.
- Rank to slot (from art labels, DERIVED): 1st 2, 2nd 1, 3rd 3, 4th 0, 5th 4, 6th 7, 7th 6, 8th 8, 9th 5, 10th 9.
- Top10_Charms.pcx and TrophyMantle.pcx are not referenced by the decompile (UNKNOWN). TROPHYparts belongs to Accomplishments (UI_SCREENS2).
- File top10.sve: 10 records of 0x9c bytes = 1560. EXACT: +0 name[64], +0x40 course[64], +0x80 fun i32, +0x84 skill i32 (hundredths), +0x88 cash i32 (units of $100), +0x8c/+0x90 zero, +0x94 i16, +0x96 i16 difficulty, +0x98 i32 course id (-1 in defaults).
- Score = (cash/10 + skill + fun) * (difficulty+1). EXACT.
- Insert: walk from the top while new < entry score; a same course id met during the walk rejects (-1); running off 10 rejects. Ties go above. Drop the lowest same-course entry below the slot, else slot 9, shift down, write the new one (name profile 0, course name, fun, skill, cash, difficulty, course id). Position remembered for highlight.
- Default file when missing, i=0..9: fun=rand100+900-100i; skill=rand90+810-90i; cash=rand800+7200-800i; difficulty=(9-i)/3; id -1; course "Harbour Lights GC"; name "a.c. dye" with char0+=rand26, char2+=rand24.
- Draw per rank (x0=col*160+80 centre): name centred y=321 (row 0) or 574 (row 1), drawn black at y, yellow 0x7ff0 at y+2, colour at y+1; colour 0x4206, or white 0x7fff for the new entry, which also gets underlines (y+0x14f yellow, y+0x14e olive, width name+4).
- Stats centred x0, colour 0x4206. Cash y: row 0 164+10r, row 1 443+4r (r=rank). "Cash: n" ; skill at y-p ("Skill: v.vv"), "Fun: n" at y-2p; p=12 row 0, 10 row 1. Total number at fun_y-(8+p) (row 1: fun_y-(2+p)) embossed black y-1, yellow y+1, main 0x5288. "Total Score (xN)" at total-13. Rank 1 also shows course name at total-27. Fonts and sizes UNKNOWN. Wait for click or key.

## 2. Best N Hole Scores (FUN_00455a30, Information menu "Best Scores")
- Art Interface/infoscreens/lowscore.pcx pieces: top (195,45,411x79), row strip (195,224,411x17), bottom (195,321,411x61); hover OK cut (593,434,44x44). EXACT.
- Dim fill, top at (195,45). Title "Best N Hole Scores" centred (413,59) large face. "Golfer" left (236,96), "Score" centred (551,96).
- Up to 10 rows, stop at first 0. Row y=124+17i: strip at (195,y), name left (246,y), score centred (551,y). Bottom at y_end=124+17*count. Hover OK cut at (544,y_end+14) when pointer in x 544..587, y y_end+14..+58. Any click or key closes.
- Recording: after a full round of all holes, score = total strokes; ascending insert, ties after, max 10, name stored. Course record popup when course has over 2 holes and score <= record and <= par sum ("... has just set a new course record ..."). EXACT.

## 3. Pair selection (FUN_00459850)
- Art PairBase.pcx at (0,0); PairButtons.pcx cuts (0,0,329x136) tan, (0,136,329x136) pale ball, (0,272,329x136) yellow, OK hover (693,502,75x75).
- Title "SELECT THE NEXT PAIR OF GOLFERS" centred (338,14) large face, black.
- Candidates: golfers waiting (record byte +0x21 == 0xff), scanned newest first over 0x98 slots.
- Card k: x0=329*(k&1), y=50+136*(k/2); plate at (x0+6,y): yellow when selected, tan otherwise (cut 1 unused here, hover use DERIVED). Head cell 140 wide at (x0,y+4), full alpha selected else 0.7 alpha. Hovered unselected card draws 1 px higher.
- Text: name centred (x0+214,y+9); title centred (x0+262,y+40); age at y+72 (index%10 + 20/30/45 from flags 1/2/4) plus " years old"; marital at y+104 (Single, Married, Divorced, Widowed from flags 8/0x10/0x20/0x40); up to 5 trait lines left x0+146 from y+(5-count)*9+36 step 18 (labels UNKNOWN).
- Hover cell = (x-6)/334 + ((y-50)/136)*2 for x in [6,664), y>=50.
- Click on a card toggles it if fewer than 2 selected or it is already selected; else error sound 0x18. Click on empty space or key accepts: none selected returns; exactly 2 launches the pair (swap into the pair slots, flag 0x20000 on both); 1 selected plays error. EXACT.
- See DECODE_FACES.md row 11. Call site UNKNOWN.
