"""The free set's full screen pages, drawn as vector art (uiart.py) around the places the engine writes on them: Load
Previous Game and Pick A Pro (Title_LoadGame, Title_Pickapro, Title_LoadGame_MO), Select a Theme Pack (Title_ThemePacks and
its _MO strip) and the credits (creditsbckgrd). The positions are the engine's (crates/simgolf/src/files_ui.rs)."""
import math

import numpy as np

from uiart import (BLUE, CREAM, GOLD, GOLD_HI, GREEN, GREY, INK, NAVY, Sheet, g_exit, g_turn, glossy_disc, lighten, mix,
                   oval, rgb)

# ---- shared pieces ------------------------------------------------------------------------------------------------------------


def page_background(sh, warm=False, seed=3):
    """The title screen's sunny course, softly out of focus and veiled, so panels read well over it."""
    from PIL import Image, ImageFilter
    import uititle
    uititle.scene(sh)
    img = Image.fromarray((np.clip(sh.rgb, 0, 1) * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(3 * 3))
    sh.rgb[:] = np.asarray(img, np.float32) / 255
    sh.paint(np.ones((sh.h * 3, sh.w * 3), np.float32), (1, 1, 1), 0.16)
    if warm:
        sh.paint(np.ones((sh.h * 3, sh.w * 3), np.float32), rgb("#ffb060"), 0.2)


def glass(sh, x, y, w, h, r=16, tint="#ffffff", alpha=0.82, rim="#2c4a73", shadow=True):
    """A candy card: a chunky rounded panel on a coloured extrude with a white rim and a soft cream-to-sky body."""
    edge = rgb("#2a62b8")
    under = sh.mask_rrect(x, y + 5, w, h, r)
    if shadow:
        sh.shadow(under, 2, 5, 6, 0.35)
    sh.paint(under, lighten(edge, -0.35))
    sh.paint(sh.mask_rrect(x, y, w, h, r), edge)
    sh.paint(sh.mask_rrect(x + 3, y + 3, w - 6, h - 6, r - 3), rgb("#ffffff"))
    inner = sh.mask_rrect(x + 5, y + 5, w - 10, h - 10, r - 5)
    sh.paint(inner, sh.vgrad(rgb("#eaf5ff"), rgb("#bcd8f4"), y, y + h), alpha + 0.1)
    sh.paint(inner * sh.mask_rrect(x + 5, y + 5, w - 10, min(36, h * 0.2), r - 5), (1, 1, 1), 0.45)


def paper(sh, x, y, w, h, r=10, stripes=None):
    """A cream field the engine writes lists on, with a green candy rim; `stripes` (y0, pitch, n) shades every other row."""
    under = sh.mask_rrect(x, y + 4, w, h, r)
    sh.shadow(under, 1, 3, 4, 0.3)
    sh.paint(under, rgb("#1f6e34"))
    sh.paint(sh.mask_rrect(x, y, w, h, r), rgb("#2e8f45"))
    inner = sh.mask_rrect(x + 3, y + 3, w - 6, h - 6, r - 3)
    sh.paint(inner, sh.vgrad(rgb("#fffdf6"), rgb("#f8efd8"), y, y + h))
    if stripes:
        y0, pitch, n = stripes
        for k in range(0, n, 2):
            sh.paint(inner * sh.mask_rrect(x + 4, y0 + pitch * k - 1, w - 8, pitch, 3), rgb("#e6f2d8"), 0.8)
    sh.paint(np.clip(inner - sh.mask_rrect(x + 3, y + 6, w - 6, h - 6, r - 3), 0, 1), rgb("#b9a678"), 0.35)


def title_plate(sh, x, y, w, h):
    """The candy title plate: a cream pill on a green extrude with a white rim (the engine writes the title on it)."""
    under = sh.mask_rrect(x, y + 4, w, h, h / 2)
    sh.shadow(under, 2, 5, 5, 0.35)
    sh.paint(under, rgb("#1f6e34"))
    sh.paint(sh.mask_rrect(x, y, w, h, h / 2), rgb("#2e8f45"))
    sh.paint(sh.mask_rrect(x + 2.5, y + 2.5, w - 5, h - 5, h / 2 - 2.5), rgb("#ffffff"))
    sh.paint(sh.mask_rrect(x + 4.5, y + 4.5, w - 9, h - 9, h / 2 - 4.5), sh.vgrad(rgb("#fffdf4"), rgb("#f3e4b8"), y, y + h))
    sh.paint(sh.mask_rrect(x + h * 0.45, y + 6, w - h * 0.9, h * 0.28, h * 0.14), (1, 1, 1), 0.6)


def dark_pill(sh, x, y, w, h):
    """A deep blue candy strip for white text."""
    r = min(h / 2, 16)
    under = sh.mask_rrect(x, y + 4, w, h, r)
    sh.shadow(under, 1, 3, 4, 0.3)
    sh.paint(under, rgb("#0a1838"))
    sh.paint(sh.mask_rrect(x, y, w, h, r), rgb("#ffffff"))
    sh.paint(sh.mask_rrect(x + 2.5, y + 2.5, w - 5, h - 5, r - 2.5), sh.vgrad(rgb("#3a64b8"), rgb("#1a3270"), y, y + h))
    sh.paint(sh.mask_rrect(x + 12, y + 5, w - 24, h * 0.28, 5), (1, 1, 1), 0.18)


def g_tick(sh, cx, cy, r):
    return sh.mask_line([(cx - r * 0.6, cy), (cx - r * 0.15, cy + r * 0.5), (cx + r * 0.7, cy - r * 0.6)], r * 0.32)


def g_bin(sh, cx, cy, r):
    lid = sh.mask_rrect(cx - r * 0.75, cy - r * 0.7, r * 1.5, r * 0.22, r * 0.08)
    knob = sh.mask_rrect(cx - r * 0.22, cy - r * 0.9, r * 0.44, r * 0.25, r * 0.08)
    body = sh.mask_poly([(cx - r * 0.6, cy - r * 0.4), (cx + r * 0.6, cy - r * 0.4), (cx + r * 0.48, cy + r * 0.9), (cx - r * 0.48, cy + r * 0.9)])
    slots = sum(sh.mask_rrect(cx + dx * r - r * 0.06, cy - r * 0.2, r * 0.12, r * 0.9, r * 0.05) for dx in (-0.27, 0, 0.27))
    return np.clip(np.maximum.reduce([lid, knob, body]) - np.clip(slots, 0, 1), 0, 1)


def g_back(sh, cx, cy, r):
    return np.maximum(sh.mask_line([(cx - r * 0.35, cy), (cx + r * 0.7, cy)], r * 0.3),
                      sh.mask_poly([(cx - r * 0.85, cy), (cx - r * 0.2, cy - r * 0.6), (cx - r * 0.2, cy + r * 0.6)]))


def g_tri(up):
    def f(sh, cx, cy, r):
        s = -1 if up else 1
        return sh.mask_poly([(cx, cy + s * r * 0.6), (cx - r * 0.7, cy - s * r * 0.45), (cx + r * 0.7, cy - s * r * 0.45)])
    return f


OK_COL, DEL_COL, BACK_COL = rgb("#3f9e57"), rgb("#d0503e"), BLUE


def rail(sh, x, y, w, h):
    """The list's scroll rail between its arrow buttons."""
    m = sh.mask_rrect(x, y, w, h, w / 2)
    sh.paint(m, rgb("#2c4a73"), 0.85)
    sh.paint(sh.mask_rrect(x + 2, y + 2, w - 4, h - 4, w / 2 - 2), rgb("#c9d9ea"), 0.9)


# ---- Load Previous Game and Pick A Pro ---------------------------------------------------------------------------------------
# files_ui.rs: the title on (504, 42); the list's rows from (320, 116) 16 apart (16 rows) with the selection 310..766; the scroll
# thumb at x 781 from 147 to 375; the save's panel at the left (an emblem at (84, 32), its numbers to y 590); the theme lines
# in white at (388, 442..474); buttons (hit centre, radius, cut): OK (633, 555) (600, 520, 70, 70), Cancel (769, 556)
# (740, 530, 60, 60), up (784, 120) (778, 100, 22, 40), down (784, 400) (778, 380, 22, 40), Delete (704, 555) (670, 520,
# 70, 70); the _MO sheet holds the lit looks at the same cuts and the greyed OK and Delete at (600, 435) and (670, 435).

FILE_BUTTONS = [((633, 555), 26, OK_COL, g_tick), ((769, 556), 22, BACK_COL, g_back), ((704, 555), 23, DEL_COL, g_bin)]
SCROLL = [((789, 120), 10, True), ((789, 400), 10, False)]


def files_page(pro):
    sh = Sheet(800, 600)
    page_background(sh, seed=5 if pro else 3)
    title_plate(sh, 330, 34, 350, 36)
    paper(sh, 304, 106, 466, 286, stripes=(116, 16, 16))
    rail(sh, 780, 132, 14, 256)
    for (cx, cy), r, up in SCROLL:
        glossy_disc(sh, cx, cy, r, BLUE, g_tri(up))
    if pro:
        # the pro's portrait sits on a golf ball tee; his skills list and signature line under it
        glass(sh, 22, 22, 252, 566, r=20)
        sh.paint(sh.mask_circle(139, 131, 76), rgb("#2c4a73"), 0.85)
        sh.paint(sh.mask_circle(139, 131, 73), sh.rgrad(rgb("#ffffff"), rgb("#c9d6e2"), 120, 105, 95))
        paper(sh, 36, 248, 224, 278, stripes=(276, 24, 10))
        dark_pill(sh, 30, 540, 238, 44)
        dark_pill(sh, 380, 437, 380, 52)
    else:
        glass(sh, 22, 22, 252, 570, r=20)
        sh.paint(sh.mask_circle(124, 72, 44), rgb("#2c4a73"), 0.5)
        sh.paint(sh.mask_circle(124, 72, 42), rgb("#ffffff"), 0.6)
        paper(sh, 34, 118, 228, 70)
        paper(sh, 34, 188, 228, 76, stripes=(190, 18, 4))
        paper(sh, 34, 266, 228, 318, stripes=(284, 17, 18))
        dark_pill(sh, 380, 437, 380, 52)
    for (cx, cy), r, col, gl in FILE_BUTTONS:
        glossy_disc(sh, cx, cy, r, col, gl)
    return sh


def files_mo():
    sh = Sheet(800, 600)
    for (cx, cy), r, col, gl in FILE_BUTTONS:
        glossy_disc(sh, cx, cy, r + 1, GOLD, gl, glyph_col=INK, state="lit")
    for (cx, cy), r, up in SCROLL:
        glossy_disc(sh, cx, cy, r + 1, GOLD, g_tri(up), glyph_col=INK)
    # the greyed OK and Delete (no file chosen), 85 above their places
    for (cx, cy), r, col, gl in (FILE_BUTTONS[0], FILE_BUTTONS[2]):
        glossy_disc(sh, cx, cy - 85, r, GREY, gl, state="off")
    return sh


# ---- Select a Theme Pack -------------------------------------------------------------------------------------------------------
# files_ui.rs draw_themes: the title at (78, 34) (18 px), the column heads centred on 346 + 90 k at y 37 / 62, the packs' rows at
# y 92 + 32 i (13) with the name at 73; the cells 87 x 31 at x 303 + 90 k; OK (615, 555) and back (749, 556). The _MO strip
# (450 x 150): (1, 33, 221, 31) the current row's name plate, (1, 1) an empty cell of the current row, (89, 1) a filled one,
# (177, 1) a filled cell of another row, (265, 1, 71, 80) OK lit drawn at (581, 520), (337, 1, 48, 49) back lit at (726, 533).

def cell(sh, x, y, w, h, top, bot, rim):
    m = sh.mask_rrect(x, y, w, h, 8)
    sh.paint(m, rgb(rim))
    sh.paint(sh.mask_rrect(x + 1.5, y + 1.5, w - 3, h - 3, 6.5), sh.vgrad(rgb(top), rgb(bot), y, y + h))
    sh.paint(sh.mask_rrect(x + 5, y + 3, w - 10, h * 0.3, 4), (1, 1, 1), 0.3)


def theme_page():
    sh = Sheet(800, 600)
    page_background(sh, seed=9)
    title_plate(sh, 62, 26, 230, 36)
    for k in range(5):
        cx = 346 + 90 * k
        y = 33 if k % 2 == 1 else 58
        sh.paint(sh.mask_rrect(cx - 44, y, 88, 22, 11), rgb("#2c4a73"), 0.85)
        sh.paint(sh.mask_rrect(cx - 42.5, y + 1.5, 85, 19, 9.5), rgb("#fffaf0"))
        sh.paint(sh.mask_rrect(cx - 1, y + 22, 2, 8 if k % 2 == 0 else 33, 1), rgb("#2c4a73"), 0.6)
    glass(sh, 44, 82, 712, 428, r=18)
    for i in range(13):
        y = 92 + 32 * i
        cell(sh, 72, y, 221, 31, "#fbf6e8", "#efe5c9", "#b8a77a")
        for k in range(5):
            cell(sh, 303 + 90 * k, y, 87, 31, "#f5f8fb", "#dfe7ef", "#a9b8c8")
    glossy_disc(sh, 615, 555, 27, OK_COL, g_tick)
    glossy_disc(sh, 749, 556, 22, BACK_COL, g_back)
    return sh


def theme_mo():
    sh = Sheet(450, 150)
    cell(sh, 1, 33, 221, 31, "#ffe9a8", "#f2b234", "#a06a10")
    cell(sh, 1, 1, 87, 31, "#fff2c8", "#f6d27a", "#a06a10")
    cell(sh, 89, 1, 87, 31, "#ffe39a", "#f2b234", "#a06a10")
    sh.paint(g_tick(sh, 89 + 43.5, 16, 9), rgb("#2f7a3a"))
    cell(sh, 177, 1, 87, 31, "#e8f6e0", "#bfe3ae", "#5f9a54")
    sh.paint(g_tick(sh, 177 + 43.5, 16, 9), rgb("#2f7a3a"))
    glossy_disc(sh, 265 + 34, 1 + 34, 28, GOLD, g_tick, glyph_col=INK, state="lit")
    glossy_disc(sh, 337 + 23, 1 + 23, 23, GOLD, g_back, glyph_col=INK, state="lit")
    return sh


# ---- the credits (creditsbckgrd) -------------------------------------------------------------------------------------------------
# files_ui.rs draw_credits: white lines scrolling up at x 50, the movie picture at (368, y).

def credits_page(logo):
    sh = Sheet(800, 600)
    sh.paint(np.ones((sh.h * 3, sh.w * 3), np.float32), sh.vgrad(rgb("#1b3a5c"), rgb("#0f2236"), 0, 600))
    # evening hills along the bottom, a moon and stars
    rng = np.random.default_rng(11)
    for k in range(70):
        x, y = rng.uniform(0, 800), rng.uniform(0, 360)
        sh.paint(sh.mask_circle(x, y, rng.uniform(0.6, 1.4)), (1, 1, 1), rng.uniform(0.3, 0.8))
    sh.glow(sh.mask_circle(660, 110, 34), 18, 0.35, rgb("#fff3c4"))
    sh.paint(sh.mask_circle(660, 110, 34), rgb("#fff6d8"))
    for i, (y, col) in enumerate(((430, "#1f4a3a"), (470, "#1a3f31"), (520, "#143428"))):
        pts = [(x, y + 18 * math.sin(x / (150 + 50 * i) + i * 2.1) + 8 * math.sin(x / 53 + i)) for x in range(-10, 811, 10)]
        sh.paint(sh.mask_poly(pts + [(810, 610), (-10, 610)]), rgb(col))
    sh.paint(sh.mask_rrect(612, 380, 2, 40, 1), (1, 1, 1), 0.8)
    sh.paint(sh.mask_poly([(614, 380), (632, 386), (614, 392)]), rgb("#e0403a"))
    # the reading column, darker so the white lines stand out
    m = sh.mask_rrect(30, -20, 440, 640, 30)
    sh.paint(m, rgb("#08121f"), 0.45)
    logo(sh, 640, 250, 0.42)
    return sh


# ---- the world map (WorldBase, WorldButton, Choose<Theme>Buttons) ---------------------------------------------------------------
# world_ui.rs draw_property: "Where will you build..." centred on 126 at y 29 and 43, the funds centred on 740 at y 30; a card per
# offer at CARD_POS (x, y): its theme ball (59 x 49) at the origin, the name centred on x + 96 at y + 15, the bonus on x + 104 at
# y + 26, the acres and price on x + 112 at y + 40, a pin at (x + 160, y + 10); the selected card (182 x 73 from the Choose
# sheet) with three more lines at y + 37, 47, 57; the legend's pins at (24 / 124 / 244, 550); the map pins at the property's
# map point (on our globe: North America at the left, the Atlantic, Europe at the right). Buttons: Reset World (760, 476) and
# Cancel (768, 557) in the box (724, 408, 76, 192): WorldButton (250, 408, 76, 114) normal, (724, 408, 76, 192) lit with
# Cancel lit at (732, 532, 68, 68); while moving the club Save (760, 476) and Load (774, 425): (550, 408 / 458) normal and
# (400, 408 / 458) lit.

CARD_POS = [(251, 13), (302, 64), (346, 115), (384, 166), (416, 220), (437, 277), (454, 337), (465, 398), (469, 458), (463, 520),
            (466, 13), (509, 64), (546, 115), (580, 166), (606, 220), (623, 277)]


def globe(sh, cx, cy, r):
    """Our own stylised globe: the North Atlantic between America and Europe, lit from the upper left."""
    disc = sh.mask_circle(cx, cy, r)
    sh.glow(disc, 14, 0.5, rgb("#ffffff"))
    sh.shadow(disc, 4, 8, 10, 0.3)
    sh.paint(disc, sh.rgrad(rgb("#5fb4e8"), rgb("#1f5f9e"), cx - r * 0.35, cy - r * 0.4, r * 1.6))
    # meridians and parallels, faint
    sh.paint(disc * sh.mask_rrect(cx - 0.6, cy - r, 1.2, 2 * r, 0), (1, 1, 1), 0.08)
    for k in (1, 2, 3):
        a = k * r / 3.4
        sh.paint(disc * np.clip(sh.mask_ellipse(cx - a, cy - r, 2 * a, 2 * r) - sh.mask_ellipse(cx - a + 1.2, cy - r, 2 * a - 2.4, 2 * r), 0, 1),
                 (1, 1, 1), 0.08)
    for k in range(-2, 3):
        y = cy + k * r * 0.38
        sh.paint(disc * sh.mask_rrect(cx - r, y - 0.6, 2 * r, 1.2, 0), (1, 1, 1), 0.1)
    land = rgb("#7cc35a")
    lands = [
        # North America: Alaska and Canada across the top, the west coast, the Gulf, Florida, the east coast, Nova Scotia
        [(14, 196), (40, 182), (78, 176), (112, 180), (140, 192), (164, 214), (174, 236), (166, 250), (152, 258), (142, 270),
         (136, 286), (128, 298), (130, 318), (124, 320), (114, 300), (100, 296), (84, 302), (82, 316), (68, 302), (58, 276),
         (46, 254), (40, 232), (24, 218)],
        # Greenland
        [(176, 172), (200, 166), (214, 178), (206, 200), (190, 208), (180, 196)],
        # Central and South America
        [(86, 316), (100, 322), (114, 334), (132, 344), (156, 352), (180, 366), (192, 388), (186, 412), (170, 432), (156, 446),
         (148, 430), (146, 404), (134, 380), (124, 362), (108, 344), (94, 330)],
        # Europe: Spain, France, the north coast, Scandinavia
        [(258, 252), (276, 246), (286, 236), (282, 222), (292, 206), (288, 186), (276, 182), (300, 170), (290, 166), (296, 160),
         (290, 158), (290, 260), (276, 266), (262, 264)],
        # Africa's top
        [(262, 276), (290, 270), (290, 360), (278, 344), (266, 316), (256, 296)],
        # Britain and Ireland
        [(262, 214), (270, 208), (276, 218), (278, 236), (268, 240), (264, 228)],
        [(252, 224), (260, 222), (262, 234), (254, 238)],
        # the islands: Hawaii and Jamaica
        [(18, 306), (26, 304), (28, 312), (22, 316)], [(20, 322), (32, 320), (34, 332), (24, 334)],
        [(110, 330), (124, 328), (126, 334), (112, 336)],
    ]
    for pts in lands:
        m = disc * sh.mask_poly(pts)
        sh.shadow(m, 0.5, 1, 1, 0.25)
        sh.paint(m, land)
    # ice caps, the light and the rim
    sh.paint(disc * sh.mask_ellipse(cx - r * 0.7, cy - r * 1.12, r * 1.4, r * 0.3), rgb("#f2f8fc"))
    shade = np.clip(sh.rgrad((0, 0, 0), (1, 1, 1), cx - r * 0.45, cy - r * 0.5, r * 1.9)[..., 0], 0, 1)
    sh.paint(disc * shade, rgb("#0b2a4a"), 0.45)
    sh.paint(disc * sh.mask_ellipse(cx - r * 0.75, cy - r * 0.9, r * 0.9, r * 0.6), (1, 1, 1), 0.18)
    sh.paint(np.clip(disc - sh.mask_circle(cx, cy, r - 2.5), 0, 1), rgb("#e8f4ff"), 0.6)


def world_base():
    sh = Sheet(800, 600)
    page_background(sh, seed=7)
    globe(sh, 150, 300, 148)
    title_plate(sh, 14, 14, 226, 46)
    title_plate(sh, 680, 16, 118, 30)
    paper(sh, 14, 544, 352, 38, r=14)
    for (x, y) in CARD_POS:
        bar = sh.mask_rrect(x + 26, y + 2, 150, 44, 14)
        sh.shadow(bar, 1, 2, 3, 0.3)
        sh.paint(bar, rgb("#2c4a73"))
        sh.paint(sh.mask_rrect(x + 27.5, y + 3.5, 147, 41, 12.5), sh.vgrad(rgb("#fffaf0"), rgb("#efe2bd"), y, y + 30))
        sh.paint(sh.mask_rrect(x + 50, y + 28, 124, 15, 6), rgb("#cfe0f2"))
        sh.paint(sh.mask_circle(x + 29, y + 24, 25), rgb("#2c4a73"))
    # the buttons' tray
    glass(sh, 712, 394, 100, 220, r=26)
    glossy_disc(sh, 760, 476, 25, BLUE, g_globe_turn)
    glossy_disc(sh, 768, 557, 25, BLUE, g_back)
    return sh


def g_globe_turn(sh, cx, cy, r):
    ring = np.clip(sh.mask_circle(cx, cy, r * 0.62) - sh.mask_circle(cx, cy, r * 0.4), 0, 1)
    gap = sh.mask_poly([(cx, cy), (cx + r, cy - r * 0.2), (cx + r, cy + r * 0.6)])
    head = sh.mask_poly([(cx + r * 0.25, cy - r * 0.05), (cx + r * 0.95, cy - r * 0.05), (cx + r * 0.6, cy + r * 0.45)])
    return np.maximum(np.clip(ring - gap, 0, 1), head)


def g_disk(sh, cx, cy, r):
    body = sh.mask_rrect(cx - r * 0.75, cy - r * 0.75, r * 1.5, r * 1.5, r * 0.15)
    label = sh.mask_rrect(cx - r * 0.45, cy - r * 0.75, r * 0.9, r * 0.6, r * 0.05)
    slot = sh.mask_rrect(cx - r * 0.45, cy + r * 0.2, r * 0.9, r * 0.45, r * 0.08)
    return np.clip(body - label * 0.0 - slot, 0, 1)


def g_folder2(sh, cx, cy, r):
    back = sh.mask_rrect(cx - r * 0.85, cy - r * 0.5, r * 1.7, r * 1.15, r * 0.12)
    tab = sh.mask_rrect(cx - r * 0.85, cy - r * 0.68, r * 0.7, r * 0.3, r * 0.1)
    return np.maximum(back, tab)


def world_button():
    sh = Sheet(800, 600)
    # Reset World: normal at x 250, lit at x 724 with Cancel lit
    glossy_disc(sh, 250 + 36, 476, 25, BLUE, g_globe_turn)
    glossy_disc(sh, 724 + 36, 476, 26, GOLD, g_globe_turn, glyph_col=INK, state="lit")
    glossy_disc(sh, 768, 557, 26, GOLD, g_back, glyph_col=INK, state="lit")
    # moving the club: Save and Load, normal at x 550 and lit at x 400
    for x0, col, st in ((550, BLUE, "normal"), (400, GOLD, "lit")):
        gc = INK if st == "lit" else None
        glossy_disc(sh, x0 + 36, 476, 24, col, g_disk, glyph_col=gc, state=st)
        glossy_disc(sh, x0 + 50, 425, 18, col, g_folder2, glyph_col=gc, state=st)
    return sh


def theme_ball(sh, cx, cy, r, theme, state):
    """A theme's ball: its little scene in a round frame. state: off (too dear), on, owned."""
    rim = {"off": "#6f7a86", "on": "#2c4a73", "owned": "#8a2a24"}[state]
    m = sh.mask_circle(cx, cy, r)
    sh.paint(m, rgb(rim))
    d = sh.mask_circle(cx, cy, r - 2.5)
    sky, ground = {"park": ("#bfe6fb", "#6cbf4e"), "links": ("#cfdfe9", "#8cb05e"), "desert": ("#ffe1a8", "#e3a866"),
                   "trop": ("#8fe0f0", "#f4e2a6")}[theme]
    sh.paint(d, sh.vgrad(rgb(sky), lighten(rgb(sky), 0.5), cy - r, cy))
    gy = cy + r * 0.15
    sh.paint(d * sh.mask_ellipse(cx - r * 1.4, gy - r * 0.1, r * 2.8, r * 1.6), rgb(ground))
    if theme == "park":
        sh.paint(d * sh.mask_rrect(cx - r * 0.38, cy - r * 0.2, r * 0.12, r * 0.45, 1), rgb("#6b4a2e"))
        for dx, dy, rr in ((-0.32, -0.38, 0.3), (-0.5, -0.2, 0.24), (-0.15, -0.22, 0.24)):
            sh.paint(d * sh.mask_circle(cx + dx * r, cy + dy * r, rr * r), rgb("#2f8a3a"))
    elif theme == "links":
        sh.paint(d * sh.mask_ellipse(cx - r * 1.2, gy + r * 0.2, r * 2.4, r), rgb("#3a6fa0"))
        sh.paint(d * sh.mask_rrect(cx - r * 0.45, cy - r * 0.55, r * 0.3, r * 0.75, 1), rgb("#a49c90"))
        sh.paint(d * sh.mask_poly([(cx - r * 0.5, cy - r * 0.55), (cx - r * 0.3, cy - r * 0.8), (cx - r * 0.1, cy - r * 0.55)]), rgb("#8a3a2a"))
    elif theme == "desert":
        sh.paint(d * sh.mask_circle(cx - r * 0.45, cy - r * 0.45, r * 0.2), rgb("#fff3b0"))
        c = rgb("#3f9a4a")
        sh.paint(d * sh.mask_rrect(cx - r * 0.42, cy - r * 0.4, r * 0.18, r * 0.75, r * 0.09), c)
        sh.paint(d * sh.mask_rrect(cx - r * 0.62, cy - r * 0.2, r * 0.14, r * 0.3, r * 0.07), c)
        sh.paint(d * sh.mask_rrect(cx - r * 0.62, cy + r * 0.0, r * 0.3, r * 0.1, r * 0.05), c)
    else:
        sh.paint(d * sh.mask_rrect(cx - r * 1.2, gy + r * 0.3, r * 2.4, r, 0), rgb("#2fb0c8"))
        trunk = [(cx - r * 0.3 + 0.1 * r * t * t, gy + r * 0.2 - r * 0.85 * t) for t in np.linspace(0, 1, 10)]
        sh.paint(d * sh.mask_line(trunk, r * 0.09), rgb("#8a6a3e"))
        tx, ty = trunk[-1]
        for a in (-2.7, -2.0, -1.1, -0.4):
            sh.paint(d * sh.mask_line([(tx, ty), (tx + math.cos(a) * r * 0.45, ty + math.sin(a) * r * 0.15 + r * 0.18)], r * 0.12),
                     rgb("#2f8f3a"))
    # a flag on every ball
    sh.paint(d * sh.mask_rrect(cx + r * 0.3, cy - r * 0.5, r * 0.06, r * 0.65, 0.5), (1, 1, 1))
    sh.paint(d * sh.mask_poly([(cx + r * 0.36, cy - r * 0.5), (cx + r * 0.66, cy - r * 0.4), (cx + r * 0.36, cy - r * 0.3)]), rgb("#e0403a"))
    sh.paint(d * sh.mask_ellipse(cx - r * 0.7, cy - r * 0.95, r * 1.4, r * 0.8), (1, 1, 1), 0.25)
    if state == "off":
        sh.paint(d, rgb("#8892a0"), 0.55)
    if state == "owned":
        sh.paint(np.clip(m - sh.mask_circle(cx, cy, r - 3.5), 0, 1), rgb("#e05a4a"))


def choose_buttons(theme):
    sh = Sheet(800, 100)
    sh.paint(np.ones((300, 2400), np.float32), (1, 0, 0))
    sh.erase(np.ones((300, 2400), np.float32))
    for x0, st in ((2, "off"), (202, "on"), (300, "owned")):
        theme_ball(sh, x0 + 29, 24, 23, theme, st)
    for x0, ok in ((402, True), (602, False)):
        card = sh.mask_rrect(x0 + 26, 1, 154, 70, 16)
        sh.shadow(card, 1, 2, 2, 0.35)
        sh.paint(card, rgb("#a06a10") if ok else rgb("#5a6470"))
        sh.paint(sh.mask_rrect(x0 + 27.5, 2.5, 151, 67, 14.5), sh.vgrad(rgb("#fff6d8") if ok else rgb("#eef1f4"),
                                                                       rgb("#ffe08a") if ok else rgb("#cfd6de"), 0, 72))
        theme_ball(sh, x0 + 29, 24, 25, theme, "on" if ok else "off")
        if ok:
            sh.glow(np.clip(sh.mask_circle(x0 + 29, 24, 26) - sh.mask_circle(x0 + 29, 24, 23), 0, 1), 2, 0.6, GOLD_HI)
    return sh


# ---- Top 10 Designers (Top10_Blank, Top10_Trophies) ------------------------------------------------------------------------------
# info_ui.rs draw_top10: ten cups in columns 160 wide, the top row cut (col * 160, 40, 160, 304), the bottom (col * 160, 344,
# 160, 256), from the same place on the trophies sheet; rank r stands in slot RANK_SLOT[r]. On each cup, centred on the column:
# the total, "Total Score (xN)" and the figures in olive (the leader's course name above), the name on the shelf's plate at
# y 321 (top) or 574 (bottom).

RANK_SLOT = [2, 1, 3, 0, 4, 7, 6, 8, 5, 9]


def top10_text_span(rank):
    """The engine's text span on cup `rank`: (top, bottom)."""
    if rank < 5:
        cash = 164 + 10 * rank
        total = cash - 24 - 20
        top = total - 13 - (27 if rank == 0 else 0)
        return top - 2, cash + 16
    cash = 443 + 4 * rank
    total = cash - 20 - 12
    return total - 13 - 2, cash + 12


def wood(sh, m, y0, y1, base="#7a4a26"):
    sh.paint(m, sh.vgrad(lighten(rgb(base), 0.12), lighten(rgb(base), -0.25), y0, y1))
    rng = np.random.default_rng(int(y0))
    for k in range(10):
        yy = rng.uniform(y0, y1)
        sh.paint(m * sh.mask_rrect(0, yy, 800, rng.uniform(0.6, 1.4), 0), rgb("#3a2010"), rng.uniform(0.08, 0.2))


def shelf(sh, y, plates):
    """A shelf whose top is at y: its top face, its front edge with a brass plate under each column."""
    wood(sh, sh.mask_rrect(-5, y - 6, 810, 8, 2), y - 6, y + 2, "#9a6236")
    front = sh.mask_rrect(-5, y + 2, 810, 30, 3)
    sh.shadow(front, 0, 4, 5, 0.45)
    wood(sh, front, y + 2, y + 32)
    sh.paint(sh.mask_rrect(-5, y + 2, 810, 1.5, 0), rgb("#d9a46a"), 0.7)
    for cx in plates:
        p = sh.mask_rrect(cx - 66, y + 6, 132, 22, 6)
        sh.shadow(p, 0, 1, 1.2, 0.4)
        sh.paint(p, rgb("#6a4a14"))
        sh.paint(sh.mask_rrect(cx - 64.5, y + 7.5, 129, 19, 5), sh.vgrad(rgb("#f5d88a"), rgb("#b88a2c"), y + 6, y + 28))
        sh.paint(sh.mask_rrect(cx - 58, y + 9, 116, 5, 2.5), (1, 1, 1), 0.25)


def top10_blank():
    sh = Sheet(800, 600)
    # a cabinet: deep blue velvet behind glass, warm light from above
    sh.paint(np.ones((1800, 2400), np.float32), sh.vgrad(rgb("#24345e"), rgb("#121a33"), 0, 600))
    for y0 in (34, 344):
        light = np.clip(1 - sh.rgrad((0, 0, 0), (1, 1, 1), 400, y0, 430)[..., 0], 0, 1)
        sh.paint(light * sh.mask_rrect(0, y0, 800, 280, 0), rgb("#ffe8b0"), 0.22)
    for x in (0, 160, 320, 480, 640, 800):
        sh.paint(sh.mask_rrect(x - 1, 0, 2, 600, 1), rgb("#0b1022"), 0.35)
    wood(sh, sh.mask_rrect(-5, 0, 810, 34, 0), 0, 34, "#6a3e1e")
    sh.paint(sh.mask_rrect(-5, 32, 810, 3, 0), rgb("#d9a46a"), 0.5)
    shelf(sh, 312, [80 + 160 * k for k in range(5)])
    shelf(sh, 566, [80 + 160 * k for k in range(5)])
    title = sh.mask_text("TOP 10 DESIGNERS", 400, 17, 22)
    sh.paint(title, rgb("#f5d88a"))
    return sh


def cup(sh, cx, top, bottom, floor, scale):
    """A golden cup whose engraved ivory face spans top..bottom, standing on the shelf at `floor`."""
    hw = 62 * scale + 6
    bowl_top, bowl_bot = top - 12 * scale, bottom + 16 * scale
    gold_hi, gold, gold_lo = rgb("#fff0b0"), rgb("#f0bf3c"), rgb("#9a6a14")
    # handles
    for sgn in (-1, 1):
        hx = cx + sgn * (hw + 2)
        pts = [(hx - sgn * 6, bowl_top + 12), (hx + sgn * 10, bowl_top + 16), (hx + sgn * 12, bowl_top + 40 * scale),
               (hx - sgn * 2, bowl_bot - 30 * scale)]
        sh.paint(sh.mask_line(pts, 7 * scale + 2), gold_lo)
        sh.paint(sh.mask_line(pts, 4 * scale + 1), gold)
    bowl = sh.mask_poly([(cx - hw - 6, bowl_top), (cx + hw + 6, bowl_top), (cx + hw + 2, bowl_top + (bowl_bot - bowl_top) * 0.55),
                         (cx + hw * 0.6, bowl_bot - 6), (cx + 14 * scale, bowl_bot + 8), (cx - 14 * scale, bowl_bot + 8),
                         (cx - hw * 0.6, bowl_bot - 6), (cx - hw - 2, bowl_top + (bowl_bot - bowl_top) * 0.55)])
    sh.shadow(bowl, 2, 3, 3, 0.4)
    xs = (np.arange(2400) / 3 - cx) / (hw + 6)
    col = mix(gold_hi, gold_lo, np.clip(np.abs(xs + 0.35) * 1.1, 0, 1)[None, :, None] * np.ones((1800, 1, 1)))
    sh.paint(bowl, col)
    sh.paint(sh.mask_ellipse(cx - hw - 6, bowl_top - 6, 2 * hw + 12, 12), gold_lo)
    sh.paint(sh.mask_ellipse(cx - hw - 3, bowl_top - 4, 2 * hw + 6, 8), rgb("#6a4610"))
    face = sh.mask_rrect(cx - hw + 4, top, 2 * hw - 8, bottom - top, 12 * scale + 2)
    sh.paint(face, rgb("#8a5e12"))
    sh.paint(sh.mask_rrect(cx - hw + 5.5, top + 1.5, 2 * hw - 11, bottom - top - 3, 12 * scale + 0.5),
             sh.vgrad(rgb("#fbf0d0"), rgb("#ecd9a4"), top, bottom))
    sh.paint(np.clip(sh.mask_rrect(cx - hw + 8, top + 4, 2 * hw - 16, bottom - top - 8, 12 * scale - 2)
                     - sh.mask_rrect(cx - hw + 9, top + 5, 2 * hw - 18, bottom - top - 10, 12 * scale - 3), 0, 1), rgb("#c09a48"), 0.8)
    # stem and base
    stem_top, base_top = bowl_bot + 6, floor - 22 * scale - 6
    sh.paint(sh.mask_poly([(cx - 9 * scale, stem_top), (cx + 9 * scale, stem_top), (cx + 5 * scale, base_top), (cx - 5 * scale, base_top)]),
             mix(gold, gold_lo, 0.3))
    sh.paint(sh.mask_ellipse(cx - 18 * scale, stem_top + (base_top - stem_top) * 0.4, 36 * scale, 10 * scale), gold)
    base = sh.mask_rrect(cx - 44 * scale, base_top, 88 * scale, floor - base_top, 4)
    sh.shadow(base, 0, 2, 2, 0.4)
    sh.paint(base, sh.vgrad(rgb("#3a2414"), rgb("#1e120a"), base_top, floor))
    sh.paint(sh.mask_rrect(cx - 46 * scale, base_top - 3, 92 * scale, 6, 2), gold)


def top10_trophies():
    sh = Sheet(800, 600)
    sh.paste(top10_blank(), 0, 0)
    for rank, slot in enumerate(RANK_SLOT):
        col, row = slot % 5, slot // 5
        cx = col * 160 + 80
        top, bottom = top10_text_span(rank)
        top -= 4
        floor = 312 if row == 0 else 566
        scale = [1.0, 0.9, 0.9, 0.82, 0.82, 0.78, 0.72, 0.72, 0.66, 0.66][rank]
        cup(sh, cx, top, bottom, floor, scale)
    return sh


# ---- the trophy room (bulletinboard&mantlewood, tacs&tees, TROPHYparts) --------------------------------------------------------
# screens_ui.rs draw_board: photos in a frame (tacs&tees (63, 251, 229, 209), the photo 200 x 160 at (7, 7) in it, the caption
# under it); the notes pad (58, 49, 207, 178) drawn at (600, 350) with up to three handwritten to-do strips from TODO_LABELS /
# TODO_ALT on it; the tee and ball (51, 482, 184, 80) and the OK tick (732, 511, 53, 52) where they are; on TROPHYparts the
# clubs (510, 13, 132, 162) at (519, 395) and (649, 13, 130, 163) at (177, 395), the cup: its rim (276, 292, 257, 47), caps
# (9, 116, 248, 30), a plaque per accomplishment (rows of 35 from (9, 221) for 0..10, (547, 221) for 11..21, the easy
# editions' at (282, 186)), each showing its top 14 pixels, the lower cap (9, 186, 248, 19), the body (283, 357, 248, 146)
# drawn at (288, 351) and the foot (317, 490, 183, 52) at (313, 484).

TODO_LABELS = [(400, 97, 135, 22), (400, 133, 117, 21), (400, 168, 125, 21), (400, 207, 124, 18), (400, 239, 133, 20),
               (400, 273, 135, 21), (400, 310, 137, 20), (400, 341, 129, 25), (400, 381, 133, 32), (400, 413, 110, 22),
               (400, 448, 112, 24), (400, 480, 177, 37), (400, 519, 160, 35), (400, 556, 141, 31), (610, 97, 129, 20),
               (610, 130, 180, 36), (610, 168, 161, 36), (610, 202, 144, 40), (610, 239, 144, 33), (610, 274, 142, 38),
               (610, 307, 144, 35), (610, 341, 155, 24)]
TODO_ALT = [(610, 381, 132, 20), (610, 415, 126, 20), (610, 450, 109, 21)]
TITLE_KEYS = ["CHALLENGE_HOLE", "HEROIC_HOLE", "SKILL_UPGRADE", "TOURNAMENT", "STRATEGIC_HOLE", "MATCH_VICTORY", "9_HOLE_COURSE",
              "TOP_100_HOLE", "500K_TOURNAMENT", "CLASSIC_HOLE", "TOP_18_HOLE", "VICTORY_9", "GRAND_SLAM_COURSE_9", "1M_TOURNAMENT",
              "18_HOLE_COURSE", "VICTORY_18", "GRAND_SLAM_COURSE_18", "GRAND_SLAM_PARKLAND", "GRAND_SLAM_DESERT",
              "GRAND_SLAM_TROPICAL", "GRAND_SLAM_LINKS", "100_STAR"]
EASY_KEYS = ["DOGLEG_RIGHT", "DOGLEG_LEFT", "PAR_5"]


def english(key):
    import os
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "lang", "en.txt")
    for line in open(path, encoding="utf-8"):
        if line.startswith(key + " ") or line.startswith(key + ":"):
            return line.split(":", 1)[1].strip()
    return key


def text_mask(sh, text, x, y, size, anchor="mm", font=None):
    from PIL import Image, ImageDraw, ImageFont
    import uiart
    m = Image.new("L", (sh.w * 3, sh.h * 3), 0)
    f = ImageFont.truetype(font or uiart.FONT, int(size * 3))
    ImageDraw.Draw(m).text((x * 3, y * 3), text, font=f, fill=255, anchor=anchor)
    return np.asarray(m, np.float32) / 255.0


def text_width(text, size, font=None):
    from PIL import ImageFont
    import uiart
    return ImageFont.truetype(font or uiart.FONT, int(size * 3)).getlength(text) / 3


def board_background():
    sh = Sheet(800, 600)
    # the clubhouse wall: soft green panelling with pinstripes, two cork boards for the photos
    sh.paint(np.ones((1800, 2400), np.float32), sh.vgrad(rgb("#5f8a6a"), rgb("#3f6a4e"), 0, 520))
    for x in range(0, 800, 26):
        sh.paint(sh.mask_rrect(x, 0, 1.2, 520, 0), rgb("#2f5a3e"), 0.35)
        sh.paint(sh.mask_rrect(x + 1.2, 0, 1, 520, 0), rgb("#8fb89a"), 0.25)
    wood(sh, sh.mask_rrect(-5, 0, 810, 22, 0), 0, 22, "#6a3e1e")
    sh.paint(sh.mask_rrect(-5, 20, 810, 3, 0), rgb("#d9a46a"), 0.45)
    for x0, x1, y1 in ((8, 300, 470), (510, 792, 340)):
        frame = sh.mask_rrect(x0, 40, x1 - x0, y1 - 40, 10)
        sh.shadow(frame, 2, 4, 6, 0.45)
        wood(sh, frame, 40, y1, "#8a5a2e")
        cork = sh.mask_rrect(x0 + 10, 50, x1 - x0 - 20, y1 - 60, 4)
        sh.paint(cork, rgb("#c99a62"))
        rng = np.random.default_rng(x0)
        yy, xx = np.mgrid[0:1800, 0:2400]
        speck = (rng.random((1800 // 6 + 1, 2400 // 6 + 1)) > 0.6).repeat(6, 0).repeat(6, 1)[:1800, :2400]
        sh.paint(cork * speck, rgb("#a87a46"), 0.35)
        del yy, xx
    # a lamp glow over the mantle
    glow = np.clip(1 - sh.rgrad((0, 0, 0), (1, 1, 1), 400, 260, 300)[..., 0], 0, 1)
    sh.paint(glow, rgb("#fff0c0"), 0.12)
    # the mantle: a shelf with a lip and a dark fireplace face below it
    wood(sh, sh.mask_rrect(-5, 520, 810, 90, 0), 520, 600, "#5a3418")
    top = sh.mask_rrect(-5, 528, 810, 16, 3)
    sh.shadow(top, 0, 5, 6, 0.5)
    wood(sh, top, 528, 544, "#a06a3a")
    sh.paint(sh.mask_rrect(-5, 528, 810, 2, 0), rgb("#e0b07a"), 0.7)
    wood(sh, sh.mask_rrect(-5, 544, 810, 16, 0), 544, 560, "#7a4a24")
    return sh


def polaroid(sh, x, y):
    m = sh.mask_rrect(x, y, 229, 209, 4)
    sh.shadow(m, 2, 3, 3, 0.5)
    sh.paint(m, sh.vgrad(rgb("#ffffff"), rgb("#ece6da"), y, y + 209))
    sh.erase(sh.mask_rrect(x + 7, y + 7, 215, 160, 0))
    sh.paint(np.clip(sh.mask_rrect(x + 6, y + 6, 217, 162, 1) - sh.mask_rrect(x + 7, y + 7, 215, 160, 0), 0, 1), rgb("#b8b0a0"))


def handwriting(sh, x, y, w, h, text):
    """A to-do line written in blue ink, fitted to its strip."""
    size = min(h * 0.72, 17)
    lines = [text]
    if text_width(text, size) > w - 6:
        words = text.split()
        best = None
        for k in range(1, len(words)):
            a, b = " ".join(words[:k]), " ".join(words[k:])
            wmax = max(text_width(a, 1), text_width(b, 1))
            if best is None or wmax < best[0]:
                best = (wmax, [a, b])
        lines = best[1] if best and h >= 28 else [text]
        size = min(size if len(lines) == 1 else h * 0.42, (w - 8) / max(text_width(("\u2022 " if k == 0 else "  ") + t, 1)
                                                                   for k, t in enumerate(lines)))
    for k, t in enumerate(lines):
        cy = y + h / 2 + (k - (len(lines) - 1) / 2) * size * 1.05
        sh.paint(text_mask(sh, "• " + t if k == 0 else "  " + t, x + 3, cy, size, "lm"), rgb("#1f3f9a"), 0.92)


def tacs_and_tees():
    sh = Sheet(800, 600)
    # the notes pad
    pad = sh.mask_rrect(58, 49, 207, 178, 6)
    sh.shadow(pad, 2, 3, 3, 0.45)
    sh.paint(pad, rgb("#fff3a8"))
    sh.paint(sh.mask_rrect(58, 49, 207, 30, 6), rgb("#e05a4a"))
    sh.paint(sh.mask_rrect(58, 72, 207, 7, 0), rgb("#e05a4a"))
    sh.paint(text_mask(sh, "Clubhouse Notes", 161, 64, 15), (1, 1, 1))
    for y in range(98, 224, 14):
        sh.paint(sh.mask_rrect(64, y, 195, 1, 0), rgb("#7fa8d8"), 0.7)
    sh.paint(sh.mask_rrect(84, 80, 1, 145, 0), rgb("#e07a7a"), 0.7)
    # the photo frame
    polaroid(sh, 63, 251)
    # the to-do strips
    for k, (x, y, w, h) in enumerate(TODO_LABELS):
        handwriting(sh, x, y, w, h, english("STR_ACCOMPLISHMENT_" + TITLE_KEYS[k]))
    for k, (x, y, w, h) in enumerate(TODO_ALT):
        handwriting(sh, x, y, w, h, english("STR_ACCOMPLISHMENT_" + EASY_KEYS[k]))
    # the tee and ball on the mantle, and the OK tick
    sh.paint(sh.mask_ellipse(70, 548, 150, 12), (0, 0, 0), 0.35)
    for k, (bx, tilt) in enumerate(((150, 0), (196, 0.5))):
        sh.paint(sh.mask_poly([(bx - 9, 520), (bx + 9, 520), (bx + 3, 528), (bx + 2, 552), (bx - 2, 552), (bx - 3, 528)]),
                 rgb("#e8a33a") if k == 0 else rgb("#e05a4a"))
        ball = sh.mask_circle(bx, 509, 13)
        sh.shadow(ball, 0.5, 1, 1, 0.3)
        sh.paint(ball, sh.rgrad((1, 1, 1), rgb("#c3d0dc"), bx - 4, 504, 18))
    card = sh.mask_poly([(58, 552), (120, 540), (128, 556), (66, 562)])
    sh.paint(card, rgb("#fffaf0"))
    for k in range(3):
        sh.paint(card * sh.mask_rrect(60, 549 + 4 * k, 70, 0.8, 0), rgb("#3a6aa6"), 0.6)
    glossy_disc(sh, 758, 537, 24, OK_COL, g_tick)
    return sh


def plaque(sh, x, y, h, text):
    m = sh.mask_rrect(x + 2, y, 244, h, 3)
    sh.paint(m, rgb("#5a3a10"))
    sh.paint(sh.mask_rrect(x + 3, y + 1, 242, h - 2, 2.5), sh.vgrad(rgb("#3a2414"), rgb("#24160c"), y, y + h))
    band = sh.mask_rrect(x + 22, y + 1.5, 204, 11.5, 2)
    sh.paint(band, sh.vgrad(rgb("#ffe7a0"), rgb("#c8962e"), y + 1, y + 13))
    sh.paint(text_mask(sh, text, x + 124, y + 7.6, 9.2), rgb("#4a2c06"))
    for sgn in (-1, 1):
        sh.paint(sh.mask_circle(x + 124 + sgn * 112, y + 7, 2.2), rgb("#e8c060"))


def trophy_parts():
    sh = Sheet(800, 600)
    gold_hi, gold, gold_lo = rgb("#fff0b0"), rgb("#f0bf3c"), rgb("#9a6a14")

    def gold_fill(m, x0, x1):
        xs = (np.arange(2400) / 3 - (x0 + x1) / 2) / ((x1 - x0) / 2)
        col = mix(gold_hi, gold_lo, np.clip(np.abs(xs + 0.35) * 1.1, 0, 1)[None, :, None] * np.ones((1800, 1, 1)))
        sh.paint(m, col)

    # the clubs: a golf bag with clubs (left of the cup) and two crossed clubs with a ball (right)
    bx, by = 488, 13
    for k, (dx, head) in enumerate(((44, "#c0c4ca"), (62, "#8a5a2e"), (80, "#c0c4ca"))):
        sh.paint(sh.mask_line([(bx + dx, by + 70), (bx + dx - 6 + k * 6, by + 14)], 3), rgb("#5a5a62"))
        sh.paint(sh.mask_rrect(bx + dx - 14 + k * 6, by + 4, 18, 12, 5), rgb(head))
    bag = sh.mask_rrect(bx + 30, by + 56, 66, 104, 14)
    sh.shadow(bag, 2, 2, 3, 0.4)
    sh.paint(bag, sh.vgrad(rgb("#d0503e"), rgb("#8a2a20"), by + 56, by + 160))
    sh.paint(sh.mask_rrect(bx + 28, by + 54, 70, 14, 6), rgb("#f2e6c8"))
    sh.paint(sh.mask_rrect(bx + 40, by + 96, 46, 30, 6), rgb("#f2e6c8"), 0.9)
    sh.paint(sh.mask_rrect(bx + 30, by + 146, 66, 14, 6), rgb("#3a2414"))
    cx2, cy2 = 649, 13
    for sgn in (-1, 1):
        x_top, x_bot = cx2 + 65 + sgn * 40, cx2 + 65 - sgn * 34
        sh.paint(sh.mask_line([(x_top, cy2 + 8), (x_bot, cy2 + 150)], 4), rgb("#4a4a52"))
        sh.paint(sh.mask_rrect(x_top - 6, cy2, 12, 18, 4), rgb("#2a2a30"))
        sh.paint(sh.mask_poly([(x_bot - 4, cy2 + 140), (x_bot + sgn * 22, cy2 + 146), (x_bot + sgn * 24, cy2 + 158), (x_bot - 6, cy2 + 158)]),
                 rgb("#c0c4ca"))
    ball = sh.mask_circle(cx2 + 65, cy2 + 146, 11)
    sh.paint(ball, sh.rgrad((1, 1, 1), rgb("#c3d0dc"), cx2 + 61, cy2 + 142, 15))
    # the rim: the cup's mouth, a gold lip over the cup's upper wall
    rx, ry = 276, 292
    wall = sh.mask_poly([(rx + 4, ry + 12), (rx + 253, ry + 12), (rx + 249, ry + 47), (rx + 8, ry + 47)])
    gold_fill(wall, rx, rx + 257)
    sh.paint(sh.mask_ellipse(rx, ry + 2, 257, 20), gold_lo)
    sh.paint(sh.mask_ellipse(rx + 4, ry + 4, 249, 15), rgb("#5a3a0c"))
    sh.paint(sh.mask_rrect(rx + 30, ry + 26, 196, 3, 1.5), (1, 1, 1), 0.35)
    # the caps and the lower cap: gold mouldings
    for (x, y, w, h) in ((9, 116, 248, 30), (9, 186, 248, 19)):
        m = sh.mask_rrect(x, y + 2, w, h - 4, (h - 4) / 2)
        gold_fill(m, x, x + w)
        sh.paint(sh.mask_rrect(x + 6, y + 4, w - 12, 3, 1.5), (1, 1, 1), 0.4)
    # the plaques
    for k in range(22):
        x, y = (9, 221 + 35 * k) if k <= 10 else (547, 221 + 35 * (k - 11))
        plaque(sh, x, y, 29, english("STR_ACCOMPLISHMENT_" + TITLE_KEYS[k]))
    for k in range(3):
        plaque(sh, 282, 186 + 35 * k, 30, english("STR_ACCOMPLISHMENT_" + EASY_KEYS[k]))
    # the body: the cup's bowl with its handles, narrowing to the stem
    bx0, by0 = 283, 357
    for sgn in (-1, 1):
        hx = bx0 + 124 + sgn * 104
        pts = [(hx - sgn * 4, by0 + 6), (hx + sgn * 14, by0 + 10), (hx + sgn * 16, by0 + 40), (hx - sgn * 10, by0 + 76)]
        sh.paint(sh.mask_line(pts, 9), gold_lo)
        sh.paint(sh.mask_line(pts, 5), gold)
    bowl = sh.mask_poly([(bx0 + 12, by0), (bx0 + 236, by0), (bx0 + 226, by0 + 50), (bx0 + 190, by0 + 92), (bx0 + 140, by0 + 112),
                         (bx0 + 136, by0 + 146), (bx0 + 112, by0 + 146), (bx0 + 108, by0 + 112), (bx0 + 58, by0 + 92),
                         (bx0 + 22, by0 + 50)])
    gold_fill(bowl, bx0 + 12, bx0 + 236)
    sh.paint(sh.mask_ellipse(bx0 + 100, by0 + 118, 48, 10), gold_lo)
    star = []
    for i in range(10):
        a = -math.pi / 2 + i * math.pi / 5
        rr = 22 if i % 2 == 0 else 9
        star.append((bx0 + 124 + rr * math.cos(a), by0 + 48 + rr * math.sin(a)))
    sh.paint(sh.mask_poly(star), rgb("#fff6c8"), 0.8)
    # the foot: a wooden plinth with a gold band
    fx, fy = 317, 490
    plinth = sh.mask_poly([(fx + 30, fy), (fx + 153, fy), (fx + 170, fy + 18), (fx + 170, fy + 50), (fx + 13, fy + 50), (fx + 13, fy + 18)])
    sh.paint(plinth, sh.vgrad(rgb("#4a2c16"), rgb("#24140a"), fy, fy + 52))
    sh.paint(sh.mask_rrect(fx + 50, fy + 24, 83, 14, 3), sh.vgrad(rgb("#ffe7a0"), rgb("#c8962e"), fy + 24, fy + 38))
    sh.paint(sh.mask_rrect(fx + 28, fy - 2, 127, 6, 3), gold)
    return sh


# ---- Select Difficulty (TitleSelDiffUnSel, TitleSelDiffMO) -----------------------------------------------------------------------
# render.rs: the title centred on 602 with its top at 42; each difficulty a face and a label pill, its name centred at
# DIFF_LABELS; the lit item cut (sx, sy, w, h) from the MO sheet is drawn at (dx, dy); Back (767, 557), lit (732, 532, 68, 68).

DIFF_FACES = [(242, 82), (203, 217), (210, 371), (252, 506)]
DIFF_PILLS = [(299, 104, 181, 49), (271, 218, 182, 49), (277, 338, 181, 49), (313, 456, 181, 49)]
DIFF_CUTS = [(0, 0, 338, 146, 193, 32), (400, 0, 338, 146, 150, 161), (0, 300, 332, 114, 161, 320), (400, 300, 332, 130, 200, 436)]
DIFF_COLS = ["#4caf50", "#f2b234", "#ef7d2c", "#d8433a"]


def ball_face(sh, cx, cy, r, mood, ring):
    """A golf ball with a face: 0 a happy grin, 1 a wary smile, 2 a worried frown with a sweat drop, 3 a shocked face."""
    ring = rgb(ring) if isinstance(ring, str) else ring
    outer = sh.mask_circle(cx, cy, r + 4)
    sh.shadow(outer, 2, 4, 5, 0.35)
    sh.paint(outer, ring)
    sh.paint(np.clip(outer - sh.mask_circle(cx, cy, r + 2.5), 0, 1), lighten(ring, -0.35))
    ball = sh.mask_circle(cx, cy, r)
    sh.paint(ball, sh.rgrad((1, 1, 1), rgb("#c3d0dc"), cx - r * 0.35, cy - r * 0.4, r * 1.6))
    for gy in range(-3, 4):
        for gx in range(-3, 4):
            px, py = cx + gx * r * 0.26 + (r * 0.13 if gy % 2 else 0), cy + gy * r * 0.24
            d = math.hypot(px - cx, py - cy) / r
            if d < 0.9 and not (abs(px - cx) < r * 0.6 and -r * 0.45 < py - cy < r * 0.62):
                sh.paint(ball * sh.mask_circle(px, py, r * 0.055), rgb("#aebdca"), 0.7)
    ink = rgb("#25303c")
    ey = cy - r * 0.12
    for sgn in (-1, 1):
        ex = cx + sgn * r * 0.3
        if mood == 3:
            sh.paint(sh.mask_circle(ex, ey, r * 0.17), (1, 1, 1))
            sh.paint(np.clip(sh.mask_circle(ex, ey, r * 0.17) - sh.mask_circle(ex, ey, r * 0.13), 0, 1), ink)
            sh.paint(sh.mask_circle(ex, ey, r * 0.06), ink)
        elif mood == 0:
            sh.paint(sh.mask_line([(ex - r * 0.12, ey + r * 0.03), (ex, ey - r * 0.08), (ex + r * 0.12, ey + r * 0.03)], r * 0.07), ink)
        else:
            sh.paint(sh.mask_ellipse(ex - r * 0.07, ey - r * 0.11, r * 0.14, r * 0.2), ink)
            sh.paint(sh.mask_circle(ex + r * 0.025, ey - r * 0.05, r * 0.03), (1, 1, 1))
        # brows
        tilt = {0: 0.0, 1: -0.03, 2: 0.09, 3: 0.12}[mood] * sgn
        by = ey - r * (0.27 if mood != 3 else 0.33)
        sh.paint(sh.mask_line([(ex - r * 0.13, by - tilt * r), (ex + r * 0.13, by + tilt * r)], r * 0.06), ink)
    # cheeks and mouth
    if mood == 0:
        for sgn in (-1, 1):
            sh.paint(sh.mask_circle(cx + sgn * r * 0.48, cy + r * 0.18, r * 0.12), rgb("#ff9a8a"), 0.55)
        mouth = sh.mask_poly([(cx - r * 0.34, cy + r * 0.16)] + [(cx + r * 0.34 * math.cos(a), cy + r * 0.16 + r * 0.3 * math.sin(a))
                                                                for a in np.linspace(0, math.pi, 12)])
        sh.paint(mouth, ink)
        sh.paint(mouth * sh.mask_ellipse(cx - r * 0.16, cy + r * 0.3, r * 0.32, r * 0.2), rgb("#e0605a"))
    elif mood == 1:
        sh.paint(sh.mask_line([(cx - r * 0.25, cy + r * 0.28), (cx, cy + r * 0.34), (cx + r * 0.25, cy + r * 0.24)], r * 0.07), ink)
    elif mood == 2:
        sh.paint(sh.mask_line([(cx - r * 0.24, cy + r * 0.38), (cx, cy + r * 0.28), (cx + r * 0.24, cy + r * 0.38)], r * 0.07), ink)
        drop = sh.mask_poly([(cx + r * 0.62, cy - r * 0.5), (cx + r * 0.72, cy - r * 0.28), (cx + r * 0.52, cy - r * 0.28)])
        sh.paint(np.maximum(drop, sh.mask_circle(cx + r * 0.62, cy - r * 0.24, r * 0.1)), rgb("#5ab4f0"))
    else:
        sh.paint(sh.mask_ellipse(cx - r * 0.13, cy + r * 0.22, r * 0.26, r * 0.32), ink)
        for k in range(3):
            a = -math.pi / 2 + (k - 1) * 0.5
            sh.paint(sh.mask_line([(cx + r * 1.05 * math.cos(a), cy + r * 1.05 * math.sin(a) - r * 0.05),
                                   (cx + r * 1.28 * math.cos(a), cy + r * 1.28 * math.sin(a) - r * 0.05)], r * 0.07), rgb("#d8433a"))


def diff_pill(sh, x, y, w, h, col, lit):
    edge = lighten(rgb(col), -0.4)
    under = sh.mask_rrect(x, y + 5, w, h, h / 2)
    sh.shadow(under, 2, 5, 5, 0.35)
    sh.paint(under, edge)
    sh.paint(sh.mask_rrect(x, y, w, h, h / 2), rgb(col) if not lit else rgb("#ffffff"))
    top, bot = ("#fffdf4", "#f1e2b4") if not lit else ("#fff2a8", "#ffbe3a")
    sh.paint(sh.mask_rrect(x + 3.5, y + 3.5, w - 7, h - 7, h / 2 - 3.5), sh.vgrad(rgb(top), rgb(bot), y, y + h))
    sh.paint(sh.mask_rrect(x + h * 0.4, y + 6, w - h * 0.8, h * 0.26, h * 0.13), (1, 1, 1), 0.6)


def difficulty_item(sh, k, dx, dy, lit):
    fx, fy = DIFF_FACES[k]
    px, py, w, h = DIFF_PILLS[k]
    diff_pill(sh, px + dx, py + dy, w, h, DIFF_COLS[k], lit)
    if lit:
        sh.glow(sh.mask_circle(fx + dx, fy + dy, 46), 6, 0.6, GOLD_HI)
    ball_face(sh, fx + dx, fy + dy, 38, k, GOLD if lit else DIFF_COLS[k])


def difficulty_page():
    sh = Sheet(800, 600)
    page_background(sh, seed=4)
    title_plate(sh, 432, 30, 340, 40)
    # a winding path of flags linking the four faces, easy to impossible
    pts = [(242, 82), (180, 150), (203, 217), (170, 300), (210, 371), (210, 440), (252, 506)]
    sh.paint(sh.mask_line(pts, 10), (1, 1, 1), 0.45)
    for k in range(4):
        difficulty_item(sh, k, 0, 0, False)
    glossy_disc(sh, 767, 557, 25, BLUE, g_back)
    return sh


def difficulty_mo():
    sh = Sheet(800, 600)
    for k, (sx, sy, w, h, dx, dy) in enumerate(DIFF_CUTS):
        difficulty_item(sh, k, sx - dx, sy - dy, True)
    glossy_disc(sh, 767, 557, 26, GOLD, g_back, glyph_col=INK, state="lit")
    return sh


# ---- the routing map's lower part (route screens_bottom) and the tract buttons (buy_land_buttons) ------------------------------
# reports_ui.rs draw_routing: the sheet's lower part (0, 253, 800, 347) behind the map; the four tabs around it, normal in that
# part, hovered from row 0 and open from row 114 / 118 of the sheet: routing (261, 0, 134, 117) at (182, 254), employees (0, 0,
# 129, 113) at (60, 313), aura (396, 0, 139, 117) at (483, 254), home site value (130, 0, 130, 113) at (612, 313); the OK tick
# lit (536, 0, 65, 65) at (662, 532); the staff list's scroll track (751, 0, 18, 178) at (775, 64) and its arrows lit (713, 1)
# and (732, 1), 18 x 37. world_ui.rs draw_land: the tract balls 59 x 61 at (1 + 60 i, 1) lit and (1 + 60 i, 64) bought, the lit
# tract diamond (1, 127, 192, 95), the compass (259, 127, 72, 61) at (71, 533), the OK tick lit (194, 127, 64, 64) at (662, 533).

ROUTE_TABS = [((261, 0, 134, 117), (182, 254)), ((0, 0, 129, 113), (60, 313)), ((396, 0, 139, 117), (483, 254)),
              ((130, 0, 130, 113), (612, 313))]
ROUTE_OPEN_ROW = [118, 114, 118, 114]


def g_route(sh, cx, cy, r):
    path = sh.mask_line([(cx - r * 0.7, cy + r * 0.6), (cx - r * 0.1, cy + r * 0.1), (cx + r * 0.2, cy + r * 0.45), (cx + r * 0.6, cy - r * 0.2)], r * 0.14)
    pole = sh.mask_rrect(cx + r * 0.55, cy - r * 0.9, r * 0.1, r * 0.75, r * 0.03)
    flag = sh.mask_poly([(cx + r * 0.65, cy - r * 0.9), (cx + r * 1.0, cy - r * 0.75), (cx + r * 0.65, cy - r * 0.6)])
    tee = sh.mask_circle(cx - r * 0.7, cy + r * 0.6, r * 0.16)
    return np.maximum.reduce([path, pole, flag, tee])


def g_staff(sh, cx, cy, r):
    head = sh.mask_circle(cx, cy - r * 0.5, r * 0.28)
    cap = sh.mask_rrect(cx - r * 0.34, cy - r * 0.82, r * 0.68, r * 0.22, r * 0.1)
    brim = sh.mask_rrect(cx - r * 0.1, cy - r * 0.66, r * 0.6, r * 0.1, r * 0.05)
    body = sh.mask_rrect(cx - r * 0.5, cy - r * 0.15, r * 1.0, r * 0.95, r * 0.3)
    return np.maximum.reduce([head, cap, brim, body])


def g_spark(sh, cx, cy, r):
    m = 0
    for (dx, dy, s) in ((0, 0, 1.0), (0.55, -0.5, 0.45), (-0.55, 0.45, 0.4)):
        pts = []
        for i in range(8):
            a = i * math.pi / 4
            rr = r * s * (0.75 if i % 2 == 0 else 0.22)
            pts.append((cx + dx * r + rr * math.cos(a), cy + dy * r + rr * math.sin(a)))
        m = np.maximum(m, sh.mask_poly(pts))
    return m


def g_house2(sh, cx, cy, r):
    roof = sh.mask_poly([(cx - r * 0.9, cy - r * 0.05), (cx, cy - r * 0.85), (cx + r * 0.9, cy - r * 0.05)])
    body = sh.mask_rrect(cx - r * 0.62, cy - r * 0.1, r * 1.24, r * 0.85, r * 0.05)
    door = sh.mask_rrect(cx - r * 0.16, cy + r * 0.25, r * 0.32, r * 0.5, r * 0.05)
    coin = sh.mask_circle(cx + r * 0.62, cy + r * 0.55, r * 0.32)
    return np.maximum(np.clip(np.maximum(roof, body) - door, 0, 1), coin)


ROUTE_GLYPHS = [(g_route, "#3f9e57"), (g_staff, "#3a77b6"), (g_spark, "#9a5cc4"), (g_house2, "#d0803a")]


def route_tab(sh, cx, cy, k, state):
    glyph, col = ROUTE_GLYPHS[k]
    if state == "open":
        sh.glow(sh.mask_circle(cx, cy, 44), 6, 0.8, GOLD_HI)
        glossy_disc(sh, cx, cy, 40, GOLD, glyph, glyph_col=INK)
    else:
        glossy_disc(sh, cx, cy, 38 if state == "normal" else 40, rgb(col), glyph, state="lit" if state == "lit" else "normal")


def route_bottom():
    sh = Sheet(800, 600)
    # the map's table: a soft green felt with a fine lattice and a vignette
    region = sh.mask_rrect(0, 253, 800, 347, 0)
    sh.paint(region, sh.vgrad(rgb("#9fb8cf"), rgb("#6f8aa6"), 253, 600))
    for x in range(0, 800, 40):
        sh.paint(region * sh.mask_rrect(x, 253, 1, 347, 0), (1, 1, 1), 0.08)
    for y in range(253, 600, 40):
        sh.paint(region * sh.mask_rrect(0, y, 800, 1, 0), (1, 1, 1), 0.08)
    sh.paint(region * sh.mask_rrect(0, 253, 800, 3, 0), rgb("#2c4a73"), 0.6)
    for k, ((sx, sy, w, h), (dx, dy)) in enumerate(ROUTE_TABS):
        route_tab(sh, dx + w / 2, dy + h / 2, k, "normal")
        route_tab(sh, sx + w / 2, sy + h / 2, k, "lit")
        route_tab(sh, sx + w / 2, ROUTE_OPEN_ROW[k] + h / 2, k, "open")
    # the compass' well and the OK tick
    sh.paint(sh.mask_circle(107, 563, 36), rgb("#2c4a73"), 0.5)
    glossy_disc(sh, 694, 564, 27, OK_COL, g_tick)
    glossy_disc(sh, 536 + 32, 32, 28, GOLD, g_tick, glyph_col=INK, state="lit")
    # the staff list's track and its arrows
    rail(sh, 751, 2, 18, 176)
    glossy_disc(sh, 722, 19, 8, GOLD, g_tri(True), glyph_col=INK)
    glossy_disc(sh, 741, 19, 8, GOLD, g_tri(False), glyph_col=INK)
    return sh


def compass(sh, cx, cy):
    sh.paint(sh.mask_circle(cx, cy, 29), rgb("#2c4a73"))
    sh.paint(sh.mask_circle(cx, cy, 27), sh.rgrad(rgb("#fffaf0"), rgb("#d9c8a0"), cx - 6, cy - 6, 34))
    for k in range(4):
        a = k * math.pi / 2 + math.pi / 4
        tip = (cx + math.cos(a) * 24, cy + math.sin(a) * 24 * 0.8)
        left = (cx + math.cos(a + 1.3) * 6, cy + math.sin(a + 1.3) * 6)
        right = (cx + math.cos(a - 1.3) * 6, cy + math.sin(a - 1.3) * 6)
        sh.paint(sh.mask_poly([tip, left, (cx, cy), right]), rgb("#d0503e") if k == 3 else rgb("#2c4a73"))
    sh.paint(sh.mask_circle(cx, cy, 3), rgb("#f2b234"))


def land_buttons():
    sh = Sheet(700, 400)
    for i in range(9):
        for row, col in ((1, rgb("#f2c234")), (64, rgb("#c9d2dc"))):
            cx, cy = 1 + 60 * i + 29.5, row + 30.5
            glossy_disc(sh, cx, cy, 26, col, None, shadow=False)
            sh.paint(sh.mask_text(str(i + 1), cx, cy + 1, 26), INK)
    d = [(1 + 96, 127), (1 + 192, 127 + 47.5), (1 + 96, 127 + 95), (1, 127 + 47.5)]
    sh.paint(sh.mask_poly(d), GOLD_HI, 0.55)
    sh.paint(np.clip(sh.mask_poly(d) - sh.mask_poly([(97, 133), (187, 174.5), (97, 216), (7, 174.5)]), 0, 1), GOLD)
    compass(sh, 259 + 36, 127 + 30.5)
    glossy_disc(sh, 194 + 32, 127 + 32, 28, GOLD, g_tick, glyph_col=INK, state="lit")
    return sh
