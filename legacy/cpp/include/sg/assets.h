// SimGolf native port: loaders for the original game's asset formats.
// Portable C++17, no dependencies.
#pragma once
#include <cstdint>
#include <string>
#include <vector>

namespace sg {

using Bytes = std::vector<uint8_t>;

bool readFile(const std::string& path, Bytes& out);

// RGBA8, row-major, top-left origin.
struct Rgba {
    uint32_t w = 0, h = 0;
    Bytes px;
};

// 8-bit palettised image (used by FLC frames and 8-bit PCX/BMP).
struct Indexed {
    uint32_t w = 0, h = 0;
    Bytes idx;
    uint8_t pal[256 * 3] = {};
};

Rgba toRgba(const Indexed& in, int transparentIndex = -1);

// Each decoder returns false and fills err on failure.
bool decodePcx(const Bytes& data, Rgba& out, std::string& err);
bool decodeTga(const Bytes& data, Rgba& out, std::string& err);
bool decodeBmp(const Bytes& data, Rgba& out, std::string& err);

struct Flc {
    uint32_t w = 0, h = 0;
    uint32_t frameMs = 0;           // per-frame delay in milliseconds
    std::vector<Indexed> frames;    // fully composited frame each
    // Firaxis extension of the 128 byte header (all sprite FLCs have it, see docs/SPRITES.md):
    // the frames are `views` runs of `framesPerView` frames, view-major. Each run is stored with one
    // extra trailing "ring" frame that decodeFlc drops. The crop box origin is where the frame sits in
    // a 480x480 render canvas whose centre (240,240) is the object's ground point.
    uint32_t views = 1, framesPerView = 0;
    uint32_t cropX = 0, cropY = 0, canvasW = 0, canvasH = 0;
    uint32_t viewMask = 0;          // bit set per stored view (0x0F = 4 views, 0xFF = 8 views)
    bool hasExt = false;
};
bool decodeFlc(const Bytes& data, Flc& out, std::string& err);

struct Wav {
    uint16_t channels = 0, bits = 0;
    uint32_t sampleRate = 0;
    Bytes pcm;
};
bool decodeWav(const Bytes& data, Wav& out, std::string& err);

bool writePng(const std::string& path, const Rgba& img);

}  // namespace sg
