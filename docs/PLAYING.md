# Playing the club

Run `./sgview --game "../game/Program_Files_(ENGLISH)"`. It opens on the title menu, drawn from the disc's own Interface art with the game's font: Start New Game (Standard) or Sandbox Mode opens the property chooser, and picking a property starts a course on that property's theme. A property is paid for out of the §100,000 starting funds (sandbox: unlimited). Esc goes back, and from the course returns to the menu. Continue Saved Game loads `course.sgc` if it exists: the terrain, paths, buildings, staff, money and date all come back.

For scripted runs (any of `--png`, `--course`, `--edit`, `--golfer`, `--sandbox`, `--follow`) the viewer skips the menu and opens on the demo course (`--new N` starts a new game on property N instead, `--savegame FILE` and `--loadgame FILE` write and read a full saved game); `--screen menu|property|play` forces a screen. Add `--theme Desert` for another look there.

- **Holes** are found from the terrain: each cluster of Tee tiles is paired with the nearest unused cluster of Putting Green tiles (at least 2 tiles each, at least 250 units apart), tees taken top row first. Paint a tee (G twice, or the Paint tool) and a green to add a hole. The console prints "N holes" after every edit.
- **Golfers** arrive over time, play every hole in order, pay a green fee at the end of each hole and leave. The first golfer comes at once, then one about every 25 seconds (slower if golfers are unhappy), up to two per hole and eight at a time. Each has different skills and one of four looks.
- **Fun** is a running average of how golfers felt after each hole. Happy golfers pay more, unhappy ones less (the game text confirms this direction; the amounts are placeholders). It shows as red, yellow or green in the title bar.
- **Money:** you start with §100,000 (confirmed from the publisher's golf.exe and the property screen). The green fee defaults to 1000 per hole, from the scale seen in course reports. Upkeep and wages are charged every game day (120 seconds). When cash stays below zero the board sends warnings and then ends the game, in four steps (the days are placeholders). Sandbox mode never charges.
- **A new game now starts on untouched land** (rough, woods, brush, a pond, a clubhouse lot, no tee, green or path), so you build the first hole yourself. The land is generated, 40 by 40 tiles for every property; the original's real land shapes and sizes are not done. **Not here yet:** difficulty choice, memberships, tournaments, and real building costs. The console prints events (strokes, fees, board messages).

- **Course Report:** press F1 on the course. It uses the disc's report art and lists every hole: yards, par, average strokes, time, fun, the +Len/+Acc/+Img scores (measured by simulating golfers with and without each skill, as the original's Shot Analysis suggests), type, average fee, revenue and profit. Yards, minutes and the profit split are placeholder scales. Esc, F1 or a click closes it. `--screen report` opens it directly.

Test hooks: `--cash N` sets the starting cash, `--time S` simulates S seconds before taking `--png`.

## Buildings (first slice)

In edit mode (Tab), press T until the title shows "Building", then [ and ] choose the amenity (Snack Bar, Pro Shop, Cart Garage, Hotel, Tennis Court, Marina; only the ones that exist for the theme). Click a building lot tile (paint one with the Building Lot brush) that touches a path joined to the clubhouse. Shift-click removes and refunds. Costs are placeholders; a Snack Bar pays 5 units (§500) when a golfer holes out within six tiles of it (placeholder rule, the amount is from the exe). The headless test option is `--edit "b:x,y,index"`.

## In-game interface (dock, panels, advisor, stories)

The lower left of the course screen now shows the game's own dock art (Interface/3mainLowerLeft.pcx). The three big round buttons open panels along the bottom: **Build Course** (terrain brushes, paths, raise and lower, each with its cost), **Add Buildings** (the amenities for the theme) and **People** (click to hire, right click to fire a club pro, ranger, groundskeeper or soda vendor). Click an item, then click the map. The small buttons zoom, rotate, open the course report, pause, and save the game (Shift+S also saves, Shift+L loads). Keyboard shortcuts still work.

The advisor box at the top gives hints based on the state of your club, written by me, not taken from the game; press H to hide it. When two or more golfers are on the course a story from the disc's Themes folder plays line by line underneath. Which story plays and what triggers it are not decoded yet, so one is picked by the game seed. The story text is read from your disc at run time and is not stored in this project.

## Opening holes (decoded from the publisher exe, see docs/PUBLISHER_EXE_NOTES.md)

A new game follows the original's flow. Build a tee, then a green (Build Course panel), then press **H** (or use "Open the hole (H)" in the panel). Golfers only play open holes, and the game will not let you start another hole until the last one is open (at most 18 holes). Each opened hole can unlock a building type: from a normal start the Putting Green comes at hole 2, the Snack Bar at 3, the Pro Shop at 4, the Swim Club slot at 5, the Driving Range at 7, the Cart Garage at 8, the Marina at 9, the Resort Hotel at 11 and the Airstrip at 12; holes 6, 10 and 18 upgrade the course (Municipal, Golf Club, Country Club, Championship). Sandbox mode has everything unlocked from the start. F1 now shows or hides the advisor. The effects of the unlocked buildings are not simulated yet.

## Goals and visitors (partly decoded)

The game now keeps a list of professional accomplishments (the exe's names; so far First 9+ hole course, First 18 hole course and the first Classic, Challenge, Heroic and Strategic holes) and shows a message when you earn one; the HUD shows goals earned. A corporate investor can visit every third month once three holes are open and golfers are happy, and invests 5000 (10000 when golfers are very happy), up to 8 times. The payout and the limit come from the exe; when a visitor comes and what pleases him are my placeholders. Tournaments, matches, membership, the other visitors, landmarks and golfer stories are the next things to build; see docs/PUBLISHER_EXE_NOTES.md for what the exe says about each.

## Your character, the advisor and the shot path preview

After you pick a property you are offered your character: stick with the default (Gary Golf, the exe's own name) or customise the name (type it, Enter to finish) and the gender (Tab). Your character gives the advice in the banner at the top. Skill points, appearance, personality and the disc's character files are not done yet; see docs/PUBLISHER_EXE_NOTES.md.

While you are editing (Build Course or Add Buildings open), every hole with a tee and green shows a dotted line from tee to flag with landing markers about a drive apart. White is an open hole and yellow is a hole not yet opened. The spacing is the port's placeholder. A one tile tee or green now counts as a hole (before, a single tile was ignored, so the hole could never be opened).
