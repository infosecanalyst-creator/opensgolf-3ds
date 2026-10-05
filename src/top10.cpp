#include "sg/top10.h"
#include <cstdio>
#include <cstring>

namespace sg {

void Top10::makeDefault(SocialRng& rng) {
    for (int i = 0; i < kRecords; i++) {
        DesignerEntry& x = e[i];
        x = DesignerEntry();
        x.fun = (int)rng.below(100) + 900 - 100 * i;
        x.skill = (int)rng.below(90) + 810 - 90 * i;
        x.cash = (int)rng.below(800) + 7200 - 800 * i;
        x.difficulty = (int16_t)((9 - i) / 3);
        x.courseId = -1;
        std::strncpy(x.course, "Harbour Lights GC", sizeof x.course - 1);
        std::strncpy(x.name, "a.c. dye", sizeof x.name - 1);
        x.name[0] = (char)(x.name[0] + (int)rng.below(26));
        x.name[2] = (char)(x.name[2] + (int)rng.below(24));
    }
    lastRank = -1;
}

bool Top10::fromBytes(const uint8_t* d, size_t n) {
    if (n < (size_t)kRecords * kRecordBytes) return false;
    for (int i = 0; i < kRecords; i++) {
        const uint8_t* r = d + i * kRecordBytes;
        DesignerEntry& x = e[i];
        std::memcpy(x.name, r, 64); x.name[63] = 0;
        std::memcpy(x.course, r + 0x40, 64); x.course[63] = 0;
        std::memcpy(&x.fun, r + 0x80, 4); std::memcpy(&x.skill, r + 0x84, 4); std::memcpy(&x.cash, r + 0x88, 4);
        std::memcpy(&x.difficulty, r + 0x96, 2); std::memcpy(&x.courseId, r + 0x98, 4);
    }
    lastRank = -1;
    return true;
}

void Top10::toBytes(uint8_t* out) const {
    std::memset(out, 0, (size_t)kRecords * kRecordBytes);
    for (int i = 0; i < kRecords; i++) {
        uint8_t* r = out + i * kRecordBytes;
        const DesignerEntry& x = e[i];
        std::memcpy(r, x.name, 64); std::memcpy(r + 0x40, x.course, 64);
        std::memcpy(r + 0x80, &x.fun, 4); std::memcpy(r + 0x84, &x.skill, 4); std::memcpy(r + 0x88, &x.cash, 4);
        std::memcpy(r + 0x96, &x.difficulty, 2); std::memcpy(r + 0x98, &x.courseId, 4);
    }
}

int Top10::insert(const DesignerEntry& n) {
    const int s = n.score();
    int slot = 0;
    while (slot < kRecords && s < e[slot].score()) {
        if (e[slot].courseId == n.courseId) return -1;   // the same course already ranks higher
        slot++;
    }
    if (slot >= kRecords) return -1;
    int drop = kRecords - 1;
    for (int k = kRecords - 1; k >= slot; k--) if (e[k].courseId == n.courseId) { drop = k; break; }   // the lowest entry of the same course goes first
    for (int k = drop; k > slot; k--) e[k] = e[k - 1];
    e[slot] = n;
    lastRank = slot;
    return slot;
}

bool top10Load(Top10& t, const std::string& path) {
    FILE* f = std::fopen(path.c_str(), "rb");
    if (!f) return false;
    uint8_t buf[Top10::kRecords * Top10::kRecordBytes];
    const size_t n = std::fread(buf, 1, sizeof buf, f);
    std::fclose(f);
    return t.fromBytes(buf, n);
}

bool top10Save(const Top10& t, const std::string& path) {
    FILE* f = std::fopen(path.c_str(), "wb");
    if (!f) return false;
    uint8_t buf[Top10::kRecords * Top10::kRecordBytes];
    t.toBytes(buf);
    const bool ok = std::fwrite(buf, 1, sizeof buf, f) == sizeof buf;
    std::fclose(f);
    return ok;
}

}
