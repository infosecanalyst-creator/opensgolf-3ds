#include "sg/ballphys.h"
#include "sg/flight.h"
#include <cstdio>
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); return 1; } } while (0)
using namespace sg;
static float restOn(int tile, int range) {
    const flight::Launch l = flight::launchFor(range);
    auto r = ballphys::run(1, 0, l.speed, l.vertical, 0, false, [&](float, float) { return tile; }, [] { return 0.5f; });
    return r.restX;
}
int main() {
    CHECK(kLie[0].bounce == 4 && kLie[1].penalty == -1 && kLie[17].cls == 17 && kLie[7].friction == 1 && kLie[20].penalty == 8);
    CHECK(lieOf(TT_WaterDeep).cls == 17 && lieId(TT_Brush) == 11);
    const float fair = restOn(TT_Fairway, 200), rough = restOn(TT_Rough, 200), sand = restOn(TT_PotSandBunker, 200);
    CHECK(fair > rough && rough > sand);                 // firmer lies run farther
    CHECK(restOn(TT_FirmFairway, 200) > fair);
    const flight::Arc a = flight::simulate(200);
    CHECK(fair > a.distance);                            // the roll adds to the carry
    CHECK(restOn(TT_Fairway, 100) < fair);
    auto w = ballphys::run(1, 0, flight::launchFor(150).speed, flight::launchFor(150).vertical, 0, false, [&](float x, float) { return x > 3000 ? (int)TT_WaterDeep : (int)TT_Fairway; }, [] { return 0.5f; });
    CHECK(w.water && w.restTile == TT_WaterDeep);
    auto o = ballphys::run(1, 0, flight::launchFor(150).speed, flight::launchFor(150).vertical, 0, false, [&](float x, float) { return x > 3000 ? -1 : (int)TT_Fairway; }, [] { return 0.5f; });
    CHECK(o.oob);
    std::printf("ballphys ok: carry %.0f, fairway rest %.0f, rough %.0f, sand %.0f\n", a.distance, fair, rough, sand); return 0;
}
