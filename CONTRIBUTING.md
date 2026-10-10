# Contributing to OpenSGolf

Thank you for helping. OpenSGolf is a free golf course management game that plays like Sid Meier's SimGolf and is built to
be modded. Code, art, sound, text, translations, courses, testing and bug reports are all welcome.

## Ground rules

* **No material from the original game in the repository.** No art, sounds, music, videos, text, fonts or code from SimGolf,
  not even small pieces, and nothing traced or upscaled from it. The engine reads those files from each player's own copy at
  runtime. The free base set and all packs here must be made by their authors.
* **No decompiled code.** Rules taken from the original are written down as facts (what happens, the numbers) in `docs/`,
  and the engine's code is written from those notes.
* By contributing you agree that your work is published under the project's licence: GPL-2.0-or-later for code and the free
  base set, or the licence named in a pack's `pack.toml`.
* Be kind: see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Code

1. Install Rust (https://rustup.rs). On Linux also `sudo apt install libasound2-dev`.
2. `cargo build --release`, then `target/release/simgolf --game "/path/to/your/SimGolf/Program_Files_(ENGLISH)"`.
3. Before a pull request: `cargo fmt`, `cargo clippy --release --workspace`, `cargo test --release --workspace`, and for
   browser changes `web/build.sh`.
4. Match the style of the code around your change. Comments say where a rule comes from (an exe address, the manual, footage
   of the original) or that it is the project's own choice.

Layout: `crates/sg-core` is the game rules and file formats (no window, unit tested); `crates/simgolf` is the game
(miniquad); `crates/sgtool` inspects and converts assets; `web/` is the browser build; `docs/` holds the notes on the
original's formats and rules.

## Art, sound and text for the free base set

See [docs/MODDING.md](docs/MODDING.md) for the pack layout. Art follows the original's sizes and layouts so it drops into the
same places; the file lists per screen are in `docs/UI_ART_MAP.md` and `docs/UI_PANELS.md`. Say which tool you used and
include source files (e.g. `.kra`, `.blend`) where you can.

## Translations

Copy `lang/en.txt` to `lang/<language code>.txt` and translate the text after the colon. Keep the `{PARAMETERS}`.

## Reporting bugs and differences from the original

Use the issue templates. For a difference from the original game, a screenshot or a video timestamp of the original next to
the same moment in OpenSGolf helps most.
