# OpenSGolf free graphics (`opensgolf-free`)

The free base set of OpenSGolf, like OpenGFX is for OpenTTD: art, sounds and text made for OpenSGolf, so the game starts and
plays without any file of the original SimGolf. Licence: GPL-2.0-or-later (the fonts: SIL Open Font License, see below).

## How it is made

The set has two parts, both built into the game (nothing is downloaded at run time):

* **Rendered art** in `art/` (about 9 MB): sprite sheets of the trees, bushes, cacti, palms, scenic plants, buildings and
  golfers, staff and celebrities, rendered offline by `tools/freeset_art/build.py` from 3D models made in code (a painterly tree grower, a building
  kit in each theme's style, a jointed golfer posed for every animation) with the game's own camera (pitch 38.7 degrees,
  4 or 8 views), and the sounds and music made by `tools/freeset_art/sounds.py` from CC0 recordings and our own synthesis
  (see `CREDITS.md`). `crates/sg-core/src/freeset/baked.rs` turns a sheet into the FLC the engine reads.
  Golfers are drawn in the palette slots the engine recolours, so every golfer still wears their own colours; trees use
  the leaf ramp the colour variant palettes change. The main interface sheets (the dock, title screen, course badge,
  the Terrain, Amenities, Buildings and Elevation panels with the theme's lot pictures, terrain buttons, dialogs and the
  course report) are vector art drawn by `tools/freeset_art/ui.py` at the places the engine cuts them.
* **Drawn at run time** by code in `crates/sg-core/src/freeset/` for everything else:
  * **Terrain** (`tex.rs`): each theme's surfaces painted from wrapping noise with lit relief (mowing stripes, blades and
    clumps of grass, scrub, stones, sand grain and ripples, water sheen), in the nine blend shapes the terrain renderer uses.
  * **Pictures** (`paint.rs`, `ui.rs`): panels, buttons, screens, the title logo, dock symbols, terrain buttons, hole signs
    and faces, drawn with signed-distance shapes.
  * **Sprites without a rendered sheet** (`sprite.rs`): animals, flowers, benches, flags, bridges, boats, monuments and the
    rest, from a few shapes each, in one palette that keeps the engine's colour slots.
  * **Sounds without a recording** (`sound.rs`): synthesised.
  * **Text** (`text.rs`): our own stories, characters, famous golfers, celebrities, terrain descriptions, lighting, credits and
    a starting high score table; every name is made up.
  * **Fonts**: Liberation Sans Bold (SIL Open Font License 1.1, `crates/simgolf/fonts/LiberationSans-OFL.txt`) under the two
    names the game loads, `KLEPTO__.TTF` and `manu3_.TTF`.

To rebuild the rendered art: `python3 tools/freeset_art/build.py` (numpy and Pillow; `--only REGEX` for some sprites), `python3 tools/freeset_art/ui.py` for the interface sheets, and
for the sounds `python3 tools/freeset_art/fetch_sources.py DIR` then `python3 tools/freeset_art/sounds.py DIR` (ffmpeg
with libvorbis). Both rewrite `crates/sg-core/src/freeset/baked_list.rs`, the list of files built into the game.

Every file has the name, size and layout the engine expects of the original's file: the same picture sizes, the same places
for the pieces the engine cuts out of a sheet, the same sprite views, frame counts and anchors, so everything lands where
the engine puts the original's art. Those numbers are the only thing taken from the original. They are in
`crates/sg-core/src/freeset/manifest.txt`, measured from an installed copy by `tools/freeset_layout.py` (sizes, the boxes
of the opaque parts of each sheet with a coarse shape and tone class, sprite timings and sound lengths). No pixel, palette
entry, sample or sentence of the original is in the set.

## Using it

* With no SimGolf files in the game folder (or no `--game` folder at all) the game uses this set by itself and says so.
* `simgolf --base opensgolf-free` uses it for one run even when your copy is there; `--base original` the other way round.
* In the game: System Functions > Preferences > Content packs, pick *OpenSGolf free graphics* as the base set (takes effect
  at the next start).
* In the browser version: *Play with the free OpenSGolf graphics* on the start page.
* `sgtool freeset DIR` writes the whole set to a folder (this layout: `gfx/`, `sfx/`, `text/`, `fonts/`, `pack.toml`).

## Replacing pieces (for artists)

This folder is the place for hand-made replacements. Put a file here under the path the game uses for it, and it is loaded
over the drawn one (when this folder is in a packs folder: `packs/` in the save folder or next to the program, docs/MODDING.md):

```
opensgolf-free/
  pack.toml
  gfx/Interface/TitleBASE.png        the title screen background (800 x 600)
  gfx/Flics/Trees/TreeMapleLarge.flc a tree (an FLC with the same views, frames and anchor)
  sfx/Sounds/Applause.ogg            a sound (OGG or WAV)
  text/Themes/Standard/MyStory.txt   another story
```

Start from the drawn file: `sgtool freeset /tmp/free` writes it, keep its size (and for a sheet, keep each piece inside the
box the drawn one uses: the engine cuts the sheet at fixed places). See CONTRIBUTING.md for the list of files and their
sizes, and docs/MODDING.md section 9 for the formats (PNG for PCX/TGA/BMP pictures, FLC for sprites, OGG or WAV for sounds).
Contributions must be your own work (or under a GPL-compatible free licence) and must not be traced, recoloured or derived
from the original game's art, sounds or text.
