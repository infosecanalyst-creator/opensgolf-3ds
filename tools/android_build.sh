#!/bin/sh
# Builds the OpenSGolf Android app (an .apk) with cargo-quad-apk. Run it inside the notfl3/cargo-apk docker image, which has
# the Android SDK, the NDK and cargo-quad-apk:
#
#   docker run --rm -v "$(pwd)":/root/src -w /root/src notfl3/cargo-apk tools/android_build.sh [release|debug] [OUT_DIR]
#
# Out: OUT_DIR/opensgolf-<profile>.apk (default OUT_DIR: target/android), for arm64, 32-bit arm and x86_64 phones, signed
# with a debug key (cargo-quad-apk makes one), which is all Android needs to sideload it.
#
# cargo-quad-apk reads the manifests with the Cargo of Rust 1.61 (it is built on the cargo crate 0.62), which knows neither
# workspace inheritance (`version.workspace = true`), version 4 lock files nor crates of the 2024 edition. So the build
# runs on a copy of the source whose manifests spell the inherited fields out, with a lock file of its own that picks
# dependency versions made for Rust 1.84 or older (Cargo's rust-version aware resolver, from a current toolchain the
# script installs with rustup; that toolchain's rustc also compiles the game). The tree itself stays as it is.
set -e
cd "$(dirname "$0")/.."
src="$(pwd)"
profile="${1:-release}"
out="${2:-$src/target/android}"
work="${ANDROID_BUILD_DIR:-/tmp/opensgolf-android}"
targets="aarch64-linux-android armv7-linux-androideabi x86_64-linux-android"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
edition=$(sed -n 's/^edition = "\(.*\)"/\1/p' Cargo.toml | head -1)
license=$(sed -n 's/^license = "\(.*\)"/\1/p' Cargo.toml | head -1)

# a current Rust (the image's is older than some of the game's dependencies need); ANDROID_TARGETS builds fewer targets,
# SKIP_RUSTUP=1 uses a toolchain that is already there (a quick local build)
targets="${ANDROID_TARGETS:-$targets}"
if [ -z "$SKIP_RUSTUP" ]; then
    rustup toolchain install stable --profile minimal $(for t in $targets; do printf ' --target %s' "$t"; done)
    rustup default stable
fi

mkdir -p "$work"
# the source, without build output, the old C++ port or the web page (target/ is kept between runs)
tar cf - --exclude=./target --exclude=./.git --exclude=./legacy --exclude=./web --exclude=./build \
    --exclude=./game --exclude=./.claude . | (cd "$work" && tar xf -)
cd "$work"
for m in crates/*/Cargo.toml; do
    sed -i -e "s/^version.workspace = true/version = \"$version\"\nrust-version = \"1.84\"/" \
        -e "s/^edition.workspace = true/edition = \"$edition\"/" -e "s/^license.workspace = true/license = \"$license\"/" "$m"
done
if [ -n "$ANDROID_TARGETS" ]; then
    list=$(for t in $targets; do printf '"%s", ' "$t"; done)
    sed -i "s/^build_targets = .*/build_targets = [${list%, }]/" crates/simgolf/Cargo.toml
fi
rm -f Cargo.lock
CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback cargo generate-lockfile
# the wasm-bindgen family (cpal's browser side, not built here) as of 2024: the newer ones declare features the old Cargo
# cannot resolve (js-sys's `dep:futures-util`)
cargo update -p web-sys --precise 0.3.70
cargo update -p wasm-bindgen-futures --precise 0.4.43
cargo update -p js-sys --precise 0.3.70
cargo update -p wasm-bindgen --precise 0.2.93
# a version 3 lock file says the same for crates.io dependencies
sed -i 's/^version = 4$/version = 3/' Cargo.lock

# Sound goes through cpal's oboe output, whose C++ (oboe-sys) is compiled with the NDK. Should that fail, the app is built
# silent rather than not at all (TODO: drop the fallback once a CI build with sound has been tried on a phone).
rel=""
if [ "$profile" = release ]; then rel="--release"; fi
if ! cargo quad-apk build $rel -p opensgolf; then
    echo "warning: the build with sound failed; building the app without sound" >&2
    cargo quad-apk build $rel -p opensgolf --no-default-features
fi
mkdir -p "$out"
apk=$(find "target/android-artifacts/$profile/apk" -name '*.apk' | head -1)
cp "$apk" "$out/opensgolf-$profile.apk"
echo "built $out/opensgolf-$profile.apk"
