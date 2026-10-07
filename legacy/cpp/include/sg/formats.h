// SimGolf native port: parsers for the game's non-image data formats.
// Formats were worked out from the original files; see docs/FORMATS.md.
#pragma once
#include "sg/assets.h"

namespace sg {

// .chr / .glf / .pro : character definition.
//   0x000 char[16] title (e.g. "Heiress", "Space Cowboy")   null padded
//   0x010 char[16] name                                      null padded
//   0x020 u8[16]   raw attribute bytes (meaning not yet decoded)
//   0x030 ...      zero padding up to 0x230
//   0x230 char[50] x 25 dialogue slots (empty slot = all zero), then 16 zero bytes -> 1826 bytes
//   0x722 optional: "*PCXFILE" tag + 8-bit PCX, 140x420 = three 140x140 portraits stacked
//         (happy, neutral, angry). The PCX runs to the end of the file.
struct Character {
    std::string title, name;
    uint8_t attr[16] = {};
    std::vector<std::string> lines;   // always 25 entries, "" when unused
    std::vector<Rgba> portraits;      // 3 images when present, else empty
};
bool parseCharacter(const Bytes& data, Character& out, std::string& err);

// top10.sve : ten fixed 156-byte records.
//   char[64] player, char[64] course, u32 a (descending sort key), u32 b, u32 c,
//   u32 0, u32 0, u16 0, u16 tier, i32 -1
struct Top10Entry {
    std::string player, course;
    uint32_t a = 0, b = 0, c = 0;
    uint16_t tier = 0;
};
bool parseTop10(const Bytes& data, std::vector<Top10Entry>& out, std::string& err);

// .dta : text tables. Lines starting with '*' are comments; other non-blank lines are CSV rows
// (fields trimmed). celebrities.dta: name,type,skin,hair,shirt,pants.
// progolfers.dta: name,body,skin,hat,shirt,pants,<10 skill digits>.
using DtaRows = std::vector<std::vector<std::string>>;
bool parseDta(const std::string& text, DtaRows& out, std::string& err);

// One row of progolfers.dta. skill[] order (from the file's own header): power hitter, long driver, accurate
// driver, accurate irons, accurate putter, draw shot, fade shot, high backspin shot, recovery skills, luck.
// Each is 0..15 (hex digit); the header calls them the "max value for each skill".
struct ProGolfer {
    std::string name;
    int body = 0, skin = 0, hat = 0, shirt = 0, pants = 0;
    int skill[10] = {};
    int rating = 0;  // trailing number on some rows (30..115), meaning unknown
};
bool parseProGolfers(const std::string& text, std::vector<ProGolfer>& out, std::string& err);

// Themes/*/*.txt : conversation scripts. First line is the title; the rest are blank-line
// separated blocks, each a prompt line followed by 1-3 indented reply lines.
// Text may contain placeholders such as PARTNER and DATA that the game substitutes.
struct StoryBlock { std::string prompt; std::vector<std::string> replies; };
struct Story { std::string title; std::vector<StoryBlock> blocks; };
bool parseStory(const std::string& text, Story& out, std::string& err);

}  // namespace sg
