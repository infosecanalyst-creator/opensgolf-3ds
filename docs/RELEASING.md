# Releasing OpenSGolf

Pushing a tag such as `v0.4.1` (or running Actions → release → *Run workflow* with a version) builds every package and
publishes the release (`.github/workflows/release.yml`). The package names carry no version, so the README's links to
`releases/latest/download/<name>` always give the newest.

## Signed releases ("Update now")

The game's **Update now** (`crates/simgolf/src/autoupdate.rs`) installs a package only when its SHA-256 is listed in the
release's `SHA256SUMS` and that file's ed25519 signature, `SHA256SUMS.sig`, matches the public key built into the game.
The release workflow's *Sign the packages* step writes both files with the private key from the GitHub Actions secret
**OPENSGOLF_UPDATE_KEY**. Without the secret the release is still published, with `SHA256SUMS` but no signature and a
warning in the run's summary; the game then refuses Update now for it and offers the download page.

### Setting up the key (once)

1. Make a key pair on a computer you trust (Linux, macOS or WSL):

   ```sh
   cargo run --release -p sgtool -- update-keygen
   ```

   It prints the **public key** and the **private key**, 64 hex digits each. (Without sgtool: `openssl genpkey -algorithm
   ed25519 -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n'` prints a private key; the workflow prints the public
   key it signs with.)
2. In the repository on GitHub: Settings → Secrets and variables → Actions → *New repository secret*, name
   `OPENSGOLF_UPDATE_KEY`, value the private key. Never commit it, paste it anywhere else or put it in an issue; keep an
   offline copy (a password manager) so later releases can be signed with the same key.
3. Put the public key in `crates/simgolf/src/autoupdate.rs` as `PUBLIC_KEY_HEX` (replacing the all-zero placeholder,
   which makes every build refuse Update now) and commit that.
4. The next release's *Sign the packages* step prints `signed with public key ...`: it must be the same as
   `PUBLIC_KEY_HEX`.

Versions built before step 3 carry the placeholder and never update themselves; from the first version with the key on,
each one can update to the next.

### Changing the key

A new key needs a new build: players' copies trust only the key they were built with. Make a new pair, put the new public
key in `autoupdate.rs` and release that version signed with the **old** key (so current copies install it), then replace
the secret with the new private key for the releases after it. If the private key leaked, replace it the same way at
once; copies that never updated past the leak will trust the old key, so say so in the release notes.

### Signing by hand

`OPENSGOLF_UPDATE_KEY=<private key> sgtool update-sign SHA256SUMS` writes `SHA256SUMS.sig` (and prints the public key),
for a release whose files were uploaded by hand. `sha256sum opensgolf-* > SHA256SUMS` makes the list; upload both.
