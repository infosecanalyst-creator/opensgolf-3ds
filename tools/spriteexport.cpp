// Exports every sprite .flc under <game>/Flics to PNG sheets: one PNG per sprite, a row per view, a column per animation frame (at most 12 columns, evenly sampled).
// Shadow sprites are written beside their sprite as <name>_shadow.png. Usage: sgsprites <game dir> <output dir>
#include "sg/sprites.h"
#include <algorithm>
#include <cstdio>
#include <filesystem>
namespace fs = std::filesystem;
using namespace sg;
int main(int argc, char** argv) {
    if (argc < 3) { std::fprintf(stderr, "usage: sgsprites <game dir> <output dir>\n"); return 1; }
    const fs::path root = fs::path(argv[1]) / "Flics", out = argv[2];
    int ok = 0, bad = 0;
    for (auto& ent : fs::recursive_directory_iterator(root)) {
        if (!ent.is_regular_file()) continue;
        std::string ext = ent.path().extension().string(); for (char& c : ext) c = (char)std::tolower((unsigned char)c);
        if (ext != ".flc") continue;
        std::string stem = ent.path().stem().string();
        const bool shadow = stem.size() > 6 && stem.compare(stem.size() - 6, 6, "Shadow") == 0;
        Sprite sp; std::string err;
        if (!loadSprite(ent.path().string(), sp, err, shadow)) { std::fprintf(stderr, "skip %s: %s\n", ent.path().c_str(), err.c_str()); bad++; continue; }
        const int cols = std::min(sp.framesPerView, 12), rows = sp.views;
        Rgba sheet; sheet.w = sp.w * (uint32_t)cols; sheet.h = sp.h * (uint32_t)rows; sheet.px.assign((size_t)sheet.w * sheet.h * 4, 0);
        for (int v = 0; v < rows; v++) for (int c = 0; c < cols; c++) {
            const int f = sp.framesPerView <= 12 ? c : c * sp.framesPerView / cols;
            const Rgba& fr = sp.frame(v, f);
            for (uint32_t y = 0; y < sp.h; y++) for (uint32_t x = 0; x < sp.w; x++)
                std::copy_n(&fr.px[((size_t)y * sp.w + x) * 4], 4, &sheet.px[((size_t)(v * sp.h + y) * sheet.w + (size_t)c * sp.w + x) * 4]);
        }
        fs::path rel = fs::relative(ent.path().parent_path(), root);
        fs::create_directories(out / rel);
        if (shadow) stem = stem.substr(0, stem.size() - 6) + "_shadow";
        if (writePng((out / rel / (stem + ".png")).string(), sheet)) ok++; else bad++;
    }
    std::printf("exported %d sprites, %d failed\n", ok, bad);
    return bad ? 2 : 0;
}
