"""The landmarks of the free set: windmills, lighthouses, statues, a pagoda, a barn, a volcano and the rest, modelled from a
few solids in the palette ramps (mats.py) so the landmark palette files (sprite.rs `landmark_colours`, which repeats the
colours given here) colour them in the game. A part's tone (its colour's mean over 0.5) makes trim and joints lighter or
darker within a ramp. World units: a tile is 100; the landmark stands on the origin. Animated ones (the sails of the
windmills, the oil pump's beam) take the frame's phase."""
import numpy as np

from mats import ID
from scene import Scene, box, ellipsoid, quad, tube


class M:
    """A model being built: parts in a material ramp, with a tone."""

    def __init__(self):
        self.s = Scene()
        self.s.tone = True
        self.s.leafy, self.s.rag = 0.3, 0.2

    def part(self, tris, tag, tone=1.0, glow=False):
        tris = np.asarray(tris, float).reshape(-1, 3, 3)
        self.s.add_tris(tris, [0.5 * tone] * 3, ID[tag], emissive=glow)

    def cyl(self, a, b, r0, r1, tag, sides=10, tone=1.0, caps=True):
        self.part(tube(a, b, r0, r1, sides, caps), tag, tone)

    def box(self, c, size, tag, rot=0.0, tone=1.0):
        self.part(box(c, size, rot), tag, tone)

    def ell(self, c, r, tag, n=12, m=8, tone=1.0):
        self.part(ellipsoid(c, r, n, m), tag, tone)

    def leaves(self, c, r, tag="LEAF"):
        self.s.add_ball(c, r, np.array([0.3, 0.5, 0.25]), ID[tag])

    def quad(self, a, b, c, d, tag, tone=1.0, glow=False):
        self.part(quad(a, b, c, d), tag, tone, glow)

    def pyramid(self, c, w, d, h, tag, tone=1.0, top=0.0):
        """A (truncated) pyramid on the rectangle w x d centred at c (its base), rising h; `top` the share of the top."""
        c = np.asarray(c, float)
        b = [c + [sx * w / 2, 0, sz * d / 2] for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        t = [c + [sx * w / 2 * top, h, sz * d / 2 * top] for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        for i in range(4):
            j = (i + 1) % 4
            self.quad(b[i], b[j], t[j], t[i], tag, tone)
        if top > 0:
            self.quad(t[0], t[1], t[2], t[3], tag, tone)

    def gable(self, x0, z0, x1, z1, y, h, tag, over=4.0, wall="WALL", tone=1.0):
        """A gable roof along x over the rectangle, with its two gable ends in the wall's material."""
        cz = (z0 + z1) / 2
        a, b = np.array([x0 - over, y, z0 - over]), np.array([x1 + over, y, z0 - over])
        c, d = np.array([x1 + over, y, z1 + over]), np.array([x0 - over, y, z1 + over])
        r0, r1 = np.array([x0 - over, y + h, cz]), np.array([x1 + over, y + h, cz])
        self.quad(a, b, r1, r0, tag, tone)
        self.quad(c, d, r0, r1, tag, tone * 0.96)
        for x in (x0, x1):
            self.part([[[x, y, z0], [x, y, z1], [x, y + h, cz]]], wall)
        # the ridge
        self.cyl(r0 + [0, 0.5, 0], r1 + [0, 0.5, 0], 1.2, 1.2, tag, 5, tone * 1.25)

    def hip(self, x0, z0, x1, z1, y, h, tag, over=4.0, tone=1.0, flare=0.0):
        """A hipped roof; `flare` lifts the eave corners (a Chinese roof)."""
        x0, z0, x1, z1 = x0 - over, z0 - over, x1 + over, z1 + over
        cx, cz = (x0 + x1) / 2, (z0 + z1) / 2
        dx, dz = (x1 - x0) / 2, (z1 - z0) / 2
        r = min(dx, dz)
        corners = [np.array([x0, y, z0]), np.array([x1, y, z0]), np.array([x1, y, z1]), np.array([x0, y, z1])]
        for p in corners:
            p[1] += flare
        if dx >= dz:
            r1, r2 = np.array([cx - (dx - r), y + h, cz]), np.array([cx + (dx - r), y + h, cz])
            faces = [(0, 1, r2, r1), (2, 3, r1, r2), (3, 0, r1, r1), (1, 2, r2, r2)]
        else:
            r1, r2 = np.array([cx, y + h, cz - (dz - r)]), np.array([cx, y + h, cz + (dz - r)])
            faces = [(0, 1, r1, r1), (2, 3, r2, r2), (3, 0, r1, r2), (1, 2, r2, r1)]
        for i, j, rr, ss in faces:
            p, q = corners[i], corners[j]
            if flare:
                # a sagging middle on each eave
                mid = (p + q) / 2 - [0, flare * 0.9, 0]
                self.part([[p, mid, ss], [mid, q, rr], [mid, rr, ss]], tag, tone)
            else:
                self.part([[p, q, rr], [p, rr, ss]], tag, tone)
        if np.linalg.norm(r2 - r1) > 1e-3:
            self.cyl(r1 + [0, 0.5, 0], r2 + [0, 0.5, 0], 1.3, 1.3, tag, 5, tone * 1.25)

    def lattice(self, base_w, top_w, h, tag, levels=8, tone=1.0, r=0.9, y0=0.0):
        """A four-legged lattice tower with cross bracing."""
        def corner(t, sx, sz):
            w = base_w + (top_w - base_w) * t
            return np.array([sx * w / 2, y0 + h * t, sz * w / 2])
        cs = ((-1, -1), (1, -1), (1, 1), (-1, 1))
        for sx, sz in cs:
            self.cyl(corner(0, sx, sz), corner(1, sx, sz), r * 1.3, r, tag, 5, tone)
        for k in range(levels):
            t0, t1 = k / levels, (k + 1) / levels
            for i in range(4):
                (ax, az), (bx, bz) = cs[i], cs[(i + 1) % 4]
                self.cyl(corner(t1, ax, az), corner(t1, bx, bz), r * 0.7, r * 0.7, tag, 4, tone)
                self.cyl(corner(t0, ax, az), corner(t1, bx, bz), r * 0.5, r * 0.5, tag, 4, tone * 0.9)


def ry(deg):
    a = np.radians(deg)
    c, s = np.cos(a), np.sin(a)
    return np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])


def rz(deg):
    a = np.radians(deg)
    c, s = np.cos(a), np.sin(a)
    return np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])


def rx(deg):
    a = np.radians(deg)
    c, s = np.cos(a), np.sin(a)
    return np.array([[1, 0, 0], [0, c, -s], [0, s, c]])


# ---- the landmarks ---------------------------------------------------------------------------------------------------------

def plinth(m, w, h, tag="STONE", steps=2):
    """A stepped stone base, w across, h high; returns its top."""
    y = 0.0
    for k in range(steps):
        ww = w * (1 - 0.12 * k)
        m.box([0, y + h / steps / 2, 0], [ww, h / steps, ww], tag, tone=1.0 - 0.05 * k)
        y += h / steps
    return y


def sundial(t, wide=False):
    m = M()
    if wide:
        for k, (r, hh) in enumerate(((30, 3), (25, 3), (20, 3))):
            m.cyl([0, k * 3, 0], [0, k * 3 + hh, 0], r, r, "STONE", 16, 1.0 - 0.06 * k)
        y = 9.0
        m.cyl([0, y, 0], [0, y + 14, 0], 5, 4, "STONE", 8, 1.05)
        y += 14
    else:
        m.box([0, 2, 0], [26, 4, 26], "STONE", tone=0.92)
        m.cyl([0, 4, 0], [0, 20, 0], 5.5, 4.5, "STONE", 8, 1.05)
        m.cyl([0, 20, 0], [0, 23, 0], 8, 8, "STONE", 8)
        y = 23.0
    m.cyl([0, y, 0], [0, y + 1.5, 0], 11, 11, "GOLD", 18)
    # hour marks and the gnomon
    for k in range(12):
        a = 2 * np.pi * k / 12
        p = np.array([np.cos(a) * 9, y + 1.6, np.sin(a) * 9])
        m.box(p, [1.4, 0.3, 1.4], "GOLD", tone=0.7)
    m.part([[[-8, y + 1.5, 0], [6, y + 1.5, 0], [-8, y + 11, 0]]], "GOLD", 1.1)
    return m, {"STONE": (190, 184, 170), "GOLD": (196, 150, 70)}


def barn(t):
    m = M()
    W, D, H = 48, 32, 30
    m.box([0, H / 2, 0], [W * 2, H, D * 2], "WALL")
    # board lines
    for x in np.linspace(-W + 6, W - 6, 15):
        for z in (-D - 0.3, D + 0.3):
            m.box([x, H / 2, z], [0.8, H, 0.4], "WALL", tone=0.82)
    # a gambrel roof: steep lower slopes, shallow upper ones
    for sz in (-1, 1):
        a = np.array([-W - 3, H, sz * (D + 3)])
        b = np.array([W + 3, H, sz * (D + 3)])
        c = np.array([W + 3, H + 18, sz * D * 0.55])
        d = np.array([-W - 3, H + 18, sz * D * 0.55])
        m.quad(a, b, c, d, "ROOF", 0.95)
        e = np.array([W + 3, H + 27, 0])
        f = np.array([-W - 3, H + 27, 0])
        m.quad(d, c, e, f, "ROOF", 1.05)
    for sx in (-1, 1):
        x = sx * W
        pts = [[x, H, -D], [x, H + 18, -D * 0.55], [x, H + 27, 0], [x, H + 18, D * 0.55], [x, H, D]]
        for i in range(1, 4):
            m.part([[pts[0], pts[i], pts[i + 1]]], "WALL")
        # big doors with a white X, and a hay loft door
        m.box([x + sx * 0.6, 13, 0], [0.6, 26, 26], "WALL", tone=0.8)
        for zz, yy in ((-1, 1), (1, 1)):
            m.cyl([x + sx * 0.9, 1, -12 * zz], [x + sx * 0.9, 25, 12 * zz], 0.9, 0.9, "WHITE", 4)
        for zz in (-13, 13):
            m.cyl([x + sx * 0.9, 0, zz], [x + sx * 0.9, 26, zz], 1.0, 1.0, "WHITE", 4)
        m.cyl([x + sx * 0.9, 26, -13], [x + sx * 0.9, 26, 13], 1.0, 1.0, "WHITE", 4)
        m.box([x + sx * 0.8, H + 10, 0], [0.6, 10, 10], "DARK", tone=0.8)
    # corner trim
    for sx in (-1, 1):
        for sz in (-1, 1):
            m.box([sx * W, H / 2, sz * D], [1.6, H, 1.6], "WHITE")
    # a weathervane and a hay bale
    m.cyl([0, H + 27, 0], [0, H + 38, 0], 0.5, 0.5, "DARK", 4)
    m.box([0, H + 36, 0], [8, 0.8, 1.2], "DARK")
    m.cyl([-W - 14, 5, 18], [-W - 14, 5, 30], 5, 5, "YELLOW", 10)
    return m, {"WALL": (176, 58, 46), "ROOF": (92, 94, 100), "WHITE": (238, 234, 222), "DARK": (60, 54, 50),
               "YELLOW": (220, 190, 100)}


def wheel(m, c, axis, r, tag, spokes=10):
    axis = np.asarray(axis, float) / np.linalg.norm(axis)
    up = np.array([0, 1.0, 0])
    side = np.cross(axis, up)
    side /= np.linalg.norm(side)
    n = 16
    for k in range(n):
        a0, a1 = 2 * np.pi * k / n, 2 * np.pi * (k + 1) / n
        p0 = c + (np.cos(a0) * up + np.sin(a0) * side) * r
        p1 = c + (np.cos(a1) * up + np.sin(a1) * side) * r
        m.cyl(p0, p1, r * 0.09, r * 0.09, tag, 4)
    for k in range(spokes):
        a = 2 * np.pi * k / spokes
        m.cyl(c, c + (np.cos(a) * up + np.sin(a) * side) * r, r * 0.04, r * 0.04, tag, 3, 0.9)
    m.cyl(c - axis * r * 0.15, c + axis * r * 0.15, r * 0.14, r * 0.14, tag, 6, 0.8)


def cannon(t):
    m = M()
    # the carriage trail resting on the ground, two wheels, the barrel pointing ahead
    m.box([-14, 5, 0], [34, 4, 7], "WOOD", rot=0, tone=0.95)
    m.part(tube([-30, 0.5, 0], [-2, 9, 0], 2.4, 2.4, 4), "WOOD", 0.9)
    for z in (-9, 9):
        wheel(m, np.array([0, 11, z]), [0, 0, 1], 11, "WOOD")
    m.cyl([0, 11, -10], [0, 11, 10], 1.2, 1.2, "DARK", 6)
    m.cyl([-12, 15, 0], [24, 17, 0], 4.2, 3.0, "GOLD", 12)
    m.ell([-13, 15, 0], [3.5, 3.5, 3.5], "GOLD")
    m.cyl([24, 17, 0], [26, 17.2, 0], 3.6, 3.6, "GOLD", 12, 1.1)
    # a stack of shot
    for i, (x, z, y) in enumerate(((18, 18, 2.4), (23, 18, 2.4), (20.5, 22, 2.4), (20.5, 19.5, 6.2))):
        m.ell([x, y, z], [2.4, 2.4, 2.4], "DARK", 8, 6)
    return m, {"WOOD": (120, 84, 52), "DARK": (52, 52, 56), "GOLD": (150, 112, 64)}


def stones(t):
    m = M()
    # a trilithon and fallen stones on a low mound of grass
    def stone(c, size, tilt=0.0, yaw=0.0, tone=1.0):
        sx, sy, sz = size
        tri = box([0, 0, 0], [sx, sy, sz])
        # roughen: bulge the faces a little by pulling the corners in at the top
        tri = tri * np.array([1.0, 1.0, 1.0])
        tri[..., 0] *= 1 - 0.12 * (tri[..., 1] > 0)
        tri[..., 2] *= 1 - 0.12 * (tri[..., 1] > 0)
        tri = tri @ rz(tilt).T @ ry(yaw).T + np.asarray(c, float)
        m.part(tri, "STONE", tone)
    m.ell([0, -6, 0], [70, 10, 55], "LEAF", 16, 6, 0.95)
    stone([-16, 26, 0], [12, 52, 10])
    stone([16, 26, 0], [12, 52, 10], tone=0.96)
    stone([0, 55, 0], [48, 10, 11], tone=1.05)
    stone([-44, 18, 18], [10, 40, 9], tilt=6, yaw=30, tone=0.94)
    stone([40, 16, -20], [11, 34, 9], tilt=-8, yaw=-20)
    stone([30, 4, 28], [34, 8, 10], yaw=40, tone=0.9)
    return m, {"STONE": (150, 146, 136), "LEAF": (96, 140, 66)}


def farm_windmill(t):
    """An American farm windmill: a lattice tower, a many-bladed wheel turning, a tail vane."""
    m = M()
    m.lattice(30, 6, 96, "WOOD", levels=6, r=1.1)
    m.box([0, 97, 0], [8, 3, 8], "DARK")
    hub = np.array([0, 102, 7])
    ph = 2 * np.pi * t / 18
    n = 18
    for k in range(n):
        a = ph + 2 * np.pi * k / n
        d = np.array([np.cos(a), np.sin(a), 0])
        p0 = hub + d * 5
        p1 = hub + d * 24
        side = np.array([-np.sin(a), np.cos(a), 0]) * 2.6
        m.quad(p0 - side * 0.5, p0 + side * 0.5 + [0, 0, 1.0], p1 + side + [0, 0, 1.6], p1 - side, "WHITE",
               1.0 if k % 2 else 0.88)
    m.cyl(hub - [0, 0, 2], hub + [0, 0, 2], 4.5, 4.5, "RED", 8)
    for r in (13, 24):
        pts = [hub + np.array([np.cos(a), np.sin(a), 0.5]) * r for a in np.linspace(0, 2 * np.pi, 19)]
        for p, q in zip(pts, pts[1:]):
            m.cyl(p, q, 0.6, 0.6, "DARK", 3)
    m.cyl(hub - [0, 0, 2], hub - [0, 0, 30], 1.0, 1.0, "DARK", 4)
    m.quad(hub + [0, -6, -18], hub + [0, -4, -34], hub + [0, 9, -34], hub + [0, 7, -18], "RED", 1.0)
    # a water trough at the foot
    m.box([20, 4, 18], [26, 8, 10], "WOOD", tone=0.85)
    m.box([20, 7.6, 18], [23, 0.6, 7.5], "WATER")
    return m, {"WOOD": (150, 120, 84), "DARK": (70, 66, 64), "WHITE": (232, 230, 220), "RED": (190, 60, 50),
               "WATER": (70, 130, 170)}


def boulders(t):
    m = M()
    for c, r, tone in (([0, 16, 0], [30, 24, 26], 1.0), ([-26, 9, 16], [16, 13, 15], 0.94), ([22, 7, 18], [13, 10, 12], 0.98),
                       ([16, 26, -6], [16, 14, 14], 1.04)):
        m.ell(c, r, "STONE", 14, 9, tone)
    # moss and grass tufts at the foot
    for c in ([-12, 2, 24], [10, 2, 26], [30, 2, 2], [-32, 2, 0]):
        m.leaves(c, 6)
    return m, {"STONE": (142, 138, 128), "LEAF": (90, 130, 60)}


def statue_figure(m, pose, tag, scale=1.0, move=(0, 0, 0), rot=None):
    import people
    body = people.Body(hat="none", sleeves="long", legs="pants")
    fig = people.figure(body, pose).transformed(scale, rot=rot, move=move)
    t, c, g, e = fig.merged()
    m.part(t, tag, 1.0)


def soldier(t):
    m = M()
    y = plinth(m, 40, 30, "STONE", steps=3)
    m.box([0, y + 1, 0], [26, 2, 26], "STONE", tone=1.08)
    import people
    pose = dict(people.DEFAULT, lsp=10, rsp=150, rsr=20, rel=20, hp=-10, lhp=12, lkn=8)
    statue_figure(m, pose, "GOLD", 0.85, move=[0, y + 2, 0])
    # the sword raised
    m.cyl([-6, y + 66, 2], [-8, y + 86, 4], 0.7, 0.4, "GOLD", 4, 1.2)
    # a plaque
    m.box([0, y * 0.55, -20.5], [14, 8, 0.6], "GOLD", tone=0.9)
    return m, {"STONE": (184, 178, 166), "GOLD": (110, 128, 104)}


def lighthouse(t, tall=False):
    m = M()
    H = 110 if tall else 64
    r0, r1 = (15, 10) if tall else (14, 10)
    if not tall:
        m.cyl([0, 0, 0], [0, 8, 0], 22, 22, "STONE", 14, 0.95)
    bands = 6 if tall else 4
    y0 = 0 if tall else 8
    for k in range(bands):
        a, b = y0 + (H - y0) * k / bands, y0 + (H - y0) * (k + 1) / bands
        ra = r0 + (r1 - r0) * k / bands
        rb = r0 + (r1 - r0) * (k + 1) / bands
        m.cyl([0, a, 0], [0, b, 0], ra, rb, "RED" if k % 2 else "WHITE", 14)
    # gallery, lantern and cap
    m.cyl([0, H, 0], [0, H + 2, 0], r1 + 4, r1 + 4, "DARK", 14)
    for k in range(12):
        a = 2 * np.pi * k / 12
        m.cyl([np.cos(a) * (r1 + 3.5), H + 2, np.sin(a) * (r1 + 3.5)], [np.cos(a) * (r1 + 3.5), H + 6, np.sin(a) * (r1 + 3.5)],
              0.4, 0.4, "DARK", 3)
    m.part(tube([0, H + 2, 0], [0, H + 12, 0], r1 * 0.7, r1 * 0.7, 10), "YELLOW", 1.0, glow=True)
    for k in range(6):
        a = 2 * np.pi * k / 6
        m.cyl([np.cos(a) * r1 * 0.72, H + 2, np.sin(a) * r1 * 0.72], [np.cos(a) * r1 * 0.72, H + 12, np.sin(a) * r1 * 0.72],
              0.6, 0.6, "DARK", 3)
    m.cyl([0, H + 12, 0], [0, H + 19, 0], r1 * 0.85, 1.0, "DARK", 12)
    m.ell([0, H + 20, 0], [1.6, 1.6, 1.6], "DARK", 6, 4)
    # windows up the tower
    for k in range(1, 4):
        y = y0 + (H - y0) * (k / 4)
        r = r0 + (r1 - r0) * k / 4
        m.box([0, y, -r - 0.3], [3, 5, 0.8], "DARK", tone=0.7)
    if tall:
        # the keeper's cottage
        m.box([30, 12, 6], [30, 24, 24], "WHITE", tone=0.98)
        m.gable(15, -6, 45, 18, 24, 12, "ROOF", over=3, wall="WHITE")
        m.box([30, 10, -6.3], [6, 12, 0.6], "RED", tone=0.8)
        for x in (21, 39):
            m.box([x, 14, -6.3], [5, 6, 0.6], "DARK", tone=0.7)
    return m, {"WHITE": (240, 238, 230), "RED": (196, 52, 48), "DARK": (54, 60, 70), "YELLOW": (255, 226, 120),
               "STONE": (150, 146, 138), "ROOF": (70, 74, 84)}


def buddha(t):
    m = M()
    y = plinth(m, 54, 12, "STONE", steps=2)
    # a lotus throne
    for k in range(12):
        a = 2 * np.pi * k / 12
        m.ell([np.cos(a) * 17, y + 3, np.sin(a) * 17], [7, 4, 5], "RED", 8, 5, 1.0)
    m.cyl([0, y, 0], [0, y + 6, 0], 18, 16, "RED", 14, 0.9)
    y += 6
    # crossed legs, body, arms in the lap, head with a topknot
    m.ell([0, y + 6, 2], [20, 7, 12], "GOLD", 14, 8)
    m.ell([0, y + 20, 0], [13, 17, 10], "GOLD", 14, 10, 1.02)
    m.ell([0, y + 11, 8], [9, 4, 5], "GOLD", 10, 6, 0.95)
    for sx in (-1, 1):
        m.ell([sx * 13, y + 22, 1], [4.5, 11, 5], "GOLD", 10, 6, 0.97)
    m.ell([0, y + 41, 0], [8, 9.5, 8.5], "GOLD", 14, 10, 1.05)
    m.ell([0, y + 50, -1], [4, 4, 4], "GOLD", 10, 6)
    for sx in (-1, 1):
        m.ell([sx * 8, y + 38, 0], [1.5, 5, 2.2], "GOLD", 6, 5)
    # a halo
    pts = [np.array([np.cos(a) * 16, y + 42 + np.sin(a) * 16, -10]) for a in np.linspace(0, 2 * np.pi, 25)]
    for p, q in zip(pts, pts[1:]):
        m.cyl(p, q, 1.3, 1.3, "GOLD", 4, 0.85)
    return m, {"STONE": (170, 162, 146), "GOLD": (214, 168, 70), "RED": (176, 64, 60)}


def dutch_windmill(t):
    m = M()
    # a brick base, a tapered timber tower, a cap and four sails turning
    m.cyl([0, 0, 0], [0, 14, 0], 30, 29, "STONE", 8, 0.95)
    m.cyl([0, 14, 0], [0, 15.5, 0], 33, 33, "WOOD", 8, 0.85)
    for k in range(8):
        a = 2 * np.pi * (k + 0.5) / 8
        m.cyl([np.cos(a) * 32, 15.5, np.sin(a) * 32], [np.cos(a) * 32, 21, np.sin(a) * 32], 0.5, 0.5, "WOOD", 3, 0.8)
    m.cyl([0, 14, 0], [0, 76, 0], 27, 16, "WALL", 8)
    m.box([0, 8, -29.5], [10, 16, 1], "WOOD", tone=0.7)
    for y in (32, 54):
        m.box([0, y, -24 + (y - 14) * 0.18], [6, 8, 1], "DARK", tone=0.6)
    m.cyl([0, 76, 0], [0, 80, 0], 18, 18, "WOOD", 8, 0.85)
    m.ell([0, 80, 0], [18, 16, 20], "ROOF", 10, 6)
    hub = np.array([0, 84, -21])
    m.cyl(hub + [0, 0, 6], hub - [0, 0, 3], 3, 3, "WOOD", 8, 0.8)
    ph = 2 * np.pi * t / 4
    for k in range(4):
        a = ph + np.pi / 2 * k + 0.3
        d = np.array([np.cos(a), np.sin(a), 0])
        side = np.array([-np.sin(a), np.cos(a), 0])
        tip = hub + d * 50
        m.cyl(hub, tip, 1.4, 1.0, "WOOD", 4)
        # the lattice sail with its cloth
        p0, p1 = hub + d * 12, tip
        m.quad(p0 + side * 1.5, p1 + side * 1.5, p1 + side * 12 - [0, 0, 1], p0 + side * 10 - [0, 0, 1], "WHITE", 1.0)
        for s in np.linspace(0.2, 1.0, 5):
            q = hub + d * (12 + 38 * s)
            m.cyl(q + side * 1.5, q + side * 12, 0.5, 0.5, "WOOD", 3, 0.9)
    return m, {"STONE": (150, 84, 64), "WALL": (226, 214, 190), "WOOD": (110, 80, 56), "ROOF": (96, 92, 88),
               "WHITE": (240, 234, 220), "DARK": (60, 60, 66)}


def horse_rider(t):
    m = M()
    y = plinth(m, 60, 34, "STONE", steps=2)
    m.box([0, y + 1, 0], [56, 2, 30], "STONE", tone=1.06)
    y += 2
    # the horse: a body, neck and head, four legs (one raised), a tail
    m.ell([0, y + 30, 0], [22, 10, 9], "GOLD", 14, 9)
    m.cyl([16, y + 34, 0], [26, y + 50, 0], 6, 4.4, "GOLD", 10)
    m.ell([30, y + 50, 0], [8, 4, 4], "GOLD", 10, 6)
    m.ell([27, y + 54, 0], [2.4, 3, 1.6], "GOLD", 6, 4)
    for x, z, raised in ((15, -5, True), (15, 5, False), (-15, -5, False), (-15, 5, False)):
        if raised:
            m.cyl([x, y + 26, z], [x + 10, y + 16, z], 2.6, 2.0, "GOLD", 6)
            m.cyl([x + 10, y + 16, z], [x + 8, y + 8, z], 2.0, 1.8, "GOLD", 6)
        else:
            m.cyl([x, y + 26, z], [x + (2 if x > 0 else -2), y, z], 2.8, 1.8, "GOLD", 6)
    m.cyl([-21, y + 32, 0], [-30, y + 14, 0], 2.5, 1.0, "GOLD", 6)
    # the rider
    import people
    pose = dict(people.DEFAULT, py=0, lhp=70, rhp=70, lkn=70, rkn=70, lhr=30, rhr=30, rsp=120, rel=20, lsp=40, lel=60, hp=-5)
    statue_figure(m, pose, "GOLD", 0.72, move=[2, y + 16, 0], rot=ry(90))
    return m, {"STONE": (186, 180, 170), "GOLD": (104, 122, 100)}


def moai(t):
    m = M()
    m.ell([0, -4, 0], [56, 8, 40], "LEAF", 14, 6, 0.95)
    for x, z, h, yaw in ((-22, 4, 62, 8), (4, -2, 70, -4), (28, 6, 56, 12)):
        R = ry(yaw)
        def P(p):
            return np.asarray(p, float) @ R.T + [x, 0, z]
        # a long head on a torso: a tapered block with a heavy brow, nose and chin
        w = h * 0.32
        tri = tube([0, 0, 0], [0, h, 0], w * 0.55, w * 0.48, 8)
        tri[..., 2] *= 0.8
        m.part(tri @ R.T + [x, 0, z], "STONE", 1.0)
        m.box(P([0, h * 0.72, -w * 0.42]), [w * 0.95, h * 0.07, w * 0.25], "STONE", rot=yaw, tone=0.9)
        m.part(tube(P([0, h * 0.7, -w * 0.4]), P([0, h * 0.5, -w * 0.62]), w * 0.12, w * 0.2, 6), "STONE", 1.08)
        m.box(P([0, h * 0.4, -w * 0.4]), [w * 0.5, h * 0.05, w * 0.16], "STONE", rot=yaw, tone=0.75)
        m.box(P([0, h * 1.02, 0]), [w * 0.8, h * 0.08, w * 0.7], "RED", rot=yaw, tone=0.9)
        for sx in (-1, 1):
            m.box(P([sx * w * 0.24, h * 0.64, -w * 0.4]), [w * 0.2, h * 0.04, w * 0.1], "DARK", rot=yaw, tone=0.7)
    return m, {"STONE": (132, 126, 116), "LEAF": (96, 140, 66), "RED": (150, 76, 58), "DARK": (60, 56, 52)}


def pagoda(t):
    m = M()
    m.box([0, 3, 0], [64, 6, 64], "STONE", tone=0.95)
    y = 6.0
    w = 44.0
    for k in range(5):
        h = 18 - k * 1.5
        m.box([0, y + h / 2, 0], [w, h, w], "WALL", tone=1.0)
        for sx in (-1, 1):
            for sz in (-1, 1):
                m.cyl([sx * w / 2, y, sz * w / 2], [sx * w / 2, y + h, sz * w / 2], 1.3, 1.3, "WALL", 6, 0.75)
        m.box([0, y + h * 0.45, -w / 2 - 0.3], [w * 0.3, h * 0.6, 0.6], "DARK", tone=0.6)
        m.hip(-w / 2, -w / 2, w / 2, w / 2, y + h, 8, "ROOF", over=8, flare=4)
        y += h + 4
        w *= 0.8
    m.cyl([0, y, 0], [0, y + 26, 0], 1.6, 0.6, "GOLD", 6)
    for k in range(5):
        m.cyl([0, y + 3 + k * 3.4, 0], [0, y + 4.2 + k * 3.4, 0], 4 - k * 0.5, 4 - k * 0.5, "GOLD", 8)
    return m, {"STONE": (170, 162, 150), "WALL": (178, 56, 46), "ROOF": (54, 92, 84), "GOLD": (212, 168, 70),
               "DARK": (60, 40, 36)}


def temple(t):
    m = M()
    W, D = 52, 34
    m.box([0, 4, 0], [W * 2 + 20, 8, D * 2 + 20], "STONE")
    for k in range(3):
        m.box([0, 8 - k * 2.6 - 1.3, -D - 10 - 4 - k * 4], [30, 2.6, 4], "STONE", tone=0.95)
    m.box([0, 8 + 17, 0], [W * 2 - 8, 34, D * 2 - 8], "WALL", tone=0.92)
    for x in np.linspace(-W, W, 7):
        for z in (-D, D):
            m.cyl([x, 8, z], [x, 42, z], 2.4, 2.2, "RED", 8)
    for z in np.linspace(-D, D, 5):
        for x in (-W, W):
            m.cyl([x, 8, z], [x, 42, z], 2.4, 2.2, "RED", 8)
    # lattice screens and a lintel
    for x in np.linspace(-W + 9, W - 9, 6):
        m.box([x, 24, -D + 3.6], [11, 24, 0.6], "WOOD", tone=0.8)
    m.box([0, 43, 0], [W * 2 + 6, 3, D * 2 + 6], "RED", tone=0.85)
    m.hip(-W, -D, W, D, 45, 22, "ROOF", over=12, flare=7)
    m.hip(-W * 0.55, -D * 0.5, W * 0.55, D * 0.5, 64, 12, "ROOF", over=8, flare=4, tone=1.05)
    for sx in (-1, 1):
        m.ell([sx * W * 0.5, 77, 0], [3, 4, 2], "GOLD", 6, 5)
    # two lanterns
    for sx in (-1, 1):
        m.cyl([sx * 20, 0, -D - 30], [sx * 20, 10, -D - 30], 1.5, 1.5, "STONE", 6)
        m.box([sx * 20, 13, -D - 30], [6, 6, 6], "YELLOW")
        m.pyramid([sx * 20, 16, -D - 30], 9, 9, 5, "STONE")
    return m, {"STONE": (184, 176, 160), "WALL": (226, 210, 180), "RED": (180, 48, 40), "ROOF": (64, 78, 92),
               "GOLD": (220, 176, 70), "WOOD": (120, 70, 50), "YELLOW": (250, 210, 120)}


def tarpit(t):
    m = M()
    pool = [np.array([np.cos(a) * 52 * (1 + 0.12 * np.sin(3 * a)), 1.0, np.sin(a) * 34 * (1 + 0.1 * np.cos(2 * a))])
            for a in np.linspace(0, 2 * np.pi, 25)]
    for p, q in zip(pool, pool[1:]):
        m.part([[[0, 1.2, 0], p, q]], "DARK", 1.0)
        m.part([[p, q, q * 1.18 + [0, -1, 0]], [p, q * 1.18 + [0, -1, 0], p * 1.18 + [0, -1, 0]]], "EARTH", 0.85)
    # bubbles and glints, rocks and a mammoth's tusks rising out of the tar
    for x, z, r in ((-12, 6, 3.5), (8, -10, 2.6), (22, 8, 2.0), (-28, -6, 1.8)):
        m.ell([x, 1.5, z], [r, r * 0.6, r], "DARK", 8, 4, 1.4)
    for sx in (-1, 1):
        pts = [np.array([10 + sx * 6 + 10 * np.sin(a) * sx * 0.4, 1 + 18 * np.sin(a), -4 - 12 * np.cos(a)])
               for a in np.linspace(0, 2.4, 7)]
        for p, q in zip(pts, pts[1:]):
            m.cyl(p, q, 1.8, 1.4, "WHITE", 6)
    for c, r in (([-50, 4, 20], [9, 7, 8]), ([46, 3, -24], [7, 5, 6]), ([56, 3, 14], [5, 4, 5])):
        m.ell(c, r, "STONE", 10, 6)
    for c in ([-40, 2, -30], [30, 2, 34], [-58, 2, -4]):
        m.leaves(c, 5)
    return m, {"DARK": (34, 32, 36), "EARTH": (118, 92, 62), "WHITE": (232, 222, 196), "STONE": (138, 130, 118),
               "LEAF": (110, 120, 60)}


def water_tower(t):
    m = M()
    H = 70
    for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
        m.cyl([sx * 20, 0, sz * 20], [sx * 13, H, sz * 13], 1.8, 1.4, "STONE", 6)
    for y in (22, 46):
        f = y / H
        w = 20 - 7 * f
        for (ax, az), (bx, bz) in (((-1, -1), (1, -1)), ((1, -1), (1, 1)), ((1, 1), (-1, 1)), ((-1, 1), (-1, -1))):
            m.cyl([ax * w, y, az * w], [bx * w, y, bz * w], 0.8, 0.8, "STONE", 4, 0.9)
    m.cyl([0, H, 0], [0, H + 30, 0], 22, 22, "WALL", 16)
    for y in (H + 8, H + 20):
        m.cyl([0, y, 0], [0, y + 1.2, 0], 22.4, 22.4, "WALL", 16, 0.8)
    m.cyl([0, H + 30, 0], [0, H + 44, 0], 23.5, 1.5, "ROOF", 16)
    m.ell([0, H + 45, 0], [2, 2, 2], "ROOF", 6, 4)
    m.cyl([0, H - 2, 0], [0, H, 0], 24, 24, "DARK", 16)
    m.cyl([0, 0, 0], [0, H, 0], 2.4, 2.4, "STONE", 6, 0.8)
    m.cyl([16, 0, -14], [16, H, -14], 0.6, 0.6, "DARK", 3)
    return m, {"STONE": (150, 150, 152), "WALL": (226, 220, 206), "ROOF": (170, 70, 56), "DARK": (70, 72, 78)}


def radio_tower(t):
    m = M()
    H = 160
    m.box([0, 1.5, 0], [36, 3, 36], "STONE")
    # red and white sections
    n = 8
    for k in range(n):
        y0, y1 = H * k / n, H * (k + 1) / n
        w0, w1 = 28 - 22 * k / n, 28 - 22 * (k + 1) / n
        m.lattice(w0, w1, y1 - y0, "RED" if k % 2 else "WHITE", levels=2, r=0.9, y0=y0)
    m.cyl([0, H, 0], [0, H + 16, 0], 0.8, 0.4, "WHITE", 4)
    m.part(ellipsoid([0, H + 17, 0], [2.4, 2.4, 2.4], 8, 5), "RED", 1.3, glow=True)
    for y in (H * 0.5, H * 0.75):
        m.part(ellipsoid([0, y, 0], [1.8, 1.8, 1.8], 8, 5), "RED", 1.3, glow=True)
    # a small equipment hut
    m.box([30, 8, 16], [20, 16, 16], "WALL")
    m.box([30, 16.8, 16], [22, 1.6, 18], "DARK")
    return m, {"RED": (200, 56, 48), "WHITE": (236, 236, 232), "STONE": (150, 148, 142), "WALL": (196, 190, 176),
               "DARK": (70, 72, 78)}


def oil_pump(t):
    """A pumpjack: the walking beam rocks on its post, the crank and counterweights turn."""
    m = M()
    ph = 2 * np.pi * t
    m.box([0, 2, 0], [86, 4, 24], "STONE", tone=0.9)
    # the samson post (an A frame)
    pivot = np.array([0, 44, 0])
    for sx in (-1, 1):
        for sz in (-1, 1):
            m.cyl([sx * 12, 4, sz * 8], pivot + [0, 0, sz * 3], 1.6, 1.4, "DARK", 5)
    ang = 14 * np.sin(ph)
    R = rz(ang)
    def B(p):
        return np.asarray(p, float) @ R.T + pivot
    # the beam and the horse head
    m.part(box([0, 0, 0], [72, 4, 4]) @ R.T + pivot + [0, 2.5, 0], "RED")
    head = [B([-36, 6, -3]), B([-36, -16, -3]), B([-42, -12, -3]), B([-44, 2, -3])]
    head2 = [p + [0, 0, 6] for p in head]
    m.quad(*head, "RED", 0.95)
    m.quad(*head2, "RED", 0.95)
    for i in range(4):
        m.quad(head[i], head[(i + 1) % 4], head2[(i + 1) % 4], head2[i], "RED", 0.85)
    # the polished rod down to the wellhead
    bridle = B([-43, -14, 0])
    m.cyl(bridle, [-43 * np.cos(np.radians(ang)), 10, 0], 0.5, 0.5, "WHITE", 3)
    m.cyl([-40, 4, 0], [-40, 12, 0], 3, 3, "DARK", 8)
    m.box([-40, 13, 0], [8, 2, 8], "DARK")
    # the crank with its counterweights, the gearbox, the pitman arms to the beam's tail
    crank = np.array([30, 18, 0])
    m.box([30, 10, 0], [16, 12, 12], "DARK", tone=0.9)
    for sz in (-8, 8):
        arm_end = crank + np.array([np.cos(ph) * 12, np.sin(ph) * 12, sz])
        m.cyl(crank + [0, 0, sz], arm_end, 2.2, 2.2, "RED", 5)
        m.part(box([0, 0, 0], [10, 14, 2.5]) @ rz(np.degrees(ph) + 90).T + crank + [np.cos(ph) * 14, np.sin(ph) * 14, sz], "RED", 0.9)
        m.cyl(arm_end, B([36, 0, sz * 0.5]), 0.9, 0.9, "DARK", 4)
    m.cyl(pivot - [0, 0, 5], pivot + [0, 0, 5], 2, 2, "DARK", 6)
    return m, {"STONE": (150, 146, 138), "DARK": (60, 62, 68), "RED": (196, 60, 46), "WHITE": (220, 220, 220)}


def chapel(t):
    m = M()
    W, D, H = 30, 20, 32
    m.box([0, H / 2, 0], [W * 2, H, D * 2], "WALL")
    m.gable(-W, -D, W, D, H, 22, "ROOF", over=3)
    # the steeple at the front
    m.box([-W - 6, H / 2 + 8, 0], [18, H + 16, 18], "WALL", tone=1.02)
    m.box([-W - 6, H + 22, 0], [14, 12, 14], "WHITE")
    for zz in (-7.2, 7.2):
        m.box([-W - 6, H + 22, zz], [6, 8, 0.6], "DARK", tone=0.6)
    m.pyramid([-W - 6, H + 28, 0], 16, 16, 30, "ROOF", 1.0)
    m.cyl([-W - 6, H + 58, 0], [-W - 6, H + 68, 0], 0.6, 0.6, "GOLD", 4)
    m.box([-W - 6, H + 65, 0], [0.8, 0.8, 6], "GOLD")
    m.box([-W - 15.3, 10, 0], [0.6, 20, 10], "WOOD", tone=0.8)
    # arched windows along the sides
    for x in np.linspace(-W + 10, W - 10, 4):
        for z in (-D - 0.3, D + 0.3):
            m.box([x, 18, z], [6, 13, 0.6], "PURPLE")
            m.ell([x, 24.5, z], [3, 3, 0.4], "PURPLE", 8, 4)
    return m, {"WALL": (240, 236, 226), "ROOF": (84, 88, 100), "WHITE": (244, 244, 240), "DARK": (60, 60, 70),
               "GOLD": (220, 180, 80), "WOOD": (120, 80, 52), "PURPLE": (90, 110, 170)}


def ruins(t):
    m = M()
    # the broken walls of a nave with pointed window openings, a gable end standing, fallen blocks and grass
    m.ell([0, -5, 0], [70, 8, 46], "LEAF", 14, 6, 0.9)
    def wall(x0, z0, x1, z1, h, tone=1.0):
        d = np.array([x1 - x0, 0, z1 - z0])
        L = np.linalg.norm(d)
        n = int(L / 10)
        for k in range(n):
            a = np.array([x0, 0, z0]) + d * (k / n)
            b = np.array([x0, 0, z0]) + d * ((k + 1) / n)
            hh = h * (0.45 + 0.55 * abs(np.sin(k * 1.7 + x0 * 0.1)))
            c = (a + b) / 2
            if k % 2 == 1 and hh > h * 0.6:
                # a window opening: two jambs and a pointed head
                m.box(c + [0, hh * 0.2, 0], [abs(d[0]) / n + 4 if d[0] else 6, hh * 0.4, abs(d[2]) / n + 4 if d[2] else 6], "STONE", tone=tone)
                m.box(c + [0, hh * 0.9, 0], [abs(d[0]) / n + 4 if d[0] else 6, hh * 0.2, abs(d[2]) / n + 4 if d[2] else 6], "STONE", tone=tone)
            else:
                m.box(c + [0, hh / 2, 0], [abs(d[0]) / n + 4 if d[0] else 6, hh, abs(d[2]) / n + 4 if d[2] else 6], "STONE", tone=tone)
    wall(-50, -22, 40, -22, 46)
    wall(-50, 22, 20, 22, 36, 0.95)
    wall(-50, -22, -50, 22, 30, 1.02)
    # the gable end with a big window
    m.box([44, 30, -14], [7, 60, 14], "STONE", tone=1.05)
    m.box([44, 30, 14], [7, 60, 14], "STONE", tone=1.05)
    m.part([[[44, 60, -21], [44, 60, 21], [44, 84, 0]]], "STONE", 1.0)
    m.part([[[47.5, 60, -21], [47.5, 60, 21], [47.5, 84, 0]]], "STONE", 1.0)
    m.box([44, 4, 0], [7, 8, 14], "STONE")
    for c, s in (([10, 3, 0], [12, 6, 9]), ([-20, 3, 8], [9, 6, 8]), ([28, 2, 32], [10, 4, 7])):
        m.box(c, s, "STONE", rot=20, tone=0.92)
    for c in ([-30, 4, 0], [0, 4, -8], [20, 4, 10], [-46, 10, 24], [36, 30, -22]):
        m.leaves(c, 6)
    return m, {"STONE": (164, 156, 140), "LEAF": (92, 136, 62)}


def gargoyle(t):
    m = M()
    y = plinth(m, 30, 22, "STONE", steps=2)
    # a crouching beast: haunches, body leaning forward, head with horns, wings folded up behind
    m.ell([0, y + 8, 4], [10, 8, 9], "STONE", 12, 8, 0.95)
    m.ell([0, y + 18, -2], [8, 11, 7], "STONE", 12, 8)
    m.ell([0, y + 28, -8], [6, 6, 6.5], "STONE", 12, 8, 1.04)
    m.ell([0, y + 26, -14], [3.5, 2.5, 3], "STONE", 8, 5, 1.0)
    for sx in (-1, 1):
        m.cyl([sx * 4, y + 32, -8], [sx * 7, y + 40, -4], 1.4, 0.3, "STONE", 5, 1.1)
        m.part([[[sx * 4, y + 22, 2], [sx * 18, y + 40, 8], [sx * 12, y + 14, 8]],
                [[sx * 4, y + 22, 2], [sx * 12, y + 14, 8], [sx * 5, y + 12, 5]]], "STONE", 0.86)
        m.cyl([sx * 7, y + 16, -6], [sx * 6, y + 4, -9], 2, 1.6, "STONE", 5)
        m.ell([sx * 2.6, y + 29, -14], [1.0, 1.0, 0.6], "RED", 5, 4, 1.2)
    return m, {"STONE": (128, 126, 122), "RED": (200, 60, 50)}


def gold(t):
    m = M()
    for k, (x, z, s) in enumerate(((0, 0, 1.0), (-7, 5, 0.8), (6, 6, 0.7), (2, -6, 0.75), (-4, -3, 0.6))):
        m.ell([x, 3 * s, z], [5 * s, 3.5 * s, 4.4 * s], "GOLD", 8, 6, 1.0 + 0.04 * k)
    m.box([0, 7.5, 1], [9, 3, 4], "GOLD", rot=20, tone=1.15)
    return m, {"GOLD": (226, 180, 60)}


def hanging_gardens(t):
    m = M()
    w, d, y = 200.0, 140.0, 0.0
    for k in range(5):
        h = 22
        m.box([0, y + h / 2, 0], [w, h, d], "STONE", tone=1.0 - 0.02 * k)
        # arcades along the front and side
        for x in np.linspace(-w / 2 + 10, w / 2 - 10, int(w / 20)):
            m.box([x, y + h * 0.42, -d / 2 - 0.3], [9, h * 0.6, 0.6], "DARK", tone=0.6)
        for z in np.linspace(-d / 2 + 10, d / 2 - 10, int(d / 20)):
            m.box([-w / 2 - 0.3, y + h * 0.42, z], [0.6, h * 0.6, 9], "DARK", tone=0.6)
        y += h
        # planting along the terrace's edge, hanging down
        for x in np.linspace(-w / 2 + 6, w / 2 - 6, int(w / 12)):
            m.leaves([x, y + 3, -d / 2 + 5], 6.5)
            m.part(quad([x - 3, y, -d / 2 - 0.6], [x + 3, y, -d / 2 - 0.6], [x + 2, y - 12, -d / 2 - 0.6], [x - 2, y - 12, -d / 2 - 0.6]),
                   "LEAF", 0.85)
        for z in np.linspace(-d / 2 + 6, d / 2 - 6, int(d / 12)):
            m.leaves([-w / 2 + 5, y + 3, z], 6.5)
        if k % 2 == 0:
            for x in np.linspace(-w / 2 + 20, w / 2 - 20, 3):
                m.cyl([x, y, 0], [x, y + 12, 0], 1.2, 0.8, "WOOD", 4)
                m.leaves([x, y + 16, 0], 9)
        w *= 0.78
        d *= 0.78
    m.box([0, y + 9, 0], [w * 0.6, 18, d * 0.6], "WALL")
    m.pyramid([0, y + 18, 0], w * 0.7, d * 0.7, 10, "GOLD")
    return m, {"STONE": (206, 176, 128), "DARK": (90, 66, 48), "LEAF": (78, 140, 60), "WOOD": (120, 90, 60),
               "WALL": (226, 206, 170), "GOLD": (210, 168, 70)}


def pyramid(t):
    m = M()
    S, H = 200.0, 130.0
    # courses of stone: each a slightly smaller block, so the steps catch the light
    n = 16
    for k in range(n):
        f0, f1 = k / n, (k + 1) / n
        w0 = S * (1 - f0)
        m.box([0, H * (f0 + f1) / 2, 0], [w0, H / n, w0], "STONE", tone=1.0 - 0.04 * (k % 2))
    m.pyramid([0, H, 0], S / n, S / n, H / n * 1.2, "GOLD", 1.1)
    # an entrance on one face
    m.box([0, H * 0.18, -S * 0.41], [12, 14, 2], "DARK", tone=0.5)
    return m, {"STONE": (216, 182, 120), "GOLD": (236, 196, 90), "DARK": (70, 54, 40)}


def volcano(t):
    m = M()
    # a cone of rock with gullies, a crater with a lava lake, lava flows down one side and green lower slopes
    R, H = 170.0, 110.0
    n = 36
    rings = []
    for j, f in enumerate(np.linspace(0, 1, 9)):
        r = R * (1 - f) ** 1.1 + 22 * f
        y = H * (1 - (1 - f) ** 1.6)
        ring = []
        for k in range(n):
            a = 2 * np.pi * k / n
            wob = 1 + 0.07 * np.sin(5 * a + j) + 0.05 * np.sin(11 * a)
            ring.append(np.array([np.cos(a) * r * wob, y, np.sin(a) * r * wob * 0.85]))
        rings.append((ring, f))
    for (r0, f0), (r1, f1) in zip(rings, rings[1:]):
        for k in range(n):
            q = (k + 1) % n
            tag = "LEAF" if f1 <= 0.25 else "EARTH"
            tone = 1.0 if k % 2 else 0.92
            m.part([[r0[k], r0[q], r1[q]], [r0[k], r1[q], r1[k]]], tag, tone)
    top = rings[-1][0]
    lava_c = np.array([0, H - 8, 0])
    for k in range(n):
        q = (k + 1) % n
        inner_k = top[k] * [0.75, 1, 0.75] + [0, -8, 0]
        inner_q = top[q] * [0.75, 1, 0.75] + [0, -8, 0]
        m.part([[top[k], top[q], inner_q], [top[k], inner_q, inner_k]], "DARK", 0.9)
        m.part([[inner_k, inner_q, lava_c]], "RED", 1.0, glow=True)
    # lava flows: glowing ribbons down the slope
    for a0, L in ((0.3, 0.75), (0.9, 0.55), (-0.5, 0.4)):
        pts = []
        for f in np.linspace(0, L, 10):
            r = 22 + (R - 22) * f
            y = H * (1 - f ** 0.9) - 2
            a = a0 + 0.2 * np.sin(f * 6)
            pts.append(np.array([np.cos(a) * r, y + 1.2, np.sin(a) * r * 0.85]))
        for i in range(len(pts) - 1):
            w = 5 * (1 - i / len(pts)) + 1.5
            d = pts[i + 1] - pts[i]
            side = np.cross(d, [0, 1, 0])
            side = side / np.linalg.norm(side) * w
            m.quad(pts[i] - side, pts[i] + side, pts[i + 1] + side * 0.85, pts[i + 1] - side * 0.85,
                   "RED" if i < 6 else "YELLOW", 1.0, glow=i < 6)
    # smoke
    for k, (c, r) in enumerate((([4, H + 14, 0], 12), ([10, H + 30, -4], 15), ([20, H + 48, -6], 18))):
        m.s.add_ball(np.array(c, float), r, np.array([0.8, 0.8, 0.8]), ID["WHITE"])
    for c in ([-150, 6, 40], [140, 6, 30], [-60, 6, 120], [80, 6, -120]):
        m.leaves(c, 14)
    return m, {"LEAF": (84, 134, 60), "EARTH": (110, 90, 76), "DARK": (54, 48, 46), "RED": (240, 96, 40),
               "YELLOW": (250, 180, 60), "WHITE": (216, 214, 210)}


def tracks(t):
    m = M()
    L = 80
    m.box([0, 1, 0], [L, 2, 30], "STONE", tone=0.9)
    for x in np.linspace(-L / 2 + 3, L / 2 - 3, 11):
        m.box([x, 3, 0], [4.5, 2, 26], "WOOD", tone=0.95 + 0.05 * np.sin(x))
    for z in (-8, 8):
        m.box([0, 5, z], [L, 2.2, 1.6], "DARK", tone=1.2)
    return m, {"STONE": (140, 132, 120), "WOOD": (110, 80, 56), "DARK": (100, 100, 108)}


def rock_garden(t):
    m = M()
    m.box([0, 1, 0], [70, 2, 46], "WOOD", tone=0.9)
    m.box([0, 2.2, 0], [66, 0.6, 42], "WHITE")
    # raked lines round the stones
    for z in np.linspace(-18, 18, 10):
        m.box([0, 2.6, z], [64, 0.2, 0.6], "WHITE", tone=0.86)
    for c, r in (([-14, 4, -4], [8, 7, 6]), ([12, 3, 8], [6, 5, 5]), ([20, 2.5, -10], [4, 3.5, 4])):
        m.ell(c, r, "STONE", 10, 7)
        m.leaves([c[0] + 3, 3, c[2] + r[2]], 2.4)
    return m, {"WOOD": (120, 90, 64), "WHITE": (236, 232, 220), "STONE": (120, 118, 112), "LEAF": (88, 130, 60)}


def ground_base(kind):
    def build(t):
        m = M()
        m.s.no_shadow = True
        if kind == "dirt":
            m.ell([0, -3, 0], [44, 3.6, 30], "EARTH", 16, 4)
        else:
            m.box([0, 0.5, 0], [70, 1, 54], "STONE")
            for x in np.linspace(-30, 30, 7):
                m.box([x, 1.05, 0], [0.6, 0.1, 54], "STONE", tone=0.85)
        return m, {"EARTH": (150, 120, 84), "STONE": (184, 178, 164)}
    return build


# file name (lower case, no folder or extension) -> model
LANDMARKS = {
    "sundial": lambda t: sundial(t), "sundialb": lambda t: sundial(t, wide=True), "barn": barn, "civilwarcannon": cannon,
    "stone two": stones, "wmilla": farm_windmill, "parklandrock": boulders, "civilwarstatue": soldier,
    "lighthousec": lambda t: lighthouse(t), "lighthouseb": lambda t: lighthouse(t, tall=True), "buddha": buddha,
    "windmill": dutch_windmill, "equestrian": horse_rider, "easter": moai, "pagoda": pagoda, "chinesehouse": temple,
    "tarpit": tarpit, "wtow2": water_tower, "radio tower": radio_tower, "red oil pump": oil_pump, "chapel": chapel,
    "cathedral_ruins": ruins, "gargoyle": gargoyle, "gold": gold, "hgardens": hanging_gardens, "pyramid": pyramid,
    "volcano": volcano, "railroad tracks": tracks, "rock garden": rock_garden,
    "chapel_dirt": ground_base("dirt"), "chinesehousebase": ground_base("paving"), "lighthousec_base": ground_base("dirt"),
}


def model(name, t):
    """(scene, palette colours) of a landmark at animation phase t (0..1), or None."""
    b = LANDMARKS.get(name)
    if b is None:
        return None
    m, pal = b(t)
    m.s.pal = pal
    return m.s
