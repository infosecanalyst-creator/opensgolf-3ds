# hd_pack: make an HD art pack from your own copy of SimGolf

`hd_pack.py` upscales the game's art (×4 by default, or ×2) into a folder the port loads when **HD graphics** are turned on.
For what HD mode is and its limits, see [docs/HD.md](../../docs/HD.md).

> **Legal note.** The art belongs to Electronic Arts. The pack is a derivative of it, so make it yourself, from your own
> copy, on your own machine, and **never share, upload or commit it**. This repository ships only this tool and the game's
> loader. No art, upscaled or otherwise, is ever bundled or downloadable from this project.

## Requirements

- Python 3.8+ and Pillow: `pip install Pillow`
- For AI upscaling: **realesrgan-ncnn-vulkan** (BSD-3-Clause) from
  <https://github.com/xinntao/Real-ESRGAN/releases> (the `realesrgan-ncnn-vulkan-*-windows/ubuntu/macos.zip` files; the
  zips include the models `realesrgan-x4plus` and `realesrgan-x4plus-anime`). Unzip it anywhere, then put the executable on
  your PATH, next to `hd_pack.py`, or pass `--realesrgan PATH`. You need a GPU with Vulkan support. It also runs on a CPU Vulkan
  driver such as Mesa lavapipe, but very slowly.

## Usage

```sh
# AI upscaling (Real-ESRGAN), into the folder the game looks in by default (<save folder>/HD):
python3 tools/hd_pack/hd_pack.py --game "C:/Program Files/Maxis/SimGolf" --out "%APPDATA%/SimGolf/HD"
python3 tools/hd_pack/hd_pack.py --game game/Program_Files_(ENGLISH) --out ~/.local/share/simgolf/HD          # Linux
python3 tools/hd_pack/hd_pack.py --game ... --out "~/Library/Application Support/SimGolf/HD"                   # macOS

# no AI, plain Lanczos resampling (to test the pipeline and the loader quickly):
python3 tools/hd_pack/hd_pack.py --game ... --out /tmp/hdpack --method lanczos
```

Options:

| option | meaning |
|---|---|
| `--scale 4` / `--scale 2` | upscale factor (default 4). ×2 packs are a quarter of the size and use less video memory |
| `--method auto\|realesrgan\|lanczos` | `auto` (default) uses Real-ESRGAN when found, else Lanczos with a warning |
| `--model realesrgan-x4plus-anime` | the other Real-ESRGAN model (sharper flat colours, which suits the interface) |
| `--alpha-ai` | upscale transparency masks with the AI too (default: Lanczos, re-sharpened) |
| `--kinds images,textures,sprites` | which groups to pack. `images` (interface, about 160 pictures) is the quickest to try |
| `--only REGEX` | only source files whose path matches, for trying things out |
| `--tile N`, `--gpu N` | passed to Real-ESRGAN (lower `--tile` if the GPU runs out of memory) |

The tool can be run again into the same folder: the manifest merges new entries, so you can for example pack `images`
first and `sprites` later. Packs of different scales are not mixed.

A full ×4 pack holds about 25,000 pictures and takes about 1.6 GB (Lanczos: about 10 minutes on one CPU core; Real-ESRGAN:
from tens of minutes to hours depending on the GPU). In a browser, prefer `--kinds images,textures` or `--scale 2`, since the
page keeps the pack in memory.

## What is in a pack

```
manifest.json            {"format": "simgolf-hd-pack", "version": 1, "scale": 4, "files": {source → entry}}
images/<path>.png        every PCX outside Flics and Bodies (Interface, infoscreens, Heads, Data/<theme>.pcx, root files)
textures/<path>.png      Data/Textures/<theme>/*.bmp, *.tga (the terrain), upscaled as wrapping tiles
sprites/<path>/NNNN.png  the frames of every Flics/**/*.flc except shadows and people
```

Manifest keys are the source path relative to the game folder, in lower case. An entry has the source's size (`w`, `h`)
and the picture (`file`) or, for sprites, the frame folder (`dir`) and `frames`. Every picture is exactly `scale` times its
source, so sprite-sheet cuts stay where they were.

How each kind is prepared (mirroring how the port loads it):

- **Transparency.** Magenta (255,0,255, and the near-magenta 248,0,248) is the key everywhere, plus the per-file key colours
  the port's loaders use (`EXTRA_KEYS` in the script). Sprites use palette index 255. Alpha sheets (`*_A.pcx`,
  `*_alpha.pcx`) are packed as masks of their own; the game combines them as it does with the originals.
- **No colour fringes.** Before upscaling, transparent pixels are filled with the colours of their opaque neighbours
  (push-pull fill), so the key colour never bleeds into edges. The RGB goes through the upscaler and the alpha separately.
  Hard-edged masks are re-sharpened after upscaling, giving a narrow anti-aliased edge.
- **Borders.** Every picture gets 8 pixels of context (edge repeat, or wrap-around for terrain tiles), cropped off afterwards.

Not packed, because the game recolours them per person at run time (an upscale per palette is not practical): the golfer,
celebrity and staff clips (`Flics/Male`, `Female`, `Celebs`, `Employee`) and the body sheets (`Bodies`). The sprite
shadows (translucent black) are not packed either. In HD the game draws all of these from the classic art, smoothly filtered.
Sprites drawn in another palette than their file's (tree and building colour variants) use the pack's frames with the
palette change applied at load time.

`--no-resume` redoes every picture; by default a run continues an interrupted one made with the same method and scale.

Without a GPU, Real-ESRGAN runs on the CPU through Mesa's lavapipe Vulkan driver (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json` on Linux). The `realesr-animevideov3` model is about six times faster than `realesrgan-x4plus-anime` there (a full x4 pack takes several hours on four cores).
