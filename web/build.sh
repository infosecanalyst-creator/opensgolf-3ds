#!/bin/sh
# Builds the browser version into web/dist: the game compiled to WebAssembly (silent build for now), miniquad's gl.js loader
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
echo "web/dist is ready"
