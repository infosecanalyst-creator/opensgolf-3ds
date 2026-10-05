# DECODE_BODIES: golfer palettes, body sheets, sprites and head offsets

Scope: how a golfer is recoloured and drawn. Sources: the decompile text (function names below), the repo docs, and PIL/FLC measurements of disc art. No executable was opened. Everything is in my own words.

Tags: EXACT = read directly from the decompile or measured value. DERIVED (how) = inferred. UNKNOWN = not established.

## 1. Palette objects and the draw queue

- EXACT: palette objects are 0x58 bytes each, array starting at 0x81ca10, palette pointer at +4. Object 0 (loaded from MaleSwap01) is the shared working palette, recomposed just before each golfer draw. Objects 1..10 are MaleSwap01..10. Object 0x8c+n is FemaleSwap(n+1). Objects 0x82..0x89 are the employee palettes in order Greeter, Ranger, GK, TrayGirl, GolfCeleb, Marshall, LawnTech, SodaVendor. Objects 0x78 and up are animal palettes.
- EXACT: draw flags. Value 0x100|slot means compose the palette for golfer slot, then draw with object 0. 0x200 draws a scaled ghost sprite with object (flags & 0x1ff). 0x400 and 0x800 are tint variants (constants 0x80007fff and 0x80007c00). Any other value is used directly as the object index. So the Customise flag 0x199 is "compose slot 0x99", not object 0x199.
- EXACT: the queue is FUN_004628d0 (and FUN_00462a30, sorted by depth) taking x, y, depth, box w, box h, sprite, zoom, flags. FUN_00462be0 draws entries, FUN_00463100 walks the depth sorted list, FUN_004741b0 is the sprite plus palette object blit.
- EXACT: palette entries are 3 byte RGB. Index 255 is the key colour (255,0,255) and is skipped when drawing. The embedded palette inside each FLC equals none of the Swap files and is replaced at draw time (DERIVED: measured equality test of FLC palette against all Swap files).
- EXACT: golfer sprites index only 0..106 plus 255 (measured with the FLC decoder over every Male, Female and Celebs file). Employee FLCs use indices up to 247/254 and carry their own PCX palettes.

## 2. The composer FUN_00462020

Input: a golfer slot number. Output: object 0 rewritten from the Swap objects.

### 2.1 Order of operations (EXACT, with ranges cross-checked against measured Swap variation)

1. Copy entries 0..119 of object 0 from Swap01 of the chosen gender (male objects 1.., or FemaleSwap01).
2. Overwrite 20 entry groups from the file chosen by a value: 0..19 (shirt), 20..39 (pants), 40..59 (skin), 60..79 (hat or celeb hair).
3. Overwrite 80..89 (10 entries) and 90..96 (7 entries) from two more value choices.
4. Tail 144..171 (28 entries): from FemaleSwap02 if ((byte at 0x5849e0 + identity*0x2c + 2) & 7) is greater than 3, else FemaleSwap01. Applies to both genders; identity slots at or above 0x98 use Swap01. UNKNOWN: which art uses 144..171 (golfer sprites do not index it; the two files differ there, gold ramp vs grey).
5. Celeb or non golfer branch (slot >= 0x9c): extra writes at 107..121, 122..131 and 132..135. Male swaps are black there. DERIVED: practically irrelevant for drawing.

### 2.2 Value to file mapping

- EXACT: male value v (0..9) uses MaleSwap(v+1), object v+1. Female value v uses object v+0x8c, which is FemaleSwap(v+1). The composer adds 0x8b to female values then the object table adds the object base (EXACT arithmetic, DERIVED sign of base from the table layout at 0x81fa30).
- So colour values 0..9 are files Swap01..Swap10 for both genders.

### 2.3 What each range is (DERIVED: rendered each range of every Swap with PIL and compared with body sheets)

| entries | meaning | notes |
|---|---|---|
| 0..19 | shirt ramp | 20 steps, light to dark |
| 20..39 | pants ramp | light to dark |
| 40..59 | skin ramp | Swap01..04 are skins 0..3 light to dark. Swap05..10 are black (never used for skin). Female Swap06 is aqua (unused). |
| 60..79 | hat ramp | celebs use it as hair: value hair+5 gives files 6..10 (silver, blond, orange, brown, black) |
| 80..89 | male: only 80..86 vary (alt skin / hands), 87..89 constant. Female: hair, 6 colours | male Swap01..04 skins, Swap05 pale. Female Swap01..06 blonde, red, auburn, grey, white, orange |
| 90..96 (98) | male: constant 90..99. Female: alt skin / hands | female Swap01..04 skins, 05 pale, 06 yellow |
| 100..119 | constant | male hair is baked into 100..106; female 100..116 differs only in Swap06 |
| 250..255 | identical in all swaps | 255 is key (255,0,255) |

DERIVED: which composer variable feeds 80..89 and 90..96 per gender comes from the variation pattern, not confirmed in code. UNKNOWN: exact role of male 87..99 constants beyond "shared".

### 2.4 Tag legend vs measured ramps

- EXACT: shirt colour names in the progolfers.dta header match the measured 0..19 ramps exactly with file = value+1: white, yellow, orange, peach, red, green, teal, blue, purple, black.
- EXACT: pants names (black, blue, light blue, green, light green, brown, red, tan, yellow, white) match the measured 20..39 ramps with file = value+1. Skin names (caucasian, asian/tanned, latino, black) match Swap01..04.
- EXACT: the hat legend in the file header (black, lblue, blue, lgreen, green, orange, red, yellow, ltyellow, white) does NOT match the measured 60..79 ramps. Measured male: white-grey, red, blue, lgreen, green, silver, pale yellow, orange, brown, dark. Measured female: white, yellow, orange, pink, red, red-pink, aqua, blue, purple, black. DERIVED: the legend text is stale or index reversed; trust the measured ramps. UNKNOWN which is intended.

### 2.5 Inputs to the composer

- EXACT: slot < 0x9c: identity is the short at slot record +0xb6. Gender comes from FUN_0046c940 (profile +0x21 bit 7 set means female).
- EXACT defaults when the profile flag dword at +0x2c is 0: pants = id % 10; shirt = (id*3) % 10 male, (id*2) % 3 + 1 female; skin and hair from the per head default table at DAT_004d55e8 (stride 0x44, +0 skin, +3 hair, female uses entry head+0x14); hat selector = shirt if slot toggle bit 4 is set, else 0 male or 9 female; alt skin = skin, or 4 when toggle bit 2 is set.
- EXACT when the dword is non zero: +0x24 shirt, +0x25 pants, +0x26 alt skin, +0x27 skin, +0x28 hair, +0x23 low nibble hat selector, +0x23 bits 4..5 body type. Unless the slot is a celeb or scratch pseudo slot, the composer writes the resolved values back into the profile.
- EXACT: slot type byte (type & 0xE0) == 0x20 (pro or player): values come from the pro table row (base 0x58dd50, stride 0x38, row index from slot byte +0xa3): +0x20 body, +0x21 skin, +0x22 hat, +0x23 shirt, +0x24 pants; alt skin forced to 4. The decompile forces the male object path even for female pros. UNKNOWN whether that is real behaviour or a decompile artifact. UNKNOWN who sets +0xa3 for slot 0x98.
- EXACT: slot >= 0x9c: celebrity record n = slot - 0x9c at 0x55d758, stride 0x25: +0 type 0..10 (anim set), +1 skin, +2 hair (hat ramp value hair+5), +4 pants. Male object path, alt skin 4. Shirt at +3 is not visible in the decompile (UNKNOWN). Spectators (FUN_004011e0 creates, FUN_004017d0 draws) use slot 0x9c + celebrity id.
- Scratch slots (EXACT): 0x98 player (type 0x20); 0x99 Customise preview and type -6 entity (type byte 0 in Customise; identity at DAT_00582e6e, toggles DAT_00582dd1, row DAT_00582e73); 0x9c scratch for the skill points card.
- UNKNOWN: meaning of toggle bits 1, 2, 4 beyond: bit 4 or bit 2 select the body set, bit 2 also sets alt skin to 4 (pale hands). The label table pointer is DAT_004c2858 (first label begins "length").

## 3. Measured RGB tables

Each table lists the lightest, middle and darkest entry of the ramp for that file. File number is value+1. Measured from the disc PCX files in Bodies/ with PIL (DERIVED from art, exact pixel values).


### Male 0-19 shirt (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (0) | mid (9) | dark (19) |
|---|---|---|---|---|
| MaleSwap01 | 0 | F0F0F0 | B8B8B8 | 707070 |
| MaleSwap02 | 1 | F8F870 | 989038 | 282008 |
| MaleSwap03 | 2 | F88838 | A05818 | 281810 |
| MaleSwap04 | 3 | F8A8A8 | C04848 | 580000 |
| MaleSwap05 | 4 | F82828 | 901010 | 200000 |
| MaleSwap06 | 5 | 58F8B0 | 309050 | 102008 |
| MaleSwap07 | 6 | 60F8F8 | 289898 | 003030 |
| MaleSwap08 | 7 | 70B0F8 | 3850A0 | 101830 |
| MaleSwap09 | 8 | C088F8 | 7840A0 | 180830 |
| MaleSwap10 | 9 | 787878 | 484848 | 080808 |

### Male 20-39 pants (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (20) | mid (29) | dark (39) |
|---|---|---|---|---|
| MaleSwap01 | 0 | 686868 | 404040 | 080808 |
| MaleSwap02 | 1 | 1850D0 | 183078 | 081828 |
| MaleSwap03 | 2 | 70B8D0 | 407088 | 102830 |
| MaleSwap04 | 3 | 38D800 | 188000 | 082008 |
| MaleSwap05 | 4 | B0E8B0 | 58B840 | 206020 |
| MaleSwap06 | 5 | D08030 | 885030 | 281818 |
| MaleSwap07 | 6 | F00808 | 800808 | 280808 |
| MaleSwap08 | 7 | D8C090 | 907848 | 403820 |
| MaleSwap09 | 8 | F8F800 | 908808 | 303010 |
| MaleSwap10 | 9 | F0F0F0 | B8B8B8 | 686868 |

### Male 40-59 skin (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (40) | mid (49) | dark (59) |
|---|---|---|---|---|
| MaleSwap01 | 0 | E8C8A8 | B87860 | 805840 |
| MaleSwap02 | 1 | C0A078 | 987048 | 584828 |
| MaleSwap03 | 2 | B08048 | 683828 | 383020 |
| MaleSwap04 | 3 | 783820 | 301810 | 181010 |
| MaleSwap05 | 4 | 000000 | 000000 | 000000 |
| MaleSwap06 | 5 | 000000 | 000000 | 000000 |
| MaleSwap07 | 6 | 000000 | 000000 | 000000 |
| MaleSwap08 | 7 | 000000 | 000000 | 000000 |
| MaleSwap09 | 8 | 000000 | 000000 | 000000 |
| MaleSwap10 | 9 | 000000 | 000000 | 000000 |

### Male 60-79 hat (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (60) | mid (69) | dark (79) |
|---|---|---|---|---|
| MaleSwap01 | 0 | F8F8F8 | B8B8B8 | 707070 |
| MaleSwap02 | 1 | F84040 | 901010 | 300000 |
| MaleSwap03 | 2 | 3868F8 | 0030B0 | 001048 |
| MaleSwap04 | 3 | 98F898 | 50C850 | 287030 |
| MaleSwap05 | 4 | 50D028 | 286818 | 203018 |
| MaleSwap06 | 5 | D0D0D0 | 808080 | 282828 |
| MaleSwap07 | 6 | F8F090 | C0B050 | 605018 |
| MaleSwap08 | 7 | E86820 | A05020 | 483020 |
| MaleSwap09 | 8 | A87008 | 704820 | 302020 |
| MaleSwap10 | 9 | 707070 | 383838 | 080808 |

### Male 80-89 (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (80) | mid (84) | dark (89) |
|---|---|---|---|---|
| MaleSwap01 | 0 | D8A890 | B08068 | D8D8D8 |
| MaleSwap02 | 1 | 986840 | 885830 | D8D8D8 |
| MaleSwap03 | 2 | 805038 | 604030 | D8D8D8 |
| MaleSwap04 | 3 | 502818 | 382010 | D8D8D8 |
| MaleSwap05 | 4 | E8E0E8 | D0C8C0 | D8D8D8 |
| MaleSwap06 | 5 | 000000 | 000000 | D8D8D8 |
| MaleSwap07 | 6 | 000000 | 000000 | D8D8D8 |
| MaleSwap08 | 7 | 000000 | 000000 | D8D8D8 |
| MaleSwap09 | 8 | 000000 | 000000 | D8D8D8 |
| MaleSwap10 | 9 | 000000 | 000000 | D8D8D8 |

### Male 90-99 (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (90) | mid (94) | dark (99) |
|---|---|---|---|---|
| MaleSwap01 | 0 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap02 | 1 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap03 | 2 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap04 | 3 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap05 | 4 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap06 | 5 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap07 | 6 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap08 | 7 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap09 | 8 | D8D0C8 | C0B8A8 | 787060 |
| MaleSwap10 | 9 | D8D0C8 | C0B8A8 | 787060 |

### Male 144-171 tail
- MaleSwap01: 144=F8F8EA 150=CFCFC4 158=989892 165=5C5B5B 171=000000
- MaleSwap02: 144=FDFFBE 150=EFD053 158=CD990D 165=5F4A0F 171=000000

### Female 0-19 shirt (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (0) | mid (9) | dark (19) |
|---|---|---|---|---|
| FemaleSwap01 | 0 | F8F8F8 | B0B0B0 | 606060 |
| FemaleSwap02 | 1 | F8F8B0 | D0C058 | 807020 |
| FemaleSwap03 | 2 | F8B090 | D87860 | 984030 |
| FemaleSwap04 | 3 | F8B0B0 | E84848 | 802828 |
| FemaleSwap05 | 4 | F83030 | A01010 | 280808 |
| FemaleSwap06 | 5 | 8080E8 | 1010D8 | 181850 |
| FemaleSwap07 | 6 | A0F8F8 | 40B0B0 | 105858 |
| FemaleSwap08 | 7 | A0D8F8 | 4080F8 | 0020A8 |
| FemaleSwap09 | 8 | C8B0F8 | 9058D8 | 482088 |
| FemaleSwap10 | 9 | 787878 | 484848 | 080808 |

### Female 20-39 pants (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (20) | mid (29) | dark (39) |
|---|---|---|---|---|
| FemaleSwap01 | 0 | 606060 | 383838 | 080808 |
| FemaleSwap02 | 1 | 0058F8 | 083898 | 101830 |
| FemaleSwap03 | 2 | 70D8E8 | 5098B0 | 284048 |
| FemaleSwap04 | 3 | 38F000 | 209000 | 082808 |
| FemaleSwap05 | 4 | C0F8A8 | 48B048 | 184020 |
| FemaleSwap06 | 5 | 00A000 | 107000 | 103810 |
| FemaleSwap07 | 6 | F80000 | 981010 | 281010 |
| FemaleSwap08 | 7 | E8C078 | 887848 | 282010 |
| FemaleSwap09 | 8 | F0E878 | A8A038 | 383820 |
| FemaleSwap10 | 9 | F8F8F8 | B0B0B0 | 585858 |

### Female 40-59 skin (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (40) | mid (49) | dark (59) |
|---|---|---|---|---|
| FemaleSwap01 | 0 | E8C8A8 | B87860 | 805840 |
| FemaleSwap02 | 1 | C0A078 | 987048 | 584828 |
| FemaleSwap03 | 2 | B08048 | 683828 | 383020 |
| FemaleSwap04 | 3 | 783820 | 301810 | 181010 |
| FemaleSwap05 | 4 | 000000 | 000000 | 000000 |
| FemaleSwap06 | 5 | 80F8F8 | 38B0B0 | 009090 |
| FemaleSwap07 | 6 | 000000 | 000000 | 000000 |
| FemaleSwap08 | 7 | 000000 | 000000 | 000000 |
| FemaleSwap09 | 8 | 000000 | 000000 | 000000 |
| FemaleSwap10 | 9 | 000000 | 000000 | 000000 |

### Female 60-79 hat (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (60) | mid (69) | dark (79) |
|---|---|---|---|---|
| FemaleSwap01 | 0 | F8F8F8 | B0B0B0 | 606060 |
| FemaleSwap02 | 1 | F8F800 | A8A000 | 302810 |
| FemaleSwap03 | 2 | F88838 | C85840 | 683018 |
| FemaleSwap04 | 3 | F8B8B8 | C06060 | 700000 |
| FemaleSwap05 | 4 | F80000 | 900000 | 180808 |
| FemaleSwap06 | 5 | F84040 | 981818 | 680000 |
| FemaleSwap07 | 6 | 00F8C0 | 109870 | 084038 |
| FemaleSwap08 | 7 | 0078F8 | 003888 | 000828 |
| FemaleSwap09 | 8 | 8838F8 | 5818A0 | 301048 |
| FemaleSwap10 | 9 | 787878 | 383838 | 080808 |

### Female 80-89 (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (80) | mid (84) | dark (89) |
|---|---|---|---|---|
| FemaleSwap01 | 0 | F8E888 | B8A860 | 605028 |
| FemaleSwap02 | 1 | F06030 | B04828 | 583018 |
| FemaleSwap03 | 2 | C87018 | 985018 | 603028 |
| FemaleSwap04 | 3 | 686868 | 404040 | 101010 |
| FemaleSwap05 | 4 | D8D8D8 | A8A8A8 | 707070 |
| FemaleSwap06 | 5 | F88008 | C06008 | 804008 |
| FemaleSwap07 | 6 | 000000 | 000000 | 000000 |
| FemaleSwap08 | 7 | 000000 | 000000 | 000000 |
| FemaleSwap09 | 8 | 000000 | 000000 | 000000 |
| FemaleSwap10 | 9 | 000000 | 000000 | 000000 |

### Female 90-99 (RGB hex: lightest entry / middle entry / darkest entry)

| file | value | light (90) | mid (94) | dark (99) |
|---|---|---|---|---|
| FemaleSwap01 | 0 | D8A890 | B08068 | D8D8D8 |
| FemaleSwap02 | 1 | 986840 | 885830 | D8D8D8 |
| FemaleSwap03 | 2 | 805038 | 604030 | D8D8D8 |
| FemaleSwap04 | 3 | 502818 | 382010 | D8D8D8 |
| FemaleSwap05 | 4 | E8E0E8 | D0C8C0 | D8D8D8 |
| FemaleSwap06 | 5 | F8F820 | E0D010 | D8D8D8 |
| FemaleSwap07 | 6 | 000000 | 000000 | D8D8D8 |
| FemaleSwap08 | 7 | 000000 | 000000 | D8D8D8 |
| FemaleSwap09 | 8 | 000000 | 000000 | D8D8D8 |
| FemaleSwap10 | 9 | 000000 | 000000 | D8D8D8 |

### Female 144-171 tail
- FemaleSwap01: 144=F8F8EA 150=CFCFC4 158=989892 165=5C5B5B 171=000000
- FemaleSwap02: 144=FDFFBE 150=EFD053 158=CD990D 165=5F4A0F 171=000000

Spot check (EXACT): entries 250..255 identical in every Swap; 255 = FF00FF.

## 4. Which body sheet a profile uses

- EXACT: the body type for a member is the 2 bit field at profile +0x23 bits 4..5 (0..3). Pros carry a 0..7 body value in progolfers.dta (body % 8): 0 PLS, 1 KLS, 2 PSS, 3 SSS (male); 4 FemalePLS, 5 FemaleSSS, 6 FemalePSS, 7 FemaleSkTT; bit 2 set means female. In the Standard theme the pro body counts are: type 2 x53, 0 x16, 3 x10, 7 x6, 1 x5, 5 x4, 6 x1, 4 x1 (measured by reading the file).
- EXACT mapping for tournament pros when the profile load fails: +0x23 = hat | (body & 3) << 4; +0x21 = 0x09 male or 0x89 female; +0x24 shirt, +0x25 pants, +0x27 skin from the row; +0x20 trait = 1 << rand(5); toggles byte = 7.
- EXACT animation set `a` (0..8): 0 MalePLS, 1 MaleKLS, 2 MalePSS, 3 MaleSSS, 4 not loaded, 5 FemalePLS, 6 FemaleSSS, 7 FemalePSS, 8 FemaleSkTT. FemaleSkSS FLCs exist on disc but are never loaded. Anim id is the offset in section 6 plus the set index added by the loader.
- EXACT Customise rule, male: toggle value 4 off gives KLS; with 4 on, value 2 on gives PSS, value 2 off gives SSS. With the dword non zero, a = body nibble.
- EXACT Customise rule, female with dword 0: default 7; Young gives 6; trait bit 0x08 gives 8; Mature gives 5. With the dword non zero, a = nibble + 5.
- Body sheet PCX (Bodies/, 60x120 pieces): DERIVED from loader order only that sheet index equals a (male 0..3, female a-5). The sprite pointer is hidden, so UNKNOWN for certain. Sheets 0..3 are full size, 4..7 are `_sm`. DERIVED by PIL: a `_sm` sheet is a 0.73 scale figure (86 vs 118 px high), top aligned in the same 60x120 canvas. UNKNOWN: where `_sm` sheets are used (not found in the decompile). Barrel.pcx is loaded as a separate piece (37 px high, bottom of canvas) with its own palette; its use is UNKNOWN.

## 5. In game sprite selection

- EXACT: sprite accessor FUN_0043d6f0(animId, frame, view) returns a 0x2c byte sprite record. Drawing view = (DAT_005685f4 - facing - 2) & 7, facing being golfer +0x1a (0x5794d2). The camera term matches the existing SPRITES.md convention.
- EXACT: loader FUN_0043d740 loads each FLC. People files have 8 views; Sitting and SitSq have 4 (view mask 0x0F, others 0xFF). Default people crop is a 60x60 window at canvas (210,200) (param_4 = 0); other modes crop larger squares. Canvas is 480x480. Every sprite has a `<name>Shadow.flc` companion.
- UNKNOWN: the in world golfer queue call site was not found in the decompile. Only the spectator call and the held golfer call use the compose flag. Treat in world golfer drawing as: choose anim id by state, frame by timer, set `a` from the profile (section 4), flag 0x100|slot (DERIVED from the Customise and spectator calls).
- Female sets omit PerfectSwing, NoAccSwing and NoimagSwing (EXACT: not present on disc); female NormalSwing takes id offset 0x28.

## 6. FLC inventory per body set

Cells are frames per view / ms per frame. Views are 8 unless marked (4v). Measured with my pure Python FLC decoder (DERIVED from file headers and decoded frame counts after dropping the ring frame). Id offset is EXACT from the loader.

| Animation | id offset | MalePLS | MaleKLS | MalePSS | MaleSSS | FemalePLS | FemaleSSS | FemalePSS | FemaleSkTT | FemaleSkSS |
|---|---|---|---|---|---|---|---|---|---|---|
| NormalWalk | 0x00 | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 16/83ms | 16/83ms | 16/83ms | 16/83ms | 16/83ms |
| PerfectSwing | 0x28 | 20/83ms | 20/83ms | 20/83ms | 20/83ms | - | - | - | - | - |
| NormalSwing | 0x28 (female) | - | - | - | - | 20/83ms | 20/83ms | 20/83ms | 20/83ms | 20/83ms |
| NoAccSwing | 0x32 | 24/83ms | 24/83ms | 24/83ms | 24/83ms | - | - | - | - | - |
| NoimagSwing | 0x37 | 21/83ms | 21/83ms | 21/83ms | 21/83ms | - | - | - | - | - |
| Sitting | 0x3c | 20/83ms (4v) | 20/83ms (4v) | 20/83ms (4v) | 21/83ms (4v) | 20/83ms (4v) | 20/66ms (4v) | 20/83ms (4v) | 19/83ms (4v) | 20/83ms (4v) |
| SitSq | 0x46 | 11/83ms (4v) | 11/83ms (4v) | 11/83ms (4v) | 11/83ms (4v) | 12/83ms (4v) | 11/83ms (4v) | 11/83ms (4v) | 11/83ms (4v) | 11/83ms (4v) |
| Pitch | 0x50 | 13/83ms | 13/83ms | 13/83ms | 13/83ms | 14/83ms | 14/83ms | 14/83ms | 14/83ms | 14/83ms |
| Putt | 0x5a | 34/83ms | 33/83ms | 34/83ms | 33/90ms | 31/83ms | 31/83ms | 31/83ms | 31/83ms | 31/83ms |
| Happy | 0x1e | 29/83ms | 20/83ms | 29/83ms | 22/83ms | 25/83ms | 20/83ms | 21/83ms | 28/83ms | 30/83ms |
| SuccessA | 0xb4 | 20/83ms | 26/83ms | 17/83ms | 21/83ms | 35/83ms | 27/83ms | 22/83ms | 29/83ms | 25/83ms |
| Sad | 0xbe | 20/83ms | 23/83ms | 23/83ms | 19/90ms | 23/83ms | 22/83ms | 21/83ms | 14/83ms | 25/83ms |
| FailureA | 0xc8 | 16/83ms | 26/83ms | 24/66ms | 19/83ms | 29/83ms | 24/83ms | 18/83ms | 23/83ms | 19/66ms |
| Sq | 0x0a | 17/83ms | 14/83ms | 14/83ms | 18/83ms | 23/83ms | 23/83ms | 24/83ms | 24/66ms | 21/83ms |
| LineUpPutt | 0x64 | 29/83ms | 30/83ms | 30/83ms | 27/83ms | 26/83ms | 28/83ms | 27/83ms | 27/83ms | 27/83ms |
| PointAt | 0x6e | 16/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 17/83ms |
| Fidget | 0x14 | 21/83ms | 19/83ms | 16/83ms | 22/83ms | 22/83ms | 13/83ms | 20/83ms | 24/83ms | 19/66ms |
| TiredWalk | 0xd2 | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 19/83ms |
| NormalAddress | 0x78 | 18/66ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms |
| PuttAddress | 0x8c | 18/83ms | 19/83ms | 18/83ms | 18/90ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms |
| PitchAddress | 0x82 | 18/83ms | 19/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms | 18/83ms |
| LeanLeft | 0xa0 | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms |
| LookUp | 0xaa | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms |
| LeanRight | 0x96 | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms | 22/83ms |
| HandShake | 0xdc | 16/83ms | 16/83ms | 16/83ms | 16/83ms | 16/83ms | 16/83ms | 16/83ms | 16/83ms | 16/66ms |
| Cart | 0xe6 | 10/83ms | 10/66ms | 10/83ms | 10/83ms | 7/83ms | 7/83ms | 7/83ms | 7/83ms | 7/83ms |
| Failure (not loaded) | none | - | - | - | - | - | - | - | 21/83ms | - |
| NoAccSwing_Shorten (not loaded) | none | 18/83ms | - | - | - | - | - | - | - | - |
| PerfectSwing_shorten (not loaded) | none | 16/83ms | - | - | - | - | - | - | - | - |

Additional EXACT loader offsets not in the table: Sq 0x0a (Sq is walk-in-place "squat" set), all offsets are added to a per set base so ids do not collide.

Files on disc never loaded (EXACT, by absence from the loader): Failure.flc (FemaleSkTT), the Male directory's `*_SwapPals1..5` and `BasPal` files, Bev_* files, FatGuyStride, Ranger cart files, Celebs Like/DontLike/Interact files, FemaleSkSS.

## 7. Employees, celebrities and animals

- EXACT employee ids: sets 0..7 use anim base 0x20e (Walk), 0x216 (SQ), 0x21e (Action). Employee states: below 0xb walk, 0xb SQ, 0xc action. Palette object = 0x82 + set.
- EXACT FUN_004038f0 entity types: -2..-5 give set = -type-2 (Greeter, Ranger, GK, TrayGirl); with flag 8 it is -type+2 (GolfCeleb, Marshall, LawnTech, SodaVendor). Type -6 is the player scene: set = body nibble (+5 if female), composed from slot 0x99 with profile 0, flag 0x199. The held golfer path (DAT_004c2e10 >= 0x80) draws Happy (anim 0x1e + clamp(body,0,9)) at depth 999 with the composer on that slot. UNKNOWN/suspicious: as decompiled it masks +0x23 with 0xf, clearing the body nibble. Female body for this path is UNKNOWN.
- EXACT celebrity ids: SQ 0x140, Walk 0x15a, Char 0x14d, each plus type 0..10. Animal ids 0x103, 0x10c, 0x115, 0x11e (palette objects 0x78 and up).
- EXACT letter map for celebrity sets 0..10: A ActionStar, B Female_PopSinger, C Politician, D Comedian, E Supermodel, F FitnessFem, G FemComic, H GenMale, I MoviePrincess, J RockStar, K Basketball. Set 11 AgingStar has no letter.
- Celeb colouring: DERIVED from the composer: male object path, hat ramp = hair+5, alt skin 4, skin and pants from the record. The `*_Palette1..5.pcx` files shipped beside the celeb FLCs are never loaded by name (UNKNOWN: not found in the decompile).
- Employees are coloured only by their own palette object (full 256 entries, no composer).

## 8. Applying palettes to FLC sprites

- EXACT: golfer sprites are stored as palette indexed frames. At draw time the queue picks the palette object by the flag rules in section 1, so recolouring means: run the composer, then blit with object 0, skipping index 255.
- Reimplementation recipe (DERIVED): build a 256 entry RGB table from Swap01, overwrite ranges per section 2, draw the frame with that table. Shadows are separate `Shadow.flc` sprites drawn through the normal blend path. UNKNOWN: the shadow blend constant and whether shadows share object 0.

## 9. Heads and offsets

- EXACT (consistent with DECODE_FACES and DECODE_CUSTOMISE): body origin = head origin + (0x13, 0x60) for stock heads (stats card FUN_0045c200 mode 1 draws head at x, y-0x14 and body at x+0x13, y+0x4c). Custom heads are drawn 0x14 further left, so the body offset is (0x27, 0x60).
- EXACT Customise preview: stock head (0x150, 0x14), custom head (0x13c, 0x14), body (0x163, 0x74), accessory (0x160, 0x74). HeadBodyBck window at (336,20) stock, (316,20) custom. Two walking figures at (315,239) and (335,239), box 30x40, zoom 4, frame 5 of NormalWalk views 0 and 4, flag 0x199.
- EXACT stats card: backdrop at (x0+0xf7, 0x56), head via FUN_0045c200(slot, x0+0xff, 0x66, 1).
- Head assignment: data driven. The head byte is profile +0x22. UNKNOWN: the 75 member default profile table is executable data and was not available as text. Members arrive with identity rand 1..75, rejected on id or id % 19 clash. Theme .chr files override profiles 1..n (FUN_004658b0). Special visitors Joe Pro (0x4c), I.M. Picky (0x4d), Ivana Richman (0x4e), J.P. Bigdome (0x4f) load from .glf (FUN_004659a0).

## 10. UNKNOWN list

1. Which composer variable drives 80..89 and 90..96 per gender (pattern only).
2. Use of palette entries 144..171.
3. Hat legend vs measured hat ramps.
4. Whether pro body sheets for female pros really use the male object path.
5. Source of pro row index for slot 0x98.
6. In world golfer draw call site.
7. Where `_sm` body sheets and Barrel.pcx are used.
8. Toggle bit labels.
9. Shadow blend rule.
10. Member default head table.
11. Celebrity shirt field.
