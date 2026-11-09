#!/bin/sh
# Test of the save self-check (crates/simgolf/src/selfcheck.rs, docs/PLAYING.md "Save self-check") on the free base set (no
# original game files needed), each run headless on a virtual display:
#   1. a scripted game saved twice (at a game tick inside the simulation and at the end): both saves read back to the
#      identical game, and no report is written,
#   2. the same with SG_SELFCHECK_BREAK=1 (the scratch copy's money changed): a report is written to Reports in the save
#      folder, naming the economy section and its cash field, with the save beside it,
#   3. seven such runs keep only the newest five reports,
#   4. the next start tells the player once on the ticker (SG_TEXTLOG) and notes it (told.txt); the start after says nothing.
# The logs stay in OUT_DIR.
#
# usage: tools/selfcheck_test.sh [OUT_DIR]   (run from the repository root after cargo build --release)
# env:   XVFB   the virtual display command (default "xvfb-run -a"); BIN  the folder of the opensgolf to run
set -u
OUT=${1:-selfcheck-test}
XVFB=${XVFB:-xvfb-run -a}
BIN=${BIN:-target/release}
rm -rf "$OUT"
mkdir -p "$OUT/save"
OPENSGOLF_SAVE_DIR="$PWD/$OUT/save"
SG_SELFCHECK=1
export OPENSGOLF_SAVE_DIR SG_SELFCHECK
REPORTS="$OUT/save/Reports"
fail=0
EDIT="t:16,30,0;t:32,24,1;p:20,29,2,0,1;p:24,28,2,0,1;p:28,26,2,0,1;o;h:2,24,27;h:3,22,26;y:36,20;j:0"

# run NAME TIME [ARGS...]: one headless run of the scripted game, saved at the end
run() {
    name=$1
    time=$2
    shift 2
    # shellcheck disable=SC2086
    SG_TEXTLOG="$OUT/$name.text" $XVFB $BIN/opensgolf --game "$OUT/no-game-folder" --mute --size 800x600 \
        --png "$OUT/$name.png" --property 0 --clock 4242 --edit "$EDIT" --time "$time" --save-game "$OUT/$name.sgs" "$@" \
        > "$OUT/$name.log" 2>&1
    code=$?
    if [ $code -ne 0 ]; then
        echo "FAIL $name: exit $code"
        tail -5 "$OUT/$name.log"
        fail=1
    fi
}

ok() {
    echo "ok   $1"
}
bad() {
    echo "FAIL $1"
    fail=1
}

# 1. two saves, both read back identical
SG_SAVE_AT="600,$OUT/plain-600.sgs" run plain 120
n=$(grep -c "^selfcheck: ok" "$OUT/plain.log")
if [ "$n" -eq 2 ] && [ ! -d "$REPORTS" ]; then
    ok "two saves read back to the identical game, no report ($(grep "^selfcheck: ok" "$OUT/plain.log" | tail -1))"
else
    bad "plain saves: $n checks passed, reports: $(ls "$REPORTS" 2>/dev/null | tr '\n' ' ')"
    grep "^selfcheck" "$OUT/plain.log"
fi

# 2. a difference forced in the scratch copy is reported
SG_SELFCHECK_BREAK=1 run broken-1 5
txt=$(ls "$REPORTS"/selfcheck-*.txt 2>/dev/null | tail -1)
if [ -n "$txt" ] && grep -q "^differ: econ econ.cash$" "$txt" && [ -s "${txt%.txt}.sgs" ] && grep -q "^version: " "$txt"; then
    ok "a forced difference is reported ($(basename "$txt") and its save)"
else
    bad "no report of the forced difference"
fi
# the report holds no folder of the computer
if [ -n "$txt" ] && grep -q "$HOME\|$PWD" "$txt"; then
    bad "the report names a folder"
fi

# 3. the newest five are kept
for i in 2 3 4 5 6 7; do
    sleep 1
    SG_SELFCHECK_BREAK=1 run "broken-$i" 5
done
n=$(ls "$REPORTS"/selfcheck-*.txt | wc -l)
m=$(ls "$REPORTS"/selfcheck-*.sgs | wc -l)
if [ "$n" -eq 5 ] && [ "$m" -eq 5 ]; then
    ok "seven reports, the newest five kept"
else
    bad "$n report texts and $m saves kept, not 5"
fi

# 4. told once at the next start, then not again (broken-7's start told of broken-6's report: start from a fresh note)
rm -f "$REPORTS/told.txt"
run told 5
run quiet 5
newest=$(ls "$REPORTS"/selfcheck-*.txt | tail -1)
newest=$(basename "$newest" .txt)
if grep -q "found a problem with a saved game" "$OUT/told.text" && [ "$(cat "$REPORTS/told.txt")" = "$newest" ]; then
    ok "the next start tells the player once"
else
    bad "the next start did not tell the player"
fi
if grep -q "found a problem with a saved game" "$OUT/quiet.text"; then
    bad "the player was told twice"
else
    ok "the start after says nothing"
fi

if [ $fail -ne 0 ]; then
    echo "self-check test: FAILED (logs in $OUT)"
    exit 1
fi
echo "self-check test: passed (logs in $OUT)"
