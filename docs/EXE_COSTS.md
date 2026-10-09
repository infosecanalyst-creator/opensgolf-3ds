# Costs, employees, land and the ledger (from the publisher's golf.exe)

Facts in our own words. Money is in stored units of $100. Tags: EXACT (table or code read), PROBABLE (shape read, meaning inferred), PLACEHOLDER (not found).
Code: include/sg/costs.h, src/costs.cpp. Test: /tmp/sgcosts/t.cpp (outside the repo).

## 1. Buildings (EXACT)
A 0x14-byte table (name, footprint, base cost in units) lists 20 object types; the index is also the unlock level. Base cost in units and footprint:
Pathway 1 (1), Benches 2 (1), Flower Bed 5 (1), Ball Washer 50 (1), Landmark 15 (1, see below), Home Site 10 (2), Putting Green 100 (3), Snack Bar 150 (2),
Pro Shop 200 (2), Swim Club 300 (3), Driving Range 250 (5), Cart Garage 400 (2), Marina 1000 (2), Resort Hotel 2500 (4), Airstrip 5000 (6), Clubhouse 200 (4),
Willow Tree 25, TV Tower 10, TV Booth 10, Scenic Bridge 100 (the last four are table entries only). In dollars multiply by 100 (Snack Bar $15,000, Airstrip $500,000).
- Placement cost = base * (level + 2) / 2 plus a site-work term from the terrain under the footprint (routine 0x40db90, not decoded: PLACEHOLDER 0). A pathway halves the site term. The panel shows base for level 0 and 1.5 x base once the type has a level.
- Level is a per-type counter (starts 0, goes up when a building of that type is finished). A type at level 2 or more cannot be placed again; level 1 or more needs more than 9 holes ("upgraded buildings" text). Footprint grows with level (exact growth rule not settled: PLACEHOLDER).
- Landmark: charged (5 * kind + 25) * 2 units, nothing when it was donated by the heiress. The table's 15 is not used.
- Home site: placing one pays the club a quarter of the lot value (booked in Home Sites); removing one costs lot value / 2 plus stored value / 50.
- Removing a path, bench, flower bed or ball washer refunds its base cost; a landmark refunds (kind + 5) * 10. Removing a type 6 to 15 building refunds nothing that I found; the player confirms the demolition. Removing the clubhouse allows re-placing it (the manual says a replacement fee; PLACEHOLDER = normal cost).
- Pathway costs 1 unit ($100) a tile, matching the strategy guide.
- Income per visit (Food/Drink): Snack Bar 5; Putting Green 4, or 8 at level 2+; Pro Shop 6 or 10; Driving Range 8 or 12; a Soda Vendor sale 2. Ball Washer, Swim Club, Cart Garage, Marina, Resort Hotel, Airstrip pay nothing per visit; their effects are in PUBLISHER_EXE_NOTES (Airstrip raises the fee, Marina raises lot values).
- Upkeep per building: none found. Only holes cost upkeep (below).

## 2. Employees (EXACT unless marked)
Four kinds, each with a basic and a skilled form: Club Pro / Celebrity, Ranger / Marshall, Groundskeeper / Technician, Soda Vendor / Refresher. Skilled forms need a Daily Fee course (grade 1 or more, six or more holes).
- Wage per wage event (also the price on the hire screen), basic/skilled: Club Pro 3/7, Ranger 2/3, Groundskeeper 2/4, Soda Vendor 2/5 (units).
- Wage events run every 1024 / (difficulty + 2) ticks. Each employee pays only if a random roll below (4 - difficulty) is not above the course grade, so grade 3 (18 holes) always pays and small courses pay on some events only (PROBABLE roll bound). Skipped when game flag 0x4000000 is set (PROBABLE practice mode).
- Hire fee 2 units (PROBABLE, ledger write seen), fire fee 25 units. Both book to Salaries.
- Effects: text only (greeting, speeding play near a tee, weed removal, serving thirst). Magnitudes not decoded.
- sgview currently has placeholder daily wages 30/25/20/15 scaled by 25 per day; the exe figures are far smaller per event but charged about 2 to 5 times a month. Replace with wageUnits and periodTicks.

## 3. Running costs and the ledger
- Every period (same 1024 / (difficulty + 2) ticks) each existing hole costs 1 unit (Maint./Interest).
- Once a month (ticks % 1024 == 0) negative cash is charged interest of cash / 50 (2 percent of the debt), booked in Maint./Interest.
- Ledger: 100-month ring of eight rows plus total: Greens Fees, Home Sites, Food/Drink, Build course, Facilities, Salaries, Maint./Interest, Other. Booking map is in costs.h. Greens fees are the exe's mood-based fee, already in sg::holeFee (docs/DECODE_SOCIAL.md section 5); sgview uses holeFee, so no separate feeUnitsFor exists to compare. The fee has no base price term.
- The report shows nine monthly columns ending at the current month; a game year is eight months. The year-end summary items are in DECODE_SOCIAL section 1.
- Purchases that exceed cash ask for confirmation once the course has 4 or more planned holes (otherwise they pass silently); the player may go into debt. Board debt ladder is already in economy.cpp.
- Other cash events: tournament prizes, match wagers (10 x stake on a win), CEO investment, land purchases, all in Other.

## 4. Land and start of game
- Start cash 1000 units ($100,000); sandbox 10000 ($1,000,000) (a second path stores 100000: PLACEHOLDER which applies). Property prices by index: 500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000 units, ascending, so only the first four fit the standard cash. sgview's rule (price <= start funds, cash = funds minus price) matches; its price list in dollars is the same set.
- Extra land: a 3 x 3 grid of 16 x 16 tile tracts; unowned tiles carry tile id 20. Price in units = ((5 + sum of (1 << kind) over unowned tiles passing a 1-in-3 roll) * 20 / 100) * 10, where kind is probably the course theme. The roll is redrawn each time the screen is built (PROBABLE). A full unowned tract costs about 180 units on theme 0.

## 5. Terrain editing
Paint costs per tile id are the table already in the notes (water 50, elm 25, green 10 ...). Raising or lowering one vertex costs 2 units, 15 over water (PROBABLE). Painting charges are booked to Build course; undoing a tile returns its stored cost.

## Unknowns
Site-work term, footprint growth, upgrade level semantics beyond the counter, employee effect sizes, clubhouse replacement fee, tract roll timing, sandbox cash.
