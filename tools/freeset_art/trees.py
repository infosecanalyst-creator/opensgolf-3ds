"""Trees, palms, cacti and bushes for the free set, grown from a few parameters per species: a trunk and branches of
tapered tubes, and a crown of leafy balls (or palm fronds, cactus arms, conifer tiers). Every tree of a species differs a
little (the seed is the file name). Sizes are in world units before the sprite is fitted to the original's frame."""
import numpy as np

from mats import ID
from scene import Scene, ellipsoid, tube, quad

LEAF, BARKT, RED, WOOD, YELLOW, WHITE, PURPLE = (ID[k] for k in ("LEAF", "BARK", "RED", "WOOD", "YELLOW", "WHITE", "PURPLE"))


def c255(*rgb):
    return np.array(rgb, float) / 255.0


def jitter(col, rng, k=0.08):
    return np.clip(col * (1 + rng.uniform(-k, k)) + rng.uniform(-k, k, 3) * 0.3, 0, 1)


BARK = c255(104, 78, 56)
BARK_GREY = c255(128, 118, 104)
BARK_PALM = c255(150, 120, 86)


def growth(f, n):
    """Size and leafiness of frame f of n: the original's 16 frames go from a seedling to the full tree."""
    if n <= 1:
        return 1.0, 1.0
    t = f / (n - 1)
    size = 0.22 + 0.78 * (1 - (1 - t) ** 1.6)
    leaf = np.clip((t - 0.08) / 0.55, 0, 1)
    return size, leaf


def broadleaf(rng, height=100.0, crown=0.5, width=0.36, squash=1.05, leaf=c255(78, 130, 52), n=26, bark=BARK,
              leafiness=1.0, ball=0.34, flowers=None, flower_share=0.0, droop=0.0, lean=0.0):
    """A deciduous tree: trunk to `crown` of the height, a crown `width` of the height across (radius), made of n balls."""
    s = Scene()
    s.leafy, s.rag = 0.32, 0.17
    # fewer, larger clumps than asked for: a crown of a few clear lobes reads as a tree at the game's size, a crown of many
    # small balls as a heap of bubbles
    n = max(8, int(round(n * 0.55)))
    ball = ball * 1.3
    top = np.array([lean * height, height * crown, 0.0])
    # a chunky, slightly cartoony tree: a stout trunk flaring at the foot and a big round crown
    s.add_tris(tube([0, 0, 0], top * 0.92, height * 0.068, height * 0.04, 8), bark, BARKT)
    s.add_tris(tube([0, 0, 0], [0, height * 0.06, 0], height * 0.085, height * 0.066, 8), bark, BARKT)
    cr = height * width * 1.12
    cy = height * crown + cr * squash * 0.6
    centre = np.array([lean * height, cy, 0.0])
    s.mass = (centre, cr * 1.05)
    # a few branches reaching into the crown
    for k in range(5):
        a = rng.uniform(0, 2 * np.pi)
        end = centre + np.array([np.cos(a) * cr * 0.6, rng.uniform(-0.2, 0.4) * cr * squash, np.sin(a) * cr * 0.6])
        start = top * rng.uniform(0.7, 0.95)
        s.add_tris(tube(start, end, height * 0.022, height * 0.01, 5), bark * 0.95, BARKT)
    if leafiness <= 0.02:
        return s
    pts = []
    for k in range(n):
        # points spread through the crown, more of them near its surface
        d = rng.normal(size=3)
        d /= np.linalg.norm(d)
        rr = rng.uniform(0.3, 0.75) ** 0.5
        p = centre + d * rr * cr * np.array([1, squash, 1])
        p[1] -= droop * cr * (np.hypot(d[0], d[2]) ** 2)
        pts.append(p)
    pts.append(centre)
    for k, p in enumerate(pts):
        r = cr * ball * rng.uniform(0.8, 1.2) * (1.6 if k == len(pts) - 1 else 1.0) * (0.35 + 0.65 * leafiness)
        s.add_ball(p, r, jitter(leaf, rng, 0.03), LEAF)
    if flowers is not None and leafiness > 0.3:
        # blossom: many small clusters scattered over the crown's surface, the leaves showing between them
        for k in range(int(n * flower_share * 4)):
            d = rng.normal(size=3)
            d[1] = abs(d[1]) * 0.8 + 0.1
            d /= np.linalg.norm(d)
            p = centre + d * cr * rng.uniform(0.82, 0.98) * np.array([1, squash, 1])
            s.add_ball(p, cr * ball * rng.uniform(0.32, 0.46) * leafiness, jitter(flowers, rng, 0.05), RED)
    return s


def conifer(rng, height=110.0, width=0.24, tiers=6, leaf=c255(46, 92, 54), bark=BARK, leafiness=1.0, top_round=False,
            bare=0.12):
    """A pine, spruce or fir: tiers of balls round the trunk, narrowing upwards."""
    s = Scene()
    s.leafy, s.rag = 0.42, 0.26
    s.add_tris(tube([0, 0, 0], [0, height * 0.95, 0], height * 0.03, height * 0.008, 6), bark, BARKT)
    # the crown's mass: about as wide as the lower tiers (a wider one leaves the whole tree in its own shade)
    s.mass = (np.array([0, height * 0.42, 0]), height * max(width * 1.2, 0.3))
    if leafiness <= 0.02:
        return s
    for t in range(tiers):
        f = t / max(tiers - 1, 1)
        y = height * (bare + (0.9 - bare) * f)
        rad = height * width * (1 - f * 0.82) * (0.4 + 0.6 * leafiness)
        n = max(3, int(round(7 * (1 - f * 0.6))))
        a0 = rng.uniform(0, 2 * np.pi)
        for k in range(n):
            a = a0 + 2 * np.pi * k / n + rng.uniform(-0.2, 0.2)
            p = np.array([np.cos(a) * rad * 0.62, y - rad * 0.15, np.sin(a) * rad * 0.62])
            s.add_ball(p, rad * 0.5 * rng.uniform(0.85, 1.1), jitter(leaf, rng, 0.06), LEAF)
        s.add_ball([0, y, 0], rad * 0.62, jitter(leaf, rng, 0.05), LEAF)
    s.add_ball([0, height * 0.92, 0], height * width * 0.26 * (0.4 + 0.6 * leafiness), leaf * (1.0 if top_round else 1.05), LEAF)
    return s


def scots_pine(rng, height=110.0, leaf=c255(52, 96, 58), leafiness=1.0):
    """A tall bare trunk with a flat, cloudy crown of a few clumps (Scots pine, Japanese black pine)."""
    s = Scene()
    s.leafy, s.rag = 0.4, 0.25
    bark = c255(150, 96, 70)
    lean = rng.uniform(-0.12, 0.12)
    top = np.array([lean * height, height * 0.78, 0.0])
    s.add_tris(tube([0, 0, 0], top, height * 0.03, height * 0.018, 6), bark, BARKT)
    s.mass = (top + [0, height * 0.05, 0], height * 0.35)
    for k in range(6):
        a = rng.uniform(0, 2 * np.pi)
        end = top + np.array([np.cos(a) * height * 0.24, rng.uniform(-0.08, 0.14) * height, np.sin(a) * height * 0.24])
        s.add_tris(tube(top * rng.uniform(0.75, 1.0), end, height * 0.012, height * 0.006, 5), bark, BARKT)
        if leafiness > 0.02:
            for j in range(3):
                p = end + rng.normal(size=3) * height * 0.04 * np.array([1, 0.4, 1])
                s.add_ball(p, height * 0.085 * (0.4 + 0.6 * leafiness) * rng.uniform(0.85, 1.15), jitter(leaf, rng, 0.06), LEAF)
    if leafiness > 0.02:
        s.add_ball(top + [0, height * 0.06, 0], height * 0.11 * (0.4 + 0.6 * leafiness), leaf, LEAF)
    return s


def palm(rng, height=120.0, fronds=9, frond_len=0.42, curve=0.18, leaf=c255(70, 136, 56), leafiness=1.0, nuts=True):
    """A palm: a curved ringed trunk swelling at the foot, and a full head of arching fronds (a drooping lower ring and a
    shorter rising upper one) with feathered leaflets, lit darker underneath."""
    s = Scene()
    s.leafy, s.rag = 0.2, 0.15
    a = rng.uniform(0, 2 * np.pi)
    lean = np.array([np.cos(a), 0, np.sin(a)]) * curve * height
    segs = 10
    prev = np.zeros(3)
    for k in range(1, segs + 1):
        t = k / segs
        p = np.array([0, height * t, 0]) + lean * t * t
        f0, f1 = (k - 1) / segs, k / segs
        r0 = height * (0.03 - 0.009 * f0 + 0.022 * max(0, 0.15 - f0) / 0.15)
        r1 = height * (0.03 - 0.009 * f1 + 0.022 * max(0, 0.15 - f1) / 0.15)
        s.add_tris(tube(prev, p, r0, r1, 8, caps=False), BARK_PALM * (0.88 if k % 2 else 1.06), BARKT)
        prev = p
    crown = prev
    s.mass = (crown, height * frond_len)
    if leafiness <= 0.02:
        return s
    # the boss of old leaf bases under the crown
    s.add_ball(crown - [0, height * 0.02, 0], height * 0.045, BARK_PALM * 0.8, BARKT)
    L0 = height * frond_len * (0.35 + 0.65 * leafiness)
    a0 = rng.uniform(0, 2 * np.pi)
    rings = [(fronds, 1.0, (0.1, 0.45)), (max(4, fronds // 2 + 1), 0.7, (0.55, 0.95))]
    for ri, (count, lscale, rises) in enumerate(rings):
        for k in range(count):
            ang = a0 + 2 * np.pi * (k + 0.5 * ri) / count + rng.uniform(-0.2, 0.2)
            d = np.array([np.cos(ang), 0, np.sin(ang)])
            side = np.array([-d[2], 0, d[0]])
            rise = rng.uniform(*rises)
            L = L0 * lscale * rng.uniform(0.9, 1.1)
            pts = []
            for j in range(11):
                t = j / 10
                pts.append(crown + d * L * t + np.array([0, L * (rise * t - (0.5 + rise * 0.55) * t * t), 0]))
            col = jitter(leaf, rng, 0.06) * (1.0 if ri else 0.92)
            for j in range(10):
                p0, p1 = pts[j], pts[j + 1]
                w0 = L * 0.24 * np.sin(np.pi * (j + 0.4) / 10.8) ** 0.8
                w1 = L * 0.24 * np.sin(np.pi * (j + 1.4) / 10.8) ** 0.8
                # leaflets hang from the rib, feathered: the outer points swept back and alternately short
                down = np.array([0, -L * 0.07, 0])
                back = -(p1 - p0) * 0.35
                j0 = 1.0 if j % 2 == 0 else 0.78
                s.add_tris(quad(p0, p1, p1 + side * w1 + down + back, p0 + side * w0 * j0 + down + back), col, LEAF)
                s.add_tris(quad(p0, p0 - side * w0 * j0 + down + back, p1 - side * w1 + down + back, p1), col * 0.88, LEAF)
    if nuts:
        for k in range(4):
            ang = rng.uniform(0, 2 * np.pi)
            s.add_ball(crown + np.array([np.cos(ang) * height * 0.03, -height * 0.035, np.sin(ang) * height * 0.03]),
                       height * 0.024, c255(120, 92, 48), WOOD)
    return s


def saguaro(rng, height=90.0, arms=2, col=c255(86, 128, 70), leafiness=1.0):
    """A columnar cactus with arms, ribbed."""
    s = Scene()
    s.leafy, s.rag = 0.0, 0.0
    r = height * 0.075 * (0.6 + 0.4 * leafiness)

    def column(a, b, rad):
        t = tube(a, b, rad, rad, 12, caps=False)
        cols = np.array([col * (1.08 if (i // 2) % 2 == 0 else 0.86) for i in range(len(t))])
        s.add_tris(t, cols, LEAF)
        s.add_ball(b, rad, col * 1.02, LEAF)

    column([0, 0, 0], [0, height * 0.92, 0], r)
    if leafiness > 0.4:
        for k in range(arms):
            a = rng.uniform(0, 2 * np.pi) if k == 0 else a + np.pi + rng.uniform(-0.6, 0.6)
            d = np.array([np.cos(a), 0, np.sin(a)])
            y0 = height * rng.uniform(0.32, 0.55)
            elbow = np.array([0, y0, 0]) + d * height * 0.2
            column(np.array([0, y0, 0]), elbow, r * 0.75)
            column(elbow, elbow + np.array([0, height * rng.uniform(0.2, 0.32), 0]), r * 0.75)
    s.mass = None
    return s


def barrel_cactus(rng, height=40.0, col=c255(92, 134, 74), n=3, leafiness=1.0, flower=c255(236, 200, 70)):
    """A clump of round ribbed cacti with a flower or two."""
    s = Scene()
    s.leafy, s.rag = 0.15, 0.05
    for k in range(n):
        a = rng.uniform(0, 2 * np.pi)
        off = np.array([np.cos(a), 0, np.sin(a)]) * height * (0.0 if k == 0 else 0.45)
        rad = height * (0.4 if k == 0 else 0.28) * (0.5 + 0.5 * leafiness)
        s.add_ball(off + [0, rad * 0.9, 0], rad, jitter(col, rng, 0.06), LEAF)
        if leafiness > 0.8 and k < 2:
            s.add_ball(off + [0, rad * 1.85, 0], rad * 0.2, flower, YELLOW)
    return s


def prickly_pear(rng, height=45.0, col=c255(104, 146, 82), leafiness=1.0):
    """Flat oval pads growing out of each other."""
    s = Scene()

    def pad(c, r, a, tilt):
        d = np.array([np.cos(a), 0, np.sin(a)])
        up = np.array([0, 1, 0]) * np.cos(tilt) + d * np.sin(tilt)
        n = 12
        ang = np.linspace(0, 2 * np.pi, n + 1)
        side = np.cross(up, d)
        ring = [c + side * np.cos(t) * r * 0.75 + up * np.sin(t) * r for t in ang]
        tris = [[c + d * r * 0.08, ring[i], ring[i + 1]] for i in range(n)]
        tris += [[c - d * r * 0.08, ring[i + 1], ring[i]] for i in range(n)]
        s.add_tris(np.array(tris), jitter(col, rng, 0.06), LEAF)
        return c + up * r * 0.9

    sc = 0.5 + 0.5 * leafiness
    base = pad(np.array([0, height * 0.3 * sc, 0]), height * 0.3 * sc, rng.uniform(0, 6.3), 0.1)
    for k in range(3 if leafiness > 0.5 else 1):
        a = rng.uniform(0, 6.3)
        pad(base + np.array([np.cos(a), 0.2, np.sin(a)]) * height * 0.15 * sc, height * 0.24 * sc, a, rng.uniform(-0.6, 0.6))
    return s


def joshua(rng, height=90.0, leaf=c255(96, 120, 70), leafiness=1.0):
    """A Joshua tree: a shaggy trunk forking into arms that end in spiky tufts."""
    s = Scene()
    s.leafy, s.rag = 0.5, 0.45
    bark = c255(120, 104, 80)
    fork = np.array([0, height * 0.45, 0])
    s.add_tris(tube([0, 0, 0], fork, height * 0.05, height * 0.04, 6), bark, BARKT)
    for k in range(4 if leafiness > 0.3 else 1):
        a = rng.uniform(0, 2 * np.pi)
        mid = fork + np.array([np.cos(a) * height * 0.16, height * 0.2, np.sin(a) * height * 0.16])
        end = mid + np.array([np.cos(a) * height * 0.1, height * rng.uniform(0.1, 0.25), np.sin(a) * height * 0.1])
        s.add_tris(tube(fork, mid, height * 0.035, height * 0.03, 5), bark, BARKT)
        s.add_tris(tube(mid, end, height * 0.03, height * 0.025, 5), bark, BARKT)
        s.add_ball(end + [0, height * 0.04, 0], height * 0.085 * (0.4 + 0.6 * leafiness), jitter(leaf, rng, 0.06), LEAF)
    return s


def dragon_tree(rng, height=90.0, leaf=c255(70, 120, 60), leafiness=1.0):
    """A dragon tree (dracaena): a thick trunk splitting into many arms under a dense umbrella crown."""
    s = Scene()
    s.leafy, s.rag = 0.45, 0.3
    bark = c255(140, 126, 106)
    fork = np.array([0, height * 0.4, 0])
    s.add_tris(tube([0, 0, 0], fork, height * 0.06, height * 0.05, 7), bark, BARKT)
    s.mass = (np.array([0, height * 0.75, 0]), height * 0.4)
    for k in range(9):
        a = 2 * np.pi * k / 9 + rng.uniform(-0.2, 0.2)
        end = np.array([np.cos(a) * height * 0.3, height * rng.uniform(0.66, 0.78), np.sin(a) * height * 0.3])
        s.add_tris(tube(fork, end, height * 0.025, height * 0.015, 5), bark, BARKT)
        if leafiness > 0.02:
            s.add_ball(end + [0, height * 0.04, 0], height * 0.13 * (0.4 + 0.6 * leafiness), jitter(leaf, rng, 0.06), LEAF)
    if leafiness > 0.02:
        s.add_ball([0, height * 0.82, 0], height * 0.18 * (0.4 + 0.6 * leafiness), leaf, LEAF)
    return s


def bush(rng, height=40.0, width=0.75, leaf=c255(70, 120, 50), n=9, flowers=None, flower_share=0.0, leafiness=1.0):
    """A rounded shrub of a few leafy balls, with flowers if given."""
    s = Scene()
    s.leafy, s.rag = 0.4, 0.22
    cr = height * width * (0.4 + 0.6 * leafiness)
    s.mass = (np.array([0, height * 0.45, 0]), cr)
    for k in range(n):
        a = rng.uniform(0, 2 * np.pi)
        rr = rng.uniform(0.1, 0.75) * cr
        p = np.array([np.cos(a) * rr, height * rng.uniform(0.3, 0.75) * (0.4 + 0.6 * leafiness), np.sin(a) * rr])
        s.add_ball(p, cr * rng.uniform(0.38, 0.52), jitter(leaf, rng, 0.07), LEAF)
    if flowers is not None and leafiness > 0.3:
        # flowers in small clusters over the outside of the shrub
        top = height * 0.5 * (0.4 + 0.6 * leafiness)
        for k in range(int(n * flower_share * 4)):
            d = rng.normal(size=3)
            d[1] = abs(d[1]) * 0.9 + 0.15
            d /= np.linalg.norm(d)
            p = np.array([0, top, 0]) + d * cr * np.array([1.0, 0.7, 1.0]) * rng.uniform(0.75, 0.95)
            s.add_ball(p, cr * rng.uniform(0.13, 0.19), jitter(flowers, rng, 0.05), RED)
    return s


def willow(rng, height=100.0, leaf=c255(110, 150, 70), leafiness=1.0):
    """A weeping willow: a crown of balls with long strands of leaves hanging down to the ground."""
    s = broadleaf(rng, height, crown=0.55, width=0.4, squash=0.7, leaf=leaf, n=20, leafiness=leafiness, droop=0.3)
    if leafiness <= 0.02:
        return s
    cr = height * 0.4 * (0.4 + 0.6 * leafiness)
    cy = height * 0.68
    for k in range(26):
        a = rng.uniform(0, 2 * np.pi)
        rr = cr * rng.uniform(0.7, 1.05)
        top = np.array([np.cos(a) * rr, cy + rng.uniform(-0.05, 0.1) * height, np.sin(a) * rr])
        bot = top * np.array([1.08, 0, 1.08]) + np.array([0, height * rng.uniform(0.08, 0.25), 0])
        side = np.array([-np.sin(a), 0, np.cos(a)]) * height * 0.03
        s.add_tris(quad(top - side, top + side, bot + side * 0.5, bot - side * 0.5), jitter(leaf * 0.95, rng, 0.08), LEAF)
    return s


def reeds(rng, height=40.0, col=c255(120, 150, 70), n=14, heads=None, leafiness=1.0):
    """Grass, reeds, cattails, bamboo: thin blades with optional heads."""
    s = Scene()
    for k in range(n):
        a = rng.uniform(0, 2 * np.pi)
        r = rng.uniform(0, height * 0.3)
        b = np.array([np.cos(a) * r, 0, np.sin(a) * r])
        tip = b + np.array([rng.uniform(-0.15, 0.15) * height, height * rng.uniform(0.6, 1.0) * (0.3 + 0.7 * leafiness),
                            rng.uniform(-0.15, 0.15) * height])
        w = height * 0.025
        side = np.array([-np.sin(a), 0, np.cos(a)]) * w
        s.add_tris([[b - side, b + side, tip]], jitter(col, rng, 0.1), LEAF)
        if heads is not None and leafiness > 0.6 and k % 2 == 0:
            s.add_tris(tube(tip - (tip - b) * 0.15, tip, height * 0.03, height * 0.03, 5), heads, WOOD)
    return s


# ---- the species of each file name -----------------------------------------------------------------------------------------

def rgb(c):
    return tuple(int(round(v * 255)) for v in c)


def cactus_group(rng, leafiness):
    s = saguaro(rng, 80, arms=2, leafiness=leafiness)
    for k in range(2):
        a = rng.uniform(0, 6.3)
        off = np.array([np.cos(a), 0, np.sin(a)]) * 30
        s.extend(barrel_cactus(rng, 26, leafiness=leafiness).transformed(1.0, move=off))
    return s


def dead_trees(rng, leafiness):
    s = Scene()
    for k in range(3):
        a = rng.uniform(0, 6.3)
        off = np.array([np.cos(a), 0, np.sin(a)]) * (0 if k == 0 else 28)
        h = rng.uniform(60, 95)
        top = off + np.array([rng.uniform(-6, 6), h, rng.uniform(-6, 6)])
        s.add_tris(tube(off, top, 3.5, 1.5, 6), c255(120, 112, 100), BARKT)
        for j in range(3):
            b = rng.uniform(0, 6.3)
            st = off + (top - off) * rng.uniform(0.45, 0.85)
            s.add_tris(tube(st, st + np.array([np.cos(b) * 18, rng.uniform(6, 18), np.sin(b) * 18]), 1.6, 0.6, 5),
                       c255(120, 112, 100), BARKT)
    return s


def mangrove(rng, leafiness):
    s = broadleaf(rng, 80, crown=0.5, width=0.5, squash=0.6, leafiness=leafiness)
    for k in range(7):
        a = 2 * np.pi * k / 7
        foot = np.array([np.cos(a) * 22, 0, np.sin(a) * 22])
        s.add_tris(tube(foot, np.array([0, 22, 0]), 1.6, 2.2, 5), BARK, BARKT)
    return s


def oasis(rng, leafiness):
    s = Scene()
    # a little pool with two palms by it
    ring = [np.array([np.cos(t) * 30, 0.5, np.sin(t) * 22]) for t in np.linspace(0, 2 * np.pi, 17)]
    s.add_tris(np.array([[np.array([0, 0.5, 0]), ring[i], ring[i + 1]] for i in range(16)]), c255(70, 140, 180),
               ID["WATER"])
    for k in range(2):
        a = rng.uniform(0, 6.3)
        s.extend(palm(rng, 90, fronds=8, leafiness=leafiness).transformed(1.0, move=[np.cos(a) * 34, 0, np.sin(a) * 26]))
    return s


def vineyard(rng, leafiness):
    s = Scene()
    for r in range(3):
        z = (r - 1) * 22
        s.add_tris(tube([-34, 0, z], [-34, 16, z], 1.2, 1.2, 4), c255(120, 90, 60), ID["WOOD"])
        s.add_tris(tube([34, 0, z], [34, 16, z], 1.2, 1.2, 4), c255(120, 90, 60), ID["WOOD"])
        for k in range(7):
            x = -30 + k * 10
            col, tag = (c255(84, 130, 56), LEAF) if k % 3 else (c255(100, 50, 120), PURPLE)
            s.add_ball([x, 11, z], 6.0, col, LEAF)
            if tag == PURPLE:
                s.add_ball([x + 2, 7, z + 4], 2.4, col, PURPLE)
    s.leafy, s.rag = 0.4, 0.25
    return s


SPECIES = [
    # (name test, builder, palette colours)
    (lambda n: "aspen" in n, lambda r, L: broadleaf(r, 105, crown=0.5, width=0.3, squash=1.3, bark=c255(220, 214, 196),
                                                     leafiness=L), {"LEAF": (196, 170, 64), "BARK": (222, 216, 200)}),
    (lambda n: "cypress" in n, lambda r, L: conifer(r, 160, width=0.13, tiers=9, leafiness=L, bare=0.05),
     {"LEAF": (58, 104, 64)}),
    (lambda n: "dogwood" in n, lambda r, L: broadleaf(r, 90, width=0.42, squash=0.8, flowers=c255(240, 236, 226),
                                                       flower_share=0.45, leafiness=L),
     {"LEAF": (80, 130, 60), "RED": (240, 236, 226)}),
    (lambda n: "japanesemaple" in n, lambda r, L: broadleaf(r, 85, crown=0.45, width=0.55, squash=0.55, leafiness=L,
                                                             lean=0.08), {"LEAF": (190, 56, 48)}),
    (lambda n: "japanesetree" in n, lambda r, L: scots_pine(r, 90, leafiness=L), {"LEAF": (56, 100, 60), "BARK": (120, 96, 80)}),
    (lambda n: "mesquite" in n, lambda r, L: broadleaf(r, 80, crown=0.4, width=0.5, squash=0.6, n=34, ball=0.24,
                                                        leafiness=L, lean=0.1), {"LEAF": (112, 130, 72)}),
    (lambda n: "peachtree" in n, lambda r, L: broadleaf(r, 85, width=0.5, squash=0.75, flowers=c255(244, 166, 186),
                                                         flower_share=0.5, leafiness=L),
     {"LEAF": (90, 136, 60), "RED": (244, 166, 186)}),
    (lambda n: n.startswith("plum"), lambda r, L: broadleaf(r, 85, width=0.48, squash=0.8, flowers=c255(226, 150, 206),
                                                             flower_share=0.6, leafiness=L),
     {"LEAF": (120, 70, 96), "RED": (226, 150, 206)}),
    (lambda n: "scenic_tree" in n, lambda r, L: broadleaf(r, 115, width=0.45, n=34, leafiness=L), {"LEAF": (78, 130, 56)}),
    (lambda n: "scenicelm" in n, lambda r, L: broadleaf(r, 110, crown=0.5, width=0.5, squash=0.8, n=36, droop=0.15,
                                                         leafiness=L), {"LEAF": (70, 124, 54)}),
    (lambda n: "bougainvillea" in n, lambda r, L: bush(r, 55, width=0.6, n=12, flowers=c255(214, 60, 150), flower_share=0.6,
                                                       leafiness=L), {"LEAF": (64, 120, 52), "RED": (214, 60, 150)}),
    (lambda n: "rhody" in n, lambda r, L: bush(r, 38, width=0.8, n=10, flowers=c255(222, 92, 170), flower_share=0.5,
                                               leafiness=L), {"LEAF": (50, 100, 50), "RED": (222, 92, 170)}),
    (lambda n: "heather" in n, lambda r, L: bush(r, 22, width=1.3, n=14, leafiness=L), {"LEAF": (146, 92, 146)}),
    (lambda n: "ograss" in n, lambda r, L: reeds(r, 32, col=c255(150, 166, 86), n=26, leafiness=L), {"LEAF": (150, 166, 86)}),
    (lambda n: "cattails" in n, lambda r, L: reeds(r, 48, col=c255(110, 140, 70), n=22, heads=c255(110, 74, 44), leafiness=L),
     {"LEAF": (110, 140, 70), "WOOD": (110, 74, 44)}),
    (lambda n: "bamboo" in n, lambda r, L: reeds(r, 110, col=c255(120, 164, 72), n=18, leafiness=L), {"LEAF": (120, 164, 72)}),
    (lambda n: "vineyard" in n, vineyard, {"LEAF": (84, 130, 56), "PURPLE": (100, 50, 120), "WOOD": (120, 90, 60)}),
    (lambda n: "cactusgroup" in n, cactus_group, {"LEAF": (88, 130, 72), "YELLOW": (236, 200, 70)}),
    (lambda n: "mangrove" in n, mangrove, {"LEAF": (66, 110, 60)}),
    (lambda n: "deadswamp" in n, dead_trees, {"BARK": (120, 112, 100)}),
    (lambda n: "oasis" in n, oasis, {"LEAF": (74, 140, 58), "BARK": (150, 120, 86), "WATER": (70, 140, 180)}),
    (lambda n: "maple" in n, lambda r, L: broadleaf(r, 100, leaf=c255(84, 134, 52), leafiness=L), {"LEAF": (84, 134, 52)}),
    (lambda n: "pinefir" in n, lambda r, L: conifer(r, 120, width=0.22, tiers=7, leafiness=L), {"LEAF": (56, 112, 68)}),
    (lambda n: "pinespruce" in n, lambda r, L: conifer(r, 120, width=0.25, tiers=8, leafiness=L), {"LEAF": (62, 108, 86)}),
    (lambda n: n.startswith("treepine") or "linkspine" in n, lambda r, L: conifer(r, 115, width=0.27, tiers=6, leafiness=L),
     {"LEAF": (70, 120, 64)}),
    (lambda n: "scotspine" in n or "blackpine" in n, lambda r, L: scots_pine(r, 110, leafiness=L),
     {"LEAF": (58, 104, 62), "BARK": (150, 96, 70)}),
    (lambda n: "linkstree3" in n, lambda r, L: broadleaf(r, 95, width=0.46, squash=0.75, leafiness=L, lean=0.06),
     {"LEAF": (96, 132, 62)}),
    (lambda n: "linkstree4" in n, lambda r, L: broadleaf(r, 95, width=0.44, squash=0.9, leafiness=L), {"LEAF": (74, 120, 60)}),
    (lambda n: "willow" in n, lambda r, L: willow(r, 100, leafiness=L), {"LEAF": (118, 156, 74)}),
    (lambda n: "tallpalm" in n, lambda r, L: palm(r, 140, fronds=10, frond_len=0.3, curve=0.12, leafiness=L),
     {"LEAF": (74, 140, 58), "BARK": (150, 120, 86), "WOOD": (120, 90, 50)}),
    (lambda n: "palm" in n, lambda r, L: palm(r, 110, fronds=9, frond_len=0.4, curve=0.22, leafiness=L),
     {"LEAF": (74, 140, 58), "BARK": (150, 120, 86), "WOOD": (120, 90, 50)}),
    (lambda n: "cactusa" in n, lambda r, L: saguaro(r, 90, arms=2, leafiness=L), {"LEAF": (86, 128, 70)}),
    (lambda n: "cactusb" in n, lambda r, L: barrel_cactus(r, 40, leafiness=L), {"LEAF": (92, 134, 74)}),
    (lambda n: "cactusc" in n, lambda r, L: prickly_pear(r, 45, leafiness=L), {"LEAF": (104, 146, 82)}),
    (lambda n: "treecactus" in n, lambda r, L: saguaro(r, 110, arms=3, leafiness=L), {"LEAF": (96, 136, 76)}),
    (lambda n: "joshua" in n, lambda r, L: joshua(r, 90, leafiness=L), {"LEAF": (96, 120, 70), "BARK": (120, 104, 80)}),
    (lambda n: "cerc" in n, lambda r, L: broadleaf(r, 90, width=0.45, leafiness=L), {"LEAF": (190, 104, 160)}),
    (lambda n: "drac" in n, lambda r, L: dragon_tree(r, 90, leafiness=L), {"LEAF": (70, 120, 60), "BARK": (140, 126, 106)}),
    (lambda n: "tropicalbrush" in n,
     lambda r, L: bush(r, 50, width=0.8, n=11, flowers=c255(236, 90, 90), flower_share=0.08, leafiness=L),
     {"LEAF": (56, 128, 56), "RED": (236, 90, 90)}),
]


def species(name, rng, leafiness):
    """The tree for a sprite file name (lower case, no folder), with its palette colours in `pal`, or None."""
    for test, build, pal in SPECIES:
        if test(name):
            s = build(rng, leafiness)
            s.pal = pal
            return s
    return None


# ---- little sight gags on a few of the plants --------------------------------------------------------------------------------

def sun_hat(s, top, r):
    """A straw sun hat with a red band, sitting on `top`, tipped a little."""
    top = np.asarray(top, float)
    tip = np.array([0.25, 1.0, 0.1])
    tip /= np.linalg.norm(tip)
    s.add_tris(tube(top + tip * r * 0.05, top + tip * r * 0.22, r * 1.6, r * 1.6, 14), c255(236, 206, 120), YELLOW)
    s.add_tris(tube(top + tip * r * 0.2, top + tip * r * 1.5, r * 0.95, r * 0.75, 12), c255(236, 206, 120), YELLOW)
    s.add_tris(tube(top + tip * r * 0.22, top + tip * r * 0.6, r * 0.99, r * 0.93, 12), c255(200, 50, 50), RED)


def squirrel(s, p, side):
    """A squirrel clinging head-up to a trunk at p, on the trunk's `side` (a unit vector out of the bark)."""
    p = np.asarray(p, float)
    side = np.asarray(side, float)
    # a grey squirrel (it stands out from the bark), a little larger than life
    fur = ID["STONE"]
    k = 2.6
    body = p + side * 2.4 * k
    def blob(c, r):
        c = p + (np.asarray(c, float) - p) * k
        s.add_tris(ellipsoid(c, np.asarray(r, float) * k, 8, 6), c255(150, 148, 140), fur)
    blob(body, [2.0, 3.0, 2.0])
    blob(body + [0, 3.4, 0] + side * 0.5, [1.6, 1.6, 1.6])
    for sx in (-1, 1):
        blob(body + [sx * 0.9, 4.9, 0] + side * 0.3, [0.45, 0.8, 0.45])
    # the bushy tail curling up behind
    for k, (dy, out) in enumerate(((-2.6, 1.2), (-0.8, 2.2), (1.2, 2.6), (3.0, 2.2))):
        blob(body + [0, dy, 0] + side * out, [1.7 - k * 0.15] * 3)


def lost_ball(s, p):
    """A golf ball lost at the foot of a bush."""
    s.add_tris(ellipsoid(np.asarray(p, float), [4.6, 4.6, 4.6], 10, 7), c255(250, 250, 250), ID["WHITE"])


def gags(name, s, leaf):
    """Adds the gag a plant carries (on its grown frames only)."""
    if leaf < 0.99:
        return s
    if name == "cactusgroupa":
        sun_hat(s, [0, 80 * 0.92 + 80 * 0.075, 0], 80 * 0.075 * 1.1)
    elif name == "treecactuslg":
        sun_hat(s, [0, 110 * 0.92 + 110 * 0.075, 0], 110 * 0.075 * 1.1)
    elif name == "treemaplelarge":
        squirrel(s, [0, 9, 0], [0.6, 0.0, 0.8])
    elif name == "rhody":
        lost_ball(s, [26, 4.6, 18])
    elif name == "tropicalbrush_lg":
        lost_ball(s, [34, 4.6, 22])
    return s
