// SimGolf native port: pre-rendered sprites (trees, buildings, golfers...).
// Layout facts come from the Firaxis header extension of the .flc files, see docs/SPRITES.md.
#pragma once
#include "sg/assets.h"

namespace sg {

// World units covered by one sprite pixel. The sprites were rendered for the 800x600 camera, whose
// orthographic window is 1767 world units wide (see docs/TERRAIN.md): 1767 / 800.
constexpr float kSpriteUnitsPerPixel = 1767.0f / 800.0f;

struct Sprite {
    int views = 1, framesPerView = 1;
    int anchorX = 0, anchorY = 0;   // pixel in the frame that sits on the object's ground point
    uint32_t w = 0, h = 0, frameMs = 0;
    uint32_t viewMask = 0;
    bool shadow = false;
    std::vector<Rgba> frames;       // view-major, RGBA, transparent where the key colour was
    const Rgba& frame(int view, int f) const {
        view %= views; f %= framesPerView;
        return frames[(size_t)view * framesPerView + f];
    }
};

// Loads a sprite .flc. Palette index 255 is the transparent key colour (magenta or cyan depending on the file). For shadow sprites the palette is
// a ramp of 4 greens plus white that becomes translucent black of increasing density.
// `palettePcx`, when not empty, replaces the FLC palette with the one of an 8-bit PCX (colour variants).
bool loadSprite(const std::string& flcPath, Sprite& out, std::string& err, bool shadow = false,
                const std::string& palettePcx = std::string(), const uint8_t* rawPalette = nullptr);   // rawPalette: 768 RGB bytes, wins over palettePcx

// Palette of an 8-bit PCX (the 768 bytes after the 0x0C marker at the end of the file).
bool readPcxPalette(const Bytes& pcx, uint8_t pal[768]);

}  // namespace sg
