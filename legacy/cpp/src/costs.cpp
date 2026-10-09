#include "sg/costs.h"

namespace sg {
namespace costs {

int buildCostUnits(int type, int level, int siteUnits) {
    if (type < 0 || type >= BuildingKinds) return 0;
    if (type == Landmark) return landmarkCostUnits(0, false) + siteUnits;   // callers with a landmark kind should use landmarkCostUnits
    const int base = kBuildings[type].baseCostUnits * (level + 2) / 2;
    return base + (type == Pathway ? siteUnits / 2 : siteUnits);
}

int removalRefundUnits(int type, int storedKind) {
    if (type == Landmark) return (storedKind + 5) * 10;
    if (type >= Pathway && type <= BallWasher) return kBuildings[type].baseCostUnits;
    return 0;   // type 5 charges instead (homeSiteRemovalChargeUnits); types 6..15 refund nothing found
}

int amenityVisitUnits(int type, int level) {
    const int plus = level >= 2 ? 4 : 0;
    switch (type) {
        case SnackBar: return 5;
        case PuttingGreen: return 4 + plus;
        case ProShop: return 6 + plus;
        case DrivingRange: return 8 + plus;
        default: return 0;
    }
}

const char* employeeName(int kind, bool skilled) {
    static const char* n[EmployeeKinds][2] = {{"Club Pro", "Celebrity"}, {"Ranger", "Marshall"}, {"Groundskeeper", "Technician"}, {"Soda Vendor", "Refresher"}};
    return kind >= 0 && kind < EmployeeKinds ? n[kind][skilled ? 1 : 0] : "?";
}

int wageUnits(int kind, bool skilled) {
    static const int w[EmployeeKinds][2] = {{3, 7}, {2, 3}, {2, 4}, {2, 5}};
    return kind >= 0 && kind < EmployeeKinds ? w[kind][skilled ? 1 : 0] : 0;
}

int tractPriceUnits(int kind, int sampledUnownedTiles) { return tractAcres(kind, sampledUnownedTiles) * 10; }

int terrainPaintCostUnits(int tileId) {
    static const int k[20] = {5, 10, 3, 3, 1, 2, 4, 6, 4, 8, 10, 4, 4, 10, 10, 10, 25, 50, 2, 6};
    return tileId >= 0 && tileId < 20 ? k[tileId] : 0;
}

const char* ledgerRowName(int row) {
    static const char* n[LedgerRows] = {"Greens Fees", "Home Sites", "Food/Drink", "Build course", "Facilities", "Salaries", "Maint./Interest", "Other"};
    return row >= 0 && row < LedgerRows ? n[row] : "?";
}

LedgerMonth Ledger::yearSum(int endMonth) const {
    LedgerMonth s;
    for (int m = endMonth - 7; m <= endMonth; m++) {
        if (m < 0) continue;
        const LedgerMonth& x = at(m);
        for (int r = 0; r < LedgerRows; r++) s.row[r] = int16_t(s.row[r] + x.row[r]);
    }
    return s;
}

}  // namespace costs
}  // namespace sg
