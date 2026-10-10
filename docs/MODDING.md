# Modding and content packs

OpenSGolf is built to be modded the way OpenTTD is: the engine holds the rules, and everything a player sees or reads comes
from **packs** that are loaded in order. This document is the design and the record of what is done. Status markers:
**done**, **in progress**, **planned**.

## 1. Packs

A pack is a folder (or a `.zip` of that folder) with a `pack.toml` at its root:

```toml
[pack]
id = "my-desert-courses"          # unique, lower case, letters, digits and dashes
name = "Desert Courses"
version = "1.2.0"                 # semantic version
kind = "mod"                      # base | mod | translation | scenario
authors = ["Your Name"]
license = "CC-BY-SA-4.0"          # SPDX identifier
description = "Adds two desert properties and a cactus garden."
requires = ["original >= 1.0"]   # other packs this one needs (`original`: the built-in base set)
conflicts = []
```

Kinds:

| Kind | What it is | How many are loaded |
|------|------------|---------------------|
| `base` | A complete set of art, sounds, music, text and data the game can run on. | exactly one |
| `mod` | Adds or overrides data, art, sounds or text. | any number, in the player's order |
| `translation` | Text in one language (`lang/<code>.txt`). | the player's language |
| `scenario` | A course, a starting property, goals and settings. | chosen when a game starts |

Two base sets are planned:

* **`original`**: built into the engine. It reads the art, sounds, music, videos and text of the player's own installed copy
  of SimGolf at runtime and never copies them anywhere. This is the 1:1 game.
* **`opensgolf-free`**: a free base set (GPL-2.0-or-later) in this repository, with art, sounds and text made for OpenSGolf,
  so the game runs without the original files. It starts as simple placeholder art and is meant to grow through
  contributions, like OpenGFX did for OpenTTD.

Where packs live (**done**): `packs/` in the save folder (`~/.local/share/simgolf/packs` on Linux,
`~/Library/Application Support/SimGolf/packs` on macOS, `%APPDATA%\SimGolf\packs` on Windows, or `$SIMGOLF_SAVE_DIR/packs`),
plus `packs/` next to the program. Each pack is a folder, or a `.zip` with `pack.toml` at its root or in its one top
folder. The browser version keeps the packs the player adds on the start page (*Optional: content packs*) in the browser's
storage. A pack whose id was already found earlier (the save folder is searched first) is listed as a duplicate and not
loaded.

Status: `original` is built in (crates/sg-core/src/packs.rs, `ORIGINAL`). Another `base` pack can be chosen in Preferences >
Content; for now it is one more layer over the player's game folder (the engine still needs the game folder until the free
base set exists). `translation` packs are loaded last, one at a time, but their `lang/` text is not read yet (section 4).
`scenario` packs are listed and not loaded yet (section 5).

## 2. Load order and overrides

The base set loads first, then mods in the order the player chooses in **Preferences > Content**, then the translation.
Later packs win. Every pack can:

* **Override files** (**done**): any file under `gfx/`, `sfx/`, `music/` or `video/` replaces the game folder's file with
  the same relative path (case insensitive), e.g. `gfx/Interface/BuildPanel.png` replaces `Interface/BuildPanel.pcx` and
  `sfx/Sounds/Applause.ogg` replaces `Sounds/Applause.wav`. PNG, PCX, TGA, BMP and FLC are accepted for art; WAV and OGG for
  sound.
* **Change data** (**done** for six files, section 3): `data/*.toml` files are merged entry by entry (an entry with the same
  `id` replaces fields of the earlier one; `remove = true` deletes it).
* **Change text** (planned, section 4): `lang/<code>.txt` lines replace strings with the same key.

The player's choices (which packs are on, the mods' order, the base set and the translation) are saved under `"content"` in
`settings.json` in the save folder as soon as they are made on the Content screen, and **take effect the next time the game
starts**: the packs are mounted once, before any art, sound or data is read, so a game in progress never changes under the
player. A mod that was never switched on is off: new packs start switched off and go to the end of the order.

Problems never stop the game. A pack with a bad `pack.toml`, a missing or switched off requirement, a requirement loaded
after it, a version that does not match, or a conflict with a loaded pack is not loaded, and the Content screen shows why
(packs that need it are then not loaded either). A data entry with a bad value is left out with a warning on the pack's
row; the rest of the pack loads.

The HD graphics mode (docs/HD.md) is the first override layer: an enabled pack's file wins over the HD upscale of the file it
replaces (which is then loaded the classic way); everything else of HD is unchanged. It stays a local thing that is never
shared, because it is made from the original art.

How it works: every read of a game file goes through `sg_core::fsutil::read_file`, which asks `sg_core::vfs` first. With no
packs enabled nothing is mounted and the reads are exactly what they were.

## 3. Data files

Each file is a list of tables with an `id`. The engine's built-in values are the `original` base set's data;
`sgtool dump-data DIR` writes them out as `DIR/data/*.toml`, with the meaning of every field at the top of each file (the
text comes from the code, crates/sg-core/src/data.rs). The reference is in section 9.

| File | Contents | Status |
|------|----------|--------|
| `data/terrain.toml` | tile types: name, paint and clearing cost, home-site view, bounce, roll, hazard (lie), walking cost, blend class, scenery group, shape flags | **done** (art set and panel slot planned) |
| `data/buildings.toml` | buildings: name, footprint, price | **done** (unlock hole, upgrade levels, effects planned) |
| `data/amenities.toml` | paths, benches, flower beds, ball washers, landmarks, willows, TV towers and booths, bridges: name, footprint, price | **done** (effects, strip art planned) |
| `data/staff.toml` | employee kinds: name, wage, experienced wage | **done** (task, speed, art planned) |
| `data/properties.toml` | the sixteen properties: place, course name, bonus, map position, theme, coast, relief | **done** (emblem planned) |
| `data/economy.toml` | starting money (normal and sandbox), offer slot prices, firing cost, hole maintenance, debt interest, rank thresholds | **done** (fee formula planned) |
| `data/golfers.toml` | name lists, traits, skill ranges, celebrity and VIP visitors | planned |
| `data/tournaments.toml` | SGA events, purses, entry rules, rival pros | planned |

## 4. Text and translations

All text the engine shows has a key. `lang/en.txt` is plain UTF-8, one string per line, in the OpenTTD style:

```
STR_TOOLTIP_COST           :Cost: {MONEY}
STR_MSG_FIRST_GREENS_FEE   :{GOLFER} has just paid you your first greens fee of {MONEY}!
```

Parameters in braces are filled by the engine. A missing key falls back to English, then to the key itself.

The `original` base set takes text from the player's copy where the copy has it (the Interface, story and character files).
Text that only exists inside the original program is kept in this repository's `lang/en.txt`, word for word, so the game
reads like the original; that text remains Electronic Arts'. The free base set will carry its own rewritten text so it can
be shared without any of EA's material. Translations are made in the open (planned: Weblate).

## 5. Scenarios and courses

A scenario pack holds `scenario.toml` (property, theme, money, difficulty, goals with the text to show, starting holes) and
optionally a saved course. Courses saved from the game can be exported as scenario packs from the save menu.

## 6. Scripts (planned)

Game scripts, like OpenTTD's GameScripts, in **Rhai** (pure Rust, sandboxed, works in the browser): event hooks (month
start, golfer finishes a hole, building placed), access to read the game state and to show messages, set goals and award
money. Scripts cannot touch files or the network.

## 7. Content service (planned)

An index file (`index.json`) in a public repository lists packs with id, version, kind, licence, download URL and checksum.
**Preferences > Content > Find more** downloads packs into the packs folder and checks the checksum. Anyone can submit a
pack with a pull request to the index; packs must have a free licence and must not contain material from the original game.

## 8. Progress

| Step | Status |
|------|--------|
| Licence (GPL-2.0-or-later) and name | done |
| Text keys and `lang/en.txt` (original English text kept) | planned |
| Pack loader, load order, file overrides (folders and .zip, desktop and browser) | done |
| PNG for PCX/TGA/BMP art, OGG for WAV sound | done |
| Data files for terrain, buildings, amenities, staff, properties, economy | done |
| `sgtool dump-data` (the built-in tables as a template) | done |
| Example mod (`packs/example-cheap-fairways`) | done |
| Preferences > Content screen | done |
| Data files for golfers and tournaments; the planned fields of section 3 | planned |
| Overrides for the intro videos (sgplay does not read packs yet) | planned |
| Free base set (placeholder art) | planned |
| Scenario packs | planned |
| Rhai scripts | planned |
| Content index and in-game downloader | planned |

## 9. Reference for pack makers

The repository's `packs/example-cheap-fairways/` is a complete small mod to start from. To try a pack, put its folder (or a
.zip of it) in one of the packs folders of section 1, start the game, switch it on in System Functions > Preferences >
Content packs, and start the game again. The terminal shows which packs loaded and every problem found
(`content pack ...: ...`).

### pack.toml

One `[pack]` table; unknown keys are an error.

| Field | Required | Meaning |
|-------|----------|---------|
| `id` | yes | Unique id: lower case letters, digits and dashes (at most 64). `original` is taken by the built-in base set. |
| `name` | yes | The name the Content screen shows. |
| `version` | yes | Numbers separated by dots, e.g. `1.2.0` (`1.2` is the same as `1.2.0`; a `-beta` or `+build` suffix is ignored when versions are compared). |
| `kind` | yes | `base`, `mod`, `translation` or `scenario` (section 1). |
| `authors` | no | List of names. |
| `license` | no | An SPDX identifier, e.g. `CC-BY-SA-4.0`, `CC0-1.0`, `GPL-2.0-or-later`. |
| `description` | no | One or two sentences; shown when the pack is picked on the Content screen. |
| `requires` | no | Packs that must be loaded before this one, each `"id"` or `"id OP VERSION[, OP VERSION]"` with OP one of `>=`, `>`, `<=`, `<`, `=`, `!=`, `^` (same first number, at least this version) or `~` (same first two numbers, at least this version). `original >= 1.0` is the built-in base set. |
| `conflicts` | no | Packs this one cannot be loaded with, in the same form. The check goes both ways. |

### Files of a pack

```
my-pack/
  pack.toml
  data/terrain.toml             merged over the tables (section 3; the six files of the table above)
  gfx/Interface/BuildPanel.png  replaces Interface/BuildPanel.pcx of the game folder
  gfx/Data/Textures/...         terrain textures (BMP or TGA in the game; PNG accepted)
  gfx/Flics/...                 sprites and animations (FLC only)
  sfx/Sounds/Applause.ogg       replaces Sounds/Applause.wav
  music/Sounds/...              music lives under Sounds/ in the game folder too
  video/Flics/...               the intro videos (planned: not read yet)
  README.md                     anything else is ignored
```

The path under `gfx/`, `sfx/`, `music/` and `video/` is the path in the game folder, matched without regard to case. The
four folders take these types: `gfx/` PNG, PCX, TGA, BMP, FLC; `sfx/` and `music/` WAV, OGG; `video/` BIK, FLC. A file of
another type is ignored. A PNG replaces the PCX, TGA or BMP of the same name and an OGG the WAV of the same name; a file
with the original's own name and type wins over a replacement of another type in the same pack.

What to know about replacement art and sound:

* PNG: 8 bits per channel, not interlaced (greyscale, RGB, palette, with or without alpha). Keep the original's size:
  the game cuts many pictures into parts at fixed places. Where the original uses magenta (255, 0, 255) as transparent,
  so does the replacement; PNG alpha works too.
* Pictures the game reads as palette art (the customise-golfer sheets) take their palette from the PNG's first 256
  colours in the order they appear; more colours are mapped to the nearest of those. Palette files (`*.pcx` the sprites
  are recoloured with) must stay PCX.
* Sprites and animations (`.flc`) are replaced by FLC files only.
* OGG Vorbis, mono or stereo, any sample rate; WAV 8 or 16 bit PCM. The game converts both to 44.1 kHz stereo.

### Data files

Every file is TOML. Entries are tables in an array named after the file (`[[terrain]]`, `[[building]]`, `[[amenity]]`,
`[[staff]]`, `[[property]]`); `economy.toml` is one `[economy]` table. Each entry names its `id`; the fields given replace
the ones of the entry with that id (from the base set or an earlier pack), the others stay. `remove = true` deletes the
entry. A new id makes a new entry.

Rules the engine applies:

* **Money is the game's money as shown**: a fairway costs `300`. The engine counts in hundreds, so every amount must be a
  multiple of 100.
* The slot fields (`tile`, `kind`, `index`) of the built-in entries are fixed: a pack cannot move an entry to another slot.
* The engine has one slot for each built-in id. Removing a built-in entry keeps its built-in values in the game (with a
  warning); a new id is kept but not used yet (with a warning). Both are for packs to come (new terrain, new buildings).
* A wrong value (out of range, wrong type, unknown field) leaves that entry as it was and puts a warning on the pack's row.

#### terrain.toml

| Field | Unit | Meaning |
|-------|------|---------|
| `id`, `tile` | | Built-in ids by tile: 0 `tee`, 1 `green`, 2 `fairway`, 3 `firm-fairway`, 4 `rough`, 5 `deep-rough`, 6 `mounds`, 7 `sand-trap`, 8 `waste-bunker`, 9 `pot-bunker`, 10 `ravine`, 11 `brush`, 12 `rocks`, 13 `tree`, 14 `pine-tree`, 15 `palm-tree`, 16 `elm-tree`, 17 `water`, 18 `wetlands`, 19 `marsh`, 20 `out-of-bounds`, 21 `home-site`, 22 `building`; paint-only ids with a price and nothing else: 23 `water-middle`, 24 `water-deep`, 25 `water-shallow-desert`, 26 `tricky-green`, 27 `sand-bunker`. |
| `name` | text | The exe's name of the type (tract descriptions say "acres of rough, ..."). |
| `cost` | money | Painting one tile (the Build Course panel's figure). |
| `clear_cost` | money | Clearing one tile of it before something is painted or built over it (at most 25500 for tiles 0..22). |
| `view` | points | What one such tile next to a home site adds to the lot's view (with the lie as a factor). |
| `bounce`, `roll` | 0..15 | How a ball bounces and rolls on it (the tooltip's meters). |
| `hazard` | -1..8 | The lie: how hard a shot from it is; -1 is the green (the tooltip's face). |
| `walk` | | Walking cost for golfers and staff (also the tile's draw layer). |
| `class` | | Blend class: tiles of one class blend at their borders (0 tee, 1 green, 2 fairway, 4 rough, 7 sand, 8 waste, 13 trees, 17 water, 18 out of bounds). |
| `group` | | Scenery group: which trees, rocks or grasses are drawn on it. |
| `flags` | bits | Shape: 1 level table, 2 flat at the lowest corner, 4 flat at the highest, 8 height 0; 0x100.. scenery bits. |

```toml
[[terrain]]
id = "fairway"
cost = 100          # was 300

[[terrain]]
id = "rough"
roll = 1            # balls stop sooner in the rough
```

#### buildings.toml and amenities.toml

| Field | Unit | Meaning |
|-------|------|---------|
| `id`, `kind` | | buildings.toml: 5 `home-site`, 6 `putting-green`, 7 `snack-bar`, 8 `pro-shop`, 9 `swim-club`, 10 `driving-range`, 11 `cart-garage`, 12 `marina`, 13 `resort-hotel`, 14 `airstrip`, 15 `clubhouse`. amenities.toml: 0 `pathway`, 1 `benches`, 2 `flower-bed`, 3 `ball-washer`, 4 `landmark`, 16 `willow-tree`, 17 `tv-tower`, 18 `tv-booth`, 19 `scenic-bridge`. |
| `name` | text | The name the panels and messages use. |
| `footprint` | tiles | Edge of the square footprint, 1..8. |
| `price` | money | Price (a building's upgrade costs price x (level + 2) / 2; a home site's price is what preparing the lot costs). |

```toml
[[building]]
id = "snack-bar"
price = 10000
```

#### staff.toml

| Field | Unit | Meaning |
|-------|------|---------|
| `id`, `kind` | | 0 `club-pro`, 1 `ranger`, 2 `groundskeeper`, 3 `soda-vendor`. |
| `name` | text | The kind's name (the hire buttons of the people list). |
| `wage`, `wage_experienced` | money | Wage per pay interval (every 1024 / (difficulty + 2) ticks, with a chance that grows with the course's rank); the hire panel shows it "per week". |

#### properties.toml

| Field | Unit | Meaning |
|-------|------|---------|
| `id`, `index` | | 0 `monterey`, 1 `san-diego`, 2 `rocky-mtns`, 3 `las-vegas`, 4 `phoenix`, 5 `hawaii`, 6 `oahu`, 7 `nova-scotia`, 8 `northeast`, 9 `carolina`, 10 `ireland`, 11 `scotland`, 12 `wales`, 13 `spain`, 14 `florida`, 15 `jamaica` (0..9 on the world map's left arc, 10..15 on the right). |
| `name`, `course`, `bonus` | text | The place, the course's own name and the bonus, as the world map and the messages show them. |
| `map_x`, `map_y` | pixels | The marker on the world map art. |
| `theme` | | `parkland`, `desert`, `tropical` or `links`: the art, sounds and land of the property. |
| `coast` | | `inland` (the owned land is a square), `coast` (the sea runs along one side) or `island` (water all round); also changes the acreage of the offer. |
| `relief` | | `gentle`, `rolling` or `hilly`: how the land is generated. |

The offer of the world map deals the properties to slots by theme (the four dearest slots get four different themes, the
cheapest a Parkland one), so a pack that changes themes changes the deal too.

#### economy.toml

| Field | Unit | Built in | Meaning |
|-------|------|----------|---------|
| `start_funds` | money | 100000 | What a normal game starts with, before the property's price is taken off. |
| `sandbox_funds` | money | 1000000 | What a sandbox game starts with. |
| `offer_prices` | money x 16 | 50000 .. 1000000 | The price of each offer slot of the world map, cheapest first. |
| `firing_cost` | money | 2500 | Firing an employee. |
| `hole_maintenance` | money | 100 | Maintenance of each active hole, charged with the wages. |
| `debt_interest_percent` | percent | 2 | Interest on debt at each month start (whole hundreds, rounded toward zero). |
| `rank_holes` | holes x 3 | [6, 10, 18] | Holes for Daily Fee, Country Club and Championship; fewer than the first is Municipal. |

```toml
[economy]
start_funds = 200000
debt_interest_percent = 5
```

### For engine developers

The loaders and their tests are in crates/sg-core/src: `packs.rs` (pack.toml, discovery, load order, loading), `vfs.rs`
(the override layers), `zip.rs`, `data.rs` (the schemas, merging, the tables the engine reads through `data::get()` and its
accessors), with the game side in crates/simgolf/src/content.rs (start up, settings, the Content screen). A table the engine
reads moves into `data.rs` by giving it an entry type, a built-in conversion in `Tables::builtin`, a slot in `Data`, and a
test that the merged defaults equal the built-in table (`merged_defaults_equal_the_builtin_tables`).
