# Research: golfer voice playback (pan, pitch, delay, emotion banks, types 12/13)

Date 2026-10-05. Read-only research; no repo files edited.
Tags: EXACT = read directly from code or bytes. DERIVED = follows from code plus a stated assumption. WEAK = plausible reading only. UNKNOWN = not established.

## 0. Method and a caveat about sources

* The decompile `spec/golf_decomp.c` does answer the voice questions about ids, formulas, delays at call sites and banks, but it does NOT show which sound object each registration targets (the hidden thiscall `this`). That is why DECODE_VOICES.md could not tie ids to files.
* I closed that gap by reading the bytes of the unpacked exe `/mnt/user-data/uploads/golf.exe` (md5 8fe51a6b...; the copy under `game/Program_Executable_(ENGLISH)/golf.exe` is encrypted and unusable). In that exe, `FUN_00448220` (0x448220) is a run of `push flags; push &path; mov ecx, <object>; call 0x484e30`, 219 times. Object address = 0x80d840 + id * 0x6c (the array built by `FUN_00448160`). So the id of every file is EXACT now. Script: `/tmp/claude-0/pe_ids.py`, raw output `scratchpad/res_ids.txt`.
* I also disassembled react (0x467a00) to confirm argument order, and looked briefly at `sound.dll` (game dir, `Program_Files_(ENGLISH)/sound.dll`) for the pitch and delay consumers. That part is incomplete (section 5).
* Line numbers below are lines of the decompile in this scratchpad (they differ from the numbers in DECODE_VOICES.md).

## 1. Summary of findings that change the port

1. Emotion banks are now EXACT (section 3): Happy = 0xd2, Sad = 0xdc, Success = 0xe6, Failure = 0x10.
   * Type 1: normal golfer plays HAPPY (0xd2 + k); golfer with flag 0x20000 plays SUCCESS (0xe6 + k).
   * Types 2, 3, 8: normal golfer plays SAD (0xdc + k); golfer with flag 0x20000 plays FAILURE slot (0x10 + k), which is shadowed by other clips (section 3.3).
   * The port (sgview.cpp 1147) has these reversed: it uses Success/Failure for the normal case and Happy/Sad for the "after" case.
2. File names the exe really asks for differ from the port: male PLS, KLS and SSS use the `PLS ... mix`, `MKLS ... mix`, `SSS ... mix` files, not `MalePLS*`, `MaleKLS*`, `MaleSSS*`. Only male PSS and the four female classes use the `Male...`/`Female...` names (section 3.2).
3. Types 12 and 13 play id 8 + g, delay 500 ms. Slot 9 (male) is `mWATER0`. Slot 8 (female) is registered twice; the first registrant `Ball Tree Leaves.wav` very likely wins, so a female golfer plays the leaves sound effect, and `fWATER0` is shadowed (DERIVED). `OOPS0` is not used by types 12 or 13; its slots 0x52/0x53 have no direct caller (section 4).
4. Pan, volume and pitch jitter formulas are EXACT (section 2). Pitch unit and start-delay consumption are in `sound.dll` and are not fully traced (section 5).

## 2. Positional helper `FUN_0040c500(id, x, y, extra)` (decompile line 10770; EXACT)

Call shape (confirmed in the disassembly of react): the caller pushes `extra, y, x`, then calls `FUN_0046c940(g)` (cdecl, one arg, `add esp,4` afterwards, so it leaves x, y, extra on the stack) and pushes `id = g + base` on top. So every voice call is `FUN_0040c500(base + g, golferX, golferY, delay)`. Decompile prints this oddly (args attached to `FUN_0046c940`).

Steps:
1. `FUN_0042fb90(x, y, &sx, &sy, 0)` projects world (1/1024 tile units) to screen. If the zoomed-render flag `DAT_005a9cbc` is nonzero: `sx = sx*2 - 400`, `sy = sy*2 - 400`.
2. If `extra != -1`: silent unless `0 <= sx <= 799` and `0 <= sy <= 500` (fixed 800 x 500 test even if the window is 1024 or 1280 wide; `DAT_00822c8c` is the screen width, 0x400 or 0x500 or 800). If `extra == -1` the point is clamped (sx 0..799, sy 0..599), delay 0. Voices never pass -1.
3. Volume (0..127 field, `& 0x7f` in `FUN_00485140`): `vol = (|sx - 400| >> 4) + 50 - (zoom <= 3 ? 40 : 0)`. `zoom = DAT_004c2844` (1..4; set to 4 in play at lines 30590 and 64603, to 2 at 64574). So at zoom 4: 50 at screen centre to 74 at the far edges; at zoom <= 3: 10 to 34. It GROWS with distance from the centre (EXACT arithmetic; odd but real).
4. Pan: `pan = sx * 127 / 800 - 64` (C integer division), clamped to -64..63 by `FUN_004847f0` (and again in sound.dll at 0x1002b4a0). Left edge -64, centre about -1 or 0 (sx=400 gives 63-64 = -1), right edge 62.
5. Pitch: `pitch = 300 - FUN_0045c1e0(600)` fresh on EVERY positional play. `FUN_0045c1e0(n)` = float random * n then `__ftol` (so uniform integer 0..n-1, DERIVED from the float call). Range -299..+300. Clamped to -1200..1200 in `FUN_00484f40` and in sound.dll (0x1002bff0: bounds 0xfffffb50 and 0x4b0).
6. `extra` (delay, see 5.2) is passed as the last arg of `FUN_004481b0(id, vol, pan, pitch, extra)`, which calls pitch setter, volume setter, pan setter, extra setter (`FUN_004846b0`), then play (`FUN_00484940`). Order: pitch, volume, pan, extra, play.
7. A second branch taken when `DAT_005a9cd8 != 0` (vol from `sy/12`, pitch from a 36-entry table at 0x4c2fa4) is dead as far as the text shows: `DAT_005a9cd8` is only written 0 in this function (lines 10811 to 10823). EXACT absence of a writer in the decompile; not checked in the binary.

Extra values used with voices (all calls in react): 0, 500 (types 12 and 13), 0 or 1000 (types 2 and 3, see 4), 1500 (partner reply in the story code `FUN_00466370`, decompile lines 69340 to 69354, 0x5dc).

## 3. Emotion banks

### 3.1 Which bank is which (EXACT from exe bytes plus react)

react, golfer record dword at +0x10 (`DAT_005794c8`, address 0x5794c8 + g*0x100): call this `F`. Bit 0x20000 of F = `upset` (see 3.4).
`k = ((template[+3]) >> 4) & 0xf`, plus 5 when `FUN_0046c940(g) == 0` (female). Template array base 0x4d60a8, stride 0x230, indexed by the short at golfer record `DAT_0057956e`. `FUN_0046c940(g) = (~template[+1] >> 7) & 1` so female (bit 7 set) returns 0, male returns 1.

| Type | Condition | id | Bank | Delay |
|---|---|---|---|---|
| 1 | F bit 0x20000 clear | 0xd2 + k | HAPPY | 0 |
| 1 | F bit 0x20000 set | 0xe6 + k (0xd2 + 0x14) | SUCCESS | 0 |
| 2 | bit clear | 0xdc + k | SAD | 1000 if the landing tile (terrain byte at 0x5722e8 [(x>>10)*50 + (y>>10)]) is 0x11 (water), else 0 |
| 3 | same as 2 | same | same | same |
| 2, 3 | bit set | 0x10 + k (0xdc - 0xcc) | FAILURE slot (shadowed, 3.3) | same |
| 8 | bit clear | 0xdc + k | SAD | 0 |
| 8 | bit set | 0x10 + k | FAILURE slot (shadowed) | 0 |

Disassembly check: type 1 at 0x467c58 (`and ecx,0x20000; neg; sbb; and ecx,0x14; add ecx,0xd2; add ecx,esi`), types 2/3/8 at 0x467d50, 0x467d8b, 0x467dc8, join at 0x467ddd (`and cl,0x34; add ecx,0xdc` on a 0xffffffxx value, giving 0xdc - 0xcc = 0x10 when set).

Slot order inside a bank (exe bytes): male PLS = +0, KLS = +1, PSS = +2, SSS = +3; female PLS = +5, SSS = +6, PSS = +7, SkTT = +8. Slot +4 is unused. So k 0..3 male, 5..8 female; the port's `kM`/`kF` order is right.

### 3.2 Exact file the exe requests for each slot (paths are case-insensitive on Windows; use the disc spelling)

| k | Happy (0xd2+) | Sad (0xdc+) | Success (0xe6+) | Failure (0x10+) |
|---|---|---|---|---|
| 0 male PLS | `PLS Happy mix.wav` | `PLS Sid mix.wav` (typo; disc has `PLS Sad mix.wav`) | `PLS Success mix.wav` | `PLS Failure mix.wav` |
| 1 male KLS | `MKLS Happy Mix.wav` | `MKLS Sad mix.wav` | `MKLS Success mix.wav` | `MKLS Failure mix.wav` |
| 2 male PSS | `MalePSSHappy.wav` | `MalePSSSad.wav` | `MalePSSSuccess.wav` | `MalePSSFailure.wav` |
| 3 male SSS | `SSS Happy mix.wav` | `SSS Sad mix.wav` | `SSS Success mix.wav` | `SSS Failure mix.wav` |
| 5 female PLS | `FemalePLSHappy.wav` | `FemalePLSSad.wav` | `FemalePLSSuccess.wav` | `FemalePLSFailure.wav` |
| 6 female SSS | `FemaleSSSHappy.wav` | `FemaleSSSSad.wav` | `FemaleSSSSuccess.wav` | `FemaleSSSFailure.wav` |
| 7 female PSS | `FemalePSSHappy.wav` | `FemalePSSSad.wav` | `FemalePSSSuccess.wav` | `FemalePSSFailure.wav` |
| 8 female SkTT | `FemaleSkTTHappy.wav` | `FemaleSkTTSad.wav` | `FemaleSkTTSuccess.wav` | `FemaleSkTTFailure.wav` (disc spelling `FemaleSKTT*`) |

All under `Sounds/Emotion/`. Never requested by the exe (so the original never plays them; EXACT): `MalePLS*`, `MaleKLS*`, `MaleSSS*`, `FM * mix`, `FemaleSKSS*`. In particular the port's current male PLS/KLS/SSS names are the wrong recordings.

Male PLS Sad: the exe asks for `PLS Sid mix.wav`, which is not on the disc, so that slot has no buffer. A male PLS golfer therefore has NO sound for types 2, 3, 8 in the normal state (DERIVED: `FUN_00484c20` returns an error and `FUN_00484940` returns 0x14 without playing when no buffer exists; sound.dll error handling not read). Faithful port: stay silent. Cosmetic port: use `PLS Sad mix.wav`.

### 3.3 The Failure bank is shadowed by earlier registrations (DERIVED, fairly strong)

Slots 0x10 to 0x18 are registered twice. Registration order matters: `FUN_00484a40` (object vtable +0x88, confirmed from the vtable bytes at 0x4bacf8) copies the new name, but only loads a file `if (obj[0x10] == 0)` (`FUN_00484c20`, returns 0xc if a buffer already exists). So the FIRST registrant keeps the audio buffer. The voice and effect registrations come before the emotion block (reg #17 to #24 and #104 versus #172 to #200), therefore:

| Slot | Plays (first registrant) | The Failure clip that never loads |
|---|---|---|
| 0x10 | fHARD0 | PLS Failure mix (male PLS) |
| 0x11 | mHARD0 | MKLS Failure mix (male KLS) |
| 0x12 | fEASY0 | MalePSSFailure (male PSS) |
| 0x13 | mEASY0 | SSS Failure mix (male SSS) |
| 0x15 | mTRICKY0 | FemalePLSFailure |
| 0x16 | fBLIND | FemaleSSSFailure |
| 0x17 | mBLIND | FemalePSSFailure |
| 0x18 | `effects\boing.wav` | FemaleSkTTFailure |

Supporting evidence for "first wins": the UI refusal beep uses id 0x18 (decompile lines 9141, 9558, 12413 and others) and the ball-thud at line 21720 uses 0x18 too; "boing" fits, a female sports-talk failure clip does not. Same for 0x19 = `cash` (line 17551). I did not execute the game, so this stays DERIVED.

Consequence: an upset golfer (flag 0x20000) reacting to type 2, 3 or 8 plays HARD/EASY/TRICKY/BLIND voices or the boing effect, picked by body class. A faithful port can do exactly that; a "what the designers meant" port can play the Failure clips from the table in 3.2. Both are defensible; say which one you pick in the code comment. Types 4, 5, 6 and 65 use the voice registrants themselves (fHARD0 etc.), which are the winners, so they behave as DECODE_VOICES.md says. Only the emotion Failure slots lose.

### 3.4 What flag 0x20000 is (WEAK naming, EXACT sites)

Bit 17 of the golfer dword `DAT_005794c8`. Set at: decompile 8232 (inside `FUN_00407e00`, followed by react(type 1, loc 1)), 20588 (tantrum, type 0x23, 50 percent set, 50 percent cleared by `& 0xfffdffff`), 21857 (right after react types 2 or 3 for a bad terrain, with the pose), 57544 and 57545 (both golfers of a pair). Cleared at: 22048 (golfer reset to the ball position when flag 0x200 set), 62468 and 62470 (`FUN_0045eXXX` pair routine: both golfers of a pair at the end of a partner chat). Reading: "golfer is in an upset or heightened state", not a pure "last reaction was bad" flag. The port uses `g.rx.polarity() == 2` as a stand-in; I did not prove the two agree. A more faithful port keeps a per-golfer `upset` bit set by the sites above.

## 4. Reaction types 12 and 13 (EXACT ids; file for female DERIVED)

react cases 0xc and 0xd share one body (0x467f.. region; decompile 71110 to 71116): `id = FUN_0046c940(g) + 8`, `FUN_0040c500(id, golferX, golferY, 500)`. Female g = 0 gives id 8; male g = 1 gives id 9. The delay argument is 500 (ms, see 5.2).

* id 9 = `SimsFX/Male/mWATER0.wav` (EXACT, only registrant).
* id 8 = registered twice: first `Golf sfx/Ball Tree Leaves.wav` (reg #16), later `SimsFX/Female/fWATER0.wav` (reg #63). By the first-wins rule in 3.3 the audio at id 8 is Ball Tree Leaves. Corroboration: the random tree-hit sound at decompile 21605 and 21703 is `6 + rand(3)`, i.e. ids 6, 7, 8, and the three tree files are registered as reg #14, #15, #16 (Ball Tree, Ball Tree2, Ball Tree Leaves). So id 8 is intended as the third tree sound. DERIVED.
* Where they fire: type 12 right after the tree-hit sound effect (decompile 21608, `FUN_00467a00(g, 0xc, ...)`); type 13 right after the splash effect id 5 (decompile 21861 to 21863, terrain 0x11). So the voice follows its impact sound by 500 ms.
* Port change: types 12 and 13 should use `mWATER0` for male and, for a faithful result, `Sounds/Golf_Sfx/Ball Tree Leaves.wav` for female (or `fWATER0` if you prefer intent). `OOPS0` is wrong for both. `OOPS0` ids are 0x52 (f) and 0x53 (m); no direct caller found for them (the react switch has none; computed ids elsewhere not exhaustively excluded).

## 5. Open items inside `sound.dll`

### 5.1 Pitch unit (UNKNOWN)
Game side clamps to +-1200 (`FUN_00484f40`, exe) and sound.dll clamps again at 0x1002bff0, then forwards to buffer vtable +0x9c. A +-1200 range fits CENTS (one octave), which would make the jitter 2^(p/1200) = 0.84 to 1.19 playback ratio. I did not find the code in sound.dll that turns the value into a DirectSound frequency (no 1200.0 constant in the DLL). Treat "cents" as WEAK. Port: `rate = pow(2, (300 - rand(600)) / 1200)` is the best available guess.

### 5.2 Delay (`extra`) (DERIVED, not traced to the consumer)
The wrapper stores `extra` at sound-object +0x34 (`FUN_004846b0`) and calls buffer vtable +0x4c; sound.dll has the matching setters (0x1002b2c0 stores +0x34 and forwards; 0x1000ee30 stores +0x34). I did not find where a play reads +0x34. The reading as a start delay in milliseconds rests on the values (0, 500, 1000, 1500) and on the call sites (splash at 0 then voice at 500; story initiator at 0, partner at 1500; water lie at 1000). The DLL imports `timeSetEvent`, consistent with a timer-based delayed start. WEAK to DERIVED. If the port needs certainty, trace `Wave_Buffer::play` in sound.dll.

### 5.3 Volume mapping (UNKNOWN beyond range)
Volume 0..127 goes to buffer vtable +0x40. How it maps to DirectSound attenuation is not read. Port: `gain = vol / 127` is the simplest guess; the original values are quiet (10 to 74 of 127), so a single constant like 0.55 or 0.8 hides the zoom and distance effect.

## 6. Voice slot table from the exe bytes (EXACT)

Registration number (reg #) is the position in `FUN_00448220`. "COLLISION" rows: the first registrant keeps the buffer (3.3).

| Id | File(s) | Note |
|---|---|---|
| 0x08 | sounds\Golf sfx\Ball Tree Leaves.wav (reg #16) ; simsfx\Female\fWATER0.wav (reg #63) | COLLISION |
| 0x09 | simsfx\Male\mWATER0.wav (reg #37) |  |
| 0x0a | simsfx\Female\fHAPPY0.wav (reg #45) |  |
| 0x0b | simsfx\Male\mHAPPY0.wav (reg #19) |  |
| 0x0c | simsfx\Female\fHAPPY1.wav (reg #46) |  |
| 0x0d | simsfx\Male\mHAPPY1.wav (reg #20) |  |
| 0x0e | sounds\Female\Good Shot FM2.wav (reg #43) |  |
| 0x0f | sounds\Male\Good Shot M2.wav (reg #44) |  |
| 0x10 | simsfx\Female\fHARD0.wav (reg #47) ; sounds\emotion\PLS Failure mix.wav (reg #172) | COLLISION |
| 0x11 | simsfx\Male\mHARD0.wav (reg #21) ; sounds\emotion\MKLS Failure mix.wav (reg #176) | COLLISION |
| 0x12 | simsfx\Female\fEASY0.wav (reg #48) ; sounds\emotion\MalePSSFailure.wav (reg #180) | COLLISION |
| 0x13 | simsfx\Male\mEASY0.wav (reg #22) ; sounds\emotion\SSS Failure mix.wav (reg #184) | COLLISION |
| 0x14 | simsfx\Female\fTRICKY0.wav (reg #49) |  |
| 0x15 | simsfx\Male\mTRICKY0.wav (reg #23) ; sounds\emotion\FemalePLSFailure.wav (reg #188) | COLLISION |
| 0x16 | simsfx\Female\fBLIND.wav (reg #50) ; sounds\emotion\FemaleSSSFailure.wav (reg #192) | COLLISION |
| 0x17 | simsfx\Male\mBLIND.wav (reg #24) ; sounds\emotion\FemalePSSFailure.wav (reg #196) | COLLISION |
| 0x18 | sounds\effects\boing.wav (reg #104) ; sounds\emotion\FemaleSkTTFailure.wav (reg #200) | COLLISION |
| 0x3c | simsfx\Female\fCOKE0.wav (reg #51) |  |
| 0x3d | simsfx\Male\mCOKE0.wav (reg #25) |  |
| 0x3e | simsfx\Female\fBAD0.wav (reg #52) |  |
| 0x3f | simsfx\Male\mBAD0.wav (reg #26) |  |
| 0x40 | simsfx\Female\fSAD2.wav (reg #53) |  |
| 0x41 | simsfx\Male\mSAD2.wav (reg #27) |  |
| 0x42 | simsfx\Female\fCRAB0.wav (reg #54) |  |
| 0x43 | simsfx\Male\mCRAB0.wav (reg #28) |  |
| 0x44 | simsfx\Female\fWAITING0.wav (reg #55) |  |
| 0x45 | simsfx\Male\mWAITING0.wav (reg #29) |  |
| 0x46 | simsfx\Female\fMAD0.wav (reg #56) |  |
| 0x47 | simsfx\Male\mMAD0.wav (reg #30) |  |
| 0x48 | simsfx\Female\fBENCH0.wav (reg #57) |  |
| 0x49 | simsfx\Male\mBENCH0.wav (reg #31) |  |
| 0x4a | simsfx\Female\fSENIC0.wav (reg #58) |  |
| 0x4b | simsfx\Male\mSENIC0.wav (reg #32) |  |
| 0x4c | simsfx\Female\fLOVELY0.wav (reg #59) |  |
| 0x4d | simsfx\Male\mLOVELY0.wav (reg #33) |  |
| 0x4e | simsfx\Female\fCELEB0.wav (reg #60) |  |
| 0x4f | simsfx\Male\mCELEB0.wav (reg #34) |  |
| 0x50 | simsfx\Female\fWET0.wav (reg #61) |  |
| 0x51 | simsfx\Male\mWET0.wav (reg #35) |  |
| 0x52 | simsfx\Female\fOOPS0.wav (reg #62) |  |
| 0x53 | simsfx\Male\mOOPS0.wav (reg #36) |  |
| 0x56 | simsfx\Female\fUGLY0.wav (reg #64) |  |
| 0x57 | simsfx\Male\mUGLY0.wav (reg #38) |  |
| 0x58 | simsfx\Female\fSTORY0.wav (reg #65) |  |
| 0x59 | simsfx\Male\mSTORY0.wav (reg #39) |  |
| 0x5a | simsfx\Female\fSTORYNO.wav (reg #66) |  |
| 0x5b | simsfx\Male\mSTORYNO.wav (reg #40) |  |
| 0x5c | simsfx\Male\mGREETING.wav (reg #83) |  |
| 0x5d | sounds\Celebs\Ranger.wav (reg #84) |  |
| 0x5e | simsfx\Male\mCRABGRASS2.wav (reg #85) |  |
| 0x5f | sounds\Celebs\Tray girl.wav (reg #86) |  |
| 0x60 | sound\Celebs\Politician Like.wav (reg #87) |  |
| 0x61 | sounds\Celebs\Marshal Male.wav (reg #88) |  |
| 0x62 | sounds\Celebs\Lawn Technician.wav (reg #89) |  |
| 0x63 | simsfx\Female\fHAVEADRINK.wav (reg #90) |  |
| 0x64 | sounds\Celebs\mGREETING.wav (reg #91) |  |
| 0x65 | sounds\Celebs\Politician Dislike.wav (reg #92) |  |
| 0x66 | sounds\Celebs\Action Star Dislike.wav (reg #93) |  |
| 0x67 | sounds\Celebs\Politician Like.wav (reg #94) |  |
| 0x68 | sounds\Celebs\Spears Like.wav (reg #95) |  |
| 0x69 | sounds\Celebs\Spears Dislike.wav (reg #96) |  |
| 0x6a | sounds\Celebs\Fitness Female.wav (reg #97) |  |
| 0x6b | sounds\Celebs\Tray Girl.wav (reg #98) |  |
| 0x96 | simsfx\female\fVARIETY.wav (reg #76) |  |
| 0x97 | simsfx\Male\mVARIETY.wav (reg #69) |  |
| 0x98 | simsfx\female\fSAME.wav (reg #77) |  |
| 0x99 | simsfx\Male\mSAME.wav (reg #70) |  |
| 0x9a | simsfx\female\fBADHOLE.wav (reg #78) |  |
| 0x9b | simsfx\Male\mBADHOLE.wav (reg #71) |  |
| 0x9c | simsfx\female\fHUNGRY.wav (reg #79) |  |
| 0x9d | simsfx\Male\mHUNGRY.wav (reg #72) |  |
| 0x9e | simsfx\female\fTHIRSTY.wav (reg #80) |  |
| 0x9f | simsfx\Male\mTHIRSTY.wav (reg #73) |  |
| 0xa0 | simsfx\female\fTIRED.wav (reg #81) |  |
| 0xa1 | simsfx\Male\mTIRED.wav (reg #74) |  |
| 0xa2 | simsfx\female\fOPTIONS.wav (reg #82) |  |
| 0xa3 | simsfx\Male\mOPTIONS.wav (reg #75) |  |
| 0xa4 | simsfx\Female\fSTORYOK.wav (reg #67) |  |
| 0xa5 | simsfx\Male\mSTORYOK.wav (reg #41) |  |
| 0xa6 | simsfx\Female\fSTORYYES.wav (reg #68) |  |
| 0xa7 | simsfx\Male\mSTORYYES.wav (reg #42) |  |
| 0xd2 | sounds\emotion\PLS Happy mix.wav (reg #169) |  |
| 0xd3 | sounds\emotion\MKLS Happy Mix.wav (reg #173) |  |
| 0xd4 | sounds\emotion\MalePSSHappy.wav (reg #177) |  |
| 0xd5 | sounds\emotion\SSS Happy mix.wav (reg #181) |  |
| 0xd7 | sounds\emotion\FemalePLSHappy.wav (reg #185) |  |
| 0xd8 | sounds\emotion\FemaleSSSHappy.wav (reg #189) |  |
| 0xd9 | sounds\emotion\FemalePSSHappy.wav (reg #193) |  |
| 0xda | sounds\emotion\FemaleSkTTHappy.wav (reg #197) |  |
| 0xdc | sounds\emotion\PLS Sid mix.wav (reg #170) |  |
| 0xdd | sounds\emotion\MKLS Sad mix.wav (reg #174) |  |
| 0xde | sounds\emotion\MalePSSSad.wav (reg #178) |  |
| 0xdf | sounds\emotion\SSS Sad mix.wav (reg #182) |  |
| 0xe1 | sounds\emotion\FemalePLSSad.wav (reg #186) |  |
| 0xe2 | sounds\emotion\FemaleSSSSad.wav (reg #190) |  |
| 0xe3 | sounds\emotion\FemalePSSSad.wav (reg #194) |  |
| 0xe4 | sounds\emotion\FemaleSkTTSad.wav (reg #198) |  |
| 0xe6 | sounds\emotion\PLS Success mix.wav (reg #171) |  |
| 0xe7 | sounds\emotion\MKLS Success mix.wav (reg #175) |  |
| 0xe8 | sounds\emotion\MalePSSSuccess.wav (reg #179) |  |
| 0xe9 | sounds\emotion\SSS Success mix.wav (reg #183) |  |
| 0xeb | sounds\emotion\FemalePLSSuccess.wav (reg #187) |  |
| 0xec | sounds\emotion\FemaleSSSSuccess.wav (reg #191) |  |
| 0xed | sounds\emotion\FemalePSSSuccess.wav (reg #195) |  |
| 0xee | sounds\emotion\FemaleSkTTSuccess.wav (reg #199) |  |

Facts that correct DECODE_VOICES.md section 4 and 9:
* Pairs are (female, male) = (even, odd) and match the doc for ids 0x10 to 0x5b and 0x96 to 0xa7.
* `HAPPY0` = 0x0a/0x0b, `HAPPY1` = 0x0c/0x0d, `Good Shot FM2` = 0x0e, `Good Shot M2` = 0x0f. No play call with these ids exists in react or elsewhere in the decompile (a grep of all literal ids passed to `FUN_0040c500` and `FUN_004481b0` found none), so they are loaded but never played (EXACT absence in the text).
* `WATER0` is at 0x08/0x09 (not 0x54/0x55); `OOPS0` 0x52/0x53 has no caller; 0x54/0x55 are unused.
* Staff block 0x5c to 0x67 (staff voices in `FUN_00402a40`, decompile 1587 to 1645, ids `0x5c..0x5f + iVar13`, where DECODE_VOICES.md reads the offset as 0, 4 or 8 by staff kind; I did not re-verify the offset): 0x5c mGREETING, 0x5d Ranger, 0x5e mCRABGRASS2, 0x5f Tray girl, 0x60 Politician Like (path typo `sound\Celebs`), 0x61 Marshal Male, 0x62 Lawn Technician, 0x63 fHAVEADRINK, 0x64 Celebs\mGREETING, 0x65 Politician Dislike, 0x66 Action Star Dislike, 0x67 Politician Like. Which staff kind uses which group is DERIVED only.
* Swing ids: 0xbc Tiger Bounce 3, 0xbd Tiger Swing 1, 0xbe Golf Swing Perfect M, 0xbf NoAcc, 0xc0 NoImag M, 0xc1 Perfect FM (EXACT), 0xc7/0xc8 bass up/down 2.

## 7. Recommended port behaviour (speakVoice, sgview.cpp ~1142)

1. Types 1, 2, 3, 8: use the table in 3.2 with the bank rule in 3.1. `upset` bit instead of `polarity()==2` if you can model the sites in 3.4.
2. Types 12, 13: male `simsfx/male/mWATER0.wav`; female `Sounds/Golf_Sfx/Ball Tree Leaves.wav` (faithful) or `simsfx/female/fWATER0.wav`; delay 500 ms.
3. Delay: 1000 ms for types 2 and 3 when the terrain under the ball is water (0x11); 500 ms for 12 and 13; 1500 ms for the partner reply; else 0.
4. Pan = `clamp(sx*127/800 - 64, -64, 63) / 64` (normalised), where sx is the screen x in an 800 px frame; silent when the golfer is outside 0..799 by 0..500 on screen.
5. Volume byte = `(|sx-400|>>4) + 50 - (zoom<=3 ? 40 : 0)`, gain = byte/127.
6. Pitch jitter = uniform integer in -299..+300 per play, unit probably cents.
