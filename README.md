# simgolf-native

## ▶ [Play SimGolf in your browser](https://crspytopgn.github.io/simgolf-native/)

Open the link above in a desktop browser (Chrome, Edge or Firefox), click **Choose your SimGolf game folder** and pick the
folder of your own installed copy of SimGolf (the folder that contains `Flics`, `Sounds` and `Data`). The game starts right away.

**You need your own copy of SimGolf.** The game's art, sounds and data belong to Electronic Arts and cannot be put on GitHub, so the
page uses the files from your copy. They are read inside your browser tab only; nothing is uploaded anywhere. Saved courses stay in
your browser. Sound starts after your first click.

---

Clean-room native port of Sid Meier's SimGolf (Firaxis, 2002), written in Rust, for macOS (Apple silicon and Intel), Windows and Linux.
It reads the data files from your own legitimately owned copy of the game. No game assets or code from the original are included here,
and nothing here removes or bypasses copy protection. The goal is a game that plays like the original in every respect; the rules are
being decoded from the publisher-supplied golf.exe (facts only, written down in docs/PUBLISHER_EXE_NOTES.md, no code copied).

## Status

Playable first loop: title menu, property chooser, a generated course on that property's theme, golfers who play every hole and pay
green fees, money and the board's debt warnings, course editing, buildings, staff, the course report, sound and the intro videos.
Many numbers are still placeholders until they are read from the exe; every placeholder is marked as one in the code and docs.
See docs/FORMATS.md (data formats), docs/TERRAIN.md (terrain engine) and docs/PLAYING.md (what plays today).

| Format | Files on disc | Decoded OK |
|--------|---------------|------------|
| PCX (8-bit RLE, 1 x 24-bit) | 649 | 649 |
| TGA (32-bit) | 36 | 36 |
| BMP (24-bit, 1 x 8-bit) | 2631 | 2631 |
| FLC animation (8-bit) | 1893 | 1893 |
| WAV (PCM) | 320 | 319 (1 is AIFF with a .wav name) |

## Build and run

Install Rust (https://rustup.rs), then from this folder:

    cargo build --release
    target/release/simgolf --game "/path/to/game/Program_Files_(ENGLISH)"

On Windows the programs are `target\release\simgolf.exe` and so on. Nothing else needs installing on macOS or Windows. On Linux the
sound needs the ALSA headers to build (`sudo apt install libasound2-dev`), or build without sound with
`cargo build --release -p simgolf --no-default-features`.

| Program | What it does |
|---------|--------------|
| `simgolf` | The game. |
| `sgtool` | Inspect and convert the original assets: `check DIR` decodes everything and reports failures; `png file.pcx out.png` (also tga, bmp, flc [frame]); `sheet anim.flc sheet.png`; `chr Gack.chr gack`; `top10 top10.sve`; `dta file.dta`; `story file.txt`; `fixture DIR [font.ttf]` writes a stand-in game folder with placeholder art for tests. |
| `sgplay` | Plays the Bink intro and closing videos through an installed ffmpeg (docs/SOUND_VIDEO.md). |

Code layout: `crates/sg-core` holds the decoders and the game rules (no window, no audio device, unit tested, including golden values
captured from the earlier C++ port); `crates/simgolf` is the game (miniquad for the window and OpenGL, cpal for sound, fontdue for the
game's font); `crates/sgtool` and `crates/sgplay` are the tools. The earlier C++ port is kept in `legacy/cpp` for reference.

`cargo test` runs the unit tests. CI builds and tests on macOS, Windows and Linux, and on Linux also starts the game on a virtual
display against a placeholder data folder and saves a screenshot.

## In a browser

`web/build.sh` builds the browser version into `web/dist` (needs `rustup target add wasm32-unknown-unknown`). Serve that folder
with any static web server, for example `python3 -m http.server -d web/dist`, and open it. The page asks for the folder of your
own SimGolf copy (the one containing Flics, Sounds and Data); its files are read into the browser tab only, nothing is uploaded.
Sound starts after the first click (a browser rule). Saved courses are kept in the browser's local storage. CI builds it too
(artifact `simgolf-web`).

## Playing

Keys: arrows/WASD pan, Q/E rotate, +/- or mouse wheel zoom, 1-4 themes, R new demo course, P toggle scenery, F follow the golfer,
F2 screenshot, M music, N mute, H advisor, Esc menu. Tab toggles course editing (see docs/EDITING.md). `--png out.png --size 1280x800 [--time SECONDS]` renders one frame and exits.
Videos: install ffmpeg (macOS: `brew install ffmpeg`), then `target/release/sgplay "/path/to/game/Program_Files_(ENGLISH)/Flics/SMSG_IntroFinal.bik"`.
The course is a made-up demo. Terrain edges are blended per triangle exactly as the original
`Terrain.dll` does it (see docs/TERRAIN.md); the type grouping that decides where borders appear is my
own, because the original receives it from the protected executable.

## Getting the game data

The retail disc is an InstallShield install (data1.hdr + data1.cab + data2.cab).
Extract it with `unshield` (`brew install unshield`, then `unshield x data1.cab`).
`golf.exe` is wrapped in SafeDisc; this project never touches it.

## Findings so far

* Engine split: `golf.exe` (logic, protected), `Terrain.dll` (terrain + OpenGL rendering, plain C++
  exports such as `Terrain::tileAt`, `getElevation`, `render`), `jgl.dll` (2D layer on GDI),
  `sound.dll` (DirectSound/WinMM), `binkw32.dll` (video).
* FLC quirk: Firaxis' writer left some chunk size fields uninitialised (0xCDCDCDCD), mostly on
  palette chunks. The decoder recovers the real size (see crates/sg-core/src/flc.rs).
* Sprite FLCs use magenta as the transparency key. Sprites have 8 camera angles per animation.
* Character files (.glf/.chr/.pro): 1826-byte record (title, name, 25 dialogue slots) plus an
  optional embedded 140x420 PCX of three portraits (happy, neutral, angry).
* Still undecoded: the 16 attribute bytes in character files, top10 fields, the story file name codes.

## Roadmap

1. Asset loaders (done)
2. Reverse the custom formats (done, a few fields unresolved, see docs/FORMATS.md)
3. Analyse Terrain.dll to reproduce the tile model and rendering (viewer with edge blending, paths, walls and a water shimmer done; the original water animation and cliff rules are not decoded)
3b. Sprite layer: FLC sprite format, views and anchors decoded, trees, a clubhouse and walking golfers in the viewer (see docs/SPRITES.md)
4. Game logic: the exe's golfers (arrivals, walking, needs, amenities, the shot planner, ball flight, fees, memberships), see docs/GAMELOGIC.md
5. Course editing: paint tile types, raise and lower terrain, save and load (done, basic, with path overlays; see docs/EDITING.md)
6. Club money (placeholder numbers, see docs/EDITING.md), retaining walls, water shimmer and the desert water swap (done)
7. Sound (mixer plus event sounds in the game) and Bink video playback via ffmpeg (done, see docs/SOUND_VIDEO.md)
8. Notes from the original manual (rules only, it gives no amounts): docs/MANUAL_NOTES.md. Done from it: SGA hole classes, basic employees, golfer fun and attitude, the pathway-to-clubhouse rule
9. Facts read from a publisher-supplied golf.exe (rules and numbers only, no code copied): docs/PUBLISHER_EXE_NOTES.md
10. First playable loop: several holes, a stream of golfers paying fees, the board's debt warnings (docs/PLAYING.md)
11. Port to Rust for macOS, Windows and Linux (done; the C++ port's courses, shots, ratings and exe maths are reproduced exactly, checked by golden tests)
12. Next: tile decoration drawing (the hole tool is in: holes are built, planned and opened as in the exe), stories and special visitors, ratings, tournaments, whole-game saves
