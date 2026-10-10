# Facts read from screenshots of the original game

Supplied by the project owner. Only things clearly legible are recorded; the course report numbers were too small to read reliably and are left out.

## Currency and clock
- Money is shown with the section sign (the Sims "simoleon" symbol), for example §100,000, not a dollar sign.
- The game has a calendar with months and years. Seen: "April 2001" on a new course and "June 2011" on a mature one. The "two years to get back into the black" warning is therefore calendar years. How many real seconds a game month lasts is not known yet.

## Main menu (five buttons)
Continue Saved Game, Start New Game (Standard), Sandbox Mode, Select A Theme, Play a Championship.

## Choosing a property ("Where will you build your golf course?")
- The player starts with §100,000 and the property price comes out of it: affordable ones are yellow, ones that cost too much are grey, bought ones red. So the cash left to build with is 100,000 minus the price (this corrects the manual-derived note that cash "depends on the property").
- Sixteen properties, with size and price:

| Property | Acres | Price | Bonus |
|---|---|---|---|
| Northeast | 50 | §50,000 | Civil War Battlefield |
| San Diego | 50 | §60,000 | Dolphins |
| Oahu | 50 | §70,000 | Japanese Garden |
| Hawaii | 70 | §80,000 | Scenic Waterfall |
| Wales | 80 | §200,000 | Stonehenge |
| Florida | 90 | §150,000 | Free Pro Shop |
| Las Vegas | 100 | §120,000 | Fun, Fun, Fun |
| Monterey | 110 | §250,000 | Scenic Cypress |
| Spain | 120 | §300,000 | Scenic Vineyards |
| Scotland | 130 | §400,000 | Free Castle |
| Carolina | 160 | §500,000 | Free Putting Green |
| Ireland | 170 | §600,000 | Leprechauns |
| Nova Scotia | 170 | §800,000 | Scenic Lighthouse |
| Rocky Mtns. | 180 | §700,000 | Free Hotel |
| Jamaica | 200 | §900,000 | Scenic Statues |
| Phoenix | 210 | §1,000,000 | Free Spa |

Only Northeast, San Diego, Oahu and Hawaii are affordable at the standard start. There is a world map with pins, and a back and a refresh button.

## Player skills screen
- Ten skills in this order: Power Hitter, Long Driver, Accurate Driver, Accurate Irons, Accurate Putter, Draw Shot (R to L), Fade Shot (L to R), High Backspin Shot, Recovery Skills, Luck.
- Each skill point shows as +10 percent (a character with 5, 4 and 1 points shows +50%, +40%, +10%). This confirms the manual's "10 percent per point". Points are added before play and more are won with trophies ("Add N skill points").

## Main screen
- Top left: a club badge with the course name and the date, a row of star rating dots, and a rank such as "1st" with a multiplier.
- Top right, three counters: money, a smiley (fun, which can go above 100, e.g. 112.0) and a third value shown with two decimals (possibly the skill rating, unconfirmed).
- Bottom: round buttons on the left (view controls, pause, info, settings), a golfer-face button, and a purple tray of terrain tools. Terrain brush shapes (a curved fairway and others) sit in the tray.
- A course has grey paths, retaining walls, rivers with bridges, buildings, carts and golfers.

## Information menu entries
Repeat Last Message, Course Report, Player Comments, Routing Map, Histogram, SGA Evaluation, Financial Report, Membership Roster, Professional Accomplishments, World Map, Best Scores, Top 10 Designers.

## Course report (table, one row per hole plus a total)
Columns: Yds, Par, Avg (average strokes), Time (minutes per hole), Fun (a percentage that can exceed 100), +Len, +Acc, +Img (three decimal scores, positive or negative, that decide the hole type), Type (for example Breather), Avg.Fee, Revenue, Profit (red when negative). Markers for Top 100 hole, Top 18 hole and Scenic Hole.

## Second batch: hole stats, shot analysis, membership

### Hole Stats dialog (click a hole)
- Title "HOLE STATS for N <hole name> (N)". Holes have names.
- Rows: Fun Factor (a percentage, here more than 100, with a word label such as "outstanding"), Length, Accuracy and Imagination (signed decimals, each with a word label), Yards, Avg. Drive (yards), Par, Stroke average.
- "Average shots on this hole": a histogram of how many golfers took 3, 4, 5, 6, 7 and 8 or more strokes.
- Comments: a list of golfer remarks, each with the fun percentage it carried (green when high, red when low), for example about a fountain, the variety of the course, a ball under a palm tree, a scenic bridge.
- The calendar runs far ahead in a long game (a date of May 2252 was seen).

### Shot Analysis ("Golfers playing hole N...")
- It draws sample shot paths on the hole and lists three comparisons: golfers with all skills, then "no Imagination skill", "no Accuracy skill" and "no Length skill", each with a distance in yards (seen: -4, -41 and 8).
- This strongly suggests how the +Len, +Acc and +Img scores are measured: by simulating golfers with and without each skill and seeing how much worse the shots get. A hole demands accuracy if golfers lacking it do much worse, and so on. This is far better than the hazard counting placeholder in sg/holes.h and is the plan for replacing it.

### HUD counters (cross checked with the exe's text)
- The exe's state dump prints "SkillRating=%.2f" and "FunRating=%d". The three HUD counters are therefore: money, then fun rating as a whole number (seen 1035, 1044, 2575), then skill rating with two decimals (seen 6.24, 16.07, 25.11, 79.21). So the club's fun rating is a whole number that can reach thousands; the per hole Fun Factor is a separate percentage. This corrects the 0 to 100 "fun" in sg/economy.h.
- The top left badge shows the course name, the month and year, a row of dots (the star rating) and, in one shot, a coloured bar.

### Membership
- A popup with a golfer's face announces upgrades, for example a golfer moving to Silver membership, that Silver members pay well for home sites, and that he will bring a friend. It ends with the current counts: Silver members, Basic members and Visitors (seen 15, 15 and 17). Tiers seen: Basic and Silver (the exe also has Gold and Platinum).
- Golfers show speech bubbles ("Birdie. This hole is too easy.", "really improved my putting.", "There's nothing like a good snack.") and name labels; staff are labelled with their job (Groundskeeper, Club Pro).

### Editing tray (Desert theme)
Tees, Green, Sand trap, Desert, Pot bunker, Brush, Water, Fairway, Firm fairway, Rough, Waste bunker, Ravine, Rocks, then trees and plants (Cactus, Joshua tree, Palm tree). Another tray shows terrain shaping tools (raise and lower hills). A price of §500 appeared near the tray, probably for the selected item; unconfirmed.

## Third batch: sandbox property screen, fees, tournaments, celebrities

- In Sandbox Mode the property screen shows "Unlimited §" in place of the money box and every property is available; the cards then show only the name and bonus (no acres or price line).
- Theme per property, now clearly visible from the card icons: Parkland (tree): Monterey, San Diego, Rocky Mtns., Northeast, Carolina. Links (castle): Ireland, Scotland, Wales, Nova Scotia. Desert (desert building): Las Vegas, Phoenix, Spain. Tropical (dolphin): Hawaii, Oahu, Florida, Jamaica. (sg/properties.h follows this.) The card icons are in the disc's Interface/Choose*Buttons.pcx files, the title art in TitleBASE/TitleUnSel/TitleMO.pcx, and report backgrounds in Interface/infoscreens.
- "Select A Theme" on the title screen chooses a theme pack, not a terrain style. The Themes folder on the disc holds Standard, Firaxis, More_Stories, The_Sims and Championship.
- Fees: two Course Report screenshots show average fees of roughly 875 to 1,100 per hole and per hole revenues of 4,000 to 66,600, with profit in red or green. So a green fee is on the order of a thousand, not the 40 that was a placeholder here. The default is now 1000 (still a placeholder for the rule), with upkeep and wages scaled to match.
- Course report hole rows carry names (Alexandria, Dorothy, Elizabeth...) as well as numbers, show types such as Breather, Heroic, Freeway and Challenge, and mark Top 100, Top 18 and Scenic holes.
- Events seen as popups: the SGA offers to hold a named tournament at your course with a top prize (here §110,000) and a button in a panel to begin it; a celebrity buys a vacation home on the course and golfers enjoy seeing celebrities. The badge top left can also show star and heart counts (for example x20, x17, x14, x17).
- Staff walk the course with name labels (for example "Joe Groundskeeper"), and an airship flies over the course in one shot.

## Footage of the original (YouTube thumbnails, October 2026)

Frames from public gameplay videos (thumbnails only; the videos themselves could not be fetched from the build server).
Nothing from them is stored in the repository.

- Badge: the course name, not the state: "Ocean Grove MC" (Florida), "Flamingo Shores MC" (Hawaii), "Dolphin Coast MC",
  "Jurassic Springs MC", "County Kincaide GC" with the month and year under it. Fixed in `hud_course_name`.
- Hills are shaded smoothly; no hard triangle facets. Matches TERRAIN.md (vertex normals are the normalised sums of the
  face normals); the port had used face normals. Fixed in `terrain::build_tile_triangles`.
- Golfer names: small white text with a dark outline under the golfer. Thought lines: grey text on a translucent dark bar.
- Money popups float as red "-300" at the build cursor.
- A new hole shows "Hole 2 / 420 yards / Par 4" in white, centred over the tee.

## Main screen overlays against the Giant Bomb footage (2012, two parts)

Measured on frames scaled to the game's 800 x 600 (nothing from the video is stored here). Webcam cut-outs over the
picture's edges were ignored.

- Badge: the course name is in the pills' font (Manual SSi Bold 15), not Arial, white over a black shadow one pixel
  below, capitals from y 11; the date is Arial Bold 10, also white over black, capitals from y 26 (three frames from
  2001 to 2007 agree). Fixed in `hud_ui::draw_badge`. The plate's darkening (half) and the pills' places match.
- The shadowed text calls (0x404bc0 / 0x404ad0, "palette colour 1") draw a dark red shadow, not black: a red fringe is
  plain under the cash pill's green digits, "Hole 2 / 256 yards / Par 4", "Press 'h' to open hole", the SimFoto date and
  "Paused" (over black too, so it is not the video's chroma). Now `ui::SHADOW_1` (0x800000) in the pills, the hole
  label, the floating money, the SimFoto date (which had no shadow) and "Paused" (which had none).
- Message box beside a speaker: 4, 5, 7 and 8 line messages have frames 27..137, 27..137, 25..167 and 24..182, i.e. 16
  pixels shorter than the port drew, and short texts are centred: the text height beside a speaker is 15 a line plus
  0x10 (not 0x20). Fixed in `message_ui::draw_ticker`; the speaker ball, the text's left edge and the wrap match.
- Tournament leader board: the translucent dialog frame from the top left corner (right line x 141, bottom 158 with ten
  rows), the title "LEADER BOARD of" / "the §120,000" / "2004 San Diego Open" (the property's name, not the club's),
  rows "N. Name (E)" in white with the pro in green, black shadows. The port had a plain box, the purse last, a score
  column and the rows in yellow. Rewritten in `tourney_ui::draw_leaderboard`; the SGA offer box now names the same
  "2004 San Diego Open".
- Dock: the open panel's big button stays gold and a gold wire (cuts of 3mainLowerLeft.pcx) runs from it along the
  rim to the panel. Now drawn from the exe's decoded dock draw, see the dock panels section below.
- Matching already: golfer names (Arial Bold 10, white, no shadow, top at the golfer's feet), thought bars (Arial
  Bold 10 on a half black 10 pixel bar, green for a pleased newest thought), hole label size and spacing, the SimFoto's
  border, frame line and "SimFoto" caption, pill text, "Paused" place.
- Not changed: one 5 line message during a shot (p1 3000) sits lower (box from y 137), maybe placed for the shot camera;
  the aim line's dark edge looks softer than the port's black line but the video cannot settle it; "Paused" is a pixel
  taller in the port (font rasterising).

Test hooks added for these stills: `SG_MESSAGE="speaker;text"` posts a ticker message (a golfer slot, -1 for nobody, or
a picture code such as -21 for the laurel ball, -4 for the club emblem), `SG_PAUSED=1` pauses.

## Dock panels in footage of the original (Giant Bomb "On The Green With Sim Golf", part 1)

Measured on frames scaled to the game's 800 x 600 and compared with port stills; nothing from the video is stored here.

- Dock buttons (p1 1600, 1750-1775, 4440): the open mode's big button is gold and a gold trail runs from it along the
  dock to the panel; hovering a big button gives its bright blue look, not gold. Confirmed by the exe's dock draw
  (0x432ba0): hover cut column 0 at the table 0x4c7960, mode cut column 100 and trail cut (300, 100 * mode) at (34,509),
  (95,525), (142,555); the paused Pause button blinks gold. Fixed in `render.rs` (`DOCK_LIT`, `DOCK_TRAIL`); the port had
  drawn the gold look on hover and nothing for the open mode.
- A game starts with the Build Course panel open (dock mode 0, as the exe's load and new game leave 0x567afc), with no
  tool armed, and the Green button carries the theme's waving flag while the hole has no green (0x433190: sprite 0x189 +
  theme at slot + (31, 24), still unless the Green tool is armed). Fixed in `app.rs` (new game, load) and the terrain panel.
- Add Buildings info box (p1 4440, 4460): "Putting Green / Cost: 10,000 / Helps imaginative golfers": no money sign, an
  opaque cream box from y 455 (the pop-up frame grows 160 x 100 to 160 x 111 and moves up, 0x40d0b0) whose lower part is
  covered by the panel (the box is drawn before the body). Fixed in `panels_ui.rs` (`lot_info`, `popup_frame`).
- Amenity slot tooltip (p1 2171, 2172): two tooltip bars at the slot, "Scenic Bridge" and "10000" (the cost in dollars as
  a plain number), both as wide as the name; "Course Terrain" sits 20 right of the pointer. Decoded from 0x432f.. (slot
  table 0x4c7a98, Landmarks "5000-20,000", Undo without cost) and replaces the placeholder box.
- Flower strip (p1 4300-4345), Elevation panel and grid (p1 4040-4080), Employee panel with Joe Groundskeeper / Sally Soda
  Vendor (p1 3860-4000, 5180): match the port; no change.
- Golfers panel (p1 1760): pill cards per pair with a badge (hole number, "x" going home), italic names, mood faces and
  four story hearts; the port had a text list. Now drawn from MemberPanel as the exe does (0x435760, `member_panel.rs`).
  The People button opens it unless the Employee overlay flag is set (0x561254).
- Player panel while the pro aims (p1 2860, 2940, 3220): "Attitude: calm / Club: 4 Iron / Distance: 142 yds / Lie: tees"
  in dark blue Manual 15 centred on x 406 and the skills, white where they count for the shot and grey otherwise; the port
  had shown them in a box over the map. Now in the panel (0x41b62f, `player_panel.rs`). The scorecard ("Exhibition",
  "§2,000/hole" from x 320) matches.

## Giant Bomb and G4 footage: dialogs and full screens (October 2026)

Measured on frames of the Giant Bomb "On The Green With Sim Golf" videos (part 1 at 2200, 2320, 2700, 3000 to 3060,
3120, 3160, 3580, 3800, 4540 to 4660, 4820 s) cropped to the game's 800 x 600, and the G4 review clip (Information menu,
Routing Map tabs). Near the top of the screen the crops sit about 1.5 pixels high, which was taken as registration error.
Nothing from the footage is stored in the repository.

- **End of Year**: the title reads "END of YEAR: 2001" (colon, wide gaps, as UI_SCREENS 7 says); the course keeps about
  0.72 of its brightness (the info screens' 0.25 dim, not 0.6); the dock, the name plate, the rating pills and the
  leader board are hidden while golfers' names and words stay; the tick is the blue OkStates cut at (550, y + 8) over the
  art's gold one; a rise is in the dark green 0x1284. Fixed in `draw_year_end` and `draw_hud`.
- **Hire an Employee**: wages read "Club Pro: 300 per week" (no dollar sign); the pointed line sits on a solid yellow bar
  231..521 by 16; the lines' capitals start 1 below the band (5 higher than before); a Municipal course lists the skilled
  lines in black; the figures carry their soft shadows; the blue OK tick stands at (553, 422) and closes the dialog;
  the screen under it is dimmed by about 0.3. Fixed in `draw_hire_dialog`, `draw_walking` and `hire_click`. Left: the
  figures stand about 3 pixels left of the port's (the exe's 576 is kept).
- **Generic popup (0x46d6e0)**: an opaque box in the lavender of its option balls (148, 150, 198) with a light top and left
  and a shaded bottom and right edge, no dim; headings centred 18 apart in white over the dark red shadow a pixel below, the first
  capital 14 below the call's top; options 24 apart from 23 below the last heading, teal, with the dark ball of
  general_selectionBOX 11 into the box and the text 42 in; the option under the pointer white with the shadow and the lit
  ball; height (lines * 3 + 3) * 8 from 4 above the call's top; width the longest heading + 0x31 or the longest option +
  75. Checkbox lists (the tournament recommendations at (400, 100)) draw pale yellow headings, white options over dark
  shadows, cream boxes ticked in green and the yellow OK tick. No OK ball in the radio boxes. The Information menu's
  heading is "Information...". All built on `popup_ui::ChoiceBox`; the SGA offer, the skill points waiver and the
  recommendations now use it.
- **Retirement question** (System Functions, Quit), at (400, 100): "After a short career / Gary Golf plans his
  retirement." with " Wait, I don't want to quit yet!", " I'd like to save this game first.", " So long for now." Ported
  as `PopupKind::Retire`; the last answer leaves like the career's end. PLACEHOLDER: when it says "lengthy" (10 years
  on here) and whether the save answer quits afterwards.
- **SGA offer**: "The SGA offers to hold the / 2004 San Diego Open tournament / at your course with a / first prize of
  §120,000.": the property's place names the Open. The report's tick is the blue cut, the offer up or not.
- **Tournament recommendations**: "In preparation for the tournament the / following changes have been recommended:"
  then "Roll your greens extra smooth and fast.", "Increase the depth of rough and deep rough grass.", "Change hole 1
  from a par 4 to a par 3." ... all ticked. The TV towers are not a line of the list (they stand during the tournament
  anyway), so their bit stays set.
- **Skill points card**: the card is solid black inside (0x40cef0 called with 1); "Add 16 skill points." in white;
  "Draw Shot (R to L)" and "Fade Shot (L to R)"; the OK ball is the yellow cut. For an accomplishment's three points the
  card stands over the trophy room under a thin light frame (200..529 from y 11) with "Add three skill points to your
  player..." in cyan. The waiver reads "You haven't used all your skill points! / Do you really want to exit?" with "
  Yea, I don't need no stinkin' skill points." and " Whoops, my bad.". Left: the explanation box under the card on its
  first opening ("Before you play your course you may customize your character ...") is not placed yet.
- **Tracts for Sale**: one sentence, "Buy tract #1, 22 acres of / rough, trees, and water", wrapped to 165 with 17
  between lines from 1 above the card's text origin, then the price; the map's tract numbers are black. Fixed in
  `draw_land`.
- Unchanged, matching: Top Ten Designers layout, Shot Analysis box (282..537), the trophy room's layout. Left: the board's
  tack colours vary (yellow, red, grey seen) and are not decoded; the port keeps the grey tack.
