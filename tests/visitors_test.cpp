#include "sg/visitors.h"
#include "sg/panels.h"
static bool ui_panels_landmark_price_check() { return sg::panels::amenities::landmarkPriceUnits(0) == 50 && sg::panels::amenities::landmarkPriceUnits(7) == 120; }
#include <cstdio>
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); return 1; } } while (0)
using namespace sg;
int main() {
    int fresh = 0;
    for (int seed = 1; seed <= 200; seed++) {
        VisitorState s; s.availMask = 0x000f; SocialRng r((uint64_t)seed);
        const Outcome o = resolve(Visitor::Heiress, s, true, 6, 9, r);
        CHECK(o.accepted && o.landmarkType >= 0 && o.landmarkType <= 15);
        CHECK((s.availMask >> o.landmarkType & 1u) && (s.landmarkMask >> o.landmarkType & 1u));
        CHECK(o.effect == (o.landmarkType & 3) && o.landmarkDollars == (long)(o.landmarkType * 5 + 5) * 200);
        if (o.landmarkType >= 4) fresh++;
    }
    CHECK(fresh > 100);   // the draw prefers designs the strip does not have yet
    VisitorState s; SocialRng r(3); CHECK(!resolve(Visitor::Heiress, s, true, 2, 9, r).accepted);   // mood below 3: she declines
    VisitorState full; full.availMask = 0xffff; SpawnContext c; c.pairIndex = 5; c.holes = 9; c.funRating = 5000; c.skillRating = 5000; CHECK(visitorFor(full, c) != Visitor::Heiress);
    CHECK(ui_panels_landmark_price_check());
    std::printf("visitors ok\n"); return 0;
}
