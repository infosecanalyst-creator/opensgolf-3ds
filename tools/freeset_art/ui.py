#!/usr/bin/env python3
"""The free set's main interface pictures, drawn as vector art (uiart.py) at the places the engine cuts them (the cut
positions are in the engine: crates/simgolf/src/render.rs DOCK*, hud_ui.rs PLATES, panels_ui.rs TERRAIN_*, the title's
MENU_BTN; the boxes of the other pieces come from the layout manifest). Written to packs/opensgolf-free/art/gfx/... as PNG;
the engine draws the rest of the interface itself (crates/sg-core/src/freeset/ui.rs).

usage: ui.py [--only REGEX]"""
import argparse
import math
import os
import re
import sys

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import layout  # noqa: E402
import trees  # noqa: E402
from raster import Frame  # noqa: E402
from uiart import (S, BLUE, CREAM, GOLD, GOLD_HI, GREEN, GREEN_HI, GREY, INK, NAVY, SKY, Sheet, g_ball_tee, g_cards,  # noqa: E402
                   g_dollar, g_exit, g_flag, g_folder, g_golfer, g_house, g_info, g_minus, g_pause, g_people, g_plus,
                   g_shovel, g_smile, g_trophy, g_turn, g_wrench, glossy_disc, lighten, mix, oval, pill, rgb)

OUT = os.path.join(layout.ROOT, "packs", "opensgolf-free", "art", "gfx")


def save(rel, sheet, key=None, alpha_rel=None, cut=0.5):
    """Writes the picture (keyed: RGB with the key colour where clear; else RGBA flattened on its own colour) and its alpha
    sheet if asked, as 8-bit palette PNGs when that is lossless enough."""
    path = os.path.join(OUT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if key is not None:
        rgb_, a = sheet.keyed(key, cut)
        img = Image.fromarray(rgb_, "RGB")
    else:
        im = sheet.image()
        img = Image.fromarray(im[..., :3], "RGB")
        a = im[..., 3]
    q = img.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.FLOYDSTEINBERG)
    q.save(path, optimize=True, bits=8)
    if alpha_rel is not None:
        Image.fromarray(a, "L").convert("RGB").quantize(colors=64, dither=Image.Dither.NONE).save(os.path.join(OUT, alpha_rel), bits=8,
                                                                                                optimize=True)


# ---- the dock (Interface/3mainLowerLeft.pcx) ---------------------------------------------------------------------------------

DOCK = [(43, 473, 33), (117, 497, 30), (177, 536, 26), (32, 543, 11), (31, 585, 11), (17, 565, 11), (47, 565, 11),
        (75, 552, 14), (107, 568, 14), (133, 583, 13)]
DOCK_LIT_AT = [(7, 436), (84, 463), (146, 503), (19, 530), (19, 573), (4, 551), (33, 552), (59, 535), (94, 553), (121, 570)]
DOCK_LIT_CUT = [(0, 0), (0, 100), (0, 200), (600, 50), (600, 100), (600, 150), (600, 200), (600, 250), (600, 300), (600, 350)]
DOCK_GLYPHS = [g_flag, g_house, g_people, g_plus, g_minus, g_turn(1), g_turn(-1), g_info, g_pause, g_wrench]
DOCK_KEY = (248, 0, 248)


def dock():
    sh = Sheet(800, 600)
    # the dock's base: a quarter disc in the corner with a raised rim and a groove
    base = sh.mask_ellipse(-212, 446, 424, 330)
    sh.shadow(base, 2, -1, 4, 0.5)
    sh.paint(base, sh.rgrad(rgb("#5b7fa6"), rgb("#2b4664"), 40, 520, 230))
    inner = sh.mask_ellipse(-198, 460, 396, 302)
    sh.paint(base - inner, rgb("#9cc0e0"), 0.55)
    groove = sh.mask_ellipse(-150, 498, 300, 220) - sh.mask_ellipse(-146, 502, 292, 212)
    sh.paint(np.clip(groove, 0, 1), rgb("#1c3049"), 0.6)
    for k, (cx, cy, r) in enumerate(DOCK):
        glossy_disc(sh, cx, cy, r, BLUE if k < 3 else rgb("#4c86c0"), DOCK_GLYPHS[k])
    # the lit (bright blue) and selected (gold) looks at their cuts, the same button drawn about the same centre
    for k, (cx, cy, r) in enumerate(DOCK):
        ax, ay = DOCK_LIT_AT[k]
        sx, sy = DOCK_LIT_CUT[k]
        ox, oy = cx - ax, cy - ay
        glossy_disc(sh, sx + ox, sy + oy, r + 1, rgb("#6fb3ee"), DOCK_GLYPHS[k], state="lit")
        gold_dx = 100 if k < 3 else 50
        glossy_disc(sh, sx + gold_dx + ox, sy + oy, r + 1, GOLD, DOCK_GLYPHS[k], glyph_col=INK)
    # the gold trails from a button to its panel, cut (300, 100 * mode, 214, 91)
    for mode, (ex, ey) in enumerate([(140, 84), (110, 70), (60, 56)]):
        y0 = 100 * mode
        pts = [(300 + 4 + t * (ex - 4) - math.sin(t * math.pi) * 10, y0 + 4 + (ey - 4) * (t ** 1.6)) for t in np.linspace(0, 1, 30)]
        pts += [(300 + ex + 4, y0 + ey), (300 + 212, y0 + ey)]
        m = sh.mask_line(pts, 4.5)
        sh.glow(m, 2, 0.6, GOLD_HI)
        sh.paint(m, GOLD)
        sh.paint(sh.mask_line(pts, 1.5), GOLD_HI, 0.9)
    save("Interface/3mainLowerLeft.png", sh, key=DOCK_KEY)


# ---- the course badge and the rating pills (Interface/courseinfo.pcx and its alpha and shadow sheets) -----------------------

PLATES = [(48, 6, 183, 58), (647, 13, 138, 36), (675, 55, 110, 37), (697, 99, 88, 36)]


def plate_mask(sh, k):
    x, y, w, h = PLATES[k]
    if k == 0:
        return sh.mask_rrect(x, y, w, h, 20)
    return sh.mask_rrect(x, y + 4, w - 12, h - 8, (h - 8) / 2)


def courseinfo():
    sh = Sheet(800, 600)
    glass_top, glass_bot = rgb("#33536f"), rgb("#15293b")
    m = plate_mask(sh, 0)
    sh.paint(m, rgb("#a8c8e4"))
    x, y, w, h = PLATES[0]
    sh.paint(sh.mask_rrect(x + 2, y + 2, w - 4, h - 4, 18), sh.vgrad(glass_top, glass_bot, y, y + h))
    sh.paint(sh.mask_rrect(x + 30, y + 4, w - 40, 10, 5), (1, 1, 1), 0.12)
    icons = [(g_dollar, rgb("#3f9e57")), (g_smile, rgb("#e8b52c")), (g_golfer, rgb("#3a9fcf"))]
    for k in (1, 2, 3):
        x, y, w, h = PLATES[k]
        pm = plate_mask(sh, k)
        sh.paint(pm, rgb("#a8c8e4"))
        sh.paint(sh.mask_rrect(x + 2, y + 6, w - 16, h - 12, (h - 12) / 2), sh.vgrad(glass_top, glass_bot, y, y + h))
        glyph, col = icons[k - 1]
        glossy_disc(sh, x + w - 15, y + h / 2, 15, col, glyph, glyph_col=rgb("#fffdf0"), shadow=True)
    # the emblem circles of the sheet (the club emblems the engine uses are in parklink.pcx and tropdesert.pcx)
    rgb_, a = sheet_out(sh)
    save_pair("Interface/courseinfo.png", "Interface/courseinfo_A.png", rgb_, a, (255, 0, 255))
    # the shadow sheet: where the plates darken the course
    s2 = Sheet(800, 600)
    for k in range(4):
        x, y, w, h = PLATES[k]
        s2.paint(plate_mask(s2, k), (0, 1, 0))
    save("Interface/s_courseinfo.png", s2, key=(255, 0, 255))


def sheet_out(sh):
    im = sh.image()
    return im[..., :3], im[..., 3]


def save_pair(rel, rel_a, rgb_, a, key):
    """A picture and its alpha sheet: the picture keeps its colour wherever the alpha sheet lets any of it through."""
    out = rgb_.copy()
    out[a < 8] = key
    path = os.path.join(OUT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    Image.fromarray(out, "RGB").quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.FLOYDSTEINBERG).save(
        path, optimize=True, bits=8)
    Image.fromarray(a, "L").convert("RGB").quantize(colors=64, dither=Image.Dither.NONE).save(os.path.join(OUT, rel_a), bits=8,
                                                                                            optimize=True)


# ---- club emblems (Interface/parklink.pcx, Interface/tropdesert.pcx) ------------------------------------------------------------

def emblem(main, CX, CY, r, theme, k):
    """A round club emblem: a rim and a small scene. theme: park, links, trop, desert; k 0..3 picks the scene."""
    size = int(2 * r + 8)
    sh = Sheet(size, size)
    cx = cy = size / 2
    rim_col = {"park": rgb("#2f6b3a"), "links": rgb("#3b4a63"), "trop": rgb("#1f7f8c"), "desert": rgb("#8a4a24")}[theme]
    outer = sh.mask_circle(cx, cy, r)
    sh.shadow(outer, 1, 2, 2, 0.4)
    sh.paint(outer, rgb("#f3e6c0"))
    sh.paint(sh.mask_circle(cx, cy, r * 0.94), rim_col)
    disc = sh.mask_circle(cx, cy, r * 0.84)
    skies = {"park": ("#bfe3f5", "#e8f6ff"), "links": ("#9db6cc", "#dbe6ee"), "trop": ("#5fd0e6", "#fff1b8"),
             "desert": ("#f0a050", "#ffe2a0")}
    top, bot = skies[theme]
    if theme == "desert" and k == 0:
        top, bot = "#1d2a5a", "#5a4a8a"
    sh.paint(disc, sh.vgrad(rgb(top), rgb(bot), cy - r, cy + r * 0.3))
    ground_y = cy + r * 0.25

    def clip(m):
        return m * disc

    if theme == "park":
        sh.paint(clip(sh.mask_ellipse(cx - r * 1.4, ground_y - r * 0.15, r * 2.2, r * 1.2)), rgb("#6cb24e"))
        sh.paint(clip(sh.mask_ellipse(cx - r * 0.4, ground_y - r * 0.05, r * 1.9, r * 1.2)), rgb("#4e9a3c"))
        if k == 0:
            sh.paint(clip(sh.mask_rrect(cx - r * 0.07, cy - r * 0.15, r * 0.14, r * 0.45, r * 0.03)), rgb("#6b4a2e"))
            for dx, dy, rr in ((-0.25, -0.3, 0.3), (0.2, -0.32, 0.32), (0, -0.55, 0.32), (-0.05, -0.2, 0.28)):
                sh.paint(clip(sh.mask_circle(cx + dx * r, cy + dy * r, rr * r)), rgb("#2f7d32"))
            sh.paint(clip(sh.mask_circle(cx - r * 0.1, cy - r * 0.62, r * 0.14)), rgb("#4fa34a"))
        elif k == 1:
            sh.paint(clip(sh.mask_ellipse(cx - r * 0.7, ground_y - r * 0.05, r * 1.4, r * 0.35)), rgb("#4a9fd0"))
            sh.paint(clip(sh.mask_ellipse(cx - r * 0.2, ground_y - r * 0.12, r * 0.36, r * 0.18)), rgb("#ffffff"))
            sh.paint(clip(sh.mask_circle(cx + r * 0.12, ground_y - r * 0.18, r * 0.09)), rgb("#2f7d32"))
            sh.paint(clip(sh.mask_poly([(cx + r * 0.2, ground_y - r * 0.2), (cx + r * 0.32, ground_y - r * 0.15),
                                        (cx + r * 0.2, ground_y - r * 0.13)])), rgb("#f2b234"))
        elif k == 2:
            pts = []
            for i in range(11):
                a = -math.pi / 2 + 2 * math.pi * i / 10
                rr = r * (0.62 if i % 2 == 0 else 0.3)
                pts.append((cx + rr * math.cos(a), cy - r * 0.1 + rr * math.sin(a)))
            sh.paint(clip(sh.mask_poly(pts)), rgb("#d2502c"))
            sh.paint(clip(sh.mask_rrect(cx - r * 0.03, cy, r * 0.06, r * 0.6, r * 0.02)), rgb("#8a3a20"))
        else:
            sh.paint(clip(sh.mask_poly([(cx - r, ground_y), (cx - r * 0.3, cy - r * 0.45), (cx + r * 0.2, ground_y)])), rgb("#6f7fa0"))
            sh.paint(clip(sh.mask_poly([(cx - r * 0.2, ground_y), (cx + r * 0.4, cy - r * 0.3), (cx + r, ground_y)])), rgb("#56688c"))
            sh.paint(clip(sh.mask_circle(cx + r * 0.45, cy - r * 0.45, r * 0.16)), rgb("#ffd25a"))
        # a flag on every parkland emblem
        sh.paint(clip(sh.mask_rrect(cx + r * 0.45, ground_y - r * 0.5, r * 0.04, r * 0.5, r * 0.01)), rgb("#ffffff"))
        sh.paint(clip(sh.mask_poly([(cx + r * 0.49, ground_y - r * 0.5), (cx + r * 0.72, ground_y - r * 0.42),
                                    (cx + r * 0.49, ground_y - r * 0.34)])), rgb("#e0403a"))
    elif theme == "links":
        sh.paint(clip(sh.mask_rrect(cx - r, ground_y + r * 0.05, r * 2, r, 0)), rgb("#3a6fa0"))
        sh.paint(clip(sh.mask_ellipse(cx - r * 1.2, ground_y - r * 0.12, r * 1.8, r * 0.6)), rgb("#7aa65a"))
        if k == 0:
            sh.paint(clip(sh.mask_rrect(cx - r * 0.3, cy - r * 0.5, r * 0.6, r * 0.85, 0)), rgb("#9a9a92"))
            for i in range(3):
                sh.paint(clip(sh.mask_rrect(cx - r * 0.3 + i * r * 0.22, cy - r * 0.6, r * 0.14, r * 0.14, 0)), rgb("#9a9a92"))
            sh.paint(clip(sh.mask_rrect(cx - r * 0.07, cy + r * 0.05, r * 0.14, r * 0.3, r * 0.06)), rgb("#3b3b38"))
        elif k == 1:
            sh.paint(clip(sh.mask_rrect(cx - r * 0.03, cy - r * 0.1, r * 0.06, r * 0.55, 0)), rgb("#3f7a3a"))
            sh.paint(clip(sh.mask_ellipse(cx - r * 0.22, cy - r * 0.2, r * 0.44, r * 0.3)), rgb("#4f9a48"))
            sh.paint(clip(sh.mask_ellipse(cx - r * 0.17, cy - r * 0.5, r * 0.34, r * 0.38)), rgb("#9b4fb0"))
        elif k == 2:
            shield = sh.mask_poly([(cx - r * 0.45, cy - r * 0.55), (cx + r * 0.45, cy - r * 0.55), (cx + r * 0.45, cy),
                                   (cx, cy + r * 0.5), (cx - r * 0.45, cy)])
            sh.paint(clip(shield), rgb("#b8343c"))
            sh.paint(clip(shield * sh.mask_rrect(cx - r * 0.45, cy - r * 0.55, r * 0.45, r * 0.5, 0)), rgb("#e8d9a8"))
            sh.paint(clip(sh.mask_circle(cx + r * 0.2, cy - r * 0.05, r * 0.12)), (1, 1, 1))
        else:
            sh.paint(clip(sh.mask_poly([(cx - r * 0.14, ground_y), (cx - r * 0.08, cy - r * 0.55), (cx + r * 0.08, cy - r * 0.55),
                                        (cx + r * 0.14, ground_y)])), rgb("#f4f0e6"))
            for i in range(3):
                yy = cy - r * 0.4 + i * r * 0.25
                sh.paint(clip(sh.mask_rrect(cx - r * 0.13, yy, r * 0.26, r * 0.1, 0) * sh.mask_poly(
                    [(cx - r * 0.14, ground_y), (cx - r * 0.08, cy - r * 0.55), (cx + r * 0.08, cy - r * 0.55), (cx + r * 0.14, ground_y)])),
                    rgb("#c8343a"))
            sh.paint(clip(sh.mask_rrect(cx - r * 0.12, cy - r * 0.72, r * 0.24, r * 0.18, r * 0.04)), rgb("#ffd25a"))
    elif theme == "trop":
        sh.paint(clip(sh.mask_rrect(cx - r, ground_y, r * 2, r, 0)), rgb("#2aa3c8"))
        sh.paint(clip(sh.mask_ellipse(cx - r * 0.6, ground_y - r * 0.12, r * 1.2, r * 0.35)), rgb("#f4e2a6"))
        if k == 0:
            trunk = [(cx + r * 0.05 + 0.15 * r * t * t, ground_y - r * 0.05 - r * 0.75 * t) for t in np.linspace(0, 1, 12)]
            sh.paint(clip(sh.mask_line(trunk, r * 0.09)), rgb("#8a6a3e"))
            tx, ty = trunk[-1]
            for a in (-2.6, -2.0, -1.2, -0.5, 0.2):
                end = (tx + math.cos(a) * r * 0.5, ty + math.sin(a) * r * 0.3 + r * 0.15)
                sh.paint(clip(sh.mask_line([(tx, ty), ((tx + end[0]) / 2, ty - r * 0.12), end], r * 0.12)), rgb("#2f8f3a"))
            sh.paint(clip(sh.mask_circle(cx - r * 0.45, cy - r * 0.45, r * 0.15)), rgb("#fff3a0"))
        elif k == 1:
            body = [(cx - r * 0.55 + r * 1.1 * t, cy + r * 0.05 - math.sin(t * math.pi) * r * 0.45) for t in np.linspace(0, 1, 16)]
            sh.paint(clip(sh.mask_line(body, r * 0.22)), rgb("#5b8fb8"))
            sh.paint(clip(sh.mask_poly([(cx + r * 0.0, cy - r * 0.35), (cx - r * 0.12, cy - r * 0.6), (cx + r * 0.14, cy - r * 0.38)])),
                     rgb("#5b8fb8"))
            sh.paint(clip(sh.mask_circle(cx + r * 0.62, cy - r * 0.05, r * 0.11)), (1, 1, 1))
        elif k == 2:
            for i in range(5):
                a = -math.pi / 2 + 2 * math.pi * i / 5
                sh.paint(clip(sh.mask_circle(cx + math.cos(a) * r * 0.28, cy - r * 0.15 + math.sin(a) * r * 0.28, r * 0.24)),
                         rgb("#e84a6a"))
            sh.paint(clip(sh.mask_circle(cx, cy - r * 0.15, r * 0.1)), rgb("#ffd25a"))
        else:
            sh.paint(clip(sh.mask_poly([(cx - r * 0.7, ground_y), (cx - r * 0.15, cy - r * 0.45), (cx + r * 0.15, cy - r * 0.45),
                                        (cx + r * 0.7, ground_y)])), rgb("#6a5a4e"))
            sh.paint(clip(sh.mask_ellipse(cx - r * 0.25, cy - r * 0.85, r * 0.5, r * 0.4)), rgb("#d8d4cc"), 0.8)
            sh.paint(clip(sh.mask_poly([(cx - r * 0.15, cy - r * 0.45), (cx + r * 0.15, cy - r * 0.45), (cx + r * 0.05, cy - r * 0.2),
                                        (cx - r * 0.08, cy - r * 0.25)])), rgb("#e8582c"))
    else:
        sh.paint(clip(sh.mask_rrect(cx - r, ground_y, r * 2, r, 0)), rgb("#d9a066"))
        if k == 0:
            sh.paint(clip(sh.mask_circle(cx + r * 0.35, cy - r * 0.45, r * 0.2)), rgb("#f6f0c8"))
            sh.paint(clip(sh.mask_rrect(cx - r * 0.12, cy - r * 0.45, r * 0.24, r * 0.75, r * 0.12)), rgb("#3f8a4a"))
            sh.paint(clip(sh.mask_rrect(cx - r * 0.42, cy - r * 0.2, r * 0.18, r * 0.3, r * 0.09)), rgb("#3f8a4a"))
            sh.paint(clip(sh.mask_rrect(cx - r * 0.42, cy + r * 0.0, r * 0.4, r * 0.12, r * 0.06)), rgb("#3f8a4a"))
            sh.paint(clip(sh.mask_rrect(cx + r * 0.22, cy - r * 0.3, r * 0.18, r * 0.35, r * 0.09)), rgb("#3f8a4a"))
            sh.paint(clip(sh.mask_rrect(cx + r * 0.02, cy - r * 0.05, r * 0.36, r * 0.12, r * 0.06)), rgb("#3f8a4a"))
        elif k == 1:
            sh.paint(clip(sh.mask_circle(cx, cy - r * 0.1, r * 0.35)), rgb("#ffe27a"))
            sh.paint(clip(sh.mask_poly([(cx - r, ground_y), (cx - r * 0.6, cy - r * 0.15), (cx - r * 0.15, cy - r * 0.15),
                                        (cx + r * 0.05, ground_y)])), rgb("#b4562e"))
            sh.paint(clip(sh.mask_poly([(cx + r * 0.1, ground_y), (cx + r * 0.3, cy), (cx + r * 0.75, cy), (cx + r, ground_y)])),
                     rgb("#c8683a"))
        elif k == 2:
            sh.paint(clip(sh.mask_circle(cx - r * 0.3, cy - r * 0.35, r * 0.22)), rgb("#fff2b0"))
            road = sh.mask_poly([(cx - r * 0.05, ground_y), (cx + r * 0.05, ground_y), (cx + r * 0.5, cy + r), (cx - r * 0.5, cy + r)])
            sh.paint(clip(road), rgb("#5a5a62"))
            sh.paint(clip(sh.mask_poly([(cx - r, ground_y), (cx - r * 0.4, cy - r * 0.05), (cx, ground_y)])), rgb("#a64a2a"))
        else:
            for i, (dx, dy, rot) in enumerate(((-0.35, -0.2, -0.3), (0.0, -0.3, 0.0), (0.35, -0.2, 0.3))):
                w2, h2 = r * 0.4, r * 0.56
                c, s_ = math.cos(rot), math.sin(rot)
                pts = [(cx + dx * r + c * px - s_ * py, cy + dy * r + s_ * px + c * py) for px, py in
                       ((-w2 / 2, -h2 / 2), (w2 / 2, -h2 / 2), (w2 / 2, h2 / 2), (-w2 / 2, h2 / 2))]
                sh.paint(clip(sh.mask_poly(pts)), (1, 1, 1))
                sh.paint(clip(sh.mask_circle(cx + dx * r, cy + dy * r, r * 0.08)), rgb("#c8303a") if i != 1 else INK)
            sh.paint(clip(sh.mask_circle(cx, cy + r * 0.35, r * 0.18)), (1, 1, 1))
    # a golf ball at the bottom of every emblem's rim
    sh.paint(sh.mask_circle(cx, cy + r * 0.9, r * 0.12), (1, 1, 1))
    main.paste(sh, CX - size / 2, CY - size / 2)


def emblem_sheet(rel, themes):
    sh = Sheet(800, 600)
    small_y = (395, 395) if rel.endswith("parklink.png") else (481, 481)
    # the big emblems: two rows of four (for the property chooser), first theme on top
    for row, theme in enumerate(themes[::-1] if rel.endswith("tropdesert.png") else themes):
        for k in range(4):
            emblem(sh, 5 + 172 * k + 77, (16 if row == 0 else 186) + 77, 75, theme, k)
    # the small ones: 80 x 80 cuts at (3 + 88 k, y), the first four the first theme
    for k in range(8):
        theme = themes[0] if k < 4 else themes[1]
        emblem(sh, 3 + 88 * k + 40, small_y[0] + 40, 38, theme, k % 4)
    rgb_, a = sheet_out(sh)
    save_pair(rel, rel[:-4] + "_A.png", rgb_, a, (0, 0, 0))


# ---- the title screen (Interface/TitleBASE, TitleUnSel, TitleMO) ----------------------------------------------------------------

MENU_BTN = [(40, 25, 325, 120), (415, 40, 360, 135), (12, 395, 285, 100), (508, 380, 270, 95), (285, 478, 285, 105)]
# the pill of each button (where the label is written) and its icon bubble
PILLS = [(75, 52, 219, 65), (452, 71, 247, 73), (47, 421, 193, 53), (545, 400, 178, 49), (321, 513, 193, 53)]
ICONS = [((304, 63), 34, g_folder), ((711, 79), 40, g_ball_tee), ((249, 421), 32, g_shovel), ((726, 398), 30, g_cards),
         ((521, 511), 30, g_trophy), ((748, 555), 23, g_exit)]
LOGO_BOX = (86, 172, 628, 206)


def title_background(sh):
    # a soft morning sky over rolling fairways
    sh.paint(np.ones((sh.h * S, sh.w * S), np.float32), sh.vgrad(rgb("#9fd3f0"), rgb("#e9f6fb"), 0, 330))
    for i, (y, col) in enumerate(((330, "#8ec96a"), (370, "#6fb455"), (420, "#5aa448"), (480, "#4b9440"))):
        pts = [(x, y + 18 * math.sin(x / (160 + 40 * i) + i * 1.7) + 10 * math.sin(x / 57 + i)) for x in range(-10, 811, 10)]
        pts += [(810, 610), (-10, 610)]
        sh.paint(sh.mask_poly(pts), rgb(col))
    # mowing stripes on the nearest hill
    for k in range(10):
        x0 = k * 90 - 40
        stripe = sh.mask_poly([(x0, 610), (x0 + 45, 610), (x0 + 120, 470), (x0 + 75, 470)])
        sh.paint(stripe * sh.mask_poly([(x, 480 + 18 * math.sin(x / 280 + 5.1) + 10 * math.sin(x / 57 + 3)) for x in range(-10, 811, 10)]
                                       + [(810, 610), (-10, 610)]), (1, 1, 1), 0.06)
    # a flag far away and some trees on the horizon
    rng = np.random.default_rng(3)
    for k in range(18):
        x = rng.uniform(0, 800)
        y = 330 + 18 * math.sin(x / 160 + 0) + 10 * math.sin(x / 57) - 2
        r = rng.uniform(8, 16)
        sh.paint(sh.mask_rrect(x - 1.5, y - r * 0.5, 3, r * 0.8, 1), rgb("#6b4a2e"))
        sh.paint(sh.mask_circle(x, y - r * 0.9, r * 0.75), rgb("#3f8a3a") if k % 3 else rgb("#2f7032"))
    sh.paint(sh.mask_rrect(612, 300, 2, 34, 1), (1, 1, 1))
    sh.paint(sh.mask_poly([(614, 300), (630, 305), (614, 311)]), rgb("#e0403a"))
    # a big golf ball behind the logo
    ball = sh.mask_circle(400, 285, 205)
    sh.shadow(ball, 6, 10, 12, 0.3)
    sh.paint(ball, sh.rgrad(rgb("#ffffff"), rgb("#c9d6e2"), 330, 200, 300))
    for gy in range(85, 495, 26):
        for gx in range(195, 610, 26):
            ox = 13 if (gy // 26) % 2 else 0
            d = math.hypot(gx + ox - 400, gy - 285)
            if d < 190:
                sh.paint(sh.mask_circle(gx + ox, gy, 7 * (1 - (d / 205) ** 2) ** 0.3), rgb("#b8c6d4"), 0.5)


def logo(sh, cx, cy, scale=1.0, lit=False):
    """The OpenSGolf name: 'Open' in green and 'SGolf' in navy on a cream oval, a flag in the O."""
    x, y, w, h = LOGO_BOX
    oval(sh, x, y, w, h, CREAM, rgb("#e7dcbc"), rim=rgb("#2e6b3e"), rim_w=5)
    if lit:
        sh.glow(sh.mask_ellipse(x, y, w, h) - sh.mask_ellipse(x + 6, y + 6, w - 12, h - 12), 4, 0.6, GOLD_HI)
    t1 = sh.mask_text("Open", cx - 118, cy - 12, 78 * scale)
    t2 = sh.mask_text("SGolf", cx + 92, cy - 12, 78 * scale)
    for t, col in ((t1, GREEN), (t2, NAVY)):
        sh.shadow(t, 2, 3, 2, 0.3)
        sh.paint(t, lighten(col, 0.15) if lit else col)
    sub = sh.mask_text("free golf course designer", cx, cy + 50, 22 * scale)
    sh.paint(sub, rgb("#4a5a66"))
    # a ball on a tee to the left
    sh.paint(sh.mask_circle(x + 78, y + 85, 26), sh.rgrad((1, 1, 1), rgb("#c9d6e2"), x + 70, y + 76, 34))
    sh.paint(sh.mask_poly([(x + 66, y + 108), (x + 90, y + 108), (x + 81, y + 118), (x + 80, y + 140), (x + 76, y + 140),
                           (x + 75, y + 118)]), rgb("#d0a05a"))


def title_pill(sh, k, lit=False):
    x, y, w, h = PILLS[k]
    if lit:
        oval(sh, x - 8, y - 6, w + 16, h + 12, GOLD_HI, GOLD, rim=rgb("#a06a10"))
    else:
        oval(sh, x - 8, y - 6, w + 16, h + 12, rgb("#2f7a52"), rgb("#1c5338"), rim=rgb("#e9f2d8"))


def title():
    base = Sheet(800, 600)
    title_background(base)
    for k in range(5):
        title_pill(base, k)
    logo(base, 400, 268)
    save("Interface/TitleBASE.png", base)
    un = Sheet(800, 600)
    for (cx, cy), r, gl in ICONS:
        glossy_disc(un, cx, cy, r, BLUE, gl)
    save("Interface/TitleUnSel.png", un, key=(255, 0, 255))
    mo = Sheet(800, 600)
    for k in range(5):
        title_pill(mo, k, lit=True)
    for (cx, cy), r, gl in ICONS:
        glossy_disc(mo, cx, cy, r, GOLD, gl, glyph_col=INK)
    logo(mo, 400, 268, lit=True)
    save("Interface/TitleMO.png", mo, key=(255, 0, 255))


# ---- the terrain panel (Interface/BaseTerrainPanel.pcx and its alpha) ------------------------------------------------------------

def terrain_panel():
    sh = Sheet(800, 600)
    # the tray: a glass shelf along the bottom with a rounded left end that holds the panel's own buttons
    body = sh.mask_rrect(215, 488, 620, 140, 40)
    sh.paint(body, sh.vgrad(rgb("#3e6588"), rgb("#1d3650"), 488, 600), 0.78)
    sh.paint(body - sh.mask_rrect(218, 491, 620, 140, 38), rgb("#a9cdeb"), 0.8)
    sh.paint(sh.mask_rrect(250, 493, 540, 6, 3), (1, 1, 1), 0.12)
    # the buttons that live on the tray: the current mode (gold, top left), to Amenities, Undo
    glossy_disc(sh, 270, 497, 17, GOLD, g_flag, glyph_col=INK)
    glossy_disc(sh, 237, 525, 18, BLUE, g_shovel)
    glossy_disc(sh, 261, 578, 14, rgb("#e8dcb8"), g_turn(-1), glyph_col=INK)
    # the loose pieces: the to-Amenities button lit (150, 480) and the Undo looks (0..120, 550)
    glossy_disc(sh, 167, 497, 17, rgb("#6fb3ee"), g_shovel, state="lit")
    for k in range(4):
        st = ["normal", "lit", "normal", "off"][k]
        col = [rgb("#e8dcb8"), rgb("#f7ecc8"), GOLD, GREY][k]
        glossy_disc(sh, 17 + 40 * k, 569, 15, col, g_turn(-1), glyph_col=INK, state=st)
    for k in range(3):
        glossy_disc(sh, 16 + 40 * k, 497, 15, [rgb("#e8dcb8"), rgb("#f7ecc8"), GOLD][k], g_flag, glyph_col=INK)
    rgb_, a = sheet_out(sh)
    save_pair("Interface/BaseTerrainPanel.png", "Interface/BaseTerrainPanel_A.png", rgb_, a, (255, 0, 255))


# ---- the terrain buttons (Interface/<Theme>TerrainButtons.pcx) ----------------------------------------------------------------

THEME_COLOURS = {
    "park": dict(grass="#5fae48", green="#7cc85a", rough="#4a8f3a", deep="#3a7530", sand="#eadfb4", water="#3f86c4",
                 ground="#8a6a48", rock="#9a968c"),
    "desert": dict(grass="#6aac4a", green="#86c860", rough="#d99a5c", deep="#c4824a", sand="#f0dcb0", water="#3f86b4",
                   ground="#c98a54", rock="#b07a54"),
    "trop": dict(grass="#4fb444", green="#72d058", rough="#3f9a3a", deep="#2f7f32", sand="#f7efd2", water="#2fb0c8",
                 ground="#9a7a52", rock="#8e8a80"),
    "links": dict(grass="#78a052", green="#8cb85e", rough="#8a9a5a", deep="#9a8e56", sand="#e4d4a8", water="#4a7896",
                  ground="#8a7650", rock="#8c8a84"),
}
# the tree buttons' sizes per theme (panels_ui.rs TREE_SIZE, exe theme order Parkland, Desert, Tropical, Links)
TREE_SIZE = {"park": [(65, 84), (62, 70), (62, 88)], "desert": [(75, 74), (62, 67), (65, 104)],
             "trop": [(72, 79), (62, 81), (65, 104)], "links": [(67, 84), (62, 88), (67, 76)]}
# the slots' tile ids (panels_ui.rs TERRAIN_SLOTS): tees, green, sand trap, rough, pot bunker, stream, water, fairway,
# firm fairway, deep rough, waste bunker, brush, rocks; cell 13 (unused by the panel) gets a mound
SLOT_TILES = [0, 1, 7, 4, 9, 10, 17, 2, 3, 5, 8, 11, 12, 6]


def iso_tile(main, X, Y, w, h, top_col, kind, theme, state, rng):
    """One iso tile button in the 62 x 54 cell at (X, Y): a diamond top with a lip, the terrain's swatch, a mark."""
    sh = Sheet(w + 4, h + 4)
    x, y = 0, 0
    cx = x + w / 2
    top = [(cx, y + 3), (x + w - 3, y + h * 0.42), (cx, y + h * 0.81), (x + 3, y + h * 0.42)]
    side = [(x + 3, y + h * 0.42), (cx, y + h * 0.81), (x + w - 3, y + h * 0.42), (x + w - 3, y + h * 0.56), (cx, y + h - 2),
            (x + 3, y + h * 0.56)]
    rim = {"normal": rgb("#cfd9e3"), "lit": rgb("#ffffff"), "sel": GOLD, "off": rgb("#7f8790")}[state]
    sh.shadow(sh.mask_poly(side) + sh.mask_poly(top), 1, 2, 1.5, 0.35)
    sh.paint(sh.mask_poly(side), lighten(rim, -0.35))
    tm = sh.mask_poly(top)
    sh.paint(tm, rim)
    inner = sh.mask_poly([(cx, y + 6.5), (x + w - 8.5, y + h * 0.42), (cx, y + h * 0.81 - 3.5), (x + 8.5, y + h * 0.42)])
    col = np.asarray(top_col, float)
    if state == "off":
        col = mix(col, GREY, 0.75)
    elif state == "lit":
        col = lighten(col, 0.15)
    sh.paint(inner, sh.vgrad(lighten(col, 0.12), lighten(col, -0.12), y + 6, y + h * 0.8))
    # a speckle so the swatch reads as a surface
    for i in range(14):
        px, py = rng.uniform(x + 14, x + w - 14), rng.uniform(y + 10, y + h * 0.7)
        sh.paint(inner * sh.mask_circle(px, py, 1.4), lighten(col, rng.choice([-0.2, 0.15])), 0.6)
    ink = INK if state != "off" else rgb("#55595e")
    if kind == 0:
        for dx in (-9, 9):
            sh.paint(sh.mask_circle(cx + dx, y + h * 0.4, 3), rgb("#e0403a") if state != "off" else ink)
    elif kind == 1:
        sh.paint(sh.mask_rrect(cx - 0.8, y + 7, 1.6, 17, 0.5), (1, 1, 1))
        sh.paint(sh.mask_poly([(cx + 0.8, y + 7), (cx + 10, y + 10), (cx + 0.8, y + 13)]), rgb("#e0403a") if state != "off" else ink)
        sh.paint(sh.mask_ellipse(cx - 3, y + 22.5, 6, 3), ink)
    elif kind in (7, 8, 9):
        sand = rgb(THEME_COLOURS[theme]["sand"]) if state != "off" else GREY
        sh.paint(inner * sh.mask_ellipse(cx - 14, y + 12, 28, 16), sand)
    elif kind in (10, 17):
        for k in range(2):
            pts = [(x + 14 + t * (w - 28), y + 16 + k * 7 + 2.5 * math.sin(t * 10)) for t in np.linspace(0, 1, 12)]
            sh.paint(inner * sh.mask_line(pts, 1.4), (1, 1, 1), 0.7)
    elif kind == 11:
        for dx, dy in ((-8, 2), (6, -2), (0, 6)):
            sh.paint(sh.mask_circle(cx + dx, y + h * 0.4 + dy, 5), rgb("#3a7a32") if state != "off" else ink)
    elif kind == 12:
        for dx, dy in ((-7, 3), (6, -1), (2, 6)):
            sh.paint(sh.mask_ellipse(cx + dx - 5, y + h * 0.4 + dy - 3, 10, 6), rgb("#d8d4cc") if state != "off" else ink)
    elif kind == 6:
        sh.paint(inner * sh.mask_ellipse(cx - 13, y + 10, 26, 18), lighten(col, 0.2))
    main.paste(sh, X, Y)


def tile_colour(kind, theme):
    c = THEME_COLOURS[theme]
    return rgb({0: c["green"], 1: c["green"], 7: c["sand"], 4: c["rough"], 9: c["sand"], 10: c["water"], 17: c["water"],
                2: c["grass"], 3: c["grass"], 5: c["deep"], 8: c["sand"], 11: c["rough"], 12: c["rock"], 6: c["grass"]}[kind])


def tree_icon(theme, slot, w, h):
    """A little rendered tree for the tree buttons."""
    names = {"park": ["treemaplelarge", "treepinelarge", "treepalmlg"], "desert": ["joshuatree_lg", "cactusa_lg", "tallpalm_large"],
             "trop": ["drac_large", "cerc_large", "treepalmlg"], "links": ["linkstree3_tall", "linkspine_tall", "scotspine_lg"]}
    sc = trees.species(names[theme][slot], np.random.default_rng(slot + 3), 1.0)
    from scene import fit_scale
    sc = sc.transformed(fit_scale([(sc, 0.0)], w, h, w // 2, int(h * 0.92), shadow=False, margin=2.0))
    fr = Frame(w, h, w // 2, int(h * 0.92), ss=4)
    sc.render(fr, 0.0, shadow=False)
    col, op, tag, bright, _ = fr.resolve(edge_dark=0.15)
    # trees in ramp colours: give them their own palette's colours
    leaf = np.asarray(sc.pal.get("LEAF", (80, 130, 60)), float) / 255
    bark = np.asarray(sc.pal.get("BARK", (110, 80, 50)), float) / 255
    import mats
    out = np.zeros((h, w, 4))
    for t, base in ((mats.ID["LEAF"], leaf), (mats.ID["BARK"], bark)):
        m = op & (tag == t)
        out[m, :3] = np.clip(base[None, :] * bright[m][:, None], 0, 1)
        out[m, 3] = 1
    m = op & (tag != mats.ID["LEAF"]) & (tag != mats.ID["BARK"])
    out[m, :3] = col[m]
    out[m, 3] = 1
    return out


def terrain_buttons(theme, rel):
    sh = Sheet(800, 600)
    rng = np.random.default_rng(5)
    for k, kind in enumerate(SLOT_TILES):
        for st, state in enumerate(["normal", "lit", "sel", "off"]):
            x = (248 if k > 6 else 0) + st * 62
            y = (k % 7) * 54
            iso_tile(sh, x, y, 62, 54, tile_colour(kind, theme), kind, theme, state, rng)
    # the three tree buttons: cells at x 520 + 80 state, y 0, 150, 300, of the size the panel cuts (panels_ui.rs TREE_SIZE)
    size = TREE_SIZE[theme]
    for slot in range(3):
        cw, ch = size[slot]
        icon = tree_icon(theme, slot, cw, ch - 16)
        for st, state in enumerate(["normal", "lit", "sel"]):
            x, y = 520 + st * 80, slot * 150
            iso_tile(sh, x, y + ch - 50, cw - 2, 48, rgb(THEME_COLOURS[theme]["ground"]), -1, theme, state, rng)
            sub = Sheet(cw, ch - 16)
            big = np.kron(icon, np.ones((S, S, 1))).astype(np.float32)
            sub.rgb[:] = big[..., :3]
            sub.a[:] = big[..., 3]
            sh.paste(sub, x, y)
    save(rel, sh, key=(255, 0, 255))


# ---- dialogs (Interface/TransPopups.pcx, its alpha and s_TransPopups.pcx) ----------------------------------------------------

def golf_ball(sh, cx, cy, r):
    m = sh.mask_circle(cx, cy, r)
    sh.shadow(m, 1, 2, 2, 0.35)
    sh.paint(m, rgb("#8fa3b5"))
    body = sh.mask_circle(cx, cy, r * 0.96)
    sh.paint(body, sh.rgrad(rgb("#ffffff"), rgb("#d6dee6"), cx - r * 0.35, cy - r * 0.4, r * 1.5))
    step = max(4.0, r / 6)
    yy = cy - r
    row = 0
    while yy < cy + r:
        xx = cx - r + (step / 2 if row % 2 else 0)
        while xx < cx + r:
            d = math.hypot(xx - cx, yy - cy) / r
            if d < 0.9:
                sh.paint(body * sh.mask_circle(xx, yy, step * 0.28 * (1 - d * d) ** 0.4), rgb("#b9c6d2"), 0.55)
            xx += step
        yy += step * 0.87
        row += 1


def g_arrow(direction):
    def f(sh, cx, cy, r):
        if direction == "left":
            return np.maximum(sh.mask_poly([(cx - r * 0.8, cy), (cx - r * 0.1, cy - r * 0.6), (cx - r * 0.1, cy + r * 0.6)]),
                              sh.mask_rrect(cx - r * 0.2, cy - r * 0.22, r * 0.95, r * 0.44, r * 0.08))
        up = sh.mask_poly([(cx, cy - r * 0.85), (cx + r * 0.6, cy - r * 0.15), (cx - r * 0.6, cy - r * 0.15)])
        down = sh.mask_poly([(cx, cy + r * 0.85), (cx + r * 0.6, cy + r * 0.15), (cx - r * 0.6, cy + r * 0.15)])
        return {"up": up, "down": down, "updown": np.maximum(up, down)}[direction]
    return f


def g_tick(sh, cx, cy, r):
    return sh.mask_line([(cx - r * 0.6, cy), (cx - r * 0.15, cy + r * 0.5), (cx + r * 0.7, cy - r * 0.6)], r * 0.32)


def trans_popups():
    sh = Sheet(800, 600)
    line = rgb("#cfe4f6")
    # the dialog frame: a rounded outline across the 3 x 3 cuts at (17 col, 17 row)
    outer = sh.mask_rrect(0, 0, 50, 50, 11) - sh.mask_rrect(2, 2, 46, 46, 9)
    sh.paint(np.clip(outer, 0, 1), line)
    sh.paint(np.clip(sh.mask_rrect(2, 2, 46, 46, 9) - sh.mask_rrect(3, 3, 44, 44, 8), 0, 1), rgb("#0d1822"), 0.6)
    # the ticker rail and its ends
    for x0 in (300, 334):
        sh.paint(sh.mask_ellipse(x0, 40, 16, 10), line)
    sh.paint(sh.mask_rrect(317, 45, 16, 5, 2), line)
    # the scroll buttons: up and down, lit, off
    for k, (x0, st) in enumerate(((359, "normal"), (399, "lit"), (440, "off"))):
        glossy_disc(sh, x0 + 15.5, 60.5, 15, GOLD if st != "off" else GREY, g_arrow("updown"), glyph_col=INK, state=st)
    # a tab strip (left, middle, right) and a skill row (an oval and a bar)
    sh.paint(sh.mask_rrect(0, 60, 26, 19, 6), CREAM)
    sh.paint(sh.mask_rrect(17, 60, 16, 19, 0), CREAM)
    sh.paint(sh.mask_rrect(24, 60, 26, 19, 6), CREAM)
    oval(sh, 32, 100, 48, 24, CREAM, rgb("#e2d6b4"), shadow=False, rim_w=2)
    pill(sh, 80, 102, 143, 19, CREAM, rgb("#e2d6b4"), shadow=False)
    # the pads: a grey one, up and down arrows (blue, then gold)
    oval(sh, 0, 150, 40, 25, rgb("#b8c8d8"), rgb("#7f94aa"), shadow=False, rim_w=2)
    for k, (y0, col) in enumerate(((200, rgb("#8fb4dc")), (220, rgb("#8fb4dc")), (250, GOLD), (270, GOLD))):
        oval(sh, 3, y0, 32, 12, lighten(col, 0.2), col, shadow=False, rim_w=1.5)
        sh.paint(g_arrow("up" if k % 2 == 0 else "down")(sh, 19, y0 + 6, 5), INK)
    # the portrait balls
    golf_ball(sh, 70, 370, 63)
    golf_ball(sh, 175, 334, 32)
    # the buttons block: ticks (normal, lit, off), radio balls and arrow buttons in three sizes
    for k, st in enumerate(("normal", "lit", "off")):
        glossy_disc(sh, 316 + 50 * k, 156, 15, [rgb("#9fc8f0"), GOLD, GREY][k], g_tick, glyph_col=INK, state=st)
    for row, (y0, r) in enumerate(((206, 16), (252, 12), (308, 7))):
        for k, st in enumerate(("lit", "normal", "off")):
            glossy_disc(sh, 316 + 50 * k, y0, r, GOLD if st != "off" else rgb("#6a5a20"), None, state=st, shadow=False)
            glossy_disc(sh, 466 + 50 * k, y0, r, GOLD if st != "off" else rgb("#6a5a20"), g_arrow("left"), glyph_col=INK,
                        state=st, shadow=False)
    sh.paint(g_arrow("left")(sh, 460, 357, 9), rgb("#2a3e5a"))
    rgb_, a = sheet_out(sh)
    save_pair("Interface/TransPopups.png", "Interface/TransPopups_A.png", rgb_, a, (255, 0, 255))
    # the masks the translucent fills darken through: the frame's rounded box over its 3 x 3 cuts, the ticker's squares
    s2 = Sheet(800, 600)
    s2.paint(s2.mask_rrect(1, 1, 48, 48, 10), (0, 1, 0))
    s2.paint(s2.mask_rrect(300, 0, 50, 50, 0), (0, 1, 0))
    save("Interface/s_TransPopups.png", s2, key=(248, 0, 248))


# ---- the course report (Interface/infoscreens/coursereport.pcx) and the OK tick (OkStates.pcx) --------------------------------
# info_ui.rs: the header (0, 0, 800, 104), a row strip (y 129 odd, 169 even, 17 high), the coloured cells (y 205 red, 241
# green, 275 dark green, 15 high) of each column (CX, CW), the hole icons (194 + 15 k, 557), the dogleg marks (547, 549 + 14 k),
# the totals strip (0, 420, 800, 29) and the bottom with the legend (0, 468, 800, 72), the OK tick at (734, bottom + 24)

CX = [112, 157, 190, 233, 283, 330, 375, 422, 468, 571, 641, 716]
CW = [41, 29, 39, 46, 43, 41, 43, 42, 99, 66, 71, 74]
FRAME_TOP, FRAME_BOT = rgb("#a9bedb"), rgb("#7f97ba")
FRAME_RIM = rgb("#e6eef8")
CELL = rgb("#fbf7ea")


def frame_band(sh, y, h, top=False, bottom=False):
    """The report's frame across the whole width at y..y+h: rounded where it starts and ends."""
    r = 18
    y0 = y - (0 if top else r)
    y1 = y + h + (0 if bottom else r)
    m = sh.mask_rrect(0, y0, 800, y1 - y0, r) * sh.mask_rrect(0, y, 800, h, 0)
    sh.paint(m, sh.vgrad(FRAME_TOP, FRAME_BOT, 0, 600))
    edge = (m - sh.mask_rrect(3, y0 + 3, 794, y1 - y0 - 6, r - 3) * sh.mask_rrect(0, y, 800, h, 0))
    sh.paint(np.clip(edge, 0, 1), FRAME_RIM, 0.8)


def cell(sh, x, y, w, h, col, r=4):
    m = sh.mask_rrect(x, y, w, h, r)
    sh.paint(m, lighten(col, -0.35))
    sh.paint(sh.mask_rrect(x + 1, y + 1, w - 2, h - 2, r - 1), sh.vgrad(lighten(col, 0.08), lighten(col, -0.06), y, y + h))


def course_report():
    sh = Sheet(800, 600)
    # the header: a frame with a title plate and the column heads
    frame_band(sh, 20, 84, top=True)
    tm = sh.mask_rrect(113, 24, 410, 36, 18)
    sh.shadow(tm, 1, 2, 2, 0.3)
    cell(sh, 113, 24, 410, 36, CELL, 18)
    for x, w in zip(CX, CW):
        cell(sh, x, 72, w, 20, rgb("#dfe8f5"), 9)
    # the hole rows (odd, even)
    for y0, tint in ((129, 0.0), (169, -0.04)):
        frame_band(sh, y0, 17)
        cell(sh, 10, y0 + 1, 95, 15, lighten(CELL, tint), 3)
        for x, w in zip(CX, CW):
            cell(sh, x, y0 + 1, w, 15, lighten(CELL, tint), 3)
    # the coloured cells
    for y0, col in ((205, rgb("#e0594d")), (241, rgb("#5cc36a")), (275, rgb("#3f9d4f"))):
        for x, w in zip(CX, CW):
            cell(sh, x, y0, w, 15, col, 3)
    # the totals strip
    frame_band(sh, 420, 29)
    cell(sh, 10, 429, 95, 16, CELL, 3)
    for x, w in zip(CX, CW):
        cell(sh, x, 429, w, 16, CELL, 3)
    # the bottom: legend pills with their marks, a place for the OK tick
    frame_band(sh, 468, 72, bottom=True)
    for x, mark in ((113, 0), (271, 1), (423, 2)):
        cell(sh, x + 20, 474, 90, 20, rgb("#dfe8f5"), 10)
        hole_icon(sh, x, 477, mark)
    sh.paint(sh.mask_circle(756, 513, 25), rgb("#6f86ab"), 0.6)
    # the hole icons: Top 100, Top 18, scenic, and scenic with either
    for k, marks in enumerate(([0], [1], [2], [0, 2], [1, 2])):
        for j, mk in enumerate(marks):
            hole_icon(sh, 194 + 15 * k + (4 if j else 0), 557 + (4 if j else 0), mk, small=len(marks) > 1)
    # the dogleg marks: left, straight, right
    for k in range(3):
        x0, y0 = 547, 549 + 14 * k
        if k == 1:
            sh.paint(sh.mask_rrect(x0 + 2, y0 + 6, 8, 2, 1), INK)
        else:
            sgn = -1 if k == 0 else 1
            pts = [(x0 + 6, y0 + 12), (x0 + 6, y0 + 6), (x0 + 6 + sgn * 4, y0 + 3)]
            sh.paint(sh.mask_line(pts, 1.6), INK)
    rgb_, a = sheet_out(sh)
    save_pair("Interface/infoscreens/coursereport.png", "Interface/infoscreens/coursereport_alpha.png", rgb_, a, (255, 0, 255))


def hole_icon(sh, x, y, mark, small=False):
    """A legend mark in a 14 x 15 box: an orange square (Top 100), a yellow one (Top 18) or a scenic flower."""
    k = 0.7 if small else 1.0
    if mark < 2:
        cell(sh, x + 1, y + 2, 11 * k, 11 * k, rgb("#f08a3c") if mark == 0 else rgb("#f2d43a"), 2)
    else:
        cx, cy = x + 7 * k, y + 7 * k
        for i in range(5):
            a = 2 * math.pi * i / 5
            sh.paint(sh.mask_circle(cx + math.cos(a) * 3 * k, cy + math.sin(a) * 3 * k, 2.4 * k), rgb("#c84a8a"))
        sh.paint(sh.mask_circle(cx, cy, 1.6 * k), rgb("#ffd25a"))


def ok_states():
    sh = Sheet(266, 131)
    for k, (st, col) in enumerate((("normal", rgb("#9fc8f0")), ("lit", GOLD), ("off", GREY))):
        glossy_disc(sh, 1 + 45 * k + 22, 1 + 21, 19, col, g_tick, glyph_col=INK, state=st)
    save("Interface/infoscreens/OkStates.png", sh, key=(255, 0, 255))


# ---- the Buildings panel (Interface/BuildingPanel.pcx and its alpha) and the lots' pictures (Interface/<theme> layout.pcx) ----
# panels_ui.rs: the body (214, 482, 586, 118); lot i's pad at (343 + 80 i, 501) for i < 5, else (384 + 80 (i - 5), 542), cut
# hovered at (0, 0, 74, 64) / (0, 100, 77, 58) and selected 100 pixels right; to-Elevation lit at (150, 480, 34, 34) drawn at
# (220, 510); Undo hovered (40, 550, 34, 40) and armed (80, 550) drawn at (261, 553). Lot i's picture is the 75 x 100 cell
# (75 i, row) of the theme's layout sheet, rows grey 0, colour 200, grey upgraded 100, colour upgraded 300, drawn at
# (344 + 80 i, 453) or (384 + 80 (i - 5), 493).

LOT_PADS = [(343 + 80 * i, 501, 74, 64) for i in range(5)] + [(384 + 80 * i, 542, 77, 58) for i in range(4)]


def lot_pad(sh, x, y, w, h, state):
    """The plinth a lot's picture stands on: a flat diamond with a lip."""
    cx, cy = x + w / 2, y + h * 0.56
    rx, ry = w * 0.46, h * 0.3
    top = [(cx, cy - ry), (cx + rx, cy), (cx, cy + ry), (cx - rx, cy)]
    side = [(cx - rx, cy), (cx, cy + ry), (cx + rx, cy), (cx + rx, cy + 4), (cx, cy + ry + 4), (cx - rx, cy + 4)]
    rim = {"normal": rgb("#9fbad3"), "lit": rgb("#ffffff"), "sel": GOLD}[state]
    sh.shadow(np.clip(sh.mask_poly(side) + sh.mask_poly(top), 0, 1), 0, 2, 2, 0.3)
    sh.paint(sh.mask_poly(side), lighten(rim, -0.4), 0.9)
    sh.paint(sh.mask_poly(top), rim, 0.95 if state != "normal" else 0.5)
    k = 4
    inner = [(cx, cy - ry + k * 0.6), (cx + rx - k, cy), (cx, cy + ry - k * 0.6), (cx - rx + k, cy)]
    fill = {"normal": rgb("#34597a"), "lit": rgb("#5d8db8"), "sel": rgb("#6b8fb0")}[state]
    sh.paint(sh.mask_poly(inner), sh.vgrad(lighten(fill, 0.15), fill, cy - ry, cy + ry), 0.85)


def building_panel():
    sh = Sheet(800, 600)
    body = sh.mask_rrect(215, 488, 620, 140, 40)
    sh.paint(body, sh.vgrad(rgb("#3e6588"), rgb("#1d3650"), 488, 600), 0.78)
    sh.paint(body - sh.mask_rrect(218, 491, 620, 140, 38), rgb("#a9cdeb"), 0.8)
    sh.paint(sh.mask_rrect(250, 493, 540, 6, 3), (1, 1, 1), 0.12)
    for (x, y, w, h) in LOT_PADS:
        lot_pad(sh, x, y, w, h, "normal")
    # the tray's own buttons: the current mode (gold), to Elevation, Undo
    glossy_disc(sh, 270, 497, 17, GOLD, g_house, glyph_col=INK)
    glossy_disc(sh, 236, 526, 17, BLUE, g_shovel)
    glossy_disc(sh, 277, 572, 13, rgb("#e8dcb8"), g_turn(-1), glyph_col=INK)
    # the loose pieces
    glossy_disc(sh, 167, 497, 17, rgb("#6fb3ee"), g_shovel, state="lit")
    for k in range(4):
        st = ["normal", "lit", "normal", "off"][k]
        col = [rgb("#e8dcb8"), rgb("#f7ecc8"), GOLD, GREY][k]
        glossy_disc(sh, 17 + 40 * k, 569, 13, col, g_turn(-1), glyph_col=INK, state=st)
    for row, (w, h) in enumerate(((74, 64), (77, 58))):
        lot_pad(sh, 0, 100 * row, w, h, "lit")
        lot_pad(sh, 100, 100 * row, w, h, "sel")
        lot_pad(sh, 200, 100 * row, w, h, "normal")
    rgb_, a = sheet_out(sh)
    save_pair("Interface/BuildingPanel.png", "Interface/BuildingPanel_A.png", rgb_, a, (255, 0, 255))


# the lots in panel order (STR_LOT_*): putting green, snack bar, pro shop, swim club, driving range, cart garage, marina,
# resort hotel, airstrip
def lot_scene(i, style, level):
    import buildings as bd
    rng = np.random.default_rng(7)
    f = [lambda: bd.putting(style, rng, level), lambda: bd.shop(style, rng, "snack", level),
         lambda: bd.shop(style, rng, "pro", level), lambda: bd.tennis(style, rng, level),
         lambda: bd.shop(style, rng, "range", level), lambda: bd.shop(style, rng, "garage", level),
         lambda: bd.marina(style, rng, level), lambda: bd.hotel(style, rng, level),
         lambda: bd.airstrip(style, rng, level)][i]
    body, base = f()
    body.mass = None
    body.ambient, body.diffuse = 0.66, 0.55
    base.ambient, base.diffuse = 0.72, 0.42
    return body, base


def lot_icon(i, style, level):
    """Lot i's picture, 75 x 100 at the sheet's resolution: rgb and coverage at S times the size."""
    from raster import UNITS_PER_PX, eye_matrix
    body, base = lot_scene(i, style, level)
    # the largest scale at which the whole lot fits 71 x 62 pixels, standing on the cell's pad (its bottom at y 94)
    p = np.concatenate([body.points(), base.points()])
    e = p @ eye_matrix(0.0).T / UNITS_PER_PX
    x0, x1, y0, y1 = e[:, 0].min(), e[:, 0].max(), e[:, 1].min(), e[:, 1].max()
    k = min(71 * S / (x1 - x0), 62 * S / (y1 - y0))
    w, h = 75 * S, 100 * S
    ax, ay = int(w / 2 - k * (x0 + x1) / 2), int(94 * S - k * y1)
    body, base = body.transformed(k), base.transformed(k)
    fr = Frame(w, h, ax, ay, ss=1)
    base.render(fr, 0.0, shadow=False)
    body.render(fr, 0.0, shadow=False)
    col, op, _, _, _ = fr.resolve(edge_dark=0.0)
    return col.astype(np.float32), op.astype(np.float32)


def layout_sheet(theme, rel):
    sh = Sheet(751, 812)
    for i in range(9):
        for level in (1, 2):
            col, a = lot_icon(i, theme, level)
            grey = col.mean(-1, keepdims=True)
            dim = np.clip(mix(np.repeat(grey, 3, -1), rgb("#9aa8b8"), 0.35) * 0.9 + 0.05, 0, 1)
            for row, c in ((200 if level == 1 else 300, col), (0 if level == 1 else 100, dim)):
                sub = Sheet(75, 100)
                sub.shadow(a, 1, 1.5, 1.5, 0.35)
                sub.paint(a, c)
                sh.paste(sub, 75 * i, row)
        print("  lot", i, flush=True)
    save(rel, sh, key=(248, 0, 248))


# ---- the Amenities panel (Interface/AmenitiesPanel.pcx and its alpha) -------------------------------------------------------
# panels_ui.rs AMENITIES: each slot's hit circle, where its looks are drawn and its hover cut (selected 100 right, disabled 200
# right); the back button lit at (0, 400, 34, 34) drawn at (254, 483); the design strip's ends and middle (STRIP_*, 79 tall,
# drawn along y 477) and its buttons (strip_button: 47 x 40 at y 350 for the even designs, 47 x 41 at y 400 for the odd ones,
# x 300 hovered, 400 current, 600 otherwise).

def g_washer(sh, cx, cy, r):
    post = sh.mask_rrect(cx - r * 0.1, cy - r * 0.1, r * 0.2, r * 0.95, r * 0.05)
    box = sh.mask_rrect(cx - r * 0.5, cy - r * 0.75, r * 1.0, r * 0.7, r * 0.15)
    crank = sh.mask_line([(cx + r * 0.5, cy - r * 0.4), (cx + r * 0.85, cy - r * 0.4), (cx + r * 0.85, cy - r * 0.75)], r * 0.12)
    foot = sh.mask_rrect(cx - r * 0.4, cy + r * 0.75, r * 0.8, r * 0.15, r * 0.05)
    return np.maximum.reduce([post, box, crank, foot])


def g_path(sh, cx, cy, r):
    outer = sh.mask_line([(cx - r * 0.75, cy + r * 0.85), (cx - r * 0.1, cy), (cx + r * 0.35, cy - r * 0.85)], r * 0.55)
    stones = sum(sh.mask_circle(cx + dx * r, cy + dy * r, r * 0.12) for dx, dy in ((-0.45, 0.45), (-0.05, -0.05), (0.2, -0.5)))
    return np.clip(outer - np.clip(stones, 0, 1), 0, 1)


def g_lot(sh, cx, cy, r):
    plot = sh.mask_poly([(cx, cy - r * 0.2), (cx + r * 0.9, cy + r * 0.25), (cx, cy + r * 0.7), (cx - r * 0.9, cy + r * 0.25)])
    inner = sh.mask_poly([(cx, cy - r * 0.02), (cx + r * 0.55, cy + r * 0.25), (cx, cy + r * 0.52), (cx - r * 0.55, cy + r * 0.25)])
    stake = sh.mask_rrect(cx - r * 0.06, cy - r * 0.9, r * 0.12, r * 1.0, r * 0.03)
    sign = sh.mask_rrect(cx - r * 0.45, cy - r * 0.9, r * 0.9, r * 0.45, r * 0.08)
    return np.clip(np.maximum.reduce([plot - inner, stake, sign]), 0, 1)


def g_bench(sh, cx, cy, r):
    back = sh.mask_rrect(cx - r * 0.85, cy - r * 0.55, r * 1.7, r * 0.3, r * 0.08)
    seat = sh.mask_rrect(cx - r * 0.85, cy, r * 1.7, r * 0.25, r * 0.08)
    legs = np.maximum.reduce([sh.mask_rrect(cx + dx * r - r * 0.08, cy - r * 0.3, r * 0.16, r * 1.0, r * 0.03)
                              for dx in (-0.6, 0.6)])
    return np.maximum.reduce([back, seat, legs])


def g_obelisk(sh, cx, cy, r):
    shaft = sh.mask_poly([(cx - r * 0.3, cy + r * 0.6), (cx - r * 0.18, cy - r * 0.6), (cx, cy - r * 0.95), (cx + r * 0.18, cy - r * 0.6),
                          (cx + r * 0.3, cy + r * 0.6)])
    base = sh.mask_rrect(cx - r * 0.6, cy + r * 0.55, r * 1.2, r * 0.35, r * 0.06)
    return np.maximum(shaft, base)


def g_flower(sh, cx, cy, r):
    m = sh.mask_circle(cx, cy - r * 0.25, r * 0.18)
    ring = 0
    for k in range(5):
        a = 2 * math.pi * k / 5 - math.pi / 2
        ring = np.maximum(ring, sh.mask_circle(cx + math.cos(a) * r * 0.38, cy - r * 0.25 + math.sin(a) * r * 0.38, r * 0.24))
    stem = sh.mask_line([(cx, cy + r * 0.1), (cx, cy + r * 0.9)], r * 0.12)
    leaf = sh.mask_ellipse(cx, cy + r * 0.35, r * 0.5, r * 0.22)
    return np.clip(np.maximum.reduce([ring - m, stem, leaf]), 0, 1)


def g_tree(sh, cx, cy, r):
    crown = np.maximum.reduce([sh.mask_circle(cx + dx * r, cy + dy * r, rr * r)
                               for dx, dy, rr in ((0, -0.4, 0.5), (-0.38, -0.12, 0.38), (0.38, -0.12, 0.38))])
    trunk = sh.mask_poly([(cx - r * 0.1, cy), (cx + r * 0.1, cy), (cx + r * 0.16, cy + r * 0.9), (cx - r * 0.16, cy + r * 0.9)])
    return np.maximum(crown, trunk)


def g_bridge(sh, cx, cy, r):
    arch = sh.mask_line(_arc_pts(cx, cy + r * 0.55, r * 0.95, math.radians(200), math.radians(340)), r * 0.2)
    deck = sh.mask_line(_arc_pts(cx, cy + r * 1.1, r * 1.35, math.radians(215), math.radians(325)), r * 0.16)
    posts = np.maximum.reduce([sh.mask_line([(cx + dx * r, cy - r * 0.05 - (0.2 - abs(dx) * 0.25) * r),
                                             (cx + dx * r, cy + r * 0.3)], r * 0.1) for dx in (-0.5, 0, 0.5)])
    return np.maximum.reduce([arch, deck, posts])


def _arc_pts(cx, cy, r, a0, a1, n=24):
    return [(cx + math.cos(a0 + (a1 - a0) * k / n) * r, cy + math.sin(a0 + (a1 - a0) * k / n) * r) for k in range(n + 1)]


# slot: hit centre and radius, where drawn, hover cut, glyph, colour
AMENITY_SLOTS = [
    ((302, 563, 20), (274, 536), (0, 0, 60, 56), g_washer, rgb("#7fbfe8")),
    ((333, 526, 20), (302, 504), (0, 100, 64, 50), g_path, rgb("#e2cf9c")),
    ((363, 563, 20), (334, 538), (0, 200, 62, 50), g_lot, rgb("#9ad07c")),
    ((470, 575, 18), (449, 553), (300, 0, 51, 47), g_bench, rgb("#d6a878")),
    ((591, 575, 18), (571, 553), (300, 100, 51, 45), g_obelisk, rgb("#c7c0d8")),
    ((652, 575, 18), (632, 553), (300, 150, 51, 47), g_flower, rgb("#f2a6c4")),
    ((714, 575, 18), (693, 553), (300, 200, 51, 45), g_tree, rgb("#86c46a")),
    ((248, 567, 12), (234, 553), (0, 450, 36, 36), None, rgb("#e8dcb8")),
    ((781, 563, 15), (762, 545), (0, 500, 38, 36), g_info, rgb("#9fc8f0")),
    ((531, 580, 12), (510, 553), (300, 50, 51, 45), g_bridge, rgb("#b8a088")),
]


def amenities_panel():
    sh = Sheet(800, 600)
    body = sh.mask_rrect(215, 488, 620, 140, 40)
    sh.paint(body, sh.vgrad(rgb("#3e6588"), rgb("#1d3650"), 488, 600), 0.78)
    sh.paint(body - sh.mask_rrect(218, 491, 620, 140, 38), rgb("#a9cdeb"), 0.8)
    sh.paint(sh.mask_rrect(250, 493, 540, 6, 3), (1, 1, 1), 0.12)
    # the tray's own buttons: the current mode (gold), back to the terrain
    glossy_disc(sh, 237, 525, 17, GOLD, g_shovel, glyph_col=INK)
    glossy_disc(sh, 269, 500, 15, BLUE, g_flag)
    glossy_disc(sh, 17, 417, 15, rgb("#6fb3ee"), g_flag, state="lit")
    for k, ((hx, hy, r), (dx, dy), cut, glyph, col) in enumerate(AMENITY_SLOTS):
        g = glyph if glyph is not None else g_turn(-1)
        if k != 8:
            glossy_disc(sh, hx, hy, r, col, g, glyph_col=INK)
        # the looks at the cut: hovered, selected, disabled
        ox, oy = hx - dx, hy - dy
        for j, st in enumerate(("lit", "sel", "off")):
            c = col if st != "sel" else GOLD
            glossy_disc(sh, cut[0] + 100 * j + ox, cut[1] + oy, r + (1 if st == "lit" else 0), c, g, glyph_col=INK,
                        state="normal" if st == "sel" else st)
    # the design strip: a glass shelf in pieces
    for (x, w, left, right) in ((380, 68, True, False), (450, 47, False, False), (500, 67, False, True), (600, 67, False, True)):
        strip_piece(sh, x, 250, w, 79, left, right)
    for y, h in ((350, 40), (400, 41)):
        for x, st in ((300, "lit"), (400, "sel"), (500, "normal"), (600, "normal")):
            design_tile(sh, x, y, 47, h, st)
    rgb_, a = sheet_out(sh)
    save_pair("Interface/AmenitiesPanel.png", "Interface/AmenitiesPanel_A.png", rgb_, a, (248, 0, 248))


def strip_piece(sh, x, y, w, h, left, right):
    """One piece of the design strip: a band from y + 10 to y + h - 8, rounded at a left or right end."""
    top, bot = y + 10, y + h - 8
    ext_l = 0 if left else 30
    ext_r = 0 if right else 30
    m = sh.mask_rrect(x - ext_l, top, w + ext_l + ext_r, bot - top, 20) * sh.mask_rrect(x, y, w, h, 0)
    sh.paint(m, sh.vgrad(rgb("#4d78a0"), rgb("#24425f"), top, bot), 0.88)
    rim = m - sh.mask_rrect(x - ext_l + 3, top + 3, w + ext_l + ext_r - 6, bot - top - 6, 17) * sh.mask_rrect(x, y, w, h, 0)
    sh.paint(np.clip(rim, 0, 1), rgb("#a9cdeb"), 0.85)


def design_tile(sh, x, y, w, h, state):
    rim = {"lit": rgb("#ffffff"), "sel": GOLD, "normal": rgb("#9fbad3")}[state]
    m = sh.mask_rrect(x + 2, y + 2, w - 4, h - 4, 8)
    sh.paint(m, rim, 0.95)
    sh.paint(sh.mask_rrect(x + 4, y + 4, w - 8, h - 8, 6),
             sh.vgrad(rgb("#e9f2fa") if state != "normal" else rgb("#c9d9e8"), rgb("#9db7cf"), y, y + h), 0.9)


# ---- the Elevation panel (Interface/ElevationPanel.pcx and its alpha) --------------------------------------------------------
# panels_ui.rs ELEVATION_TOOLS: vertex, square and area drawn at (312, 501), (429, 501), (546, 501), 88 x 99, hovered at
# (0, 0), (0, 100), (0, 200) and selected 100 right; Analyze Golf Shot at (688, 501), 72 x 99, hovered (400, 0) and selected
# (500, 0); back lit (0, 400, 41, 44) at (254, 483); Undo hovered (0, 450, 34, 40) and armed (50, 450) at (253, 546).

def mini_land(sh, cx, cy, heights, tw=9.0, th=4.6, hs=1.0):
    """A little iso patch of ground: heights is an (n+1) x (n+1) grid of vertex heights in pixels, drawn back to front."""
    n = heights.shape[0] - 1
    o = n / 2

    def pt(i, j):
        return (cx + (i - j) * tw, cy + (i + j - 2 * o) * th - heights[i, j] * hs)
    # the skirt: the patch's front edges dropped to a base
    front = [pt(n, j) for j in range(n + 1)][::-1] + [pt(i, n) for i in range(n, -1, -1)][::-1]
    skirt = [pt(0, n), pt(n, n), pt(n, 0)]
    base = [(x, y + 7) for x, y in skirt]
    sh.paint(sh.mask_poly(skirt + base[::-1]), rgb("#6b5236"))
    for i in range(n):
        for j in range(n):
            q = [pt(i, j), pt(i + 1, j), pt(i + 1, j + 1), pt(i, j + 1)]
            # light from the upper left: tiles rising to the left are lit
            slope = (heights[i + 1, j] + heights[i + 1, j + 1] - heights[i, j] - heights[i, j + 1]) + \
                (heights[i, j + 1] + heights[i + 1, j + 1] - heights[i, j] - heights[i + 1, j])
            k = float(np.clip(-slope * 0.025, -0.3, 0.3))
            col = lighten(rgb("#6dbb52"), k)
            m = sh.mask_poly(q)
            sh.paint(m, col)
            sh.paint(np.clip(m - sh.mask_poly(_shrink(q, 0.9)), 0, 1), lighten(col, 0.25), 0.6)
    del front


def _shrink(q, f):
    cx = sum(p[0] for p in q) / len(q)
    cy = sum(p[1] for p in q) / len(q)
    return [(cx + (x - cx) * f, cy + (y - cy) * f) for x, y in q]


def tool_land(kind):
    n = 4
    h = np.zeros((n + 1, n + 1))
    if kind == 0:
        h[2, 2] = 26
    elif kind == 1:
        h[2:4, 1:3] = 16
    else:
        h[1:4, 1:4] = 12
        h[2, 2] = 14
    return h


def g_up(sh, cx, cy, r):
    return np.maximum(sh.mask_poly([(cx, cy - r), (cx + r * 0.75, cy - r * 0.15), (cx - r * 0.75, cy - r * 0.15)]),
                      sh.mask_rrect(cx - r * 0.25, cy - r * 0.3, r * 0.5, r * 1.1, r * 0.08))


def g_target(sh, cx, cy, r):
    ring = np.clip(sh.mask_circle(cx, cy, r * 0.9) - sh.mask_circle(cx, cy, r * 0.62), 0, 1)
    dot = sh.mask_circle(cx, cy, r * 0.28)
    arc = sh.mask_line(_arc_pts(cx - r * 0.2, cy + r * 0.9, r * 1.4, math.radians(200), math.radians(300)), r * 0.14)
    return np.maximum.reduce([ring, dot, arc * 0])


def tool_plate(sh, x, y, w, h, state):
    if state == "normal":
        return
    rim = {"lit": rgb("#ffffff"), "sel": GOLD}[state]
    m = sh.mask_rrect(x + 3, y + 22, w - 6, h - 26, 16)
    sh.shadow(m, 1, 2, 2, 0.3)
    sh.paint(m, rim, 0.95)
    sh.paint(sh.mask_rrect(x + 6, y + 25, w - 12, h - 32, 13), sh.vgrad(rgb("#6d9cc6"), rgb("#355f86"), y + 25, y + h), 0.9)


def elevation_tool(sh, x, y, kind, state):
    w = 72 if kind == 3 else 88
    tool_plate(sh, x, y, w, 99, state)
    if kind == 3:
        glossy_disc(sh, x + 27, y + 46, 16, rgb("#f2d16b") if state != "normal" else rgb("#9fc8f0"), g_target, glyph_col=INK)
        return
    mini_land(sh, x + 44, y + 64, tool_land(kind))
    glossy_disc(sh, x + 70, y + 36, 10, rgb("#f7ecc8"), g_up, glyph_col=INK)


ELEV_AT = [(312, 501), (429, 501), (546, 501), (688, 501)]
ELEV_CUTS = [((0, 0), (100, 0)), ((0, 100), (100, 100)), ((0, 200), (100, 200)), ((400, 0), (500, 0))]


def elevation_panel():
    sh = Sheet(800, 600)
    body = sh.mask_rrect(215, 488, 620, 140, 40)
    sh.paint(body, sh.vgrad(rgb("#3e6588"), rgb("#1d3650"), 488, 600), 0.78)
    sh.paint(body - sh.mask_rrect(218, 491, 620, 140, 38), rgb("#a9cdeb"), 0.8)
    sh.paint(sh.mask_rrect(250, 493, 540, 6, 3), (1, 1, 1), 0.12)
    glossy_disc(sh, 237, 525, 17, GOLD, g_shovel, glyph_col=INK)
    glossy_disc(sh, 270, 499, 15, BLUE, g_house)
    glossy_disc(sh, 267, 563, 12, rgb("#e8dcb8"), g_turn(-1), glyph_col=INK)
    for k, (x, y) in enumerate(ELEV_AT):
        elevation_tool(sh, x, y, k, "normal")
        (hx, hy), (sx, sy) = ELEV_CUTS[k]
        elevation_tool(sh, hx, hy, k, "lit")
        elevation_tool(sh, sx, sy, k, "sel")
        if k < 3:
            elevation_tool(sh, hx + 200, hy, k, "normal")
            elevation_tool(sh, hx + 300, hy, k, "lit")
        else:
            elevation_tool(sh, 600, 0, k, "normal")
            elevation_tool(sh, 700, 0, k, "lit")
    # back lit at (0, 400) drawn at (254, 483): its centre at (270, 499)
    glossy_disc(sh, 16, 416, 15, rgb("#6fb3ee"), g_house, state="lit")
    glossy_disc(sh, 66, 416, 15, BLUE, g_house)
    for k, (st, col) in enumerate((("lit", rgb("#f7ecc8")), ("normal", GOLD), ("normal", rgb("#e8dcb8")), ("off", GREY))):
        glossy_disc(sh, [0, 50, 101, 151][k] + 14, 465, 12, col, g_turn(-1), glyph_col=INK, state=st)
    rgb_, a = sheet_out(sh)
    save_pair("Interface/ElevationPanel.png", "Interface/ElevationPanel_A.png", rgb_, a, (255, 0, 255))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="")
    a = ap.parse_args()
    jobs = [("dock", dock), ("courseinfo", courseinfo), ("title", title), ("terrainpanel", terrain_panel),
            ("transpopups", trans_popups), ("coursereport", course_report), ("okstates", ok_states),
            ("buildingpanel", building_panel), ("amenitiespanel", amenities_panel),
            ("elevationpanel", elevation_panel),
            ("parklink", lambda: emblem_sheet("Interface/parklink.png", ["park", "links"])),
            ("tropdesert", lambda: emblem_sheet("Interface/tropdesert.png", ["desert", "trop"]))]
    for theme, rel in (("park", "ParkLandTerrainButtons"), ("desert", "DesertTerrainButtons"), ("trop", "TropicalTerrainButtons"),
                       ("links", "LinksTerrainButtons")):
        jobs.append((rel, lambda t=theme, r=rel: terrain_buttons(t, "Interface/%s.png" % r)))
    for theme, rel in (("park", "parklands"), ("desert", "desert"), ("trop", "trop"), ("links", "links")):
        jobs.append((rel + " layout", lambda t=theme, r=rel: layout_sheet(t, "Interface/%s layout.png" % r)))
    for name, f in jobs:
        if a.only and not re.search(a.only, name, re.I):
            continue
        f()
        print("ok", name, flush=True)
    import build
    build.write_index()


if __name__ == "__main__":
    main()
