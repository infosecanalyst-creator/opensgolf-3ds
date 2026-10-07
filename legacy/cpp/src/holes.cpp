#include "sg/holes.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>
#include <queue>
#include "sg/shot.h"

namespace sg {

std::vector<uint8_t> pathsConnectedToClubhouse(const Terrain& t) {
    std::vector<uint8_t> ok((size_t)t.w * t.h, 0);
    if (t.pathKind.empty()) return ok;
    std::queue<int> q;
    auto isBuilding = [&](int x, int y) { return x >= 0 && y >= 0 && x < t.w && y < t.h && t.type[(size_t)t.tileIndex(x, y)] == TT_Building; };
    static const int dx[4] = {0, 1, 0, -1}, dy[4] = {-1, 0, 1, 0};
    for (int y = 0; y < t.h; y++)
        for (int x = 0; x < t.w; x++) {
            if (!t.pathAt(x, y)) continue;
            for (int k = 0; k < 4; k++)
                if (isBuilding(x + dx[k], y + dy[k])) { ok[(size_t)t.tileIndex(x, y)] = 1; q.push(t.tileIndex(x, y)); break; }
        }
    while (!q.empty()) {
        int i = q.front(); q.pop();
        int x = i % t.w, y = i / t.w;
        for (int k = 0; k < 4; k++) {
            int nx = x + dx[k], ny = y + dy[k];
            if (!t.pathAt(nx, ny)) continue;
            size_t ni = (size_t)t.tileIndex(nx, ny);
            if (!ok[ni]) { ok[ni] = 1; q.push((int)ni); }
        }
    }
    return ok;
}

static bool isHazard(int type) {
    switch (type) {
        case TT_WaterShallow: case TT_WaterMiddle: case TT_WaterDeep: case TT_WaterShallowDesert: case TT_Marsh:
        case TT_PotSandBunker: case 7: case TT_SandBunker1: case TT_SandBunker1 + 1: case TT_SandBunker1 + 2: case TT_SandBunker1 + 3:
        case TT_Woods: case TT_Rock: case TT_Brush: case TT_Ravine: case TT_GrassBunker: return true;
        default: return false;
    }
}

HoleInfo analyzeHole(const Terrain& t) {
    HoleInfo h;
    if (t.path.size() < 4) return h;
    h.valid = true;
    h.teeX = t.path[0]; h.teeZ = t.path[1];
    h.holeX = t.path[t.path.size() - 2]; h.holeZ = t.path[t.path.size() - 1];
    const float dx = h.holeX - h.teeX, dz = h.holeZ - h.teeZ;
    h.length = std::hypot(dx, dz);
    h.par = h.length < 1300 ? 3 : (h.length < 2600 ? 4 : 5);   // PLACEHOLDER
    // Hazard stretches on the straight line.
    bool inHazard = false;
    const int steps = std::max(1, (int)(h.length / 25.0f));
    std::vector<uint8_t> onLine((size_t)t.w * t.h, 0);
    for (int i = 0; i <= steps; i++) {
        float x = h.teeX + dx * i / steps, z = h.teeZ + dz * i / steps;
        int ty = t.typeAtWorld(x, z);
        bool hz = ty >= 0 && isHazard(ty);
        if (hz && !inHazard) h.hazardsOnLine++;
        inHazard = hz;
        if (ty >= 0) {
            int tx = (int)std::floor((x + t.w * kTileSize * 0.5f) / kTileSize), tz = (int)std::floor((z + t.h * kTileSize * 0.5f) / kTileSize);
            onLine[(size_t)t.tileIndex(tx, tz)] = 1;
        }
    }
    // Hazard tiles within two tiles (200 units) of the line.
    for (int y = 0; y < t.h; y++)
        for (int x = 0; x < t.w; x++) {
            if (!isHazard(t.type[(size_t)t.tileIndex(x, y)])) continue;
            float cx = -t.w * kTileSize * 0.5f + (x + 0.5f) * kTileSize, cz = -t.h * kTileSize * 0.5f + (y + 0.5f) * kTileSize;
            // distance from the tile centre to the segment tee-hole
            float t0 = std::clamp(((cx - h.teeX) * dx + (cz - h.teeZ) * dz) / (h.length * h.length), 0.0f, 1.0f);
            float d = std::hypot(cx - (h.teeX + dx * t0), cz - (h.teeZ + dz * t0));
            if (d < 200.0f && !onLine[(size_t)t.tileIndex(x, y)]) h.hazardsNearLine++;
        }
    h.length_ = h.length > 1200.0f;
    h.accuracy = h.hazardsNearLine >= 8;
    h.imagination = h.hazardsOnLine >= 1;
    static const char* names[8] = {"Breather", "Freeway", "Precise", "Challenge", "Creative", "Heroic", "Strategic", "Classic"};
    // bit0 length, bit1 accuracy, bit2 imagination: 0 Breather, 1 Freeway, 2 Precise, 3 Challenge (L+A), 4 Creative, 5 Heroic (L+I), 6 Strategic (A+I), 7 Classic
    h.cls = names[(h.length_ ? 1 : 0) | (h.accuracy ? 2 : 0) | (h.imagination ? 4 : 0)];
    return h;
}

std::string HoleInfo::report() const {
    if (!valid) return "no hole";
    char b[200];
    std::snprintf(b, sizeof b, "hole: %.0f units, par %d, %s (length %s, accuracy %s, imagination %s; %d hazards on the line, %d near it)",
                  length, par, cls, length_ ? "yes" : "no", accuracy ? "yes" : "no", imagination ? "yes" : "no", hazardsOnLine, hazardsNearLine);
    return b;
}

}  // namespace sg

namespace sg {

std::vector<HoleRoute> findHoles(const Terrain& t) {
    std::vector<HoleRoute> out;
    const int w = t.w, h = t.h;
    if (w <= 0 || h <= 0) return out;
    struct Blob { float x = 0, z = 0; int n = 0, minY = 0; bool used = false; };
    auto centreOf = [&](float tx, float ty, float& x, float& z) { x = tx * kTileSize - w * kTileSize * 0.5f + kTileSize * 0.5f; z = ty * kTileSize - h * kTileSize * 0.5f + kTileSize * 0.5f; };
    auto blobs = [&](int type) {
        std::vector<Blob> res;
        std::vector<uint8_t> seen((size_t)w * h, 0);
        for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) {
            size_t i = (size_t)t.tileIndex(x, y);
            if (seen[i] || t.type[i] != type) continue;
            std::vector<int> stack{(int)i}; seen[i] = 1;
            double sx = 0, sy = 0; int n = 0;
            while (!stack.empty()) {
                int c = stack.back(); stack.pop_back();
                int cx = c % w, cy = c / w; sx += cx; sy += cy; n++;
                for (int dy = -1; dy <= 1; dy++) for (int dx = -1; dx <= 1; dx++) {
                    int nx = cx + dx, ny = cy + dy;
                    if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
                    size_t j = (size_t)t.tileIndex(nx, ny);
                    if (!seen[j] && t.type[j] == type) { seen[j] = 1; stack.push_back((int)j); }
                }
            }
            if (n < 2) continue;   // ignore a single stray tile
            Blob b; centreOf((float)(sx / n), (float)(sy / n), b.x, b.z); b.n = n; b.minY = y;
            res.push_back(b);
        }
        return res;
    };
    std::vector<Blob> tees = blobs(TT_Tee), greens = blobs(TT_PuttingGreen);
    for (const Blob& tb : tees) {
        int best = -1; float bd = 1e30f;
        for (size_t g = 0; g < greens.size(); g++) {
            if (greens[g].used) continue;
            float d = std::hypot(greens[g].x - tb.x, greens[g].z - tb.z);
            if (d < bd) { bd = d; best = (int)g; }
        }
        if (best < 0 || bd < 250.0f) continue;   // a tee with no green left, or one right on top of it
        greens[(size_t)best].used = true;
        HoleRoute r; r.teeX = tb.x; r.teeZ = tb.z; r.greenX = greens[(size_t)best].x; r.greenZ = greens[(size_t)best].z;
        r.length = bd; r.par = bd < 1300 ? 3 : bd < 2600 ? 4 : 5;
        const std::vector<float>& P = t.path;
        if (P.size() >= 4 && std::hypot(P[0] - r.teeX, P[1] - r.teeZ) < 300 && std::hypot(P[P.size() - 2] - r.greenX, P[P.size() - 1] - r.greenZ) < 300) r.route = P;
        else r.route = {r.teeX, r.teeZ, r.greenX, r.greenZ};
        out.push_back(r);
    }
    if (out.empty() && t.path.size() >= 4) {   // an old course with a stored route but no recognisable tee and green
        HoleRoute r; r.route = t.path;
        r.teeX = t.path[0]; r.teeZ = t.path[1]; r.greenX = t.path[t.path.size() - 2]; r.greenZ = t.path[t.path.size() - 1];
        r.length = std::hypot(r.greenX - r.teeX, r.greenZ - r.teeZ); r.par = r.length < 1300 ? 3 : r.length < 2600 ? 4 : 5;
        out.push_back(r);
    }
    return out;
}

}  // namespace sg

namespace sg {

HoleRating rateHole(const Terrain& t, const HoleRoute& r, int samples, int difficulty) {
    HoleRating out;
    if (r.route.size() < 4 || samples < 1) return out;
    auto avg = [&](const GolferSkills& s, float* drive) {
        double sum = 0, drv = 0;
        for (int i = 0; i < samples; i++) {
            ShotSim sim;
            sim.skills = s; sim.loop = false;
            sim.setRoute(&r.route);
            sim.init(t, 1234u + (uint32_t)i * 977u);
            int guard = 0, firstStroke = 0;
            while (!sim.finished && guard++ < 30 * 400) {
                sim.step(1.0f / 30.0f);
                if (!firstStroke && sim.stroke >= 1 && std::strcmp(sim.event, "drive") != 0 && sim.ballH <= 0) { firstStroke = 1; drv += std::hypot(sim.ballX - r.route[0], sim.ballZ - r.route[1]); }
            }
            sum += std::min(sim.stroke, 12);
        }
        if (drive) *drive = (float)(drv / samples);
        return (float)(sum / samples);
    };
    GolferSkills full;
    for (int& v : full.v) v = 15;
    out.avgStrokes = avg(full, &out.avgDrive);
    GolferSkills noLen = full; noLen.v[GolferSkills::Power] = 0; noLen.v[GolferSkills::LongDriver] = 0;
    GolferSkills noAcc = full; noAcc.v[GolferSkills::AccDriver] = 0; noAcc.v[GolferSkills::AccIrons] = 0;
    out.len = std::max(0.0f, avg(noLen, nullptr) - out.avgStrokes);
    out.acc = std::max(0.0f, avg(noAcc, nullptr) - out.avgStrokes);
    out.img = 0;
    static const char* names[8] = {"Breather", "Freeway", "Precise", "Challenge", "Creative", "Heroic", "Strategic", "Classic"};
    // The exe flags a skill as demanded when full-skill golfers beat golfers lacking it by 50 hundredths of a stroke or more (25 on the easiest difficulty).
    const float thr = difficulty == 0 ? 0.25f : 0.50f;
    out.typeIndex = (out.len >= thr ? 1 : 0) | (out.acc >= thr ? 2 : 0) | (out.img >= thr ? 4 : 0);
    out.type = names[out.typeIndex];
    return out;
}

}  // namespace sg
