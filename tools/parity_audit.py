#!/usr/bin/env python3
"""Compare every file on the owner's disc with the names the port source mentions and the names the exe decompile mentions.
Usage: tools/parity_audit.py GAME_DIR DECOMP_TEXT [OUT_MD]. Run after every batch of work; the gap list in docs/PARITY_AUDIT.md must shrink."""
import os, sys, collections
game, decomp = sys.argv[1], sys.argv[2]
out_md = sys.argv[3] if len(sys.argv) > 3 else "docs/PARITY_AUDIT.md"
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
src = ""
for d in ("tools", "src", "include"):
    for r, _, fs in os.walk(os.path.join(root, d)):
        for f in fs:
            if f.endswith((".cpp", ".h")): src += open(os.path.join(r, f), errors="ignore").read().lower() + "\n"
dec = open(decomp, errors="ignore").read().lower()
rows = collections.defaultdict(list)
for r, _, fs in os.walk(game):
    for f in fs:
        rel = os.path.relpath(os.path.join(r, f), game)
        top = rel.split(os.sep)[0] if os.sep in rel else "(root)"
        base = os.path.splitext(f)[0].lower().strip()
        rows[top].append((rel, base in src or f.lower() in src, base in dec or f.lower() in dec))
out = ["# Parity audit: disc assets versus the port\n", "Files the exe uses that the port never names are likely gaps. Pattern loaded sets (terrain tiles, FLC animations) need a manual look.\n"]
for top, l in sorted(rows.items()):
    u = sorted(x for x in l if x[2] and not x[1])
    out.append(f"## {top}: {len(l)} files, {sum(1 for x in l if x[1])} named in the port, {len(u)} used by the exe but never named in the port\n")
    out += [f"- {x[0]}" for x in u[:60]]
    if len(u) > 60: out.append(f"- ... {len(u) - 60} more")
    out.append("")
open(out_md, "w").write("\n".join(out))
print(sum(len([x for x in l if x[2] and not x[1]]) for l in rows.values()), "gaps")
