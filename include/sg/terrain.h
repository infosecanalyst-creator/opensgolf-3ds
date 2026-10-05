// SimGolf native port: terrain model.
// Facts here come from analysis of the original Terrain.dll (see docs/TERRAIN.md). Where the
// original behaviour is not yet reproduced it is marked APPROXIMATION.
#pragma once
#include "sg/assets.h"
#include <map>

namespace sg {

constexpr int kTypeCount = 36;          // ids 0..30 are the original's (NUM_TEXTURED_TILES); 31..35 are OUR editor-only ids, see below
constexpr float kTileSize = 100.0f;     // world units per tile edge (original: 100)
constexpr float kHeightStep = 12.0f;    // APPROXIMATION: world units per elevation level
constexpr float kDepthRange = 5000.0f;  // original ortho near/far: -5000 .. +5000

// Tile type ids, in the order the original Terrain constructor registers their names.
// Ids 6, 7, 14, 15, 16, 20, 21 are unnamed in the original (reserved/unused).
enum TileType : uint8_t {
    TT_Tee = 0, TT_PuttingGreen = 1, TT_Fairway = 2, TT_FirmFairway = 3, TT_Rough = 4, TT_DeepRough = 5,
    TT_GrassySand = 8, TT_PotSandBunker = 9, TT_Overgrowth = 10, TT_Brush = 11, TT_Rock = 12, TT_Woods = 13,
    TT_WaterShallow = 17, TT_Marsh = 18, TT_Overgrowth2 = 19, TT_Building = 22, TT_WaterMiddle = 23,
    TT_WaterDeep = 24, TT_WaterShallowDesert = 25, TT_TrickyGreen = 26, TT_SandBunker1 = 27,
    // NOT original ids. These texture sets ship with the game (Cliff*, Ravine*, FlowerBed*, ZenSand*, GrassBunker*) but
    // the ids and rules that use them live in golf.exe, which is not read, so the editor gives them ids of its own.
    TT_Cliff = 31, TT_Ravine = 32, TT_FlowerBed = 33, TT_ZenSand = 34, TT_GrassBunker = 35
};
const char* tileTypeName(int type);     // nullptr for unnamed ids

// Texture files for one theme folder (Parkland, Links, Desert, Tropical):
//   <Name><Set A..E><Variation 0001..0009>.bmp (files[type][set][variation 0..8], empty if missing), 64x64, looked up case-insensitively.
struct TextureCatalog {
    std::string dir;
    std::vector<std::vector<std::vector<std::string>>> files;  // [type][set][variation] -> path
    bool load(const std::string& themeDir, std::string& err);
    // Best texture for a tile: wraps set/variation into what exists, falls back to similar types.
    const std::string* pick(int type, int set, int variation) const;
};

// <Theme>Lighting.txt: "#AMBIENT / r g b", "#DIFFUSE", "#SPECULAR" (0..255). The "#HIGHLIGHT"
// hex block is not decoded.
struct Lighting {
    float ambient[3] = {0.78f, 0.78f, 0.78f};
    float diffuse[3] = {0.94f, 0.94f, 0.94f};
    float specular[3] = {1, 1, 1};
};
bool loadLighting(const std::string& path, Lighting& out);

struct Terrain {
    int w = 0, h = 0;
    std::vector<uint8_t> type, variation, set;  // per tile, w*h
    std::vector<int8_t> corner;                 // (w+1)*(h+1) corner elevation levels
    int cornerAt(int x, int y) const { return corner[(size_t)y * (w + 1) + x]; }
    int tileIndex(int x, int y) const { return y * w + x; }
    std::vector<uint8_t> pathKind;              // per tile, w*h: 0 none, 1 gravel path, 2 paved path (our own overlay model)
    int pathAt(int x, int y) const { return (x < 0 || y < 0 || x >= w || y >= h || pathKind.empty()) ? 0 : pathKind[(size_t)tileIndex(x, y)]; }
    // Retaining walls: one flag per tile edge (bit 0 north = -z, 1 east, 2 south, 3 west). Terrain.dll's Tile keeps a flag
    // byte per direction (Tile::getWall/setWall); what the walls look like and cost is decided elsewhere, so the height and
    // look here are placeholders. Edits keep the two tiles that share an edge in step.
    std::vector<uint8_t> wallMask;
    bool wallAt(int x, int y, int dir) const { return x >= 0 && y >= 0 && x < w && y < h && !wallMask.empty() && (wallMask[(size_t)tileIndex(x, y)] >> dir & 1); }
    void setWall(int x, int y, int dir, bool on);
    bool desert = false;                        // desert theme: shallow water uses the WaterShallowDesert textures
    std::vector<float> path;                    // demo walking route, x,z pairs in world units (tee to green)
    int clubhouseX = -1, clubhouseY = -1;       // tile of the demo clubhouse footprint centre, -1 if none
    // Ground height at a world position (bilinear between corner elevations; ignores sand dishing).
    float heightAt(float wx, float wz) const;
    int typeAtWorld(float wx, float wz) const;  // tile type under a world position, -1 off the map

    // Editing. Water depth is the tile's variation byte (0 shallow, 1 middle, 2 deep) as in the original.
    static constexpr int kMaxLevel = 24;
    void paint(int x, int y, int type, int variationByte = 0);
    // Raises (+) or lowers (-) the corner by `delta` levels, then keeps neighbouring corners within one
    // level of each other by spreading the change (APPROXIMATION: the original's slope rule is not decoded).
    void raiseCorner(int cx, int cy, int delta);
    void flattenTile(int x, int y);   // sets the tile's four corners to their average level (water, greens, tees, buildings)
    void relax();
    // Course file: a small text format, see docs/GAMELOGIC.md. Returns false and fills err on failure.
    bool save(const std::string& file, std::string& err) const;
    static bool load(const std::string& file, Terrain& out, std::string& err);
    // A made-up demonstration course (tee, winding fairway, green, bunkers, pond, woods).
    static Terrain demoCourse(int w, int h, uint32_t seed);
    // A new property as the player first sees it: untouched land (rough, woods, brush, a pond) with a clubhouse lot and no tee, fairway,
    // green or path. The layout is generated here and is not the original's land.
    static Terrain emptyPlot(int w, int h, uint32_t seed);
};

struct Vertex { float x, y, z, u, v, nx, ny, nz; };

// One of the 8 triangles of a tile (2 per 50x50 quadrant of the 3x3 patch). Each triangle carries
// its own texture: the original picks texture variation 1..9 per triangle from how the tile's
// neighbours differ from it (see docs/TERRAIN.md, "Edge blending").
struct TileTri {
    int texType;    // texture type id (may differ from the tile type: water depth, bunker style...)
    int set;        // random per-tile look, wraps into the sets that exist (A..E)
    int variation;  // 0..8 -> file suffix 0001..0009
    Vertex v[3];
};

// Terrain "class" used to decide where borders are drawn: tiles of the same class blend seamlessly.
// APPROXIMATION: the original receives this table from golf.exe at run time (SetTypeClasses export),
// so the grouping here is chosen to look like the game, not read from it.
int typeClass(int type);

// Relation of a tile to a neighbour: 0 same, 1 different class, 2 same class but different type.
// Out-of-map neighbours count as 0. Water depth (tile.variation 0/1/2) takes part, as in the original.
int tileRelation(const Terrain& t, int x, int y, int nx, int ny);

// Variation 0..8 chosen from the relations (a, b, c) of a triangle: its two adjacent edge
// neighbours and the diagonal one. Direct port of the original lookup (Tile::m1028 + m1262).
int blendVariation(int a, int b, int c);

// Appends the 8 triangles of tile (tx,ty). Tile = 3x3 vertex patch (corners, edge midpoints, centre),
// 50 units apart; quadrants split along their NW-SE diagonal; UVs 0, 0.5, 1. Sand (type 7) tiles
// are dished 13 units like the original. World X/Z are centred on the map: x = tx*100 - w*50.
void buildTileTriangles(const Terrain& t, int tx, int ty, std::vector<TileTri>& out);

}  // namespace sg
