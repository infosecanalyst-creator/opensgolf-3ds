// The Clubhouse Notes goals: 22 achievements, each awarded once, driven by game events. Triggers read from the publisher's golf.exe
// (docs/DECODE_SOCIAL.md). EXACT: all 22 triggers and thresholds, difficulty switching of goals 0, 1 and 4, hole class and dogleg
// rules. PLACEHOLDER: wording (our own), the three sheet-only lines (set when the first dogleg right, dogleg left or par five opens),
// and the 100000 prize test of the theme victory goals, which looks odd in the original but is kept as read.
#pragma once
#include <cstdint>
#include <vector>

namespace sg {

enum Goal {
    G_Challenge, G_Heroic, G_SkillUpgrade, G_FirstTournament, G_Strategic, G_FirstMatchWin, G_NineHoleCourse, G_Top100Hole,
    G_Tournament500k, G_Classic, G_Top18Hole, G_TournamentWin9, G_GrandSlam9, G_Tournament1m, G_EighteenHoleCourse,
    G_TournamentWin18, G_GrandSlam18, G_VictoryParkland, G_VictoryDesert, G_VictoryTropical, G_VictoryLinks, G_HundredStars,
    kGoalCount
};
enum SheetOnly { S_DoglegRight, S_DoglegLeft, S_ParFive, kSheetOnly };

enum HoleTypeBits { kLength = 1, kAccuracy = 2, kImagination = 4 };   // hole type is the OR of the skills it demands (0..7)
const char* holeTypeName(int type);   // 0 breather, 1 freeway, 2 precise, 3 challenge, 4 creative, 5 heroic, 6 strategic, 7 classic
const char* goalName(int goal);

// Hole class (par) from raw route length in yards. driverLevel is the driving range upgrade level.
int holeClassFromYards(double rawYards, bool dogleg, int driverLevel);
// Dogleg from the angle difference (tee to green minus first waypoint to green), in 2^32 units per circle: +1 left, -1 right, 0 none.
int doglegFromAngle(int32_t diff);

struct GoalEvent {
    enum Kind { HoleOpened, HoleTyped, SkillUpgrade, TournamentAccepted, MatchWon, HoleRanked, TournamentResult, CourseRated } kind;
    int hole = 0;            // HoleOpened: number of the hole that opened (1..18)
    int holeCount = 0;       // holes on the course when the event happens
    int doglegDir = 0;       // HoleOpened: +1 left, -1 right
    int holeClass = 0;       // HoleOpened: 3, 4, 5 ... (par)
    int type = 0;            // HoleTyped: 0..7
    int prizeThousands = 0;  // TournamentAccepted / TournamentResult: prize in thousands of dollars
    long prizeDollars = 0;   // TournamentResult
    int place = 0;           // TournamentResult: 1 is a win
    bool top100 = false, top18 = false;   // HoleRanked
    int total = 0;           // CourseRated: SGA total 0..100
    int theme = 0;           // TournamentResult: 0 Parkland, 1 Desert, 2 Tropical, 3 Links
};

class GoalTracker {
public:
    explicit GoalTracker(int difficulty = 0) : diff(difficulty) {}
    // Feeds an event; returns the goals newly awarded (each is awarded once).
    std::vector<int> onEvent(const GoalEvent& ev);
    bool done(int g) const { return g >= 0 && g < kGoalCount && got[g]; }
    bool sheetDone(int s) const { return s >= 0 && s < kSheetOnly && sheet[s]; }
    int doneCount() const { int n = 0; for (bool b : got) n += b; return n; }
    int difficulty() const { return diff; }
    void mark(int g) { if (g >= 0 && g < kGoalCount) got[g] = true; }          // restoring a saved game
    void markSheet(int g) { if (g >= 0 && g < kSheetOnly) sheet[g] = true; }
private:
    void award(int g, std::vector<int>& out) { if (!got[g]) { got[g] = true; out.push_back(g); } }
    int diff;
    bool got[kGoalCount] = {};
    bool sheet[kSheetOnly] = {};
};

}
