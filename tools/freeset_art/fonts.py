#!/usr/bin/env python3
"""The free set's two fonts (SIL Open Font License 1.1): Fredoka SemiBold (the headings, KLEPTO__.TTF) and Nunito ExtraBold
(the text, manu3_.TTF), made from the variable fonts of github.com/google/fonts: the weight instance, Latin-1 and a little
punctuation, and Liberation Sans' vertical metrics so text keeps its baseline and line height in the engine's boxes. Writes
crates/simgolf/fonts/*.ttf. Needs fonttools.

usage: fonts.py DIR      (DIR: an empty folder for the downloads)"""
import os
import sys
import urllib.request

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "..", "crates", "simgolf", "fonts")
BASE = "https://raw.githubusercontent.com/google/fonts/main/ofl/"
FONTS = [("fredoka/Fredoka%5Bwdth,wght%5D.ttf", {"wght": 600, "wdth": 100}, "Fredoka-SemiBold.ttf"),
         ("nunito/Nunito%5Bwght%5D.ttf", {"wght": 800}, "Nunito-ExtraBold.ttf")]


def make(src, axes, out):
    t = instancer.instantiateVariableFont(TTFont(src), axes, updateFontNames=True)
    o, h = t["OS/2"], t["hhea"]
    o.usWinAscent, o.usWinDescent = 905, 212
    h.ascent, h.descent, h.lineGap = 905, -212, 33
    o.sTypoAscender, o.sTypoDescender, o.sTypoLineGap = 728, -210, 150
    opts = subset.Options()
    opts.layout_features = ["kern", "liga"]
    opts.name_IDs = ["*"]
    opts.name_languages = ["*"]
    opts.notdef_outline = True
    opts.hinting = False
    s = subset.Subsetter(opts)
    s.populate(unicodes=list(range(0x20, 0x7f)) + list(range(0xa0, 0x180)) +
               [0x2013, 0x2014, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2026, 0x20ac, 0x2122, 0x2190, 0x2191, 0x2192, 0x2193, 0x2212])
    s.subset(t)
    t.save(out)


def main():
    d = sys.argv[1]
    for rel, axes, name in FONTS:
        src = os.path.join(d, name + ".src")
        urllib.request.urlretrieve(BASE + rel, src)
        make(src, axes, os.path.join(OUT, name))
        print("ok", name)


if __name__ == "__main__":
    main()
