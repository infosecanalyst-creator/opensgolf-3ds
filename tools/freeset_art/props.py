"""The small things of the free set: hole flags and tee markers, the driving range and putting green flags, benches and the
swing, flower beds, course signs, the ball washer, sailboats and the windsurfer, and the bridges. Modelled like the
landmarks (landmarks.M: parts in the palette ramps with a tone), so the palette files the game draws them in
(sprite.rs `prop_colours` and `flower_colour`, which repeat the colours given here) colour them. Model space: y up, a tile
is 100 units; flags fly towards +x, a bench faces +z."""
import re

import numpy as np

from landmarks import M, ry
from mats import ID
from scene import box, ellipsoid, quad, tube

TAU = 2 * np.pi


# ---- flags and markers -----------------------------------------------------------------------------------------------------

FLAG_COLOURS = {"park": (226, 44, 44), "desert": (244, 140, 36), "trop": (236, 70, 150), "links": (244, 206, 48)}


def flag(t, h=62.0, w=26.0, rise=1.0, cup=True, stripes=False):
    """A pin with a flag rippling in the wind (t the wave's phase); `rise` < 1 while it pops up out of the hole."""
    m = M()
    top = h * rise
    # a stout pin (a thin one vanishes at the game's size) with a ball on top
    m.part(tube([0, 0, 0], [0, top, 0], 1.9, 1.7, 6), "WHITE", 0.97, glow=True)
    m.ell([0, top + 1.0, 0], [2.4, 2.4, 2.4], "YELLOW", 6, 4, 1.15)
    if cup:
        m.cyl([0, 0.2, 0], [0, 0.6, 0], 4.0, 4.0, "DARK", 12, 0.6)
    fw = w * rise
    fh = w * 0.62 * rise
    n = 8
    ph = TAU * t
    for k in range(n):
        u0, u1 = k / n, (k + 1) / n
        def at(u, y):
            # the cloth hangs a little and ripples more towards its free end
            return np.array([u * fw, top - 1 - y - u * fh * 0.12, np.sin(ph * 1.0 - u * 5.0) * u * w * 0.16])
        tone = 1.0 + 0.12 * np.cos(ph - (u0 + u1) * 2.5)
        m.quad(at(u0, 0), at(u1, 0), at(u1, fh * (1 - u1 * 0.15)), at(u0, fh * (1 - u0 * 0.15)), "RED", tone, glow=True)
        if stripes and k % 2 == 1:
            m.quad(at(u0, fh * 0.4) + [0, 0, 0.2], at(u1, fh * 0.4) + [0, 0, 0.2], at(u1, fh * 0.6) + [0, 0, 0.2],
                   at(u0, fh * 0.6) + [0, 0, 0.2], "WHITE", tone)
    return m


def tee_marker(t, colour):
    """A tee marker popping up: a big glossy ball on a short peg, rising out of the ground."""
    m = M()
    rise = min(1.0, t * 1.25)
    over = 1.0 + 0.15 * np.sin(np.pi * min(1.0, max(0.0, (t - 0.6) / 0.4))) if t > 0.6 else 1.0
    r = 5.0 * over * max(rise, 0.15)
    y = 4.0 * rise
    m.cyl([0, 0, 0], [0, max(y, 0.1), 0], 1.2, 1.2, "DARK", 6)
    m.ell([0, y + r * 0.2, 0], [r, r * 0.95, r], colour, 14, 9)
    m.ell([-r * 0.35, y + r * 0.6, r * 0.35], [r * 0.25] * 3, "WHITE", 6, 4, 1.25)
    return m


# ---- benches -------------------------------------------------------------------------------------------------------------------

def bench(kind):
    """A park bench, 50 long: slatted seat and back (none on the backless one), legs of iron or wood."""
    m = M()
    L, D = 50.0, 14.0
    legs = "DARK" if kind in ("iron", "lovers", "woodplank") else "WOOD"
    slat = {"red": "RED", "iron": "DARK"}.get(kind, "WOOD")
    if kind == "lovers":
        # two seats facing opposite ways, joined in an S
        for s in (-1, 1):
            R = ry(90 + 90 * s)
            sub = bench("woodplank")
            t, c, g, e = sub.s.merged()
            m.s.tris.append((t * 0.62) @ R.T + [s * 10, 0, 0])
            m.s.cols.append(c)
            m.s.ttags.append(g)
            m.s.emissive.append(e)
        return m
    if kind == "round":
        # a round bench about a tree's trunk
        for k in range(12):
            a0, a1 = TAU * k / 12, TAU * (k + 1) / 12
            for r0, r1, y in ((16, 24, 12.0),):
                p = [np.array([np.cos(a) * r, y, np.sin(a) * r]) for a, r in ((a0, r0), (a1, r0), (a1, r1), (a0, r1))]
                m.quad(*p, "WOOD", 1.0 if k % 2 else 0.9)
            m.cyl([np.cos(a0) * 20, 0, np.sin(a0) * 20], [np.cos(a0) * 20, 12, np.sin(a0) * 20], 1.0, 1.0, "WOOD", 4, 0.8)
            m.quad([np.cos(a0) * 24, 12, np.sin(a0) * 24], [np.cos(a1) * 24, 12, np.sin(a1) * 24],
                   [np.cos(a1) * 25, 22, np.sin(a1) * 25], [np.cos(a0) * 25, 22, np.sin(a0) * 25], "WOOD", 0.95)
        m.cyl([0, 0, 0], [0, 40, 0], 6, 5, "WOOD", 8, 0.7)
        m.s.add_ball(np.array([0, 48.0, 0]), 16.0, np.ones(3), ID["LEAF"])
        m.s.add_ball(np.array([8, 42.0, 6]), 11.0, np.ones(3), ID["LEAF"])
        m.s.add_ball(np.array([-9, 44.0, -4]), 12.0, np.ones(3), ID["LEAF"])
        return m
    if kind == "box":
        m.box([0, 6, 0], [L, 12, D], "WOOD", tone=0.9)
        for x in np.linspace(-L / 2 + 3, L / 2 - 3, 6):
            m.box([x, 12.3, 0], [L / 7, 0.6, D], "WOOD", tone=1.05)
        m.box([0, 18, -D / 2], [L, 12, 2], "WOOD")
        return m
    if kind == "swing":
        # a porch swing hung from an A frame
        for sx in (-1, 1):
            m.cyl([sx * 30, 0, -12], [sx * 28, 44, 0], 1.4, 1.4, "WOOD", 5)
            m.cyl([sx * 30, 0, 12], [sx * 28, 44, 0], 1.4, 1.4, "WOOD", 5)
            m.cyl([sx * 20, 44, 0], [sx * 20, 14, 0], 0.4, 0.4, "DARK", 3)
        m.cyl([-30, 44, 0], [30, 44, 0], 1.6, 1.6, "WOOD", 5)
        m.box([0, 13, 0], [42, 2, 12], "WHITE")
        m.box([0, 20, -6], [42, 12, 1.6], "WHITE", tone=0.95)
        for x in (-12, 0, 12):
            m.ell([x, 18, -4.6], [4, 4, 1.6], "RED", 8, 5)
        return m
    # legs and arm rests
    for sx in (-1, 1):
        x = sx * (L / 2 - 4)
        m.cyl([x, 0, D / 2 - 2], [x, 12, D / 2 - 2], 1.1, 1.1, legs, 5)
        m.cyl([x, 0, -D / 2 + 2], [x, 12 if kind == "backless" else 24, -D / 2 + 1], 1.1, 1.1, legs, 5)
        if kind != "backless":
            m.cyl([x, 17, D / 2 - 1], [x, 18, -D / 2 + 1], 1.0, 1.0, legs, 5)
            m.cyl([x, 12, D / 2 - 1], [x, 17, D / 2 - 1], 0.9, 0.9, legs, 4)
    for k, z in enumerate(np.linspace(-D / 2 + 2, D / 2 - 2, 4)):
        m.box([0, 12.6, z], [L, 1.2, D / 5], slat, tone=1.0 + 0.04 * (k % 2))
    if kind != "backless":
        for k, y in enumerate((16, 20, 24)):
            m.box([0, y, -D / 2 + 1 - (y - 14) * 0.08], [L - 2, 2.6, 1.0], slat, tone=1.0 - 0.04 * k)
    return m


# ---- flower beds ---------------------------------------------------------------------------------------------------------------

def flower_bed(piece, t, style, wall):
    """A flower bed piece, at the world's scale (beds join tile to tile): mulched soil with leafy plants coming into bloom
    over the frames. A single bed is round; joined pieces are square plots, so they line up whatever their neighbours.
    `style` is plain, sinuous (rounder, looser plants) or tropical (big leaves); `wall` gives a low stone kerb."""
    m = M()
    grow = 0.35 + 0.65 * t
    rng = np.random.default_rng(abs(hash(piece + style)) % 2 ** 32)
    single = piece == "single"
    half = 40.0
    if single:
        n = 20
        ring = [np.array([np.cos(a) * half * 0.85, 3.0, np.sin(a) * half * 0.85]) for a in np.linspace(0, TAU, n + 1)]
        for p, q in zip(ring, ring[1:]):
            m.part([[[0, 3.6, 0], p, q]], "EARTH", 0.9)
            m.part([[p, q, q * [1.04, 0, 1.04]], [p, q * [1.04, 0, 1.04], p * [1.04, 0, 1.04]]], "EARTH", 0.7)
    else:
        m.box([0, 1.5, 0], [half * 2, 3, half * 2], "EARTH", tone=0.9)
    if wall:
        if single:
            for k in range(24):
                a = TAU * k / 24
                m.box([np.cos(a) * half * 0.9, 3, np.sin(a) * half * 0.9], [9, 6, 5], "STONE", rot=-np.degrees(a) + 90,
                      tone=0.95 + 0.08 * (k % 2))
        else:
            for k in range(8):
                x = -half + 5 + k * 10
                for (cx, cz, ang) in ((x, -half + 2.5, 0), (x, half - 2.5, 0), (-half + 2.5, x, 90), (half - 2.5, x, 90)):
                    m.box([cx, 3, cz], [9.6, 6, 5], "STONE", rot=ang, tone=0.95 + 0.08 * (k % 2))
    elif not single:
        # a neat edge of small stones
        for k in range(10):
            x = -half + 4 + k * 8
            for (cx, cz) in ((x, -half + 1.5), (x, half - 1.5), (-half + 1.5, x), (half - 1.5, x)):
                m.ell([cx, 2.6, cz], [3, 1.6, 3], "STONE", 6, 4, 0.95)
    # plants on a loose grid, each a leafy clump with blooms on top
    pitch = 13.0 if style == "trop" else 11.0
    xs = np.arange(-half + 8, half - 7, pitch)
    for i, x in enumerate(xs):
        for j, z in enumerate(xs):
            if single and x * x + z * z > (half * 0.78) ** 2:
                continue
            jx, jz = rng.uniform(-2.5, 2.5, 2) if style != "plain" else rng.uniform(-1, 1, 2)
            c = np.array([x + jx + (pitch / 2 if (j % 2 and style == "sin") else 0), 4, z + jz])
            if abs(c[0]) > half - 5 or abs(c[2]) > half - 5:
                continue
            h = rng.uniform(6, 9) * grow
            r = (4.6 if style == "trop" else 3.8) * grow
            m.s.add_ball(c + [0, h * 0.5, 0], r, np.ones(3), ID["LEAF"])
            if style == "trop":
                for a in rng.uniform(0, TAU, 3):
                    tip = c + [np.cos(a) * r * 2.2, h * 0.3, np.sin(a) * r * 2.2]
                    m.quad(c + [0, h * 0.5, 0], tip + [1.2, 0, 0], tip, tip - [1.2, 0, 0], "LEAF", 0.9)
            if t > 0.3:
                bloom = min(1.0, (t - 0.3) / 0.5)
                for b in range(3):
                    off = rng.normal(size=3) * r * 0.45
                    off[1] = abs(off[1]) * 0.4 + r * 0.6
                    m.ell(c + [0, h * 0.5, 0] + off, [1.6 * bloom + 0.2] * 3, "RED" if (i + j + b) % 5 else "WHITE", 6, 4,
                          1.1 if b == 0 else 1.0)
    return m


# ---- signs, the ball washer -----------------------------------------------------------------------------------------------------

def course_sign(style):
    """The course sign: a board on two posts with a little roof (the theme's style), a stone foot and flowers."""
    m = M()
    post = {"desert": "EARTH", "links": "STONE"}.get(style, "WOOD")
    for sx in (-1, 1):
        m.cyl([sx * 14, 0, 0], [sx * 14, 30, 0], 1.6, 1.4, post, 6)
    m.box([0, 21, 0], [32, 15, 2], "WHITE")
    m.box([0, 21, 0], [34, 17, 1.4], "RED", tone=0.85)
    # 'letters': darker strokes on the board's face
    for k, w in enumerate((18, 24, 14)):
        m.box([0, 25 - k * 4, 1.15], [w, 1.4, 0.3], "DARK", tone=0.8)
    if style == "trop":
        m.hip(-18, -3, 18, 3, 30, 6, "YELLOW", over=2)
    elif style == "desert":
        m.box([0, 31, 0], [38, 3, 6], "EARTH", tone=1.05)
    else:
        m.gable(-18, -3, 18, 3, 30, 6, "ROOF", over=2, wall="WOOD")
    m.box([0, 1.5, 0], [44, 3, 12], "STONE", tone=0.9)
    for x in (-15, -5, 5, 15):
        m.s.add_ball(np.array([x, 5, 6.5]), 3.4, np.ones(3), ID["LEAF"])
        m.ell([x + 1, 7.5, 8.5], [1.3] * 3, "PURPLE", 5, 4)
    return m


def ball_washer(t):
    """A ball washer on its post: the paddle handle pumps up and down, with a towel and a little rubbish bin."""
    m = M()
    m.box([0, 1, 0], [16, 2, 16], "STONE", tone=0.9)
    m.cyl([0, 0, 0], [0, 26, 0], 1.4, 1.2, "DARK", 6)
    m.box([0, 30, 0], [8, 9, 5], "RED")
    m.cyl([0, 34.4, 0], [0, 35.4, 0], 3.2, 3.2, "DARK", 8)
    lift = 4 * (0.5 - 0.5 * np.cos(TAU * t))
    m.cyl([0, 35.4, 0], [0, 38 + lift, 0], 0.6, 0.6, "WHITE", 4)
    m.box([0, 38.5 + lift, 0], [5, 1.4, 1.4], "DARK")
    m.quad([4.2, 32, 2.6], [4.2, 32, -2.6], [4.4, 23, -2.4], [4.4, 23, 2.4], "WHITE", 1.0)
    m.cyl([10, 0, 6], [10, 9, 6], 3.2, 3.6, "LEAF", 10, 0.85)
    return m


# ---- boats ---------------------------------------------------------------------------------------------------------------------

def sailboat(t, sail="WHITE", big=False):
    """A small sloop bobbing on the swell: hull, deck, mast, mainsail and jib."""
    m = M()
    s = 1.25 if big else 1.0
    L, B = 46 * s, 14 * s
    hull = []
    n = 10
    for k in range(n + 1):
        u = k / n
        z = (u - 0.5) * L
        w = B / 2 * (np.sin(np.pi * min(1.0, u * 1.25)) ** 0.6 if u < 0.8 else np.sin(np.pi * u) * 1.2)
        hull.append((z, max(w, 0.4)))
    for (z0, w0), (z1, w1) in zip(hull, hull[1:]):
        for sx in (-1, 1):
            m.quad([sx * w0, 6, z0], [sx * w1, 6, z1], [sx * w1 * 0.55, -1, z1], [sx * w0 * 0.55, -1, z0], "WHITE", 0.95)
        m.quad([-w0, 6, z0], [w0, 6, z0], [w1, 6, z1], [-w1, 6, z1], "WOOD", 1.0)
    for (z0, w0), (z1, w1) in zip(hull, hull[1:]):
        for sx in (-1, 1):
            m.quad([sx * w0 * 1.01, 3.8, z0], [sx * w1 * 1.01, 3.8, z1], [sx * w1 * 1.01, 5.2, z1], [sx * w0 * 1.01, 5.2, z0],
                   "RED", 0.9)
    mast = np.array([0, 6, 4 * s])
    H = 56 * s
    m.cyl(mast, mast + [0, H, 0], 0.7, 0.5, "WOOD", 5)
    m.cyl(mast + [0, 10, 0], mast + [0, 10, -24 * s], 0.5, 0.5, "WOOD", 4)
    belly = 2.5 * np.sin(TAU * t) + 4
    tip = mast + [0, H - 2, 0]
    clew = mast + [0, 10, -24 * s]
    mid = (tip + clew) / 2 + [belly, 0, 0]
    m.part([[mast + [0, 10, 0], tip, mid], [mast + [0, 10, 0], mid, clew]], sail, 1.0)
    m.part([[mast + [0, H * 0.82, 0], mast + [0, 7, 20 * s], mast + [belly * 0.6, 9, 4 * s]]], sail, 0.92)
    if sail == "WHITE":
        m.part([[mast + [0, 12, -2], mid + [0, -6, 0], mid + [0, -1, 0]]], "RED", 1.0)
    m.cyl(tip, tip + [0, 4, 0], 0.3, 0.3, "WOOD", 3)
    m.quad(tip + [0, 4, 0], tip + [0, 4, -5], tip + [0, 2.5, -5], tip + [0, 2.5, 0], "RED", 1.0)
    return m


def windsurfer(t):
    """A windsurfer: a board with a rainbow sail leaning in the wind, and its rider."""
    m = M()
    lean = 10 * np.sin(TAU * t)
    m.ell([0, 1.2, 0], [5, 1.0, 22], "WHITE", 12, 5)
    base = np.array([0, 2.2, 2])
    R = np.array([[1, 0, 0], [0, np.cos(np.radians(lean)), -np.sin(np.radians(lean))],
                  [0, np.sin(np.radians(lean)), np.cos(np.radians(lean))]])
    top = base + R @ np.array([3, 46, -2])
    m.cyl(base, top, 0.6, 0.5, "DARK", 4)
    bands = ["RED", "YELLOW", "LEAF", "PURPLE", "RED"]
    for k, tag in enumerate(bands):
        u0, u1 = k / len(bands), (k + 1) / len(bands)
        a0, a1 = base + (top - base) * (0.08 + u0 * 0.9), base + (top - base) * (0.08 + u1 * 0.9)
        out0 = R @ np.array([2, 0, -18 * (1 - u0 * 0.8)])
        out1 = R @ np.array([2, 0, -18 * (1 - u1 * 0.8)])
        m.quad(a0, a1, a1 + out1, a0 + out0, tag, 1.0)
    # the rider, leaning back on the boom
    m.cyl([0, 2.5, -6], [0, 14, -9], 1.6, 1.6, "SHOE", 5)
    m.ell([0, 20, -10], [3.2, 6, 2.6], "PURPLE", 8, 6)
    m.ell([0, 28, -11], [2.4, 2.6, 2.4], "WHITE", 8, 5, 0.9)
    m.cyl([0, 23, -9], base + R @ np.array([1, 20, -6]), 0.8, 0.8, "WHITE", 4, 0.9)
    return m


# ---- bridges --------------------------------------------------------------------------------------------------------------------

def deck(m, x0, z0, x1, z1, y, tag, along_x):
    """Planks across the run of the deck."""
    if along_x:
        xs = np.arange(x0, x1, 5.0)
        for k, x in enumerate(xs):
            m.box([x + 2.4, y, (z0 + z1) / 2], [4.6, 1.6, z1 - z0], tag, tone=1.0 + 0.06 * ((k * 7) % 3 - 1))
    else:
        zs = np.arange(z0, z1, 5.0)
        for k, z in enumerate(zs):
            m.box([(x0 + x1) / 2, y, z + 2.4], [x1 - x0, 1.6, 4.6], tag, tone=1.0 + 0.06 * ((k * 7) % 3 - 1))


def rail(m, a, b, y, tag, posts=True):
    a, b = np.asarray(a, float), np.asarray(b, float)
    m.cyl(a + [0, y + 8, 0], b + [0, y + 8, 0], 0.9, 0.9, tag, 5, 1.08)
    if posts:
        n = max(2, int(np.linalg.norm(b - a) / 24) + 1)
        for u in np.linspace(0, 1, n):
            p = a + (b - a) * u
            m.cyl(p + [0, y - 2, 0], p + [0, y + 8.6, 0], 1.1, 1.1, tag, 5, 0.92)


BRIDGE_STYLES = {
    "park": dict(deck="WOOD", rail="WHITE", pile="WOOD"),
    "desert": dict(deck="WOOD", rail="EARTH", pile="EARTH"),
    "trop": dict(deck="WOOD", rail="YELLOW", pile="WOOD"),
    "links": dict(deck="STONE", rail="STONE", pile="STONE"),
}


def bridge(piece, style):
    """A path bridge piece at the world's scale (a tile is 100): a deck 50 wide, 8 above the water, on piles, with rails
    along its open sides. The views follow decor.rs `bridge` (map heading 0 is -z, 2 is +x, 4 +z, 6 -x; each view turns
    the piece a quarter): in view 0 TILE runs along x, L joins +z and -x, T joins -z, +z and -x, X all four, and CAP and
    Xtra end in a ramp down to the bank at -x."""
    m = M()
    st = BRIDGE_STYLES[style]
    W, Y = 25.0, 8.0
    arms = {"tile": ("-x", "+x"), "l": ("-x", "+z"), "t": ("-z", "+z", "-x"), "x": ("-x", "+x", "-z", "+z"),
            "xtra": ("-x", "+x"), "cap": ("-x", "+x")}[piece]
    ends = {"-x": (-50, 0), "+x": (50, 0), "-z": (0, -50), "+z": (0, 50)}
    # the middle square and an arm to each joined side
    deck(m, -W, -W, W, W, Y, st["deck"], True)
    for a in arms:
        ex, ez = ends[a]
        if ex:
            x0, x1 = (W, ex) if ex > 0 else (ex, -W)
            deck(m, x0, -W, x1, W, Y, st["deck"], True)
        else:
            z0, z1 = (W, ez) if ez > 0 else (ez, -W)
            deck(m, -W, z0, W, z1, Y, st["deck"], False)
    # piles at the corners of the middle square
    for sx in (-1, 1):
        for sz in (-1, 1):
            m.cyl([sx * (W - 2), -6, sz * (W - 2)], [sx * (W - 2), Y, sz * (W - 2)], 2.2, 2.2, st["pile"], 6, 0.85)
    # rails on the open sides: along every arm, and across the middle square's unjoined sides
    for a in arms:
        ex, ez = ends[a]
        if ex:
            for sz in (-1, 1):
                rail(m, [np.sign(ex) * W, 0, sz * (W - 1)], [ex, 0, sz * (W - 1)], Y, st["rail"])
        else:
            for sx in (-1, 1):
                rail(m, [sx * (W - 1), 0, np.sign(ez) * W], [sx * (W - 1), 0, ez], Y, st["rail"])
    for side, (a0, a1) in {"-x": ((-W, -W), (-W, W)), "+x": ((W, -W), (W, W)), "-z": ((-W, -W), (W, -W)),
                           "+z": ((-W, W), (W, W))}.items():
        if side not in arms:
            rail(m, [a0[0], 0, a0[1]], [a1[0], 0, a1[1]], Y, st["rail"])
    if piece in ("cap", "xtra"):
        # the ramp down to the bank at -x
        for k, x in enumerate(np.arange(-80, -50, 5.0)):
            y = Y - (-50 - x) / 30 * Y
            m.box([x + 2.4, y, 0], [4.6, 1.6, W * 2], st["deck"], tone=0.96 + 0.04 * (k % 2))
    if style == "trop":
        # torches at the corners
        for sx in (-1, 1):
            m.cyl([sx * W, Y, W], [sx * W, Y + 16, W], 0.7, 0.7, "WOOD", 4)
            m.part(ellipsoid([sx * W, Y + 17, W], [1.6, 2.4, 1.6], 6, 4), "YELLOW", 1.2, glow=True)
    return m


def scenic_bridge(kind):
    """A garden footbridge over a stream: a humped deck with rails (stone or brick arches for the stone ones)."""
    m = M()
    stone = kind in ("stone", "brick", "gen03", "gen06", "gen09")
    tag = "STONE" if stone else ("RED" if kind in ("gen04", "gen08") else "WOOD")
    n = 10
    pts = [np.array([(k / n - 0.5) * 100, 4 + 14 * np.sin(np.pi * k / n), 0]) for k in range(n + 1)]
    for p, q in zip(pts, pts[1:]):
        m.quad(p + [0, 0, -18], q + [0, 0, -18], q + [0, 0, 18], p + [0, 0, 18], "STONE" if stone else "WOOD", 1.0)
        if stone:
            m.quad(p + [0, 0, -18], q + [0, 0, -18], q + [0, -10, -18], p + [0, -10, -18], "STONE", 0.9)
            m.quad(p + [0, 0, 18], q + [0, 0, 18], q + [0, -10, 18], p + [0, -10, 18], "STONE", 0.9)
    for sz in (-17, 17):
        for p, q in zip(pts, pts[1:]):
            m.cyl(p + [0, 9, sz], q + [0, 9, sz], 1.0, 1.0, tag, 5, 1.08)
        for p in pts[::2]:
            m.cyl(p + [0, 0, sz], p + [0, 9.5, sz], 1.1, 1.1, tag, 5, 0.95)
    return m


# ---- which file is which ----------------------------------------------------------------------------------------------------------

def theme_of(path):
    p = path.lower()
    for k, v in (("des", "desert"), ("trop", "trop"), ("links", "links")):
        if k in p.rsplit("/", 1)[-1] or f"/{v if v != 'trop' else 'tropical'}/" in p:
            return v
    return "park"


def flag_tags(name):
    n = name.lower()
    if "white" in n:
        return "WHITE"
    if "yflag" in n or "dflag" in n:
        return "YELLOW"
    if "pflag" in n:
        return "PURPLE"
    return "RED"


def prop(path, t):
    """(model, palette colours, how it is fitted: 'world' or 'fit') of a prop sprite file at phase t, or None."""
    low = path.lower().replace("\\", "/")
    name = low.rsplit("/", 1)[1][:-4]
    th = theme_of(low)
    if "/tees/flag" in low:
        th = next((v for k, v in (("park", "park"), ("desert", "desert"), ("trop", "trop"), ("links", "links")) if k in name))
        rise = min(1.0, t * 1.2) if name.endswith("_pop") else 1.0
        return flag(t, rise=rise), {"RED": FLAG_COLOURS[th], "WHITE": (244, 244, 240), "YELLOW": (240, 200, 60),
                                    "DARK": (30, 30, 30)}, "fit"
    if "/tees/teemarker" in low:
        tag = "RED" if "red" in name else ("PURPLE" if "blue" in name else "WHITE")
        return tee_marker(t, tag), {"RED": (220, 40, 40), "PURPLE": (50, 90, 220), "WHITE": (244, 244, 240),
                                     "DARK": (70, 60, 50)}, "fit"
    if "/bldgs/" in low and "flag" in name and "tv" not in name:
        tag = flag_tags(name)
        m = flag(t, h=50, w=24, cup="putt" in name or "pg_" in name)
        if tag != "RED":
            for k in range(len(m.s.ttags)):
                m.s.ttags[k] = np.where(m.s.ttags[k] == ID["RED"], ID[tag], m.s.ttags[k])
        return m, {"RED": (226, 44, 44), "WHITE": (244, 244, 240), "YELLOW": (244, 206, 48), "PURPLE": (150, 80, 200),
                   "DARK": (30, 30, 30)}, "fit"
    if "bench" in name or name == "benw01" or name == "swing":
        kind = next((k for k in ("iron", "backless", "box", "lovers", "red", "round", "woodplank") if k in name), "woodplank")
        if name == "swing":
            kind = "swing"
        return bench(kind), {"WOOD": (156, 110, 66), "DARK": (54, 58, 62), "RED": (196, 54, 46), "WHITE": (240, 236, 226),
                             "LEAF": (80, 140, 60)}, "fit"
    m = re.match(r"(flowers|sinflowers|walledflowers|desflowers|dessin|tropflowers|sintrop|tropfb)_(single|1side|2side|3side|"
                 r"4side|corner)(_wall)?$", name)
    if m:
        fam, piece, wall = m.group(1), m.group(2), bool(m.group(3)) or m.group(1) == "walledflowers"
        style = "sin" if "sin" in fam else ("trop" if "trop" in fam else "plain")
        return flower_bed(piece, t, style, wall), {"LEAF": (70, 140, 56), "EARTH": (110, 76, 52), "STONE": (176, 168, 152),
                                                  "RED": (226, 54, 74), "WHITE": (246, 244, 236)}, "world"
    if name in ("sign", "dessign", "tropsign", "sign_links"):
        return course_sign(th), {"WOOD": (130, 90, 56), "WHITE": (246, 242, 230), "RED": (40, 110, 70), "DARK": (40, 60, 50),
                                 "ROOF": (70, 90, 70), "STONE": (170, 164, 150), "EARTH": (196, 146, 100),
                                 "YELLOW": (210, 180, 110), "LEAF": (80, 140, 60), "PURPLE": (200, 80, 160)}, "fit"
    if name == "upgrade bwasher":
        return ball_washer(t), {"STONE": (170, 164, 150), "DARK": (60, 66, 70), "RED": (40, 110, 70), "WHITE": (244, 244, 240),
                                "LEAF": (60, 110, 70)}, "fit"
    if name.startswith("sailboat"):
        sail = "WHITE"
        return sailboat(t, sail, big="trop" in name), {"WHITE": (246, 246, 240), "WOOD": (170, 124, 80), "RED": (210, 50, 50)}, "fit"
    if name == "rainbow sail":
        return windsurfer(t), {"WHITE": (246, 246, 240), "DARK": (60, 60, 60), "RED": (226, 50, 50), "YELLOW": (250, 210, 50),
                               "LEAF": (60, 170, 80), "PURPLE": (80, 90, 220), "SHOE": (230, 180, 140)}, "fit"
    m = re.match(r"(des|trop|links)?bridge(cap|l|t|tile|x|xtra)$", name)
    if m:
        style = {"des": "desert", "trop": "trop", "links": "links", None: "park"}[m.group(1)]
        return bridge(m.group(2), style), {"WOOD": (150, 108, 70), "WHITE": (240, 236, 226), "EARTH": (186, 136, 92),
                                           "YELLOW": (220, 190, 120), "STONE": (160, 154, 142)}, "world"
    m = re.match(r"scenicgen(\d\d)$", name)
    if m or name in ("bridgebrick", "bridgestone", "scenicbridge"):
        kind = "gen" + m.group(1) if m else name.replace("bridge", "").replace("scenic", "") or "wood"
        return scenic_bridge(kind), {"WOOD": (150, 108, 70), "STONE": (168, 160, 146), "RED": (178, 64, 50)}, "fit"
    return None
