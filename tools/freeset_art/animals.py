"""The animals of the free set (crane, crocodile, duck, elk, flamingo, gila monster, roadrunner, sheep, snake), modelled
from tubes and ellipsoids and posed for each clip (walk or run, stand, drink or eat going down and in a loop, the
crocodile's back scratch and roll, the reptiles' fidget). Drawn in the palette ramps (mats.py) with their own colours;
a part's tone darkens or lightens it within its ramp. Model space: y up, the animal faces +z."""
import numpy as np

from landmarks import M, rx, ry, rz
from mats import ID

TAU = 2 * np.pi


def chain(m, pts, radii, tag, tone=1.0, sides=8, tones=None):
    """Tubes through the points with the radii (and a ball at every joint so the bends are round)."""
    for i in range(len(pts) - 1):
        tt = tones[i] if tones is not None else tone
        m.cyl(pts[i], pts[i + 1], radii[i], radii[i + 1], tag, sides, tt, caps=False)
    for i, p in enumerate(pts):
        tt = tones[min(i, len(tones) - 1)] if tones is not None else tone
        m.ell(p, [radii[i]] * 3, tag, sides, 5, tt)


def clip(anim, t):
    """(kind, phase) of a clip name: walk, run, stand, down (going down), loop (head down), fidget, scratch, roll."""
    a = anim.lower()
    if "walk" in a:
        return "walk", t
    if "run" in a:
        return "run", t
    if "down" in a:
        return "down", t
    if "loop" in a:
        return "loop", t
    if "fidget" in a:
        return "fidget", t
    if "scratch" in a:
        return "scratch", t
    if "turnover" in a:
        return "roll", t
    return "stand", t


def head_drop(kind, t):
    """How far the head is lowered (0 up .. 1 at the ground)."""
    if kind == "down":
        return 0.5 - 0.5 * np.cos(np.pi * min(t * 1.3, 1.0))
    if kind == "loop":
        return 0.92 + 0.08 * np.sin(TAU * t * 2)
    return 0.0


# ---- birds -------------------------------------------------------------------------------------------------------------------

def bird(kind, t, sp):
    """A wading or running bird: body, neck, head and beak, legs. `sp` gives its proportions and colours."""
    m = M()
    leg_h = sp["leg"]
    body_c = np.array([0, leg_h + sp["body"][1] * 0.6, 0])
    walk = kind in ("walk", "run")
    ph = TAU * t
    bob = 0.6 * abs(np.sin(ph)) if walk else 0.0
    body_c = body_c + [0, bob, 0]
    # body, tail and wings
    m.ell(body_c, sp["body"], sp["tag"], 12, 8)
    if sp.get("wing"):
        for sx in (-1, 1):
            m.ell(body_c + [sx * sp["body"][0] * 0.55, sp["body"][1] * 0.15, -sp["body"][2] * 0.15],
                  [sp["body"][0] * 0.45, sp["body"][1] * 0.7, sp["body"][2] * 0.85], sp["wing"], 10, 6, sp.get("wing_tone", 0.9))
    tail = body_c + [0, sp["body"][1] * 0.2, -sp["body"][2]]
    m.cyl(tail, tail + [0, sp.get("tail_up", 0.5), -sp["tail"]], sp["body"][0] * 0.35, 0.3, sp.get("tail_tag", sp["tag"]), 6,
          sp.get("tail_tone", 0.85))
    # legs: alternate when walking; the flamingo stands on one at rest
    for k, sx in enumerate((-1, 1)):
        hip = body_c + [sx * sp["body"][0] * 0.4, -sp["body"][1] * 0.5, 0]
        swing = np.sin(ph + k * np.pi) * (0.5 if kind == "run" else 0.3) if walk else 0.0
        foot = np.array([hip[0], 0, hip[2] + swing * leg_h])
        lift = max(0.0, np.sin(ph + k * np.pi)) * leg_h * 0.25 if walk else 0.0
        foot[1] += lift
        if sp.get("one_leg") and kind == "stand" and k == 1:
            knee = hip + [0, -leg_h * 0.45, -leg_h * 0.2]
            m.cyl(hip, knee, sp["leg_r"], sp["leg_r"], "SHOE", 4)
            m.cyl(knee, hip + [0, -leg_h * 0.2, 0], sp["leg_r"], sp["leg_r"], "SHOE", 4)
            continue
        knee = (hip + foot) / 2 + [0, 0, -leg_h * 0.08]
        m.cyl(hip, knee, sp["leg_r"] * 1.3, sp["leg_r"], "SHOE", 4)
        m.cyl(knee, foot, sp["leg_r"], sp["leg_r"], "SHOE", 4)
        m.cyl(foot, foot + [0, 0, sp["leg_r"] * 5], sp["leg_r"], sp["leg_r"] * 0.6, "SHOE", 4)
    # the neck: from the breast up and forward to the head, bent down to the ground when drinking
    drop = head_drop(kind, t)
    base = body_c + [0, sp["body"][1] * 0.3, sp["body"][2] * 0.8]
    up = sp["neck"]
    look = 0.0
    if kind in ("stand", "fidget"):
        look = 0.35 * np.sin(TAU * t)
    head_up = base + np.array([np.sin(look) * up * 0.3, up, up * 0.25 + np.cos(look) * 0.0])
    head_down = np.array([0, sp["head"] * 0.9, base[2] + up * 0.55])
    head = head_up * (1 - drop) + head_down * drop
    mid = (base + head) / 2 + np.array([0, up * 0.12 * (1 - drop), -up * 0.1 + drop * up * 0.15])
    nr = sp["neck_r"]
    chain(m, [base, mid, head], [nr * 1.25, nr, nr * 0.9], sp.get("neck_tag", sp["tag"]), sp.get("neck_tone", 1.0), 6)
    m.ell(head, [sp["head"]] * 3, sp.get("head_tag", sp["tag"]), 10, 6, sp.get("head_tone", 1.0))
    # the beak points ahead, or down when drinking
    bd = np.array([0, -0.25 - drop * 0.6, 1.0])
    bd /= np.linalg.norm(bd)
    m.cyl(head + bd * sp["head"] * 0.7, head + bd * (sp["head"] + sp["beak"]), sp["head"] * 0.38, 0.2, "YELLOW", 5,
          sp.get("beak_tone", 1.0))
    if sp.get("crest"):
        m.cyl(head + [0, sp["head"] * 0.6, -sp["head"] * 0.2], head + [0, sp["head"] * 1.6, -sp["head"] * 1.2],
              sp["head"] * 0.4, 0.2, sp["crest"], 5, 0.8)
    if sp.get("cap"):
        m.ell(head + [0, sp["head"] * 0.7, 0], [sp["head"] * 0.55] * 3, sp["cap"], 6, 4)
    for sx in (-1, 1):
        m.ell(head + [sx * sp["head"] * 0.75, sp["head"] * 0.2, sp["head"] * 0.3], [sp["head"] * 0.18] * 3, "DARK", 5, 4, 0.5)
    return m


BIRDS = {
    "crane": (dict(tag="WHITE", body=[4.6, 4.2, 6.5], leg=16, leg_r=0.55, neck=12, neck_r=1.2, head=1.9, beak=3.8, tail=3,
                   wing="STONE", wing_tone=0.95, tail_tag="DARK", cap="RED"),
              {"WHITE": (226, 226, 222), "STONE": (150, 152, 160), "DARK": (52, 52, 58), "RED": (200, 50, 46),
               "YELLOW": (200, 180, 90), "SHOE": (60, 60, 64)}),
    "flamingo": (dict(tag="RED", body=[4.6, 4.2, 6.5], leg=17, leg_r=0.5, neck=13, neck_r=1.15, head=1.9, beak=2.8, tail=2,
                      wing="RED", wing_tone=1.08, one_leg=True, tail_tag="DARK", beak_tone=0.6),
                 {"RED": (238, 128, 150), "DARK": (40, 36, 40), "YELLOW": (220, 200, 180), "SHOE": (200, 110, 120)}),
    "rr": (dict(tag="EARTH", body=[3.6, 3.4, 6], leg=7, leg_r=0.45, neck=4, neck_r=1.4, head=2.2, beak=3.4, tail=10,
                tail_up=3.5, wing="EARTH", wing_tone=0.8, crest="DARK", head_tone=0.95),
           {"EARTH": (150, 118, 82), "DARK": (54, 46, 40), "YELLOW": (90, 80, 70), "SHOE": (110, 100, 90),
            "WHITE": (230, 226, 214)}),
    "duck": (dict(tag="STONE", body=[4.2, 3.2, 6.5], leg=2.2, leg_r=0.5, neck=3.2, neck_r=1.5, head=2.2, beak=2.6, tail=2,
                  tail_up=1.5, wing="EARTH", wing_tone=0.95, head_tag="LEAF", neck_tag="LEAF"),
             {"STONE": (170, 168, 160), "EARTH": (130, 98, 70), "LEAF": (40, 110, 70), "YELLOW": (236, 190, 60),
              "SHOE": (232, 140, 50), "DARK": (30, 30, 30)}),
}


# ---- four-footed animals ----------------------------------------------------------------------------------------------------

def quadruped(kind, t, sp):
    m = M()
    run = kind == "run"
    walk = kind in ("walk", "run")
    ph = TAU * t
    L, H = sp["len"], sp["leg"]
    bounce = (1.2 * abs(np.sin(ph)) if run else 0.4 * abs(np.sin(ph * 2)) if walk else 0.0) * sp.get("bounce", 1.0)
    body_c = np.array([0, H + sp["body"][1] * 0.55 + bounce, 0])
    if sp.get("wool"):
        # a woolly body of clumps
        rng = np.random.default_rng(3)
        for k in range(16):
            d = rng.normal(size=3)
            d /= np.linalg.norm(d)
            d[1] = abs(d[1]) * 0.8
            m.s.add_ball(body_c + d * np.array(sp["body"]) * 0.75, sp["body"][1] * 0.55, np.ones(3), ID[sp["tag"]])
        m.ell(body_c, np.array(sp["body"]) * 0.92, sp["tag"], 12, 8, 0.95)
    else:
        m.ell(body_c, sp["body"], sp["tag"], 14, 9)
        m.ell(body_c + [0, sp["body"][1] * 0.2, -sp["body"][2] * 0.35], np.array(sp["body"]) * [0.9, 0.85, 0.55],
              sp.get("rump", sp["tag"]), 12, 7, sp.get("rump_tone", 1.0))
    # legs: diagonal pairs together (a trot), all four stretched out and gathered in when running
    for i, (sx, sz) in enumerate(((-1, 1), (1, 1), (-1, -1), (1, -1))):
        hip = body_c + [sx * sp["body"][0] * 0.55, -sp["body"][1] * 0.45, sz * sp["body"][2] * 0.6]
        if run:
            sw = np.sin(ph + (0 if sz > 0 else np.pi * 0.8) + sx * 0.3)
        elif walk:
            sw = np.sin(ph * 2 + (0 if (i in (0, 3)) else np.pi))
        else:
            sw = 0.0
        foot = np.array([hip[0], 0.0, hip[2] + sw * H * (0.55 if run else 0.3)])
        foot[1] = max(0.0, -np.cos(ph * (1 if run else 2) + (0 if (i in (0, 3)) else np.pi))) * H * 0.2 if walk else 0.0
        knee = (hip + foot) / 2 + [0, 0, -sz * H * 0.06]
        m.cyl(hip, knee, sp["leg_r"] * 1.5, sp["leg_r"], sp.get("leg_tag", sp["tag"]), 6)
        m.cyl(knee, foot, sp["leg_r"], sp["leg_r"] * 0.85, sp.get("leg_tag", sp["tag"]), 6, 0.9)
        m.ell(foot + [0, sp["leg_r"] * 0.6, 0], [sp["leg_r"] * 1.1] * 3, "DARK", 6, 4)
    # neck and head, lowered to graze
    drop = head_drop(kind, t)
    base = body_c + [0, sp["body"][1] * 0.3, sp["body"][2] * 0.85]
    look = 0.3 * np.sin(TAU * t) if kind == "stand" else 0.0
    head_up = base + np.array([np.sin(look) * sp["neck"] * 0.4, sp["neck"] * 0.8, sp["neck"] * 0.45])
    head_down = np.array([0, sp["head"][1] * 1.1, base[2] + sp["neck"] * 0.7])
    head = head_up * (1 - drop) + head_down * drop
    chew = 0.15 * np.sin(TAU * t * 4) if kind == "loop" else 0.0
    m.cyl(base, head, sp["neck_r"] * 1.2, sp["neck_r"], sp.get("neck_tag", sp["tag"]), 8)
    hd = np.array([0, -0.5 - drop * 0.6 + chew, 1.0])
    hd /= np.linalg.norm(hd)
    m.ell(head, sp["head"], sp.get("head_tag", sp["tag"]), 10, 6)
    m.cyl(head, head + hd * sp["snout"], sp["head"][0] * 0.8, sp["head"][0] * 0.55, sp.get("head_tag", sp["tag"]), 8, 1.02)
    m.ell(head + hd * sp["snout"], [sp["head"][0] * 0.55] * 3, "DARK", 6, 4, 0.8)
    for sx in (-1, 1):
        m.ell(head + [sx * sp["head"][0] * 0.85, sp["head"][1] * 0.25, sp["head"][2] * 0.2], [sp["head"][0] * 0.16] * 3,
              "DARK", 5, 4, 0.4)
        ear = head + [sx * sp["head"][0] * 0.9, sp["head"][1] * 0.7, -sp["head"][2] * 0.3]
        m.cyl(ear, ear + [sx * sp["head"][0] * 0.9, sp["head"][1] * 0.25, -sp["head"][2] * 0.2], sp["head"][0] * 0.3, 0.2,
              sp.get("head_tag", sp["tag"]), 5)
    if sp.get("antlers"):
        for sx in (-1, 1):
            a = head + [sx * sp["head"][0] * 0.4, sp["head"][1] * 0.8, -sp["head"][2] * 0.2]
            b = a + [sx * 3.2, 6.5, -3.2]
            c = b + [sx * 1.5, 5, -2.5]
            m.cyl(a, b, 0.7, 0.55, "WOOD", 5)
            m.cyl(b, c, 0.55, 0.3, "WOOD", 5)
            for f in (0.35, 0.7):
                p = a + (b - a) * f
                m.cyl(p, p + [sx * 0.6, 3.2, 2.0], 0.45, 0.2, "WOOD", 4)
            m.cyl(b, b + [sx * 2.5, 2.2, 1.0], 0.45, 0.2, "WOOD", 4)
    # tail
    tb = body_c + [0, sp["body"][1] * 0.2, -sp["body"][2]]
    m.cyl(tb, tb + [0, -sp["tail"] * 0.6, -sp["tail"] * 0.4], sp["leg_r"] * 0.9, 0.3, sp.get("tail_tag", sp["tag"]), 5)
    return m


QUADS = {
    "elk": (dict(tag="EARTH", body=[6.5, 7, 14], len=28, leg=15, leg_r=1.3, neck=10, neck_r=3.2, head=[2.6, 2.8, 3.4],
                 snout=4.5, tail=3, antlers=True, rump="WHITE", rump_tone=0.95, neck_tag="WOOD", leg_tag="EARTH"),
            {"EARTH": (140, 98, 62), "WHITE": (210, 186, 140), "WOOD": (96, 66, 46), "DARK": (46, 38, 34)}),
    "sheep": (dict(tag="WHITE", body=[5, 5, 8], len=16, leg=5.5, leg_r=0.8, neck=3.5, neck_r=1.8, head=[1.8, 2.0, 2.2],
                   snout=2.2, tail=2, wool=True, head_tag="DARK", leg_tag="DARK", neck_tag="WHITE", bounce=0.5),
              {"WHITE": (236, 232, 218), "DARK": (54, 50, 50)}),
}


# ---- reptiles ----------------------------------------------------------------------------------------------------------------

def reptile(kind, t, sp, name):
    m = M()
    ph = TAU * t
    n = sp["segs"]
    L = sp["len"]
    snake = name == "snake"
    pts, radii = [], []
    roll = 0.0
    if kind == "roll":
        roll = 180 * (0.5 - 0.5 * np.cos(np.pi * min(t * 1.4, 1.0))) * (1 if t < 0.72 else max(0.0, 1 - (t - 0.72) / 0.28))
    for i in range(n + 1):
        u = i / n
        z = (0.5 - u) * L
        amp = sp["wave"]
        if kind in ("walk",):
            x = np.sin(u * TAU * sp["waves"] - ph) * amp
        elif kind == "fidget" and snake:
            # a coil with the head raised and swaying
            ang = u * TAU * 1.3
            x, z = np.cos(ang) * L * 0.13 * (1 - u * 0.4), np.sin(ang) * L * 0.13 * (1 - u * 0.4)
        else:
            x = np.sin(u * TAU * sp["waves"] * 0.5 + 0.6) * amp * 0.4
        r = sp["r"] * (1.0 if u < 0.12 else (1 - (u - 0.12) * sp["taper"]))
        y = r
        if snake and kind == "fidget" and i <= 2:
            y = r + (3 - i) * sp["r"] * 1.6 + np.sin(ph * 2) * 0.5
            z, x = z + (2 - i) * 0.5, x + np.sin(ph) * 0.8 * (3 - i)
        if kind == "scratch":
            # the crocodile arches its back and rubs it on the ground
            y = r * (1 + 0.6 * np.sin(np.pi * u) * (0.5 + 0.5 * np.sin(ph * 2)))
        pts.append(np.array([x, y, z]))
        radii.append(r)
    tones = [1.0 if (k // max(1, sp.get("band", 1))) % 2 == 0 else sp.get("band_tone", 1.0) for k in range(n + 1)]
    tags = sp.get("tags")
    if tags:
        for i in range(n):
            tg = tags[(i // sp.get("band", 1)) % len(tags)]
            m.cyl(pts[i], pts[i + 1], radii[i], radii[i + 1], tg, 8, 1.0, caps=False)
            m.ell(pts[i], [radii[i]] * 3, tg, 8, 5)
    else:
        chain(m, pts, radii, sp["tag"], sides=8, tones=tones)
    head = pts[0]
    hd = pts[0] - pts[1]
    hd /= np.linalg.norm(hd)
    m.cyl(head, head + hd * sp["snout"], sp["r"] * sp.get("head_w", 0.9), sp["r"] * 0.4, sp["tag"], 8, 1.05)
    for sx in (-1, 1):
        side = np.cross(hd, [0, 1, 0])
        m.ell(head + side * sx * sp["r"] * 0.6 + [0, sp["r"] * 0.65, 0] + hd * sp["r"] * 0.3, [sp["r"] * 0.25] * 3,
              sp.get("eye", "YELLOW"), 5, 4)
    if snake:
        m.cyl(head + hd * sp["snout"], head + hd * (sp["snout"] + 1.6), 0.15, 0.1, "RED", 3)
    if sp.get("legs"):
        for k, u in enumerate((0.2, 0.55)):
            i = int(u * n)
            p = pts[i]
            d = pts[i] - pts[i + 1]
            d /= np.linalg.norm(d)
            side = np.cross(d, [0, 1, 0])
            for sx in (-1, 1):
                sw = np.sin(ph + k * np.pi + (sx > 0) * np.pi) * sp["r"] * 0.8 if kind == "walk" else 0.0
                foot = p + side * sx * sp["r"] * 2.0 + d * sw
                foot[1] = 0.2
                m.cyl(p + side * sx * sp["r"] * 0.6, foot, sp["r"] * 0.38, sp["r"] * 0.3, sp["tag"], 5, 0.9)
    if sp.get("ridges"):
        for i in range(2, n, 2):
            m.cyl(pts[i] + [0, radii[i] * 0.85, 0], pts[i] + [0, radii[i] * 1.25, 0], radii[i] * 0.25, 0.1, sp["tag"], 4, 0.8)
    if roll:
        R = rz(roll)
        for k in range(len(m.s.tris)):
            c0 = np.array([0, sp["r"], 0])
            m.s.tris[k] = (m.s.tris[k] - c0) @ R.T + c0
        m.s.balls = [((c - [0, sp["r"], 0]) @ R.T + [0, sp["r"], 0], r, col, tg) for c, r, col, tg in m.s.balls]
    return m


REPTILES = {
    "croc": (dict(tag="LEAF", segs=10, len=60, r=3.6, wave=2.5, waves=1.0, taper=0.95, snout=9, head_w=0.85, legs=True,
                  ridges=True, band=1, band_tone=0.9),
             {"LEAF": (88, 112, 62), "YELLOW": (220, 200, 90), "RED": (200, 60, 50)}),
    "gila": (dict(tag="RED", segs=9, len=34, r=2.6, wave=1.8, waves=1.0, taper=0.8, snout=3.2, head_w=1.0, legs=True,
                  tags=["RED", "DARK"], band=1, eye="DARK"),
             {"RED": (226, 120, 80), "DARK": (46, 38, 40)}),
    "snake": (dict(tag="LEAF", segs=14, len=46, r=1.3, wave=4.0, waves=1.5, taper=0.8, snout=2.2, head_w=1.3, band=2,
                   band_tone=0.8, eye="DARK"),
              {"LEAF": (120, 140, 70), "DARK": (40, 40, 36), "RED": (200, 50, 50)}),
}


def model(name, anim, t):
    """The scene of an animal sprite (file name without the clip, lower case; the clip), or None."""
    n = name.lower()
    kind, t = clip(anim, t)
    for key, (sp, pal) in BIRDS.items():
        if n == key:
            s = bird(kind, t, sp).s
            s.pal = pal
            return s
    for key, (sp, pal) in QUADS.items():
        if n == key:
            s = quadruped(kind, t, sp).s
            s.pal = pal
            return s
    for key, (sp, pal) in REPTILES.items():
        if n == key:
            s = reptile(kind, t, sp, key).s
            s.pal = pal
            return s
    return None
