// Checks for the reaction engine and the needs clock (docs/DECODE_EVENTS_SHOTS.md section 1, DECODE_EVENTS_NEEDS.md sections 2 and 3).
#include <cstdio>
#include "sg/reactions.h"

static int fails = 0;
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); ++fails; } } while (0)

static sg::ReactOut go(sg::Reactor& r, int& mood, int type, int d = 0, int kind = 0, bool partner = false, int strokes = 1) {
    sg::ReactIn in; in.type = type; in.difficulty = d; in.kind = kind; in.strokes = strokes;
    return sg::react(r, mood, in, partner);
}

int main() {
    using namespace sg;
    { // a lone -1 complaint is free; a repeat inside the window costs 1
        Reactor r; int mood = 5;
        ReactOut o = go(r, mood, 14); CHECK(o.delta == 0 && !o.counted && mood == 5);
        o = go(r, mood, 14); CHECK(o.delta == -1 && o.counted && mood == 4);
    }
    { // d 0 window is 3 reactions: a repeat three back does not count
        Reactor r; int mood = 5;
        go(r, mood, 14); go(r, mood, 1); go(r, mood, 1);
        ReactOut o = go(r, mood, 14); CHECK(o.delta == 0);
        Reactor r5; mood = 5; go(r5, mood, 14); go(r5, mood, 1); go(r5, mood, 1);
        o = go(r5, mood, 14, 1); CHECK(o.delta == -1);   // d 1 window is 5
    }
    { // a -1 right after a negative base reaction counts even when that one was filtered
        Reactor r; int mood = 5;
        go(r, mood, 14);
        ReactOut o = go(r, mood, 2); CHECK(o.delta == -1);
    }
    { // scaling of bigger complaints: -2 becomes -1, -3 becomes -2, pros keep the full value
        Reactor r; int mood = 5;
        CHECK(go(r, mood, 20).delta == -1); CHECK(go(r, mood, 36).delta == -2);
        Reactor p; int pm = 5; CHECK(go(p, pm, 20, 0, 0x40).delta == -2);
    }
    { // mood clamps at -10 and 10
        Reactor r; int mood = 10; go(r, mood, 1); CHECK(mood == 10);
        mood = -10; go(r, mood, 36); CHECK(mood == -10);
    }
    { // polarity of the newest reaction: good 0, neutral 1, bad 2
        Reactor r; int mood = 5;
        go(r, mood, 1); CHECK(r.polarity() == 0);
        go(r, mood, 14); CHECK(r.polarity() == 2);   // filtered but base negative
        go(r, mood, 5); CHECK(r.polarity() == 1);
        go(r, mood, 20); CHECK(r.polarity() == 2);
    }
    { // snack and drink only please a hungry or thirsty golfer; the counter is read before the reset
        Reactor r; int mood = 5;
        r.hunger = 8; CHECK(go(r, mood, 18).delta == 1);
        r.hunger = 7; CHECK(go(r, mood, 18).delta == 0);
        r.thirst = 8; CHECK(go(r, mood, 25).delta == 1);
        r.fatigue = 60; CHECK(go(r, mood, 27).delta == 1); r.fatigue = 59; CHECK(go(r, mood, 27).delta == 0);
    }
    { // strokes above 9 and the disabled flag drop the reaction; chat types only speak
        Reactor r; int mood = 5; ReactIn in; in.type = 1; in.strokes = 10;
        CHECK(react(r, mood, in, false).ignored);
        in.strokes = 3; in.disabled = true; CHECK(react(r, mood, in, false).ignored);
        Reactor c; ReactOut o = go(c, mood, 63); CHECK(o.ignored && c.speech == 7);
    }
    { // a partner can answer a complaint on the harder difficulties only
        Reactor r; int mood = 5; ReactOut o = go(r, mood, 20, 3, 0, true, 1); CHECK(o.partnerReacts);
        Reactor r0; o = go(r0, mood, 20, 1, 0, true, 1); CHECK(!o.partnerReacts);
        Reactor r2; o = go(r2, mood, 47, 3, 0, true, 1); CHECK(!o.partnerReacts);
    }
    { // needs clock: thirst climbs one step per 160 ticks standing and the 16th rise raises a comment on the 16th step
        Reactor r; NeedsIn ni; ni.slot = 0; ni.hole = 4; ni.holeCount = 18; ni.clubRemarkMade = true; int out[3]; int raised14 = 0;
        auto rnd = [](int) { return 1; };   // never hunger in theme 3, never the chat remark
        for (unsigned t = 0; t < 160u * 16u + 1u; ++t) { ni.tick = t; const int n = needsTick(r, ni, rnd, out); for (int i = 0; i < n; i++) if (out[i] == 14) raised14++; }
        CHECK(r.thirst == 17 && raised14 == 1);   // tick 0 is a step, so 17 rises by tick 2560; the comment fires at rise 16 and the next one waits for 20
    }
    { Reactor r; NeedsIn ni; ni.hole = 4; ni.clubRemarkMade = true; int out[3]; int got = 0; auto rnd = [](int) { return 1; };
      r.thirst = 15; ni.tick = 0; got = needsTick(r, ni, rnd, out); CHECK(got == 1 && out[0] == 14 && r.thirst == 16); }
    { // scripted pair golfers never complain about needs
        Reactor r; NeedsIn ni; ni.hole = 4; ni.kind = 0x20; ni.clubRemarkMade = true; int out[3]; auto rnd = [](int) { return 1; };
        r.thirst = 15; ni.tick = 0; CHECK(needsTick(r, ni, rnd, out) == 0 && r.thirst == 16);
    }
    { // fatigue comment when it crosses a multiple of 40 above 159
        Reactor r; NeedsIn ni; ni.hole = 3; ni.moving = true; ni.walking = true; ni.effort = 3; ni.clubRemarkMade = true; int out[3]; auto rnd = [](int) { return 1; };
        r.fatigue = 158; ni.tick = 0; const int n = needsTick(r, ni, rnd, out);   // 158 + 3 = 161, 160/40 = 4 differs from 158/40 = 3
        bool has26 = false; for (int i = 0; i < n; i++) has26 |= out[i] == 26; CHECK(has26);
    }
    { // glance mask: 0xff neutral, a quarter of 0x1ff after a good reaction, 0x1ff after a filtered complaint, half again while walking
        Reactor r; int mood = 5;
        CHECK(glanceMask(r, false) == 0xff && glanceMask(r, true) == 0x7f);
        go(r, mood, 1); CHECK(glanceMask(r, false) == 0x7f && glanceMask(r, true) == 0x3f);
        Reactor q; go(q, mood, 14); CHECK(q.histLoc[0] == (0x14 | kLocBaseBad) && glanceMask(q, false) == 0x1ff);
    }
    { // quitting needs a bad mood and a silent speaker
        Reactor r; CHECK(wantsToQuit(r, -1, 0)); r.speech = 3; CHECK(!wantsToQuit(r, -1, 0)); r.speech = 0; CHECK(!wantsToQuit(r, 0, 0)); CHECK(!wantsToQuit(r, -1, 0x20));
        for (int i = 0; i < 7; i++) { r.speech = 7; for (unsigned t = 0; t < 8; t++) speechTick(r, t); CHECK(r.speech == 6); r.speech = 0; }
    }
    CHECK(quitReasonCategory(13) == 13 && quitReasonCategory(1) == 0);
    if (!fails) std::printf("reactions_test: all passed\n");
    return fails ? 1 : 0;
}
