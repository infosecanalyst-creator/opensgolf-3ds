#include "sg/goals.h"

namespace sg {

const char* holeTypeName(int t) {
    static const char* n[8] = {"Breather", "Freeway", "Precise", "Challenge", "Creative", "Heroic", "Strategic", "Classic"};
    return (t >= 0 && t < 8) ? n[t] : "";
}

const char* goalName(int g) {
    static const char* n[kGoalCount] = {
        "Open a challenge hole", "Open a heroic hole", "First skill upgrade", "First tournament", "Open a strategic hole",
        "First match victory", "First course of 9 holes", "Hole in the Top 100", "Accept a 500k tournament", "Open a classic hole",
        "Hole in the Top 18", "Win a tournament (9 holes)", "Grand Slam course (9 holes)", "Accept a 1M tournament",
        "First course of 18 holes", "Win a tournament (18 holes)", "Grand Slam course (18 holes)", "Victory on a parkland course",
        "Victory on a desert course", "Victory on a tropical course", "Victory on a links course", "Perfect 100 rating"};
    return (g >= 0 && g < kGoalCount) ? n[g] : "";
}

int holeClassFromYards(double raw, bool dogleg, int driverLevel) {
    int y = int(raw);
    if (y > 250) y += (y - 250) / 4;
    if (dogleg && y > 250) y += 25;
    if (y > 300) y -= 25 * driverLevel;
    int c = 3;
    if (y < 51) c = 2;
    if (y > 249) c = 4;
    if (y > 474) c = 5;
    if (y > 625) c = 6;
    return c;
}

int doglegFromAngle(int32_t d) {
    const int32_t lim = 0x071c71c6;   // ten degrees
    if (d > lim) return 1;
    if (d < -lim) return -1;
    return 0;
}

std::vector<int> GoalTracker::onEvent(const GoalEvent& ev) {
    std::vector<int> out;
    bool hard = diff >= 2;
    switch (ev.kind) {
    case GoalEvent::HoleOpened:
        if (ev.doglegDir < 0) sheet[S_DoglegRight] = true;
        if (ev.doglegDir > 0) sheet[S_DoglegLeft] = true;
        if (ev.holeClass == 5) sheet[S_ParFive] = true;
        if (!hard) {
            if (ev.doglegDir < 0) award(G_Challenge, out);
            if (ev.doglegDir > 0) award(G_Heroic, out);
            if (ev.holeClass == 5) award(G_Strategic, out);
        }
        if (ev.hole >= 9 || ev.holeCount >= 9) { if (ev.hole == 9) award(G_NineHoleCourse, out); }
        if (ev.hole == 18) award(G_EighteenHoleCourse, out);
        break;
    case GoalEvent::HoleTyped:
        if (hard) {
            if (ev.type == 3) award(G_Challenge, out);
            if (ev.type == 5) award(G_Heroic, out);
            if (ev.type == 6) award(G_Strategic, out);
        }
        if (ev.type == 7) award(G_Classic, out);
        break;
    case GoalEvent::SkillUpgrade: award(G_SkillUpgrade, out); break;
    case GoalEvent::TournamentAccepted:
        award(G_FirstTournament, out);
        if (ev.prizeThousands >= 500) award(G_Tournament500k, out);
        if (ev.prizeThousands >= 1000) award(G_Tournament1m, out);
        break;
    case GoalEvent::MatchWon: award(G_FirstMatchWin, out); break;
    case GoalEvent::HoleRanked:
        if (ev.top100) award(G_Top100Hole, out);
        if (ev.top18) award(G_Top18Hole, out);
        break;
    case GoalEvent::TournamentResult:
        if (ev.place == 1) {
            if (ev.holeCount >= 9) award(G_TournamentWin9, out);
            if (ev.holeCount >= 18) award(G_TournamentWin18, out);
            if (ev.holeCount >= 18 && ev.prizeDollars > 100000 && ev.theme >= 0 && ev.theme < 4) award(Goal(G_VictoryParkland + ev.theme), out);
        }
        break;
    case GoalEvent::CourseRated:
        if (ev.total >= 90 && ev.holeCount >= 9) award(G_GrandSlam9, out);
        if (ev.total >= 90 && ev.holeCount >= 18) award(G_GrandSlam18, out);
        if (ev.total == 100 && ev.holeCount >= 18) award(G_HundredStars, out);
        break;
    }
    return out;
}

}
