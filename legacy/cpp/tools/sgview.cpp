// sgview: native terrain viewer for the SimGolf port (SDL2 + legacy fixed-function OpenGL).
//
//   sgview --game "<dir>/Program_Files_(ENGLISH)" [--theme parkland|links|desert|tropical]
//          [--seed N] [--size WxH] [--zoom Z] [--rot DEG] [--center TX,TY] [--time SECONDS] [--follow] [--png out.png]
//
// Keys: arrows/WASD pan, Q/E rotate, +/- or mouse wheel zoom, 1-4 theme, R new demo course, P toggle scenery, F follow the golfer,
//       F2 screenshot, Esc quit.  Left-drag pans.
// The camera follows the original: ortho +-5000 depth, pitch about X, then yaw 45 degrees.
#ifdef __APPLE__
#define GL_SILENCE_DEPRECATION
#endif
#include <SDL.h>
#include <SDL_opengl.h>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <algorithm>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <map>
#include <memory>
#include <set>
#include "sg/audio.h"
#include "sg/economy.h"
#include "sg/formats.h"
#include "sg/holes.h"
#include "sg/properties.h"
#include "ui.h"
#include "sg/shot.h"
#include "sg/sprites.h"
#include "sg/terrain.h"

#ifndef GL_GENERATE_MIPMAP
#define GL_GENERATE_MIPMAP 0x8191
#endif
#ifndef GL_CLAMP_TO_EDGE
#define GL_CLAMP_TO_EDGE 0x812F
#endif

using namespace sg;

static const char* kThemes[4] = {"Parkland", "Links", "Desert", "Tropical"};

// A sprite plus its lazily created GL textures (one per view/frame).
struct GlSprite {
    Sprite s;
    std::vector<GLuint> tex;
};

// A placed object. Sprites are billboards anchored on the ground at (x, z).
struct Prop {
    float x, z;
    GlSprite *body = nullptr, *shadow = nullptr;
    bool flat = false;  // ground overlay (building base), drawn under everything else
    int frame = 0;      // frame within a view
    float heading = -1000;  // world heading in degrees (atan2(z, x)); only used by 8 view sprites
    int walker = -1;        // index into App::walkers, -1 for static props
    int golfer = -1;        // index into App::golfers, -1 for other props
    bool hidden = false;    // a pool golfer who is not on the course
    bool tree = false;      // scenery regenerated from the terrain after edits
    bool building = false;  // an amenity placed by the player (kept when props are regenerated)
};

// One golfer on the course. A fixed pool is allocated so the routes the simulations point at never move.
struct Golfer {
    ShotSim sim;
    bool active = false;
    int look = 0, hole = 0, strokesRound = 0, lastStroke = -1;
    const char* lastEvent = nullptr;
    std::vector<float> route;
    double holeStart = 0;
    int mood = 4;                // the golfer's mood value, which sets the green fee (the exe keeps it per golfer as a small integer)
};
constexpr int kMaxGolfers = 8;
constexpr int kLooks = 4;

struct Walker {
    float dist = 0;   // distance along the path, world units
    float speed = 0;  // world units per second
};

struct App {
    std::string gameDir;
    int theme = 0;
    uint32_t seed = 7;
    Terrain terrain;
    TextureCatalog catalog;
    Lighting light;
    std::map<std::string, GLuint> textures;
    std::map<GLuint, std::vector<Vertex>> batches;
    std::map<GLuint, std::vector<Vertex>> pathBatches;
    std::map<GLuint, std::vector<Vertex>> mudBatches;   // paths not joined to the clubhouse, drawn as mud tracks (manual p. 18)
    HoleInfo hole;
    std::string lastReport;
    std::map<GLuint, std::vector<Vertex>> wallBatches;  // retaining walls
    std::set<GLuint> waterTex;                          // textures that get the shimmer
    Economy econ;  // alpha-blended path overlays
    float camX = 0, camZ = 0, zoom = 0.36f, rot = 0;
    int drawW = 1024, drawH = 768;
    float dpi = 1.0f;  // drawable pixels per window point (2 on Retina)
    std::map<std::string, std::unique_ptr<GlSprite>> sprites;
    std::vector<Prop> props;
    bool showProps = true;
    std::vector<Walker> walkers;
    std::vector<float> pathLen;  // cumulative length of Terrain::path segments
    double time = 0;             // seconds, drives animation
    std::vector<Golfer> golfers;
    std::vector<HoleRoute> holes;                       // found from the painted tees and greens (see sg/holes.h)
    double simTime = 0, spawnTimer = 1e9;
    GlSprite* lookBody[kLooks][6] = {}; GlSprite* lookShadow[kLooks][6] = {};   // [look][GolferAnim]
    bool lookOk[kLooks] = {};
    int roundsStarted = 0;
    int difficulty = 1;              // 0..3 as in the exe (the standard game's value is not known yet; 1 is a guess)
    std::string courseName = "Demo Course";
    // Screens: the title menu, the property chooser and the course itself.
    enum { ScreenMenu, ScreenProperty, ScreenPlay, ScreenReport };
    int screen = ScreenPlay;
    ui::Image titleBase, titleUn, titleMo, worldBase, themeIcons[4], reportArt, dockArt;
    bool showAdvisor = true;     // H toggles the advisor and story banners
    bool pauseToggle = false;    // set by the dock's pause button, handled in the main loop
    int panel = 0;               // open dock panel: 0 none, 1 terrain, 2 buildings, 3 people
    int dockHover = -1;          // dock button under the mouse (index into kDock), -1 none
    int panelHover = -1;         // list row under the mouse in an open panel
    std::vector<std::string> storyLines;   // lines of a story file from the disc (loaded at run time, never copied into the repo)
    std::string storyTitle;
    size_t storyPos = 0; double storyNext = 0;
    // Per hole statistics for the course report (reset when the course changes shape).
    struct HoleStat { int plays = 0; double strokes = 0, seconds = 0, revenue = 0, mood = 0; int hist[6] = {}; int moodSum = 0; };   // moodSum: total of the mood changes golfers had on this hole (the exe's per-hole counter)
    std::vector<HoleStat> holeStats;
    std::vector<HoleRating> ratings;
    ui::Font font;
    bool uiOk = false;
    ui::View view;
    int hover = -1;                  // button or card under the mouse, -1 none
    int themePack = 0;               // chosen on the title screen; its text and golfers are not used yet
    bool resetClock = false;
    bool sandboxChoice = false;      // the property chooser was opened from Sandbox Mode
    std::string toast;
    double toastUntil = 0;
    bool follow = false;         // camera follows the golfer
    // Course editing
    bool edit = false;
    int tool = 0;                // 0 paint, 1 raise (shift lowers), 2 path (shift removes)
    int pathKind = 1;
    int buildIdx = 0;            // selected amenity in kBuild (tool 4)
    struct Placed { int def, tx, ty; };
    std::vector<Placed> buildings;
    int raiseSign = 1;           // = selects raising, - selects lowering (the original's hotkeys); shift flips it
    bool paused = false;
    int wallDir = 0;             // edge chosen under the mouse, for the wall tool            // 1 gravel, 2 paved
    int paintIdx = 0;
    int brush = 0;               // radius in tiles
    bool hasHit = false;
    float hitX = 0, hitZ = 0;    // ground point under the mouse
    int lastCell = -1;           // last tile/corner edited during a drag
    bool dirty = false;
    GLdouble mv[16] = {};
    float upp = 1;               // world units per drawable pixel (last render)
    std::string courseFile = "course.sgc";
    GolferSkills skills;
    // Sound
    std::unique_ptr<Mixer> mixer;
    AudioDevice audioDev;
    bool mute = false, soundLog = false, musicOn = false;
    int ambience = -1, music = -1;
    size_t musicIdx = 0;
};

static void snd(App& app, const char* rel, float vol = 1.0f, bool loop = false, int* voiceOut = nullptr) {
    if (!app.mixer || app.mute) return;
    int id = app.mixer->play(rel, vol, loop);
    if (voiceOut) *voiceOut = id;
    if (app.soundLog) std::printf("  sound: %s%s\n", rel, id < 0 ? "  (missing)" : "");
}

static void startAmbience(App& app) {
    if (app.ambience >= 0 && app.mixer) app.mixer->stop(app.ambience);
    app.ambience = -1;
    snd(app, "GolfAmbience122.wav", 0.22f, true, &app.ambience);
}

static void toggleMusic(App& app) {
    if (!app.mixer) return;
    if (app.music >= 0) app.mixer->stop(app.music);
    app.music = -1;
    app.musicOn = !app.musicOn;
    if (!app.musicOn) return;
    static const char* kFolders[4] = {"music/misc_music/", "music/links_music/", "music/desert_music/", "music/tropical_music/"};
    auto tracks = app.mixer->list(kFolders[app.theme]);
    if (tracks.empty()) return;
    snd(app, tracks[app.musicIdx++ % tracks.size()].c_str(), 0.35f, true, &app.music);
}

struct PaintEntry { const char* name; int type; int vbyte; };
static const PaintEntry kPaint[] = {
    {"Fairway", 2, 0}, {"Firm fairway", 3, 0}, {"Green", 1, 0}, {"Tee", 0, 0}, {"Rough", 4, 0}, {"Deep rough", 5, 0},
    {"Woods", 13, 0}, {"Sand bunker", 7, 0}, {"Pot bunker", 9, 0}, {"Water shallow", 17, 0}, {"Water middle", 17, 1},
    {"Water deep", 17, 2}, {"Rock", 12, 0}, {"Brush", 11, 0}, {"Building lot", 22, 0},
    {"Cliff (editor id)", 31, 0}, {"Ravine (editor id)", 32, 0}, {"Flower bed (editor id)", 33, 0}, {"Zen sand (editor id)", 34, 0}, {"Grass bunker (editor id)", 35, 0}};
static const int kPaintCount = (int)(sizeof kPaint / sizeof kPaint[0]);

// Original pitch angles, chosen per resolution in Terrain::initSystem (38.68, 40.54, 40.83 degrees).
static double pitchFor(int w, int h) {
    if (w == 800 && h == 600) return 38.682186;
    if (w == 1280 && h == 1024) return 40.832218;
    return 40.541603;
}

static GLuint textureFor(App& app, const std::string& path) {
    auto it = app.textures.find(path);
    if (it != app.textures.end()) return it->second;
    Bytes d; Rgba img; std::string err;
    GLuint id = 0;
    const bool tga = path.size() > 4 && strcasecmp(path.c_str() + path.size() - 4, ".tga") == 0;
    if (readFile(path, d) && (tga ? decodeTga(d, img, err) : decodeBmp(d, img, err))) {
        glGenTextures(1, &id);
        glBindTexture(GL_TEXTURE_2D, id);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_GENERATE_MIPMAP, GL_TRUE);
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, (GLsizei)img.w, (GLsizei)img.h, 0, GL_RGBA, GL_UNSIGNED_BYTE, img.px.data());
    }
    app.textures[path] = id;
    return id;
}

// Path overlay (our own model, see docs/PATHS.md): every path tile gets a centre piece, plus an arm toward each path
// neighbour, cut out of the game's cross shaped Path.tga (arms 0..0.33 / 0.66..1 of the tile, centre between).
static void buildPaths(App& app) {
    app.pathBatches.clear();
    app.mudBatches.clear();
    const Terrain& t = app.terrain;
    if (t.pathKind.empty()) return;
    const std::vector<uint8_t> connected = pathsConnectedToClubhouse(t);
    const float ox = -t.w * kTileSize * 0.5f, oz = -t.h * kTileSize * 0.5f;
    auto vert = [&](float wx, float wz, float u, float v) {
        float y = t.heightAt(wx, wz) + 1.5f;
        float e = 12.0f;
        float nx = t.heightAt(wx - e, wz) - t.heightAt(wx + e, wz), nz = t.heightAt(wx, wz - e) - t.heightAt(wx, wz + e), ny = 2 * e;
        float l = std::sqrt(nx * nx + ny * ny + nz * nz);
        return Vertex{wx, y, wz, u, v, nx / l, ny / l, nz / l};
    };
    for (int ty = 0; ty < t.h; ty++)
        for (int tx = 0; tx < t.w; tx++) {
            int kind = t.pathAt(tx, ty);
            if (!kind) continue;
            const bool N = t.pathAt(tx, ty - 1), S = t.pathAt(tx, ty + 1), W = t.pathAt(tx - 1, ty), E = t.pathAt(tx + 1, ty);
            const int links = N + S + W + E;
            const char* file = kind == 2 ? "PathX.tga" : "Path.tga";
            const char* disc = kind == 2 ? "PathCurveX.tga" : "PathCurve.tga";
            // Rectangles in tile fractions {u0,v0,u1,v1}; piece textures: cross for arms and busy centres, disc for ends and bends.
            struct Piece { float r[4]; bool useDisc; };
            std::vector<Piece> pieces;
            const float a = 0.33f, b = 0.67f;
            const bool straight = (N && S && !W && !E) || (W && E && !N && !S);
            if (links >= 3 || straight) pieces.push_back({{a, a, b, b}, false});
            else pieces.push_back({{0.22f, 0.22f, 0.78f, 0.78f}, true});
            if (N) pieces.push_back({{a, 0, b, a}, false});
            if (S) pieces.push_back({{a, b, b, 1}, false});
            if (W) pieces.push_back({{0, a, a, b}, false});
            if (E) pieces.push_back({{b, a, 1, b}, false});
            for (const Piece& p : pieces) {
                GLuint tex = textureFor(app, app.gameDir + "/Data/Textures/" + kThemes[app.theme] + "/" + (p.useDisc ? disc : file));
                auto& batch = (connected[(size_t)t.tileIndex(tx, ty)] ? app.pathBatches : app.mudBatches)[tex];
                const int D = 2;
                for (int j = 0; j < D; j++)
                    for (int i = 0; i < D; i++) {
                        float u0 = p.r[0] + (p.r[2] - p.r[0]) * i / D, u1 = p.r[0] + (p.r[2] - p.r[0]) * (i + 1) / D;
                        float v0 = p.r[1] + (p.r[3] - p.r[1]) * j / D, v1 = p.r[1] + (p.r[3] - p.r[1]) * (j + 1) / D;
                        // Disc pieces map the whole texture onto the rectangle.
                        auto tu = [&](float u) { return p.useDisc ? (u - p.r[0]) / (p.r[2] - p.r[0]) : u; };
                        auto tv = [&](float v) { return p.useDisc ? (v - p.r[1]) / (p.r[3] - p.r[1]) : v; };
                        auto V = [&](float u, float v) { return vert(ox + (tx + u) * kTileSize, oz + (ty + v) * kTileSize, tu(u), tv(v)); };
                        Vertex q00 = V(u0, v0), q10 = V(u1, v0), q11 = V(u1, v1), q01 = V(u0, v1);
                        for (const Vertex* vv : {&q00, &q10, &q11, &q00, &q11, &q01}) batch.push_back(*vv);
                    }
            }
        }
}

// Retaining walls: PLACEHOLDER look, a vertical strip standing on the tile edge, kWallHeight units tall, plus a thin cap.
static void buildWalls(App& app) {
    app.wallBatches.clear();
    const Terrain& t = app.terrain;
    if (t.wallMask.empty()) return;
    const float kWallHeight = 30.0f, kCap = 5.0f;
    const float ox = -t.w * kTileSize * 0.5f, oz = -t.h * kTileSize * 0.5f;
    GLuint tex = textureFor(app, app.gameDir + "/Data/Textures/" + kThemes[app.theme] + "/RetainingWallA.bmp");
    auto& batch = app.wallBatches[tex];
    for (int ty = 0; ty < t.h; ty++)
        for (int tx = 0; tx < t.w; tx++)
            for (int dir = 0; dir < 4; dir++) {
                if (!t.wallAt(tx, ty, dir)) continue;
                // A shared edge is drawn by the north / west tile only, unless it lies on the map border.
                static const int dx[4] = {0, 1, 0, -1}, dy[4] = {-1, 0, 1, 0};
                if ((dir == 1 || dir == 2) && t.wallAt(tx + dx[dir], ty + dy[dir], (dir + 2) & 3)) continue;
                float x0, z0, x1, z1, ix = 0, iz = 0;   // edge endpoints, and the inward direction for the cap
                const float X = ox + tx * kTileSize, Z = oz + ty * kTileSize;
                if (dir == 0) { x0 = X; z0 = Z; x1 = X + kTileSize; z1 = Z; iz = kCap; }
                else if (dir == 2) { x0 = X; z0 = Z + kTileSize; x1 = X + kTileSize; z1 = z0; iz = -kCap; }
                else if (dir == 3) { x0 = X; z0 = Z; x1 = X; z1 = Z + kTileSize; ix = kCap; }
                else { x0 = X + kTileSize; z0 = Z; x1 = x0; z1 = Z + kTileSize; ix = -kCap; }
                const int N = 4;
                for (int i = 0; i < N; i++) {
                    float a = (float)i / N, b = (float)(i + 1) / N;
                    float xa = x0 + (x1 - x0) * a, za = z0 + (z1 - z0) * a, xb = x0 + (x1 - x0) * b, zb = z0 + (z1 - z0) * b;
                    float ya = t.heightAt(xa, za), yb = t.heightAt(xb, zb);
                    float nx = dir == 3 ? -1.f : dir == 1 ? 1.f : 0.f, nz = dir == 0 ? -1.f : dir == 2 ? 1.f : 0.f;
                    Vertex p0{xa, ya, za, a, 0.3f, nx, 0, nz}, p1{xb, yb, zb, b, 0.3f, nx, 0, nz};
                    Vertex p2{xb, yb + kWallHeight, zb, b, 0.0f, nx, 0, nz}, p3{xa, ya + kWallHeight, za, a, 0.0f, nx, 0, nz};
                    for (const Vertex* q : {&p0, &p1, &p2, &p0, &p2, &p3}) batch.push_back(*q);
                    // cap
                    Vertex c0{xa, ya + kWallHeight, za, a, 0.3f, 0, 1, 0}, c1{xb, yb + kWallHeight, zb, b, 0.3f, 0, 1, 0};
                    Vertex c2{xb + ix, yb + kWallHeight, zb + iz, b, 0.4f, 0, 1, 0}, c3{xa + ix, ya + kWallHeight, za + iz, a, 0.4f, 0, 1, 0};
                    for (const Vertex* q : {&c0, &c1, &c2, &c0, &c2, &c3}) batch.push_back(*q);
                }
            }
}

static void rebuildBatches(App& app) {
    app.batches.clear();
    app.waterTex.clear();
    app.terrain.desert = app.theme == 2;   // the original swaps shallow water for its desert variant in this theme
    for (int y = 0; y < app.terrain.h; y++)
        for (int x = 0; x < app.terrain.w; x++) {
            std::vector<TileTri> tris;
            buildTileTriangles(app.terrain, x, y, tris);
            for (const TileTri& t : tris) {
                const std::string* path = app.catalog.pick(t.texType, t.set, t.variation);
                GLuint texId = path ? textureFor(app, *path) : 0;
                if (texId && (t.texType == TT_WaterShallow || t.texType == TT_Marsh || t.texType == TT_WaterMiddle || t.texType == TT_WaterDeep || t.texType == TT_WaterShallowDesert)) app.waterTex.insert(texId);
                auto& batch = app.batches[texId];
                for (const Vertex& v : t.v) batch.push_back(v);
            }
        }
    buildPaths(app);
    buildWalls(app);
}

static GlSprite* spriteFor(App& app, const std::string& rel, bool shadow, const std::string& pal = std::string()) {
    const std::string path = app.gameDir + "/Flics/" + rel;
    auto it = app.sprites.find(path + "|" + pal);
    if (it != app.sprites.end()) return it->second.get();
    auto gs = std::make_unique<GlSprite>();
    std::string err;
    const std::string key = path + "|" + pal;
    if (!loadSprite(path, gs->s, err, shadow, pal.empty() ? pal : app.gameDir + "/Flics/" + pal)) {
        if (!shadow) std::fprintf(stderr, "sprite: %s\n", err.c_str());  // many objects simply have no shadow file
        app.sprites[key] = nullptr;
        return nullptr;
    }
    gs->tex.assign(gs->s.frames.size(), 0);
    return (app.sprites[key] = std::move(gs)).get();
}

static GLuint spriteTexture(GlSprite& g, int view, int frame) {
    size_t i = (size_t)(view % g.s.views) * g.s.framesPerView + (size_t)(frame % g.s.framesPerView);
    if (!g.tex[i]) {
        const Rgba& img = g.s.frames[i];
        glGenTextures(1, &g.tex[i]);
        glBindTexture(GL_TEXTURE_2D, g.tex[i]);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, (GLsizei)img.w, (GLsizei)img.h, 0, GL_RGBA, GL_UNSIGNED_BYTE, img.px.data());
    }
    return g.tex[i];
}

namespace {
struct Rng {
    uint32_t s;
    uint32_t next() { s ^= s << 13; s ^= s >> 17; s ^= s << 5; return s; }
    float unit() { return (next() & 0xFFFFFF) / float(0x1000000); }
    int range(int n) { return (int)(next() % (uint32_t)n); }
};
}  // namespace

static void addTrees(App& app) {
    static const char* kTrees[4][6] = {
        {"Trees/TreeMapleLarge", "Trees/TreeMapleMedium", "Trees/TreePineLarge", "Trees/TreePineMedium", "Trees/TreePineFirLg", "Trees/TreeMapleSmall"},
        {"Trees/Links/LinksPine_Tall", "Trees/Links/LinksPine_Med", "Trees/Links/LinksTree3_Tall", "Trees/Links/LinksTree3_Med", "Trees/Links/LinksTree4_Tall", "Trees/Links/LinksTree4_Med"},
        {"Trees/Desert/JoshuaTree_Lg", "Trees/Desert/JoshuaTree_Md", "Trees/Desert/CactusA_Lg", "Trees/Desert/CactusB_Md", "Trees/Desert/TreeCactusLg", "Trees/Desert/CactusC_Lg"},
        {"Trees/Tropic/TreePalm/TreePalmLg", "Trees/Tropic/TreePalm/TreePalmMed", "Trees/Tropic/Tree_Cerc/Cerc_Large", "Trees/Tropic/Tree_Drac/Drac_Large", "Trees/Tropic/Tree_Tall_Palm/TallPalm_Large", "Trees/Tropic/Tree_Cerc/Cerc_Med"}};
    const Terrain& t = app.terrain;
    auto tileCentre = [&](int tx, int ty, float& x, float& z) { x = tx * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f; z = ty * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f; };
    for (int ty = 0; ty < t.h; ty++)
        for (int tx = 0; tx < t.w; tx++) {
            if (t.type[(size_t)t.tileIndex(tx, ty)] != 13) continue;
            Rng rng{app.seed * 2654435761u + (uint32_t)(ty * 64 + tx) * 40503u + 12345u};
            rng.next(); rng.next();
            int n = 1 + rng.range(2);
            for (int k = 0; k < n; k++) {
                const char* base = kTrees[app.theme][rng.range(6)];
                Prop p;
                p.tree = true;
                tileCentre(tx, ty, p.x, p.z);
                p.x += (rng.unit() - 0.5f) * 80; p.z += (rng.unit() - 0.5f) * 80;
                // Colour variants live in separate palette files next to the sprites.
                std::string b = base, pal;
                if (b.find("Tropic/TreePalm/") != std::string::npos) pal = "Trees/Tropic/TreePalm/PalGreenPalm.pcx";
                else if (b.find("Tree_Cerc") != std::string::npos) pal = "Trees/Tropic/Tree_Cerc/PalCerc.pcx";
                else if (b.find("Tree_Drac") != std::string::npos) pal = "Trees/Tropic/Tree_Drac/PalDrac.pcx";
                else if (b.find("Tree_Tall_Palm") != std::string::npos) pal = "Trees/Tropic/Tree_Tall_Palm/PalTallPalm.pcx";
                p.body = spriteFor(app, std::string(base) + ".flc", false, pal);
                p.shadow = spriteFor(app, std::string(base) + "Shadow.flc", true);
                if (!p.body) continue;
                p.frame = p.body->s.framesPerView - 1;  // fully grown
                app.props.push_back(p);
            }
        }
}

static void refreshTrees(App& app) {
    app.props.erase(std::remove_if(app.props.begin(), app.props.end(), [](const Prop& p) { return p.tree; }), app.props.end());
    // addTrees appends; keep trees first so painter's order ties stay as before.
    size_t before = app.props.size();
    addTrees(app);
    std::rotate(app.props.begin(), app.props.begin() + before, app.props.end());
}

// Amenities. Sprite names are the level 1 files found per theme in Flics/Bldgs; costs are PLACEHOLDERS (the exe's building costs are not decoded).
// A Snack Bar visit pays 5 units, which is the exe's figure for building type 7 (assumed to be the snack bar).
struct BuildDef { const char* name; const char* sprite[4]; int cost; int visit; };
static const BuildDef kBuild[] = {
    {"Snack Bar",    {"Bldgs/Park/ParkSnackL1", nullptr, nullptr, "Bldgs/Tropical/TROPsnackL1"}, 10, 5},
    {"Pro Shop",     {"Bldgs/Park/ProsL1", nullptr, "Bldgs/Desert/dproL1", "Bldgs/Tropical/TROPproshopL1"}, 30, 0},
    {"Cart Garage",  {"Bldgs/Park/cartL1", "Bldgs/links/Cart_garageL1", "Bldgs/Desert/DEScartL1", "Bldgs/Tropical/TROPcartL1"}, 20, 0},
    {"Hotel",        {"Bldgs/Park/HotelL1", "Bldgs/links/HotelL1", "Bldgs/Desert/DesHotelL1", "Bldgs/Tropical/TROPhotelL1"}, 50, 0},
    {"Tennis Court", {"Bldgs/Park/tenL1", nullptr, "Bldgs/Desert/tenL1", nullptr}, 25, 0},
    {"Marina",       {"Bldgs/Park/MarL1", nullptr, nullptr, "Bldgs/Tropical/TROPmarL1"}, 40, 0},
};
static const int kBuildCount = (int)(sizeof kBuild / sizeof kBuild[0]);
static bool buildAvailable(const App& app, int d) { return kBuild[d].sprite[app.theme] != nullptr; }

static void addBuildingProp(App& app, const App::Placed& b) {
    const Terrain& t = app.terrain;
    Prop p;
    p.building = true;
    p.x = b.tx * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f;
    p.z = b.ty * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f;
    const char* base = kBuild[b.def].sprite[app.theme];
    if (!base) return;
    p.body = spriteFor(app, std::string(base) + ".flc", false);
    p.shadow = spriteFor(app, std::string(base) + "Shadow.flc", true);
    if (p.body) app.props.push_back(p);
}

// Demo scenery: trees on Woods tiles and a clubhouse. File names per theme are the ones found in Flics/.
static void populateProps(App& app) {
    app.props.clear();
    static const char* kClub[4][2] = {{"Bldgs/Park/clubL2", "Bldgs/Park/clubL2_base"}, {"Bldgs/links/clubL2", "Bldgs/links/clubL2_dirt"},
                                      {"Bldgs/Desert/DESclubL2", "Bldgs/Desert/DesClubL1base"}, {"Bldgs/Tropical/TROPclubL2", "Bldgs/Tropical/TROPclubL2_base"}};
    const Terrain& t = app.terrain;
    auto tileCentre = [&](int tx, int ty, float& x, float& z) { x = tx * kTileSize - t.w * kTileSize * 0.5f + kTileSize * 0.5f; z = ty * kTileSize - t.h * kTileSize * 0.5f + kTileSize * 0.5f; };
    addTrees(app);
    for (const App::Placed& b : app.buildings) addBuildingProp(app, b);
    if (t.clubhouseX >= 0) {
        Prop base, body;
        tileCentre(t.clubhouseX, t.clubhouseY, body.x, body.z);
        base.x = body.x; base.z = body.z;
        std::string b = kClub[app.theme][0], g = kClub[app.theme][1];
        body.body = spriteFor(app, b + ".flc", false);
        body.shadow = spriteFor(app, b + "Shadow.flc", true);
        base.body = spriteFor(app, g + ".flc", false);
        base.shadow = spriteFor(app, g + "Shadow.flc", true);
        base.flat = true;
        if (base.body) app.props.push_back(base);
        if (body.body) app.props.push_back(body);
    }
    // The pool of golfers (their sprites differ for variety). Golfers arrive over time, see stepGame().
    app.walkers.clear();
    app.pathLen.clear();
    static const char* kLook[kLooks] = {"Male/MaleKLS", "Male/MalePLS", "Female/FemalePLS", "Female/FemaleSSS"};
    static const char* kAnim[6] = {"_NormalWalk", "_NormalAddress", "_PerfectSwing", "_PuttAddress", "_Putt", "_Happy"};
    for (int l = 0; l < kLooks; l++) {
        app.lookOk[l] = true;
        for (int i = 0; i < 6; i++) {
            const std::string anim = (i == 2 && l >= 2) ? "_NormalSwing" : kAnim[i];   // the women have no PerfectSwing clip
            app.lookBody[l][i] = spriteFor(app, std::string(kLook[l]) + anim + ".flc", false);
            app.lookShadow[l][i] = spriteFor(app, std::string(kLook[l]) + anim + "Shadow.flc", true);
            if (!app.lookBody[l][i]) app.lookOk[l] = false;
        }
    }
    app.holes = findHoles(app.terrain);
    app.golfers.assign(kMaxGolfers, Golfer());
    app.econ.init(app.terrain);
    app.simTime = 0; app.spawnTimer = 1e9;
    for (int i = 0; i < kMaxGolfers; i++) {
        Prop p;
        p.golfer = i; p.hidden = true;
        p.body = app.lookBody[0][0]; p.shadow = app.lookShadow[0][0];
        app.props.push_back(p);
    }
}

// Starts a golfer on the first hole. Skills vary from golfer to golfer (PLACEHOLDER spread of 4 to 11 out of 15).
static void spawnGolfer(App& app) {
    if (app.holes.empty()) return;
    for (int i = 0; i < kMaxGolfers; i++) {
        if (app.golfers[(size_t)i].active) continue;
        Golfer& g = app.golfers[(size_t)i];
        uint32_t r = app.seed * 2654435761u + (uint32_t)(app.simTime * 1000) + (uint32_t)i * 97u + (uint32_t)app.roundsStarted * 7919u + 1u;
        auto next = [&]() { r ^= r << 13; r ^= r >> 17; r ^= r << 5; return r; };
        g = Golfer();
        g.active = true;
        g.look = (int)(next() % kLooks);
        if (!app.lookOk[g.look]) g.look = 0;
        for (int k = 0; k < 10; k++) g.sim.skills.v[k] = 4 + (int)(next() % 8);
        if (app.roundsStarted == 0) g.sim.skills = app.skills;   // the first golfer is the one picked with --golfer
        g.route = app.holes[0].route;
        g.holeStart = app.simTime;
        // The exe starts each golfer's mood at 3 plus a random 0 to 2, or at 4 on the easiest difficulty.
        g.mood = app.difficulty == 0 ? 4 : 3 + (int)(next() % 3);
        g.sim.loop = false;
        g.sim.setRoute(&g.route);
        g.sim.init(app.terrain, next());
        app.roundsStarted++;
        return;
    }
}

static int golfersOnCourse(const App& app) { int n = 0; for (const Golfer& g : app.golfers) n += g.active; return n; }

static void golferSounds(App& app, Golfer& g) {
    const char* ev = g.sim.event;
    if (!std::strcmp(ev, "drive")) snd(app, !std::strcmp(g.sim.club, "iron") ? "Golf_Sfx/Iron.wav" : "Golf_Sfx/Drive With Ball.wav", 0.8f);
    else if (!std::strcmp(ev, "putt")) snd(app, "Golf_Sfx/Putt.wav", 0.8f);
    else if (!std::strcmp(ev, "on the course")) snd(app, "Golf_Sfx/Ball Drop Fairway.wav", 0.5f);
    else if (!std::strcmp(ev, "in the sand")) snd(app, "Golf_Sfx/Ball Drop Sand.wav", 0.6f);
    else if (!std::strncmp(ev, "splash", 6)) snd(app, "Golf_Sfx/Ball Water.wav", 0.8f);
    else if (!std::strncmp(ev, "out of bounds", 13)) snd(app, "Golf_Sfx/Ball Tree.wav", 0.8f);
    else if (!std::strcmp(ev, "holed")) {
        snd(app, "Golf_Sfx/Ball In Hole.wav", 0.8f);
        snd(app, g.sim.stroke <= 5 ? "ApplauseGood.wav" : "Applause.wav", 0.5f);
        snd(app, "Effects/cash.wav", 0.6f);
    }
}

// Advances the club by dt seconds: arrivals, every golfer's round, fees, wages and the board's messages.
static void stepGame(App& app, float dt) {
    const bool open = !app.holes.empty() && !app.econ.gameOver;
    // Arrivals (PLACEHOLDER rates): the first golfer comes at once, then one every 25 s when golfers are happy, slower when not;
    // the course takes at most two golfers per hole, up to the size of the pool.
    app.spawnTimer += dt;
    const double every = 25.0 / (0.5 + app.econ.fun / 100.0);
    const int capacity = std::min(kMaxGolfers, 2 * (int)app.holes.size());
    if (open && app.spawnTimer >= every && golfersOnCourse(app) < capacity) { spawnGolfer(app); app.spawnTimer = 0; }
    app.econ.step(dt);
    if (app.econ.notice[0]) { std::printf("[%6.1fs] board: %s\n", app.simTime, app.econ.notice); app.econ.notice = ""; }
    for (size_t gi = 0; gi < app.golfers.size(); gi++) {
        Golfer& g = app.golfers[gi];
        if (!g.active) continue;
        g.sim.paceScale = app.econ.staff[Economy::Ranger] ? 1.2f : 1.0f;   // a Ranger speeds play up (PLACEHOLDER; the original works near one tee)
        g.sim.step(dt);
        if (g.sim.event != g.lastEvent || g.sim.stroke != g.lastStroke) {
            g.lastEvent = g.sim.event; g.lastStroke = g.sim.stroke;
            golferSounds(app, g);
            if (!std::strcmp(g.sim.event, "holed")) {
                const int par = g.hole < (int)app.holes.size() ? app.holes[(size_t)g.hole].par : 4;
                const double before = app.econ.cash;
                // Green fee, from the exe's fee routine, in units of 100: the golfer's mood, plus 2 for a Creative class hole or 5 for a
                // Heroic, Strategic or Classic one (the exe adds 2 for the hole type values above 3, and 3 more unless the value is 4; here
                // the type index is L + 2A + 4I, which is an ASSUMPTION about the exe's numbering). The exe also adds 2 each for two hole
                // flags (probably Top 100 and Top 18) and an Airstrip bonus; none of those exist here yet.
                int bonus = 0;
                if (g.hole < (int)app.ratings.size()) { const int t = app.ratings[(size_t)g.hole].typeIndex; if (t > 3) bonus = 2 + (t != 4 ? 3 : 0); }
                const int feeUnits = std::max(0, g.mood) + bonus;
                app.econ.holeFinished(g.sim.stroke, par, feeUnits);
                // PLACEHOLDER rule: a golfer who holes out within 6 tiles of a paying amenity uses it once (the exe's visit rules are not decoded).
                if (g.hole < (int)app.holes.size())
                    for (const App::Placed& b : app.buildings) {
                        if (kBuild[b.def].visit <= 0) continue;
                        const float bx = b.tx * kTileSize - app.terrain.w * kTileSize * 0.5f + kTileSize * 0.5f, bz = b.ty * kTileSize - app.terrain.h * kTileSize * 0.5f + kTileSize * 0.5f;
                        if (std::hypot(bx - app.holes[(size_t)g.hole].greenX, bz - app.holes[(size_t)g.hole].greenZ) < 6 * kTileSize) { app.econ.earn(kBuild[b.def].visit * Economy::kUnit); break; }
                    }
                // After the hole the exe lowers mood by (hole field + 6 + holes played) * (mood - 1 + difficulty) * (difficulty + 1) /
                // ((course factor * 5 + 15) * 8), integer division. The hole field and the course factor are not decoded; both are taken as 0.
                { const int d = app.difficulty; const int dec = ((6 + g.hole) * (g.mood - 1 + d) * (d + 1)) / 120; g.mood -= dec; }
                // PLACEHOLDER: par or better makes a golfer a little happier (the real mood events are not decoded yet).
                const int moodDelta = g.sim.stroke <= par ? 1 : (g.sim.stroke >= par + 3 ? -1 : 0);   // PLACEHOLDER events
                if (moodDelta > 0 && g.mood < 10) g.mood++;
                if (app.holeStats.size() != app.holes.size()) app.holeStats.assign(app.holes.size(), App::HoleStat());
                if (g.hole < (int)app.holeStats.size()) {
                    App::HoleStat& hs = app.holeStats[(size_t)g.hole];
                    hs.plays++; hs.moodSum += moodDelta; hs.strokes += g.sim.stroke; hs.seconds += app.simTime - g.holeStart; hs.revenue += app.econ.cash - before; hs.mood += app.econ.lastMood;
                    hs.hist[std::min(5, std::max(0, g.sim.stroke - 3))]++;
                }
                g.strokesRound += g.sim.stroke;
                std::printf("[%6.1fs] golfer %zu holed hole %d in %d (par %d), fee $%.0f, cash $%.0f, fun %.0f\n", app.simTime, gi, g.hole + 1, g.sim.stroke, par, app.econ.cash - before, app.econ.cash, app.econ.fun);
            }
        }
        if (g.sim.finished) {
            if (++g.hole < (int)app.holes.size()) {
                g.route = app.holes[(size_t)g.hole].route;
                g.holeStart = app.simTime;
                g.sim.init(app.terrain, g.sim.stroke * 7919u + (uint32_t)g.hole + (uint32_t)gi);
            } else {
                std::printf("[%6.1fs] golfer %zu finished the round in %d strokes\n", app.simTime, gi, g.strokesRound);
                g.active = false;
            }
        }
    }
}

// Advances animation: golfers play, sprite frames cycle at the file's frame time.
static void updateProps(App& app) {
    for (; app.simTime < app.time; app.simTime += 1.0 / 60.0) stepGame(app, 1.0f / 60.0f);
    if (app.time < app.simTime - 1.0) { app.simTime = 0; for (Golfer& g : app.golfers) g.active = false; app.spawnTimer = 1e9; }   // clock went backwards
    for (Prop& p : app.props) {
        if (p.golfer < 0) continue;
        const Golfer& g = app.golfers[(size_t)p.golfer];
        p.hidden = !g.active;
        if (!g.active) continue;
        const ShotSim& s = g.sim;
        int a = (int)s.anim;
        if (!app.lookBody[g.look][a]) { p.hidden = true; continue; }
        p.body = app.lookBody[g.look][a]; p.shadow = app.lookShadow[g.look][a];
        p.x = s.golferX; p.z = s.golferZ; p.heading = s.golferHeading;
        float fr = s.animTime * 1000.0f / (float)std::max<uint32_t>(1, p.body->s.frameMs);
        bool loop = s.anim == GolferAnim::Walk || s.anim == GolferAnim::Happy;
        int n = p.body->s.framesPerView;
        p.frame = loop ? (int)fr % n : std::min((int)fr, n - 1);
    }
}

static bool loadTheme(App& app, int theme) {
    std::string err, dir = app.gameDir + "/Data/Textures/" + kThemes[theme];
    TextureCatalog cat;
    if (!cat.load(dir, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return false; }
    for (auto& kv : app.textures) if (kv.second) glDeleteTextures(1, &kv.second);
    app.textures.clear();
    app.catalog = std::move(cat);
    app.theme = theme;
    app.sprites.clear();  // GL textures of sprites are leaked on theme change; they are small
    app.light = Lighting();
    loadLighting(app.gameDir + "/" + kThemes[theme] + "Lighting.txt", app.light);
    rebuildBatches(app);
    populateProps(app);
    return true;
}

// Draws props as camera-facing quads. Sprites are pre-rendered, so they ignore lighting and depth:
// ground overlays first, then shadows, then bodies far to near (painter's order).
static void drawProps(App& app) {
    if (!app.showProps || app.props.empty()) return;
    updateProps(app);
    if (app.follow) for (const Golfer& g : app.golfers) if (g.active) { app.camX = g.sim.golferX; app.camZ = g.sim.golferZ; break; }
    GLdouble mv[16];
    glGetDoublev(GL_MODELVIEW_MATRIX, mv);
    const float rx = (float)mv[0], ry = (float)mv[4], rz = (float)mv[8];   // eye X axis in world space
    const float ux = (float)mv[1], uy = (float)mv[5], uz = (float)mv[9];   // eye Y axis in world space
    // The sprites were rendered at 4 camera yaws, 90 degrees apart; 8 view sprites (people) face 45 degree steps.
    const int quarter = ((int)std::lround(app.rot / 90.0f) % 4 + 4) % 4;
    const float s = kSpriteUnitsPerPixel;

    struct Item { float depth; const Prop* p; };
    std::vector<Item> bodies;
    for (const Prop& p : app.props) {
        if (p.hidden) continue;
        float y = app.terrain.heightAt(p.x, p.z);
        float depth = (float)(mv[2] * p.x + mv[6] * y + mv[10] * p.z);
        bodies.push_back({depth, &p});
    }
    std::sort(bodies.begin(), bodies.end(), [](const Item& a, const Item& b) { return a.depth < b.depth; });

    glDisable(GL_LIGHTING);
    glDisable(GL_DEPTH_TEST);
    glEnable(GL_TEXTURE_2D);
    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_REPLACE);
    auto quad = [&](GlSprite* g, const Prop& p) {
        if (!g) return;
        int views = g->s.views;
        int view = views >= 8 ? (quarter * 2) % 8 : (views >= 4 ? quarter % views : (views == 2 ? quarter % 2 : 0));
        if (views >= 8 && p.heading > -999) {
            // View 0 faces world -X (heading 180); each further view turns 45 degrees counter clockwise
            // on screen, i.e. heading - 45; a 90 degree camera turn adds 2 views.
            int k = (int)std::lround((180.0f - p.heading) / 45.0f);
            view = ((k + 2 * quarter) % 8 + 8) % 8;
        }
        glBindTexture(GL_TEXTURE_2D, spriteTexture(*g, view, p.frame));
        float y = app.terrain.heightAt(p.x, p.z);
        float l = -g->s.anchorX * s, r = ((float)g->s.w - g->s.anchorX) * s;
        float t = g->s.anchorY * s, b = -((float)g->s.h - g->s.anchorY) * s;  // up is positive
        auto corner = [&](float cx, float cy, float u, float v) {
            glTexCoord2f(u, v);
            glVertex3f(p.x + rx * cx + ux * cy, y + ry * cx + uy * cy, p.z + rz * cx + uz * cy);
        };
        glBegin(GL_QUADS);
        corner(l, t, 0, 0); corner(r, t, 1, 0); corner(r, b, 1, 1); corner(l, b, 0, 1);
        glEnd();
    };
    for (const Item& it : bodies) if (it.p->flat) { quad(it.p->body, *it.p); }
    for (const Item& it : bodies) { quad(it.p->shadow, *it.p); }
    for (const Item& it : bodies) if (!it.p->flat) quad(it.p->body, *it.p);
    // The ball: a small white disc with a dark disc on the ground below it.
    for (const Golfer& gl : app.golfers) {
        if (!gl.active || gl.sim.ballH < 0) continue;
        const ShotSim& sm = gl.sim;
        float gy = app.terrain.heightAt(sm.ballX, sm.ballZ);
        glDisable(GL_TEXTURE_2D);
        auto disc = [&](float x, float y, float z, float rpx, float rr, float gg, float bb, float aa, bool upright) {
            glColor4f(rr, gg, bb, aa);
            glBegin(GL_TRIANGLE_FAN);
            glVertex3f(x, y, z);
            for (int i = 0; i <= 12; i++) {
                float a = i * 6.2831853f / 12, cx = std::cos(a) * rpx, cy = std::sin(a) * rpx;
                if (upright) glVertex3f(x + rx * cx + ux * cy, y + ry * cx + uy * cy, z + rz * cx + uz * cy);
                else glVertex3f(x + cx, y, z + cy * 0.8f);
            }
            glEnd();
        };
        disc(sm.ballX, gy + 1, sm.ballZ, 4.5f, 0, 0, 0, 0.45f, false);
        disc(sm.ballX, gy + sm.ballH + 4.0f, sm.ballZ, 5.0f, 1, 1, 1, 1, true);
        glColor4f(1, 1, 1, 1);
    }
    glDisable(GL_BLEND);
    glEnable(GL_DEPTH_TEST);
}

// Course Status Report (the original's F1): the hole's class and length, and how many paths are joined to the clubhouse.
static void reportCourse(App& app, bool force) {
    app.hole = analyzeHole(app.terrain);
    app.holes = findHoles(app.terrain);
    if (app.holeStats.size() != app.holes.size()) app.holeStats.assign(app.holes.size(), App::HoleStat());
    int paths = 0, joined = 0;
    const std::vector<uint8_t> conn = pathsConnectedToClubhouse(app.terrain);
    for (size_t i = 0; i < app.terrain.pathKind.size(); i++) if (app.terrain.pathKind[i]) { paths++; joined += conn[i]; }
    char b[160];
    std::snprintf(b, sizeof b, "%zu holes (tee to green pairs); paths: %d tiles, %d joined to the clubhouse, %d shown as mud", app.holes.size(), paths, joined, paths - joined);
    std::string r = app.hole.report() + "; " + b;
    if (force || r != app.lastReport) { std::printf("course: %s\n", r.c_str()); app.lastReport = r; }
}

// ---- Screens: title menu, property chooser, heads-up display --------------------------------------------------------

struct Rect { float x, y, w, h; bool has(float px, float py) const { return px >= x && py >= y && px < x + w && py < y + h; } };

// Regions of the 800x600 title art (read off TitleBASE / TitleUnSel / TitleMO in the disc's Interface folder).
static const Rect kMenuBtn[6] = {{40, 25, 325, 120}, {415, 40, 360, 135}, {12, 395, 285, 100}, {508, 380, 270, 95}, {285, 478, 285, 105}, {718, 528, 64, 64}};
static const char* kThemePacks[5] = {"Standard", "Firaxis", "More Stories", "The Sims", "Championship"};   // the folders under Themes/
static const char* kMenuLabel[5] = {"Continue Saved Game", "Start New Game (Standard)", "Sandbox Mode", "Select A Theme", "Play a Championship"};
static const float kMenuLabelX[5] = {180, 595, 150, 650, 418}, kMenuLabelY[5] = {83, 112, 450, 427, 543};

static bool loadUi(App& app) {
    const std::string i = app.gameDir + "/Interface/";
    bool ok = app.font.load(app.gameDir + "/KLEPTO__.TTF");
    ok = ui::loadPcx(i + "TitleBASE.pcx", app.titleBase, false) && ok;
    ok = ui::loadPcx(i + "TitleUnSel.pcx", app.titleUn, true) && ok;
    ok = ui::loadPcx(i + "TitleMO.pcx", app.titleMo, true) && ok;
    ok = ui::loadPcx(i + "WorldBase.pcx", app.worldBase, false) && ok;
    ui::loadPcx(i + "infoscreens/coursereport.pcx", app.reportArt, true);   // optional
    ui::loadPcx(i + "3mainLowerLeft.pcx", app.dockArt, false, 0xF800F8);               // optional: the lower left dock
    static const char* kIcon[4] = {"ChooseParklandButtons.pcx", "ChooseLinksButtons.pcx", "ChooseDesertButtons.pcx", "ChooseTropicalButtons.pcx"};
    for (int t = 0; t < 4; t++) ui::loadPcx(i + kIcon[t], app.themeIcons[t], false, 0xFF0000);   // icons are optional
    app.uiOk = ok;
    return ok;
}

static std::string money(long long v) {   // "§1,234,567" (the game's currency sign, UTF-8)
    std::string digits = std::to_string(v < 0 ? -v : v), out;
    for (size_t k = 0; k < digits.size(); k++) { if (k && (digits.size() - k) % 3 == 0) out += ','; out += digits[k]; }
    return std::string(v < 0 ? "-" : "") + "\xC2\xA7" + out;
}

static void drawMenu(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.titleBase, 0, 0);
    ui::drawImage(app.titleMo, 170, 190, 170, 190, 480, 165);       // the logo sits in the highlight layer
    ui::drawImage(app.titleUn, 0, 0);
    if (app.hover >= 0 && app.hover < 6) { const Rect& r = kMenuBtn[app.hover]; ui::drawImage(app.titleMo, r.x, r.y, r.x, r.y, r.w, r.h); }
    for (int b = 0; b < 5; b++) {
        std::string label = kMenuLabel[b];
        app.font.drawCentered(kMenuLabelX[b], kMenuLabelY[b], label, 19, 0.12f, 0.12f, 0.38f);
    }
    app.font.drawCentered(kMenuLabelX[3], kMenuLabelY[3] + 17, std::string("Theme: ") + kThemePacks[app.themePack], 13, 0.12f, 0.12f, 0.38f);
    if (!app.toast.empty() && SDL_GetTicks() / 1000.0 < app.toastUntil) {
        const float w = app.font.width(app.toast, 18) + 24;
        ui::fillRect(400 - w / 2, 560, w, 30, 0.1f, 0.1f, 0.3f, 0.9f);
        app.font.drawCentered(400, 581, app.toast, 18, 1, 1, 0.8f);
    }
    ui::endScreen();
}

static Rect propertyCard(int i) {
    static const float lx[10] = {250, 301, 345, 383, 416, 437, 453, 464, 468, 462}, lw[10] = {175, 173, 174, 174, 173, 173, 174, 176, 173, 174};
    static const float ly[10] = {12, 64, 115, 167, 220, 277, 337, 398, 457, 520};
    static const float rx[6] = {465, 508, 545, 579, 605, 622}, ry[6] = {12, 64, 115, 167, 220, 277};
    const Property& p = kProperties[i];
    int idx = 0;
    for (int k = 0; k < i; k++) if (kProperties[k].column == p.column) idx++;
    return p.column == 0 ? Rect{lx[idx], ly[idx], lw[idx], 48} : Rect{rx[idx], ry[idx], 175, 48};
}

static bool canAfford(const App& app, int i) { return app.sandboxChoice || kProperties[i].price <= kStartFunds; }

static void drawProperty(App& app) {
    app.view = ui::beginScreen(app.drawW, app.drawH);
    ui::drawImage(app.worldBase, 0, 0);
    app.font.draw(24, 36, "Where will you build your golf course?", 14, 0.1f, 0.1f, 0.35f);
    app.font.drawCentered(737, 32, app.sandboxChoice ? "Unlimited \xC2\xA7" : money(kStartFunds), 17, 0.1f, 0.1f, 0.35f);
    for (int i = 0; i < 16; i++) {
        const Property& p = kProperties[i];
        const Rect r = propertyCard(i);
        const bool ok = canAfford(app, i);
        const float a = ok ? 1.0f : 0.5f;
        ui::drawImage(app.themeIcons[p.theme], r.x + 1, r.y - 1, ok ? 200.0f : 0.0f, 0, 52, 52);
        const float cx = r.x + 62 + (r.w - 74) * 0.5f;
        app.font.drawCentered(cx, r.y + 15, p.name, 16, 0.08f, 0.08f, 0.3f, ok ? 1.0f : 0.55f);
        app.font.drawCentered(cx, r.y + 26, p.bonus, 11, 0.2f, 0.2f, 0.35f, ok ? 1.0f : 0.55f);
        char line[64]; std::snprintf(line, sizeof line, "%d acres: ", p.acres);
        if (!app.sandboxChoice) app.font.drawCentered(cx, r.y + 43, std::string(line) + money(p.price), 12, 0.25f, 0.18f, 0.1f, ok ? 1.0f : 0.55f);
        if (app.hover == i) { ui::fillRect(r.x, r.y, r.w, r.h, 1, 1, 0.4f, 0.22f); }
    }
    if (app.hover >= 0 && app.hover < 16) {
        const Property& p = kProperties[app.hover];
        char l[160]; std::snprintf(l, sizeof l, "%s, %d acres. Bonus: %s.", p.name, p.acres, p.bonus);
        app.font.draw(26, 568, l, 14, 0.1f, 0.1f, 0.3f);
        if (!canAfford(app, app.hover)) app.font.draw(26, 586, "Not enough funds.", 12, 0.55f, 0.1f, 0.1f);
    }
    ui::endScreen();
}

// The Course Report (the original's Information menu, Course Report). Drawn from the disc's coursereport.pcx pieces: a header band,
// repeated row strips, a total strip and a legend. Column layout read off that art. Yards use 0.15 yards per world unit (so a full
// drive of 900 units is 135 yards, close to the 128 yard average drive seen in a Hole Stats screenshot): a placeholder scale.
static std::string num(double v, int dec) {
    if (dec == 0) { std::string out; long long n = std::llround(v); std::string d = std::to_string(n < 0 ? -n : n); for (size_t k = 0; k < d.size(); k++) { if (k && (d.size() - k) % 3 == 0) out += ','; out += d[k]; } return (n < 0 ? "-" : "") + out; }
    char b[48]; std::snprintf(b, sizeof b, "%.*f", dec, v);
    return b;
}

static void drawReport(App& app) {
    if (!app.uiOk || !app.reportArt.tex) return;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    static const float cx[12] = {112, 157, 190, 233, 283, 330, 375, 422, 468, 571, 641, 716}, cw[12] = {41, 29, 39, 46, 43, 41, 43, 42, 99, 66, 71, 74};
    static const char* head[12] = {"Yds", "Par", "Avg", "Time", "Fun", "+Len", "+Acc", "+Img", "Type", "Avg.Fee", "Revenue", "Profit"};
    const int n = (int)app.holes.size();
    const int rowH = 22, top = 104, bodyY = 108;
    const float totalY = (float)(bodyY + n * rowH + 6);
    ui::fillRect(0, 100, 800, totalY - 100, 148 / 255.0f, 150 / 255.0f, 198 / 255.0f, 1);
    ui::drawImage(app.reportArt, 0, 0, 0, 0, 800, (float)top);
    app.font.drawCentered(323, 53, "COURSE REPORT", 26, 0.15f, 0.12f, 0.3f);
    for (int c = 0; c < 12; c++) app.font.drawCentered(cx[c] + cw[c] / 2, 89, head[c], 12, 0.1f, 0.1f, 0.3f);
    if (app.ratings.size() != app.holes.size()) { app.ratings.clear(); for (const HoleRoute& r : app.holes) app.ratings.push_back(rateHole(app.terrain, r, 40, app.difficulty)); }
    double tY = 0, tPar = 0, tStr = 0, tSec = 0, tMood = 0, tRev = 0, tPlays = 0, tProf = 0, tLen = 0, tAcc = 0, tImg = 0;
    const double upkeepShare = n ? (app.econ.upkeepPaid + app.econ.wagesPaid) / n : 0;
    for (int i = 0; i < n; i++) {
        const float y = (float)(bodyY + i * rowH);
        ui::drawImage(app.reportArt, 0, y, 0, i % 2 ? 168.0f : 128.0f, 800, 19);
        const HoleRoute& r = app.holes[(size_t)i];
        const HoleRating& rt = app.ratings[(size_t)i];
        const App::HoleStat hs = i < (int)app.holeStats.size() ? app.holeStats[(size_t)i] : App::HoleStat();
        const double yds = r.length * 0.15, avg = hs.plays ? hs.strokes / hs.plays : 0, mins = hs.plays ? hs.seconds / hs.plays * 0.35 : 0;   // 0.35 game minutes per sim second: placeholder
        const double fun = hs.plays ? hs.mood / hs.plays : 0, fee = hs.plays ? hs.revenue / hs.plays : 0, profit = hs.revenue - upkeepShare;
        tY += yds; tPar += r.par; tStr += avg; tSec += mins; tMood += fun; tRev += hs.revenue; tPlays += hs.plays; tProf += profit; tLen += rt.len; tAcc += rt.acc; tImg += rt.img;
        auto cell = [&](int c, const std::string& s, int tint) {   // tint: 0 none, 1 green, 2 red
            if (tint) ui::drawImage(app.reportArt, cx[c], y + 2, cx[c], tint == 2 ? 205.0f : 240.0f, cw[c], 15);
            app.font.drawCentered(cx[c] + cw[c] / 2, y + 14, s, 12, tint ? 1.0f : 0.1f, tint ? 1.0f : 0.1f, tint ? 1.0f : 0.3f);
        };
        app.font.draw(16, y + 14, "Hole " + std::to_string(i + 1), 12, 0.1f, 0.1f, 0.3f);
        cell(0, num(yds, 0), 0); cell(1, std::to_string(r.par), 0);
        cell(2, hs.plays ? num(avg, 2) : "-", 0); cell(3, hs.plays ? num(mins, 0) + "m" : "-", 0);
        cell(4, hs.plays ? num(fun, 0) + "%" : "-", hs.plays ? (fun >= 70 ? 1 : fun < 45 ? 2 : 0) : 0);
        cell(5, num(rt.len, 2), 0); cell(6, num(rt.acc, 2), 0); cell(7, num(rt.img, 2), 0);
        cell(8, rt.type, 0);
        cell(9, hs.plays ? num(fee, 0) : "-", 0); cell(10, num(hs.revenue, 0), 0); cell(11, num(profit, 0), profit < 0 ? 2 : (profit > 0 ? 1 : 0));
    }
    ui::drawImage(app.reportArt, 0, totalY, 0, 420, 800, 92);
    ui::fillRect(0, totalY + 28, 24, 26, 148 / 255.0f, 150 / 255.0f, 198 / 255.0f, 1);   // the art carries some layout numbers in its margin
    {
        const float y = totalY + 8 + 14;
        auto cell = [&](int c, const std::string& s) { app.font.drawCentered(cx[c] + cw[c] / 2, y, s, 12, 0.1f, 0.1f, 0.3f); };
        app.font.draw(16, y, "Total", 12, 0.1f, 0.1f, 0.3f);
        const double k = n ? 1.0 / n : 0;
        cell(0, num(tY, 0)); cell(1, num(tPar, 0)); cell(2, tPlays ? num(tStr, 2) : "-"); cell(3, tPlays ? num(tSec, 0) + "m" : "-");
        cell(4, tPlays ? num(tMood * k, 0) + "%" : "-"); cell(5, num(tLen * k, 2)); cell(6, num(tAcc * k, 2)); cell(7, num(tImg * k, 2));
        cell(9, tPlays ? num(tRev / tPlays, 0) : "-"); cell(10, num(tRev, 0)); cell(11, num(tProf, 0));
        const float ly = totalY + 55 + 6;
        app.font.draw(138, ly, "Top 100 Hole", 11, 0.1f, 0.1f, 0.3f); app.font.draw(296, ly, "Top 18 Hole", 11, 0.1f, 0.1f, 0.3f); app.font.draw(448, ly, "Scenic Hole", 11, 0.1f, 0.1f, 0.3f);
    }
    ui::endScreen();
}

static void loadStory(App& app);
static void startGame(App& app, int propIdx, bool sandbox) {
    const Property& p = kProperties[propIdx];
    app.econ.sandbox = sandbox;
    app.econ.startCash = sandbox ? kStartFunds : kStartFunds - p.price;   // the property is paid for out of the starting funds
    app.seed = app.seed * 1664525u + 1013904223u + (uint32_t)propIdx * 7919u;
    app.terrain = Terrain::demoCourse(40, 40, app.seed);
    app.courseName = std::string(p.name) + " GC";
    loadTheme(app, p.theme);
    app.screen = App::ScreenPlay;
    app.hover = -1;
    app.edit = false;
    app.econ.day = 1;
    app.panel = 0; app.buildings.clear();
    loadStory(app);
    app.resetClock = true;
    std::printf("new game: %s, %d acres, price %d, cash left %.0f%s\n", p.name, p.acres, p.price, app.econ.startCash, sandbox ? " (sandbox)" : "");
}

// Club ratings as the exe computes them. Fun: each hole contributes 100 * (sum of mood changes) / (plays + 4), and the club's value is
// the sum over holes (the exe also divides by half of another per-hole counter that is not decoded; taken as 0). Skill: the sum over holes
// of the Length, Accuracy and Imagination differences in strokes (the exe keeps hundredths and prints with two decimals).
static int clubFun(const App& app) {
    long long s = 0;
    for (const App::HoleStat& h : app.holeStats) if (h.plays) s += (100LL * h.moodSum) / (h.plays + 4);
    return (int)s;
}
static double clubSkill(const App& app) {
    double s = 0;
    for (const HoleRating& r : app.ratings) s += r.len + r.acc + r.img;
    return s;
}

static const char* kMonths[12] = {"January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"};

// Heads-up display over the course: club name and date, and the money and fun counters. Layout is my own, from the screenshots.
// One economy day counts as one month (1024 ticks in the exe).

// ---- In-game interface: the lower left dock, its panels, the advisor and the golfer stories -------------------------------------------------
// The dock art is the disc's own Interface/3mainLowerLeft.pcx (the assembled dock sits in the bottom left of the sheet, with the highlighted
// versions of each button elsewhere on it). Button positions are measured from that art. The panels, advisor text and layout are my own.
struct DockBtn { float cx, cy, r; float sx, sy, sw, sh, hoverDx; };   // dock centre and radius; normal sprite on the sheet; offset to the highlighted one
static const DockBtn kDock[10] = {
    {43, 473, 33, 0, 0, 76, 80, 100},     // Build Course
    {117, 497, 30, 0, 100, 70, 78, 100},  // Add Buildings
    {177, 536, 26, 0, 200, 64, 76, 100},  // People
    {32, 543, 11, 598, 48, 30, 30, 50},   // zoom in
    {31, 585, 11, 598, 98, 30, 30, 50},   // zoom out
    {17, 565, 11, 598, 148, 30, 30, 50},  // rotate right
    {47, 565, 11, 598, 198, 30, 30, 50},  // rotate left
    {75, 552, 14, 598, 248, 34, 34, 50},  // information (course report)
    {107, 568, 14, 598, 298, 34, 34, 50}, // pause
    {133, 583, 13, 598, 348, 34, 34, 50}, // tools (save the course)
};
static const char* kDockHelp[10] = {"Build Course", "Add Buildings", "People", "Zoom in", "Zoom out", "Rotate right", "Rotate left", "Course report", "Pause", "Save the course"};

// Items in the open panel, laid out in three columns from the panel's top left.
struct PanelItem { std::string label; int kind; int arg; };   // kind: 0 paint (arg kPaint index), 1 path, 2 raise, 3 lower, 4 building (arg kBuild index), 5 staff (arg Staff)
static std::vector<PanelItem> panelItems(const App& app) {
    std::vector<PanelItem> v;
    char b[96];
    if (app.panel == 1) {
        for (int i = 0; i < 15; i++) { std::snprintf(b, sizeof b, "%s  %s", kPaint[i].name, money(Economy::terrainCostUnits(kPaint[i].type) * 100LL).c_str()); v.push_back({b, 0, i}); }
        v.push_back({"Path, gravel", 1, 1}); v.push_back({"Path, paved", 1, 2}); v.push_back({"Raise ground", 2, 0}); v.push_back({"Lower ground", 3, 0});
    } else if (app.panel == 2) {
        for (int i = 0; i < kBuildCount; i++) {
            if (!buildAvailable(app, i)) continue;
            std::snprintf(b, sizeof b, "%s  %s", kBuild[i].name, money(kBuild[i].cost * 100LL).c_str());
            v.push_back({b, 4, i});
        }
    } else if (app.panel == 3) {
        for (int k = 0; k < Economy::StaffKinds; k++) { std::snprintf(b, sizeof b, "%s: %d  (click to hire, right click to fire)", Economy::staffName(k), app.econ.staff[k]); v.push_back({b, 5, k}); }
    }
    return v;
}
static void panelItemRect(int i, int cols, float& x, float& y, float& w, float& h) {
    const float cw = cols == 1 ? 540.0f : 182.0f;
    x = 232 + (i % cols) * cw; y = 462 + (i / cols) * 18.5f; w = cw - 4; h = 17;
}
static int panelCols(const App& app) { return app.panel == 3 ? 1 : 3; }

static void selectPanelItem(App& app, const PanelItem& it, bool rightClick) {
    switch (it.kind) {
        case 0: app.tool = 0; app.paintIdx = it.arg; app.edit = true; break;
        case 1: app.tool = 2; app.pathKind = it.arg; app.edit = true; break;
        case 2: app.tool = 1; app.raiseSign = 1; app.edit = true; break;
        case 3: app.tool = 1; app.raiseSign = -1; app.edit = true; break;
        case 4: app.tool = 4; app.buildIdx = it.arg; app.edit = true; break;
        case 5: if (rightClick) app.econ.fire(it.arg); else app.econ.hire(it.arg); break;
    }
}

static int dockHit(float vx, float vy) {
    for (int i = 0; i < 10; i++) { const float dx = vx - kDock[i].cx, dy = vy - kDock[i].cy; if (dx * dx + dy * dy <= kDock[i].r * kDock[i].r) return i; }
    return -1;
}

// Returns true when the click was on the dock or an open panel.
static bool dockClick(App& app, float vx, float vy, bool rightClick, bool& togglePause) {
    if (!app.uiOk || !app.dockArt.tex) return false;
    const int d = dockHit(vx, vy);
    if (d >= 0) {
        if (d <= 2) {
            const int want = d + 1;
            app.panel = app.panel == want ? 0 : want;
            app.edit = app.panel == 1 || app.panel == 2;
            if (app.panel == 1) app.tool = 0;
            if (app.panel == 2) { app.tool = 4; while (!buildAvailable(app, app.buildIdx)) app.buildIdx = (app.buildIdx + 1) % kBuildCount; }
        } else if (d == 3) app.zoom *= 1.12f;
        else if (d == 4) app.zoom /= 1.12f;
        else if (d == 5) app.rot += 15;
        else if (d == 6) app.rot -= 15;
        else if (d == 7) { reportCourse(app, true); if (app.reportArt.tex) { app.ratings.clear(); app.screen = App::ScreenReport; } }
        else if (d == 8) togglePause = true;
        else if (d == 9) { std::string err; if (app.terrain.save(app.courseFile, err)) { app.toast = "Course saved"; } else app.toast = "Could not save the course"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; }
        snd(app, "Interface/Button1.wav");
        return true;
    }
    if (app.panel) {
        const std::vector<PanelItem> items = panelItems(app);
        const int cols = panelCols(app);
        for (size_t i = 0; i < items.size(); i++) {
            float x, y, w, h; panelItemRect((int)i, cols, x, y, w, h);
            if (vx >= x && vx < x + w && vy >= y && vy < y + h) { selectPanelItem(app, items[i], rightClick); snd(app, "Interface/Button2.wav"); return true; }
        }
        if (vx >= 226 && vx < 796 && vy >= 452 && vy < 596) return true;   // the panel background swallows clicks
    }
    return false;
}

static void dockHoverUpdate(App& app, float vx, float vy) {
    app.dockHover = dockHit(vx, vy);
    app.panelHover = -1;
    if (app.panel) {
        const std::vector<PanelItem> items = panelItems(app);
        const int cols = panelCols(app);
        for (size_t i = 0; i < items.size(); i++) {
            float x, y, w, h; panelItemRect((int)i, cols, x, y, w, h);
            if (vx >= x && vx < x + w && vy >= y && vy < y + h) app.panelHover = (int)i;
        }
    }
}

// Loads one story script from the disc's Themes folder. Format (read from the files): a title line, then blocks separated by blank lines; the first line of a
// block is one golfer's line, the lines after it that start with a space are the other golfer's possible replies. PARTNER stands for the other golfer.
// What decides which story plays, and when, is not decoded; this picks one by seed and shows it when two golfers are on the course.
static void loadStory(App& app) {
    app.storyLines.clear(); app.storyTitle.clear(); app.storyPos = 0; app.storyNext = 0;
    namespace fs = std::filesystem;
    std::vector<std::string> files;
    std::error_code ec;
    for (const fs::directory_entry& de : fs::directory_iterator(app.gameDir + "/Themes/Standard", ec))
        if (de.path().extension() == ".txt") files.push_back(de.path().string());
    if (files.empty()) return;
    std::sort(files.begin(), files.end());
    std::ifstream in(files[app.seed % files.size()], std::ios::binary);
    std::string line; bool first = true, newBlock = true;
    while (std::getline(in, line)) {
        while (!line.empty() && (line.back() == '\r' || line.back() == '\n')) line.pop_back();
        if (first) { size_t a = line.find_first_not_of(' '); app.storyTitle = a == std::string::npos ? "" : line.substr(a); first = false; continue; }
        if (line.empty()) { newBlock = true; continue; }
        const bool indented = line[0] == ' ';
        size_t p;
        while ((p = line.find("PARTNER")) != std::string::npos) line.replace(p, 7, "pal");
        size_t a = line.find_first_not_of(' ');
        if (a == std::string::npos) continue;
        line = line.substr(a);
        if (!indented || newBlock) app.storyLines.push_back("A|" + line); else app.storyLines.push_back("B|" + line);
        newBlock = false;
    }
}

static std::vector<std::string> wrapText(const App& app, const std::string& s, float size, float maxW) {
    std::vector<std::string> out; std::string cur, word;
    auto flush = [&]() { if (!cur.empty()) { out.push_back(cur); cur.clear(); } };
    std::istringstream is(s);
    while (is >> word) {
        const std::string t = cur.empty() ? word : cur + " " + word;
        if (app.font.width(t, size) > maxW && !cur.empty()) { flush(); cur = word; } else cur = t;
    }
    flush();
    return out;
}

// Advisor text from what the club looks like now. These hints are my own words.
static std::string advisorText(const App& app) {
    if (app.holes.empty()) return "Welcome to your new club. Open Build Course (the big round button at the bottom left), then paint a tee and a green a good distance apart to make your first hole.";
    if (app.buildings.empty()) return "Golfers are on the course. Open Add Buildings and put up a snack bar: pick a building lot next to a path that joins the clubhouse. Visitors spend money there.";
    if (app.holes.size() < 3) return "More holes bring more golfers and more money. Build another tee and green, and make the holes different: long, narrow and tricky shots raise the club's skill rating.";
    if (app.econ.staffCount() == 0) return "Your course is growing. The People button lets you hire a club pro, ranger, groundskeeper or soda vendor, who cost wages but keep golfers happy.";
    return "Watch the fun and skill numbers at the top right. Press the information button for the course report, and keep cash above zero so the board stays calm.";
}

static void drawDockUi(App& app) {
    if (!app.uiOk) return;
    if (app.dockArt.tex) {
        ui::drawImage(app.dockArt, 0, 430, 0, 430, 215, 170);
        if (app.dockHover >= 0) {
            const DockBtn& b = kDock[app.dockHover];
            const float dx = b.cx - (b.sx + b.sw * 0.5f), dy = b.cy - (b.sy + b.sh * 0.5f);
            ui::drawImage(app.dockArt, b.sx + dx, b.sy + dy, b.sx + b.hoverDx, b.sy, b.sw, b.sh);
        }
        if (app.dockHover >= 0) app.font.draw(228, 448, kDockHelp[app.dockHover], 14, 1, 1, 0.7f);
    }
    if (app.panel) {
        ui::fillRect(226, 452, 570, 144, 0.16f, 0.14f, 0.34f, 0.88f);
        const std::vector<PanelItem> items = panelItems(app);
        const int cols = panelCols(app);
        for (size_t i = 0; i < items.size(); i++) {
            float x, y, w, h; panelItemRect((int)i, cols, x, y, w, h);
            bool sel = false;
            if (items[i].kind == 0) sel = app.tool == 0 && app.paintIdx == items[i].arg;
            else if (items[i].kind == 1) sel = app.tool == 2 && app.pathKind == items[i].arg;
            else if (items[i].kind == 2) sel = app.tool == 1 && app.raiseSign > 0;
            else if (items[i].kind == 3) sel = app.tool == 1 && app.raiseSign < 0;
            else if (items[i].kind == 4) sel = app.tool == 4 && app.buildIdx == items[i].arg;
            if (sel) ui::fillRect(x, y, w, h, 0.9f, 0.75f, 0.2f, 0.55f);
            else if ((int)i == app.panelHover) ui::fillRect(x, y, w, h, 0.5f, 0.5f, 0.9f, 0.45f);
            app.font.draw(x + 4, y + 13, items[i].label, 13, 1, 1, 1);
        }
    }
    // Advisor and story, top centre.
    if (app.showAdvisor) {
        const std::vector<std::string> lines = wrapText(app, advisorText(app), 14, 290);
        const float h = 10 + 17.0f * lines.size();
        ui::fillRect(244, 8, 306, h, 0.12f, 0.1f, 0.3f, 0.82f);
        for (size_t i = 0; i < lines.size(); i++) app.font.draw(252, 25 + 17.0f * i, lines[i], 14, 1, 0.95f, 0.7f);
        float y = 8 + h + 6;
        if (!app.storyLines.empty() && golfersOnCourse(app) >= 2) {
            const std::string& s = app.storyLines[app.storyPos % app.storyLines.size()];
            const bool a = s.size() > 2 && s[0] == 'A';
            const std::vector<std::string> sl = wrapText(app, s.substr(2), 14, 290);
            const float sh = 12 + 17.0f * (sl.size() + 1);
            ui::fillRect(244, y, 306, sh, a ? 0.25f : 0.12f, a ? 0.12f : 0.22f, a ? 0.12f : 0.12f, 0.82f);
            app.font.draw(252, y + 16, app.storyTitle + (a ? " (golfer one)" : " (golfer two)"), 12, 0.8f, 0.9f, 1);
            for (size_t i = 0; i < sl.size(); i++) app.font.draw(252, y + 33 + 17.0f * i, sl[i], 14, 1, 1, 1);
        }
    }
    if (!app.toast.empty() && SDL_GetTicks() / 1000.0 < app.toastUntil) {
        const float w = app.font.width(app.toast, 16) + 24;
        ui::fillRect(400 - w / 2, 410, w, 28, 0.5f, 0.1f, 0.1f, 0.88f);
        app.font.drawCentered(400, 430, app.toast, 16, 1, 1, 1);
    }
}

static void drawHud(App& app) {
    if (!app.uiOk) return;
    app.view = ui::beginScreen(app.drawW, app.drawH, false);
    // The exe's date stamp routine counts months in blocks of 1024 ticks and shows month numbers 3..10, so a year here is eight months,
    // March to October (medium confidence; the start year 2001 is a placeholder).
    const int mi = app.econ.day - 1;
    char date[48]; std::snprintf(date, sizeof date, "%s %d", kMonths[(2 + mi % 8) % 12], 2001 + mi / 8);
    ui::fillRect(8, 8, 230, 46, 0.12f, 0.1f, 0.3f, 0.78f);
    app.font.draw(18, 28, app.courseName, 17, 1, 1, 1);
    app.font.draw(18, 47, date, 14, 0.85f, 0.85f, 1);
    ui::fillRect(560, 8, 232, 70, 0.12f, 0.1f, 0.3f, 0.78f);
    app.font.draw(572, 30, app.econ.sandbox ? "Sandbox" : money((long long)app.econ.cash), 19, app.econ.cash < 0 && !app.econ.sandbox ? 1.0f : 1.0f, app.econ.cash < 0 && !app.econ.sandbox ? 0.5f : 1.0f, app.econ.cash < 0 && !app.econ.sandbox ? 0.5f : 0.7f);
    if (app.ratings.size() != app.holes.size()) { app.ratings.clear(); for (const HoleRoute& r : app.holes) app.ratings.push_back(rateHole(app.terrain, r, 20, app.difficulty)); }
    char fun[64]; std::snprintf(fun, sizeof fun, "Fun %d  Skill %.2f", clubFun(app), clubSkill(app));
    app.font.draw(572, 52, fun, 15, 0.9f, 0.9f, 1);
    char g[64]; std::snprintf(g, sizeof g, "Golfers %d, holes %zu", golfersOnCourse(app), app.holes.size());
    app.font.draw(572, 71, g, 13, 0.75f, 0.75f, 0.95f);
    drawDockUi(app);
    if (app.econ.gameOver) {
        ui::fillRect(200, 250, 400, 80, 0.5f, 0.05f, 0.05f, 0.9f);
        app.font.drawCentered(400, 300, "GAME OVER", 40, 1, 1, 1);
    }
    ui::endScreen();
}

// ---- Course editing ----------------------------------------------------------------------------------------

static void setTitle(App& app, SDL_Window* win) {
    char t[320];
    char cash[160]; std::snprintf(cash, sizeof cash, "%s, month %d%s", app.econ.sandbox ? "sandbox, unlimited funds" : (std::string("$") + std::to_string((long long)app.econ.cash)).c_str(), app.econ.day, app.econ.gameOver ? ", GAME OVER" : "");
    { char extra[96]; std::snprintf(extra, sizeof extra, " | golfers %d, holes %zu | fun %.0f (%s) | staff %d", golfersOnCourse(app), app.holes.size(), app.econ.fun, app.econ.attitude(), app.econ.staffCount()); std::strncat(cash, extra, sizeof cash - std::strlen(cash) - 1); }
    if (app.edit) std::snprintf(t, sizeof t, "SimGolf native: EDIT | %s | %s | brush %d | %s | Tab exit, T tool, [ ] type, , . brush, F5 save, F9 load, shift=lower",
                                app.tool == 0 ? "Paint" : app.tool == 1 ? "Raise/Lower" : app.tool == 2 ? "Path" : app.tool == 3 ? "Wall" : "Building", app.tool == 0 ? kPaint[app.paintIdx].name : app.tool == 2 ? (app.pathKind == 1 ? "gravel" : "paved") : app.tool == 3 ? "edge" : app.tool == 4 ? kBuild[app.buildIdx].name : "terrain", app.brush * 2 + 1, cash);
    else std::snprintf(t, sizeof t, "SimGolf native: %s | %s | Tab = edit course, M music, N mute", kThemes[app.theme], cash);
    SDL_SetWindowTitle(win, t);
}

// Ground point under a window position: ortho ray against the heightfield (a few fixed-point iterations).
static bool pickGround(App& app, int mx, int my, float dpi, float& wx, float& wz) {
    const GLdouble* m = app.mv;
    double ex = (mx * dpi - app.drawW * 0.5) * app.upp, ey = (app.drawH * 0.5 - my * dpi) * app.upp;
    double v[3] = {ex - m[12], ey - m[13], 0 - m[14]};
    double o[3], d[3];
    for (int j = 0; j < 3; j++) { o[j] = m[4 * j] * v[0] + m[4 * j + 1] * v[1] + m[4 * j + 2] * v[2]; d[j] = -m[4 * j + 2]; }
    if (std::fabs(d[1]) < 1e-6) return false;
    // March from above the highest possible ground down along the ray, then bisect the first crossing.
    const double top = Terrain::kMaxLevel * kHeightStep + 1.0;
    auto at = [&](double s, double& x, double& z) { x = o[0] + d[0] * s; z = o[2] + d[2] * s; return o[1] + d[1] * s - app.terrain.heightAt((float)x, (float)z); };
    double s0 = (top - o[1]) / d[1], s1 = s0, x = 0, z = 0;
    const double span = top / std::fabs(d[1]);
    const int steps = 120;
    double prev = at(s0, x, z);
    bool hit = false;
    for (int i = 1; i <= steps; i++) {
        s1 = s0 + span * i / steps;
        double v = at(s1, x, z);
        if (v <= 0) { hit = true; break; }
        s0 = s1; prev = v;
    }
    (void)prev;
    if (!hit) return false;
    for (int i = 0; i < 24; i++) { double m = 0.5 * (s0 + s1); if (at(m, x, z) > 0) s0 = m; else s1 = m; }
    at(s1, x, z);
    const float half = kTileSize;
    if (std::fabs(x) > app.terrain.w * half * 0.5 || std::fabs(z) > app.terrain.h * half * 0.5) return false;
    wx = (float)x; wz = (float)z;
    return true;
}

static void tileOf(const App& a, float wx, float wz, int& tx, int& ty) {
    tx = (int)std::floor((wx + a.terrain.w * kTileSize * 0.5f) / kTileSize);
    ty = (int)std::floor((wz + a.terrain.h * kTileSize * 0.5f) / kTileSize);
}
static void cornerOf(const App& a, float wx, float wz, int& cx, int& cy) {
    cx = (int)std::lround((wx + a.terrain.w * kTileSize * 0.5f) / kTileSize);
    cy = (int)std::lround((wz + a.terrain.h * kTileSize * 0.5f) / kTileSize);
}

static void editPaint(App& app, int tx, int ty) {
    const PaintEntry& pe = kPaint[app.paintIdx];
    const int r = app.brush;
    for (int dy = -r; dy <= r; dy++)
        for (int dx = -r; dx <= r; dx++) {
            if (dx * dx + dy * dy > r * r + r) continue;
            int x = tx + dx, y = ty + dy;
            int vb = pe.type == 0 ? (int)(((uint32_t)(x * 7 + y * 13)) % 5) : pe.vbyte;  // tees use this byte as their look
            if (x >= 0 && y >= 0 && x < app.terrain.w && y < app.terrain.h && app.terrain.type[(size_t)app.terrain.tileIndex(x, y)] != pe.type)
                app.econ.spend(Economy::terrainCostUnits(pe.type) * Economy::kUnit);
            app.terrain.paint(x, y, pe.type, vb);
            if (pe.type == 0 || pe.type == 1 || pe.type == 17 || pe.type == 22) app.terrain.flattenTile(x, y);
        }
    app.dirty = true;
    {
        const int ty = pe.type;
        const char* s = ty == 2 || ty == 3 ? "Interface/Place Fairway.wav" : ty == 0 || ty == 1 ? "Interface/Place GreenTee.wav"
                      : ty == 7 || ty == 9 || ty == 34 || ty == 35 ? "Interface/Place Bunker.wav" : ty == 17 ? "Interface/Place Water.wav"
                      : ty == 12 || ty == 31 || ty == 32 ? "Interface/Place Rocks.wav" : ty == 22 ? "Interface/Building.wav"
                      : ty == 33 ? "Effects/Flower Bed.wav" : "Interface/Place Rough.wav";
        snd(app, s, 0.7f);
    }
}

static void editPath(App& app, int tx, int ty, bool remove) {
    const int r = app.brush;
    if (app.terrain.pathKind.size() != app.terrain.type.size()) app.terrain.pathKind.assign(app.terrain.type.size(), 0);
    for (int dy = -r; dy <= r; dy++)
        for (int dx = -r; dx <= r; dx++) {
            int x = tx + dx, y = ty + dy;
            if (dx * dx + dy * dy > r * r + r || x < 0 || y < 0 || x >= app.terrain.w || y >= app.terrain.h) continue;
            uint8_t& pk = app.terrain.pathKind[(size_t)app.terrain.tileIndex(x, y)];
            if (!remove && pk == 0) app.econ.spend(Economy::kPathTileCost);
            pk = remove ? 0 : (uint8_t)app.pathKind;
        }
    app.dirty = true;
    snd(app, "Interface/Path.wav", 0.7f);
}

// Amenity placement. Rules from the game text: buildings go on a building lot and need a path to the clubhouse. The lot must touch a path tile that
// is connected to the clubhouse. Removing refunds the cost (the "money will be refunded" undo text).
static void editBuilding(App& app, int tx, int ty, bool remove) {
    const Terrain& t = app.terrain;
    if (tx < 0 || ty < 0 || tx >= t.w || ty >= t.h) return;
    for (size_t i = 0; i < app.buildings.size(); i++)
        if (app.buildings[i].tx == tx && app.buildings[i].ty == ty) {
            if (remove) {
                app.econ.earn(kBuild[app.buildings[i].def].cost * Economy::kUnit);
                app.buildings.erase(app.buildings.begin() + (long)i);
                app.props.erase(std::remove_if(app.props.begin(), app.props.end(), [](const Prop& p) { return p.building; }), app.props.end());
                for (const App::Placed& b : app.buildings) addBuildingProp(app, b);
                app.toast = "Building removed, money refunded"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3;
                snd(app, "Interface/Building.wav", 0.7f);
            }
            return;
        }
    if (remove) return;
    if (!buildAvailable(app, app.buildIdx)) { app.toast = "Not available in this theme"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; return; }
    if (t.type[(size_t)t.tileIndex(tx, ty)] != TT_Building) { app.toast = "Buildings go on a building lot"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; return; }
    const std::vector<uint8_t> conn = pathsConnectedToClubhouse(t);
    bool ok = false;
    static const int dx[4] = {0, 1, 0, -1}, dy[4] = {-1, 0, 1, 0};
    for (int k = 0; k < 4 && !ok; k++) {
        const int nx = tx + dx[k], ny = ty + dy[k];
        if (nx >= 0 && ny >= 0 && nx < t.w && ny < t.h && conn[(size_t)t.tileIndex(nx, ny)]) ok = true;
    }
    if (!ok) { app.toast = "Buildings need a path to the clubhouse"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; return; }
    const double cost = kBuild[app.buildIdx].cost * Economy::kUnit;
    if (!app.econ.sandbox && app.econ.cash < cost) { app.toast = "Not enough money"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; return; }
    app.econ.spend(cost);
    App::Placed b{app.buildIdx, tx, ty};
    app.buildings.push_back(b);
    addBuildingProp(app, b);
    snd(app, "Interface/Building.wav", 0.7f);
}

static void editWall(App& app, bool remove) {
    // Nearest tile edge to the picked point.
    int tx, ty; tileOf(app, app.hitX, app.hitZ, tx, ty);
    float fx = (app.hitX + app.terrain.w * kTileSize * 0.5f) / kTileSize - tx, fz = (app.hitZ + app.terrain.h * kTileSize * 0.5f) / kTileSize - ty;
    float d[4] = {fz, 1 - fx, 1 - fz, fx};  // distances to N, E, S, W edges
    int dir = (int)(std::min_element(d, d + 4) - d);
    app.terrain.setWall(tx, ty, dir, !remove);
    app.dirty = true;
    snd(app, "Interface/Place Rocks Generic.wav", 0.7f);
}

static void editRaise(App& app, int cx, int cy, int delta) {
    const int r = app.brush;
    for (int dy = -r; dy <= r; dy++)
        for (int dx = -r; dx <= r; dx++)
            if (dx * dx + dy * dy <= r * r + r) app.terrain.raiseCorner(cx + dx, cy + dy, delta);
    app.dirty = true;
    snd(app, delta > 0 ? "Interface/Bass Up 2.wav" : "Interface/Bass Down 2.wav", 0.6f);
}

// Applies the current tool at the picked ground point. During a drag it only fires when the cell changes.
static void applyTool(App& app, bool lower, bool dragging) {
    if (!app.hasHit) return;
    int a, b;
    if (app.tool != 1) tileOf(app, app.hitX, app.hitZ, a, b); else cornerOf(app, app.hitX, app.hitZ, a, b);
    int cell = (b << 12) | a | (app.tool << 24);
    if (dragging && cell == app.lastCell) return;
    app.lastCell = cell;
    if (app.tool == 0) editPaint(app, a, b); else if (app.tool == 2) editPath(app, a, b, lower); else if (app.tool == 3) editWall(app, lower); else if (app.tool == 4) editBuilding(app, a, b, lower); else editRaise(app, a, b, (lower != (app.raiseSign < 0)) ? -1 : 1);
}

static void drawCursor(App& app) {
    const Terrain& t = app.terrain;
    const float ox = -t.w * kTileSize * 0.5f, oz = -t.h * kTileSize * 0.5f;
    float x0, z0, x1, z1;
    const float r = (float)app.brush;
    if (app.tool != 1) {
        int tx, ty; tileOf(app, app.hitX, app.hitZ, tx, ty);
        x0 = ox + (tx - r) * kTileSize; z0 = oz + (ty - r) * kTileSize;
        x1 = ox + (tx + r + 1) * kTileSize; z1 = oz + (ty + r + 1) * kTileSize;
    } else {
        int cx, cy; cornerOf(app, app.hitX, app.hitZ, cx, cy);
        x0 = ox + (cx - r - 0.5f) * kTileSize; z0 = oz + (cy - r - 0.5f) * kTileSize;
        x1 = ox + (cx + r + 0.5f) * kTileSize; z1 = oz + (cy + r + 0.5f) * kTileSize;
    }
    glDisable(GL_LIGHTING); glDisable(GL_TEXTURE_2D); glDisable(GL_DEPTH_TEST);
    glLineWidth(2.0f);
    glColor3f(1.0f, 0.95f, 0.3f);
    glBegin(GL_LINE_LOOP);
    const int N = 8;
    auto pt = [&](float x, float z) { x = std::clamp(x, ox, -ox); z = std::clamp(z, oz, -oz); glVertex3f(x, t.heightAt(x, z) + 3.0f, z); };
    for (int i = 0; i < N; i++) pt(x0 + (x1 - x0) * i / N, z0);
    for (int i = 0; i < N; i++) pt(x1, z0 + (z1 - z0) * i / N);
    for (int i = 0; i < N; i++) pt(x1 - (x1 - x0) * i / N, z1);
    for (int i = 0; i < N; i++) pt(x0, z1 - (z1 - z0) * i / N);
    glEnd();
    glEnable(GL_DEPTH_TEST);
}

static void render(App& app) {
    glViewport(0, 0, app.drawW, app.drawH);
    glClearColor(0.04f, 0.06f, 0.09f, 1);
    glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

    float upp = 2.0f / (app.zoom * app.dpi);  // world units per drawable pixel
    float hw = app.drawW * upp * 0.5f, hh = app.drawH * upp * 0.5f;
    glMatrixMode(GL_PROJECTION);
    glLoadIdentity();
    glOrtho(-hw, hw, -hh, hh, -kDepthRange, kDepthRange);
    glMatrixMode(GL_MODELVIEW);
    glLoadIdentity();

    // Directional light fixed in eye space. APPROXIMATION: the original's light setup is not decoded.
    const GLfloat pos[4] = {-0.45f, 0.85f, 0.45f, 0.0f};
    glLightfv(GL_LIGHT0, GL_POSITION, pos);
    GLfloat amb[4] = {app.light.ambient[0] * 0.6f, app.light.ambient[1] * 0.6f, app.light.ambient[2] * 0.6f, 1};
    GLfloat dif[4] = {app.light.diffuse[0] * 0.55f, app.light.diffuse[1] * 0.55f, app.light.diffuse[2] * 0.55f, 1};
    glLightfv(GL_LIGHT0, GL_AMBIENT, amb);
    glLightfv(GL_LIGHT0, GL_DIFFUSE, dif);
    const GLfloat none[4] = {0, 0, 0, 1}, white[4] = {1, 1, 1, 1};
    glLightModelfv(GL_LIGHT_MODEL_AMBIENT, none);
    glMaterialfv(GL_FRONT_AND_BACK, GL_AMBIENT_AND_DIFFUSE, white);

    glRotated(pitchFor(app.drawW, app.drawH), 1, 0, 0);
    glRotatef(45.0f + app.rot, 0, 1, 0);
    glTranslatef(-app.camX, 0, -app.camZ);

    glEnable(GL_DEPTH_TEST);
    glEnable(GL_LIGHTING);
    glEnable(GL_LIGHT0);
    glEnable(GL_NORMALIZE);
    glDisable(GL_CULL_FACE);
    glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_MODULATE);
    for (auto& kv : app.batches) {
        // APPROXIMATION: water shimmers by drifting its texture a pixel or so; the original's water animation is not decoded.
        const bool water = app.waterTex.count(kv.first) != 0;
        if (water) {
            glMatrixMode(GL_TEXTURE); glLoadIdentity();
            glTranslatef(0.014f * std::sin((float)app.time * 1.3f), 0.014f * std::cos((float)app.time * 0.9f), 0);
            glMatrixMode(GL_MODELVIEW);
        }
        if (kv.first) { glEnable(GL_TEXTURE_2D); glBindTexture(GL_TEXTURE_2D, kv.first); }
        else { glDisable(GL_TEXTURE_2D); glColor3f(0.4f, 0.45f, 0.4f); }
        glBegin(GL_TRIANGLES);
        for (const Vertex& v : kv.second) {
            glNormal3f(v.nx, v.ny, v.nz);
            glTexCoord2f(v.u, v.v);
            glVertex3f(v.x, v.y, v.z);
        }
        glEnd();
        if (water) { glMatrixMode(GL_TEXTURE); glLoadIdentity(); glMatrixMode(GL_MODELVIEW); }
    }
    glEnable(GL_TEXTURE_2D);
    for (auto& kv : app.wallBatches) {
        glBindTexture(GL_TEXTURE_2D, kv.first);
        glBegin(GL_TRIANGLES);
        for (const Vertex& v : kv.second) { glNormal3f(v.nx, v.ny, v.nz); glTexCoord2f(v.u, v.v); glVertex3f(v.x, v.y, v.z); }
        glEnd();
    }
    if (!app.pathBatches.empty()) {
        glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
        glEnable(GL_ALPHA_TEST); glAlphaFunc(GL_GREATER, 0.04f);
        glDepthMask(GL_FALSE);
        glEnable(GL_TEXTURE_2D);
        for (auto& kv : app.pathBatches) {
            glBindTexture(GL_TEXTURE_2D, kv.first);
            glBegin(GL_TRIANGLES);
            for (const Vertex& v : kv.second) { glNormal3f(v.nx, v.ny, v.nz); glTexCoord2f(v.u, v.v); glVertex3f(v.x, v.y, v.z); }
            glEnd();
        }
        glDepthMask(GL_TRUE); glDisable(GL_ALPHA_TEST); glDisable(GL_BLEND);
    }
    if (!app.mudBatches.empty()) {   // unconnected paths: the same artwork in a muddy brown
        const GLfloat mud[4] = {0.45f, 0.32f, 0.22f, 1.0f};
        glMaterialfv(GL_FRONT_AND_BACK, GL_AMBIENT_AND_DIFFUSE, mud);
        glEnable(GL_BLEND); glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
        glEnable(GL_ALPHA_TEST); glAlphaFunc(GL_GREATER, 0.04f);
        glDepthMask(GL_FALSE);
        glEnable(GL_TEXTURE_2D);
        for (auto& kv : app.mudBatches) {
            glBindTexture(GL_TEXTURE_2D, kv.first);
            glBegin(GL_TRIANGLES);
            for (const Vertex& v : kv.second) { glNormal3f(v.nx, v.ny, v.nz); glTexCoord2f(v.u, v.v); glVertex3f(v.x, v.y, v.z); }
            glEnd();
        }
        glDepthMask(GL_TRUE); glDisable(GL_ALPHA_TEST); glDisable(GL_BLEND);
        const GLfloat white[4] = {1, 1, 1, 1};
        glMaterialfv(GL_FRONT_AND_BACK, GL_AMBIENT_AND_DIFFUSE, white);
    }
    glGetDoublev(GL_MODELVIEW_MATRIX, app.mv);
    app.upp = upp;
    drawProps(app);
    if (app.edit && app.hasHit) drawCursor(app);
}

static bool screenshot(const App& app, const char* file) {
    Rgba img;
    img.w = (uint32_t)app.drawW; img.h = (uint32_t)app.drawH;
    Bytes raw((size_t)img.w * img.h * 4);
    glPixelStorei(GL_PACK_ALIGNMENT, 1);
    glReadPixels(0, 0, app.drawW, app.drawH, GL_RGBA, GL_UNSIGNED_BYTE, raw.data());
    img.px.resize(raw.size());
    for (uint32_t y = 0; y < img.h; y++)  // GL origin is bottom-left
        std::memcpy(&img.px[(size_t)y * img.w * 4], &raw[(size_t)(img.h - 1 - y) * img.w * 4], (size_t)img.w * 4);
    for (size_t i = 3; i < img.px.size(); i += 4) img.px[i] = 255;
    return writePng(file, img);
}

static void pan(App& app, float right, float up) {
    float yaw = (45.0f + app.rot) * 3.14159265f / 180.0f;
    app.camX += std::cos(yaw) * right + std::sin(yaw) * up;
    app.camZ += std::sin(yaw) * right - std::cos(yaw) * up;
}

int main(int argc, char** argv) {
    App app;
    app.gameDir = "game/Program_Files_(ENGLISH)";
    int winW = 1280, winH = 800;
    int centerX = -1, centerY = -1;  // optional start position in tile coordinates
    const char* pngOut = nullptr;
    std::string golfer;
    std::string screenArg;   // menu, property or play (default: the menu unless the run is scripted)
    const char* loadFile = nullptr; const char* saveFile = nullptr; std::string editSpec;
    for (int i = 1; i < argc; i++) {
        std::string a = argv[i];
        auto next = [&]() { return i + 1 < argc ? argv[++i] : (const char*)""; };
        if (a == "--game") app.gameDir = next();
        else if (a == "--theme") { std::string t = next(); for (int k = 0; k < 4; k++) if (strcasecmp(t.c_str(), kThemes[k]) == 0) app.theme = k; }
        else if (a == "--seed") app.seed = (uint32_t)std::strtoul(next(), nullptr, 10);
        else if (a == "--size") std::sscanf(next(), "%dx%d", &winW, &winH);
        else if (a == "--zoom") app.zoom = (float)std::atof(next());
        else if (a == "--rot") app.rot = (float)std::atof(next());
        else if (a == "--png") pngOut = next();
        else if (a == "--time") app.time = std::atof(next());
        else if (a == "--follow") app.follow = true;
        else if (a == "--golfer") golfer = next();
        else if (a == "--mute") app.mute = true;
        else if (a == "--sound-log") app.soundLog = true;
        else if (a == "--sandbox") app.econ.sandbox = true;
        else if (a == "--screen") screenArg = next();
        else if (a == "--cash") app.econ.startCash = std::atof(next());   // test hook: starting cash
        else if (a == "--course") loadFile = next();
        else if (a == "--save") saveFile = next();
        else if (a == "--edit") editSpec = next();
        else if (a == "--panel") { app.panel = std::atoi(next()); app.dockHover = app.panel > 0 ? app.panel - 1 : -1; app.edit = app.panel == 1 || app.panel == 2; }
        else if (a == "--center") std::sscanf(next(), "%d,%d", &centerX, &centerY);
        else { std::fprintf(stderr, "usage: sgview --game DIR [--theme T] [--seed N] [--size WxH] [--zoom Z] [--rot DEG] [--png FILE]\n"); return 2; }
    }
    if (!golfer.empty()) {  // play as a golfer from progolfers.dta (case-insensitive name match)
        Bytes d; std::vector<ProGolfer> pros; std::string err;
        if (!readFile(app.gameDir + "/Themes/Standard/progolfers.dta", d) || !parseProGolfers(std::string(d.begin(), d.end()), pros, err)) {
            std::fprintf(stderr, "error: cannot read progolfers.dta (%s)\n", err.c_str()); return 1;
        }
        auto low = [](std::string s) { for (char& ch : s) ch = (char)std::tolower((unsigned char)ch); return s; };
        const ProGolfer* hit = nullptr;
        for (const auto& g : pros) if (low(g.name).find(low(golfer)) != std::string::npos) { hit = &g; break; }
        if (!hit) { std::fprintf(stderr, "no golfer matching '%s'\n", golfer.c_str()); return 1; }
        for (int k = 0; k < 10; k++) app.skills.v[k] = hit->skill[k];
        std::printf("golfer: %s, skills", hit->name.c_str());
        for (int k = 0; k < 10; k++) std::printf(" %X", hit->skill[k]);
        std::printf("\n");
    }
    if (SDL_Init(SDL_INIT_VIDEO) != 0) { std::fprintf(stderr, "SDL_Init: %s\n", SDL_GetError()); return 1; }
    SDL_GL_SetAttribute(SDL_GL_DEPTH_SIZE, 24);
    SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
    SDL_Window* win = SDL_CreateWindow("SimGolf native: terrain viewer", SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED, winW, winH,
                                       SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE | SDL_WINDOW_ALLOW_HIGHDPI);
    if (!win) { std::fprintf(stderr, "SDL_CreateWindow: %s\n", SDL_GetError()); return 1; }
    SDL_GLContext ctx = SDL_GL_CreateContext(win);
    if (!ctx) { std::fprintf(stderr, "SDL_GL_CreateContext: %s\n", SDL_GetError()); return 1; }
    SDL_GL_SetSwapInterval(1);
    std::printf("GL: %s | %s\n", glGetString(GL_VERSION), glGetString(GL_RENDERER));

    app.mixer = std::make_unique<Mixer>(app.gameDir + "/Sounds");
    if (app.mixer->clipsLoaded() == 0 && app.mixer->list("").empty()) { std::fprintf(stderr, "sound: %s\n", app.mixer->lastError.c_str()); app.mixer.reset(); }
    else if (!app.mute && !pngOut) {
        if (app.audioDev.open(*app.mixer)) startAmbience(app);
        else std::fprintf(stderr, "sound: no audio device, running silent\n");
    } else if (app.soundLog) startAmbience(app);
    app.terrain = Terrain::demoCourse(40, 40, app.seed);
    if (!loadTheme(app, app.theme)) return 1;
    if (!loadUi(app)) std::fprintf(stderr, "ui: could not load the Interface art or KLEPTO__.TTF, starting on the course\n");
    loadStory(app);
    {
        const bool scripted = pngOut || loadFile || !editSpec.empty() || saveFile || !golfer.empty() || app.econ.sandbox || app.follow;
        std::string s = screenArg.empty() ? (scripted ? "play" : "menu") : screenArg;
        if (s == "report" && app.uiOk) app.screen = App::ScreenReport;
        if (app.uiOk && s == "menu") app.screen = App::ScreenMenu;
        else if (app.uiOk && s == "property") app.screen = App::ScreenProperty;
    }
    if (loadFile) {
        std::string err;
        if (!Terrain::load(loadFile, app.terrain, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        rebuildBatches(app); refreshTrees(app);
    }
    // Scripted edits for tests: "p:x,y,type[,vbyte,radius];r:cx,cy,delta[,radius];b:x,y,building"
    for (size_t pos = 0; pos < editSpec.size();) {
        size_t end = editSpec.find(';', pos);
        std::string item = editSpec.substr(pos, end == std::string::npos ? std::string::npos : end - pos);
        pos = end == std::string::npos ? editSpec.size() : end + 1;
        int a = 0, b = 0, c3 = 0, d = 0, r = 0;
        if (item.size() > 2 && item[0] == 'p' && std::sscanf(item.c_str() + 2, "%d,%d,%d,%d,%d", &a, &b, &c3, &d, &r) >= 3) {
            for (int dy = -r; dy <= r; dy++) for (int dx = -r; dx <= r; dx++)
                if (dx * dx + dy * dy <= r * r + r) {
                    app.terrain.paint(a + dx, b + dy, c3, d);
                    if (c3 == 0 || c3 == 1 || c3 == 17 || c3 == 22) app.terrain.flattenTile(a + dx, b + dy);
                }
        } else if (item.size() > 2 && item[0] == 'w' && std::sscanf(item.c_str() + 2, "%d,%d,%d,%d", &a, &b, &c3, &d) >= 3) {
            if (app.terrain.pathKind.size() != app.terrain.type.size()) app.terrain.pathKind.assign(app.terrain.type.size(), 0);
            for (int dy = -d; dy <= d; dy++) for (int dx = -d; dx <= d; dx++) {
                int x = a + dx, y = b + dy;
                if (dx * dx + dy * dy <= d * d + d && x >= 0 && y >= 0 && x < app.terrain.w && y < app.terrain.h) app.terrain.pathKind[(size_t)app.terrain.tileIndex(x, y)] = (uint8_t)c3;
            }
        } else if (item.size() > 2 && item[0] == 'b' && std::sscanf(item.c_str() + 2, "%d,%d,%d", &a, &b, &c3) == 3) {
            app.buildIdx = c3 % kBuildCount;
            editBuilding(app, a, b, false);
            std::printf("building %s at %d,%d: %zu placed, toast '%s' (clubhouse %d,%d)\n", kBuild[app.buildIdx].name, a, b, app.buildings.size(), app.toast.c_str(), app.terrain.clubhouseX, app.terrain.clubhouseY);
        } else if (item.size() > 2 && item[0] == 'k' && std::sscanf(item.c_str() + 2, "%d,%d,%d", &a, &b, &c3) == 3) {
            app.terrain.setWall(a, b, c3, true);
        } else if (item.size() > 2 && item[0] == 'r' && std::sscanf(item.c_str() + 2, "%d,%d,%d,%d", &a, &b, &c3, &d) >= 3) {
            for (int dy = -d; dy <= d; dy++) for (int dx = -d; dx <= d; dx++)
                if (dx * dx + dy * dy <= d * d + d) app.terrain.raiseCorner(a + dx, b + dy, c3);
        } else std::fprintf(stderr, "bad --edit item: %s\n", item.c_str());
    }
    if (!editSpec.empty()) { rebuildBatches(app); refreshTrees(app); app.econ.updateUpkeep(app.terrain); }
    if (saveFile) {
        std::string err;
        if (!app.terrain.save(saveFile, err)) { std::fprintf(stderr, "error: %s\n", err.c_str()); return 1; }
        std::printf("saved course %s\n", saveFile);
    }
    if (centerX >= 0) {
        app.camX = centerX * kTileSize - app.terrain.w * kTileSize * 0.5f + kTileSize * 0.5f;
        app.camZ = centerY * kTileSize - app.terrain.h * kTileSize * 0.5f + kTileSize * 0.5f;
    }

    bool editing = false;
    unsigned shownVersion = 0;
    reportCourse(app, true);
    bool running = true, dragging = false, needShot = pngOut != nullptr;
    double pausedTotal = 0, pauseStart = 0, lastTick = SDL_GetTicks() / 1000.0;
    int frames = 0;
    while (running) {
        SDL_Event e;
        while (SDL_PollEvent(&e)) {
            if (app.screen != App::ScreenPlay && app.uiOk) {   // title menu and property chooser
                if (e.type == SDL_QUIT) running = false;
                else if (app.screen == App::ScreenReport) { if ((e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_F1)) || e.type == SDL_MOUSEBUTTONDOWN) { app.screen = App::ScreenPlay; app.hover = -1; } }
                else if (e.type == SDL_KEYDOWN && e.key.keysym.sym == SDLK_ESCAPE) { if (app.screen == App::ScreenMenu) running = false; else app.screen = App::ScreenMenu; app.hover = -1; }
                else if (e.type == SDL_MOUSEMOTION || (e.type == SDL_MOUSEBUTTONDOWN && e.button.button == SDL_BUTTON_LEFT)) {
                    const bool click = e.type == SDL_MOUSEBUTTONDOWN;
                    const float vx = app.view.toVirtualX((click ? e.button.x : e.motion.x) * app.dpi), vy = app.view.toVirtualY((click ? e.button.y : e.motion.y) * app.dpi);
                    int hit = -1;
                    if (app.screen == App::ScreenMenu) { for (int b = 0; b < 6; b++) if (kMenuBtn[b].has(vx, vy)) hit = b; }
                    else {
                        for (int p = 0; p < 16; p++) if (propertyCard(p).has(vx, vy)) hit = p;
                        if (Rect{748, 538, 46, 46}.has(vx, vy)) hit = 100;
                    }
                    app.hover = hit;
                    if (click && hit >= 0) {
                        if (app.screen == App::ScreenMenu) {
                            if (hit == 0) {
                                std::string err;
                                if (Terrain::load(app.courseFile, app.terrain, err)) { rebuildBatches(app); populateProps(app); app.screen = App::ScreenPlay; app.resetClock = true; app.econ.sandbox = false; }
                                else { app.toast = "No saved game found (course.sgc)"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; }
                            }
                            else if (hit == 1 || hit == 2) { app.sandboxChoice = hit == 2; app.screen = App::ScreenProperty; app.hover = -1; }
                            else if (hit == 3) app.themePack = (app.themePack + 1) % 5;
                            else if (hit == 4) { app.toast = "Championships are not available yet"; app.toastUntil = SDL_GetTicks() / 1000.0 + 3; }
                            else if (hit == 5) running = false;
                        } else {
                            if (hit == 100) { app.screen = App::ScreenMenu; app.hover = -1; }
                            else if (canAfford(app, hit)) startGame(app, hit, app.sandboxChoice);
                        }
                    }
                }
                continue;
            }
            if (e.type == SDL_QUIT) running = false;
            else if (e.type == SDL_KEYDOWN) {
                float step = 60.0f / app.zoom;
                // Hotkeys of the original (manual p. 3 and 4): Z / X zoom, Shift+S / Shift+L save and load, Shift+P pause,
                // Shift+T trees; in edit mode F fairway, G green/tee, R rough, S sandtrap, W water, P pathway, - lower, = raise.
                {
                    const SDL_Keycode k = e.key.keysym.sym;
                    const bool shift = (SDL_GetModState() & KMOD_SHIFT) != 0;
                    bool handled = true;
                    auto pick = [&](int type, int vb) { app.tool = 0; for (int i = 0; i < kPaintCount; i++) if (kPaint[i].type == type && kPaint[i].vbyte == vb) app.paintIdx = i; setTitle(app, win); };
                    if (shift && k == SDLK_s) { std::string err; if (app.terrain.save(app.courseFile, err)) std::printf("saved %s\n", app.courseFile.c_str()); else std::fprintf(stderr, "error: %s\n", err.c_str()); }
                    else if (shift && k == SDLK_l) { std::string err; if (Terrain::load(app.courseFile, app.terrain, err)) { app.dirty = true; std::printf("loaded %s\n", app.courseFile.c_str()); } else std::fprintf(stderr, "error: %s\n", err.c_str()); }
                    else if (shift && k == SDLK_p) { app.paused = !app.paused; if (app.paused) pauseStart = SDL_GetTicks() / 1000.0; else pausedTotal += SDL_GetTicks() / 1000.0 - pauseStart; }
                    else if (shift && k == SDLK_t) app.showProps = !app.showProps;
                    else if ((shift || (SDL_GetModState() & KMOD_CTRL)) && (k == SDLK_c || k == SDLK_r || k == SDLK_g || k == SDLK_v)) {
                        // Hire (Shift) or fire (Ctrl) a Club Pro, Ranger, Groundskeeper or Soda Vendor.
                        const int kind = k == SDLK_c ? Economy::ClubPro : k == SDLK_r ? Economy::Ranger : k == SDLK_g ? Economy::Groundskeeper : Economy::SodaVendor;
                        bool ok = shift ? app.econ.hire(kind) : app.econ.fire(kind);
                        std::printf("%s %s: %s (staff now %d, wages $%.0f a day)\n", shift ? "hire" : "fire", Economy::staffName(kind), ok ? "done" : "not possible", app.econ.staffCount(), app.econ.dailyWages());
                        snd(app, "Interface/Button2.wav"); setTitle(app, win);
                    }
                    else if (k == SDLK_F1) { reportCourse(app, true); if (app.uiOk && app.reportArt.tex) { app.ratings.clear(); app.screen = App::ScreenReport; } }
                    else if (k == SDLK_z) app.zoom *= 1.12f;
                    else if (k == SDLK_x) app.zoom /= 1.12f;
                    else if (app.edit && !shift && k == SDLK_f) pick(TT_Fairway, 0);
                    else if (app.edit && !shift && k == SDLK_g) pick(kPaint[app.paintIdx].type == TT_PuttingGreen && app.tool == 0 ? TT_Tee : TT_PuttingGreen, 0);
                    else if (app.edit && !shift && k == SDLK_r) pick(TT_Rough, 0);
                    else if (app.edit && !shift && k == SDLK_s) pick(7, 0);
                    else if (app.edit && !shift && k == SDLK_w) pick(TT_WaterShallow, 0);
                    else if (app.edit && !shift && k == SDLK_p) { app.tool = 2; setTitle(app, win); }
                    else if (app.edit && !shift && k == SDLK_MINUS) { app.tool = 1; app.raiseSign = -1; setTitle(app, win); }
                    else if (app.edit && !shift && k == SDLK_EQUALS) { app.tool = 1; app.raiseSign = 1; setTitle(app, win); }
                    else handled = false;
                    if (handled) continue;
                }
                switch (e.key.keysym.sym) {
                    case SDLK_ESCAPE: if (app.uiOk) { app.screen = App::ScreenMenu; app.hover = -1; } else running = false; break;
                    case SDLK_LEFT: case SDLK_a: pan(app, -step, 0); break;
                    case SDLK_RIGHT: case SDLK_d: pan(app, step, 0); break;
                    case SDLK_UP: case SDLK_w: pan(app, 0, step); break;
                    case SDLK_DOWN: case SDLK_s: pan(app, 0, -step); break;
                    case SDLK_q: app.rot -= 5; break;
                    case SDLK_e: app.rot += 5; break;
                    case SDLK_EQUALS: case SDLK_PLUS: case SDLK_KP_PLUS: app.zoom *= 1.12f; break;
                    case SDLK_MINUS: case SDLK_KP_MINUS: app.zoom /= 1.12f; break;
                    case SDLK_1: case SDLK_2: case SDLK_3: case SDLK_4: loadTheme(app, e.key.keysym.sym - SDLK_1); break;
                    case SDLK_r: app.seed = app.seed * 1664525u + 1013904223u; app.terrain = Terrain::demoCourse(40, 40, app.seed); rebuildBatches(app); populateProps(app); break;
                    case SDLK_TAB: app.edit = !app.edit; setTitle(app, win); break;
                    case SDLK_t: app.tool = (app.tool + 1) % 5; setTitle(app, win); break;
                    case SDLK_LEFTBRACKET: if (app.tool == 4) { do app.buildIdx = (app.buildIdx + kBuildCount - 1) % kBuildCount; while (!buildAvailable(app, app.buildIdx)); setTitle(app, win); break; } if (app.tool == 3) break; if (app.tool == 2) { app.pathKind = 3 - app.pathKind; setTitle(app, win); break; } app.paintIdx = (app.paintIdx + kPaintCount - 1) % kPaintCount; setTitle(app, win); break;
                    case SDLK_RIGHTBRACKET: if (app.tool == 4) { do app.buildIdx = (app.buildIdx + 1) % kBuildCount; while (!buildAvailable(app, app.buildIdx)); setTitle(app, win); break; } if (app.tool == 3) break; if (app.tool == 2) { app.pathKind = 3 - app.pathKind; setTitle(app, win); break; } app.paintIdx = (app.paintIdx + 1) % kPaintCount; setTitle(app, win); break;
                    case SDLK_COMMA: app.brush = std::max(0, app.brush - 1); setTitle(app, win); break;
                    case SDLK_PERIOD: app.brush = std::min(6, app.brush + 1); setTitle(app, win); break;
                    case SDLK_F5: { std::string err; if (app.terrain.save(app.courseFile, err)) { std::printf("saved %s\n", app.courseFile.c_str()); snd(app, "Interface/Button1.wav"); } else std::fprintf(stderr, "error: %s\n", err.c_str()); break; }
                    case SDLK_F9: { std::string err; if (Terrain::load(app.courseFile, app.terrain, err)) { app.dirty = true; std::printf("loaded %s\n", app.courseFile.c_str()); } else std::fprintf(stderr, "error: %s\n", err.c_str()); break; }
                    case SDLK_m: toggleMusic(app); break;
                    case SDLK_n:
                        app.mute = !app.mute;
                        if (app.mute) { if (app.mixer) app.mixer->stopAll(); app.ambience = app.music = -1; } else { startAmbience(app); if (app.musicOn) { app.musicOn = false; toggleMusic(app); } }
                        break;
                    case SDLK_p: app.showProps = !app.showProps; break;
                    case SDLK_h: app.showAdvisor = !app.showAdvisor; break;
                    case SDLK_f: app.follow = !app.follow; break;
                    case SDLK_F2: if (screenshot(app, "simgolf-shot.png")) std::printf("saved simgolf-shot.png\n"); break;
                    default: break;
                }
            } else if (e.type == SDL_MOUSEWHEEL) {
                app.zoom *= e.wheel.y > 0 ? 1.1f : (e.wheel.y < 0 ? 1 / 1.1f : 1.0f);
            } else if (e.type == SDL_MOUSEBUTTONDOWN && (e.button.button == SDL_BUTTON_LEFT || e.button.button == SDL_BUTTON_RIGHT)
                       && app.uiOk && dockClick(app, app.view.toVirtualX(e.button.x * app.dpi), app.view.toVirtualY(e.button.y * app.dpi), e.button.button == SDL_BUTTON_RIGHT, app.pauseToggle)) {
                setTitle(app, win);
            } else if (e.type == SDL_MOUSEBUTTONDOWN && (e.button.button == SDL_BUTTON_LEFT || e.button.button == SDL_BUTTON_RIGHT)) {
                if (app.edit && e.button.button == SDL_BUTTON_LEFT) { editing = true; app.lastCell = -1; applyTool(app, (SDL_GetModState() & KMOD_SHIFT) != 0, false); }
                else dragging = true;
            }
            else if (e.type == SDL_MOUSEBUTTONUP && (e.button.button == SDL_BUTTON_LEFT || e.button.button == SDL_BUTTON_RIGHT)) { dragging = false; editing = false; }
            else if (e.type == SDL_MOUSEMOTION && app.uiOk && (app.dockHover = -2, dockHoverUpdate(app, app.view.toVirtualX(e.motion.x * app.dpi), app.view.toVirtualY(e.motion.y * app.dpi)), false)) {}
            else if (e.type == SDL_MOUSEMOTION && editing) {
                app.hasHit = pickGround(app, e.motion.x, e.motion.y, app.dpi, app.hitX, app.hitZ);
                applyTool(app, (SDL_GetModState() & KMOD_SHIFT) != 0, true);
            }
            else if (e.type == SDL_MOUSEMOTION && dragging) {
                int ww, wh; SDL_GetWindowSize(win, &ww, &wh);
                (void)ww;
                float k = 2.0f / app.zoom;  // window points -> world units
                float sinP = (float)std::sin(pitchFor(app.drawW, app.drawH) * 3.14159265 / 180.0);
                pan(app, -e.motion.xrel * k, e.motion.yrel * k / sinP);
            }
        }
        if (app.pauseToggle) { app.pauseToggle = false; app.paused = !app.paused; if (app.paused) pauseStart = SDL_GetTicks() / 1000.0; else pausedTotal += SDL_GetTicks() / 1000.0 - pauseStart; }
        if (!app.storyLines.empty() && !app.paused && SDL_GetTicks() / 1000.0 > app.storyNext) { app.storyPos++; app.storyNext = SDL_GetTicks() / 1000.0 + 7.0; }
        if (app.edit) { int mx, my; SDL_GetMouseState(&mx, &my); app.hasHit = pickGround(app, mx, my, app.dpi, app.hitX, app.hitZ); }
        if (app.dirty) { rebuildBatches(app); refreshTrees(app); app.econ.updateUpkeep(app.terrain); app.dirty = false; reportCourse(app, false); setTitle(app, win); }
        if (app.econ.version != shownVersion) { shownVersion = app.econ.version; setTitle(app, win); }
        {
            const double now = SDL_GetTicks() / 1000.0;
            if (app.resetClock) { pausedTotal = now; app.time = 0; app.resetClock = false; }
            if (app.screen != App::ScreenPlay && !app.paused) pausedTotal += now - lastTick;   // the club does not run while a menu is open
            lastTick = now;
            if (!pngOut && !app.paused && app.screen == App::ScreenPlay) app.time = now - pausedTotal;
        }
        SDL_GL_GetDrawableSize(win, &app.drawW, &app.drawH);
        { int ww, wh; SDL_GetWindowSize(win, &ww, &wh); app.dpi = ww > 0 ? (float)app.drawW / (float)ww : 1.0f; }
        if (app.screen == App::ScreenMenu && app.uiOk) drawMenu(app);
        else if (app.screen == App::ScreenProperty && app.uiOk) drawProperty(app);
        else { render(app); drawHud(app); if (app.screen == App::ScreenReport) drawReport(app); }
        if (getenv("SG_PICKTEST") && frames == 1) {  // project known ground points to the screen and pick them back
            float worst = 0;
            for (float wx : {-600.f, 0.f, 750.f}) for (float wz : {-500.f, 100.f, 900.f}) {
                float y = app.terrain.heightAt(wx, wz);
                double ex = app.mv[0] * wx + app.mv[4] * y + app.mv[8] * wz + app.mv[12], ey = app.mv[1] * wx + app.mv[5] * y + app.mv[9] * wz + app.mv[13];
                int sx = (int)std::lround((ex / app.upp + app.drawW * 0.5) / app.dpi), sy = (int)std::lround((app.drawH * 0.5 - ey / app.upp) / app.dpi);
                float px, pz;
                bool ok = pickGround(app, sx, sy, app.dpi, px, pz);
                float err = ok ? std::hypot(px - wx, pz - wz) : 9999;
                worst = std::max(worst, err); std::printf("  (%.0f,%.0f) y=%.1f -> %.1f,%.1f err %.1f\n", wx, wz, y, px, pz, err);
            }
            std::printf("pick test: worst error %.2f world units (1 px = %.2f)\n", worst, app.upp);
        }
        if (needShot && ++frames >= 2) {  // let the first frame settle
            if (!screenshot(app, pngOut)) { std::fprintf(stderr, "could not write %s\n", pngOut); return 1; }
            std::printf("saved %s (%dx%d)\n", pngOut, app.drawW, app.drawH);
            break;
        }
        SDL_GL_SwapWindow(win);
    }
    SDL_GL_DeleteContext(ctx);
    SDL_DestroyWindow(win);
    SDL_Quit();
    return 0;
}
