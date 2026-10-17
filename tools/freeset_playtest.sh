#!/bin/sh
# Headless play test of OpenSGolf on its free base set (opensgolf-free), with no original game files at all:
#   1. writes the free set to a folder and decodes every file with the engine's own decoders (sgtool check),
#   2. starts a new game, builds a hole and hires a groundskeeper, lets golfers play for about three in-game months and saves,
#   3. loads the save and opens every report screen,
#   4. starts the example scenario with the example script pack on, saves, and opens its goals report,
#   5. makes a scenario in the course editor on a sample heightmap, loads it back into the editor and saves it again (the
#      files must not change), then plays it,
# each step on a virtual display, failing on a crash, a non-zero exit or any missing-file or decode error in the output.
#
# usage: tools/freeset_playtest.sh [OUT_DIR]      (run from the repository root after cargo build --release)
# env:   XVFB   the virtual display command (default "xvfb-run -a"); TIME  seconds of play to simulate (default 300)
set -u
OUT=${1:-freeset-test}
XVFB=${XVFB:-xvfb-run -a}
TIME=${TIME:-300}
BIN=target/release
rm -rf "$OUT"
mkdir -p "$OUT/save"
OPENSGOLF_SAVE_DIR="$PWD/$OUT/save"
export OPENSGOLF_SAVE_DIR
fail=0

step() {
    name=$1
    shift
    # shellcheck disable=SC2086
    $XVFB $BIN/opensgolf --game "$OUT/no-game-folder" --mute --size 800x600 --png "$OUT/$name.png" "$@" > "$OUT/$name.log" 2>&1
    code=$?
    if [ $code -ne 0 ] || [ ! -s "$OUT/$name.png" ]; then
        echo "FAIL $name: exit $code"
        tail -5 "$OUT/$name.log"
        fail=1
    elif grep -E -i "panick|error|cannot read|cannot load|could not|missing|no sounds|sprite:" "$OUT/$name.log" > "$OUT/$name.problems"; then
        echo "FAIL $name:"
        head -5 "$OUT/$name.problems"
        fail=1
    else
        echo "ok   $name"
    fi
}

echo "== the free set, decoded"
$BIN/sgtool freeset "$OUT/set" || exit 1
$BIN/sgtool check "$OUT/set" > "$OUT/check.log"
if grep -q "^FAIL" "$OUT/check.log"; then
    grep "^FAIL" "$OUT/check.log" | head
    fail=1
else
    sed 's/^/     /' "$OUT/check.log"
fi

echo "== a new game: a hole, a groundskeeper, $TIME seconds of play, saved"
# the property's land with --clock 4242: clubhouse door at 25,16; tee at 16,30, green at 32,24, fairway between
step newgame --property 0 --clock 4242 \
    --edit "t:16,30,0;t:32,24,1;p:20,29,2,0,1;p:24,28,2,0,1;p:28,26,2,0,1;o;h:2,24,27" \
    --time "$TIME" --save-game "$OUT/save/playtest.sgs"
grep -E "open hole|hired|saved game" "$OUT/newgame.log" | sed 's/^/     /'
if [ ! -s "$OUT/save/playtest.sgs" ]; then
    echo "FAIL: the game was not saved"
    exit 1
fi

echo "== the save loaded, every report"
for s in play report comments histograph finance routing shortcuts roster board yearend holestats golfers land content; do
    step "load-$s" --load-game "$OUT/save/playtest.sgs" --screen "$s" --time 2
done
echo "== the title screens"
for s in menu property difficulty load themes credits pickapro champ; do
    step "title-$s" --screen "$s" --time 2
done

echo "== the example scenario and script packs"
mkdir -p "$OUT/save/packs"
cp -r packs/example-scenario-desert-challenge packs/example-script-bonus "$OUT/save/packs/"
printf '{"content": {"mods": [{"id": "example-script-bonus", "enabled": true}]}}\n' > "$OUT/save/settings.json"
step scenario-start --screen scenarios --time 2
step scenario-play --scenario example-scenario-desert-challenge --time 60 --save-game "$OUT/save/scenario.sgs"
grep -E "scenario|scripts:" "$OUT/scenario-play.log" | sed 's/^/     /'
step scenario-goals --load-game "$OUT/save/scenario.sgs" --screen goals --time 2

echo "== the course editor: a scenario made on a heightmap, saved, loaded back unchanged, and played"
# a minute in the editor: no time passes, no golfers come, and building and painting cost nothing (the cash stays 75000)
step editor-make --editor --property 0 --clock 4242 --heightmap tools/heightmaps/valley.png \
    --editor-set "name=Valley Test;author=CI;cash=75000;difficulty=1;water=2;goal=holes:1@2003;goal=members:5;tract=1:own" \
    --edit "t:16,30,0;t:32,24,1;p:20,29,2,0,1;p:24,28,2,0,1;p:28,26,2,0,1;o;B:40,40,44,44,2" \
    --save-scenario valley-test --screen editor --time 60 --save-game "$OUT/editor.sgs"
grep -E "editor|saved" "$OUT/editor-make.log" | sed 's/^/     /'
if ! grep -q "scenario valley-test saved: cash 75000, game tick 0" "$OUT/editor-make.log" ||
    ! grep -q "editor.sgs at tick 0, cash 75000" "$OUT/editor-make.log"; then
    echo "FAIL editor-make: the editor charged money or ran the game"
    fail=1
fi
cp -r "$OUT/save/packs/valley-test" "$OUT/valley-test-first"
step editor-load --edit-scenario valley-test --save-scenario valley-test --time 2
for f in pack.toml scenario.toml course.sgs; do
    if ! cmp -s "$OUT/valley-test-first/$f" "$OUT/save/packs/valley-test/$f"; then
        echo "FAIL editor-load: $f changed when the scenario was loaded into the editor and saved again"
        fail=1
    fi
done
step editor-play --scenario valley-test --time 30 --save-game "$OUT/save/valley.sgs"
grep -E "scenario|course:" "$OUT/editor-play.log" | sed 's/^/     /'
step editor-goals --load-game "$OUT/save/valley.sgs" --screen goals --time 2

if [ $fail -ne 0 ]; then
    echo "free set play test: FAILED (logs and stills in $OUT)"
    exit 1
fi
echo "free set play test: passed (stills in $OUT)"
