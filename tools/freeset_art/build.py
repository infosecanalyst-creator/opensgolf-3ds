#!/usr/bin/env python3
"""Renders the free set's sprites (tools/freeset_art) into packs/opensgolf-free/art/: one indexed PNG sheet per sprite file
(frames in a grid, a row per view), which the engine turns into the FLC the game reads (crates/sg-core/src/freeset/baked.rs).
The frame sizes, views, frame counts and ground points come from the layout manifest (crates/sg-core/src/freeset/
manifest.txt); everything drawn is our own (procedural models, see trees.py and the other recipe modules).

usage: build.py [--only REGEX] [--list] [--jobs N]
needs: numpy, Pillow (python3 -m pip install numpy pillow)"""
import argparse
import hashlib
import os
import re
import sys
from concurrent.futures import ProcessPoolExecutor

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
OUT = os.path.join(ROOT, "packs", "opensgolf-free", "art")

import layout  # noqa: E402
import mats  # noqa: E402
import recipes  # noqa: E402
from raster import Frame  # noqa: E402
from scene import fit_scale  # noqa: E402

KEY = 255
SHADOW_CORE = 0xF9
SHADOW_EDGE = 0xFA


def manifest():
    return layout.sprites()


def seed_of(s):
    return int(hashlib.md5(s.lower().encode()).hexdigest()[:8], 16)


def quantise(frames_rgb, masks, colours=240):
    """One palette for all frames of a sprite: (palette 256 x 3, index frames) with 255 the key."""
    px = np.concatenate([f[m] for f, m in zip(frames_rgb, masks)]) if any(m.any() for m in masks) else np.zeros((1, 3))
    px8 = np.clip(px * 255 + 0.5, 0, 255).astype(np.uint8)
    uniq = np.unique(px8, axis=0)
    if len(uniq) <= colours:
        pal = uniq
    else:
        img = Image.fromarray(px8.reshape(1, -1, 3))
        q = img.quantize(colors=colours, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
        p = np.array(q.getpalette()[:colours * 3], dtype=np.uint8).reshape(-1, 3)
        used = np.unique(np.asarray(q))
        pal = p[used]
    palf = pal.astype(np.float32)
    out = []
    for f, m in zip(frames_rgb, masks):
        idx = np.full(m.shape, KEY, dtype=np.uint8)
        if m.any():
            c = np.clip(f[m] * 255 + 0.5, 0, 255).astype(np.float32)
            d = ((c[:, None, :] - palf[None, :, :]) ** 2).sum(-1)
            idx[m] = d.argmin(1).astype(np.uint8)
        out.append(idx)
    full = np.zeros((256, 3), dtype=np.uint8)
    full[:len(pal)] = pal
    full[KEY] = (255, 0, 255)
    return full, out


def save_sheet(path, frames, w, h, views, fpv, pal, frame=None):
    """An indexed PNG of the frames, a row per view; `frame` ("w h ax ay") goes in a tEXt chunk named "frame" when the
    sprite has a frame of its own (baked.rs reads it)."""
    sheet = np.full((views * h, fpv * w), KEY, dtype=np.uint8)
    for v in range(views):
        for f in range(fpv):
            sheet[v * h:(v + 1) * h, f * w:(f + 1) * w] = frames[v * fpv + f]
    img = Image.fromarray(sheet, mode="P")
    img.putpalette(pal.reshape(-1).tolist())
    os.makedirs(os.path.dirname(path), exist_ok=True)
    info = None
    if frame is not None:
        from PIL import PngImagePlugin
        info = PngImagePlugin.PngInfo()
        info.add_text("frame", frame)
    img.save(path, optimize=True, bits=8, pnginfo=info)


def render_one(args):
    path, spec, shadow_path = args
    w, h, views, fpv, ax, ay = spec
    name = path.rsplit("/", 1)[-1][:-4].lower()
    r = recipes.recipe(path, name)
    if r is None:
        return path, "skip"
    seed = seed_of(path)
    ax, ay = r.anchor(w, h, ax, ay)
    nv = max(views, 1)
    step = 45.0 if nv >= 8 else 90.0
    # the scale: the largest at which every view of the frames the recipe names for fitting fits, with the shadow
    fit_frames = r.fit_frames(fpv)
    scenes = [(r.scene(np.random.default_rng(seed), f, fpv, v * step), r.yaw(v * step)) for v in range(nv) for f in fit_frames]
    fixed = r.fixed_scale()
    if fixed is not None:
        scale = fixed
    else:
        scale = fit_scale(scenes, w, h, ax, ay, shadow=shadow_path is not None) * r.shrink
    frame_note = None
    if r.auto_frame:
        # a frame of our own size around everything the views show (the anchor at most 240 from the left and top: the
        # FLC header keeps the crop of a 480 x 480 canvas), so a building is drawn at the world's scale
        big = 480
        xs0, ys0, xs1, ys1 = [], [], [], []
        for v in range(nv):
            sc = r.scene(np.random.default_rng(seed), 0, fpv, v * step).transformed(scale)
            fr = Frame(big * 2, big * 2, big, big, ss=1)
            sc.render(fr, r.yaw(v * step), shadow=shadow_path is not None)
            m = np.isfinite(fr.depth) | fr.shadow
            if m.any():
                yy, xx = np.nonzero(m)
                xs0.append(xx.min()); xs1.append(xx.max()); ys0.append(yy.min()); ys1.append(yy.max())
        if xs0:
            x0, x1, y0, y1 = min(xs0) - 2, max(xs1) + 3, min(ys0) - 2, max(ys1) + 3
            ax, ay = min(big - x0, 240), min(big - y0, 240)
            w, h = ax + (x1 - big), ay + (y1 - big)
        else:
            w, h, ax, ay = 2, 2, 1, 1
        frame_note = "%d %d %d %d" % (w, h, ax, ay)
    rgb, masks, shadows, ramp = [], [], [], []
    pal_cols = {}
    for v in range(nv):
        for f in range(fpv):
            sc = r.scene(np.random.default_rng(seed), f, fpv, v * step).transformed(scale)
            pal_cols = sc.pal or pal_cols
            fr = Frame(w, h, ax, ay, ss=r.ss)
            sc.render(fr, r.yaw(v * step), shadow=shadow_path is not None)
            col, op, tag, bright, sh = fr.resolve(edge_dark=r.edge_dark)
            rgb.append(col)
            masks.append(op)
            shadows.append(sh)
            if r.mode == "ramp":
                ramp.append(np.where(op, mats.index(np.where(op, tag, -1), bright), KEY).astype(np.uint8))
    if r.mode == "ramp":
        pal, idx = mats.palette(pal_cols), ramp
    else:
        pal, idx = quantise(rgb, masks)
    save_sheet(os.path.join(OUT, path[:-4] + ".png"), idx, w, h, nv, fpv, pal, frame_note)
    if shadow_path is not None:
        spal = np.zeros((256, 3), dtype=np.uint8)
        spal[KEY] = (255, 0, 255)
        spal[SHADOW_CORE] = (0, 160, 0)
        sidx = [np.where(s, SHADOW_CORE, KEY).astype(np.uint8) for s in shadows]
        save_sheet(os.path.join(OUT, shadow_path[:-4] + ".png"), sidx, w, h, nv, fpv, spal, frame_note)
    return path, "ok"


LIST = os.path.join(ROOT, "crates", "sg-core", "src", "freeset", "baked_list.rs")


def write_index():
    """crates/sg-core/src/freeset/baked_list.rs: every file of the art folder, sorted by its lower-case path, built in."""
    files = []
    for d, _, names in os.walk(OUT):
        for n in names:
            full = os.path.join(d, n)
            rel = os.path.relpath(full, OUT).replace(os.sep, "/")
            files.append((rel.lower(), os.path.relpath(full, os.path.dirname(LIST)).replace(os.sep, "/")))
    files.sort()
    lines = ["// Written by tools/freeset_art/build.py: the files of packs/opensgolf-free/art (baked.rs). Do not edit.",
             "pub static FILES: &[(&str, &[u8])] = &["]
    for key, rel in files:
        lines.append(f'    ("{key}", include_bytes!("{rel}")),')
    lines.append("];")
    open(LIST, "w", encoding="utf-8").write("\n".join(lines) + "\n")
    total = sum(os.path.getsize(os.path.join(os.path.dirname(LIST), r)) for _, r in files)
    print(f"{len(files)} files, {total / 1e6:.1f} MB in {os.path.relpath(OUT, ROOT)}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 1)
    ap.add_argument("--index", action="store_true", help="only rewrite baked_list.rs")
    a = ap.parse_args()
    if a.index:
        write_index()
        return
    m = manifest()
    jobs = []
    for path, spec in sorted(m.items()):
        if path.lower().endswith("shadow.flc"):
            continue
        if a.only and not re.search(a.only, path, re.I):
            continue
        name = path.rsplit("/", 1)[-1][:-4].lower()
        if recipes.recipe(path, name) is None:
            continue
        sp = path[:-4] + "Shadow.flc"
        shadow = next((p for p in m if p.lower() == sp.lower()), None)
        if shadow is not None and m[shadow][:2] != spec[:2]:
            shadow = None
        jobs.append((path, spec, shadow))
    if a.list:
        for j in jobs:
            print(j[0], "+ shadow" if j[2] else "")
        print(len(jobs), "sprites")
        return
    with ProcessPoolExecutor(a.jobs) as ex:
        for path, status in ex.map(render_one, jobs):
            print(status, path, flush=True)
    write_index()


if __name__ == "__main__":
    main()
