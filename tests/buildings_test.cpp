#include "sg/buildings.h"
#include <cstdio>
#include <vector>
using namespace sg;
using namespace sg::buildings_exe;

static int fails = 0;
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); fails++; } } while (0)

static std::vector<BuildTile> makeMap() {
    std::vector<BuildTile> m(BuildingSystem::kMapTiles);
    for (auto& t : m) { t.id = 1; t.owned = true; t.flags = 0; }
    return m;
}
static void applyChanges(std::vector<BuildTile>& m, const PlaceResult& r) {
    for (const auto& c : r.changes) {
        if (c.newId >= 0) m[c.tile].id = (uint8_t)c.newId;
        m[c.tile].flags = (uint16_t)((m[c.tile].flags | c.setFlags) & ~c.clearFlags);
    }
}

int main() {
    auto map = makeMap();
    BuildContext ctx; ctx.tiles = map.data(); ctx.difficulty = 1; ctx.grade = 1;
    BuildingSystem bs;
    auto at = [](int x, int y) { return x * 50 + y; };

    // first placement: clubhouse, then a putting green two path tiles away
    PlaceResult r = bs.place(Clubhouse, 10, 10, ctx);
    CHECK(r.ok); applyChanges(map, r);
    r = bs.place(PuttingGreen, 16, 10, ctx);
    CHECK(r.ok && r.priceUnits == 100 && r.firstPlacement); applyChanges(map, r);
    CHECK(bs.level(PuttingGreen) == 1);
    CHECK(bs.effectLevel(PuttingGreen) == 0);   // not connected yet
    // flood fill: path 14,10 and 15,10
    for (int x = 14; x <= 15; x++) { PlaceResult p = bs.place(Pathway, x, 10, ctx); CHECK(p.ok && p.priceUnits == 1); applyChanges(map, p); }
    bs.rebuild(ctx);
    CHECK(bs.effectLevel(PuttingGreen) == 1);
    CHECK(bs.imaginationBoost(20) == 26);
    // difficulty 0 rule connects without a path
    BuildContext easy = ctx; easy.difficulty = 0;
    auto m2 = map; m2[at(14, 10)].flags = 0; easy.tiles = m2.data();
    bs.rebuild(easy); CHECK(bs.effectLevel(PuttingGreen) == 1);
    bs.rebuild(ctx);

    // grade-2 requirement for the level 1 upgrade
    PlaceCheck pc = bs.canPlace(PuttingGreen, 16, 10, ctx);
    CHECK(!pc.ok);
    ctx.grade = 2;
    pc = bs.canPlace(PuttingGreen, 16, 10, ctx);
    CHECK(pc.ok && pc.priceUnits == 150 && pc.side == 4);
    // upgrade rebuild and footprint growth
    r = bs.place(PuttingGreen, 16, 10, ctx); CHECK(r.ok && !r.firstPlacement && r.demolished.size() == 1); applyChanges(map, r);
    CHECK(bs.level(PuttingGreen) == 2 && bs.records().size() == 2);
    CHECK(bs.records()[1].side == 4);
    CHECK(bs.recordAtTile(19, 13) == 1);
    bs.rebuild(ctx);
    CHECK(bs.effectLevel(PuttingGreen) == 2 && bs.imaginationBoost(20) == 30);
    CHECK(BuildingSystem::incomePerVisit(PuttingGreen, bs.level(PuttingGreen)) == 8);
    // level 2 cap
    CHECK(!bs.canPlace(PuttingGreen, 16, 10, ctx).ok);

    // site work cost: 2x2 pro shop over four tree tiles
    for (int dx = 0; dx < 2; dx++) for (int dy = 0; dy < 2; dy++) map[at(30 + dx, 30 + dy)].id = 13;
    pc = bs.canPlace(ProShop, 30, 30, ctx);
    CHECK(pc.ok && pc.siteUnits == 20 && pc.priceUnits == 220);
    // path halving: willow-class tile id 16 costs 10, path pays 1 + 5
    map[at(40, 40)].id = 16;
    pc = bs.canPlace(Pathway, 40, 40, ctx);
    CHECK(pc.ok && pc.priceUnits == 6);
    // rejections
    map[at(5, 5)].owned = false;
    CHECK(!bs.canPlace(ProShop, 5, 5, ctx).ok);
    CHECK(!bs.canPlace(ProShop, 10, 10, ctx).ok);   // over the clubhouse (other type)
    CHECK(!bs.canPlace(ProShop, 49, 49, ctx).ok);   // off map

    // undo round trip
    PlaceResult p = bs.place(Pathway, 40, 40, ctx); CHECK(p.ok && p.priceUnits == 6); applyChanges(map, p);
    CHECK(bs.undoSlot(40, 40).code == kUndoPath && bs.undoSlot(40, 40).refundUnits == 6);
    UndoResult u = bs.applyUndo(40, 40);
    CHECK(u.ok && u.refundUnits == 6 && u.clearFlags == kFlagPath);
    map[at(40, 40)].flags &= ~u.clearFlags;
    CHECK(bs.undoSlot(40, 40).code == kUndoNone);
    CHECK(!bs.applyUndo(40, 40).ok);

    std::printf(fails ? "buildings_test: %d failure(s)\n" : "buildings_test: ok\n", fails);
    return fails ? 1 : 0;
}
