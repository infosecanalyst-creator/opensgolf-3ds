# Where development stands

A short handoff note so work can be picked up in a fresh session.

## Released

- **v0.1.5** is the latest release (`main`). It has the Android start fix, the Firefox black-screen fix (#9), the sound
  fixes for other WAV formats and OGG (#10), the Mac lag fix and the browser sound fix (AudioWorklet).
- The browser version at <https://crspytopgn.github.io/opensgolf/> deploys automatically from `main`.
- Releases are made by running the `release.yml` workflow on `main` with `version: vX.Y.Z`. Bump `version` in
  `Cargo.toml` and `version_code` / `version_name` in `crates/simgolf/Cargo.toml` first.

## 0.2 (not released yet)

**On `claude/magical-tesla-d1thkl`** (builds, all tests pass):

- Course sizes: Original 50, Large 74, Huge 98, Giant 128 (`sg_core::mapsize`, use `land::mn()` instead of 50).
- Gameplay extras, off by default (Preferences, "OpenSGolf extras"): turning buildings (Tab), cut-content landmarks,
  Hardcore economy, staff limits (`Extras::staff_limit`), buyable animals. See `docs/CUT_CONTENT.md`.
- Controls: remappable keys, optional WASD camera, middle-drag pan, U hides the interface, frame-rate cap and v-sync,
  mod profiles, packs recorded in saves.
- "Fix the original game's bugs" setting (on by default): waterfalls, double charges, upgrades and golfer slots. See
  `docs/FIXES.md`.
- `--perf` performance log.

**On `claude/v02-ui-extras`** (finished but NOT merged):

- The People screen, golfer profiles, message history, name tags, report tabs and path help: commits `c5c71e5` and
  `a8ed33a`.
- It is based on v0.1.3 and conflicts with `claude/magical-tesla-d1thkl` in `app.rs`, `main.rs`, `panels_ui.rs`,
  `popup_ui.rs` and `lang/en.txt`.
- When merging, keep both sides:
  - The name-tag key must not clash with the remappable keys.
  - Staff counts use `Extras::staff_limit`.
  - Map sizes use `mapsize`.
  - Keep every new `lang/en.txt` key.

## To ship 0.2.0

1. Merge `claude/v02-ui-extras` into `claude/magical-tesla-d1thkl` and resolve the conflicts above.
2. Run the checks below, play through each new feature, and check the browser build in Chromium.
3. Bump to 0.2.0, open a PR to `main`, wait for CI to pass, merge, then run `release.yml` with `v0.2.0`.

## Checks before every commit

- `cargo fmt`
- `cargo clippy --release --workspace`: only the existing gfx.rs "manual checked division" warning may remain.
- `cargo test --release --workspace`
- `cd web && ./build.sh`
- `tools/freeset_playtest.sh`
- With every extra off, the Classic look must stay unchanged.

Never commit game files, decompiled code or model names.

## Open player reports

- Browser sound with the original files: fixed in v0.1.5; waiting to hear back from the Reddit player.
- Mac lag: fixed in v0.1.4 but not confirmed on a real Mac.
- Still unexplained: why Enhanced graphics felt faster than Classic on that Mac.
- Unfixed: a GPU texture churn of about 13,500 textures over a long session. Look at it next if Mac players still
  report hitches.
