# OpenSGolf free graphics (`opensgolf-free`)

The free base set of OpenSGolf, like OpenGFX is for OpenTTD: art, sounds and text made for OpenSGolf, so the game starts and
plays without any file of the original SimGolf. Licence: GPL-2.0-or-later (the fonts: SIL Open Font License, see below).

## How it is made

The set is **drawn by the engine itself**. Its 5,465 files (about 82 MB if written out) are made by code in
`crates/sg-core/src/freeset/` when the game reads them, so nothing has to be downloaded or committed:

* **Pictures** (`paint.rs`, `ui.rs`, `tex.rs`): panels, buttons, screens and terrain textures in a simple flat-shaded style,
  drawn with signed-distance shapes; terrain textures per theme in the nine blend shapes the terrain renderer uses; the
  title logo, dock symbols, terrain buttons, hole signs and faces get their own drawings.
* **Sprites** (`sprite.rs`): people from a jointed figure posed for each animation (walk, address, swing, putt, cheer, sit,
  drive a cart...) and turned for each of the 8 views; buildings as boxes with roofs, doors and windows turned for the 4
  views; trees grown over their 16 frames; animals, flowers, benches, flags, bridges, boats and monuments from a few shapes
  each. All in one palette that keeps the engine's colour slots (the golfer clothes, skin and hair ramps the swap palettes
  recolour, the transparent key, the shadow ramp).
* **Sounds** (`sound.rs`): synthesised from tones, noise and envelopes (a click and whoosh for a drive, a plop for water,
  a babble of vowel sounds for a golfer's voice, little tunes for jingles). There is no music yet.
* **Text** (`text.rs`): our own stories, characters, famous golfers, celebrities, terrain descriptions, lighting, credits and
  a starting high score table; every name is made up.
* **Fonts**: Liberation Sans Bold (SIL Open Font License 1.1, `crates/simgolf/fonts/LiberationSans-OFL.txt`) under the two
  names the game loads, `KLEPTO__.TTF` and `manu3_.TTF`.

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
