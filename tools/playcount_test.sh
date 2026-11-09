#!/bin/sh
# Tests the play-time count (crates/simgolf/src/playcount.rs) as a player's game runs it, against a stand-in counter on this
# computer (OPENSGOLF_COUNT_TEST_URL, which only takes 127.0.0.1 or localhost): the counter answering, failing, hanging and
# missing; a read-only and a damaged settings.json; a restart. Each case checks what arrived, what was kept for later, that
# the game kept running (its play-time clock went on) and that it never crashed.
#
# usage: tools/playcount_test.sh [BIN] [OUT_DIR]   (run from the repository root after cargo build --release)
set -u
BIN=${1:-target/release/opensgolf}
OUT=${2:-playcount-test}
PORT=${PORT:-8771}
RUN=${RUN:-70}
rm -rf "$OUT"; mkdir -p "$OUT/empty"
here=$(cd "$(dirname "$0")" && pwd)
fail=0

# the stand-in counter: MODE ok answers 200, error 500, hang never answers; each request is logged
server() {
    MODE=$1 python3 - "$PORT" "$OUT/hits.log" <<'PY' &
import http.server, sys, os, time, urllib.parse
mode = os.environ["MODE"]
log = open(sys.argv[2], "a", buffering=1)
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        q = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
        log.write("%s %s %s\n" % (mode, q.get("p", ["?"])[0], self.headers.get("User-Agent")))
        if mode == "hang":
            time.sleep(120); return
        self.send_response(200 if mode == "ok" else 500); self.end_headers(); self.wfile.write(b"ok")
    def log_message(self, *a): pass
http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
PY
    SERVER=$!
    sleep 1
}

# one game: settings.json from $2, played for $RUN seconds as a player would (no SG_* hooks, no --png/--time)
game() {
    case=$1; dir="$OUT/$case"; mkdir -p "$dir/save"
    printf '%s' "$2" > "$dir/save/settings.json"
    [ -n "${READONLY:-}" ] && chmod 444 "$dir/save/settings.json"
    : > "$OUT/hits.log"
    OPENSGOLF_SAVE_DIR="$dir/save" OPENSGOLF_COUNT_TEST_URL="${URL:-http://127.0.0.1:$PORT/count}" \
        timeout "$RUN" xvfb-run -a "$BIN" --game "$OUT/empty" --mute --size 800x600 --load-game "$here/../tests/data/playcount.sgs" \
        > "$dir/out.log" 2>&1
    code=$?
    cp "$OUT/hits.log" "$dir/hits.log"
    if grep -q -i 'panicked\|RUST_BACKTRACE' "$dir/out.log" || [ "$code" != 124 ]; then
        echo "FAIL $case: the game stopped (exit $code) or panicked; see $dir/out.log"; fail=1
    fi
}
hits() { grep -c " $1" "$OUT/$2/hits.log" 2>/dev/null || true; }
field() { python3 -c "import json,sys;d=json.load(open(sys.argv[1]));print(eval(sys.argv[2]))" "$OUT/$1/save/settings.json" "$2" 2>/dev/null; }
check() { if [ "$2" = "$3" ]; then echo "ok   $1: $2"; else echo "FAIL $1: $2, wanted $3"; fail=1; fi; }
S='"support":{"played":893.0,"asks":2,"asked_at":0,"done":true}'

server ok
game sends "{$S,\"playcount\":{\"noticed\":true,\"pending\":3}}"
check "counter answers: start count" "$(hits start/linux sends)" 1
check "counter answers: 15-minute counts (3 kept and the new one)" "$(hits play-15min/linux sends)" 4
check "counter answers: nothing kept afterwards" "$(field sends "d['playcount']['pending']")" 0
check "counter answers: the clock went on" "$(field sends "int(d['support']['played'] >= 900)")" 1
game restart "$(cat "$OUT/sends/save/settings.json")"
check "a restart: the start count only, nothing counted twice" "$(hits play-15min restart)/$(hits start restart)" 0/1
game toomany "{$S,\"playcount\":{\"noticed\":true,\"pending\":4000000000}}"
check "an edited settings.json: at most a week kept, 16 sent at a time" "$(hits play-15min toomany)/$(field toomany "d['playcount']['pending']")" 33/640
game off "{$S,\"playcount\":{\"off\":true,\"noticed\":true,\"pending\":5}}"
check "the tick off: nothing sent" "$(cat "$OUT/off/hits.log" | wc -l | tr -d ' ')" 0
kill $SERVER; wait $SERVER 2>/dev/null

server error
game errors "{$S,\"playcount\":{\"noticed\":true}}"
check "counter fails: the 15-minute count kept" "$(field errors "d['playcount']['pending']")" 1
check "counter fails: the clock went on" "$(field errors "int(d['support']['played'] >= 900)")" 1
kill $SERVER; wait $SERVER 2>/dev/null

server hang
game hangs "{$S,\"playcount\":{\"noticed\":true,\"pending\":2}}"
check "counter hangs: the game kept running" "$(field hangs "int(d['support']['played'] >= 900)")" 1
check "counter hangs: the counts kept after the timeout" "$(field hangs "d['playcount']['pending']")" 3
kill $SERVER; wait $SERVER 2>/dev/null

URL="http://127.0.0.1:9/count" game offline "{$S,\"playcount\":{\"noticed\":true}}"
check "no internet: the 15-minute count kept" "$(field offline "d['playcount']['pending']")" 1
check "no internet: the clock went on" "$(field offline "int(d['support']['played'] >= 900)")" 1

server ok
READONLY=1 game readonly "{$S,\"playcount\":{\"noticed\":true}}"
check "read-only settings.json: still counts" "$(hits play-15min readonly)" 1
game damaged '{"support": {"played": 893.0, "asks": 2, "asked_at": 0, "done": tr'
check "damaged settings.json: no crash, a fresh start count" "$(hits start/linux damaged)" 1
kill $SERVER; wait $SERVER 2>/dev/null

[ $fail = 0 ] && echo "play count test: passed (logs in $OUT)" || { echo "play count test: FAILED (logs in $OUT)"; exit 1; }
