# HD graphics (optional)

SimGolf's art was drawn for 800×600. On today's screens the port scales it up, and it gets soft. **HD mode** draws the same
art upscaled ×4 (or ×2), by the AI upscaler built into the game (or one you run yourself, such as Real-ESRGAN), plus
trilinear filtering and anti-aliasing.
Everything stays where it was: the layouts, the sprite cuts and the click areas are the classic ones.

**Classic is the default and is never changed by HD.** With HD off, the game runs exactly the code it ran before HD
existed and produces the same pixels (checked by rendering the same scenes with and without the HD code and comparing the
PNGs byte for byte). HD is opt-in, and switching back to Classic restores the classic output exactly.

## Legal note

The game's art belongs to Electronic Arts and the port reads it at run time from your own copy. An upscaled pack is a
derivative of that art, so:

- you make it **yourself, locally, from your own copy**, in the game (Preferences → Graphics → *Make HD graphics from my
  SimGolf files (beta)...*) or with `tools/hd_pack`;
- it is **never** committed to this repository, bundled with a release, put into `web/dist`, or offered for download by
  this project; `.gitignore` excludes `HD/` and pack folders;
- **do not share or upload your pack.**

The repository ships only the pack makers and the loader. The game's own AI upscaler is built in (Real-ESRGAN's small
general model, BSD-3-Clause, see the README's licence section). The game never downloads anything for HD.

## Making a pack

**In the game (one click; beta).** With your SimGolf files in use, System Functions → Preferences → Graphics → *Make HD
graphics from my SimGolf files (beta)...*. The game times its AI upscaler on your device for a moment and says how many
pictures, how much space and how long *on this device*. It then makes the pack in the background while you play: *Keep
playing* closes the box and a note at the top of the screen shows the percentage and the time left (a click on it brings
the box back, with *Stop*). A run that is stopped, or cut short by closing the game, carries on by itself the next time the
game starts. When it is done HD switches itself on (no restart), a box says so, and it stays on. *Remove HD graphics...*
(same box) switches back to Classic and deletes the pack.

| | scale | size | time |
|---|---|---|---|
| *Make it* | ×4 | about 1.2 GB | measured on the device; about 10 minutes on an 8-core laptop, 25 on a 4-core one |
| *Make a smaller one* | ×2 | about 0.6 GB | the same (the model always works at ×4; ×2 is its output halved) |

The upscaler is Real-ESRGAN's small general model (realesr-general-wdn-x4v3, BSD-3-Clause, Xintao Wang), run on the
processor in plain Rust (`crates/sg-core/src/aiscale.rs`, 2.4 MB of weights): no graphics card, no download, the same
result on Windows, macOS, Linux and Android. It works out only the pixels that can reach a visible one (49% of the art's 245
million; the clear space round sprites costs nearly nothing) and uses the processor's vector unit (AVX2 or AVX-512 with
FMA on PCs, NEON on ARM). The colours come from the model and the transparency from xBRZ's upscale of the same picture
(colour keys become real transparency first). Pictures of the course's theme are made first. Browsers are left out for now:
there the model runs on one thread without the vector unit, which would take many hours.

Natively the pack goes to `<save folder>/HD` (or `--hd-pack DIR`). The manifest's `method` is
`ai:realesr-general-wdn-x4v3`; otherwise it is the pack `hd_pack.py` writes, so everything below holds for either.

Test hook: `opensgolf --game <folder> --make-hd [--hd-pack DIR] [--hd-scale 2|4] [--hd-method ai|xbrz|realesrgan]` makes
the pack without a window and says how long it took (docs/PLAYING.md lists the rest).

**With the script.** See [tools/hd_pack/README.md](../tools/hd_pack/README.md). In short:

```sh
pip install Pillow
# with Real-ESRGAN (realesrgan-ncnn-vulkan on PATH or next to the script):
python3 tools/hd_pack/hd_pack.py --game <your SimGolf folder> --out <save folder>/HD
# without AI (Lanczos, for testing):
python3 tools/hd_pack/hd_pack.py --game <your SimGolf folder> --out <save folder>/HD --method lanczos
```

The save folder is `%APPDATA%\OpenSGolf` on Windows, `~/Library/Application Support/OpenSGolf` on macOS and
`~/.local/share/opensgolf` (or `$XDG_DATA_HOME/opensgolf`) on Linux, or `OPENSGOLF_SAVE_DIR` when set.

## Smaller packs

Pictures in a pack may be WebP instead of PNG (same name with `.webp`). At quality 95 a pack is about a tenth of
the size and looks the same, which helps when moving a 4x pack between machines.

## Turning it on

- **In the game:** System Functions → Preferences → *HD graphics (off: Classic)*. Switching reloads all the art: the
  interface, the terrain textures and the sprites. The choice is saved in `settings.json` in the save folder.
- **Command line:** `--hd` or `--classic` for one run (overriding the saved choice), and `--hd-pack DIR` for a pack
  somewhere other than `<save folder>/HD`.
- **Browser:** under *Optional: HD graphics* on the start page, pick the pack folder or a .zip of it. It is kept in the
  browser like the game files, and the game then starts in HD. Preferences switch it off and on.

If HD is chosen and no pack is found, the game says how to make one and stays Classic.

Content packs (docs/MODDING.md) sit above the HD pack: when an enabled mod replaces a picture, terrain texture or sprite,
the mod's file is used, loaded the classic way (or at its own resolution for a high-resolution pack of your own art,
docs/MODDING.md section 11), and the HD pack's upscale of the original is not. Everything else is as described here.

## What changes in HD

| | Classic | HD |
|---|---|---|
| Interface, info screens, heads, theme tiles | original PCX | pack PNG at ×scale, same on-screen size |
| Terrain textures | original BMP/TGA, mipmapped | pack PNG at ×scale, mipmapped |
| Buildings, trees, scenery, animals, water, bridges, flowers... | original FLC frames | pack frames at ×scale, mipmapped; colour variants recoloured from the pack's frames |
| Golfers, celebrities, staff, bodies, sprite shadows | original, bilinear | original, bilinear (not upscaled, see below) |
| Anti-aliasing | none | 4× MSAA (when the renderer can resolve multisampled targets) |
| Texture filtering | bilinear | trilinear (mipmaps) for pack textures |

How it works: each loader (`ui::load_pcx`, `load_pcx_alpha`, `split_overlay`, the terrain `texture_for` and the sprite
loader) first checks `hd::active()`. Only in HD does it look the file up in the pack's manifest. It then creates the
texture at the pack's resolution but reports the original size, so all drawing code keeps working in the original's pixel
units (texture coordinates are relative). Anything the pack lacks, or whose size does not match, loads the classic way.
The code is in `crates/simgolf/src/hd.rs`.

## Limitations

- People (golfers, celebrities, staff) and the body sheets are recoloured per person from palettes at run time, so they
  are not upscaled. They are drawn from the classic art with smooth filtering.
- The hole signposts, portraits and accomplishment snapshots stay classic.
- A file loaded both with and without its key colour shows the pack's colours under the key (the classic magenta would
  show there instead). Some files contain both 255,0,255 and 248,0,248, and the pack treats both as transparent.
- Palette variants of sprites are made by adding the palette change, spread smoothly, to the default-palette upscale. This
  is close to, but not exactly, an upscale of the variant.
- A ×4 pack is large (about 0.8 GB on disk with xBRZ, 1.6 GB with Real-ESRGAN or Lanczos), and HD sprites need much more
  video memory than classic ones. Use ×2 on small GPUs and in browsers.
- The quality depends on the upscaler. The Lanczos path of hd_pack.py is for testing only.
