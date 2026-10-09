#!/bin/sh
# Builds the browser version into web/dist: the game compiled to WebAssembly (sound mixed in Rust and played through Web Audio), miniquad's gl.js loader
# taken from the miniquad crate the game is built with, and this folder's page and file glue. Serve web/dist with any static
# web server (for example: python3 -m http.server -d web/dist) and open it in a browser.
set -e
cd "$(dirname "$0")/.."
cargo build -p simgolf --release --target wasm32-unknown-unknown --no-default-features
mkdir -p web/dist
cp target/wasm32-unknown-unknown/release/simgolf.wasm web/dist/
MQ=$(cargo metadata --format-version 1 | python3 -c 'import json,sys; m=json.load(sys.stdin); print([p["manifest_path"] for p in m["packages"] if p["name"]=="miniquad"][0])')
cp "$(dirname "$MQ")/js/gl.js" web/dist/
cp web/index.html web/simgolf.js web/dist/
# the player's own art and HD packs (docs/HD.md) must never end up in the published folder
if find web/dist -iname '*.png' -o -iname '*.pcx' -o -iname 'manifest.json' -o -iname 'HD' | grep -q .; then
    echo "web/dist contains game art or an HD pack: remove it before publishing" >&2
    exit 1
fi
echo "web/dist is ready"
