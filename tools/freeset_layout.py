#!/usr/bin/env python3
"""Measures the layout facts the free base set (opensgolf-free) needs from a SimGolf install, and writes them as
crates/sg-core/src/freeset/manifest.txt.

What it records, for each file the engine reads, is only what the engine needs to find things in the right place:

* pictures: the size, the transparent key colour, and for sprite sheets the boxes the engine cuts (bounding boxes of the
  opaque parts), each with a coarse shape class (rectangle, ellipse, diamond) and a coarse tone class (which of the
  free set's own colours to paint it in: base, highlight, gold, green, grey, paper, dark, red) so that button states stay
  distinguishable;
* sprites (FLC): frame size, number of views, frames per view, the anchor (crop origin in the 480x480 canvas), the
  frame time and the view mask;
* sounds: the length (the free set synthesises its own sound of about that length);
* text files: only their names (and, for the terrain descriptions, the terrain names used as keys).

No pixel, palette entry, sample or sentence of the original is written: the free set's art, sounds and text are drawn,
synthesised and written by crates/sg-core/src/freeset from these numbers alone.

usage: freeset_layout.py GAME_DIR [OUT]   (GAME_DIR: the folder with Interface/, Flics/, Sounds/ ...)
Needs Python 3 with Pillow and numpy.
"""
import os
import re
import struct
import sys

import numpy as np
from PIL import Image

KEYS = [(255, 0, 255), (248, 0, 248), (0x9B, 0xE7, 0xFF), (255, 0, 0), (0, 0, 0)]

# Files the engine reads outside the folders taken whole (see `wanted`).
UI_DIRS = ('interface/', 'heads/', 'bodies/', 'data/')
SKIP_UI = ('interface/loading_screens/', 'heads/head template.pcx', 'interface/pictbckgrnd1.pcx', 'interface/barrel.pcx',
           'interface/starsheartsetc.pcx', 'interface/top10_charms', 'interface/trophymantle.pcx',
           'interface/bulletinboard&mantlecork.pcx', 'interface/infoscreens/general_selectionbox',
           'interface/chooselphabuttons_a.pcx', 'interface/employees_a.pcx', 'interface/3mainlowerleft_a.pcx',
           'data/desert tiles.pcx', 'bodies/barrel')
ROOT_FILES = ('bldg.pcx', 'gbubbles.pcx', 'bink64.pcx', 'cliffs01.pcx', 'creditsbckgrd.pcx', 'logo.pcx')


def wanted(rel):
    low = rel.lower()
    ext = os.path.splitext(low)[1]
    if low.startswith('flics/'):
        return ext in ('.flc', '.pcx')
    if low.startswith('data/textures/'):
        name = os.path.basename(low)
        return (ext == '.tga' or re.fullmatch(r'[a-z0-9]+[a-e]\d{4}\.bmp', name) is not None
                or name in ('strata.bmp', 'clifftest.bmp', 'retainwalla.bmp', 'retainingwalla.bmp'))
    if low.startswith('sounds/') or low.startswith('simsfx/'):
        return ext == '.wav' and not low.startswith('sounds/music/')
    if low.startswith(UI_DIRS):
        if any(low.startswith(s) for s in SKIP_UI) or low.startswith('data/') and '/' in low[5:]:
            return False
        return ext == '.pcx' or (low.startswith('interface/') and ext == '.txt')
    return low in ROOT_FILES


def label(mask):
    """4-connected components of a boolean mask: list of (x, y, w, h, area, pixel mask of the box)."""
    h, w = mask.shape
    lab = np.zeros((h, w), np.int32)
    parent = [0]

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    runs_prev = []
    nxt = 1
    for y in range(h):
        row = mask[y]
        if not row.any():
            runs_prev = []
            continue
        d = np.diff(np.concatenate(([0], row.astype(np.int8), [0])))
        starts = np.nonzero(d == 1)[0]
        ends = np.nonzero(d == -1)[0]
        runs = []
        for s, e in zip(starts, ends):
            ids = [r for (ps, pe, r) in runs_prev if ps < e and pe > s]
            if ids:
                root = find(ids[0])
                for o in ids[1:]:
                    ro = find(o)
                    if ro != root:
                        parent[ro] = root
                rid = root
            else:
                parent.append(nxt)
                rid = nxt
                nxt += 1
            lab[y, s:e] = rid
            runs.append((s, e, rid))
        runs_prev = runs
    roots = np.array([find(i) for i in range(len(parent))], np.int32)
    lab = roots[lab]
    out = []
    for r in np.unique(lab):
        if r == 0:
            continue
        ys, xs = np.nonzero(lab == r)
        x0, x1, y0, y1 = xs.min(), xs.max() + 1, ys.min(), ys.max() + 1
        out.append((int(x0), int(y0), int(x1 - x0), int(y1 - y0), len(xs), lab[y0:y1, x0:x1] == r))
    return out


def erode(m, r):
    out = m.copy()
    h, w = m.shape
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if dx * dx + dy * dy > r * r:
                continue
            sh = np.zeros_like(m)
            sh[max(0, dy):h + min(0, dy), max(0, dx):w + min(0, dx)] = m[max(0, -dy):h + min(0, -dy), max(0, -dx):w + min(0, -dx)]
            out &= sh
    return out


def cut_points(prof, lo):
    """Places along a coverage profile where touching sprites meet: runs below `lo`, cut at their middle."""
    cuts, i, n = [], 0, len(prof)
    while i < n:
        if prof[i] <= lo:
            j = i
            while j < n and prof[j] <= lo:
                j += 1
            if i > 6 and j < n - 6:
                cuts.append((i + j) // 2)
            i = j
        else:
            i += 1
    return cuts


def xy_split(x0, y0, m, depth=0):
    """Splits a part along columns, then rows, where only a thin bridge joins its pieces (grids of touching buttons)."""
    h, w = m.shape
    if depth > 6 or w * h < 400:
        return None
    for axis in (0, 1):
        prof = m.sum(axis=axis)
        lo = max(2, int(prof.max() * 0.22))
        cuts = cut_points(prof, lo)
        if cuts:
            edges = [0] + cuts + [len(prof)]
            out = []
            for a, b in zip(edges, edges[1:]):
                sub = m[:, a:b] if axis == 0 else m[a:b, :]
                if not sub.any():
                    continue
                ys, xs = np.nonzero(sub)
                sx0, sy0 = (a, 0) if axis == 0 else (0, a)
                tx0, ty0 = sx0 + xs.min(), sy0 + ys.min()
                tight = m[ty0:sy0 + ys.max() + 1, tx0:sx0 + xs.max() + 1]
                if axis == 0:
                    tight = tight & np.pad(np.ones((1, 1), bool), 0)
                inner = xy_split(x0 + tx0, y0 + ty0, sub[ys.min():ys.max() + 1, xs.min():xs.max() + 1], depth + 1)
                out.extend(inner or [(x0 + tx0, y0 + ty0, x0 + sx0 + xs.max() + 1, y0 + sy0 + ys.max() + 1)])
            return out
    return None


def period(prof):
    """The repeat distance of a coverage profile (a row or column of equal buttons), or 0."""
    n = len(prof)
    p = prof - prof.mean()
    if n < 40 or not p.any():
        return 0
    ac = np.correlate(p, p, 'full')[n - 1:] / (np.dot(p, p) + 1e-9)
    best, lag = 0.0, 0
    for k in range(16, n // 2 + 1):
        if ac[k] > best and ac[k] >= ac[k - 1] and (k + 1 >= len(ac) or ac[k] >= ac[k + 1]):
            best, lag = ac[k], k
            break
    return lag if best > 0.45 else 0


def grid_split(x0, y0, m):
    """A part made of equal buttons side by side (a row or grid of them touching): cut it at its repeat distance."""
    h, w = m.shape
    if m.mean() > 0.93 or w * h < 3000:
        return None
    px, py = period(m.sum(axis=0).astype(float)), period(m.sum(axis=1).astype(float))
    if not px and not py:
        return None
    xs = list(range(0, w, px)) + [w] if px else [0, w]
    ys = list(range(0, h, py)) + [h] if py else [0, h]
    if xs[-2] > w - 8 and len(xs) > 2:
        xs.pop(-2)
    if ys[-2] > h - 8 and len(ys) > 2:
        ys.pop(-2)
    out = []
    for a, b in zip(ys, ys[1:]):
        for c, d in zip(xs, xs[1:]):
            sub = m[a:b, c:d]
            if sub.sum() < 12:
                continue
            yy, xx = np.nonzero(sub)
            out.append((x0 + c + xx.min(), y0 + a + yy.min(), x0 + c + xx.max() + 1, y0 + a + yy.max() + 1))
    return out if len(out) > 1 else None


def split(x0, y0, m, depth=0):
    h, w = m.shape
    if m.mean() > 0.6 and w * h > 15000:
        return [(x0, y0, x0 + w, y0 + h)]  # a solid panel: the buttons on it are drawn by the engine from other cuts
    parts = xy_split(x0, y0, m)
    if parts:
        out = []
        for (a, b, c, d) in parts:
            sub = m[b - y0:d - y0, a - x0:c - x0]
            out.extend(grid_split(a, b, sub) or [(a, b, c, d)])
        return out
    return grid_split(x0, y0, m) or erode_split(x0, y0, m, depth)


def erode_split(x0, y0, m, depth=0):
    """Sprites drawn touching each other (button grids) come out of the labelling as one part: erode the part step by step
    until it falls into pieces of similar size, and keep each piece's box grown back by the erosion."""
    h, w = m.shape
    area = int(m.sum())
    if depth < 3 and w * h > 400 and min(w, h) > 12:
        for r in range(1, 7):
            e = erode(m, r)
            parts = [c for c in label(e) if c[4] >= max(12, area // 40)]
            if len(parts) >= 2:
                out = []
                for (px, py, pw, ph, _, pm) in parts:
                    bx0, by0 = max(0, px - r), max(0, py - r)
                    bx1, by1 = min(w, px + pw + r), min(h, py + ph + r)
                    sub = m[by0:by1, bx0:bx1] & dilate_box(pm, px - bx0, py - by0, r, by1 - by0, bx1 - bx0)
                    out.extend(erode_split(x0 + bx0, y0 + by0, sub, depth + 1))
                return out
            if not e.any():
                break
    return [(x0, y0, x0 + w, y0 + h)]


def dilate_box(pm, ox, oy, r, h, w):
    """The piece's pixels grown by r, on an h x w canvas where the piece's box starts at (ox, oy)."""
    canvas = np.zeros((h, w), bool)
    ph, pw = pm.shape
    canvas[oy:oy + ph, ox:ox + pw] = pm
    out = canvas.copy()
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            sh = np.zeros_like(canvas)
            sh[max(0, dy):h + min(0, dy), max(0, dx):w + min(0, dx)] = canvas[max(0, -dy):h + min(0, -dy), max(0, -dx):w + min(0, -dx)]
            out |= sh
    return out


def merge_close(comps):
    """Joins components whose boxes overlap (one sprite drawn in several pieces)."""
    boxes = [[c[0], c[1], c[0] + c[2], c[1] + c[3], c[4]] for c in comps]
    changed = True
    while changed:
        changed = False
        for i in range(len(boxes)):
            for j in range(i + 1, len(boxes)):
                a, b = boxes[i], boxes[j]
                if a and b and a[0] < b[2] and b[0] < a[2] and a[1] < b[3] and b[1] < a[3]:
                    boxes[i] = [min(a[0], b[0]), min(a[1], b[1]), max(a[2], b[2]), max(a[3], b[3]), a[4] + b[4]]
                    boxes[j] = None
                    changed = True
        boxes = [b for b in boxes if b]
    return boxes


def shape_class(m):
    h, w = m.shape
    fill = m.mean()
    if w < 4 or h < 4:
        return 'R'
    cy, cx = h // 2, w // 2
    k = max(1, min(w, h) // 10)
    corners = [m[:k, :k].mean(), m[:k, -k:].mean(), m[-k:, :k].mean(), m[-k:, -k:].mean()]
    if min(corners) > 0.5 or fill > 0.9:
        return 'R'
    if fill < 0.62 and m[cy, :k].any() and m[cy, -k:].any() and m[:k, cx].any():
        return 'D'
    if fill < 0.88:
        return 'E'
    return 'O'  # rounded rectangle


def tone_class(rgb):
    r, g, b = [float(v) / 255 for v in rgb]
    mx, mn = max(r, g, b), min(r, g, b)
    light = (mx + mn) / 2
    sat = 0 if mx == mn else (mx - mn) / (1 - abs(2 * light - 1) + 1e-6)
    lv = min(3, int(light * 4))
    if sat < 0.18:
        if light > 0.78:
            return 'w%d' % lv
        if light < 0.25:
            return 'k%d' % lv
        return 's%d' % lv
    if mx == r:
        hue = (60 * ((g - b) / (mx - mn))) % 360
    elif mx == g:
        hue = 60 * ((b - r) / (mx - mn)) + 120
    else:
        hue = 60 * ((r - g) / (mx - mn)) + 240
    if hue < 20 or hue >= 330:
        c = 'r'
    elif hue < 70:
        c = 'g'
    elif hue < 165:
        c = 'e'
    else:
        c = 'b'
    return '%s%d' % (c, lv)


def flat_regions(a):
    """A whole-screen picture: the boxes of its large even areas (the fields and panels the engine writes into), by tone
    class, biggest first. Textured parts (clouds, wood) give nothing."""
    f = a.astype(np.float32) / 255
    mx, mn = f.max(axis=2), f.min(axis=2)
    light = (mx + mn) / 2
    sat = np.where(mx > mn, (mx - mn) / (1 - np.abs(2 * light - 1) + 1e-6), 0)
    # even: a pixel close to its right and lower neighbours
    d = np.zeros(light.shape, bool)
    d[:-1, :-1] = (np.abs(a[:-1, :-1] - a[1:, :-1]).sum(axis=2) < 24) & (np.abs(a[:-1, :-1] - a[:-1, 1:]).sum(axis=2) < 24)
    classes = {
        'w': (light > 0.78),
        'k': light < 0.18,
        'b': (light >= 0.18) & (light <= 0.8) & (sat >= 0.15),
        's': (light >= 0.18) & (light <= 0.8) & (sat < 0.15),
    }
    out = []
    for cls, m in classes.items():
        m = m & d
        for (x0, y0, w0, h0, area, cm) in label(m):
            if area < 500 or area < 0.45 * w0 * h0 or w0 < 12 or h0 < 8:
                continue
            rgb = a[y0:y0 + h0, x0:x0 + w0][cm].mean(axis=0)
            out.append((area, '  %d %d %d %d %s %s' % (x0, y0, w0, h0, shape_class(cm), cls + tone_class(rgb)[1])))
    out.sort(key=lambda t: -t[0])
    return [l for _, l in out[:120]]


def pcx_size(d):
    xmin, ymin, xmax, ymax = struct.unpack_from('<HHHH', d, 4)
    return xmax - xmin + 1, ymax - ymin + 1


def key_of(a):
    best, n = None, 0
    for k in KEYS[:4]:
        c = int(((a[:, :, 0] == k[0]) & (a[:, :, 1] == k[1]) & (a[:, :, 2] == k[2])).sum())
        if c > n:
            best, n = k, c
    if best and n > a.shape[0] * a.shape[1] * 0.01:
        return best
    return None


def picture(game, rel, files_low):
    p = os.path.join(game, rel)
    im = Image.open(p).convert('RGB')
    a = np.asarray(im).astype(np.int32)
    h, w = a.shape[:2]
    low = rel.lower()
    stem, ext = os.path.splitext(rel)
    key = key_of(a)
    is_alpha = low.endswith('_a.pcx') or low.endswith('_alpha.pcx')
    mask = None
    if is_alpha:
        return 'A %s %d %d' % (rel, w, h), []
    if key:
        mask = ~((a[:, :, 0] == key[0]) & (a[:, :, 1] == key[1]) & (a[:, :, 2] == key[2]))
    else:
        for suf in ('_A.pcx', '_alpha.pcx'):
            cand = (stem + suf).lower()
            if cand in files_low:
                al = np.asarray(Image.open(os.path.join(game, files_low[cand])).convert('L'))
                if al.shape == (h, w):
                    mask = al > 16
                break
        if low.endswith('sga.pcx'):
            cand = 'interface/infoscreens/sgareport_alpha.pcx'
            al = np.asarray(Image.open(os.path.join(game, files_low[cand])).convert('L'))
            mask = al > 16
    keystr = '%02x%02x%02x' % key if key else '-'
    head = 'P %s %d %d %s' % (rel, w, h, keystr)
    if mask is None or mask.all():
        regions = flat_regions(a)
        return head + ' full %d' % len(regions), regions
    boxes = []
    for (x0, y0, w0, h0, area, m) in label(mask):
        if area >= 6:
            boxes.extend(split(x0, y0, m))
    lines = []
    for (x0, y0, x1, y1) in boxes:
        m = mask[y0:y1, x0:x1]
        sel = m
        rgb = a[y0:y1, x0:x1][sel].mean(axis=0) if sel.any() else (128, 128, 128)
        lines.append('  %d %d %d %d %s %s' % (x0, y0, x1 - x0, y1 - y0, shape_class(m), tone_class(rgb)))
    lines.sort(key=lambda l: (int(l.split()[1]), int(l.split()[0])))
    return head + ' cut %d' % len(lines), lines


def flc(game, rel):
    d = open(os.path.join(game, rel), 'rb').read(128)
    _, _, frames, w, h, _, _, speed = struct.unpack_from('<IHHHHHHI', d, 0)
    views, fpv, cx, cy, cw, ch, ms, _, mask = struct.unpack_from('<HHHHHHHHH', d, 96)
    if not (1 <= views <= 8 and fpv >= 1 and views * fpv == frames and cw == 480 and ch == 480):
        views, fpv, cx, cy, mask = 0, frames, 0, 0, 0  # no Firaxis extension: a plain animation
    return 'F %s %d %d %d %d %d %d %d %d %d' % (rel, w, h, views, fpv, cx, cy, ms, mask, speed)


def wav(game, rel):
    d = open(os.path.join(game, rel), 'rb').read()
    secs = 1.0
    if d[:4] == b'RIFF':
        p, fmt, n = 12, None, 0
        while p + 8 <= len(d):
            cid, sz = d[p:p + 4], struct.unpack_from('<I', d, p + 4)[0]
            if cid == b'fmt ':
                fmt = struct.unpack_from('<HHIIHH', d, p + 8)
            elif cid == b'data':
                n = min(sz, len(d) - p - 8)
            p += 8 + sz + (sz & 1)
        if fmt:
            secs = n / max(1, fmt[1] * fmt[2] * fmt[5] // 8)
    return 'W %s %.2f' % (rel, secs)


def texture(game, rel):
    low = rel.lower()
    if low.endswith('.tga'):
        d = open(os.path.join(game, rel), 'rb').read(18)
        w, h = struct.unpack_from('<HH', d, 12)
        return 'G %s %d %d' % (rel, w, h)
    return 'T %s' % rel


def main():
    game = sys.argv[1]
    here = os.path.dirname(os.path.abspath(__file__))
    out = sys.argv[2] if len(sys.argv) > 2 else os.path.join(here, '..', 'crates', 'sg-core', 'src', 'freeset', 'manifest.txt')
    rels = []
    for root, _, files in os.walk(game):
        for f in files:
            rels.append(os.path.relpath(os.path.join(root, f), game).replace(os.sep, '/'))
    files_low = {r.lower(): r for r in rels}
    lines = ['# opensgolf-free layout manifest: written by tools/freeset_layout.py (sizes, cut boxes, sprite and sound',
             '# timings measured from a SimGolf install; no pixels, samples or text). Kinds: P picture (w h key comps),',
             '# A alpha sheet of the picture without _A/_alpha, F sprite (w h views frames/view cx cy ms mask speed),',
             '# T terrain texture (64x64 24-bit BMP), G overlay texture (w h), W sound (seconds), K terrain text keys.']
    for rel in sorted(rels, key=str.lower):
        if not wanted(rel):
            continue
        low = rel.lower()
        ext = os.path.splitext(low)[1]
        try:
            if ext == '.flc':
                lines.append(flc(game, rel))
            elif ext == '.wav':
                lines.append(wav(game, rel))
            elif low.startswith('data/textures/'):
                lines.append(texture(game, rel))
            elif ext == '.txt':
                keys = [l[1:].strip() for l in open(os.path.join(game, rel), encoding='latin-1').read().splitlines()
                        if l.startswith('*')]
                lines.append('K %s %s' % (rel, '|'.join(keys)))
            elif ext == '.pcx':
                if low.startswith('flics/') and not low.endswith('holemark.pcx') or low.startswith('bodies/') and 'swap' in low:
                    w, h = pcx_size(open(os.path.join(game, rel), 'rb').read(128))
                    lines.append('L %s %d %d' % (rel, w, h))
                else:
                    head, comps = picture(game, rel, files_low)
                    lines.append(head)
                    lines.extend(comps)
        except Exception as e:  # a broken original file: leave it out
            print('skip', rel, e, file=sys.stderr)
    with open(out, 'w', newline='\n') as f:
        f.write('\n'.join(lines) + '\n')
    print(len(lines), 'lines ->', out)


if __name__ == '__main__':
    main()
