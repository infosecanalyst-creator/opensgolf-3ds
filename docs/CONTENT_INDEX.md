# The content index

OpenSGolf can download content packs (docs/MODDING.md) from **System Functions > Preferences > Content packs > Find
more...**. The list comes from a *content index*: one JSON file, `index.json`, at an address the player sets. The index is
meant to live in a public repository where anyone can add a pack with a pull request, the way OpenTTD's BaNaNaS lists
content; a pack only needs a free licence and must not contain material from the original game.

The engine reads the index in crates/sg-core/src/index.rs (with its tests); the screen is in crates/simgolf/src/content.rs
and the downloads in crates/simgolf/src/net.rs.

## Setting the address

There is no index built in. Put its address in `settings.json` in the save folder (`~/.local/share/opensgolf/` on Linux,
`%APPDATA%\OpenSGolf\` on Windows, `~/Library/Application Support/OpenSGolf/` on macOS, or `$OPENSGOLF_SAVE_DIR`), next to
the other settings:

```json
{
  "content_index_url": "https://example.org/opensgolf-content/index.json"
}
```

Until it is set, Find more says so and lists nothing. The address can be:

* `https://...` (or `http://...`): fetched by the game's HTTPS client on the desktop (the default `net` cargo feature; a
  build with `--no-default-features` cannot download and says so), by the page's `fetch()` in a browser (the server must
  send CORS headers that allow it; GitHub's raw files and release downloads do);
* `file:///path/to/index.json` or a plain path: read from the disk, everywhere. This is the easy way to try an index or a
  pack before publishing it.

## index.json

```json
{
  "format": 1,
  "packs": [
    {
      "id": "example-cheap-fairways",
      "name": "Cheap Fairways",
      "version": "1.0.0",
      "kind": "mod",
      "licence": "CC0-1.0",
      "authors": ["OpenSGolf contributors"],
      "description": "Fairways cost a third of the usual price.",
      "url": "packs/example-cheap-fairways-1.0.0.zip",
      "sha256": "9f2c...64 hex digits...",
      "size": 2817,
      "requires": ["original >= 1.0"]
    }
  ]
}
```

| Field | Required | Meaning |
|-------|----------|---------|
| `format` | no | The index format, 1. An index of a newer format is refused with "update the game". |
| `packs` | yes | The packs. |
| `id`, `name`, `version`, `kind` | yes | As in the pack's pack.toml (docs/MODDING.md 9). The downloaded pack must have the same id and version. |
| `licence` (or `license`) | yes | The SPDX identifier of the pack's licence: free licences only (e.g. `CC0-1.0`, `CC-BY-4.0`, `CC-BY-SA-4.0`, `GPL-2.0-or-later`). |
| `authors`, `description`, `requires` | no | The authors and the description are shown on the Find more screen; `requires` is as in pack.toml (for the index's readers: the game checks the pack's own pack.toml when it loads it). |
| `url` | yes | Where the pack's `.zip` is: an absolute address, or a path relative to the index's own address. |
| `sha256` | yes | The SHA-256 of the .zip, 64 hex digits (`sha256sum pack.zip`). |
| `size` | yes | The .zip's size in bytes (at most 256 MB). |

Unknown fields are an error for that entry. An entry with a problem (a bad id, a missing field, a checksum that is not 64
hex digits, an id listed twice) is left out and shown as a problem under the list; the rest of the index is used.

## What the game does

1. Reads the index and lists the packs the player does not have, or has in an older version (*update*), by kind and
   name.
2. **Download** fetches the pack's .zip and checks, before anything is written: the size, the SHA-256 checksum, that it is
   a pack (a pack.toml at the root or in its one top folder) and that its id and version are the index's.
3. Writes it to `<save folder>/packs/<id>.zip`. An older .zip of the same pack in that folder is removed; an older version
   installed there as a folder must be removed by the player first (the game says so). In a browser the pack is kept with
   the packs added on the start page.
4. The new pack is listed on the Content screen, switched off (a mod) or ready to be chosen (a base set, translation or
   scenario); like every change there it takes effect when the game starts again.

## Making a pack for the index

* Make the pack (docs/MODDING.md 9), zip its folder (`zip -r my-pack-1.0.0.zip my-pack`), and try it from the packs folder.
* Write its entry: `sha256sum` and `stat -c %s` (or `wc -c`) give the checksum and the size.
* Try the entry with a local index: put index.json and the zip in a folder, set `content_index_url` to
  `file:///that/folder/index.json`, and use Find more.
* A new version is a new entry version, a new zip and a new checksum; players with the older version see it as an update.
