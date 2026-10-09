#include "sg/sga.h"
#include <algorithm>
#include <cstdlib>

namespace sg {

static int clamp10(int v) { return std::clamp(v, 0, 10); }

int courseGrade(int holes) {
    if (holes < 6) return 0;
    if (holes < 10) return 1;
    if (holes < 18) return 2;
    return holes == 18 ? 3 : -1;
}

SgaResult evaluateCourse(const SgaInput& in) {
    SgaResult r;
    r.grade = courseGrade(in.holes);
    // Hole counts the exe aims at: five, nine, one less than the holes built (ten to seventeen) or eighteen.
    int c0, c8;
    switch (r.grade) {
        case 0: c0 = 5; c8 = 5; break;
        case 1: c0 = 9; c8 = 9; break;
        case 2: c0 = 17; c8 = in.holes - 1; break;
        default: c0 = 18; c8 = 18; break;
    }
    const int g = r.grade < 0 ? 3 : r.grade;
    r.requiredHoles = c0;
    r.idealYards = ((c0 * 100) / 18) * (g * 5 + 57);
    // Length of course.
    int len = c0 == 18 ? -((r.idealYards - in.totalYards) / 100)
                       : (int)(((long long)(in.totalYards - r.idealYards) * (g + 4) * 5) / r.idealYards);
    r.scores[0] = clamp10(len + 10);
    // Number of holes: a point lost per hole away from the target, times (4 - grade).
    r.scores[1] = clamp10(10 - std::abs(c0 - in.holes) * (4 - g));
    // Time to play: ten up to 235 minutes, one point lost per six minutes, 300 and over is zero.
    {
        const int t = in.avgMinutes;
        int v = -((t - 235) / 6) + 10;
        if (v == 0 && t < 301) v = 1;
        r.scores[2] = clamp10(v);
    }
    r.scores[3] = clamp10((in.funPercent - 109) / 10 + 10);
    r.scores[4] = clamp10(in.varietyHoles - c8 + 10);
    r.scores[5] = clamp10(in.scenicHoles - c8 + 10);
    r.scores[6] = clamp10(in.lengthHoles - c8 + 10);
    r.scores[7] = clamp10(in.accuracyHoles - c8 + 10);
    r.scores[8] = clamp10(in.imaginationHoles - c8 + 10);
    r.scores[9] = clamp10((in.facilityKinds - c8 / 2) * 2 + 8);
    int total = 0;
    for (int s : r.scores) { total += s; if (s == 0) r.notAcceptable = true; }
    r.total = r.notAcceptable ? -999 : total;
    return r;
}

const char* tournamentName(int total, bool championshipGrade) {
    if (total < 1) return nullptr;
    switch ((total - 40) / 5) {   // C division, so everything under 45 lands on the default
        case 1: return "SGA Qualifying School";
        case 2: return "Jr. Tour Event";
        case 3: return "Jr. Tour Championship!";
        case 4: return "SGA Amateur Championship";
        case 5: return "Senior SGA Tour Event";
        case 6: return "Senior SGA Championship";
        case 7: return "SGA Tour Event";
        case 8: return "SGA Players Championship";
        case 9: return "SGA Championship!";
        case 10: case 11: case 12: return championshipGrade ? "Grand Slam Championship!" : "Mini Slam Championship!";
        default: return "Jr. Qualifying School";
    }
}

int tournamentPrizeThousands(int total, int grade, int holes) {
    return (((total - 50) / 5) + 5) * (grade + 1) * (holes + 1) * 2;
}

}  // namespace sg
