#include "sg/charrec.h"
#include <cstdio>
#include <cstring>
using namespace sg;
static int fails = 0;
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); fails++; } } while (0)
int main() {
    CharRec c; std::strcpy(c.name, "Test Golfer"); std::strcpy(c.title, "Pro"); c.flagB = 0x80; c.head = 7; c.skills[3] = 9; std::strcpy(c.dialogue[0], "Hello");
    c.cycleBody(true); CHECK(c.bodyType() == 1); c.cycleBody(false); c.cycleBody(false); CHECK(c.bodyType() == 3);
    c.cycleShirt(false); CHECK(c.shirt == 9); c.cycleShirt(true); CHECK(c.shirt == 0);
    c.cycleSkin(false); CHECK(c.skin == 3); c.cycleHair(false); CHECK(c.hair == 4);
    CHECK(c.age() == -1); c.cycleAge(true); CHECK(c.age() == 0); c.cycleAge(true); c.cycleAge(true); c.cycleAge(true); CHECK(c.age() == 0);
    c.cycleMarital(false); CHECK(c.marital() == 3); c.cycleMarital(true); CHECK(c.marital() == 0);
    CHECK((c.flags & 0x80) != 0 && c.female());
    CharRec d; CHECK(charFromBytes(charToBytes(c), d));
    CHECK(!std::strcmp(d.name, "Test Golfer") && d.head == 7 && d.skills[3] == 9 && !std::strcmp(d.dialogue[0], "Hello") && d.flagB == c.flagB && d.flags == c.flags);
    CHECK(charNameValid("Gary Golf") && !charNameValid("") && !charNameValid("a/b") && !charNameValid("x*"));
    std::printf(fails ? "charrec: %d failures\n" : "charrec: all passed\n", fails);
    return fails != 0;
}
