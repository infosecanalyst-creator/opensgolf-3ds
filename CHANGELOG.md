# What's new in OpenSGolf

The big things each version brought, newest first. Every release, with its downloads, is on the
[releases page](https://github.com/crspytopgn/opensgolf/releases). The game itself can update to the newest one
(*Check for updates* on the title menu, then *Update now*).

Almost everything OpenSGolf adds to the original game is **off until you switch it on**, so the game plays as it did in 2002
unless you choose otherwise. Most of it is in Preferences: *Game options* holds the extras and presets, *Graphics* holds
Enhanced graphics and HD.

## 0.7 (7 November 2026)

**Highlights**

- **A second course on the same land (#77):** with *Second course: 18 more holes* (Preferences > Game options,
  Gameplay; off unless you tick it), once your 18 holes are open a Course A | Course B switch appears over the Build
  Course panel and the usual tools build holes B1 to B18. Arriving golfers play one course or the other, and Course B
  has its own Course Report and Hole Stats. The SGA, tournaments, ratings and the other reports stay with Course A.
- **Forward and middle tees (#120):** with *Multiple tees* (Gameplay, off unless you tick it), Tab with the Tees tool
  places a hole's forward and middle tees. Less skilled golfers play from the front, average ones from the middle, and strong
  golfers, pros and tournaments from the back. A hole's length, par and ratings stay those of its own tee.
- **Fly over a hole:** with *Hole flyover* (Looks), Hole Stats gets a Fly over button that flies the camera from the
  tee down the hole to the green and back.
- **Double-click a save to load it (#145)**, or double-tap it.
- **Longer translations fit (#164):** text that is too wide for its button, tab, sign or box now shrinks a little, then
  wraps, then is cut short with "…", instead of running over its neighbours. English looks as before.
- **Home names only under the pointer (#160):** a new Looks option shows a celebrity's name over a home site only while
  the pointer is on it.
- **Exact replays in problem reports:** *Report a problem* now also writes a replay of the game since it was loaded
  (your own actions, nothing else), so a reported problem can be played back exactly as it happened. It stays on your
  device unless you attach it to a report.
- **Keep the land offer** has moved to the Sandbox page of the options.

**Fix releases**

- **0.7.3:** a launcher before the title menu (game files, how to play, display and sound, news), shown at each start
  unless unticked; What's new shows 0.7's highlights (it still showed 0.6's) and reads them from GitHub, as the web
  page's Latest news now does; Start New Game goes straight to Choose Difficulty unless you have scenario packs of your
  own; *Original + helpers* is the recommended preset, and the *OpenSGolf* preset adds weather that matters, forward
  and middle tees and the second course.
- **0.7.2:** racing wheels, pedals and flight sticks no longer move the pointer or scroll the course by themselves, and a
  controller's stick only counts once it has been at rest in its centre (#169); longer translations also fit the
  scorecard's labels, the SGA report, the routing map's box, the Membership Roster's legend and the golfers' comments
  (#164).
- **0.7.1:** longer translations also fit the building info box, the home-site details, the golfer card, the pro's
  skills, the world map's cards and legend, the shot panel, the foot of Comments by Hole and the Course Report's legend,
  instead of running out of them (#164).

## 0.6 (28 October 2026)

**Highlights**

- **Smoother play.** The short freezes as golfers came and went are gone, and the course draws faster, most of all in
  browsers.
- **Enhanced graphics keep up on big screens and slower machines.** In a browser they draw at up to 1440p and stretch to a 4K
  screen, and when the game runs slowly they ease off a step at a time by themselves.
- **Make HD graphics from your own SimGolf files (beta):** an AI upscaler built into the game redraws the art sharper,
  on your own computer or phone, while you play (Preferences > Graphics). It switches itself on when it is done, and
  *Remove HD graphics* undoes it. Not in browsers yet.
- **A new cast of cartoon faces** for the free graphics' golfers.
- **Checks for a new version when the game starts** and offers it, saying nothing if there is none (Preferences > More
  switches it off).
- **The terrain brush's preview keeps to its tile when zoomed out** (#115).
- **The game says when some of your SimGolf files cannot be read**, instead of quietly leaving things off the course.
- **New options:** aim and swing with the mouse the way touch screens do; *right click puts the tool away* (B on a
  controller), so a brush left in hand no longer undoes a hole's tee or green; and on wide screens the interface now sits at the
  screen's edges with Classic graphics too (on big TVs the dock and money no longer sit in the middle; Looks switches it
  back); *Sandbox: no new
  dandelions*; and **[** now slows the game down as **]** speeds it up, each stopping at the end instead of going round.
- **Desert names as in the original:** the desert's Brush and Ravine buttons now carry the names (and descriptions) the
  original gives their pictures, so the placing preview matches too, and the *Fix the desert's swapped Ravine and Brush
  art* option, which only made up for the wrong names, is gone. Undo's *Reset to* and the shot panel's lie use the
  theme's names (desert, rough, canyon and the rest) instead of the Parkland ones.
- **Desert rough is edged with grass as in the original:** no more sand rings round buildings and landmarks, and the
  rough reaches the map's edge (#118). As in the original, the desert's rough also counts as its own ground for weeds,
  spectators and what golfers think of it.

**Fix releases**

- **0.6.10:** desert ponds draw the Desert's sandy shore again, keeping the grassy shore only on the edges that meet a tee,
  green, fairway or sand trap, as the original does (#161); a golfer becoming a member, or moving up a level, plays the
  club's fanfare with the message (#162).
- **0.6.9:** Hungarian ő Ő ű Ű are drawn in the game's own fonts, made from their ó Ó ú Ú (ő was blank and Ő another
  symbol with the original game's fonts) (#153); with Enhanced graphics, the blimp's reflection sits below the water's
  surface again instead of over its shadow, pointing the blimp's way (#147).
- **0.6.8:** the game's text draws Central European letters (Hungarian ő ű, Polish, Czech and the rest) and the
  typographic quotes, dashes and ellipsis („ ” – …), for translations (#153). A new gameplay extra, off by default:
  **Keep the land offer**, to buy land when you like after the County commissioner approves an expansion (from puddin11).
- **0.6.7:** on a touch screen, a one-finger drag with no tool in hand only scrolls the view (it no longer picks up the
  employee under it, or opens a golfer's card); the aim's plate moves aside so it no longer hides the end of the shot; with
  Enhanced graphics, no hard red and purple circles after rain. From puddin11.
- **0.6.6:** with Enhanced graphics, the blimp's reflection in the water points the way the blimp does (#147).
- **0.6.5:** undo (Ctrl+Z, the touch Undo button) puts back only what the click changed and its money, and no longer winds
  the clock, the golfers and the money earned since back; what was built since (a tournament's TV towers) stays. Placing,
  removing and undoing show the money over each tile, as in the original ("-400" in red, "+5,000" in green). No ball is
  left by the pin of the last hole planned. Golfers swing along the ball's line instead of 45 degrees off it, as in the
  original's art. The SGA's tournament offer says where Begin Tournament is (the People panel, or Shift+J), and a new
  "How do I...?" page answers common questions. From puddin11: the undo, money, ball and swing changes.
- **0.6.4:** the view can no longer be scrolled off the map, and in full screen the pointer at the screen's edge scrolls it
  as in the original (Edge scrolling, a new Controls option, on by default) (#138); painting over a flower bed, a garden
  border or a small object takes it away from the screen too (#136); a game's options stay that game's, so new games start
  with the ones chosen on the title screens (#137); with Fix original bugs, rough painted beside a driving range no longer
  joins it (#139); celebrities at their homes wear their own colours instead of blue skin, and each occupied home
  site shows who lives there, as in the original (#141); the build panel's buttons and design pictures answer only at
  their foot, as in the original, so a tree's crown no longer picks the tree beside it (#142); Sandbox Mode goes straight
  to the world map, each property keeps its own card there, and sandbox land shows no price, as in the original; the
  course size and Game boxes are on the world map for a sandbox (#143); with Fix original bugs, a new game no longer
  starts with land past the property's edge: landmarks out in the sea round an island, or marsh past a coastal map's edge
  (#144); the raise and lower land keys (- and =) work only with a tool armed, and no longer keep the spot the last tool
  left, even into the next game (#146).
- **0.6.3:** with Fix original bugs, a green painted beside a putting green no longer joins it, and gets its flag (#128);
  painting water over the sea keeps its depth and colour (#132); with the interface at the window's edges, the pointer to
  the right of the build panel no longer picks a tree (#130).
  From puddin11: sprites take about a third of the memory (the browser on a phone no longer reloads); Enhanced
  graphics draw only the grass tufts on screen, for a much higher frame rate; a touch Undo button while building; the
  browser fills a sideways iPhone's screen; Pause in the background also stops the sound, and Autosave saves as the game
  goes to the background; Analyze Golf Shot stays armed after going to the Buildings panel and back.
- **0.6.2:** with Fix original bugs, the pine brush no longer shows a path tile under the pointer (#126).
- **0.6.1:** a first-start box to choose how to play (the four presets), and What's new in 0.6 once after updating; Game
  options' Helpers page split into Saves and Controls, and every page's options listed most wanted first; typing in the bug report form opened from *Report a problem* no
  longer loses what you type, and the form's platform and base set come filled in; the version on the title screen; loading a
  save in the browser no longer freezes for seconds while the golfers' animations load; with Fix original bugs, a new
  game's free building beside the clubhouse no longer stands past the edge of a small property (#125).

**Fix releases before it**

- **0.5.13:** the picture no longer goes black or jumbled after resizing the window; Classic shades hills smoothly as the
  original does; each theme's buildings have their own names again (Spa, Helipad, Casino, Castle and more).
- **0.5.12:** golfer stories play as in the original, in bubbles over the two golfers while you watch them.
- **0.5.11:** the landmark, flower bed or tree you chose is the one placed; sand traps keep the shape you turned them to;
  paths join up as in the original; tees of open holes show their arrow; no more flash of blocky shadows.
- **0.5.10:** the tropical trees show with retail CD installs whose folders have spaces in their names.
- **0.5.9:** problem reports say why a game file could not be opened.

## 0.5 (20 October 2026)

**Highlights**

- **Touch screens:** aim your pro and place buildings with a target you move, then *Swing* or *Place*; look around while
  aiming.
- **Import saved games from the original SimGolf** (.sve), and move saves between devices.
- **Looks:** lit windows at night, dawn sprinklers, winter frost, pixel-perfect scaling and a fly-over showcase.
- **Trees and flower beds grow** after planting, as in the original.
- **Landing heat maps** and a **Text size** setting.
- **Options made clear:** three kinds (Gameplay, Helpers, Looks), four presets and Preferences in plain sections.
- **Updates you can trust:** *Update now* only installs releases signed by the project.
- **High-resolution content packs** and tools for making your own art (0.5.2).
- **Report a problem** from inside the game (0.5.6).

**Fix releases**

- 0.5.4: free graphics for the People panels, eagles and water banks; any number of Snack Bars; backspin and skill awards
  checked against the original.
- 0.5.7: the Analyze Golf Shot button works.
- 0.5.8: your total time played on the title menu.
- Plus many smaller fixes in 0.5.1 to 0.5.6.

## 0.4 (17 October 2026)

**Highlights**

- **Course editor:** build your own scenarios in the game, with heightmap import.
- **History graphs** of the club's and each hole's months.
- **Camera bookmarks** and zooming towards the pointer.
- **Enhanced water:** depth colour, waves, shorelines, caustics, rain rings and frozen ponds.
- **Building tools:** a measuring tape, and rectangle and fill for the terrain brush.
- **Left-handed golfers**, a **cheats box**, quick save, slower seasons and sandbox switches.
- **Back up, import and export saved games**, in the browser too.
- **Use the SimGolf CD itself**, or a CD image (.iso, or a Mac's .cdr), with nothing to install (0.4.1, 0.4.4).
- **More optional extras:** weather that matters, visitors, drag to place, path styles and lamps, live scorecards (0.4.2).
- **Reworked free graphics and sounds** and an interface overhaul (0.4.7).

## 0.3 (15 October 2026)

**Highlights**

- **Controllers and the Steam Deck**, on the desktop and in the browser; full screen in Steam's Game Mode.
- **Share courses** as a course code, and challenge friends.
- **Undo** for building and terrain (Ctrl+Z), **autosave** and save pictures.
- **Enhanced graphics:** life on the course (birds, butterflies, autumn leaves), tournament crowds, grass tufts, winter and
  snow, and weather by place and year.
- **Use your own SimGolf files** from a .zip or a folder, on desktop and Android.

## 0.2 (13 October 2026)

**Highlights**

- **The first OpenSGolf extras:** the People screen, golfer profiles, message history, name tags, report tabs and path help.
- **Bigger courses:** Original, Large, Huge and Giant maps.
- **Fixes for the original game's own bugs** (on by default, and listed in [docs/FIXES.md](docs/FIXES.md)).
- **Remappable keys**, WASD camera, a frame rate cap and v-sync.
- **Windows on ARM** downloads.

## 0.1 (12 October 2026)

- **The first release:** SimGolf playable on Windows, macOS, Linux, Android and in the browser, with your own game files
  or the free graphics.
