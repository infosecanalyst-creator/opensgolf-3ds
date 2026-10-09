#include "sg/buildings.h"
#include <algorithm>
#include <cstring>

namespace sg {
using namespace buildings_exe;

namespace {
constexpr int N = kMapSide;
inline bool inMap(int x, int y) { return x >= 0 && y >= 0 && x < N && y < N; }
inline int idx(int x, int y) { return x * N + y; }
constexpr uint16_t kFlag2000 = 0x2000;
// Types whose footprint grows with level. Clubhouse, snack bar and home site never grow.
inline bool grows(int t) { return typeHasLevel(t) && t != Clubhouse && t < WillowTree; }
// Types that are stored as building records by this system.
inline bool isRecordType(int t) { return t == BallWasher || t == Landmark || t == HomeSite || (t >= PuttingGreen && t <= Clubhouse); }
}  // namespace

BuildingSystem::BuildingSystem() { clear(); }

void BuildingSystem::clear() {
    rec_.clear();
    std::fill(grid_, grid_ + kMapTiles, -1);
    std::fill(L_, L_ + TypeCount, 0);
    std::fill(E_, E_ + TypeCount, 0);
    std::fill(conn_, conn_ + TypeCount, false);
    for (auto& u : undo_) u = UndoSlot();
}

void BuildingSystem::refreshGrid() {
    std::fill(grid_, grid_ + kMapTiles, -1);
    for (int i = 0; i < (int)rec_.size(); i++) {
        const auto& r = rec_[i];
        for (int dx = 0; dx < r.side; dx++)
            for (int dy = 0; dy < r.side; dy++)
                if (inMap(r.x + dx, r.y + dy)) grid_[idx(r.x + dx, r.y + dy)] = i;
    }
}

int BuildingSystem::recordAtTile(int tx, int ty) const { return inMap(tx, ty) ? grid_[idx(tx, ty)] : -1; }

bool BuildingSystem::addRecord(const BuildingRecord& r) {
    if ((int)rec_.size() >= kMaxBuildingRecords) return false;
    rec_.push_back(r);
    refreshGrid();
    return true;
}

bool BuildingSystem::demolish(int i) {
    if (i < 0 || i >= (int)rec_.size()) return false;
    rec_.erase(rec_.begin() + i);
    refreshGrid();
    return true;
}

void BuildingSystem::rebuild(const BuildContext& ctx) {
    refreshGrid();
    // Flood fill from the clubhouse footprint over path or building tiles, four directions.
    std::vector<char> seen(kMapTiles, 0);
    std::vector<int> stack;
    for (const auto& r : rec_) {
        if (r.type != Clubhouse) continue;
        for (int dx = 0; dx < r.side; dx++)
            for (int dy = 0; dy < r.side; dy++)
                if (inMap(r.x + dx, r.y + dy) && !seen[idx(r.x + dx, r.y + dy)]) { seen[idx(r.x + dx, r.y + dy)] = 1; stack.push_back(idx(r.x + dx, r.y + dy)); }
    }
    static const int ddx[4] = {1, -1, 0, 0}, ddy[4] = {0, 0, 1, -1};
    while (!stack.empty()) {
        int t = stack.back(); stack.pop_back();
        int x = t / N, y = t % N;
        for (int d = 0; d < 4; d++) {
            int nx = x + ddx[d], ny = y + ddy[d];
            if (!inMap(nx, ny)) continue;
            int n = idx(nx, ny);
            if (seen[n]) continue;
            // PLACEHOLDER: "across land tiles" is read as: any building tile, or any tile with the path flag (a scenic bridge carries the path flag over water).
            bool pathy = ctx.tiles && (ctx.tiles[n].flags & kFlagPath);
            if (grid_[n] >= 0 || pathy) { seen[n] = 1; stack.push_back(n); }
        }
    }
    std::fill(L_, L_ + TypeCount, 0);
    std::fill(E_, E_ + TypeCount, 0);
    std::fill(conn_, conn_ + TypeCount, false);
    for (auto& r : rec_) {
        bool c = false;
        for (int dx = 0; dx < r.side && !c; dx++)
            for (int dy = 0; dy < r.side && !c; dy++)
                if (inMap(r.x + dx, r.y + dy) && seen[idx(r.x + dx, r.y + dy)]) c = true;
        if (r.type == Clubhouse) c = true;
        if (ctx.difficulty == 0 && !ctx.freeBuildFlag && r.type >= PuttingGreen && r.type <= WillowTree) c = true;
        r.connected = c;
        if (c) conn_[r.type] = true;
        if (typeHasLevel(r.type)) {
            int lv = std::min(99, r.storedLevel + 1);
            L_[r.type] = std::max(L_[r.type], lv);
            if (c) E_[r.type] = std::max(E_[r.type], lv);
        }
    }
    L_[SnackBar] = 0; E_[SnackBar] = 0;   // forced
    L_[HomeSite] = 0; E_[HomeSite] = 0;
}

int BuildingSystem::level(int t) const { return (t >= 0 && t < TypeCount) ? L_[t] : 0; }
int BuildingSystem::effectLevel(int t) const { return (t >= 0 && t < TypeCount) ? E_[t] : 0; }
bool BuildingSystem::isConnected(int t) const { return (t >= 0 && t < TypeCount) && conn_[t]; }

int BuildingSystem::levelForPlacement(int t) const { return grows(t) ? L_[t] : 0; }

PlaceCheck BuildingSystem::canPlace(int type, int tx, int ty, const BuildContext& ctx) const {
    PlaceCheck pc;
    auto fail = [&](const char* why) { pc.ok = false; pc.reason = why; return pc; };
    if (!ctx.tiles) return fail("No map");
    if (type == Pathway) {
        if (!inMap(tx, ty)) return fail("Out of bounds");
        const BuildTile& t = ctx.tiles[idx(tx, ty)];
        pc.side = 1;
        if (t.flags & kFlagPath) { pc.ok = true; pc.priceUnits = 0; return pc; }   // existing path: costs 0, does nothing
        pc.siteUnits = siteCostOfTile(t.id, type) / 2;                               // pathway halves the site sum
        pc.priceUnits = kBaseCostUnits[Pathway] + pc.siteUnits;
        pc.ok = true;
        return pc;
    }
    if (!isRecordType(type)) return fail("Not a building record type");
    const int L = levelForPlacement(type);
    if (typeHasLevel(type) && type != Clubhouse) {
        if (L >= kMaxLevel) return fail("Already fully upgraded");
        if (L >= 1 && ctx.grade != -1 && ctx.grade < kUpgradeMinGrade) return fail("Needs a course grade of 2 (ten holes)");
    }
    if ((int)rec_.size() >= kMaxBuildingRecords) {
        // Upgrading frees the old records of this type first, so only refuse if that does not make room.
        int freed = 0;
        if (grows(type)) for (const auto& r : rec_) if (r.type == type) freed++;
        if (freed == 0) return fail("Too many buildings");
    }
    const int side = placementScanSide(kBaseSide[type], L);
    pc.side = side;
    const bool marinaWater = (type == Marina) && ((1 << (ctx.theme & 31)) & kMarinaThemeMask);
    int site = 0;
    for (int dx = 0; dx < side; dx++)
        for (int dy = 0; dy < side; dy++) {
            int x = tx + dx, y = ty + dy;
            if (!inMap(x, y)) return fail("Out of bounds");
            const BuildTile& t = ctx.tiles[idx(x, y)];
            if (!t.owned) return fail("You don't own that land");
            if (t.id == 0 || t.id == kTileBuildingHome) return fail("Can't build there");
            if (t.id == kTileBuildingOther || (t.flags & kFlagBuilding)) {
                int ri = grid_[idx(x, y)];
                if (ri < 0 || rec_[ri].type != type) return fail("Something is already built there");
            }
            if (t.flags & (kFlagLocked | kFlagGreenTee)) return fail("Can't build there");
            if (marinaWater && t.id != kSiteTileShallowWater) return fail("The marina must be built on water");
            site += siteCostOfTile(t.id, type);
        }
    if (type != Landmark) {
        bool dry = false;
        for (int x = tx - 1; x <= tx + side && !dry; x++)
            for (int y = ty - 1; y <= ty + side && !dry; y++) {
                if (x >= tx && x < tx + side && y >= ty && y < ty + side) continue;   // interior
                if (!inMap(x, y)) continue;
                // PLACEHOLDER: "water" is read as tile id 17 only.
                if (ctx.tiles[idx(x, y)].id != kSiteTileShallowWater) dry = true;
            }
        if (!dry) return fail("Needs some dry land next to it");
    }
    if (type == HomeSite && ctx.lotValueUnits < kHomeSiteMinLotValueUnits) return fail("That is an unattractive lot");
    pc.siteUnits = site;
    if (type == Landmark) {
        // PLACEHOLDER: the landmark price is read as (5 * kind + 25) * 2 with no site work added.
        pc.priceUnits = landmarkCostUnits(ctx.landmarkKind, ctx.landmarkDonated);
    } else {
        pc.priceUnits = placementCostUnits(type, L, site);
    }
    pc.ok = true;
    return pc;
}

PlaceResult BuildingSystem::place(int type, int tx, int ty, const BuildContext& ctx) {
    PlaceResult res;
    PlaceCheck pc = canPlace(type, tx, ty, ctx);
    if (!pc.ok) { res.reason = pc.reason; return res; }
    res.ok = true;
    res.priceUnits = pc.priceUnits;
    if (type == Pathway) {
        const BuildTile& t = ctx.tiles[idx(tx, ty)];
        if (t.flags & kFlagPath) return res;   // nothing to do, costs 0
        TileChange c; c.tile = idx(tx, ty); c.setFlags = kFlagPath;
        if (t.id == kSiteTileShallowWater) c.clearFlags = kFlagScenic;
        res.changes.push_back(c);
        // Neighbouring building tiles (type 6 and up) get the path flag too.
        static const int ddx[4] = {1, -1, 0, 0}, ddy[4] = {0, 0, 1, -1};
        for (int d = 0; d < 4; d++) {
            int nx = tx + ddx[d], ny = ty + ddy[d];
            int ri = recordAtTile(nx, ny);
            if (ri >= 0 && rec_[ri].type >= PuttingGreen) { TileChange n; n.tile = idx(nx, ny); n.setFlags = kFlagPath; res.changes.push_back(n); }
        }
        recordUndo(tx, ty, kUndoPath, pc.priceUnits);
        return res;   // caller applies the flags, then calls rebuild()
    }
    const int L = levelForPlacement(type);
    res.firstPlacement = (level(type) == 0);
    if (grows(type) && L > 0) {
        for (int i = (int)rec_.size() - 1; i >= 0; i--)
            if (rec_[i].type == type) { res.demolished.push_back(rec_[i]); rec_.erase(rec_.begin() + i); }
    }
    BuildingRecord r;
    r.type = type; r.x = tx; r.y = ty; r.side = pc.side; r.storedLevel = typeHasLevel(type) && type != Clubhouse ? L : 0;
    r.landmarkKind = type == Landmark ? ctx.landmarkKind : 0;
    r.connected = (type == Clubhouse);
    rec_.push_back(r);
    res.recordIndex = (int)rec_.size() - 1;
    for (int dx = 0; dx < r.side; dx++)
        for (int dy = 0; dy < r.side; dy++) {
            TileChange c;
            c.tile = idx(tx + dx, ty + dy);
            c.newId = type == HomeSite ? kTileBuildingHome : kTileBuildingOther;
            c.clearFlags = kFlagScenic | kFlagBench | kFlagFlowers | kFlag2000;
            c.setFlags = kFlagBuilding;   // PLACEHOLDER: the doc names a building flag but not whether the stamp sets it; set here so the overlap rule works
            if (type == Clubhouse) c.setFlags |= kFlagPath | kFlagConnected;   // clubhouse tiles carry 0x60
            res.changes.push_back(c);
        }
    for (const auto& d : res.demolished)
        for (int dx = 0; dx < d.side; dx++)
            for (int dy = 0; dy < d.side; dy++) {
                int x = d.x + dx, y = d.y + dy;
                if (x >= tx && x < tx + r.side && y >= ty && y < ty + r.side) continue;
                res.vacatedTiles.push_back(idx(x, y));   // PLACEHOLDER: the exe's tile cleanup for an upgrade that shifts position is not documented
            }
    rebuild(ctx);
    return res;
}

// ---------------------------------------------------------------- undo
void BuildingSystem::recordUndo(int tx, int ty, uint8_t code, int refund) {
    if (!inMap(tx, ty)) return;
    undo_[idx(tx, ty)].code = code;
    undo_[idx(tx, ty)].refundUnits = (int16_t)refund;
}
UndoSlot BuildingSystem::undoSlot(int tx, int ty) const { return inMap(tx, ty) ? undo_[idx(tx, ty)] : UndoSlot(); }

UndoResult BuildingSystem::previewUndo(int tx, int ty) const {
    UndoResult r;
    if (!inMap(tx, ty)) return r;
    const UndoSlot& s = undo_[idx(tx, ty)];
    if (s.code == kUndoNone) return r;
    r.code = s.code; r.refundUnits = s.refundUnits;
    switch (s.code) {
        case kUndoPath: r.clearFlags = kFlagPath; break;
        case kUndoBench: r.clearFlags = kFlagBench; break;
        case kUndoFlower: r.clearFlags = kFlagFlowers; break;
        case kUndoTree: r.clearFlags = kFlagScenic; break;   // PLACEHOLDER: tree undo clears the scenic flag; the restored tile id is not documented
        case kUndoBridge: r.clearFlags = kFlagPath | kFlagScenic; break;   // PLACEHOLDER: bridge sets 0x120 so both bits are cleared
        default:
            if (s.code < 0x7d) r.restoreTileId = s.code;
            else if (s.code >= 0xa0 && s.code < 0xc0) r.restoreTileId = s.code & 0x1f;   // PLACEHOLDER: DERIVED low-bits layout
            else return r;   // unknown code
    }
    r.ok = true;
    return r;
}

UndoResult BuildingSystem::applyUndo(int tx, int ty) {
    UndoResult r = previewUndo(tx, ty);
    if (r.ok) undo_[idx(tx, ty)] = UndoSlot();
    return r;
}

const char* BuildingSystem::undoName(uint8_t code) {
    switch (code) {
        case kUndoPath: return "Remove pathway";
        case kUndoBench: return "Remove bench";
        case kUndoFlower: return "Remove flower bed";
        case kUndoTree: return "Remove tree";
        case kUndoBridge: return "Remove bridge";
        default: return code < 0x7d || (code >= 0xa0 && code < 0xc0) ? "Undo terrain change" : "";
    }
}

// ---------------------------------------------------------------- viewer queries
int BuildingSystem::incomePerVisit(int type, int lvl) { return visitIncomeUnits(type, lvl); }

std::vector<DrawItem> BuildingSystem::drawList() const {
    std::vector<DrawItem> v;
    for (const auto& r : rec_) v.push_back({r.type, r.x, r.y, r.side, r.storedLevel + 1, r.landmarkKind, r.connected});
    std::stable_sort(v.begin(), v.end(), [](const DrawItem& a, const DrawItem& b) { return a.x + a.y < b.x + b.y; });
    return v;
}

int BuildingSystem::imaginationBoost(int base) const { return buildings_exe::imaginationBoost(base, E_[PuttingGreen]); }
int BuildingSystem::accuracyDivisor() const { return buildings_exe::accuracyDivisor(E_[ProShop]); }
int BuildingSystem::driveBonus() const { return buildings_exe::driveBonus(E_[DrivingRange]); }
int BuildingSystem::holeLengthAfterRange(int len) const { return buildings_exe::holeLengthAfterRange(len, L_[DrivingRange]); }
int BuildingSystem::arrivalMood(int roll, bool d0) const { return buildings_exe::arrivalMood(E_[SwimClub], roll, d0); }
int BuildingSystem::swimMemberProgress() const { return E_[SwimClub]; }
bool BuildingSystem::cartsActive() const { return E_[CartGarage] > 0; }
int BuildingSystem::cartSpeed() const { return E_[CartGarage] > 0 ? buildings_exe::cartSpeed(E_[CartGarage]) : 0; }
int BuildingSystem::marinaValue(int v) const { return buildings_exe::marinaValue(v, E_[Marina]); }
int BuildingSystem::moodDecayDivisor() const { return buildings_exe::moodDecayDivisor(E_[ResortHotel]); }
int BuildingSystem::airstripFeeBonus() const { return buildings_exe::airstripFeeBonus(E_[Airstrip]); }

}  // namespace sg
