// Autodesk FLI/FLC animation decoder (8-bit). Frames are fully composited.
#include "sg/assets.h"
#include <cstring>

namespace sg {

static uint16_t u16(const uint8_t* p) { return (uint16_t)(p[0] | (p[1] << 8)); }
static uint32_t u32(const uint8_t* p) { return p[0] | (p[1] << 8) | (p[2] << 16) | ((uint32_t)p[3] << 24); }

namespace {
struct Cur {
    const uint8_t* p; const uint8_t* end;
    bool ok = true;
    size_t left() const { return (size_t)(end - p); }
    uint8_t u8() { if (p >= end) { ok = false; return 0; } return *p++; }
    uint16_t w() { if (left() < 2) { ok = false; p = end; return 0; } uint16_t v = u16(p); p += 2; return v; }
};
}  // namespace

static void readPalette(Cur& c, Indexed& f, bool sixBit) {
    unsigned n = c.w(), idx = 0;
    while (n-- && c.ok) {
        idx += c.u8();
        unsigned cnt = c.u8();
        if (cnt == 0) cnt = 256;
        for (unsigned i = 0; i < cnt && c.ok; i++, idx++) {
            uint8_t rgb[3] = {c.u8(), c.u8(), c.u8()};
            if (idx < 256)
                for (int k = 0; k < 3; k++)
                    f.pal[idx * 3 + k] = sixBit ? (uint8_t)((rgb[k] << 2) | (rgb[k] >> 4)) : rgb[k];
        }
    }
}

static void byteRun(Cur& c, Indexed& f) {
    for (uint32_t y = 0; y < f.h && c.ok; y++) {
        c.u8();  // packet count, unreliable
        uint32_t x = 0;
        uint8_t* row = &f.idx[(size_t)y * f.w];
        while (x < f.w && c.ok) {
            int8_t s = (int8_t)c.u8();
            if (s >= 0) {
                uint8_t v = c.u8();
                for (int i = 0; i < s && x < f.w; i++) row[x++] = v;
            } else {
                for (int i = 0; i < -s && c.ok; i++) { uint8_t v = c.u8(); if (x < f.w) row[x++] = v; }
            }
        }
    }
}

// DELTA_FLI (type 12): byte oriented.
static void deltaFli(Cur& c, Indexed& f) {
    uint32_t y = c.w();
    uint32_t lines = c.w();
    for (; lines && c.ok && y < f.h; lines--, y++) {
        unsigned packets = c.u8();
        uint32_t x = 0;
        uint8_t* row = &f.idx[(size_t)y * f.w];
        while (packets-- && c.ok) {
            x += c.u8();
            int8_t s = (int8_t)c.u8();
            if (s >= 0) {
                for (int i = 0; i < s && c.ok; i++) { uint8_t v = c.u8(); if (x < f.w) row[x] = v; x++; }
            } else {
                uint8_t v = c.u8();
                for (int i = 0; i < -s; i++, x++) if (x < f.w) row[x] = v;
            }
        }
    }
}

// DELTA_FLC (type 7): word oriented.
static void deltaFlc(Cur& c, Indexed& f) {
    unsigned lines = c.w();
    uint32_t y = 0;
    while (lines && c.ok) {
        uint16_t op = c.w();
        if (!c.ok) break;
        switch (op >> 14) {
            case 0: {  // packet count for this line
                unsigned packets = op;
                uint32_t x = 0;
                while (packets-- && c.ok) {
                    x += c.u8();
                    int8_t s = (int8_t)c.u8();
                    if (s >= 0) {
                        for (int i = 0; i < s && c.ok; i++) {
                            uint8_t a = c.u8(), b = c.u8();
                            if (y < f.h) { if (x < f.w) f.idx[(size_t)y * f.w + x] = a; if (x + 1 < f.w) f.idx[(size_t)y * f.w + x + 1] = b; }
                            x += 2;
                        }
                    } else {
                        uint8_t a = c.u8(), b = c.u8();
                        for (int i = 0; i < -s; i++, x += 2)
                            if (y < f.h) { if (x < f.w) f.idx[(size_t)y * f.w + x] = a; if (x + 1 < f.w) f.idx[(size_t)y * f.w + x + 1] = b; }
                    }
                }
                y++; lines--;
                break;
            }
            case 2:  // last pixel of the current line; line continues with next word
                if (y < f.h) f.idx[(size_t)y * f.w + f.w - 1] = (uint8_t)(op & 0xFF);
                break;
            case 3:  // skip lines
                y += (uint16_t)(-(int16_t)op);
                break;
            default:
                return;  // opcode 01 is undefined
        }
    }
}

bool decodeFlc(const Bytes& d, Flc& out, std::string& err) {
    if (d.size() < 128) { err = "short FLC"; return false; }
    uint16_t magic = u16(&d[4]);
    if (magic != 0xAF12 && magic != 0xAF11) { err = "bad FLC magic"; return false; }
    uint32_t nframes = u16(&d[6]);
    out.w = u16(&d[8]); out.h = u16(&d[10]);
    if (u16(&d[12]) != 8) { err = "not 8-bit FLC"; return false; }
    uint32_t speed = u32(&d[16]);
    {   // Firaxis header extension, accepted only when it is self-consistent.
        uint32_t v = u16(&d[96]), fpv = u16(&d[98]);
        if (v >= 1 && v <= 8 && fpv >= 1 && v * fpv == nframes && u16(&d[104]) == 480 && u16(&d[106]) == 480) {
            out.views = v; out.framesPerView = fpv; out.hasExt = true;
            out.cropX = u16(&d[100]); out.cropY = u16(&d[102]);
            out.canvasW = 480; out.canvasH = 480;
            out.viewMask = u16(&d[112]);
        }
    }
    out.frameMs = magic == 0xAF11 ? speed * 1000 / 70 : speed;
    if (out.w == 0 || out.h == 0 || out.w > 4096 || out.h > 4096) { err = "bad dimensions"; return false; }

    Indexed cur;
    cur.w = out.w; cur.h = out.h; cur.idx.assign((size_t)out.w * out.h, 0);
    size_t pos = 128;
    uint32_t done = 0;
    // With the extension every view is stored as framesPerView + 1 frames (the last is a ring frame).
    const uint32_t stored = out.hasExt ? out.views * (out.framesPerView + 1) : nframes;
    while (done < stored && pos + 16 <= d.size()) {
        uint32_t fsize = u32(&d[pos]);
        uint16_t ftype = u16(&d[pos + 4]), nchunks = u16(&d[pos + 6]);
        if (fsize < 16 || pos + fsize > d.size()) { err = "bad frame size"; return false; }
        if (ftype != 0xF1FA) { pos += fsize; continue; }  // e.g. prefix chunk
        size_t cp = pos + 16, fend = pos + fsize;
        for (unsigned i = 0; i < nchunks; i++) {
            if (cp + 6 > fend) break;
            uint32_t csz = u32(&d[cp]);
            uint16_t ct = u16(&d[cp + 4]);
            if (csz < 6 || cp + csz > fend) {
                // Firaxis' FLC writer sometimes left the chunk size field uninitialised
                // (0xCDCDCDCD). Recover: palettes are self-delimiting, a final chunk
                // runs to the end of the frame.
                if (ct == 4 || ct == 11) {
                    Indexed scratch;
                    Cur probe{&d[cp + 6], &d[fend]};
                    readPalette(probe, scratch, ct == 11);
                    if (!probe.ok) { err = "bad palette chunk size"; return false; }
                    csz = 6 + (uint32_t)(probe.p - &d[cp + 6]);
                } else if (i + 1 == nchunks) {
                    csz = (uint32_t)(fend - cp);
                } else {
                    err = "bad chunk size";
                    return false;
                }
            }
            Cur c{&d[cp + 6], &d[cp + csz]};
            switch (ct) {
                case 4:  readPalette(c, cur, false); break;
                case 11: readPalette(c, cur, true); break;
                case 7:  deltaFlc(c, cur); break;
                case 12: deltaFli(c, cur); break;
                case 15: byteRun(c, cur); break;
                case 13: std::fill(cur.idx.begin(), cur.idx.end(), 0); break;
                case 16:
                    if (c.left() >= cur.idx.size()) std::memcpy(cur.idx.data(), c.p, cur.idx.size());
                    break;
                default: break;  // 18 = postage stamp, 18/others ignored
            }
            if (!c.ok) { err = "truncated chunk in frame " + std::to_string(done); return false; }
            cp += csz;
        }
        if (!(out.hasExt && done % (out.framesPerView + 1) == out.framesPerView)) out.frames.push_back(cur);
        done++;
        pos = fend;
    }
    if (out.frames.empty()) { err = "no frames"; return false; }
    return true;
}

}  // namespace sg
