// Surface table of the publisher's golf.exe (23 records of 0x30 bytes at 0x4c1a40, bytes +0x20..+0x27 of each record), read from the unprotected exe's data section.
// Field meanings are from docs/DECODE_PLAYCORE.md section 1 (EXACT). The slope term of the ground friction is not applied by the port (PLACEHOLDER: the port's heights are an approximation).
#pragma once
#include <cstdint>
#include "sg/terrain.h"

namespace sg {

struct Lie {
    uint8_t bounce;      // +0x20 bounce factor k
    uint8_t friction;    // +0x21 roll friction (green: the putt solver's exponent)
    int8_t penalty;      // +0x22 (green -1)
    uint8_t buildCost;   // +0x23
    uint8_t siteCost;    // +0x24
    uint8_t walk;        // +0x25 walk effort
    uint8_t cls;         // +0x26 class: 13 trees, 17 water, 18 out of bounds or building, 7 and 8 sand
    uint8_t sub;         // +0x27
};
constexpr int kLieCount = 23;
constexpr Lie kLie[kLieCount] = {
    {4, 3, 0, 5, 0, 2, 0, 0},   // 0 tees
    {3, 3, -1, 10, 0, 1, 1, 1},   // 1 green
    {4, 3, 0, 3, 0, 1, 2, 2},   // 2 fairway
    {5, 4, 0, 3, 0, 1, 2, 15},   // 3 firm fairway
    {2, 2, 1, 1, 0, 2, 4, 3},   // 4 rough
    {1, 2, 2, 2, 0, 3, 4, 17},   // 5 deep rough
    {2, 2, 2, 4, 0, 3, 4, 4},   // 6 mound
    {1, 1, 3, 6, 0, 4, 7, 5},   // 7 sand trap
    {2, 2, 2, 4, 0, 3, 8, 5},   // 8 waste bunker
    {1, 1, 5, 8, 0, 5, 7, 5},   // 9 pot bunker
    {2, 2, 5, 10, 0, 3, 4, 6},   // 10 ravine
    {1, 1, 3, 4, 0, 3, 4, 6},   // 11 brush
    {5, 1, 3, 4, 5, 4, 4, 9},   // 12 rocks
    {2, 2, 4, 10, 5, 4, 13, 7},   // 13 tree
    {2, 2, 4, 10, 5, 4, 13, 7},   // 14 pine tree
    {2, 2, 4, 10, 5, 4, 13, 6},   // 15 palm tree
    {2, 2, 4, 25, 10, 4, 13, 7},   // 16 elm tree
    {0, 0, 8, 50, 10, 16, 17, 8},   // 17 water
    {2, 1, 3, 2, 50, 4, 4, 14},   // 18 wetlands
    {2, 1, 3, 6, 100, 4, 4, 14},   // 19 marsh
    {2, 2, 8, 10, 0, 12, 18, 6},   // 20 out of bounds
    {2, 2, 4, 0, 0, 4, 18, 16},   // 21 building
    {2, 2, 4, 0, 0, 4, 18, 16},   // 22 building
};
// The port's tile ids are the Terrain.dll tile ids. Ids the exe table has no record for (the editor's own Cliff, Ravine, FlowerBed, ZenSand, GrassBunker, and
// the tricky green and the extra water depths) take the nearest record: a PLACEHOLDER choice.
inline int lieId(int type) {
    switch (type) {
        case TT_Tee: return 0;
        case TT_PuttingGreen: case TT_TrickyGreen: return 1;
        case TT_Fairway: return 2;
        case TT_FirmFairway: return 3;
        case TT_Rough: case TT_FlowerBed: return 4;
        case TT_DeepRough: return 5;
        case TT_GrassySand: case TT_GrassBunker: return 8;
        case TT_PotSandBunker: return 9;
        case TT_SandBunker1: case TT_ZenSand: return 7;
        case TT_Overgrowth: case TT_Ravine: return 10;
        case TT_Brush: return 11;
        case TT_Rock: case TT_Cliff: return 12;
        case TT_Woods: return 13;
        case TT_WaterShallow: case TT_WaterMiddle: case TT_WaterDeep: case TT_WaterShallowDesert: return 17;
        case TT_Marsh: return 18;
        case TT_Overgrowth2: return 19;
        case TT_Building: return 22;
        default: return 4;
    }
}
inline const Lie& lieOf(int type) { return kLie[lieId(type)]; }
inline bool isWater(int type) { return lieOf(type).cls == 17; }

}  // namespace sg
