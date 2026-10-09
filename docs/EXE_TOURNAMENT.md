# Tournament lifecycle: summary and verification pass

This is a compact restatement of docs/DECODE_TOURNAMENTS.md (the full decode, already implemented in `sg/tournament.h` and `src/tournament.cpp`), written after a second check of the facts against the publisher exe. Confidence: E exact, M medium, G guess.

## Lifecycle
1. Offer (E). At the start of July (tick % 8192 == 4096) with no match pending and neither sandbox (0x1000000) nor a running tournament, the SGA evaluation runs silently. A non-zero prize sets the offer flag (0x400000) and shows a message; a zero prize clears it. An offer never expires otherwise.
2. Button (E). The tournament button shows only while the offer flag is set and no tournament or championship load is active. Pressing it re-evaluates the course, so the prize may change or vanish (then the SGA screen opens instead). The dialog offers "Great, let the games begin." or "I think I need more practice." (the latter only clears the prize; the offer stays).
3. Accept (E). Sets the running flag (0x200000), snapshots tables, stamps accomplishments 3, 8 (first prize >= 500) and 13 (>= 1000).
4. Field (E). One round, one pass over every open hole, no cut, no second round. 2H golfers (36 on 18 holes), shotgun start, two per hole. Slot 1 is the player, slot 0 the partner. The 2H-1 others are drawn from the pro table (record 0 never) by a strength window that rises with prize and cash and loosens with tries. Celebrities are never entrants.
5. Course is not closed to normal golfers in any code seen (M, absence of evidence); the tournament golfers are extra simulated golfers.
6. Scoring (E). Per golfer: sum of strokes minus par over completed holes. Places are unique; ties go to the lower slot. Paid places are 1..H; first prize in thousands, each next place two thirds of the previous (integer division). A first prize of zero becomes H*20.
7. Payout (E, player only, paid places only): history code 0xE0|place, cash += prize*10 units of $100, fame += 4-place for places 1..3 else 1, win trophies 11 (H>=9) and 15 (H>=18).
8. Cancel (E): confirmed cancel drops both the running tournament and the pending offer, no payout. Saving a championship course during a tournament is refused.

## Correction to the older notes
The maximum first prize on 18 holes is ((100-50)/5+5)*(grade+1)*19*2 = 2280 (thousand), not 1140; the Grand Slam victory trophy threshold (above 100000) is still unreachable.

## Files
progolfers.dta: CRLF text, comment lines start with '*', records `name,body,skin,hat,shirt,pants,SSSSSSSSSS` (ten skill characters, 0-9 then A-F), trailing loose number ignored; 96 records on the shipped Standard theme, cap 100. celebrities.dta: `name,type,skin,hair,shirt,pants`, type letters A to K, 21 records, cap 100. Loaders read the user's disc at runtime.

## Unknowns
Tournament preparation skill warning (0x4065c0), objects of kinds 0x11 and 0x12 removed at the end, match wager formula, championship name mapping by difficulty, whether `*.pro` files extend the pro table, the real golfer stroke simulation (the port uses a placeholder generator).
