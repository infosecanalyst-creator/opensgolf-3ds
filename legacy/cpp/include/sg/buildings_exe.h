// Building levels, site-work cost, amenity rules, facility and employee effects, Golfers panel layout and undo codes,
// read from the publisher-supplied golf exe (see docs/DECODE_BUILDINGS.md for the evidence and the confidence of each item).
// Pure data and tiny inline helpers: no SDL/GL. Money is in stored units of $100 (display = units * 100).
// Tags: EXACT (table or code read), DERIVED (shape read, meaning inferred), UNKNOWN (value read, purpose not settled).
#pragma once
#include <cstdint>

namespace sg {
namespace buildings_exe {

// ---------------------------------------------------------------- building types (same numbering as sg::costs::Building)
enum Type : int {
    Pathway = 0, Benches, FlowerBed, BallWasher, Landmark, HomeSite, PuttingGreen, SnackBar, ProShop, SwimClub,
    DrivingRange, CartGarage, Marina, ResortHotel, Airstrip, Clubhouse, WillowTree, TvTower, TvBooth, ScenicBridge,
    TypeCount
};

constexpr int kMaxBuildingRecords = 256;   // EXACT: 16-byte records, stamp refuses a 257th
constexpr int kMapSide = 50;               // EXACT: tile index is x * 50 + y

// ---------------------------------------------------------------- 1. levels and footprints (EXACT)
// Level L of a type is 0 (none built), 1 (built once) or 2 (upgraded). Types 6 and up except 7 (snack bar, forced 0) and 5 carry a level.
// An upgrade is "build it again": the old building is demolished, the new one is stamped with one more tile per side.
constexpr int kMaxLevel = 2;
constexpr int kUpgradeMinGrade = 2;        // placing at level 1 or more needs course grade >= 2 (ten or more holes built)
inline constexpr bool typeHasLevel(int type) { return type >= PuttingGreen && type != SnackBar && type != HomeSite; }
// Footprint side of the building as built, by level (level 0 = not built: the side a first placement scans).
//   scanned when placing = base + L (L = level before the placement); side of a built level n = base + n - 1.
inline constexpr int placementScanSide(int baseSide, int levelBefore) { return baseSide + levelBefore; }
inline constexpr int builtSide(int baseSide, int level) { return level > 0 ? baseSide + level - 1 : 0; }
// Base footprint side per type (table byte at +0x10 of the 0x14 byte entries).
constexpr int kBaseSide[TypeCount] = {1, 1, 1, 1, 1, 2, 3, 2, 2, 3, 5, 2, 2, 4, 6, 4, 1, 1, 1, 1};
// Base cost in units per type (table short at +0x12).
constexpr int kBaseCostUnits[TypeCount] = {1, 2, 5, 50, 15, 10, 100, 150, 200, 300, 250, 400, 1000, 2500, 5000, 200, 25, 10, 10, 100};
// Placement price = base * (L + 2) / 2 (integer, toward zero) + site-work term. The pathway halves the site term first.
inline constexpr int placementCostUnits(int type, int levelBefore, int siteUnits) {
    return kBaseCostUnits[type] * (levelBefore + 2) / 2 + siteUnits;
}
// Income per visit by level, paid to Food/Drink (EXACT): index [type][level 0..2]; levels 0 and 1 pay the first figure, 2 the second.
//   Snack Bar 5 flat; Putting Green 4/8; Pro Shop 6/10; Driving Range 8/12. Ball washer, swim club and the rest pay nothing.
inline constexpr int visitIncomeUnits(int type, int level) {
    switch (type) {
        case SnackBar: return 5;
        case PuttingGreen: return level >= 2 ? 8 : 4;
        case ProShop: return level >= 2 ? 10 : 6;
        case DrivingRange: return level >= 2 ? 12 : 8;
        default: return 0;
    }
}

// ---------------------------------------------------------------- 2. placement validity and site-work term (EXACT, routine 0x40db90)
// Per tile id (0..19): site-work cost and tile class, from the tile table copied to 0x578350 (fields +0x24 and +0x26).
// A tile contributes its cost when: class == 13, or id == 12 (rock), or id == 17 (shallow water) unless the type is the marina,
// or id == 18 (marsh), or id == 19 (overgrowth). All other tiles cost nothing to clear.
struct TileSite { int8_t cost; int8_t tileClass; };
constexpr TileSite kTileSite[20] = {
    {0, 2},  {0, 1},  {0, 2},  {0, 2},  {0, 4},  {0, 4},  {0, 4},  {0, 7},  {0, 8},  {0, 7},   // 0..9
    {0, 4},  {0, 4},  {5, 4},  {5, 13}, {5, 13}, {5, 13}, {10, 13}, {10, 17}, {50, 4}, {100, 4}  // 10..19
};
constexpr int kSiteTileRock = 12, kSiteTileShallowWater = 17, kSiteTileMarsh = 18, kSiteTileOvergrowth = 19, kSiteClassTrees = 13;
constexpr int kTileBuildingHome = 21, kTileBuildingOther = 22;   // ids stamped under a building (home site uses 21, the rest 22)
inline constexpr int siteCostOfTile(int tileId, int type) {
    return (tileId < 0 || tileId > 19) ? 0
         : (kTileSite[tileId].tileClass == kSiteClassTrees || tileId == kSiteTileRock || tileId == kSiteTileMarsh ||
            tileId == kSiteTileOvergrowth || (tileId == kSiteTileShallowWater && type != Marina))
               ? kTileSite[tileId].cost : 0;
}
// Tile flag bits (EXACT).
constexpr uint16_t kFlagPath = 0x20, kFlagConnected = 0x40, kFlagGreenTee = 0x80, kFlagScenic = 0x100, kFlagBench = 0x200,
                   kFlagBuilding = 0x400, kFlagWeeds = 0x800, kFlagFlowers = 0x1000, kFlagWeedTimer = 0x4000, kFlagLocked = 0x8000;
// Rejection rules (a tile outside the map or unowned also rejects; the weeds flag does not):
//   interior tile id 0 (tee) or 21 (home-site tile) rejects; id 22 or a building flag rejects unless the same type is there (that is how an
//   upgrade overlaps its predecessor); flags 0x8080 (locked, green or tee) reject; the marina on Parkland or Desert needs every interior tile to be
//   water 17. The one-tile ring around the footprint must contain at least one non-water tile (pathway and landmark are exempt).
//   A pathway only checks bounds. Home site also needs lot value >= 50 units (routine 0x42ef40).
constexpr int kHomeSiteMinLotValueUnits = 50;
constexpr int kMarinaThemeMask = 0x5;      // themes 0 and 2 (bit set) force the marina onto water

// ---------------------------------------------------------------- 3. amenities (EXACT unless marked)
// Undo codes written per tile when the player places an item (byte at 0x5830b8; refund byte at 0x59c090 = what was paid, signed):
//   0xff nothing to undo; 0x80 pathway; 0x81 bench; 0x82 flower bed; 0x90 tree (willow); 0x93 bridge; 0xa0 terrain paint with the previous tile code
//   in the low bits (DERIVED); values below 0x7d are previous tile ids from the terrain painter.
constexpr uint8_t kUndoNone = 0xff, kUndoPath = 0x80, kUndoBench = 0x81, kUndoFlower = 0x82, kUndoTree = 0x90, kUndoBridge = 0x93;
constexpr int kBenchVariants = 5;          // bench picture index = selection counter % 5
constexpr int kBenchSearchHalfWidth = 4;   // a tired golfer scans a 9 x 9 tile window for a bench tile
constexpr int kBenchRadiusTiles = 2;       // nearest bench must be within 2 tiles (4 tiles once the golfer is already heading there)
constexpr int kBenchRestingTiredThreshold = 59;   // event 0x1b gives +1 mood only when fatigue was above 59
constexpr int kLandmarkUglyKindFrom = 16;  // kinds 0..15 give the "nice" glance (+1), kinds 16 and up give the "ugly" one (-2)
inline constexpr int landmarkCostUnits(int kind, bool donated) { return donated ? 0 : (5 * kind + 25) * 2; }
inline constexpr int landmarkRefundUnits(int kind) { return (kind + 5) * 10; }
// Scenic glance (golfer looks one tile ahead in a random near-forward direction): chance gate is (tick + golfer * 0x21) & mask == 0 with
// mask 0xff normally (DERIVED: 0x1ff when neither flag bit of the golfer's attribute word 0xc000 is set, 0x3f when 0x4000 is set, halved again
// while a short counter is positive). Results: landmark kind < 16 +1 (event 0x0b), landmark kind >= 16 -2 (event 0x14); flower tile +1 only for golfers
// whose taste roll equals 2; any tile with both flowers and weeds gives -2 (event 0x14, weeds win).
constexpr int kScenicMaskDefault = 0xff;

// ---------------------------------------------------------------- 4. facilities as visited by golfers (EXACT)
// A facility works only while its record has the connected bit (a path link to the clubhouse) on difficulty 1 to 3; on difficulty 0 buildings
// with a level always count as connected. Effect level E equals the level L of the type when connected, else 0 (snack bar: always 0).
struct FacilityVisit {
    int type;
    int radiusTiles;          // search radius from the golfer to the footprint centre
    int radiusWhileHeading;   // radius once the golfer has already committed (hysteresis)
    int golferAttrBit;        // attribute bit the golfer must have (0 = any golfer)
    int pauseBase;            // pause ticks = pauseBase + random(pauseSpread) (EXACT values: golfer timer goes to -(base + roll))
    int pauseSpread;
    int moodEvent;            // exe event id fired on use
};
constexpr FacilityVisit kFacilityVisits[] = {
    {SnackBar,      8, 10, 0, 48, 0, 0x12},   // attr none; sets hunger and thirst to 0; pays 5; mood +1 only if hunger was above 7
    {PuttingGreen,  5,  7, 4, 16, 9, 0x35},   // imaginative golfers, normal kind, once per round (golfer flag 0x10)
    {ProShop,       5,  7, 2, 16, 9, 0x34},   // accurate golfers, once per round (golfer flag 0x08)
    {DrivingRange,  6,  9, 1, 16, 9, 0x33},   // long hitters, once per round (golfer flag 0x04)
    {BallWasher,    3,  3, 0, 12, 8, 0x29},   // tile-object type 3 near the golfer; once per hole (flag 0x4000000); accuracy counter +3
};
constexpr int kBallWasherIncomeUnits = 0;   // no payment
constexpr int kSnackBarHungerThresholdBasic = 8, kSnackBarHungerThresholdHeading = 4;   // DERIVED (hunger needed to go looking)
// Mood events 0x33, 0x34 and 0x35 give +1 only on difficulty 0 (EXACT, see PUBLISHER_EXE_NOTES).

// Effect magnitudes by effect level E (0 none, 1 built, 2 upgraded). Each helper returns the exe's integer result.
// Putting Green (E6): imaginative golfers (attribute bit 4, normal kind): base value 20 (12 when game flag 0x5a47e0 bit 1 set) grows by value / (4 - E).
inline constexpr int imaginationBoost(int base, int e6) { return e6 ? base + base / (4 - e6) : base; }
// Pro Shop (E8): the error spread of accurate golfers (attribute bit 2, or golfer flag 1) is divided by (E + 2); applied even at E = 0 for them.
inline constexpr int accuracyDivisor(int e8) { return e8 + 2; }
// Driving Range (E10): long hitters (attribute bit 1) drive 15 units further per level; driving range LEVEL L also trims long-hole length (below).
inline constexpr int driveBonus(int e10) { return e10 * 15; }
constexpr int kHoleLengthTrimThreshold = 300, kHoleLengthTrimPerLevel = 25;   // length above 300 loses 25 per driving range level L (not E)
inline constexpr int holeLengthAfterRange(int length, int rangeLevel) { return length > kHoleLengthTrimThreshold ? length - kHoleLengthTrimPerLevel * rangeLevel : length; }
// Swim Club (E9): a newly arrived golfer starts at mood E + 4 instead of 3 + random(3) (4 on difficulty 0). Also adds E to a member-progress counter.
inline constexpr int arrivalMood(int e9, int roll0to2, bool difficulty0) { return e9 ? e9 + 4 : (difficulty0 ? 4 : 3 + roll0to2); }
// Cart Garage (E11): arriving golfers get the cart flag 0x10000; riding speed value E * 4 + 8 (12 or 16) replaces the walking value (3..5).
inline constexpr int cartSpeed(int e11) { return e11 * 4 + 8; }
// Marina (E12): home and lot value grows by value * E / 3.
inline constexpr int marinaValue(int value, int e12) { return e12 ? value + (e12 * value) / 3 : value; }
// Resort Hotel (E13): per-hole mood loss divisor becomes (E * 5 + 15) * 8 instead of 120.
inline constexpr int moodDecayDivisor(int e13) { return (e13 * 5 + 15) * 8; }
// Airstrip (E14): the green fee per hole gets + E units.
inline constexpr int airstripFeeBonus(int e14) { return e14; }

// ---------------------------------------------------------------- 5. employees (EXACT unless marked)
// Kind byte stored in the employee record: hire code c gives kind = -2 - c / 3; skilled when c % 3 == 2.
enum EmployeeKind : int { KindClubPro = -2, KindRanger = -3, KindGroundskeeper = -4, KindSodaVendor = -5, KindAdvisor = -6 };
constexpr int kMaxEmployees = 64;                // records of 0x4c bytes
constexpr int kGroundsSearchHalfTiles = 16;       // 33 x 33 tile scan window around the post
constexpr int kGroundsReachBasic = 24, kGroundsReachSkilled = 32;   // weed search distance in half tiles (DERIVED unit)
constexpr int kActionTimerStart = 7;              // action animation counts 7 down once every 4 ticks
constexpr int kGreetMoodDelta = 1;                // Club Pro, event 0x22
constexpr int kVendorThirstMinBasic = 5, kVendorThirstMinSkilled = 1;  // vendor goes to golfers with thirst at least this (thirst > 4 basic, > 0 skilled)
constexpr int kVendorSaleUnits = 2;               // paid to cash and Food/Drink
constexpr int kVendorPauseTicks = 32;             // golfer is held 32 ticks while served
constexpr int kClubProPauseTicks = 16;            // golfer is held 16 ticks while greeted
constexpr int kWeedClearDelayBasic = 3;           // tile weed timer is set to (global value - 3); skilled work finishes faster (DERIVED: delay halved)
constexpr int kHurryFlag = 0x4000;                // golfer flag set by a Ranger: faster walking (+1 step value)
constexpr uint32_t kGolferLeavingFlag = 0x20000000;   // golfer has finished and is heading out

// ---------------------------------------------------------------- 6. Golfers panel (exe mode 2, routine 0x435760) (EXACT)
constexpr int kGolferSlots = 152;                 // 0x98
// Compact layout (when the panel size index is below 4): 7 rows per column, columns 60 px apart, rows 14 px apart.
constexpr int kCompactRowsPerColumn = 7, kCompactColumnStep = 60, kCompactRowStep = 14, kCompactFirstX = 310, kCompactFirstY = 498, kCompactMaxColumnX = 0x2f3;
// Card layout (size index 4 or more): 4 cards per column, columns 121 px apart, cards 21 px apart.
constexpr int kCardRowsPerColumn = 4, kCardColumnStep = 121, kCardRowStep = 21, kCardFirstX = 310, kCardFirstY = 498, kCardMaxColumnX = 0x2b7;
constexpr int kCardNameX = 326, kCardNameYOffset = 5, kCardFaceX = 412, kCardFaceYOffset = 2, kCardHeartsX = 342, kCardHeartsYOffset = 17, kCardHeartStep = 10;
constexpr int kCardHeartSlots = 4;                // four outline hearts, filled ones = story chapter reached
constexpr int kCardHoleYOffset = 15;              // hole number text y offset, x = card x + 5 (two digits) or + 8 (one digit)
constexpr int kScrollBarX = 0x15f, kScrollBarY = 0x24e, kScrollBarHeight = 6, kScrollBarTrack = 400;
constexpr uint32_t kScrollBarColour = 0x8000739f;
// Name colours (RGB555 with bit 31 set), checked in this order of priority: leaving, hungry, thirsty, tired, normal.
constexpr uint32_t kNameNormal = 0x80000000, kNameTired = 0x80004210, kNameThirsty = 0x80000018, kNameHungry = 0x80006000, kNameLeaving = 0x80007d08;
constexpr int kTiredAbove = 160, kThirstyAbove = 16, kHungryAbove = 16;      // golfer counters (fatigue, thirst, hunger)
inline constexpr int moodFaceIndex(int mood) { return mood + 2 < 1 ? 1 : (mood + 2 > 10 ? 10 : mood + 2); }   // sprite 1..10 from mood -10..10
constexpr int kLeavingHoleCode = 19;              // hole index shown as "x" when the golfer is leaving
// Panel hover titles: -2 Golfers, -3 Gary Golf (mascot), -4 Hire Employees.

}  // namespace buildings_exe
}  // namespace sg
