# Decode: golfer voices, sound ids and how a reaction picks a clip

Source: only `spec/golf_decomp.c` (Ghidra text of the publisher golf.exe) and the disc folders `SimsFX/`, `Sounds/`. No binary was opened. Own words. Line numbers are lines of the decompile (approximate to a few lines).

Tags: EXACT = read directly from decompiled code. DERIVED = follows from code plus a stated assumption or a naming match. UNKNOWN = not established.

## 0. Summary

* Every golfer voice is played by the reaction routine `FUN_00467a00` (react) through a positional sound helper `FUN_0040c500(id, x, y, extra)`. The id is `base + g`, where `g` is 1 for a male golfer and 0 for a female golfer, so a voice id pair is (female, male) with an even female id. (EXACT)
* The id is an index into a table of 300 sound objects. The registration function `FUN_00448220` lists 219 files, but the LIST ORDER IS NOT THE ID ORDER: the decompile drops the object each file is loaded into, so the id of a file is not visible. Evidence that the two orders differ is in section 3. (EXACT list, DERIVED conclusion)
* Because of that, the mapping id to file below is reconstructed from the meaning of the reaction type plus runs of consecutive ids that follow list order. It is solid for the 7-clip block (variety, same, bad hole, hungry, thirsty, tired, options) and the 15-clip block (coke ... ugly, story), and weaker elsewhere. Marked per row.
* No voice test of the speech timer, the conversation state, the golfer kind (pro, VIP) or a probability exists in the voice calls. A voice plays every time its reaction type is raised, if the golfer is on screen. (EXACT)
* There is no random variant digit. The disc names with a digit (`HAPPY0`, `HAPPY1`, `HARD0`, `EASY0` ...) are just names; the digit never selects among variants in code. `HAPPY0` and `HAPPY1` have no play call that I could find. (EXACT absence of a visible caller, UNKNOWN reason)

## 1. The sound engine front end

### 1.1 Objects and registration (EXACT)

* `FUN_00448160` (line 41376) builds an array of 300 sound objects, 0x6c bytes each, starting at 0x80d840. `FUN_004490b0` (line 41656) releases them.
* `FUN_00448220` (lines 41423 to 41655) makes 219 calls `register(file, flags)` in a fixed order (index 0 to 218). The flag is 4 for effects, voices and emotion clips, and 0x14 for the buy jingles and the music loops (so 0x14 is probably "streamed or looped"; UNKNOWN). The object the file goes into is not shown in the decompile (it is the hidden `this` argument).
* One path in that list is not on this disc: `simsfx/male/Vox_clear_throat_M3` (list index 164). The disc has `Sounds/Golf_Sfx/Vox_gruntM1.wav` instead, which the list does not name. One emotion path in the list reads `PLS Sid mix` while the disc file is `PLS Sad mix.wav`, so that clip would not load in the original either (DERIVED from the string text). Paths in the list are written `sounds/...` and `simsfx/Male|Female|male|female/...`, case does not matter on disc.

### 1.2 Play wrapper `FUN_004481b0(id, volume, pan, pitch, extra)` (lines 41396 to 41410, EXACT)

* Does nothing for id -1.
* Sets on object `id`: pitch offset (clamped -1200 to 1200), volume (`& 0x7f`), pan (clamped -64 to 63), field `extra`, then plays.
* UI calls pass `(id, 100, 0, 0, 0)`: volume 100 of 127, centre, no pitch change.
* `FUN_00448200` (line 41413) stops a sound with a constant 1000 (a fade time, UNKNOWN). Called for id 0x7f and for 0x80 or 0x7d (lines 32250, 34174, 45997), all music or jingle ids.

### 1.3 Positional helper `FUN_0040c500(id, x, y, extra)` (lines 10929 to 10985, EXACT unless noted)

1. Converts the world position (1/1024 tile units) to a screen position with the view projection `FUN_0042fb90` (margin 0). When the zoomed flag `DAT_005a9cbc` is set the screen coordinates become `c * 2 - 400`.
2. If `extra != -1`: the sound plays only if the point is on screen (screen x 0 to 799, screen y 0 to 500). Off screen means silence. If `extra == -1`: the point is clamped to the screen and the sound always plays, with extra 0.
3. Volume (value passed to the wrapper): `|sx - 400| / 16 + 50`, minus 40 when the zoom level `DAT_004c2844` is 3 or less. So 10 to 35 at low zoom and 50 to 75 at high zoom. The value grows toward the screen sides, which looks odd; the direction of the scale is UNKNOWN. A second branch (taken only when `DAT_005a9cd8` is nonzero, a flag I cannot find being set anywhere in the text, so DERIVED dead) uses `sy / 12 + 50 (-40)` and a pitch from a 36 entry table.
4. Pan: `sx * 127 / 800 - 64`.
5. Pitch: `300 - rand(600)` each play, so every positional sound, voices included, gets a random pitch offset of about -299 to +300. The unit is UNKNOWN (the wrapper clamps at 1200, which would be one octave if hundredths of a semitone).
6. The `extra` value goes through to the wrapper as the last argument. Callers use 0, 500, 1000, 1500 or -1, so it behaves like a start delay in milliseconds (DERIVED from the values and the splash then voice ordering below; UNKNOWN as code).

Note on the decompile: for the voice calls inside react the call is printed `FUN_0040c500(base + g)` with one argument. The position and the delay appear as the extra arguments of the neighbouring gender helper `FUN_0046c940(g, x, y, delay)` (whose body only reads `g`). I read them as belonging to the sound call. (DERIVED)

### 1.4 Gender helper `FUN_0046c940(g)` (line 71398 region, EXACT)

Returns `(~flagByte >> 7) & 1` where `flagByte` is byte +1 of the golfer's character template (the same flag byte whose bit 7 means female, see `DECODE_CUSTOMISE.md`). Female returns 0, male returns 1. The golfer body position used for the sound is the golfer record's x and y (0x5794b8, 0x5794bc).

## 2. Reaction type to voice id (react, lines 71470 to 71730)

All of these are inside the switch of `FUN_00467a00`. The routine returns before the switch when the golfer's strokes exceed 9, when the golfer index is 0x98 or more (staff and pseudo golfers never speak), or when game flag word 0x59e7b8 has bit 0x2000000; and type 0x13 (hole score) returns before any sound (lines 71433 to 71479). (EXACT)

Voice calls ignore the speech timer, the conversation state, kind, difficulty and the mood delta. The delta only matters for the advisor message. For types 25 (drink) and 27 (bench) the voice plays even when the "+1 only if thirsty or tired enough" condition fails. (EXACT)

`k` below is the body class slot (section 4). `g` is 0 female, 1 male.

| Type | Meaning | id sent | extra | Notes | Mark |
|---|---|---|---|---|---|
| 1 | good shot | `0xd2 + k`, or `0xe6 + k` when golfer flag 0x20000 is set | 0 | emotion banks, section 4 | EXACT id |
| 2, 3 | bad lie | `0xdc + k`, or `0x10 + k` when flag 0x20000 is set | 1000 if the landing tile is water (terrain 0x11), else 0 | the `0x10 + k` branch overlaps voice ids 0x10 to 0x19, UNKNOWN | EXACT id |
| 8 | bad design | `0xdc + k`, or `0x10 + k` when flag 0x20000 set | 0 | same banks as 2, 3 | EXACT id |
| 4 | missed easy shot | `0x12 + g` | 0 | | EXACT |
| 5, 31 | hazard ahead, lots of one terrain | `0x10 + g` | 0 | | EXACT |
| 6, 39 | uses a slope, scared an animal | `0x14 + g` | 0 | type 39 only fires for animals | EXACT |
| 7 | scenic bridge (only when location 0) | `0x4c + g` | 0 | | EXACT |
| 9 | ball nearly hit someone | `0x46 + g` | 0 | | EXACT |
| 10, 43 | walk through obstacle, steep slope | `0x50 + g` | 0 | | EXACT |
| 11 | scenic object | `0x4a + g` | 0 | | EXACT |
| 12, 13 | tree or rock trouble, ball in water | `0x08 + g` | 500 | ids 6 to 8 are the three random ball-hits-tree sounds (line 21809), so id 8 collides; UNKNOWN | EXACT id |
| 14 | thirsty | `0x9e + g` | 0 | | EXACT |
| 15 | hungry | `0x9c + g` | 0 | | EXACT |
| 20 | ugly object | `0x56 + g` | 0 | also sets a pose and hold timer | EXACT |
| 21 | slow play | `0x44 + g` | 0 | | EXACT |
| 22 | celebrity house | `0x4e + g` | 0 | | EXACT |
| 23 | hole too hard or too easy | `0x9a + g` | 0 | | EXACT |
| 24 | weeds | `0x42 + g` | 0 | | EXACT |
| 25 | drink taken | `0x3c + g` | 0 | | EXACT |
| 26 | tired | `0xa0 + g` | 0 | | EXACT |
| 27 | bench | `0x48 + g` | 0 | | EXACT |
| 28 | lovely object | `0x4c + g` | 0 | also sets a pose | EXACT |
| 29 | variety | `0x96 + g` | 0 | | EXACT |
| 30 | repetition | `0x98 + g` | 0 | | EXACT |
| 32, 33 | choosing distance, choosing side | `0xa2 + g` | 0 | | EXACT |
| 34 | reply to partner chat | female `0xa4`, male `0xa7` | 0 | asymmetric on purpose or by accident; location (the three reply grades) is ignored | EXACT |
| 35 | tantrum | `0x46 + g` | 0 | played TWICE the first time: once before the history check (line 71474) and once in the switch; a repeat of the same type plays once then returns | EXACT |
| 36 | another golfer snapped | `0x3e + g` | 0 | | EXACT |
| 58 | Club Pro reply, needs too high | female `0x5a`, male `0xa5` | 0 | | EXACT |
| 65 (0x41) | label unknown; id `0x16 + g` | 0 | no caller found in the text | EXACT id |
| 18 snack, 16 hook, 17 slice, 37, 38, 40, 41, 42, 44 to 49, 54 to 57, 59 to 63 | no sound call in react | | | type 40 plays a swing sound instead, section 5 | EXACT |

Who is allowed to speak: every golfer index below 0x98 with a body on screen, ordinary visitors, pros and VIPs alike. No kind test exists in these calls. (EXACT)

## 3. Why the registration list is not the id list

Facts from the code that contradict "id equals list index":

1. Ids 4 to 8 are used for: ball drops into the hole (4, line 21749), water (5, line 22065), three random tree hits (6 to 8, line 21809). In the list those three kinds are consecutive at indexes 12 to 16 (Ball In Hole, Ball Water, Ball Tree, Ball Tree2, Ball Tree Leaves). So id 4 is list index 12, a shift of 8, not zero. Ids 2 and 3 (chip club and putter strike, lines 22302 region) match list indexes 10 and 11 (Chip, Putt) with the same shift of 8. (DERIVED)
2. Types 5, 31, 4, 6 use ids 0x10, 0x12, 0x14 in pairs; the list holds male and female clips as two separate blocks, never interleaved. (EXACT)
3. Id 0x18 is the UI "refused" beep (lines 28258, 57266, 57912, 63340, 81855) and also the ball-hits-something thud at line 21924, which sits between voice ids 0x16 and 0x3c. Nothing in the list sits between the hard/easy/tricky/blind block and the coke block. (DERIVED)
4. Id 0x7f is the end of year screen sound (line 44387, 45997); list index 0x7f is `buy2short`. (DERIVED)

So the order of registration is the order the author typed the lines, and each line targets an explicit slot that the decompile omits. Runs that the author typed in slot order survive as runs of consecutive ids, which is what the voice block anchors below use.

## 4. Voice clip to id (reconstruction)

Method: take clips in list order, interleave (female, male) per stem, and pin the run to ids by the reaction meaning. The meaning anchors are strong because every type's meaning matches its stem name.

Block A (female id even, male id +1):

| Stem | Female id | Anchor | Mark |
|---|---|---|---|
| HARD0 | 0x10 | types 5, 31 | DERIVED |
| EASY0 | 0x12 | type 4 "too easy" | DERIVED (strong) |
| TRICKY0 | 0x14 | types 6, 39 | DERIVED |
| BLIND | 0x16 | type 65 | DERIVED |
| (gap 0x18 to 0x3b: UI, thuds, effects) | | | |
| COKE0 | 0x3c | type 25 | DERIVED (strong) |
| BAD0 | 0x3e | type 36 | DERIVED |
| SAD2 | 0x40 | no caller found | DERIVED id, UNKNOWN use |
| CRAB0 | 0x42 | type 24 weeds | DERIVED (strong) |
| WAITING0 | 0x44 | type 21 | DERIVED (strong) |
| MAD0 | 0x46 | types 9, 35 | DERIVED |
| BENCH0 | 0x48 | type 27 | DERIVED (strong) |
| SENIC0 | 0x4a | type 11 | DERIVED (strong) |
| LOVELY0 | 0x4c | types 28, 7 | DERIVED (strong) |
| CELEB0 | 0x4e | type 22 | DERIVED (strong) |
| WET0 | 0x50 | types 10, 43 | DERIVED |
| OOPS0 | 0x52 | no caller found | DERIVED id, UNKNOWN use |
| WATER0 | 0x54 | no caller found | DERIVED id, UNKNOWN use |
| UGLY0 | 0x56 | type 20 | DERIVED (strong) |
| STORY0 | 0x58 | story initiator (section 6) | DERIVED |
| STORYNO | 0x5a | story reject and type 58 female | DERIVED |

Block B (the second 7-clip block, exact consecutive run):

| Stem | Female id | Type | Mark |
|---|---|---|---|
| VARIETY | 0x96 | 29 | DERIVED (strong) |
| SAME | 0x98 | 30 | DERIVED (strong) |
| BADHOLE | 0x9a | 23 | DERIVED (strong) |
| HUNGRY | 0x9c | 15 | DERIVED (strong) |
| THIRSTY | 0x9e | 14 | DERIVED (strong) |
| TIRED | 0xa0 | 26 | DERIVED (strong) |
| OPTIONS | 0xa2 | 32, 33 | DERIVED (strong) |
| STORYOK | 0xa4 | story accept, type 34 female, type 58 male (0xa5) | DERIVED |
| STORYYES | 0xa6 | story full yes, type 34 male (0xa7) | DERIVED |

The unusual case: ids 0xa4 and 0xa6 are placed after OPTIONS, not after STORYNO as the list order would suggest, because the story code needs (accept, reject, yes) as 0xa4, 0x5a, 0xa6 and the staff code owns 0x5c to 0x5f.

Not placed by any id found: `HAPPY0`, `HAPPY1` (a chain run would put them at 0x0c and 0x0e, DERIVED weak, but no call uses 0x0c to 0x0f), the `Good Shot FM2` and `Good Shot M2` clips from `Sounds/Female` and `Sounds/Male` (list indexes 43 and 44), `mGREETING` (list 83), `mCRABGRASS2` (85), `fHAVEADRINK` (90). Their ids are UNKNOWN.

Body class slot `k` for the emotion banks: `k = (template byte +3, high nibble)`, plus 5 for a female golfer (lines 71504 to 71510). Male slots 0 to 3 are PLS, KLS, PSS, SSS; female slots 5 to 8 are PLS, SSS, PSS, SkTT (this is also the body order in `DECODE_CUSTOMISE.md`). (EXACT arithmetic, DERIVED naming)

### 4.1 Emotion clips (`Sounds/Emotion/`)

The list holds 32 emotion clips in eight classes of four (Happy, Sad, Success, Failure): male PLS, KLS, PSS, SSS (the PLS, KLS and SSS ones use the `... mix` files) then female PLS, SSS, PSS, SkTT. The code uses four banks of ten consecutive ids, class-major inside the bank: starts 0x10 (only when flag 0x20000), 0xd2, 0xdc, 0xe6 (section 2, types 1, 2, 3, 8). So the id is kind-major (bank by kind, slot by class) while the list is class-major. Which bank is which kind is not visible. Best reading by meaning (DERIVED, weak): type 1 normal is Success, type 1 after a complaint is Happy, types 2, 3, 8 normal are Failure, after a complaint are Sad. The clips `FemaleSKSS*`, `MalePLS*`, `MaleKLS*`, `MaleSSS*`, `FM * mix` on disc are never named in the list (EXACT absence).

## 5. Swing sound ids (type 40 and the strike)

At swing frame 5 (lines 22270 to 22313) the strike sound id is chosen for the golfer, played with `FUN_0040c500(id, x, y, 0)` (so positional, on screen only, random pitch):

* default 0xbe; if skill bit 1 (length) is clear 0xc1; if bit 2 (accuracy) is clear 0xbf; if bit 4 (imagination) is clear 0xc0 (later tests win); (EXACT)
* if plan flag 0x400000 (pure strike) is set: 0xbd, and `react(g, 40, 0)` runs with the speech timer forced to 3; (EXACT) so type 40 has NO voice, only the strike sound 0xbd;
* if the club index is above 9: 2 (chip); if on the putting surface: 3 (putt). (DERIVED meaning)

The five ids 0xbd to 0xc1 are five swing clips. The list has Tiger Swing 1, Golf Swing Perfect (M), NoAcc, NoImag M and Perfect FM in that order at indexes 1, 4, 5, 6, 7, which fits 0xbd, 0xbe, 0xbf, 0xc0, 0xc1 as a run, with 0xbf as NoAcc and 0xc0 as NoImag matching their skill bit meaning. Whether 0xbd is Tiger Swing 1 and 0xc1 is Perfect FM: DERIVED weak. (There is also a second strike sound, id 0x2e with extra 1000, played for the controlled golfer when kind bits are set, UNKNOWN clip.)

Other ids met next to the reaction code, for orientation (DERIVED from meaning): 4 ball in hole, 5 water splash, 6 to 8 tree hits, 0x36 or 0x37 ball bounce on ground (0x37 on terrain 7), 0x30 negative mood alert for the controlled golfer (line 71761), 0x31 and 0x39 and 0xd1 ambient or effect sounds.

## 6. Other voice uses

* Story and chat between partners, `FUN_00466370` (lines 69734 to 69900). The initiating golfer says STORY0: `FUN_0040c500(0x58 + g)` (line 69778). The partner reacts 1500 ms later (the `0x5dc` argument) with, by how its story counter compares to the initiator: exactly one behind gives 0xa4 (accept), further behind gives 0x5a (reject, and the partner's counter drops by 1), caught up or ahead gives 0xa6 (yes). Each is the partner's own gender `+ g`. (EXACT ids and rule, DERIVED clip names). When the initiator's counter is 4 and the partner's is 4 the pair becomes friends and a theme stinger plays (ids 0x33, 0x78, 0x6e or 0x73 by theme `DAT_005a34e0`, non voice). Counter 1 plays 0x32, 0x34, 0x6e/0x70, 0x73/0x75 or 0x78/0x7a (non voice).
* Staff, `FUN_00402a40` (the staff step, lines 1223 to 1700): staff members speak with `0x5c`, `0x5d` (500 ms), `0x5e`, `0x5f` plus an offset 0, 4 or 8 depending on the staff kind flag. Ids 0x5c to 0x67 are therefore staff lines; the candidate clips are `mGREETING`, `Ranger`, `Marshall Male`, `Tray Girl`, `fHAVEADRINK`, `Lawn Technician`, `mCRABGRASS2`. Assignment UNKNOWN. (EXACT ids)
* Options, `STORYOK`, `BLIND`, `CRAB`: OPTIONS is the "choosing distance or side" voice (types 32, 33). BLIND is wired to type 65 only, which no caller raises. CRAB is the weeds voice (type 24). STORYOK is the accept reply above.
* Celebrities: the list names `Celebs/*` clips (Spears, Politician, Action Star, Fitness Female, Movie Princess, Rock Star, Basketball, Aging Star, Comedian, Female Comic, Super Model, I'll Be Back) at list indexes 84 to 98 and 157 to 168, but their ids are not recoverable and I found no call that picks them by name. UNKNOWN. Note the celebrity house reaction (type 22) uses the ordinary CELEB0 voice, not a celebrity clip.
* Visitor greetings, CEO: no sound call with a CEO or greeting tie was found. `mGREETING` and `Vox clear throat` are registered; their callers are UNKNOWN.
* Menu and options screen voices: none found. The UI calls use ids 0x18, 0x23, 0x24, 0x26, 0x29, 0x2a, 0x2c to 0x2f with volume 100, and 0x38 for a confirm (lines 4634 with volume 70, 58910, 82544); these are non voice clips. (EXACT ids, clips UNKNOWN)

## 7. Unused voice clips

Present on disc and never named in the registration list (so never loaded): `mBENCHOLD`, `mCRABGRASS`, `mHURRYUP`, `mSAD0`, `mSTORYx`, `fHUNGRY2`, `fSAME2`, `fSNACK0`, `fVARIETY2`, `fSAD0`, `sCOKE0`. (EXACT, searched all `simsfx` strings in the text)

## 8. Port recommendation

Because ids cannot be tied to files exactly, key the port by stem, not by id:

1. On every react(type) for a golfer index below 0x98 whose body is on screen, play `SimsFX/<Female|Male>/<f|m><STEM>.wav` from section 9, no timer test, no kind test.
2. Female prefix `f` when the template flag bit 7 is set, male prefix `m` otherwise.
3. Delay 500 ms for types 12 and 13, 1000 ms for bad-lie types 2 and 3 when the tile is water, 1500 ms for the chat partner's reply, else immediate.
4. Pan from the screen x as in 1.3, volume a constant (the original scale is UNKNOWN), pitch jitter small (unit UNKNOWN).
5. Type 35 plays twice the first time (can be reproduced or ignored).
6. Types 1, 2, 3, 8 use the emotion clips by body class, types 12 and 13 use OOPS0 or WATER0 until the id collision is settled (UNKNOWN).

## 9. Final table (every clip on disc)

Stem files are `SimsFX/Male/m<STEM>.wav` and `SimsFX/Female/f<STEM>.wav`. Type numbers as in `holestats.h` and `DECODE_COMMENTS.md`. Delay is the start delay of section 1.3.

| Clip file (male / female) | Reaction type or situation | Rule | Mark |
|---|---|---|---|
| mHARD0 / fHARD0 | 5 hazard ahead, 31 lots of one terrain | id 0x10 + g, immediate | DERIVED (id by name run) |
| mEASY0 / fEASY0 | 4 missed easy shot | id 0x12 + g, immediate | DERIVED |
| mTRICKY0 / fTRICKY0 | 6 uses a slope, 39 scared an animal | id 0x14 + g | DERIVED |
| mBLIND / fBLIND | 65 (no caller found) | id 0x16 + g | DERIVED id, UNKNOWN use |
| mCOKE0 / fCOKE0 | 25 drink taken | id 0x3c + g, voice plays even without thirst bonus | DERIVED |
| mBAD0 / fBAD0 | 36 another golfer snapped | id 0x3e + g | DERIVED |
| mSAD2 / fSAD2 | none found | id 0x40 + g | UNKNOWN use |
| mCRAB0 / fCRAB0 | 24 weeds | id 0x42 + g | DERIVED |
| mWAITING0 / fWAITING0 | 21 slow play | id 0x44 + g | DERIVED |
| mMAD0 / fMAD0 | 9 ball nearly hit, 35 tantrum (twice first time) | id 0x46 + g | DERIVED |
| mBENCH0 / fBENCH0 | 27 bench | id 0x48 + g | DERIVED |
| mSENIC0 / fSENIC0 | 11 scenic object | id 0x4a + g | DERIVED |
| mLOVELY0 / fLOVELY0 | 28 lovely object, 7 scenic bridge | id 0x4c + g | DERIVED |
| mCELEB0 / fCELEB0 | 22 celebrity house | id 0x4e + g | DERIVED |
| mWET0 / fWET0 | 10 walk through obstacle, 43 steep slope | id 0x50 + g | DERIVED |
| mOOPS0 / fOOPS0 | maybe 12 or 13 (id 0x08 clash) | id 0x52 + g by run, or 0x08 | UNKNOWN |
| mWATER0 / fWATER0 | maybe 13 ball in water | id 0x54 + g by run | UNKNOWN |
| mUGLY0 / fUGLY0 | 20 ugly object | id 0x56 + g | DERIVED |
| mSTORY0 / fSTORY0 | story initiator, partner chat | id 0x58 + g | DERIVED |
| mSTORYNO / fSTORYNO | partner rejects; type 58 (female) | id 0x5a + g | DERIVED |
| mSTORYOK / fSTORYOK | partner accepts; type 34 (female), 58 (male) | id 0xa4 (f) 0xa5 (m); 1500 ms | DERIVED |
| mSTORYYES / fSTORYYES | partner caught up; type 34 (male) | id 0xa6 (f) 0xa7 (m); 1500 ms | DERIVED |
| mVARIETY / fVARIETY | 29 variety | id 0x96 + g | DERIVED |
| mSAME / fSAME | 30 repetition | id 0x98 + g | DERIVED |
| mBADHOLE / fBADHOLE | 23 hole too hard or too easy | id 0x9a + g | DERIVED |
| mHUNGRY / fHUNGRY | 15 hungry | id 0x9c + g | DERIVED |
| mTHIRSTY / fTHIRSTY | 14 thirsty | id 0x9e + g | DERIVED |
| mTIRED / fTIRED | 26 tired | id 0xa0 + g | DERIVED |
| mOPTIONS / fOPTIONS | 32 choosing distance, 33 choosing side | id 0xa2 + g | DERIVED |
| mHAPPY0 / fHAPPY0 | none found | registered, no call | UNKNOWN |
| mHAPPY1 / fHAPPY1 | none found | registered, no call | UNKNOWN |
| mGREETING | none found (also a `Celebs/mGREETING` entry) | registered, maybe staff 0x5c to 0x67 | UNKNOWN |
| mCRABGRASS2 | none found | registered, maybe lawn technician staff | UNKNOWN |
| fHAVEADRINK | none found | registered, maybe tray girl staff | UNKNOWN |
| Male/Good Shot M2, Female/Good Shot FM2 (under `Sounds/`) | none found | registered, no id known | UNKNOWN |
| mBENCHOLD, mCRABGRASS, mHURRYUP, mSAD0, mSTORYx, fHUNGRY2, fSAME2, fSNACK0, fVARIETY2, fSAD0, sCOKE0 | never loaded | not in the registration list | EXACT |
| Emotion/* (32 registered, 8 classes by 4 kinds) | types 1, 2, 3, 8 | id bank by kind, slot by body class; flag 0x20000 picks the second bank; bank to kind order UNKNOWN | EXACT ids, UNKNOWN files |
| Golf_Sfx swing clips (Tiger Swing 1, Perfect, NoAcc, NoImag, Perfect FM) | strike at swing frame 5, type 40 uses 0xbd | ids 0xbd to 0xc1 by skill bits | EXACT ids, DERIVED weak files |
