#!/bin/sh
# Writes the OpenSGolf application icon (our own drawing: sg_core::freeset::app_icon) for the packages:
#   crates/simgolf/android/res/mipmap-*/ic_launcher.png   the Android launcher icon (committed)
#   $1 (default target/icons)                              icon_<size>.png at every size, for the desktop packages
set -e
cd "$(dirname "$0")/.."
out="${1:-target/icons}"
cargo run --release -q -p sgtool -- icon "$out"
for d in mdpi:48 hdpi:72 xhdpi:96 xxhdpi:144 xxxhdpi:192; do
    mkdir -p "crates/simgolf/android/res/mipmap-${d%%:*}"
    cp "$out/icon_${d##*:}.png" "crates/simgolf/android/res/mipmap-${d%%:*}/ic_launcher.png"
done
echo "icons in $out and crates/simgolf/android/res"
