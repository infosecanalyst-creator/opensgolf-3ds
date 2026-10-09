#!/usr/bin/env python3
"""Builds an HD art pack for the SimGolf port from YOUR OWN copy of the game.

The game's art belongs to Electronic Arts. This tool reads it from your game folder, upscales it on your machine and writes the
result into a folder of your choosing. The pack is a derivative of the original art: keep it for yourself, never share or upload
it. The port's repository ships only this tool and the loader, never any art.

    python3 hd_pack.py --game "C:/Games/SimGolf" --out "%APPDATA%/SimGolf/HD"
    python3 hd_pack.py --game game/Program_Files_(ENGLISH) --out ~/.local/share/simgolf/HD --method lanczos

What goes in the pack (each image exactly SCALE times its source size, so every cut, layout and hit test of the game keeps
working; the game draws the HD image in the place of the classic one):

  * every PCX of the interface, the info screens, Heads, Data and the game folder's root, transparency as the port keys it
    (the magenta key, the per-file key colours of the port's loaders); the separate alpha sheets (``*_A.pcx``,
    ``*_alpha.pcx``) are upscaled as masks;
  * the terrain textures (Data/Textures/<theme>/*.bmp, *.tga), upscaled as tiles (wrapping at the edges);
  * the frames of the sprite animations under Flics (buildings, trees, scenery, animals, water, bridges, landmarks, homes,
    flowers, tees), with palette index 255 transparent as in the port.

Left out (the game smooths them instead): the golfer, celebrity and staff clips (Flics/Male, Female, Celebs, Employee) and the
body sheets (Bodies), which the game recolours per person at run time, and the sprite shadows.

Upscalers:
  --method realesrgan  runs realesrgan-ncnn-vulkan (https://github.com/xinntao/Real-ESRGAN-ncnn-vulkan/releases, BSD-3-Clause;
                       the release zips include the models). Found on PATH, next to this script, or given with --realesrgan.
  --method lanczos     plain Lanczos resampling with Pillow (not AI; for testing the pipeline and the game's loader).
  --method auto        (default) Real-ESRGAN when found, else Lanczos with a warning.

Requires Python 3.8+ and Pillow (pip install Pillow).
"""

import argparse
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time

try:
    from PIL import Image, ImageChops
except ImportError:  # pragma: no cover
    sys.exit("hd_pack needs Pillow: pip install Pillow")

PACK_FORMAT = "simgolf-hd-pack"
PACK_VERSION = 1

# Colour keys the port's loaders apply besides magenta, by file (lower case, relative to the game folder). Mirrors the key
# arguments of ui::load_pcx calls in crates/simgolf/src (hud_ui.rs, info_ui.rs, main.rs).
EXTRA_KEYS = {
    "interface/starsheartsetc.pcx": [(0x00, 0xFF, 0x00)],
    "interface/memberpanel.pcx": [(0x6B, 0x6B, 0x9C)],
    "gbubbles.pcx": [(0x9B, 0xE7, 0xFF)],
    "interface/chooseparklandbuttons.pcx": [(0xFF, 0x00, 0x00)],
    "interface/chooselinksbuttons.pcx": [(0xFF, 0x00, 0x00)],
    "interface/choosedesertbuttons.pcx": [(0xFF, 0x00, 0x00)],
    "interface/choosetropicalbuttons.pcx": [(0xFF, 0x00, 0x00)],
}

# Sprite folders the game recolours per person: not upscaled.
PEOPLE_DIRS = ("flics/male/", "flics/female/", "flics/celebs/", "flics/employee/")

PAD = 8  # context pixels around each image for the upscaler (cropped off again)


def log(*a):
    print(*a, flush=True)


# ---- decoding the game's formats -------------------------------------------------------------------------------------------


def keyed_rgba(img, extra):
    """RGB image -> RGBA with the port's key colours transparent: magenta as the port tests it (255,0,255 and the
    near-magenta 248,0,248 of some sheets) and the file's extra key colours."""
    img = img.convert("RGB")
    r, g, b = img.split()
    both = lambda x, y: ImageChops.multiply(x, y)  # logical and of 0/255 masks
    key = both(both(r.point(lambda v: 255 if v >= 240 else 0), g.point(lambda v: 255 if v <= 8 else 0)), b.point(lambda v: 255 if v >= 240 else 0))
    for kr, kg, kb in extra or ():
        k = both(both(r.point(lambda v, c=kr: 255 if v == c else 0), g.point(lambda v, c=kg: 255 if v == c else 0)), b.point(lambda v, c=kb: 255 if v == c else 0))
        key = ImageChops.lighter(key, k)
    out = img.convert("RGBA")
    if key.getextrema()[1] > 0:
        out.putalpha(ImageChops.invert(key))
    return out


def read_image(path):
    im = Image.open(path)
    im.load()
    return im


class FlcError(Exception):
    pass


def decode_flc(data):
    """A port of sg_core::flc::decode_flc: returns (w, h, frames, views, frames_per_view); frames are (indices, palette)."""
    if len(data) < 128:
        raise FlcError("short FLC")
    u16 = lambda o: struct.unpack_from("<H", data, o)[0]
    u32 = lambda o: struct.unpack_from("<I", data, o)[0]
    magic = u16(4)
    if magic not in (0xAF12, 0xAF11):
        raise FlcError("bad FLC magic")
    nframes = u16(6)
    w, h = u16(8), u16(10)
    if u16(12) != 8:
        raise FlcError("not 8-bit")
    if w == 0 or h == 0 or w > 4096 or h > 4096:
        raise FlcError("bad dimensions")
    views, fpv, has_ext = 1, nframes, False
    v, f = u16(96), u16(98)
    if 1 <= v <= 8 and f >= 1 and v * f == nframes and u16(104) == 480 and u16(106) == 480:
        views, fpv, has_ext = v, f, True
    idx = bytearray(w * h)
    pal = bytearray(768)
    frames = []
    pos, done = 128, 0
    stored = views * (fpv + 1) if has_ext else nframes

    def read_palette(p, end, six_bit, target):
        n = struct.unpack_from("<H", data, p)[0] if p + 2 <= end else 0
        p += 2
        i = 0
        while n > 0:
            n -= 1
            if p + 2 > end:
                return p, False
            i += data[p]
            cnt = data[p + 1] or 256
            p += 2
            for _ in range(cnt):
                if p + 3 > end:
                    return p, False
                if i < 256 and target is not None:
                    for k in range(3):
                        c = data[p + k]
                        target[i * 3 + k] = ((c << 2) | (c >> 4)) & 255 if six_bit else c
                p += 3
                i += 1
        return p, True

    while done < stored and pos + 16 <= len(data):
        fsize, ftype, nchunks = u32(pos), u16(pos + 4), u16(pos + 6)
        if fsize < 16 or pos + fsize > len(data):
            raise FlcError("bad frame size")
        if ftype != 0xF1FA:
            pos += fsize
            continue
        cp, fend = pos + 16, pos + fsize
        for ci in range(nchunks):
            if cp + 6 > fend:
                break
            csz, ct = u32(cp), u16(cp + 4)
            if csz < 6 or cp + csz > fend:
                if ct in (4, 11):
                    p, ok = read_palette(cp + 6, fend, ct == 11, None)
                    if not ok:
                        raise FlcError("bad palette chunk")
                    csz = p - cp
                elif ci + 1 == nchunks:
                    csz = fend - cp
                else:
                    raise FlcError("bad chunk size")
            p, end = cp + 6, cp + csz
            if ct in (4, 11):
                read_palette(p, end, ct == 11, pal)
            elif ct == 15:
                _brun(data, p, end, idx, w, h)
            elif ct == 7:
                _delta_flc(data, p, end, idx, w, h)
            elif ct == 12:
                _delta_fli(data, p, end, idx, w, h)
            elif ct == 13:
                idx[:] = bytes(len(idx))
            elif ct == 16 and end - p >= len(idx):
                idx[:] = data[p : p + len(idx)]
            cp += csz
        if not (has_ext and done % (fpv + 1) == fpv):
            frames.append((bytes(idx), bytes(pal)))
        done += 1
        pos = fend
    if not frames:
        raise FlcError("no frames")
    return w, h, frames, views, (fpv if has_ext else len(frames))


def _brun(d, p, end, idx, w, h):
    for y in range(h):
        if p >= end:
            return
        p += 1
        x, row = 0, y * w
        while x < w and p < end:
            s = d[p]
            p += 1
            if s < 128:
                if p >= end:
                    return
                v = d[p]
                p += 1
                n = min(s, w - x)
                idx[row + x : row + x + n] = bytes([v]) * n
                x += n
            else:
                n = 256 - s
                for _ in range(n):
                    if p >= end:
                        return
                    if x < w:
                        idx[row + x] = d[p]
                        x += 1
                    p += 1


def _delta_fli(d, p, end, idx, w, h):
    if p + 4 > end:
        return
    y, lines = struct.unpack_from("<HH", d, p)
    p += 4
    while lines > 0 and y < h and p < end:
        packets = d[p]
        p += 1
        x, row = 0, y * w
        while packets > 0 and p + 2 <= end:
            packets -= 1
            x += d[p]
            s = d[p + 1]
            p += 2
            if s < 128:
                for _ in range(s):
                    if p >= end:
                        break
                    if x < w:
                        idx[row + x] = d[p]
                    x += 1
                    p += 1
            else:
                if p >= end:
                    break
                v = d[p]
                p += 1
                for _ in range(256 - s):
                    if x < w:
                        idx[row + x] = v
                    x += 1
        lines -= 1
        y += 1


def _delta_flc(d, p, end, idx, w, h):
    if p + 2 > end:
        return
    lines = struct.unpack_from("<H", d, p)[0]
    p += 2
    y = 0

    def put(y, x, v):
        if y < h and x < w:
            idx[y * w + x] = v

    while lines > 0 and p + 2 <= end:
        op = struct.unpack_from("<H", d, p)[0]
        p += 2
        kind = op >> 14
        if kind == 0:
            packets, x = op, 0
            while packets > 0 and p + 2 <= end:
                packets -= 1
                x += d[p]
                s = d[p + 1]
                p += 2
                if s < 128:
                    for _ in range(s):
                        if p + 2 > end:
                            break
                        put(y, x, d[p])
                        put(y, x + 1, d[p + 1])
                        x += 2
                        p += 2
                else:
                    if p + 2 > end:
                        break
                    a, b = d[p], d[p + 1]
                    p += 2
                    for _ in range(256 - s):
                        put(y, x, a)
                        put(y, x + 1, b)
                        x += 2
            y += 1
            lines -= 1
        elif kind == 2:
            if y < h and w > 0:
                idx[y * w + w - 1] = op & 0xFF
        elif kind == 3:
            y += (65536 - op) & 0xFFFF
        else:
            return


def frame_rgba(w, h, indices, pal):
    """Palette lookup with index 255 transparent (sg_core::assets::to_rgba(.., Some(255)))."""
    im = Image.frombytes("P", (w, h), indices)
    im.putpalette(pal)
    rgba = im.convert("RGBA")
    mask = Image.frombytes("L", (w, h), indices).point(lambda v: 0 if v == 255 else 255)
    rgba.putalpha(mask)
    return rgba


# ---- image preparation ------------------------------------------------------------------------------------------------------


def _push_pull(im):
    """RGBA -> opaque RGBA whose colours spread from the opaque pixels into the transparent ones (Pillow resamples RGBA
    premultiplied, so averages carry only real colours)."""
    w, h = im.size
    if w <= 2 and h <= 2:
        out = im.copy()
        out.putalpha(255)
        return out
    small = im.resize((max(1, (w + 1) // 2), max(1, (h + 1) // 2)), Image.BOX)
    up = _push_pull(small).resize((w, h), Image.BILINEAR)
    return Image.alpha_composite(up, im)


def split_rgb_alpha(rgba):
    """(opaque RGB with the transparent areas filled from their surroundings, alpha L or None when fully opaque)."""
    a = rgba.getchannel("A")
    lo, hi = a.getextrema()
    if lo == 255:
        return rgba.convert("RGB"), None
    if hi == 0:
        return Image.new("RGB", rgba.size, (0, 0, 0)), a
    filled = _push_pull(rgba).convert("RGB")
    seen = a.point(lambda v: 255 if v else 0)
    return Image.composite(rgba.convert("RGB"), filled, seen), a


def pad(im, p, wrap):
    """Adds p pixels around the image: the image tiled (wrap) or its edges repeated."""
    w, h = im.size
    out = Image.new(im.mode, (w + 2 * p, h + 2 * p))
    if wrap:
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                out.paste(im, (p + dx * w, p + dy * h))
        return out
    out.paste(im, (p, p))
    out.paste(im.crop((0, 0, w, 1)).resize((w, p)), (p, 0))
    out.paste(im.crop((0, h - 1, w, h)).resize((w, p)), (p, p + h))
    out.paste(out.crop((p, 0, p + 1, h + 2 * p)).resize((p, h + 2 * p)), (0, 0))
    out.paste(out.crop((p + w - 1, 0, p + w, h + 2 * p)).resize((p, h + 2 * p)), (p + w, 0))
    return out


def steepen(a):
    """Sharpens an upscaled hard-edged mask back to a narrow anti-aliased edge."""
    return a.point(lambda v: max(0, min(255, (v - 128) * 2 + 128)))


def is_binary(a):
    cols = a.getcolors(256)
    return cols is not None and all(c in (0, 255) for _, c in cols)


# ---- upscalers -----------------------------------------------------------------------------------------------------------


class Lanczos:
    name = "lanczos"

    def __init__(self, scale):
        self.scale = scale

    def run(self, images):
        s = self.scale
        return [im.resize((im.width * s, im.height * s), Image.LANCZOS) for im in images]


class RealEsrgan:
    def __init__(self, exe, model, scale, tile, gpu):
        self.exe, self.model, self.scale, self.tile, self.gpu = exe, model, scale, tile, gpu
        self.name = "realesrgan:" + model

    def run(self, images):
        if not images:
            return []
        tmp = tempfile.mkdtemp(prefix="sg_hd_")
        try:
            src, dst = os.path.join(tmp, "in"), os.path.join(tmp, "out")
            os.makedirs(src)
            os.makedirs(dst)
            for i, im in enumerate(images):
                im.convert("RGB").save(os.path.join(src, "%06d.png" % i))
            # the x4 models only upscale 4 times; a 2x pack is made from their output
            model_scale = 2 if (self.scale == 2 and "x2" in self.model) else 4
            cmd = [self.exe, "-i", src, "-o", dst, "-n", self.model, "-s", str(model_scale), "-f", "png"]
            if self.tile:
                cmd += ["-t", str(self.tile)]
            if self.gpu is not None:
                cmd += ["-g", str(self.gpu)]
            models = os.path.join(os.path.dirname(os.path.abspath(self.exe)), "models")
            if os.path.isdir(models):
                cmd += ["-m", models]
            r = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            if r.returncode != 0:
                raise RuntimeError("realesrgan-ncnn-vulkan failed:\n" + r.stdout.decode(errors="replace")[-2000:])
            out = []
            for i, im in enumerate(images):
                p = os.path.join(dst, "%06d.png" % i)
                up = read_image(p).convert("RGB")
                want = (im.width * self.scale, im.height * self.scale)
                if up.size != want:
                    up = up.resize(want, Image.LANCZOS)
                out.append(up)
            return out
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


def find_realesrgan(given):
    names = ["realesrgan-ncnn-vulkan", "realesrgan-ncnn-vulkan.exe"]
    if given:
        return given if os.path.isfile(given) else None
    here = os.path.dirname(os.path.abspath(__file__))
    for n in names:
        p = shutil.which(n)
        if p:
            return p
        for d in (here, os.path.join(here, "realesrgan")):
            c = os.path.join(d, n)
            if os.path.isfile(c):
                return c
    return None


# ---- what to pack -----------------------------------------------------------------------------------------------------------


def rel_of(game, path):
    return os.path.relpath(path, game).replace("\\", "/")


def walk(game, top):
    base = os.path.join(game, top) if top else game
    if not os.path.isdir(base):
        return
    for root, dirs, files in os.walk(base):
        dirs.sort()
        for f in sorted(files):
            yield os.path.join(root, f)


def find_dir(game, name):
    """A top level folder of the game, matched case-insensitively."""
    for d in os.listdir(game):
        if d.lower() == name.lower() and os.path.isdir(os.path.join(game, d)):
            return d
    return None


class Job:
    """One picture to upscale: the low resolution RGBA plus where it goes."""

    __slots__ = ("rgba", "out", "wrap", "mask")

    def __init__(self, rgba, out, wrap=False, mask=False):
        self.rgba, self.out, self.wrap, self.mask = rgba, out, wrap, mask


def collect(game, kinds, only):
    """Yields (manifest key, entry, [jobs]) for every source picture."""
    match = (lambda r: re.search(only, r, re.I) is not None) if only else (lambda r: True)
    flics = find_dir(game, "Flics")
    bodies = find_dir(game, "Bodies")
    if "images" in kinds:
        for path in walk(game, ""):
            rel = rel_of(game, path)
            low = rel.lower()
            if not low.endswith(".pcx") or not match(rel):
                continue
            top = rel.split("/")[0]
            if top in (flics, bodies):
                continue
            try:
                im = read_image(path)
            except Exception as e:  # noqa: BLE001
                log("  skip %s: %s" % (rel, e))
                continue
            if im.mode == "P" and im.size == (1, 1):
                continue
            mask = re.search(r"(_a|_alpha)\.pcx$", low) is not None
            if mask:
                rgba = im.convert("RGB").convert("RGBA")
            else:
                rgba = keyed_rgba(im, EXTRA_KEYS.get(low))
            out = "images/" + rel[:-4] + ".png"
            entry = {"file": out, "w": im.width, "h": im.height}
            yield low, entry, [Job(rgba, out, mask=mask)]
    if "textures" in kinds:
        data = find_dir(game, "Data")
        tex = None
        if data:
            for d in os.listdir(os.path.join(game, data)):
                if d.lower() == "textures":
                    tex = data + "/" + d
        for path in walk(game, tex) if tex else ():
            rel = rel_of(game, path)
            low = rel.lower()
            if not (low.endswith(".bmp") or low.endswith(".tga")) or not match(rel):
                continue
            try:
                im = read_image(path).convert("RGBA")
            except Exception as e:  # noqa: BLE001
                log("  skip %s: %s" % (rel, e))
                continue
            out = "textures/" + rel[:-4] + ".png"
            yield low, {"file": out, "w": im.width, "h": im.height}, [Job(im, out, wrap=True)]
    if "sprites" in kinds and flics:
        for path in walk(game, flics):
            rel = rel_of(game, path)
            low = rel.lower()
            if not low.endswith(".flc") or low.endswith("shadow.flc") or not match(rel):
                continue
            if any(low.startswith(p) for p in PEOPLE_DIRS):
                continue
            try:
                with open(path, "rb") as f:
                    w, h, frames, views, fpv = decode_flc(f.read())
            except (FlcError, struct.error) as e:
                log("  skip %s: %s" % (rel, e))
                continue
            d = "sprites/" + rel[:-4]
            jobs = [Job(frame_rgba(w, h, ix, pal), "%s/%04d.png" % (d, i)) for i, (ix, pal) in enumerate(frames)]
            yield low, {"dir": d, "w": w, "h": h, "frames": len(frames)}, jobs


# ---- the pack ---------------------------------------------------------------------------------------------------------------


def process(jobs, up, scale, alpha_up, out_dir):
    rgbs, alphas = [], []
    for j in jobs:
        rgb, a = split_rgb_alpha(j.rgba)
        if j.mask:
            rgb, a = rgb.convert("L").convert("RGB"), None
        rgbs.append(pad(rgb, PAD, j.wrap))
        alphas.append(a)
    big = up.run(rgbs)
    alpha_jobs = [(i, a) for i, a in enumerate(alphas) if a is not None]
    if alpha_up is not None:
        big_a = alpha_up.run([pad(a.convert("RGB"), PAD, False) for _, a in alpha_jobs])
        big_a = [b.convert("L") for b in big_a]
    else:
        big_a = [pad(a, PAD, False).resize((a.width * scale + 2 * PAD * scale, a.height * scale + 2 * PAD * scale), Image.LANCZOS) for _, a in alpha_jobs]
    alpha_of = {}
    for (i, a), b in zip(alpha_jobs, big_a):
        b = b.crop((PAD * scale, PAD * scale, PAD * scale + a.width * scale, PAD * scale + a.height * scale))
        if is_binary(a):
            b = steepen(b)
        alpha_of[i] = b
    for i, (j, b) in enumerate(zip(jobs, big)):
        w, h = j.rgba.size
        b = b.crop((PAD * scale, PAD * scale, PAD * scale + w * scale, PAD * scale + h * scale))
        if j.mask:
            b = steepen(b) if is_binary(j.rgba.convert("L")) else b
            b = b.convert("L").convert("RGB")
        out = b.convert("RGBA")
        if i in alpha_of:
            out.putalpha(alpha_of[i])
        assert out.size == (w * scale, h * scale)
        p = os.path.join(out_dir, j.out)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        out.save(p, compress_level=6)


def main():
    ap = argparse.ArgumentParser(description="Make an HD art pack for the SimGolf port from your own copy of the game.")
    ap.add_argument("--game", required=True, help="the SimGolf game folder (the one holding Flics, Interface and Data)")
    ap.add_argument("--out", required=True, help="where the pack goes (the game looks in <save folder>/HD by default)")
    ap.add_argument("--scale", type=int, choices=(2, 4), default=4, help="upscale factor (default 4)")
    ap.add_argument("--method", choices=("auto", "realesrgan", "lanczos"), default="auto")
    ap.add_argument("--realesrgan", help="path of the realesrgan-ncnn-vulkan executable")
    ap.add_argument("--model", default="realesrgan-x4plus", help="Real-ESRGAN model: realesrgan-x4plus (default) or realesrgan-x4plus-anime")
    ap.add_argument("--alpha-ai", action="store_true", help="upscale transparency masks with the AI too (default: Lanczos, re-sharpened)")
    ap.add_argument("--tile", type=int, default=0, help="Real-ESRGAN tile size (0 = automatic; lower it if the GPU runs out of memory)")
    ap.add_argument("--gpu", type=int, help="Real-ESRGAN GPU id")
    ap.add_argument("--kinds", default="images,textures,sprites", help="what to pack: any of images, textures, sprites")
    ap.add_argument("--only", help="regular expression: only source files whose path matches (for trying things out)")
    ap.add_argument("--batch", type=int, default=64, help="pictures per upscaler run")
    args = ap.parse_args()

    game = os.path.abspath(args.game)
    if not find_dir(game, "Flics") or not find_dir(game, "Interface"):
        sys.exit("%s does not look like the SimGolf game folder (no Flics and Interface folders)" % game)
    out_dir = os.path.abspath(args.out)
    if os.path.commonpath([out_dir, game]) == game:
        sys.exit("put the pack outside the game folder")

    exe = find_realesrgan(args.realesrgan)
    if args.method == "realesrgan" and not exe:
        sys.exit("realesrgan-ncnn-vulkan not found: download it from https://github.com/xinntao/Real-ESRGAN-ncnn-vulkan/releases "
                 "and put it on PATH, next to this script, or pass --realesrgan")
    if args.method == "lanczos" or not exe:
        if args.method == "auto":
            log("note: realesrgan-ncnn-vulkan not found, using plain Lanczos resampling (not AI); see README.md")
        up = Lanczos(args.scale)
    else:
        up = RealEsrgan(exe, args.model, args.scale, args.tile, args.gpu)
    alpha_up = up if (args.alpha_ai and isinstance(up, RealEsrgan)) else None
    kinds = set(k.strip() for k in args.kinds.split(","))

    os.makedirs(out_dir, exist_ok=True)
    man_path = os.path.join(out_dir, "manifest.json")
    files = {}
    if os.path.isfile(man_path):
        try:
            with open(man_path) as f:
                old = json.load(f)
            if old.get("scale") == args.scale:
                files = old.get("files", {})
        except (OSError, ValueError):
            pass

    t0 = time.time()
    pending, pending_entries, count = [], [], 0

    def flush():
        nonlocal pending, pending_entries
        if pending:
            process(pending, up, args.scale, alpha_up, out_dir)
            for k, e in pending_entries:
                files[k] = e
            pending, pending_entries = [], []
            write_manifest(man_path, args.scale, up.name, files)

    for key, entry, jobs in collect(game, kinds, args.only):
        pending.extend(jobs)
        pending_entries.append((key, entry))
        count += len(jobs)
        if len(pending) >= args.batch:
            flush()
            log("  %d pictures, %.0f s" % (count, time.time() - t0))
    flush()
    write_manifest(man_path, args.scale, up.name, files)
    log("done: %d pictures upscaled %dx with %s into %s (%.0f s)" % (count, args.scale, up.name, out_dir, time.time() - t0))
    log("This pack is made from your own copy of the game's art: keep it to yourself, do not share or upload it.")


def write_manifest(path, scale, method, files):
    man = {
        "format": PACK_FORMAT,
        "version": PACK_VERSION,
        "scale": scale,
        "method": method,
        "note": "Made locally from the user's own copy of SimGolf. Derived from EA's art: not to be shared.",
        "files": dict(sorted(files.items())),
    }
    tmp = path + ".tmp"
    with open(tmp, "w") as f:
        json.dump(man, f, indent=0)
    os.replace(tmp, path)


if __name__ == "__main__":
    main()
