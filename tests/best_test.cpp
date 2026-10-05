#include "sg/bestscores.h"
#include <cstdio>
#include <cstring>
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); return 1; } } while (0)
int main() {
    sg::BestScores b;
    CHECK(b.setsRecord(70, 18, 72) && !b.setsRecord(73, 18, 72) && !b.setsRecord(2, 2, 3));
    CHECK(b.insert(80, "a") == 0 && b.insert(75, "b") == 0 && b.insert(80, "c") == 2 && b.count == 3);   // ties go after
    CHECK(std::strcmp(b.name[0], "b") == 0 && std::strcmp(b.name[2], "c") == 0);
    for (int i = 0; i < 10; i++) b.insert(60 + i, "x");
    CHECK(b.count == 10 && b.score[0] == 60 && b.score[9] == 69);
    CHECK(b.insert(90, "late") == -1 && b.insert(65, "mid") == 6 && b.score[9] == 68);
    CHECK(b.setsRecord(60, 18, 72) && !b.setsRecord(61, 18, 72));
    std::printf("best ok\n"); return 0;
}
