// Golfer comment sentences for the Hole Stats dialog and the Player Comments report.
// Structure follows docs/DECODE_COMMENTS.md (what is inserted, which argument picks which variant, colour per type).
// Every sentence here is the port's own wording; no text of the original game is reproduced.
// Both views speak as the "null golfer": male, no conversation state, empty scorecard, both name tokens use one name.
#pragma once
#include <string>

namespace sg {

enum class CommentColour { Neutral, Good, Bad };   // drawn black, dark green, red

struct CommentOut {
    std::string text;      // the sentence without surrounding quotes
    CommentColour colour = CommentColour::Neutral;
};

// What the map offers about the cell that triggered a scenic comment (types 11, 20, 28). Filled by the viewer.
struct CellInfo {
    int tile = 0;                 // terrain id 0..22 (same numbering as terrainName)
    bool decoration = false;      // cell decoration flag 0x1000 (flower beds instead of a plain rough tile)
    bool bridge = false;          // cell flag 0x20 on a water tile
    bool hasObjectByte = false;   // water cell with an object byte (dolphin)
    int objectByte = 0;           // picks the flower bed name (mod 5)
    int landmarkKind = -1;        // building tile: landmark kind 0..18, or -1 when not a landmark
    bool flowerbedRecord = false; // building tile whose object record is a flowerbed
};

struct CommentCtx {
    int theme = 0;                // 0 parkland, 1 desert, 2 tropical, 3 links (DERIVED numbering)
    int par = 0, prevPar = 0;     // pars of this hole and the one before it (type 30)
    unsigned flags = 0, prevFlags = 0;   // hole flag words (type 23 uses 4 and 8, type 30 uses 0x20 and 0x40)
    unsigned tick = 0;            // game tick counter (type 35 rotates with it)
    std::string name = "Pat";     // personal name used for both name tokens
    CellInfo cell;                // the cell named by the stored location (cell = a + 50 * b)
    std::string celebrity;        // celebrity name for type 22 (location indexes the table; the viewer resolves it)
};

// Name of a terrain id 0..22 (singular, plural when plural is true and the tile is a tree class). Empty for other ids.
const char* terrainName(int id, bool plural);
bool terrainIsTree(int id);
// Name of a landmark kind 0..18 (16 to 18 are the unsightly ones), empty otherwise.
const char* landmarkKindName(int kind);
// Name of an animal index 0..8, empty otherwise.
const char* animalName(int idx);
// Bare object name for a scenic cell (section 3.2 of the doc).
std::string cellObjectName(const CellInfo& c, int theme);

// Sentence for an event type and its stored location (already masked with 0x3fff).
CommentOut commentText(int type, int location, const CommentCtx& ctx);

// Hole type name for the Hole Stats title etc. is in holestats.h. Default hole names for proper-named holes.
std::string defaultHoleName(int par, int holeNumber);

}  // namespace sg
