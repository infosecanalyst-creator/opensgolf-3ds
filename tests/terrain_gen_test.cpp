#include "sg/terrain.h"
#include <cmath>
#include <cstdio>
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); return 1; } } while (0)
using sg::Terrain;
static bool water(const Terrain& t, int x, int y) { const int ty = t.type[(size_t)y * t.w + x]; return ty == sg::TT_WaterShallow; }
static float relief(const Terrain& t) { double s = 0, s2 = 0; const size_t n = t.corner.size(); for (int8_t c : t.corner) { s += c; s2 += (double)c * c; } const double m = s / n; return (float)std::sqrt(s2 / n - m * m); }
int main() {
    for (int lie = 0; lie < 3; lie++)
        for (int theme = 0; theme < 4; theme++) {
            const Terrain t = Terrain::generate(50, 50, 1234 + theme * 17 + lie, theme, lie, 1, 8, false);
            CHECK(t.w == 50 && t.h == 50 && t.clubhouseX >= 17 && t.clubhouseX <= 33 && t.clubhouseY >= 17 && t.clubhouseY <= 33);
            for (int y = t.clubhouseY - 2; y <= t.clubhouseY + 2; y++) for (int x = t.clubhouseX - 2; x <= t.clubhouseX + 2; x++) CHECK(t.type[(size_t)y * 50 + x] == sg::TT_Building);
            int edgeWater[4] = {0, 0, 0, 0};
            for (int i = 4; i < 46; i++) { edgeWater[0] += water(t, i, 3); edgeWater[1] += water(t, 46, i); edgeWater[2] += water(t, i, 46); edgeWater[3] += water(t, 3, i); }
            int sides = 0; for (int k = 0; k < 4; k++) sides += edgeWater[k] > 20;
            if (lie == 2) CHECK(sides == 4);
            if (lie == 1) CHECK(sides >= 1);
            if (lie == 0) CHECK(sides == 0);
            // neighbouring corners differ by at most one level
            for (int y = 0; y <= 50; y++) for (int x = 0; x < 50; x++) { CHECK(std::abs(t.cornerAt(x, y) - t.cornerAt(x + 1, y)) <= 1); }
        }
    const Terrain flat = Terrain::generate(50, 50, 99, 0, 0, 0, 12, false), hilly = Terrain::generate(50, 50, 99, 0, 0, 2, 12, false);
    CHECK(relief(hilly) > relief(flat) + 0.5f);
    const Terrain a = Terrain::generate(50, 50, 7, 3, 1, 1, 5, false), b = Terrain::generate(50, 50, 7, 3, 1, 1, 5, false);
    CHECK(a.type == b.type && a.corner == b.corner && a.clubhouseX == b.clubhouseX);
    std::printf("terrain_gen ok\n"); return 0;
}
