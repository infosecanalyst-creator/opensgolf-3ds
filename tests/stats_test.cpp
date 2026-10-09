// Checks for the Hole Stats dialog numbers, the roster Low/Hcp rules and the comment sentence builder (docs/DECODE_HOLE_STATS2.md, DECODE_COMMENTS.md).
#include <cstdio>
#include <cstring>
#include "sg/comments.h"
#include "sg/holestats.h"

static int fails = 0;
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); ++fails; } } while (0)

int main() {
    using namespace sg;
    CHECK(signedHundredths(35) == "+0.35"); CHECK(signedHundredths(-120) == "-1.20"); CHECK(signedHundredths(7) == "+0.07"); CHECK(signedHundredths(0) == "+0.00");
    CHECK(!std::strcmp(diffWord(-1), "poor")); CHECK(!std::strcmp(diffWord(0), "fair")); CHECK(!std::strcmp(diffWord(24), "fair")); CHECK(!std::strcmp(diffWord(25), "good"));
    CHECK(!std::strcmp(diffWord(50), "very good")); CHECK(!std::strcmp(diffWord(99), "very good")); CHECK(!std::strcmp(diffWord(100), "outstanding"));
    CHECK(!std::strcmp(funWord(-5), "poor")); CHECK(!std::strcmp(funWord(19), "fair")); CHECK(!std::strcmp(funWord(20), "good")); CHECK(!std::strcmp(funWord(59), "very good")); CHECK(!std::strcmp(funWord(60), "outstanding"));

    // Dialog differentials: an unopened hole shows zero everywhere; golfers lacking a skill scoring worse raise it.
    HoleStats h; h.par = 4;
    CHECK(h.dialogDiff100(kSkillLength) == 0);
    h.hist[7][4] = 10; h.hist[6][5] = 10;               // all-skill golfers make 4, no-length golfers make 5
    // group 7: (32 + 40) / 18 = 4.0 -> 400; group 6: (32 + 50) / 18 = 4.55 -> 455
    CHECK(h.dialogDiff100(kSkillLength) == 455 - 400);
    h.hist[7][1] = 5;                                      // a hole in one is below the first visible bin (f = 2) and ignored
    CHECK(h.dialogVisibleCount() == 20);
    CHECK(h.dialogAvg100() == (4 * 10 + 5 * 10) * 100 / 20);

    // Roster rules: unplayed holes count 5 strokes and 1 over par.
    RoundEnd r = roundEnd(40, 36, 9);
    CHECK(r.adjusted == 40 + 45); CHECK(r.overPar == 4 + 9);
    r = roundEnd(80, 72, 18); CHECK(r.adjusted == 80 && r.overPar == 8);
    CHECK(newLow(0, 85) == 85); CHECK(newLow(90, 85) == 85); CHECK(newLow(80, 85) == 80);
    CHECK(newHandicap(0, 8) == 8); CHECK(newHandicap(8, 12) == 10); CHECK(newHandicap(9, 12) == 10); CHECK(newHandicap(10, -3) == 3);

    // Comment builder.
    CommentCtx x;
    CommentOut o = commentText(26, 20, x); CHECK(o.colour == CommentColour::Neutral && !o.text.empty());       // tired is black though its delta is negative
    o = commentText(1, 20, x); CHECK(o.colour == CommentColour::Good && o.text.find("Pat") != std::string::npos && o.text.find("PARTNER") == std::string::npos);
    o = commentText(9, 20, x); CHECK(o.colour == CommentColour::Bad);
    o = commentText(10, 13, x); CHECK(o.text.find("these trees") == std::string::npos && o.text.find("this tree") != std::string::npos);   // singular name, singular demonstrative
    o = commentText(10, 12, x); CHECK(o.text.find("these rocks") != std::string::npos);                      // ends in s: plural demonstrative
    o = commentText(2, 14, x); CHECK(o.text.find("under the pine tree") != std::string::npos);
    o = commentText(2, 7, x); CHECK(o.text.find("in the sand trap") != std::string::npos);
    o = commentText(30, 20, x); CHECK(o.text.find("par 0") != std::string::npos);                              // equal pars
    x.par = 4; x.prevPar = 3; o = commentText(30, 20, x); CHECK(o.text.find("last hole") != std::string::npos);
    x.flags = x.prevFlags = 0x20; o = commentText(30, 20, x); CHECK(o.text.find("left") != std::string::npos);
    x.flags = x.prevFlags = 0x60; o = commentText(30, 20, x); CHECK(o.text.find("left") != std::string::npos);   // left wins when both bits are shared
    x.flags = x.prevFlags = 0x40; o = commentText(30, 20, x); CHECK(o.text.find("right") != std::string::npos);
    x = CommentCtx(); x.par = 4; x.flags = 4; o = commentText(23, 20, x); CHECK(o.text.find("tough") != std::string::npos && o.text.rfind("Triple eagle", 0) == 0);
    x.flags = 8; o = commentText(23, 20, x); CHECK(o.text.find("easy") != std::string::npos);
    x.par = 5; o = commentText(23, 20, x); CHECK(o.text.rfind("Argh", 0) == 0);
    x = CommentCtx(); x.tick = 0; const std::string a = commentText(35, 0, x).text; x.tick = 80; const std::string b = commentText(35, 0, x).text; CHECK(a != b);
    CHECK(commentText(36, 0, x).text != commentText(36, 1, x).text);
    CHECK(commentText(34, 5, x).text.empty() && commentText(34, 5, x).colour == CommentColour::Good);
    CHECK(commentText(0, 0, x).text.empty() && commentText(64, 0, x).text.empty());
    x.celebrity = "Test Star"; CHECK(commentText(22, 0, x).text.find("Test Star") != std::string::npos);
    CHECK(commentText(39, 4, x).text.find("crocodile") != std::string::npos);

    // Object phrases.
    CellInfo c; c.tile = 22; c.landmarkKind = 7; CHECK(cellObjectName(c, 0) == "lighthouse");
    c = CellInfo(); c.tile = 22; CHECK(cellObjectName(c, 0) == "landmark"); c.tile = 21; CHECK(cellObjectName(c, 0) == "home site");
    c = CellInfo(); c.tile = 17; CHECK(cellObjectName(c, 0) == "fountain"); CHECK(cellObjectName(c, 1) == "rock formation"); c.bridge = true; CHECK(cellObjectName(c, 0) == "scenic bridge");
    c = CellInfo(); c.tile = 3; CHECK(cellObjectName(c, 0) == "wildflower");
    c = CellInfo(); c.tile = 16; CHECK(cellObjectName(c, 0) == "scenic tree"); c.tile = 15; CHECK(cellObjectName(c, 0) == "statue");
    CHECK(defaultHoleName(3, 1) == "Alexandra"); CHECK(defaultHoleName(4, 18) == "Holly"); CHECK(defaultHoleName(5, 1) == "Pride");

    if (!fails) std::printf("stats_test: all passed\n");
    return fails ? 1 : 0;
}
