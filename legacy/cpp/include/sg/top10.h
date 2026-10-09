// The Top 10 Designers table (docs/DECODE_TOP10_PAIR.md section 1): file format of top10.sve, the score formula, the insert rules and the default table.
#pragma once
#include <cstdint>
#include <string>
#include "sg/rng.h"

namespace sg {

struct DesignerEntry {
    char name[64] = {};
    char course[64] = {};
    int32_t fun = 0, skill = 0, cash = 0;   // skill in hundredths, cash in units of $100
    int16_t difficulty = 0;
    int32_t courseId = -1;
    // (cash / 10 + skill + fun) * (difficulty + 1)
    int score() const { return (cash / 10 + skill + fun) * (difficulty + 1); }
};

struct Top10 {
    static constexpr int kRecords = 10, kRecordBytes = 0x9c;
    DesignerEntry e[kRecords];
    int lastRank = -1;                                   // the slot the newest entry took, for the highlight
    void makeDefault(SocialRng& rng);                    // the table the exe writes when the file is missing
    bool fromBytes(const uint8_t* data, size_t n);       // 1560 bytes
    void toBytes(uint8_t* out) const;
    // Inserts an entry; returns its rank (0 based) or -1 when it did not make the table (same course already ranked above it, or too low).
    int insert(const DesignerEntry& n);
};

bool top10Load(Top10& t, const std::string& path);
bool top10Save(const Top10& t, const std::string& path);

}
