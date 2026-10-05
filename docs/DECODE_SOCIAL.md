# Social systems decoded from the publisher's golf.exe

Facts only, in our own words. Money is in units of $100. Ticks: 1024 a month, 8192 a year (8 months). Confidence: EXACT (read in code), PROBABLE, PLACEHOLDER (not found).
Code: include/sg/membership.h, goals.h, visitors.h. Tests: scratchpad/tests_social/t.cpp.

## 1. Membership (EXACT unless noted)
- A pool of 75 golfer identities persists. Each has a tier, a resigned flag and a 2-bit "liked" counter per hole.
- Tiers: 0 none (never invited), 1 Visitor, 2 Member, 3 Silver, 4 Gold, 5 Platinum. Resigned is permanent.
- New game: 12 golfers start as Visitors, three from each of four archetype classes (archetype mapping PLACEHOLDER).
- After a completed round points = (sum of per-hole liked counters, masked to 0..3) + 1 if mood >= 2*difficulty+6 + 1 if (a late hole liked, or difficulty 0, or first year, or cash < 200 units with no members) + the improvement level of the member-attracting building.
- Ordinary golfers only. Promote when (1 << tier) <= points and tier < 5: 2, 4, 8, 16 points. A promotion invites a random tier-0 golfer as his or her friend.
- Mood below zero makes a golfer quit mid round. At tier 2 or more this is a resignation (tier 0, flag permanent, hole blamed). Reasons are chosen by the event that last upset them.
- Fee bonus per hole: Gold +2, Platinum +5. Silver buys home sites. This corrects the older note that tied that fee term to hole type.
- Member count = tier 2 or more and not resigned. Roster screen shows Platinum, Gold, Silver, Member counts, Resigned, and a "Waiting list" (definition not read, PLACEHOLDER).
- Arrivals pick a random non-resigned id with tier above 0 that is not on course and shares no id % 19 with anyone on course. 999 failures show "membership is declining".
- Year-end screen: This Year and Last Year columns, month ring compared with the entry 8 months earlier, lines for cash, fun, skill and membership (increased, same, decreased), then highlight events (hole opened, building, match won, Top 100, Top 18, tournament place, CEO joins board, happy ending, land bought, landmark donated).

## 2. Visitors (EXACT)
Spawn when a pair launches, pair index p = (slot/2) % 6; off in sandbox.
- CEO: p = 1, heiress bit clear (an apparent original quirk), fewer than 8 so far, holes >= 2*(count+1). Pays 50 or 100 units if finished, mood > 2 and holes >= 2*count (new count); mood > 4 pays double.
- Commissioner: p = 3, none on course, property kind not 2, rating > (k+2)*(difficulty+2)*50. Rating is fun below difficulty 2, else skill (hundredths). Finished and mood > 2 opens a land purchase offer.
- Heiress: p = 5, none on course, (h+1)^2*25 < rating (rating choice PROBABLE), landmarks left. Finished and mood >= 3 donates a landmark 0..15, effect = type & 3 (happy thoughts, no dandelions, faster player skills, happy stories), worth (type*5+5)*200 dollars. Draw loop approximate.
- International celebrity: unreachable test in this build, no spawn. Not implemented.

## 3. Goals (EXACT)
22 table goals (not 28) plus 3 sheet-only lines (first dogleg right, dogleg left, par five).
Hole type bits L=1, A=2, I=4: 1 freeway, 2 precise, 3 challenge, 4 creative, 5 heroic, 6 strategic, 7 classic, 0 breather.
- Challenge, Heroic, Strategic: below difficulty 2 a dogleg right, dogleg left, par five hole opens; from difficulty 2 hole type 3, 5, 6.
- Skill upgrade; first tournament accepted; first match win; 9th hole opens; 18th hole opens.
- Top 100 hole, Top 18 hole (hole flags 1, 2). Classic hole (type 7, any difficulty).
- Tournament accepted with prize >= 500k, >= 1M (thousands field).
- Win a tournament with >= 9 or >= 18 holes. Course rating >= 90 with >= 9 or >= 18 holes. Rating 100 with 18 holes.
- Victory by theme (4 goals): win, 18 holes, prize > 100000 (odd value, kept).
- Dogleg: angle tee-green minus first waypoint-green; beyond 10 degrees left sets 0x20, right 0x40. The waypoint route search was not fully decoded.
- Par from route yards d: d>250 adds (d-250)/4; +25 for dogleg over 250; over 300 minus 25 per driving range level; class 3, 2 if < 51, 4 if > 249, 5 if > 474, 6 if > 625.

## 4. Tutorial, advisor, comments (EXACT structure, text not copied)
- Advisor bubble only on difficulty 0 in the first year; picks by missing tee, green, first hole, second hole, golfers arriving, hole fun percent, then periodic hints by panel. Character customize prompt at tick 0x2200.
- Tutorial: flag 0x8000, pages 1 to 11 (fun) and 21 to 29 (skill), any key advances, Escape reloads the player's own snapshot. Start trigger not found.
- Golfer comments use one event id table shared with the mood routine (about 60 ids, e.g. 1 great shot +1, 2 bad lie -2, 9 near hit -3, 0x1e repetition -2). Repetition check compares hole N with N-1: dogleg left, dogleg right, same class.
- Stories: pool from theme text files, up to 3 draws of 100 at pair launch, first pair gets the opening story, chapters 0..4, happy ending logs and plays a sound, chance of losing a story rises after 7 endings.

## 5. Round economy
- Green fee per hole finished = mood, doubled with the highest greens fee improvement, +2 Top 100, +2 Top 18, + airstrip bonus, + tier bonus. EXACT.
- Arrivals: one golfer when (ticks & 127) = 0 or fewer than 2 wait, and under 8 wait. No dependence on price or fun except through the pool. EXACT.
- Amenity income per building type: 7 pays 5, 6 pays 4 or 8, 8 pays 6 or 10, 10 pays 8 or 12 (lower below upgrade level 2). EXACT.
- Hole upkeep: every 1024/(difficulty+2) ticks each existing hole costs 1 unit. PROBABLE.
- Employee wage: every period (same clock) each employee passes a random test (random below 4-difficulty not above a course-size count) then costs units from a table: kinds 2..5 cost 3, 2, 2, 2, or 7, 3, 4, 5 when the skilled flag (8) is set. PROBABLE, kind mapping not confirmed. Skipped in sandbox.
- Construction and path cost: NOT FOUND. Land purchase and commissioner price: NOT FOUND.
- Cash writes elsewhere: wager pay and collect, tournament prize, CEO investment, removal charge, property price table.
