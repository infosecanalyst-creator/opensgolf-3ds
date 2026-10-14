# Where development stands

A short handoff note so work can be picked up in a fresh session.

## Released

- **v0.2.5** is the latest release (`main`): all of 0.2 (course sizes, controls, mod profiles, original-bug fixes,
  gameplay and interface extras) plus the Windows on ARM fix (patched miniquad in `vendor/miniquad`, see its
  `README-OPENSGOLF.md`; drop it once miniquad fixes `GWLP_USERDATA` on aarch64).
- The browser version at <https://crspytopgn.github.io/opensgolf/> deploys automatically from `main`.
- Releases are made by running the `release.yml` workflow on `main` with `version: vX.Y.Z`. Bump `version` in
  `Cargo.toml` and `version_code` / `version_name` in `crates/simgolf/Cargo.toml` first.
- CI: checks run once per PR (and on pushes to `main`); the Android check builds arm64 only with a cached build;
  releases build every target.

## Next: 0.3

See "Roadmap: 0.3" in the README. The rule: the original game is always one click away; everything new is optional
and off by default, except fixes for the original's own bugs. Planned: a one-click Original/Extras switch on the New
Game screen, course sharing and challenges, undo, autosave and save slots with pictures, controller support.

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
