# Translating OpenSGolf

Every text the game shows comes from a string table, in the OpenTTD style. The English table, `lang/en.txt`, is built
into the program; a translation is a second file with the same keys and the text in another language. A key the
translation does not have shows in English, and a key nobody has shows as the key itself (so a missing text is easy to
spot).

Text that comes from the player's own game files (the golfer stories, the characters' names and sayings, the credits, the
terrain descriptions in `Interface/<theme>.txt`, the celebrities and pro golfers lists) is not in the table: it shows as
the files have it.

## The file

A translation is a UTF-8 text file named after its language code, `lang/<code>.txt`: `de.txt`, `fr.txt`, `pt_BR.txt`
(letters, digits, `-` and `_`). Start from a copy of `lang/en.txt`:

```
##language de
##name Deutsch
#
# comments start with #
STR_MSG_CHALLENGE                       :{PRO} fordert {GOLFER} zu einem Match heraus!
STR_TOAST_NOT_ENOUGH_MONEY              :Du hast nicht genug Geld.
```

* `##language <code>` and `##name <name>` at the top: the code and the name the Preferences show for the language.
* One text per line: the key, any spaces, a colon, then the text. **The text is everything after the colon, spaces
  included.** Some English texts start or end with a space because the game joins them with other pieces (the
  remarkable-shot message is built from about ten of them); keep those spaces where your language needs them. Your editor
  must not trim spaces at the ends of lines (the repository's `.editorconfig` says so for `lang/*.txt`).
* The options of a pop-up box (Information, System Functions, Preferences, the quit question, the land offer, ...) are the
  lines whose text starts with a space; the other lines are its headings. Keep the leading space on the options.
* Escapes: `\n` a line break, `\t` a tab, `\\` a backslash, `\{` and `\}` braces that are not a parameter.
* Lines starting with `#` and blank lines are ignored. You do not need to keep the order or the comments, but keeping the
  English file's order makes it easy to compare.
* Leave out what you have not translated yet: those texts stay English.
* The letters the game can draw: Latin-1 (Western European) and Latin Extended-A (Central European: Hungarian ő ű, Polish
  ą ł, Czech č ř and the rest), plus the typographic quotes ‘ ’ ‚ “ ” „, the dashes – — and the ellipsis …. Where the game's
  own font has no glyph for one of the added letters it is drawn from Arial Bold (or the bundled stand-in), else as the
  plain letter. Anything else (Cyrillic, Greek, CJK) shows as `?` for now.

## Longer texts

Translations are often longer than the English the screens were laid out for. You do not have to squeeze your words:
a label that is too wide for its button, tab, menu sign, panel or screen title, or for the widest a pop-up box can grow
(the width of the screen), is first drawn a little smaller (down to three quarters of its size), then, where its box is
tall enough, wrapped onto a second line, and only as a last resort cut short with an ellipsis (…), so it never runs over
its neighbours. Message windows and descriptions wrap as before. Text that fits is drawn exactly as it always was. Still,
prefer a shorter word where your language has one: smaller and cut text is harder to read. `SG_TEXTFIT=1` (an
environment variable) prints every text the game had to fit this way, each once, with the width it had.

The boxes and places this covers:

* buttons and tabs of the game's own screens, the pop-up boxes (their lines and page tabs), the title menu signs, Select
  Difficulty, the info screens' titles and the Load list;
* the building's info box over the Build Course panel (name, cost, what it does): a description too wide for the box
  goes onto up to three lines, the box moving up to show them;
* the home-site details box (the Home-site details extra): a line too wide goes onto two lines and the box grows; the
  celebrity's name and "Home" over the lot on the course;
* the golfer card: the name, the marital status and age, the traits, the quote and the likes between the face and the
  bars, and the five bars' names (Fun, Attitude, Energy, Hunger, Thirst);
* the skill names in their pills on a pro's skills card and in the skills dialog;
* the world map: the question over the globe, each card's place, bonus and acres line, the legend, and the lit card's
  offer. The offer's three texts (`STR_WORLD_BUY_ACRES`, `STR_WORLD_PROPERTY_KIND`, `STR_WORLD_FOR_ONLY`) are read as one
  sentence when one of them is too wide: they are joined with spaces and wrapped in the card, so write them to follow
  on from each other;
* the shot panel while your pro aims: the Attitude, Club, Distance and Lie lines in their box, and the skill lines in
  their two columns (and the box the aim's lines get when the Player panel is not open);
* Comments by Hole: the two lines at its foot keep left of the OK tick;
* the Course Report's legend (Top 100, Top 18, scenic hole): words too wide to sit centred beside their mark start
  after the mark and may run on to the next one;
* the round's scorecard (on the Player panel during your pro's round, and the live scorecard under the golfer card):
  the two lines on the round keep left of the "Hole" and "Par" labels; a label too wide for its place ends at the first
  hole's cell and starts as far left as the line beside it and the dark header allow (a little smaller or cut if it still
  does not fit); "Total" keeps to its column's cell;
* the SGA report: the Selection Criteria line and "Grade:" in their pills, each criterion's name and value in its pill,
  and the ideals inside the frame;
* the Routing Map: the box's title in its pill, and the two lines of help in the box (each a little smaller, else on
  two lines);
* the Player Comments report: the three headings in their pills, and each comment in its column (a little smaller, else
  cut, so the rows stay in line and never run under the hole's number);
* the Membership Roster's legend (member levels, Photo Opp, Happy Ending): each word in its pill between the marks, on
  two lines in the pill if it does not fit on one;
* the Course Terrain tooltip's title (the tile's name and cost) on its tab.

## Course Terrain descriptions

The text under a Course Terrain tooltip's title comes from your SimGolf files, in English. Each of the four themes has
its own file, and they describe the same tile in different words (the Links tees are not described like the Desert
tees), so there are 64 descriptions in all. To give your own, add the tile's name key with `_DESC` and the theme's
name to your file:

```
STR_TILE_FIRM_FAIRWAY_DESC_PARKLANDS    :Watered less often and cut shorter than the fairway...
STR_TILE_FIRM_FAIRWAY_DESC_DESERT       :More freeway than fairway...
STR_TILE_FIRM_FAIRWAY_DESC_TROPICAL     :Like a fairway, with more zip...
STR_TILE_FIRM_FAIRWAY_DESC_LINKS        :Short, tight grass on compacted soil...
```

The theme names are `PARKLANDS`, `DESERT`, `TROPICAL` and `LINKS`. A key with `_DESC` alone (`STR_TILE_FIRM_FAIRWAY_DESC`,
as 0.7.4 read it) is still read, for every theme that has no key of its own, so a file written for 0.7.4 keeps working.
A tile with neither keeps the English text.

These are the 64 keys, by theme (Mound has no description in any theme):

| Theme | Keys (add `_DESC_<THEME>`) |
|---|---|
| Parklands | STR_TILE_TEES, STR_TILE_GREEN, STR_TILE_SAND_TRAP, STR_TILE_ROUGH, STR_TILE_POT_BUNKER, STR_TILE_STREAM, STR_TILE_WATER, STR_TILE_TREE, STR_TILE_PINE_TREE, STR_TILE_PALM_TREE, STR_TILE_FAIRWAY, STR_TILE_FIRM_FAIRWAY, STR_TILE_DEEP_ROUGH, STR_TILE_WASTE_BUNKER, STR_TILE_BRUSH, STR_TILE_ROCKS |
| Desert | STR_TILE_TEES, STR_TILE_GREEN, STR_TILE_SAND_TRAP, STR_TILE_DESERT, STR_TILE_POT_BUNKER, STR_TILE_RAVINE, STR_TILE_WATER, STR_TILE_JOSHUA_TREE, STR_TILE_CACTUS, STR_TILE_PALM_TREE, STR_TILE_FAIRWAY, STR_TILE_FIRM_FAIRWAY, STR_TILE_DESERT_ROUGH, STR_TILE_WASTE_BUNKER, STR_TILE_BRUSH, STR_TILE_ROCKS |
| Tropical | STR_TILE_TEES, STR_TILE_GREEN, STR_TILE_SAND_TRAP, STR_TILE_ROUGH, STR_TILE_POT_BUNKER, STR_TILE_STREAM, STR_TILE_WATER, STR_TILE_TROPICAL_BUSH, STR_TILE_TROPICAL_TREE, STR_TILE_PALM_TREE, STR_TILE_FAIRWAY, STR_TILE_FIRM_FAIRWAY, STR_TILE_DEEP_ROUGH, STR_TILE_WASTE_BUNKER, STR_TILE_BRUSH, STR_TILE_ROCKS |
| Links | STR_TILE_TEES, STR_TILE_GREEN, STR_TILE_SAND_TRAP, STR_TILE_ROUGH, STR_TILE_POT_BUNKER, STR_TILE_BURN, STR_TILE_WATER, STR_TILE_MAPLE_TREE, STR_TILE_PINE, STR_TILE_SCOTS_PINE, STR_TILE_FAIRWAY, STR_TILE_FIRM_FAIRWAY, STR_TILE_DEEP_ROUGH, STR_TILE_WASTE_BUNKER, STR_TILE_GORSE, STR_TILE_ROCKS |

So the Links tees are `STR_TILE_TEES_DESC_LINKS` and the Desert cactus `STR_TILE_CACTUS_DESC_DESERT`. The Desert's
"Desert" and "Rough" tiles are `STR_TILE_DESERT` and `STR_TILE_DESERT_ROUGH`.

## Parameters

Names in braces are filled in by the game: `{GOLFER}` a golfer's name, `{PRO}` the player's pro (or a famous golfer),
`{MONEY}` an amount (as digits: the money sign is in the text, `§{MONEY}`), `{HOLE}` a hole number, `{YEAR}`, `{COURSE}`,
`{TERRAIN}`, `{BUILDING}` and so on; the comment above each group of texts says what its parameters are. Use them in the
order your language needs and leave out none: a parameter your text does not name is simply not shown. A few texts take
positions instead of names (`{0}`, `{1}`), the same way.

Where the English depends on a number or a person, the game has a key for each form: `STR_MATCH_LEADS_BY_ONE` and
`STR_MATCH_LEADS_BY` (one shot, several shots), `STR_NARR_HE` and `STR_NARR_SHE`, `STR_MSG_HIS` and `STR_MSG_HER`. Some
English sentences are made of pieces the original game puts together (a golfer's quit message is `{GOLFER} {REASON}
{WHEN}.`, a hole's opening announcement is seven pieces); translate the pieces so that they read well together.

## Trying it out

* Put the file in a `lang` folder in the save folder (`~/.local/share/opensgolf/lang/` on Linux,
  `%APPDATA%\OpenSGolf\lang\` on Windows, `~/Library/Application Support/OpenSGolf/lang/` on macOS), next to the program,
  or in the folder you start the game from. Packs (docs/MODDING.md) can carry `lang/<code>.txt` too.
* Choose the language: once a translation is there, **System Functions > Preferences** shows a `Language:` line; each
  click switches to the next language and remembers it in `settings.json` (`"language": "de"`). You can also write the
  setting yourself, or start the game with `--lang de` for one run.
* `--lang-file path/to/de.txt` starts the game with that file over the English, wherever it is, which is handy while you
  work on it.
* `SG_TEXTLOG=texts.txt` (an environment variable) writes every text the game draws or posts to the message window to
  `texts.txt`, each different text once. Together with the screen test hooks of docs/PLAYING.md (`--screen report`,
  `--png out.png`, ...) it shows which texts you still have to do; a pseudo translation (every text in `[[ ]]`) shows any
  text that does not come from the table at all.

## For programmers

The table is `sg_core::lang`: `tr("STR_KEY")` gives a text (a `&'static str`), `trf("STR_KEY", &[("GOLFER", &name)])`
fills its parameters, `lang::plural(n, one, other)` picks a key by a count, `lang::load_overlay(text)` lays a translation
(or a pack's) over the table, `lang::reset()` goes back to English. New text goes into `lang/en.txt` under a key named
`STR_<AREA>_<WHAT>` in the group of its area, with a comment saying where it shows and, for the original's text, the exe
address the code cites. Keys are always written out in full in the code (tables of keys are fine, keys built at run time
are not), so the tests can check that every key the code uses is in `lang/en.txt`, that none is there twice and that none
is unused (`cargo test -p sg-core lang`). A sample of the formatted messages is checked word for word against the text
the game made before its strings moved into the table (`lang_golden.rs`).
