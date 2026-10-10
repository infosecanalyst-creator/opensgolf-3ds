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
requires = ["opensgolf-base >= 1.0"]   # other packs this one needs
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

Where packs live: `<data dir>/packs/` (the save folder's sibling on each platform), plus `packs/` next to the program. The
browser version keeps packs the player adds in the browser's storage.

## 2. Load order and overrides

The base set loads first, then mods in the order the player chooses in **Preferences > Content**, then the translation.
Later packs win. Every pack can:

* **Override files**: any file under `gfx/`, `sfx/`, `music/` or `video/` replaces the base set's file with the same
  relative path (case insensitive), e.g. `gfx/Interface/BuildPanel.png` replaces `Interface/BuildPanel.pcx`. PNG, PCX, TGA,
  BMP and FLC are accepted for art; WAV and OGG for sound.
* **Change data**: `data/*.toml` files are merged entry by entry (an entry with the same `id` replaces fields of the earlier
  one; `remove = true` deletes it).
* **Change text**: `lang/<code>.txt` lines replace strings with the same key.

The HD graphics mode (docs/HD.md) is the first override layer and becomes an ordinary local pack of kind `mod` that is never
shared, because it is made from the original art.

## 3. Data files

Each file is a list of tables with an `id`. The engine's built-in values become the `original` base set's data; the
documentation of each field (meaning, unit, where the value comes from) is generated from the code.

| File | Contents |
|------|----------|
| `data/terrain.toml` | tile types: name key, cost, bounce, roll, hazard (lie), rough class, art set, panel slot |
| `data/buildings.toml` | buildings: name key, footprint, price, unlock hole, upgrade levels, effects (fees, happiness, members) |
| `data/amenities.toml` | benches, flower beds, landmarks, bridges, paths: price, effects, strip art |
| `data/staff.toml` | employee kinds: wage, task, speed, art |
| `data/properties.toml` | the sixteen properties: place, course name, theme, relief, coast, bonus, emblem |
| `data/economy.toml` | starting money, offer slot prices, fee formula constants, interest, rank thresholds |
| `data/golfers.toml` | name lists, traits, skill ranges, celebrity and VIP visitors |
| `data/tournaments.toml` | SGA events, purses, entry rules, rival pros |

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
| Pack loader, load order, file overrides | planned |
| Data files for terrain, buildings, amenities, staff, properties, economy | planned |
| Free base set (placeholder art) | planned |
| Preferences > Content screen | planned |
| Scenario packs | planned |
| Rhai scripts | planned |
| Content index and in-game downloader | planned |
