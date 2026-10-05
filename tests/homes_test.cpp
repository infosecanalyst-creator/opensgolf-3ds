#include <string>
// Checks of the home site rules against the numbers in docs/DECODE_HOMES.md.
#include <cstdio>
#include <vector>
#include "sg/homes.h"
using namespace sg;
using namespace sg::homes;
static int fails = 0;
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); fails++; } } while (0)

int main() {
    // Value smoothing: steady state 12 * lot, first month gives the lot value itself.
    { int v = 0; const int lot = 100; v = nextValue(v, lot); CHECK(v == 100); for (int i = 0; i < 400; i++) v = nextValue(v, lot); CHECK(v >= 1190 && v <= 1200); }
    // Sale thresholds from the doc table.
    CHECK(saleThreshold(0, 0) == 800 && saleThreshold(1, 0) == 1400 && saleThreshold(6, 0) == 4400);
    CHECK(saleThreshold(0, 1) == 1200 && saleThreshold(2, 2) == 4000 && saleThreshold(0, 3) == 2000 && saleThreshold(6, 3) == 11000);
    // Months to sell: d = 0, N = 0. Lot 100 sells after about 13 months, lot 66 never, lot 67 in about 53.
    auto months = [](int lot, int d, int n) { Site s; int sales = n; for (int m = 1; m <= 400; m++) { if (monthlyPass(s, lot, sales, d, true, [] { return 0; })) return m; } return -1; };
    CHECK(months(100, 0, 0) >= 12 && months(100, 0, 0) <= 14);
    CHECK(months(66, 0, 0) == -1);
    CHECK(months(67, 0, 0) >= 50 && months(67, 0, 0) <= 56);
    CHECK(months(500, 0, 0) == 2 || months(500, 0, 0) == 3);
    { Site s; int sales = 0; int calls = 0; s.value = 5000; CHECK(!monthlyPass(s, 300, sales, 0, false, [&] { calls++; return 3; })); CHECK(calls == 0 && s.buyer == 0);
      CHECK(monthlyPass(s, 300, sales, 0, true, [&] { calls++; return 3; })); CHECK(s.buyer == 4 && sales == 1 && calls == 1);
      CHECK(!monthlyPass(s, 300, sales, 0, true, [] { return 0; })); CHECK(s.buyer == 4); }
    // Size classes.
    CHECK(sizeClass(-5) == 0 && sizeClass(199) == 0 && sizeClass(200) == 1 && sizeClass(599) == 1 && sizeClass(600) == 2 && sizeClass(1199) == 2 && sizeClass(1200) == 3 && sizeClass(1999) == 3 && sizeClass(2000) == 4);
    // Money.
    CHECK(clubShare(99) == 24 && removalCharge(99, 0) == 49 && removalCharge(99, 49) == 49 && removalCharge(99, 50) == 50 && removalCharge(100, 100) == 52);
    CHECK(freeSites(5, 2) == 3 && freeSites(1, 3) == -2);
    // Map colour.
    CHECK(mapGreen(400, 0) == 18 && mapGreen(1000, 0) == 31 && mapGreen(20, 50) == 0 && mapGreen(4 * 171, 0) == 31);
    // Lot value on a small synthetic map: a water ring and one hole nearby.
    std::vector<BuildTile> tiles(50 * 50);
    for (BuildTile& t : tiles) { t.id = 4; t.owned = true; }   // all rough: ring term +12 each
    HoleInfo hole; hole.exists = true; hole.plays = 20; hole.moodSum = 20; hole.teeX = 10; hole.teeY = 10; hole.greenX = 30; hole.greenY = 10;
    LotContext c; c.tiles = tiles.data(); c.theme = 0; c.difficulty = 1; c.holes = &hole; c.holeCount = 1;
    CHECK(ringSum(c, 20, 20) == 12 * 12);
    CHECK(holeScore(hole, 1) == 1000 * 20 / 24 + 200);
    // Distance from (10,12) to the tee (10,10) is 2 so the divisor is 10.
    CHECK(bestHoleTerm(c, 10, 12) == holeScore(hole, 1) / 10);
    CHECK(lotValue(c, 10, 12) == (holeScore(hole, 1) / 10) * 144 / 40);
    c.marinaEffect = 2; { const int m = holeScore(hole, 1) / 10; CHECK(lotValue(c, 10, 12) == ((m + 2 * m / 3) * 144) / 40); } c.marinaEffect = 0;
    // A hole with no plays contributes nothing; ring terms ignore unowned tiles.
    hole.plays = 0; CHECK(lotValue(c, 10, 12) == 0); hole.plays = 20;
    tiles[19 * 50 + 19].owned = false; CHECK(ringTerm(c, 19, 19) == 0);
    tiles[19 * 50 + 19].owned = true; tiles[19 * 50 + 19].id = 17; CHECK(ringTerm(c, 19, 19) == 32);
    tiles[19 * 50 + 19].id = 21; CHECK(ringTerm(c, 19, 19) == -16);
    tiles[19 * 50 + 19].id = 18; CHECK(ringTerm(c, 19, 19) == 3); c.theme = 1; CHECK(ringTerm(c, 19, 19) == 16);
    CHECK(ringTerm(c, -1, 0) == 0 && ringTerm(c, 0, 50) == 0);
    CHECK(std::string(celebrityKindWord('B')) == "pop sensation" && std::string(celebrityKindWord('K')) == "superstar");
    if (fails) { std::printf("%d failures\n", fails); return 1; }
    std::printf("homes_test: all checks passed\n");
    return 0;
}
