// Home sites: lot value, the monthly value pass, the celebrity sale rule, the Silver member counter and the Home Site Value map colour.
// Pure logic (no SDL/GL). Rules from docs/DECODE_HOMES.md (read from the publisher exe); tile index is x * 50 + y, money in units of $100.
// PLACEHOLDER items are marked with that word in homes.cpp.
#pragma once
#include <cstdint>
#include <vector>
#include "sg/buildings.h"

namespace sg {
namespace homes {

// ---- lot value (routine 0x42ef40) ----
// One hole as the lot value reads it. `plays` is the hole's play counter (a hole with no plays scores nothing), `moodSum` its mood total.
struct HoleInfo {
    bool exists = false;
    int plays = 0, moodSum = 0;
    bool top100 = false, top18 = false;   // PLACEHOLDER: the exe's ranking flags (0x575eb8 bits 0 and 1) are not computed by the port, so they stay false
    int teeX = 0, teeY = 0, greenX = 0, greenY = 0;   // tile coordinates
};
struct LotContext {
    const BuildTile* tiles = nullptr;   // 50 x 50 tiles, id 21 under a home site, 20 or owned == false means outside the course
    int theme = 0;                      // viewer theme index: wetlands (id 18) add +16 on theme 1 (the exe's test is "theme byte equals 1")
    int difficulty = 1;                 // the exe's global 0x822c88
    int marinaEffect = 0;               // Marina effect level E (connected marinas only)
    const HoleInfo* holes = nullptr;
    int holeCount = 0;
};
// Per-tile term of the ring sum (routine 0x42ee80).
int ringTerm(const LotContext& c, int x, int y);
// Ring sum R around the 2 x 2 footprint whose minimum corner is (x, y).
int ringSum(const LotContext& c, int x, int y);
// Score of one hole before the distance falloff: 1000 * moodSum / (plays + 4) + (3 - difficulty) * 100, plus 100 per ranking flag.
int holeScore(const HoleInfo& h, int difficulty);
// Best hole score divided by (distance + 8), floor 0, before the Marina term. Distance is the smaller of the tee and green distances in tiles.
int bestHoleTerm(const LotContext& c, int x, int y);
// Lot value in units: (M * R) / 40 with M raised by the Marina term (M + E * M / 3).
int lotValue(const LotContext& c, int x, int y);

// ---- money ----
constexpr int kMinLotValue = 50;
inline int clubShare(int lot) { return lot / 4; }                    // paid to the Home Sites row at placement
inline int removalCharge(int lot, int buyerField) { return lot / 2 + buyerField / 50; }   // the exe divides the buyer field, not the value field (read as stated)

// ---- value pass and sale (monthly, per site) ----
inline int nextValue(int v, int lot) { return lot + v - v / 12; }    // steady state 12 * lot
inline int saleThreshold(int salesSoFar, int difficulty) { return (300 * salesSoFar + 400) * (difficulty + 2); }
// 0 sign, 1 under construction, 2 and 3 finished house, 4 house with decorations (for a site that is not sold).
inline int sizeClass(int v) { return v <= 0 ? 0 : v < 200 ? 0 : v < 600 ? 1 : v < 1200 ? 2 : v < 2000 ? 3 : 4; }

struct Site {
    int x = 0, y = 0;
    int buyer = 0;     // 0 for sale, else celebrity index + 1
    int value = 0;     // the smoothed value V
};
// One monthly pass over one site. Returns true when the site sold on this pass: the caller then posts the ticker message, plays the sound, logs the
// highlight and spawns the resident. `tickerFree` is false when a message is showing (the sale is then skipped until next month).
// `pickCelebrity` returns a uniformly random loaded celebrity index (0 based); it is only called when the sale test passes.
template <class Pick>
bool monthlyPass(Site& s, int lot, int& salesSoFar, int difficulty, bool tickerFree, Pick&& pickCelebrity) {
    s.value = nextValue(s.value, lot);
    if (s.buyer != 0) return false;
    if (s.value <= saleThreshold(salesSoFar, difficulty)) return false;
    if (!tickerFree) return false;
    s.buyer = pickCelebrity() + 1;
    salesSoFar++;
    return true;
}

// ---- the Home Site tool gate ----
// Free sites = Silver-or-better (tier 3 and up) members who have not resigned, minus every home site record (sold or not). The same number is the
// "Waiting list" figure. The tool is unusable while this is below 1.
inline int freeSites(int silverOrBetterActive, int homeSiteRecords) { return silverOrBetterActive - homeSiteRecords; }

// ---- Home Site Value map (overview mode 2) ----
// site = site work in units from the placement scan (-1 when the lot cannot be built). Returns the green channel 0..31 or -1 for dark red.
inline int mapGreen(int lot, int siteWork) {
    if (siteWork < 0) return -1;
    int q = (lot / 4 - siteWork) * 3 / 2;
    if (q < 0) q = 0;
    if (q > 255) q = 255;
    return q / 8;
}

// ---- text ----
const char* unattractiveLotText();                 // refusal at placement
const char* celebrityKindWord(char typeLetter);   // "action star", "pop sensation", ... by celebrities.dta type letter A..K

}  // namespace homes
}  // namespace sg
