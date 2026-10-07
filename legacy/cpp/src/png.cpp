// Minimal PNG writer (stored deflate blocks, no zlib dependency). For debugging/export.
#include "sg/assets.h"
#include <cstdio>
#ifdef SG_HAVE_ZLIB
#include <zlib.h>
#endif

namespace sg {

static uint32_t crcTable[256];
static void initCrc() {
    static bool done = false;
    if (done) return;
    for (uint32_t n = 0; n < 256; n++) {
        uint32_t c = n;
        for (int k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320u ^ (c >> 1) : c >> 1;
        crcTable[n] = c;
    }
    done = true;
}
static uint32_t crc(const uint8_t* p, size_t n, uint32_t c = 0xFFFFFFFFu) {
    for (size_t i = 0; i < n; i++) c = crcTable[(c ^ p[i]) & 0xFF] ^ (c >> 8);
    return c;
}
static void be32(Bytes& b, uint32_t v) { for (int s = 24; s >= 0; s -= 8) b.push_back((uint8_t)(v >> s)); }
static void chunk(FILE* f, const char* type, const Bytes& body) {
    Bytes b;
    be32(b, (uint32_t)body.size());
    b.insert(b.end(), type, type + 4);
    b.insert(b.end(), body.begin(), body.end());
    uint32_t c = ~crc(b.data() + 4, b.size() - 4);
    be32(b, c);
    std::fwrite(b.data(), 1, b.size(), f);
}

bool writePng(const std::string& path, const Rgba& img) {
    initCrc();
    if (!img.w || !img.h || img.px.size() < (size_t)img.w * img.h * 4) return false;
    FILE* f = std::fopen(path.c_str(), "wb");
    if (!f) return false;
    static const uint8_t sig[8] = {0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
    std::fwrite(sig, 1, 8, f);
    Bytes ihdr;
    be32(ihdr, img.w); be32(ihdr, img.h);
    ihdr.insert(ihdr.end(), {8, 6, 0, 0, 0});
    chunk(f, "IHDR", ihdr);
    Bytes raw;
    raw.reserve((size_t)(img.w * 4 + 1) * img.h);
    for (uint32_t y = 0; y < img.h; y++) {
        raw.push_back(0);
        raw.insert(raw.end(), &img.px[(size_t)y * img.w * 4], &img.px[(size_t)y * img.w * 4] + img.w * 4);
    }
    Bytes z;
#ifdef SG_HAVE_ZLIB
    uLongf zlen = compressBound((uLong)raw.size());
    z.resize(zlen);
    if (compress2(z.data(), &zlen, raw.data(), (uLong)raw.size(), 6) != Z_OK) { std::fclose(f); return false; }
    z.resize(zlen);
#else
    z = {0x78, 0x01};
    uint32_t a = 1, b = 0;
    for (uint8_t v : raw) { a = (a + v) % 65521; b = (b + a) % 65521; }
    for (size_t pos = 0; pos < raw.size();) {
        size_t n = raw.size() - pos < 65535 ? raw.size() - pos : 65535;
        z.push_back(pos + n == raw.size() ? 1 : 0);
        z.push_back(n & 0xFF); z.push_back(n >> 8);
        z.push_back(~n & 0xFF); z.push_back((~n >> 8) & 0xFF);
        z.insert(z.end(), raw.begin() + pos, raw.begin() + pos + n);
        pos += n;
    }
    be32(z, (b << 16) | a);
#endif
    chunk(f, "IDAT", z);
    chunk(f, "IEND", {});
    std::fclose(f);
    return true;
}

}  // namespace sg
