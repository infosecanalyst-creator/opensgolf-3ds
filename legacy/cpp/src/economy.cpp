#include "sg/economy.h"
#include <algorithm>

namespace sg {

// PLACEHOLDER upkeep per tile per game day, by tile type.
static double tileUpkeep(int type) {
    switch (type) {
        case TT_PuttingGreen: case TT_TrickyGreen: return 3.0;
        case TT_Tee: return 2.0;
        case TT_Fairway: case TT_FirmFairway: return 1.5;
        case TT_Rough: case TT_GrassySand: return 0.4;
        case TT_PotSandBunker: case 7: case TT_SandBunker1: case TT_SandBunker1 + 1: case TT_SandBunker1 + 2:
        case TT_SandBunker1 + 3: case TT_ZenSand: case TT_GrassBunker: return 1.0;
        case TT_FlowerBed: return 1.2;
        case TT_Building: return 5.0;
        case TT_WaterShallow: case TT_WaterMiddle: case TT_WaterDeep: case TT_Marsh: return 0.3;
        default: return 0.1;
    }
}

double Economy::upkeepFor(const Terrain& t) {
    double sum = 0;
    for (uint8_t ty : t.type) sum += tileUpkeep(ty) * 0.2;
    for (uint8_t p : t.pathKind) sum += p == 2 ? 0.12 : (p == 1 ? 0.06 : 0.0);
    for (uint8_t w : t.wallMask) for (int b = 0; b < 4; b++) if (w & (1 << b)) sum += 0.05;
    return sum;
}

void Economy::init(const Terrain& t) {
    for (int& n : staff) n = 0;
    wagesPaid = 0; fun = 50;
    cash = startCash; day = 1; holesPlayed = 0; daysInRed = 0; debtStage = 0; notice = ""; gameOver = false; income = upkeepPaid = 0; clock_ = 0;
    updateUpkeep(t);
    version++;
}

void Economy::updateUpkeep(const Terrain& t) { dailyUpkeep = upkeepFor(t) * kMoneyScale; }

void Economy::holeCompleted(double feeUnits) {
    if (gameOver) return;
    const double fee = feeUnits * kUnit;
    cash += fee; income += fee; holesPlayed++; version++;
}

const char* Economy::staffName(int k) {
    static const char* n[StaffKinds] = {"Club Pro", "Ranger", "Groundskeeper", "Soda Vendor"};
    return k >= 0 && k < StaffKinds ? n[k] : "?";
}

// PLACEHOLDER daily wages.
double Economy::dailyWages() const {
    static const double w[StaffKinds] = {30, 25, 20, 15};
    double sum = 0;
    for (int k = 0; k < StaffKinds; k++) sum += w[k] * staff[k];
    return sum * kMoneyScale;
}

bool Economy::hire(int kind) {
    if (kind < 0 || kind >= StaffKinds || (!sandbox && cash < 100)) return false;   // PLACEHOLDER: hiring needs some cash in hand
    staff[kind]++; version++;
    return true;
}

bool Economy::fire(int kind) {
    if (kind < 0 || kind >= StaffKinds || staff[kind] == 0) return false;
    staff[kind]--; version++;
    return true;
}

void Economy::holeFinished(int strokes, int par, double feeUnits) {
    holeCompleted(feeUnits);
    // PLACEHOLDER mood model: fun is a running average of how golfers feel after each hole (the game text says it is the golfers' comments
    // averaged). Par or better pleases them, a bad hole annoys them, and each employee kind nudges it a little: the Club Pro makes golfers
    // feel welcome and the Soda Vendor serves the thirsty.
    double mood = strokes <= par ? 85 : (strokes <= par + 1 ? 55 : 20);
    mood += 8.0 * (staff[ClubPro] > 0) + 6.0 * (staff[SodaVendor] > 0);
    mood = mood > 100 ? 100 : mood;
    lastMood = mood;
    funEvent(0.12 * (mood - fun));
}

void Economy::step(double dt) {
    if (gameOver) return;
    clock_ += dt;
    while (clock_ >= dayLength) {
        clock_ -= dayLength;
        day++; version++;
        if (sandbox) continue;   // unlimited funds: nothing is charged and the game cannot end
        cash -= dailyUpkeep + dailyWages(); upkeepPaid += dailyUpkeep; wagesPaid += dailyWages();
        daysInRed = cash < 0 ? daysInRed + 1 : 0;
        if (daysInRed == 0) debtStage = 0;
        else if (daysInRed >= graceDays) { gameOver = true; notice = "The board has ended your contract. Game over."; }
        else if (daysInRed >= graceDays * 3 / 4 && debtStage < 3) { debtStage = 3; notice = "The board is very worried about the lingering debt."; }
        else if (daysInRed >= graceDays / 2 && debtStage < 2) { debtStage = 2; notice = "The board is concerned about the club's negative cash."; }
        else if (debtStage < 1) { debtStage = 1; notice = "Warning: you have two years to get the club back into the black."; }
    }
}

}  // namespace sg
