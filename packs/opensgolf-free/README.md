# OpenSGolf free graphics (`opensgolf-free`)

The free base set of OpenSGolf, like OpenGFX is for OpenTTD: art, sounds and text made for OpenSGolf, so the game starts and
plays without any file of the original SimGolf. Licence: GPL-2.0-or-later (the fonts: SIL Open Font License, see below).

## How it is made

The set has two parts, both built into the game (nothing is downloaded at run time):

* **Rendered art** in `art/` (about 11 MB): sprite sheets of the trees, bushes, cacti, palms, scenic plants, buildings,
  landmarks, animals and golfers, staff and celebrities, rendered offline by `tools/freeset_art/build.py` from 3D models
  made in code (a painterly tree grower whose crowns are lit as one rounded mass, with warm highlights and cool shade; a
  building kit in each theme's style with lit windows; a jointed golfer whose hands follow the club's grip through the
  swing; `landmarks.py` for the windmills, lighthouses, statues, pagoda, barn, volcano and the rest; `animals.py` for the
  crane, crocodile, duck, elk, flamingo, gila monster, roadrunner, sheep and snake; `props.py` for the hole flags and
  tee markers, the range and putting green flags, benches and the swing, flower beds (their blooms in the colour of
  the bed's palette), course signs, the ball washer, sailboats, the windsurfer and the bridges). A few sight gags hide in the world: a
  sun hat on a cactus, a squirrel on the big maple, a lost ball under a bush, a gopher by the rocks, ducklings following
  their mother, and by the clubhouse deckchairs, a parked golf cart, a picnic and a garden gnome with the game's own camera (pitch 38.7 degrees,
  4 or 8 views), and the sounds and music made by `tools/freeset_art/sounds.py` entirely in code (see
  **Sounds** below). `crates/sg-core/src/freeset/baked.rs` turns a sheet into the FLC the engine reads.
  Golfers are drawn in the palette slots the engine recolours, so every golfer still wears their own colours; trees use
  the leaf ramp the colour variant palettes change. The main interface sheets (the dock, title screen, course badge,
  the Terrain, Amenities, Buildings and Elevation panels with the theme's lot pictures, terrain buttons, dialogs, the popup
  frame, the golfer card and the course report) and the full screen pages (Select Difficulty, the world map with its theme
  balls, Load Game, Pick A Pro, Theme Packs, the credits, Top 10 Designers, the trophy room, the routing map's lower part and
  the tract buttons) are vector art drawn by `tools/freeset_art/ui.py` and `uipages.py` at the places the engine cuts them. The title menu
  (`uititle.py`) is a sunny course with the OpenSGolf logotype and big candy plates (Start New Game as the hero, its golf ball
  winking; a gopher peeks over Sandbox Mode). The large screens are kept in full colour.
  * **The engine's own boxes** (popup menus, Preferences, the Extras and Enhanced graphics boxes, People, Keyboard controls,
    Content packs, the save and rename prompts, the title's small buttons): while this set is loaded they are drawn in its look
    by `crates/simgolf/src/glass_ui.rs` (candy panels with a header band, capsule buttons and tabs, switches, a highlighted
    row under the pointer), with the same places and hit areas; with the original's files they look exactly as before.
* **Drawn at run time** by code in `crates/sg-core/src/freeset/` for everything else:
  * **Terrain** (`tex.rs`): each theme's surfaces painted from wrapping noise with lit relief (mowing stripes, blades and
    tussocks of grass, a mossy forest floor with fallen leaves, heather, scrub, stones, sand grain and ripples, flower
    clumps), in the nine blend shapes the terrain renderer uses. Water is one seamless open-water pattern in every depth
    and set (the terrain renderer draws the free set's water as one surface, `Terrain::water_one`, so every water tile
    facing land takes the shore pieces whatever its depth); its shallow pieces that face land carry the shore (lighter shallows, a broken line of foam, wet sand, a beach
    or an earthy bank by theme). `cargo test -p sg-core tex_sheet -- --ignored` with `TEX_SHEET=out.png` writes a
    contact sheet of every surface.
  * **Pictures** (`paint.rs`, `ui.rs`): panels, buttons, screens, the title logo, dock symbols, terrain buttons, hole signs
    and faces, drawn with signed-distance shapes.
  * **Sprites without a rendered sheet** (`sprite.rs`): the TV towers, water effects, homes, the gazebo and the
    rest, from a few shapes each, in one palette that keeps the engine's colour slots; and the palette files, which
    colour the rendered trees, golfers, landmarks and props through those slots.
  * **Sounds without a recording** (`sound.rs`): synthesised (a fallback; every sound of the game has a recording).
  * **Text** (`text.rs`): our own stories (thirteen, from Opening Day to The Podcast), characters with twenty sayings each,
    famous golfers, celebrities, a description of every terrain per theme, lighting, credits and a starting high score
    table; every name is made up.
  * **Fonts**: Fredoka SemiBold for the headings (`KLEPTO__.TTF`) and Nunito ExtraBold for the text (`manu3_.TTF`), both
    SIL Open Font License 1.1 (`crates/simgolf/fonts/*-OFL.txt`, made by `tools/freeset_art/fonts.py`, see `CREDITS.md`).

To rebuild the rendered art: `python3 tools/freeset_art/build.py` (numpy and Pillow; `--only REGEX` for some sprites), `python3 tools/freeset_art/ui.py` for the interface sheets, and
for the sounds `python3 tools/freeset_art/sounds.py` (numpy, scipy, ffmpeg with libvorbis; `--only REGEX` for some). Both rewrite `crates/sg-core/src/freeset/baked_list.rs`, the list of files built into the game.

Every file has the name, size and layout the engine expects of the original's file: the same picture sizes, the same places
for the pieces the engine cuts out of a sheet, the same sprite views, frame counts and anchors, so everything lands where
the engine puts the original's art. Those numbers are the only thing taken from the original. They are in
`crates/sg-core/src/freeset/manifest.txt`, measured from an installed copy by `tools/freeset_layout.py` (sizes, the boxes
of the opaque parts of each sheet with a coarse shape and tone class, sprite timings and sound lengths). No pixel, palette
entry, sample or sentence of the original is in the set.

## Sounds

Every sound and piece of music is made by `tools/freeset_art/sounds.py` and its kit, nothing recorded:

* `snd_fx.py`: golf (swing whooshes, driver / iron / chip / putt contact, the ball on grass, sand, water and trees, the cup's
  rattle), the interface (soft pops and ticks, little mallet confirmations; the fairway painting tune's note is an F so
  the engine's pitched tune lands in key), building and terrain tools, applause and crowds, wind, birds, insects, waves,
  lake water, animals, engines, and cartoon comedy: a springy boing, a 'bloop' for a ball in the water, a slide whistle
  for a bad shot, a kazoo 'wah-wah-waaah' for a disappointed crowd, a brass fanfare for the big moments.
* `snd_voice.py`: a formant voice (glottal pulse, five resonators, consonant noise) speaking *Golfish*, a made-up babble,
  in cartoon-sized moods (happy, excited, sad, angry, tired, awed, disgusted, sly) with laughs, giggles, grumbles,
  'ooh!', 'aww!' and 'wheee!', sighs, yawns, gasps, hums and grunts, breath, vibrato and a natural drift of pitch; a cast of
  men's and women's voices and one for each celebrity.
* `snd_music.py`, `snd_songs.py`: instruments (e-piano, nylon guitar, harp, harpsichord, marimba, vibes, glockenspiel,
  kalimba, steel pan, flutes, whistle, pads, strings, basses, drums and hand percussion) and original pieces: a main
  theme (title, accomplishments, year end, story jingles), a cheeky honky-tonk rag (story chapters), a calm piece, a
  clubhouse bossa, and a theme each for the tropical,
  desert and links courses, arranged for the story jingles and the in-game playlists (`Sounds/Music`, the M key) and
  medleyed for the credits. Playlist pieces loop without a seam.

Levels: music -18 LUFS integrated, the course ambience -29 LUFS, effects and voices by their loudest 100 ms (interface -19,
golf contact -14, voices -17), true peaks under -1 dBFS before encoding. A piece the game's folders hold twice is
recorded once (`ALIASES` in sounds.py and sound.rs).

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
