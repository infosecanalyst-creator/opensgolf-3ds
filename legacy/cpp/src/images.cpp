#include "sg/assets.h"
#include <cstdio>
#include <cstring>

namespace sg {

bool readFile(const std::string& path, Bytes& out) {
    FILE* f = std::fopen(path.c_str(), "rb");
    if (!f) return false;
    std::fseek(f, 0, SEEK_END);
    long n = std::ftell(f);
    std::fseek(f, 0, SEEK_SET);
    out.resize(n > 0 ? (size_t)n : 0);
    size_t got = n > 0 ? std::fread(out.data(), 1, out.size(), f) : 0;
    std::fclose(f);
    return got == out.size();
}

static uint16_t u16(const uint8_t* p) { return (uint16_t)(p[0] | (p[1] << 8)); }
static uint32_t u32(const uint8_t* p) { return p[0] | (p[1] << 8) | (p[2] << 16) | ((uint32_t)p[3] << 24); }

Rgba toRgba(const Indexed& in, int transparentIndex) {
    Rgba o;
    o.w = in.w; o.h = in.h;
    o.px.resize((size_t)in.w * in.h * 4);
    for (size_t i = 0; i < (size_t)in.w * in.h; i++) {
        uint8_t c = in.idx[i];
        o.px[i * 4 + 0] = in.pal[c * 3 + 0];
        o.px[i * 4 + 1] = in.pal[c * 3 + 1];
        o.px[i * 4 + 2] = in.pal[c * 3 + 2];
        o.px[i * 4 + 3] = (int)c == transparentIndex ? 0 : 255;
    }
    return o;
}

// ---------------------------------------------------------------- PCX (v5, RLE)
bool decodePcx(const Bytes& d, Rgba& out, std::string& err) {
    if (d.size() < 128 || d[0] != 10 || d[2] != 1) { err = "not an RLE PCX"; return false; }
    int bpp = d[3], planes = d[65];
    uint32_t w = u16(&d[8]) - u16(&d[4]) + 1, h = u16(&d[10]) - u16(&d[6]) + 1;
    uint32_t bpl = u16(&d[66]);
    if (w == 0 || h == 0 || w > 16384 || h > 16384) { err = "bad dimensions"; return false; }
    if (bpp != 8 || (planes != 1 && planes != 3) || bpl < w) { err = "unsupported PCX layout"; return false; }
    size_t pos = 128, lineBytes = (size_t)planes * bpl;
    Bytes scan(lineBytes);
    Indexed ix; Rgba rgb;
    if (planes == 1) {
        ix.w = w; ix.h = h; ix.idx.resize((size_t)w * h);
    } else {
        rgb.w = w; rgb.h = h; rgb.px.resize((size_t)w * h * 4);
    }
    for (uint32_t y = 0; y < h; y++) {
        size_t n = 0;
        while (n < lineBytes) {
            if (pos >= d.size()) { err = "truncated PCX data"; return false; }
            uint8_t b = d[pos++];
            size_t run = 1;
            if ((b & 0xC0) == 0xC0) {
                run = b & 0x3F;
                if (pos >= d.size()) { err = "truncated PCX data"; return false; }
                b = d[pos++];
            }
            while (run-- && n < lineBytes) scan[n++] = b;
        }
        if (planes == 1) {
            std::memcpy(&ix.idx[(size_t)y * w], scan.data(), w);
        } else {
            for (uint32_t x = 0; x < w; x++) {
                uint8_t* o = &rgb.px[((size_t)y * w + x) * 4];
                o[0] = scan[x]; o[1] = scan[bpl + x]; o[2] = scan[2 * bpl + x]; o[3] = 255;
            }
        }
    }
    if (planes == 3) { out = std::move(rgb); return true; }
    if (d.size() < pos + 769 || d[d.size() - 769] != 0x0C) { err = "missing PCX palette"; return false; }
    std::memcpy(ix.pal, &d[d.size() - 768], 768);
    out = toRgba(ix);
    return true;
}

// ---------------------------------------------------------------- TGA
bool decodeTga(const Bytes& d, Rgba& out, std::string& err) {
    if (d.size() < 18) { err = "short TGA"; return false; }
    int idLen = d[0], type = d[2], bpp = d[16], desc = d[17];
    uint32_t w = u16(&d[12]), h = u16(&d[14]);
    if (d[1] != 0 || (type != 2 && type != 10) || (bpp != 24 && bpp != 32)) { err = "unsupported TGA"; return false; }
    if (w == 0 || h == 0 || w > 16384 || h > 16384) { err = "bad dimensions"; return false; }
    size_t bytesPer = bpp / 8, pos = 18 + idLen, total = (size_t)w * h;
    Bytes raw(total * bytesPer);
    if (type == 2) {
        if (d.size() < pos + raw.size()) { err = "truncated TGA"; return false; }
        std::memcpy(raw.data(), &d[pos], raw.size());
    } else {
        size_t n = 0;
        while (n < total) {
            if (pos >= d.size()) { err = "truncated TGA"; return false; }
            uint8_t c = d[pos++];
            size_t cnt = (c & 0x7F) + 1;
            if (cnt > total - n) cnt = total - n;
            if (c & 0x80) {
                if (pos + bytesPer > d.size()) { err = "truncated TGA"; return false; }
                for (size_t i = 0; i < cnt; i++) std::memcpy(&raw[(n + i) * bytesPer], &d[pos], bytesPer);
                pos += bytesPer;
            } else {
                if (pos + cnt * bytesPer > d.size()) { err = "truncated TGA"; return false; }
                std::memcpy(&raw[n * bytesPer], &d[pos], cnt * bytesPer);
                pos += cnt * bytesPer;
            }
            n += cnt;
        }
    }
    out.w = w; out.h = h; out.px.resize(total * 4);
    bool topDown = desc & 0x20;
    for (uint32_t y = 0; y < h; y++) {
        uint32_t sy = topDown ? y : h - 1 - y;
        for (uint32_t x = 0; x < w; x++) {
            const uint8_t* s = &raw[((size_t)sy * w + x) * bytesPer];
            uint8_t* o = &out.px[((size_t)y * w + x) * 4];
            o[0] = s[2]; o[1] = s[1]; o[2] = s[0]; o[3] = bytesPer == 4 ? s[3] : 255;
        }
    }
    return true;
}

// ---------------------------------------------------------------- BMP (uncompressed 8/24 bit)
bool decodeBmp(const Bytes& d, Rgba& out, std::string& err) {
    if (d.size() < 54 || d[0] != 'B' || d[1] != 'M') { err = "not a BMP"; return false; }
    uint32_t off = u32(&d[10]), dib = u32(&d[14]);
    int32_t w = (int32_t)u32(&d[18]), h = (int32_t)u32(&d[22]);
    int bpp = u16(&d[28]);
    uint32_t comp = u32(&d[30]);
    bool bottomUp = h > 0;
    if (h < 0) h = -h;
    if (comp != 0 || (bpp != 24 && bpp != 8) || w <= 0 || h <= 0 || w > 16384 || h > 16384) { err = "unsupported BMP"; return false; }
    size_t stride = ((size_t)w * bpp / 8 + 3) & ~(size_t)3;
    if (d.size() < off + stride * h) { err = "truncated BMP"; return false; }
    out.w = w; out.h = h; out.px.resize((size_t)w * h * 4);
    for (int32_t y = 0; y < h; y++) {
        const uint8_t* row = &d[off + stride * (bottomUp ? h - 1 - y : y)];
        for (int32_t x = 0; x < w; x++) {
            uint8_t* o = &out.px[((size_t)y * w + x) * 4];
            if (bpp == 24) { o[0] = row[x * 3 + 2]; o[1] = row[x * 3 + 1]; o[2] = row[x * 3]; }
            else {
                const uint8_t* p = &d[14 + dib + row[x] * 4];
                o[0] = p[2]; o[1] = p[1]; o[2] = p[0];
            }
            o[3] = 255;
        }
    }
    return true;
}

// ---------------------------------------------------------------- WAV (PCM)
bool decodeWav(const Bytes& d, Wav& out, std::string& err) {
    if (d.size() < 12 || std::memcmp(&d[0], "RIFF", 4) || std::memcmp(&d[8], "WAVE", 4)) { err = "not a RIFF WAVE"; return false; }
    size_t pos = 12; bool haveFmt = false;
    while (pos + 8 <= d.size()) {
        uint32_t sz = u32(&d[pos + 4]);
        const uint8_t* body = &d[pos + 8];
        size_t avail = d.size() - (pos + 8);
        if (!std::memcmp(&d[pos], "fmt ", 4) && sz >= 16 && avail >= 16) {
            if (u16(body) != 1) { err = "non-PCM WAV"; return false; }
            out.channels = u16(body + 2); out.sampleRate = u32(body + 4); out.bits = u16(body + 14);
            haveFmt = true;
        } else if (!std::memcmp(&d[pos], "data", 4)) {
            if (!haveFmt) { err = "data before fmt"; return false; }
            size_t n = sz < avail ? sz : avail;
            out.pcm.assign(body, body + n);
            return true;
        }
        pos += 8 + sz + (sz & 1);
    }
    err = "no data chunk";
    return false;
}

}  // namespace sg
