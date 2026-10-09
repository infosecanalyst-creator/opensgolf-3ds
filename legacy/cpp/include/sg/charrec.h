// The player's character record and the .pro / .chr / .glf character files (docs/DECODE_CUSTOMISE.md section 3.1 and 5.5, docs/FORMATS.md).
// Layout read from the owner's disc files and the exe's save and load routines; the port reads and writes the base record and skip the embedded portrait.
#pragma once
#include <cstdint>
#include <string>
#include <vector>

namespace sg {

struct CharRec {
    char title[16] = {};            // "Profession" line
    char name[16] = {};
    uint8_t traits = 0;             // five trait bits (the left column of buttons)
    uint8_t flagB = 0;              // bits 0..2 age group, bits 3..6 marital status, bit 7 female
    uint8_t head = 0;               // head index (0..18 built in, 19 and up custom portraits)
    uint8_t bodyHat = 0;            // bits 4..5 body type, low nibble a fourth palette selector
    uint8_t shirt = 0, pants = 0, altSkin = 0, skin = 0, hair = 0;
    uint32_t flags = 0;             // bits 0..2 clothing toggles, bit 3 child, bit 7 edited
    char bio[512] = {};
    char dialogue[25][50] = {};
    uint8_t skills[10] = {};        // power, long driver, accurate driver, accurate irons, accurate putter, draw, fade, backspin, recovery, luck
    bool female() const { return flagB & 0x80; }
    int bodyType() const { return (bodyHat >> 4) & 3; }
    int age() const { return flagB & 1 ? 0 : flagB & 2 ? 1 : flagB & 4 ? 2 : -1; }          // 0 young, 1 middle aged, 2 mature, -1 unset
    int marital() const { return flagB & 8 ? 0 : flagB & 0x10 ? 1 : flagB & 0x20 ? 2 : flagB & 0x40 ? 3 : -1; }   // single, married, divorced, widowed
    void setAge(int a) { flagB = (uint8_t)((flagB & ~7) | (1 << a)); }
    void setMarital(int m) { flagB = (uint8_t)((flagB & ~0x78) | (8 << m)); }
    void cycleBody(bool up) { int b = (bodyType() + (up ? 1 : 3)) & 3; bodyHat = (uint8_t)((bodyHat & 0xcf) | (b << 4)); flags |= 0x80; }
    void cycleShirt(bool up) { shirt = (uint8_t)((shirt + (up ? 1 : 9)) % 10); flags |= 0x80; }
    void cyclePants(bool up) { pants = (uint8_t)((pants + (up ? 1 : 9)) % 10); flags |= 0x80; }
    void cycleSkin(bool up) { skin = (uint8_t)((skin + (up ? 1 : 3)) % 4); flags |= 0x80; }
    void cycleHair(bool up) { hair = (uint8_t)((hair + (up ? 1 : 4)) % 5); flags |= 0x80; }
    void cycleAge(bool up) { int a = age(); a = a < 0 ? (up ? 0 : 2) : (a + (up ? 1 : 2)) % 3; setAge(a); }
    void cycleMarital(bool up) { int m = marital(); m = m < 0 ? (up ? 0 : 3) : (m + (up ? 1 : 3)) % 4; setMarital(m); }
};

constexpr size_t kCharRecBytes = 0x230, kCharFileBase = 0x722;   // record, then 25 x 50 byte slots, then 16 skill bytes, then optional "*PCXFILE" and a PCX

bool charFromBytes(const std::vector<uint8_t>& b, CharRec& out);
std::vector<uint8_t> charToBytes(const CharRec& c);
bool charLoadFile(const std::string& path, CharRec& out);
bool charSaveFile(const std::string& path, const CharRec& c);
bool charNameValid(const std::string& n);   // a plain file name: non empty, no path separators or wildcard characters

}  // namespace sg
