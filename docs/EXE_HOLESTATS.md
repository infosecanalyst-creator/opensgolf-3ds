# Per-hole statistics: index and cross-check

The full decode of the per-hole record (layout, writers, reaction deltas, the report maths, variety counter, Top 100 and Top 18 tests) already lives in docs/DECODE_HOLE_STATS.md, implemented in include/sg/holestats.h and src/holestats.cpp. Goals, dogleg and par rules live in docs/DECODE_SOCIAL.md and include/sg/goals.h. This file records a re-check of those results against the publisher exe's decompile, in our own words.

## Re-checked facts (all agree with the existing docs)

- The record table starts at 0x575ab0, 0x208 bytes per hole, hole numbers 1 based. A par byte of 0 means the hole is not open.
- Hole class (par) from yards: 2 up to 50, 3 up to 249, 4 up to 474, 5 up to 625, 6 above. Adjustments before the test: when either dogleg bit is set and the figure is above 250, add 25; above 300 subtract 25 per driving range theme level. Hand-measured distance L is stored as L plus a quarter of the excess over 250.
- Dogleg bits sit in the hole flag word: 0x20 left, 0x40 right.
- Hole type is the three skill demand bits (length 1, accuracy 2, imagination 4): 0 Breather, 1 Freeway, 2 Precise, 3 Challenge, 4 Creative, 5 Heroic, 6 Strategic, 7 Classic.

## Trophy latch ids (clarification)

The accomplishment table has 22 once-only latches, one per name string. The latch id is the position in this order: 0 Challenge, 1 Heroic, 2 skill upgrade, 3 first tournament, 4 Strategic, 5 first match victory, 6 first 9+ hole course, 7 Top 100, 8 first $500,000 tournament, 9 Classic, 10 Top 18, and so on to 21 for the 100 star rating (names read from the name block that precedes the latch array).

When a hole opens on difficulty 0 or 1, a right dogleg sets latch 0 (Challenge), a left dogleg sets latch 1 (Heroic) and a par five sets latch 4 (Strategic). From difficulty 2 the real type masks 3, 5 and 6 are required instead. So the "1st dogleg right", "1st dogleg left" and "1st par five" lines on the notes sheet are not separate exe strings: they are the easy-difficulty wording of the Challenge, Heroic and Strategic trophies. The Top 100 and Top 18 awards use latches 7 and 10.

## Verification

Test program (outside the repo): build with
`g++ -std=c++17 -Iinclude -include string src/holestats.cpp <test_holestats.cpp>`; the existing test prints "all holestats tests passed".

## Still unknown

Meaning of game flags 0x40 and 0x200000, globals 0x543cf4 and 0x543cd8; who sets the uphill and downhill hole flags; whether hole expense (record +0x1f0) is ever written; the waypoint route search behind the dogleg angle.
