// SimGolf native port: a minimal club economy.
//
// WHAT THE ORIGINAL MANUAL SAYS (Sid Meier's SimGolf manual, EA 2002; no amounts are given anywhere in it):
//  * the player starts with a fixed allotment of cash that depends on the property chosen (sixteen, differing in size, location and
//    cost) and on the difficulty (Easy, Moderate, Difficult, Impossible; attitudes get more volatile as it rises) (p. 5, 7);
//  * Sandbox mode has unlimited funds (p. 6);
//  * a golfer pays a green fee when he or she completes a hole, and it is the primary income; happy golfers also bring friends and buy
//    memberships (p. 14, 15). Holes the SGA ranks in the Top 100, then Top 18, earn higher fees, and an Airstrip raises fees per hole
//    cumulatively on those holes (p. 20, 24);
//  * building lots earn "a sizeable amount of income relatively quickly", more near water, trees and fun holes; a Marina raises their
//    profit (p. 19, 20); winning pro-challenges and tournaments pays prize money (p. 22);
//  * employees are paid (p. 12) and the Financial Report lists expenses, income and profit by year (p. 24); moving the Clubhouse costs
//    a replacement fee (p. 18); hiring skilled staff needs a daily fee course of six or more holes (p. 20).
// OTHER PUBLIC SOURCES: a strategy guide says a path costs 100 a square; Wikipedia says the game ends if the budget stays in the red
// long enough. NOT FOUND ANYWHERE I COULD READ: the starting cash (the 100,000 here is unverified), the fee amounts, all upkeep and
// wage amounts, the length of a game day, and how long in the red ends the game. Those are PLACEHOLDERS; the per tile upkeep below is
// only a stand-in for wages and other running costs. The real figures live in golf.exe, which this project does not read.
#pragma once
#include "sg/costs.h"
#include "sg/terrain.h"

namespace sg {

struct Economy {
    double startCash = 100000;      // starting amount: 1000 stored units of $100 in the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md)
    double cash = 100000;
    bool sandbox = false;           // Sandbox mode: unlimited funds, nothing is charged and the game cannot end
    // Money in the original is stored in units of 100 (docs/PUBLISHER_EXE_NOTES.md). A green fee is computed per golfer and per hole
    // from the golfer's mood plus bonuses (see feeUnitsFor in the viewer); this is only the unit.
    static constexpr double kUnit = 100.0;
    static constexpr double kMoneyScale = 25.0;   // placeholder upkeep and wages are scaled by this to stay in proportion
    double dayLength = 120;         // real seconds per game day (placeholder)
    int day = 1;
    int holesPlayed = 0;
    static constexpr int kMonthsPerYear = 8;   // the original's year runs March to October: 1024 ticks a month, 8192 a year (read from the exe)
    // Board strikes at year end (exact rule, see yearEnd): 0 none, 1 concerned, 2 very worried, 3 contract terminated.
    int debtStage = 0;
    int difficulty = 1, holes = 0;   // set by the viewer; wages and upkeep depend on them (docs/EXE_COSTS.md)
    double ledgerSalaries = 0;       // running total booked under Salaries
    costs::Ledger ledger;            // the Financial Report's monthly ledger (units of 100); the viewer books its own events with book()
    void book(int row, double dollars) { ledger.add(row, (int)(dollars >= 0 ? dollars / kUnit + 0.5 : dollars / kUnit - 0.5)); }
    void yearEnd();
    const char* notice = "";        // latest message for the player, cleared by the viewer once shown
    bool gameOver = false;
    double income = 0, upkeepPaid = 0;
    double dailyUpkeep = 0;         // PLACEHOLDER, recomputed from the course
    unsigned version = 0;           // bumps whenever cash changes (for the HUD)

    // Basic employees from the manual (p. 20): Club Pro, Ranger, Groundskeeper, Soda Vendor. They are hired from the Hire Employees
    // menu, begin work at once and are paid. The wages and the size of every effect are PLACEHOLDERS.
    enum Staff { ClubPro, Ranger, Groundskeeper, SodaVendor, StaffKinds };
    static const char* staffName(int k);
    int staff[StaffKinds] = {};                  // how many of each are employed
    double wagesPaid = 0;
    int staffCount() const { int n = 0; for (int s : staff) n += s; return n; }
    double dailyWages() const;
    int skilled[StaffKinds] = {};                // how many of each kind are the skilled version (counted inside staff[])
    bool hire(int kind, bool skilledHire = false);
    bool fire(int kind, bool skilledFire = false);
    // Golfer fun, 0..100, the average of the golfers' comments (manual p. 13 and 15). Shown as the attitude colour: red, yellow, green.
    double fun = 50;
    const char* attitude() const { return fun < 35 ? "red" : fun < 65 ? "yellow" : "green"; }
    void funEvent(double delta) { fun = fun + delta < 0 ? 0 : (fun + delta > 100 ? 100 : fun + delta); version++; }
    double lastMood = 50;                        // how the golfer who just finished felt, 0..100 (the hole's fun contribution)
    void holeFinished(int strokes, int par, double feeUnits);   // collects the fee (in units of 100) and applies the effect on fun

    void init(const Terrain& t);
    void updateUpkeep(const Terrain& t);   // call after editing the course
    void holeCompleted(double feeUnits);   // collects a green fee
    // Laying a path tile costs 100 dollars a square (SimuLord's GameFAQs strategy guide, section 3C). Paved paths are assumed to
    // cost the same, since nothing found says otherwise. Removing a tile is free here.
    static constexpr int kPathTileCost = 100;
    // Cost per tile of laying a terrain type, in units of 100, from the exe's terrain table (the figure its build menu shows is this times 100).
    // Indexed by the original tile id 0..19; ids without an entry (building lots, editor-only ids) cost nothing.
    static int terrainCostUnits(int type) {
        static const int k[20] = {5, 10, 3, 3, 1, 2, 4, 6, 4, 8, 10, 4, 4, 10, 10, 10, 25, 50, 2, 6};
        if (type >= 0 && type < 20) return k[type];
        return (type == 23 || type == 24 || type == 25) ? 50 : (type == 26 ? 10 : (type == 27 ? 6 : 0));   // water depths, tricky green, sand bunker
    }
    void earn(double amount) { cash += amount; version++; }
    void spend(double amount) { if (sandbox) return; cash -= amount; version++; }
    void step(double dt);
    static double upkeepFor(const Terrain& t);

  private:
    double clock_ = 0;
    unsigned rng_ = 12345;
};

}  // namespace sg
