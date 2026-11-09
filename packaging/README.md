# Packaging OpenSGolf for stores and package managers

This folder has what each store or package manager needs, ready to submit. They all build from or point at the
packages that `.github/workflows/release.yml` publishes for a tag `vX.Y.Z` (docs/RELEASING.md). Nothing here changes
those packages.

| Where | Files | What it uses |
|---|---|---|
| Linux desktops (all stores) | `linux/io.github.crspytopgn.OpenSGolf.metainfo.xml`, `linux/io.github.crspytopgn.OpenSGolf.desktop`, `linux/icons/` | AppStream metadata, menu entry, icons |
| Flathub | `flatpak/io.github.crspytopgn.OpenSGolf.yml`, `flatpak/cargo-sources.json` | builds from the tag's source |
| F-Droid | `fdroid/org.opensgolf.game.yml`, `../fastlane/metadata/android/en-US/` | builds the APK from the tag's source |
| Homebrew (macOS) | `homebrew/opensgolf.rb` | `opensgolf-macos.zip` |
| winget (Windows) | `winget/crspytopgn.OpenSGolf*.yaml` | `opensgolf-windows.zip`, `opensgolf-windows-arm64.zip` |
| Scoop (Windows) | `scoop/opensgolf.json` | the same two .zip files |

Store screenshots are in `screenshots/` (desktop, 1280 x 720) and `../fastlane/metadata/android/en-US/images/
phoneScreenshots/` (phone, 1600 x 720, touch buttons on). They show only OpenSGolf's own free graphics. **Never use a
picture of the original game's art in any store listing**, and never name or link a place to get the game's files: a
listing only says that players can use their own copy. How the pictures were taken is at the end of this file.

`linux/opensgolf.desktop` is the older menu entry that the AppImage and the .tar.gz still carry; the new one has the
same content with the Flathub id as its icon name.

## Store builds: no self-update

The game's **Check for updates** and **Update now** (crates/simgolf/src/update.rs, autoupdate.rs) must not run where a
store installs the updates. Building with the environment variable `OPENSGOLF_NO_SELF_UPDATE=1` leaves both out (the
title button, the Preferences line and the answer box: `update::STORE_BUILD`); everything else, downloading content
packs included, stays. The Flatpak manifest and the F-Droid recipe set it. The Homebrew, winget and Scoop packages are
the release's own .zip files, so Update now is still there; it works, but the package manager then no longer knows the
version installed, so their notes tell players to update through the package manager.

## Each release

1. Tag and publish the release as usual (docs/RELEASING.md). Wait for the release workflow; `SHA256SUMS` in the
   release lists the SHA-256 of every package.
2. Add a `<release version="X.Y.Z" date="YYYY-MM-DD">` at the top of `<releases>` in the metainfo, and
   `../fastlane/metadata/android/en-US/changelogs/<version_code>.txt` (the `version_code` in crates/simgolf/Cargo.toml,
   raised by one each release; at most 500 characters). Commit both **before** tagging, so the tag carries them.
3. If Cargo.lock changed, make `flatpak/cargo-sources.json` again (below).
4. Fill in the drafts' placeholders and submit the update to each store as described below. A script for step 4, run
   with the release's version and its downloaded `SHA256SUMS`:

   ```sh
   v=0.5.6; sums=SHA256SUMS
   h() { awk -v f="$1" '$2 == f { print $1 }' "$sums"; }
   sed -e "s/@VERSION@/$v/g" -e "s/@SHA256@/$(h opensgolf-macos.zip)/" homebrew/opensgolf.rb
   sed -e "s/@VERSION@/$v/g" -e "s/@SHA256_X64@/$(h opensgolf-windows.zip)/" \
       -e "s/@SHA256_ARM64@/$(h opensgolf-windows-arm64.zip)/" -e "s/@DATE@/$(date +%F)/" \
       winget/crspytopgn.OpenSGolf.installer.yaml   # and the same for the other two winget files and scoop/opensgolf.json
   ```

   **A CI step (an idea, not set up)**: a job after `publish` in release.yml that runs this, then opens the pull
   requests itself: `brew bump-cask-pr --version X.Y.Z opensgolf` (Homebrew), `wingetcreate update crspytopgn.OpenSGolf
   --version X.Y.Z --urls ... --submit` (winget, needs a GitHub token secret with `public_repo`), and a commit to your
   own Scoop bucket. Flathub notices new tags itself with `x-checker-data` (below); F-Droid reads the tags itself
   (`UpdateCheckMode: Tags`).

## Checking the files

```sh
appstreamcli validate --pedantic packaging/linux/io.github.crspytopgn.OpenSGolf.metainfo.xml
desktop-file-validate packaging/linux/io.github.crspytopgn.OpenSGolf.desktop
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest packaging/flatpak/io.github.crspytopgn.OpenSGolf.yml
```

The first two passed for this version (the only pedantic note is that the id has capital letters, which Flathub
accepts). Without `--no-net`, appstreamcli also fetches the screenshots, which exist at their addresses only once this
folder is on `main`.

## Flathub

What it is: the main store for Linux apps (GNOME Software, KDE Discover, Steam Deck's desktop mode). The app id
`io.github.crspytopgn.OpenSGolf` proves ownership through the GitHub account crspytopgn (Flathub checks it once, through
a GitHub login, when the app is verified).

Building it locally first (needs `flatpak` and `flatpak-builder`, about 2 GB for the SDK):

```sh
flatpak install --user flathub org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.rust-stable//25.08 org.flatpak.Builder
# in the manifest, replace the git source with  - type: dir / path: ../..  for a build of your checkout
flatpak run org.flatpak.Builder --user --install --force-clean build-dir packaging/flatpak/io.github.crspytopgn.OpenSGolf.yml
flatpak run io.github.crspytopgn.OpenSGolf
```

Check: the game starts on the free graphics; "Use my SimGolf files..." opens the desktop's file chooser and the game
then plays with the chosen files after a restart; sound plays; the title menu has no Check for updates button.

`cargo-sources.json` (every crate of Cargo.lock, so the build needs no network) is made with
[flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo):

```sh
pip install aiohttp tomlkit
python3 flatpak-cargo-generator.py Cargo.lock -o packaging/flatpak/cargo-sources.json
```

Submitting (once):

1. Fork https://github.com/flathub/flathub and make a branch from its `new-pr` branch (not `master`).
2. Add `io.github.crspytopgn.OpenSGolf.yml` and `cargo-sources.json` at the top of the branch, with the git source's
   `tag: vX.Y.Z` and `commit:` filled in (`git rev-list -n 1 vX.Y.Z`). Optionally add, under the git source,
   `x-checker-data: {type: git, tag-pattern: '^v([\d.]+)$'}` so Flathub's bot opens a pull request for each new tag.
3. Open a pull request against `new-pr` titled "Add io.github.crspytopgn.OpenSGolf". Reviewers comment there; reply
   to `bot, build` comments to start a test build. Expect questions about `--device=all` (controllers) and
   `--share=network` (content packs); the manifest's comments say why each is there.
4. Once merged, Flathub makes the repository `flathub/io.github.crspytopgn.OpenSGolf` and gives you write access. Each
   later release is a pull request there (the bot's, or your own) changing the tag, commit and `cargo-sources.json`.
5. Verify the app on https://flathub.org/developers (log in with GitHub) to get the verified tick.

Flathub requires: the metainfo and desktop file as here, an icon of at least 128 x 128, a licence, screenshots in the
metainfo, and that the app builds from source without network. It does not want apps that update themselves (handled by
`OPENSGOLF_NO_SELF_UPDATE`).

## F-Droid

What it is: the store for free software on Android. F-Droid builds the APK itself from the source and signs it with
its own key, so its APK cannot install over the one from GitHub (or the other way round): players switch by
uninstalling first (copy the saves out first, docs/INSTALL.md).

The store texts, icon and phone screenshots are read from `fastlane/metadata/android/en-US/` at the release's tag.
`changelogs/40.txt` is for the next version (0.5.6, version_code 40): check it before tagging.

Submitting (once):

1. Get a GitLab account. Either open a Request For Packaging (https://gitlab.com/fdroid/rfp/-/issues, the template asks
   for the source, licence and anti-features) and wait for a packager, or, quicker, do it yourself:
2. Fork https://gitlab.com/fdroid/fdroiddata, copy `fdroid/org.opensgolf.game.yml` to `metadata/org.opensgolf.game.yml`,
   fill in the cargo-quad-apk commit, and try it with fdroidserver (`fdroid readmeta`, `fdroid lint org.opensgolf.game`,
   `fdroid build -v -l org.opensgolf.game`; the easiest way is the fdroidserver docker image or the buildserver VM,
   https://f-droid.org/docs/Submitting_to_F-Droid_Quick_Start_Guide/).
3. Open a merge request against fdroiddata. Its CI builds the app; reviewers comment in the merge request.
4. After it is merged, F-Droid picks up new tags itself (`AutoUpdateMode`/`UpdateCheckMode`): bumping `version_code`
   and `version_name` in crates/simgolf/Cargo.toml and tagging is all a release needs. A build takes a few days to appear.

What F-Droid needs and how the recipe does it:

* **Built from source, no prebuilt binaries.** The recipe runs the same `tools/android_build.sh` as the release
  workflow, outside docker: rustup (F-Droid's `rustup` srclib), cargo-quad-apk installed from its source, and F-Droid's
  own SDK and NDK. F-Droid's scanner refuses binary files in the source: `build_asan/` (a CMake build folder with static
  libraries) and `legacy/` are removed before the build with `scandelete`. Better: delete `build_asan/` from the
  repository.
* **NDK**: r25 (25.0.8775105), the one in the notfl3/cargo-apk image the workflow uses; minSdk 26, targetSdk 33.
  F-Droid accepts targetSdk 33, Google Play would not.
* **No self-update**: `OPENSGOLF_NO_SELF_UPDATE=1`, and the prebuild step removes the one permission it needed
  (`REQUEST_INSTALL_PACKAGES`), so the F-Droid build asks for no permissions at all.
* **Open questions for the first build** (to settle with the reviewers): `android_build.sh` makes a fresh lock file
  for cargo-quad-apk's old Cargo at every build, so two builds of one tag can pick different crate versions; committing
  that lock file (for example `crates/simgolf/android/Cargo.lock`) would make the build repeatable and is needed if you
  ever want reproducible builds. cargo-quad-apk signs the APK with a debug key it makes; fdroidserver signs the output
  with F-Droid's key, but if it asks for an unsigned APK, the APK needs its signature removed in a `postbuild` step. The
  recipe's SDK packages (`platforms;android-33`, `build-tools;34.0.0`) are a guess at what cargo-quad-apk needs.

**Anti-features**: none are needed, as far as can be judged. The app ships its own free graphics and sounds (GPL, like
the code; the font is under the SIL OFL), so `NonFreeAssets` does not apply; using the player's own SimGolf files is
optional and nothing of them is included or downloaded. There is no tracking, advertising or non-free dependency. The
network is used only when the player asks: content packs from the project's own content index on GitHub (free content,
so not `NonFreeNet`), and "Report a problem" opens the issue form in the browser. Say this in the merge request so the
reviewers can judge it.

## Homebrew (macOS)

`homebrew/opensgolf.rb` installs `OpenSGolf.app` from `opensgolf-macos.zip`.

**Blocker for the official `homebrew/cask`**: Homebrew no longer accepts casks for apps that fail macOS's Gatekeeper
check, which means apps not signed with an Apple Developer ID and notarised. OpenSGolf's app has only an ad-hoc
signature (release.yml). Until it is notarised (an Apple Developer Program membership, then `codesign` with the
Developer ID and `notarytool` in release.yml), publish the cask in your own tap:

1. Make a public GitHub repository `crspytopgn/homebrew-tap` with the filled-in cask as `Casks/opensgolf.rb`.
2. Players install with `brew install --cask crspytopgn/tap/opensgolf`. (Homebrew may still show Gatekeeper's warning
   on first start; docs/INSTALL.md says how to open it.)
3. Each release: update `version` and `sha256` and push, or run `brew bump-cask-pr --version X.Y.Z
   crspytopgn/tap/opensgolf`.

Once the app is notarised: fork https://github.com/Homebrew/homebrew-cask, add the cask as `Casks/o/opensgolf.rb`, check
it with `brew audit --cask --new --online opensgolf` and `brew style --fix opensgolf`, and open a pull request.
Homebrew's autobump then usually updates it for new releases by itself.

## winget (Windows)

The three files in `winget/` are a multi-file manifest for the portable .zip packages (x64 and arm64).
`ArchiveBinariesDependOnPath` puts the unpacked folder on PATH, because the program needs its `packs` and `lang`
folders next to it. winget does not make a Start menu entry for portable packages: players start it with `opensgolf`
from a terminal or the Run box, or make a shortcut.

1. Fill in the placeholders (above), then check with `winget validate --manifest packaging\winget` and test with
   `winget install --manifest packaging\winget` on a Windows PC (needs Settings, *Enable local manifest files*, or
   `winget settings --enable LocalManifestFiles` as administrator).
2. Fork https://github.com/microsoft/winget-pkgs, put the files in `manifests/c/crspytopgn/OpenSGolf/X.Y.Z/` and open a
   pull request. Microsoft's bots validate and test-install it; a moderator merges it. The first submission may get the
   label *Needs-Review* while the publisher is checked.
3. Each release: `wingetcreate update crspytopgn.OpenSGolf --version X.Y.Z --urls <x64 zip url>|x64 <arm64 zip
   url>|arm64 --submit` (wingetcreate, https://github.com/microsoft/winget-create; it needs a GitHub token).

Unsigned programs are accepted, but SmartScreen and some antivirus scanners in the validation pipeline can flag an
unsigned .exe; if that happens, the pull request says so.

## Scoop (Windows)

`scoop/opensgolf.json` installs the portable .zip, adds `opensgolf` to the command line and a Start menu shortcut, and
updates itself from the GitHub releases (`checkver`, `autoupdate` reading SHA256SUMS).

The official buckets: games go to the community bucket *games* (https://github.com/Calinou/scoop-games); fork it, add
`bucket/opensgolf.json` (the filled-in file), check it with that repository's `bin/checkver.ps1 opensgolf` and
`bin/checkhashes.ps1 opensgolf`, and open a pull request. Its bot (Excavator) then updates the version for each release
by itself. Alternatively keep it in your own bucket (a repository `crspytopgn/scoop-bucket` with `bucket/opensgolf.json`;
players run `scoop bucket add opensgolf https://github.com/crspytopgn/scoop-bucket`, then `scoop install opensgolf`).

## What the maintainer has to do

* **Accounts**: GitHub (Flathub, Homebrew, winget, Scoop: all pull requests); GitLab (F-Droid); for the official
  Homebrew cask also an Apple Developer Program membership (paid) for notarisation.
* **Reviews**: each first submission is reviewed by people, who ask questions in the pull or merge request; answer
  them there. Flathub and F-Droid usually take days to weeks; winget and Scoop a few days.
* **Per release**: the release entry in the metainfo and the fastlane changelog before tagging (above); afterwards the
  Flathub pull request (or the bot's), and the Homebrew/winget/Scoop updates if no bot does them.
* **Legal wording**: keep the "not affiliated with Electronic Arts or Firaxis" line in every listing, and use only the
  free-graphics screenshots.

## How the screenshots were made

On a virtual display, from a release build, with `--game` pointing at an empty folder (so the free set loads; the
title menu pictures were taken with no game folder anywhere instead, which is the same, and from a build with
`OPENSGOLF_NO_SELF_UPDATE=1`, as the stores get) and an empty `OPENSGOLF_SAVE_DIR`:

```sh
xvfb-run -a target/release/opensgolf --game /tmp/empty --mute --size 1280x720 --property 0 --clock 4242 --cash 500000 \
  --edit "t:16,30,0;t:32,24,1;p:20,29,2,0,1;p:24,28,2,0,1;p:28,26,2,0,1;o;t:33,29,0;t:31,20,1;p:33,26,2,0,1;p:32,23,2,0,1;o;t:33,14,0;t:17,13,1;p:30,13,2,0,1;p:26,13,2,0,1;p:21,13,2,0,1;p:24,16,22,0,1;o;t:18,22,0;t:20,8,1;p:18,19,2,0,1;p:19,15,2,0,1;p:20,11,2,0,1;o;h:2,24,27;h:2,22,14;h:1,25,20" \
  --time 400 --save-game course.sgs --png 02-course.png
xvfb-run -a target/release/opensgolf --game /tmp/empty --mute --size 1280x720 --load-game course.sgs --time 20 --png 03-golfers.png
xvfb-run -a target/release/opensgolf --game /tmp/empty --mute --size 1280x720 --load-game course.sgs --screen holestats --time 20 --png 04-hole-stats.png
xvfb-run -a target/release/opensgolf --mute --size 1280x720 --screen menu --time 6 --png 01-title.png
```

The phone pictures are the same with `SG_TOUCH=1` and `--size 1600x720`. The log must say `playing with the free
OpenSGolf graphics`.
