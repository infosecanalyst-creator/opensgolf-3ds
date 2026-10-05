// See include/sg/holestats.h and docs/DECODE_HOLE_STATS.md.
#include "sg/holestats.h"
#include <cstdio>

#include <cmath>
#include <cstdlib>
#include <cstring>

namespace sg {

namespace {
int clampi(int v, int lo, int hi) { return v < lo ? lo : (v > hi ? hi : v); }
constexpr int kBinLo = 1, kBinHi = 9;  // bins the exe reads (strokes 1..9, nine and more share the last bin)
}  // namespace

int holeFee(const HoleFeeInput& in, unsigned holeFlags) {
    int fee = in.golferMood;
    if (in.doubleRate) fee *= 2;
    if (holeFlags & kHoleTop100) fee += 2;
    if (holeFlags & kHoleTop18) fee += 2;
    fee += in.globalBonus;
    int tier = in.memberTier & 7;
    if (tier > 3) fee += (tier != 4 ? 3 : 0) + 2;
    return fee;
}

void HoleStats::reset(int newPar, int newYards, unsigned newFlags) {
    par = newPar;
    yards = newYards;
    flags = newFlags;
    rounds = shotPlans = 0;
    std::memset(hist, 0, sizeof hist);
    std::memset(events, 0, sizeof events);
    std::memset(eventLoc, 0, sizeof eventLoc);
    fun = quits = driveSum = fairways = gir = putts = longestDrive = 0;
    timeHalfTicks = 0;
    revenue = 0;
    // The exe's reset leaves expense, the longest drive, fairways, greens and putts alone; it clears the fun total, quits,
    // revenue, time, rounds, plans, histogram bins 0..9 and the event counts. We clear everything, which only differs for
    // fields the exe would carry over a re-open (likely an oversight there).
}

void HoleStats::recordBallLanded(const BallLandedInput& in) {
    if (in.strokesBefore == 0) {
        ++rounds;
        driveSum = (int16_t)(driveSum + in.driveYards);
        if (longestDrive < in.driveYards) longestDrive = (int16_t)in.driveYards;
        if (in.fairwayOrBetter) ++fairways;
    }
    if (in.strokesBefore + 1 == par - 2 && in.onGreen) ++gir;
}

void HoleStats::recordEvent(int type, int location, int delta) {
    if (type < 0 || type >= kEvCount || type == kEvHoleScore) return;
    fun = (int16_t)(fun + delta);
    if (delta != 0) {
        ++events[type];
        eventLoc[type] = (int16_t)location;
    }
}

void HoleStats::recordHoleFinished(const HoleFinishInput& in) {
    if (in.ordinaryGolfer) {
        int g = (int)(in.skillMask & 7);
        int bin = clampi(in.strokes, 0, 9);
        ++hist[g][bin];
    }
    if (!in.feesDisabled) revenue += in.fee;
    if (in.elapsedTicks > 0) timeHalfTicks += in.elapsedTicks / 2;
}

int HoleStats::finishedRounds() const {
    int n = 0;
    for (int g = 0; g < 8; ++g)
        for (int b = kBinLo; b <= kBinHi; ++b) n += hist[g][b];
    return n;
}

int HoleStats::avgStrokes100() const {
    int n = 0, s = 0;
    for (int g = 0; g < 8; ++g)
        for (int b = kBinLo; b <= kBinHi; ++b) {
            n += hist[g][b];
            s += hist[g][b] * b;
        }
    return n ? s * 100 / n : 0;
}

SkillAvg HoleStats::group(unsigned mask) const {
    SkillAvg a;
    a.rounds = 8;
    a.strokes = par * 8;
    for (int b = kBinLo; b <= kBinHi; ++b) {
        a.rounds += hist[mask & 7][b];
        a.strokes += hist[mask & 7][b] * b;
    }
    return a;
}

int HoleStats::funPercent() const {
    if (rounds == 0) return 0;
    return (int)fun * 100 / (shotPlans / 2 + 4 + rounds);
}

int HoleStats::funPercentDialog(bool* shown) const {
    bool ok = shotPlans != 0;
    if (shown) *shown = ok;
    if (!ok) return 0;
    return (int)fun * 100 / (shotPlans / 2 + 4 + rounds);
}

int HoleStats::avgMinutes() const {
    if (rounds == 0) return 0;
    return (timeHalfTicks / rounds) / 40;
}

bool HoleStats::isScenic() const { return (int)events[kEvNiceFeature] / 2 + events[kEvLovelyView] + events[kEvScenicView] > 7; }

int HoleStats::avgFee100() const {
    int n = finishedRounds();
    return n ? revenue * 100 / n : 0;
}

int HoleStats::demand(unsigned skill, bool flag40) const {
    SkillAvg all = group(7), without = group(7u & ~skill);
    int d = without.avg100() - all.avg100();
    if (flag40) {
        SkillAvg none = group(0), alone = group(skill);
        d += none.avg100() - alone.avg100();
    }
    return d;
}

namespace {
struct Mask {
    unsigned mask = 0;
    int lowest = 100;
    unsigned lowestBit = 0;
};
Mask buildMask(const int d[3], int threshold) {
    Mask m;
    const unsigned bits[3] = {kSkillLength, kSkillAccuracy, kSkillImagination};
    for (int i = 0; i < 3; ++i) {
        if (d[i] >= threshold) m.mask |= bits[i];
        if (i == 0) {
            if (d[0] < 100) m.lowest = d[0];
            m.lowestBit = d[0] < 100 ? 1u : 0u;
        } else if (d[i] < m.lowest) {
            m.lowestBit = bits[i];
            m.lowest = d[i];
        }
    }
    return m;
}
}  // namespace

unsigned HoleStats::typeMask(int difficulty, bool flag40) const {
    int d[3] = {demandLength(flag40), demandAccuracy(flag40), demandImagination(flag40)};
    Mask m = buildMask(d, demandThreshold(difficulty));
    if (m.lowest < 100) m.mask &= ~m.lowestBit;
    return m.mask;
}

unsigned HoleStats::typeMaskAnalysis(int difficulty) const {
    int d[3] = {demandLength(false), demandAccuracy(false), demandImagination(false)};
    Mask m = buildMask(d, demandThreshold(difficulty));
    if (m.lowest < (difficulty != 0 ? 100 : 50)) m.mask &= ~m.lowestBit;
    return m.mask;
}

unsigned HoleStats::analysisFlags(int difficulty) const {
    unsigned f = flags & ~(unsigned)(kHoleTooHard | kHoleTooEasy | kHoleTransient | 0x700u);
    int n = finishedRounds();
    if (n > 9) {
        int s = 0;
        for (int g = 0; g < 8; ++g)
            for (int b = kBinLo; b <= kBinHi; ++b) s += hist[g][b] * b;
        if (((6 - difficulty) * n) / 3 + par * n < s) f |= kHoleTooHard;
        if (s < par * n - ((3 - difficulty) * n) / 6) f |= kHoleTooEasy;
    }
    if (demandLength(false) > 49) f |= kHoleStrongLength;
    if (demandAccuracy(false) > 49) f |= kHoleStrongAccuracy;
    if (demandImagination(false) > 49) f |= kHoleStrongImagination;
    return f;
}

int HoleStats::analysisScore(int difficulty) const {
    int score = demandLength(false) + demandAccuracy(false) + demandImagination(false);
    int funTerm = funPercent() * 6 / (difficulty != 0 ? 3 : 2);
    if (difficulty < 2 && score < funTerm) score = funTerm;
    return score;
}

unsigned HoleStats::awardCandidate(int difficulty) const {
    unsigned f = analysisFlags(difficulty) | (flags & (kHoleTop100 | kHoleTop18));
    int score = analysisScore(difficulty);
    unsigned r = 0;
    if (revenue > 200 && score > 200 && (f & 0xd) == 0) {
        r |= kHoleTop100;
        f |= kHoleTop100;
    }
    if (revenue > 400 && score > 300 && (f & 0xe) == 0) r |= kHoleTop18;
    return r;
}

int HoleStats::computeVariety(int holeIndex, const HoleGeometry& self, const HoleGeometry& prev, unsigned thisMask, unsigned prevMask, int difficulty) const {
    int c = 0;
    if (holeIndex <= 1) return 0;
    if (thisMask == prevMask && rounds > 7 && thisMask != 7) ++c;
    if (((prev.flags ^ self.flags) & 0x60) == 0) ++c;
    if (events[kEvUphillTricky] == 0 && events[kEvDownhillNice] == 0) ++c;
    if (self.par == prev.par) ++c;
    uint32_t a = headingAngle(self.greenX - self.teeX, self.greenY - self.teeY);
    uint32_t b = headingAngle(prev.greenX - prev.teeX, prev.greenY - prev.teeY);
    int32_t diff = (int32_t)(a - b);
    if (std::abs(diff >> 24) < 40) ++c;
    if (c != 0 && difficulty < 2) --c;
    return c;
}

int HoleStats::scoreEventType(int strokes, bool golferKind20) const {
    if (golferKind20 || rounds <= 9) return kEvHoleScore;
    if (((flags & kHoleTooHard) && strokes - par > 1) || ((flags & kHoleTooEasy) && strokes - par < 0)) return kEvTooHardEasy;
    return kEvHoleScore;
}

int HoleStats::topComments(int out[5], int pctOut[5]) const {
    int copy[64];
    for (int i = 0; i < 64; ++i) copy[i] = events[i];
    int n = 0;
    for (int k = 0; k < 5; ++k) {
        int best = 0, idx = 0;
        for (int t = 0; t < 64; ++t)
            if (t < 50 && best < copy[t]) {
                best = copy[t];
                idx = t;
            }
        if (best != 0 && rounds != 0) {
            out[n] = idx;
            pctOut[n] = best * 100 / rounds;
            ++n;
        }
        copy[idx] = 0;
    }
    return n;
}

const char* funWord(int f) {
    if (f < 0) return "poor";
    switch (f / 20) {
        case 0: return "fair";
        case 1: return "good";
        case 2: return "very good";
        default: return "outstanding";
    }
}
const char* HoleStats::funLabel() const { return funWord(funPercentDialog()); }

const char* diffWord(int v) { return v < 0 ? "poor" : v < 25 ? "fair" : v < 50 ? "good" : v < 100 ? "very good" : "outstanding"; }
std::string signedHundredths(int v) {
    const int a = v < 0 ? -v : v;
    char b[32];
    std::snprintf(b, sizeof b, "%c%d.%02d", v < 0 ? '-' : '+', a / 100, a % 100);
    return b;
}

int HoleStats::dialogDiff100(unsigned skill) const {
    auto avg = [&](unsigned mask) {
        int n = 8, s = 8 * par;
        for (int b = dialogFirstBin(); b <= kBinHi; ++b) { n += hist[mask & 7][b]; s += b * hist[mask & 7][b]; }
        return s * 100 / n;
    };
    return avg(7u & ~skill) - avg(7);
}
int HoleStats::dialogVisibleCount() const {
    int n = 0;
    for (int g = 0; g < 8; ++g) for (int b = dialogFirstBin(); b <= kBinHi; ++b) n += hist[g][b];
    return n;
}
int HoleStats::dialogAvg100() const {
    int n = 0, s = 0;
    for (int g = 0; g < 8; ++g) for (int b = dialogFirstBin(); b <= kBinHi; ++b) { n += hist[g][b]; s += b * hist[g][b]; }
    return n ? s * 100 / n : 0;
}

RoundEnd roundEnd(int strokesPlayed, int parPlayed, int holesPlayed) {
    const int unplayed = holesPlayed < 18 ? 18 - holesPlayed : 0;
    RoundEnd r;
    r.adjusted = strokesPlayed + 5 * unplayed;
    r.overPar = strokesPlayed - parPlayed + unplayed;
    return r;
}
int newLow(int oldLow, int adjusted) { return (oldLow == 0 || adjusted < oldLow) ? (adjusted & 0xff) : oldLow; }
int newHandicap(int oldHcp, int overPar) {
    const int8_t n = (int8_t)overPar;
    return oldHcp == 0 ? n : (int8_t)((oldHcp + n) / 2);
}

int eventBaseDelta(int type, int difficulty, bool cond) {
    switch (type) {
        case 1: case 6: case 0xb: case 0x16: case 0x1c: case 0x1d: case 0x20: case 0x21: case 0x22: case 0x2c: case 0x2e: return 1;
        case 2: case 0xc: case 0xd: case 0xe: case 0xf: case 0x1a: return -1;
        case 3: case 4: case 8: case 0xa: case 0x2b: case 0x14: case 0x15: case 0x18: case 0x1e: case 0x23: case 0x2f: return -2;
        case 9: case 0x24: return -3;
        case 7: case 0x12: case 0x19: case 0x1b: case 0x3b: return cond ? 1 : 0;
        case 0x17: return -1 - (difficulty != 0 ? 1 : 0);
        case 0x27: case 0x36: return difficulty < 2 ? 1 : 0;
        case 0x33: case 0x34: case 0x35: return difficulty == 0 ? 1 : 0;
        case 0x41: return difficulty > 1 ? -2 : 0;
        default: return 0;  // includes 5, 0x1f, 0x2d, 0x25, 0x26 and the unlisted types
    }
}

int eventDelta(int type, int difficulty, bool cond, bool ordinaryGolfer, bool repeatedRecently, bool alreadyComplained, bool proKind) {
    int d = eventBaseDelta(type, difficulty, cond);
    if (d == -1 && ordinaryGolfer) {
        d = (repeatedRecently || alreadyComplained) ? -1 : 0;
    }
    if (d < 0 && !proKind) d = (d - 1) / 2;
    return d;
}

uint32_t headingAngle(int dx, int dy) {
    const double kTwoPi = 6.283185307179586;
    double a = std::atan2((double)dx, (double)-dy);
    if (a < 0) a += kTwoPi;
    double v = a / kTwoPi * 4294967296.0;
    if (v >= 4294967296.0) v = 0;
    return (uint32_t)v;
}

int headingOctant(uint32_t angle) { return (int)((((angle >> 28) & 0xf) + 1) >> 1) & 7; }

int measuredToYards(int l) { return l > 250 ? l + (l - 250) / 4 : l; }

int holeParFromYards(int yards, bool dogleg, int themeIndex) {
    int y = yards;
    if (dogleg && y > 250) y += 25;
    if (y > 300) y -= themeIndex * 25;
    int par = 3;
    if (y < 51) par = 2;
    if (y > 249) par = 4;
    if (y > 474) par = 5;
    if (y > 625) par = 6;
    return par;
}

}  // namespace sg
