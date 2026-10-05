// Building costs, employees, wages, land and the ledger, read from the publisher's golf.exe (see docs/EXE_COSTS.md).
// Pure module: no SDL/GL. Money is in stored units of $100 everywhere (display = units * 100).
// Confidence tags: EXACT (read in code or table), PROBABLE (structure read, meaning inferred), PLACEHOLDER (not found).
#pragma once
#include <cstdint>

namespace sg {
namespace costs {

constexpr int kUnit = 100;                 // dollars per stored unit

// ---------------------------------------------------------------- start of game (EXACT)
constexpr int kStartCashUnits = 1000;      // $100,000 on a normal new game; the property price is then subtracted
constexpr int kSandboxCashUnits = 10000;   // $1,000,000 (new game with the sandbox flag); a second sandbox path stores 100000 (PLACEHOLDER which one the UI shows)
// Property chooser: prices by property index 0..15, ascending (EXACT table; index order equals ascending price, which fits the screenshot prices).
constexpr int kPropertyPriceUnits[16] = {500, 600, 700, 800, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000};
inline constexpr bool propertyAffordable(int index, int cashUnits, bool sandbox) { return sandbox || kPropertyPriceUnits[index] <= cashUnits; }

// ---------------------------------------------------------------- buildings (EXACT, table at 0x4c26b0, 0x14 bytes per entry)
// Index equals the exe's building type and unlock level (0..15); 16..19 are scenery or scripted objects.
enum Building : int {
    Pathway = 0, Benches, FlowerBed, BallWasher, Landmark, HomeSite, PuttingGreen, SnackBar, ProShop, SwimClub,
    DrivingRange, CartGarage, Marina, ResortHotel, Airstrip, Clubhouse, WillowTree, TvTower, TvBooth, ScenicBridge,
    BuildingKinds
};
struct BuildingInfo { const char* name; int footprint; int baseCostUnits; };
constexpr BuildingInfo kBuildings[BuildingKinds] = {
    {"Pathway", 1, 1},     {"Benches", 1, 2},       {"Flower Bed", 1, 5},      {"Ball Washer", 1, 50},  {"Landmark", 1, 15},
    {"Home Site", 2, 10},  {"Putting Green", 3, 100}, {"Snack Bar", 2, 150},   {"Pro Shop", 2, 200},    {"Swim Club", 3, 300},
    {"Driving Range", 5, 250}, {"Cart Garage", 2, 400}, {"Marina", 2, 1000},   {"Resort Hotel", 4, 2500}, {"Airstrip", 6, 5000},
    {"Clubhouse", 4, 200}, {"Willow Tree", 1, 25},  {"TV Tower", 1, 10},       {"TV Booth", 1, 10},     {"Scenic Bridge", 1, 100},
};
// Landmark base 15 in the table is NOT what is charged: see landmarkCostUnits. Types 16..19 are table entries only (use by the exe not traced).

// Types that can be built from the Buildings panel (6..15) need the unlock counter above their index (see PUBLISHER_EXE_NOTES).
constexpr int kMaxUpgradeLevel = 2;        // a type whose level is above 1 refuses further placement; level 1 or more needs more than 9 holes (grade >= 2)
inline constexpr bool upgradeAllowed(int level, int grade) { return level < 2 && (level == 0 || grade >= 2); }

// Cost of placing building `type` when that type's level is `level`, before site work: base * (level + 2) / 2 (EXACT; the panel shows base for level 0 and 1.5 x base after).
// Types 4 (landmark) and the home-site income have their own rules below. `siteUnits` is the clearing term the exe adds from the terrain under the footprint
// (PLACEHOLDER: its routine 0x40db90 was not decoded; pass 0). A pathway halves it (EXACT) when it is not the error value.
int buildCostUnits(int type, int level, int siteUnits = 0);
// Landmark placement: (5 * kind + 25) * 2 units, free (0) when the landmark was donated by the heiress (EXACT). kind = the landmark's index 0..15.
inline constexpr int landmarkCostUnits(int kind, bool donated) { return donated ? 0 : (5 * kind + 25) * 2; }
// Home site: placing one pays the club a quarter of the lot's value (EXACT, value is the property value routine 0x42ef40, not decoded here);
// the net cash change is -(cost - value / 4). The same amount is booked in the Home Sites ledger row.
inline constexpr int homeSitePlacementIncomeUnits(int lotValueUnits) { return lotValueUnits / 4; }
// Removing a placed amenity of type 0..4 (path, bench, flower bed, ball washer, landmark) gives back its base cost in full (EXACT); a landmark gives (stored kind + 5) * 10.
int removalRefundUnits(int type, int storedKind = 0);
// Removing a home site costs the club lotValue / 2 + storedSaleValue / 50 (EXACT); booked in Home Sites.
inline constexpr int homeSiteRemovalChargeUnits(int lotValueUnits, int storedUnits) { return lotValueUnits / 2 + storedUnits / 50; }
// Types 6..15 give nothing back (no refund code found); the exe asks to confirm a demolition. Demolishing the clubhouse sets a flag that
// lets it be re-placed (the guide text says a replacement fee: the fee itself is PLACEHOLDER = its normal cost).

// Income per visit, paid to Food/Drink (EXACT). level is the type's upgrade level: below 2 pays the first figure, 2 or more adds 4.
// Snack Bar 5 (all levels); Putting Green 4/8; Pro Shop 6/10; Driving Range 8/12. Ball washer, swim club, cart garage and the rest pay nothing per visit.
int amenityVisitUnits(int type, int level);
constexpr int kSodaVendorSaleUnits = 2;    // a Soda Vendor sale pays 2 units (EXACT, booked in Food/Drink)

// ---------------------------------------------------------------- employees
enum Employee : int { ClubPro = 0, Ranger, Groundskeeper, SodaVendor, EmployeeKinds };
// Names by [kind][skilled]: the hire screen lists Club Pro/Celebrity, Ranger/Marshall, Groundskeeper/Technician, Soda Vendor/Refresher (EXACT).
const char* employeeName(int kind, bool skilled);
// Wage in units per wage event (EXACT table at 0x4c2e2c, also the price the hire screen shows): [kind][skilled]
// Club Pro 3/7, Ranger 2/3, Groundskeeper 2/4, Soda Vendor 2/5.
int wageUnits(int kind, bool skilled);
constexpr int kHireFeeUnits = 2;           // paid on hiring, booked in Salaries (PROBABLE: the subtraction instruction was not shown in the disassembly, the ledger write of -2 is)
constexpr int kFireFeeUnits = 25;          // paid on firing, booked in Salaries (EXACT)
// Skilled staff need a Daily Fee course: grade >= 1 (six or more holes) (EXACT text; the exe checks hire code % 3 == 2 against the grade).
inline constexpr bool skilledHireAllowed(int grade) { return grade >= 1; }
// Course grade used by the wage test: holes built < 6 -> 0, < 10 -> 1, < 18 -> 2, exactly 18 -> 3, more than 18 -> -1 (EXACT, routine 0x44faf0).
inline constexpr int courseGrade(int holesBuilt) { return holesBuilt < 6 ? 0 : holesBuilt < 10 ? 1 : holesBuilt < 18 ? 2 : holesBuilt == 18 ? 3 : -1; }
// Wage events happen every periodTicks(difficulty). Each active employee pays only when a random roll in [0, 4 - difficulty) is <= grade
// (EXACT shape; whether the roll bound is exclusive is PROBABLE). Small courses on hard settings therefore pay staff less often, not more.
inline constexpr int periodTicks(int difficulty) { return 1024 / (difficulty + 2); }
inline constexpr bool wageDue(int roll, int grade) { return roll <= grade; }
constexpr int kHoleUpkeepUnits = 1;        // every existing hole costs 1 unit per period, booked in Maint./Interest (EXACT)
// Wages and upkeep are skipped when the game flag 0x4000000 is set (PROBABLE: practice or tutorial mode; sandbox also never goes broke).

// Employee EFFECTS (text only, numbers not decoded): Club Pro greets golfers (mood), Ranger/Marshall speeds play near its tee, Groundskeeper
// removes weeds, Soda Vendor serves thirst and earns kSodaVendorSaleUnits per sale. Skilled versions are stronger (PLACEHOLDER magnitude).

// ---------------------------------------------------------------- debt
// Once a month (ticks % 1024 == 0), when cash is negative, interest of cash / 50 (a negative number, 2 percent of the debt, truncated toward zero)
// is added to cash and to the Maint./Interest row (EXACT). Returns 0 when cash >= 0.
inline constexpr int monthlyInterestUnits(int cashUnits) { return cashUnits < 0 ? cashUnits / 50 : 0; }
// A purchase asks for confirmation (shows "This change costs X. You have only Y") when price > 0, price > cash, not sandbox and at least 4 holes
// are planned (next hole index >= 4). Otherwise it goes through silently (EXACT, routine 0x406c30). The player may then go into debt.
inline constexpr bool purchaseNeedsConfirm(int priceUnits, int cashUnits, bool sandbox, int nextHoleIndex) { return priceUnits > 0 && priceUnits > cashUnits && !sandbox && nextHoleIndex >= 4; }

// ---------------------------------------------------------------- land (buy-land screen)
// The county commissioner's approval opens a 3 x 3 grid of tracts, each 16 x 16 tiles of the 50 x 50 map (EXACT). Unowned tiles carry id 20.
// Price in units for a tract: f = 5 + sum over each unowned tile that passes a 1-in-3 roll of (1 << kind); acres = f * 20 / 100; price = acres * 10
// (EXACT shape; `kind` is the exe's global at 0x53a450, probably the course theme 0..3: PROBABLE). The result is booked in Other.
int tractPriceUnits(int kind, int sampledUnownedTiles);
inline constexpr int tractAcres(int kind, int sampledUnownedTiles) { return (5 + sampledUnownedTiles * (1 << kind)) * 20 / 100; }

// ---------------------------------------------------------------- terrain editing
// Painting cost per tile id 0..19 in units (table at 0x4c1a40, EXACT; also in Economy::terrainCostUnits).
int terrainPaintCostUnits(int tileId);
// Raising or lowering one vertex: 2 units, or 15 when the tile under it is water (id 17) (PROBABLE, read from the cost routine at 0x420a00).
inline constexpr int elevationCostUnits(bool onWater) { return onWater ? 15 : 2; }
constexpr int kPathTileUnits = 1;          // path tile: base 1 unit (EXACT), halved site work added by the exe is PLACEHOLDER

// ---------------------------------------------------------------- ledger (Financial Report, EXACT layout)
// 100 months in a ring, indexed by month counter % 100, ten 16-bit slots each; eight rows then a total.
enum LedgerRow : int { GreensFees = 0, HomeSites, FoodDrink, BuildCourse, Facilities, Salaries, MaintInterest, Other, LedgerRows };
const char* ledgerRowName(int row);
constexpr int kLedgerMonths = 100;
// What books where (EXACT unless noted): fees -> GreensFees; home site placement/removal -> HomeSites; snack bar/pro shop/putting green/driving range/
// soda sale -> FoodDrink; terrain painting and elevation undo -> BuildCourse; buildings, paths, landmarks -> Facilities; hire/fire fees and wages -> Salaries;
// hole upkeep and interest -> MaintInterest; tournament prizes, match wagers, CEO investment, land purchase -> Other.
struct LedgerMonth { int16_t row[LedgerRows] = {}; int total() const { int t = 0; for (int16_t v : row) t += v; return t; } };
struct Ledger {
    LedgerMonth month[kLedgerMonths];
    int monthCounter = 0;                  // the exe's month index; it is incremented every 1024 ticks
    void add(int r, int units) { month[monthCounter % kLedgerMonths].row[r] = int16_t(month[monthCounter % kLedgerMonths].row[r] + units); }
    const LedgerMonth& at(int m) const { return month[((m % kLedgerMonths) + kLedgerMonths) % kLedgerMonths]; }
    // The report draws the last 9 columns ending at the current month (EXACT: from max(0, current - 8)); sum of eight months = one game year.
    LedgerMonth yearSum(int endMonth) const;
};

}  // namespace costs
}  // namespace sg
