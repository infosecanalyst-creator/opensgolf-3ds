#!/bin/sh
# Determinism and save fidelity check of OpenSGolf on its free base set (no original game files needed), with the
# simulation fingerprint (SG_FINGERPRINT, docs/PLAYING.md):
#   1. twin runs: the same scripted game twice (a hole, a groundskeeper and a soda vendor, golfers playing for TIME
#      seconds, a celebrity's home, the pro's practice round); the two runs' fingerprints, one per game tick, must be identical,
#   2. save and reload: the same game saved at a game tick a fifth of the way and halfway (SG_SAVE_AT), loaded and played
#      on; from the save on, every game tick's state must be the one the uninterrupted run had,
#   3. a hole being built: the game with a second hole's tee and cup laid but the hole not opened, saved at the same game
#      ticks; loading plans that hole again, as the original's load does (docs/DECODE_SAVE.md section 1), so the loaded game
#      is not compared with the uninterrupted run but two loads of the save must play on identically (the plan made again
#      is the same each time), and the save self-check must say it skipped that save,
#   4. with --extras: 1 and 2 again with each gameplay extra (sg_core::extras) on in turn (the --extras test hook's bits),
#   5. the save self-check (SG_SELFCHECK=1, selfcheck.rs): each save at a game tick must read back, in memory, to the game
#      it was saved from (tools/selfcheck_test.sh tests its report),
#   6. a replay (replay.rs): the scripted game's action log (SG_REPLAY_OUT) played again with --replay in a window of
#      another size must have every game tick's fingerprint the first run had, and match at every hash in its log.
# Each run is on a virtual display; the logs and fingerprint files stay in OUT_DIR. A difference names the first game
# tick the runs part at; SG_FINGERPRINT_PARTS=2 (set for every run) gives each line's sections and fields, which say what
# differs there.
#
# usage: tools/determinism_check.sh [OUT_DIR] [--extras]   (run from the repository root after cargo build --release)
# env:   XVFB   the virtual display command (default "xvfb-run -a"); TIME  seconds of play to simulate (default 300);
#        BITS   the extras' bits to try with --extras (default: each one); BIN  the folder of the opensgolf to run
set -u
OUT=${1:-determinism-test}
EXTRAS=${2:-}
XVFB=${XVFB:-xvfb-run -a}
TIME=${TIME:-300}
BIN=${BIN:-target/release}
rm -rf "$OUT"
mkdir -p "$OUT/save"
OPENSGOLF_SAVE_DIR="$PWD/$OUT/save"
SG_FINGERPRINT_PARTS=2
SG_SELFCHECK=1
export OPENSGOLF_SAVE_DIR SG_FINGERPRINT_PARTS SG_SELFCHECK
fail=0
# the property's land with --clock 4242 (as tools/freeset_playtest.sh): a hole from the tee at 16,30 to the green at
# 32,24, a groundskeeper and a soda vendor posted by it, a home site with a celebrity living there, and the pro's
# practice round
EDIT="t:16,30,0;t:32,24,1;p:20,29,2,0,1;p:24,28,2,0,1;p:28,26,2,0,1;o;h:2,24,27;h:3,22,26;y:36,20;j:0"
# the game ticks the game is saved at and reloaded from: a fifth of the way through (the pro's round under way) and
# halfway (one game tick is 87 ms)
SAVES=$(awk -v t="$TIME" 'BEGIN { print int(t * 1000 / 5 / 87), int(t * 1000 / 2 / 87) }')

# run NAME [ARGS...]: one headless run writing NAME.fp, NAME.log and NAME.png
run() {
    name=$1
    shift
    # shellcheck disable=SC2086
    SG_FINGERPRINT="$OUT/$name.fp" $XVFB $BIN/opensgolf --game "$OUT/no-game-folder" --mute --size 800x600 \
        --png "$OUT/$name.png" "$@" > "$OUT/$name.log" 2>&1
    code=$?
    if [ $code -ne 0 ] || [ ! -s "$OUT/$name.fp" ]; then
        echo "FAIL $name: exit $code, no fingerprints"
        tail -5 "$OUT/$name.log"
        fail=1
        return 1
    fi
    return 0
}

# first_difference A B [state]: the first game tick both files have whose fingerprints differ, with the parts that differ
# there (nothing when they agree on every tick they share); the save and load lines are left out. With "state", only the
# game's state counts: not the total, and not what the game hands the simulation before each tick (club_unsaved: the
# ticker's busy flag and the view's; inputs), which a fresh start of the program may set otherwise
first_difference() {
    awk -v state="${3:-}" '$NF == "save" || $NF == "load" { next }
        NR == FNR { a[$1] = $0; next }
        ($1 in a) {
            n = split(a[$1], x, " "); split($0, y, " "); parts = ""
            for (i = 2; i <= n; i++) {
                if (x[i] == y[i]) continue
                k = x[i]; sub(/=.*/, "", k)
                if (state != "" && (i == 2 || k == "club_unsaved" || k == "inputs")) continue
                if (i == 2) k = "total"
                parts = parts " " k
            }
            if (parts != "") { print "tick " $1 ":" parts; exit }
        }' "$1" "$2"
}

# shared A B: how many game ticks both files have
shared() {
    awk '$NF == "save" || $NF == "load" { next } NR == FNR { a[$1] = 1; next } ($1 in a) { n++ } END { print n + 0 }' "$1" "$2"
}

# check LABEL [--extras BITS]: the twin runs and the save and reload under one set of extras
check() {
    label=$1
    shift
    run "$label-a" --property 0 --clock 4242 --edit "$EDIT" --time "$TIME" "$@" || return
    run "$label-b" --property 0 --clock 4242 --edit "$EDIT" --time "$TIME" "$@" || return
    ticks=$(grep -c . "$OUT/$label-a.fp")
    if cmp -s "$OUT/$label-a.fp" "$OUT/$label-b.fp"; then
        echo "ok   $label: twin runs identical over $ticks game ticks"
    else
        echo "FAIL $label: twin runs differ at $(first_difference "$OUT/$label-a.fp" "$OUT/$label-b.fp")"
        fail=1
    fi
    for at in $SAVES; do
        reload "$label" "$at" "$@"
    done
}

# reload LABEL TICK [--extras BITS]: the game saved at game tick TICK (SG_SAVE_AT), loaded and played on
reload() {
    label=$1
    at=$2
    shift 2
    SG_SAVE_AT="$at,$OUT/save/$label-$at.sgs" run "$label-full-$at" --property 0 --clock 4242 --edit "$EDIT" --time "$TIME" "$@" ||
        return
    if [ ! -s "$OUT/save/$label-$at.sgs" ]; then
        echo "FAIL $label: no save at game tick $at"
        fail=1
        return
    fi
    if grep -q "^selfcheck: ok at tick $at " "$OUT/$label-full-$at.log"; then
        echo "ok   $label: the save self-check read the save at game tick $at back to the identical game"
    else
        echo "FAIL $label: the save self-check at game tick $at: $(grep "selfcheck" "$OUT/$label-full-$at.log" | head -2)"
        fail=1
    fi
    # (A: the scripted runs' aiming at the pin, which j:0 set in the first run, for the rest of the practice round)
    run "$label-reload-$at" --load-game "$OUT/save/$label-$at.sgs" --edit "A" --time "$TIME" || return
    full="$OUT/$label-full-$at.fp"
    again="$OUT/$label-reload-$at.fp"
    d=$(first_difference "$full" "$again" state)
    n=$(shared "$full" "$again")
    if [ "$n" -eq 0 ]; then
        echo "FAIL $label: the reloaded game played no game tick the first run had"
        fail=1
    elif [ -n "$d" ]; then
        echo "FAIL $label: saved at game tick $at and reloaded, the game parts at $d"
        fail=1
    else
        all=$(first_difference "$full" "$again")
        [ -n "$all" ] && echo "note $label: after the reload at $at the game hands the simulation other per-tick flags from $all"
        echo "ok   $label: saved at game tick $at and reloaded, identical over the $n game ticks after"
    fi
}

# reload_twice LABEL TICK: the game with a hole being built saved at game tick TICK, loaded twice and played on
reload_twice() {
    label=$1
    at=$2
    SG_SAVE_AT="$at,$OUT/save/$label-$at.sgs" run "$label-full-$at" --property 0 --clock 4242 --edit "$EDIT" --time "$TIME" ||
        return
    if [ ! -s "$OUT/save/$label-$at.sgs" ]; then
        echo "FAIL $label: no save at game tick $at"
        fail=1
        return
    fi
    if grep -q "^selfcheck: skipped at tick $at (a hole being built" "$OUT/$label-full-$at.log"; then
        echo "ok   $label: the save self-check skipped the save at game tick $at (a hole being built)"
    else
        echo "FAIL $label: the save at game tick $at had no hole being built: $(grep "selfcheck" "$OUT/$label-full-$at.log" | head -2)"
        fail=1
    fi
    run "$label-load-a-$at" --load-game "$OUT/save/$label-$at.sgs" --edit "A" --time "$TIME" || return
    run "$label-load-b-$at" --load-game "$OUT/save/$label-$at.sgs" --edit "A" --time "$TIME" || return
    a="$OUT/$label-load-a-$at.fp"
    b="$OUT/$label-load-b-$at.fp"
    n=$(shared "$a" "$b")
    d=$(first_difference "$a" "$b")
    if [ "$n" -eq 0 ]; then
        echo "FAIL $label: the loads of the save at game tick $at played no game tick"
        fail=1
    elif [ -n "$d" ]; then
        echo "FAIL $label: two loads of the save at game tick $at part at $d"
        fail=1
    else
        echo "ok   $label: two loads of the save at game tick $at identical over $n game ticks"
    fi
}

echo "== Classic (no extras)"
check classic

echo "== Classic, the second hole built (tee at 18,22, cup green at 34,30) but not opened"
PLAIN=$EDIT
EDIT="$PLAIN;t:18,22,0;t:34,30,1"
for at in $SAVES; do
    reload_twice classic-building "$at"
done
EDIT=$PLAIN

echo "== Classic, its action log played again (--replay, another window size)"
SG_REPLAY_OUT="$OUT/save/replay.sgr" run replay-a --property 0 --clock 4242 --edit "$EDIT" --time "$TIME" &&
    run replay-b --replay "$OUT/save/replay.sgr" --time "$TIME" --size 1024x700 && {
    d=$(first_difference "$OUT/replay-a.fp" "$OUT/replay-b.fp")
    n=$(shared "$OUT/replay-a.fp" "$OUT/replay-b.fp")
    if [ -n "$d" ] || [ "$n" -eq 0 ] || ! grep -q "^replay: done at tick .* the game matched" "$OUT/replay-b.log"; then
        echo "FAIL replay: the replayed game parts at ${d:-?} ($n game ticks shared; $(grep "replay" "$OUT/replay-b.log" | tail -1))"
        fail=1
    else
        echo "ok   replay: the replayed game identical over $n game ticks, $(grep "^replay: done" "$OUT/replay-b.log")"
    fi
}

if [ "$EXTRAS" = "--extras" ]; then
    echo "== each gameplay extra on in turn"
    # sg_core::extras::Extras::from_bits: rotate, cut content, hardcore, more staff, animals, set par, half and quarter
    # seasons, pause calendar, freeze moods, unlock all, lefties, weather, path style 1, more visitors
    for bits in ${BITS:-1 2 4 8 16 32 64 128 256 512 1024 2048 4096 8192 65536}; do
        check "extras-$bits" --extras "$bits"
    done
fi

if [ $fail -ne 0 ]; then
    echo "determinism check: FAILED (logs and fingerprints in $OUT)"
    exit 1
fi
echo "determinism check: passed (logs and fingerprints in $OUT)"
