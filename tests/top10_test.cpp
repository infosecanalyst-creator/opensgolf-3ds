#include "sg/top10.h"
#include <cstdio>
#include <cstring>
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); return 1; } } while (0)
int main() {
    sg::SocialRng rng(5);
    sg::Top10 t; t.makeDefault(rng);
    for (int i = 1; i < 10; i++) CHECK(t.e[i - 1].score() >= t.e[i].score());   // the default table is sorted
    uint8_t buf[1560]; t.toBytes(buf);
    sg::Top10 u; CHECK(u.fromBytes(buf, sizeof buf));
    CHECK(std::strcmp(u.e[3].name, t.e[3].name) == 0 && u.e[7].cash == t.e[7].cash && u.e[9].difficulty == t.e[9].difficulty);
    sg::DesignerEntry n; std::strcpy(n.name, "me"); n.courseId = 3; n.fun = 2000; n.skill = 1500; n.cash = 9000; n.difficulty = 3;
    CHECK(t.insert(n) == 0 && t.e[0].courseId == 3 && std::strcmp(t.e[9].name, u.e[8].name) == 0);   // takes the top, the old last entry drops off
    sg::DesignerEntry m = n; m.fun = 10; CHECK(t.insert(m) == -1);                                        // the same course ranks only once above it
    sg::DesignerEntry w = n; w.courseId = 4; w.fun = 1; w.skill = 1; w.cash = 1; w.difficulty = 0; CHECK(t.insert(w) == -1);   // too low
    sg::DesignerEntry b = n; b.courseId = 3; b.fun = 3000; CHECK(t.insert(b) == 0 && t.e[1].courseId == -1);   // a better entry of the same course replaces the old one
    std::printf("top10 ok\n"); return 0;
}
