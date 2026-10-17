"""Which model draws which sprite file. A recipe gives the scene of each frame (in world units, the ground point at the
origin) and how it is fitted to the frame; build.py renders it."""
import re

import numpy as np

import animals
import buildings
import landmarks
import props
import layout
import people
import trees
from scene import Scene, fit_scale


class Recipe:
    ss = 4
    shrink = 1.0
    # a darker rim keeps the silhouette crisp against the grass
    edge_dark = 0.2
    # 'ramp': drawn in the palette layout of mats.py (for sprites the engine recolours); 'free': a palette of its own
    mode = "ramp"
    # True: the sprite gets a frame of its own size (build.py), drawn at the world's scale
    auto_frame = False

    def fixed_scale(self):
        """A scale set by the recipe (None: fit the frame)."""
        return None

    def fit_frames(self, fpv):
        return [fpv - 1]

    def yaw(self, view_deg):
        return view_deg

    def scene(self, rng, f, fpv, view_deg):
        raise NotImplementedError


class Tree(Recipe):
    """A tree growing over its frames (the original's 16 frames: seedling to full crown)."""

    def __init__(self, name):
        self.name = name

    def scene(self, rng, f, fpv, view_deg):
        # the scenic elm's frames are not a growth cycle (it has no views either): always the full tree
        size, leaf = (1.0, 1.0) if "scenicelm" in self.name else trees.growth(f, fpv)
        s = trees.species(self.name, rng, leaf)
        s = trees.gags(self.name, s, leaf)
        return s.transformed(size)


class Building(Recipe):
    """One layer of a building: the body, the flat base (paving, lawn) or the animated overlay (empty for now). The body
    and base of a family share one scale, the largest at which both fit their frames (and never above the world's)."""

    mode = "free"
    edge_dark = 0.1
    ss = 3
    auto_frame = True

    def __init__(self, path, build, layer, family, size=1.0):
        self.path, self.build, self.layer, self.family, self.size = path, build, layer, family, size

    def parts(self, rng):
        return self.build(buildings.theme_style(self.path), rng)

    def scene(self, rng, f, fpv, view_deg):
        body, base = self.parts(np.random.default_rng(7))
        body.mass = None
        body.ambient, body.diffuse = 0.56, 0.68
        base.ambient, base.diffuse = 0.7, 0.45
        if self.layer == "body":
            return body
        if self.layer == "base":
            base.no_shadow = True
            return base
        s = Scene()
        s.no_shadow = True
        return s

    def fixed_scale(self):
        return self.size


def layer_of(name):
    if name.endswith("_anim") or name.endswith("anim") or "anim" in name:
        return "anim"
    if name.endswith("_base") or name.endswith("base") or name.endswith("_dirt"):
        return "base"
    return "body"


def family_of(low):
    """The body, base and anim files of the building `low` belongs to, by layer."""
    stem = low.rsplit("/", 1)
    folder, name = stem[0], stem[1][:-4]
    core = re.sub(r"(_anim|anim|_base|base|_dirt)$", "", name)
    out = {}
    for p in layout.sprites():
        pl = p.lower()
        if not pl.startswith(folder + "/") or pl.endswith("shadow.flc"):
            continue
        n = pl.rsplit("/", 1)[1][:-4]
        if re.sub(r"(_anim|anim|_base|base|_dirt)$", "", n) == core:
            out[layer_of(n)] = p
    return out


class Person(Recipe):
    """A golfer clip: the figure posed for the animation over its frames and turned for its 8 views, at the world's scale
    (every clip of a body the same size)."""

    edge_dark = 0.24

    def __init__(self, prefix, anim_name):
        self.prefix, self.anim = prefix, anim_name

    def fixed_scale(self):
        return 1.0

    def yaw(self, view_deg):
        # view v heads 180 - 45 v degrees in the world (docs/SPRITES.md); the figure faces its +z
        return view_deg - 90.0

    def scene(self, rng, f, fpv, view_deg):
        loop = "walk" in self.anim or "stride" in self.anim or self.anim in ("sq", "fidget", "sitting", "cart") or \
            self.prefix == "_staff"
        t = f / fpv if loop else (f / max(fpv - 1, 1))
        return people.person_scene(self.prefix, self.anim, t, view_deg)


class Landmark(Recipe):
    """A landmark (landmarks.py), in the palette ramps its palette file colours; animated ones fitted over all frames."""

    def __init__(self, name):
        self.name = name

    def fit_frames(self, fpv):
        return list(range(fpv))

    def scene(self, rng, f, fpv, view_deg):
        return landmarks.model(self.name, f / max(fpv, 1))


class Animal(Recipe):
    """An animal clip (animals.py), turned for its views like the golfers, fitted over all its frames."""

    edge_dark = 0.24

    def __init__(self, species, anim):
        self.species, self.anim = species, anim

    _scales = {}

    def fixed_scale(self):
        """One scale for the clips of a species (the median of the scales at which each fits its frame), so an animal keeps
        its size from clip to clip; a clip whose frame is tighter than that (some frames sit oddly round their ground point)
        is drawn smaller."""
        if self.species not in Animal._scales:
            fits = {}
            for p, (w, h, views, fpv, ax, ay) in layout.sprites().items():
                low = p.lower()
                if "/animals/" not in low or low.endswith("shadow.flc"):
                    continue
                n = low.rsplit("/", 1)[1][:-4]
                sp, _, an = n.partition("_")
                if sp != self.species:
                    continue
                r = Animal(sp, an)
                nv = max(views, 1)
                step = 45.0 if nv >= 8 else 90.0
                scenes = [(r.scene(None, f, fpv, v * step), r.yaw(v * step)) for v in range(nv) for f in range(0, fpv, 2)]
                fits[an] = fit_scale(scenes, w, h, ax, ay)
            Animal._scales[self.species] = (float(np.median(list(fits.values()))), fits)
        ref, fits = Animal._scales[self.species]
        return min(ref, fits.get(self.anim, ref)) * 0.97

    def yaw(self, view_deg):
        return view_deg - 90.0

    def scene(self, rng, f, fpv, view_deg):
        loop = any(k in self.anim for k in ("walk", "run", "loop", "sq"))
        t = f / fpv if loop else f / max(fpv - 1, 1)
        return animals.model(self.species, self.anim, t)


class Prop(Recipe):
    """A small thing (props.py): flags, markers, benches, flower beds, signs, the ball washer, boats and bridges. Beds and
    bridges are drawn at the world's scale (they join tile to tile), the rest fitted to their frame over all frames."""

    def __init__(self, path):
        self.path = path

    def fit_frames(self, fpv):
        return list(range(0, fpv, max(1, fpv // 6)))

    def fixed_scale(self):
        return 1.0 if props.prop(self.path, 0.0)[2] == "world" else None

    def yaw(self, view_deg):
        # a flag flies with the wind, to the right on the screen in every view (its frame sits round the pin's foot)
        if "flag" in self.path.lower():
            return -45.0
        return view_deg

    def scene(self, rng, f, fpv, view_deg):
        loop = not any(k in self.path.lower() for k in ("_pop", "flowers_", "sin", "tropfb", "_single", "side", "corner"))
        t = f / fpv if loop else f / max(fpv - 1, 1)
        m, pal, _ = props.prop(self.path, t)
        m.s.pal = pal
        return m.s


TREE_DIRS = ("flics/trees/", "flics/scenic/")


def recipe(path, name):
    low = path.lower().replace("\\", "/")
    for entry in buildings.FAMILIES:
        rx, build = entry[0], entry[1]
        if re.search(rx, low):
            return Building(path, build, layer_of(name), family_of(low), entry[2] if len(entry) > 2 else 1.0)
    if "/flics/employee/" in "/" + low or "/flics/celebs/" in "/" + low:
        if people.staff_or_celeb(name) is not None:
            return Person("_staff", name)
    if "/flics/male/" in "/" + low or "/flics/female/" in "/" + low:
        if "_" in name and not name.endswith("pal"):
            prefix, anim_name = name.split("_", 1)
            return Person(prefix, anim_name)
    if props.prop(path, 0.0) is not None:
        return Prop(path)
    if "/flics/animals/" in "/" + low and "_" in name:
        sp, an = name.split("_", 1)
        if animals.model(sp, an, 0.0) is not None:
            return Animal(sp, an)
    if "/flics/landmarks/" in "/" + low and name in landmarks.LANDMARKS:
        return Landmark(name)
    if any(d in low for d in TREE_DIRS):
        import numpy as np
        if trees.species(name, np.random.default_rng(0), 1.0) is not None:
            return Tree(name)
    return None
