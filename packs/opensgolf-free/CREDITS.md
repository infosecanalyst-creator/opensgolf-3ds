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

## Sounds

`art/Sounds/` is made by `tools/freeset_art/sounds.py` from these recordings, each dedicated to the public domain under
**CC0 1.0** (https://creativecommons.org/publicdomain/zero/1.0/), mixed, trimmed and levelled, together with our own
synthesis (applause, crowd, water, wind, birds, animals, swishes) and generated music:

* **Kenney** (https://kenney.nl): *Impact Sounds*, *RPG Audio* and *Music Jingles*, CC0 1.0. Thank you, Kenney.
* **uisfx** by Romain Simon (https://github.com/romainsimon/uisfx), interface sounds, CC0 1.0 (as published on npm in
  `@wubbleai/community-sfx`).

`tools/freeset_art/fetch_sources.py` downloads these recordings again to rebuild the sounds.

## Fonts

Liberation Sans Bold, SIL Open Font License 1.1 (`crates/simgolf/fonts/LiberationSans-OFL.txt`).
