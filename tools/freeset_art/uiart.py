"""Vector drawing for the free set's interface pictures (ui.py): shapes as masks drawn four times oversize and filled with
flat colours, gradients and soft shadows, glossy round buttons, and the icon glyphs, then brought down to size."""
import math
import os

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
FONT = os.path.join(HERE, "..", "..", "crates", "simgolf", "fonts", "LiberationSans-Bold.ttf")
S = 3  # oversampling


def rgb(h):
    h = h.lstrip("#")
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], float) / 255.0


def mix(a, b, t):
    return np.asarray(a) * (1 - t) + np.asarray(b) * t


def lighten(c, k):
    c = np.asarray(c, float)
    return c + (1 - c) * k if k >= 0 else c * (1 + k)


class Sheet:
    """An RGBA picture (floats 0..1, straight alpha) drawn oversize."""

    def __init__(self, w, h, bg=None):
        self.w, self.h = w, h
        self.rgb = np.zeros((h * S, w * S, 3), np.float32)
        self.a = np.zeros((h * S, w * S), np.float32)
        if bg is not None:
            self.rgb[:] = bg
            self.a[:] = 1.0

    # ---- masks (in picture coordinates) ----
    def _canvas(self):
        return Image.new("L", (self.w * S, self.h * S), 0)

    def mask_ellipse(self, x, y, w, h):
        m = self._canvas()
        ImageDraw.Draw(m).ellipse([x * S, y * S, (x + w) * S - 1, (y + h) * S - 1], fill=255)
        return np.asarray(m, np.float32) / 255.0

    def mask_circle(self, cx, cy, r):
        return self.mask_ellipse(cx - r, cy - r, 2 * r, 2 * r)

    def mask_rrect(self, x, y, w, h, r):
        m = self._canvas()
        ImageDraw.Draw(m).rounded_rectangle([x * S, y * S, (x + w) * S - 1, (y + h) * S - 1], radius=max(0, r * S), fill=255)
        return np.asarray(m, np.float32) / 255.0

    def mask_poly(self, pts):
        m = self._canvas()
        ImageDraw.Draw(m).polygon([(px * S, py * S) for px, py in pts], fill=255)
        return np.asarray(m, np.float32) / 255.0

    def mask_line(self, pts, width, joint="curve"):
        m = self._canvas()
        d = ImageDraw.Draw(m)
        d.line([(px * S, py * S) for px, py in pts], fill=255, width=max(1, int(width * S)), joint=joint)
        for px, py in (pts[0], pts[-1]):
            r = width * S / 2
            d.ellipse([px * S - r, py * S - r, px * S + r, py * S + r], fill=255)
        return np.asarray(m, np.float32) / 255.0

    def mask_text(self, text, x, y, size, anchor="mm"):
        m = self._canvas()
        f = ImageFont.truetype(FONT, int(size * S))
        ImageDraw.Draw(m).text((x * S, y * S), text, font=f, fill=255, anchor=anchor)
        return np.asarray(m, np.float32) / 255.0

    # ---- paint ----
    def paint(self, mask, colour, alpha=1.0):
        """Draws a colour (or an h x w x 3 colour field at oversize) through a mask."""
        m = np.clip(mask * alpha, 0, 1).astype(np.float32)
        c = np.asarray(colour, np.float32)
        if c.ndim == 1:
            c = c[None, None, :]
        out_a = m + self.a * (1 - m)
        safe = np.maximum(out_a, 1e-6)
        self.rgb = (c * m[..., None] + self.rgb * (self.a * (1 - m))[..., None]) / safe[..., None]
        self.a = out_a

    def vgrad(self, top, bottom, y0, y1):
        """A vertical gradient colour field from y0 (top colour) to y1."""
        ys = (np.arange(self.h * S) / S - y0) / max(y1 - y0, 1e-6)
        t = np.clip(ys, 0, 1)[:, None, None]
        return np.broadcast_to(np.asarray(top) * (1 - t) + np.asarray(bottom) * t, (self.h * S, self.w * S, 3))

    def rgrad(self, inner, outer, cx, cy, r):
        yy, xx = np.mgrid[0:self.h * S, 0:self.w * S] / S
        t = np.clip(np.hypot(xx - cx, yy - cy) / max(r, 1e-6), 0, 1)[..., None]
        return np.asarray(inner) * (1 - t) + np.asarray(outer) * t

    def shadow(self, mask, dx=2, dy=3, blur=3, alpha=0.45, colour=(0, 0, 0)):
        img = Image.fromarray((mask * 255).astype(np.uint8))
        img = img.filter(ImageFilter.GaussianBlur(blur * S))
        sh = np.asarray(img, float) / 255.0
        sh = np.roll(np.roll(sh, int(dy * S), 0), int(dx * S), 1)
        self.paint(sh, colour, alpha)

    def glow(self, mask, blur=4, alpha=0.6, colour=(1, 1, 1)):
        self.shadow(mask, 0, 0, blur, alpha, colour)

    def paste(self, sub, x, y):
        """Draws another (smaller) sheet over this one with its top left at (x, y) (whole pixels)."""
        x0, y0 = int(x) * S, int(y) * S
        h, w = sub.a.shape
        ys0, xs0 = max(0, -y0), max(0, -x0)
        ys1, xs1 = min(h, self.a.shape[0] - y0), min(w, self.a.shape[1] - x0)
        if ys1 <= ys0 or xs1 <= xs0:
            return
        m = sub.a[ys0:ys1, xs0:xs1]
        dst_rgb = self.rgb[y0 + ys0:y0 + ys1, x0 + xs0:x0 + xs1]
        dst_a = self.a[y0 + ys0:y0 + ys1, x0 + xs0:x0 + xs1]
        out_a = m + dst_a * (1 - m)
        safe = np.maximum(out_a, 1e-6)
        dst_rgb[:] = (sub.rgb[ys0:ys1, xs0:xs1] * m[..., None] + dst_rgb * (dst_a * (1 - m))[..., None]) / safe[..., None]
        dst_a[:] = out_a

    def erase(self, mask):
        self.a *= 1 - mask

    # ---- output ----
    def image(self):
        """The picture at size: RGBA uint8."""
        rgb = self.rgb * self.a[..., None]
        k = S
        a = self.a.reshape(self.h, k, self.w, k).mean((1, 3))
        pr = rgb.reshape(self.h, k, self.w, k, 3).mean((1, 3))
        c = pr / np.maximum(a, 1e-6)[..., None]
        out = np.zeros((self.h, self.w, 4), np.uint8)
        out[..., :3] = np.clip(c * 255 + 0.5, 0, 255)
        out[..., 3] = np.clip(a * 255 + 0.5, 0, 255)
        return out

    def keyed(self, key=(255, 0, 255), cut=0.5):
        """RGB with the key colour where the picture is less than `cut` opaque (the edges kept by the alpha sheet)."""
        im = self.image()
        out = im[..., :3].copy()
        out[im[..., 3] < cut * 255] = key
        return out, im[..., 3]


# ---- building blocks -------------------------------------------------------------------------------------------------------

NAVY = rgb("#1d3448")
BLUE = rgb("#3a77b6")
SKY = rgb("#9fd0f2")
GOLD = rgb("#f2b234")
GOLD_HI = rgb("#ffe08a")
CREAM = rgb("#f6efd8")
GREEN = rgb("#3c9a4c")
GREEN_HI = rgb("#8ed37c")
GREY = rgb("#9aa3ad")
INK = rgb("#16222e")


def glossy_disc(sh, cx, cy, r, base, glyph=None, glyph_col=None, rim=None, shadow=True, state="normal"):
    """A round glossy button: a soft drop shadow, a rim, a body with a radial light, a highlight cap and a glyph."""
    if state == "lit":
        base = lighten(base, 0.25)
    elif state == "off":
        base = mix(base, GREY, 0.7)
    rim = lighten(base, -0.45) if rim is None else rim
    outer = sh.mask_circle(cx, cy, r)
    if shadow:
        sh.shadow(outer, 1.5, 2.5, 2.2, 0.45)
    sh.paint(outer, rim)
    body = sh.mask_circle(cx, cy, r * 0.88)
    sh.paint(body, sh.rgrad(lighten(base, 0.35), lighten(base, -0.25), cx - r * 0.3, cy - r * 0.35, r * 1.5))
    cap = sh.mask_ellipse(cx - r * 0.62, cy - r * 0.8, r * 1.24, r * 0.75)
    sh.paint(cap * body, (1, 1, 1), 0.28)
    if glyph is not None:
        gc = glyph_col if glyph_col is not None else (rgb("#fdfbf2") if np.mean(base) < 0.6 else INK)
        m = glyph(sh, cx, cy, r * 0.62)
        sh.shadow(m, 0.6, 0.9, 0.6, 0.35)
        sh.paint(m, gc)
    if state == "lit":
        sh.glow(outer * (1 - sh.mask_circle(cx, cy, r * 0.8)), 2, 0.35, GOLD_HI)


def pill(sh, x, y, w, h, top, bottom, rim=None, alpha=1.0, shadow=True):
    m = sh.mask_rrect(x, y, w, h, h / 2)
    if shadow:
        sh.shadow(m, 1.5, 2.5, 2.5, 0.4 * alpha)
    rim = lighten(bottom, -0.45) if rim is None else rim
    sh.paint(m, rim, alpha)
    inner = sh.mask_rrect(x + 2, y + 2, w - 4, h - 4, h / 2 - 2)
    sh.paint(inner, sh.vgrad(top, bottom, y, y + h), alpha)
    hi = sh.mask_rrect(x + h * 0.3, y + 3, w - h * 0.6, h * 0.32, h * 0.16)
    sh.paint(hi * inner, (1, 1, 1), 0.18 * alpha)


def oval(sh, x, y, w, h, top, bottom, rim=None, alpha=1.0, shadow=True, rim_w=3.0):
    m = sh.mask_ellipse(x, y, w, h)
    if shadow:
        sh.shadow(m, 3, 5, 5, 0.35 * alpha)
    rim = lighten(bottom, -0.4) if rim is None else rim
    sh.paint(m, rim, alpha)
    inner = sh.mask_ellipse(x + rim_w, y + rim_w, w - 2 * rim_w, h - 2 * rim_w)
    sh.paint(inner, sh.vgrad(top, bottom, y, y + h), alpha)
    hi = sh.mask_ellipse(x + w * 0.12, y + h * 0.06, w * 0.76, h * 0.4)
    sh.paint(hi * inner, (1, 1, 1), 0.16 * alpha)


# ---- glyphs: each returns a mask in the sheet, centred on (cx, cy) with radius r ----------------------------------------------

def g_plus(sh, cx, cy, r):
    t = r * 0.28
    return np.maximum(sh.mask_rrect(cx - r * 0.8, cy - t / 2, r * 1.6, t, t / 3), sh.mask_rrect(cx - t / 2, cy - r * 0.8, t, r * 1.6, t / 3))


def g_minus(sh, cx, cy, r):
    t = r * 0.28
    return sh.mask_rrect(cx - r * 0.8, cy - t / 2, r * 1.6, t, t / 3)


def _arc(cx, cy, r, a0, a1, n=24):
    return [(cx + r * math.cos(a), cy + r * math.sin(a)) for a in np.linspace(a0, a1, n)]


def g_turn(sgn):
    def f(sh, cx, cy, r):
        a0, a1 = (math.radians(200), math.radians(-20)) if sgn > 0 else (math.radians(-20), math.radians(200))
        pts = _arc(cx, cy + r * 0.15, r * 0.62, a0, a1)
        m = sh.mask_line(pts, r * 0.24)
        ex, ey = pts[-1]
        head = [(ex + sgn * r * 0.32, ey - r * 0.05), (ex - sgn * r * 0.22, ey - r * 0.38), (ex - sgn * r * 0.2, ey + r * 0.3)]
        return np.maximum(m, sh.mask_poly(head))
    return f


def g_info(sh, cx, cy, r):
    return np.maximum(sh.mask_circle(cx, cy - r * 0.58, r * 0.18), sh.mask_rrect(cx - r * 0.14, cy - r * 0.25, r * 0.28, r * 0.95, r * 0.08))


def g_pause(sh, cx, cy, r):
    return np.maximum(sh.mask_rrect(cx - r * 0.5, cy - r * 0.6, r * 0.32, r * 1.2, r * 0.08),
                      sh.mask_rrect(cx + r * 0.18, cy - r * 0.6, r * 0.32, r * 1.2, r * 0.08))


def g_wrench(sh, cx, cy, r):
    m = sh.mask_line([(cx - r * 0.55, cy + r * 0.55), (cx + r * 0.25, cy - r * 0.25)], r * 0.26)
    head = sh.mask_circle(cx + r * 0.38, cy - r * 0.38, r * 0.38)
    notch = sh.mask_rrect(cx + r * 0.38, cy - r * 0.86, r * 0.3, r * 0.5, r * 0.05)
    return np.clip(np.maximum(m, head) - notch, 0, 1)


def g_flag(sh, cx, cy, r):
    """A flag on a green."""
    green = sh.mask_ellipse(cx - r * 0.9, cy + r * 0.38, r * 1.8, r * 0.55)
    pole = sh.mask_rrect(cx - r * 0.06, cy - r * 0.9, r * 0.12, r * 1.6, r * 0.04)
    flag = sh.mask_poly([(cx + r * 0.05, cy - r * 0.9), (cx + r * 0.75, cy - r * 0.65), (cx + r * 0.05, cy - r * 0.38)])
    return np.maximum(np.maximum(green, pole), flag)


def g_house(sh, cx, cy, r):
    roof = sh.mask_poly([(cx - r * 0.9, cy - r * 0.05), (cx, cy - r * 0.85), (cx + r * 0.9, cy - r * 0.05)])
    body = sh.mask_rrect(cx - r * 0.62, cy - r * 0.1, r * 1.24, r * 0.85, r * 0.05)
    door = sh.mask_rrect(cx - r * 0.16, cy + r * 0.25, r * 0.32, r * 0.5, r * 0.05)
    return np.clip(np.maximum(roof, body) - door, 0, 1)


def g_people(sh, cx, cy, r):
    m = sh.mask_circle(cx - r * 0.35, cy - r * 0.45, r * 0.26)
    m = np.maximum(m, sh.mask_circle(cx + r * 0.35, cy - r * 0.45, r * 0.26))
    m = np.maximum(m, sh.mask_rrect(cx - r * 0.72, cy - r * 0.12, r * 0.7, r * 0.95, r * 0.3))
    m = np.maximum(m, sh.mask_rrect(cx + 0.02 * r, cy - r * 0.12, r * 0.7, r * 0.95, r * 0.3))
    return m


def g_folder(sh, cx, cy, r):
    back = sh.mask_rrect(cx - r * 0.85, cy - r * 0.55, r * 1.7, r * 1.15, r * 0.12)
    tab = sh.mask_rrect(cx - r * 0.85, cy - r * 0.72, r * 0.7, r * 0.3, r * 0.1)
    return np.maximum(back, tab)


def g_ball_tee(sh, cx, cy, r):
    ball = sh.mask_circle(cx, cy - r * 0.3, r * 0.55)
    tee = sh.mask_poly([(cx - r * 0.4, cy + r * 0.22), (cx + r * 0.4, cy + r * 0.22), (cx + r * 0.08, cy + r * 0.45),
                        (cx + r * 0.06, cy + r * 0.95), (cx - r * 0.06, cy + r * 0.95), (cx - r * 0.08, cy + r * 0.45)])
    dimples = np.zeros_like(ball)
    for dx, dy in ((-0.2, -0.45), (0.15, -0.5), (0.0, -0.25), (-0.25, -0.15), (0.25, -0.2)):
        dimples = np.maximum(dimples, sh.mask_circle(cx + dx * r, cy + dy * r, r * 0.07))
    return np.maximum(np.clip(ball - dimples * 0.7, 0, 1), tee)


def g_shovel(sh, cx, cy, r):
    handle = sh.mask_line([(cx - r * 0.6, cy - r * 0.75), (cx + r * 0.15, cy + r * 0.1)], r * 0.16)
    blade = sh.mask_poly([(cx + r * 0.05, cy + r * 0.0), (cx + r * 0.45, cy - r * 0.15), (cx + r * 0.85, cy + r * 0.5),
                          (cx + r * 0.5, cy + r * 0.85), (cx + r * 0.15, cy + r * 0.6)])
    grip = sh.mask_rrect(cx - r * 0.85, cy - r * 0.95, r * 0.42, r * 0.22, r * 0.1)
    return np.maximum(np.maximum(handle, blade), grip)


def g_cards(sh, cx, cy, r):
    a = sh.mask_rrect(cx - r * 0.8, cy - r * 0.75, r * 0.95, r * 1.15, r * 0.15)
    b_out = sh.mask_rrect(cx - r * 0.2, cy - r * 0.4, r * 0.95, r * 1.15, r * 0.15)
    b_in = sh.mask_rrect(cx - r * 0.08, cy - r * 0.28, r * 0.71, r * 0.91, r * 0.08)
    return np.maximum(np.clip(a - b_out, 0, 1), np.clip(b_out - b_in * 0.0, 0, 1))


def g_trophy(sh, cx, cy, r):
    cup = sh.mask_poly([(cx - r * 0.6, cy - r * 0.8), (cx + r * 0.6, cy - r * 0.8), (cx + r * 0.45, cy - r * 0.1),
                        (cx + r * 0.12, cy + r * 0.15), (cx - r * 0.12, cy + r * 0.15), (cx - r * 0.45, cy - r * 0.1)])
    stem = sh.mask_rrect(cx - r * 0.1, cy + r * 0.1, r * 0.2, r * 0.45, r * 0.04)
    foot = sh.mask_rrect(cx - r * 0.45, cy + r * 0.52, r * 0.9, r * 0.25, r * 0.08)
    handles = np.maximum(sh.mask_line(_arc(cx - r * 0.55, cy - r * 0.45, r * 0.3, math.radians(90), math.radians(270)), r * 0.12),
                         sh.mask_line(_arc(cx + r * 0.55, cy - r * 0.45, r * 0.3, math.radians(-90), math.radians(90)), r * 0.12))
    return np.maximum.reduce([cup, stem, foot, handles])


def g_exit(sh, cx, cy, r):
    door = sh.mask_rrect(cx - r * 0.75, cy - r * 0.8, r * 0.85, r * 1.6, r * 0.08)
    inner = sh.mask_rrect(cx - r * 0.6, cy - r * 0.65, r * 0.55, r * 1.3, r * 0.04)
    arrow = sh.mask_line([(cx - r * 0.25, cy), (cx + r * 0.5, cy)], r * 0.22)
    head = sh.mask_poly([(cx + r * 0.85, cy), (cx + r * 0.4, cy - r * 0.38), (cx + r * 0.4, cy + r * 0.38)])
    return np.maximum.reduce([np.clip(door - inner, 0, 1), arrow, head])


def g_dollar(sh, cx, cy, r):
    return sh.mask_text("$", cx, cy + r * 0.05, r * 1.7)


def g_smile(sh, cx, cy, r):
    face = sh.mask_circle(cx, cy, r * 0.85)
    eyes = np.maximum(sh.mask_ellipse(cx - r * 0.42, cy - r * 0.4, r * 0.22, r * 0.32), sh.mask_ellipse(cx + r * 0.2, cy - r * 0.4, r * 0.22, r * 0.32))
    mouth = sh.mask_line(_arc(cx, cy + r * 0.0, r * 0.48, math.radians(20), math.radians(160)), r * 0.16)
    return np.clip(face - eyes - mouth, 0, 1)


def g_golfer(sh, cx, cy, r):
    """A golfer at the top of the swing: the skill rating."""
    head = sh.mask_circle(cx - r * 0.05, cy - r * 0.6, r * 0.2)
    body = sh.mask_line([(cx - r * 0.05, cy - r * 0.35), (cx + r * 0.05, cy + r * 0.25)], r * 0.26)
    legs = np.maximum(sh.mask_line([(cx + r * 0.05, cy + r * 0.2), (cx - r * 0.3, cy + r * 0.85)], r * 0.18),
                      sh.mask_line([(cx + r * 0.05, cy + r * 0.2), (cx + r * 0.4, cy + r * 0.85)], r * 0.18))
    arms = sh.mask_line([(cx, cy - r * 0.25), (cx + r * 0.35, cy - r * 0.55)], r * 0.14)
    club = sh.mask_line([(cx + r * 0.35, cy - r * 0.55), (cx - r * 0.6, cy - r * 0.9)], r * 0.08)
    return np.maximum.reduce([head, body, legs, arms, club])
