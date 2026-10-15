#!/usr/bin/env python3
"""Writes the course editor's sample heightmaps (docs/PLAYING.md, "Course editor"): valley.png, ridge.png and island.png,
64 x 64 grey PNGs made from simple formulas (original work, CC0). Run it from anywhere; the pictures go next to it.

    python3 tools/heightmaps/make_heightmaps.py

Only the standard library is used, so the output is the same bytes everywhere."""
import math
import os
import struct
import zlib

SIZE = 64


def ripple(x, y):
    """Gentle bumps so the land is not too smooth: a few fixed waves."""
    return (math.sin(x * 0.31 + 1.3) * math.cos(y * 0.27 + 0.4) * 0.5
            + math.sin(x * 0.11 - y * 0.17 + 2.0) * 0.5) * 0.5


def valley(x, y):
    # a river valley from the top-left to the bottom-right, the sides rising to rolling hills
    u, v = x / (SIZE - 1), y / (SIZE - 1)
    d = abs((u - v) + 0.12 * math.sin(u * 6.0)) / math.sqrt(2)
    side = 1.0 - math.exp(-(d / 0.22) ** 2)
    return 0.05 + 0.8 * side + 0.08 * ripple(x, y)


def ridge(x, y):
    # a ridge across the middle, falling away to both sides, higher towards one end
    u, v = x / (SIZE - 1), y / (SIZE - 1)
    d = abs(v - 0.5 - 0.08 * math.sin(u * 5.0))
    crest = math.exp(-(d / 0.2) ** 2) * (0.6 + 0.4 * u)
    return 0.1 + 0.8 * crest + 0.06 * ripple(x, y)


def island(x, y):
    # an island: high in the middle, beaches all round, sea at the edges
    u, v = x / (SIZE - 1) - 0.5, y / (SIZE - 1) - 0.5
    r0 = math.hypot(u, v * 1.15)
    a = math.atan2(v, u)
    # a wavy shore, the waves fading towards the middle
    r = r0 + (0.025 * math.sin(a * 3.0 + 0.5) + 0.015 * math.sin(a * 7.0)) * min(1.0, r0 / 0.25)
    land = max(0.0, 1.0 - (r / 0.42) ** 2)
    return 0.9 * land ** 0.8 + 0.06 * ripple(x, y) * land


def write_png(path, f):
    rows = b""
    for y in range(SIZE):
        rows += b"\0" + bytes(max(0, min(255, round(f(x, y) * 255))) for x in range(SIZE))

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 0, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")
    with open(path, "wb") as out:
        out.write(data)


here = os.path.dirname(os.path.abspath(__file__))
for name, f in (("valley", valley), ("ridge", ridge), ("island", island)):
    write_png(os.path.join(here, name + ".png"), f)
    print("wrote", os.path.join(here, name + ".png"))
