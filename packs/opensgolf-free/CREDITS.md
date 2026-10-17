# Credits of the OpenSGolf free graphics (`opensgolf-free`)

Everything in this set is our own work or under a public domain dedication. Nothing in it is taken, traced, upscaled or
recoloured from the original SimGolf: the only numbers taken from the original are the layout of its files (picture sizes,
the places the engine cuts sheets at, sprite frame sizes, views, frame counts, anchors and sound lengths, in
`crates/sg-core/src/freeset/manifest.txt`).

## Pictures

* **Sprites** (trees, bushes, cacti, palms, buildings, golfers) in `art/Flics/`: rendered by `tools/freeset_art/build.py`
  from models built in code (`trees.py`, `buildings.py`, `people.py`) with our own renderer (`raster.py`). GPL-2.0-or-later,
  like the rest of OpenSGolf. No third-party models are used.
* **Terrain textures, interface, the sprites without a rendered sheet**: drawn by the engine at run time
  (`crates/sg-core/src/freeset/`), GPL-2.0-or-later.

## Sounds and music

`art/Sounds/` and `art/SimsFX/` are made entirely in code by `tools/freeset_art/sounds.py` (with `snd_dsp.py`, `snd_fx.py`,
`snd_voice.py`, `snd_music.py` and `snd_songs.py`): synthesised effects and voices and music composed for OpenSGolf, all
GPL-2.0-or-later like the rest of OpenSGolf. No recordings or samples of anyone else are used. The voices speak a babble
invented here, and every melody and arrangement is our own.

## Text

The stories, sayings, names, descriptions and credits in `crates/sg-core/src/freeset/text.rs` are written for OpenSGolf;
every person, course and celebrity in them is made up.

## Fonts

Liberation Sans Bold, SIL Open Font License 1.1 (`crates/simgolf/fonts/LiberationSans-OFL.txt`).
