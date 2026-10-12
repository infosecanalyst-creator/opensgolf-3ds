# Playing the club

## Your own SimGolf files

Double-click OpenSGolf (`opensgolf.exe`, `OpenSGolf.app`, the AppImage); no options are needed. It looks for your copy of
SimGolf in this order and plays on the first folder that holds the game's data (`Interface/TitleBASE.pcx`, `Data/Textures`,
`Flics` and `Themes`, names in any case; a folder above it such as the copied CD with its `Program_Files_(ENGLISH)` is
followed down up to two levels):

1. `--game DIR` (for scripts and tests; it always wins);
2. the folder you chose in the game, saved as `game_dir` in `settings.json` in the save folder, while it still holds the files;
3. a `SimGolf` or `game` folder next to the program (and, on macOS, in `OpenSGolf.app/Contents/Resources` or beside the
   app; beside the AppImage file);
4. `game` in the save folder, then the usual install places: Windows `Program Files (x86)` / `Program Files` with
   `Maxis\SimGolf`, `EA GAMES\SimGolf` or `Electronic Arts\SimGolf`, and `C:\SimGolf`; macOS `~/Games/SimGolf`, `~/SimGolf`,
   `/Applications/SimGolf`; Linux `~/.local/share/opensgolf/game`, `~/SimGolf`, `~/Games/SimGolf` and a Wine install
   (`~/.wine/drive_c/Program Files*/Maxis/SimGolf` or `EA GAMES/SimGolf`);
5. `game` in the folder it was started from.

With none found it plays on the free base set (opensgolf-free), and the first time the title screen says so next to its
**Use my SimGolf files...** button (top left). That button, and the same line at the end of Preferences, opens the system's
own folder picker (Windows and macOS dialogs, the desktop's XDG portal on Linux). A folder without the game's data is
refused with a message naming what is not there; a good one is saved and OpenSGolf starts again on the original files (a
real restart, so the original's art, sounds, fonts and tables load exactly as on a normal start; from Preferences during a
game it asks first, as anything not saved is lost). On the original files the button reads **Use the free graphics**, which
saves the free set as the base set (Preferences > Content's choice) and starts again. The browser version has its own
picker on the page and no such button; on Android the button says which folder to copy the files to (see Android below).
Test hook: `SG_PICK_FOLDER=DIR` takes DIR as the folder picked, right after start up (`SG_PICK_FOLDER=free` chooses the
free graphics).

A Windows release build is a windowed program (no console window); started from a console with options it writes its
messages to that console. Screenshots (Shift+F6) go to the save folder as `opensgolf-shot.png`.

## The title menu

Start OpenSGolf (from a source build: `opensgolf --game "../game/Program_Files_(ENGLISH)"`). It opens on the title menu, drawn from the disc's own Interface art with the game's font: Start New Game (Standard) or Sandbox Mode opens the property chooser, and picking a property starts a course on that property's theme. A property is paid for out of the §100,000 starting funds (sandbox: unlimited). Esc goes back, and from the course returns to the menu. Continue Saved Game opens Load Previous Game (the disc's Title_LoadGame art): the saved games in a list of 16 rows with the scroll arrows, a click on a row shows its holes, par, yards, cash, ratings, theme pack, designer and course record on the left, the tick loads it, the folder deletes it after a yes/no box, the arrow goes back; saves whose names start with '&' are listed as autosaves. Select A Theme opens Select a Theme Pack (the folders under Themes with what each holds); a click on a row picks it, the tick keeps it, the arrow or a key goes back to Standard. A click on the logo shows the credits (credits.txt from the game folder over creditsbckgrd.pcx; a click or key ends them).

For scripted runs (any of `--png`, `--course`, `--edit`, `--golfer`, `--sandbox`, `--follow`) the game skips the menu and opens on the demo course; `--screen menu|property|play` forces a screen. Add `--theme Desert` for another look there. `--property NAME` (or 0..15) starts a new game on that property straight away, and `--clock MS` fixes the value the exe's random generator is seeded from, so the same land comes back every run.

- **Holes** are built one at a time with the exe's rules (docs/GAMELOGIC.md): paint one Tee tile (G twice, or the Paint tool) and a Putting Green, then press H or click the cup green to open the hole. Its par comes from its planned length. Painting over a hole's tee or cup closes that hole until it is rebuilt. Courses made outside the tool (the demo course, saved terrain) have their tee and green pairs turned into holes when they load. Shift+H hides the story lines.
- **Golfers** follow the exe's rules (docs/GAMELOGIC.md): invited members come in pairs, walk and play every hole, use the amenities, pay a green fee equal to their mood at the end of each hole, and quit for good when their mood falls below zero. Happy members upgrade their membership and bring friends.
- **Fun** sums each hole's mood changes the way the exe does. Happy golfers pay more, unhappy ones less.
- **Money:** you start with §100,000 (confirmed from the publisher's golf.exe and the property screen). The green fee defaults to 1000 per hole, from the scale seen in course reports. Upkeep and wages are charged every game day (120 seconds). When cash stays below zero the board sends warnings and then ends the game, in four steps (the days are placeholders). Sandbox mode never charges.
- **The land** of a new game is built the way the exe builds it: each game deals the properties into sixteen offer slots (which set the acreage and price shown on the chooser), and the land generator lays out trees, creek, coast, ponds and rocks, wildlife spots, the clubhouse, the property's own feature and the difficulty's obstacles, and leaves the land beyond the purchased acreage out of bounds (not drawn) or, on an island, under water. Landmarks, obstacles and free buildings are drawn with the exe's art for the theme. Your own pro starts at the clubhouse and, until the first hole opens, follows the pointer as the exe's pro does; hired staff (People button) work around their post the way the exe's staff do: the Soda Vendor sells drinks to thirsty golfers, the Club Pro chats to golfers, the Ranger hurries them, the Groundskeeper pulls weeds. Click an employee on the course, then click where they should work, to move their post (the scripted hook `--edit "h:KIND,X,Y"` hires one with a post). Weeds grow and golfers complain about grown ones (-2 mood). The console prints events (strokes, fees, board messages).

- **Course Report:** press F1 on the course. It uses the disc's report art and lists every hole: yards, par, average strokes, time, fun, the +Len/+Acc/+Img scores (measured by simulating golfers with and without each skill, as the original's Shot Analysis suggests), type, average fee, revenue and profit. Yards, minutes and the profit split are placeholder scales. Esc, F1 or a click closes it. `--screen report` opens it directly.

Saving: Shift+S (or System Functions, Save the current game) opens the Save dialog ("SAVE GAME, edit name then press Enter"): Enter saves the whole game (land, golfers, holes, staff, money, calendar, theme pack and the random generator, so it continues exactly) to `Saved Games/<name>.sgs` in the save folder and shows "Game Saved"; Esc cancels. Shift+L (or Load a previous game) opens Load Previous Game over the game, whose arrow goes back to it. Test hooks: `--screen load|save|themes|credits|champ|pickapro` opens these screens (with `--property` the load and save screens open over a game), `SG_HOVER=n` lights button n (0 OK, 1 back, 2 up, 3 down, 4 delete) with its tooltip, `SG_SELECT=r` selects row r and `SG_CONFIRM=1` opens the delete box. Test hooks: `--save-game F` saves after the `--time` simulation, `--load-game F` starts from a saved game. `--cash N` sets the starting cash, `--time S` simulates S seconds before taking `--png`, `--edit "t:x,y,type;o"` paints one tile through the hole tool (type 0 tee, 1 green) and opens the hole being built, `--edit "e:x,y,n,r"` raises (or with a negative n lowers) every vertex of the square of side 2r + 1 round vertex x,y by n steps (stills of steps and retaining walls).

## Buildings and garden items

On the Add Buildings panel (or b for benches, Shift+B for a home site; Tab turns the next building) choose: benches, flower beds, ball washers, willow trees and scenic bridges at any time, the amenities as holes unlock them. Prices, clearing charges, upgrades and the path-to-clubhouse rule follow the exe. Benches need rough ground with a free neighbour; bridges go on water near a shore. Shift-click undoes: a path, bench, flower bed, willow or bridge comes off with what it cost back, a building is demolished with no refund. Golfers use what you build: snack bars and the practice buildings earn money on every visit, tired golfers sit on benches, flower beds and landmarks cheer them up. The headless test option is `--edit "b:x,y,kind"`.

## In-game interface (dock, panels, advisor, stories)

The lower left of the course screen now shows the game's own dock art (Interface/3mainLowerLeft.pcx). The three big round buttons open panels along the bottom: **Build Course** (terrain brushes, paths, raise and lower, each with its cost), **Add Buildings** (the amenities for the theme) and **People** (click to hire, right click to fire a club pro, ranger, groundskeeper or soda vendor). Click an item, then click the map. The small buttons zoom, rotate, open the course report, pause, and save the course. The keys are the original's (its F8 screen lists them): g f r s t w p b pick the green/tee, fairway, rough, sand, trees, water, path and bench tools (v the ravine), e the elevation tools, = and - raise and lower the ground under the pointer (Tab steps the elevation brush), h opens the hole being built; z and x zoom between the original's three levels, PageUp and Home turn the view a quarter (0 turns it back), the arrows scroll four tiles, 1 to 9 and Shift+1 to 9 jump to holes 1 to 18; Space drops the tool; Shift+P pause, Shift+T trees, Shift+N names, Shift+R routing map, Shift+W world map, Shift+J begin a tournament, Shift+C save the course for championship play, Shift+S and Shift+L save and load. The original has no game speed control; the port's own ] cycles normal, x2 and x4. With Enhanced graphics on, the port's own c (a key the original leaves unused) opens photo mode (see *Photo mode* below).

System Functions → Preferences → *Keyboard controls...* (the port's own screen) lists every key the game takes, grouped:
moving around the course, time, building, holes and messages, reports and maps, your pro and the golfers, saving and
loading, pictures, and the keys that work everywhere, with the browser's and the Mac's stand-ins. Its pages turn with the
Previous and Next buttons, Left and Right (or PageUp, PageDown, Space), or a click or tap in the box; Esc, Enter, the tick
or a click outside the box close it. (The title menu has no Preferences, as in the original.) `--screen keys` opens it for
stills, `SG_KEYS_PAGE=N` on page N (from 0).

Messages appear at the top of the screen as in the original (the message window): a translucent strip with nobody speaking, or a framed box with the speaker's ball portrait, the newly unlocked building or an icon; a new message replaces the one showing, minor ones wait until it is gone, and Repeat Last Message (Information menu) brings the last one back. On Easy, during the first year, your pro gives advice in a line over his head, in the exe's words. Paused shows at the top centre. Golfer stories play as in the original: a pair that fits a story from your Themes folder gets it at the first tee, and its lines appear at the top as the two walk together (Shift+H hides them); good moods carry it to a Happy Ending, which unlocks a landmark. Special visitors (the Corporate CEO, the County commissioner, the Wealthy heiress) come once your ratings earn them; please them and they invest, approve a land expansion (the TRACTS FOR SALE screen opens) or donate a landmark. Donated and owned landmarks can be placed with the building tool. Story text is read from your disc at run time and is not stored in this project.

## Your pro (Gary Golf)

Open the **People** panel: its first rows are your pro's. **Practice Round** sends him out with a partner; **Match** is lit once a famous golfer has challenged your club, which happens from the second year on when a newcomer with all three skills arrives and you have enough holes. Before his first round you hand out ten skill points (plus three for each accomplishment already earned) in the skill dialog: + and - on each row, then Done. The skills entry opens the dialog again; points won later (his first skill gain, his first match win) are added there.

On each full shot away from the green the game waits for you: move the pointer over the course to aim (the white line shows the shot, bending for a draw or fade; the box sits on the aimed tile or its nearest corner) and click to swing. The panel offers the five shot types (S straight, F fade, D draw, H high backspin, L low punch); fade, draw and high are refused from a hazard. The panel shows his attitude (his momentum), club, distance and lie, with the skills that count for this shot in white and the others in grey. The computer plays his putts and short chips. N, pressed twice, cancels the round; his partner plays on alone.

A match is played for the wager level's stake on each hole and again on the match; losing or tying lowers the stake for the next challenge. Remarkable shots, judged the way the exe judges them, raise one of his skills (up to a cap set by the course's rank); finishing shots in hazards may cost him one. As in the original these are judged only while the view is at its closest zoom and the shot was struck on screen: the game then freezes on a SimFoto of the shot (its track and flight framed in white, red for a bad one) with the message, until you click or press a key (F10 also saves the photo to the snapshots folder); when the photo cannot frame your pro's shot the message goes to the ticker. Other golfers' great shots get a photo too while your pro is not out, the first on each hole. `--screen greatshot` shows one for a still (`SG_GREATSHOT_TEXT` replaces its message). Scripted runs: `--edit "j:0"` starts a practice round and lets him aim at the pin by himself (and take up challenges); `j:0,1` only shows the aim, for stills; `--screen skills` opens the skill dialog.

## SGA tournaments

At the start of the fifth month each year the SGA looks at your course (when your pro is not out). If it passes, the SGA offers a tournament and the People panel shows **Begin Tournament**. Clicking it evaluates the course again: a failing course shows the SGA report (ten criteria scored out of 10, with what the SGA wants for your course's class: length, number of holes, time to play, fun, variety, scenic holes, holes that test length, accuracy and imagination, and facilities joined to the clubhouse); a passing one shows the event, its first prize and **Great, let the games begin!**.

The course empties and a shotgun field starts: two players on each hole, your pro on hole 1 with a famous partner, everyone else a famous pro picked to suit the purse and your cash. A checklist follows: TV towers and booths, extra smooth greens, deeper rough, and each hole's tournament par; untick what you don't want. The leaderboard sits on the left while they play (your pro in green). When the last player is in, the results show each place's prize (the purse for first, two thirds of the place before for each place after, for as many places as you have holes); your pro's prize goes to the club, and places bring trophies. The towers come down, the old golfers come back, and two regular golfers start on each hole. N twice cancels a tournament with no prizes. Scripted runs: `--edit "g:0"` asks the SGA (the evaluation must pass), `g:1` starts one straight away with the default purse, `g:2` the same with no skill points card or trophy room in the way (for stills of the leader board and the Professional Tournament scorecard); `--screen sga` shows the report.

## Play a Championship

In a game, Shift+C (or System Functions, *Save course for Championship*) saves your course for championship play and System Functions, *Save your pro for Championship* saves your pro (into a `Championship` folder next to your saves; pro files use the exe's layout, so the disc's `Themes/Championship/Gary Golf.pro` is listed too). From the title menu, **Play a Championship** lets you pick a difficulty, then a course (Select Championship Course, the Load screen's layout with the course's numbers on the left) and a pro (Pick A Pro: portrait, the ten skills as +N0%, the signature saying), then plays a tournament on it straight away: $100,000 in cash, June, every preparation item taken, the event named by difficulty (SGA Qualifying School to SGA Open Championship), no wages and no accomplishments. When the results are closed you are back at the title menu; nothing is saved. Scripted runs: `--champ COURSE.sgch PRO.pro`, and `--edit "x"` saves the course.

## Life on the course

- **Thoughts:** every mood event shows a line over the golfer who says it (or the partner, for remarks about the golfer), white then grey as it fades, green or red by its tone. Lines start with the words the exe's labels show; the rest is written for this port, since the exe's strings are data.
- **Wildlife:** when you retype the land, an animal whose habitat and theme fit a tile next to it may move in with a young one (elk, cranes, flamingos, sheep, crocodiles, snakes, gila monsters, roadrunners, ducks). They wander, graze or drink, stay in their habitat and run from golfers and low balls; a golfer who startles one is pleased on the easier difficulties. Which kind lives where is our choice (the exe's table is data). A hot-air balloon, a sailboat, a bird of prey and a blimp pass over now and then.
- **Water:** a ball in the water splashes; ripples line the shores, rocks and coral sit in the shallows, and scenic water has dolphins or a fountain.
- **Celebrity homes:** a home site bought by a celebrity becomes one of two celebrity houses, and the celebrity walks about the lot and now and then does a signature move.
- Test hooks: `a:x,y,kind` places an animal, `y:x,y` makes a celebrity home.

## The clubhouse and the records

- Click the clubhouse to choose who tees off next: the golfers waiting are shown newest first; pick two and click outside the cards (no picks cancels).
- People panel, **Golfers on the course**: every golfer playing (hole, name coloured by thirst, hunger, tiredness or anger, and mood). Click one to centre on them and open their card: the five bars (a long bar is bad), the scorecard against par and recent thoughts; X closes it.
- **F10** shows the Professional Accomplishments board, which also opens by itself after each of the 22 accomplishments (from the first Challenge hole to the 100 star rating): a photo card for each, the plaques on the trophy and the next three things to do.
- At each new year the year-end report compares cash, fun, skill and membership with last year and lists the year's highlights from the event log (holes opened, buildings built, matches won, top-rated holes, celebrity homes, tournament places, Happy Endings, land bought).
- **F9** shows the Membership Roster; **F7** shows the SGA evaluation. `--screen pair|board|yearend|roster|golfers` opens them for stills.

## Reports (F keys)

As in the original: **F1** Course Report, **F2** Player Comments (the twenty most frequent comments over all holes, with the hole where each is heard most and how often per tee shot; recent months count more, as every hole's statistics lose an eighth each month), **F3** Histograph (skill, cash, fun and membership month by month, with the event log's highlights), **F4** Financial Report (the ledger by year: green fees, home sites, food and drink, course building, facilities, salaries, maintenance and interest, other, total), **F5** Routing Map (the course in miniature with each hole's tee-to-green line; tabs for course routing (par, yards, minutes), employees and their work areas, course aura (where golfers were happy or unhappy) and home site value), **F7** SGA Evaluation, **F8** Keyboard Shortcuts, **F9** Membership Roster, **F10** Professional Accomplishments. `--screen comments|histograph|finance|routing|shortcuts` opens them for stills. On the routing map, left click selects a hole and right click moves it to another place in the order (the holes between shift, golfers follow; the exe allows this in every tab). **F6** opens the world map to move the club to another property: cash, the calendar and your pro's career go along and the new course starts afresh (Shift+F6 takes a screenshot).

## Keys in the browser and on a Mac

Every key of the original works in the browser too: the page keeps the browser from acting on the keys the game uses
(F1 help, F3 and / find, F5 reload, F6 and F7, F10, Tab, Backspace, Space and the arrows, PageUp and Home scrolling the
page), and a key pressed after the page lost the game's focus goes to the game and gives it the focus back. Cmd
shortcuts stay the browser's; the game needs none. F11 (full screen) and F12 (developer tools) are left to the browser.

The port's own stand-ins, in the browser and on the desktop: **Alt + 1 to 9 and 0** (on a Mac **Option**, ⌥) open what
F1 to F10 open, for keyboards whose function keys need Fn (Macs, most laptops); Ctrl with a digit does the same where
the browser lets it (on a Mac it always does; elsewhere Ctrl + digit may switch tabs). Shift goes with them as with the
F keys (Shift + Alt + 6 is Shift+F6). A Mac keyboard has no PageUp or Home: **fn + ↑** is PageUp and **fn + ←** is Home
(turn the view), or use the dock's turn buttons. In the browser a screenshot (Shift+F6), a SimFoto and a great shot's
photo (F10) are downloaded; saved games and the accomplishment snapshots stay in the browser's storage.

Content packs (mods, docs/MODDING.md): System Functions, Preferences, *Content packs...* lists the installed packs; switch them on or off and order the mods there (Up and Down, or PageUp and PageDown on the picked row). The choices are saved in `settings.json` at once and take effect the next time the game starts. `--screen content` opens the screen for stills.

Scenarios (docs/MODDING.md 5): with a scenario pack installed, *Start New Game* first offers a free game (the original's
way on: Select Difficulty, then a property) or one of the scenarios, each with its goals; a scenario that names no
difficulty goes by way of Select Difficulty. Messages say when a goal is reached or missed and when the scenario is won or
lost; *Information, Scenario Goals* (the menu's last line, shown when the game has goals) lists them. The Save dialog's
*Export as scenario pack* button (or Tab) writes the game as a scenario pack into the save folder's `packs`, under the
name typed. Scripts of the enabled packs run by themselves (docs/MODDING.md 6); when one fails a toast says so and the
Content screen shows the error. *Find more...* on the Content screen lists the packs of the content index set in
`settings.json` (`content_index_url`, docs/CONTENT_INDEX.md) and downloads them. Test hooks: `--scenario ID` starts a
scenario pack (Moderate when it names no difficulty), `--screen scenarios` (from the title) and `--screen goals` or
`--screen content-index` (with a game) open those screens, `SG_INDEX_DOWNLOAD=N` downloads offer N of the index at once,
`SG_EXPORT_SCENARIO=1` exports the game as a scenario pack.

## Touch (phones and tablets)

The same touch controls work in the browser and in the Android app (crates/simgolf/src/touch.rs).

Tap for a click; drag one finger to scroll the course (or to paint with the selected tool); hold a finger still for half a
second for a right click (undo, remove, close a screen). With two fingers: drag to scroll, pinch to step through the three
zoom levels, twist to turn the view a quarter. Once the screen has been touched, four buttons on the right stand in for keys
that have no button in the game: Open hole (h), Turn (Tab), Speed (]) and Message (?). Naming a course or a save raises the
phone's keyboard (in the browser through a hidden text box on the page, on Android the system keyboard).

### Android

Installing the APK (sideloading) is described in the README, [INSTALL.md](INSTALL.md). The app runs in landscape and
full screen, starts on the free base set, and keeps its saves, settings, content packs and championship files in its private
storage (`Context.getFilesDir()`, removed when the app is uninstalled). To play with your own copy of SimGolf, copy the
game's `Program_Files_(ENGLISH)` folder to `Android/data/org.opensgolf.game/files/game/` on the device's shared storage (the
app creates `game/` the first time it starts; the folder's contents straight in `game/` work too) and restart the app (the
title screen's **Where do my SimGolf files go?** shows the folder). On
Android 11 and newer the phone's own file manager may hide `Android/data`; copy from a computer over USB instead.
The back button works as Esc: it closes the screen that is open, on the course opens System Functions (save, load, quit),
on the title screens goes back to the title menu, and on the title menu leaves the game. Sound goes through the phone's
audio output (AAudio or OpenSL ES, through cpal's oboe backend); an APK whose sound could not be built (tools/android_build.sh
falls back to a silent build) plays without it. The content packs and translations that the desktop packages put next to
the program are not inside the APK yet, and the app's packs folder is in its private storage (TODO: bundle the packs as
assets and read player packs from the external files folder).

## Enhanced graphics

System Functions → Preferences → the *Enhanced graphics* tick, or *Enhanced graphics settings...* for the details (the port's own, not in the original; docs/ENHANCED.md) turns on a
way of drawing the game for large and wide screens: sharp text, smooth interface art (with smooth outlines for the colour-keyed buttons, icons and sprites instead of stair steps), the HUD at the window's corners on a
wide window, sharper and anti-aliased course, a smooth camera and smooth motion. Each part has its own tick; Classic stays
the default. With the smooth camera the mouse wheel and a pinch zoom continuously between the three levels (z, x and the
zoom buttons still step to the next level), turns are animated, arrow-key scrolls glide and a dragged course coasts to a
stop. The screen has three pages, chosen with the tabs along its top (a click or a tap; Left, Right or Tab on the
keyboard): *Display* (the master switch and the options above), *Effects* (the shader effects, cast shadows, the balls'
trail and impacts, and the effects' quality, which a click steps through low, medium and high) and *Atmosphere* (sky, time
of day, cloud shadows, haze, rain, mist and the post-processing). The master switch heads every page. `--enhanced` and
`--classic-render` switch it for one run. Test hooks: `--edit M:5` opens the screen (`M:5,P` on page P, 0 to 2);
`SG_POINTER=x,y` moves the pointer and `SG_CLICK=x,y[;x,y...]` clicks, in window pixels, after the first frame of a
scripted still (for checking where clicks land on a wide window).

### Photo mode

With Enhanced graphics on, **C** (or the camera button under the rating pills, top right; a tap on a phone) opens photo
mode (docs/ENHANCED.md, "Photo mode"): the interface goes, the game pauses (Space or P runs it on), and the camera is free:
drag or the arrows to move, the wheel, a pinch, Z / X or + / - to zoom well past the usual levels, PageUp / Home to turn. A
click or tap puts the focus there; F turns the focus blur on and off, [ and ] set its strength, L steps through the looks
(Natural, Warm, Vivid, B&W, Vintage), V the vignette, R the size (window, twice the window, 4K), Enter takes the photo, H
hides the hint and buttons, Esc or C leaves and puts everything back. Buttons along the bottom do the same by mouse or
touch. Photos are PNGs in `Photos` in the save folder; the browser downloads them; on Android they go to
`Android/data/org.opensgolf.game/files/Photos`. The Display page's *Photo mode* tick turns the key and the button off. Test
hook: `--edit "P:LOOK,BLUR,VIGNETTE,ZOOM,HINT,SHOOT"` opens photo mode for stills (LOOK 0 to 4, BLUR 0 to 5, VIGNETTE 0 or
1, ZOOM in percent of the closest level or 0, HINT 1 to show the hint and buttons, SHOOT 1 to 3 to take a photo at the
window's size, twice it or 4K); `SG_CLICK=x,y` sets the focus first.

### Enhanced graphics: the effects

The effects are ticks on the same screen; `--fx` turns on only the optional shader effects for one run (docs/ENHANCED.md, "Effects"): smoother magnified sprites and interface art,
animated water, terrain light with occlusion and shadows, a sky beyond the map, colour grading, bloom and vignette.
`--fx-set LIST` picks options (`scaler=off|sharp|edge`, `water`, `lighting`, `occlusion`, `shadows`, `sprite-shadows`,
`sky`, `tilt-shift`, `grading`, `bloom`, `vignette`, `cast-shadows`, `time-of-day`, `clouds`, `haze`, `rain`, `mist`,
`ball-trail`, `ball-impacts` = on/off, `quality=low|medium|high`), `--fx-list` prints them, and `OPENSGOLF_FX` does the
same from the environment. Without them the game looks exactly as before. Test hooks: `SG_FX_BENCH=N` with `--png` prints
the frame time over N frames, `SG_FX_LOG=1` the light map's build time and each ball's landing (where, on what, at which
tick), `SG_FX_DUMP=FILE` writes the light map, `SG_FX_SOFT=sharp,edge` overrides the scaler's softness (texels),
`SG_FX_HOUR=P` holds the time of day at phase P (0 dawn, 0.15 morning, 0.5 midday, 0.85 golden hour, 0.89 sunset, 0.95
dusk) and `SG_FX_RAIN=1` (or `0`) makes every day wet (dry) while rain is on.

### Time of day and weather

With *Time of day* on, one game month is one day of light: the month opens in the morning, has a long midday in the
original's light, a golden afternoon, a sunset and a short blue dusk, then the next month dawns. The sun swings round the
front of the view, so shadows lengthen sideways rather than down the screen, and the course never gets darker than an
easy-to-read dusk; the interface is not touched. *Rain* wets about one day in four (never on desert or tropical courses, more on the
links), the same days whenever the save is loaded; the light dims and the shadows fade while it rains. *Mist* lies in
hollows and over water, thick at dawn and gone by late morning with the time of day (a light mist all day without it).
*Cloud shadows* drift over the course, *haze* pales the far (upper) part of the view, more when zoomed out. None of these
changes the game: golfers play the same in rain or at dusk.

## Language

The game is in English unless a translation is installed (docs/TRANSLATING.md): then System Functions > Preferences has a
`Language:` line that switches to the next language and remembers it (`"language"` in `settings.json`). Test hooks:
`--lang CODE` uses a language for one run, `--lang-file FILE` lays a translation file over the English, and the
environment variable `SG_TEXTLOG=FILE` writes every text drawn or posted to the message window to FILE, each once.
