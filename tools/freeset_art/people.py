"""Golfers, celebrities and staff for the free set: a jointed figure of smooth lofted parts (head, cap, torso, arms, legs,
shoes; a club or putter in the hands), posed by keyframes for each animation the game plays (walk, address, swing, putt,
cheer, sulk, sit, drive a cart...). Clothes, skin, cap and hair are drawn in the palette slots the game recolours for every
golfer (mats.py), so outfits vary as in the game. Model space: y up, the figure faces +z, its left is +x; world units (a
golfer is about 68 tall)."""
import numpy as np

from mats import ID
from scene import Scene, box, tube

SHIRT, PANTS, SKIN, HAT, ALT, HANDS_F, SHOE, WHITE, DARK, STONE, WOOD, RED, YELLOW, GOLD, EARTH = (
    ID[k] for k in ("SHIRT", "PANTS", "SKIN", "HAT", "ALT", "HANDS_F", "SHOE", "WHITE", "DARK", "STONE", "WOOD", "RED",
                    "YELLOW", "GOLD", "EARTH"))


def rx(d):
    a = np.radians(d)
    c, s = np.cos(a), np.sin(a)
    return np.array([[1, 0, 0], [0, c, -s], [0, s, c]])


def ry(d):
    a = np.radians(d)
    c, s = np.cos(a), np.sin(a)
    return np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])


def rz(d):
    a = np.radians(d)
    c, s = np.cos(a), np.sin(a)
    return np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])


def loft(origin, R, rings, sides=10):
    """A smooth solid along a local y axis: rings (y, rx, rz) in the frame (origin, rotation R): triangles."""
    ang = np.linspace(0, 2 * np.pi, sides + 1)[:-1]
    pts = []
    for y, a, b in rings:
        ring = np.stack([np.cos(ang) * a, np.full(sides, y), np.sin(ang) * b], 1)
        pts.append(ring @ R.T + origin)
    tris = []
    for i in range(len(pts) - 1):
        p, q = pts[i], pts[i + 1]
        for k in range(sides):
            j = (k + 1) % sides
            tris.append([p[k], p[j], q[j]])
            tris.append([p[k], q[j], q[k]])
    c0 = np.asarray(origin) + R @ np.array([0, rings[0][0], 0])
    c1 = np.asarray(origin) + R @ np.array([0, rings[-1][0], 0])
    for k in range(sides):
        j = (k + 1) % sides
        tris.append([c0, pts[0][j], pts[0][k]])
        tris.append([c1, pts[-1][k], pts[-1][j]])
    return np.array(tris)


def blob(c, r, sides=8, stack=6):
    """A small ellipsoid r = (rx, ry, rz) at c."""
    rings = []
    for i in range(stack + 1):
        t = -np.pi / 2 + np.pi * i / stack
        rings.append((np.sin(t) * r[1], max(np.cos(t) * r[0], 0.05), max(np.cos(t) * r[2], 0.05)))
    return loft(np.asarray(c, float), np.eye(3), rings, sides)


# ---- bodies ------------------------------------------------------------------------------------------------------------------

class Body:
    """What a figure wears and how it is built."""

    def __init__(self, female=False, legs="pants", sleeves="short", hat="cap", scale=1.0, build=1.0, hair=None,
                 shirt=SHIRT, pants=PANTS, hatcol=HAT, skirt=False, socks=WHITE, bald=False, jacket=None):
        self.female = female
        self.legs = legs          # pants, shorts, knickers, skirt
        self.sleeves = sleeves    # long, short, tank
        self.hat = hat            # cap, visor, none, wide
        self.scale = scale
        self.build = build        # girth
        self.hair = hair if hair is not None else (ALT if female else EARTH)
        self.hands = HANDS_F if female else ALT
        self.shirt, self.pants, self.hatcol, self.socks, self.bald = shirt, pants, hatcol, socks, bald
        self.jacket = jacket


def body_of(name):
    """The body of a file name prefix (MaleKLS, FemaleSkSS, ...)."""
    n = name.lower()
    female = n.startswith("female")
    code = n[6:] if female else n[4:]
    code = code.split("_")[0]
    legs = {"kls": "knickers", "pls": "pants", "pss": "pants", "sss": "shorts", "skss": "skirt", "sktt": "skirt"}.get(code, "pants")
    sleeves = {"kls": "long", "pls": "long", "pss": "short", "sss": "short", "skss": "short", "sktt": "tank"}.get(code, "short")
    hat = "visor" if female else "cap"
    return Body(female=female, legs=legs, sleeves=sleeves, hat=hat, scale=0.95 if female else 1.0)


DEFAULT = dict(px=0.0, py=0.0, pz=0.0, pyaw=0.0, ppitch=0.0, sp=0.0, sy=0.0, sr=0.0, hp=0.0, hy=0.0,
               lsp=0.0, lsr=6.0, lsy=0.0, lel=8.0, rsp=0.0, rsr=6.0, rsy=0.0, rel=8.0,
               lhp=0.0, lhr=2.0, lkn=0.0, rhp=0.0, rhr=2.0, rkn=0.0, lft=0.0, rft=0.0)


def pose_at(keys, t):
    """Interpolated pose (smooth between keys) of a key list [(t, {..}), ...]; each key's values apply over DEFAULT."""
    full = [(k, {**DEFAULT, **v}, c) for k, v, c in keys]
    if t <= full[0][0]:
        return full[0][1], full[0][2]
    for (t0, a, ca), (t1, b, cb) in zip(full, full[1:]):
        if t0 <= t <= t1:
            u = 0 if t1 == t0 else (t - t0) / (t1 - t0)
            u = u * u * (3 - 2 * u)
            pose = {k: a[k] + (b[k] - a[k]) * u for k in a}
            club = None
            if ca is not None and cb is not None:
                club = dict(ca, dir=np.asarray(ca["dir"], float) * (1 - u) + np.asarray(cb["dir"], float) * u)
            elif ca is not None or cb is not None:
                club = ca if u < 0.5 else cb
            return pose, club
    return full[-1][1], full[-1][2]


def figure(body: Body, pose, club=None, seed=0):
    """The scene of a posed figure (model space, feet on y = 0)."""
    s = Scene()
    S = body.scale
    g = body.build
    f = body.female
    hipY = 34.0 * S
    # pelvis and torso frames
    Rp = ry(pose["pyaw"]) @ rx(pose["ppitch"])
    pelvis = np.array([pose["px"], hipY + pose["py"], pose["pz"]])
    Rt = Rp @ ry(pose["sy"]) @ rx(pose["sp"]) @ rz(pose["sr"])
    waist = pelvis + Rp @ np.array([0, 3.5 * S, 0])
    T = 19.0 * S
    shoulder_w = (6.4 if f else 7.6) * S * g
    hip_w = (4.0 if f else 3.6) * S * g

    def tag_skin_or(t, cover):
        return cover if cover is not None else SKIN

    # legs
    leg_parts = []
    for side, sgn in (("l", 1), ("r", -1)):
        hip = pelvis + Rp @ np.array([sgn * hip_w, 0, 0])
        Rth = Rp @ rz(sgn * pose[side + "hr"]) @ rx(-pose[side + "hp"])
        thigh_len, shin_len = 17.0 * S, 15.5 * S
        knee = hip + Rth @ np.array([0, -thigh_len, 0])
        Rsh = Rth @ rx(pose[side + "kn"])
        ankle = knee + Rsh @ np.array([0, -shin_len, 0])
        upper = body.pants
        lower = body.pants if body.legs in ("pants",) else SKIN
        if body.legs == "knickers":
            lower = body.socks
        if body.legs == "skirt":
            upper = SKIN
        r_th = 3.1 * S * g
        s.add_tris(loft(hip, Rth, [(0, r_th, r_th), (-thigh_len * 0.5, r_th * 0.92, r_th * 0.92), (-thigh_len, r_th * 0.72, r_th * 0.72)]),
                   [0.5] * 3, upper)
        if body.legs == "knickers":
            # baggy plus-fours over the knee
            s.add_tris(blob(knee + Rsh @ np.array([0, -2.0 * S, 0]), (2.9 * S, 3.0 * S, 2.9 * S)), [0.5] * 3, body.pants)
        if body.legs == "shorts":
            s.add_tris(loft(hip, Rth, [(0, r_th * 1.12, r_th * 1.12), (-thigh_len * 0.55, r_th * 1.05, r_th * 1.05)]), [0.5] * 3,
                       body.pants)
            upper_tag = SKIN
        r_sh = 2.2 * S * g
        s.add_tris(loft(knee, Rsh, [(0, r_sh, r_sh), (-shin_len * 0.4, r_sh * 1.05, r_sh * 1.05), (-shin_len, r_sh * 0.7, r_sh * 0.7)]),
                   [0.5] * 3, lower)
        # the shoe points along the pelvis' forward, flat on the ground (lifted with the ankle)
        fwd = Rp @ np.array([0, 0, 1.0])
        fwd[1] = 0
        fwd /= np.linalg.norm(fwd) + 1e-9
        heel = ankle + np.array([0, -1.0 * S, 0])
        toe_up = pose[side + "ft"]
        toe = heel + fwd * 6.5 * S + np.array([0, toe_up, 0])
        s.add_tris(tube(heel - fwd * 1.2 * S, toe, 1.9 * S, 1.6 * S, 6), [0.5] * 3, SHOE)
        leg_parts.append(ankle)
    # skirt: a cone from the waist
    if body.legs == "skirt":
        s.add_tris(loft(pelvis, Rp, [(4.5 * S, 5.6 * S * g, 4.2 * S * g), (-1.0 * S, 7.0 * S * g, 5.4 * S * g),
                                     (-9.5 * S, 8.4 * S * g, 6.8 * S * g)], 12), [0.5] * 3, body.pants)
    # hips (trousers)
    s.add_tris(loft(pelvis, Rp, [(-3.5 * S, 5.6 * S * g, 3.6 * S * g), (0, 6.2 * S * g, 4.0 * S * g),
                                 (4.0 * S, 5.6 * S * g, 3.7 * S * g)], 12), [0.5] * 3,
               body.pants if body.legs != "skirt" else body.pants)
    # torso: waist to shoulders, a little fuller at the chest
    shirt = body.shirt
    rings = [(0, 5.4 * S * g, 3.7 * S * g), (T * 0.45, 6.2 * S * g, 4.2 * S * g), (T * 0.8, 7.2 * S * g, 4.4 * S * g),
             (T, 6.4 * S * g, 3.6 * S * g)]
    if f:
        rings = [(0, 4.8 * S * g, 3.4 * S * g), (T * 0.4, 5.6 * S * g, 4.2 * S * g), (T * 0.7, 6.2 * S * g, 4.6 * S * g),
                 (T, 5.6 * S * g, 3.4 * S * g)]
    s.add_tris(loft(waist, Rt, rings, 12), [0.5] * 3, shirt)
    if body.jacket is not None:
        s.add_tris(loft(waist, Rt, [(y, a * 1.06, b * 1.08) for y, a, b in rings[:-1]] + [(T * 0.98, rings[-1][1] * 1.04, rings[-1][2] * 1.05)], 12),
                   [0.5] * 3, body.jacket)
    top = waist + Rt @ np.array([0, T, 0])
    # neck and head
    Rh = Rt @ rx(pose["hp"]) @ ry(pose["hy"])
    neck = top
    s.add_tris(loft(neck, Rh, [(-1.0 * S, 1.7 * S, 1.7 * S), (3.0 * S, 1.6 * S, 1.6 * S)], 8), [0.5] * 3, SKIN)
    hc = neck + Rh @ np.array([0, 7.0 * S, 0.4 * S])
    hr = np.array([4.2, 5.0, 4.6]) * S
    s.add_tris(blob(hc, hr, 10, 7) - hc + hc, [0.5] * 3, SKIN)
    # nose: a tiny bump forward, so the facing reads
    s.add_tris(blob(hc + Rh @ np.array([0, -0.5 * S, 4.4 * S]), (0.8 * S, 1.0 * S, 0.9 * S), 6, 4), [0.5] * 3, SKIN)
    # hair: a cap of hair at the back and top (women: fuller, with a ponytail)
    if not body.bald:
        hair_c = hc + Rh @ np.array([0, 0.9 * S, -0.9 * S])
        s.add_tris(blob(hair_c, hr * np.array([1.04, 0.92, 1.0]), 10, 7), [0.5] * 3, body.hair)
        if f:
            s.add_tris(blob(hc + Rh @ np.array([0, -1.0 * S, -4.8 * S]), (1.9 * S, 3.2 * S, 1.9 * S), 8, 5), [0.5] * 3, body.hair)
    # cap / visor / hat
    if body.hat == "cap":
        s.add_tris(blob(hc + Rh @ np.array([0, 1.8 * S, -0.2 * S]), hr * np.array([1.07, 0.75, 1.07]), 10, 6), [0.5] * 3, body.hatcol)
        brim_c = hc + Rh @ np.array([0, 1.4 * S, 4.6 * S])
        s.add_tris(blob(brim_c, (3.2 * S, 0.45 * S, 2.6 * S), 8, 3), [0.5] * 3, body.hatcol)
    elif body.hat == "visor":
        brim_c = hc + Rh @ np.array([0, 1.6 * S, 4.4 * S])
        s.add_tris(blob(brim_c, (3.4 * S, 0.45 * S, 2.6 * S), 8, 3), [0.5] * 3, body.hatcol)
        s.add_tris(loft(hc + Rh @ np.array([0, 1.2 * S, 0]), Rh, [(0, hr[0] * 1.05, hr[2] * 1.05), (1.4 * S, hr[0] * 1.02, hr[2] * 1.02)], 10),
                   [0.5] * 3, body.hatcol)
    elif body.hat == "wide":
        s.add_tris(blob(hc + Rh @ np.array([0, 2.5 * S, 0]), (8.0 * S, 0.6 * S, 8.0 * S), 12, 3), [0.5] * 3, body.hatcol)
        s.add_tris(blob(hc + Rh @ np.array([0, 3.8 * S, 0]), (4.4 * S, 2.6 * S, 4.4 * S), 10, 5), [0.5] * 3, body.hatcol)
    # arms
    hands = {}
    for side, sgn in (("l", 1), ("r", -1)):
        sh = top + Rt @ np.array([sgn * shoulder_w, -1.6 * S, 0])
        Ru = Rt @ rz(sgn * pose[side + "sr"]) @ rx(-pose[side + "sp"]) @ ry(pose[side + "sy"])
        ua, fa = 11.0 * S, 10.0 * S
        el = sh + Ru @ np.array([0, -ua, 0])
        Rf = Ru @ rx(-pose[side + "el"])
        wr = el + Rf @ np.array([0, -fa, 0])
        sleeve = shirt if body.sleeves != "tank" else SKIN
        r_u = 2.2 * S * g
        s.add_tris(loft(sh, Ru, [(1.0 * S, r_u * 1.25, r_u * 1.25), (-ua * 0.5, r_u, r_u), (-ua, r_u * 0.85, r_u * 0.85)]), [0.5] * 3,
                   sleeve)
        if body.sleeves == "short":
            # the sleeve ends half way down the upper arm; skin below
            s.add_tris(loft(sh, Ru, [(-ua * 0.55, r_u * 0.95, r_u * 0.95), (-ua, r_u * 0.85, r_u * 0.85)]), [0.5] * 3, SKIN)
        fore = shirt if body.sleeves == "long" else SKIN
        if body.jacket is not None:
            fore = body.jacket
        s.add_tris(loft(el, Rf, [(0, r_u * 0.85, r_u * 0.85), (-fa, r_u * 0.68, r_u * 0.68)]), [0.5] * 3, fore)
        hand = el + Rf @ np.array([0, -fa - 1.4 * S, 0])
        s.add_tris(blob(hand, (1.6 * S, 1.9 * S, 1.6 * S), 6, 4), [0.5] * 3, body.hands)
        hands[side] = hand
    # the club: from the hands (or one hand) along a direction given in model space
    if club is not None:
        kind = club.get("kind", "iron")
        d = np.asarray(club["dir"], float)
        d = d / (np.linalg.norm(d) + 1e-9)
        grip = hands["r"] * 0.5 + hands["l"] * 0.5 if club.get("hands", "both") == "both" else hands[club["hands"]]
        length = {"driver": 40.0, "iron": 36.0, "wedge": 32.0, "putter": 30.0}.get(kind, 36.0) * S
        tip = grip + d * length
        s.add_tris(tube(grip - d * 2.5 * S, tip, 1.15 * S, 0.95 * S, 5), [0.5] * 3, STONE)
        # the head: a little block across the shaft's end
        side = np.cross(d, np.array([0, 1.0, 0]))
        if np.linalg.norm(side) < 1e-3:
            side = np.array([1.0, 0, 0])
        side /= np.linalg.norm(side)
        hl = 3.6 if kind == "driver" else 3.0
        s.add_tris(tube(tip - side * 0.3 * S, tip + side * hl * S, 1.3 * S if kind == "driver" else 0.9 * S, 1.1 * S, 6), [0.5] * 3,
                   DARK if kind != "putter" else STONE)
    return s


# ---- the animations ----------------------------------------------------------------------------------------------------------
# Keys are (t 0..1, pose values, club). Club dir is in the figure's model space (it faces +z, its left is +x).

def walk(t, tired=False, carry=True):
    a = np.sin(2 * np.pi * t)
    b = np.cos(2 * np.pi * t)
    k = 0.7 if tired else 1.0
    p = dict(DEFAULT)
    stride = 26 * k
    p.update(lhp=stride * a, rhp=-stride * a, lkn=max(0, 30 * k * -b) + 6, rkn=max(0, 30 * k * b) + 6,
             lsp=-20 * k * a, rsp=20 * k * a * (0.2 if carry else 1), py=-1.2 * abs(b) + 0.6, pyaw=5 * a, sy=-7 * a,
             sp=12 if tired else 3, hp=10 if tired else 0, lel=14, rel=22, lsr=7, rsr=8)
    club = dict(kind="iron", hands="r", dir=[0.15, -0.75, -0.6]) if carry else None
    return p, club


ADDRESS = dict(sp=32, py=-2.5, lhp=12, rhp=12, lkn=18, rkn=18, lhr=5, rhr=5, lsp=36, rsp=34, lsr=-14, rsr=-12, lel=4, rel=14,
               hp=6, rsy=10)
CLUB_ADDRESS = dict(kind="driver", hands="both", dir=[0.18, -0.72, 0.66])


def swing_keys(kind="driver", power=1.0, bad=0):
    """A right-handed swing in the golfer's frame (faces +z towards the ball, target on the left, +x)."""
    top = dict(ADDRESS, sy=-80 * power, pyaw=-35 * power, lsp=110 * power, lsr=-55 * power, rsp=95 * power, rsr=25,
               rel=95 * power, lel=10, lhr=3, rkn=22, lkn=26, hp=12, hy=40 * power)
    impact = dict(ADDRESS, sy=12, pyaw=28, lkn=12, rkn=26, rhp=6, rft=1.0)
    finish = dict(ADDRESS, sp=6, py=-1, sy=95, pyaw=75, lsp=120, lsr=40, rsp=125, rsr=-30, lel=70, rel=50, rkn=34, rhp=-12,
                  lhp=4, lkn=6, rft=2.5, hy=-70, hp=-6)
    club_top = dict(kind=kind, hands="both", dir=[0.85 * power + 0.15, 0.35, 0.2 - 0.6 * (1 - power)])
    club_imp = dict(kind=kind, hands="both", dir=[0.05, -0.75, 0.66])
    club_fin = dict(kind=kind, hands="both", dir=[-0.75, -0.25, -0.55])
    keys = [(0.0, ADDRESS, CLUB_ADDRESS | dict(kind=kind)), (0.1, ADDRESS, CLUB_ADDRESS | dict(kind=kind)),
            (0.45, top, club_top), (0.5, top, club_top), (0.6, impact, club_imp), (0.78, finish, club_fin),
            (1.0, finish, club_fin)]
    if bad == 1:
        # a lunge and a stumble after a mishit
        keys[-2] = (0.78, dict(finish, sp=20, py=-4, rkn=40, lkn=30, hy=-20, hp=20, lsp=60, rsp=70), dict(club_fin, dir=[-0.4, -0.7, 0.3]))
        keys[-1] = (1.0, dict(finish, sp=24, py=-4, rkn=40, lkn=30, hy=10, hp=25, lsp=40, rsp=50, lel=40), dict(club_fin, dir=[0.1, -0.9, 0.4]))
    if bad == 2:
        keys[2] = (0.45, dict(top, sy=-50, lsp=80, rsp=70), dict(club_top, dir=[0.5, 0.6, 0.3]))
        keys[3] = (0.5, dict(top, sy=-50, lsp=80, rsp=70), dict(club_top, dir=[0.5, 0.6, 0.3]))
        keys[-1] = (1.0, dict(finish, sy=60, lsp=80, rsp=80, hp=20, hy=-30), dict(club_fin, dir=[-0.6, -0.3, 0.4]))
    return keys


def putt_keys():
    addr = dict(ADDRESS, sp=40, lsp=38, rsp=38, lsr=-18, rsr=-16, lel=18, rel=22, hp=14, lkn=14, rkn=14)
    back = dict(addr, sy=-6, lsp=30, rsp=28)
    thru = dict(addr, sy=6, lsp=46, rsp=48)
    c0 = dict(kind="putter", hands="both", dir=[0.0, -0.9, 0.42])
    cb = dict(kind="putter", hands="both", dir=[-0.35, -0.85, 0.38])
    cf = dict(kind="putter", hands="both", dir=[0.35, -0.85, 0.38])
    return [(0, addr, c0), (0.2, addr, c0), (0.45, back, cb), (0.6, thru, cf), (0.75, thru, cf), (0.9, dict(addr, hp=-5, hy=20), cf),
            (1.0, dict(addr, hp=-5, hy=20), cf)]


def anim(name, t, body):
    """The pose and club of animation `name` (lower case, without the body prefix) at t in 0..1."""
    n = name
    if n in ("normalwalk", "walk"):
        return walk(t)
    if n == "tiredwalk":
        return walk(t, tired=True)
    if n in ("perfectswing", "normalswing"):
        return pose_at(swing_keys("driver"), t)
    if n.startswith("noaccswing"):
        return pose_at(swing_keys("driver", 1.0, bad=1), t)
    if n == "noimagswing":
        return pose_at(swing_keys("driver", 0.8, bad=2), t)
    if n == "pitch":
        return pose_at(swing_keys("wedge", 0.55), t)
    if n.startswith("perfectswing_"):
        return pose_at(swing_keys("driver"), t * 0.8)
    if n in ("normaladdress", "pitchaddress"):
        w = np.sin(2 * np.pi * t)
        kind = "wedge" if n.startswith("pitch") else "driver"
        return dict(DEFAULT, **ADDRESS, sy=-4 * max(w, 0), hy=10 * max(-w, 0) + 4), dict(CLUB_ADDRESS, kind=kind, dir=[0.18 - 0.12 * max(w, 0), -0.72, 0.66])
    if n == "puttaddress":
        k = putt_keys()
        p, c = pose_at(k, 0.0)
        return dict(p, hy=12 * np.sin(np.pi * t) ** 2), c
    if n == "putt":
        return pose_at(putt_keys(), t)
    if n == "lineupputt":
        crouch = dict(py=-14, lhp=70, rhp=40, lkn=120, rkn=80, sp=24, lsp=40, rsp=70, rel=60, lel=60, hp=-6, rft=-1)
        k = [(0, dict(sp=4), None), (0.25, crouch, dict(kind="putter", hands="r", dir=[0, 0.9, 0.3])),
             (0.75, crouch, dict(kind="putter", hands="r", dir=[0, 0.9, 0.3])), (1.0, dict(sp=4), None)]
        return pose_at(k, t)
    if n in ("happy",):
        j = np.sin(np.pi * min(1, t * 2)) if t < 0.5 else 0
        p = dict(DEFAULT, py=8 * j, lsp=170, rsp=170, lsr=25, rsr=25, lel=10, rel=10, lkn=30 * j, rkn=30 * j, hp=-15)
        return p, dict(kind="iron", hands="r", dir=[0.1, 0.9, 0.3])
    if n == "successa":
        pump = abs(np.sin(3 * np.pi * t))
        return dict(DEFAULT, rsp=60 + 80 * pump, rel=120 - 60 * pump, rsr=10, lsp=-10, hp=-10, sy=10), None
    if n == "failurea":
        return dict(DEFAULT, sp=10 + 10 * np.sin(np.pi * t), lsp=150, rsp=150, lsr=-30, rsr=-30, lel=130, rel=130, hp=12), None
    if n == "sad":
        return dict(DEFAULT, sp=14, hp=24, hy=25 * np.sin(4 * np.pi * t), lsp=-6, rsp=-6, lsr=2, rsr=2), dict(kind="iron", hands="r", dir=[0, -0.9, 0.3])
    if n == "fidget":
        w = np.sin(2 * np.pi * t)
        return dict(DEFAULT, px=1.5 * w, sr=4 * w, hy=30 * np.sin(4 * np.pi * t), lhr=6 + 3 * w, rhr=6 - 3 * w, lsp=8, rsp=8,
                    lel=40, rel=40, lsr=-10, rsr=-10), None
    if n == "lookup":
        return dict(DEFAULT, hp=-30, rsp=140, rel=110, rsr=-20, sp=-4), None
    if n == "pointat":
        u = min(1, t * 3)
        return dict(DEFAULT, rsp=85 * u, rsr=8, rel=4, hy=-10, sy=-10), None
    if n == "leanleft" or n == "leanright":
        sgn = 1 if n == "leanleft" else -1
        w = np.sin(np.pi * t)
        return dict(DEFAULT, sr=sgn * 22 * w, px=sgn * 2 * w, lsr=12 + 30 * w * (sgn > 0), rsr=12 + 30 * w * (sgn < 0),
                    lhr=6, rhr=6, hy=sgn * 15 * w), None
    if n == "handshake":
        w = np.sin(4 * np.pi * t)
        return dict(DEFAULT, rsp=50 + 8 * w, rel=30, rsr=-6, sp=6), None
    if n == "sq":
        w = np.sin(2 * np.pi * t)
        return dict(DEFAULT, hy=10 * w, lsp=4, rsp=4, lel=12, rel=12), dict(kind="iron", hands="r", dir=[0.15, -0.95, 0.25])
    if n in ("sitsq", "sitting"):
        w = np.sin(2 * np.pi * t) if n == "sitting" else 0
        return dict(DEFAULT, py=-14, lhp=88, rhp=88, lkn=90, rkn=90, lsp=30, rsp=30, lel=50, rel=50, hy=25 * w, sp=-4), None
    if n == "cart":
        return dict(DEFAULT, py=-14, lhp=80, rhp=80, lkn=80, rkn=80, lsp=40, rsp=40, lel=40, rel=40, sp=-4), None
    # anything else: standing at ease
    return dict(DEFAULT), None


def golf_frame(name):
    """Swings and putts are posed facing the ball; the sprite's heading is the shot's: turn the figure so its left points
    along the heading (+z)."""
    n = name
    return any(k in n for k in ("swing", "address", "pitch", "putt"))


def cart_scene(rng=None):
    """A golf cart round the origin (model space, facing +z): body, seats, roof on posts, wheels."""
    s = Scene()
    w, L = 13.0, 26.0
    s.add_tris(box([0, 5.5, 0], [w * 2, 5, L]), [0.9] * 3, WHITE)
    s.add_tris(box([0, 9.5, L * 0.32], [w * 2, 6, 7]), [0.9] * 3, WHITE)
    s.add_tris(box([0, 11, -2], [w * 1.8, 3, 12]), [0.9] * 3, DARK)
    s.add_tris(box([0, 15, -7], [w * 1.8, 9, 2.5]), [0.9] * 3, DARK)
    for x in (-w + 1.5, w - 1.5):
        for z in (-L * 0.42, L * 0.3):
            s.add_tris(tube([x, 8, z], [x, 34, z], 0.6, 0.6, 4), [0.9] * 3, STONE)
    s.add_tris(box([0, 35, -2], [w * 2.1, 1.6, L * 0.9]), [0.9] * 3, WHITE)
    for x in (-w, w):
        for z in (-L * 0.36, L * 0.36):
            s.add_tris(tube([x - 1.2 * np.sign(x), 3.4, z], [x + 0.8 * np.sign(x), 3.4, z], 3.4, 3.4, 8), [0.9] * 3, DARK)
    # a golf bag at the back
    s.add_tris(tube([0, 6, -L * 0.52], [0, 22, -L * 0.6], 3.2, 3.2, 8), [0.9] * 3, RED)
    return s


def prop_mower():
    s = Scene()
    s.add_tris(box([0, 5, 14], [16, 6, 14]), [0.5] * 3, RED)
    s.add_tris(box([0, 9, 14], [8, 4, 7]), [0.5] * 3, DARK)
    for x in (-8, 8):
        for z in (8, 20):
            s.add_tris(tube([x - 1, 2.5, z], [x + 1, 2.5, z], 2.5, 2.5, 6), [0.5] * 3, DARK)
    s.add_tris(tube([0, 8, 8], [0, 30, -2], 0.8, 0.8, 4), [0.5] * 3, STONE)
    s.add_tris(tube([-6, 30, -2], [6, 30, -2], 0.8, 0.8, 4), [0.5] * 3, STONE)
    return s


def prop_cart_vendor():
    s = Scene()
    s.add_tris(box([0, 14, 22], [22, 16, 16]), [0.5] * 3, WHITE)
    s.add_tris(box([0, 22.5, 22], [23, 1.5, 17]), [0.5] * 3, RED)
    for x in (-10, 10):
        s.add_tris(tube([x - 1, 4, 22], [x + 1, 4, 22], 4, 4, 8), [0.5] * 3, DARK)
    s.add_tris(tube([0, 23, 22], [0, 46, 22], 0.6, 0.6, 4), [0.5] * 3, STONE)
    n = 8
    for k in range(n):
        a0, a1 = 2 * np.pi * k / n, 2 * np.pi * (k + 1) / n
        s.add_tris(np.array([[[0, 50, 22], [np.cos(a0) * 16, 44, 22 + np.sin(a0) * 16], [np.cos(a1) * 16, 44, 22 + np.sin(a1) * 16]]]),
                   [0.5] * 3, RED if k % 2 == 0 else WHITE)
    return s


def prop_tray(y=38):
    s = Scene()
    s.add_tris(tube([0, y, 9], [0, y + 1, 9], 7, 7, 10), [0.5] * 3, STONE)
    for dx in (-3, 2):
        s.add_tris(tube([dx, y + 1, 9], [dx, y + 5, 9], 1.2, 1.4, 6), [0.5] * 3, RED)
    return s


# staff: their body, and what their action is
STAFF = {
    "gk": (dict(hat="cap"), "rake"), "lawntech": (dict(hat="cap"), "mow"), "greeter": (dict(hat="none"), "wave"),
    "ranger": (dict(hat="wide"), "point"), "marshall": (dict(hat="wide"), "point"), "sodavendor": (dict(hat="cap"), "vend"),
    "traygirl": (dict(female=True, hat="none", legs="skirt"), "tray"), "golfceleb": (dict(hat="cap"), "swing"),
    "bev": (dict(female=True, hat="visor", legs="skirt"), "tray"), "fatguy": (dict(build=1.35, hat="cap"), "walk"),
}
CELEBS = {
    "actionstar": dict(build=1.2, hat="none", sleeves="tank"), "agingstar": dict(hat="wide", bald=True),
    "basketball": dict(scale=1.15, legs="shorts", sleeves="tank", hat="none"), "comedian": dict(hat="none"),
    "female": dict(female=True, legs="skirt", hat="none"), "femcomic": dict(female=True, hat="none"),
    "fitnessfem": dict(female=True, legs="shorts", sleeves="tank", hat="visor"), "genmale": dict(hat="cap"),
    "movieprincess": dict(female=True, legs="skirt", hat="wide"), "politician": dict(hat="none", jacket=PANTS),
    "rockstar": dict(hat="none", sleeves="long"), "supermodel": dict(female=True, scale=1.05, legs="skirt", hat="none"),
}


def staff_or_celeb(name):
    """(body, animation) of an Employee or Celebs sprite name (lower case), or None."""
    n = name
    for key, (kw, action) in STAFF.items():
        if n.startswith(key):
            rest = n[len(key):].lstrip("_")
            if "walk" in rest or "stride" in rest:
                a = "walk"
            elif rest.startswith("sq"):
                a = "sq"
            elif "cart" in rest or "incart" in rest:
                a = "cart"
            else:
                a = action
            return Body(**kw), a
    for key, kw in CELEBS.items():
        if n.startswith(key):
            rest = n[len(key):]
            if "walk" in rest:
                a = "walk"
            elif "_sq" in rest:
                a = "sq"
            elif "dont" in rest or "dislike" in rest:
                a = "sad"
            elif "like" in rest:
                a = "happy"
            elif "interact" in rest:
                a = "handshake"
            else:
                a = "celebrate"
            return Body(**kw), a
    return None


def staff_anim(a, t):
    """Poses of the staff's own actions."""
    w = np.sin(2 * np.pi * t)
    if a == "rake":
        return dict(DEFAULT, sp=18, lsp=50 + 25 * w, rsp=40 + 25 * w, lsr=-20, rsr=-20, lel=20, rel=30, lkn=10, rkn=10), \
            dict(kind="iron", hands="both", dir=[0.0, -0.75, 0.66])
    if a == "mow":
        return dict(DEFAULT, sp=8, lsp=40, rsp=40, lsr=-12, rsr=-12, lel=30, rel=30, lhp=14 * w, rhp=-14 * w, lkn=10, rkn=10), None
    if a == "wave":
        return dict(DEFAULT, rsp=150, rsr=20 + 25 * w, rel=30, hy=10), None
    if a == "point":
        return dict(DEFAULT, rsp=88, rsr=5, rel=4, hy=-12, sy=-10, lsp=10), None
    if a in ("vend", "tray"):
        return dict(DEFAULT, lsp=60, rsp=60 + 20 * max(w, 0), lsr=-15, rsr=-15, lel=80, rel=70 - 20 * max(w, 0)), None
    if a == "celebrate":
        return dict(DEFAULT, lsp=150 + 20 * w, rsp=150 - 20 * w, lsr=30, rsr=30, lel=20, rel=20, py=3 * abs(w), hp=-10), None
    return None


def person_scene(prefix, anim_name, t, view_deg):
    """The scene of a person sprite frame: the figure posed and turned to its view's heading (model +z is the heading)."""
    body = body_of(prefix) if prefix.lower().startswith(("male", "female")) else Body()
    props = None
    if prefix == "_staff":
        body, anim_name = staff_or_celeb(anim_name)
        sp = staff_anim(anim_name, t)
        if anim_name == "mow":
            props = prop_mower()
        elif anim_name == "vend":
            props = prop_cart_vendor()
        elif anim_name == "tray":
            props = prop_tray()
        if sp is not None:
            fig = figure(body, sp[0], sp[1])
            if props is not None:
                fig.extend(props)
            if anim_name == "swing":
                fig = fig.transformed(1.0, rot=ry(-90))
            return fig
        if anim_name == "swing":
            anim_name = "perfectswing"
    pose, club = anim(anim_name, t, body)
    fig = figure(body, pose, club)
    if golf_frame(anim_name):
        fig = fig.transformed(1.0, rot=ry(-90))
    if anim_name == "cart":
        cart = cart_scene()
        fig = fig.transformed(1.0, move=[-6, 6, -4])
        cart.extend(fig)
        fig = cart
    return fig
