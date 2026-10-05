// The SGA course evaluation and tournament offer, written from the rules read out of the publisher exe (docs/PUBLISHER_EXE_NOTES.md,
// "SGA evaluation"). Pure functions: the viewer supplies the course statistics.
#pragma once
#include <string>

namespace sg {

struct SgaInput {
    int holes = 0;              // open holes
    int difficulty = 1;         // 0..3; 0 uses the easier length differential (25), others 50
    int totalYards = 0;         // total course length
    int avgMinutes = 0;         // average round time in minutes
    int funPercent = 0;         // average of the per-hole fun figures
    int varietyHoles = 0;       // holes counted as different from the rest
    int scenicHoles = 0;
    int lengthHoles = 0;        // "Length Holes": holes that reward a long hitter
    int accuracyHoles = 0;
    int imaginationHoles = 0;
    int facilityKinds = 0;      // distinct facility building kinds on site
};

struct SgaResult {
    int grade = 0;              // 0 Municipal, 1 Golf Club, 2 Country Club, 3 Championship (-1: more than 18 holes)
    int scores[10] = {};        // length, holes, time, fun, variety, scenic, length holes, accuracy, imagination, facilities
    int total = 0;              // 0..100; -999 when any criterion scored 0
    bool notAcceptable = false;
    int idealYards = 0, requiredHoles = 0;
};

constexpr const char* kGradeName[4] = {"Municipal", "Golf Club", "Country Club", "Championship"};
constexpr const char* kSgaCriterion[10] = {"Length of Course", "Number of Holes", "Time to Play", "Fun Factor", "Holes with Variety", "Scenic Holes",
                                           "Length Holes", "Accuracy Holes", "Imagination Holes", "Facilities on Site"};

int courseGrade(int holes);
SgaResult evaluateCourse(const SgaInput& in);

// Tournament chosen by a total score (0 means improvement required: no tournament). `grandSlam` is whether the course is graded Championship.
const char* tournamentName(int total, bool championshipGrade);
// First prize in thousands: (((total - 50) / 5 + 5) * (grade + 1) * (holes + 1) * 2), C integer division. The second place gets two thirds of it, and so on.
int tournamentPrizeThousands(int total, int grade, int holes);

}  // namespace sg
