#include "sg/charrec.h"
#include <cstdio>
#include <cstring>

namespace sg {

bool charFromBytes(const std::vector<uint8_t>& b, CharRec& c) {
    if (b.size() < 0x722) return false;
    c = CharRec();
    std::memcpy(c.title, &b[0], 16); std::memcpy(c.name, &b[0x10], 16);
    c.title[15] = c.name[15] = 0;
    c.traits = b[0x20]; c.flagB = b[0x21]; c.head = b[0x22]; c.bodyHat = b[0x23]; c.shirt = b[0x24]; c.pants = b[0x25]; c.altSkin = b[0x26]; c.skin = b[0x27]; c.hair = b[0x28];
    c.flags = (uint32_t)b[0x2c] | (uint32_t)b[0x2d] << 8 | (uint32_t)b[0x2e] << 16 | (uint32_t)b[0x2f] << 24;
    std::memcpy(c.bio, &b[0x30], 512); c.bio[511] = 0;
    for (int i = 0; i < 25; i++) { std::memcpy(c.dialogue[i], &b[0x230 + 50 * (size_t)i], 50); c.dialogue[i][49] = 0; }
    std::memcpy(c.skills, &b[0x712], 10);
    return true;
}

std::vector<uint8_t> charToBytes(const CharRec& c) {
    std::vector<uint8_t> b(0x722, 0);
    std::memcpy(&b[0], c.title, 16); std::memcpy(&b[0x10], c.name, 16);
    b[0x20] = c.traits; b[0x21] = c.flagB; b[0x22] = c.head; b[0x23] = c.bodyHat; b[0x24] = c.shirt; b[0x25] = c.pants; b[0x26] = c.altSkin; b[0x27] = c.skin; b[0x28] = c.hair;
    for (int i = 0; i < 4; i++) b[0x2c + (size_t)i] = (uint8_t)(c.flags >> (8 * i));
    std::memcpy(&b[0x30], c.bio, 512);
    for (int i = 0; i < 25; i++) std::memcpy(&b[0x230 + 50 * (size_t)i], c.dialogue[i], 50);
    std::memcpy(&b[0x712], c.skills, 10);
    return b;
}

bool charLoadFile(const std::string& path, CharRec& out) {
    FILE* f = std::fopen(path.c_str(), "rb");
    if (!f) return false;
    std::vector<uint8_t> b; uint8_t buf[4096]; size_t n;
    while ((n = std::fread(buf, 1, sizeof buf, f)) > 0) b.insert(b.end(), buf, buf + n);
    std::fclose(f);
    return charFromBytes(b, out);
}

bool charSaveFile(const std::string& path, const CharRec& c) {
    FILE* f = std::fopen(path.c_str(), "wb");
    if (!f) return false;
    const std::vector<uint8_t> b = charToBytes(c);
    const bool ok = std::fwrite(b.data(), 1, b.size(), f) == b.size();
    std::fclose(f);
    return ok;
}

bool charNameValid(const std::string& n) {
    if (n.empty() || n.size() > 15) return false;
    for (char ch : n) if (ch == '/' || ch == '\\' || ch == ':' || ch == '*' || ch == '?' || ch == '"' || ch == '<' || ch == '>' || ch == '|' || (unsigned char)ch < 32) return false;
    return true;
}

}  // namespace sg
