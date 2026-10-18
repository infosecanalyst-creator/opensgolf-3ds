#!/bin/sh
# Builds the browser version into web/dist: the game compiled to WebAssembly (sound mixed in Rust and played through Web Audio), miniquad's gl.js loader
# taken from the miniquad crate the game is built with, and this folder's page and file glue. Serve web/dist with any static
# web server (for example: python3 -m http.server -d web/dist) and open it in a browser.
set -e
cd "$(dirname "$0")/.."
cargo build -p opensgolf --release --target wasm32-unknown-unknown --no-default-features
rm -rf web/dist
mkdir -p web/dist
cp target/wasm32-unknown-unknown/release/opensgolf.wasm web/dist/
META=$(cargo metadata --format-version 1)
MQ=$(printf "%s" "$META" | python3 -c 'import json,sys; m=json.load(sys.stdin); print([p["manifest_path"] for p in m["packages"] if p["name"]=="miniquad"][0])')
VERSION=$(printf "%s" "$META" | python3 -c 'import json,sys; m=json.load(sys.stdin); print([p["version"] for p in m["packages"] if p["name"]=="opensgolf"][0])')
cp "$(dirname "$MQ")/js/gl.js" web/dist/
cp web/index.html web/files.js web/opensgolf.js web/start-bg.jpg web/dist/
# the page's version, and every file it loads named with the version and a hash of the published files in its address, so
# a browser (Safari keeps scripts for a while) never runs an older gl.js, page script or game with a newer page
python3 - "$VERSION" <<'PY'
import hashlib, sys
version = sys.argv[1]
d = "web/dist/"
h = hashlib.sha256()
for f in ["opensgolf.wasm", "gl.js", "files.js", "opensgolf.js", "index.html", "start-bg.jpg"]:
    h.update(open(d + f, "rb").read())
tag = version + "-" + h.hexdigest()[:10]
def stamp(name, pairs):
    s = open(d + name, encoding="utf-8").read()
    for old, new in pairs:
        if s.count(old) != 1:
            sys.exit("web/build.sh: %r is not in %s once" % (old, name))
        s = s.replace(old, new)
    open(d + name, "w", encoding="utf-8").write(s)
stamp("files.js", [('"%SG_VERSION%"', '"' + version + '"')])
stamp("opensgolf.js", [('load("opensgolf.wasm")', 'load("opensgolf.wasm?v=' + tag + '")')])
stamp("index.html", [('src="%s"' % f, 'src="%s?v=%s"' % (f, tag)) for f in ["gl.js", "files.js", "opensgolf.js"]] +
    [('url("start-bg.jpg")', 'url("start-bg.jpg?v=%s")' % tag)])
print("web/dist: version " + version + ", files tagged " + tag)
PY
# the player's own art and HD packs (docs/HD.md) must never end up in the published folder
if find web/dist -iname '*.png' -o -iname '*.pcx' -o -iname 'manifest.json' -o -iname 'HD' | grep -q .; then
    echo "web/dist contains game art or an HD pack: remove it before publishing" >&2
    exit 1
fi
# the page's own zip reading (web/files.js) against zips made by 7-Zip and Python in each method
if command -v node >/dev/null 2>&1; then
    node web/test.js
fi
echo "web/dist is ready"
