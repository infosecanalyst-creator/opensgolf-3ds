"""The free set's title menu (Interface/TitleBASE, TitleUnSel, TitleMO): a sunny course scene, the OpenSGolf logotype and
five big menu plates with round icon medallions, Start New Game the hero. The plates sit inside the engine's hit boxes
(render.rs MENU_BTN); the engine writes the labels at FREE_MENU_LABELS (render.rs, the free set only). Everything at rest is
in TitleBASE (soft edges); TitleMO holds each plate lit, one connected shape per box; TitleUnSel is left empty."""
import math

import numpy as np
from PIL import Image, ImageDraw, ImageFont

import uiart
from uiart import S, INK, Sheet, g_ball_tee, g_cards, g_exit, g_folder, g_shovel, g_trophy, lighten, mix, rgb

MENU_BTN = [(40, 25, 325, 120), (415, 40, 360, 135), (12, 395, 285, 100), (508, 380, 270, 95), (285, 478, 285, 105),
            (718, 528, 64, 64)]
# each plate: (x, y, w, h) of the pill, the medallion's centre and radius; keep in step with render.rs FREE_MENU_LABELS
PLATES = [((86, 52, 268, 66), (88, 85, 42)),
          ((474, 62, 292, 92), (474, 108, 56)),
          ((54, 418, 232, 58), (56, 447, 37)),
          ((548, 402, 222, 58), (550, 431, 37)),
          ((326, 500, 236, 60), (328, 530, 38))]
EXIT = ((750, 560), 26)
GLYPHS = [g_folder, g_ball_tee, g_shovel, g_cards, g_trophy]
# (top, bottom, edge) of each plate's body: the hero in sunny orange, the rest in fairway green
HERO = ("#ffd44f", "#f58a1f", "#a8500a")
GREEN = ("#6fd067", "#2c9443", "#1a5e2a")
HERO_LIT = ("#fff07a", "#ffb02e", "#b8600c")
GREEN_LIT = ("#a6f07a", "#46b852", "#1f6e2e")
MEDAL = ["#3a8fd8", "#e2453a", "#c98a2a", "#8a5cd0", "#2f9fb0"]


def text_mask(sh, text, x, y, size, anchor="mm", font=None, stroke=0):
    m = Image.new("L", (sh.w * S, sh.h * S), 0)
    f = ImageFont.truetype(font or uiart.FONT, int(size * S))
    ImageDraw.Draw(m).text((x * S, y * S), text, font=f, fill=255, anchor=anchor, stroke_width=int(stroke * S), stroke_fill=255)
    return np.asarray(m, np.float32) / 255.0


def text_width(text, size, font=None):
    return ImageFont.truetype(font or uiart.FONT, int(size * S)).getlength(text) / S


def soft(sh, m, blur):
    img = Image.fromarray((np.clip(m, 0, 1) * 255).astype(np.uint8)).filter(__import__("PIL.ImageFilter").ImageFilter.GaussianBlur(blur * S))
    return np.asarray(img, np.float32) / 255.0


# ---- the scene ------------------------------------------------------------------------------------------------------------------

def hill(x, base, amp, f1, p1, amp2=8, f2=57, p2=0.0):
    return base + amp * math.sin(x / f1 + p1) + amp2 * math.sin(x / f2 + p2)


def scene(sh):
    W, H = 800, 600
    full = np.ones((H * S, W * S), np.float32)
    sh.paint(full, sh.vgrad(rgb("#3d9be6"), rgb("#cdeefa"), 0, 330))
    # the sun behind the hero plate, its glow and soft rays
    sun = sh.mask_circle(660, 40, 46)
    sh.paint(soft(sh, sh.mask_circle(660, 40, 120), 40), rgb("#fff6d0"), 0.55)
    yy, xx = np.mgrid[0:H * S, 0:W * S] / S
    ang = np.arctan2(yy - 40, xx - 660)
    rays = (np.sin(ang * 14) > 0.55).astype(np.float32) * np.clip(1 - np.hypot(xx - 660, yy - 40) / 420, 0, 1)
    sh.paint(rays * (yy < 330), (1, 1, 1), 0.10)
    sh.paint(sun, rgb("#fffbe8"))
    # clouds
    rng = np.random.default_rng(21)
    for cx, cy, r in ((150, 170, 34), (380, 150, 26), (260, 60, 22), (560, 200, 30), (730, 230, 24), (60, 260, 20)):
        blobs = [(0, 0, 1), (-0.95, 0.25, 0.72), (1.0, 0.22, 0.78), (0.45, -0.38, 0.75), (-0.4, -0.25, 0.6)]
        cl = np.clip(sum(sh.mask_circle(cx + dx * r, cy + dy * r, rr * r) for dx, dy, rr in blobs), 0, 1)
        sh.paint(cl * sh.mask_rrect(cx - 3 * r, cy - 2 * r, 6 * r, 2.2 * r, 0), rgb("#ffffff"), 0.92)
        sh.paint(cl * np.clip((yy - cy) / r, 0, 1), rgb("#c9dcee"), 0.6)
    # far hills, hazy, with a tree line and the clubhouse on the left one
    far = lambda x: hill(x, 300, 16, 140, 0.4, 7, 47, 1.0)
    pts = [(x, far(x)) for x in range(-10, 811, 6)] + [(810, 610), (-10, 610)]
    sh.paint(sh.mask_poly(pts), rgb("#9fcf96"))
    for k in range(40):
        x = rng.uniform(0, 800)
        if 50 < x < 170:
            continue
        r = rng.uniform(5, 10)
        sh.paint(sh.mask_circle(x, far(x) - r * 0.6, r), rgb("#7fb880") if k % 2 else rgb("#6eaa74"))
    clubhouse(sh, 110, far(110) + 3)
    # the middle hills
    mid = lambda x: hill(x, 340, 20, 190, 2.2, 8, 61, 0.3)
    sh.paint(sh.mask_poly([(x, mid(x)) for x in range(-10, 811, 6)] + [(810, 610), (-10, 610)]), rgb("#79c25a"))
    for k in range(22):
        x = rng.uniform(0, 800)
        r = rng.uniform(9, 16)
        y = mid(x) - 2
        sh.paint(sh.mask_rrect(x - 1.5, y - r * 0.5, 3, r * 0.8, 1), rgb("#6b4a2e"))
        sh.paint(sh.mask_circle(x, y - r, r * 0.8), rgb("#3f8f3a") if k % 3 else rgb("#2f7a35"))
        sh.paint(sh.mask_circle(x - r * 0.25, y - r * 1.25, r * 0.35), rgb("#6fb25a"), 0.6)
    # the near course: a fairway sweeping to a green with its flag at the right, a pond and a bunker
    near = lambda x: hill(x, 382, 14, 260, 4.1, 6, 71, 2.0)
    land = sh.mask_poly([(x, near(x)) for x in range(-10, 811, 6)] + [(810, 610), (-10, 610)])
    sh.paint(land, sh.vgrad(rgb("#68b84a"), rgb("#3f8f37"), 370, 600))
    fair = sh.mask_poly([(-10, 600), (180, 600), (420, 470), (640, 395), (760, 372), (800, 372), (800, 352), (720, 352),
                         (560, 372), (300, 450), (-10, 560)])
    sh.paint(fair * land, rgb("#8fd463"))
    for k in range(14):
        x0 = -200 + k * 80
        stripe = sh.mask_poly([(x0, 610), (x0 + 40, 610), (x0 + 520, 330), (x0 + 480, 330)])
        sh.paint(stripe * fair * land, (1, 1, 1), 0.08)
    green = sh.mask_ellipse(640, 352, 150, 40)
    sh.paint(green, rgb("#a6e27a"))
    sh.paint(np.clip(green - sh.mask_ellipse(646, 356, 138, 32), 0, 1), rgb("#6fbb4f"), 0.6)
    sh.paint(sh.mask_ellipse(694, 368, 10, 4), rgb("#24401f"))
    sh.paint(sh.mask_rrect(697.5, 300, 2.5, 70, 1), rgb("#f4f4f4"))
    flag = sh.mask_poly([(700, 300), (728, 306), (724, 312), (729, 318), (700, 320)])
    sh.paint(flag, rgb("#e8402f"))
    sh.paint(sh.mask_ellipse(600, 386, 46, 14), rgb("#f2e3b0"))
    sh.paint(np.clip(sh.mask_ellipse(600, 386, 46, 14) - sh.mask_ellipse(602, 389, 42, 11), 0, 1), rgb("#c9b47a"), 0.7)
    pond = sh.mask_ellipse(20, 368, 150, 30)
    sh.paint(pond, sh.vgrad(rgb("#6cc4ee"), rgb("#2f88c8"), 368, 398))
    sh.paint(pond * sh.mask_rrect(40, 374, 70, 2, 1), (1, 1, 1), 0.5)
    sh.paint(pond * sh.mask_rrect(80, 384, 50, 2, 1), (1, 1, 1), 0.35)
    # a soft vignette and light along the bottom
    vig = np.clip((np.hypot((xx - 400) / 520, (yy - 300) / 420) - 0.62) * 1.6, 0, 1)
    sh.paint(vig, rgb("#0b2a20"), 0.35)


def clubhouse(sh, cx, base):
    """A small clubhouse on the far hill, in the haze's colours: a long low house, a roof, a little tower with a flag."""
    wall, roof, win = rgb("#e8eef0"), rgb("#7f8fa8"), rgb("#8aa2c0")
    sh.paint(sh.mask_rrect(cx - 46, base - 22, 92, 22, 1), wall)
    sh.paint(sh.mask_poly([(cx - 52, base - 21), (cx - 38, base - 34), (cx + 38, base - 34), (cx + 52, base - 21)]), roof)
    sh.paint(sh.mask_rrect(cx - 8, base - 50, 16, 30, 1), wall)
    sh.paint(sh.mask_poly([(cx - 11, base - 49), (cx, base - 60), (cx + 11, base - 49)]), roof)
    for k in range(6):
        sh.paint(sh.mask_rrect(cx - 38 + k * 14, base - 15, 7, 8, 1), win)
    sh.paint(sh.mask_rrect(cx - 0.6, base - 74, 1.2, 15, 0.5), rgb("#6a7480"))
    sh.paint(sh.mask_poly([(cx + 0.6, base - 74), (cx + 10, base - 71), (cx + 0.6, base - 68)]), rgb("#e05a4a"))


# ---- the logotype --------------------------------------------------------------------------------------------------------------

def logotype(sh, cx, cy, k=1.0):
    """'Open' on a tilted navy tag over a big 'SGolf' whose o is a golf ball and whose l is a flagstick, a tagline under it."""
    size = 118 * k
    parts = [("S", None), ("G", None), ("o", "ball"), ("l", "pole"), ("f", None)]
    widths = []
    for ch, kind in parts:
        widths.append(size * 0.58 if kind == "ball" else (size * 0.2 if kind == "pole" else text_width(ch, size)))
    gap = -2 * k
    total = sum(widths) + gap * (len(parts) - 1)
    x = cx - total / 2
    base = cy + size * 0.28
    letters = np.zeros((sh.h * S, sh.w * S), np.float32)
    ball_at = pole_at = None
    for (ch, kind), w in zip(parts, widths):
        if kind == "ball":
            ball_at = (x + w / 2, base - size * 0.25, size * 0.27)
        elif kind == "pole":
            pole_at = (x + w / 2, base)
        else:
            letters = np.maximum(letters, text_mask(sh, ch, x, base, size, "ls"))
        x += w + gap
    # the outline layers: a deep green edge, a white band, then the letters' gradient
    outer = soft(sh, letters, 0.0)
    for r_px, col in ((9 * k, rgb("#14421f")), (6 * k, rgb("#ffffff"))):
        grown = np.asarray(Image.fromarray((letters * 255).astype(np.uint8)).filter(
            __import__("PIL.ImageFilter").ImageFilter.MaxFilter(int(r_px * S) * 2 + 1)), np.float32) / 255
        if col[0] < 0.5:
            sh.shadow(grown, 3 * k, 6 * k, 5 * k, 0.45)
        sh.paint(grown, col)
    sh.paint(outer, sh.vgrad(rgb("#9ae86a"), rgb("#1f8a3a"), base - size * 0.75, base))
    sh.paint(outer * (np.arange(sh.h * S)[:, None] / S < base - size * 0.42), (1, 1, 1), 0.18)
    # the ball, ringed like the letters
    bx, by, br = ball_at
    for rr, col in ((br + 9 * k, rgb("#14421f")), (br + 6 * k, rgb("#ffffff"))):
        sh.paint(sh.mask_circle(bx, by, rr), col)
    ball = sh.mask_circle(bx, by, br)
    sh.paint(ball, sh.rgrad((1, 1, 1), rgb("#b9c8d6"), bx - br * 0.35, by - br * 0.4, br * 1.6))
    for gy in range(-3, 4):
        for gx in range(-3, 4):
            px, py = bx + gx * br * 0.3 + (br * 0.15 if gy % 2 else 0), by + gy * br * 0.27
            d = math.hypot(px - bx, py - by) / br
            if d < 0.86:
                sh.paint(ball * sh.mask_circle(px, py, br * 0.07 * (1 - d * d) ** 0.4), rgb("#a7b8c8"), 0.75)
    # the flagstick standing in for the l, its pennant waving out over the f
    px, py = pole_at
    top = py - size * 0.95
    for wdt, col in ((10 * k, rgb("#14421f")), (7 * k, rgb("#ffffff"))):
        sh.paint(sh.mask_rrect(px - wdt / 2, top - (wdt - 3 * k) / 2, wdt, py - top + (wdt - 3 * k) / 2, wdt / 2), col)
    sh.paint(sh.mask_rrect(px - 2 * k, top, 4 * k, py - top, 2 * k), rgb("#f4f4f4"))
    pen = sh.mask_poly([(px + 2 * k, top), (px + 46 * k, top + 9 * k), (px + 38 * k, top + 17 * k), (px + 48 * k, top + 26 * k),
                        (px + 2 * k, top + 32 * k)])
    sh.shadow(pen, 2 * k, 3 * k, 2 * k, 0.35)
    sh.paint(pen, sh.vgrad(rgb("#ff6a4a"), rgb("#d0302a"), top, top + 32 * k))
    # 'Open' on its tag, tilted, over the S and G
    tw = text_width("OPEN", 30 * k) + 34 * k
    tag = Sheet(int(tw + 20 * k), int(60 * k))
    tm = tag.mask_rrect(10 * k, 10 * k, tw, 40 * k, 12 * k)
    tag.shadow(tm, 2 * k, 3 * k, 3 * k, 0.4)
    tag.paint(tm, rgb("#ffffff"))
    tag.paint(tag.mask_rrect(13 * k, 13 * k, tw - 6 * k, 34 * k, 9.5 * k), tag.vgrad(rgb("#2f6fd0"), rgb("#1b3f8a"), 10 * k, 50 * k))
    tag.paint(text_mask(tag, "OPEN", 10 * k + tw / 2, 31 * k, 30 * k), (1, 1, 1))
    rot = Image.fromarray((np.dstack([tag.rgb, tag.a[..., None]]) * 255).clip(0, 255).astype(np.uint8), "RGBA").rotate(
        7, resample=Image.BICUBIC, expand=True)
    arr = np.asarray(rot, np.float32) / 255
    sub = Sheet(arr.shape[1] // S + 1, arr.shape[0] // S + 1)
    sub.rgb[:arr.shape[0], :arr.shape[1]] = arr[..., :3]
    sub.a[:arr.shape[0], :arr.shape[1]] = arr[..., 3]
    sh.paste(sub, cx - total / 2 - 22 * k, base - size * 1.12)
    # the tagline
    t = "free golf course designer"
    lw = text_width(t, 21 * k, uiart.TEXT_FONT) + 40 * k
    pill = sh.mask_rrect(cx - lw / 2, base + 18 * k, lw, 34 * k, 17 * k)
    sh.shadow(pill, 1.5 * k, 3 * k, 3 * k, 0.4)
    sh.paint(pill, rgb("#ffffff"))
    sh.paint(sh.mask_rrect(cx - lw / 2 + 2.5 * k, base + 20.5 * k, lw - 5 * k, 29 * k, 14.5 * k),
             sh.vgrad(rgb("#24508f"), rgb("#152e5a"), base + 18 * k, base + 52 * k))
    sh.paint(text_mask(sh, t, cx, base + 35 * k, 21 * k, font=uiart.TEXT_FONT), rgb("#ffe68a"))


# ---- the plates ------------------------------------------------------------------------------------------------------------------

def plate(sh, k, lit=False, hard=False):
    """Menu plate k: a thick glossy pill with a round icon medallion at its left end."""
    (x, y, w, h), (mx, my, mr) = PLATES[k]
    top, bot, edge = (HERO_LIT if lit else HERO) if k == 1 else (GREEN_LIT if lit else GREEN)
    depth = 6 if k == 1 else 5
    body = sh.mask_rrect(x, y, w, h, h / 2)
    under = sh.mask_rrect(x, y + depth, w, h, h / 2)
    medal = sh.mask_circle(mx, my, mr + 5)
    if not hard:
        sh.shadow(np.maximum(under, sh.mask_circle(mx, my + depth, mr + 5)), 3, 8, 8, 0.42)
    sh.paint(under, rgb(edge))
    sh.paint(body, rgb("#ffffff") if lit else lighten(rgb(edge), -0.1))
    inner = sh.mask_rrect(x + 3, y + 3, w - 6, h - 6, h / 2 - 3)
    sh.paint(inner, sh.vgrad(rgb(top), rgb(bot), y, y + h))
    gloss = sh.mask_rrect(x + h * 0.5, y + 5, w - h * 0.9, h * 0.36, h * 0.18)
    sh.paint(gloss * inner, sh.vgrad((1, 1, 1), rgb(top), y + 5, y + 5 + h * 0.36), 0.55)
    sh.paint(inner * sh.mask_rrect(x, y + h * 0.72, w, h, 0), rgb(edge), 0.18)
    # the medallion
    sh.paint(sh.mask_circle(mx, my + depth, mr + 5), rgb(edge))
    sh.paint(medal, rgb("#ffffff"))
    disc = sh.mask_circle(mx, my, mr)
    col = rgb(MEDAL[k])
    if lit:
        col = lighten(col, 0.2)
    sh.paint(disc, sh.rgrad(lighten(col, 0.35), lighten(col, -0.25), mx - mr * 0.35, my - mr * 0.45, mr * 1.6))
    sh.paint(disc * sh.mask_ellipse(mx - mr * 0.7, my - mr * 0.92, mr * 1.4, mr * 0.85), (1, 1, 1), 0.3)
    if k == 1:
        ball_buddy(sh, mx, my, mr, lit)
    else:
        g = GLYPHS[k](sh, mx, my + mr * 0.03, mr * 0.6)
        if not hard:
            sh.shadow(g, 0.8, 1.5, 1.0, 0.4)
        sh.paint(g, rgb("#ffffff"))
    if k == 2:
        gopher(sh, x + w - 58, y)


def exit_button(sh, lit=False, hard=False):
    (cx, cy), r = EXIT
    if not hard:
        sh.shadow(sh.mask_circle(cx, cy + 4, r + 4), 2, 5, 5, 0.4)
    sh.paint(sh.mask_circle(cx, cy + 4, r + 4), rgb("#5a1a14"))
    sh.paint(sh.mask_circle(cx, cy, r + 4), rgb("#ffffff"))
    col = rgb("#ff7a5a") if lit else rgb("#e0453a")
    d = sh.mask_circle(cx, cy, r)
    sh.paint(d, sh.rgrad(lighten(col, 0.3), lighten(col, -0.3), cx - r * 0.35, cy - r * 0.45, r * 1.6))
    sh.paint(d * sh.mask_ellipse(cx - r * 0.7, cy - r * 0.92, r * 1.4, r * 0.85), (1, 1, 1), 0.3)
    sh.paint(g_exit(sh, cx, cy, r * 0.58), rgb("#ffffff"))


def title_sheets():
    base = Sheet(800, 600)
    scene(base)
    logotype(base, 404, 258, 1.18)
    for k in range(5):
        plate(base, k)
    exit_button(base)
    un = Sheet(800, 600)
    mo = Sheet(800, 600)
    for k in range(5):
        plate(mo, k, lit=True, hard=True)
    exit_button(mo, lit=True, hard=True)
    return base, un, mo


def ball_buddy(sh, cx, cy, r, lit):
    """The hero's medallion: a cheeky golf ball on a tee, winking (both eyes open when pointed at)."""
    tee = sh.mask_poly([(cx - r * 0.28, cy + r * 0.36), (cx + r * 0.28, cy + r * 0.36), (cx + r * 0.07, cy + r * 0.5),
                        (cx + r * 0.05, cy + r * 0.86), (cx - r * 0.05, cy + r * 0.86), (cx - r * 0.07, cy + r * 0.5)])
    sh.paint(tee, rgb("#ffd36a"))
    br = r * 0.5
    bx, by = cx, cy - r * 0.1
    ball = sh.mask_circle(bx, by, br)
    sh.shadow(ball, 1, 2, 1.5, 0.35)
    sh.paint(ball, sh.rgrad((1, 1, 1), rgb("#c8d4e0"), bx - br * 0.35, by - br * 0.4, br * 1.7))
    for dx, dy in ((-0.55, -0.55), (0.0, -0.75), (0.55, -0.55), (-0.75, 0.05), (0.75, 0.05)):
        sh.paint(ball * sh.mask_circle(bx + dx * br, by + dy * br, br * 0.08), rgb("#aebdca"), 0.8)
    ink = rgb("#26303c")
    sh.paint(sh.mask_ellipse(bx - br * 0.42, by - br * 0.3, br * 0.22, br * 0.34), ink)
    if lit:
        sh.paint(sh.mask_ellipse(bx + br * 0.2, by - br * 0.3, br * 0.22, br * 0.34), ink)
    else:
        sh.paint(sh.mask_line([(bx + br * 0.18, by - br * 0.1), (bx + br * 0.32, by - br * 0.2), (bx + br * 0.46, by - br * 0.1)], br * 0.1), ink)
    mouth = sh.mask_poly([(bx - br * 0.45, by + br * 0.18)] + [(bx + br * 0.45 * math.cos(a), by + br * 0.18 + br * 0.38 * math.sin(a))
                                                             for a in np.linspace(0, math.pi, 12)])
    sh.paint(mouth, ink)
    sh.paint(mouth * sh.mask_ellipse(bx - br * 0.2, by + br * 0.35, br * 0.4, br * 0.3), rgb("#ff7a6a"))
    for sgn in (-1, 1):
        sh.paint(sh.mask_circle(bx + sgn * br * 0.62, by + br * 0.18, br * 0.13), rgb("#ff9a8a"), 0.6)


def gopher(sh, cx, top):
    """A gopher peeking over the Sandbox plate, paws on its edge."""
    fur, belly, ink = rgb("#a8743e"), rgb("#e8c89a"), rgb("#26303c")
    head = sh.mask_ellipse(cx - 15, top - 22, 30, 28)
    for sgn in (-1, 1):
        sh.paint(sh.mask_circle(cx + sgn * 12, top - 19, 5.5), fur)
        sh.paint(sh.mask_circle(cx + sgn * 12, top - 19, 2.8), rgb("#e89a8a"))
    sh.paint(head, sh.rgrad(lighten(fur, 0.2), lighten(fur, -0.15), cx - 4, top - 16, 22))
    sh.paint(sh.mask_ellipse(cx - 8, top - 8, 16, 11), belly)
    for sgn in (-1, 1):
        sh.paint(sh.mask_circle(cx + sgn * 6, top - 13, 3.2), (1, 1, 1))
        sh.paint(sh.mask_circle(cx + sgn * 6 + 0.8, top - 12.6, 1.8), ink)
    sh.paint(sh.mask_ellipse(cx - 3, top - 9, 6, 4), rgb("#4a2a1a"))
    sh.paint(sh.mask_rrect(cx - 3, top - 4.5, 6, 5, 1), (1, 1, 1))
    sh.paint(sh.mask_rrect(cx - 0.3, top - 4.5, 0.6, 5, 0), rgb("#c8c8c8"))
    for sgn in (-1, 1):
        paw = sh.mask_ellipse(cx + sgn * 13 - 6, top - 3, 12, 7)
        sh.paint(paw, lighten(fur, 0.1))
        for k in range(3):
            sh.paint(paw * sh.mask_rrect(cx + sgn * 13 - 3.5 + k * 3, top + 1, 0.8, 3, 0), lighten(fur, -0.4))
