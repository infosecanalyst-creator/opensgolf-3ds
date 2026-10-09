#include "sg/economy.h"
#include <algorithm>
#include "sg/costs.h"

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
    for (int& n : skilled) n = 0;
    wagesPaid = 0; fun = 50;
    cash = startCash; day = 1; holesPlayed = 0; debtStage = 0; notice = ""; gameOver = false; income = upkeepPaid = 0; clock_ = 0;
    updateUpkeep(t);
    version++;
}

// Exe rule (docs/EXE_COSTS.md): every hole costs 1 unit per period, with (difficulty + 2) periods a month. Tile upkeep is not in the exe.
void Economy::updateUpkeep(const Terrain&) { dailyUpkeep = holes * costs::kHoleUpkeepUnits * kUnit * (difficulty + 2); }

void Economy::holeCompleted(double feeUnits) {
    if (gameOver) return;
    const double fee = feeUnits * kUnit;
    cash += fee; income += fee; holesPlayed++; version++;
    book(costs::GreensFees, fee);
}

const char* Economy::staffName(int k) {
    static const char* n[StaffKinds] = {"Club Pro", "Ranger", "Groundskeeper", "Soda Vendor"};
    return k >= 0 && k < StaffKinds ? n[k] : "?";
}

// Expected wages for a month: each employee draws (difficulty + 2) wage events, each paid when a roll in [0, 4 - difficulty) is at most the course grade
// (docs/EXE_COSTS.md). Basic wages by kind: Club Pro 3, Ranger 2, Groundskeeper 2, Soda Vendor 2 units. Skilled staff are not modelled yet.
double Economy::dailyWages() const {
    const int g = costs::courseGrade(holes) < 0 ? 3 : costs::courseGrade(holes);
    const int span = 4 - difficulty;
    const double p = span <= 0 ? 1.0 : std::min(1.0, (g + 1.0) / span);
    double sum = 0;
    for (int k = 0; k < StaffKinds; k++) sum += (costs::wageUnits(k, false) * (staff[k] - skilled[k]) + costs::wageUnits(k, true) * skilled[k]) * kUnit * (difficulty + 2) * p;
    return sum;
}

bool Economy::hire(int kind, bool skilledHire) {
    if (kind < 0 || kind >= StaffKinds) return false;
    staff[kind]++; if (skilledHire) skilled[kind]++; version++;
    if (!sandbox) { cash -= costs::kHireFeeUnits * kUnit; ledgerSalaries -= costs::kHireFeeUnits * kUnit; book(costs::Salaries, -costs::kHireFeeUnits * kUnit); }   // hire fee 2 units (probable)
    return true;
}

bool Economy::fire(int kind, bool skilledFire) {
    if (kind < 0 || kind >= StaffKinds || staff[kind] == 0) return false;
    staff[kind]--; if (skilledFire && skilled[kind] > 0) skilled[kind]--; version++;
    if (!sandbox) { cash -= costs::kFireFeeUnits * kUnit; ledgerSalaries -= costs::kFireFeeUnits * kUnit; book(costs::Salaries, -costs::kFireFeeUnits * kUnit); }   // firing costs 25 units
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

// Year-end board check, read from the publisher exe (docs/PUBLISHER_EXE_NOTES.md): with cash below zero the strike counter goes 0 to 1 (the board is
// concerned, two years left), 1 to 2 (very worried, one more year), and on the third negative year end the contract is terminated. Cash at or above
// zero resets the counter. Sandbox games skip the check.
void Economy::yearEnd() {
    if (sandbox) return;
    if (cash >= 0) { debtStage = 0; return; }
    if (debtStage == 0) { debtStage = 1; notice = "The board is concerned about our negative cash situation. You have two years to return to positive cash."; }
    else if (debtStage == 1) { debtStage = 2; notice = "The board is very worried about our lingering debt. You have one more year to get out of debt."; }
    else { debtStage = 3; gameOver = true; notice = "You have been unable to make a profit on this course. Regrettably, the board has terminated your contract."; }
}

void Economy::step(double dt) {
    if (gameOver) return;
    clock_ += dt;
    while (clock_ >= dayLength) {
        clock_ -= dayLength;
        day++; version++;
        ledger.monthCounter = day - 1;
        if (sandbox) continue;   // unlimited funds: nothing is charged and the game cannot end
        // Per month: hole upkeep, wage events (each rolls against the course grade) and interest on debt, as read from the exe.
        {
            const int g = costs::courseGrade(holes) < 0 ? 3 : costs::courseGrade(holes);
            const int periods = difficulty + 2, span = std::max(1, 4 - difficulty);
            double wages = 0;
            for (int k = 0; k < StaffKinds; k++)
                for (int n = 0; n < staff[k]; n++)
                    for (int e = 0; e < periods; e++) { rng_ = rng_ * 1664525u + 1013904223u; if ((int)((rng_ >> 16) % span) <= g) wages += costs::wageUnits(k, n < skilled[k]) * kUnit; }
            const double upkeep = dailyUpkeep;
            const double interest = cash < 0 ? -(double)((long long)(-cash / kUnit) / 50) * kUnit : 0;   // cash / 50 units, truncated toward zero
            cash -= upkeep + wages; cash += interest;
            upkeepPaid += upkeep - interest; wagesPaid += wages; ledgerSalaries -= wages;
            book(costs::Salaries, -wages); book(costs::MaintInterest, -upkeep + interest);
        }
        if ((day - 1) % kMonthsPerYear == 0) yearEnd();
    }
}

}  // namespace sg
