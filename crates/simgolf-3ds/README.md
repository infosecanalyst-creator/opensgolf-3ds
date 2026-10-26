# OpenSGolf on the New Nintendo 3DS

A 3DS frontend for [OpenSGolf](https://github.com/crspytopgn/opensgolf), the open-source engine that plays Sid Meier's SimGolf.
The course is on the stereoscopic top screen and you build it with the stylus on the bottom screen, in the style of DS-era
building games. Target hardware: New 3DS / New 3DS XL (an Old 3DS is not supported yet).

**Status: work in progress.** The whole game crate runs on the console: painting the course costs money through the game's own
rules, holes open with a par, golfers arrive and play, staff can be hired, the game saves and loads. Tournaments, the land
market, the pro's own rounds and the rest of the desktop's menus are not wired up yet.

## Playing

The game opens on a title screen (no "press Start": straight to the menu, with Continue chosen):

| Title screen | |
| --- | --- |
| Continue | the newest save: the autosave written when you leave a game, or any later one |
| New Course | the Parkland plots on offer, with acres, price and what is left to build with (plots the club cannot afford are greyed out) |
| Load Game | the saves, newest first |
| Settings | golfers' thoughts on or off, sound volume (kept in `sdmc:/3ds/opensgolf/settings.txt`) |
| Controls | the controls page |
| Quit | back to the Homebrew Launcher |

Tap a row, or use the D-pad and A; B goes back. The game is saved to the autosave when you choose Title screen or Quit from
the pause menu, when the console sleeps or goes to the HOME Menu, and every five minutes of play.

Opening a course takes about 5 seconds on a start after the first: the sprites a course needed are listed in
`sdmc:/3ds/opensgolf/cache/sprites.txt` and decoded on a spare CPU core while the title is up.

The touch screen is where you act, the top screen is where you read, as in DS-era building games.

**Bottom screen:** the course as a flat map, and a strip of tools down its right edge.

| On the map | |
| --- | --- |
| Drag | paint or build along the stylus |
| Tap | do it once |
| Hold still for half a second | inspect: a golfer, a hole, a building or the ground, shown on the top screen |
| Hold L or R and drag | slide the view (also the D-pad or C-stick; dragging into the map's edge while painting scrolls too) |

| Strip | |
| --- | --- |
| COURSE tab | HOLE (tap a tee spot, then a green spot, or drag tee to green: it builds both and opens the hole, showing length and par as you go), tee, green, fairway, rough, deep rough, sand, water, trees, raise, lower, move |
| BUILD tab | gravel and paved paths, bench, flower bed, ball washer, willow, scenic bridge, BLDG (the building list with prices), TURN, CLEAR |
| STAFF tab | hire (green) and fire (red) club pros, rangers, groundskeepers and soda vendors |
| INFO tab | money, holes and golfers panels on the top screen, SAVE, LOAD, golfers' thoughts on or off, HELP |
| Bar under the map, always | UNDO (30 steps, money included), PAUSE / PLAY, SPEED (1x, 2x, 4x), SIZE (brush 1, 2, 3, remembered per tool) |

Every button is at least 28 pixels (about 8 mm) across, so a fingertip works as well as the stylus.

The status line over the map names the tool and its price per tile (exactly, for the tile under the stylus), counts what a
stroke has spent, and says what to do next. Tiles the game refuses (not your land, too dear, a building) flash red; land the
club does not own is dimmed.

**Top screen:** the 3D course with a bar along the top (cash, with what you just spent or earned, the date and speed, golfers on
the course, fun and skill ratings), the game's messages and golfers' thoughts along the foot, and the inspect card or the INFO
panels over it.

| Buttons | |
| --- | --- |
| L / R (tap) | zoom out / in (three steps closer than the PC game) |
| C-stick / D-pad | slide the view |
| Circle Pad / A | move the top screen's cursor / use the tool there |
| B | undo |
| X | pause |
| Y | next tool tab |
| ZL / ZR | game speed down / up (through pause) |
| Select | money, holes, golfers and staff panels on the top screen, then none |
| Start | pause menu: resume, save, load, controls, title screen, quit (D-pad and A, or tap a row) |
| 3D slider | depth: raised land comes out of the screen, lowered land and water sink into it |

### Game files

OpenSGolf needs art and sound. Either:

- **Your own copy of SimGolf:** copy the folder containing `Data`, `Flics` and `Sounds` to `sdmc:/3ds/opensgolf/game`, or
- **none:** on first start OpenSGolf writes its free base set (about 82 MB) to `sdmc:/3ds/opensgolf/free`. That takes a few
  minutes on the console; `sgtool freeset <dir>` on a PC is faster.

No game files are included in this repository or its builds.

### Installing

Copy `simgolf-3ds.3dsx` (from a release, the Actions artifacts, or your own build) to `sdmc:/3ds/` and start it from the
Homebrew Launcher, or send it over Wi-Fi with `3dslink` (Homebrew Launcher, Y for NetLoader). The console needs custom firmware
(<https://3ds.hacks.guide>) and, for sound, its DSP firmware dumped once with DSP1.

## Building

You need devkitPro's `3ds-dev` group (devkitARM, libctru, citro3d, picasso), Rust nightly with `rust-src`, `cargo-3ds`, and
libclang (ctru-sys runs bindgen at build time).

```sh
rustup toolchain install nightly -c rust-src
cargo install --locked cargo-3ds
cargo +nightly 3ds build --release -p simgolf-3ds
# target/armv6k-nintendo-3ds/release/simgolf-3ds.3dsx
```

GitHub Actions builds it on every push (`.github/workflows/3ds.yml`, in the devkitpro/devkitarm container).
`tools/3ds-windows/` has the PowerShell scripts used on Windows: toolchain setup, build, Azahar, Wi-Fi launch and FTP install.
They expect to sit in the folder that contains this repository, checked out as `opensgolf`.

## How it fits together

- `src/main.rs`: the main loop. It runs the game crate (`crates/simgolf`) unchanged, compiled in through `#[path]` modules, and
  draws its world on the top screen.
- `src/gfx.rs`: a citro3d renderer standing in for the game's miniquad one: stereo shear per eye, 16-bit textures where the
  picture allows, and sprite frames packed into shared atlas pages so a course of trees and golfers draws in a few dozen calls.
- `src/plan.rs`: the bottom screen's course plan and tool strip, editing through `App::paint_tiles`, `edit_elevation` and the
  game's undo.
- `vendor/miniquad-3ds`: a no-op stand-in for miniquad's windowing and GPU, so the game crate builds unchanged.
- `vendor/ctru-rs`, `vendor/citro3d-rs`: the rust3ds crates with small build-script fixes for Windows paths.
- `src/hud.rs`: the top screen's bar, messages, thoughts, inspect card and panels; `src/text.rs`: Liberation Sans Bold rasterized
  with fontdue for them.
- In `sg-core`, the 3DS keeps sprite frames packed at half size until they are drawn (`Sprite::packed`): one golfer's decoded
  clips alone were 35 MB, more than the console had free. `sg_core::preload` decodes every golfer body on a spare CPU core at
  start, so a new kind of golfer arriving does not stop the game.

## License

GPL-2.0-or-later, as OpenSGolf (`LICENSE` at the repository root). The vendored rust3ds crates keep their own licenses.
SimGolf is a trademark of its owners; this project is not affiliated with or endorsed by them.
