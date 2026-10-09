#include "sg/terrain.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <cctype>
#include <filesystem>

namespace sg {

static const char* kNames[kTypeCount] = {
    "Tee", "PuttingGreen", "Fairway", "FirmFairway", "Rough", "DeepRough", nullptr, nullptr,
    "GrassySand", "PotSandBunker", "Overgrowth", "Brush", "Rock", "Woods", nullptr, nullptr, nullptr,
    "WaterShallow", "Marsh", "Overgrowth", nullptr, nullptr, "Building", "WaterMiddle", "WaterDeep",
    "WaterShallowDesert", "TrickyGreen", "SandBunker1", "SandBunker2", "SandBunker3", "SandBunker4",
    "Cliff", "Ravine", "FlowerBed", "ZenSand", "GrassBunker"};

const char* tileTypeName(int type) { return type >= 0 && type < kTypeCount ? kNames[type] : nullptr; }

static std::string lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(), [](unsigned char c) { return std::tolower(c); });
    return s;
}

bool TextureCatalog::load(const std::string& themeDir, std::string& err) {
    namespace fs = std::filesystem;
    std::error_code ec;
    std::map<std::string, std::string> index;  // lower-case name -> real path
    for (auto& e : fs::directory_iterator(themeDir, ec))
        if (e.is_regular_file()) index[lower(e.path().filename().string())] = e.path().string();
    if (ec || index.empty()) { err = "cannot read texture folder " + themeDir; return false; }
    dir = themeDir;
    files.assign(kTypeCount, {});
    int found = 0;
    for (int t = 0; t < kTypeCount; t++) {
        if (!kNames[t]) continue;
        for (char letter = 'A'; letter <= 'E'; letter++) {
            std::vector<std::string> vars(9);  // index = variation 0..8; empty when the file does not exist
            int have = 0;
            for (int v = 1; v <= 9; v++) {
                char suffix[16];
                std::snprintf(suffix, sizeof suffix, "%c%04d.bmp", letter, v);
                auto it = index.find(lower(std::string(kNames[t]) + suffix));
                if (it != index.end()) { vars[(size_t)v - 1] = it->second; have++; }
            }
            if (have) { found += have; files[t].push_back(std::move(vars)); }
        }
    }
    if (!found) { err = "no terrain textures found in " + themeDir; return false; }
    return true;
}

const std::string* TextureCatalog::pick(int type, int set, int variation) const {
    // Missing types fall back to something visually close.
    static const int fallbacks[][2] = {{TT_Overgrowth2, TT_Overgrowth}, {TT_WaterShallowDesert, TT_WaterShallow},
                                       {TT_Tee, TT_Fairway}, {TT_TrickyGreen, TT_PuttingGreen},
                                       {TT_FirmFairway, TT_Fairway}, {TT_GrassySand, TT_Rough},
                                       {TT_Marsh, TT_Rough}, {TT_Building, TT_Rock}, {TT_Overgrowth, TT_Brush},
                                       {TT_Brush, TT_Rough}, {TT_Woods, TT_Brush}, {TT_PotSandBunker, TT_GrassySand},
                                       {TT_SandBunker1, TT_PotSandBunker}, {TT_Rough, TT_Fairway}};
    for (int guard = 0; guard < 6; guard++) {
        if (type >= 0 && type < (int)files.size() && !files[type].empty()) {
            const auto& sets = files[type];
            const auto& vars = sets[(size_t)set % sets.size()];
            const std::string* want = &vars[(size_t)variation % vars.size()];
            if (!want->empty()) return want;
            for (const auto& s : vars) if (!s.empty()) return &s;  // variation missing: use the first that exists
        }
        int next = -1;
        for (auto& f : fallbacks) if (f[0] == type) next = f[1];
        if (next < 0) break;
        type = next;
    }
    return nullptr;
}

bool loadLighting(const std::string& path, Lighting& out) {
    Bytes d;
    if (!readFile(path, d)) return false;
    std::string text(d.begin(), d.end());
    auto grab = [&](const char* tag, float dst[3]) {
        size_t p = text.find(tag);
        if (p == std::string::npos) return;
        int r, g, b;
        if (std::sscanf(text.c_str() + p + std::char_traits<char>::length(tag), " %d %d %d", &r, &g, &b) == 3) {
            dst[0] = r / 255.f; dst[1] = g / 255.f; dst[2] = b / 255.f;
        }
    };
    grab("#AMBIENT", out.ambient);
    grab("#DIFFUSE", out.diffuse);
    grab("#SPECULAR", out.specular);
    return true;
}

float Terrain::heightAt(float wx, float wz) const {
    float fx = (wx + w * kTileSize * 0.5f) / kTileSize, fz = (wz + h * kTileSize * 0.5f) / kTileSize;
    fx = std::clamp(fx, 0.0f, (float)w - 0.001f); fz = std::clamp(fz, 0.0f, (float)h - 0.001f);
    int x = (int)fx, y = (int)fz;
    float u = fx - x, v = fz - y;
    float a = cornerAt(x, y) * (1 - u) + cornerAt(x + 1, y) * u, b = cornerAt(x, y + 1) * (1 - u) + cornerAt(x + 1, y + 1) * u;
    return (a * (1 - v) + b * v) * kHeightStep;
}

int Terrain::typeAtWorld(float wx, float wz) const {
    int x = (int)std::floor((wx + w * kTileSize * 0.5f) / kTileSize), y = (int)std::floor((wz + h * kTileSize * 0.5f) / kTileSize);
    return (x < 0 || y < 0 || x >= w || y >= h) ? -1 : (int)type[(size_t)tileIndex(x, y)];
}

void Terrain::paint(int x, int y, int ty, int vbyte) {
    if (x < 0 || y < 0 || x >= w || y >= h) return;
    size_t i = (size_t)tileIndex(x, y);
    type[i] = (uint8_t)ty;
    variation[i] = (uint8_t)vbyte;
    set[i] = (uint8_t)(((x * 73856093u) ^ (y * 19349663u) ^ (uint32_t)ty * 83492791u) >> 7) % 5;  // stable random look
}

void Terrain::setWall(int x, int y, int dir, bool on) {
    if (x < 0 || y < 0 || x >= w || y >= h) return;
    if (wallMask.size() != type.size()) wallMask.assign(type.size(), 0);
    static const int dx[4] = {0, 1, 0, -1}, dy[4] = {-1, 0, 1, 0};
    auto set = [&](int tx, int ty, int d) {
        if (tx < 0 || ty < 0 || tx >= w || ty >= h) return;
        uint8_t& m = wallMask[(size_t)tileIndex(tx, ty)];
        m = (uint8_t)(on ? (m | (1 << d)) : (m & ~(1 << d)));
    };
    set(x, y, dir);
    set(x + dx[dir], y + dy[dir], (dir + 2) & 3);
}

void Terrain::relax() {
    auto at = [&](int x, int y) -> int8_t& { return corner[(size_t)y * (w + 1) + x]; };
    // No corner may differ from a neighbour by more than 1 level (4-neighbourhood). Higher corners pull lower ones up.
    bool changed = true;
    for (int guard = 0; changed && guard < 64; guard++) {
        changed = false;
        for (int y = 0; y <= h; y++)
            for (int x = 0; x <= w; x++) {
                const int dx[4] = {1, -1, 0, 0}, dy[4] = {0, 0, 1, -1};
                for (int k = 0; k < 4; k++) {
                    int nx = x + dx[k], ny = y + dy[k];
                    if (nx < 0 || ny < 0 || nx > w || ny > h) continue;
                    int a = at(x, y), b = at(nx, ny);
                    if (a - b > 1) { at(nx, ny) = (int8_t)(a - 1); changed = true; }
                }
            }
    }
}

void Terrain::raiseCorner(int cx, int cy, int delta) {
    if (cx < 0 || cy < 0 || cx > w || cy > h) return;
    int8_t& c = corner[(size_t)cy * (w + 1) + cx];
    c = (int8_t)std::clamp(c + delta, 0, kMaxLevel);
    relax();
}

void Terrain::flattenTile(int x, int y) {
    if (x < 0 || y < 0 || x >= w || y >= h) return;
    int sum = 0;
    for (int j = 0; j < 2; j++) for (int i = 0; i < 2; i++) sum += cornerAt(x + i, y + j);
    int8_t level = (int8_t)((sum + 2) / 4);
    for (int j = 0; j < 2; j++) for (int i = 0; i < 2; i++) corner[(size_t)(y + j) * (w + 1) + x + i] = level;
    relax();
}

bool Terrain::save(const std::string& file, std::string& err) const {
    FILE* f = std::fopen(file.c_str(), "w");
    if (!f) { err = "cannot write " + file; return false; }
    std::fprintf(f, "SGCOURSE 1\n%d %d\n", w, h);
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) { size_t i = (size_t)tileIndex(x, y); std::fprintf(f, "%d,%d,%d ", type[i], variation[i], set[i]); }
        std::fprintf(f, "\n");
    }
    for (int y = 0; y <= h; y++) {
        for (int x = 0; x <= w; x++) std::fprintf(f, "%d ", cornerAt(x, y));
        std::fprintf(f, "\n");
    }
    bool anyPath = false;
    for (uint8_t k : pathKind) anyPath |= k != 0;
    if (anyPath) {
        std::fprintf(f, "PATHS\n");
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) std::fprintf(f, "%d", pathAt(x, y));
            std::fprintf(f, "\n");
        }
    }
    bool anyWall = false;
    for (uint8_t m : wallMask) anyWall |= m != 0;
    if (anyWall) {
        std::fprintf(f, "WALLS\n");
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) std::fprintf(f, "%X", wallMask[(size_t)tileIndex(x, y)] & 15);
            std::fprintf(f, "\n");
        }
    }
    std::fclose(f);
    return true;
}

bool Terrain::load(const std::string& file, Terrain& out, std::string& err) {
    FILE* f = std::fopen(file.c_str(), "r");
    if (!f) { err = "cannot read " + file; return false; }
    Terrain t;
    int ver = 0;
    bool ok = std::fscanf(f, "SGCOURSE %d", &ver) == 1 && ver == 1 && std::fscanf(f, "%d %d", &t.w, &t.h) == 2 && t.w > 0 && t.h > 0 && t.w <= 512 && t.h <= 512;
    if (ok) {
        t.type.resize((size_t)t.w * t.h); t.variation.resize(t.type.size()); t.set.resize(t.type.size());
        t.corner.resize((size_t)(t.w + 1) * (t.h + 1));
        for (size_t i = 0; ok && i < t.type.size(); i++) {
            int a, b, c;
            ok = std::fscanf(f, "%d,%d,%d", &a, &b, &c) == 3;
            t.type[i] = (uint8_t)a; t.variation[i] = (uint8_t)b; t.set[i] = (uint8_t)c;
        }
        for (size_t i = 0; ok && i < t.corner.size(); i++) { int v; ok = std::fscanf(f, "%d", &v) == 1; t.corner[i] = (int8_t)v; }
        t.pathKind.assign(t.type.size(), 0);
        t.wallMask.assign(t.type.size(), 0);
        char word[16] = {};
        while (ok && std::fscanf(f, " %15s", word) == 1) {  // optional sections: PATHS, WALLS (one row of digits per tile row)
            const bool paths = std::string(word) == "PATHS", walls = std::string(word) == "WALLS";
            if (!paths && !walls) break;
            for (int y = 0; ok && y < t.h; y++) {
                char row[600] = {};
                ok = std::fscanf(f, " %599s", row) == 1 && (int)std::strlen(row) >= t.w;
                for (int x = 0; ok && x < t.w; x++) {
                    int v = std::isxdigit((unsigned char)row[x]) ? std::stoi(std::string(1, row[x]), nullptr, 16) : 0;
                    (paths ? t.pathKind : t.wallMask)[(size_t)y * t.w + x] = (uint8_t)(paths ? std::min(v, 2) : v);
                }
            }
        }
    }
    std::fclose(f);
    if (!ok) { err = "bad course file " + file; return false; }
    // The golfers' route and the clubhouse are scenery of the demo, not part of the course file.
    t.path = out.path;
    t.clubhouseX = out.clubhouseX; t.clubhouseY = out.clubhouseY;
    t.desert = out.desert;
    out = std::move(t);
    return true;
}

namespace {
struct Rng {
    uint32_t s;
    uint32_t next() { s ^= s << 13; s ^= s >> 17; s ^= s << 5; return s; }
    float unit() { return (next() & 0xFFFFFF) / float(0x1000000); }
    int range(int n) { return (int)(next() % (uint32_t)n); }
};
}  // namespace

Terrain Terrain::demoCourse(int w, int h, uint32_t seed) {
    Terrain t;
    t.w = w; t.h = h;
    t.type.assign((size_t)w * h, TT_Rough);
    t.variation.resize((size_t)w * h);
    t.set.resize((size_t)w * h);
    t.corner.assign((size_t)(w + 1) * (h + 1), 0);
    t.pathKind.assign((size_t)w * h, 0);
    Rng rng{seed ? seed : 1u};
    auto at = [&](int x, int y) -> uint8_t& { return t.type[(size_t)y * w + x]; };
    auto inside = [&](int x, int y) { return x >= 0 && y >= 0 && x < w && y < h; };
    auto disc = [&](float cx, float cy, float rx, float ry, uint8_t ty, bool onlyRough) {
        for (int y = 0; y < h; y++)
            for (int x = 0; x < w; x++) {
                float dx = (x + 0.5f - cx) / rx, dy = (y + 0.5f - cy) / ry;
                if (dx * dx + dy * dy <= 1.f && (!onlyRough || at(x, y) == TT_Rough || at(x, y) == TT_DeepRough)) at(x, y) = ty;
            }
    };
    // Outer deep rough, then scattered woods and brush.
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) {
            int edge = std::min({x, y, w - 1 - x, h - 1 - y});
            if (edge < 2) at(x, y) = TT_DeepRough;
        }
    for (int i = 0; i < w * h / 14; i++) {
        float cx = (float)rng.range(w), cy = (float)rng.range(h), r = 1.0f + rng.unit() * 2.2f;
        uint8_t ty = rng.range(3) == 0 ? TT_Brush : (rng.range(3) == 0 ? TT_Rock : TT_Woods);
        disc(cx, cy, r, r, ty, true);
    }
    // Fairway along a curve from tee (bottom-left) to green (top-right).
    float sx = 5, sy = h - 6, ex = w - 7, ey = 6;
    for (int step = 0; step <= 200; step++) {
        float s = step / 200.f;
        float cx = sx + (ex - sx) * s + std::sin(s * 3.14159f * 1.4f) * h * 0.14f;
        float cy = sy + (ey - sy) * s;
        disc(cx, cy, 3.4f, 3.4f, TT_Rough, false);
    }
    for (int step = 0; step <= 200; step++) {
        float s = step / 200.f;
        float cx = sx + (ex - sx) * s + std::sin(s * 3.14159f * 1.4f) * h * 0.14f;
        float cy = sy + (ey - sy) * s;
        disc(cx, cy, 2.3f, 2.3f, TT_Fairway, false);
        if (step % 5 == 0) { t.path.push_back(cx * kTileSize - w * kTileSize * 0.5f); t.path.push_back(cy * kTileSize - h * kTileSize * 0.5f); }
        if (step % 4 == 0) disc(cx, cy, 1.0f, 1.0f, TT_FirmFairway, false);
    }
    float gcx = ex + std::sin(3.14159f * 1.4f) * h * 0.14f, gcy = ey;
    disc(gcx, gcy, 4.6f, 4.6f, TT_FirmFairway, false);
    disc(gcx, gcy, 3.2f, 3.2f, TT_PuttingGreen, false);
    disc(sx, sy, 2.2f, 1.6f, TT_Tee, false);
    // Clubhouse lot beside the tee: a 5x5 block of Building tiles.
    t.clubhouseX = (int)sx + 6; t.clubhouseY = (int)sy - 3;
    for (int y = t.clubhouseY - 2; y <= t.clubhouseY + 2; y++)
        for (int x = t.clubhouseX - 2; x <= t.clubhouseX + 2; x++)
            if (inside(x, y)) at(x, y) = TT_Building;
    // Bunkers beside the green and mid fairway, a pond to one side.
    disc(gcx - 4.5f, gcy + 2.5f, 1.9f, 1.3f, TT_PotSandBunker, false);
    disc(gcx + 3.2f, gcy + 3.6f, 1.6f, 1.6f, TT_PotSandBunker, false);
    disc(w * 0.5f + 3, h * 0.5f - 1, 1.7f, 1.2f, TT_PotSandBunker, false);
    disc(w * 0.5f - 2, h * 0.5f + 6, 2.3f, 1.5f, 7, true);  // blended sand bunker (type 7), dished
    float pcx = w * 0.30f, pcy = h * 0.38f;
    disc(pcx, pcy, 5.5f, 4.2f, TT_WaterShallow, false);
    disc(pcx, pcy, 4.2f, 3.0f, TT_WaterMiddle, false);
    disc(pcx, pcy, 2.4f, 1.6f, TT_WaterDeep, false);
    for (size_t i = 0; i < t.type.size(); i++) {
        // Sets A..E are alternative looks of the same terrain, chosen at random per tile (as the original does).
        t.set[i] = (uint8_t)rng.range(5);
        t.variation[i] = 0;
        if (t.type[i] == TT_Tee) t.variation[i] = (uint8_t)rng.range(5);  // tees use this byte as their set
        // Water is one type; its depth is the tile's variation byte (1 = middle, 2 = deep).
        if (t.type[i] == TT_WaterMiddle) { t.type[i] = TT_WaterShallow; t.variation[i] = 1; }
        else if (t.type[i] == TT_WaterDeep) { t.type[i] = TT_WaterShallow; t.variation[i] = 2; }
    }
    // Gentle hills, flattened under water and greens.
    float p1 = rng.unit() * 6.28f, p2 = rng.unit() * 6.28f;
    for (int y = 0; y <= h; y++)
        for (int x = 0; x <= w; x++) {
            float v = 3.2f * std::sin(x * 0.17f + p1) + 2.6f * std::cos(y * 0.13f + p2) + 2.0f * std::sin((x + y) * 0.09f);
            t.corner[(size_t)y * (w + 1) + x] = (int8_t)std::clamp((int)std::lround(v + 5.0f), 0, 12);
        }
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) {
            uint8_t ty = at(x, y);
            bool flat = ty == TT_WaterShallow || ty == TT_WaterMiddle || ty == TT_WaterDeep || ty == TT_PuttingGreen || ty == TT_Tee || ty == TT_Building;
            if (!flat) continue;
            int level = (ty == TT_PuttingGreen || ty == TT_Tee) ? 6 : (ty == TT_Building ? 5 : 1);
            for (int dy = 0; dy <= 1; dy++)
                for (int dx = 0; dx <= 1; dx++) t.corner[(size_t)(y + dy) * (w + 1) + (x + dx)] = (int8_t)level;
        }
    (void)inside;
    return t;
}

Terrain Terrain::emptyPlot(int w, int h, uint32_t seed) {
    Terrain t;
    t.w = w; t.h = h;
    t.type.assign((size_t)w * h, TT_Rough);
    t.variation.assign((size_t)w * h, 0);
    t.set.resize((size_t)w * h);
    t.corner.assign((size_t)(w + 1) * (h + 1), 0);
    t.pathKind.assign((size_t)w * h, 0);
    t.wallMask.assign((size_t)w * h, 0);
    Rng rng{seed ? seed : 1u};
    auto at = [&](int x, int y) -> uint8_t& { return t.type[(size_t)y * w + x]; };
    auto disc = [&](float cx, float cy, float rx, float ry, uint8_t ty, bool onlyRough) {
        for (int y = 0; y < h; y++)
            for (int x = 0; x < w; x++) {
                float dx = (x + 0.5f - cx) / rx, dy = (y + 0.5f - cy) / ry;
                if (dx * dx + dy * dy <= 1.f && (!onlyRough || at(x, y) == TT_Rough || at(x, y) == TT_DeepRough)) at(x, y) = ty;
            }
    };
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++)
            if (std::min({x, y, w - 1 - x, h - 1 - y}) < 2) at(x, y) = TT_DeepRough;
    for (int i = 0; i < w * h / 16; i++) {
        float cx = (float)rng.range(w), cy = (float)rng.range(h), r = 1.0f + rng.unit() * 2.4f;
        uint8_t ty = rng.range(3) == 0 ? TT_Brush : (rng.range(4) == 0 ? TT_Rock : TT_Woods);
        disc(cx, cy, r, r, ty, true);
    }
    // Clubhouse lot near the bottom left; keep the ground around it clear so the first tee can go beside it.
    t.clubhouseX = 8; t.clubhouseY = h - 8;
    disc((float)t.clubhouseX + 0.5f, (float)t.clubhouseY + 0.5f, 7.0f, 7.0f, TT_Rough, false);
    for (int y = t.clubhouseY - 2; y <= t.clubhouseY + 2; y++)
        for (int x = t.clubhouseX - 2; x <= t.clubhouseX + 2; x++)
            if (x >= 0 && y >= 0 && x < w && y < h) at(x, y) = TT_Building;
    // A pond away from the clubhouse.
    float pcx = w * (0.55f + 0.2f * rng.unit()), pcy = h * (0.25f + 0.2f * rng.unit());
    disc(pcx, pcy, 5.0f, 3.8f, TT_WaterShallow, false);
    disc(pcx, pcy, 3.6f, 2.6f, TT_WaterMiddle, false);
    disc(pcx, pcy, 2.0f, 1.4f, TT_WaterDeep, false);
    for (size_t i = 0; i < t.type.size(); i++) {
        t.set[i] = (uint8_t)rng.range(5);
        if (t.type[i] == TT_WaterMiddle) { t.type[i] = TT_WaterShallow; t.variation[i] = 1; }
        else if (t.type[i] == TT_WaterDeep) { t.type[i] = TT_WaterShallow; t.variation[i] = 2; }
    }
    float p1 = rng.unit() * 6.28f, p2 = rng.unit() * 6.28f;
    for (int y = 0; y <= h; y++)
        for (int x = 0; x <= w; x++) {
            float v = 2.6f * std::sin(x * 0.17f + p1) + 2.2f * std::cos(y * 0.13f + p2) + 1.6f * std::sin((x + y) * 0.09f);
            t.corner[(size_t)y * (w + 1) + x] = (int8_t)std::clamp((int)std::lround(v + 5.0f), 0, 12);
        }
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) {
            uint8_t ty = at(x, y);
            bool flat = ty == TT_WaterShallow || ty == TT_Building;
            if (!flat) continue;
            int level = ty == TT_Building ? 5 : 1;
            for (int dy = 0; dy <= 1; dy++)
                for (int dx = 0; dx <= 1; dx++) t.corner[(size_t)(y + dy) * (w + 1) + (x + dx)] = (int8_t)level;
        }
    t.relax();
    return t;
}

Terrain Terrain::generate(int w, int h, uint32_t seed, int theme, int lie, int hilliness, int slot, bool sandbox) {
    Terrain t;
    t.w = w; t.h = h;
    t.type.assign((size_t)w * h, TT_Rough);
    t.variation.assign((size_t)w * h, 0);
    t.set.resize((size_t)w * h);
    t.corner.assign((size_t)(w + 1) * (h + 1), 0);
    t.pathKind.assign((size_t)w * h, 0);
    t.wallMask.assign((size_t)w * h, 0);
    Rng rng{seed ? seed : 1u};
    for (int i = 0; i < 4; i++) rng.next();
    auto at = [&](int x, int y) -> uint8_t& { return t.type[(size_t)y * w + x]; };
    auto isWater = [&](int x, int y) { const uint8_t ty = at(x, y); return ty == TT_WaterShallow || ty == TT_WaterMiddle || ty == TT_WaterDeep; };
    auto disc = [&](float cx, float cy, float rx, float ry, uint8_t ty, bool onlyRough) {
        for (int y = 0; y < h; y++)
            for (int x = 0; x < w; x++) {
                float dx = (x + 0.5f - cx) / rx, dy = (y + 0.5f - cy) / ry;
                if (dx * dx + dy * dy <= 1.f && (!onlyRough || at(x, y) == TT_Rough || at(x, y) == TT_DeepRough)) at(x, y) = ty;
            }
    };
    slot = std::clamp(slot, 0, 15);
    // Ground cover scatter: density grows with the price slot; the mix depends on the theme (PLACEHOLDER mixes, the exe's tile lists per theme are not decoded).
    const int clusters = (int)(w * h / 16 * (0.65f + 0.9f * slot / 15.0f));
    for (int i = 0; i < clusters; i++) {
        const float cx = (float)rng.range(w), cy = (float)rng.range(h), r = 1.0f + rng.unit() * 2.4f;
        const int d = rng.range(100);
        uint8_t ty;
        switch (theme) {
            case 1:  ty = d < 38 ? TT_Brush : d < 72 ? TT_Rock : d < 92 ? TT_GrassySand : TT_Woods; break;
            case 2:  ty = d < 62 ? TT_Woods : d < 88 ? TT_Brush : d < 96 ? TT_Marsh : TT_Rock; break;
            case 3:  ty = d < 48 ? TT_Brush : d < 78 ? TT_GrassySand : d < 90 ? TT_Rock : TT_Woods; break;
            default: ty = d < 60 ? TT_Woods : d < 88 ? TT_Brush : TT_Rock; break;
        }
        disc(cx, cy, r, r, ty, true);
    }
    // Water. Coastal land has one open shore, an island is ringed, inland land has ponds.
    auto shore = [&](int side, float width) {   // side 0 north (y small), 1 east, 2 south, 3 west
        const float p1 = rng.unit() * 6.28f, p2 = rng.unit() * 6.28f;
        for (int y = 0; y < h; y++)
            for (int x = 0; x < w; x++) {
                const float along = (side == 0 || side == 2) ? (float)x : (float)y;
                const float depth = side == 0 ? (float)y : side == 2 ? (float)(h - 1 - y) : side == 3 ? (float)x : (float)(w - 1 - x);
                const float edge = width + 2.2f * std::sin(along * 0.23f + p1) + 1.3f * std::sin(along * 0.51f + p2);
                if (depth < edge) {
                    at(x, y) = TT_WaterShallow;
                    t.variation[(size_t)y * w + x] = depth < edge - 3.5f ? 2 : depth < edge - 1.5f ? 1 : 0;
                } else if (depth < edge + 1.4f && at(x, y) != TT_WaterShallow) at(x, y) = TT_Brush;   // shoreline strip
            }
    };
    if (lie == 1) shore(rng.range(4), 6.0f + rng.unit() * 2.0f);
    else if (lie == 2) for (int s = 0; s < 4; s++) shore(s, 4.5f + rng.unit() * 2.5f);
    if (theme == 0 || lie == 0 || theme == 2) {   // inland ponds (every theme gets about one, tropical and links lie more)
        const int ponds = lie == 0 ? 1 + (theme == 2 ? 2 : theme == 1 ? 0 : 1) + rng.range(2) : (rng.range(2) ? 1 : 0);
        for (int i = 0; i < ponds; i++) {
            const float pcx = 10.0f + rng.unit() * (w - 20), pcy = 10.0f + rng.unit() * (h - 20), r = 2.2f + rng.unit() * 2.8f;
            disc(pcx, pcy, r * 1.3f, r, TT_WaterShallow, false);
            for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) {
                float dx = (x + 0.5f - pcx) / (r * 0.85f), dy = (y + 0.5f - pcy) / (r * 0.65f);
                if (dx * dx + dy * dy <= 1.f && at(x, y) == TT_WaterShallow) t.variation[(size_t)y * w + x] = (dx * dx + dy * dy) < 0.35f ? 2 : 1;
            }
            if (theme == 2 || theme == 3) disc(pcx, pcy + r * 0.9f, r * 1.0f, r * 0.45f, TT_Marsh, true);
        }
    }
    // The outermost ring is unplayable deep rough where it is not water.
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++)
            if (std::min({x, y, w - 1 - x, h - 1 - y}) < 1 && !isWater(x, y)) at(x, y) = TT_DeepRough;
    // Clubhouse: a random cell of a 17 x 17 window at the centre whose 13 x 13 surroundings hold no water; the lot is cleared to rough (exe: retries until the
    // cell has the base fill tile and the footprint fits).
    int cx = w / 2, cy = h / 2;
    for (int tries = 0; tries < 400; tries++) {
        const int x = w / 2 - 8 + rng.range(17), y = h / 2 - 8 + rng.range(17);
        bool clear = true;
        for (int yy = y - 6; yy <= y + 6 && clear; yy++) for (int xx = x - 6; xx <= x + 6; xx++) if (xx < 0 || yy < 0 || xx >= w || yy >= h || isWater(xx, yy)) { clear = false; break; }
        if (clear) { cx = x; cy = y; break; }
    }
    disc((float)cx + 0.5f, (float)cy + 0.5f, 5.5f, 5.5f, TT_Rough, false);
    for (int y = cy - 2; y <= cy + 2; y++) for (int x = cx - 2; x <= cx + 2; x++) at(x, y) = TT_Building;
    t.clubhouseX = cx; t.clubhouseY = cy;
    for (size_t i = 0; i < t.type.size(); i++) {
        t.set[i] = (uint8_t)rng.range(5);
        if (t.type[i] == TT_WaterMiddle) { t.type[i] = TT_WaterShallow; t.variation[i] = 1; }
        else if (t.type[i] == TT_WaterDeep) { t.type[i] = TT_WaterShallow; t.variation[i] = 2; }
    }
    // Height. Exe: base height 0x30 flat, 0x20 rolling, 0x10 hilly, +50 percent for price slots 0..3, +25 percent for 4..7, -25 percent for 12..15, +0x10 in the sandbox;
    // a bigger base means flatter land here (PLACEHOLDER reading: the relief amplitude in levels is 96 / base).
    float base = hilliness == 0 ? 48.0f : hilliness == 1 ? 32.0f : 16.0f;
    base *= (slot / 4 == 0) ? 1.5f : (slot / 4 == 1) ? 1.25f : (slot / 4 == 3) ? 0.75f : 1.0f;
    if (sandbox) base += 16.0f;
    const float amp = 96.0f / base;
    const float f1 = 0.10f + rng.unit() * 0.08f, f2 = 0.07f + rng.unit() * 0.06f, p1 = rng.unit() * 6.28f, p2 = rng.unit() * 6.28f, p3 = rng.unit() * 6.28f;
    const float mid = 6.0f + amp * 0.6f;
    for (int y = 0; y <= h; y++)
        for (int x = 0; x <= w; x++) {
            float v = amp * (0.6f * std::sin(x * f1 + p1) * std::cos(y * f2 + p2) + 0.4f * std::sin((x + y) * 0.06f + p3)) + 0.25f * amp * std::sin(x * 0.31f + y * 0.27f);
            t.corner[(size_t)y * (w + 1) + x] = (int8_t)std::clamp((int)std::lround(mid + v), 0, kMaxLevel);
        }
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) {
            const uint8_t ty = at(x, y);
            if (ty != TT_WaterShallow && ty != TT_Building) continue;
            const int level = ty == TT_Building ? (int)std::lround(mid) : 1;
            for (int dy = 0; dy <= 1; dy++)
                for (int dx = 0; dx <= 1; dx++) t.corner[(size_t)(y + dy) * (w + 1) + (x + dx)] = (int8_t)level;
        }
    t.relax();
    return t;
}

int typeClass(int type) {
    switch (type) {
        case TT_Tee: return 0;
        case TT_PuttingGreen: case TT_TrickyGreen: return 1;
        case TT_Fairway: case TT_FirmFairway: return 2;
        case TT_Rough: case 6: return 3;
        case TT_DeepRough: case TT_Overgrowth: case TT_Overgrowth2: case TT_Brush: return 4;
        case 7: case TT_GrassySand: case TT_PotSandBunker: case TT_SandBunker1: case TT_SandBunker1 + 1:
        case TT_SandBunker1 + 2: case TT_SandBunker1 + 3: return 5;
        case TT_WaterShallow: case TT_WaterMiddle: case TT_WaterDeep: case TT_WaterShallowDesert: case TT_Marsh: return 6;
        case TT_Rock: return 7;
        case 13: case 14: case 15: case 16: return 8;
        case TT_Building: return 9;
        case TT_Cliff: return 10;
        case TT_Ravine: return 11;
        case TT_FlowerBed: return 12;
        case TT_ZenSand: case TT_GrassBunker: return 5;
        default: return 100 + type;
    }
}

int tileRelation(const Terrain& t, int x, int y, int nx, int ny) {
    if (nx < 0 || ny < 0 || nx >= t.w || ny >= t.h) return 0;
    size_t me = (size_t)t.tileIndex(x, y), nb = (size_t)t.tileIndex(nx, ny);
    int mt = t.type[me], nt = t.type[nb];
    if (mt == TT_WaterShallow) {
        int level = t.variation[me], nl = nt == TT_WaterShallow ? t.variation[nb] : 0;
        if (level == 0 && nt != TT_WaterShallow) return 1;
        if (nl == level) return 0;
        return (level == 0 || (level == 1 && nl == 2)) ? 2 : 1;
    }
    if (typeClass(mt) != typeClass(nt)) return 1;
    return mt != nt ? 2 : 0;
}

int blendVariation(int a, int b, int c) {
    static const uint8_t kRaw[71] = {0, 0, 0, 1, 2, 3, 4, 3, 9, 9, 0, 9, 0, 9, 2, 9, 4, 9, 9, 9, 0, 0, 9, 9, 2, 3, 9, 9, 9, 9, 5, 9, 9, 9, 2,
                                     9, 9, 9, 9, 9, 6, 6, 6, 1, 9, 9, 9, 9, 9, 9, 7, 9, 7, 9, 9, 9, 9, 9, 9, 9, 8, 6, 9, 9, 9, 9, 9, 9, 9, 9, 7};
    static const uint8_t kMap[10] = {0, 2, 4, 3, 1, 6, 8, 7, 5, 0};
    int code = (a == 1 ? 4 : a == 2 ? 40 : 0) + (b == 1 ? 1 : b == 2 ? 10 : 0) + (c == 1 ? 2 : c == 2 ? 20 : 0);
    return kMap[kRaw[code]];
}

namespace {
// Texture type the original picks for a tile type (Tile::m116d).
int textureTypeFor(int tileType, int variationByte, int param, bool desert) {
    switch (tileType) {
        case 1: return (variationByte & 0x80) ? TT_TrickyGreen : param;
        case 6: case 21: return TT_Rough;
        case 7: return TT_SandBunker1 + (variationByte & 3);
        case 13: case 14: case 15: case 16: return 13;
        case 17: return variationByte == 1 ? TT_WaterMiddle : variationByte == 2 ? TT_WaterDeep : (desert ? (int)TT_WaterShallowDesert : param);
        case 22: return (variationByte >= 0x40 && variationByte < 0x48) ? TT_Rough : param;
        default: return param;
    }
}
}  // namespace

void buildTileTriangles(const Terrain& t, int tx, int ty, std::vector<TileTri>& out) {
    const size_t me = (size_t)t.tileIndex(tx, ty);
    const int type = t.type[me], vbyte = t.variation[me], set = t.set[me];
    auto typeAt = [&](int x, int y) { return (x < 0 || y < 0 || x >= t.w || y >= t.h) ? -1 : (int)t.type[(size_t)t.tileIndex(x, y)]; };

    // Heights of the 3x3 patch: bilinear between the tile's corner elevations, sand tiles dished.
    float c[2][2];
    for (int j = 0; j < 2; j++)
        for (int i = 0; i < 2; i++) c[j][i] = t.cornerAt(tx + i, ty + j) * kHeightStep;
    float hgt[3][3];
    for (int j = 0; j < 3; j++)
        for (int i = 0; i < 3; i++) {
            float fx = i * 0.5f, fy = j * 0.5f;
            hgt[j][i] = (c[0][0] * (1 - fx) + c[0][1] * fx) * (1 - fy) + (c[1][0] * (1 - fx) + c[1][1] * fx) * fy;
        }
    if (type == 7) {
        const float dish = 13.0f;
        hgt[1][1] -= dish;
        auto sand = [&](int dx, int dy) { return typeAt(tx + dx, ty + dy) == 7; };
        if (sand(0, -1)) hgt[0][1] -= dish;
        if (sand(0, 1)) hgt[2][1] -= dish;
        if (sand(-1, 0)) hgt[1][0] -= dish;
        if (sand(1, 0)) hgt[1][2] -= dish;
        if (sand(0, -1) && sand(-1, 0) && sand(-1, -1)) hgt[0][0] -= dish;
        if (sand(0, -1) && sand(1, 0) && sand(1, -1)) hgt[0][2] -= dish;
        if (sand(0, 1) && sand(-1, 0) && sand(-1, 1)) hgt[2][0] -= dish;
        if (sand(0, 1) && sand(1, 0) && sand(1, 1)) hgt[2][2] -= dish;
    }
    const float ox = tx * kTileSize - t.w * kTileSize * 0.5f, oz = ty * kTileSize - t.h * kTileSize * 0.5f;
    auto V = [&](int j, int k) { return Vertex{ox + k * kTileSize * 0.5f, hgt[j][k], oz + j * kTileSize * 0.5f, k * 0.5f, j * 0.5f, 0, 1, 0}; };

    // Variation per triangle record (rec = 2*quadrant + {0,1}; quadrants NW, NE, SW, SE).
    int var[8];
    if (type == TT_Tee || type == TT_PotSandBunker) {
        for (int& v : var) v = type == TT_Tee ? 0 : 3;  // one flat texture; pot bunkers are always the rounded piece
    } else {
        const int N = tileRelation(t, tx, ty, tx, ty - 1), S = tileRelation(t, tx, ty, tx, ty + 1);
        const int W = tileRelation(t, tx, ty, tx - 1, ty), E = tileRelation(t, tx, ty, tx + 1, ty);
        const int NW = tileRelation(t, tx, ty, tx - 1, ty - 1), NE = tileRelation(t, tx, ty, tx + 1, ty - 1);
        const int SW = tileRelation(t, tx, ty, tx - 1, ty + 1), SE = tileRelation(t, tx, ty, tx + 1, ty + 1);
        // Original triangle index -> (a, b, c), and -> record.
        const int abc[8][3] = {{N, E, NE}, {E, N, NE}, {E, S, SE}, {S, E, SE}, {S, W, SW}, {W, S, SW}, {W, N, NW}, {N, W, NW}};
        const int recOf[8] = {2, 3, 7, 6, 5, 4, 0, 1};
        for (int i = 0; i < 8; i++) {
            int v = blendVariation(abc[i][0], abc[i][1], abc[i][2]);
            int texType = textureTypeFor(type, vbyte, type, t.desert);
            if ((texType == TT_Rough || type == 7 || texType == TT_Fairway) && v > 4) v = 0;
            var[recOf[i]] = v;
        }
    }
    const int texType = type == TT_Tee || type == TT_PotSandBunker ? type : textureTypeFor(type, vbyte, type, t.desert);
    const int useSet = type == TT_Tee ? vbyte : set;
    for (int q = 0; q < 4; q++) {
        int j = q / 2, k = q % 2;
        Vertex tri[2][3] = {{V(j, k), V(j + 1, k), V(j + 1, k + 1)}, {V(j, k), V(j + 1, k + 1), V(j, k + 1)}};
        for (int s = 0; s < 2; s++) {
            Vertex* T = tri[s];
            float ux = T[1].x - T[0].x, uy = T[1].y - T[0].y, uz = T[1].z - T[0].z;
            float vx = T[2].x - T[0].x, vy = T[2].y - T[0].y, vz = T[2].z - T[0].z;
            float nx = uy * vz - uz * vy, ny = uz * vx - ux * vz, nz = ux * vy - uy * vx;
            float len = std::sqrt(nx * nx + ny * ny + nz * nz);
            if (len > 0) { nx /= len; ny /= len; nz /= len; }
            if (ny < 0) { nx = -nx; ny = -ny; nz = -nz; }
            TileTri tt{texType, useSet, var[q * 2 + s], {T[0], T[1], T[2]}};
            for (auto& v : tt.v) { v.nx = nx; v.ny = ny; v.nz = nz; }
            out.push_back(tt);
        }
    }
}

}  // namespace sg
