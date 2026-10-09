// See include/sg/reactions.h. Rules from docs/DECODE_EVENTS_SHOTS.md and DECODE_EVENTS_NEEDS.md.
#include "sg/reactions.h"
#include "sg/holestats.h"

namespace sg {

int reactionBase(int type, int d, const Reactor& r) {
    bool cond = true;
    if (type == kEvSnack) cond = r.hunger > 7;
    else if (type == 25) cond = r.thirst > 7;
    else if (type == 27) cond = r.fatigue > 59;
    return eventBaseDelta(type, d, cond);
}

ReactOut react(Reactor& r, int& mood, const ReactIn& in, bool partnerFree) {
    ReactOut out;
    if (in.strokes > 9 || in.disabled) { out.ignored = true; return out; }
    const int type = in.type;
    r.speech = 7; r.lastBubbleType = type;
    if (type == 35 && r.hist[0] == 35) { out.ignored = true; return out; }
    if (type == kEvHoleScore) { out.ignored = true; return out; }   // the score call has no effect on mood or statistics
    for (int i = 9; i > 0; --i) { r.hist[i] = r.hist[i - 1]; r.histLoc[i] = r.histLoc[i - 1]; }
    r.hist[0] = type; r.histLoc[0] = in.loc & 0x3fff;
    const int base = reactionBase(type, in.difficulty, r);
    if (type > 0x2f && base == 0) { out.ignored = true; return out; }
    int a = base;
    if (in.kind != 0) {   // momentum (only non ordinary golfers; it feeds the shot planner, which the port does not use)
        if ((r.momentum > 0 && base < 0) || (r.momentum < 0 && base > 0)) r.momentum = 0; else r.momentum += base;
        if ((type == 2 || type == 3 || type == 12 || type == 13) && r.momentum < 0 && base < 0 && r.momentum < base) r.momentum = 1;
    }
    if (base == -1 && in.kind == 0 && in.mode543cf4 != 2) {   // a lone -1 complaint only counts on a repeat
        a = 0;
        const int w = in.difficulty == 0 ? 3 : in.difficulty == 3 ? 10 : 5;
        for (int s = 1; s < w; ++s) if (r.hist[s] == type) a = -1;
        if (r.histLoc[1] & kLocBaseBad) a = -1;
        if (in.mode543cf4 == 1) a = 0;
    }
    if (a < 0 && (in.kind & 0xe0) != 0x40) a = (a - 1) / 2;   // C division
    if (a < 0 && (in.kind & 0xe0) != 0x20 && (((in.strokes + in.slot) & 1) + 2) <= in.difficulty && type != 0x2f && type != 0x15 && type != 0x18 && partnerFree) out.partnerReacts = true;
    mood += a; if (mood > 10) mood = 10; if (mood < -10) mood = -10;
    out.delta = a;
    out.litterRoll = a < 0 && type != 0x2f;
    if (a != 0) out.counted = true;
    if (a > 0) r.histLoc[0] |= kLocGood;
    if (base < 0) r.histLoc[0] |= kLocBaseBad;
    if (a < 0) r.histLoc[0] |= kLocGood | kLocBaseBad;
    return out;
}

int needsTick(Reactor& r, const NeedsIn& in, const std::function<int(int)>& rnd, int out[3]) {
    int n = 0;
    if (in.hole >= 19 || in.hole <= 0) return 0;
    const unsigned P = in.walking ? 120u : 160u;
    if ((in.tick + 0x25u * (unsigned)in.slot) % P == 0) {
        // the chat remark (type 63): only on holes 2 and up before any shot remark; about one needs step in (holes + 1) / 2
        if (in.hole >= 2 && !in.clubRemarkMade && in.holeCount + 1 >= 2 && rnd((in.holeCount + 1) / 2) == 0) out[n++] = 63;
        const bool hungerBranch = in.nearHot || (in.theme == 3 && rnd(2) != 0);
        if (hungerBranch) {
            if (in.hole >= 3 && rnd(2) == 0) {
                ++r.hunger;
                if (r.hunger > 15 && r.hunger % 4 == 0) { if ((in.kind & 0xe0) == 0x20) r.hunger = 16; else out[n++] = 15; }
            }
        } else {
            ++r.thirst;
            if (r.thirst > 15 && r.thirst % 4 == 0) { if ((in.kind & 0xe0) == 0x20) r.thirst = 16; else out[n++] = 14; }
        }
    }
    if (in.moving && (in.tick + 0x0bu * (unsigned)in.slot) % 24u == 0) {
        const int old = r.fatigue;
        r.fatigue += in.effort + in.hole / 6;
        if (r.fatigue > 159 && r.fatigue / 40 != old / 40) { if ((in.kind & 0xe0) == 0x20) r.fatigue = 160; else out[n++] = 26; }
    }
    return n;
}

int glanceMask(const Reactor& r, bool walking) {
    int m = 0x1ff;
    if (!(r.histLoc[0] & (kLocGood | kLocBaseBad))) m = 0xff;
    if (r.histLoc[0] & kLocGood) m >>= 2;
    if (walking) m >>= 1;
    return m;
}

int quitReasonCategory(int t) {
    switch (t) { case 4: case 8: case 9: case 12: case 13: case 14: case 15: case 21: case 23: case 26: case 30: return t; default: return 0; }
}

}  // namespace sg
