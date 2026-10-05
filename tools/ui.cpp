#include "ui.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <vector>
#include "sg/assets.h"
#define STB_TRUETYPE_IMPLEMENTATION
#include "stb_truetype.h"

#ifndef GL_CLAMP_TO_EDGE
#define GL_CLAMP_TO_EDGE 0x812F
#endif

namespace ui {

// Colour-keyed art keeps anti-aliased pixels that are a blend of the art and the key colour, which show as a pink ring once the texture is filtered.
// Remove those pixels next to the transparent area, then copy the nearest opaque colour into the transparent pixels so filtering cannot pull the key colour in.
static void cleanKeyFringe(sg::Rgba& img, bool pinkKey) {
    const int w = (int)img.w, h = (int)img.h;
    auto A = [&](int x, int y) -> unsigned char& { return img.px[((size_t)y * w + x) * 4 + 3]; };
    auto R = [&](int x, int y) -> unsigned char* { return &img.px[((size_t)y * w + x) * 4]; };
    auto nearClear = [&](int x, int y) { for (int dy = -1; dy <= 1; dy++) for (int dx = -1; dx <= 1; dx++) { const int xx = x + dx, yy = y + dy; if (xx >= 0 && yy >= 0 && xx < w && yy < h && A(xx, yy) == 0) return true; } return false; };
    if (pinkKey)
        for (int pass = 0; pass < 2; pass++) {
            std::vector<size_t> kill;
            for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) {
                if (A(x, y) == 0) continue;
                const unsigned char* c = R(x, y);
                const int r = c[0], g = c[1], b = c[2];
                if (r > 150 && b > 110 && g + 55 < std::min(r, b) && nearClear(x, y)) kill.push_back((size_t)y * w + x);
            }
            for (size_t i : kill) img.px[i * 4 + 3] = 0;
        }
    std::vector<unsigned char> copy = img.px;
    for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) {
        if (A(x, y) != 0) continue;
        int sr = 0, sg = 0, sb = 0, n = 0;
        for (int dy = -2; dy <= 2; dy++) for (int dx = -2; dx <= 2; dx++) {
            const int xx = x + dx, yy = y + dy; if (xx < 0 || yy < 0 || xx >= w || yy >= h) continue;
            const unsigned char* c = &copy[((size_t)yy * w + xx) * 4]; if (c[3] == 0) continue;
            sr += c[0]; sg += c[1]; sb += c[2]; n++;
        }
        unsigned char* d = R(x, y);
        if (n) { d[0] = (unsigned char)(sr / n); d[1] = (unsigned char)(sg / n); d[2] = (unsigned char)(sb / n); }
        else { d[0] = d[1] = d[2] = 0; }
    }
}

bool loadPcx(const std::string& path, Image& out, bool magentaKey, int keyRgb, const std::string& alphaPath) {
    sg::Bytes d; sg::Rgba img; std::string err;
    if (!sg::readFile(path, d) || !sg::decodePcx(d, img, err)) return false;
    if (!alphaPath.empty()) {   // a separate greyscale alpha sheet of the same size (the disc's _A / _alpha files)
        sg::Bytes da; sg::Rgba al;
        if (sg::readFile(alphaPath, da) && sg::decodePcx(da, al, err) && al.w == img.w && al.h == img.h)
            for (size_t i = 0; i + 3 < img.px.size(); i += 4) img.px[i + 3] = al.px[i];
    }
    if (magentaKey)
        for (size_t i = 0; i + 3 < img.px.size(); i += 4)
            if (img.px[i] == 255 && img.px[i + 1] == 0 && img.px[i + 2] == 255) img.px[i + 3] = 0;
    if (keyRgb >= 0)
        for (size_t i = 0; i + 3 < img.px.size(); i += 4)
            if (img.px[i] == ((keyRgb >> 16) & 255) && img.px[i + 1] == ((keyRgb >> 8) & 255) && img.px[i + 2] == (keyRgb & 255)) img.px[i + 3] = 0;
    if (magentaKey || keyRgb >= 0) cleanKeyFringe(img, magentaKey || keyRgb == 0xff00ff);
    glGenTextures(1, &out.tex);
    glBindTexture(GL_TEXTURE_2D, out.tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, (GLsizei)img.w, (GLsizei)img.h, 0, GL_RGBA, GL_UNSIGNED_BYTE, img.px.data());
    out.w = (int)img.w; out.h = (int)img.h;
    return true;
}

bool loadPcxCircles(const std::string& path, Image& out, int cx0, int cy0, int pitch, int count, float r) {
    sg::Bytes d; sg::Rgba img; std::string err;
    if (!sg::readFile(path, d) || !sg::decodePcx(d, img, err)) return false;
    for (int y = 0; y < (int)img.h; y++) for (int x = 0; x < (int)img.w; x++) {
        float best = 1e9f;
        for (int k = 0; k < count; k++) { const float dx = x + 0.5f - (cx0 + k * pitch), dy = y + 0.5f - cy0; best = std::min(best, std::sqrt(dx * dx + dy * dy)); }
        unsigned char* p = &img.px[((size_t)y * img.w + x) * 4];
        const float a = std::clamp(r + 0.5f - best, 0.0f, 1.0f);
        p[3] = (unsigned char)(a * 255);
    }
    cleanKeyFringe(img, false);
    glGenTextures(1, &out.tex);
    glBindTexture(GL_TEXTURE_2D, out.tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, (GLsizei)img.w, (GLsizei)img.h, 0, GL_RGBA, GL_UNSIGNED_BYTE, img.px.data());
    out.w = (int)img.w; out.h = (int)img.h;
    return true;
}

bool uploadRgba(const unsigned char* rgba, int w, int h, Image& out) {
    glGenTextures(1, &out.tex);
    glBindTexture(GL_TEXTURE_2D, out.tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, rgba);
    out.w = w; out.h = h;
    return true;
}

bool loadShade(const std::string& path, Image& out, float alpha) {
    sg::Bytes d; sg::Rgba img; std::string err;
    if (!sg::readFile(path, d) || !sg::decodePcx(d, img, err)) return false;
    for (size_t i = 0; i + 3 < img.px.size(); i += 4) {
        const bool green = img.px[i] == 0 && img.px[i + 1] == 255 && img.px[i + 2] == 0;
        img.px[i] = img.px[i + 1] = img.px[i + 2] = 0; img.px[i + 3] = green ? (unsigned char)(alpha * 255) : 0;
    }
    glGenTextures(1, &out.tex);
    glBindTexture(GL_TEXTURE_2D, out.tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE); glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, (GLsizei)img.w, (GLsizei)img.h, 0, GL_RGBA, GL_UNSIGNED_BYTE, img.px.data());
    out.w = (int)img.w; out.h = (int)img.h;
    return true;
}

bool Font::load(const std::string& ttfPath) {
    sg::Bytes ttf;
    if (!sg::readFile(ttfPath, ttf)) return false;
    std::vector<unsigned char> bmp((size_t)kAtlas * kAtlas, 0);
    static_assert(sizeof(stbtt_bakedchar) == 20, "stbtt_bakedchar layout");
    int r = stbtt_BakeFontBitmap(ttf.data(), 0, (float)kBakePx, bmp.data(), kAtlas, kAtlas, kFirst, kCount, (stbtt_bakedchar*)cdata_);
    if (r <= 0) return false;
    glGenTextures(1, &tex_);
    glBindTexture(GL_TEXTURE_2D, tex_);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_ALPHA, kAtlas, kAtlas, 0, GL_ALPHA, GL_UNSIGNED_BYTE, bmp.data());
    return true;
}

// UTF-8 to Latin-1 code points (enough for the section sign); anything else becomes '?'.
static std::vector<int> decode(const std::string& s) {
    std::vector<int> out;
    for (size_t i = 0; i < s.size(); i++) {
        unsigned char c = (unsigned char)s[i];
        if (c < 0x80) out.push_back(c == 'I' ? 'l' : c);   // the face's capital I is drawn like a dotted i; its l is the plain stroke the game shows for I
        else if ((c & 0xE0) == 0xC0 && i + 1 < s.size()) { out.push_back(((c & 0x1F) << 6) | ((unsigned char)s[i + 1] & 0x3F)); i++; }
        else out.push_back('?');
    }
    return out;
}

float Font::width(const std::string& s, float size) const {
    const float k = size / kBakePx;
    float x = 0, y = 0;
    for (int cp : decode(s)) {
        if (cp < kFirst || cp >= kFirst + kCount) continue;
        stbtt_aligned_quad q;
        stbtt_GetBakedQuad((const stbtt_bakedchar*)cdata_, kAtlas, kAtlas, cp - kFirst, &x, &y, &q, 1);
    }
    return x * k;
}

void Font::draw(float x, float y, const std::string& s, float size, float r, float g, float b, float a) const {
    if (!tex_) return;
    const float k = size / kBakePx;
    glEnable(GL_TEXTURE_2D);
    glBindTexture(GL_TEXTURE_2D, tex_);
    glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_MODULATE);
    glColor4f(r, g, b, a);
    float cx = 0, cy = 0;
    glBegin(GL_QUADS);
    for (int cp : decode(s)) {
        if (cp < kFirst || cp >= kFirst + kCount) continue;
        stbtt_aligned_quad q;
        stbtt_GetBakedQuad((const stbtt_bakedchar*)cdata_, kAtlas, kAtlas, cp - kFirst, &cx, &cy, &q, 1);
        glTexCoord2f(q.s0, q.t0); glVertex2f(x + q.x0 * k, y + q.y0 * k);
        glTexCoord2f(q.s1, q.t0); glVertex2f(x + q.x1 * k, y + q.y0 * k);
        glTexCoord2f(q.s1, q.t1); glVertex2f(x + q.x1 * k, y + q.y1 * k);
        glTexCoord2f(q.s0, q.t1); glVertex2f(x + q.x0 * k, y + q.y1 * k);
    }
    glEnd();
}

void Font::drawCentered(float cx, float y, const std::string& s, float size, float r, float g, float b, float a) const {
    draw(cx - width(s, size) * 0.5f, y, s, size, r, g, b, a);
}

View beginScreen(int drawW, int drawH, bool clear) {
    View v;
    v.scale = std::min((float)drawW / 800.0f, (float)drawH / 600.0f);
    v.ox = (drawW - 800.0f * v.scale) * 0.5f;
    v.oy = (drawH - 600.0f * v.scale) * 0.5f;
    glViewport(0, 0, drawW, drawH);
    if (clear) { glClearColor(0, 0, 0, 1); glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT); }
    glMatrixMode(GL_PROJECTION); glLoadIdentity();
    glOrtho(-v.ox / v.scale, (drawW - v.ox) / v.scale, (drawH - v.oy) / v.scale, -v.oy / v.scale, -1, 1);
    glMatrixMode(GL_MODELVIEW); glLoadIdentity();
    glDisable(GL_DEPTH_TEST); glDisable(GL_LIGHTING); glDisable(GL_CULL_FACE);
    glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    return v;
}

void endScreen() { glDisable(GL_BLEND); glEnable(GL_DEPTH_TEST); glColor4f(1, 1, 1, 1); }

void drawImage(const Image& im, float dx, float dy, float sx, float sy, float sw, float sh) {
    if (!im.tex) return;
    glEnable(GL_TEXTURE_2D);
    glBindTexture(GL_TEXTURE_2D, im.tex);
    glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_REPLACE);
    glColor4f(1, 1, 1, 1);
    // A sub-rectangle of a sheet is sampled half a texel inside its edges, so linear filtering at a scaled window does not pull in the neighbouring cut.
    const bool sub = sx > 0 || sy > 0 || sw < (float)im.w || sh < (float)im.h;
    const float in = sub ? 0.5f : 0.0f;
    const float u0 = (sx + in) / im.w, v0 = (sy + in) / im.h, u1 = (sx + sw - in) / im.w, v1 = (sy + sh - in) / im.h;
    glBegin(GL_QUADS);
    glTexCoord2f(u0, v0); glVertex2f(dx, dy);
    glTexCoord2f(u1, v0); glVertex2f(dx + sw, dy);
    glTexCoord2f(u1, v1); glVertex2f(dx + sw, dy + sh);
    glTexCoord2f(u0, v1); glVertex2f(dx, dy + sh);
    glEnd();
}

void drawImageScaled(const Image& im, float dx, float dy, float dw, float dh, float sx, float sy, float sw, float sh) {
    if (!im.tex) return;
    glEnable(GL_TEXTURE_2D);
    glBindTexture(GL_TEXTURE_2D, im.tex);
    glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_REPLACE);
    glColor4f(1, 1, 1, 1);
    const float u0 = (sx + 0.5f) / im.w, v0 = (sy + 0.5f) / im.h, u1 = (sx + sw - 0.5f) / im.w, v1 = (sy + sh - 0.5f) / im.h;
    glBegin(GL_QUADS);
    glTexCoord2f(u0, v0); glVertex2f(dx, dy);
    glTexCoord2f(u1, v0); glVertex2f(dx + dw, dy);
    glTexCoord2f(u1, v1); glVertex2f(dx + dw, dy + dh);
    glTexCoord2f(u0, v1); glVertex2f(dx, dy + dh);
    glEnd();
}

void drawImage(const Image& im, float dx, float dy) { drawImage(im, dx, dy, 0, 0, (float)im.w, (float)im.h); }

void fillRect(float x, float y, float w, float h, float r, float g, float b, float a) {
    glDisable(GL_TEXTURE_2D);
    glColor4f(r, g, b, a);
    glBegin(GL_QUADS);
    glVertex2f(x, y); glVertex2f(x + w, y); glVertex2f(x + w, y + h); glVertex2f(x, y + h);
    glEnd();
    glColor4f(1, 1, 1, 1);
}

}  // namespace ui
