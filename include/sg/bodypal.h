// Golfer recolouring (docs/DECODE_BODIES.md section 2, EXACT unless marked). A golfer sprite is palette indexed; the exe rebuilds a 256 entry table from the
// MaleSwap01..10 / FemaleSwap01..10 palettes before every draw: entries 0..119 from Swap01, then 0..19 (shirt), 20..39 (pants), 40..59 (skin) and 60..79 (hat)
// each from the Swap file numbered value + 1, then 80..89 and 90..96 from two more choices. Index 255 stays the key colour.
// DERIVED: which variable feeds 80..89 and 90..96 (male: 80..86 take the alternate skin; female: 80..89 the hair, 90..96 the alternate skin), from the way the files vary.
#pragma once
#include <cstdint>
#include <cstring>
#include <map>
#include <string>
#include "sg/assets.h"
#include "sg/sprites.h"

namespace sg {

struct BodyLook {
    bool female = false;
    int body = 0;                    // 0..7: 0 PLS, 1 KLS, 2 PSS, 3 SSS (male); 4 FemalePLS, 5 FemaleSSS, 6 FemalePSS, 7 FemaleSkTT
    int shirt = 0, pants = 0, skin = 0, hat = 0, hair = 0, altSkin = 0;   // 0..9 (skin 0..3)
    std::string key() const { char b[64]; std::snprintf(b, sizeof b, "%d.%d.%d.%d.%d.%d.%d.%d", female, body, shirt, pants, skin, hat, hair, altSkin); return b; }
};

inline const char* bodySetName(const BodyLook& l) {
    static const char* m[4] = {"MalePLS", "MaleKLS", "MalePSS", "MaleSSS"};
    static const char* f[4] = {"FemalePLS", "FemaleSSS", "FemalePSS", "FemaleSkTT"};
    return l.female ? f[((l.body - 4) % 4 + 4) % 4] : m[l.body & 3];
}

// Builds the 768 byte table. Returns false when the Swap files are missing.
inline bool composeBodyPalette(const std::string& gameDir, const BodyLook& l, uint8_t out[768]) {
    static std::map<std::string, std::vector<uint8_t>> cache;
    auto swap = [&](int n) -> const uint8_t* {
        char nm[96]; std::snprintf(nm, sizeof nm, "%s/Bodies/%sSwap%02d.pcx", gameDir.c_str(), l.female ? "Female" : "Male", std::clamp(n, 0, 9) + 1);
        auto it = cache.find(nm);
        if (it == cache.end()) {
            std::vector<uint8_t> pal(768, 0); Bytes d; uint8_t p[768];
            if (readFile(nm, d) && readPcxPalette(d, p)) std::memcpy(pal.data(), p, 768); else pal.clear();
            it = cache.emplace(nm, std::move(pal)).first;
        }
        return it->second.empty() ? nullptr : it->second.data();
    };
    const uint8_t* base = swap(0); if (!base) return false;
    std::memcpy(out, base, 768);
    auto copyRange = [&](int value, int lo, int hi) { const uint8_t* s = swap(value); if (!s) return; std::memcpy(out + lo * 3, s + lo * 3, (size_t)(hi - lo + 1) * 3); };
    copyRange(l.shirt, 0, 19); copyRange(l.pants, 20, 39); copyRange(l.skin, 40, 59); copyRange(l.hat, 60, 79);
    if (l.female) { copyRange(l.hair, 80, 89); copyRange(l.altSkin, 90, 96); } else copyRange(l.altSkin, 80, 86);
    out[255 * 3] = 255; out[255 * 3 + 1] = 0; out[255 * 3 + 2] = 255;
    return true;
}

}  // namespace sg
