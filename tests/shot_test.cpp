#include "sg/shot.h"
#include <cstdio>
#include <numeric>
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); return 1; } } while (0)
using namespace sg;
int main() {
    Terrain t = Terrain::demoCourse(40, 40, 7);
    std::vector<int> strokes; int stuck = 0;
    for (uint32_t seed = 1; seed <= 60; seed++) {
        ShotSim s; s.loop = false; s.init(t, seed);
        double clock = 0; int last = -1, same = 0;
        while (!s.finished && clock < 900) { s.step(0.05f); clock += 0.05; if (s.stroke > 30) break; }
        if (!s.finished) stuck++; else strokes.push_back(s.stroke);
        (void)last; (void)same;
    }
    CHECK(stuck <= 3 && strokes.size() >= 55);
    const double avg = std::accumulate(strokes.begin(), strokes.end(), 0.0) / (double)strokes.size();
    std::printf("shot ok: avg %.2f strokes over %zu holes, %d unfinished\n", avg, strokes.size(), stuck);
    CHECK(avg > 2.5 && avg < 12);
    return 0;
}
