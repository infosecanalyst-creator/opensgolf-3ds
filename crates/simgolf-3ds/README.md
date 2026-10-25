# OpenSGolf on the New Nintendo 3DS

A 3DS frontend for [OpenSGolf](https://github.com/crspytopgn/opensgolf), the open-source engine that plays Sid Meier's SimGolf.
The course is on the stereoscopic top screen and you build it with the stylus on the bottom screen, in the style of DS-era
building games. Target hardware: New 3DS / New 3DS XL (an Old 3DS is not supported yet).

**Status: work in progress.** The whole game crate runs on the console: painting the course costs money through the game's own
rules, holes open with a par, golfers arrive and play, and the game's undo works. Menus, saving, staff and the rest of the
desktop interface are not wired up yet.

## Playing

| Control | |
| --- | --- |
| Stylus, bottom screen | paint with the armed tool; drag on the edge to scroll; tap the tool strip to change tool or brush size |
| Tap a new hole's green | open the hole (the status line says when one is ready) |
| Circle Pad / D-pad | move the view |
| L / R | zoom out / in |
| A | use the armed tool at the top screen's cursor |
| B | undo (up to 30 steps, money included) |
| X | pause |
| 3D slider | depth: raised land comes out of the screen, lowered land and water sink into it |
| Start + Select | quit |

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
- In `sg-core`, the 3DS keeps sprite frames packed until they are drawn (`Sprite::packed`): one golfer's decoded clips alone were
  35 MB, more than the console had free.

## License

GPL-2.0-or-later, as OpenSGolf (`LICENSE` at the repository root). The vendored rust3ds crates keep their own licenses.
SimGolf is a trademark of its owners; this project is not affiliated with or endorsed by them.
