#include "sg/sprites.h"
#include <cstring>

namespace sg {

bool readPcxPalette(const Bytes& d, uint8_t pal[768]) {
    if (d.size() < 769 || d[d.size() - 769] != 0x0C) return false;
    std::memcpy(pal, &d[d.size() - 768], 768);
    return true;
}

bool loadSprite(const std::string& path, Sprite& out, std::string& err, bool shadow, const std::string& palettePcx) {
    Bytes d;
    Flc f;
    if (!readFile(path, d)) { err = "cannot read " + path; return false; }
    if (!decodeFlc(d, f, err)) { err = path + ": " + err; return false; }
    uint8_t override_[768];
    bool useOverride = false;
    if (!palettePcx.empty()) {
        Bytes p;
        useOverride = readFile(palettePcx, p) && readPcxPalette(p, override_);
    }
    out = Sprite();
    out.shadow = shadow;
    out.w = f.w; out.h = f.h; out.frameMs = f.frameMs;
    out.views = f.hasExt ? (int)f.views : 1;
    out.framesPerView = f.hasExt ? (int)f.framesPerView : (int)f.frames.size();
    out.viewMask = f.viewMask;
    out.anchorX = f.hasExt ? (int)(f.canvasW / 2) - (int)f.cropX : (int)f.w / 2;
    out.anchorY = f.hasExt ? (int)(f.canvasH / 2) - (int)f.cropY : (int)f.h;
    for (const Indexed& src : f.frames) {
        Indexed ix = src;
        if (useOverride) std::memcpy(ix.pal, override_, 768);
        Rgba img = toRgba(ix, 255);
        if (!shadow) {  // texture filtering must not pull the key colour into the edges: spread neighbour colours outwards
            const int W = (int)img.w, H = (int)img.h;
            for (int pass = 0; pass < 2; pass++) {
                Bytes next = img.px;
                for (int y = 0; y < H; y++)
                    for (int x = 0; x < W; x++) {
                        uint8_t* p = &next[((size_t)y * W + x) * 4];
                        if (img.px[((size_t)y * W + x) * 4 + 3]) continue;
                        for (int dy = -1; dy <= 1; dy++)
                            for (int dx = -1; dx <= 1; dx++) {
                                int nx = x + dx, ny = y + dy;
                                if (nx < 0 || ny < 0 || nx >= W || ny >= H) continue;
                                const uint8_t* q = &img.px[((size_t)ny * W + nx) * 4];
                                if (q[3] && (p[0] != q[0] || p[1] != q[1] || p[2] != q[2])) { p[0] = q[0]; p[1] = q[1]; p[2] = q[2]; goto done; }
                            }
                    done:;
                    }
                img.px = std::move(next);
            }
        }
        if (shadow) {
            for (size_t i = 0; i < (size_t)ix.w * ix.h; i++) {
                if (ix.idx[i] == 255) continue;
                // white = lightest density, pure green = darkest.
                float dens = 1.0f - ix.pal[ix.idx[i] * 3] / 255.0f;
                img.px[i * 4 + 0] = img.px[i * 4 + 1] = img.px[i * 4 + 2] = 0;
                img.px[i * 4 + 3] = (uint8_t)(70 + dens * 90);
            }
        }
        out.frames.push_back(std::move(img));
    }
    return true;
}

}  // namespace sg
