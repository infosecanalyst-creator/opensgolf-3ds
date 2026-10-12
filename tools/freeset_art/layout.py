"""The layout manifest of the free set (crates/sg-core/src/freeset/manifest.txt): the frame size, views, frames per view
and ground point of every sprite file, measured from the original by tools/freeset_layout.py (numbers only)."""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
MANIFEST = os.path.join(ROOT, "crates", "sg-core", "src", "freeset", "manifest.txt")

_cache = None


def sprites():
    """Sprite entries: path -> (w, h, views, fpv, ax, ay)."""
    global _cache
    if _cache is not None:
        return _cache
    out = {}
    for line in open(MANIFEST, encoding="utf-8"):
        if not line.startswith("F "):
            continue
        parts = line.split()
        n = parts[-9:]
        path = " ".join(parts[1:-9])
        w, h, views, fpv, cx, cy = (int(v) for v in n[:6])
        if views == 0:
            # no Firaxis extension: the engine puts the ground point at the middle of the bottom (sprites.rs)
            out[path] = (w, h, 0, max(fpv, 1), w // 2, h)
        else:
            out[path] = (w, h, views, max(fpv, 1), 240 - cx, 240 - cy)
    _cache = out
    return out


def lookup(path_lower):
    for p, spec in sprites().items():
        if p.lower() == path_lower:
            return p, spec
    return None, None
