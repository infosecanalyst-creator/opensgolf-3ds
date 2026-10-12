"""The free set's sprite palette layout (the same as crates/sg-core/src/freeset/sprite.rs): colour ramps per material.
Entries 0..96 are the golfer slots the engine recolours (shirt, trousers, skin, hat, hair or hands), 255 is the key.
Sprites the engine draws in another palette (tree and flower colour variants, golfers) use these ramps, so a palette file
changes their colours the way it changes the run-time drawn ones."""
import numpy as np

MATS = {
    "SHIRT": (0, 20), "PANTS": (20, 20), "SKIN": (40, 20), "HAT": (60, 20), "ALT": (80, 10), "HANDS_F": (90, 7),
    "LEAF": (97, 16), "BARK": (113, 8), "WALL": (121, 16), "ROOF": (137, 16), "STONE": (153, 12), "WATER": (165, 12),
    "EARTH": (177, 8), "DARK": (185, 8), "WHITE": (193, 8), "RED": (201, 8), "YELLOW": (209, 8), "PURPLE": (217, 8),
    "WOOD": (225, 8), "GOLD": (233, 8), "SHOE": (241, 8),
}
NAMES = list(MATS)
ID = {n: i for i, n in enumerate(NAMES)}

# the run-time palette's own colours (sprite.rs palette(0, false, 0)) for the ramps a sprite does not set
DEFAULT = {
    "SHIRT": (60, 110, 200), "PANTS": (200, 180, 130), "SKIN": (240, 200, 165), "HAT": (235, 235, 235),
    "ALT": (240, 200, 165), "HANDS_F": (240, 200, 165), "LEAF": (70, 140, 55), "BARK": (115, 80, 50),
    "WALL": (235, 225, 200), "ROOF": (170, 70, 55), "STONE": (150, 148, 140), "WATER": (70, 130, 190),
    "EARTH": (150, 115, 75), "DARK": (60, 70, 85), "WHITE": (240, 240, 235), "RED": (210, 50, 55),
    "YELLOW": (245, 210, 60), "PURPLE": (140, 90, 200), "WOOD": (160, 115, 70), "GOLD": (220, 180, 70),
    "SHOE": (70, 60, 55),
}


def ramp_k(t):
    return -0.62 * (1 - t / 0.6) if t < 0.6 else (t - 0.6) / 0.4 * 0.32


def shade(c, k):
    c = np.asarray(c, float)
    return c + (255 - c) * k if k >= 0 else c * (1 + k)


def palette(colours=None):
    """A 256 x 3 palette: every ramp from its colour (`colours` overrides DEFAULT by material name)."""
    cols = dict(DEFAULT)
    cols.update(colours or {})
    p = np.zeros((256, 3), dtype=np.uint8)
    for name, (base, n) in MATS.items():
        for i in range(n):
            t = i / max(n - 1, 1)
            p[base + i] = np.clip(shade(cols[name], ramp_k(t)), 0, 255).astype(np.uint8)
    p[255] = (255, 0, 255)
    return p


def level(bright):
    """The ramp position 0..1 of a rendered brightness (1 = the material's own colour, which sits at 0.6 of its ramp)."""
    b = np.asarray(bright, float)
    dark = 0.6 * (1 + (b - 1) / 0.62)
    light = 0.6 + 0.4 * np.minimum((b - 1) / 0.32, 1)
    return np.clip(np.where(b < 1, dark, light), 0, 1)


def index(tag, bright):
    """Palette indices for material ids and brightness (arrays); -1 tags give the key."""
    tag = np.asarray(tag)
    out = np.full(tag.shape, 255, dtype=np.uint8)
    lv = level(bright)
    for i, name in enumerate(NAMES):
        m = tag == i
        if m.any():
            base, n = MATS[name]
            out[m] = (base + np.round(lv[m] * (n - 1))).astype(np.uint8)
    return out
