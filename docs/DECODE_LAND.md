# Starting land ownership (golf.exe), research notes

Answer: the decompile DOES contain it. The start is NOT tract based. It is a tile ring of "for sale" tiles (code 0x14) stamped around a centred owned square, inside terrain generator FUN_00470a60 (slot index = param_1, p = slot * 0x2e from base 0x571ff4).

## Acres byte meaning (EXACT)
Slot byte +1 (DAT_00571ff5) is acres; 1 acre = 10 tiles. The owned core is the largest centred square of whole tiles whose area is <= acres*10 (more precisely the first ring count k that satisfies it, see below). Consistent with the Buy Land screen showing "acres = for-sale tile count / 10".

## Ring thickness k (EXACT, decompile lines 86365-86371)
```
iVar13 = 25; iVar18 = 0;
do { iVar24 = iVar18; iVar18++; iVar13--; } while (acres*10 < iVar13*iVar13*4);   // signed
```
After the loop iVar18 = k and iVar24 = k-1. Equivalent: k = smallest integer >= 1 with 4*(25-k)^2 <= acres*10, i.e. owned side s = 50-2k, area s*s <= 10*acres. Owned tiles are x,y in [k, 49-k] (centred square on the 50x50 map, indices 0..49).
Examples: 110 acres -> k=9, side 32 (tiles 9..40); 50 acres -> k=14, side 22; 40 -> k=15 (side 20); 30 -> k=17 (16); 190 -> k=4 (42); 210 -> k=3 (44); 250 (sandbox byte 0xfa) -> k=1 (side 48, tiles 1..48 = all nine tracts).
Table (slot, acres, k, side): acres 50:14:22, 60:13:24, 70:12:26, 80:11:28, 90:10:30, 100:10:30, 110:9:32, 120:8:34, 130:7:36, 140:7:36, 150:6:38, 160:5:40, 170:5:40, 180:4:42, 190:4:42, 200:3:44, 210:3:44, 40:15:20, 30:17:16.
Acres per slot (see DECODE_WORLD2 4.3): (slot+4)*10, +-10 (slot<=3) or +-20 (slot>3) for inland/island, coastal unchanged; sandbox 250.

## Stamping (EXACT, lines 86373-86400)
1. Before stamping the whole 2500-byte tile map (0x271 dwords from DAT_005722e8) is copied to the shadow array DAT_00542414, so the real terrain survives under for-sale tiles (restored on purchase).
2. If lie (DAT_00571ff7) != 2 (inland 0 or coastal 1): for i = 0..k-1, for j = 0..49 stamp 0x14 via FUN_00470a10(x,y):
   - (x=i, y=j)       left strip (x low)
   - (x=j, y=i) only if lie == 0 (inland): y low strip
   - (x=j, y=49-i)    y high strip
   - (x=49-i, y=j)    x high strip
   So INLAND is ringed on all four sides; COASTAL leaves the y-low side (y = 0..k-1) owned all the way to the map edge (only three sides get a for-sale border), so the owned region is x,y' in [k,49-k] x [0,49-k]. Which screen edge y-low is depends on the iso projection (not checked).
3. FUN_00470a10 (line 85506): writes tile = (((lie != 2) - 1) & 0xfd) + 0x14. For lie != 2 that is 0x14. For lie == 2 (island) it writes 0x111 truncated to byte 0x11 = shallow water, and clears flag bits 0x0120 of DAT_0053caf0 (mask 0xfcdf). So on ISLANDS the "border" is a ring of water, not for-sale land: there is NO unowned land at start; the whole map is yours (weak reading that no 0x14 exists on islands: the island branch, lines 86403-86420, only calls FUN_00470a10 which writes water for island).
4. Island branch (lie==2), lines 86403-86420: for i in 0..49 width w = FUN_00467130(|i-25|, 8, 0x32) + (k-1) - 11 + rand(3); for t in 0..w-1 stamp water at (t,i),(i,t),(i,49-t),(49-t,i): an irregular water ring that thickens toward the corners. FUN_00467130 is an interpolation helper (not read, weak). Land tiles inside are all owned.
5. Rows/cols 0 and 49 are 0x14 whenever k>=1, but tracts only cover tiles 1..48, so those outer tiles can never be bought (inland/coastal). 

## Consequences for the port
- Ownership start is per TILE, not per tract. Owned test at start: inland: k <= x,y <= 49-k; coastal: k <= x <= 49-k and y <= 49-k (y-low side open); island: everything owned (water ring is just terrain); sandbox: k=1.
- Tract t (16x16, origin 1+16*(t%3), 1+16*(t/3)) is usually PARTIALLY owned at start. Buy Land price scans remaining 0x14 tiles (UI_SCREENS2 section 4), so the tract card says e.g. "N acres" = remaining 0x14 count / 10, and price scales with the count. Port's tract-level ownMask/landModel cannot express this; it needs a per tile owned flag (or ownMask plus per-tile start ring), and buying a tract restores all its 0x14 tiles (already per docs).
- Clubhouse (FUN_0040e000(x,y,0xf,0x60), lines 85989-86001): x = rand(17)+15, y = rand(17)+15, i.e. tiles 15..31, so it always lies inside the owned square for every k <= 14 (k=15..17 only for slots 0 with coastal/island 30-40 acres, side 16-20: squares 15..34 or 17..32; clubhouse footprint may partly exceed on 30 acre islands, but islands are fully owned anyway; for 40 acre coastal k=15 owned 15..34 ok).
- The guess clamp((acres+10)/15,4,9) tracts is wrong in kind. At 110 acres the real start is 1024 owned tiles (about 4 tracts' worth of area, but a centred block overlapping all 9 tracts), not whole tracts.

## Tract purchase price (already known, EXACT, UI_SCREENS2 4 / routine 0x4587a0; not re-derived here)
price units = 10 * floor((5 + sum over for-sale tiles passing rnd(3)==0 of 2^n) * 20 / 100), n = DAT_0053a450 purchase counter. With a partly owned tract the sum is over fewer tiles. Unchanged by this finding; only the tile count per tract differs.

## Confidence
- k loop, stamping geometry, shadow copy, FUN_00470a10 tile value: EXACT (read directly from lines 85506-85513, 86365-86400).
- Coastal missing y-low side: EXACT in code (the lie==0 guard on the second call); which visual side: weak.
- Island = no for-sale land: EXACT that 0x14 is never written for lie==2; the water ring shape: weak.
- Not found: any mention of the sandbox or load-game path changing this; any rule beyond acres/lie. No per-site override of ownership.
