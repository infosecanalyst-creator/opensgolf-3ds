// Ball flight and range rules read from the publisher's golf.exe (see docs/PUBLISHER_EXE_NOTES.md, "Maximum range and ball flight").
// All integer maths as in the exe: one tile is 1024 units, and a shot "range" of 25 is one tile. What a range unit is in yards is NOT known;
// the viewer uses kRangeUnitsPerTile and a placeholder tick rate, both marked below.
#pragma once
#include <algorithm>

namespace sg {
namespace flight {

constexpr int kRangeUnitsPerTile = 25;   // from the exe: target distance in world units = range * 1024 / 25
constexpr float kTicksPerSecond = 40.0f; // PLACEHOLDER: the exe's tick length is not decoded

// Coarse range estimate the exe uses while searching for the launch speed (its routine at 0x4223f0): steps stand for two ticks.
inline int coarseDistance(int speed, int vertical) {
    int dist = 0, height = 0;
    do {
        dist += (speed + (speed < 0 ? 7 : 0)) >> 3;
        height += (vertical + (vertical < 0 ? 15 : 0)) >> 4;
        speed -= speed >> 4;
        vertical -= 128;
    } while (height > 0);
    return dist;
}

// The vertical launch speed for a shot of the given range, and the horizontal speed found by bisection so the carry equals 4/5 of the range.
struct Launch { int speed = 0, vertical = 0; };
inline Launch launchFor(int range) {
    Launch l;
    const int u = range * 20 / 25;
    int est = (u * 33 - (u * u) / 48) + 64;
    l.vertical = ((est + (est < 0 ? 7 : 0)) >> 3) + 512;
    const int carry = range * 4 / 5;
    const int target = (carry << 10) / 25;
    int speed = est, step = est / 2;
    do {
        const int d = coarseDistance(speed, l.vertical);
        if (target < d) speed -= step;
        if (d < target) speed += step;
        step /= 2;
    } while (step > 2);
    l.speed = speed;
    return l;
}

// Flight of one shot with the exe's per-tick rules: returns ticks in the air, the distance covered (in 1024 units per tile) and the peak height.
struct Arc { int ticks = 0; float distance = 0, peak = 0; };
inline Arc simulate(int range) {
    const Launch l = launchFor(range);
    int speed = l.speed, vert = l.vertical, height = 0;
    long long dist = 0;
    Arc a;
    do {
        dist += speed >> 4;
        height += vert >> 5;
        speed -= speed >> 5;
        vert -= 64;
        a.peak = std::max(a.peak, (float)height);
        a.ticks++;
    } while (height > 0 && a.ticks < 4000);
    a.distance = (float)dist;
    return a;
}

// Maximum range of a shot before the cap of 330, from the exe's routine at 0x422530. Inputs follow the exe's meaning as far as it is decoded:
// baseByte is the per-golfer byte used on difficulties above 0, lengthDigit and accuracyDigit are 0..9 skill digits (-1 when the golfer lacks the skill flag),
// pro adds the pro bonuses, hazard is the terrain hazard severity under the ball (0..3 after clamping), onTee is true for a tee shot.
inline int maxRange(int difficulty, int baseByte, int lengthDigit, int accuracyDigit, bool pro, int hazard, bool onTee) {
    int r = difficulty < 1 ? (pro ? 40 : 25) : (baseByte * 50) / 3;
    r += 150 + (pro ? 50 : 0);
    if (lengthDigit >= 0) r += -20 + lengthDigit * 4;
    if (accuracyDigit >= 0 && onTee) r += (accuracyDigit - 5) * 6;
    hazard = std::max(0, std::min(3, hazard));
    if (hazard > 0) r -= (hazard * r) / 8;
    if (!onTee) r -= r / 5;
    return std::min(r, 330);
}

}  // namespace flight
}  // namespace sg
