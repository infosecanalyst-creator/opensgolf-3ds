// BuildingSystem: exe-faithful building records, levels, connectivity, placement rules and per-tile undo.
// Pure logic (no SDL/GL). Rules from docs/DECODE_BUILDINGS.md sections 1 to 4 and 7; numbers from sg/buildings_exe.h.
// Tile index is x * 50 + y. Money is in stored units of $100.
// PLACEHOLDER items (not settled by the docs) are marked in buildings.cpp with the word PLACEHOLDER.
#pragma once
#include <cstdint>
#include <vector>
#include "sg/buildings_exe.h"

namespace sg {

// One map tile as the caller sees it. The caller fills kMapTiles entries (index x * 50 + y).
struct BuildTile {
    uint8_t id = 0;        // terrain tile id (0 tee, 12..19 clearing classes, 17 shallow water, 21/22 under buildings)
    bool owned = false;    // course owns the tile
    uint16_t flags = 0;    // buildings_exe::kFlag* bits
};

// Everything the placement rules read from the outside world.
struct BuildContext {
    const BuildTile* tiles = nullptr;   // kMapTiles entries
    int theme = 0;                      // course theme (0 and 2 force the marina onto water)
    int difficulty = 1;                 // 0 easiest
    bool freeBuildFlag = false;         // game flag 0x1000000
    int grade = 0;                      // course grade 0..3 (-1 for more than 18 holes counts as >= 2)
    int lotValueUnits = 0;              // home-site lot value (routine 0x42ef40), needs >= 50
    int landmarkKind = 0;               // for Landmark: chosen kind 0..
    bool landmarkDonated = false;       // landmark price is zero when donated
};

struct PlaceCheck {
    bool ok = false;
    const char* reason = "";   // rejection text when !ok
    int priceUnits = 0;        // total price when ok
    int siteUnits = 0;         // site-work term (already halved for a pathway)
    int side = 0;              // footprint side scanned (base + L)
};

struct BuildingRecord {
    int type = 0;
    int x = 0, y = 0;          // anchor tile: the clicked tile, minimum corner of the footprint
    int side = 0;              // footprint side as stamped (base + stored level)
    int storedLevel = 0;       // level the type had when placed (0 first, 1 replacement)
    bool connected = false;
    int landmarkKind = 0;
};

struct TileChange {            // what the caller must apply to its map
    int tile = 0;              // x * 50 + y
    int newId = -1;            // -1 keep
    uint16_t setFlags = 0, clearFlags = 0;
};

struct PlaceResult {
    bool ok = false;
    const char* reason = "";
    int priceUnits = 0;
    int recordIndex = -1;                 // index in records(), -1 for a pathway
    bool firstPlacement = false;          // L was 0: caller runs the unlock hook (0x40c6f0, purpose UNKNOWN)
    std::vector<TileChange> changes;      // stamp (and path flags)
    std::vector<int> vacatedTiles;        // tiles of a demolished record that the new footprint no longer covers
    std::vector<BuildingRecord> demolished;
};

struct UndoSlot { uint8_t code = buildings_exe::kUndoNone; int16_t refundUnits = 0; };

struct UndoResult {
    bool ok = false;
    int code = 0;
    int refundUnits = 0;
    uint16_t clearFlags = 0;   // flag bits to clear
    int restoreTileId = -1;    // previous tile id to restore, -1 none
};

struct DrawItem { int type, x, y, side, builtLevel, landmarkKind; bool connected; };

class BuildingSystem {
public:
    static constexpr int kMapTiles = buildings_exe::kMapSide * buildings_exe::kMapSide;

    BuildingSystem();
    void clear();

    // ---- records
    const std::vector<BuildingRecord>& records() const { return rec_; }
    int recordAtTile(int tx, int ty) const;                 // index or -1
    // Add a record directly (loading a saved game). Returns false when 256 records exist.
    bool addRecord(const BuildingRecord& r);
    // Remove a record (demolish, no refund). The caller handles tile cleanup.
    bool demolish(int recordIndex);

    // ---- levels (rebuilt by rebuild())
    // Rebuild the occupancy grid, connected bits (flood fill from the clubhouse) and the L and E arrays.
    // Call after any change to path flags (path placement, undo) and after addRecord/demolish.
    void rebuild(const BuildContext& ctx);
    int level(int type) const;        // L: 0 none, 1 built, 2 upgraded (0 for non-level types)
    int effectLevel(int type) const;  // E: L counting only connected records (snack bar forced 0)
    bool isConnected(int type) const; // any record of the type has the connected bit (snack bar uses this)

    // ---- placement
    PlaceCheck canPlace(int type, int tx, int ty, const BuildContext& ctx) const;
    PlaceResult place(int type, int tx, int ty, const BuildContext& ctx);

    // ---- undo (one slot per tile)
    void recordUndo(int tx, int ty, uint8_t code, int refundUnits);
    UndoSlot undoSlot(int tx, int ty) const;
    UndoResult previewUndo(int tx, int ty) const;           // does not change the slot
    UndoResult applyUndo(int tx, int ty);                   // clears the slot; caller applies flags/id/refund then rebuild()
    static const char* undoName(uint8_t code);              // "Remove ..." item name for the preview text

    // ---- viewer queries
    static int incomePerVisit(int type, int level);         // Snack Bar 5, Putting Green 4/8, Pro Shop 6/10, Driving Range 8/12
    int incomePerVisit(int type) const { return incomePerVisit(type, level(type)); }   // uses L
    std::vector<DrawItem> drawList() const;                 // sorted back to front (x + y)

    // Effect numbers by type, from E (L for the driving range hole trim). Each mirrors one row of the section 1 table.
    int imaginationBoost(int baseValue) const;              // Putting Green
    int accuracyDivisor() const;                            // Pro Shop (applies even at E 0)
    int driveBonus() const;                                 // Driving Range, long hitters
    int holeLengthAfterRange(int holeLength) const;         // Driving Range, uses L
    int arrivalMood(int roll0to2, bool difficulty0) const;  // Swim Club
    int swimMemberProgress() const;                         // Swim Club E (DERIVED meaning)
    bool cartsActive() const;                               // Cart Garage E > 0
    int cartSpeed() const;                                  // E * 4 + 8 when active, else 0
    int marinaValue(int value) const;                       // Marina
    int moodDecayDivisor() const;                           // Resort Hotel (120 at E 0)
    int airstripFeeBonus() const;                           // Airstrip

private:
    std::vector<BuildingRecord> rec_;
    int grid_[kMapTiles];
    int L_[buildings_exe::TypeCount];
    int E_[buildings_exe::TypeCount];
    bool conn_[buildings_exe::TypeCount];
    UndoSlot undo_[kMapTiles];
    void refreshGrid();
    int levelForPlacement(int type) const;
};

}  // namespace sg
