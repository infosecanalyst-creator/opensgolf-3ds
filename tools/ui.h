// Screen-space drawing for the SimGolf viewer: images from the disc's Interface folder, and text in the game's own font.
// Everything is drawn in a virtual 800x600 space (the original's UI resolution), scaled to fit the window with black bars.
#pragma once
#ifdef __APPLE__
#define GL_SILENCE_DEPRECATION
#endif
#include <SDL_opengl.h>
#include <string>

namespace ui {

struct Image {
    GLuint tex = 0;
    int w = 0, h = 0;
};
// Loads a PCX from the disc. With magentaKey, pure magenta pixels become transparent (the original's colour key).
// Loads a shadow sheet (the disc's s_*.pcx): green pixels become translucent black (alpha 0..1), everything else transparent. The exe blends these as drop shades behind panels.
bool uploadRgba(const unsigned char* rgba, int w, int h, Image& out);   // makes a texture from raw RGBA
bool loadShade(const std::string& path, Image& out, float alpha);
bool loadPcxCircles(const std::string& path, Image& out, int cx0, int cy0, int pitch, int count, float r);   // keys everything outside a circle in each cell (round buttons drawn on a square backdrop)
bool loadPcx(const std::string& path, Image& out, bool magentaKey, int keyRgb = -1, const std::string& alphaPath = "");   // keyRgb: another colour key as 0xRRGGBB

class Font {
  public:
    bool load(const std::string& ttfPath);                 // bakes Latin-1 glyphs at a fixed size
    float width(const std::string& utf8, float size) const;
    // Draws with the baseline at y. Sizes are in virtual pixels. Colour is modulated with the glyph alpha.
    void draw(float x, float y, const std::string& utf8, float size, float r, float g, float b, float a = 1.0f) const;
    void drawCentered(float cx, float y, const std::string& utf8, float size, float r, float g, float b, float a = 1.0f) const;
    bool ok() const { return tex_ != 0; }

  private:
    GLuint tex_ = 0;
    static constexpr int kFirst = 32, kCount = 224, kBakePx = 32, kAtlas = 512;
    alignas(4) unsigned char cdata_[kCount * 20] = {};      // stbtt_bakedchar (20 bytes each), kept opaque here
};

// The virtual to window mapping of the last beginScreen call.
struct View {
    float scale = 1, ox = 0, oy = 0;
    float toVirtualX(float winPx) const { return (winPx - ox) / scale; }
    float toVirtualY(float winPx) const { return (winPx - oy) / scale; }
};
View beginScreen(int drawW, int drawH, bool clear = true);   // orthographic projection for 800x600 virtual units; clears to black unless told not to
void endScreen();
void drawImage(const Image& im, float dx, float dy, float sx, float sy, float sw, float sh);   // sub-rectangle at its own size
void drawImageScaled(const Image& im, float dx, float dy, float dw, float dh, float sx, float sy, float sw, float sh);   // sub-rectangle stretched to dw x dh
void drawImage(const Image& im, float dx, float dy);                                           // whole image
void fillRect(float x, float y, float w, float h, float r, float g, float b, float a);

}  // namespace ui
