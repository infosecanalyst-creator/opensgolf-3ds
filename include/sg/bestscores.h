// "Best N Hole Scores" table (docs/DECODE_TOP10_PAIR.md section 2). One table per course, ten entries, lowest total strokes first.
#pragma once
#include <cstdio>
#include <string>

namespace sg {

struct BestScores {
    static constexpr int kMax = 10;
    int count = 0;
    int score[kMax] = {};
    char name[kMax][32] = {};

    // Ascending insert; a tie goes after the entries it equals. Returns the 0-based rank, or -1 when the score does not make the ten.
    int insert(int s, const std::string& who) {
        if (s <= 0) return -1;
        int at = 0;
        while (at < count && score[at] <= s) at++;
        if (at >= kMax) return -1;
        const int last = count < kMax ? count : kMax - 1;
        for (int i = last; i > at; i--) { score[i] = score[i - 1]; std::snprintf(name[i], sizeof name[i], "%s", name[i - 1]); }
        score[at] = s; std::snprintf(name[at], sizeof name[at], "%s", who.c_str());
        if (count < kMax) count++;
        return at;
    }
    int record() const { return count ? score[0] : 0; }
    // The course record popup (exe: more than two holes, the score ties or beats the old record, and is not over par).
    bool setsRecord(int s, int holes, int parSum) const { return holes > 2 && s > 0 && s <= parSum && (count == 0 || s <= score[0]); }
};

}  // namespace sg
