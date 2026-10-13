# Where development stands

A short handoff note so work can be picked up in a fresh session.

## Released

- **v0.1.5** is the latest release (`main`). It has the Android start fix, the Firefox black-screen fix (#9), the sound
  fixes for other WAV formats and OGG (#10), the Mac lag fix and the browser sound fix (AudioWorklet).
- The browser version at <https://crspytopgn.github.io/opensgolf/> deploys automatically from `main`.
- Releases are made by running the `release.yml` workflow on `main` with `version: vX.Y.Z`. Bump `version` in
  `Cargo.toml` and `version_code` / `version_name` in `crates/simgolf/Cargo.toml` first.

## 0.2 (not released yet)

**On `claude/v02-ui-extras`** (all of 0.2 so far: `v02-work` merged into the UI extras; builds, all tests pass):

- Course sizes: Original 50, Large 74, Huge 98, Giant 128 (`sg_core::mapsize`, use `land::mn()` instead of 50).
- Gameplay extras, off by default (Preferences, "OpenSGolf extras"): turning buildings (Tab), cut-content landmarks,
  Hardcore economy, staff limits (`Extras::staff_limit`), buyable animals. See `docs/CUT_CONTENT.md`.
- Controls: remappable keys, optional WASD camera, middle-drag pan, U hides the interface, frame-rate cap and v-sync,
  mod profiles, packs recorded in saves.
- "Fix the original game's bugs" setting (on by default): waterfalls, double charges, upgrades and golfer slots. See
  `docs/FIXES.md`.
- `--perf` performance log.
- UI extras, on by default (merged): the People screen, golfer profiles, message history, name tags, report tabs and
  path help (`extras.rs`; settings.json `"ui_extras"`, `--no-extras` turns them off for a run). Preferences >
  OpenSGolf extras lists the gameplay extras, then the interface extras. Their keys (Shift+G, Shift+M, Shift+H,
  Shift+V) are remappable actions in `controls.rs`; People's staff limit is `max_employees(Extras::staff_limit())`.
- The web build no longer stops at its first frame (`perf::frame_begin` read the clock with the log off).

## To ship 0.2.0

1. Merge `claude/v02-ui-extras` into `claude/magical-tesla-d1thkl` (which since has v0.1.6, the Windows on ARM
   download button and the PR #17 merge).
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
