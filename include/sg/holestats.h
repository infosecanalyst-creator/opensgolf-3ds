// Per hole statistics and the numbers the course report derives from them.
// Written from facts read out of the publisher exe (see docs/DECODE_HOLE_STATS.md); no exe code is copied.
// Pure data plus integer arithmetic. The simulation feeds one HoleStats per hole through the record* methods.
// Everything marked EXACT mirrors the exe's integer maths (C truncating division); PLACEHOLDER marks guesses.
#pragma once
#include <cstdint>
#include <string>

namespace sg {

// Skill bits of a golfer (low three bits of the golfer's class byte) and of a hole's demand mask.
enum HoleSkill : unsigned { kSkillLength = 1, kSkillAccuracy = 2, kSkillImagination = 4 };

// Hole type names indexed by the demand mask (bit 1 length, bit 2 accuracy, bit 4 imagination).
constexpr const char* kHoleTypeName[8] = {"Breather", "Freeway", "Precise", "Challenge", "Creative", "Heroic", "Strategic", "Classic"};

// Bits of the per hole flag word.
enum HoleFlag : unsigned {
    kHoleTop100 = 0x1,        // named one of the best 100 holes (Golf Enquirer)
    kHoleTop18 = 0x2,         // named one of the Top 18 holes (Great Golf Holes)
    kHoleTooHard = 0x4,       // average strokes far above par (set by analysis)
    kHoleTooEasy = 0x8,       // average strokes below par (set by analysis)
    kHoleTransient = 0x10,    // cleared by every analysis pass
    kHoleDoglegLeft = 0x20,   // layout bit (setter not located); compared between neighbours for variety
    kHoleDoglegRight = 0x40,  // layout bit, same note (guess: right; the left one is confirmed by the name text)
    kHoleNamePending = 0x80,  // set while the Top 100 naming popup is pending
    kHoleStrongLength = 0x100,       // length differential at least 50
    kHoleStrongAccuracy = 0x200,     // accuracy differential at least 50
    kHoleStrongImagination = 0x400,  // imagination differential at least 50
    kHoleUphill = 0x1000,
    kHoleDownhill = 0x2000,
};

// Golfer reaction ("comment") event types. The record keeps a count and the last location per type (types 0..63).
// Only the types named below matter for the numbers the report derives; the rest are listed for the comment screen.
enum HoleEvent : int {
    kEvGoodShot = 1, kEvBadShotPartner = 2, kEvBadShotSelf = 3, kEvEasyShotMiss = 4, kEvHazardAhead = 5, kEvUsesSlope = 6,
    kEvBridge = 7, kEvBadDesign = 8, kEvBallNearlyHit = 9, kEvWalkThroughObject = 10,
    kEvScenicView = 11,        // "check out this nice <object>"; +1; one of the three scenic counters
    kEvSlicedTrouble = 12, kEvWater = 13, kEvThirsty = 14, kEvHungry = 15, kEvHook = 16, kEvSlice = 17, kEvSnack = 18,
    kEvHoleScore = 19,         // the score call at the end of a hole; never stored, may turn into 23
    kEvUglyView = 20,          // -2 (reduced by the negative rule)
    kEvSlowPlay = 21,
    kEvNiceFeature = 22,       // "hey, that's <name>'s house" style object comment; +1; counts half toward scenic
    kEvTooHardEasy = 23,       // "this hole is too hard / too easy"
    kEvWeeds = 24, kEvDrink = 25, kEvTired = 26, kEvBench = 27,
    kEvLovelyView = 28,        // "look at that lovely <object>"; +1; one of the three scenic counters
    kEvVariety = 29, kEvSameAsLast = 30, kEvSeeAHundred = 31,
    kEvPartnerAttitude = 47,   // 0x2f
    kEvUphillTricky = 45,      // 0x2d: base 0, therefore never counted
    kEvDownhillNice = 46,      // 0x2e: +1
    kEvDrivingRange = 51,      // 0x33..0x35
    kEvCount = 64
};

// What the sim supplies when a golfer finishes a hole.
struct HoleFinishInput {
    int strokes = 0;                 // strokes including the holing putt (clamped 0..9 into a bin)
    unsigned skillMask = 7;          // golfer skills, low three bits (1 length, 2 accuracy, 4 imagination)
    bool ordinaryGolfer = true;      // false for staff, pros, celebrities: they never enter the histogram
    int fee = 0;                     // fee paid for the hole in money units of 100 (see holeFee)
    int elapsedTicks = 0;            // ticks between the golfer arriving at the tee and holing out (<= 0 adds nothing)
    bool feesDisabled = false;       // exe game flag 0x200000: no fee is recorded (meaning of the flag not decoded)
};

// Inputs of the fee a golfer pays at the end of a hole (money units of 100). Terms are exact, the base is the golfer mood.
struct HoleFeeInput {
    int golferMood = 0;       // golfer mood counter, clamped -10..10 by the reaction code
    bool doubleRate = false;  // a mode that doubles the mood term (exe global 0x543cf4 == 2; meaning unknown)
    int globalBonus = 0;      // a global additive fee bonus (exe 0x543cd8; source unknown, PLACEHOLDER 0)
    int memberTier = 0;       // golfer's membership tier byte & 7: 4 adds 2, 5 to 7 add 5
};
int holeFee(const HoleFeeInput& in, unsigned holeFlags);

// Inputs when a ball lands (every landing, tee shot included).
struct BallLandedInput {
    int strokesBefore = 0;   // strokes already taken on the hole before this one
    int driveYards = 0;      // only used when strokesBefore == 0: carry plus roll of the tee shot in yards
    bool fairwayOrBetter = false;  // landing tile's lie penalty is zero or below (fairway, tee, green...)
    bool onGreen = false;
};

// One group of the stroke histogram: golfers with the same skill mask.
struct SkillAvg {
    int rounds = 0;   // finished rounds including the prior of 8
    int strokes = 0;  // strokes including the prior of 8 * par
    int avg100() const { return rounds ? strokes * 100 / rounds : 0; }
};

// Geometry the variety test needs from a hole. Heading is a compass bearing of the straight line tee to green.
struct HoleGeometry {
    int par = 0;
    unsigned flags = 0;
    int teeX = 0, teeY = 0, greenX = 0, greenY = 0;  // tile coordinates
};

struct HoleStats {
    // --- layout (record offsets +0x00..+0x1f) ---
    int par = 0;      // 0 means the hole is not open
    int yards = 0;
    unsigned flags = 0;

    // --- counters (+0x20..+0x207) ---
    int rounds = 0;       // golfers who have hit a tee shot here ("rounds counted"), +0x20
    int shotPlans = 0;    // times a golfer planned a normal shot toward the default target, +0x24
    int16_t hist[8][11] = {};  // finished rounds by golfer skill mask (group) and strokes (bin 0..10; 1..9 are read), +0x28
    int16_t events[64] = {};     // count of non zero reactions per event type, +0xd8
    int16_t eventLoc[64] = {};   // last location argument per event type (display uses & 0x3fff), +0x16c
    int16_t fun = 0;          // signed sum of reaction deltas, +0x158
    int16_t quits = 0;        // golfers who gave up here, +0x15a
    int16_t driveSum = 0;     // sum of tee drive yards over counted rounds, +0x15c
    int16_t fairways = 0;     // tee shots landing on fairway or better, +0x15e
    int16_t gir = 0;          // greens in regulation, +0x160
    int16_t putts = 0;        // putts taken, +0x162
    int16_t longestDrive = 0; // +0x166
    int timeHalfTicks = 0;    // sum of (elapsed ticks / 2) of finished holes, +0x1ec
    int expense = 0;          // +0x1f0 (no writer with a hole index was found; stays 0 in the exe)
    int revenue = 0;          // fees collected, money units of 100, +0x1f4
    int expense2 = 0;         // +0x1f8 (never written in the exe)
    int variety = 0;          // similarity-to-previous counter, +0x1fc (see computeVariety)

    // ---- feeding ----
    void reset(int newPar, int newYards, unsigned newFlags);  // zeroes the statistics, as when a hole is (re)opened
    void recordShotPlan() { ++shotPlans; }
    void recordBallLanded(const BallLandedInput& in);
    void recordPutt() { ++putts; }
    void recordQuit() { ++quits; }
    // Apply an already adjusted reaction delta (see eventDelta). Adds to fun and, when delta != 0, bumps the type's count.
    void recordEvent(int type, int location, int delta);
    void recordHoleFinished(const HoleFinishInput& in);

    // ---- derived numbers (all EXACT unless stated) ----
    int finishedRounds() const;                  // histogram bins 1..9 over all groups
    int avgStrokes100() const;                   // report "Avg": strokes*100/finished, no prior
    SkillAvg group(unsigned mask) const;         // prior of 8 rounds at par plus bins 1..9 of one group
    int funPercent() const;                      // report Fun column; 0 without counted rounds
    int funPercentDialog(bool* shown = nullptr) const;  // Hole Stats dialog variant, shown only when shotPlans != 0
    int avgMinutes() const;                      // (timeHalfTicks / rounds) / 40
    bool isScenic() const;                       // events[22]/2 + events[28] + events[11] > 7
    int avgFee100() const;                       // report average fee, money units: revenue*100/finished
    int profit() const { return revenue - expense - expense2; }

    // Skill demand differentials (hundredths of a stroke). flag40 is the 0x40 bit of the game flags: it adds the
    // "skill alone against no skill" term and lets every golfer mask appear (otherwise golfers only carry masks 3,5,6,7).
    int demand(unsigned skill, bool flag40) const;
    int demandLength(bool flag40) const { return demand(kSkillLength, flag40); }
    int demandAccuracy(bool flag40) const { return demand(kSkillAccuracy, flag40); }
    int demandImagination(bool flag40) const { return demand(kSkillImagination, flag40); }
    // Threshold that counts a hole as demanding a skill: 25 on difficulty 0, else 50.
    static int demandThreshold(int difficulty) { return difficulty == 0 ? 25 : 50; }
    bool demandsLength(int difficulty, bool flag40) const { return demandLength(flag40) >= demandThreshold(difficulty); }
    bool demandsAccuracy(int difficulty, bool flag40) const { return demandAccuracy(flag40) >= demandThreshold(difficulty); }
    bool demandsImagination(int difficulty, bool flag40) const { return demandImagination(flag40) >= demandThreshold(difficulty); }

    // Hole type mask as the course report computes it (0..7), and its name.
    unsigned typeMask(int difficulty, bool flag40) const;
    const char* holeType(int difficulty, bool flag40) const { return kHoleTypeName[typeMask(difficulty, flag40)]; }
    // The periodic background analysis uses a slightly different rule (no flag40 term, drop threshold 50 on difficulty 0).
    unsigned typeMaskAnalysis(int difficulty) const;

    // Background analysis outputs.
    unsigned analysisFlags(int difficulty) const;  // hard/easy and strong-demand bits recomputed from the histogram
    int analysisScore(int difficulty) const;       // sum of the three differentials, floored by 3*fun on difficulty < 2
    // Top 100 / Top 18 naming test. Returns the flag to add (0, kHoleTop100, kHoleTop18 or both).
    unsigned awardCandidate(int difficulty) const;

    // Variety counter of this hole against the previous record (EXACT rule, see the doc). prevMask is the type mask
    // of the previous open hole (use 0xffffffff when there is none), holeIndex is 1 based.
    int computeVariety(int holeIndex, const HoleGeometry& self, const HoleGeometry& prev, unsigned thisMask, unsigned prevMask, int difficulty) const;
    // The report counts a hole toward "Holes with Variety" when holeIndex > 0 and variety < 2.
    bool countsAsVariety(int holeIndex) const { return holeIndex > 0 && variety < 2; }

    // Event type of the end-of-hole score call: 23 ("too hard/too easy" complaint) when the hole is flagged hard and the golfer
    // finished 2 or more over par, or flagged easy and the golfer finished under par, after more than 9 counted rounds, and the
    // golfer is not of kind 0x20; otherwise 19 (plain score call, never stored). EXACT.
    int scoreEventType(int strokes, bool golferKind20) const;

    // The comment screen: up to five event types with the highest counts (types 0..49), highest first, ties to the lowest type.
    int topComments(int out[5], int pctOut[5]) const;
    // ---- Hole Stats dialog numbers (docs/DECODE_HOLE_STATS2.md); only strokes f..9 count (the six visible columns), f = max(par - 2, 1) ----
    int dialogFirstBin() const { return par - 2 > 1 ? par - 2 : 1; }
    int dialogDiff100(unsigned skill) const;       // group (all skills minus one) average minus group 7 average, 8 phantom par rounds each
    int dialogVisibleCount() const;                // finished rounds in the visible columns
    int dialogAvg100() const;                      // stroke average of the visible columns (0 when none)
    // Fun label for the dialog: "(poor)" below 0, then fair, good, very good, outstanding per 20 points.
    const char* funLabel() const;
};

// Signed hundredths as the dialog prints them: "+0.35", "-1.20". Word for a differential: poor below 0, fair, good from 25, very good from 50, outstanding from 100.
std::string signedHundredths(int v);
const char* diffWord(int v);
// Word for the Fun Factor percent: poor below 0, then fair, good, very good, outstanding per 20 points.
const char* funWord(int pct);

// Round end bookkeeping for the roster (docs/DECODE_HOLE_STATS2.md section 5). Unplayed holes of the 18 count 5 strokes and 1 over par.
struct RoundEnd { int adjusted = 0; int overPar = 0; };
RoundEnd roundEnd(int strokesPlayed, int parPlayed, int holesPlayed);
int newLow(int oldLow, int adjusted);            // byte: the adjusted total when below the old low or the old low is 0
int newHandicap(int oldHcp, int overPar);        // byte: the over par sum when old is 0, else (old + new) / 2 with truncation

// Reaction delta for an event type before the repeat and negative rules. cond is the golfer state test some types use
// (types 0x12, 0x19, 0x1b: a golfer need counter over a threshold; 0x07: param == 0; 0x3b: object flag set).
int eventBaseDelta(int type, int difficulty, bool cond);
// Full rule: base, then (for a base of -1 on an ordinary golfer) the delta only counts when the same type appears in the
// golfer's recent history or the golfer already complained, then negatives become (d-1)/2 unless the golfer is a pro kind.
int eventDelta(int type, int difficulty, bool cond, bool ordinaryGolfer, bool repeatedRecently, bool alreadyComplained, bool proKind);

// Compass bearing of (dx,dy) with y pointing down, as a 32 bit angle (0 = up, 0x40000000 = right). Approximates the exe's atan2.
uint32_t headingAngle(int dx, int dy);
// Octant 0..7 of an angle: ((angle >> 28) + 1) >> 1 & 7.
int headingOctant(uint32_t angle);

// Yardage the exe stores when a hole's yards were not set by hand: path length L, plus (L - 250) / 4 above 250. EXACT.
int measuredToYards(int pathYards);
// Par from the adjusted yardage (EXACT thresholds). dogleg adds 25 yards above 250; themeIndex subtracts 25 per index above 300.
int holeParFromYards(int yards, bool dogleg, int themeIndex);

}  // namespace sg
