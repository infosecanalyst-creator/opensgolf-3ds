#include "sg/homes.h"
#include <algorithm>
#include <cmath>

namespace sg {
namespace homes {

namespace {
// Tile terms by id, from DECODE_HOMES.md section 3.1. Ids follow the exe's table (0..22); the port's own water ids 23 to 25 count as water,
// 26 as a green and 27 to 30 as sand traps. Editor-only ids 31 to 35 are PLACEHOLDERS (the exe has no such tiles): cliff and ravine as ravine (25),
// flower bed as rough (12), zen sand and grass bunker as a sand trap (9).
int termOfId(int id, int theme) {
    switch (id) {
        case 0: case 1: case 2: case 3: return -8;   // tee, green, fairway, firm fairway: hazard byte of 0 or less
        case 4: return 12;                           // rough
        case 5: return 2;                            // deep rough
        case 6: return 4;                            // mound
        case 7: return 9;                            // sand trap
        case 8: return 4;                            // waste bunker (port id 8: grassy sand)
        case 9: return 20;                           // pot bunker
        case 10: return 25;                          // ravine (port id 10: overgrowth)
        case 11: return 6;                           // brush
        case 12: return 6;                           // rocks
        case 13: case 14: case 15: return 25;        // tree, pine, palm
        case 16: return 62;                          // elm
        case 17: return 32;                          // water
        case 18: return theme == 1 ? 16 : 3;         // wetlands
        case 19: return 9;                           // marsh
        case 20: return 0;                           // outside the course
        case 21: return -16;                         // another home site
        case 22: return 0;                           // other building
        case 23: case 24: case 25: return 32;        // port water variants
        case 26: return -8;                          // port tricky green
        case 27: case 28: case 29: case 30: return 9;
        case 31: case 32: return 25;
        case 33: return 12;
        case 34: case 35: return 9;
        default: return 0;
    }
}
inline int distTiles(int dx, int dy) { return (int)std::sqrt((double)dx * dx + (double)dy * dy); }   // DERIVED: Euclidean truncated
}  // namespace

int ringTerm(const LotContext& c, int x, int y) {
    if (x < 0 || y < 0 || x >= buildings_exe::kMapSide || y >= buildings_exe::kMapSide || !c.tiles) return 0;
    const BuildTile& t = c.tiles[x * buildings_exe::kMapSide + y];
    if (!t.owned) return 0;
    return termOfId(t.id, c.theme);
}

int ringSum(const LotContext& c, int x, int y) {
    int sum = 0;
    for (int i = 0; i < 4; i++) sum += ringTerm(c, x - 1 + i, y - 1) + ringTerm(c, x - 1 + i, y + 2);
    for (int j = 0; j < 2; j++) sum += ringTerm(c, x - 1, y + j) + ringTerm(c, x + 2, y + j);
    return sum;
}

int holeScore(const HoleInfo& h, int difficulty) {
    int s = (1000 * h.moodSum) / (h.plays + 4) + (3 - difficulty) * 100;
    if (h.top100) s += 100;
    if (h.top18) s += 100;
    return s;
}

int bestHoleTerm(const LotContext& c, int x, int y) {
    int best = 0;
    for (int i = 0; i < c.holeCount; i++) {
        const HoleInfo& h = c.holes[i];
        if (!h.exists || h.plays == 0) continue;
        const int d = std::min(distTiles(x - h.teeX, y - h.teeY), distTiles(x - h.greenX, y - h.greenY));
        const int v = holeScore(h, c.difficulty) / (d + 8);
        if (v > best) best = v;
    }
    return best;
}

int lotValue(const LotContext& c, int x, int y) {
    int m = bestHoleTerm(c, x, y);
    if (c.marinaEffect != 0) m += (c.marinaEffect * m) / 3;
    return (m * ringSum(c, x, y)) / 40;
}

const char* unattractiveLotText() {
    return "This is not a very attractive location for a building lot. Home buyers like water, woods, and grass near a golf hole with a good fun factor.";
}

const char* celebrityKindWord(char t) {
    static const char* k[11] = {"action star", "pop sensation", "noted statesman", "funny man", "supermodel", "fitness guru", "funny lady", "leading man", "beauty", "rocker", "superstar"};
    return (t >= 'A' && t <= 'K') ? k[t - 'A'] : k[0];
}

}  // namespace homes
}  // namespace sg
