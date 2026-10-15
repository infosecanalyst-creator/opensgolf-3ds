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
**Use my SimGolf files...** button (top left). That button, and the same line at the end of Preferences, asks whether the
files are a .zip of the SimGolf folder or the folder itself, then opens the system's own picker for it (Windows and macOS
dialogs, the desktop's XDG portal on Linux, Android's file picker). A .zip is unpacked into `game` in the save folder, by the
browser version's rules: the game folder is the one holding `Flics`, at any depth; `__MACOSX`, `._` files, videos and
Windows programs are left out (crates/simgolf/src/import.rs). On macOS a picked folder is copied there as well (macOS's
privacy rules for Desktop, Documents and Downloads may refuse the folder on a later start). A folder without the game's data
is refused with a message naming what is not there, a folder the system does not let the game read with a message saying
so; a good one is saved and OpenSGolf starts again on the original files (a
real restart, so the original's art, sounds, fonts and tables load exactly as on a normal start; from Preferences during a
game it asks first, as anything not saved is lost). On the original files the button reads **Use the free graphics**, which
saves the free set as the base set (Preferences > Content's choice) and starts again. The browser version has its own
picker on the page and no such button; on Android the files are copied into the app (see Android below).
Test hook: `SG_PICK_FOLDER=DIR` takes DIR (a folder or a .zip) as picked, right after start up (`SG_PICK_FOLDER=free`
chooses the free graphics); on Android the intent extra `sg_pick` does the same
(`adb shell am start -n org.opensgolf.game/.MainActivity --es sg_pick PATH`).

A Windows release build is a windowed program (no console window); started from a console with options it writes its
messages to that console. Screenshots (Shift+F6) go to the save folder as `opensgolf-shot.png`.

## The title menu

Start OpenSGolf (from a source build: `opensgolf --game "../game/Program_Files_(ENGLISH)"`). It opens on the title menu, drawn from the disc's own Interface art with the game's font: Start New Game (Standard) or Sandbox Mode opens the property chooser, and picking a property starts a course on that property's theme. A property is paid for out of the §100,000 starting funds (sandbox: unlimited). Esc goes back, and from the course returns to the menu. Continue Saved Game opens Load Previous Game (the disc's Title_LoadGame art): the saved games in a list of 16 rows with the scroll arrows, a click on a row shows its holes, par, yards, cash, ratings, theme pack, designer and course record on the left, the tick loads it, the folder deletes it after a yes/no box, the arrow goes back; saves whose names start with '&' are listed as autosaves. Select A Theme opens Select a Theme Pack (the folders under Themes with what each holds); a click on a row picks it, the tick keeps it, the arrow or a key goes back to Standard. A click on the logo shows the credits (credits.txt from the game folder over creditsbckgrd.pcx; a click or key ends them).

**Check for updates** (the port's own button, top right of the title menu, on Windows, macOS, Linux and Android) asks
GitHub for OpenSGolf's newest release and says whether you have it. When a newer version is out, **Open the download
page** opens its release page (https://github.com/crspytopgn/opensgolf/releases/latest) in the system's browser, where you
download and install it as the first time; the game never downloads or replaces itself. Privacy: nothing goes online until
you click the button, and then only one request to `api.github.com` (GitHub sees your IP address and
`OpenSGolf/<version>` as the program asking, as for any web page); nothing about you or your games is sent. The browser
version is always the newest, so it has no such button, nor has a build without the `net` feature. Test hook:
`SG_UPDATE_URL=file:///path/latest.json` checks at start against a saved answer (`{"tag_name":"v0.4.0"}`) instead of GitHub.

For scripted runs (any of `--png`, `--course`, `--edit`, `--golfer`, `--sandbox`, `--follow`) the game skips the menu and opens on the demo course; `--screen menu|property|play` forces a screen. Add `--theme Desert` for another look there. `--property NAME` (or 0..15) starts a new game on that property straight away, and `--clock MS` fixes the value the exe's random generator is seeded from, so the same land comes back every run.

- **Holes** are built one at a time with the exe's rules (docs/GAMELOGIC.md): paint one Tee tile (G twice, or the Paint tool) and a Putting Green, then press H or click the cup green to open the hole. Its par comes from its planned length. Painting over a hole's tee or cup closes that hole until it is rebuilt. Courses made outside the tool (the demo course, saved terrain) have their tee and green pairs turned into holes when they load. Shift+H hides the story lines.
- **Golfers** follow the exe's rules (docs/GAMELOGIC.md): invited members come in pairs, walk and play every hole, use the amenities, pay a green fee equal to their mood at the end of each hole, and quit for good when their mood falls below zero. Happy members upgrade their membership and bring friends.
- **Fun** sums each hole's mood changes the way the exe does. Happy golfers pay more, unhappy ones less.
- **Money:** you start with §100,000 (confirmed from the publisher's golf.exe and the property screen). The green fee defaults to 1000 per hole, from the scale seen in course reports. Upkeep and wages are charged every game day (120 seconds). When cash stays below zero the board sends warnings and then ends the game, in four steps (the days are placeholders). Sandbox mode never charges.
- **The land** of a new game is built the way the exe builds it: each game deals the properties into sixteen offer slots (which set the acreage and price shown on the chooser), and the land generator lays out trees, creek, coast, ponds and rocks, wildlife spots, the clubhouse, the property's own feature and the difficulty's obstacles, and leaves the land beyond the purchased acreage out of bounds (not drawn) or, on an island, under water. Landmarks, obstacles and free buildings are drawn with the exe's art for the theme. Your own pro starts at the clubhouse and, until the first hole opens, follows the pointer as the exe's pro does; hired staff (People button) work around their post the way the exe's staff do: the Soda Vendor sells drinks to thirsty golfers, the Club Pro chats to golfers, the Ranger hurries them, the Groundskeeper pulls weeds. Click an employee on the course, then click where they should work, to move their post (the scripted hook `--edit "h:KIND,X,Y[,F]"` hires one with a post; F bit 1 the experienced version, bit 2 arms Move). Weeds grow and golfers complain about grown ones (-2 mood). The console prints events (strokes, fees, board messages).

- **Course Report:** press F1 on the course. It uses the disc's report art and lists every hole: yards, par, average strokes, time, fun, the +Len/+Acc/+Img scores (measured by simulating golfers with and without each skill, as the original's Shot Analysis suggests), type, average fee, revenue and profit. Yards, minutes and the profit split are placeholder scales. Esc, F1 or a click closes it. `--screen report` opens it directly.

Saving: Shift+S (or System Functions, Save the current game) opens the Save dialog ("SAVE GAME, edit name then press Enter"): Enter saves the whole game (land, golfers, holes, staff, money, calendar, theme pack and the random generator, so it continues exactly) to `Saved Games/<name>.sgs` in the save folder and shows "Game Saved"; Esc cancels. Shift+L (or Load a previous game) opens Load Previous Game over the game, whose arrow goes back to it. Test hooks: `--screen load|save|themes|credits|champ|pickapro` opens these screens (with `--property` the load and save screens open over a game), `SG_HOVER=n` lights button n (0 OK, 1 back, 2 up, 3 down, 4 delete) with its tooltip, `SG_SELECT=r` selects row r and `SG_CONFIRM=1` opens the delete box. Test hooks: `--save-game F` saves after the `--time` simulation, `--load-game F` starts from a saved game. `--cash N` sets the starting cash, `--time S` simulates S seconds before taking `--png`, `--edit "t:x,y,type;o"` paints one tile through the hole tool (type 0 tee, 1 green) and opens the hole being built, `--edit "e:x,y,n,r"` raises (or with a negative n lowers) every vertex of the square of side 2r + 1 round vertex x,y by n steps (stills of steps and retaining walls).

## Course size

OpenSGolf can start a game on a map larger than the original's. The choice is on Select Difficulty (Start New Game and
Sandbox Mode), in the box at the bottom left: a click steps through the sizes. Original is the default and plays exactly as
SimGolf does; scenarios and championships always use their own course's size.

| Size | Map | Yards a side | A sandbox (all the map) | Tracts for sale |
|------|-----|--------------|-------------------------|-----------------|
| Original | 50 x 50 tiles | 1,250 | 250 acres | 9 of 16 x 16 tiles |
| Large | 74 x 74 | 1,850 | 548 acres | 9 of 24 x 24 |
| Huge | 98 x 98 | 2,450 | 960 acres | 9 of 32 x 32 |
| Giant | 128 x 128 | 3,200 | 1,638 acres | 9 of 42 x 42 |

What changes with the size (n is the map's edge in tiles, 50 on the original's map):

- **The view** zooms further out, so the whole map fits on the screen as the original's does at its farthest zoom: one
  more step (half the original's farthest) on Large and Huge, two (a quarter) on Giant.
- **The land** is made by the same generator with its counts and places scaled: the woods are laid in n^2/2500 passes, there
  is one creek per 50 tiles of edge, the sea's width (5, between 2 and 12 tiles on the original) and the ponds' grid scale
  with n, the wildlife spots, property landmarks and difficulty obstacles grow with the area, and the clubhouse stands in the
  middle of the map. Hills use a mix of two readings of the original's height noise so they do not repeat across the map.
- **The deal:** a property comes with its slot's acres times n^2/2500 (the same share of the map), and its price, the
  starting funds and the sandbox funds are multiplied by n/50. So on Huge (n = 98) the cheapest slot costs §98,000 out of
  §196,000 starting funds and brings about 3.8 times the acres.
- **Buying land:** the Tracts for Sale screen still shows nine tracts on its diamond, each (n - 2)/3 tiles square. A tract's
  price is rolled tile by tile as in the original (and still doubles with every purchase), so land costs the same per acre on
  every size; a bigger tract costs more in proportion to its acres.
- **Golfers and staff** walk across the whole map: the path search's cost limits scale with n. Weeds sprout n^2/2500 times as
  often (the same pace per acre), and the club may employ 20 x n^2/2500 people (at most 64). The routing map and the tract
  screen draw the whole map in the original's diamond, with tiles 50/n as large.
- **One original bug is fixed on the larger sizes only:** the exe's distance function comes out 8 times too long when both
  legs are over 16 tiles; on a larger map the distance is the true one. The original's map keeps the original behaviour.
- **Saves** record the size: a game on a larger map is saved as version 2 with its size and loads at that size; version 1
  saves (every save made before course sizes, and every Original game, byte for byte as before) load as before. OpenSGolf
  0.1 does not know the size field, so open larger-size saves with 0.2 or later; this version refuses a save from a newer
  version or one whose map does not match its size instead of loading it wrong.

Scripted runs take `--course-size N` (an edge from 50 to 200 tiles) with `--property`. With `SG_TICK_BENCH=1` a scripted run
prints the mean and slowest simulation step and the time of each terrain rebuild.

Limits: the object table holds 255 buildings, landmarks and lots (as the original's byte-sized index allows; the port refuses
the 256th), the terrain is rebuilt whole after every edit (about 80 ms on Giant against 30 ms on Original, desktop CPU), and the
Enhanced light and detail maps use fewer texels per tile on the larger sizes (at most about 400 texels a side).

## Buildings and garden items

On the Add Buildings panel (or b for benches, Shift+B for a home site; Tab turns the next building) choose: benches, flower beds, ball washers, willow trees and scenic bridges at any time, the amenities as holes unlock them. Prices, clearing charges, upgrades and the path-to-clubhouse rule follow the exe. Benches need rough ground with a free neighbour; bridges go on water near a shore. Shift-click undoes: a path, bench, flower bed, willow or bridge comes off with what it cost back, a building is demolished with no refund. Golfers use what you build: snack bars and the practice buildings earn money on every visit, tired golfers sit on benches, flower beds and landmarks cheer them up. The headless test option is `--edit "b:x,y,kind"`.

## Original game or OpenSGolf extras

Select Difficulty has a **Game** box next to Course size. One click switches between:

- **The original**: every OpenSGolf extra off and Classic graphics; the game exactly as it was (fixes for the original's
  own bugs stay on). A new player starts here.
- **OpenSGolf extras**: the interface extras (People screen, message history, name tags, report tabs, path help, autosave,
  save pictures, course sharing) and the
  gameplay extras that do not change the game's balance (turning buildings, animals, more staff).

If you ticked your own choice in Preferences, the box says "your own mix"; a click goes back to the original. The choice
is saved and is what new games start with.

## OpenSGolf extras (optional, not in the original)

Preferences (System Functions, Preferences) has an **OpenSGolf extras...** line. It opens a box of optional changes,
all off unless you tick them, on three pages (tabs along its top; a click on a tab, Left and Right or Tab turn them):
**Gameplay**, **Time & sandbox** and **Interface** (the interface extras, see *OpenSGolf extras* further down). With all
of them off the game plays exactly as the original. What you tick on the first two pages applies to the game you are
playing, is saved with that game (a save made with everything off is unchanged), and is what new games start with (kept
in settings.json under `extras`).

- **Tab turns placed buildings and the clubhouse.** With no building tool in hand, Tab turns the building under the
  pointer a quarter: the clubhouse, putting greens, the other amenities and landmarks (the driving range and home sites
  keep their way; the range is drawn tile by tile). Only the picture turns: footprints are square, so paths, golfer
  routes, putting, the clubhouse door and the ratings do not change. With a garden item or the landmark tool in hand, Tab
  steps through its designs (benches, flower beds, scenic trees, bridges, landmarks) as the strip of pictures over the
  panel shows them. The original already turns a putting green or building you are about to place with Tab, and picks a
  random design when you pick a flower bed, scenic tree or bridge (and the first bench); you then choose from the strip.
- **Cut content: unused landmarks.** Fourteen pieces of art that are in the game files but that the original never loads
  (docs/CUT_CONTENT.md) can be placed as landmarks: press the Landmark button on the Amenities panel, and press it again
  to switch between your own landmarks, the cut ones and (with the next extra) the animals. Each costs $6,000 (the port's
  price), golfers enjoy the sight as they enjoy a landmark's (and the course planner treats them as features), but they
  have none of the four landmark powers. They stay for good, as landmarks do.
- **Hardcore economy.** Tighter money and choosier golfers: by default wages 150%, hole maintenance 200%, land (the
  property and tracts) 150%, debt interest 5% a month, at most $600 per golfer per hole in fees, every bad moment costs a
  golfer one more mood step, and at most 12 employees. The numbers are data/economy.toml's `hardcore_*` fields (docs/MODDING.md).
- **Higher staff limits.** The club may have 40 employees instead of the original's 20 (data/economy.toml's
  `staff_limit_high`, at most 63: the original keeps its walkers in 64 records).
- **Buyable animals.** The landmark strip gets a page of the theme's animals (elk and cranes in Parkland, snakes, gila
  monsters and road runners in the Desert, flamingos, crocodiles, snakes and ducks in the Tropics, elk, sheep and ducks on
  the Links). A click on open ground of your course buys a pair (leader and young, $2,000) that drops in as wild animals do
  and then keeps within about three tiles of where it was set down; bought animals never wander off for good. Golfers who
  glance at a tile next to one enjoy the sight (the scenic thought, +1 mood, as a landmark's sight gives), and scaring one
  is the same as scaring a wild one. Up to 32 pairs. The Preferences' "Ambient animals" tick hides and stops them, as it
  does the wild ones.
- **Set par.** The game gives each hole its par from its length when it opens, and sometimes judges a hole too hard or
  too easy for it. With this on, Hole Stats (click a hole on the Course Report) shows **-** and **+** buttons beside the
  par (or press minus and plus there); after a confirmation the hole's par is one less or one more, from 2 to 6. A par you
  set is shown in blue and counts everywhere par does: golfers' scores and their moods over them, birdies and eagles, the
  hole's hard and easy marks and its name, the reports and the best rounds. It is saved with the game and moves with the
  hole on the Routing screen. Rebuilding the hole (painting over its tee or cup green and opening it again) gives it the
  game's par anew, a tournament's checklist sets its own pars, and switching the extra off puts every hole back to the
  game's par. Pars cannot be changed during a tournament.

On the **Time & sandbox** page:

- **Slower seasons: the calendar at 1x / 1/2x / 1/4x.** A click steps the speed (1x is the original). The month and year
  move on at half or a quarter of the original's pace while everything that happens by the game's own clock keeps its
  pace: golfers arrive, play and pay as before, staff work, weeds grow, and wages and hole maintenance are charged as
  often as ever (the original charges them every 1024/(difficulty+2) game ticks, not by the month). So a year holds two or
  four times as much golf, and two or four times as much income and as many wage and upkeep bills. What goes by the
  calendar waits for it: the month's start (2% interest on debt, the monthly reminders about buildings off the paths, the
  month's entry in the history the year-end report and the Histograph draw, the holes' statistics easing by an eighth, the
  scripts' month hook and the goals' checks), the year end (the board's look at the cash, the year-end report, the Top 10,
  the Financial Report's year, the goals' deadlines), the SGA's mid-year tournament offer (once a year, so at 1/2x half as
  often in play time), every date the game shows (the dock, SimFotos, the message history, accomplishments, hiring dates)
  and Enhanced graphics' seasons (spring blossom and autumn colours, winter snow and its snowy years). Enhanced graphics'
  days of light and weather stay on the game's own clock, so a month holds two or four days of light at the slower
  speeds. Things that measure how long you have played rather than the time of year also stay on the game's clock: the
  course's welcome story and design tip, the advisor's first year on Easy, the opening story pair and the famous golfers'
  challenges (from the second year of play). Switching the speed in the middle of a game carries on from the date
  reached; a championship always keeps the original calendar.
- **Sandbox: pause the calendar.** The time of year never moves on (it wins over the speed above): golfers keep coming
  and paying, wages and upkeep keep being charged, but no month starts and no year ends, so there is no interest, no
  board, no year-end report and no SGA tournament offer while it is on, and the season stays as it is. Untick it to let
  the calendar go on from where it stood.
- **Sandbox: freeze golfer happiness.** No golfer's mood ever drops: a bad moment, fatigue at the end of a hole, a sad
  story beat are all undone, while good moments still lift moods. A pair's mood is still set at the first tee from how
  well the two get on (their starting mood). Nobody leaves unhappy.
- **Sandbox: unlock all buildings and landmarks.** Every building kind, home sites whatever the members' demand and all
  fourteen landmarks of the theme are on the panels from the start, as in Sandbox Mode; the unlock announcements are not
  made. Upgrading a building still needs a Country Club course (10 holes), as in Sandbox Mode. Unticking it takes back only
  what the course has not earned; landmarks the club owns stay.

The three "Sandbox:" switches are what a Sandbox Mode game (unlimited money) is most often wanted with, but each works in
a normal game too.

Test hooks: `--extras BITS` sets the extras a new game starts with (1 Tab turns, 2 cut content, 4 hardcore, 8 higher staff
limits, 16 animals, 32 set par, 64 or 128 the calendar at 1/2x or 1/4x, 256 pause the calendar, 512 freeze golfer
happiness, 1024 unlock all; not saved to the settings); `--edit "R:N"` asks for par N on the Hole Stats' hole (its
confirmation box; `R:N,1` sets it at once); `--edit "X:BITS"` sets the extras of the game being played; `--edit "T:x,y"`
presses the Tab of the first extra with the pointer on tile x,y; `--edit "S:page[,design]"` opens the Amenities panel with the
landmark tool on strip page 0 (own), 1 (cut) or 2 (animals); `--edit "b:x,y,4,t"` places landmark type t (19..32 the cut ones,
100 + kind an animal pair); `--edit "M:6"` opens the extras box (`M:6,P` on page P: 0 Gameplay, 1 Time & sandbox, 2
Interface).

## Searching the Preferences

Preferences has a **Search options...** line, and typing a letter or digit while Preferences is open does the same: a
box that finds an option on any of the Preferences' pages by its words. Type to narrow the list (each word you type must
start a word of the option or of its page's name, in any order and any case; Backspace takes back a letter); it shows up
to ten options at once with the page each is on (an Enhanced graphics page, an extras page, *Graphics* for Enhanced
graphics itself, *Keys* for the keyboard controls), and how many more there are. A click on a found option ticks or
unticks it, or steps a setting with choices, as on its own page; a click on a key opens Change keys with that action
waiting for its new key. **Show only what differs from the original game** keeps the options that are not as a new player
has them. **Reset everything to the original game...** asks first, then puts every option back: every gameplay and
interface extra off, Enhanced graphics off with all its options at their defaults (the bug fixes on, as they are by
default and as the "Original game" choice of Select Difficulty keeps them), and every key back to the original's. The
search box opens again afterwards on the same words. As in the other Preferences boxes nothing changes until its OK
(Enter, or the tick); Esc leaves everything as it was.

Test hooks: `--edit "M:7"` opens the search box (`SG_PREFS_SEARCH="words"` types them first, `SG_PREFS_CHANGED=1` ticks
*Show only what differs*), `--edit "M:8"` its reset question.

## In-game interface (dock, panels, advisor, stories)

The lower left of the course screen now shows the game's own dock art (Interface/3mainLowerLeft.pcx). The three big round buttons open panels along the bottom: **Build Course** (terrain brushes, paths, raise and lower, each with its cost), **Add Buildings** (the amenities for the theme) and **People** (click to hire, right click to fire a club pro, ranger, groundskeeper or soda vendor). Click an item, then click the map. On a Mac, Ctrl+click is a right click (in the app and in the browser), as is a two-finger click on a trackpad with *Secondary click* on. The small buttons zoom, rotate, open the course report, pause, and save the course. The keys are the original's (its F8 screen lists them): g f r s t w p b pick the green/tee, fairway, rough, sand, trees, water, path and bench tools (v the ravine), e the elevation tools, = and - raise and lower the ground under the pointer (Tab steps the elevation brush), h opens the hole being built; z and x zoom between the original's three levels, PageUp and Home turn the view a quarter (0 turns it back), the arrows scroll four tiles, 1 to 9 and Shift+1 to 9 jump to holes 1 to 18; Space drops the tool; Shift+P pause, Shift+T trees, Shift+N names, Shift+R routing map, Shift+W world map, Shift+J begin a tournament, Shift+C save the course for championship play, Shift+S and Shift+L save and load. The original has no game speed control; the port's own ] cycles normal, x2 and x4. With Enhanced graphics on, the port's own c (a key the original leaves unused) opens photo mode (see *Photo mode* below).

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

## OpenSGolf extras

The port's own additions to the interface, none of them in the original (crates/simgolf/src/extras.rs and the modules it
names). They only add a screen, a tag, a strip, a button or a save. A new player starts with them off; the Game box on Select
Difficulty turns them on in one click, and System Functions, Preferences,
*OpenSGolf extras...* has a tick for each on its Interface page (kept in `settings.json` under `ui_extras`), and `--no-extras` turns them all off
for one run. With every tick off the game draws and plays exactly as before (the stills of every report, panel and menu
compare pixel for pixel with the build before the extras).

- **People** (**Shift+G**, or *Information, People*): every golfer on the course, sortable by name, type (visitor,
  member, silver, gold, pro, VIP), hole, mood, handicap and money spent this round (greens fees, food and drink, wagers);
  every employee, sortable by name, job, date hired, weekly wage, golfers served and pay so far; and a *Your pro* tab with
  the original's pro screens a click away (Customise, skills, practice round, match, tournament, under the Player panel's
  own conditions). Click a heading to sort (again to reverse). For a golfer: *Locate* (the camera there), *Follow* (the
  camera keeps on them until they leave, Space, an arrow key or a drag), *Card* (the golfer card) and *Profile*. For the
  staff: *Hire...* (the original's hire dialog with its rules and wages), *Fire* (the Employee panel's, 25 units under
  Salaries), *Move* (the next click on the course is the new post), *Rename* (also the Employee panel's Rename button, which
  the original had and the port lacked), *Locate* and *Follow*. Keys: Left/Right or Tab the tab, Up/Down the row,
  PageUp/PageDown, Enter locate, F follow, C card, P profile; H hire, Delete fire, M move, R rename; 1 to 5 the pro's
  buttons; Esc closes.
- **Golfer profile** (People's *Profile*, or the golfer card's *Profile* button): the portrait, the card's identity lines,
  the roster's record (rounds, best, handicap), the round in progress, the golfer's bio (the person record's own, which
  Customise's Update Bio edits; *Edit bio* or B types one here) and everything the golfer has said on the course this game,
  newest first, good thoughts in green and bad ones in red (kept with the save, the last 60 per golfer).
- **Message history** (**Shift+M**, *Information, Message history*, or a click on the ticker): every ticker message with
  its date and its speaker's portrait or picture, newest first, the last thousand, saved with the game. A click on a message
  takes the camera to the golfer it is about (and opens the card) or to the place it is about.
- **Name tags** (**Shift+H** turns them off and on): the golfer or employee under the pointer gets a small tag with the
  name, a golfer's face of the moment and type or an employee's job, whether the original's names (Shift+N) are on or not.
  Not drawn while a tool is armed, over the dock and its panel, or after the screen has been touched.
- **Tabbed reports**: over the F1 to F10 screens, a strip of ten tabs flips from one report to another without going back
  to the course (on a window taller than 4:3 the strip sits above the page). Click or tap a tab, press Tab or Shift+Tab, or
  another report's F key (or Alt/Ctrl with its digit); the report's own key and Esc close it as before.
- **Path help**: **Shift+V** (or *Information, Path view on / off*) marks every path tile, green when joined to the
  clubhouse and orange when not, and puts a *No path* marker on each building the paths do not reach (the buildings the
  original leaves closed) and on each hole with no joined path within three tiles of its tee, with a legend in the upper
  right. The original's monthly warning that a building is not operational gets a **Show** button on the ticker that takes
  the camera to the building.
- **Autosave**: every five minutes of play (time on the course, not in reports or menus) the game is saved to Saved Games
  as *autosave (Latest)*, overwriting the last one, so a crash or a closed tab loses at most five minutes. Test hook:
  `SG_AUTOSAVE_SECS=N` saves every N seconds of play instead. Load it from Load Previous Game like any save.
- **Save pictures**: Load Previous Game draws a map of the selected save's course in the list's top right corner: fairways,
  greens, sand, water, woods, paths and buildings, with the land the club does not own darker. It reads the save itself, so
  older saves get a picture too.
- **Course sharing**: System Functions, *Share course (copy a course code)* copies a code for your course (it starts
  `OPENSGOLF-COURSE-1:`) and saves the course for championship play. Send the code to a friend in a message or a text
  file. They press **Ctrl+V** (Cmd+V on a Mac) anywhere in OpenSGolf, or click *Paste a course code* on Play a
  Championship's course list, and the course joins their championship courses. Dropping a shared `.sgch` course, a saved
  game (`.sgs`) or a text file holding a code on the window brings it in too. The challenge: your course record travels
  with the course, and the course list shows it to your friend as the score to beat.

Test hooks for stills: `--screen people|staff|pro|history|profile|pathview|tags|ticker|card` (with a game;
`SG_PEOPLE_SORT=COL[,1]` sorts the People list by column COL, downward with 1), `--edit "M:6"` opens the extras box.

**Ctrl+Z** (Cmd+Z on a Mac) undoes the last click or drag with a building or terrain tool that changed the course:
terrain, paths, holes, buildings and garden items. The whole game goes back to just before it, money included, as if that moment had been saved and loaded.
Up to 30 steps are kept; the list starts empty with each game and is not saved. (The port's own; not in the original.)

On Windows, macOS and Linux, **F11** or **Alt+Enter** switches between a window and full screen (in the browser, use the
browser's own full screen, F11).

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

Content packs (mods, docs/MODDING.md): System Functions, Preferences, *Content packs...* lists the installed packs; switch them on or off and order the mods there (Up and Down, or PageUp and PageDown on the picked row). The choices are saved in `settings.json` at once and take effect the next time the game starts. `--screen content` opens the screen for stills. *Save profile...* keeps the current choices under a name and *Profiles...* lists the saved ones to load (the choices then take effect at the next start) or delete. A game saved with other packs than the ones in use asks before it loads from *Load Previous Game*: switch to the save's packs (the desktop game starts again on them and loads the save; on Android and in a browser restart or reload, then load it), load it anyway, or not load it (docs/MODDING.md 2).

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

## Changing the keys

The port's own (crates/simgolf/src/controls.rs and controls_ui.rs), in the browser and on the desktop alike. With nothing
changed every key does what it did before.

* **Change keys.** System Functions → Preferences → *Keyboard controls...* → *Change keys...* lists every action of the
  course screen with its key: the original's (tools, screens, the F keys, the shot shapes while your pro aims) and the
  port's own (game speed, photo mode, hiding the interface). Click an action, then press its new key, with or without
  Shift; Esc (or a click) while it waits keeps the old key. Two actions on one key are printed in red and a line says
  which: only one of them answers (the shot shapes only clash with each other, since they count only while your pro
  aims). *Reset to defaults* puts every key back. The digits (holes 1 to 18) and Esc cannot be given to an action. The
  keys are saved at once under `"controls"` in `settings.json` (only the ones that differ, e.g.
  `"controls": {"wasd": false, "keys": {"water": "Shift+Q", "zoom_in": "G"}}`; an action set to `""` has no key) and the
  *Keyboard controls* screen prints the keys in use.
* **WASD camera** (off by default). W, S and D are the original's water, sand and (while your pro aims) draw keys, so
  WASD scrolling is a choice on the Change keys screen: with it on, **w a s d** scroll as the arrow keys do (the arrows
  still work), the water tool moves to **q** and the sand tool to **k**; while your pro aims, s and d stay the straight
  shot and the draw.
* **Middle-button drag** (always on) drags the course whatever tool is held, as a left drag does with no tool.
* **u** hides the whole interface (dock, panels, messages, the money and ratings) during normal play, for a clear view of
  the course; a small note in the bottom left corner says which key brings it back. Clicks then go to the course. Photo
  mode (Enhanced graphics) has its own view and is not affected.
* **Frame rate.** Enhanced graphics settings → *Display* has *Frame rate cap* (30, 60, 120, 144, 240 a second or
  uncapped, the default) and *V-sync* (on by default); both count with Enhanced graphics off too. Only how often the
  screen is drawn changes: the game itself always runs on its 87 ms ticks. On the desktop the cap waits out the rest of
  each frame; v-sync takes effect at the next start (macOS and Android always pace to the display). In a browser the page
  skips animation frames to keep under the cap, and v-sync cannot be turned off: browsers always draw on the display's
  refresh.

Test hooks: `SG_PRESS="u;Shift+Q;..."` presses keys (the names `settings.json` uses) after a scripted still's first frame,
after `SG_CLICK`; letters and Space also type their character.

## Controllers and the Steam Deck

An Xbox, PlayStation, Switch Pro or Steam Deck controller plays the whole game (the port's own; on Windows, macOS, Linux
and in the browser, not in the Android app yet):

| Control | Does |
|---|---|
| Left stick | moves the pointer (the game draws it while the controller is in use; moving the mouse gives it back) |
| Right stick | scrolls the course |
| A | click; hold to drag or paint |
| B | on the course a right click (undo, remove); on other screens Esc (close, back) |
| X | Tab (turn a building, or the next design) |
| Y | game speed (]) |
| LB / RB | turn the view |
| LT / RT | zoom out / in |
| D-pad | the arrow keys (scroll, lists) |
| Start | Esc (System Functions: save, load, quit) |
| Back / Select | pause |
| Left stick click | open a hole (h) |
| Right stick click | the last message again (?) |

**Steam Deck:** download the Linux AppImage (`opensgolf-linux-x86_64.AppImage`) in Desktop Mode, make it executable
(Properties, Permissions, "Is executable"), then in Steam use *Games, Add a Non-Steam Game to My Library* and pick it.
In Game Mode the game starts full screen and the Deck's controls work as above (pick the *Gamepad* controller layout if
Steam offers another). The trackpads move the mouse as usual. To name a course or a save, press Steam + X for Steam's
keyboard. `--fullscreen` starts full screen on any computer (F11 or Alt+Enter switches while playing).

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
storage (`Context.getFilesDir()`, removed when the app is uninstalled). To play with your own copy of SimGolf, tap **Use my
SimGolf files...** and pick a .zip of the SimGolf folder or the folder itself in Android's file picker
(ACTION_OPEN_DOCUMENT / ACTION_OPEN_DOCUMENT_TREE, no storage permission needed): the app copies it into
`Android/data/org.opensgolf.game/files/game/`, showing how far it is, and starts again on it. Copying the game's
`Program_Files_(ENGLISH)` folder there by hand works too, but Android 11 and newer hide `Android/data` from file managers and
USB transfers (`adb push` still reaches it).
The back button works as Esc: it closes the screen that is open, on the course opens System Functions (save, load, quit),
on the title screens goes back to the title menu, and on the title menu leaves the game. Sound goes through the phone's
audio output (AAudio or OpenSL ES, through cpal's oboe backend); an APK whose sound could not be built (tools/android_build.sh
falls back to a silent build) plays without it. Everything the game prints goes to the system log (tag `OpenSGolf`), and a
crash is also written to `Android/data/org.opensgolf.game/files/crash.txt` (INSTALL.md says how to get either). A GPU that
will not compile an Enhanced graphics shader turns those effects off for the run instead of closing the game, and a game
folder without the course textures (a copy cut short) is set aside for the free base set. The content packs and translations that the desktop packages put next to
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
`ball-trail`, `ball-impacts`, `reflections`, `wind`, `evening-lights`, `seasons`, `after-rain`, `lightning`, `life`, `crowds`, `winter`, `sharpen`, `grass` = on/off,
`quality=low|medium|high`, `day-length=short|normal|long`), `--fx-list` prints them, and `OPENSGOLF_FX` does the
same from the environment. Without them the game looks exactly as before. Test hooks: `SG_FX_BENCH=N` with `--png` prints
the frame time over N frames, `SG_FX_LOG=1` the light map's build time and each ball's landing (where, on what, at which
tick), `SG_FX_DUMP=FILE` writes the light map, `SG_FX_SOFT=sharp,edge` overrides the scaler's softness (texels),
`SG_FX_HOUR=P` holds the time of day at phase P (0 dawn, 0.15 morning, 0.5 midday, 0.85 golden hour, 0.89 sunset, 0.95
dusk) and `SG_FX_RAIN=1` (or `0`) makes every day wet (dry) while rain is on. For stills of the newer effects:
`SG_FX_MONTH=M` holds the calendar month (3 March .. 10.99 late October, fractions allowed: 4.5 is mid April),
`SG_FX_WIND=S` or `S,DEG` fixes the wind's strength (0 calm .. 1 a strong breeze) and its direction (degrees from the
screen's right towards its top), `SG_FX_WET=W` makes the ground that wet with its puddles that far spread,
`SG_FX_RAINBOW=K` shows a rainbow, `SG_FX_LIGHTNING=K` a lightning flash of that brightness, and `SG_FX_FACETED=1` lights
the ground with the tiles' flat faces as before the smooth normals (for comparisons).

### Time of day and weather

With *Time of day* on, a day of light lasts three game months (about four and a half minutes at normal speed; *Day
length* on the Atmosphere page picks short, one month as in the first versions, normal, or long, six months): the day
opens in the morning, has a long midday in the
original's light, a golden afternoon, a sunset and a short blue dusk, then the next month dawns. The sun swings round the
front of the view, so shadows lengthen sideways rather than down the screen, and the course never gets darker than an
easy-to-read dusk; the interface is not touched. *Rain* follows the property's climate
(rarely in the desert, often in Ireland, Scotland and Wales) and each year is wetter or drier than usual, the same days
whenever the save is loaded; the light dims and the shadows fade while it rains. *Mist* lies in
hollows and over water, thick at dawn and gone by late morning with the time of day (a light mist all day without it).
*Cloud shadows* drift over the course, *haze* pales the far (upper) part of the view, more when zoomed out. None of these
changes the game: golfers play the same in rain or at dusk. Rain spells, the cloud cover and the wind last a whole day of
light and blend into the next day's over the first and last eighth of it, so the weather comes and goes gently (the short
day length keeps the first versions' sudden changes).

The **Weather** page and the newer Atmosphere options (docs/ENHANCED.md has the details):

- *Reflections and sun glitter on the water* (on): trees, buildings and golfers standing near water are mirrored in it,
  rippling with the waves; the sun's glitter grows warm and strong at golden hour; the water laps at the shore.
- *Wind* (on): each day has its wind; trees, palms, bushes and tall grasses lean and sway in gusts, flags fly downwind, the
  rain slants and the clouds drift with it.
- *Lit windows, lamps and fireflies at dusk* (on, with the time of day): from sunset the buildings' windows glow, lamps
  light up beside the buildings and along the paths, and fireflies drift over the rough on parkland and tropical courses.
- *Seasons* (on): blossom on the trees and fresh greens in March and April, autumn colours in September and October, frost
  and dew on spring and autumn mornings (with the time of day). Palms, cacti and conifers barely change.
- *Wet ground, puddles and rainbows* (on, with rain): during a rainy day and drying through the next the ground is darker
  and glossy, puddles fill on level fairway and rough and shrink as they dry, and about every other time the sun comes out
  after rain a rainbow stands in the sky.
- *Lightning in heavy rain* (off): flashes and distant flicker in the heaviest rain on parkland and links courses.

## Language

The game is in English unless a translation is installed (docs/TRANSLATING.md): then System Functions > Preferences has a
`Language:` line that switches to the next language and remembers it (`"language"` in `settings.json`). Test hooks:
`--lang CODE` uses a language for one run, `--lang-file FILE` lays a translation file over the English, and the
environment variable `SG_TEXTLOG=FILE` writes every text drawn or posted to the message window to FILE, each once.
