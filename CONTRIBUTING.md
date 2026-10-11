# Contributing to OpenSGolf

Thank you for helping. OpenSGolf is a free golf course management game that plays like Sid Meier's SimGolf and is built to
be modded. Code, art, sound, text, translations, courses, testing and bug reports are all welcome.

## Ground rules

* **No art, sound, music, video, fonts or code from the original game in the repository**, not even small pieces, and
  nothing traced or upscaled from it. The engine reads those files from each player's own copy at runtime. The one exception
  is the original's English message text in `lang/en.txt`, kept so the game reads word for word like the original. The free
  base set and all packs here must be made by their authors and contain nothing from the original.
* **No decompiled code.** Rules taken from the original are written down as facts (what happens, the numbers) in `docs/`,
  and the engine's code is written from those notes.
* By contributing you agree that your work is published under the project's licence: GPL-2.0-or-later for code and the free
  base set, or the licence named in a pack's `pack.toml`.
* Be kind: see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Code

1. Install Rust (https://rustup.rs). On Linux also `sudo apt install libasound2-dev`.
2. `cargo build --release`, then `target/release/opensgolf --game "/path/to/your/SimGolf/Program_Files_(ENGLISH)"`.
3. Before a pull request: `cargo fmt`, `cargo clippy --release --workspace`, `cargo test --release --workspace`, and for
   browser changes `web/build.sh`.
4. Match the style of the code around your change. Comments say where a rule comes from (an exe address, the manual, footage
   of the original) or that it is the project's own choice.

Layout: `crates/sg-core` is the game rules and file formats (no window, unit tested); `crates/simgolf` is the game
(miniquad); `crates/sgtool` inspects and converts assets; `web/` is the browser build; `docs/` holds the notes on the
original's formats and rules.

## Art, sound and text for the free base set

The free base set (`opensgolf-free`, [packs/opensgolf-free](packs/opensgolf-free/README.md)) is drawn by the engine itself
from code in `crates/sg-core/src/freeset/`, as simple placeholder art. It is meant to be replaced, piece by piece, by
hand-made work, the way OpenGFX grew for OpenTTD. Two ways to help:

* **Replace a file.** Run `sgtool freeset /tmp/free` to get every drawn file, pick one, make your own version with the same
  name and size, and put it under the same path in `packs/opensgolf-free/` (pictures as PNG under `gfx/`, sprites as FLC under
  `gfx/Flics/`, sounds as OGG or WAV under `sfx/`, text under `text/`). Files there are loaded over the drawn ones. In a sprite
  sheet keep every piece inside the box the drawn version uses: the engine cuts sheets at fixed places (the boxes of each sheet
  are in `crates/sg-core/src/freeset/manifest.txt`). Sprites must keep the frame size, views, frames per view and anchor
  (`sgtool sheet file.flc out.png` shows the frames) and the palette slots described in `sprite.rs` (golfers: shirt 0..19,
  trousers 20..39, skin 40..59, hat 60..79, hair or hands 80..96; index 255 transparent).
* **Improve the drawing code.** Better shapes, poses, buildings or sounds in `freeset/*.rs` improve every file of a kind at
  once. Run `tools/freeset_playtest.sh` (headless; it needs xvfb) and look at the stills it writes.

Rules: your own work, or work under a GPL-compatible free licence (say which, and the tool you used; include source files
such as `.kra`, `.blend` or `.svg` where you can). Nothing traced, upscaled, recoloured or otherwise taken from the original
game's art, sounds, music or text. See [docs/MODDING.md](docs/MODDING.md) for the formats.

### The files and their sizes

About 5,500 files in all (some 82 MB written out). The engine reads them by these paths in the game folder; in the pack they sit under
`gfx/`, `sfx/`, `text/` and `fonts/` (KLEPTO__.TTF and manu3_.TTF are the two fonts).

| Folder | Kind | Files | Sizes |
|--------|------|------:|-------|
| `(game folder)` | pictures | 6 | 64x64, 640x480, 800x600 |
| `Bodies` | pictures | 20 | 60x60, 60x120 |
| `Bodies` | palettes (PCX) | 20 | 60x60 |
| `Data` | pictures | 4 | 800x600 |
| `Data/Textures` | terrain textures (BMP) | 2583 | 64x64 |
| `Data/Textures` | path overlays (TGA) | 36 | 64x64 |
| `Flics/Animals` | sprites (FLC) | 68 | frames 17x15 to 75x51 px, 4 to 8 views, 9 to 24 frames a view |
| `Flics/Animals` | palettes (PCX) | 10 | 60x60, 480x480 |
| `Flics/Bldgs` | sprites (FLC) | 502 | frames 11x13 to 442x298 px, 1 to 4 views, 1 to 31 frames a view |
| `Flics/Bldgs` | palettes (PCX) | 101 | 21x24, 320x320, 480x480, 751x812 |
| `Flics/Bldgs` | pictures | 1 | 800x600 |
| `Flics/Bridges` | sprites (FLC) | 92 | frames 41x29 to 88x58 px, 1 to 4 views, 1 to 25 frames a view |
| `Flics/Bridges` | palettes (PCX) | 7 | 320x320, 480x480 |
| `Flics/Celebs` | sprites (FLC) | 84 | frames 22x28 to 45x41 px, 8 views, 13 to 35 frames a view |
| `Flics/Celebs` | palettes (PCX) | 55 | 160x160, 320x320, 480x480 |
| `Flics/Employee` | sprites (FLC) | 62 | frames 24x31 to 118x97 px, 8 views, 1 to 35 frames a view |
| `Flics/Employee` | palettes (PCX) | 10 | 60x60, 480x480 |
| `Flics/Female` | palettes (PCX) | 6 | 160x160 |
| `Flics/Female` | sprites (FLC) | 232 | frames 23x26 to 76x65 px, 4 to 8 views, 7 to 35 frames a view |
| `Flics/Flowers` | sprites (FLC) | 156 | frames 33x30 to 107x104 px, 4 to 8 views, 1 to 38 frames a view |
| `Flics/Flowers` | palettes (PCX) | 52 | 480x480 |
| `Flics/Homes` | palettes (PCX) | 31 | 480x480 |
| `Flics/Homes` | sprites (FLC) | 66 | frames 2x2 to 165x138 px, 4 views, 1 to 24 frames a view |
| `Flics/Landmarks` | sprites (FLC) | 66 | frames 30x24 to 383x247 px, 4 to 8 views, 1 to 31 frames a view |
| `Flics/Landmarks` | palettes (PCX) | 28 | 60x60, 480x480 |
| `Flics/Male` | sprites (FLC) | 204 | frames 25x28 to 149x97 px, 4 to 8 views, 10 to 34 frames a view |
| `Flics/Male` | palettes (PCX) | 6 | 160x160, 480x480 |
| `Flics/Scenic` | sprites (FLC) | 146 | frames 16x16 to 227x177 px, 0 to 8 views, 1 to 52 frames a view |
| `Flics/Scenic` | palettes (PCX) | 71 | 60x60, 480x480 |
| `Flics/Tees` | palettes (PCX) | 6 | 480x480 |
| `Flics/Tees` | sprites (FLC) | 36 | frames 17x26 to 104x93 px, 1 to 4 views, 1 to 16 frames a view |
| `Flics/Trees` | sprites (FLC) | 122 | frames 26x30 to 90x95 px, 4 views, 9 to 16 frames a view |
| `Flics/Trees` | palettes (PCX) | 57 | 480x480 |
| `Flics/Water` | sprites (FLC) | 57 | frames 32x28 to 78x59 px, 2 to 4 views, 1 to 35 frames a view |
| `Flics/Water` | palettes (PCX) | 4 | 480x480 |
| `Heads` | pictures | 10 | 140x420, 1000x744, 1400x840 |
| `Interface` | pictures | 60 | 115x229, 450x150, 751x812, 800x100 ... |
| `Interface` | alpha sheets | 19 | 800x100, 800x600 |
| `Interface` | terrain descriptions | 4 |  |
| `Interface/infoscreens` | pictures | 23 | 18x447, 266x131, 300x127, 700x400 ... |
| `Interface/infoscreens` | alpha sheets | 12 | 800x600 |
| `SimsFX/Female` | sounds (WAV) | 38 | 0.2 s to 3.1 s in the original |
| `SimsFX/Male` | sounds (WAV) | 38 | 0.4 s to 4.8 s in the original |
| `Sounds` | sounds (WAV) | 19 | 0.7 s to 68.2 s in the original |
| `Sounds/Celebs` | sounds (WAV) | 44 | 0.6 s to 6.3 s in the original |
| `Sounds/Effects` | sounds (WAV) | 35 | 0.4 s to 6.2 s in the original |
| `Sounds/Emotion` | sounds (WAV) | 52 | 1.5 s to 4.8 s in the original |
| `Sounds/Female` | sounds (WAV) | 9 | 0.5 s to 2.5 s in the original |
| `Sounds/Golf_Sfx` | sounds (WAV) | 26 | 0.4 s to 4.0 s in the original |
| `Sounds/Interface` | sounds (WAV) | 32 | 0.1 s to 3.6 s in the original |
| `Sounds/Male` | sounds (WAV) | 7 | 0.7 s to 2.8 s in the original |
| `Sounds/World` | sounds (WAV) | 3 | 10.1 s to 20.0 s in the original |

Exact sizes per file: `crates/sg-core/src/freeset/manifest.txt` (P picture w h, F sprite w h views frames-per-view anchor,
W sound length) or the files `sgtool freeset` writes.

## Translations

Copy `lang/en.txt` to `lang/<language code>.txt`, set the `##language` and `##name` lines at the top, and translate the
text after the colon. Keep the `{PARAMETERS}` (in any order your language needs), the spaces at the start or end of a text
where the English has them, and the leading space of pop-up options. Texts you leave out stay English. Try it with
`--lang-file lang/<code>.txt`, or put it in a `lang` folder and pick it in Preferences. The details are in
[docs/TRANSLATING.md](docs/TRANSLATING.md).

New text in the code goes into `lang/en.txt` with a key (`STR_<AREA>_<WHAT>`), never as a literal in the code: use
`sg_core::lang::tr("STR_...")` or `trf("STR_...", &[("NAME", &value)])`. `cargo test -p sg-core lang` checks that every key
the code uses is in `lang/en.txt` once and that none there is unused.

## Content packs, scenarios and scripts

Mods, scenarios, game scripts and translations are content packs ([docs/MODDING.md](docs/MODDING.md)); the examples in
`packs/` (`example-cheap-fairways`, `example-scenario-desert-challenge`, `example-script-bonus`) are complete small packs to
start from. A finished pack is shared through the content index, a separate repository (planned: `opensgolf/content`) that
the game's Preferences > Content > *Find more* reads: open a pull request there that adds your pack's entry to
`index.json`, as described in [docs/CONTENT_INDEX.md](docs/CONTENT_INDEX.md). Packs must have a free licence and contain
nothing from the original game. Engine changes for packs (the loader, scenarios, the Rhai API, the index) come to this
repository with tests, like any other code.

## Reporting bugs and differences from the original

Use the issue templates. For a difference from the original game, a screenshot or a video timestamp of the original next to
the same moment in OpenSGolf helps most.
